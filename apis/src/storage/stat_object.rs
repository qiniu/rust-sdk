#[derive(Debug, Clone, Default)]
#[doc = "调用 API 所用的路径参数"]
pub struct PathParams {
    r#entry: Option<std::borrow::Cow<'static, str>>,
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
        if let Some(segment) = self.r#entry {
            all_segments.push(segment);
        }
        all_segments.extend(self.extended_segments);
        all_segments
    }
}
impl PathParams {
    #[inline]
    #[doc = "指定目标对象空间与目标对象名称"]
    pub fn set_entry_as_str(mut self, value: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        self.r#entry = Some(qiniu_utils::base64::urlsafe(value.into().as_bytes()).into());
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
    #[doc = "获取 对象大小，单位为字节"]
    pub fn get_size_as_int(&self) -> i64 {
        self.0
            .as_object()
            .unwrap()
            .get("fsize")
            .unwrap()
            .as_i64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 对象大小，单位为字节"]
    pub fn set_size_as_int(&mut self, new: i64) -> Option<i64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("fsize".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 对象大小，单位为字节"]
    pub fn get_size_as_uint(&self) -> u64 {
        self.0
            .as_object()
            .unwrap()
            .get("fsize")
            .unwrap()
            .as_u64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 对象大小，单位为字节"]
    pub fn set_size_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("fsize".to_owned(), new.into())
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 对象哈希值"]
    pub fn get_hash_as_str(&self) -> &str {
        self.0
            .as_object()
            .unwrap()
            .get("hash")
            .unwrap()
            .as_str()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 对象哈希值"]
    pub fn set_hash_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("hash".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 对象 MIME 类型"]
    pub fn get_mime_type_as_str(&self) -> &str {
        self.0
            .as_object()
            .unwrap()
            .get("mimeType")
            .unwrap()
            .as_str()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 对象 MIME 类型"]
    pub fn set_mime_type_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("mimeType".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 对象存储类型，`0` 表示普通存储，`1` 表示低频存储，`2` 表示归档存储"]
    pub fn get_type_as_int(&self) -> i64 {
        self.0
            .as_object()
            .unwrap()
            .get("type")
            .unwrap()
            .as_i64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 对象存储类型，`0` 表示普通存储，`1` 表示低频存储，`2` 表示归档存储"]
    pub fn set_type_as_int(&mut self, new: i64) -> Option<i64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("type".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 对象存储类型，`0` 表示普通存储，`1` 表示低频存储，`2` 表示归档存储"]
    pub fn get_type_as_uint(&self) -> u64 {
        self.0
            .as_object()
            .unwrap()
            .get("type")
            .unwrap()
            .as_u64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 对象存储类型，`0` 表示普通存储，`1` 表示低频存储，`2` 表示归档存储"]
    pub fn set_type_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("type".to_owned(), new.into())
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 文件上传时间，UNIX 时间戳格式，单位为 100 纳秒"]
    pub fn get_put_time_as_int(&self) -> i64 {
        self.0
            .as_object()
            .unwrap()
            .get("putTime")
            .unwrap()
            .as_i64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 文件上传时间，UNIX 时间戳格式，单位为 100 纳秒"]
    pub fn set_put_time_as_int(&mut self, new: i64) -> Option<i64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("putTime".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 文件上传时间，UNIX 时间戳格式，单位为 100 纳秒"]
    pub fn get_put_time_as_uint(&self) -> u64 {
        self.0
            .as_object()
            .unwrap()
            .get("putTime")
            .unwrap()
            .as_u64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 文件上传时间，UNIX 时间戳格式，单位为 100 纳秒"]
    pub fn set_put_time_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("putTime".to_owned(), new.into())
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 归档存储文件的解冻状态，`2` 表示解冻完成，`1` 表示解冻中；归档文件冻结时，不返回该字段"]
    pub fn get_unfreezing_status_as_int(&self) -> Option<i64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("restoreStatus"))
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 归档存储文件的解冻状态，`2` 表示解冻完成，`1` 表示解冻中；归档文件冻结时，不返回该字段"]
    pub fn set_unfreezing_status_as_int(&mut self, new: i64) -> Option<i64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("restoreStatus".to_owned(), new.into())
                .and_then(|val| val.as_i64())
        })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 归档存储文件的解冻状态，`2` 表示解冻完成，`1` 表示解冻中；归档文件冻结时，不返回该字段"]
    pub fn get_unfreezing_status_as_uint(&self) -> Option<u64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("restoreStatus"))
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 归档存储文件的解冻状态，`2` 表示解冻完成，`1` 表示解冻中；归档文件冻结时，不返回该字段"]
    pub fn set_unfreezing_status_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("restoreStatus".to_owned(), new.into())
                .and_then(|val| val.as_u64())
        })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 文件状态。`1` 表示禁用；只有禁用状态的文件才会返回该字段"]
    pub fn get_status_as_int(&self) -> Option<i64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("status"))
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 文件状态。`1` 表示禁用；只有禁用状态的文件才会返回该字段"]
    pub fn set_status_as_int(&mut self, new: i64) -> Option<i64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("status".to_owned(), new.into())
                .and_then(|val| val.as_i64())
        })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 文件状态。`1` 表示禁用；只有禁用状态的文件才会返回该字段"]
    pub fn get_status_as_uint(&self) -> Option<u64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("status"))
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 文件状态。`1` 表示禁用；只有禁用状态的文件才会返回该字段"]
    pub fn set_status_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("status".to_owned(), new.into())
                .and_then(|val| val.as_u64())
        })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 对象 MD5 值，只有通过直传文件和追加文件 API 上传的文件，服务端确保有该字段返回"]
    pub fn get_md_5_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("md5"))
            .and_then(|val| val.as_str())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 对象 MD5 值，只有通过直传文件和追加文件 API 上传的文件，服务端确保有该字段返回"]
    pub fn set_md_5_as_str(&mut self, new: String) -> Option<String> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("md5".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 文件过期删除日期，UNIX 时间戳格式，文件在设置过期时间后才会返回该字段"]
    pub fn get_expiration_time_as_int(&self) -> Option<i64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("expiration"))
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 文件过期删除日期，UNIX 时间戳格式，文件在设置过期时间后才会返回该字段"]
    pub fn set_expiration_time_as_int(&mut self, new: i64) -> Option<i64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("expiration".to_owned(), new.into())
                .and_then(|val| val.as_i64())
        })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 文件过期删除日期，UNIX 时间戳格式，文件在设置过期时间后才会返回该字段"]
    pub fn get_expiration_time_as_uint(&self) -> Option<u64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("expiration"))
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 文件过期删除日期，UNIX 时间戳格式，文件在设置过期时间后才会返回该字段"]
    pub fn set_expiration_time_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("expiration".to_owned(), new.into())
                .and_then(|val| val.as_u64())
        })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 文件生命周期中转为低频存储的日期，UNIX 时间戳格式，文件在设置转低频后才会返回该字段"]
    pub fn get_transition_to_ia_time_as_int(&self) -> Option<i64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("transitionToIA"))
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 文件生命周期中转为低频存储的日期，UNIX 时间戳格式，文件在设置转低频后才会返回该字段"]
    pub fn set_transition_to_ia_time_as_int(&mut self, new: i64) -> Option<i64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("transitionToIA".to_owned(), new.into())
                .and_then(|val| val.as_i64())
        })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 文件生命周期中转为低频存储的日期，UNIX 时间戳格式，文件在设置转低频后才会返回该字段"]
    pub fn get_transition_to_ia_time_as_uint(&self) -> Option<u64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("transitionToIA"))
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 文件生命周期中转为低频存储的日期，UNIX 时间戳格式，文件在设置转低频后才会返回该字段"]
    pub fn set_transition_to_ia_time_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("transitionToIA".to_owned(), new.into())
                .and_then(|val| val.as_u64())
        })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 文件生命周期中转为归档存储的日期，UNIX 时间戳格式，文件在设置转归档后才会返回该字段"]
    pub fn get_transition_to_archive_time_as_int(&self) -> Option<i64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("transitionToARCHIVE"))
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 文件生命周期中转为归档存储的日期，UNIX 时间戳格式，文件在设置转归档后才会返回该字段"]
    pub fn set_transition_to_archive_time_as_int(&mut self, new: i64) -> Option<i64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("transitionToARCHIVE".to_owned(), new.into())
                .and_then(|val| val.as_i64())
        })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 文件生命周期中转为归档存储的日期，UNIX 时间戳格式，文件在设置转归档后才会返回该字段"]
    pub fn get_transition_to_archive_time_as_uint(&self) -> Option<u64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("transitionToARCHIVE"))
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 文件生命周期中转为归档存储的日期，UNIX 时间戳格式，文件在设置转归档后才会返回该字段"]
    pub fn set_transition_to_archive_time_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("transitionToARCHIVE".to_owned(), new.into())
                .and_then(|val| val.as_u64())
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
        credential: Box<dyn qiniu_http_client::credential::CredentialProvider>,
    ) -> SyncRequestBuilder {
        SyncRequestBuilder(
            self.0
                .get(&[qiniu_http_client::ServiceName::Rs], into_endpoints.into())
                .authorization(qiniu_http_client::Authorization::v2(credential))
                .idempotent(qiniu_http_client::Idempotent::Default)
                .path(crate::base_utils::join_path(
                    "/stat",
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
        credential: Box<dyn qiniu_http_client::credential::CredentialProvider>,
    ) -> AsyncRequestBuilder {
        AsyncRequestBuilder(
            self.0
                .async_get(&[qiniu_http_client::ServiceName::Rs], into_endpoints.into())
                .authorization(qiniu_http_client::Authorization::v2(credential))
                .idempotent(qiniu_http_client::Idempotent::Default)
                .path(crate::base_utils::join_path(
                    "/stat",
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
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<ResponseBody<'static>>> {
        let request = self.0;
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
    ) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<ResponseBody<'static>>> {
        let request = self.0;
        let response = request.call().await?;
        let parsed = response.parse_json().await?;
        Ok(parsed)
    }
}
