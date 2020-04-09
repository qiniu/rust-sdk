//! 资源管理模块
//!
//! 封装与资源管理相关的数据结构

use super::object::Object;
use serde::Deserialize;
use std::{
    fmt,
    time::{Duration, SystemTime},
};

pub(super) trait ToURI {
    fn to_uri(&self) -> String;
}

pub(super) struct Stat<'a> {
    object: &'a Object,
}

impl<'a> Stat<'a> {
    pub(super) fn new(object: &'a Object) -> Self {
        Self { object }
    }
}

impl ToURI for Stat<'_> {
    fn to_uri(&self) -> String {
        "/stat/".to_owned() + self.object.encoded_entry_uri()
    }
}

pub(super) struct Delete<'a> {
    object: &'a Object,
}

impl<'a> Delete<'a> {
    pub(super) fn new(object: &'a Object) -> Self {
        Self { object }
    }
}

impl ToURI for Delete<'_> {
    fn to_uri(&self) -> String {
        "/delete/".to_owned() + self.object.encoded_entry_uri()
    }
}

/// 对象详细信息
#[derive(Deserialize)]
pub struct ObjectInfo {
    fsize: u64,

    hash: String,

    #[serde(rename(deserialize = "mimeType"))]
    mime_type: String,

    #[serde(rename(deserialize = "putTime"))]
    put_time: u64,
}

impl ObjectInfo {
    /// 获取对象尺寸
    ///
    /// 单位为字节
    #[inline]
    pub fn size(&self) -> u64 {
        self.fsize
    }

    /// 获取对象 HASH 值
    ///
    /// 一般返回该对象内容的 Etag 值
    #[inline]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// 获取对象的 MIME 类型
    #[inline]
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }

    /// 获取对象的创建时间
    #[inline]
    pub fn uploaded_at(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_nanos(self.put_time * 100)
    }

    /// 获取对象的创建时间
    ///
    /// 与 `uploaded_at()` 返回相同的内容
    #[inline]
    pub fn put_time(&self) -> SystemTime {
        self.uploaded_at()
    }
}

impl fmt::Debug for ObjectInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ObjectInfo")
            .field("size", &self.size())
            .field("hash", &self.hash())
            .field("mime_type", &self.mime_type())
            .field("put_time", &self.put_time())
            .finish()
    }
}
