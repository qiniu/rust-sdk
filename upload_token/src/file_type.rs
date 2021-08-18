use duplicate::duplicate;
use std::{convert::TryFrom, error::Error, fmt};

/// 文件存储类型
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FileType {
    /// 标准存储
    Normal = 0,

    /// 低频存储
    InfrequentAccess = 1,

    /// 归档存储
    Glacial = 2,
}

impl Default for FileType {
    #[inline]
    fn default() -> Self {
        Self::Normal
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
        file_type as ty
    }
}

#[duplicate(
    ty;
    [u8];
    [u16];
    [u32];
    [u64];
)]
impl TryFrom<ty> for FileType {
    type Error = InvalidFileType;

    fn try_from(value: ty) -> Result<Self, Self::Error> {
        match value as u8 {
            0 => Ok(Self::Normal),
            1 => Ok(Self::InfrequentAccess),
            2 => Ok(Self::Glacial),
            _ => Err(InvalidFileType(value.into())),
        }
    }
}

/// 非法的文件类型
#[derive(Copy, Clone, Debug)]
pub struct InvalidFileType(pub u64);

impl fmt::Display for InvalidFileType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "File type ({}) is invalid", self.0)
    }
}

impl Error for InvalidFileType {}
