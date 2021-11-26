#[derive(Debug, Clone, Default)]
#[doc = "调用 API 所用的路径参数"]
pub struct PathParams {
    r#ctx: Option<std::borrow::Cow<'static, str>>,
    r#chunk_offset: Option<std::borrow::Cow<'static, str>>,
    extended_segments: Vec<std::borrow::Cow<'static, str>>,
}
impl PathParams {
    #[inline]
    pub fn push_segment(mut self, segment: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        self.extended_segments.push(segment.into());
        self
    }
    #[inline]
    fn build(self) -> Vec<std::borrow::Cow<'static, str>> {
        let mut all_segments: Vec<_> = Default::default();
        if let Some(segment) = self.r#ctx {
            all_segments.push(segment);
        }
        if let Some(segment) = self.r#chunk_offset {
            all_segments.push(segment);
        }
        all_segments.extend(self.extended_segments);
        all_segments
    }
}
impl PathParams {
    #[inline]
    #[doc = "前一次上传返回的块级上传控制信息"]
    pub fn set_ctx_as_str(mut self, value: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        self.r#ctx = Some(value.into());
        self
    }
    #[inline]
    #[doc = "当前片在整个块中的起始偏移"]
    pub fn set_chunk_offset_as_int(mut self, value: i64) -> Self {
        self.r#chunk_offset = Some(value.to_string().into());
        self
    }
    #[inline]
    #[doc = "当前片在整个块中的起始偏移"]
    pub fn set_chunk_offset_as_uint(mut self, value: u64) -> Self {
        self.r#chunk_offset = Some(value.to_string().into());
        self
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
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 本次上传成功后的块级上传控制信息，用于后续上传片（bput）及创建文件（mkfile）"]
    pub fn get_ctx_as_str(&self) -> &str {
        self.0
            .as_object()
            .unwrap()
            .get("ctx")
            .unwrap()
            .as_str()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 本次上传成功后的块级上传控制信息，用于后续上传片（bput）及创建文件（mkfile）"]
    pub fn set_ctx_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("ctx".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 上传块 SHA1 值，使用 URL 安全的 Base64 编码"]
    pub fn get_checksum_as_str(&self) -> &str {
        self.0
            .as_object()
            .unwrap()
            .get("checksum")
            .unwrap()
            .as_str()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 上传块 SHA1 值，使用 URL 安全的 Base64 编码"]
    pub fn set_checksum_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("checksum".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 上传块 CRC32 值，客户可通过此字段对上传块的完整性进行校验"]
    pub fn get_crc_32_as_int(&self) -> i64 {
        self.0
            .as_object()
            .unwrap()
            .get("crc32")
            .unwrap()
            .as_i64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 上传块 CRC32 值，客户可通过此字段对上传块的完整性进行校验"]
    pub fn set_crc_32_as_int(&mut self, new: i64) -> Option<i64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("crc32".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 上传块 CRC32 值，客户可通过此字段对上传块的完整性进行校验"]
    pub fn get_crc_32_as_uint(&self) -> u64 {
        self.0
            .as_object()
            .unwrap()
            .get("crc32")
            .unwrap()
            .as_u64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 上传块 CRC32 值，客户可通过此字段对上传块的完整性进行校验"]
    pub fn set_crc_32_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("crc32".to_owned(), new.into())
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 下一个上传块在切割块中的偏移"]
    pub fn get_offset_as_int(&self) -> i64 {
        self.0
            .as_object()
            .unwrap()
            .get("offset")
            .unwrap()
            .as_i64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 下一个上传块在切割块中的偏移"]
    pub fn set_offset_as_int(&mut self, new: i64) -> Option<i64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("offset".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 下一个上传块在切割块中的偏移"]
    pub fn get_offset_as_uint(&self) -> u64 {
        self.0
            .as_object()
            .unwrap()
            .get("offset")
            .unwrap()
            .as_u64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 下一个上传块在切割块中的偏移"]
    pub fn set_offset_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("offset".to_owned(), new.into())
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 后续上传接收地址"]
    pub fn get_host_as_str(&self) -> &str {
        self.0
            .as_object()
            .unwrap()
            .get("host")
            .unwrap()
            .as_str()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 后续上传接收地址"]
    pub fn set_host_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("host".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 `ctx` 过期时间"]
    pub fn get_expired_at_as_str(&self) -> &str {
        self.0
            .as_object()
            .unwrap()
            .get("expired_at")
            .unwrap()
            .as_str()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 `ctx` 过期时间"]
    pub fn set_expired_at_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("expired_at".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
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
        path_params: PathParams,
        upload_token: Box<dyn qiniu_http_client::upload_token::UploadTokenProvider>,
    ) -> SyncRequestBuilder {
        SyncRequestBuilder(
            self.0
                .post(&[qiniu_http_client::ServiceName::Up], into_endpoints.into())
                .authorization(qiniu_http_client::Authorization::uptoken(upload_token))
                .idempotent(qiniu_http_client::Idempotent::Always)
                .path(crate::base_utils::join_path(
                    "/bput",
                    "",
                    path_params.build(),
                ))
                .accept_json(),
        )
    }
    #[inline]
    #[cfg(feature = "async")]
    pub fn new_async_request(
        &self,
        into_endpoints: impl Into<qiniu_http_client::IntoEndpoints<'client>>,
        path_params: PathParams,
        upload_token: Box<dyn qiniu_http_client::upload_token::UploadTokenProvider>,
    ) -> AsyncRequestBuilder {
        AsyncRequestBuilder(
            self.0
                .async_post(&[qiniu_http_client::ServiceName::Up], into_endpoints.into())
                .authorization(qiniu_http_client::Authorization::uptoken(upload_token))
                .idempotent(qiniu_http_client::Idempotent::Always)
                .path(crate::base_utils::join_path(
                    "/bput",
                    "",
                    path_params.build(),
                ))
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
        body: impl std::io::Read
            + qiniu_http_client::http::Reset
            + std::fmt::Debug
            + Send
            + Sync
            + 'static,
        content_length: u64,
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<ResponseBody<'static>>> {
        let request = self.0.stream_as_body(body, content_length, None);
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
        body: impl futures::io::AsyncRead
            + qiniu_http_client::http::AsyncReset
            + Unpin
            + std::fmt::Debug
            + Send
            + Sync
            + 'static,
        content_length: u64,
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<ResponseBody<'static>>> {
        let request = self.0.stream_as_body(body, content_length, None);
        let response = request.call().await?;
        let parsed = response.parse_json().await?;
        Ok(parsed)
    }
}
