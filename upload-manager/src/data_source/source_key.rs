use digest::{generic_array::GenericArray, OutputSizeUser};
use sha1::Sha1;
use std::{
    fmt::{self, Debug},
    ops::Deref,
};

pub struct SourceKey<A: OutputSizeUser = Sha1>(GenericArray<u8, A::OutputSize>);

impl<A: OutputSizeUser> SourceKey<A> {
    #[inline]
    pub fn new(array: impl Into<GenericArray<u8, A::OutputSize>>) -> Self {
        Self::from(array.into())
    }
}

impl<A: OutputSizeUser> Deref for SourceKey<A> {
    type Target = GenericArray<u8, A::OutputSize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A: OutputSizeUser> From<GenericArray<u8, A::OutputSize>> for SourceKey<A> {
    #[inline]
    fn from(array: GenericArray<u8, A::OutputSize>) -> Self {
        Self(array)
    }
}

impl<A: OutputSizeUser> Debug for SourceKey<A> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SourceKey").field(&self.0).finish()
    }
}

impl<A: OutputSizeUser> Clone for SourceKey<A> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
