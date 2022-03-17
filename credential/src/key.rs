use qiniu_utils::{smallstr::SmallString, wrap_smallstr};
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

/// 七牛 Access Key
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccessKey {
    inner: SmallString<[u8; 64]>,
}

/// 七牛 Secret Key
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecretKey {
    inner: SmallString<[u8; 64]>,
}

wrap_smallstr!(AccessKey);
wrap_smallstr!(SecretKey);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str as from_json_str, to_string as to_json_string};

    #[test]
    fn test_serialize() -> anyhow::Result<()> {
        let access_key = AccessKey::from("access_key");
        assert_eq!(to_json_string(&access_key)?, "\"access_key\"");
        Ok(())
    }

    #[test]
    fn test_deserialize() -> anyhow::Result<()> {
        let access_key: AccessKey = from_json_str("\"access_key\"")?;
        assert_eq!(access_key.as_str(), "access_key");
        Ok(())
    }
}
