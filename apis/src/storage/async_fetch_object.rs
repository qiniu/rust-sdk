// THIS FILE IS GENERATED BY api-generator, DO NOT EDIT DIRECTLY!
//
#[derive(Clone, Debug, serde :: Serialize, serde :: Deserialize)]
#[serde(transparent)]
#[doc = "调用 API 所用的请求体参数"]
pub struct RequestBody(serde_json::Value);
impl RequestBody {
    #[allow(dead_code)]
    pub(crate) fn new(value: serde_json::Value) -> Self {
        Self(value)
    }
}
impl Default for RequestBody {
    #[inline]
    fn default() -> Self {
        Self(serde_json::Value::Object(Default::default()))
    }
}
impl From<RequestBody> for serde_json::Value {
    #[inline]
    fn from(val: RequestBody) -> Self {
        val.0
    }
}
impl AsRef<serde_json::Value> for RequestBody {
    #[inline]
    fn as_ref(&self) -> &serde_json::Value {
        &self.0
    }
}
impl AsMut<serde_json::Value> for RequestBody {
    #[inline]
    fn as_mut(&mut self) -> &mut serde_json::Value {
        &mut self.0
    }
}
impl RequestBody {
    #[doc = "获取 需要抓取的 URL，支持设置多个用于高可用，以’;'分隔，当指定多个 URL 时可以在前一个 URL 抓取失败时重试下一个"]
    pub fn get_body_as_str(&self) -> &str {
        self.0.as_object().unwrap().get("body").unwrap().as_str().unwrap()
    }
}
impl RequestBody {
    #[doc = "设置 需要抓取的 URL，支持设置多个用于高可用，以’;'分隔，当指定多个 URL 时可以在前一个 URL 抓取失败时重试下一个"]
    pub fn set_body_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .as_object_mut()
            .unwrap()
            .insert("body".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl RequestBody {
    #[doc = "获取 所在区域的存储空间"]
    pub fn get_bucket_as_str(&self) -> &str {
        self.0.as_object().unwrap().get("bucket").unwrap().as_str().unwrap()
    }
}
impl RequestBody {
    #[doc = "设置 所在区域的存储空间"]
    pub fn set_bucket_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .as_object_mut()
            .unwrap()
            .insert("bucket".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl RequestBody {
    #[doc = "获取 从指定 URL 下载数据时使用的 Host"]
    pub fn get_host_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("host"))
            .and_then(|val| val.as_str())
    }
}
impl RequestBody {
    #[doc = "设置 从指定 URL 下载数据时使用的 Host"]
    pub fn set_host_as_str(&mut self, new: String) -> Option<String> {
        self.0.as_object_mut().and_then(|object| {
            object.insert("host".to_owned(), new.into()).and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
        })
    }
}
impl RequestBody {
    #[doc = "获取 对象名称，如果不传，则默认为文件的哈希值"]
    pub fn get_key_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("key"))
            .and_then(|val| val.as_str())
    }
}
impl RequestBody {
    #[doc = "设置 对象名称，如果不传，则默认为文件的哈希值"]
    pub fn set_key_as_str(&mut self, new: String) -> Option<String> {
        self.0.as_object_mut().and_then(|object| {
            object.insert("key".to_owned(), new.into()).and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
        })
    }
}
impl RequestBody {
    #[doc = "获取 对象内容的 ETag，传入以后会在存入存储时对文件做校验，校验失败则不存入指定空间"]
    pub fn get_etag_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("etag"))
            .and_then(|val| val.as_str())
    }
}
impl RequestBody {
    #[doc = "设置 对象内容的 ETag，传入以后会在存入存储时对文件做校验，校验失败则不存入指定空间"]
    pub fn set_etag_as_str(&mut self, new: String) -> Option<String> {
        self.0.as_object_mut().and_then(|object| {
            object.insert("etag".to_owned(), new.into()).and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
        })
    }
}
impl RequestBody {
    #[doc = "获取 回调 URL"]
    pub fn get_callback_url_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("callbackurl"))
            .and_then(|val| val.as_str())
    }
}
impl RequestBody {
    #[doc = "设置 回调 URL"]
    pub fn set_callback_url_as_str(&mut self, new: String) -> Option<String> {
        self.0.as_object_mut().and_then(|object| {
            object
                .insert("callbackurl".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl RequestBody {
    #[doc = "获取 回调负荷，如果 callback_url 不为空则必须指定"]
    pub fn get_callback_body_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("callbackbody"))
            .and_then(|val| val.as_str())
    }
}
impl RequestBody {
    #[doc = "设置 回调负荷，如果 callback_url 不为空则必须指定"]
    pub fn set_callback_body_as_str(&mut self, new: String) -> Option<String> {
        self.0.as_object_mut().and_then(|object| {
            object
                .insert("callbackbody".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl RequestBody {
    #[doc = "获取 回调负荷内容类型，默认为 \"application/x-www-form-urlencoded\""]
    pub fn get_callback_body_type_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("callbackbodytype"))
            .and_then(|val| val.as_str())
    }
}
impl RequestBody {
    #[doc = "设置 回调负荷内容类型，默认为 \"application/x-www-form-urlencoded\""]
    pub fn set_callback_body_type_as_str(&mut self, new: String) -> Option<String> {
        self.0.as_object_mut().and_then(|object| {
            object
                .insert("callbackbodytype".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl RequestBody {
    #[doc = "获取 回调时使用的 Host"]
    pub fn get_callback_host_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("callbackhost"))
            .and_then(|val| val.as_str())
    }
}
impl RequestBody {
    #[doc = "设置 回调时使用的 Host"]
    pub fn set_callback_host_as_str(&mut self, new: String) -> Option<String> {
        self.0.as_object_mut().and_then(|object| {
            object
                .insert("callbackhost".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl RequestBody {
    #[doc = "获取 存储文件类型 `0`: 标准存储(默认)，`1`: 低频存储，`2`: 归档存储"]
    pub fn get_file_type_as_i64(&self) -> Option<i64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("file_type"))
            .and_then(|val| val.as_i64())
    }
}
impl RequestBody {
    #[doc = "设置 存储文件类型 `0`: 标准存储(默认)，`1`: 低频存储，`2`: 归档存储"]
    pub fn set_file_type_as_i64(&mut self, new: i64) -> Option<i64> {
        self.0.as_object_mut().and_then(|object| {
            object
                .insert("file_type".to_owned(), new.into())
                .and_then(|val| val.as_i64())
        })
    }
}
impl RequestBody {
    #[doc = "获取 存储文件类型 `0`: 标准存储(默认)，`1`: 低频存储，`2`: 归档存储"]
    pub fn get_file_type_as_u64(&self) -> Option<u64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("file_type"))
            .and_then(|val| val.as_u64())
    }
}
impl RequestBody {
    #[doc = "设置 存储文件类型 `0`: 标准存储(默认)，`1`: 低频存储，`2`: 归档存储"]
    pub fn set_file_type_as_u64(&mut self, new: u64) -> Option<u64> {
        self.0.as_object_mut().and_then(|object| {
            object
                .insert("file_type".to_owned(), new.into())
                .and_then(|val| val.as_u64())
        })
    }
}
impl RequestBody {
    #[doc = "获取 如果空间中已经存在同名文件则放弃本次抓取（仅对比对象名称，不校验文件内容）"]
    pub fn get_ignore_same_key_as_bool(&self) -> Option<bool> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("ignore_same_key"))
            .and_then(|val| val.as_bool())
    }
}
impl RequestBody {
    #[doc = "设置 如果空间中已经存在同名文件则放弃本次抓取（仅对比对象名称，不校验文件内容）"]
    pub fn set_ignore_same_key_as_bool(&mut self, new: bool) -> Option<bool> {
        self.0.as_object_mut().and_then(|object| {
            object
                .insert("ignore_same_key".to_owned(), new.into())
                .and_then(|val| val.as_bool())
        })
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
impl Default for ResponseBody {
    #[inline]
    fn default() -> Self {
        Self(serde_json::Value::Object(Default::default()))
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
impl ResponseBody {
    #[doc = "获取 异步任务 ID"]
    pub fn get_id_as_str(&self) -> &str {
        self.0.as_object().unwrap().get("id").unwrap().as_str().unwrap()
    }
}
impl ResponseBody {
    #[doc = "设置 异步任务 ID"]
    pub fn set_id_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .as_object_mut()
            .unwrap()
            .insert("id".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl ResponseBody {
    #[doc = "获取 当前任务前面的排队任务数量，`0` 表示当前任务正在进行，`-1` 表示任务已经至少被处理过一次（可能会进入重试逻辑）"]
    pub fn get_queued_tasks_count_as_i64(&self) -> i64 {
        self.0.as_object().unwrap().get("wait").unwrap().as_i64().unwrap()
    }
}
impl ResponseBody {
    #[doc = "设置 当前任务前面的排队任务数量，`0` 表示当前任务正在进行，`-1` 表示任务已经至少被处理过一次（可能会进入重试逻辑）"]
    pub fn set_queued_tasks_count_as_i64(&mut self, new: i64) -> Option<i64> {
        self.0
            .as_object_mut()
            .unwrap()
            .insert("wait".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl ResponseBody {
    #[doc = "获取 当前任务前面的排队任务数量，`0` 表示当前任务正在进行，`-1` 表示任务已经至少被处理过一次（可能会进入重试逻辑）"]
    pub fn get_queued_tasks_count_as_u64(&self) -> u64 {
        self.0.as_object().unwrap().get("wait").unwrap().as_u64().unwrap()
    }
}
impl ResponseBody {
    #[doc = "设置 当前任务前面的排队任务数量，`0` 表示当前任务正在进行，`-1` 表示任务已经至少被处理过一次（可能会进入重试逻辑）"]
    pub fn set_queued_tasks_count_as_u64(&mut self, new: u64) -> Option<u64> {
        self.0
            .as_object_mut()
            .unwrap()
            .insert("wait".to_owned(), new.into())
            .and_then(|val| val.as_u64())
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
            let mut builder = self.0.post(&[qiniu_http_client::ServiceName::Api], endpoints_provider);
            builder.authorization(qiniu_http_client::Authorization::v2(credential));
            builder.idempotent(qiniu_http_client::Idempotent::Default);
            builder.path("sisyphus/fetch");
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
        credential: impl qiniu_http_client::credential::CredentialProvider + Clone + 'client,
    ) -> AsyncRequestBuilder<'client, E> {
        RequestBuilder({
            let mut builder = self
                .0
                .async_post(&[qiniu_http_client::ServiceName::Api], endpoints_provider);
            builder.authorization(qiniu_http_client::Authorization::v2(credential));
            builder.idempotent(qiniu_http_client::Idempotent::Default);
            builder.path("sisyphus/fetch");
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
        body: &RequestBody,
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<ResponseBody>> {
        let request = self.0.json(body)?;
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
        body: &RequestBody,
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<ResponseBody>> {
        let request = self.0.json(body)?;
        let response = request.call().await?;
        let parsed = response.parse_json().await?;
        Ok(parsed)
    }
}
