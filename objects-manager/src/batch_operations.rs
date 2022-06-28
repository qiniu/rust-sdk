use super::{callbacks::Callbacks, list::make_callback_error, Bucket, OperationProvider};
use anyhow::{Error as AnyError, Result as AnyResult};
use assert_impl::assert_impl;
use auto_impl::auto_impl;
use dyn_clonable::clonable;
use qiniu_apis::{
    http::{ResponseErrorKind as HttpResponseErrorKind, ResponseParts, StatusCode},
    http_client::{
        ApiResult, RegionsProvider, RegionsProviderEndpoints, RequestBuilderParts, Response, ResponseError,
        ResponseErrorKind,
    },
    storage::batch_ops::{
        OperationResponse, OperationResponseData, RequestBody, ResponseBody,
        SyncRequestBuilder as BatchOpsSyncRequestBuilder,
    },
};
use std::{
    collections::VecDeque,
    error::Error as StdError,
    fmt::{self, Debug, Display},
    mem::take,
};

/// 最大批量操作数获取接口
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait BatchSizeProvider: Clone + Debug + Send + Sync {
    /// 获取最大批量操作数
    fn batch_size(&self) -> usize;
}

impl BatchSizeProvider for usize {
    #[inline]
    fn batch_size(&self) -> usize {
        *self
    }
}

/// 批量操作
pub struct BatchOperations<'a> {
    bucket: &'a Bucket,
    operations: Option<Box<dyn Iterator<Item = Box<dyn OperationProvider + 'a>> + Send + Sync + 'a>>,
    batch_size: Option<Box<dyn BatchSizeProvider + 'a>>,
    callbacks: Callbacks<'a>,
}

impl<'a> BatchOperations<'a> {
    pub(super) fn new(bucket: &'a Bucket) -> Self {
        Self {
            bucket,
            operations: Default::default(),
            batch_size: Default::default(),
            callbacks: Default::default(),
        }
    }

    /// 设置最大批量操作数提供者
    #[inline]
    pub fn batch_size(&mut self, batch_size: impl BatchSizeProvider + 'a) -> &mut Self {
        self.batch_size = Some(Box::new(batch_size));
        self
    }

    /// 添加对象操作提供者
    #[inline]
    pub fn add_operation(&mut self, operation: impl OperationProvider + 'a) -> &mut Self {
        let new_iter = vec![Box::new(operation) as Box<dyn OperationProvider + 'a>].into_iter();
        self.add_operations(new_iter)
    }

    /// 批量添加操作提供者
    #[inline]
    pub fn add_operations<I: IntoIterator<Item = Box<dyn OperationProvider + 'a>> + Send + Sync + 'a>(
        &mut self,
        new_iter: I,
    ) -> &mut Self
    where
        <I as IntoIterator>::IntoIter: Sync + Send,
    {
        if let Some(iter) = take(&mut self.operations) {
            self.operations = Some(Box::new(iter.chain(new_iter.into_iter())));
        } else {
            self.operations = Some(Box::new(new_iter.into_iter()));
        }
        self
    }

    /// 设置请求前回调函数
    #[inline]
    pub fn before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.callbacks.insert_before_request_callback(callback);
        self
    }

    /// 设置响应成功回调函数
    #[inline]
    pub fn after_response_ok_callback(
        &mut self,
        callback: impl FnMut(&mut ResponseParts) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.callbacks.insert_after_response_ok_callback(callback);
        self
    }

    /// 设置响应失败回调函数
    #[inline]
    pub fn after_response_error_callback(
        &mut self,
        callback: impl FnMut(&ResponseError) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.callbacks.insert_after_response_error_callback(callback);
        self
    }

    /// 阻塞发起批量操作，返回操作结果迭代器
    ///
    /// 该方法的的异步版本为 [`Self::async_call`]。
    #[inline]
    pub fn call(&mut self) -> BatchOperationsIterator<'a> {
        BatchOperationsIterator {
            operations: self.take_self(),
            buffer: Default::default(),
            closed: false,
        }
    }

    /// 异步发起批量操作，返回操作结果流
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub fn async_call(&mut self) -> BatchOperationsStream<'a> {
        BatchOperationsStream::new(self.take_self())
    }

    fn take_self(&mut self) -> Self {
        Self {
            bucket: self.bucket,
            operations: take(&mut self.operations),
            batch_size: take(&mut self.batch_size),
            callbacks: take(&mut self.callbacks),
        }
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl Debug for BatchOperations<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BatchOperations")
            .field("bucket", &self.bucket)
            .field("batch_size", &self.batch_size)
            .finish()
    }
}

/// 批量操作迭代器
///
/// 实现 [`std::iter::Iterator`] 接口，
/// 在迭代过程中阻塞发起批量操作 API
#[derive(Debug)]
pub struct BatchOperationsIterator<'a> {
    operations: BatchOperations<'a>,
    buffer: VecDeque<ApiResult<OperationResponseData>>,
    closed: bool,
}

impl Iterator for BatchOperationsIterator<'_> {
    type Item = ApiResult<OperationResponseData>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(first) = self.buffer.pop_front() {
            Some(first)
        } else if self.closed {
            None
        } else {
            self.next_response().map(|v| v.map(Ok)).unwrap_or_else(|e| Some(Err(e)))
        }
    }
}

const DEFAULT_BATCH_SIZE: usize = 1000;
type RefRegionProviderEndpoints<'a> = RegionsProviderEndpoints<&'a dyn RegionsProvider>;

impl<'a> BatchOperationsIterator<'a> {
    fn next_response(&mut self) -> ApiResult<Option<OperationResponseData>> {
        if let Some(request_body) = self.generate_request_body() {
            let request = self.make_request()?;
            let response = self.call_request(request, request_body)?;
            self.handle_response(response.into_body()).transpose()
        } else {
            Ok(None)
        }
    }

    fn make_request(&self) -> ApiResult<BatchOpsSyncRequestBuilder<'a, RefRegionProviderEndpoints<'a>>> {
        let request = self
            .operations
            .bucket
            .objects_manager()
            .client()
            .storage()
            .batch_ops()
            .new_request(
                RegionsProviderEndpoints::new(self.operations.bucket.region_provider()?),
                self.operations.bucket.objects_manager().credential(),
            );
        Ok(request)
    }

    fn call_request(
        &mut self,
        mut request: BatchOpsSyncRequestBuilder<'_, RefRegionProviderEndpoints>,
        request_body: RequestBody,
    ) -> ApiResult<Response<ResponseBody>> {
        self.operations
            .callbacks
            .before_request(request.parts_mut())
            .map_err(make_callback_error)?;
        let mut response_result = request.call(request_body);
        self.operations
            .callbacks
            .after_response(&mut response_result)
            .map_err(make_callback_error)?;
        response_result
    }

    fn handle_response(&mut self, response_body: ResponseBody) -> Option<ApiResult<OperationResponseData>> {
        let responses = response_body.to_operation_response_vec();
        self.buffer
            .extend(responses.into_iter().map(from_response_to_response_data_result));
        self.buffer.pop_front()
    }

    fn generate_request_body(&mut self) -> Option<RequestBody> {
        let mut request_body = RequestBody::default();
        let mut operation_count = 0usize;
        for _ in 0..self
            .operations
            .batch_size
            .as_ref()
            .map(|provider| provider.batch_size())
            .unwrap_or(DEFAULT_BATCH_SIZE)
        {
            if let Some(mut operation) = self.operations.operations.as_mut().and_then(|op| op.next()) {
                request_body = request_body.append_operations_as_str(operation.to_operation());
                operation_count += 1;
            } else {
                self.closed = true;
                break;
            }
        }
        if operation_count > 0 {
            Some(request_body)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

#[cfg(feature = "async")]
mod async_stream {
    use super::*;
    use futures::{future::BoxFuture, ready, FutureExt, Stream};
    use qiniu_apis::storage::batch_ops::AsyncRequestBuilder as BatchOpsAsyncRequestBuilder;
    use smart_default::SmartDefault;
    use std::{
        fmt::{self, Debug},
        io::Result as IOResult,
        pin::Pin,
        task::{Context, Poll},
    };

    /// 批量操作流
    ///
    /// 实现 [`futures::stream::Stream`] 接口，
    /// 在迭代过程中异步发起批量操作 API
    #[must_use]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    #[derive(Debug)]
    pub struct BatchOperationsStream<'a> {
        operations: BatchOperations<'a>,
        current_step: BatchOperationsStep<'a>,
        closed: bool,
    }

    #[derive(SmartDefault)]
    enum BatchOperationsStep<'a> {
        #[default]
        FromBuffer {
            buffer: VecDeque<ApiResult<OperationResponseData>>,
        },
        WaitForResponse {
            task: BoxFuture<'a, ApiResult<Response<ResponseBody>>>,
        },
        WaitForRegionProvider {
            task: BoxFuture<'a, IOResult<&'a dyn RegionsProvider>>,
        },
        Done,
    }

    impl Debug for BatchOperationsStep<'_> {
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

    impl Stream for BatchOperationsStream<'_> {
        type Item = ApiResult<OperationResponseData>;

        #[inline]
        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            match self.current_step {
                BatchOperationsStep::FromBuffer { .. } => self.read_from_buffer(cx),
                BatchOperationsStep::WaitForResponse { .. } => self.wait_for_response(cx),
                BatchOperationsStep::WaitForRegionProvider { .. } => self.wait_for_region(cx),
                BatchOperationsStep::Done { .. } => Poll::Ready(None),
            }
        }
    }

    impl<'a> BatchOperationsStream<'a> {
        pub(super) fn new(operations: BatchOperations<'a>) -> Self {
            Self {
                operations,
                current_step: Default::default(),
                closed: false,
            }
        }

        fn read_from_buffer(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<<Self as Stream>::Item>> {
            if let BatchOperationsStep::FromBuffer { buffer } = &mut self.current_step {
                if let Some(response) = buffer.pop_front() {
                    Poll::Ready(Some(response))
                } else if self.closed {
                    self.current_step = BatchOperationsStep::Done;
                    self.poll_next(cx)
                } else {
                    let bucket = self.operations.bucket;
                    self.current_step = BatchOperationsStep::WaitForRegionProvider {
                        task: Box::pin(async move { bucket.async_region_provider().await }),
                    };
                    self.poll_next(cx)
                }
            } else {
                unreachable!()
            }
        }

        fn wait_for_response(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<<Self as Stream>::Item>> {
            if let BatchOperationsStep::WaitForResponse { task } = &mut self.current_step {
                let mut response_result = ready!(task.poll_unpin(cx));
                if let Err(err) = self.operations.callbacks.after_response(&mut response_result) {
                    self.current_step = BatchOperationsStep::Done;
                    return Poll::Ready(Some(Err(make_callback_error(err))));
                }
                match response_result {
                    Ok(response) => {
                        self.current_step = BatchOperationsStep::FromBuffer {
                            buffer: response
                                .into_body()
                                .to_operation_response_vec()
                                .into_iter()
                                .map(from_response_to_response_data_result)
                                .collect(),
                        };
                        self.poll_next(cx)
                    }
                    Err(err) => {
                        self.current_step = BatchOperationsStep::Done;
                        Poll::Ready(Some(Err(err)))
                    }
                }
            } else {
                unreachable!()
            }
        }

        fn wait_for_region(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<<Self as Stream>::Item>> {
            if let BatchOperationsStep::WaitForRegionProvider { task } = &mut self.current_step {
                match ready!(task.poll_unpin(cx)) {
                    Ok(region_provider) => {
                        if let Some(request_body) = self.generate_request_body() {
                            let mut request = self.make_request(region_provider);
                            if let Err(err) = self.operations.callbacks.before_request(request.parts_mut()) {
                                self.current_step = BatchOperationsStep::Done;
                                Poll::Ready(Some(Err(make_callback_error(err))))
                            } else {
                                self.current_step = BatchOperationsStep::WaitForResponse {
                                    task: Box::pin(async move { request.call(request_body).await }),
                                };
                                self.poll_next(cx)
                            }
                        } else {
                            self.current_step = BatchOperationsStep::Done;
                            self.poll_next(cx)
                        }
                    }
                    Err(err) => {
                        self.current_step = BatchOperationsStep::Done;
                        Poll::Ready(Some(Err(err.into())))
                    }
                }
            } else {
                unreachable!()
            }
        }

        fn generate_request_body(&mut self) -> Option<RequestBody> {
            let mut request_body = RequestBody::default();
            let mut operation_count = 0usize;
            for _ in 0..self
                .operations
                .batch_size
                .as_ref()
                .map(|provider| provider.batch_size())
                .unwrap_or(DEFAULT_BATCH_SIZE)
            {
                if let Some(mut operation) = self.operations.operations.as_mut().and_then(|op| op.next()) {
                    request_body = request_body.append_operations_as_str(operation.to_operation());
                    operation_count += 1;
                } else {
                    self.closed = true;
                    break;
                }
            }
            if operation_count > 0 {
                Some(request_body)
            } else {
                None
            }
        }

        fn make_request(
            &self,
            region_provider: &'a dyn RegionsProvider,
        ) -> BatchOpsAsyncRequestBuilder<'a, RefRegionProviderEndpoints<'a>> {
            self.operations
                .bucket
                .objects_manager()
                .client()
                .storage()
                .batch_ops()
                .new_async_request(
                    RegionsProviderEndpoints::new(region_provider),
                    self.operations.bucket.objects_manager().credential(),
                )
        }

        #[allow(dead_code)]
        fn assert() {
            assert_impl!(Send: Self);
            // assert_impl!(Sync: Self);
        }
    }
}

#[cfg(feature = "async")]
pub use async_stream::*;

fn from_response_to_response_data_result(response: OperationResponse) -> ApiResult<OperationResponseData> {
    let status_code = StatusCode::from_u16(
        response
            .get_code_as_u64()
            .try_into()
            .map_err(make_invalid_request_response_error)?,
    )
    .map_err(make_invalid_request_response_error)?;

    return if status_code == StatusCode::OK {
        Ok(response.get_data().unwrap_or_default())
    } else {
        Err(ResponseError::new(
            ResponseErrorKind::StatusCodeError(status_code),
            response
                .get_data()
                .and_then(|data| data.get_error_as_str().map(|err| AnyError::msg(err.to_owned())))
                .unwrap_or_else(|| NoErrorMessageFromOperation.into()),
        ))
    };

    fn make_invalid_request_response_error(err: impl Into<AnyError>) -> ResponseError {
        ResponseError::new(HttpResponseErrorKind::InvalidRequestResponse.into(), err)
    }

    #[derive(Clone, Debug)]
    struct NoErrorMessageFromOperation;

    impl Display for NoErrorMessageFromOperation {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            Display::fmt("No Error Message from operation", f)
        }
    }

    impl StdError for NoErrorMessageFromOperation {}
}

#[cfg(test)]
mod tests {
    use super::{super::ObjectsManager, *};
    use qiniu_apis::{
        credential::Credential,
        http::{HeaderValue, HttpCaller, SyncRequest, SyncResponse, SyncResponseResult},
        http_client::{DirectChooser, HttpClient, NeverRetrier, Region, SyncResponseBody, NO_BACKOFF},
    };
    use qiniu_utils::BucketName;
    use serde_json::{json, to_vec as json_to_vec};
    use std::{
        io::Read,
        sync::atomic::{AtomicUsize, Ordering},
    };

    #[cfg(feature = "async")]
    use {
        futures::{future::BoxFuture, AsyncReadExt, StreamExt},
        qiniu_apis::http::{AsyncRequest, AsyncResponse, AsyncResponseBody, AsyncResponseResult},
    };

    #[test]
    fn test_sync_batch_ops() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            counter: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                let n = self.counter.fetch_add(1, Ordering::SeqCst);
                let mut req_body = Vec::new();
                request.body_mut().read_to_end(&mut req_body).unwrap();
                let pairs: Vec<(String, String)> = form_urlencoded::parse(&req_body).into_owned().collect();
                assert_eq!(pairs.len(), 3);
                assert!(pairs.iter().all(|(k, _)| k == "op"));
                let body = match n {
                    0 => SyncResponseBody::from_bytes(
                        json_to_vec(&json!([
                            {"code": 200, "data": {}},
                            {"code": 200, "data": {}},
                            {"code": 200, "data": {}},
                        ]))
                        .unwrap(),
                    ),
                    1 => SyncResponseBody::from_bytes(
                        json_to_vec(&json!([
                            {"code": 200, "data": {}},
                            {"code": 200, "data": {}},
                            {"code": 612, "data": {"error": "no such file or directory"}},
                        ]))
                        .unwrap(),
                    ),
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

        let bucket = get_bucket(FakeHttpCaller::default());
        let mut ops = batch_ops(&bucket);
        let mut iter = ops.call();
        for _ in 0..5 {
            iter.next().unwrap().unwrap();
        }
        assert_eq!(
            iter.next().unwrap().unwrap_err().kind(),
            ResponseErrorKind::StatusCodeError(StatusCode::from_u16(612)?)
        );
        Ok(())
    }

    #[cfg(feature = "async")]
    #[async_std::test]
    async fn test_async_batch_ops() -> anyhow::Result<()> {
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
                    let mut req_body = Vec::new();
                    request.body_mut().read_to_end(&mut req_body).await.unwrap();
                    let pairs: Vec<(String, String)> = form_urlencoded::parse(&req_body).into_owned().collect();
                    assert_eq!(pairs.len(), 3);
                    assert!(pairs.iter().all(|(k, _)| k == "op"));
                    let body = match n {
                        0 => AsyncResponseBody::from_bytes(
                            json_to_vec(&json!([
                                {"code": 200, "data": {}},
                                {"code": 200, "data": {}},
                                {"code": 200, "data": {}},
                            ]))
                            .unwrap(),
                        ),
                        1 => AsyncResponseBody::from_bytes(
                            json_to_vec(&json!([
                                {"code": 200, "data": {}},
                                {"code": 200, "data": {}},
                                {"code": 612, "data": {"error": "no such file or directory"}},
                            ]))
                            .unwrap(),
                        ),
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

        let bucket = get_bucket(FakeHttpCaller::default());
        let mut ops = batch_ops(&bucket);
        let mut iter = ops.async_call();
        for _ in 0..5 {
            iter.next().await.unwrap().unwrap();
        }
        assert_eq!(
            iter.next().await.unwrap().unwrap_err().kind(),
            ResponseErrorKind::StatusCodeError(StatusCode::from_u16(612)?)
        );
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
        object_manager.bucket_with_region(get_bucket_name(), single_rs_domain_region())
    }

    fn batch_ops(bucket: &Bucket) -> BatchOperations<'_> {
        let mut ops = bucket.batch_ops();
        ops.batch_size(3);
        ops.add_operation(bucket.copy_object_to("fakeobject1", "fakebucket2", "fakeobject1"));
        ops.add_operation(bucket.copy_object_to("fakeobject2", "fakebucket2", "fakeobject2"));
        ops.add_operation(bucket.copy_object_to("fakeobject3", "fakebucket2", "fakeobject3"));
        ops.add_operation(bucket.copy_object_to("fakeobject4", "fakebucket2", "fakeobject4"));
        ops.add_operation(bucket.copy_object_to("fakeobject5", "fakebucket2", "fakeobject5"));
        ops.add_operation(bucket.copy_object_to("fakeobject6", "fakebucket2", "fakeobject6"));
        ops
    }

    fn get_credential() -> Credential {
        Credential::new("fakeaccesskey", "fakesecretkey")
    }

    fn get_bucket_name() -> BucketName {
        "fakebucketname".into()
    }

    fn single_rs_domain_region() -> Region {
        Region::builder("chaotic")
            .add_rs_preferred_endpoint(("fakers.example.com".to_owned(), 8080).into())
            .build()
    }
}
