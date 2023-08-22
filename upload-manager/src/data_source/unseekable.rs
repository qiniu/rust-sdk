use super::{super::PartSize, reader::first_part_number, DataSource, DataSourceReader, SourceKey};
use sha1::{digest::Digest, Sha1};
use std::{
    fmt::{self, Debug},
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult},
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

/// 不可寻址的数据源
///
/// 基于一个不可寻址的阅读器实现了数据源接口
pub struct UnseekableDataSource<R: Read + Debug + Send + Sync + 'static + ?Sized, A: Digest = Sha1>(
    Arc<Mutex<UnseekableDataSourceInner<R, A>>>,
);

impl<R: Read + Debug + Send + Sync + 'static, A: Digest> Debug for UnseekableDataSource<R, A> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UnseekableDataSource").field(&self.0).finish()
    }
}

impl<R: Read + Debug + Send + Sync + 'static, A: Digest> Clone for UnseekableDataSource<R, A> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

struct UnseekableDataSourceInner<R: Read + Debug + Send + Sync + 'static + ?Sized, A: Digest> {
    current_offset: u64,
    current_part_number: NonZeroUsize,
    source_key: Option<SourceKey<A>>,
    reader: R,
}

impl<R: Read + Debug + Send + Sync + 'static, A: Digest> UnseekableDataSource<R, A> {
    /// 创建不可寻址的数据源
    #[inline]
    pub fn new(reader: R) -> Self {
        Self(Arc::new(Mutex::new(UnseekableDataSourceInner {
            reader,
            current_offset: 0,
            current_part_number: first_part_number(),
            source_key: None,
        })))
    }
}

impl<R: Read + Debug + Send + Sync + 'static, A: Digest> DataSource<A> for UnseekableDataSource<R, A> {
    fn slice(&self, size: PartSize) -> IoResult<Option<DataSourceReader>> {
        let mut buf = Vec::new();
        let guard = &mut self.0.lock().unwrap();
        let have_read = (&mut guard.reader).take(size.as_u64()).read_to_end(&mut buf)?;
        if have_read > 0 {
            let source_reader = DataSourceReader::unseekable(guard.current_part_number, buf, guard.current_offset);
            guard.current_offset += have_read as u64;
            guard.current_part_number = guard
                .current_part_number
                .checked_add(1)
                .expect("Page number is too big");
            Ok(Some(source_reader))
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn reset(&self) -> IoResult<()> {
        Err(unsupported_reset_error())
    }

    #[inline]
    fn source_key(&self) -> IoResult<Option<SourceKey<A>>> {
        Ok(self.0.lock().unwrap().source_key.to_owned())
    }

    #[inline]
    fn total_size(&self) -> IoResult<Option<u64>> {
        Ok(None)
    }
}

impl<R: Read + Debug + Send + Sync + 'static, A: Digest> Debug for UnseekableDataSourceInner<R, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnseekableDataSourceInner")
            .field("reader", &self.reader)
            .field("current_offset", &self.current_offset)
            .field("current_part_number", &self.current_part_number)
            .field("source_key", &self.source_key)
            .finish()
    }
}

pub(super) fn unsupported_reset_error() -> IoError {
    IoError::new(IoErrorKind::Unsupported, "Cannot reset unseekable source")
}
