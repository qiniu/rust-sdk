use super::{
    super::{
        super::{EndpointsProvider, IpAddrWithPort, ServiceName},
        callbacks::CallbacksBuilder,
        request_call, ApiResult, Authorization, CallbackContext, ExtendedCallbackContext, HttpClient, ResolveAnswers,
        ResponseError, SimplifiedCallbackContext, SyncResponse,
    },
    multipart::SyncMultipart,
    request_metadata::RequestMetadata,
    Idempotent, QueryPair, QueryPairKey, QueryPairValue, SyncInnerRequest,
};
use anyhow::Result as AnyResult;
use assert_impl::assert_impl;
use mime::{Mime, APPLICATION_JSON, APPLICATION_OCTET_STREAM, APPLICATION_WWW_FORM_URLENCODED};
use qiniu_http::{
    header::{IntoHeaderName, ACCEPT, CONTENT_TYPE},
    Extensions, HeaderMap, HeaderName, HeaderValue, Method, Reset, ResponseParts, StatusCode, SyncRequestBody,
    TransferProgressInfo, UserAgent, Version,
};
use serde::Serialize;
use serde_json::Result as JsonResult;
use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    io::{Read, Result as IoResult},
    mem::take,
    time::Duration,
};

#[cfg(feature = "async")]
use {
    super::{
        super::{async_request_call, AsyncResponse},
        multipart::AsyncMultipart,
        AsyncInnerRequest,
    },
    futures::io::AsyncRead,
    qiniu_http::{AsyncRequestBody, AsyncReset},
};

/// HTTP 请求构建器部分参数
///
/// 包含 HTTP 请求构建器内除请求体和终端地址提供者以外的参数
#[derive(Default, Debug)]
pub struct RequestBuilderParts<'r> {
    callbacks: CallbacksBuilder<'r>,
    metadata: RequestMetadata<'r>,
    extensions: Extensions,
    appended_user_agent: UserAgent,
}

impl<'r> RequestBuilderParts<'r> {
    /// 设置是否使用 HTTPS
    #[inline]
    pub fn use_https(&mut self, use_https: bool) -> &mut Self {
        self.metadata.use_https = Some(use_https);
        self
    }

    /// 设置 HTTP 协议版本
    #[inline]
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.metadata.version = version;
        self
    }

    /// 设置 HTTP 请求路径
    #[inline]
    pub fn path(&mut self, path: impl Into<Cow<'r, str>>) -> &mut Self {
        self.metadata.path = path.into();
        self
    }

    /// 设置 HTTP 请求头
    #[inline]
    pub fn headers(&mut self, headers: impl Into<Cow<'r, HeaderMap>>) -> &mut Self {
        self.metadata.headers = headers.into();
        self
    }

    /// 添加 HTTP 请求头
    #[inline]
    pub fn set_header(&mut self, header_name: impl IntoHeaderName, header_value: impl Into<HeaderValue>) -> &mut Self {
        self.metadata.headers.to_mut().insert(header_name, header_value.into());
        self
    }

    fn set_content_type(&mut self, content_type: Option<Mime>) -> &mut Self {
        self.set_header(
            CONTENT_TYPE,
            HeaderValue::from_str(content_type.as_ref().unwrap_or(&APPLICATION_OCTET_STREAM).as_ref()).unwrap(),
        )
    }

    /// 设置 HTTP 响应预期为 JSON 类型
    #[inline]
    pub fn accept_json(&mut self) -> &mut Self {
        self.set_accept(APPLICATION_JSON)
    }

    /// 设置 HTTP 响应预期为二进制流类型
    #[inline]
    pub fn accept_application_octet_stream(&mut self) -> &mut Self {
        self.set_accept(APPLICATION_OCTET_STREAM)
    }

    fn set_accept(&mut self, accept: Mime) -> &mut Self {
        self.set_header(ACCEPT, HeaderValue::from_str(accept.as_ref()).unwrap())
    }

    /// 设置查询参数
    #[inline]
    pub fn query(&mut self, query: impl Into<Cow<'r, str>>) -> &mut Self {
        self.metadata.query = query.into();
        self
    }

    /// 设置查询参数
    #[inline]
    pub fn query_pairs(&mut self, query_pairs: impl Into<Vec<QueryPair<'r>>>) -> &mut Self {
        self.metadata.query_pairs = query_pairs.into();
        self
    }

    /// 追加查询参数
    #[inline]
    pub fn append_query_pair(
        &mut self,
        query_pair_key: impl Into<QueryPairKey<'r>>,
        query_pair_value: impl Into<QueryPairValue<'r>>,
    ) -> &mut Self {
        self.metadata
            .query_pairs
            .push((query_pair_key.into(), query_pair_value.into()));
        self
    }

    /// 追加 UserAgent
    #[inline]
    pub fn appended_user_agent(&mut self, user_agent: impl Into<UserAgent>) -> &mut Self {
        self.appended_user_agent = user_agent.into();
        self
    }

    /// 设置鉴权签名
    #[inline]
    pub fn authorization(&mut self, authorization: Authorization<'r>) -> &mut Self {
        self.metadata.authorization = Some(authorization);
        self
    }

    /// 设置为幂等请求
    #[inline]
    pub fn idempotent(&mut self, idempotent: Idempotent) -> &mut Self {
        self.metadata.idempotent = idempotent;
        self
    }

    /// 设置扩展信息
    #[inline]
    pub fn extensions(&mut self, extensions: Extensions) -> &mut Self {
        self.extensions = extensions;
        self
    }

    /// 添加扩展信息
    #[inline]
    pub fn add_extension<T: Send + Sync + 'static>(&mut self, val: T) -> &mut Self {
        self.extensions.insert(val);
        self
    }

    /// 设置上传进度回调函数
    #[inline]
    pub fn on_uploading_progress(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, TransferProgressInfo<'_>) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_uploading_progress(callback);
        self
    }

    /// 设置响应状态码回调函数
    #[inline]

    pub fn on_receive_response_status(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, StatusCode) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_receive_response_status(callback);
        self
    }

    /// 设置响应 HTTP 头回调函数
    #[inline]
    pub fn on_receive_response_header(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, &HeaderName, &HeaderValue) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_receive_response_header(callback);
        self
    }

    /// 设置域名解析前回调函数
    #[inline]
    pub fn on_to_resolve_domain(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_to_resolve_domain(callback);
        self
    }

    /// 设置域名解析成功回调函数
    #[inline]
    pub fn on_domain_resolved(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str, &ResolveAnswers) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_domain_resolved(callback);
        self
    }

    /// 设置 IP 地址选择前回调函数
    #[inline]
    pub fn on_to_choose_ips(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort]) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_to_choose_ips(callback);
        self
    }

    /// 设置 IP 地址选择成功回调函数
    #[inline]
    pub fn on_ips_chosen(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort], &[IpAddrWithPort]) -> AnyResult<()>
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.callbacks.on_ips_chosen(callback);
        self
    }

    /// 设置 HTTP 请求签名前回调函数
    #[inline]
    pub fn on_before_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_before_request_signed(callback);
        self
    }

    /// 设置 HTTP 请求前回调函数
    #[inline]
    pub fn on_after_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_after_request_signed(callback);
        self
    }

    /// 设置响应成功回调函数
    #[inline]
    pub fn on_response(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseParts) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_response(callback);
        self
    }

    /// 设置响应错误回调函数
    #[inline]
    pub fn on_error(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseError) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_error(callback);
        self
    }

    /// 设置退避前回调函数
    #[inline]
    pub fn on_before_backoff(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_before_backoff(callback);
        self
    }

    /// 设置退避后回调函数
    #[inline]
    pub fn on_after_backoff(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_after_backoff(callback);
        self
    }

    /// 构建为请求部分参数
    #[inline]
    pub fn build(self) -> RequestParts<'r> {
        RequestParts {
            metadata: self.metadata,
            extensions: self.extensions,
            appended_user_agent: self.appended_user_agent,
        }
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// 请求构建器
///
/// 通过 [`HttpClient::get`]， [`HttpClient::post`] 等方法创建请求构建器
#[derive(Debug)]
pub struct RequestBuilder<'r, B: 'r, E: 'r> {
    http_client: &'r HttpClient,
    service_names: &'r [ServiceName],
    endpoints_provider: E,
    parts: RequestBuilderParts<'r>,
    body: B,
}

impl<'r, B: Default + 'r, E: EndpointsProvider + 'r> RequestBuilder<'r, B, E> {
    pub(in super::super) fn new(
        http_client: &'r HttpClient,
        method: Method,
        endpoints_provider: E,
        service_names: &'r [ServiceName],
    ) -> Self {
        Self {
            http_client,
            service_names,
            endpoints_provider,
            parts: RequestBuilderParts {
                metadata: RequestMetadata {
                    method,
                    ..Default::default()
                },
                ..Default::default()
            },
            body: Default::default(),
        }
    }
}

impl<'r, B: 'r, E: 'r> RequestBuilder<'r, B, E> {
    /// 设置是否使用 HTTPS
    #[inline]
    pub fn use_https(&mut self, use_https: bool) -> &mut Self {
        self.parts.use_https(use_https);
        self
    }

    /// 设置 HTTP 协议版本
    #[inline]
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.parts.version(version);
        self
    }

    /// 设置 HTTP 请求路径
    #[inline]
    pub fn path(&mut self, path: impl Into<Cow<'r, str>>) -> &mut Self {
        self.parts.path(path);
        self
    }

    /// 设置 HTTP 请求头
    #[inline]
    pub fn headers(&mut self, headers: impl Into<Cow<'r, HeaderMap>>) -> &mut Self {
        self.parts.headers(headers);
        self
    }

    /// 添加 HTTP 请求头
    #[inline]
    pub fn set_header(&mut self, header_name: impl IntoHeaderName, header_value: impl Into<HeaderValue>) -> &mut Self {
        self.parts.set_header(header_name, header_value);
        self
    }

    /// 设置 HTTP 响应预期为 JSON 类型
    #[inline]
    pub fn accept_json(&mut self) -> &mut Self {
        self.parts.accept_json();
        self
    }

    /// 设置 HTTP 响应预期为二进制流类型
    #[inline]
    pub fn accept_application_octet_stream(&mut self) -> &mut Self {
        self.parts.accept_application_octet_stream();
        self
    }

    /// 设置查询参数
    #[inline]
    pub fn query(&mut self, query: impl Into<Cow<'r, str>>) -> &mut Self {
        self.parts.query(query);
        self
    }

    /// 设置查询参数
    #[inline]
    pub fn query_pairs(&mut self, query_pairs: impl Into<Vec<QueryPair<'r>>>) -> &mut Self {
        self.parts.query_pairs(query_pairs);
        self
    }

    /// 追加查询参数
    #[inline]
    pub fn append_query_pair(
        &mut self,
        query_pair_key: impl Into<QueryPairKey<'r>>,
        query_pair_value: impl Into<QueryPairValue<'r>>,
    ) -> &mut Self {
        self.parts.append_query_pair(query_pair_key, query_pair_value);
        self
    }

    /// 追加 UserAgent
    #[inline]
    pub fn appended_user_agent(&mut self, user_agent: impl Into<UserAgent>) -> &mut Self {
        self.parts.appended_user_agent(user_agent);
        self
    }

    /// 设置鉴权签名
    #[inline]
    pub fn authorization(&mut self, authorization: Authorization<'r>) -> &mut Self {
        self.parts.authorization(authorization);
        self
    }

    /// 设置为幂等请求
    #[inline]
    pub fn idempotent(&mut self, idempotent: Idempotent) -> &mut Self {
        self.parts.idempotent(idempotent);
        self
    }

    /// 设置扩展信息
    #[inline]
    pub fn extensions(&mut self, extensions: Extensions) -> &mut Self {
        self.parts.extensions(extensions);
        self
    }

    /// 添加扩展信息
    #[inline]
    pub fn add_extension<T: Send + Sync + 'static>(&mut self, val: T) -> &mut Self {
        self.parts.add_extension(val);
        self
    }

    /// 设置上传进度回调函数
    #[inline]
    pub fn on_uploading_progress(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, TransferProgressInfo<'_>) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_uploading_progress(callback);
        self
    }

    /// 设置响应状态码回调函数
    #[inline]
    pub fn on_receive_response_status(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, StatusCode) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_receive_response_status(callback);
        self
    }

    /// 设置响应 HTTP 头回调函数
    #[inline]
    pub fn on_receive_response_header(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, &HeaderName, &HeaderValue) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_receive_response_header(callback);
        self
    }

    /// 设置域名解析前回调函数
    #[inline]
    pub fn on_to_resolve_domain(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_to_resolve_domain(callback);
        self
    }

    /// 设置域名解析成功回调函数
    #[inline]
    pub fn on_domain_resolved(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str, &ResolveAnswers) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_domain_resolved(callback);
        self
    }

    /// 设置 IP 地址选择前回调函数
    #[inline]
    pub fn on_to_choose_ips(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort]) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_to_choose_ips(callback);
        self
    }

    /// 设置 IP 地址选择成功回调函数
    #[inline]
    pub fn on_ips_chosen(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort], &[IpAddrWithPort]) -> AnyResult<()>
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.parts.on_ips_chosen(callback);
        self
    }

    /// 设置 HTTP 请求签名前回调函数
    #[inline]
    pub fn on_before_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_before_request_signed(callback);
        self
    }

    /// 设置 HTTP 请求前回调函数
    #[inline]
    pub fn on_after_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_after_request_signed(callback);
        self
    }

    /// 设置响应成功回调函数
    #[inline]
    pub fn on_response(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseParts) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_response(callback);
        self
    }

    /// 设置响应错误回调函数
    #[inline]
    pub fn on_error(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseError) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_error(callback);
        self
    }

    /// 设置退避前回调函数
    #[inline]
    pub fn on_before_backoff(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_before_backoff(callback);
        self
    }

    /// 设置退避后回调函数
    #[inline]
    pub fn on_after_backoff(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> AnyResult<()> + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_after_backoff(callback);
        self
    }

    /// 获取 HTTP 请求构建器部分参数
    #[inline]
    pub fn parts(&self) -> &RequestBuilderParts<'r> {
        &self.parts
    }

    /// 获取 HTTP 请求构建器部分参数的可变引用
    #[inline]
    pub fn parts_mut(&mut self) -> &mut RequestBuilderParts<'r> {
        &mut self.parts
    }

    /// 转换为 HTTP 请求构建器部分参数
    #[inline]
    pub fn into_parts(self) -> RequestBuilderParts<'r> {
        self.parts
    }

    fn get_appended_user_agent(&self) -> UserAgent {
        let mut appended_user_agent = self.http_client.appended_user_agent().to_owned();
        appended_user_agent.push_str(self.parts.appended_user_agent.as_str());
        appended_user_agent
    }
}

impl<'r, B: Sync + Send + 'r, E: Sync + Send + 'r> RequestBuilder<'r, B, E> {
    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// 阻塞请求构建器
pub type SyncRequestBuilder<'r, E> = RequestBuilder<'r, SyncRequestBody<'r>, E>;

impl<'r, E: 'r> SyncRequestBuilder<'r, E> {
    /// 设置 HTTP 请求体为输入流
    #[inline]
    pub fn stream_as_body(
        &mut self,
        body: impl Read + Reset + Debug + Send + Sync + 'static,
        content_length: u64,
        content_type: Option<Mime>,
    ) -> &mut Self {
        self.body = SyncRequestBody::from_reader(body, content_length);
        self.parts.set_content_type(content_type);
        self
    }

    /// 设置 HTTP 请求体为输入流的可变引用
    #[inline]
    pub fn referenced_stream_as_body<T: Read + Reset + Debug + Send + Sync>(
        &mut self,
        body: &'r mut T,
        content_length: u64,
        content_type: Option<Mime>,
    ) -> &mut Self {
        self.body = SyncRequestBody::from_referenced_reader(body, content_length);
        self.parts.set_content_type(content_type);
        self
    }

    /// 设置 HTTP 请求体为内存数据
    #[inline]
    pub fn bytes_as_body(&mut self, body: impl Into<Vec<u8>>, content_type: Option<Mime>) -> &mut Self {
        self.body = SyncRequestBody::from_bytes(body.into());
        self.parts.set_content_type(content_type);
        self
    }

    /// 设置 HTTP 请求体为内存数据的引用
    #[inline]
    pub fn referenced_bytes_as_body(&mut self, body: &'r [u8], content_type: Option<Mime>) -> &mut Self {
        self.body = SyncRequestBody::from_referenced_bytes(body);
        self.parts.set_content_type(content_type);
        self
    }

    /// 设置 HTTP 请求体为 JSON 对象
    #[inline]
    pub fn json(&mut self, body: impl Serialize) -> JsonResult<&mut Self> {
        Ok(self.bytes_as_body(serde_json::to_vec(&body)?, Some(APPLICATION_JSON)))
    }

    /// 设置 HTTP 请求体为表单对象
    #[inline]
    pub fn post_form<I, K, V>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: Borrow<(K, Option<V>)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut form = form_urlencoded::Serializer::new(String::new());
        for pair in iter {
            let (k, v) = pair.borrow();
            if let Some(v) = v {
                form.append_pair(k.as_ref(), v.as_ref());
            } else {
                form.append_key_only(k.as_ref());
            }
        }
        self.bytes_as_body(form.finish().into_bytes(), Some(APPLICATION_WWW_FORM_URLENCODED))
    }

    /// 设置 HTTP 请求体为 Multipart 表单对象
    #[inline]
    pub fn multipart<'a>(&mut self, multipart: impl Into<SyncMultipart<'a>>) -> IoResult<&mut Self> {
        let mut buf = Vec::new();
        let multipart = multipart.into();
        let mime = ("multipart/form-data; boundary=".to_owned() + multipart.boundary())
            .parse()
            .unwrap();
        multipart.into_read().read_to_end(&mut buf)?;
        Ok(self.bytes_as_body(buf, Some(mime)))
    }
}

impl<'r, E: EndpointsProvider + Clone + 'r> SyncRequestBuilder<'r, E> {
    /// 阻塞发起 HTTP 请求
    #[inline]
    pub fn call(&mut self) -> ApiResult<SyncResponse> {
        request_call(self.build())
    }

    pub(in super::super) fn build(&mut self) -> SyncInnerRequest<'r, E> {
        SyncInnerRequest::new(
            self.http_client,
            self.endpoints_provider.to_owned(),
            self.service_names,
            self.parts.callbacks.build(),
            take(&mut self.parts.metadata),
            take(&mut self.body),
            self.get_appended_user_agent(),
            take(&mut self.parts.extensions),
        )
    }
}

/// 异步请求构建器
#[cfg(feature = "async")]
pub type AsyncRequestBuilder<'r, E> = RequestBuilder<'r, AsyncRequestBody<'r>, E>;

#[cfg(feature = "async")]
impl<'r, E: 'r> AsyncRequestBuilder<'r, E> {
    /// 设置 HTTP 请求体为异步输入流
    #[inline]
    pub fn stream_as_body(
        &mut self,
        body: impl AsyncRead + AsyncReset + Unpin + Debug + Send + Sync + 'static,
        content_length: u64,
        content_type: Option<Mime>,
    ) -> &mut Self {
        self.body = AsyncRequestBody::from_reader(body, content_length);
        self.parts.set_content_type(content_type);
        self
    }

    /// 设置 HTTP 请求体为异步输入流的可变引用
    #[inline]
    pub fn referenced_stream_as_body<T: AsyncRead + AsyncReset + Unpin + Debug + Send + Sync>(
        &mut self,
        body: &'r mut T,
        content_length: u64,
        content_type: Option<Mime>,
    ) -> &mut Self {
        self.body = AsyncRequestBody::from_referenced_reader(body, content_length);
        self.parts.set_content_type(content_type);
        self
    }

    /// 设置 HTTP 请求体为内存数据
    #[inline]
    pub fn bytes_as_body(&mut self, body: impl Into<Vec<u8>>, content_type: Option<Mime>) -> &mut Self {
        self.body = AsyncRequestBody::from_bytes(body.into());
        self.parts.set_content_type(content_type);
        self
    }

    /// 设置 HTTP 请求体为内存数据的引用
    #[inline]
    pub fn referenced_bytes_as_body(&mut self, body: &'r [u8], content_type: Option<Mime>) -> &mut Self {
        self.body = AsyncRequestBody::from_referenced_bytes(body);
        self.parts.set_content_type(content_type);
        self
    }

    /// 设置 HTTP 请求体为 JSON 对象
    #[inline]
    pub fn json(&mut self, body: impl Serialize) -> JsonResult<&mut Self> {
        Ok(self.bytes_as_body(serde_json::to_vec(&body)?, Some(APPLICATION_JSON)))
    }

    /// 设置 HTTP 请求体为表单对象
    #[inline]
    pub fn post_form<I, K, V>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: Borrow<(K, Option<V>)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut form = form_urlencoded::Serializer::new(String::new());
        for pair in iter {
            let (k, v) = pair.borrow();
            if let Some(v) = v {
                form.append_pair(k.as_ref(), v.as_ref());
            } else {
                form.append_key_only(k.as_ref());
            }
        }
        self.bytes_as_body(form.finish().into_bytes(), Some(APPLICATION_WWW_FORM_URLENCODED))
    }

    /// 设置 HTTP 请求体为 Multipart 表单对象
    #[inline]
    pub async fn multipart<'a>(
        &mut self,
        multipart: impl Into<AsyncMultipart<'a>>,
    ) -> IoResult<&mut RequestBuilder<'r, AsyncRequestBody<'r>, E>> {
        use futures::AsyncReadExt;

        let mut buf = Vec::new();
        let multipart = multipart.into();
        let mime = ("multipart/form-data; boundary=".to_owned() + multipart.boundary())
            .parse()
            .unwrap();
        multipart.into_async_read().read_to_end(&mut buf).await?;
        Ok(self.bytes_as_body(buf, Some(mime)))
    }
}

#[cfg(feature = "async")]
impl<'r, E: EndpointsProvider + Clone + 'r> AsyncRequestBuilder<'r, E> {
    /// 异步发起 HTTP 请求
    #[inline]
    pub async fn call(&mut self) -> ApiResult<AsyncResponse> {
        async_request_call(self.build()).await
    }

    pub(in super::super) fn build(&mut self) -> AsyncInnerRequest<'r, E> {
        AsyncInnerRequest::new(
            self.http_client,
            self.endpoints_provider.to_owned(),
            self.service_names,
            self.parts.callbacks.build(),
            take(&mut self.parts.metadata),
            take(&mut self.body),
            self.get_appended_user_agent(),
            take(&mut self.parts.extensions),
        )
    }
}

/// HTTP 请求部分参数
///
/// 包含 HTTP 请求内除请求体和终端地址提供者以外的参数
#[derive(Default, Debug)]
pub struct RequestParts<'r> {
    metadata: RequestMetadata<'r>,
    extensions: Extensions,
    appended_user_agent: UserAgent,
}

impl CallbackContext for RequestParts<'_> {
    #[inline]
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    #[inline]
    fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}

impl SimplifiedCallbackContext for RequestParts<'_> {
    #[inline]
    fn use_https(&self) -> bool {
        self.metadata.use_https.unwrap_or(true)
    }

    #[inline]
    fn method(&self) -> &Method {
        &self.metadata.method
    }

    #[inline]
    fn version(&self) -> Version {
        self.metadata.version
    }

    #[inline]
    fn path(&self) -> &str {
        &self.metadata.path
    }

    #[inline]
    fn query(&self) -> &str {
        &self.metadata.query
    }

    #[inline]
    fn query_pairs(&self) -> &[QueryPair] {
        &self.metadata.query_pairs
    }

    #[inline]
    fn headers(&self) -> &HeaderMap {
        &self.metadata.headers
    }

    #[inline]
    fn appended_user_agent(&self) -> &UserAgent {
        &self.appended_user_agent
    }

    #[inline]
    fn authorization(&self) -> Option<&Authorization> {
        self.metadata.authorization.as_ref()
    }

    #[inline]
    fn idempotent(&self) -> Idempotent {
        self.metadata.idempotent
    }
}

impl RequestParts<'_> {
    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}
