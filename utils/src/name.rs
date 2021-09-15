use serde::{
    de::{Deserialize, Deserializer, Error, Visitor},
    ser::{Serialize, Serializer},
};
use smallstr::SmallString;
use std::{
    borrow::{Borrow, BorrowMut, Cow},
    fmt,
    iter::FromIterator,
    ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo},
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BucketName {
    inner: SmallString<[u8; 64]>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectName {
    inner: SmallString<[u8; 96]>,
}

macro_rules! impl_methods {
    ($name:ty) => {
        impl From<String> for $name {
            #[inline]
            fn from(key: String) -> Self {
                Self {
                    inner: SmallString::from_string(key),
                }
            }
        }

        impl<'a> From<&'a String> for $name {
            #[inline]
            fn from(key: &'a String) -> Self {
                Self {
                    inner: SmallString::from_str(key.as_str()),
                }
            }
        }

        impl From<Box<str>> for $name {
            #[inline]
            fn from(key: Box<str>) -> Self {
                Self {
                    inner: SmallString::from_string(key.into()),
                }
            }
        }

        impl<'a> From<&'a str> for $name {
            #[inline]
            fn from(key: &'a str) -> Self {
                Self {
                    inner: SmallString::from_str(key),
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    inner: SmallString::new(),
                }
            }
        }

        impl fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.inner.fmt(f)
            }
        }

        impl fmt::Debug for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.inner.fmt(f)
            }
        }

        impl AsRef<str> for $name {
            #[inline]
            fn as_ref(&self) -> &str {
                self.inner.as_ref()
            }
        }

        impl Borrow<str> for $name {
            #[inline]
            fn borrow(&self) -> &str {
                self.inner.borrow()
            }
        }

        impl BorrowMut<str> for $name {
            #[inline]
            fn borrow_mut(&mut self) -> &mut str {
                self.inner.borrow_mut()
            }
        }

        impl Deref for $name {
            type Target = str;

            #[inline]
            fn deref(&self) -> &Self::Target {
                self.inner.deref()
            }
        }

        impl DerefMut for $name {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.inner.deref_mut()
            }
        }

        impl Serialize for $name {
            #[inline]
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.inner)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            #[inline]
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct KeyVisitor;

                impl<'de> Visitor<'de> for KeyVisitor {
                    type Value = $name;

                    #[inline]
                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("a string")
                    }

                    #[inline]
                    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                        Ok(v.into())
                    }

                    #[inline]
                    fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
                        Ok(v.into())
                    }
                }

                deserializer.deserialize_str(KeyVisitor)
            }
        }

        impl Extend<char> for $name {
            #[inline]
            fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) {
                let iter = iter.into_iter();
                let (lo, _) = iter.size_hint();

                self.reserve(lo);

                for ch in iter {
                    self.push(ch);
                }
            }
        }

        impl<'a> Extend<&'a char> for $name {
            #[inline]
            fn extend<I: IntoIterator<Item = &'a char>>(&mut self, iter: I) {
                self.extend(iter.into_iter().cloned());
            }
        }

        impl<'a> Extend<Cow<'a, str>> for $name {
            #[inline]
            fn extend<I: IntoIterator<Item = Cow<'a, str>>>(&mut self, iter: I) {
                for s in iter {
                    self.push_str(&s);
                }
            }
        }

        impl<'a> Extend<&'a str> for $name {
            #[inline]
            fn extend<I: IntoIterator<Item = &'a str>>(&mut self, iter: I) {
                for s in iter {
                    self.push_str(s);
                }
            }
        }

        impl Extend<String> for $name {
            #[inline]
            fn extend<I: IntoIterator<Item = String>>(&mut self, iter: I) {
                for s in iter {
                    self.push_str(&s);
                }
            }
        }

        impl FromIterator<char> for $name {
            #[inline]
            fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
                let mut s = SmallString::new();
                s.extend(iter);
                Self { inner: s }
            }
        }

        impl<'a> FromIterator<&'a char> for $name {
            #[inline]
            fn from_iter<I: IntoIterator<Item = &'a char>>(iter: I) -> Self {
                let mut s = SmallString::new();
                s.extend(iter.into_iter().cloned());
                Self { inner: s }
            }
        }

        impl<'a> FromIterator<Cow<'a, str>> for $name {
            #[inline]
            fn from_iter<I: IntoIterator<Item = Cow<'a, str>>>(iter: I) -> Self {
                let mut s = SmallString::new();
                s.extend(iter);
                Self { inner: s }
            }
        }

        impl<'a> FromIterator<&'a str> for $name {
            #[inline]
            fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> Self {
                let mut s = SmallString::new();
                s.extend(iter);
                Self { inner: s }
            }
        }

        impl FromIterator<String> for $name {
            #[inline]
            fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
                let mut s = SmallString::new();
                s.extend(iter);
                Self { inner: s }
            }
        }

        impl $name {
            #[inline]
            pub fn new() -> Self {
                Self {
                    inner: SmallString::new(),
                }
            }

            #[inline]
            pub fn with_capacity(n: usize) -> Self {
                Self {
                    inner: SmallString::with_capacity(n),
                }
            }

            #[inline]
            pub fn len(&self) -> usize {
                self.inner.len()
            }

            #[inline]
            pub fn is_empty(&self) -> bool {
                self.inner.is_empty()
            }

            #[inline]
            pub fn capacity(&self) -> usize {
                self.inner.capacity()
            }

            #[inline]
            pub fn push(&mut self, ch: char) {
                self.inner.push(ch)
            }

            #[inline]
            pub fn push_str(&mut self, s: &str) {
                self.inner.push_str(s)
            }

            #[inline]
            pub fn pop(&mut self) -> Option<char> {
                self.inner.pop()
            }

            #[inline]
            pub fn truncate(&mut self, len: usize) {
                self.inner.truncate(len)
            }

            #[inline]
            pub fn as_str(&self) -> &str {
                self.inner.as_str()
            }

            #[inline]
            pub fn as_mut_str(&mut self) -> &mut str {
                self.inner.as_mut_str()
            }

            #[inline]
            pub fn as_bytes(&self) -> &[u8] {
                self.inner.as_bytes()
            }

            #[inline]
            pub fn clear(&mut self) {
                self.inner.clear()
            }

            #[inline]
            pub fn remove(&mut self, idx: usize) -> char {
                self.inner.remove(idx)
            }

            #[inline]
            pub fn insert(&mut self, idx: usize, ch: char) {
                self.inner.insert(idx, ch)
            }

            #[inline]
            pub fn insert_str(&mut self, idx: usize, s: &str) {
                self.inner.insert_str(idx, s)
            }

            #[inline]
            pub fn retain<F: FnMut(char) -> bool>(&mut self, f: F) {
                self.inner.retain(f)
            }

            #[inline]
            pub fn reserve(&mut self, additional: usize) {
                self.inner.reserve(additional)
            }

            #[inline]
            pub fn reserve_exact(&mut self, additional: usize) {
                self.inner.reserve_exact(additional)
            }

            #[inline]
            pub fn shrink_to_fit(&mut self) {
                self.inner.shrink_to_fit()
            }
        }
    };
}

impl_methods!(BucketName);
impl_methods!(ObjectName);

macro_rules! impl_index_str {
    ($name:ty, $index_type: ty) => {
        impl Index<$index_type> for $name {
            type Output = str;

            #[inline]
            fn index(&self, index: $index_type) -> &str {
                &self.as_str()[index]
            }
        }

        impl IndexMut<$index_type> for $name {
            #[inline]
            fn index_mut(&mut self, index: $index_type) -> &mut str {
                &mut self.as_mut_str()[index]
            }
        }
    };
}

impl_index_str!(BucketName, Range<usize>);
impl_index_str!(BucketName, RangeFrom<usize>);
impl_index_str!(BucketName, RangeTo<usize>);
impl_index_str!(BucketName, RangeFull);
impl_index_str!(ObjectName, Range<usize>);
impl_index_str!(ObjectName, RangeFrom<usize>);
impl_index_str!(ObjectName, RangeTo<usize>);
impl_index_str!(ObjectName, RangeFull);
