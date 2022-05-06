use serde_json::{Map, Value};
use std::{
    borrow::Borrow,
    convert::{AsMut, AsRef},
    hash::Hash,
    iter::{FromIterator, IntoIterator},
};

#[derive(Clone, Debug)]
/// 通用类型，表示字符串 => 字符串的映射结构
pub struct StringMap(Value);

impl StringMap {
    #[allow(dead_code)]
    pub(crate) fn new(value: Value) -> Self {
        Self(value)
    }
}

impl Default for StringMap {
    #[inline]
    fn default() -> Self {
        Self(Value::Object(Default::default()))
    }
}

impl From<StringMap> for Value {
    #[inline]
    fn from(val: StringMap) -> Self {
        val.0
    }
}

impl AsRef<Value> for StringMap {
    #[inline]
    fn as_ref(&self) -> &Value {
        &self.0
    }
}

impl AsMut<Value> for StringMap {
    #[inline]
    fn as_mut(&mut self) -> &mut Value {
        &mut self.0
    }
}

impl StringMap {
    #[doc = "根据 Key 获取相应的不可变 String 引用"]
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&str>
    where
        String: Borrow<Q>,
        Q: Ord + Eq + Hash,
    {
        self.0
            .as_object()
            .and_then(|object| object.get(key))
            .and_then(|val| val.as_str())
    }

    #[doc = "根据 Key 设置 String 值"]
    pub fn insert(&mut self, key: String, new: String) -> Option<String> {
        self.0.as_object_mut().and_then(|object| {
            object.insert(key, new.into()).and_then(|old| match old {
                Value::String(s) => Some(s),
                _ => None,
            })
        })
    }

    #[doc = "根据 Key 获取相应的不可变 String 引用"]
    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<String>
    where
        String: Borrow<Q>,
        Q: Ord + Eq + Hash,
    {
        self.0
            .as_object_mut()
            .and_then(|object| object.remove(key))
            .and_then(|val| match val {
                Value::String(s) => Some(s),
                _ => None,
            })
    }

    #[doc = "获取映射结构的元素数量"]
    pub fn len(&self) -> usize {
        self.0.as_object().unwrap().len()
    }

    #[doc = "映射结构是否为空"]
    pub fn is_empty(&self) -> bool {
        self.0.as_object().unwrap().is_empty()
    }
}

impl<T> From<T> for StringMap
where
    T: IntoIterator<Item = (String, String)>,
{
    #[inline]
    fn from(m: T) -> Self {
        Self(Value::Object(Map::from_iter(
            m.into_iter().map(|(key, value)| (key, Value::String(value))),
        )))
    }
}
