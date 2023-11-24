use super::{callbacks::Callbacks, Bucket};
use anyhow::Error as AnyError;
use assert_impl::assert_impl;
use log::warn;
use qiniu_apis::{
    http::ResponseErrorKind as HttpResponseErrorKind,
    http_client::{ApiResult, RegionsProvider, RegionsProviderEndpoints, Response, ResponseError, SyncResponseBody},
    storage::get_objects::{
        ListedObjectEntry, QueryParams, ResponseBody as GetObjectsV1ResponseBody,
        SyncRequestBuilder as GetObjectsV1SyncRequestBuilder,
    },
    storage::get_objects_v2::SyncRequestBuilder as GetObjectsV2SyncRequestBuilder,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::VecDeque,
    fmt::{self, Debug},
    io::{BufRead, BufReader, Lines},
};
use tap::prelude::*;

#[cfg(feature = "async")]
use {futures::io::BufReader as AsyncBufReader, qiniu_apis::http_client::AsyncResponseBody};

type RefRegionProviderEndpoints<'a> = RegionsProviderEndpoints<&'a dyn RegionsProvider>;

#[derive(Debug, Clone)]
struct ListParams<'a> {
    bucket: &'a Bucket,
    prefix: Option<Cow<'a, str>>,
    limit: Limit,
    marker: Marker<'a>,
    need_parts: bool,
}

#[derive(Debug, Clone)]
enum Marker<'a> {
    Original(Option<Cow<'a, str>>),
    Subsequent(Option<String>),
}

impl<'a> Marker<'a> {
    fn new(marker: Option<Cow<'a, str>>) -> Self {
        Self::Original(marker)
    }

    fn empty(&self) -> bool {
        matches!(self.as_ref().map(|s| s.is_empty()), Some(true) | None)
    }

    fn as_ref(&self) -> Option<&str> {
        match self {
            Self::Original(marker) => marker.as_deref(),
            Self::Subsequent(marker) => marker.as_deref(),
        }
    }

    fn set(&mut self, marker: Option<&str>) {
        *self = Self::Subsequent(marker.map(|s| s.to_owned()));
    }

    fn is_original(&self) -> bool {
        matches!(self, Self::Original(..))
    }
}

#[derive(Copy, Debug, Clone)]
struct Limit {
    limit: Option<usize>,
    max: Option<usize>,
}

impl Limit {
    fn new(limit: Option<usize>, version: ListVersion) -> Self {
        Self {
            limit,
            max: version.page_limit(),
        }
    }

    fn as_ref(&self) -> Option<usize> {
        match (self.limit, self.max) {
            (Some(limit), Some(max)) => Some(limit.min(max)),
            (Some(limit), None) => Some(limit),
            (None, Some(max)) => Some(max),
            (None, None) => None,
        }
    }

    fn exhausted(&self) -> bool {
        matches!(self.limit, Some(0))
    }

    fn saturating_decrease(&mut self, sub: usize) {
        if let Some(limit) = self.limit.as_mut() {
            *limit = limit.saturating_sub(sub);
        }
    }
}

impl<'a> ListParams<'a> {
    fn to_query_params(&self) -> QueryParams<'a> {
        let mut query_params = QueryParams::default().set_bucket_as_str(self.bucket.name().as_str());
        if let Some(marker) = self.marker.as_ref() {
            query_params = query_params.set_marker_as_str(marker.to_owned());
        }
        if let Some(limit) = self.limit.as_ref() {
            query_params = query_params.set_limit_as_usize(limit);
        }
        if let Some(prefix) = self.prefix.as_ref() {
            query_params = query_params.set_prefix_as_str(prefix.clone());
        }
        if self.need_parts {
            query_params = query_params.set_need_parts_as_bool(true);
        }
        query_params
    }

    fn have_done(&self) -> bool {
        self.limit.exhausted() || !self.marker.is_original() && self.marker.empty()
    }
}

/// 对象列举迭代器
///
/// 实现 [`std::iter::Iterator`] 接口，
/// 在迭代过程中阻塞发起 API 列举对象信息。
///
/// 可以通过 [`crate::ListBuilder::iter`] 方法获取该迭代器。
#[must_use]
pub struct ListIter<'a> {
    params: ListParams<'a>,
    version: SyncListVersionWithStep,
    callbacks: Callbacks<'a>,
}

impl ListIter<'_> {
    /// 获取上一次列举返回的位置标记
    pub fn marker(&self) -> Option<&str> {
        self.params.marker.as_ref()
    }
}

impl Debug for ListIter<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListIter")
            .field("params", &self.params)
            .field("version", &self.version)
            .finish()
    }
}

/// 列举 API 版本
///
/// 目前支持 V1 和 V2，默认为 V1
#[derive(Copy, Clone, Debug, Default)]
#[non_exhaustive]
pub enum ListVersion {
    /// 列举 API V1
    #[default]
    V1,

    /// 列举 API V2
    V2,
}

impl ListVersion {
    fn page_limit(self) -> Option<usize> {
        const V1_PAGE_SIZE_MAX: usize = 1000;

        match self {
            Self::V1 => Some(V1_PAGE_SIZE_MAX),
            Self::V2 => None,
        }
    }
}

#[derive(Debug)]
enum SyncListVersionWithStep {
    V1(SyncListV1Step),
    V2(SyncListV2Step),
}

impl From<ListVersion> for SyncListVersionWithStep {
    fn from(version: ListVersion) -> Self {
        match version {
            ListVersion::V1 => Self::V1(Default::default()),
            ListVersion::V2 => Self::V2(Default::default()),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum SyncListV1Step {
    Buffer {
        buffer: VecDeque<ListedObjectEntry>,
    },
    Done,
}

impl Default for SyncListV1Step {
    #[inline]
    fn default() -> Self {
        Self::Buffer { buffer: Default::default() }
    }
}

#[derive(Debug, Default)]
pub(super) enum SyncListV2Step {
    #[default]
    Start,
    Lines {
        lines: Lines<BufReader<SyncResponseBody>>,
    },
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ListedObjectEntryV2 {
    item: Option<ListedObjectEntry>,
    marker: Option<String>,
}

impl<'a> ListIter<'a> {
    pub(super) fn new(
        bucket: &'a Bucket,
        limit: Option<usize>,
        prefix: Option<Cow<'a, str>>,
        marker: Option<Cow<'a, str>>,
        need_parts: bool,
        version: ListVersion,
        callbacks: Callbacks<'a>,
    ) -> Self {
        Self {
            callbacks,
            version: version.into(),
            params: ListParams {
                bucket,
                prefix,
                need_parts,
                limit: Limit::new(limit, version),
                marker: Marker::new(marker),
            },
        }
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        // assert_impl!(Sync: Self);
    }
}

impl Iterator for ListIter<'_> {
    type Item = ApiResult<ListedObjectEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        return match &mut self.version {
            SyncListVersionWithStep::V1(step) => v1_next(&mut self.params, &mut self.callbacks, step),
            SyncListVersionWithStep::V2(step) => v2_next(&mut self.params, &mut self.callbacks, step),
        };

        fn v1_next(
            params: &mut ListParams<'_>,
            callbacks: &mut Callbacks<'_>,
            step: &mut SyncListV1Step,
        ) -> Option<ApiResult<ListedObjectEntry>> {
            match step {
                SyncListV1Step::Buffer { buffer } => {
                    if let Some(object) = buffer.pop_front() {
                        Some(Ok(object))
                    } else {
                        match v1_next_page(params, callbacks, buffer) {
                            Ok(true) => {
                                *step = SyncListV1Step::Done;
                                None
                            }
                            Ok(false) => buffer.pop_front().map(Ok),
                            Err(err) => {
                                *step = SyncListV1Step::Done;
                                Some(Err(err))
                            }
                        }
                    }
                }
                SyncListV1Step::Done => None,
            }
        }

        fn v1_next_page(
            params: &mut ListParams<'_>,
            callbacks: &mut Callbacks<'_>,
            buffer: &mut VecDeque<ListedObjectEntry>,
        ) -> ApiResult<bool> {
            let mut have_done = false;
            if params.have_done() {
                have_done = true;
            } else {
                let request = v1_make_request(params)?;
                let response_result = v1_call_request(request, callbacks);
                v1_handle_response(response_result?.into_body(), params, buffer);
            }
            Ok(have_done)
        }

        fn v1_make_request<'a>(
            params: &mut ListParams<'a>,
        ) -> ApiResult<GetObjectsV1SyncRequestBuilder<'a, RefRegionProviderEndpoints<'a>>> {
            let mut request = params
                .bucket
                .objects_manager()
                .client()
                .storage()
                .get_objects()
                .new_request(
                    RegionsProviderEndpoints::new(params.bucket.region_provider()?),
                    params.bucket.objects_manager().credential(),
                );
            request.query_pairs(params.to_query_params());
            Ok(request)
        }

        fn v1_call_request(
            mut request: GetObjectsV1SyncRequestBuilder<'_, RefRegionProviderEndpoints>,
            callbacks: &mut Callbacks<'_>,
        ) -> ApiResult<Response<GetObjectsV1ResponseBody>> {
            callbacks
                .before_request(request.parts_mut())
                .map_err(make_callback_error)?;
            let mut response_result = request.call();
            callbacks
                .after_response(&mut response_result)
                .map_err(make_callback_error)?;
            response_result
        }

        fn v1_handle_response(
            body: GetObjectsV1ResponseBody,
            params: &mut ListParams<'_>,
            buffer: &mut VecDeque<ListedObjectEntry>,
        ) {
            params.marker.set(body.get_marker_as_str());
            let listed_object_entries = body.get_items().to_listed_object_entry_vec();
            params.limit.saturating_decrease(listed_object_entries.len());
            *buffer = listed_object_entries.into();
        }

        fn v2_next(
            params: &mut ListParams<'_>,
            callbacks: &mut Callbacks<'_>,
            step: &mut SyncListV2Step,
        ) -> Option<ApiResult<ListedObjectEntry>> {
            match step {
                SyncListV2Step::Start => match v2_call(params, callbacks) {
                    Ok(Some(mut lines)) => v2_read_entry_from_lines(params, &mut lines).tap_some(|result| {
                        if result.is_ok() {
                            *step = SyncListV2Step::Lines { lines };
                        } else {
                            *step = SyncListV2Step::Done;
                        }
                    }),
                    Ok(None) => {
                        *step = SyncListV2Step::Done;
                        None
                    }
                    Err(err) => {
                        *step = SyncListV2Step::Done;
                        Some(Err(err))
                    }
                },
                SyncListV2Step::Lines { lines } => match v2_read_entry_from_lines(params, lines) {
                    Some(Ok(entry)) => Some(Ok(entry)),
                    Some(Err(err)) => {
                        warn!("Read Error from ListV2 Response Body: {}", err);
                        *step = SyncListV2Step::Start;
                        v2_next(params, callbacks, step)
                    }
                    None => {
                        *step = SyncListV2Step::Start;
                        v2_next(params, callbacks, step)
                    }
                },
                SyncListV2Step::Done => None,
            }
        }

        fn v2_read_entry_from_lines(
            params: &mut ListParams<'_>,
            lines: &mut Lines<BufReader<SyncResponseBody>>,
        ) -> Option<ApiResult<ListedObjectEntry>> {
            if params.limit.exhausted() {
                return None;
            }
            loop {
                match lines.next() {
                    Some(Ok(line)) if line.is_empty() => {
                        continue;
                    }
                    Some(Ok(line)) => match serde_json::from_str::<ListedObjectEntryV2>(&line) {
                        Ok(parsed) => {
                            params.marker.set(parsed.marker.as_deref());
                            if let Some(item) = parsed.item {
                                params.limit.saturating_decrease(1);
                                return Some(Ok(item));
                            } else {
                                continue;
                            }
                        }
                        Err(err) => {
                            return Some(Err(err.into()));
                        }
                    },
                    Some(Err(err)) => {
                        return Some(Err(err.into()));
                    }
                    None => {
                        return None;
                    }
                }
            }
        }

        fn v2_call(
            params: &mut ListParams<'_>,
            callbacks: &mut Callbacks<'_>,
        ) -> ApiResult<Option<Lines<BufReader<SyncResponseBody>>>> {
            if params.have_done() {
                return Ok(None);
            }
            let request = v2_make_request(params)?;
            let response_result = v2_call_request(request, callbacks);
            Ok(Some(BufReader::new(response_result?.into_body()).lines()))
        }

        fn v2_make_request<'a>(
            params: &mut ListParams<'a>,
        ) -> ApiResult<GetObjectsV2SyncRequestBuilder<'a, RefRegionProviderEndpoints<'a>>> {
            let mut request = params
                .bucket
                .objects_manager()
                .client()
                .storage()
                .get_objects_v2()
                .new_request(
                    RegionsProviderEndpoints::new(params.bucket.region_provider()?),
                    params.bucket.objects_manager().credential(),
                );
            request.query_pairs(params.to_query_params());
            Ok(request)
        }

        fn v2_call_request(
            mut request: GetObjectsV2SyncRequestBuilder<'_, RefRegionProviderEndpoints>,
            callbacks: &mut Callbacks<'_>,
        ) -> ApiResult<Response<SyncResponseBody>> {
            callbacks
                .before_request(request.parts_mut())
                .map_err(make_callback_error)?;
            let mut response_result = request.call();
            callbacks
                .after_response(&mut response_result)
                .map_err(make_callback_error)?;
            response_result
        }
    }
}

#[cfg(feature = "async")]
mod async_list_stream {
    use super::*;
    use futures::{future::BoxFuture, io::Lines as AsyncLines, ready, AsyncBufReadExt, FutureExt, Stream, StreamExt};
    use std::{
        fmt::{self, Debug},
        io::Result as IOResult,
        pin::Pin,
        task::{Context, Poll},
    };

    enum AsyncListVersionWithStep<'a> {
        V1(AsyncListV1Step<'a>),
        V2(AsyncListV2Step<'a>),
    }

    impl From<ListVersion> for AsyncListVersionWithStep<'_> {
        fn from(version: ListVersion) -> Self {
            match version {
                ListVersion::V1 => Self::V1(Default::default()),
                ListVersion::V2 => Self::V2(Default::default()),
            }
        }
    }

    enum AsyncListV1Step<'a> {
        FromBuffer {
            buffer: VecDeque<ListedObjectEntry>,
        },
        WaitForResponse {
            task: BoxFuture<'a, ApiResult<Response<GetObjectsV1ResponseBody>>>,
        },
        WaitForRegionProvider {
            task: BoxFuture<'a, IOResult<&'a dyn RegionsProvider>>,
        },
        Done,
    }

    impl Default for AsyncListV1Step<'_> {
        #[inline]
        fn default() -> Self {
            Self::FromBuffer { buffer: Default::default() }
        }
    }

    impl Debug for AsyncListV1Step<'_> {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::FromBuffer { buffer } => f.debug_tuple("FromBuffer").field(buffer).finish(),
                Self::WaitForResponse { .. } => f.debug_tuple("WaitForResponse").finish(),
                Self::WaitForRegionProvider { .. } => f.debug_tuple("WaitForRegionProvider").finish(),
                Self::Done => f.debug_tuple("Done").finish(),
            }
        }
    }

    trait StreamWithMarker: Stream {
        fn marker(&self) -> Option<&str>;
    }
    type BoxStreamWithMarker<'a, T> = Pin<Box<dyn StreamWithMarker<Item = T> + Send + 'a>>;
    type ListedObjectEntryResultStream<'a> = BoxStreamWithMarker<'a, ApiResult<ListedObjectEntry>>;

    /// 对象列举流
    ///
    /// 实现 [`futures::stream::Stream`] 接口，
    /// 在迭代过程中异步发起 API 列举对象信息
    ///
    /// 可以通过 [`crate::ListBuilder::stream`] 方法获取该迭代器。
    #[must_use]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub struct ListStream<'a>(ListedObjectEntryResultStream<'a>);

    impl<'a> ListStream<'a> {
        pub(in super::super) fn new(
            bucket: &'a Bucket,
            limit: Option<usize>,
            prefix: Option<Cow<'a, str>>,
            marker: Option<Cow<'a, str>>,
            need_parts: bool,
            version: ListVersion,
            callbacks: Callbacks<'a>,
        ) -> Self {
            Self(match version {
                ListVersion::V1 => v1_next(bucket, limit, prefix, marker, need_parts, callbacks),
                ListVersion::V2 => v2_next(bucket, limit, prefix, marker, need_parts, callbacks),
            })
        }

        #[allow(dead_code)]
        fn assert() {
            assert_impl!(Send: Self);
            // assert_impl!(Sync: Self);
        }

        /// 获取上一次列举返回的位置标记
        pub fn marker(&self) -> Option<&str> {
            self.0.marker()
        }
    }

    impl Stream for ListStream<'_> {
        type Item = ApiResult<ListedObjectEntry>;

        #[inline]
        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            self.0.poll_next_unpin(cx)
        }
    }

    impl Debug for ListStream<'_> {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("ListStream").finish()
        }
    }

    #[derive(Debug)]
    struct ListV1Stream<'a> {
        params: ListParams<'a>,
        callbacks: Callbacks<'a>,
        current_step: AsyncListV1Step<'a>,
    }

    fn v1_next<'a>(
        bucket: &'a Bucket,
        limit: Option<usize>,
        prefix: Option<Cow<'a, str>>,
        marker: Option<Cow<'a, str>>,
        need_parts: bool,
        callbacks: Callbacks<'a>,
    ) -> ListedObjectEntryResultStream<'a> {
        let params = ListParams {
            bucket,
            prefix,
            need_parts,
            limit: Limit::new(limit, ListVersion::V1),
            marker: Marker::new(marker),
        };
        Box::pin(ListV1Stream {
            params,
            callbacks,
            current_step: Default::default(),
        })
    }

    impl Stream for ListV1Stream<'_> {
        type Item = ApiResult<ListedObjectEntry>;

        #[inline]
        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            match self.current_step {
                AsyncListV1Step::FromBuffer { .. } => self.read_from_buffer(cx),
                AsyncListV1Step::WaitForResponse { .. } => self.wait_for_response(cx),
                AsyncListV1Step::WaitForRegionProvider { .. } => self.wait_for_region(cx),
                AsyncListV1Step::Done => Poll::Ready(None),
            }
        }
    }

    impl StreamWithMarker for ListV1Stream<'_> {
        fn marker(&self) -> Option<&str> {
            self.params.marker.as_ref()
        }
    }

    impl ListV1Stream<'_> {
        fn read_from_buffer(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<<Self as Stream>::Item>> {
            if let AsyncListV1Step::FromBuffer { buffer } = &mut self.current_step {
                if let Some(object) = buffer.pop_front() {
                    Poll::Ready(Some(Ok(object)))
                } else {
                    if self.params.have_done() {
                        self.current_step = AsyncListV1Step::Done;
                    } else {
                        let bucket = self.params.bucket;
                        self.current_step = AsyncListV1Step::WaitForRegionProvider {
                            task: Box::pin(async move { bucket.async_region_provider().await }),
                        };
                    }
                    self.poll_next(cx)
                }
            } else {
                unreachable!()
            }
        }

        fn wait_for_region(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<<Self as Stream>::Item>> {
            if let AsyncListV1Step::WaitForRegionProvider { task } = &mut self.current_step {
                match ready!(task.poll_unpin(cx)) {
                    Ok(region_provider) => {
                        let credential = self.params.bucket.objects_manager().credential();
                        let mut request = self
                            .params
                            .bucket
                            .objects_manager()
                            .client()
                            .storage()
                            .get_objects()
                            .new_async_request(RegionsProviderEndpoints::new(region_provider), credential);
                        request.query_pairs(self.params.to_query_params());
                        if let Err(err) = self.callbacks.before_request(request.parts_mut()) {
                            self.current_step = AsyncListV1Step::Done;
                            Poll::Ready(Some(Err(make_callback_error(err))))
                        } else {
                            self.current_step = AsyncListV1Step::WaitForResponse {
                                task: Box::pin(async move { request.call().await }),
                            };
                            self.poll_next(cx)
                        }
                    }
                    Err(err) => {
                        self.current_step = AsyncListV1Step::Done;
                        Poll::Ready(Some(Err(err.into())))
                    }
                }
            } else {
                unreachable!()
            }
        }

        fn wait_for_response(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<<Self as Stream>::Item>> {
            if let AsyncListV1Step::WaitForResponse { task } = &mut self.current_step {
                let mut response_result = ready!(task.poll_unpin(cx));
                if let Err(err) = self.callbacks.after_response(&mut response_result) {
                    self.current_step = AsyncListV1Step::Done;
                    Poll::Ready(Some(Err(make_callback_error(err))))
                } else {
                    match response_result {
                        Ok(response) => {
                            let body = response.into_body();
                            let listed_object_entries = body.get_items().to_listed_object_entry_vec();
                            self.params.marker.set(body.get_marker_as_str());
                            self.params.limit.saturating_decrease(listed_object_entries.len());
                            self.current_step = AsyncListV1Step::FromBuffer {
                                buffer: listed_object_entries.into(),
                            };
                            self.poll_next(cx)
                        }
                        Err(err) => {
                            self.current_step = AsyncListV1Step::Done;
                            Poll::Ready(Some(Err(err)))
                        }
                    }
                }
            } else {
                unreachable!()
            }
        }
    }

    #[derive(Default)]
    enum AsyncListV2Step<'a> {
        #[default]
        Start,
        WaitForRegionProvider {
            task: BoxFuture<'a, IOResult<&'a dyn RegionsProvider>>,
        },
        WaitForResponse {
            task: BoxFuture<'a, ApiResult<Response<AsyncResponseBody>>>,
        },
        WaitForEntries {
            lines: AsyncLines<AsyncBufReader<AsyncResponseBody>>,
            empty: bool,
        },
        Done,
    }

    struct ListV2Stream<'a> {
        params: ListParams<'a>,
        callbacks: Callbacks<'a>,
        current_step: AsyncListV2Step<'a>,
    }

    #[allow(clippy::too_many_arguments)]
    fn v2_next<'a>(
        bucket: &'a Bucket,
        limit: Option<usize>,
        prefix: Option<Cow<'a, str>>,
        marker: Option<Cow<'a, str>>,
        need_parts: bool,
        callbacks: Callbacks<'a>,
    ) -> ListedObjectEntryResultStream<'a> {
        let params = ListParams {
            bucket,
            prefix,
            need_parts,
            limit: Limit::new(limit, ListVersion::V2),
            marker: Marker::new(marker),
        };
        Box::pin(ListV2Stream {
            params,
            callbacks,
            current_step: Default::default(),
        })
    }

    impl Stream for ListV2Stream<'_> {
        type Item = ApiResult<ListedObjectEntry>;

        #[inline]
        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            match self.current_step {
                AsyncListV2Step::Start { .. } => self.start(cx),
                AsyncListV2Step::WaitForResponse { .. } => self.wait_for_response(cx),
                AsyncListV2Step::WaitForRegionProvider { .. } => self.wait_for_region(cx),
                AsyncListV2Step::WaitForEntries { .. } => self.wait_for_entries(cx),
                AsyncListV2Step::Done => Poll::Ready(None),
            }
        }
    }

    impl StreamWithMarker for ListV2Stream<'_> {
        fn marker(&self) -> Option<&str> {
            self.params.marker.as_ref()
        }
    }

    impl ListV2Stream<'_> {
        fn start(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<<Self as Stream>::Item>> {
            if let AsyncListV2Step::Start { .. } = &mut self.current_step {
                if self.params.have_done() {
                    self.current_step = AsyncListV2Step::Done;
                } else {
                    let bucket = self.params.bucket;
                    self.current_step = AsyncListV2Step::WaitForRegionProvider {
                        task: Box::pin(async move { bucket.async_region_provider().await }),
                    };
                }
                self.poll_next(cx)
            } else {
                unreachable!()
            }
        }

        fn wait_for_region(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<<Self as Stream>::Item>> {
            if let AsyncListV2Step::WaitForRegionProvider { task } = &mut self.current_step {
                match ready!(task.poll_unpin(cx)) {
                    Ok(region_provider) => {
                        let credential = self.params.bucket.objects_manager().credential();
                        let mut request = self
                            .params
                            .bucket
                            .objects_manager()
                            .client()
                            .storage()
                            .get_objects_v2()
                            .new_async_request(RegionsProviderEndpoints::new(region_provider), credential);
                        request.query_pairs(self.params.to_query_params());
                        if let Err(err) = self.callbacks.before_request(request.parts_mut()) {
                            self.current_step = AsyncListV2Step::Done;
                            Poll::Ready(Some(Err(make_callback_error(err))))
                        } else {
                            self.current_step = AsyncListV2Step::WaitForResponse {
                                task: Box::pin(async move { request.call().await }),
                            };
                            self.poll_next(cx)
                        }
                    }
                    Err(err) => {
                        self.current_step = AsyncListV2Step::Done;
                        Poll::Ready(Some(Err(err.into())))
                    }
                }
            } else {
                unreachable!()
            }
        }

        fn wait_for_response(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<<Self as Stream>::Item>> {
            if let AsyncListV2Step::WaitForResponse { task } = &mut self.current_step {
                let mut response_result = ready!(task.poll_unpin(cx));
                if let Err(err) = self.callbacks.after_response(&mut response_result) {
                    self.current_step = AsyncListV2Step::Done;
                    Poll::Ready(Some(Err(make_callback_error(err))))
                } else {
                    match response_result {
                        Ok(response) => {
                            self.current_step = AsyncListV2Step::WaitForEntries {
                                lines: AsyncBufReader::new(response.into_body()).lines(),
                                empty: true,
                            };
                            self.poll_next(cx)
                        }
                        Err(err) => {
                            self.current_step = AsyncListV2Step::Done;
                            Poll::Ready(Some(Err(err)))
                        }
                    }
                }
            } else {
                unreachable!()
            }
        }

        fn wait_for_entries(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<<Self as Stream>::Item>> {
            if let AsyncListV2Step::WaitForEntries { lines, ref mut empty } = &mut self.current_step {
                match ready!(lines.poll_next_unpin(cx)) {
                    Some(Ok(line)) if line.is_empty() => self.wait_for_entries(cx),
                    Some(Ok(line)) => match serde_json::from_str::<ListedObjectEntryV2>(&line) {
                        Ok(parsed) => {
                            *empty = false;
                            self.params.marker.set(parsed.marker.as_deref());
                            if let Some(item) = parsed.item {
                                self.params.limit.saturating_decrease(1);
                                Poll::Ready(Some(Ok(item)))
                            } else {
                                self.wait_for_entries(cx)
                            }
                        }
                        Err(err) => {
                            self.current_step = AsyncListV2Step::Done;
                            Poll::Ready(Some(Err(err.into())))
                        }
                    },
                    Some(Err(err)) => {
                        self.current_step = AsyncListV2Step::Done;
                        Poll::Ready(Some(Err(err.into())))
                    }
                    None if *empty => {
                        self.current_step = AsyncListV2Step::Done;
                        Poll::Ready(None)
                    }
                    None => {
                        self.current_step = AsyncListV2Step::Start;
                        self.poll_next(cx)
                    }
                }
            } else {
                unreachable!()
            }
        }
    }
}

pub(super) fn make_callback_error(err: AnyError) -> ResponseError {
    ResponseError::new_with_msg(HttpResponseErrorKind::CallbackError.into(), err)
}

#[cfg(feature = "async")]
pub use async_list_stream::*;

#[cfg(test)]
mod tests {
    use super::{super::ObjectsManager, *};
    use anyhow::Error as AnyError;
    use qiniu_apis::{
        credential::Credential,
        http::{HeaderValue, HttpCaller, StatusCode, SyncRequest, SyncResponse, SyncResponseResult},
        http_client::{BucketName, DirectChooser, HttpClient, NeverRetrier, Region, ResponseErrorKind, NO_BACKOFF},
    };
    use serde_json::{json, to_string as json_to_string, to_vec as json_to_vec};
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::{SystemTime, UNIX_EPOCH},
    };

    #[cfg(feature = "async")]
    use {
        futures::{future::BoxFuture, StreamExt, TryStreamExt},
        qiniu_apis::http::{AsyncRequest, AsyncResponse, AsyncResponseResult},
    };

    #[test]
    fn test_sync_list_v1() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                let n = self.counter.fetch_add(1, Ordering::SeqCst);
                let body = match n {
                    0 => {
                        assert!(request
                            .url()
                            .to_string()
                            .ends_with("/list?bucket=fakebucketname&limit=1000"));
                        SyncResponseBody::from_bytes(
                            json_to_vec(&json!({
                                "marker": "fakemarker",
                                "items": [{
                                    "key": "fakeobj1",
                                    "put_time": generate_put_time(),
                                    "hash": "fakeobj1hash",
                                    "fsize": 1usize,
                                    "mime_type": "text/plain",
                                }, {
                                    "key": "fakeobj2",
                                    "put_time": generate_put_time(),
                                    "hash": "fakeobj2hash",
                                    "fsize": 2usize,
                                    "mime_type": "text/plain",
                                }]
                            }))
                            .unwrap(),
                        )
                    }
                    1 => {
                        assert!(request
                            .url()
                            .to_string()
                            .ends_with("/list?bucket=fakebucketname&marker=fakemarker&limit=1000"));
                        SyncResponseBody::from_bytes(
                            json_to_vec(&json!({
                                "marker": "",
                                "items": [{
                                    "key": "fakeobj3",
                                    "put_time": generate_put_time(),
                                    "hash": "fakeobj3hash",
                                    "fsize": 3usize,
                                    "mime_type": "text/plain",
                                }, {
                                    "key": "fakeobj4",
                                    "put_time": generate_put_time(),
                                    "hash": "fakeobj4hash",
                                    "fsize": 4usize,
                                    "mime_type": "text/plain",
                                }]
                            }))
                            .unwrap(),
                        )
                    }
                    _ => unreachable!(),
                };
                Ok(SyncResponse::builder()
                    .status_code(StatusCode::OK)
                    .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                    .body(body)
                    .build())
            }

            #[cfg(feature = "async")]
            fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let mut counter = 0usize;
        let bucket = get_bucket(FakeHttpCaller::default());
        let mut iter = bucket.list().version(ListVersion::V1).iter();
        for (i, entry) in (&mut iter).enumerate() {
            counter += 1;
            let entry = entry?;
            assert_eq!(entry.get_key_as_str(), &format!("fakeobj{}", i + 1));
            assert_eq!(entry.get_hash_as_str(), &format!("fakeobj{}hash", i + 1));
            assert_eq!(entry.get_size_as_u64(), i as u64 + 1);
        }
        assert_eq!(iter.marker(), Some(""));
        assert_eq!(counter, 4usize);

        Ok(())
    }

    #[test]
    fn test_sync_list_v1_with_error() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                let n = self.counter.fetch_add(1, Ordering::SeqCst);
                let (code, body) = match n {
                    0 => {
                        assert!(request
                            .url()
                            .to_string()
                            .ends_with("/list?bucket=fakebucketname&limit=1000"));
                        (
                            StatusCode::OK,
                            SyncResponseBody::from_bytes(
                                json_to_vec(&json!({
                                    "marker": "fakemarker",
                                    "items": [{
                                        "key": "fakeobj1",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj1hash",
                                        "fsize": 1usize,
                                        "mime_type": "text/plain",
                                    }, {
                                        "key": "fakeobj2",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj2hash",
                                        "fsize": 2usize,
                                        "mime_type": "text/plain",
                                    }]
                                }))
                                .unwrap(),
                            ),
                        )
                    }
                    1 => {
                        assert!(request
                            .url()
                            .to_string()
                            .ends_with("/list?bucket=fakebucketname&marker=fakemarker&limit=1000"));
                        (
                            StatusCode::from_u16(599).unwrap(),
                            SyncResponseBody::from_bytes(
                                json_to_vec(&json!({
                                    "error": "Test Error"
                                }))
                                .unwrap(),
                            ),
                        )
                    }
                    _ => unreachable!(),
                };
                Ok(SyncResponse::builder()
                    .status_code(code)
                    .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                    .body(body)
                    .build())
            }

            #[cfg(feature = "async")]
            fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let before_request_callback_counter = Arc::new(AtomicUsize::new(0));
        let after_response_ok_callback_counter = Arc::new(AtomicUsize::new(0));
        let after_response_error_callback_counter = Arc::new(AtomicUsize::new(0));
        let bucket = get_bucket(FakeHttpCaller::default());
        let mut iter = bucket
            .list()
            .version(ListVersion::V1)
            .before_request_callback({
                let before_request_callback_counter = before_request_callback_counter.to_owned();
                move |_| {
                    before_request_callback_counter.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .after_response_ok_callback({
                let after_response_ok_callback_counter = after_response_ok_callback_counter.to_owned();
                move |_| {
                    after_response_ok_callback_counter.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .after_response_error_callback({
                let after_response_error_callback_counter = after_response_error_callback_counter.to_owned();
                move |_| {
                    after_response_error_callback_counter.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .iter();
        let mut entry = iter.next().unwrap()?;
        assert_eq!(entry.get_key_as_str(), "fakeobj1");
        assert_eq!(entry.get_hash_as_str(), "fakeobj1hash");
        assert_eq!(entry.get_size_as_u64(), 1u64);

        entry = iter.next().unwrap()?;
        assert_eq!(entry.get_key_as_str(), "fakeobj2");
        assert_eq!(entry.get_hash_as_str(), "fakeobj2hash");
        assert_eq!(entry.get_size_as_u64(), 2u64);

        let err = iter.next().unwrap().unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::StatusCodeError(StatusCode::from_u16(599)?)
        );
        assert!(iter.next().is_none());
        assert_eq!(iter.marker(), Some("fakemarker"));
        assert_eq!(before_request_callback_counter.load(Ordering::Relaxed), 2usize);
        assert_eq!(after_response_ok_callback_counter.load(Ordering::Relaxed), 1usize);
        assert_eq!(after_response_error_callback_counter.load(Ordering::Relaxed), 1usize);

        Ok(())
    }

    #[test]
    fn test_sync_list_v1_with_prefix_and_limitation() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                let n = self.counter.fetch_add(1, Ordering::SeqCst);
                let body = match n {
                    0 => {
                        assert!(request
                            .url()
                            .to_string()
                            .ends_with("/list?bucket=fakebucketname&limit=3&prefix=fakeobj"));
                        SyncResponseBody::from_bytes(
                            json_to_vec(&json!({
                                "marker": "fakemarker",
                                "items": [{
                                    "key": "fakeobj1",
                                    "put_time": generate_put_time(),
                                    "hash": "fakeobj1hash",
                                    "fsize": 1usize,
                                    "mime_type": "text/plain",
                                }, {
                                    "key": "fakeobj2",
                                    "put_time": generate_put_time(),
                                    "hash": "fakeobj2hash",
                                    "fsize": 2usize,
                                    "mime_type": "text/plain",
                                }]
                            }))
                            .unwrap(),
                        )
                    }
                    1 => {
                        assert!(request
                            .url()
                            .to_string()
                            .ends_with("/list?bucket=fakebucketname&marker=fakemarker&limit=1&prefix=fakeobj"));
                        SyncResponseBody::from_bytes(
                            json_to_vec(&json!({
                                "marker": "",
                                "items": [{
                                    "key": "fakeobj3",
                                    "put_time": generate_put_time(),
                                    "hash": "fakeobj3hash",
                                    "fsize": 3usize,
                                    "mime_type": "text/plain",
                                }]
                            }))
                            .unwrap(),
                        )
                    }
                    _ => unreachable!(),
                };
                Ok(SyncResponse::builder()
                    .status_code(StatusCode::OK)
                    .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                    .body(body)
                    .build())
            }

            #[cfg(feature = "async")]
            fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let mut counter = 0usize;
        let bucket = get_bucket(FakeHttpCaller::default());
        let mut iter = bucket.list().version(ListVersion::V1).prefix("fakeobj").limit(3).iter();
        for (i, entry) in (&mut iter).enumerate() {
            counter += 1;
            let entry = entry?;
            assert_eq!(entry.get_key_as_str(), &format!("fakeobj{}", i + 1));
            assert_eq!(entry.get_hash_as_str(), &format!("fakeobj{}hash", i + 1));
            assert_eq!(entry.get_size_as_u64(), i as u64 + 1);
        }
        assert_eq!(iter.marker(), Some(""));
        assert_eq!(counter, 3usize);

        Ok(())
    }

    #[test]
    fn test_sync_list_v1_with_cancellation() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                let n = self.counter.fetch_add(1, Ordering::SeqCst);
                let body = match n {
                    0 => {
                        assert!(request
                            .url()
                            .to_string()
                            .ends_with("/list?bucket=fakebucketname&limit=1000"));
                        SyncResponseBody::from_bytes(
                            json_to_vec(&json!({
                                "marker": "fakemarker",
                                "items": [{
                                    "key": "fakeobj1",
                                    "put_time": generate_put_time(),
                                    "hash": "fakeobj1hash",
                                    "fsize": 1usize,
                                    "mime_type": "text/plain",
                                }, {
                                    "key": "fakeobj2",
                                    "put_time": generate_put_time(),
                                    "hash": "fakeobj2hash",
                                    "fsize": 2usize,
                                    "mime_type": "text/plain",
                                }]
                            }))
                            .unwrap(),
                        )
                    }
                    _ => unreachable!(),
                };
                Ok(SyncResponse::builder()
                    .status_code(StatusCode::OK)
                    .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                    .body(body)
                    .build())
            }

            #[cfg(feature = "async")]
            fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let bucket = get_bucket(FakeHttpCaller::default());
        {
            let mut iter = bucket
                .list()
                .version(ListVersion::V1)
                .before_request_callback({
                    let counter = counter.to_owned();
                    move |_| {
                        if counter.load(Ordering::Relaxed) > 0 {
                            Err(AnyError::msg("Fake error"))
                        } else {
                            Ok(())
                        }
                    }
                })
                .iter();
            for (i, entry) in (&mut iter).enumerate() {
                if counter.fetch_add(1, Ordering::Relaxed) < 2 {
                    let entry = entry?;
                    assert_eq!(entry.get_key_as_str(), &format!("fakeobj{}", i + 1));
                    assert_eq!(entry.get_hash_as_str(), &format!("fakeobj{}hash", i + 1));
                    assert_eq!(entry.get_size_as_u64(), i as u64 + 1);
                } else {
                    let err = entry.unwrap_err();
                    assert!(matches!(
                        err.kind(),
                        ResponseErrorKind::HttpError(HttpResponseErrorKind::CallbackError { .. })
                    ));
                    break;
                }
            }
            assert_eq!(iter.marker(), Some("fakemarker"));
        }
        assert_eq!(Arc::try_unwrap(counter).unwrap().into_inner(), 3usize);

        Ok(())
    }

    #[test]
    fn test_sync_list_v2() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                let n = self.counter.fetch_add(1, Ordering::SeqCst);
                let body = match n {
                    0 => {
                        assert!(request.url().to_string().ends_with("/v2/list?bucket=fakebucketname"));
                        SyncResponseBody::from_bytes(
                            [
                                json_to_string(&json!({
                                    "item": {
                                        "key": "fakeobj1",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj1hash",
                                        "fsize": 1usize,
                                        "mime_type": "text/plain",
                                    },
                                    "marker": "fakemarkerobj1",
                                }))
                                .unwrap(),
                                json_to_string(&json!({
                                    "item": {
                                        "key": "fakeobj2",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj2hash",
                                        "fsize": 2usize,
                                        "mime_type": "text/plain",
                                    },
                                    "marker": "fakemarkerobj2",
                                }))
                                .unwrap(),
                            ]
                            .join("\n")
                            .as_bytes()
                            .to_owned(),
                        )
                    }
                    1 => {
                        assert!(request
                            .url()
                            .to_string()
                            .ends_with("/list?bucket=fakebucketname&marker=fakemarkerobj2"));
                        SyncResponseBody::from_bytes(
                            [
                                json_to_string(&json!({
                                    "item": {
                                        "key": "fakeobj3",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj3hash",
                                        "fsize": 3usize,
                                        "mime_type": "text/plain",
                                    },
                                    "marker": "fakemarkerobj3",
                                }))
                                .unwrap(),
                                json_to_string(&json!({
                                    "item": {
                                        "key": "fakeobj4",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj4hash",
                                        "fsize": 4usize,
                                        "mime_type": "text/plain",
                                    },
                                    "marker": "",
                                }))
                                .unwrap(),
                            ]
                            .join("\n")
                            .as_bytes()
                            .to_owned(),
                        )
                    }
                    _ => unreachable!(),
                };
                Ok(SyncResponse::builder()
                    .status_code(StatusCode::OK)
                    .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                    .body(body)
                    .build())
            }

            #[cfg(feature = "async")]
            fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let mut counter = 0usize;
        let bucket = get_bucket(FakeHttpCaller::default());
        let mut iter = bucket.list().version(ListVersion::V2).iter();
        for (i, entry) in (&mut iter).enumerate() {
            counter += 1;
            let entry = entry?;
            assert_eq!(entry.get_key_as_str(), &format!("fakeobj{}", i + 1));
            assert_eq!(entry.get_hash_as_str(), &format!("fakeobj{}hash", i + 1));
            assert_eq!(entry.get_size_as_u64(), i as u64 + 1);
        }
        assert_eq!(counter, 4usize);
        assert_eq!(iter.marker(), Some(""));

        Ok(())
    }

    #[test]
    fn test_sync_list_v2_with_non_results() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                let n = self.counter.fetch_add(1, Ordering::SeqCst);
                let body = match n {
                    0 => {
                        assert!(request
                            .url()
                            .to_string()
                            .ends_with("/v2/list?bucket=fakebucketname&prefix=non-existed"));
                        SyncResponseBody::from_bytes(Vec::new())
                    }
                    _ => unreachable!(),
                };
                Ok(SyncResponse::builder()
                    .status_code(StatusCode::OK)
                    .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                    .body(body)
                    .build())
            }

            #[cfg(feature = "async")]
            fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let mut counter = 0usize;
        let bucket = get_bucket(FakeHttpCaller::default());
        let mut iter = bucket.list().version(ListVersion::V2).prefix("non-existed").iter();
        for _entry in &mut iter {
            counter += 1;
        }
        assert_eq!(counter, 0usize);
        assert_eq!(iter.marker(), None);

        Ok(())
    }

    #[test]
    fn test_sync_list_v2_with_error() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                let n = self.counter.fetch_add(1, Ordering::SeqCst);
                let (code, body) = match n {
                    0 => {
                        assert!(request.url().to_string().ends_with("/v2/list?bucket=fakebucketname"));
                        (
                            StatusCode::OK,
                            SyncResponseBody::from_bytes(
                                [
                                    json_to_string(&json!({
                                        "item": {
                                            "key": "fakeobj1",
                                            "put_time": generate_put_time(),
                                            "hash": "fakeobj1hash",
                                            "fsize": 1usize,
                                            "mime_type": "text/plain",
                                        },
                                        "marker": "fakemarkerobj1",
                                    }))
                                    .unwrap(),
                                    json_to_string(&json!({
                                        "item": {
                                            "key": "fakeobj2",
                                            "put_time": generate_put_time(),
                                            "hash": "fakeobj2hash",
                                            "fsize": 2usize,
                                            "mime_type": "text/plain",
                                        },
                                        "marker": "fakemarkerobj2",
                                    }))
                                    .unwrap(),
                                ]
                                .join("\n")
                                .as_bytes()
                                .to_owned(),
                            ),
                        )
                    }
                    1 => {
                        assert!(request
                            .url()
                            .to_string()
                            .ends_with("/v2/list?bucket=fakebucketname&marker=fakemarkerobj2"));
                        (
                            StatusCode::from_u16(599).unwrap(),
                            SyncResponseBody::from_bytes(
                                json_to_vec(&json!({
                                    "error": "Test Error"
                                }))
                                .unwrap(),
                            ),
                        )
                    }
                    _ => unreachable!(),
                };
                Ok(SyncResponse::builder()
                    .status_code(code)
                    .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                    .body(body)
                    .build())
            }

            #[cfg(feature = "async")]
            fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let before_request_callback_counter = Arc::new(AtomicUsize::new(0));
        let after_response_ok_callback_counter = Arc::new(AtomicUsize::new(0));
        let after_response_error_callback_counter = Arc::new(AtomicUsize::new(0));
        let bucket = get_bucket(FakeHttpCaller::default());
        let mut iter = bucket
            .list()
            .version(ListVersion::V2)
            .before_request_callback({
                let before_request_callback_counter = before_request_callback_counter.to_owned();
                move |_| {
                    before_request_callback_counter.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .after_response_ok_callback({
                let after_response_ok_callback_counter = after_response_ok_callback_counter.to_owned();
                move |_| {
                    after_response_ok_callback_counter.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .after_response_error_callback({
                let after_response_error_callback_counter = after_response_error_callback_counter.to_owned();
                move |_| {
                    after_response_error_callback_counter.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .iter();
        let mut entry = iter.next().unwrap()?;
        assert_eq!(entry.get_key_as_str(), "fakeobj1");
        assert_eq!(entry.get_hash_as_str(), "fakeobj1hash");
        assert_eq!(entry.get_size_as_u64(), 1u64);

        entry = iter.next().unwrap()?;
        assert_eq!(entry.get_key_as_str(), "fakeobj2");
        assert_eq!(entry.get_hash_as_str(), "fakeobj2hash");
        assert_eq!(entry.get_size_as_u64(), 2u64);

        let err = iter.next().unwrap().unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::StatusCodeError(StatusCode::from_u16(599)?)
        );
        assert!(iter.next().is_none());
        assert_eq!(iter.marker(), Some("fakemarkerobj2"));
        assert_eq!(before_request_callback_counter.load(Ordering::Relaxed), 2usize);
        assert_eq!(after_response_ok_callback_counter.load(Ordering::Relaxed), 1usize);
        assert_eq!(after_response_error_callback_counter.load(Ordering::Relaxed), 1usize);

        Ok(())
    }

    #[test]
    fn test_sync_list_v2_with_cancellation() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                let n = self.counter.fetch_add(1, Ordering::SeqCst);
                let body = match n {
                    0 => {
                        assert!(request.url().to_string().ends_with("/v2/list?bucket=fakebucketname"));
                        SyncResponseBody::from_bytes(
                            [
                                json_to_string(&json!({
                                    "item": {
                                        "key": "fakeobj1",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj1hash",
                                        "fsize": 1usize,
                                        "mime_type": "text/plain",
                                    },
                                    "marker": "fakemarkerobj1",
                                }))
                                .unwrap(),
                                json_to_string(&json!({
                                    "item": {
                                        "key": "fakeobj2",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj2hash",
                                        "fsize": 2usize,
                                        "mime_type": "text/plain",
                                    },
                                    "marker": "fakemarkerobj2",
                                }))
                                .unwrap(),
                            ]
                            .join("\n")
                            .as_bytes()
                            .to_owned(),
                        )
                    }
                    _ => unreachable!(),
                };
                Ok(SyncResponse::builder()
                    .status_code(StatusCode::OK)
                    .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                    .body(body)
                    .build())
            }

            #[cfg(feature = "async")]
            fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let bucket = get_bucket(FakeHttpCaller::default());
        {
            let mut iter = bucket
                .list()
                .version(ListVersion::V2)
                .before_request_callback({
                    let counter = counter.to_owned();
                    move |_| {
                        if counter.load(Ordering::Relaxed) > 0 {
                            Err(AnyError::msg("Fake error"))
                        } else {
                            Ok(())
                        }
                    }
                })
                .iter();
            for (i, entry) in (&mut iter).enumerate() {
                if counter.fetch_add(1, Ordering::Relaxed) < 2 {
                    let entry = entry?;
                    assert_eq!(entry.get_key_as_str(), &format!("fakeobj{}", i + 1));
                    assert_eq!(entry.get_hash_as_str(), &format!("fakeobj{}hash", i + 1));
                    assert_eq!(entry.get_size_as_u64(), i as u64 + 1);
                } else {
                    let err = entry.unwrap_err();
                    assert!(matches!(
                        err.kind(),
                        ResponseErrorKind::HttpError(HttpResponseErrorKind::CallbackError { .. })
                    ));
                    break;
                }
            }
            assert_eq!(iter.marker(), Some("fakemarkerobj2"));
        }
        assert_eq!(Arc::try_unwrap(counter).unwrap().into_inner(), 3usize);

        Ok(())
    }

    #[async_std::test]
    #[cfg(feature = "async")]
    async fn test_async_list_v1() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    let n = self.counter.fetch_add(1, Ordering::SeqCst);
                    let body = match n {
                        0 => {
                            assert!(request
                                .url()
                                .to_string()
                                .ends_with("/list?bucket=fakebucketname&limit=1000"));
                            AsyncResponseBody::from_bytes(
                                json_to_vec(&json!({
                                    "marker": "fakemarker",
                                    "items": [{
                                        "key": "fakeobj1",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj1hash",
                                        "fsize": 1usize,
                                        "mime_type": "text/plain",
                                    }, {
                                        "key": "fakeobj2",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj2hash",
                                        "fsize": 2usize,
                                        "mime_type": "text/plain",
                                    }]
                                }))
                                .unwrap(),
                            )
                        }
                        1 => {
                            assert!(request
                                .url()
                                .to_string()
                                .ends_with("/list?bucket=fakebucketname&marker=fakemarker&limit=1000"));
                            AsyncResponseBody::from_bytes(
                                json_to_vec(&json!({
                                    "marker": "",
                                    "items": [{
                                        "key": "fakeobj3",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj3hash",
                                        "fsize": 3usize,
                                        "mime_type": "text/plain",
                                    }, {
                                        "key": "fakeobj4",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj4hash",
                                        "fsize": 4usize,
                                        "mime_type": "text/plain",
                                    }]
                                }))
                                .unwrap(),
                            )
                        }
                        _ => unreachable!(),
                    };
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(body)
                        .build())
                })
            }
        }

        let mut counter = 0usize;
        let bucket = get_bucket(FakeHttpCaller::default());
        let mut stream = bucket.list().version(ListVersion::V1).stream();
        let mut iter = (&mut stream).enumerate();
        while let Some((i, entry)) = iter.next().await {
            counter += 1;
            let entry = entry?;
            assert_eq!(entry.get_key_as_str(), &format!("fakeobj{}", i + 1));
            assert_eq!(entry.get_hash_as_str(), &format!("fakeobj{}hash", i + 1));
            assert_eq!(entry.get_size_as_u64(), i as u64 + 1);
        }
        assert_eq!(stream.marker(), Some(""));
        assert_eq!(counter, 4usize);

        Ok(())
    }

    #[async_std::test]
    #[cfg(feature = "async")]
    async fn test_async_list_v1_with_error() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    let n = self.counter.fetch_add(1, Ordering::SeqCst);
                    let (code, body) = match n {
                        0 => {
                            assert!(request
                                .url()
                                .to_string()
                                .ends_with("/list?bucket=fakebucketname&limit=1000"));
                            (
                                StatusCode::OK,
                                AsyncResponseBody::from_bytes(
                                    json_to_vec(&json!({
                                        "marker": "fakemarker",
                                        "items": [{
                                            "key": "fakeobj1",
                                            "put_time": generate_put_time(),
                                            "hash": "fakeobj1hash",
                                            "fsize": 1usize,
                                            "mime_type": "text/plain",
                                        }, {
                                            "key": "fakeobj2",
                                            "put_time": generate_put_time(),
                                            "hash": "fakeobj2hash",
                                            "fsize": 2usize,
                                            "mime_type": "text/plain",
                                        }]
                                    }))
                                    .unwrap(),
                                ),
                            )
                        }
                        1 => {
                            assert!(request
                                .url()
                                .to_string()
                                .ends_with("/list?bucket=fakebucketname&marker=fakemarker&limit=1000"));
                            (
                                StatusCode::from_u16(599).unwrap(),
                                AsyncResponseBody::from_bytes(
                                    json_to_vec(&json!({
                                        "error": "Test Error"
                                    }))
                                    .unwrap(),
                                ),
                            )
                        }
                        _ => unreachable!(),
                    };
                    Ok(AsyncResponse::builder()
                        .status_code(code)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(body)
                        .build())
                })
            }
        }

        let before_request_callback_counter = Arc::new(AtomicUsize::new(0));
        let after_response_ok_callback_counter = Arc::new(AtomicUsize::new(0));
        let after_response_error_callback_counter = Arc::new(AtomicUsize::new(0));
        let bucket = get_bucket(FakeHttpCaller::default());
        let mut iter = bucket
            .list()
            .version(ListVersion::V1)
            .before_request_callback({
                let before_request_callback_counter = before_request_callback_counter.to_owned();
                move |_| {
                    before_request_callback_counter.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .after_response_ok_callback({
                let after_response_ok_callback_counter = after_response_ok_callback_counter.to_owned();
                move |_| {
                    after_response_ok_callback_counter.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .after_response_error_callback({
                let after_response_error_callback_counter = after_response_error_callback_counter.to_owned();
                move |_| {
                    after_response_error_callback_counter.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .stream();
        let mut entry = iter.try_next().await?.unwrap();
        assert_eq!(entry.get_key_as_str(), "fakeobj1");
        assert_eq!(entry.get_hash_as_str(), "fakeobj1hash");
        assert_eq!(entry.get_size_as_u64(), 1u64);

        entry = iter.try_next().await?.unwrap();
        assert_eq!(entry.get_key_as_str(), "fakeobj2");
        assert_eq!(entry.get_hash_as_str(), "fakeobj2hash");
        assert_eq!(entry.get_size_as_u64(), 2u64);

        let err = iter.try_next().await.unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::StatusCodeError(StatusCode::from_u16(599)?)
        );
        assert!(iter.try_next().await?.is_none());
        assert_eq!(iter.marker(), Some("fakemarker"));
        assert_eq!(before_request_callback_counter.load(Ordering::Relaxed), 2usize);
        assert_eq!(after_response_ok_callback_counter.load(Ordering::Relaxed), 1usize);
        assert_eq!(after_response_error_callback_counter.load(Ordering::Relaxed), 1usize);

        Ok(())
    }

    #[async_std::test]
    #[cfg(feature = "async")]
    async fn test_async_list_v1_with_prefix_and_limitation() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    let n = self.counter.fetch_add(1, Ordering::SeqCst);
                    let body = match n {
                        0 => {
                            assert!(request
                                .url()
                                .to_string()
                                .ends_with("/list?bucket=fakebucketname&limit=3&prefix=fakeobj"));
                            AsyncResponseBody::from_bytes(
                                json_to_vec(&json!({
                                    "marker": "fakemarker",
                                    "items": [{
                                        "key": "fakeobj1",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj1hash",
                                        "fsize": 1usize,
                                        "mime_type": "text/plain",
                                    }, {
                                        "key": "fakeobj2",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj2hash",
                                        "fsize": 2usize,
                                        "mime_type": "text/plain",
                                    }]
                                }))
                                .unwrap(),
                            )
                        }
                        1 => {
                            assert!(request
                                .url()
                                .to_string()
                                .ends_with("/list?bucket=fakebucketname&marker=fakemarker&limit=1&prefix=fakeobj"));
                            AsyncResponseBody::from_bytes(
                                json_to_vec(&json!({
                                    "marker": "",
                                    "items": [{
                                        "key": "fakeobj3",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj3hash",
                                        "fsize": 3usize,
                                        "mime_type": "text/plain",
                                    }]
                                }))
                                .unwrap(),
                            )
                        }
                        _ => unreachable!(),
                    };
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(body)
                        .build())
                })
            }
        }

        let mut counter = 0usize;
        let bucket = get_bucket(FakeHttpCaller::default());
        let mut stream = bucket
            .list()
            .version(ListVersion::V1)
            .prefix("fakeobj")
            .limit(3)
            .stream();
        let mut iter = (&mut stream).enumerate();
        while let Some((i, entry)) = iter.next().await {
            counter += 1;
            let entry = entry?;
            assert_eq!(entry.get_key_as_str(), &format!("fakeobj{}", i + 1));
            assert_eq!(entry.get_hash_as_str(), &format!("fakeobj{}hash", i + 1));
            assert_eq!(entry.get_size_as_u64(), i as u64 + 1);
        }
        assert_eq!(stream.marker(), Some(""));
        assert_eq!(counter, 3usize);

        Ok(())
    }

    #[async_std::test]
    #[cfg(feature = "async")]
    async fn test_async_list_v1_with_cancellation() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    let n = self.counter.fetch_add(1, Ordering::SeqCst);
                    let body = match n {
                        0 => {
                            assert!(request
                                .url()
                                .to_string()
                                .ends_with("/list?bucket=fakebucketname&limit=1000"));
                            AsyncResponseBody::from_bytes(
                                json_to_vec(&json!({
                                    "marker": "fakemarker",
                                    "items": [{
                                        "key": "fakeobj1",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj1hash",
                                        "fsize": 1usize,
                                        "mime_type": "text/plain",
                                    }, {
                                        "key": "fakeobj2",
                                        "put_time": generate_put_time(),
                                        "hash": "fakeobj2hash",
                                        "fsize": 2usize,
                                        "mime_type": "text/plain",
                                    }]
                                }))
                                .unwrap(),
                            )
                        }
                        _ => unreachable!(),
                    };
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(body)
                        .build())
                })
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        {
            let bucket = get_bucket(FakeHttpCaller::default());
            let mut stream = bucket
                .list()
                .version(ListVersion::V1)
                .before_request_callback({
                    let counter = counter.to_owned();
                    move |_| {
                        if counter.load(Ordering::Relaxed) > 0 {
                            Err(AnyError::msg("Fake error"))
                        } else {
                            Ok(())
                        }
                    }
                })
                .stream();
            let mut iter = (&mut stream).enumerate();
            while let Some((i, entry)) = iter.next().await {
                if counter.fetch_add(1, Ordering::Relaxed) < 2 {
                    let entry = entry?;
                    assert_eq!(entry.get_key_as_str(), &format!("fakeobj{}", i + 1));
                    assert_eq!(entry.get_hash_as_str(), &format!("fakeobj{}hash", i + 1));
                    assert_eq!(entry.get_size_as_u64(), i as u64 + 1);
                } else {
                    let err = entry.unwrap_err();
                    assert!(matches!(
                        err.kind(),
                        ResponseErrorKind::HttpError(HttpResponseErrorKind::CallbackError { .. })
                    ));
                    break;
                }
            }
            assert_eq!(stream.marker(), Some("fakemarker"));
        }
        assert_eq!(Arc::try_unwrap(counter).unwrap().into_inner(), 3usize);

        Ok(())
    }

    #[async_std::test]
    #[cfg(feature = "async")]
    async fn test_async_list_v2() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    let n = self.counter.fetch_add(1, Ordering::SeqCst);
                    let body = match n {
                        0 => {
                            assert!(request.url().to_string().ends_with("/v2/list?bucket=fakebucketname"));
                            AsyncResponseBody::from_bytes(
                                [
                                    json_to_string(&json!({
                                        "item": {
                                            "key": "fakeobj1",
                                            "put_time": generate_put_time(),
                                            "hash": "fakeobj1hash",
                                            "fsize": 1usize,
                                            "mime_type": "text/plain",
                                        },
                                        "marker": "fakemarkerobj1",
                                    }))
                                    .unwrap(),
                                    json_to_string(&json!({
                                        "item": {
                                            "key": "fakeobj2",
                                            "put_time": generate_put_time(),
                                            "hash": "fakeobj2hash",
                                            "fsize": 2usize,
                                            "mime_type": "text/plain",
                                        },
                                        "marker": "fakemarkerobj2",
                                    }))
                                    .unwrap(),
                                ]
                                .join("\n")
                                .as_bytes()
                                .to_owned(),
                            )
                        }
                        1 => {
                            assert!(request
                                .url()
                                .to_string()
                                .ends_with("/list?bucket=fakebucketname&marker=fakemarkerobj2"));
                            AsyncResponseBody::from_bytes(
                                [
                                    json_to_string(&json!({
                                        "item": {
                                            "key": "fakeobj3",
                                            "put_time": generate_put_time(),
                                            "hash": "fakeobj3hash",
                                            "fsize": 3usize,
                                            "mime_type": "text/plain",
                                        },
                                        "marker": "fakemarkerobj3",
                                    }))
                                    .unwrap(),
                                    json_to_string(&json!({
                                        "item": {
                                            "key": "fakeobj4",
                                            "put_time": generate_put_time(),
                                            "hash": "fakeobj4hash",
                                            "fsize": 4usize,
                                            "mime_type": "text/plain",
                                        },
                                        "marker": "",
                                    }))
                                    .unwrap(),
                                ]
                                .join("\n")
                                .as_bytes()
                                .to_owned(),
                            )
                        }
                        _ => unreachable!(),
                    };
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(body)
                        .build())
                })
            }
        }

        let mut counter = 0usize;
        let bucket = get_bucket(FakeHttpCaller::default());
        let mut stream = bucket.list().version(ListVersion::V2).stream();
        let mut iter = (&mut stream).enumerate();
        while let Some((i, entry)) = iter.next().await {
            counter += 1;
            let entry = entry?;
            assert_eq!(entry.get_key_as_str(), &format!("fakeobj{}", i + 1));
            assert_eq!(entry.get_hash_as_str(), &format!("fakeobj{}hash", i + 1));
            assert_eq!(entry.get_size_as_u64(), i as u64 + 1);
        }
        assert_eq!(stream.marker(), Some(""));
        assert_eq!(counter, 4usize);

        Ok(())
    }

    #[async_std::test]
    #[cfg(feature = "async")]
    async fn test_async_list_v2_with_non_results() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    let n = self.counter.fetch_add(1, Ordering::SeqCst);
                    let body = match n {
                        0 => {
                            assert!(request
                                .url()
                                .to_string()
                                .ends_with("/v2/list?bucket=fakebucketname&prefix=non-exist"));
                            AsyncResponseBody::from_bytes(Vec::new())
                        }
                        _ => unreachable!(),
                    };
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(body)
                        .build())
                })
            }
        }

        let mut counter = 0usize;
        let bucket = get_bucket(FakeHttpCaller::default());
        let mut stream = bucket.list().version(ListVersion::V2).prefix("non-exist").stream();
        let mut iter = (&mut stream).enumerate();
        while let Some((_i, _entry)) = iter.next().await {
            counter += 1;
        }
        assert_eq!(stream.marker(), None);
        assert_eq!(counter, 0usize);

        Ok(())
    }

    #[async_std::test]
    #[cfg(feature = "async")]
    async fn test_async_list_v2_with_error() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    let n = self.counter.fetch_add(1, Ordering::SeqCst);
                    let (code, body) = match n {
                        0 => {
                            assert!(request.url().to_string().ends_with("/v2/list?bucket=fakebucketname"));
                            (
                                StatusCode::OK,
                                AsyncResponseBody::from_bytes(
                                    [
                                        json_to_string(&json!({
                                            "item": {
                                                "key": "fakeobj1",
                                                "put_time": generate_put_time(),
                                                "hash": "fakeobj1hash",
                                                "fsize": 1usize,
                                                "mime_type": "text/plain",
                                            },
                                            "marker": "fakemarkerobj1",
                                        }))
                                        .unwrap(),
                                        json_to_string(&json!({
                                            "item": {
                                                "key": "fakeobj2",
                                                "put_time": generate_put_time(),
                                                "hash": "fakeobj2hash",
                                                "fsize": 2usize,
                                                "mime_type": "text/plain",
                                            },
                                            "marker": "fakemarkerobj2",
                                        }))
                                        .unwrap(),
                                    ]
                                    .join("\n")
                                    .as_bytes()
                                    .to_owned(),
                                ),
                            )
                        }
                        1 => {
                            assert!(request
                                .url()
                                .to_string()
                                .ends_with("/v2/list?bucket=fakebucketname&marker=fakemarkerobj2"));
                            (
                                StatusCode::from_u16(599).unwrap(),
                                AsyncResponseBody::from_bytes(
                                    json_to_vec(&json!({
                                        "error": "Test Error"
                                    }))
                                    .unwrap(),
                                ),
                            )
                        }
                        _ => unreachable!(),
                    };
                    Ok(AsyncResponse::builder()
                        .status_code(code)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(body)
                        .build())
                })
            }
        }

        let before_request_callback_counter = Arc::new(AtomicUsize::new(0));
        let after_response_ok_callback_counter = Arc::new(AtomicUsize::new(0));
        let after_response_error_callback_counter = Arc::new(AtomicUsize::new(0));
        let bucket = get_bucket(FakeHttpCaller::default());
        let mut stream = bucket
            .list()
            .version(ListVersion::V2)
            .before_request_callback({
                let before_request_callback_counter = before_request_callback_counter.to_owned();
                move |_| {
                    before_request_callback_counter.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .after_response_ok_callback({
                let after_response_ok_callback_counter = after_response_ok_callback_counter.to_owned();
                move |_| {
                    after_response_ok_callback_counter.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .after_response_error_callback({
                let after_response_error_callback_counter = after_response_error_callback_counter.to_owned();
                move |_| {
                    after_response_error_callback_counter.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .stream();
        let mut entry = stream.try_next().await?.unwrap();
        assert_eq!(entry.get_key_as_str(), "fakeobj1");
        assert_eq!(entry.get_hash_as_str(), "fakeobj1hash");
        assert_eq!(entry.get_size_as_u64(), 1u64);

        entry = stream.try_next().await?.unwrap();
        assert_eq!(entry.get_key_as_str(), "fakeobj2");
        assert_eq!(entry.get_hash_as_str(), "fakeobj2hash");
        assert_eq!(entry.get_size_as_u64(), 2u64);

        let err = stream.try_next().await.unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::StatusCodeError(StatusCode::from_u16(599)?)
        );
        assert!(stream.try_next().await?.is_none());
        assert_eq!(stream.marker(), Some("fakemarkerobj2"));
        assert_eq!(before_request_callback_counter.load(Ordering::Relaxed), 2usize);
        assert_eq!(after_response_ok_callback_counter.load(Ordering::Relaxed), 1usize);
        assert_eq!(after_response_error_callback_counter.load(Ordering::Relaxed), 1usize);

        Ok(())
    }

    #[async_std::test]
    #[cfg(feature = "async")]
    async fn test_async_list_v2_with_cancellation() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    let n = self.counter.fetch_add(1, Ordering::SeqCst);
                    let body = match n {
                        0 => {
                            assert!(request.url().to_string().ends_with("/v2/list?bucket=fakebucketname"));
                            AsyncResponseBody::from_bytes(
                                [
                                    json_to_string(&json!({
                                        "item": {
                                            "key": "fakeobj1",
                                            "put_time": generate_put_time(),
                                            "hash": "fakeobj1hash",
                                            "fsize": 1usize,
                                            "mime_type": "text/plain",
                                        },
                                        "marker": "fakemarkerobj1",
                                    }))
                                    .unwrap(),
                                    json_to_string(&json!({
                                        "item": {
                                            "key": "fakeobj2",
                                            "put_time": generate_put_time(),
                                            "hash": "fakeobj2hash",
                                            "fsize": 2usize,
                                            "mime_type": "text/plain",
                                        },
                                        "marker": "fakemarkerobj2",
                                    }))
                                    .unwrap(),
                                ]
                                .join("\n")
                                .as_bytes()
                                .to_owned(),
                            )
                        }
                        _ => unreachable!(),
                    };
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(body)
                        .build())
                })
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        {
            let bucket = get_bucket(FakeHttpCaller::default());
            let mut stream = bucket
                .list()
                .version(ListVersion::V2)
                .before_request_callback({
                    let counter = counter.to_owned();
                    move |_| {
                        if counter.load(Ordering::Relaxed) > 0 {
                            Err(AnyError::msg("Fake error"))
                        } else {
                            Ok(())
                        }
                    }
                })
                .stream();
            let mut iter = (&mut stream).enumerate();
            while let Some((i, entry)) = iter.next().await {
                if counter.fetch_add(1, Ordering::Relaxed) < 2 {
                    let entry = entry?;
                    assert_eq!(entry.get_key_as_str(), &format!("fakeobj{}", i + 1));
                    assert_eq!(entry.get_hash_as_str(), &format!("fakeobj{}hash", i + 1));
                    assert_eq!(entry.get_size_as_u64(), i as u64 + 1);
                } else {
                    let err = entry.unwrap_err();
                    assert!(matches!(
                        err.kind(),
                        ResponseErrorKind::HttpError(HttpResponseErrorKind::CallbackError { .. })
                    ));
                    break;
                }
            }
            assert_eq!(stream.marker(), Some("fakemarkerobj2"));
        }
        assert_eq!(Arc::try_unwrap(counter).unwrap().into_inner(), 3usize);

        Ok(())
    }

    fn get_bucket(caller: impl HttpCaller + 'static) -> Bucket {
        let object_manager = ObjectsManager::builder(get_credential())
            .http_client(
                HttpClient::builder(caller)
                    .chooser(DirectChooser)
                    .request_retrier(NeverRetrier)
                    .backoff(NO_BACKOFF)
                    .build(),
            )
            .build();
        object_manager.bucket_with_region(get_bucket_name(), single_rsf_domain_region())
    }

    fn get_credential() -> Credential {
        Credential::new("fakeaccesskey", "fakesecretkey")
    }

    fn get_bucket_name() -> BucketName {
        "fakebucketname".into()
    }

    fn generate_put_time() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64 / 100
    }

    fn single_rsf_domain_region() -> Region {
        Region::builder("chaotic")
            .add_rsf_preferred_endpoint(("fakersf.example.com".to_owned(), 8080).into())
            .build()
    }
}
