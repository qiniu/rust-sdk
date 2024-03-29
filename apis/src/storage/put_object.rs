// THIS FILE IS GENERATED BY api-generator, DO NOT EDIT DIRECTLY!
//
#[doc = "阻塞 Multipart 表单"]
pub mod sync_part {
    #[derive(Debug, Default)]
    #[doc = "调用 API 所用的请求体参数"]
    pub struct RequestBody<'a> {
        multipart: qiniu_http_client::SyncMultipart<'a>,
    }
    impl<'a> RequestBody<'a> {
        #[inline]
        #[must_use]
        #[doc = "添加新的 Multipart 表单组件"]
        pub fn add_part(
            mut self,
            name: impl Into<qiniu_http_client::FieldName>,
            part: qiniu_http_client::SyncPart<'a>,
        ) -> Self {
            self.multipart = self.multipart.add_part(name.into(), part);
            self
        }
        fn build(self) -> qiniu_http_client::SyncMultipart<'a> {
            self.multipart
        }
    }
    impl<'a> From<RequestBody<'a>> for qiniu_http_client::SyncMultipart<'a> {
        #[inline]
        fn from(parts: RequestBody<'a>) -> Self {
            parts.build()
        }
    }
    impl<'a> RequestBody<'a> {
        #[inline]
        #[must_use]
        #[doc = "对象名称，如果不传入，则通过上传策略中的 `saveKey` 字段决定，如果 `saveKey` 也没有置顶，则使用对象的哈希值"]
        pub fn set_object_name(self, value: impl Into<std::borrow::Cow<'a, str>>) -> RequestBody<'a> {
            self.add_part("key", qiniu_http_client::SyncPart::text(value))
        }
        #[inline]
        #[doc = "上传凭证"]
        pub fn set_upload_token(
            self,
            token: &'a (dyn qiniu_http_client::upload_token::UploadTokenProvider + 'a),
            opts: qiniu_http_client::upload_token::ToStringOptions,
        ) -> qiniu_http_client::upload_token::ToStringResult<RequestBody<'a>> {
            Ok(self.add_part("token", qiniu_http_client::SyncPart::text(token.to_token_string(opts)?)))
        }
        #[inline]
        #[must_use]
        #[doc = "上传内容的 CRC32 校验码，如果指定此值，则七牛服务器会使用此值进行内容检验"]
        pub fn set_crc_32(self, value: impl Into<std::borrow::Cow<'a, str>>) -> RequestBody<'a> {
            self.add_part("crc32", qiniu_http_client::SyncPart::text(value))
        }
        #[inline]
        #[must_use]
        #[doc = "上传文件的内容"]
        pub fn set_file_as_reader(
            self,
            reader: impl std::io::Read + 'a,
            metadata: qiniu_http_client::PartMetadata,
        ) -> RequestBody<'a> {
            self.add_part("file", qiniu_http_client::SyncPart::stream(reader).metadata(metadata))
        }
        #[inline]
        #[must_use]
        #[doc = "上传文件的内容"]
        pub fn set_file_as_bytes(
            self,
            bytes: impl Into<std::borrow::Cow<'a, [u8]>>,
            metadata: qiniu_http_client::PartMetadata,
        ) -> RequestBody<'a> {
            self.add_part("file", qiniu_http_client::SyncPart::bytes(bytes).metadata(metadata))
        }
        #[inline]
        #[doc = "上传文件的内容"]
        pub fn set_file_as_file_path<S: AsRef<std::ffi::OsStr> + ?Sized>(
            self,
            path: &S,
        ) -> std::io::Result<RequestBody<'a>> {
            Ok(self.add_part(
                "file",
                qiniu_http_client::SyncPart::file_path(std::path::Path::new(path))?,
            ))
        }
        #[inline]
        #[must_use]
        #[doc = "自定义元数据（需要以 `x-qn-meta-` 作为前缀）或自定义变量（需要以 `x:` 作为前缀）"]
        pub fn append_custom_data(
            self,
            key: impl Into<qiniu_http_client::FieldName>,
            value: impl Into<std::borrow::Cow<'a, str>>,
        ) -> RequestBody<'a> {
            self.add_part(key, qiniu_http_client::SyncPart::text(value))
        }
    }
}
#[cfg(feature = "async")]
#[doc = "异步 Multipart 表单"]
pub mod async_part {
    #[derive(Debug, Default)]
    #[doc = "调用 API 所用的请求体参数"]
    pub struct RequestBody<'a> {
        multipart: qiniu_http_client::AsyncMultipart<'a>,
    }
    impl<'a> RequestBody<'a> {
        #[inline]
        #[must_use]
        #[doc = "添加新的 Multipart 表单组件"]
        pub fn add_part(
            mut self,
            name: impl Into<qiniu_http_client::FieldName>,
            part: qiniu_http_client::AsyncPart<'a>,
        ) -> Self {
            self.multipart = self.multipart.add_part(name.into(), part);
            self
        }
        fn build(self) -> qiniu_http_client::AsyncMultipart<'a> {
            self.multipart
        }
    }
    impl<'a> From<RequestBody<'a>> for qiniu_http_client::AsyncMultipart<'a> {
        #[inline]
        fn from(parts: RequestBody<'a>) -> Self {
            parts.build()
        }
    }
    impl<'a> RequestBody<'a> {
        #[inline]
        #[must_use]
        #[doc = "对象名称，如果不传入，则通过上传策略中的 `saveKey` 字段决定，如果 `saveKey` 也没有置顶，则使用对象的哈希值"]
        pub fn set_object_name(self, value: impl Into<std::borrow::Cow<'a, str>>) -> RequestBody<'a> {
            self.add_part("key", qiniu_http_client::AsyncPart::text(value))
        }
        #[inline]
        #[doc = "上传凭证"]
        pub async fn set_upload_token(
            self,
            token: &'a (dyn qiniu_http_client::upload_token::UploadTokenProvider + 'a),
            opts: qiniu_http_client::upload_token::ToStringOptions,
        ) -> qiniu_http_client::upload_token::ToStringResult<RequestBody<'a>> {
            Ok(self.add_part(
                "token",
                qiniu_http_client::AsyncPart::text(token.async_to_token_string(opts).await?),
            ))
        }
        #[inline]
        #[must_use]
        #[doc = "上传内容的 CRC32 校验码，如果指定此值，则七牛服务器会使用此值进行内容检验"]
        pub fn set_crc_32(self, value: impl Into<std::borrow::Cow<'a, str>>) -> RequestBody<'a> {
            self.add_part("crc32", qiniu_http_client::AsyncPart::text(value))
        }
        #[inline]
        #[must_use]
        #[doc = "上传文件的内容"]
        pub fn set_file_as_reader(
            self,
            reader: impl futures::io::AsyncRead + Send + Unpin + 'a,
            metadata: qiniu_http_client::PartMetadata,
        ) -> RequestBody<'a> {
            self.add_part("file", qiniu_http_client::AsyncPart::stream(reader).metadata(metadata))
        }
        #[inline]
        #[must_use]
        #[doc = "上传文件的内容"]
        pub fn set_file_as_bytes(
            self,
            bytes: impl Into<std::borrow::Cow<'a, [u8]>>,
            metadata: qiniu_http_client::PartMetadata,
        ) -> RequestBody<'a> {
            self.add_part("file", qiniu_http_client::AsyncPart::bytes(bytes).metadata(metadata))
        }
        #[inline]
        #[doc = "上传文件的内容"]
        pub async fn set_file_as_file_path<S: AsRef<std::ffi::OsStr> + ?Sized>(
            self,
            path: &S,
        ) -> std::io::Result<RequestBody<'a>> {
            Ok(self.add_part(
                "file",
                qiniu_http_client::AsyncPart::file_path(async_std::path::Path::new(path)).await?,
            ))
        }
        #[inline]
        #[must_use]
        #[doc = "自定义元数据（需要以 `x-qn-meta-` 作为前缀）或自定义变量（需要以 `x:` 作为前缀）"]
        pub fn append_custom_data(
            self,
            key: impl Into<qiniu_http_client::FieldName>,
            value: impl Into<std::borrow::Cow<'a, str>>,
        ) -> RequestBody<'a> {
            self.add_part(key, qiniu_http_client::AsyncPart::text(value))
        }
    }
}
#[derive(Clone, Debug, serde :: Serialize, serde :: Deserialize)]
#[serde(transparent)]
#[doc = "获取 API 所用的响应体参数"]
pub struct ResponseBody(serde_json::Value);
impl ResponseBody {
    #[allow(dead_code)]
    pub(crate) fn new(value: serde_json::Value) -> Self {
        Self(value)
    }
}
impl From<ResponseBody> for serde_json::Value {
    #[inline]
    fn from(val: ResponseBody) -> Self {
        val.0
    }
}
impl AsRef<serde_json::Value> for ResponseBody {
    #[inline]
    fn as_ref(&self) -> &serde_json::Value {
        &self.0
    }
}
impl AsMut<serde_json::Value> for ResponseBody {
    #[inline]
    fn as_mut(&mut self) -> &mut serde_json::Value {
        &mut self.0
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
    ) -> SyncRequestBuilder<'client, E> {
        RequestBuilder({
            let mut builder = self.0.post(&[qiniu_http_client::ServiceName::Up], endpoints_provider);
            builder.idempotent(qiniu_http_client::Idempotent::Default);
            builder.path("");
            builder.accept_json();
            builder
        })
    }
    #[inline]
    #[cfg(feature = "async")]
    #[doc = "创建一个新的异步请求"]
    pub fn new_async_request<E: qiniu_http_client::EndpointsProvider + 'client>(
        &self,
        endpoints_provider: E,
    ) -> AsyncRequestBuilder<'client, E> {
        RequestBuilder({
            let mut builder = self
                .0
                .async_post(&[qiniu_http_client::ServiceName::Up], endpoints_provider);
            builder.idempotent(qiniu_http_client::Idempotent::Default);
            builder.path("");
            builder.accept_json();
            builder
        })
    }
}
#[derive(Debug)]
#[doc = "API 请求构造器"]
pub struct RequestBuilder<'req, B, E>(qiniu_http_client::RequestBuilder<'req, B, E>);
#[doc = "API 阻塞请求构造器"]
pub type SyncRequestBuilder<'req, E> = RequestBuilder<'req, qiniu_http_client::SyncRequestBody<'req>, E>;
#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
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
        body: sync_part::RequestBody<'_>,
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<ResponseBody>> {
        let request = self.0.multipart(body)?;
        let response = request.call()?;
        let parsed = response.parse_json()?;
        Ok(parsed)
    }
}
#[cfg(feature = "async")]
impl<'req, E: qiniu_http_client::EndpointsProvider + Clone + 'req> AsyncRequestBuilder<'req, E> {
    #[doc = "异步发起 HTTP 请求"]
    pub async fn call(
        &mut self,
        body: async_part::RequestBody<'_>,
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<ResponseBody>> {
        let request = self.0.multipart(body).await?;
        let response = request.call().await?;
        let parsed = response.parse_json().await?;
        Ok(parsed)
    }
}
