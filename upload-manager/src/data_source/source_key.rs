use digest::{generic_array::GenericArray, Digest};
use sha1::Sha1;
use std::{
    fmt::{self, Debug},
    ops::Deref,
};

/// 数据源 KEY
///
/// 用于区分不同的数据源
pub struct SourceKey<A: Digest = Sha1>(GenericArray<u8, A::OutputSize>);

impl<A: Digest> SourceKey<A> {
    /// 创建数据源 KEY
    #[inline]
    pub fn new(array: impl Into<GenericArray<u8, A::OutputSize>>) -> Self {
        Self::from(array.into())
    }
}

impl<A: Digest> Deref for SourceKey<A> {
    type Target = GenericArray<u8, A::OutputSize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A: Digest> From<GenericArray<u8, A::OutputSize>> for SourceKey<A> {
    #[inline]
    fn from(array: GenericArray<u8, A::OutputSize>) -> Self {
        Self(array)
    }
}

impl<A: Digest> Debug for SourceKey<A> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SourceKey").field(&self.0).finish()
    }
}

impl<A: Digest> Clone for SourceKey<A> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
