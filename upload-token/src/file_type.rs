use duplicate::duplicate;
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;
use std::fmt::{self, Display};

/// 文件存储类型
#[derive(Copy, Clone, Debug, Eq, PartialEq, SmartDefault, Serialize, Deserialize)]
#[serde(from = "u8", into = "u8")]
#[non_exhaustive]
pub enum FileType {
    /// 标准存储
    #[default]
    Standard,

    /// 低频存储
    InfrequentAccess,

    /// 归档存储
    Archive,

    /// 深度归档存储
    DeepArchive,

    /// 其他存储类型
    Other(u8),
}

impl Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        usize::from(*self).fmt(f)
    }
}

#[duplicate(
    ty;
    [u8];
    [u16];
    [u32];
    [u64];
    [usize];
    [i8];
    [i16];
    [i32];
    [i64];
    [isize];
)]
impl From<FileType> for ty {
    #[inline]
    fn from(file_type: FileType) -> Self {
        match file_type {
            FileType::Standard => 0,
            FileType::InfrequentAccess => 1,
            FileType::Archive => 2,
            FileType::DeepArchive => 3,
            FileType::Other(ft) => ft as ty,
        }
    }
}

#[duplicate(
    ty;
    [u8];
    [u16];
    [u32];
    [u64];
)]
impl From<ty> for FileType {
    fn from(value: ty) -> Self {
        match value as u8 {
            0 => Self::Standard,
            1 => Self::InfrequentAccess,
            2 => Self::Archive,
            3 => Self::DeepArchive,
            ft => Self::Other(ft),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_serialization_and_deserialization() -> anyhow::Result<()> {
        assert_eq!(&serde_json::to_string(&FileType::Standard)?, "0");
        assert_eq!(&serde_json::to_string(&FileType::Other(5))?, "5");
        assert_eq!(serde_json::from_str::<FileType>("0")?, FileType::Standard);
        assert_eq!(serde_json::from_str::<FileType>("5")?, FileType::Other(5));
        Ok(())
    }
}
