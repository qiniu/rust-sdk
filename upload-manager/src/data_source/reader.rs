use super::{super::PartSize, seekable::SeekableSource, SourceKey};
use auto_impl::auto_impl;
use digest::{Digest, Output as DigestOutput};
use dyn_clonable::clonable;
use qiniu_apis::http::Reset;
use std::{
    fmt::Debug,
    io::{copy as io_copy, sink as io_sink, Cursor, Read, Result as IoResult},
    num::NonZeroUsize,
};

/// 数据源接口
///
/// 提供上传所用的数据源
///
/// 该 Trait 的异步版本为 [`AsyncDataSource`]。
///
/// [`AsyncDataSource`]: super::AsyncDataSource
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait DataSource<A: Digest>: Clone + Debug + Sync + Send {
    /// 数据源切片
    fn slice(&self, size: PartSize) -> IoResult<Option<DataSourceReader>>;

    /// 重置数据源
    fn reset(&self) -> IoResult<()>;

    /// 获取数据源 KEY
    ///
    /// 用于区分不同的数据源
    #[inline]
    fn source_key(&self) -> IoResult<Option<SourceKey<A>>> {
        Ok(None)
    }

    /// 获取数据源大小
    fn total_size(&self) -> IoResult<Option<u64>>;
}

pub(crate) trait Digestible<A: Digest>: Read + Reset {
    fn digest(&mut self) -> IoResult<DigestOutput<A>> {
        struct ReadWithDigest<A, R> {
            reader: R,
            digest: A,
        }

        impl<A: Digest, R: Read> Read for ReadWithDigest<A, R> {
            fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
                let size = self.reader.read(buf)?;
                self.digest.update(buf);
                Ok(size)
            }
        }

        let mut hasher = ReadWithDigest {
            reader: self,
            digest: A::new(),
        };
        io_copy(&mut hasher, &mut io_sink())?;
        hasher.reader.reset()?;
        Ok(hasher.digest.finalize())
    }
}

impl<T: Read + Reset, A: Digest> Digestible<A> for T {}

/// 数据源阅读器
///
/// 提供阻塞读取接口
#[derive(Debug)]
pub struct DataSourceReader {
    inner: DataSourceReaderInner,
    part_number: NonZeroUsize,
}

#[derive(Debug)]
enum DataSourceReaderInner {
    ReadSeekable(SeekableSource),
    Readable { data: Cursor<Vec<u8>>, offset: u64 },
}

impl DataSourceReader {
    /// 创建可寻址的数据源阅读器
    #[inline]
    pub fn seekable(part_number: NonZeroUsize, source: SeekableSource) -> Self {
        Self {
            inner: DataSourceReaderInner::ReadSeekable(source),
            part_number,
        }
    }

    /// 创建不可寻址的数据源阅读器
    #[inline]
    pub fn unseekable(part_number: NonZeroUsize, data: Vec<u8>, offset: u64) -> Self {
        Self {
            inner: DataSourceReaderInner::Readable {
                data: Cursor::new(data),
                offset,
            },
            part_number,
        }
    }

    pub(crate) fn part_number(&self) -> NonZeroUsize {
        self.part_number
    }

    pub(crate) fn offset(&self) -> u64 {
        match &self.inner {
            DataSourceReaderInner::ReadSeekable(source) => source.offset(),
            DataSourceReaderInner::Readable { offset, .. } => *offset,
        }
    }

    pub(crate) fn len(&self) -> IoResult<u64> {
        match &self.inner {
            DataSourceReaderInner::ReadSeekable(source) => source.len(),
            DataSourceReaderInner::Readable { data, .. } => Ok(data.get_ref().len() as u64),
        }
    }
}

impl Read for DataSourceReader {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        match &mut self.inner {
            DataSourceReaderInner::ReadSeekable(source) => source.read(buf),
            DataSourceReaderInner::Readable { data, .. } => data.read(buf),
        }
    }
}

impl Reset for DataSourceReader {
    #[inline]
    fn reset(&mut self) -> IoResult<()> {
        match &mut self.inner {
            DataSourceReaderInner::ReadSeekable(source) => source.reset(),
            DataSourceReaderInner::Readable { data, .. } => data.reset(),
        }
    }
}

pub(super) fn first_part_number() -> NonZeroUsize {
    #[allow(unsafe_code)]
    unsafe {
        NonZeroUsize::new_unchecked(1)
    }
}
