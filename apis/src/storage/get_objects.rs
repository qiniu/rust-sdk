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
    #[doc = "指定存储空间"]
    pub fn set_bucket_as_str(
        self,
        value: impl Into<qiniu_http_client::QueryPairValue<'a>>,
    ) -> Self {
        self.insert("bucket".into(), value.into())
    }
    #[inline]
    #[doc = "上一次列举返回的位置标记，作为本次列举的起点信息"]
    pub fn set_marker_as_str(
        self,
        value: impl Into<qiniu_http_client::QueryPairValue<'a>>,
    ) -> Self {
        self.insert("marker".into(), value.into())
    }
    #[inline]
    #[doc = "本次列举的条目数，范围为 1-1000"]
    pub fn set_limit_as_str(self, value: impl Into<qiniu_http_client::QueryPairValue<'a>>) -> Self {
        self.insert("limit".into(), value.into())
    }
    #[inline]
    #[doc = "指定前缀，只有资源名匹配该前缀的资源会被列出"]
    pub fn set_prefix_as_str(
        self,
        value: impl Into<qiniu_http_client::QueryPairValue<'a>>,
    ) -> Self {
        self.insert("prefix".into(), value.into())
    }
    #[inline]
    #[doc = "指定目录分隔符，列出所有公共前缀（模拟列出目录效果）"]
    pub fn set_delimiter_as_str(
        self,
        value: impl Into<qiniu_http_client::QueryPairValue<'a>>,
    ) -> Self {
        self.insert("delimiter".into(), value.into())
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
    #[doc = "获取 有剩余条目则返回非空字符串，作为下一次列举的参数传入，如果没有剩余条目则返回空字符串"]
    pub fn get_marker_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("marker"))
            .and_then(|val| val.as_str())
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 有剩余条目则返回非空字符串，作为下一次列举的参数传入，如果没有剩余条目则返回空字符串"]
    pub fn set_marker_as_str(&mut self, new: String) -> Option<String> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("marker".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
#[derive(Clone, Debug, serde :: Serialize, serde :: Deserialize)]
#[serde(transparent)]
#[doc = "公共前缀的数组"]
pub struct CommonPrefixes<'a>(std::borrow::Cow<'a, serde_json::Value>);
impl<'a> CommonPrefixes<'a> {
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn new(value: std::borrow::Cow<'a, serde_json::Value>) -> Self {
        Self(value)
    }
}
impl<'a> From<CommonPrefixes<'a>> for serde_json::Value {
    #[inline]
    fn from(val: CommonPrefixes<'a>) -> Self {
        val.0.into_owned()
    }
}
impl<'a> std::convert::AsRef<serde_json::Value> for CommonPrefixes<'a> {
    #[inline]
    fn as_ref(&self) -> &serde_json::Value {
        self.0.as_ref()
    }
}
impl<'a> std::convert::AsMut<serde_json::Value> for CommonPrefixes<'a> {
    #[inline]
    fn as_mut(&mut self) -> &mut serde_json::Value {
        self.0.to_mut()
    }
}
impl<'a> CommonPrefixes<'a> {
    #[inline]
    #[doc = "解析 JSON 得到 String 列表"]
    pub fn to_str_vec(&self) -> Vec<&str> {
        self.0
            .as_array()
            .unwrap()
            .iter()
            .map(|ele| ele.as_str().unwrap())
            .collect()
    }
}
impl<'a> From<Vec<String>> for CommonPrefixes<'a> {
    #[inline]
    fn from(val: Vec<String>) -> Self {
        Self(std::borrow::Cow::Owned(serde_json::Value::from(val)))
    }
}
impl<'a, 'b> From<&'a [String]> for CommonPrefixes<'b> {
    #[inline]
    fn from(val: &'a [String]) -> Self {
        Self(std::borrow::Cow::Owned(serde_json::Value::from(val)))
    }
}
impl<'a> CommonPrefixes<'a> {
    #[inline]
    pub fn len(&self) -> usize {
        self.0.as_array().unwrap().len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.as_array().unwrap().is_empty()
    }
}
impl<'a> CommonPrefixes<'a> {
    #[inline]
    #[doc = "在列表的指定位置插入 JSON String"]
    pub fn insert_str(&mut self, index: usize, val: String) {
        self.0
            .to_mut()
            .as_array_mut()
            .unwrap()
            .insert(index, val.into());
    }
}
impl<'a> CommonPrefixes<'a> {
    #[inline]
    #[doc = "在列表的指定位置移出 JSON String"]
    pub fn remove_as_str(&mut self, index: usize) -> Option<String> {
        match self.0.to_mut().as_array_mut().unwrap().remove(index) {
            serde_json::Value::String(s) => Some(s),
            _ => None,
        }
    }
}
impl<'a> CommonPrefixes<'a> {
    #[inline]
    #[doc = "在列表尾部追加 JSON String"]
    pub fn push_str(&mut self, val: String) {
        self.0.to_mut().as_array_mut().unwrap().push(val.into());
    }
}
impl<'a> CommonPrefixes<'a> {
    #[inline]
    #[doc = "在列表尾部取出 JSON String"]
    pub fn pop_as_str(&mut self) -> Option<String> {
        self.0
            .to_mut()
            .as_array_mut()
            .unwrap()
            .pop()
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 公共前缀的数组，如没有指定 delimiter 参数则不返回"]
    pub fn get_common_prefixes(&self) -> Option<CommonPrefixes> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("common_prefixes"))
            .map(std::borrow::Cow::Borrowed)
            .map(CommonPrefixes::new)
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 公共前缀的数组，如没有指定 delimiter 参数则不返回"]
    pub fn set_common_prefixes(&mut self, new: CommonPrefixes) -> Option<CommonPrefixes> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("common_prefixes".to_owned(), new.into())
                .map(std::borrow::Cow::Owned)
                .map(CommonPrefixes::new)
        })
    }
}
#[derive(Clone, Debug, serde :: Serialize, serde :: Deserialize)]
#[serde(transparent)]
#[doc = "条目的数组，不能用来判断是否还有剩余条目"]
pub struct ListedObjects<'a>(std::borrow::Cow<'a, serde_json::Value>);
impl<'a> ListedObjects<'a> {
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn new(value: std::borrow::Cow<'a, serde_json::Value>) -> Self {
        Self(value)
    }
}
impl<'a> From<ListedObjects<'a>> for serde_json::Value {
    #[inline]
    fn from(val: ListedObjects<'a>) -> Self {
        val.0.into_owned()
    }
}
impl<'a> std::convert::AsRef<serde_json::Value> for ListedObjects<'a> {
    #[inline]
    fn as_ref(&self) -> &serde_json::Value {
        self.0.as_ref()
    }
}
impl<'a> std::convert::AsMut<serde_json::Value> for ListedObjects<'a> {
    #[inline]
    fn as_mut(&mut self) -> &mut serde_json::Value {
        self.0.to_mut()
    }
}
#[derive(Clone, Debug, serde :: Serialize, serde :: Deserialize)]
#[serde(transparent)]
#[doc = "对象条目，包含对象的元信息"]
pub struct ListedObjectEntry<'a>(std::borrow::Cow<'a, serde_json::Value>);
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn new(value: std::borrow::Cow<'a, serde_json::Value>) -> Self {
        Self(value)
    }
}
impl<'a> From<ListedObjectEntry<'a>> for serde_json::Value {
    #[inline]
    fn from(val: ListedObjectEntry<'a>) -> Self {
        val.0.into_owned()
    }
}
impl<'a> std::convert::AsRef<serde_json::Value> for ListedObjectEntry<'a> {
    #[inline]
    fn as_ref(&self) -> &serde_json::Value {
        self.0.as_ref()
    }
}
impl<'a> std::convert::AsMut<serde_json::Value> for ListedObjectEntry<'a> {
    #[inline]
    fn as_mut(&mut self) -> &mut serde_json::Value {
        self.0.to_mut()
    }
}
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "获取 对象名称"]
    pub fn get_key_as_str(&self) -> &str {
        self.0
            .as_object()
            .unwrap()
            .get("key")
            .unwrap()
            .as_str()
            .unwrap()
    }
}
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "设置 对象名称"]
    pub fn set_key_as_str(&mut self, new: String) -> Option<String> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("key".to_owned(), new.into())
            .and_then(|val| match val {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
    }
}
impl<'a> ListedObjectEntry<'a> {
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
impl<'a> ListedObjectEntry<'a> {
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
impl<'a> ListedObjectEntry<'a> {
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
impl<'a> ListedObjectEntry<'a> {
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
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "获取 文件的哈希值"]
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
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "设置 文件的哈希值"]
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
impl<'a> ListedObjectEntry<'a> {
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
impl<'a> ListedObjectEntry<'a> {
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
impl<'a> ListedObjectEntry<'a> {
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
impl<'a> ListedObjectEntry<'a> {
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
impl<'a> ListedObjectEntry<'a> {
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
impl<'a> ListedObjectEntry<'a> {
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
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "获取 资源内容的唯一属主标识"]
    pub fn get_end_user_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("endUser"))
            .and_then(|val| val.as_str())
    }
}
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "设置 资源内容的唯一属主标识"]
    pub fn set_end_user_as_str(&mut self, new: String) -> Option<String> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("endUser".to_owned(), new.into())
                .and_then(|val| match val {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
        })
    }
}
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "获取 对象存储类型，`0` 表示普通存储，`1` 表示低频存储，`2` 表示归档存储"]
    pub fn get_type_as_int(&self) -> Option<i64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("type"))
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "设置 对象存储类型，`0` 表示普通存储，`1` 表示低频存储，`2` 表示归档存储"]
    pub fn set_type_as_int(&mut self, new: i64) -> Option<i64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("type".to_owned(), new.into())
                .and_then(|val| val.as_i64())
        })
    }
}
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "获取 对象存储类型，`0` 表示普通存储，`1` 表示低频存储，`2` 表示归档存储"]
    pub fn get_type_as_uint(&self) -> Option<u64> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("type"))
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "设置 对象存储类型，`0` 表示普通存储，`1` 表示低频存储，`2` 表示归档存储"]
    pub fn set_type_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0.to_mut().as_object_mut().and_then(|object| {
            object
                .insert("type".to_owned(), new.into())
                .and_then(|val| val.as_u64())
        })
    }
}
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "获取 文件的存储状态，即禁用状态和启用状态间的的互相转换，`0` 表示启用，`1`表示禁用"]
    pub fn get_unfreezing_status_as_int(&self) -> i64 {
        self.0
            .as_object()
            .unwrap()
            .get("status")
            .unwrap()
            .as_i64()
            .unwrap()
    }
}
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "设置 文件的存储状态，即禁用状态和启用状态间的的互相转换，`0` 表示启用，`1`表示禁用"]
    pub fn set_unfreezing_status_as_int(&mut self, new: i64) -> Option<i64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("status".to_owned(), new.into())
            .and_then(|val| val.as_i64())
    }
}
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "获取 文件的存储状态，即禁用状态和启用状态间的的互相转换，`0` 表示启用，`1`表示禁用"]
    pub fn get_unfreezing_status_as_uint(&self) -> u64 {
        self.0
            .as_object()
            .unwrap()
            .get("status")
            .unwrap()
            .as_u64()
            .unwrap()
    }
}
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "设置 文件的存储状态，即禁用状态和启用状态间的的互相转换，`0` 表示启用，`1`表示禁用"]
    pub fn set_unfreezing_status_as_uint(&mut self, new: u64) -> Option<u64> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("status".to_owned(), new.into())
            .and_then(|val| val.as_u64())
    }
}
impl<'a> ListedObjectEntry<'a> {
    #[inline]
    #[doc = "获取 对象 MD5 值，只有通过直传文件和追加文件 API 上传的文件，服务端确保有该字段返回"]
    pub fn get_md_5_as_str(&self) -> Option<&str> {
        self.0
            .as_object()
            .and_then(|obj| obj.get("md5"))
            .and_then(|val| val.as_str())
    }
}
impl<'a> ListedObjectEntry<'a> {
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
impl<'a> ListedObjects<'a> {
    #[inline]
    #[doc = "解析 JSON 得到 ListedObjectEntry 列表"]
    pub fn to_listed_object_entry_vec(&self) -> Vec<ListedObjectEntry> {
        self.0
            .as_array()
            .unwrap()
            .iter()
            .map(std::borrow::Cow::Borrowed)
            .map(ListedObjectEntry::new)
            .collect()
    }
}
impl<'a> From<Vec<ListedObjectEntry<'a>>> for ListedObjects<'a> {
    #[inline]
    fn from(val: Vec<ListedObjectEntry<'a>>) -> Self {
        Self(std::borrow::Cow::Owned(serde_json::Value::from(val)))
    }
}
impl<'a, 'b> From<&'a [ListedObjectEntry<'a>]> for ListedObjects<'b> {
    #[inline]
    fn from(val: &'a [ListedObjectEntry<'a>]) -> Self {
        Self(std::borrow::Cow::Owned(serde_json::Value::from(val)))
    }
}
impl<'a> ListedObjects<'a> {
    #[inline]
    pub fn len(&self) -> usize {
        self.0.as_array().unwrap().len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.as_array().unwrap().is_empty()
    }
}
impl<'a> ListedObjects<'a> {
    #[inline]
    #[doc = "在列表的指定位置插入 JSON ListedObjectEntry"]
    pub fn insert_listed_object_entry(&mut self, index: usize, val: ListedObjectEntry<'a>) {
        self.0
            .to_mut()
            .as_array_mut()
            .unwrap()
            .insert(index, val.into());
    }
}
impl<'a> ListedObjects<'a> {
    #[inline]
    #[doc = "在列表的指定位置移出 JSON ListedObjectEntry"]
    pub fn remove_as_listed_object_entry(&mut self, index: usize) -> ListedObjectEntry {
        ListedObjectEntry::new(std::borrow::Cow::Owned(
            self.0.to_mut().as_array_mut().unwrap().remove(index),
        ))
    }
}
impl<'a> ListedObjects<'a> {
    #[inline]
    #[doc = "在列表尾部追加 JSON ListedObjectEntry"]
    pub fn push_listed_object_entry(&mut self, val: ListedObjectEntry<'a>) {
        self.0.to_mut().as_array_mut().unwrap().push(val.into());
    }
}
impl<'a> ListedObjects<'a> {
    #[inline]
    #[doc = "在列表尾部取出 JSON ListedObjectEntry"]
    pub fn pop_listed_object_entry(&mut self) -> Option<ListedObjectEntry> {
        self.0
            .to_mut()
            .as_array_mut()
            .unwrap()
            .pop()
            .map(std::borrow::Cow::Owned)
            .map(ListedObjectEntry::new)
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "获取 条目的数组，不能用来判断是否还有剩余条目"]
    pub fn get_items(&self) -> ListedObjects {
        ListedObjects::new(std::borrow::Cow::Borrowed(
            self.0.as_object().unwrap().get("items").unwrap(),
        ))
    }
}
impl<'a> ResponseBody<'a> {
    #[inline]
    #[doc = "设置 条目的数组，不能用来判断是否还有剩余条目"]
    pub fn set_items(&mut self, new: ListedObjects) -> Option<ListedObjects> {
        self.0
            .to_mut()
            .as_object_mut()
            .unwrap()
            .insert("items".to_owned(), new.into())
            .map(std::borrow::Cow::Owned)
            .map(ListedObjects::new)
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
        credential: Box<dyn qiniu_http_client::credential::CredentialProvider>,
    ) -> SyncRequestBuilder {
        SyncRequestBuilder(
            self.0
                .get(
                    &[qiniu_http_client::ServiceName::Rsf],
                    into_endpoints.into(),
                )
                .authorization(qiniu_http_client::Authorization::v2(credential))
                .idempotent(qiniu_http_client::Idempotent::Default)
                .path("list")
                .accept_json(),
        )
    }
    #[inline]
    #[cfg(feature = "async")]
    pub fn new_async_request(
        &self,
        into_endpoints: impl Into<qiniu_http_client::IntoEndpoints<'client>>,
        credential: Box<dyn qiniu_http_client::credential::CredentialProvider>,
    ) -> AsyncRequestBuilder {
        AsyncRequestBuilder(
            self.0
                .async_get(
                    &[qiniu_http_client::ServiceName::Rsf],
                    into_endpoints.into(),
                )
                .authorization(qiniu_http_client::Authorization::v2(credential))
                .idempotent(qiniu_http_client::Idempotent::Default)
                .path("list")
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
