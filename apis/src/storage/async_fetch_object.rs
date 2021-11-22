#[derive(Clone, Debug, serde :: Serialize, serde :: Deserialize)]
#[serde(transparent)]
#[doc = "调用 API 所用的请求体参数"]
pub struct RequestBody<'a>(std::borrow::Cow<'a, serde_json::Value>);
impl<'a> RequestBody<'a> {
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn new(value: std::borrow::Cow<'a, serde_json::Value>) -> Self {
        Self(value)
    }
}
impl<'a> From<RequestBody<'a>> for serde_json::Value {
    #[inline]
    fn from(val: RequestBody<'a>) -> Self {
        val.0.into_owned()
    }
}
impl<'a> std::convert::AsRef<serde_json::Value> for RequestBody<'a> {
    #[inline]
    fn as_ref(&self) -> &serde_json::Value {
        self.0.as_ref()
    }
}
impl<'a> std::convert::AsMut<serde_json::Value> for RequestBody<'a> {
    #[inline]
    fn as_mut(&mut self) -> &mut serde_json::Value {
        self.0.to_mut()
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "获取 需要抓取的 URL，支持设置多个用于高可用，以’;'分隔，当指定多个 URL 时可以在前一个 URL 抓取失败时重试下一个"]
    pub fn get_body_as_str(&self) -> &str {
        self.0
            .as_object()
            .unwrap()
            .get("body")
            .unwrap()
            .as_str()
            .unwrap()
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "设置 需要抓取的 URL，支持设置多个用于高可用，以’;'分隔，当指定多个 URL 时可以在前一个 URL 抓取失败时重试下一个"]
    pub fn set_body_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("body".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "获取 所在区域的存储空间"]
    pub fn get_bucket_as_str(&self) -> &str {
        self.0
            .as_object()
            .unwrap()
            .get("bucket")
            .unwrap()
            .as_str()
            .unwrap()
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "设置 所在区域的存储空间"]
    pub fn set_bucket_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("bucket".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "获取 从指定 URL 下载数据时使用的 Host"]
    pub fn get_host_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("host"))
            .and_then(|val| val.as_str())
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "设置 从指定 URL 下载数据时使用的 Host"]
    pub fn set_host_as_str(&mut self, new: String) -> Option<String> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("host".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "获取 对象名称，如果不传，则默认为文件的哈希值"]
    pub fn get_key_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("key"))
            .and_then(|val| val.as_str())
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "设置 对象名称，如果不传，则默认为文件的哈希值"]
    pub fn set_key_as_str(&mut self, new: String) -> Option<String> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("key".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "获取 对象内容的 ETag，传入以后会在存入存储时对文件做校验，校验失败则不存入指定空间"]
    pub fn get_etag_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("etag"))
            .and_then(|val| val.as_str())
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "设置 对象内容的 ETag，传入以后会在存入存储时对文件做校验，校验失败则不存入指定空间"]
    pub fn set_etag_as_str(&mut self, new: String) -> Option<String> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("etag".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "获取 回调 URL"]
    pub fn get_callback_url_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("callbackurl"))
            .and_then(|val| val.as_str())
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "设置 回调 URL"]
    pub fn set_callback_url_as_str(&mut self, new: String) -> Option<String> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("callbackurl".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "获取 回调负荷，如果 callback_url 不为空则必须指定"]
    pub fn get_callback_body_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("callbackbody"))
            .and_then(|val| val.as_str())
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "设置 回调负荷，如果 callback_url 不为空则必须指定"]
    pub fn set_callback_body_as_str(&mut self, new: String) -> Option<String> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("callbackbody".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "获取 回调负荷内容类型，默认为 \"application/x-www-form-urlencoded\""]
    pub fn get_callback_body_type_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("callbackbodytype"))
            .and_then(|val| val.as_str())
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "设置 回调负荷内容类型，默认为 \"application/x-www-form-urlencoded\""]
    pub fn set_callback_body_type_as_str(&mut self, new: String) -> Option<String> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("callbackbodytype".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "获取 回调时使用的 Host"]
    pub fn get_callback_host_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("callbackhost"))
            .and_then(|val| val.as_str())
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "设置 回调时使用的 Host"]
    pub fn set_callback_host_as_str(&mut self, new: String) -> Option<String> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("callbackhost".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "获取 存储文件类型 `0`: 标准存储(默认)，`1`: 低频存储，`2`: 归档存储"]
    pub fn get_file_type_as_int(&self) -> Option<i64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("file_type"))
            .and_then(|val| val.as_i64())
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "设置 存储文件类型 `0`: 标准存储(默认)，`1`: 低频存储，`2`: 归档存储"]
    pub fn set_file_type_as_int(&mut self, new: i64) -> Option<i64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("file_type".to_owned(), new.into())
                .and_then(|val| val.as_i64())
        })
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "获取 存储文件类型 `0`: 标准存储(默认)，`1`: 低频存储，`2`: 归档存储"]
    pub fn get_file_type_as_uint(&self) -> Option<u64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("file_type"))
            .and_then(|val| val.as_u64())
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "设置 存储文件类型 `0`: 标准存储(默认)，`1`: 低频存储，`2`: 归档存储"]
    pub fn set_file_type_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("file_type".to_owned(), new.into())
                .and_then(|val| val.as_u64())
        })
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "获取 如果空间中已经存在同名文件则放弃本次抓取（仅对比对象名称，不校验文件内容）"]
    pub fn get_ignore_same_key_as_bool(&self) -> Option<bool> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("ignore_same_key"))
            .and_then(|val| val.as_bool())
    }
}
impl<'a> RequestBody<'a> {
    #[inline]
    #[doc = "设置 如果空间中已经存在同名文件则放弃本次抓取（仅对比对象名称，不校验文件内容）"]
    pub fn set_ignore_same_key_as_bool(&mut self, new: bool) -> Option<bool> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("ignore_same_key".to_owned(), new.into())
                .and_then(|val| val.as_bool())
        })
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
    #[doc = "获取 异步任务 ID"]
    pub fn get_id_as_str(&self) -> &str {
        self.0
            .as_object()
            .unwrap()
            .get("id")
            .unwrap()
            .as_str()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 异步任务 ID"]
    pub fn set_id_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("id".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 当前任务前面的排队任务数量，`0` 表示当前任务正在进行，`-1` 表示任务已经至少被处理过一次（可能会进入重试逻辑）"]
    pub fn get_queued_tasks_count_as_int(&self) -> i64 {
        self.0
            .as_object()
            .unwrap()
            .get("wait")
            .unwrap()
            .as_i64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 当前任务前面的排队任务数量，`0` 表示当前任务正在进行，`-1` 表示任务已经至少被处理过一次（可能会进入重试逻辑）"]
    pub fn set_queued_tasks_count_as_int(&mut self, new: i64) -> Option<i64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("wait".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 当前任务前面的排队任务数量，`0` 表示当前任务正在进行，`-1` 表示任务已经至少被处理过一次（可能会进入重试逻辑）"]
    pub fn get_queued_tasks_count_as_uint(&self) -> u64 {
        self.0
            .as_object()
            .unwrap()
            .get("wait")
            .unwrap()
            .as_u64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 当前任务前面的排队任务数量，`0` 表示当前任务正在进行，`-1` 表示任务已经至少被处理过一次（可能会进入重试逻辑）"]
    pub fn set_queued_tasks_count_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("wait".to_owned(), new.into())
            .and_then(|val| val.as_u64())
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
        credential: std::sync::Arc<dyn qiniu_http_client::credential::CredentialProvider>,
    ) -> SyncRequestBuilder {
        SyncRequestBuilder(
            self.0
                .post(
                    &[qiniu_http_client::ServiceName::Api],
                    into_endpoints.into(),
                )
                .authorization(qiniu_http_client::Authorization::v2(credential))
                .idempotent(qiniu_http_client::Idempotent::Default)
                .path("sisyphus/fetch")
                .accept_json(),
        )
    }
    #[inline]
    #[cfg(feature = "async")]
    pub fn new_async_request(
        &self,
        into_endpoints: impl Into<qiniu_http_client::IntoEndpoints<'client>>,
        credential: std::sync::Arc<dyn qiniu_http_client::credential::CredentialProvider>,
    ) -> AsyncRequestBuilder {
        AsyncRequestBuilder(
            self.0
                .async_post(
                    &[qiniu_http_client::ServiceName::Api],
                    into_endpoints.into(),
                )
                .authorization(qiniu_http_client::Authorization::v2(credential))
                .idempotent(qiniu_http_client::Idempotent::Default)
                .path("sisyphus/fetch")
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
        body: &RequestBody<'_>,
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<ResponseBody<'static>>> {
        let request = self.0.json(body)?;
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
        body: &RequestBody<'_>,
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<ResponseBody<'static>>> {
        let request = self.0.json(body)?;
        let response = request.call().await?;
        let parsed = response.parse_json().await?;
        Ok(parsed)
    }
}
