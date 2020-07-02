use std::{convert::TryFrom, error::Error, fmt};

/// 文件存储类型
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FileType {
    /// 标准存储
    Normal,

    /// 低频存储
    InfrequentAccess,

    /// 归档存储
    Glacial,
}

impl Default for FileType {
    #[inline]
    fn default() -> Self {
        Self::Normal
    }
}

impl From<FileType> for u8 {
    #[inline]
    fn from(file_type: FileType) -> Self {
        match file_type {
            FileType::Normal => 0,
            FileType::InfrequentAccess => 1,
            FileType::Glacial => 2,
        }
    }
}

impl TryFrom<u8> for FileType {
    type Error = InvalidFileType;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Normal),
            1 => Ok(Self::InfrequentAccess),
            2 => Ok(Self::Glacial),
            _ => Err(InvalidFileType(value)),
        }
    }
}

/// 非法的文件类型
#[derive(Copy, Clone, Debug)]
pub struct InvalidFileType(pub u8);

impl fmt::Display for InvalidFileType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "File type ({}) is invalid", self.0)
    }
}

impl Error for InvalidFileType {}
