// THIS FILE IS GENERATED BY api-generator, DO NOT EDIT DIRECTLY!
//
#[derive(Debug, Clone, Default)]
#[doc = "调用 API 所用的 URL 查询参数"]
pub struct QueryParams<'a> {
    map: indexmap::IndexMap<qiniu_http_client::QueryPairKey<'a>, qiniu_http_client::QueryPairValue<'a>>,
}
impl<'a> QueryParams<'a> {
    #[inline]
    #[must_use]
    #[doc = "插入一个新的查询参数对"]
    pub fn insert(
        mut self,
        query_pair_key: qiniu_http_client::QueryPairKey<'a>,
        query_pair_value: qiniu_http_client::QueryPairValue<'a>,
    ) -> Self {
        self.map.insert(query_pair_key, query_pair_value);
        self
    }
    fn build(self) -> Vec<qiniu_http_client::QueryPair<'a>> {
        Vec::from_iter(self.map)
    }
}
impl<'a> From<QueryParams<'a>> for Vec<qiniu_http_client::QueryPair<'a>> {
    #[inline]
    fn from(map: QueryParams<'a>) -> Self {
        map.build()
    }
}
impl<'a> QueryParams<'a> {
    #[inline]
    #[must_use]
    #[doc = "指定存储空间"]
    pub fn set_bucket_as_str(self, value: impl Into<qiniu_http_client::QueryPairValue<'a>>) -> Self {
        self.insert("bucket".into(), value.into())
    }
    #[inline]
    #[must_use]
    #[doc = "上一次列举返回的位置标记，作为本次列举的起点信息"]
    pub fn set_marker_as_str(self, value: impl Into<qiniu_http_client::QueryPairValue<'a>>) -> Self {
        self.insert("marker".into(), value.into())
    }
    #[inline]
    #[must_use]
    #[doc = "本次列举的条目数，范围为 1-1000"]
    pub fn set_limit_as_i8(self, value: i8) -> Self {
        self.insert("limit".into(), value.to_string().into())
    }
    #[inline]
    #[must_use]
    #[doc = "本次列举的条目数，范围为 1-1000"]
    pub fn set_limit_as_i16(self, value: i16) -> Self {
        self.insert("limit".into(), value.to_string().into())
    }
    #[inline]
    #[must_use]
    #[doc = "本次列举的条目数，范围为 1-1000"]
    pub fn set_limit_as_i32(self, value: i32) -> Self {
        self.insert("limit".into(), value.to_string().into())
    }
    #[inline]
    #[must_use]
    #[doc = "本次列举的条目数，范围为 1-1000"]
    pub fn set_limit_as_i64(self, value: i64) -> Self {
        self.insert("limit".into(), value.to_string().into())
    }
    #[inline]
    #[must_use]
    #[doc = "本次列举的条目数，范围为 1-1000"]
    pub fn set_limit_as_isize(self, value: isize) -> Self {
        self.insert("limit".into(), value.to_string().into())
    }
    #[inline]
    #[must_use]
    #[doc = "本次列举的条目数，范围为 1-1000"]
    pub fn set_limit_as_u8(self, value: u8) -> Self {
        self.insert("limit".into(), value.to_string().into())
    }
    #[inline]
    #[must_use]
    #[doc = "本次列举的条目数，范围为 1-1000"]
    pub fn set_limit_as_u16(self, value: u16) -> Self {
        self.insert("limit".into(), value.to_string().into())
    }
    #[inline]
    #[must_use]
    #[doc = "本次列举的条目数，范围为 1-1000"]
    pub fn set_limit_as_u32(self, value: u32) -> Self {
        self.insert("limit".into(), value.to_string().into())
    }
    #[inline]
    #[must_use]
    #[doc = "本次列举的条目数，范围为 1-1000"]
    pub fn set_limit_as_u64(self, value: u64) -> Self {
        self.insert("limit".into(), value.to_string().into())
    }
    #[inline]
    #[must_use]
    #[doc = "本次列举的条目数，范围为 1-1000"]
    pub fn set_limit_as_usize(self, value: usize) -> Self {
        self.insert("limit".into(), value.to_string().into())
    }
    #[inline]
    #[must_use]
    #[doc = "指定前缀，只有资源名匹配该前缀的资源会被列出"]
    pub fn set_prefix_as_str(self, value: impl Into<qiniu_http_client::QueryPairValue<'a>>) -> Self {
        self.insert("prefix".into(), value.into())
    }
    #[inline]
    #[must_use]
    #[doc = "指定目录分隔符，列出所有公共前缀（模拟列出目录效果）"]
    pub fn set_delimiter_as_str(self, value: impl Into<qiniu_http_client::QueryPairValue<'a>>) -> Self {
        self.insert("delimiter".into(), value.into())
    }
    #[inline]
    #[must_use]
    #[doc = "如果文件是通过分片上传的，是否返回对应的分片信息"]
    pub fn set_need_parts_as_bool(self, value: bool) -> Self {
        self.insert("needparts".into(), value.to_string().into())
    }
}
#[doc = "API 调用客户端"]
#[derive(Debug, Clone)]
pub struct Client<'client>(&'client qiniu_http_client::HttpClient);
impl<'client> Client<'client> {
    pub(super) fn new(http_client: &'client qiniu_http_client::HttpClient) -> Self {
        Self(http_client)
    }
}
impl<'client> Client<'client> {
    #[inline]
    #[doc = "创建一个新的阻塞请求，该方法的异步版本为 [`Self::new_async_request`]"]
    pub fn new_request<E: qiniu_http_client::EndpointsProvider + 'client>(
        &self,
        endpoints_provider: E,
        credential: impl qiniu_http_client::credential::CredentialProvider + Clone + 'client,
    ) -> SyncRequestBuilder<'client, E> {
        RequestBuilder({
            let mut builder = self.0.get(&[qiniu_http_client::ServiceName::Rsf], endpoints_provider);
            builder.authorization(qiniu_http_client::Authorization::v2(credential));
            builder.idempotent(qiniu_http_client::Idempotent::Default);
            builder.path("v2/list");
            builder.accept_application_octet_stream();
            builder
        })
    }
    #[inline]
    #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
    #[doc = "创建一个新的异步请求"]
    pub fn new_async_request<E: qiniu_http_client::EndpointsProvider + 'client>(
        &self,
        endpoints_provider: E,
        credential: impl qiniu_http_client::credential::CredentialProvider + Clone + 'client,
    ) -> AsyncRequestBuilder<'client, E> {
        RequestBuilder({
            let mut builder = self
                .0
                .async_get(&[qiniu_http_client::ServiceName::Rsf], endpoints_provider);
            builder.authorization(qiniu_http_client::Authorization::v2(credential));
            builder.idempotent(qiniu_http_client::Idempotent::Default);
            builder.path("v2/list");
            builder.accept_application_octet_stream();
            builder
        })
    }
}
#[derive(Debug)]
#[doc = "API 请求构造器"]
pub struct RequestBuilder<'req, B, E>(qiniu_http_client::RequestBuilder<'req, B, E>);
#[doc = "API 阻塞请求构造器"]
pub type SyncRequestBuilder<'req, E> = RequestBuilder<'req, qiniu_http_client::SyncRequestBody<'req>, E>;
#[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
#[doc = "API 异步请求构造器"]
pub type AsyncRequestBuilder<'req, E> = RequestBuilder<'req, qiniu_http_client::AsyncRequestBody<'req>, E>;
impl<'req, B, E> RequestBuilder<'req, B, E> {
    #[inline]
    #[doc = "设置是否使用 HTTPS"]
    pub fn use_https(&mut self, use_https: bool) -> &mut Self {
        self.0.use_https(use_https);
        self
    }
    #[inline]
    #[doc = "设置 HTTP 协议版本"]
    pub fn version(&mut self, version: qiniu_http_client::http::Version) -> &mut Self {
        self.0.version(version);
        self
    }
    #[inline]
    #[doc = "设置 HTTP 请求头"]
    pub fn headers(
        &mut self,
        headers: impl Into<std::borrow::Cow<'req, qiniu_http_client::http::HeaderMap>>,
    ) -> &mut Self {
        self.0.headers(headers);
        self
    }
    #[inline]
    #[doc = "添加 HTTP 请求头"]
    pub fn set_header(
        &mut self,
        header_name: impl qiniu_http_client::http::header::IntoHeaderName,
        header_value: impl Into<qiniu_http_client::http::HeaderValue>,
    ) -> &mut Self {
        self.0.set_header(header_name, header_value);
        self
    }
    #[inline]
    #[doc = "设置查询参数"]
    pub fn query(&mut self, query: impl Into<std::borrow::Cow<'req, str>>) -> &mut Self {
        self.0.query(query);
        self
    }
    #[inline]
    #[doc = "设置查询参数"]
    pub fn query_pairs(&mut self, query_pairs: impl Into<Vec<qiniu_http_client::QueryPair<'req>>>) -> &mut Self {
        self.0.query_pairs(query_pairs);
        self
    }
    #[inline]
    #[doc = "追加查询参数"]
    pub fn append_query_pair(
        &mut self,
        query_pair_key: impl Into<qiniu_http_client::QueryPairKey<'req>>,
        query_pair_value: impl Into<qiniu_http_client::QueryPairValue<'req>>,
    ) -> &mut Self {
        self.0.append_query_pair(query_pair_key, query_pair_value);
        self
    }
    #[inline]
    #[doc = "设置扩展信息"]
    pub fn extensions(&mut self, extensions: qiniu_http_client::http::Extensions) -> &mut Self {
        self.0.extensions(extensions);
        self
    }
    #[doc = "添加扩展信息"]
    #[inline]
    pub fn add_extension<T: Send + Sync + 'static>(&mut self, val: T) -> &mut Self {
        self.0.add_extension(val);
        self
    }
    #[inline]
    #[doc = "上传进度回调函数"]
    pub fn on_uploading_progress(
        &mut self,
        callback: impl Fn(
                &dyn qiniu_http_client::SimplifiedCallbackContext,
                qiniu_http_client::http::TransferProgressInfo,
            ) -> anyhow::Result<()>
            + Send
            + Sync
            + 'req,
    ) -> &mut Self {
        self.0.on_uploading_progress(callback);
        self
    }
    #[inline]
    #[doc = "设置响应状态码回调函数"]
    pub fn on_receive_response_status(
        &mut self,
        callback: impl Fn(
                &dyn qiniu_http_client::SimplifiedCallbackContext,
                qiniu_http_client::http::StatusCode,
            ) -> anyhow::Result<()>
            + Send
            + Sync
            + 'req,
    ) -> &mut Self {
        self.0.on_receive_response_status(callback);
        self
    }
    #[inline]
    #[doc = "设置响应 HTTP 头回调函数"]
    pub fn on_receive_response_header(
        &mut self,
        callback: impl Fn(
                &dyn qiniu_http_client::SimplifiedCallbackContext,
                &qiniu_http_client::http::HeaderName,
                &qiniu_http_client::http::HeaderValue,
            ) -> anyhow::Result<()>
            + Send
            + Sync
            + 'req,
    ) -> &mut Self {
        self.0.on_receive_response_header(callback);
        self
    }
    #[inline]
    #[doc = "设置域名解析前回调函数"]
    pub fn on_to_resolve_domain(
        &mut self,
        callback: impl Fn(&mut dyn qiniu_http_client::CallbackContext, &str) -> anyhow::Result<()> + Send + Sync + 'req,
    ) -> &mut Self {
        self.0.on_to_resolve_domain(callback);
        self
    }
    #[inline]
    #[doc = "设置域名解析成功回调函数"]
    pub fn on_domain_resolved(
        &mut self,
        callback: impl Fn(
                &mut dyn qiniu_http_client::CallbackContext,
                &str,
                &qiniu_http_client::ResolveAnswers,
            ) -> anyhow::Result<()>
            + Send
            + Sync
            + 'req,
    ) -> &mut Self {
        self.0.on_domain_resolved(callback);
        self
    }
    #[inline]
    #[doc = "设置 IP 地址选择前回调函数"]
    pub fn on_to_choose_ips(
        &mut self,
        callback: impl Fn(&mut dyn qiniu_http_client::CallbackContext, &[qiniu_http_client::IpAddrWithPort]) -> anyhow::Result<()>
            + Send
            + Sync
            + 'req,
    ) -> &mut Self {
        self.0.on_to_choose_ips(callback);
        self
    }
    #[inline]
    #[doc = "设置 IP 地址选择成功回调函数"]
    pub fn on_ips_chosen(
        &mut self,
        callback: impl Fn(
                &mut dyn qiniu_http_client::CallbackContext,
                &[qiniu_http_client::IpAddrWithPort],
                &[qiniu_http_client::IpAddrWithPort],
            ) -> anyhow::Result<()>
            + Send
            + Sync
            + 'req,
    ) -> &mut Self {
        self.0.on_ips_chosen(callback);
        self
    }
    #[inline]
    #[doc = "设置 HTTP 请求签名前回调函数"]
    pub fn on_before_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn qiniu_http_client::ExtendedCallbackContext) -> anyhow::Result<()> + Send + Sync + 'req,
    ) -> &mut Self {
        self.0.on_before_request_signed(callback);
        self
    }
    #[inline]
    #[doc = "设置 HTTP 请求前回调函数"]
    pub fn on_after_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn qiniu_http_client::ExtendedCallbackContext) -> anyhow::Result<()> + Send + Sync + 'req,
    ) -> &mut Self {
        self.0.on_after_request_signed(callback);
        self
    }
    #[inline]
    #[doc = "设置响应成功回调函数"]
    pub fn on_response(
        &mut self,
        callback: impl Fn(
                &mut dyn qiniu_http_client::ExtendedCallbackContext,
                &qiniu_http_client::http::ResponseParts,
            ) -> anyhow::Result<()>
            + Send
            + Sync
            + 'req,
    ) -> &mut Self {
        self.0.on_response(callback);
        self
    }
    #[inline]
    #[doc = "设置响应错误回调函数"]
    pub fn on_error(
        &mut self,
        callback: impl Fn(
                &mut dyn qiniu_http_client::ExtendedCallbackContext,
                &mut qiniu_http_client::ResponseError,
            ) -> anyhow::Result<()>
            + Send
            + Sync
            + 'req,
    ) -> &mut Self {
        self.0.on_error(callback);
        self
    }
    #[inline]
    #[doc = "设置退避前回调函数"]
    pub fn on_before_backoff(
        &mut self,
        callback: impl Fn(&mut dyn qiniu_http_client::ExtendedCallbackContext, std::time::Duration) -> anyhow::Result<()>
            + Send
            + Sync
            + 'req,
    ) -> &mut Self {
        self.0.on_before_backoff(callback);
        self
    }
    #[inline]
    #[doc = "设置退避后回调函数"]
    pub fn on_after_backoff(
        &mut self,
        callback: impl Fn(&mut dyn qiniu_http_client::ExtendedCallbackContext, std::time::Duration) -> anyhow::Result<()>
            + Send
            + Sync
            + 'req,
    ) -> &mut Self {
        self.0.on_after_backoff(callback);
        self
    }
    #[inline]
    #[doc = "获取 HTTP 请求构建器部分参数"]
    pub fn parts(&self) -> &qiniu_http_client::RequestBuilderParts<'req> {
        self.0.parts()
    }
    #[inline]
    #[doc = "获取 HTTP 请求构建器部分参数的可变引用"]
    pub fn parts_mut(&mut self) -> &mut qiniu_http_client::RequestBuilderParts<'req> {
        self.0.parts_mut()
    }
}
impl<'req, E: qiniu_http_client::EndpointsProvider + Clone + 'req> SyncRequestBuilder<'req, E> {
    #[doc = "阻塞发起 HTTP 请求"]
    pub fn call(
        &mut self,
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<qiniu_http_client::SyncResponseBody>> {
        let request = &mut self.0;
        let response = request.call()?;
        let parsed = response;
        Ok(parsed)
    }
}
#[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
impl<'req, E: qiniu_http_client::EndpointsProvider + Clone + 'req> AsyncRequestBuilder<'req, E> {
    #[doc = "异步发起 HTTP 请求"]
    pub async fn call(
        &mut self,
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<qiniu_http_client::AsyncResponseBody>> {
        let request = &mut self.0;
        let response = request.call().await?;
        let parsed = response;
        Ok(parsed)
    }
}
