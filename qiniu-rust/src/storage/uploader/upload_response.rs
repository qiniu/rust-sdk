use matches::matches;
use serde_json::{map::Map, value::Index, Value};
use std::fmt;

/// 上传响应实例
///
/// 上传响应实例对上传响应中的响应体进行封装，提供一些辅助方法。
#[derive(Debug, Clone)]
pub struct UploadResponse(UploadResponseInner);

#[derive(Debug, Clone)]
enum UploadResponseInner {
    JSON(Value),
    Bytes(Vec<u8>),
}

impl UploadResponse {
    /// 当响应体为 JSON 时，且 JSON 体包含一个 `key` 属性，且属性值会字符串类型时，则返回该属性值
    pub fn key(&self) -> Option<&str> {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.get("key").and_then(|k| k.as_str()),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 当响应体为 JSON 时，且 JSON 体包含一个 `hash` 属性，且属性值会字符串类型时，则返回该属性值
    pub fn hash(&self) -> Option<&str> {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.get("hash").and_then(|k| k.as_str()),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 当响应体为 JSON 时，返回 true
    pub fn is_json_value(&self) -> bool {
        matches!(&self.0, UploadResponseInner::JSON(_))
    }

    /// 当响应体为 JSON 时，返回 JSON 值
    pub fn as_json_value(&self) -> Option<&Value> {
        match &self.0 {
            UploadResponseInner::JSON(value) => Some(value),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 当响应体为 JSON 时，返回 JSON 值
    pub fn into_json_value(self) -> Option<Value> {
        match self.0 {
            UploadResponseInner::JSON(value) => Some(value),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在时，则返回该属性对应的值
    pub fn get<I: Index>(&self, index: I) -> Option<&Value> {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.get(index),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为对象时，则返回 `true`
    pub fn is_object(&self) -> bool {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.is_object(),
            UploadResponseInner::Bytes(_) => false,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为对象时，则返回属性值
    pub fn as_object(&self) -> Option<&Map<String, Value>> {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.as_object(),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为数组时，则返回 `true`
    pub fn is_array(&self) -> bool {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.is_array(),
            UploadResponseInner::Bytes(_) => false,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为数组时，则返回属性值
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.as_array(),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为字符串时，则返回 `true`
    pub fn is_string(&self) -> bool {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.is_string(),
            UploadResponseInner::Bytes(_) => false,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为字符串时，则返回属性值
    pub fn as_str(&self) -> Option<&str> {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.as_str(),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为数字时，则返回 `true`
    pub fn is_number(&self) -> bool {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.is_number(),
            UploadResponseInner::Bytes(_) => false,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为合法的 64 位带符号整型时，则返回 `true`
    pub fn is_i64(&self) -> bool {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.is_i64(),
            UploadResponseInner::Bytes(_) => false,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为合法的 64 位无符号整型时，则返回 `true`
    pub fn is_u64(&self) -> bool {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.is_u64(),
            UploadResponseInner::Bytes(_) => false,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为合法的 64 位浮点型时，则返回 `true`
    pub fn is_f64(&self) -> bool {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.is_f64(),
            UploadResponseInner::Bytes(_) => false,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为合法的 64 位带符号整型时，则返回属性值
    pub fn as_i64(&self) -> Option<i64> {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.as_i64(),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为合法的 64 位无符号整型时，则返回属性值
    pub fn as_u64(&self) -> Option<u64> {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.as_u64(),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为合法的 64 位浮点型时，则返回属性值
    pub fn as_f64(&self) -> Option<f64> {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.as_f64(),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为布尔型时，则返回 `true`
    pub fn is_boolean(&self) -> bool {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.is_boolean(),
            UploadResponseInner::Bytes(_) => false,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为布尔型时，则返回属性值
    pub fn as_bool(&self) -> Option<bool> {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.as_bool(),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为 `NULL` 时，则返回 `true`
    pub fn is_null(&self) -> bool {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.is_null(),
            UploadResponseInner::Bytes(_) => false,
        }
    }

    /// 当响应体为 JSON 时，且指定的属性存在，值为 `NULL` 时，则返回 `Ok(())`
    pub fn as_null(&self) -> Option<()> {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.as_null(),
            UploadResponseInner::Bytes(_) => None,
        }
    }

    /// 将响应体转换为二进制数据
    pub fn to_bytes(&self) -> Vec<u8> {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.to_string().into(),
            UploadResponseInner::Bytes(bytes) => bytes.to_owned(),
        }
    }

    /// 将响应体转换为二进制数据
    pub fn into_bytes(self) -> Vec<u8> {
        match self.0 {
            UploadResponseInner::JSON(value) => value.to_string().into(),
            UploadResponseInner::Bytes(bytes) => bytes,
        }
    }
}

impl From<Value> for UploadResponse {
    fn from(v: Value) -> Self {
        UploadResponse(UploadResponseInner::JSON(v))
    }
}

impl From<Vec<u8>> for UploadResponse {
    fn from(v: Vec<u8>) -> Self {
        UploadResponse(UploadResponseInner::Bytes(v))
    }
}

impl fmt::Display for UploadResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            UploadResponseInner::JSON(value) => value.fmt(f),
            UploadResponseInner::Bytes(bytes) => String::from_utf8(bytes.to_owned()).map_err(|_| fmt::Error)?.fmt(f),
        }
    }
}
