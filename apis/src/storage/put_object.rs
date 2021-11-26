pub mod sync_part {
    #[derive(Debug, Default)]
    #[doc = "调用 API 所用的请求体参数"]
    pub struct RequestBody {
        multipart: qiniu_http_client::SyncMultipart,
    }
    impl RequestBody {
        #[inline]
        pub fn add_part(
            mut self,
            name: impl Into<qiniu_http_client::FieldName>,
            part: qiniu_http_client::SyncPart,
        ) -> Self {
            self.multipart = self.multipart.add_part(name.into(), part);
            self
        }
        #[inline]
        fn build(self) -> qiniu_http_client::SyncMultipart {
            self.multipart
        }
    }
    impl From<RequestBody> for qiniu_http_client::SyncMultipart {
        #[inline]
        fn from(parts: RequestBody) -> Self {
            parts.build()
        }
    }
    impl RequestBody {
        #[inline]
        #[doc = "对象名称，如果不传入，则通过上传策略中的 `saveKey` 字段决定，如果 `saveKey` 也没有置顶，则使用对象的哈希值"]
        pub fn set_object_name(self, value: impl Into<std::borrow::Cow<'static, str>>) -> Self {
            self.add_part("key", qiniu_http_client::SyncPart::text(value))
        }
        #[inline]
        #[doc = "上传凭证"]
        pub fn set_upload_token(
            self,
            token: &dyn qiniu_http_client::upload_token::UploadTokenProvider,
        ) -> std::io::Result<Self> {
            Ok(self.add_part(
                "token",
                qiniu_http_client::SyncPart::text(String::from(
                    token.to_token_string(&Default::default())?,
                )),
            ))
        }
        #[inline]
        #[doc = "上传内容的 CRC32 校验码，如果指定此值，则七牛服务器会使用此值进行内容检验"]
        pub fn set_crc_32(self, value: impl Into<std::borrow::Cow<'static, str>>) -> Self {
            self.add_part("crc32", qiniu_http_client::SyncPart::text(value))
        }
        #[inline]
        #[doc = "上传文件的内容"]
        pub fn set_file_as_reader(
            self,
            reader: Box<dyn std::io::Read>,
            metadata: qiniu_http_client::PartMetadata,
        ) -> Self {
            self.add_part(
                "file",
                qiniu_http_client::SyncPart::stream(reader).metadata(metadata),
            )
        }
        #[inline]
        #[doc = "上传文件的内容"]
        pub fn set_file_as_bytes(
            self,
            bytes: impl Into<std::borrow::Cow<'static, [u8]>>,
            metadata: qiniu_http_client::PartMetadata,
        ) -> Self {
            self.add_part(
                "file",
                qiniu_http_client::SyncPart::bytes(bytes).metadata(metadata),
            )
        }
        #[inline]
        #[doc = "上传文件的内容"]
        pub fn set_file_as_file_path(
            self,
            path: impl AsRef<std::path::Path>,
        ) -> std::io::Result<Self> {
            Ok(self.add_part("file", qiniu_http_client::SyncPart::file_path(path)?))
        }
        #[inline]
        #[doc = "自定义元数据（需要以 `x-qn-meta-` 作为前缀）或自定义变量（需要以 `x:` 作为前缀）"]
        pub fn append_custom_data(
            self,
            key: impl Into<qiniu_http_client::FieldName>,
            value: impl Into<std::borrow::Cow<'static, str>>,
        ) -> Self {
            self.add_part(key, qiniu_http_client::SyncPart::text(value))
        }
    }
}
#[cfg(feature = "async")]
pub mod async_part {
    #[derive(Debug, Default)]
    #[doc = "调用 API 所用的请求体参数"]
    pub struct RequestBody {
        multipart: qiniu_http_client::AsyncMultipart,
    }
    impl RequestBody {
        #[inline]
        pub fn add_part(
            mut self,
            name: impl Into<qiniu_http_client::FieldName>,
            part: qiniu_http_client::AsyncPart,
        ) -> Self {
            self.multipart = self.multipart.add_part(name.into(), part);
            self
        }
        #[inline]
        fn build(self) -> qiniu_http_client::AsyncMultipart {
            self.multipart
        }
    }
    impl From<RequestBody> for qiniu_http_client::AsyncMultipart {
        #[inline]
        fn from(parts: RequestBody) -> Self {
            parts.build()
        }
    }
    impl RequestBody {
        #[inline]
        #[doc = "对象名称，如果不传入，则通过上传策略中的 `saveKey` 字段决定，如果 `saveKey` 也没有置顶，则使用对象的哈希值"]
        pub fn set_object_name(self, value: impl Into<std::borrow::Cow<'static, str>>) -> Self {
            self.add_part("key", qiniu_http_client::AsyncPart::text(value))
        }
        #[inline]
        #[doc = "上传凭证"]
        pub async fn set_upload_token(
            self,
            token: &dyn qiniu_http_client::upload_token::UploadTokenProvider,
        ) -> std::io::Result<Self> {
            Ok(self.add_part(
                "token",
                qiniu_http_client::AsyncPart::text(String::from(
                    token.async_to_token_string(&Default::default()).await?,
                )),
            ))
        }
        #[inline]
        #[doc = "上传内容的 CRC32 校验码，如果指定此值，则七牛服务器会使用此值进行内容检验"]
        pub fn set_crc_32(self, value: impl Into<std::borrow::Cow<'static, str>>) -> Self {
            self.add_part("crc32", qiniu_http_client::AsyncPart::text(value))
        }
        #[inline]
        #[doc = "上传文件的内容"]
        pub fn set_file_as_reader(
            self,
            reader: Box<dyn futures::io::AsyncRead + Send + Unpin>,
            metadata: qiniu_http_client::PartMetadata,
        ) -> Self {
            self.add_part(
                "file",
                qiniu_http_client::AsyncPart::stream(reader).metadata(metadata),
            )
        }
        #[inline]
        #[doc = "上传文件的内容"]
        pub fn set_file_as_bytes(
            self,
            bytes: impl Into<std::borrow::Cow<'static, [u8]>>,
            metadata: qiniu_http_client::PartMetadata,
        ) -> Self {
            self.add_part(
                "file",
                qiniu_http_client::AsyncPart::bytes(bytes).metadata(metadata),
            )
        }
        #[inline]
        #[doc = "上传文件的内容"]
        pub async fn set_file_as_file_path(
            self,
            path: impl AsRef<async_std::path::Path>,
        ) -> std::io::Result<Self> {
            Ok(self.add_part("file", qiniu_http_client::AsyncPart::file_path(path).await?))
        }
        #[inline]
        #[doc = "自定义元数据（需要以 `x-qn-meta-` 作为前缀）或自定义变量（需要以 `x:` 作为前缀）"]
        pub fn append_custom_data(
            self,
            key: impl Into<qiniu_http_client::FieldName>,
            value: impl Into<std::borrow::Cow<'static, str>>,
        ) -> Self {
            self.add_part(key, qiniu_http_client::AsyncPart::text(value))
        }
    }
}
#[derive(Clone, Debug, serde :: Serialize, serde :: Deserialize)]
#[serde(transparent)]
#[doc = "获取 API 所用的响应体参数"]
pub struct ResponseBody<'a>(std::borrow::Cow<'a, serde_json::Value>);
impl<'a> ResponseBody<'a> {
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn new(value: std::borrow::Cow<'a, serde_json::Value>) -> Self {
        Self(value)
    }
}
impl<'a> From<ResponseBody<'a>> for serde_json::Value {
    #[inline]
    fn from(val: ResponseBody<'a>) -> Self {
        val.0.into_owned()
    }
}
impl<'a> std::convert::AsRef<serde_json::Value> for ResponseBody<'a> {
    #[inline]
    fn as_ref(&self) -> &serde_json::Value {
        self.0.as_ref()
    }
}
impl<'a> std::convert::AsMut<serde_json::Value> for ResponseBody<'a> {
    #[inline]
    fn as_mut(&mut self) -> &mut serde_json::Value {
        self.0.to_mut()
    }
}
#[derive(Debug, Clone)]
pub struct Client<'client>(&'client qiniu_http_client::HttpClient);
impl<'client> Client<'client> {
    #[inline]
    pub(super) fn new(http_client: &'client qiniu_http_client::HttpClient) -> Self {
        Self(http_client)
    }
}
impl<'client> Client<'client> {
    #[inline]
    pub fn new_request(
        &self,
        into_endpoints: impl Into<qiniu_http_client::IntoEndpoints<'client>>,
    ) -> SyncRequestBuilder {
        SyncRequestBuilder(
            self.0
                .post(&[qiniu_http_client::ServiceName::Up], into_endpoints.into())
                .idempotent(qiniu_http_client::Idempotent::Default)
                .path("")
                .accept_json(),
        )
    }
    #[inline]
    #[cfg(feature = "async")]
    pub fn new_async_request(
        &self,
        into_endpoints: impl Into<qiniu_http_client::IntoEndpoints<'client>>,
    ) -> AsyncRequestBuilder {
        AsyncRequestBuilder(
            self.0
                .async_post(&[qiniu_http_client::ServiceName::Up], into_endpoints.into())
                .idempotent(qiniu_http_client::Idempotent::Default)
                .path("")
                .accept_json(),
        )
    }
}
#[derive(Debug)]
pub struct SyncRequestBuilder<'req>(qiniu_http_client::SyncRequestBuilder<'req>);
#[derive(Debug)]
#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub struct AsyncRequestBuilder<'req>(qiniu_http_client::AsyncRequestBuilder<'req>);
impl<'req> SyncRequestBuilder<'req> {
    #[inline]
    pub fn use_https(mut self, use_https: bool) -> Self {
        self.0 = self.0.use_https(use_https);
        self
    }
    #[inline]
    pub fn version(mut self, version: qiniu_http_client::http::Version) -> Self {
        self.0 = self.0.version(version);
        self
    }
    #[inline]
    pub fn headers(
        mut self,
        headers: impl Into<std::borrow::Cow<'req, qiniu_http_client::http::HeaderMap>>,
    ) -> Self {
        self.0 = self.0.headers(headers);
        self
    }
    #[inline]
    pub fn query_pairs(
        mut self,
        query_pairs: impl Into<qiniu_http_client::QueryPairs<'req>>,
    ) -> Self {
        self.0 = self.0.query_pairs(query_pairs);
        self
    }
    #[inline]
    pub fn extensions(mut self, extensions: qiniu_http_client::http::Extensions) -> Self {
        self.0 = self.0.extensions(extensions);
        self
    }
    #[inline]
    pub fn add_extension<T: Send + Sync + 'static>(mut self, val: T) -> Self {
        self.0 = self.0.add_extension(val);
        self
    }
    #[inline]
    pub fn on_uploading_progress(mut self, callback: qiniu_http_client::OnProgress) -> Self {
        self.0 = self.0.on_uploading_progress(callback);
        self
    }
    #[inline]
    pub fn on_receive_response_status(mut self, callback: qiniu_http_client::OnStatusCode) -> Self {
        self.0 = self.0.on_receive_response_status(callback);
        self
    }
    #[inline]
    pub fn on_receive_response_header(mut self, callback: qiniu_http_client::OnHeader) -> Self {
        self.0 = self.0.on_receive_response_header(callback);
        self
    }
    #[inline]
    pub fn on_to_resolve_domain(mut self, callback: qiniu_http_client::OnToResolveDomain) -> Self {
        self.0 = self.0.on_to_resolve_domain(callback);
        self
    }
    #[inline]
    pub fn on_domain_resolved(mut self, callback: qiniu_http_client::OnDomainResolved) -> Self {
        self.0 = self.0.on_domain_resolved(callback);
        self
    }
    #[inline]
    pub fn on_to_choose_ips(mut self, callback: qiniu_http_client::OnToChooseIPs) -> Self {
        self.0 = self.0.on_to_choose_ips(callback);
        self
    }
    #[inline]
    pub fn on_ips_chosen(mut self, callback: qiniu_http_client::OnIPsChosen) -> Self {
        self.0 = self.0.on_ips_chosen(callback);
        self
    }
    #[inline]
    pub fn on_before_request_signed(mut self, callback: qiniu_http_client::OnRequest) -> Self {
        self.0 = self.0.on_before_request_signed(callback);
        self
    }
    #[inline]
    pub fn on_after_request_signed(mut self, callback: qiniu_http_client::OnRequest) -> Self {
        self.0 = self.0.on_after_request_signed(callback);
        self
    }
    #[inline]
    pub fn on_success(mut self, callback: qiniu_http_client::OnSuccess) -> Self {
        self.0 = self.0.on_success(callback);
        self
    }
    #[inline]
    pub fn on_error(mut self, callback: qiniu_http_client::OnError) -> Self {
        self.0 = self.0.on_error(callback);
        self
    }
    #[inline]
    pub fn on_before_backoff(mut self, callback: qiniu_http_client::OnRetry) -> Self {
        self.0 = self.0.on_before_backoff(callback);
        self
    }
    #[inline]
    pub fn on_after_backoff(mut self, callback: qiniu_http_client::OnRetry) -> Self {
        self.0 = self.0.on_after_backoff(callback);
        self
    }
    pub fn call(
        self,
        body: sync_part::RequestBody,
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<ResponseBody<'static>>> {
        let request = self.0.multipart(body)?;
        let response = request.call()?;
        let parsed = response.parse_json()?;
        Ok(parsed)
    }
}
#[cfg(feature = "async")]
impl<'req> AsyncRequestBuilder<'req> {
    #[inline]
    pub fn use_https(mut self, use_https: bool) -> Self {
        self.0 = self.0.use_https(use_https);
        self
    }
    #[inline]
    pub fn version(mut self, version: qiniu_http_client::http::Version) -> Self {
        self.0 = self.0.version(version);
        self
    }
    #[inline]
    pub fn headers(
        mut self,
        headers: impl Into<std::borrow::Cow<'req, qiniu_http_client::http::HeaderMap>>,
    ) -> Self {
        self.0 = self.0.headers(headers);
        self
    }
    #[inline]
    pub fn query_pairs(
        mut self,
        query_pairs: impl Into<qiniu_http_client::QueryPairs<'req>>,
    ) -> Self {
        self.0 = self.0.query_pairs(query_pairs);
        self
    }
    #[inline]
    pub fn extensions(mut self, extensions: qiniu_http_client::http::Extensions) -> Self {
        self.0 = self.0.extensions(extensions);
        self
    }
    #[inline]
    pub fn add_extension<T: Send + Sync + 'static>(mut self, val: T) -> Self {
        self.0 = self.0.add_extension(val);
        self
    }
    #[inline]
    pub fn on_uploading_progress(mut self, callback: qiniu_http_client::OnProgress) -> Self {
        self.0 = self.0.on_uploading_progress(callback);
        self
    }
    #[inline]
    pub fn on_receive_response_status(mut self, callback: qiniu_http_client::OnStatusCode) -> Self {
        self.0 = self.0.on_receive_response_status(callback);
        self
    }
    #[inline]
    pub fn on_receive_response_header(mut self, callback: qiniu_http_client::OnHeader) -> Self {
        self.0 = self.0.on_receive_response_header(callback);
        self
    }
    #[inline]
    pub fn on_to_resolve_domain(mut self, callback: qiniu_http_client::OnToResolveDomain) -> Self {
        self.0 = self.0.on_to_resolve_domain(callback);
        self
    }
    #[inline]
    pub fn on_domain_resolved(mut self, callback: qiniu_http_client::OnDomainResolved) -> Self {
        self.0 = self.0.on_domain_resolved(callback);
        self
    }
    #[inline]
    pub fn on_to_choose_ips(mut self, callback: qiniu_http_client::OnToChooseIPs) -> Self {
        self.0 = self.0.on_to_choose_ips(callback);
        self
    }
    #[inline]
    pub fn on_ips_chosen(mut self, callback: qiniu_http_client::OnIPsChosen) -> Self {
        self.0 = self.0.on_ips_chosen(callback);
        self
    }
    #[inline]
    pub fn on_before_request_signed(mut self, callback: qiniu_http_client::OnRequest) -> Self {
        self.0 = self.0.on_before_request_signed(callback);
        self
    }
    #[inline]
    pub fn on_after_request_signed(mut self, callback: qiniu_http_client::OnRequest) -> Self {
        self.0 = self.0.on_after_request_signed(callback);
        self
    }
    #[inline]
    pub fn on_success(mut self, callback: qiniu_http_client::OnSuccess) -> Self {
        self.0 = self.0.on_success(callback);
        self
    }
    #[inline]
    pub fn on_error(mut self, callback: qiniu_http_client::OnError) -> Self {
        self.0 = self.0.on_error(callback);
        self
    }
    #[inline]
    pub fn on_before_backoff(mut self, callback: qiniu_http_client::OnRetry) -> Self {
        self.0 = self.0.on_before_backoff(callback);
        self
    }
    #[inline]
    pub fn on_after_backoff(mut self, callback: qiniu_http_client::OnRetry) -> Self {
        self.0 = self.0.on_after_backoff(callback);
        self
    }
    pub async fn call(
        self,
        body: async_part::RequestBody,
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<ResponseBody<'static>>> {
        let request = self.0.multipart(body).await?;
        let response = request.call().await?;
        let parsed = response.parse_json().await?;
        Ok(parsed)
    }
}