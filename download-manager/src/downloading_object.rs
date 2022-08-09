use super::{
    download_callbacks::Callbacks, DownloadRetrier, DownloadRetrierOptions, ErrorRetrier, RetriedStatsInfo,
    RetryDecision,
};
use anyhow::{Error as AnyError, Result as AnyResult};
use assert_impl::assert_impl;
use http::{
    header::{IntoHeaderName, CONTENT_LENGTH, ETAG, RANGE},
    uri::Scheme,
    HeaderMap, HeaderValue, Uri,
};
use qiniu_apis::{
    http::{
        uri::Parts as UriParts, ResponseErrorKind as HttpResponseErrorKind, ResponseParts as HttpResponseParts,
        TransferProgressInfo,
    },
    http_client::{
        ApiResult, Endpoint, HttpClient, RequestBuilderParts, Response, ResponseError, ResponseErrorKind,
        SyncResponseBody,
    },
};
use std::{
    borrow::Cow,
    fs::OpenOptions,
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult, Write},
    num::NonZeroU64,
    path::Path,
    vec::IntoIter,
};
use thiserror::Error;

#[cfg(feature = "async")]
use {
    async_std::fs::OpenOptions as AsyncOpenOptions,
    futures::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
    qiniu_apis::http_client::AsyncResponseBody,
};

/// 准备下载的对象
///
/// 可以在下载前设置范围参数或回调函数，以及写入数据的目标。
/// 需要注意的是，生成该对象并不表示数据处于下载状态了，
/// 下载将在调用 [`Self::to_path`]，[`Self::to_writer`]，[`DownloadingObjectReader::read`] 以后才正式开始。
#[must_use]
#[derive(Debug)]
pub struct DownloadingObject {
    http_client: HttpClient,
    urls_iter: IntoIter<Uri>,
    range_from: Option<NonZeroU64>,
    range_to: Option<NonZeroU64>,
    headers: HeaderMap,
    callbacks: Callbacks<'static>,
    retrier: Option<Box<dyn DownloadRetrier>>,
}

impl DownloadingObject {
    pub(super) fn new(http_client: HttpClient, urls: Vec<Uri>) -> Self {
        Self {
            http_client,
            range_from: None,
            range_to: None,
            retrier: None,
            urls_iter: urls.into_iter(),
            headers: Default::default(),
            callbacks: Default::default(),
        }
    }

    /// 设置下载范围起始位置
    ///
    /// 单位为字节，如果不调用，默认从第一个字节开始下载
    #[inline]
    pub fn range_from(mut self, range_from: NonZeroU64) -> Self {
        self.range_from = Some(range_from);
        self
    }

    /// 设置下载范围结束位置，包含该位置
    ///
    /// 例如如果要下载前 500 个字节，则调用该方法时应该传入 `499`
    ///
    /// 单位为字节，如果不调用，默认下载到最后一个字节
    #[inline]
    pub fn range_to(mut self, range_to: NonZeroU64) -> Self {
        self.range_to = Some(range_to);
        self
    }

    /// 设置下载重试器
    ///
    /// 默认使用 [`ErrorRetrier`]
    #[inline]
    pub fn retrier(mut self, retrier: impl DownloadRetrier + 'static) -> Self {
        self.retrier = Some(Box::new(retrier));
        self
    }

    /// 设置 HTTP 请求头
    #[inline]
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    /// 添加 HTTP 请求头
    #[inline]
    pub fn set_header(mut self, header_name: impl IntoHeaderName, header_value: impl Into<HeaderValue>) -> Self {
        self.headers.insert(header_name, header_value.into());
        self
    }

    /// 设置请求前的回调函数
    #[inline]
    pub fn on_before_request<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'static,
    {
        self.callbacks.insert_before_request_callback(callback);
        self
    }

    /// 设置下载进度回调函数
    #[inline]
    pub fn on_download_progress<F: Fn(DownloadingProgressInfo) -> AnyResult<()> + Send + Sync + 'static>(
        mut self,
        callback: F,
    ) -> Self {
        self.callbacks.insert_download_progress_callback(callback);
        self
    }

    /// 设置响应成功的回调函数
    #[inline]
    pub fn on_response_ok<F: Fn(&mut HttpResponseParts) -> AnyResult<()> + Send + Sync + 'static>(
        mut self,
        callback: F,
    ) -> Self {
        self.callbacks.insert_after_response_ok_callback(callback);
        self
    }

    /// 设置响应错误的回调函数
    #[inline]
    pub fn on_response_error<F: Fn(&ResponseError) -> AnyResult<()> + Send + Sync + 'static>(
        mut self,
        callback: F,
    ) -> Self {
        self.callbacks.insert_after_response_error_callback(callback);
        self
    }

    /// 将下载的对象内容写入指定的文件系统路径
    ///
    /// 需要注意，如果文件已经存在，则会覆盖该文件，如果文件不存在，则会创建该文件。
    ///
    /// 该方法的异步版本为 [`Self::async_to_path`]。
    ///
    /// ### 代码示例
    ///
    /// ```
    /// # use qiniu_download_manager::{apis::credential::Credential, DownloadManager, StaticDomainsUrlsGenerator, UrlsSigner};
    /// # fn example() -> anyhow::Result<()> {
    /// # let object_name = "test-object";
    /// # let download_manager = DownloadManager::new(UrlsSigner::new(
    /// #     Credential::new("abcdefghklmnopq", "1234567890"),
    /// #     StaticDomainsUrlsGenerator::new("my-domain.com")
    /// # ));
    /// download_manager
    ///     .download(object_name)?
    ///     .to_path("/home/qiniu/test.png")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_path(self, path: impl AsRef<Path>) -> DownloadResult<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path.as_ref())?;
        self.to_writer(&mut file)
    }

    /// 将下载的对象内容异步写入指定的文件系统路径
    ///
    /// 需要注意，如果文件已经存在，则会覆盖该文件，如果文件不存在，则会创建该文件。
    ///
    /// ### 代码示例
    ///
    /// ```
    /// # use qiniu_download_manager::{apis::credential::Credential, DownloadManager, StaticDomainsUrlsGenerator, UrlsSigner};
    /// # async fn example() -> anyhow::Result<()> {
    /// # let object_name = "test-object";
    /// # let download_manager = DownloadManager::new(UrlsSigner::new(
    /// #     Credential::new("abcdefghklmnopq", "1234567890"),
    /// #     StaticDomainsUrlsGenerator::new("my-domain.com")
    /// # ));
    /// download_manager
    ///     .async_download(object_name)
    ///     .await?
    ///     .async_to_path("/home/qiniu/test.png")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub async fn async_to_path(self, path: impl AsRef<Path>) -> DownloadResult<()> {
        let mut file = AsyncOpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path.as_ref())
            .await?;
        self.to_async_writer(&mut file).await
    }

    /// 将下载的对象内容写入指定的输出流
    ///
    /// 该方法的异步版本为 [`Self::to_async_writer`]。
    ///
    /// ### 代码示例
    ///
    /// ```
    /// # use qiniu_download_manager::{apis::credential::Credential, DownloadManager, StaticDomainsUrlsGenerator, UrlsSigner};
    /// # fn example() -> anyhow::Result<()> {
    /// # let object_name = "test-object";
    /// # let download_manager = DownloadManager::new(UrlsSigner::new(
    /// #     Credential::new("abcdefghklmnopq", "1234567890"),
    /// #     StaticDomainsUrlsGenerator::new("my-domain.com")
    /// # ));
    /// let mut buf = Vec::new();
    /// download_manager
    ///     .download(object_name)?
    ///     .to_writer(&mut buf)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_writer(self, writer: &mut dyn Write) -> DownloadResult<()> {
        let mut buf = [0u8; 1 << 15];
        let mut reader = self.into_inner_reader();
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            writer.write_all(&buf[..n])?;
        }
        Ok(())
    }

    /// 将下载的对象内容写入指定的异步输出流
    ///
    /// ### 代码示例
    ///
    /// ```
    /// # use qiniu_download_manager::{apis::credential::Credential, DownloadManager, StaticDomainsUrlsGenerator, UrlsSigner};
    /// # async fn example() -> anyhow::Result<()> {
    /// # let object_name = "test-object";
    /// # let download_manager = DownloadManager::new(UrlsSigner::new(
    /// #     Credential::new("abcdefghklmnopq", "1234567890"),
    /// #     StaticDomainsUrlsGenerator::new("my-domain.com")
    /// # ));
    /// let mut buf = Vec::new();
    /// download_manager
    ///     .async_download(object_name)
    ///     .await?
    ///     .to_async_writer(&mut buf)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub async fn to_async_writer(self, writer: &mut (dyn AsyncWrite + Send + Sync + Unpin)) -> DownloadResult<()> {
        let mut buf = [0u8; 1 << 15];
        let mut reader = self.into_inner_reader();
        loop {
            let n = reader.async_read(&mut buf).await?;
            if n == 0 {
                break;
            }
            writer.write_all(&buf[..n]).await?;
        }
        Ok(())
    }

    /// 将下载的对象内容包装成 [`Read`] 对象
    ///
    /// 但注意，由于 [`Read`] 接口的限制，错误信息会被 [`IoError`] 封装
    ///
    /// 该方法的异步版本为 [`Self::into_async_read`]。
    ///
    /// ### 代码示例
    ///
    /// ```
    /// # use qiniu_download_manager::{apis::credential::Credential, DownloadManager, StaticDomainsUrlsGenerator, UrlsSigner};
    /// # fn example() -> anyhow::Result<()> {
    /// # let object_name = "test-object";
    /// # let download_manager = DownloadManager::new(UrlsSigner::new(
    /// #     Credential::new("abcdefghklmnopq", "1234567890"),
    /// #     StaticDomainsUrlsGenerator::new("my-domain.com")
    /// # ));
    /// let mut buf = Vec::new();
    /// let mut reader = download_manager
    ///     .download(object_name)?
    ///     .into_read();
    /// std::io::copy(&mut reader, &mut buf)?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn into_read(self) -> DownloadingObjectReader {
        DownloadingObjectReader(self.into_inner_reader())
    }

    /// 将下载的对象内容包装成 [`futures::AsyncRead`] 对象
    ///
    /// 但注意，由于 [`futures::AsyncRead`] 接口的限制，错误信息会被 [`IoError`] 封装
    ///
    /// ### 代码示例
    ///
    /// ```
    /// # use qiniu_download_manager::{apis::credential::Credential, DownloadManager, StaticDomainsUrlsGenerator, UrlsSigner};
    /// # async fn example() -> anyhow::Result<()> {
    /// # let object_name = "test-object";
    /// # let download_manager = DownloadManager::new(UrlsSigner::new(
    /// #     Credential::new("abcdefghklmnopq", "1234567890"),
    /// #     StaticDomainsUrlsGenerator::new("my-domain.com")
    /// # ));
    /// let mut buf = Vec::new();
    /// let mut reader = download_manager
    ///     .async_download(object_name)
    ///     .await?
    ///     .into_async_read();
    /// futures::io::copy(&mut reader, &mut buf).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub fn into_async_read(self) -> AsyncDownloadingObjectReader {
        AsyncDownloadingObjectReader::new(self.into_inner_reader())
    }

    fn into_inner_reader<B>(self) -> InnerReader<B> {
        InnerReader {
            http_client: self.http_client,
            urls_iter: self.urls_iter,
            headers: self.headers,
            callbacks: self.callbacks,
            range_from: self.range_from,
            range_to: self.range_to,
            retrier: self.retrier.unwrap_or_else(|| Box::new(ErrorRetrier)),
            retried: Default::default(),
            have_read: 0,
            content_length: None,
            etag: None,
            response: None,
        }
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// 下载对象的内容阅读器
///
/// 实现了 [`Read`] 接口
#[must_use]
#[derive(Debug)]
pub struct DownloadingObjectReader(InnerReader<SyncResponseBody>);

impl Read for DownloadingObjectReader {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let n = self.0.read(buf)?;
        Ok(n)
    }
}

impl DownloadingObjectReader {
    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        // assert_impl!(Sync: Self);
    }
}

#[cfg(feature = "async")]
mod async_reader {
    use super::*;
    use futures::{future::BoxFuture, io::Cursor, lock::Mutex, ready, AsyncRead, FutureExt};
    use smart_default::SmartDefault;
    use std::{
        fmt::{self, Debug},
        pin::Pin,
        sync::Arc,
        task::{Context, Poll},
    };

    /// 下载对象的内容阅读器
    ///
    /// 实现了 [`AsyncRead`] 接口
    #[must_use]
    #[derive(Debug)]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub struct AsyncDownloadingObjectReader {
        step: AsyncDownloadingObjectReaderStep,
        inner: Arc<Mutex<InnerReader<AsyncResponseBody>>>,
    }

    impl AsyncDownloadingObjectReader {
        #[allow(dead_code)]
        fn assert() {
            assert_impl!(Send: Self);
            // assert_impl!(Sync: Self);
            assert_impl!(Unpin: Self);
        }
    }

    #[derive(SmartDefault)]
    enum AsyncDownloadingObjectReaderStep {
        #[default]
        Buffered(Cursor<Vec<u8>>),
        Waiting(BoxFuture<'static, IoResult<Vec<u8>>>),
        Done,
    }

    impl Debug for AsyncDownloadingObjectReaderStep {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Buffered(buffer) => f.debug_tuple("Buffered").field(buffer).finish(),
                Self::Waiting(_) => f.debug_tuple("Waiting").finish(),
                Self::Done => f.debug_tuple("Done").finish(),
            }
        }
    }

    impl AsyncDownloadingObjectReader {
        pub(super) fn new(inner: InnerReader<AsyncResponseBody>) -> Self {
            Self {
                inner: Arc::new(Mutex::new(inner)),
                step: Default::default(),
            }
        }
    }

    impl AsyncRead for AsyncDownloadingObjectReader {
        fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<IoResult<usize>> {
            match &mut self.step {
                AsyncDownloadingObjectReaderStep::Buffered(buffered) => {
                    match ready!(Pin::new(buffered).poll_read(cx, buf)) {
                        Ok(0) => {
                            let reader = self.inner.to_owned();
                            self.step = AsyncDownloadingObjectReaderStep::Waiting(Box::pin(async move {
                                let mut buf = vec![0u8; 1 << 20];
                                let have_read = reader.lock().await.async_read(&mut buf).await?;
                                buf.truncate(have_read);
                                Ok(buf)
                            }));
                            self.poll_read(cx, buf)
                        }
                        result => Poll::Ready(result),
                    }
                }
                AsyncDownloadingObjectReaderStep::Waiting(future) => match ready!(future.poll_unpin(cx)) {
                    Ok(buffered) if buffered.is_empty() => {
                        self.step = AsyncDownloadingObjectReaderStep::Done;
                        Poll::Ready(Ok(0))
                    }
                    Ok(buffered) => {
                        self.step = AsyncDownloadingObjectReaderStep::Buffered(Cursor::new(buffered));
                        self.poll_read(cx, buf)
                    }
                    Err(err) => {
                        self.step = Default::default();
                        Poll::Ready(Err(err))
                    }
                },
                AsyncDownloadingObjectReaderStep::Done => Poll::Ready(Ok(0)),
            }
        }
    }
}

#[cfg(feature = "async")]
pub use async_reader::*;

#[derive(Debug)]
struct InnerReader<B> {
    http_client: HttpClient,
    urls_iter: IntoIter<Uri>,
    headers: HeaderMap,
    callbacks: Callbacks<'static>,
    retrier: Box<dyn DownloadRetrier>,
    retried: RetriedStatsInfo,
    range_from: Option<NonZeroU64>,
    range_to: Option<NonZeroU64>,
    content_length: Option<u64>,
    have_read: u64,
    etag: Option<HeaderValue>,
    response: Option<ResponseInfo<B>>,
}

impl InnerReader<SyncResponseBody> {
    fn read(&mut self, mut buf: &mut [u8]) -> DownloadResult<usize> {
        return if let Some(response) = &mut self.response {
            let mut response_unread = None;
            if let Some(content_length) = self.content_length {
                let unread = usize::try_from(content_length - self.have_read).unwrap_or(usize::max_value());
                let to_read = buf.len().min(unread);
                buf = &mut buf[..to_read];
                response_unread = Some(unread);
            }
            match response.body.read(buf) {
                Ok(0) => match response_unread {
                    Some(unread) if unread > 0 => make_request(
                        self,
                        buf,
                        Some(ResponseError::new_with_msg(
                            ResponseErrorKind::UnexpectedEof,
                            format!("Transfer closed with {} bytes remaining to read", unread),
                        )),
                    ),
                    _ => Ok(0),
                },
                Ok(have_read) => self.handle_have_read(&buf[..have_read]),
                Err(err) => make_request(
                    self,
                    buf,
                    Some(ResponseError::new(
                        ResponseErrorKind::HttpError(HttpResponseErrorKind::ReceiveError),
                        err,
                    )),
                ),
            }
        } else {
            make_request(self, buf, None)
        };

        fn make_request(
            inner_reader: &mut InnerReader<SyncResponseBody>,
            buf: &mut [u8],
            original_err: Option<ResponseError>,
        ) -> DownloadResult<usize> {
            let response = inner_reader.make_request(original_err)?;
            inner_reader.response = Some(response);
            inner_reader.read(buf)
        }
    }

    fn make_request(&mut self, mut last_err: Option<ResponseError>) -> DownloadResult<ResponseInfo<SyncResponseBody>> {
        loop {
            let uri = if let Some(response) = &mut self.response {
                response.uri.to_owned()
            } else if let Some(next_uri) = self.urls_iter.next() {
                next_uri
            } else if let Some(last_err) = last_err {
                return Err(DownloadError::AllUrlsFailed(last_err));
            } else {
                return Err(DownloadError::NoUrlTried);
            };
            let mut uri_parts = uri.to_owned().into_parts();
            let mut request_builder = self.http_client.get(&[], make_endpoint_from_uri(&mut uri_parts)?);

            set_uri_into_request(request_builder.parts_mut(), &uri_parts)?;
            set_headers_into_request(request_builder.parts_mut(), self.headers.to_owned());
            set_range_into_request(request_builder.parts_mut(), self.range_from, self.range_to);
            before_request_call(&self.callbacks, request_builder.parts_mut())?;

            let mut response_result = request_builder.call();
            after_response_call(&self.callbacks, &mut response_result)?;

            match response_result {
                Ok(response) => {
                    drop(request_builder);
                    return self.handle_response(response, uri);
                }
                Err(err) => {
                    let decision = self
                        .retrier
                        .retry(
                            &mut request_builder.into_parts().build(),
                            DownloadRetrierOptions::new(&err, &self.retried),
                        )
                        .decision();
                    if self.handle_decision(decision) {
                        last_err = Some(err);
                    } else {
                        return Err(err.into());
                    }
                }
            }
        }
    }
}

#[cfg(feature = "async")]
impl InnerReader<AsyncResponseBody> {
    async fn async_read(&mut self, mut buf: &mut [u8]) -> DownloadResult<usize> {
        loop {
            if let Some(response) = &mut self.response {
                let mut response_unread = None;
                if let Some(content_length) = self.content_length {
                    let unread = usize::try_from(content_length - self.have_read).unwrap_or(usize::max_value());
                    let to_read = buf.len().min(unread);
                    buf = &mut buf[..to_read];
                    response_unread = Some(unread);
                }
                match response.body.read(buf).await {
                    Ok(0) => match response_unread {
                        Some(unread) if unread > 0 => {
                            let err = ResponseError::new_with_msg(
                                ResponseErrorKind::UnexpectedEof,
                                format!("Transfer closed with {} bytes remaining to read", unread),
                            );
                            self.response = Some(self.make_async_request(Some(err)).await?);
                        }
                        _ => {
                            return Ok(0);
                        }
                    },
                    Ok(have_read) => {
                        return self.handle_have_read(&buf[..have_read]);
                    }
                    Err(err) => {
                        let err =
                            ResponseError::new(ResponseErrorKind::HttpError(HttpResponseErrorKind::ReceiveError), err);
                        self.response = Some(self.make_async_request(Some(err)).await?);
                    }
                }
            } else {
                self.response = Some(self.make_async_request(None).await?);
            };
        }
    }

    async fn make_async_request(
        &mut self,
        mut last_err: Option<ResponseError>,
    ) -> DownloadResult<ResponseInfo<AsyncResponseBody>> {
        loop {
            let uri = if let Some(response) = &mut self.response {
                response.uri.to_owned()
            } else if let Some(next_uri) = self.urls_iter.next() {
                next_uri
            } else if let Some(last_err) = last_err {
                return Err(DownloadError::AllUrlsFailed(last_err));
            } else {
                return Err(DownloadError::NoUrlTried);
            };
            let mut uri_parts = uri.to_owned().into_parts();
            let mut request_builder = self.http_client.async_get(&[], make_endpoint_from_uri(&mut uri_parts)?);

            set_uri_into_request(request_builder.parts_mut(), &uri_parts)?;
            set_headers_into_request(request_builder.parts_mut(), self.headers.to_owned());
            set_range_into_request(request_builder.parts_mut(), self.range_from, self.range_to);
            before_request_call(&self.callbacks, request_builder.parts_mut())?;

            let mut response_result = request_builder.call().await;
            after_response_call(&self.callbacks, &mut response_result)?;

            match response_result {
                Ok(response) => {
                    drop(request_builder);
                    return self.handle_response(response, uri);
                }
                Err(err) => {
                    let decision = self
                        .retrier
                        .retry(
                            &mut request_builder.into_parts().build(),
                            DownloadRetrierOptions::new(&err, &self.retried),
                        )
                        .decision();
                    if self.handle_decision(decision) {
                        last_err = Some(err);
                    } else {
                        return Err(err.into());
                    }
                }
            }
        }
    }
}

impl<B> InnerReader<B> {
    fn handle_have_read(&mut self, buf: &[u8]) -> DownloadResult<usize> {
        let have_read = buf.len() as u64;
        self.range_from = NonZeroU64::new(self.range_from.map(|v| v.get()).unwrap_or(0) + have_read);
        self.have_read += have_read;
        self.callbacks
            .download_progress(DownloadingProgressInfo::new(self.have_read, self.content_length))
            .map_err(make_callback_error)?;
        Ok(buf.len())
    }

    fn handle_response(&mut self, response: Response<B>, uri: Uri) -> DownloadResult<ResponseInfo<B>> {
        let content_length = response
            .header(CONTENT_LENGTH)
            .map(|content_length| {
                content_length
                    .to_str()
                    .ok()
                    .and_then(|content_length| content_length.parse::<u64>().ok())
                    .map_or_else(
                        || {
                            Err(DownloadError::ResponseError(ResponseError::new_with_msg(
                                ResponseErrorKind::MaliciousResponse,
                                "content_length is invalid in response headers",
                            )))
                        },
                        Ok,
                    )
            })
            .transpose()?;
        if self.content_length.is_none() {
            self.content_length = content_length;
        }
        if let Some(etag) = response.header(ETAG) {
            if let Some(first_etag) = &self.etag {
                if first_etag != etag {
                    return Err(DownloadError::ContentChanged);
                }
            } else {
                self.etag = Some(etag.to_owned());
            }
            Ok(ResponseInfo {
                uri,
                body: response.into_body(),
            })
        } else {
            Err(DownloadError::ResponseError(ResponseError::new_with_msg(
                ResponseErrorKind::MaliciousResponse,
                "etag is missing in response headers",
            )))
        }
    }

    fn handle_decision(&mut self, decision: RetryDecision) -> bool {
        match decision {
            RetryDecision::DontRetry => false,
            RetryDecision::TryNextServer => {
                self.response = None;
                self.retried.switch_endpoint();
                true
            }
            RetryDecision::RetryRequest => {
                self.retried.increase();
                true
            }
        }
    }
}

impl<B: Sync + Send> InnerReader<B> {
    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// 下载进度信息
#[derive(Debug, Clone, Copy)]
pub struct DownloadingProgressInfo {
    transferred_bytes: u64,
    total_bytes: Option<u64>,
}

impl DownloadingProgressInfo {
    /// 创建下载进度信息
    #[inline]
    pub fn new(transferred_bytes: u64, total_bytes: Option<u64>) -> Self {
        Self {
            transferred_bytes,
            total_bytes,
        }
    }

    /// 获取已传输的字节数
    #[inline]
    pub fn transferred_bytes(&self) -> u64 {
        self.transferred_bytes
    }

    /// 获取总字节数
    #[inline]
    pub fn total_bytes(&self) -> Option<u64> {
        self.total_bytes
    }
}

impl<'a> From<&'a TransferProgressInfo<'a>> for DownloadingProgressInfo {
    #[inline]
    fn from(t: &'a TransferProgressInfo<'a>) -> Self {
        Self::new(t.transferred_bytes(), Some(t.total_bytes()))
    }
}

impl From<TransferProgressInfo<'_>> for DownloadingProgressInfo {
    #[inline]
    fn from(t: TransferProgressInfo<'_>) -> Self {
        Self::new(t.transferred_bytes(), Some(t.total_bytes()))
    }
}

#[derive(Debug)]
struct ResponseInfo<B> {
    body: B,
    uri: Uri,
}

/// 下载错误
#[derive(Error, Debug)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum DownloadError {
    /// 下载响应错误
    #[error("Http response error: {0}")]
    ResponseError(#[from] ResponseError),

    /// 下载的文件内容变更
    ///
    /// 这种错误无法重试，用户必须抛弃先前已经下载的内容，然后重新下载
    #[error("Response content was changed")]
    ContentChanged,

    /// 所有下载 URL 都被尝试过且失败了
    #[error("All URLs are tried and failed, last error is {0}")]
    AllUrlsFailed(ResponseError),

    /// 没有任何下载的 URL 提供
    #[error("No URL is tried")]
    NoUrlTried,
}

/// 下载结果
pub type DownloadResult<T> = Result<T, DownloadError>;

impl From<IoError> for DownloadError {
    #[inline]
    fn from(err: IoError) -> Self {
        ResponseError::from(err).into()
    }
}

impl From<DownloadError> for IoError {
    #[inline]
    fn from(err: DownloadError) -> Self {
        IoError::new(IoErrorKind::Other, err)
    }
}

fn make_callback_error(err: AnyError) -> DownloadError {
    DownloadError::ResponseError(ResponseError::new(HttpResponseErrorKind::CallbackError.into(), err))
}

fn set_uri_into_request<'a>(request: &mut RequestBuilderParts<'a>, uri: &'a UriParts) -> DownloadResult<()> {
    let scheme = uri.scheme.as_ref().unwrap_or(&Scheme::HTTPS);
    if scheme == &Scheme::HTTP {
        request.use_https(false);
    } else if scheme == &Scheme::HTTPS {
        request.use_https(true);
    } else {
        return Err(DownloadError::ResponseError(ResponseError::new_with_msg(
            ResponseErrorKind::HttpError(HttpResponseErrorKind::InvalidUrl),
            "scheme is neither http nor https in http::Uri",
        )));
    }

    if let Some(path_and_query) = &uri.path_and_query {
        request.path(path_and_query.path());
        if let Some(query) = path_and_query.query() {
            request.query(query);
        }
    } else {
        return Err(DownloadError::ResponseError(ResponseError::new_with_msg(
            ResponseErrorKind::HttpError(HttpResponseErrorKind::InvalidUrl),
            "path_and_query is neither http nor https in http::Uri",
        )));
    }

    Ok(())
}

fn set_headers_into_request(request: &mut RequestBuilderParts<'_>, headers: HeaderMap) {
    request.headers(Cow::Owned(headers));
}

fn set_range_into_request(
    request: &mut RequestBuilderParts<'_>,
    range_from: Option<NonZeroU64>,
    range_to: Option<NonZeroU64>,
) {
    let value = match (range_from, range_to) {
        (None, Some(range_to)) => {
            format!("bytes=-{}", range_to)
        }
        (Some(range_from), None) => {
            format!("bytes={}-", range_from)
        }
        (Some(range_from), Some(range_to)) => {
            format!("bytes={}-{}", range_from, range_to)
        }
        _ => {
            return;
        }
    };
    request.set_header(RANGE, HeaderValue::from_str(&value).unwrap());
}

fn before_request_call(callbacks: &Callbacks<'_>, builder_parts: &mut RequestBuilderParts) -> DownloadResult<()> {
    callbacks.before_request(builder_parts).map_err(make_callback_error)
}

fn after_response_call<B>(callbacks: &Callbacks<'_>, response: &mut ApiResult<Response<B>>) -> DownloadResult<()> {
    callbacks.after_response(response).map_err(make_callback_error)
}

fn make_endpoint_from_uri(uri: &mut UriParts) -> DownloadResult<Endpoint> {
    uri.authority.take().map(Endpoint::from).map_or_else(
        || {
            Err(DownloadError::ResponseError(ResponseError::new_with_msg(
                ResponseErrorKind::HttpError(HttpResponseErrorKind::InvalidUrl),
                "authority is missing in http::Uri",
            )))
        },
        Ok,
    )
}

#[cfg(test)]
#[cfg(feature = "async")]
mod tests {
    use super::{
        super::{DownloadManager, StaticDomainsUrlsGenerator, UrlsSigner},
        *,
    };
    use async_std::task::spawn_blocking;
    use futures::future::BoxFuture;
    use http::{
        header::{CONTENT_LENGTH, ETAG, RANGE, REFERER},
        HeaderMap, StatusCode,
    };
    use qiniu_apis::{
        credential::Credential,
        http::{
            AsyncRequest as AsyncHttpRequest, AsyncResponseBody as AsyncHttpResponseBody,
            AsyncResponseResult as AsyncHttpResponseResult, HttpCaller, Response as HttpResponse,
            ResponseError as HttpResponseError, ResponseErrorKind as HttpResponseErrorKind,
            ResponseResult as HttpResponseResult, SyncRequest as SyncHttpRequest,
            SyncResponseBody as SyncHttpResponseBody, SyncResponseResult as SyncHttpResponseResult,
        },
        http_client::NeverRetrier,
    };
    use rand::{thread_rng, RngCore};
    use sha1::{Digest, Sha1};
    use std::{
        io::copy,
        net::{IpAddr, Ipv4Addr},
        sync::{
            atomic::{AtomicU64, AtomicUsize, Ordering},
            Arc,
        },
    };

    #[async_std::test]
    async fn test_inner_reader_signature() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct CounterHttpCaller(AtomicUsize);

        impl CounterHttpCaller {
            fn handle<B: Default>(&self, url: &str, headers: &HeaderMap, body: B) -> HttpResponseResult<B> {
                assert!(url.starts_with("http://127.0.0.1/test-key?e="));
                assert!(url.contains("&token="));
                assert_eq!(headers.get(REFERER).unwrap().to_str().unwrap(), "http://example.com");
                if self.0.fetch_add(1, Ordering::Relaxed) > 0 {
                    Err(
                        HttpResponseError::builder_with_msg(HttpResponseErrorKind::InvalidUrl, "called more than once")
                            .build(),
                    )
                } else {
                    Ok(HttpResponse::builder()
                        .header("x-reqid", HeaderValue::from_static("fake-reqid"))
                        .header(CONTENT_LENGTH, HeaderValue::from_static("10"))
                        .header(ETAG, HeaderValue::from_static("fake-etag"))
                        .body(body)
                        .build())
                }
            }
        }

        impl HttpCaller for CounterHttpCaller {
            fn call(&self, request: &mut SyncHttpRequest<'_>) -> SyncHttpResponseResult {
                self.handle(
                    &request.url().to_string(),
                    request.headers(),
                    SyncHttpResponseBody::from_bytes(b"1234567890".to_vec()),
                )
            }

            #[cfg(feature = "async")]
            fn async_call<'a>(
                &'a self,
                request: &'a mut AsyncHttpRequest<'_>,
            ) -> BoxFuture<'a, AsyncHttpResponseResult> {
                Box::pin(async move {
                    self.handle(
                        &request.url().to_string(),
                        request.headers(),
                        AsyncHttpResponseBody::from_bytes(b"1234567890".to_vec()),
                    )
                })
            }
        }

        spawn_blocking(|| {
            let mut inner_reader = get_download_manager()
                .download("test-key")?
                .set_header(REFERER, HeaderValue::from_static("http://example.com"))
                .into_inner_reader();
            let mut buf = [0u8; 1024];
            assert_eq!(inner_reader.read(&mut buf)?, 10);
            assert_eq!(inner_reader.read(&mut buf)?, 0);
            Ok::<_, anyhow::Error>(())
        })
        .await?;

        {
            let mut inner_reader = get_download_manager()
                .async_download("test-key")
                .await?
                .set_header(REFERER, HeaderValue::from_static("http://example.com"))
                .into_inner_reader();
            let mut buf = [0u8; 1024];
            assert_eq!(inner_reader.async_read(&mut buf).await?, 10);
            assert_eq!(inner_reader.async_read(&mut buf).await?, 0);
        }

        return Ok(());

        fn get_download_manager() -> DownloadManager {
            DownloadManager::builder(UrlsSigner::new(
                get_credential(),
                StaticDomainsUrlsGenerator::new(Endpoint::new_from_ip_addr(IpAddr::V4(Ipv4Addr::LOCALHOST))),
            ))
            .http_client(
                HttpClient::builder(CounterHttpCaller(Default::default()))
                    .request_retrier(NeverRetrier)
                    .build(),
            )
            .build()
        }
    }

    #[async_std::test]
    async fn test_inner_reader_unexpected_eof() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct CounterHttpCaller(AtomicUsize);

        impl CounterHttpCaller {
            fn handle<B: Default>(&self, headers: &HeaderMap, body_1: B, body_2: B) -> HttpResponseResult<B> {
                let mut response_builder = HttpResponse::builder();
                response_builder
                    .header("x-reqid", HeaderValue::from_static("fake-reqid"))
                    .header(ETAG, HeaderValue::from_static("fake-etag"));
                match self.0.fetch_add(1, Ordering::Relaxed) {
                    0 => Ok(response_builder
                        .header(CONTENT_LENGTH, HeaderValue::from_static("10"))
                        .body(body_1)
                        .build()),
                    1 => {
                        assert_eq!(headers.get(&RANGE).unwrap().to_str().unwrap(), "bytes=5-");
                        Ok(response_builder
                            .header(CONTENT_LENGTH, HeaderValue::from_static("5"))
                            .body(body_2)
                            .build())
                    }
                    _ => Err(HttpResponseError::builder_with_msg(
                        HttpResponseErrorKind::InvalidUrl,
                        "called more than twice",
                    )
                    .build()),
                }
            }
        }

        impl HttpCaller for CounterHttpCaller {
            fn call(&self, request: &mut SyncHttpRequest<'_>) -> SyncHttpResponseResult {
                self.handle(
                    request.headers(),
                    SyncHttpResponseBody::from_bytes(b"12345".to_vec()),
                    SyncHttpResponseBody::from_bytes(b"67890".to_vec()),
                )
            }

            #[cfg(feature = "async")]
            fn async_call<'a>(
                &'a self,
                request: &'a mut AsyncHttpRequest<'_>,
            ) -> BoxFuture<'a, AsyncHttpResponseResult> {
                Box::pin(async move {
                    self.handle(
                        request.headers(),
                        AsyncHttpResponseBody::from_bytes(b"12345".to_vec()),
                        AsyncHttpResponseBody::from_bytes(b"67890".to_vec()),
                    )
                })
            }
        }

        spawn_blocking(|| {
            let current_progress = Arc::new(AtomicU64::new(0));
            let mut inner_reader = get_download_manager()
                .download("test-key")?
                .on_download_progress({
                    let current_progress = current_progress.to_owned();
                    move |transfer| {
                        current_progress.store(transfer.transferred_bytes(), Ordering::Relaxed);
                        assert_eq!(transfer.total_bytes(), Some(10));
                        Ok(())
                    }
                })
                .into_inner_reader();
            let mut buf = [0u8; 1024];
            assert_eq!(inner_reader.read(&mut buf)?, 5);
            assert_eq!(current_progress.load(Ordering::Relaxed), 5);
            assert_eq!(inner_reader.read(&mut buf)?, 5);
            assert_eq!(current_progress.load(Ordering::Relaxed), 10);
            assert_eq!(inner_reader.read(&mut buf)?, 0);
            Ok::<_, anyhow::Error>(())
        })
        .await?;

        {
            let current_progress = Arc::new(AtomicU64::new(0));
            let mut inner_reader = get_download_manager()
                .async_download("test-key")
                .await?
                .on_download_progress({
                    let current_progress = current_progress.to_owned();
                    move |transfer| {
                        current_progress.store(transfer.transferred_bytes(), Ordering::Relaxed);
                        assert_eq!(transfer.total_bytes(), Some(10));
                        Ok(())
                    }
                })
                .into_inner_reader();
            let mut buf = [0u8; 1024];
            assert_eq!(inner_reader.async_read(&mut buf).await?, 5);
            assert_eq!(current_progress.load(Ordering::Relaxed), 5);
            assert_eq!(inner_reader.async_read(&mut buf).await?, 5);
            assert_eq!(current_progress.load(Ordering::Relaxed), 10);
            assert_eq!(inner_reader.async_read(&mut buf).await?, 0);
        }

        return Ok(());

        fn get_download_manager() -> DownloadManager {
            DownloadManager::builder(UrlsSigner::new(
                get_credential(),
                StaticDomainsUrlsGenerator::new(Endpoint::new_from_ip_addr(IpAddr::V4(Ipv4Addr::LOCALHOST))),
            ))
            .http_client(
                HttpClient::builder(CounterHttpCaller(Default::default()))
                    .request_retrier(NeverRetrier)
                    .build(),
            )
            .build()
        }
    }

    #[async_std::test]
    async fn test_inner_reader_5xx_error() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct CounterHttpCaller(AtomicUsize);

        impl CounterHttpCaller {
            fn handle<B: Default>(&self, url: &str, body_1: B, body_2: B) -> HttpResponseResult<B> {
                let called = self.0.fetch_add(1, Ordering::Relaxed);
                let mut response_builder = HttpResponse::builder();
                response_builder
                    .header("x-reqid", HeaderValue::from_static("fake-reqid"))
                    .header(ETAG, HeaderValue::from_static("fake-etag"));
                if url.starts_with("http://127.0.0.1/test-key") {
                    assert_eq!(called, 0);
                    Ok(response_builder
                        .status_code(StatusCode::GATEWAY_TIMEOUT)
                        .header(CONTENT_LENGTH, HeaderValue::from_static("16"))
                        .body(body_1)
                        .build())
                } else if url.starts_with("http://127.0.0.2/test-key") {
                    assert_eq!(called, 1);
                    Ok(response_builder
                        .header(CONTENT_LENGTH, HeaderValue::from_static("10"))
                        .body(body_2)
                        .build())
                } else {
                    Err(HttpResponseError::builder_with_msg(
                        HttpResponseErrorKind::InvalidUrl,
                        "called more than twice",
                    )
                    .build())
                }
            }
        }

        impl HttpCaller for CounterHttpCaller {
            fn call(&self, request: &mut SyncHttpRequest<'_>) -> SyncHttpResponseResult {
                self.handle(
                    &request.url().to_string(),
                    SyncHttpResponseBody::from_bytes(b"gateway timeout".to_vec()),
                    SyncHttpResponseBody::from_bytes(b"0123456789".to_vec()),
                )
            }

            #[cfg(feature = "async")]
            fn async_call<'a>(
                &'a self,
                request: &'a mut AsyncHttpRequest<'_>,
            ) -> BoxFuture<'a, AsyncHttpResponseResult> {
                Box::pin(async move {
                    self.handle(
                        &request.url().to_string(),
                        AsyncHttpResponseBody::from_bytes(b"gateway timeout".to_vec()),
                        AsyncHttpResponseBody::from_bytes(b"0123456789".to_vec()),
                    )
                })
            }
        }

        spawn_blocking(|| {
            let mut inner_reader = get_download_manager().download("test-key")?.into_inner_reader();
            let mut buf = [0u8; 1024];
            assert_eq!(inner_reader.read(&mut buf)?, 10);
            assert_eq!(inner_reader.read(&mut buf)?, 0);
            Ok::<_, anyhow::Error>(())
        })
        .await?;

        {
            let mut inner_reader = get_download_manager()
                .async_download("test-key")
                .await?
                .into_inner_reader();
            let mut buf = [0u8; 1024];
            assert_eq!(inner_reader.async_read(&mut buf).await?, 10);
            assert_eq!(inner_reader.async_read(&mut buf).await?, 0);
        }

        return Ok(());

        fn get_download_manager() -> DownloadManager {
            DownloadManager::builder(UrlsSigner::new(
                get_credential(),
                StaticDomainsUrlsGenerator::builder(Endpoint::new_from_ip_addr(IpAddr::V4(Ipv4Addr::new(
                    127, 0, 0, 1,
                ))))
                .add_domain(Endpoint::new_from_ip_addr(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2))))
                .build(),
            ))
            .http_client(
                HttpClient::builder(CounterHttpCaller(Default::default()))
                    .request_retrier(NeverRetrier)
                    .build(),
            )
            .build()
        }
    }

    #[async_std::test]
    async fn test_inner_reader_4xx_error() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct CounterHttpCaller(AtomicUsize);

        impl CounterHttpCaller {
            fn handle<B: Default>(&self, body: B) -> HttpResponseResult<B> {
                match self.0.fetch_add(1, Ordering::Relaxed) {
                    0 => Ok(HttpResponse::builder()
                        .status_code(StatusCode::NOT_FOUND)
                        .header("x-reqid", HeaderValue::from_static("fake-reqid"))
                        .header(CONTENT_LENGTH, HeaderValue::from_static("16"))
                        .header(ETAG, HeaderValue::from_static("fake-etag"))
                        .body(body)
                        .build()),
                    _ => Err(HttpResponseError::builder_with_msg(
                        HttpResponseErrorKind::InvalidUrl,
                        "called more than once",
                    )
                    .build()),
                }
            }
        }

        impl HttpCaller for CounterHttpCaller {
            fn call(&self, _request: &mut SyncHttpRequest<'_>) -> SyncHttpResponseResult {
                self.handle(SyncHttpResponseBody::from_bytes(
                    b"{\"error\":\"no such file or directory\"}".to_vec(),
                ))
            }

            #[cfg(feature = "async")]
            fn async_call<'a>(
                &'a self,
                _request: &'a mut AsyncHttpRequest<'_>,
            ) -> BoxFuture<'a, AsyncHttpResponseResult> {
                Box::pin(async move {
                    self.handle(AsyncHttpResponseBody::from_bytes(
                        b"{\"error\":\"no such file or directory\"}".to_vec(),
                    ))
                })
            }
        }

        spawn_blocking(|| {
            let mut inner_reader = get_download_manager().download("test-key")?.into_inner_reader();
            let mut buf = [0u8; 1024];
            let err = inner_reader.read(&mut buf).unwrap_err();
            match err {
                DownloadError::ResponseError(err)
                    if matches!(err.kind(), ResponseErrorKind::StatusCodeError(StatusCode::NOT_FOUND)) => {}
                _ => panic!("unexpected error: {:?}", err),
            }
            Ok::<_, anyhow::Error>(())
        })
        .await?;

        let mut inner_reader = get_download_manager()
            .async_download("test-key")
            .await?
            .into_inner_reader();
        let mut buf = [0u8; 1024];
        let err = inner_reader.async_read(&mut buf).await.unwrap_err();
        match err {
            DownloadError::ResponseError(err)
                if matches!(err.kind(), ResponseErrorKind::StatusCodeError(StatusCode::NOT_FOUND)) => {}
            _ => panic!("unexpected error: {:?}", err),
        }

        return Ok(());

        fn get_download_manager() -> DownloadManager {
            DownloadManager::builder(UrlsSigner::new(
                get_credential(),
                StaticDomainsUrlsGenerator::builder(Endpoint::new_from_ip_addr(IpAddr::V4(Ipv4Addr::new(
                    127, 0, 0, 1,
                ))))
                .add_domain(Endpoint::new_from_ip_addr(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2))))
                .build(),
            ))
            .http_client(
                HttpClient::builder(CounterHttpCaller(Default::default()))
                    .request_retrier(NeverRetrier)
                    .build(),
            )
            .build()
        }
    }

    #[async_std::test]
    async fn test_inner_reader_response_error() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct CounterHttpCaller(AtomicUsize);

        impl CounterHttpCaller {
            fn handle<B: Default>(&self, url: &str, body: B) -> HttpResponseResult<B> {
                let called = self.0.fetch_add(1, Ordering::Relaxed);
                if url.starts_with("http://127.0.0.1/test-key") {
                    assert_eq!(called, 0);
                    Err(
                        HttpResponseError::builder_with_msg(HttpResponseErrorKind::ConnectError, "fake connect error")
                            .build(),
                    )
                } else if url.starts_with("http://127.0.0.2/test-key") {
                    assert_eq!(called, 1);
                    Ok(HttpResponse::builder()
                        .header("x-reqid", HeaderValue::from_static("fake-reqid"))
                        .header(CONTENT_LENGTH, HeaderValue::from_static("10"))
                        .header(ETAG, HeaderValue::from_static("fake-etag"))
                        .body(body)
                        .build())
                } else {
                    Err(HttpResponseError::builder_with_msg(
                        HttpResponseErrorKind::InvalidUrl,
                        "called more than twice",
                    )
                    .build())
                }
            }
        }

        impl HttpCaller for CounterHttpCaller {
            fn call(&self, request: &mut SyncHttpRequest<'_>) -> SyncHttpResponseResult {
                self.handle(
                    &request.url().to_string(),
                    SyncHttpResponseBody::from_bytes(b"0123456789".to_vec()),
                )
            }

            #[cfg(feature = "async")]
            fn async_call<'a>(
                &'a self,
                request: &'a mut AsyncHttpRequest<'_>,
            ) -> BoxFuture<'a, AsyncHttpResponseResult> {
                Box::pin(async move {
                    self.handle(
                        &request.url().to_string(),
                        AsyncHttpResponseBody::from_bytes(b"0123456789".to_vec()),
                    )
                })
            }
        }

        spawn_blocking(|| {
            let mut inner_reader = get_download_manager().download("test-key")?.into_inner_reader();
            let mut buf = [0u8; 1024];
            assert_eq!(inner_reader.read(&mut buf)?, 10);
            assert_eq!(inner_reader.read(&mut buf)?, 0);
            Ok::<_, anyhow::Error>(())
        })
        .await?;

        {
            let mut inner_reader = get_download_manager()
                .async_download("test-key")
                .await?
                .into_inner_reader();
            let mut buf = [0u8; 1024];
            assert_eq!(inner_reader.async_read(&mut buf).await?, 10);
            assert_eq!(inner_reader.async_read(&mut buf).await?, 0);
        }

        return Ok(());

        fn get_download_manager() -> DownloadManager {
            DownloadManager::builder(UrlsSigner::new(
                get_credential(),
                StaticDomainsUrlsGenerator::builder(Endpoint::new_from_ip_addr(IpAddr::V4(Ipv4Addr::new(
                    127, 0, 0, 1,
                ))))
                .add_domain(Endpoint::new_from_ip_addr(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2))))
                .build(),
            ))
            .http_client(
                HttpClient::builder(CounterHttpCaller(Default::default()))
                    .request_retrier(NeverRetrier)
                    .build(),
            )
            .build()
        }
    }

    #[async_std::test]
    async fn test_inner_reader_content_changed() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct CounterHttpCaller(AtomicUsize);

        impl CounterHttpCaller {
            fn handle<B: Default>(&self, headers: &HeaderMap, body_1: B, body_2: B) -> HttpResponseResult<B> {
                let mut request_builder = HttpResponse::builder();
                request_builder.header("x-reqid", HeaderValue::from_static("fake-reqid"));
                match self.0.fetch_add(1, Ordering::Relaxed) {
                    0 => Ok(request_builder
                        .header(CONTENT_LENGTH, HeaderValue::from_static("10"))
                        .header(ETAG, HeaderValue::from_static("fake-etag"))
                        .body(body_1)
                        .build()),
                    1 => {
                        assert_eq!(headers.get(&RANGE).unwrap().to_str().unwrap(), "bytes=5-");
                        Ok(request_builder
                            .header(CONTENT_LENGTH, HeaderValue::from_static("5"))
                            .header(ETAG, HeaderValue::from_static("fake-etag-2"))
                            .body(body_2)
                            .build())
                    }
                    _ => Err(HttpResponseError::builder_with_msg(
                        HttpResponseErrorKind::InvalidUrl,
                        "called more than twice",
                    )
                    .build()),
                }
            }
        }

        impl HttpCaller for CounterHttpCaller {
            fn call(&self, request: &mut SyncHttpRequest<'_>) -> SyncHttpResponseResult {
                self.handle(
                    request.headers(),
                    SyncHttpResponseBody::from_bytes(b"12345".to_vec()),
                    SyncHttpResponseBody::from_bytes(b"67890".to_vec()),
                )
            }

            #[cfg(feature = "async")]
            fn async_call<'a>(
                &'a self,
                request: &'a mut AsyncHttpRequest<'_>,
            ) -> BoxFuture<'a, AsyncHttpResponseResult> {
                Box::pin(async move {
                    self.handle(
                        request.headers(),
                        AsyncHttpResponseBody::from_bytes(b"12345".to_vec()),
                        AsyncHttpResponseBody::from_bytes(b"67890".to_vec()),
                    )
                })
            }
        }

        spawn_blocking(|| {
            let mut inner_reader = get_download_manager().download("test-key")?.into_inner_reader();
            let mut buf = [0u8; 1024];
            assert_eq!(inner_reader.read(&mut buf)?, 5);
            match inner_reader.read(&mut buf).unwrap_err() {
                DownloadError::ContentChanged => (),
                err => panic!("unexpected error: {:?}", err),
            }
            Ok::<_, anyhow::Error>(())
        })
        .await?;

        {
            let mut inner_reader = get_download_manager()
                .async_download("test-key")
                .await?
                .into_inner_reader();
            let mut buf = [0u8; 1024];
            assert_eq!(inner_reader.async_read(&mut buf).await?, 5);
            match inner_reader.async_read(&mut buf).await.unwrap_err() {
                DownloadError::ContentChanged => (),
                err => panic!("unexpected error: {:?}", err),
            }
        }

        return Ok(());

        fn get_download_manager() -> DownloadManager {
            DownloadManager::builder(UrlsSigner::new(
                get_credential(),
                StaticDomainsUrlsGenerator::new(Endpoint::new_from_ip_addr(IpAddr::V4(Ipv4Addr::LOCALHOST))),
            ))
            .http_client(
                HttpClient::builder(CounterHttpCaller(Default::default()))
                    .request_retrier(NeverRetrier)
                    .build(),
            )
            .build()
        }
    }

    #[async_std::test]
    async fn test_reader_read() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let bytes = {
            let mut bytes = vec![0u8; (1 << 22) + 512];
            thread_rng().fill_bytes(&mut bytes);
            bytes
        };

        let expected = {
            let mut hasher = Sha1::new();
            hasher.update(&bytes);
            hasher.finalize()
        };

        #[derive(Debug, Default)]
        struct BigFileHttpCaller(Vec<u8>);

        impl BigFileHttpCaller {
            fn handle<B: Default>(&self, content_length: usize, body: B) -> HttpResponseResult<B> {
                Ok(HttpResponse::builder()
                    .header("x-reqid", HeaderValue::from_static("fake-reqid"))
                    .header(ETAG, HeaderValue::from_static("fake-etag"))
                    .header(
                        CONTENT_LENGTH,
                        HeaderValue::from_str(&content_length.to_string()).unwrap(),
                    )
                    .body(body)
                    .build())
            }
        }

        impl HttpCaller for BigFileHttpCaller {
            fn call(&self, _request: &mut SyncHttpRequest<'_>) -> SyncHttpResponseResult {
                self.handle(self.0.len(), SyncHttpResponseBody::from_bytes(self.0.to_owned()))
            }

            #[cfg(feature = "async")]
            fn async_call<'a>(
                &'a self,
                _request: &'a mut AsyncHttpRequest<'_>,
            ) -> BoxFuture<'a, AsyncHttpResponseResult> {
                Box::pin(async move { self.handle(self.0.len(), AsyncHttpResponseBody::from_bytes(self.0.to_owned())) })
            }
        }

        spawn_blocking({
            let expected = expected.to_owned();
            let bytes = bytes.to_owned();
            move || {
                let mut reader = get_download_manager(bytes).download("test-key")?.into_read();
                let mut hasher = Sha1::new();
                copy(&mut reader, &mut hasher)?;
                let actual = hasher.finalize();
                assert_eq!(actual, expected);
                Ok::<_, anyhow::Error>(())
            }
        })
        .await?;

        {
            let mut reader = get_download_manager(bytes)
                .async_download("test-key")
                .await?
                .into_async_read();
            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).await?;
            let mut hasher = Sha1::new();
            hasher.update(&buf);
            let actual = hasher.finalize();
            assert_eq!(actual, expected);
        }

        return Ok(());

        fn get_download_manager(bytes: Vec<u8>) -> DownloadManager {
            DownloadManager::builder(UrlsSigner::new(
                get_credential(),
                StaticDomainsUrlsGenerator::new(Endpoint::new_from_ip_addr(IpAddr::V4(Ipv4Addr::LOCALHOST))),
            ))
            .http_client(
                HttpClient::builder(BigFileHttpCaller(bytes))
                    .request_retrier(NeverRetrier)
                    .build(),
            )
            .build()
        }
    }

    fn get_credential() -> Credential {
        Credential::new("ak", "sk")
    }
}
