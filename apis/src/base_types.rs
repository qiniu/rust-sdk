use serde_json::{Map, Value};
use std::{
    borrow::{Borrow, Cow},
    convert::{AsMut, AsRef},
    hash::Hash,
    iter::{FromIterator, IntoIterator},
};

#[derive(Clone, Debug)]
/// 通用类型，表示字符串 => 字符串的映射结构
pub struct StringMap<'a>(Cow<'a, Value>);

impl<'a> StringMap<'a> {
    #[allow(dead_code)]
    pub(crate) fn new(value: Cow<'a, Value>) -> Self {
        Self(value)
    }
}

impl<'a> From<StringMap<'a>> for Value {
    #[inline]
    fn from(val: StringMap<'a>) -> Self {
        val.0.into_owned()
    }
}

impl<'a> AsRef<Value> for StringMap<'a> {
    #[inline]
    fn as_ref(&self) -> &Value {
        self.0.as_ref()
    }
}

impl<'a> AsMut<Value> for StringMap<'a> {
    #[inline]
    fn as_mut(&mut self) -> &mut Value {
        self.0.to_mut()
    }
}

impl<'a> StringMap<'a> {
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
        self.0.to_mut().as_object_mut().and_then(|object| {
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
            .to_mut()
            .as_object_mut()
            .and_then(|object| object.remove(key))
            .and_then(|val| match val {
                Value::String(s) => Some(s),
                _ => None,
            })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.as_object().unwrap().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.as_object().unwrap().is_empty()
    }
}

impl<'a, T> From<T> for StringMap<'a>
where
    T: IntoIterator<Item = (String, String)>,
{
    #[inline]
    fn from(m: T) -> Self {
        Self(Cow::Owned(Value::Object(Map::from_iter(
            m.into_iter()
                .map(|(key, value)| (key, Value::String(value))),
        ))))
    }
}
