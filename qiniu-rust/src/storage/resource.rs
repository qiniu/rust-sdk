//! 资源管理模块
//!
//! 封装与资源管理相关的数据结构

use super::object::Object;

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
