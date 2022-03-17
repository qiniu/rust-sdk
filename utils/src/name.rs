use super::{smallstr::SmallString, wrap_smallstr};
use serde::{
    de::{Deserialize, Deserializer, Error, Visitor},
    ser::{Serialize, Serializer},
};
use std::{
    borrow::{Borrow, BorrowMut, Cow},
    fmt,
    iter::FromIterator,
    ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo},
};

/// 存储空间名称
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BucketName {
    inner: SmallString<[u8; 64]>,
}

/// 对象名称
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectName {
    inner: SmallString<[u8; 96]>,
}

wrap_smallstr!(BucketName);
wrap_smallstr!(ObjectName);
