#[derive(Debug, Clone)]
#[doc = "调用 API 所用的路径参数"]
pub struct PathParams {
    r#bucket_name: Option<std::borrow::Cow<'static, str>>,
    r#object_name: Option<std::borrow::Cow<'static, str>>,
    r#upload_id: Option<std::borrow::Cow<'static, str>>,
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
        if let Some(segment) = self.r#bucket_name {
            all_segments.push(segment);
        }
        all_segments.push(std::borrow::Cow::Borrowed("objects"));
        all_segments.push(
            self.r#object_name
                .unwrap_or(std::borrow::Cow::Borrowed("~")),
        );
        if let Some(segment) = self.r#upload_id {
            all_segments.push(std::borrow::Cow::Borrowed("uploads"));
            all_segments.push(segment);
        }
        all_segments.extend(self.extended_segments);
        all_segments
    }
}
impl PathParams {
    #[inline]
    #[doc = "存储空间名称"]
    pub fn set_bucket_name_as_str(
        mut self,
        value: impl Into<std::borrow::Cow<'static, str>>,
    ) -> Self {
        self.r#bucket_name = Some(value.into());
        self
    }
    #[inline]
    #[doc = "对象名称"]
    pub fn set_object_name_as_str(
        mut self,
        value: impl Into<std::borrow::Cow<'static, str>>,
    ) -> Self {
        self.r#object_name = Some(qiniu_utils::base64::urlsafe(value.into().as_bytes()).into());
        self
    }
    #[inline]
    #[doc = "在服务端申请的 Multipart Upload 任务 id"]
    pub fn set_upload_id_as_str(
        mut self,
        value: impl Into<std::borrow::Cow<'static, str>>,
    ) -> Self {
        self.r#upload_id = Some(value.into());
        self
    }
}
#[derive(Debug, Clone)]
#[doc = "调用 API 所用的 URL 查询参数"]
pub struct QueryParams<'a> {
    map: std::collections::HashMap<
        qiniu_http_client::QueryPairKey<'a>,
        qiniu_http_client::QueryPairValue<'a>,
    >,
}
impl<'a> QueryParams<'a> {
    #[inline]
    fn insert(
        mut self,
        query_pair_key: qiniu_http_client::QueryPairKey<'a>,
        query_pair_value: qiniu_http_client::QueryPairValue<'a>,
    ) -> Self {
        self.map.insert(query_pair_key, query_pair_value);
        self
    }
    #[inline]
    fn build(self) -> qiniu_http_client::QueryPairs<'a> {
        qiniu_http_client::QueryPairs::from_iter(self.map)
    }
}
impl<'a> From<QueryParams<'a>> for qiniu_http_client::QueryPairs<'a> {
    #[inline]
    fn from(map: QueryParams<'a>) -> Self {
        map.build()
    }
}
impl<'a> QueryParams<'a> {
    #[inline]
    #[doc = "max-parts"]
    pub fn set_max_parts_as_int(self, value: i64) -> Self {
        self.insert(
            "响应中的最大分片数目。默认值：1000，最大值：1000".into(),
            value.to_string().into(),
        )
    }
    #[inline]
    #[doc = "max-parts"]
    pub fn set_max_parts_as_uint(self, value: u64) -> Self {
        self.insert(
            "响应中的最大分片数目。默认值：1000，最大值：1000".into(),
            value.to_string().into(),
        )
    }
    #[inline]
    #[doc = "part-number_marker"]
    pub fn set_part_number_marker_as_int(self, value: i64) -> Self {
        self.insert(
            "指定列举的起始位置，只有 partNumber 值大于该参数的分片会被列出".into(),
            value.to_string().into(),
        )
    }
    #[inline]
    #[doc = "part-number_marker"]
    pub fn set_part_number_marker_as_uint(self, value: u64) -> Self {
        self.insert(
            "指定列举的起始位置，只有 partNumber 值大于该参数的分片会被列出".into(),
            value.to_string().into(),
        )
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
    #[doc = "获取 在服务端申请的 Multipart Upload 任务 id"]
    pub fn get_upload_id_as_str(&self) -> &str {
        self.0
            .as_object()
            .unwrap()
            .get("uploadId")
            .unwrap()
            .as_str()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 在服务端申请的 Multipart Upload 任务 id"]
    pub fn set_upload_id_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("uploadId".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 UploadId 的过期时间 UNIX 时间戳，过期之后 UploadId 不可用"]
    pub fn get_expired_at_as_int(&self) -> i64 {
        self.0
            .as_object()
            .unwrap()
            .get("expireAt")
            .unwrap()
            .as_i64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 UploadId 的过期时间 UNIX 时间戳，过期之后 UploadId 不可用"]
    pub fn set_expired_at_as_int(&mut self, new: i64) -> Option<i64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("expireAt".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 UploadId 的过期时间 UNIX 时间戳，过期之后 UploadId 不可用"]
    pub fn get_expired_at_as_uint(&self) -> u64 {
        self.0
            .as_object()
            .unwrap()
            .get("expireAt")
            .unwrap()
            .as_u64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 UploadId 的过期时间 UNIX 时间戳，过期之后 UploadId 不可用"]
    pub fn set_expired_at_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("expireAt".to_owned(), new.into())
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 下次继续列举的起始位置，0 表示列举结束，没有更多分片"]
    pub fn get_part_number_marker_as_int(&self) -> i64 {
        self.0
            .as_object()
            .unwrap()
            .get("partNumberMarker")
            .unwrap()
            .as_i64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 下次继续列举的起始位置，0 表示列举结束，没有更多分片"]
    pub fn set_part_number_marker_as_int(&mut self, new: i64) -> Option<i64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("partNumberMarker".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 下次继续列举的起始位置，0 表示列举结束，没有更多分片"]
    pub fn get_part_number_marker_as_uint(&self) -> u64 {
        self.0
            .as_object()
            .unwrap()
            .get("partNumberMarker")
            .unwrap()
            .as_u64()
            .unwrap()
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 下次继续列举的起始位置，0 表示列举结束，没有更多分片"]
    pub fn set_part_number_marker_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("partNumberMarker".to_owned(), new.into())
            .and_then(|val| val.as_u64())
    }
}
#[derive(Clone, Debug, serde :: Serialize, serde :: Deserialize)]
#[serde(transparent)]
#[doc = "所有已经上传的分片信息"]
pub struct ListedParts<'a>(std::borrow::Cow<'a, serde_json::Value>);
impl<'a> ListedParts<'a> {
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn new(value: std::borrow::Cow<'a, serde_json::Value>) -> Self {
        Self(value)
    }
}
impl<'a> From<ListedParts<'a>> for serde_json::Value {
    #[inline]
    fn from(val: ListedParts<'a>) -> Self {
        val.0.into_owned()
    }
}
impl<'a> std::convert::AsRef<serde_json::Value> for ListedParts<'a> {
    #[inline]
    fn as_ref(&self) -> &serde_json::Value {
        self.0.as_ref()
    }
}
impl<'a> std::convert::AsMut<serde_json::Value> for ListedParts<'a> {
    #[inline]
    fn as_mut(&mut self) -> &mut serde_json::Value {
        self.0.to_mut()
    }
}
#[derive(Clone, Debug, serde :: Serialize, serde :: Deserialize)]
#[serde(transparent)]
#[doc = "单个已经上传的分片信息"]
pub struct ListedPartInfo<'a>(std::borrow::Cow<'a, serde_json::Value>);
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn new(value: std::borrow::Cow<'a, serde_json::Value>) -> Self {
        Self(value)
    }
}
impl<'a> From<ListedPartInfo<'a>> for serde_json::Value {
    #[inline]
    fn from(val: ListedPartInfo<'a>) -> Self {
        val.0.into_owned()
    }
}
impl<'a> std::convert::AsRef<serde_json::Value> for ListedPartInfo<'a> {
    #[inline]
    fn as_ref(&self) -> &serde_json::Value {
        self.0.as_ref()
    }
}
impl<'a> std::convert::AsMut<serde_json::Value> for ListedPartInfo<'a> {
    #[inline]
    fn as_mut(&mut self) -> &mut serde_json::Value {
        self.0.to_mut()
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "获取 分片大小"]
    pub fn get_size_as_int(&self) -> i64 {
        self.0
            .as_object()
            .unwrap()
            .get("size")
            .unwrap()
            .as_i64()
            .unwrap()
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "设置 分片大小"]
    pub fn set_size_as_int(&mut self, new: i64) -> Option<i64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("size".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "获取 分片大小"]
    pub fn get_size_as_uint(&self) -> u64 {
        self.0
            .as_object()
            .unwrap()
            .get("size")
            .unwrap()
            .as_u64()
            .unwrap()
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "设置 分片大小"]
    pub fn set_size_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("size".to_owned(), new.into())
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "获取 分片内容的 etag"]
    pub fn get_etag_as_str(&self) -> &str {
        self.0
            .as_object()
            .unwrap()
            .get("etag")
            .unwrap()
            .as_str()
            .unwrap()
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "设置 分片内容的 etag"]
    pub fn set_etag_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("etag".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "获取 每一个上传的分片都有一个标识它的号码"]
    pub fn get_part_number_as_int(&self) -> i64 {
        self.0
            .as_object()
            .unwrap()
            .get("partNumber")
            .unwrap()
            .as_i64()
            .unwrap()
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "设置 每一个上传的分片都有一个标识它的号码"]
    pub fn set_part_number_as_int(&mut self, new: i64) -> Option<i64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("partNumber".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "获取 每一个上传的分片都有一个标识它的号码"]
    pub fn get_part_number_as_uint(&self) -> u64 {
        self.0
            .as_object()
            .unwrap()
            .get("partNumber")
            .unwrap()
            .as_u64()
            .unwrap()
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "设置 每一个上传的分片都有一个标识它的号码"]
    pub fn set_part_number_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("partNumber".to_owned(), new.into())
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "获取 分片上传时间 UNIX 时间戳"]
    pub fn get_put_time_as_int(&self) -> i64 {
        self.0
            .as_object()
            .unwrap()
            .get("put_time")
            .unwrap()
            .as_i64()
            .unwrap()
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "设置 分片上传时间 UNIX 时间戳"]
    pub fn set_put_time_as_int(&mut self, new: i64) -> Option<i64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("put_time".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "获取 分片上传时间 UNIX 时间戳"]
    pub fn get_put_time_as_uint(&self) -> u64 {
        self.0
            .as_object()
            .unwrap()
            .get("put_time")
            .unwrap()
            .as_u64()
            .unwrap()
    }
}
impl<'a> ListedPartInfo<'a> {
    #[inline]
    #[doc = "设置 分片上传时间 UNIX 时间戳"]
    pub fn set_put_time_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("put_time".to_owned(), new.into())
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ListedParts<'a> {
    #[inline]
    #[doc = "解析 JSON 得到 ListedPartInfo 列表"]
    pub fn to_listed_part_info_vec(&self) -> Vec<ListedPartInfo> {
        self.0
            .as_array()
            .unwrap()
            .iter()
            .map(std::borrow::Cow::Borrowed)
            .map(ListedPartInfo::new)
            .collect()
    }
}
impl<'a> From<Vec<ListedPartInfo<'a>>> for ListedParts<'a> {
    #[inline]
    fn from(val: Vec<ListedPartInfo<'a>>) -> Self {
        Self(std::borrow::Cow::Owned(serde_json::Value::from(val)))
    }
}
impl<'a, 'b> From<&'a [ListedPartInfo<'a>]> for ListedParts<'b> {
    #[inline]
    fn from(val: &'a [ListedPartInfo<'a>]) -> Self {
        Self(std::borrow::Cow::Owned(serde_json::Value::from(val)))
    }
}
impl<'a> ListedParts<'a> {
    #[inline]
    pub fn len(&self) -> usize {
        self.0.as_array().unwrap().len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.as_array().unwrap().is_empty()
    }
}
impl<'a> ListedParts<'a> {
    #[inline]
    #[doc = "在列表的指定位置插入 JSON ListedPartInfo"]
    pub fn insert_listed_part_info(&mut self, index: usize, val: ListedPartInfo<'a>) {
        self.0
            .to_mut()
            .as_array_mut()
            .unwrap()
            .insert(index, val.into());
    }
}
impl<'a> ListedParts<'a> {
    #[inline]
    #[doc = "在列表的指定位置移出 JSON ListedPartInfo"]
    pub fn remove_as_listed_part_info(&mut self, index: usize) -> ListedPartInfo {
        ListedPartInfo::new(std::borrow::Cow::Owned(
            self.0.to_mut().as_array_mut().unwrap().remove(index),
        ))
    }
}
impl<'a> ListedParts<'a> {
    #[inline]
    #[doc = "在列表尾部追加 JSON ListedPartInfo"]
    pub fn push_listed_part_info(&mut self, val: ListedPartInfo<'a>) {
        self.0.to_mut().as_array_mut().unwrap().push(val.into());
    }
}
impl<'a> ListedParts<'a> {
    #[inline]
    #[doc = "在列表尾部取出 JSON ListedPartInfo"]
    pub fn pop_listed_part_info(&mut self) -> Option<ListedPartInfo> {
        self.0
            .to_mut()
            .as_array_mut()
            .unwrap()
            .pop()
            .map(std::borrow::Cow::Owned)
            .map(ListedPartInfo::new)
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 返回所有已经上传成功的分片信息"]
    pub fn get_parts(&self) -> ListedParts {
        ListedParts::new(std::borrow::Cow::Borrowed(
            self.0.as_object().unwrap().get("parts").unwrap(),
        ))
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 返回所有已经上传成功的分片信息"]
    pub fn set_parts(&mut self, new: ListedParts) -> Option<ListedParts> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("parts".to_owned(), new.into())
            .map(std::borrow::Cow::Owned)
            .map(ListedParts::new)
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
        upload_token: std::sync::Arc<dyn qiniu_http_client::upload_token::UploadTokenProvider>,
    ) -> SyncRequestBuilder {
        SyncRequestBuilder(
            self.0
                .get(&[qiniu_http_client::ServiceName::Up], into_endpoints.into())
                .authorization(qiniu_http_client::Authorization::uptoken(upload_token))
                .idempotent(qiniu_http_client::Idempotent::Default)
                .path(crate::base_utils::join_path(
                    "/buckets",
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
        upload_token: std::sync::Arc<dyn qiniu_http_client::upload_token::UploadTokenProvider>,
    ) -> AsyncRequestBuilder {
        AsyncRequestBuilder(
            self.0
                .async_get(&[qiniu_http_client::ServiceName::Up], into_endpoints.into())
                .authorization(qiniu_http_client::Authorization::uptoken(upload_token))
                .idempotent(qiniu_http_client::Idempotent::Default)
                .path(crate::base_utils::join_path(
                    "/buckets",
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
    pub fn query_pairs(mut self, query_pairs: QueryParams<'req>) -> Self {
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
    pub fn query_pairs(mut self, query_pairs: QueryParams<'req>) -> Self {
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
