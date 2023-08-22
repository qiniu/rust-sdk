use super::{super::PartSize, reader::first_part_number, DataSource, DataSourceReader, SourceKey};
use digest::Digest;
use qiniu_apis::http::Reset;
use std::{
    fmt::Debug,
    io::{Error as IoError, Read, Result as IoResult, Seek, SeekFrom},
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub(super) struct SourceOffset {
    offset: u64,
    part_number: NonZeroUsize,
}

impl SourceOffset {
    pub(super) fn new(offset: u64, part_number: NonZeroUsize) -> Self {
        Self { offset, part_number }
    }

    pub(super) fn offset(&self) -> u64 {
        self.offset
    }

    pub(super) fn part_number(&self) -> NonZeroUsize {
        self.part_number
    }

    pub(super) fn offset_mut(&mut self) -> &mut u64 {
        &mut self.offset
    }

    pub(super) fn part_number_mut(&mut self) -> &mut NonZeroUsize {
        &mut self.part_number
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SeekableDataSource {
    source: SeekableSource,
    current: Arc<Mutex<SourceOffset>>,
    size: u64,
    original_offset: u64,
}

impl SeekableDataSource {
    pub(crate) fn new<R: Read + Seek + Debug + Send + Sync + 'static>(
        mut source: R,
        size: u64,
    ) -> Result<Self, (IoError, R)> {
        match source.stream_position() {
            Ok(original_offset) => Ok(Self {
                size,
                original_offset,
                current: Arc::new(Mutex::new(SourceOffset::new(original_offset, first_part_number()))),
                source: SeekableSource::new(source, 0, 0),
            }),
            Err(err) => Err((err, source)),
        }
    }
}

impl<D: Digest> DataSource<D> for SeekableDataSource {
    fn slice(&self, size: PartSize) -> IoResult<Option<DataSourceReader>> {
        let mut cur = self.current.lock().unwrap();
        if cur.offset() < self.size {
            let size = size.as_u64();
            let source_reader = DataSourceReader::seekable(
                cur.part_number(),
                self.source.clone_with_new_offset_and_length(cur.offset, size),
            );
            *cur.offset_mut() += size;
            *cur.part_number_mut() = cur.part_number().checked_add(1).expect("Page number is too big");
            Ok(Some(source_reader))
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn reset(&self) -> IoResult<()> {
        let mut cur = self.current.lock().unwrap();
        *cur.offset_mut() = self.original_offset;
        *cur.part_number_mut() = first_part_number();
        Ok(())
    }

    #[inline]
    fn total_size(&self) -> IoResult<Option<u64>> {
        Ok(Some(self.size))
    }

    #[inline]
    fn source_key(&self) -> IoResult<Option<SourceKey<D>>> {
        Ok(None)
    }
}

/// 可寻址的数据源
///
/// 用于表示一个分片，需要传入可寻址的数据源，以及分片的起始位置和长度
#[derive(Debug, Clone)]
pub struct SeekableSource {
    source: Arc<Mutex<SeekableSourceInner<dyn ReadSeek>>>,
    source_offset: u64,
    offset: u64,
    len: u64,
}

impl SeekableSource {
    /// 创建可寻址的数据源
    ///
    /// 需要传入可寻址的数据源，以及分片的起始位置和长度
    #[inline]
    pub fn new(source: impl Read + Seek + Debug + Send + Sync + 'static, offset: u64, len: u64) -> Self {
        Self {
            source: Arc::new(Mutex::new(SeekableSourceInner::new(source))),
            source_offset: 0,
            offset,
            len,
        }
    }

    pub(super) fn clone_with_new_offset_and_length(&self, offset: u64, len: u64) -> Self {
        let mut cloned = self.to_owned();
        cloned.source_offset = 0;
        cloned.offset = offset;
        cloned.len = len;
        cloned
    }

    pub(super) fn offset(&self) -> u64 {
        self.offset
    }

    pub(super) fn len(&self) -> IoResult<u64> {
        let mut locked = self.source.lock().unwrap();
        let new_pos = locked.source.seek(SeekFrom::End(0))?;
        if Some(new_pos) != locked.pos {
            locked.pos = Some(new_pos);
        }
        Ok(self.len.min(new_pos - self.offset))
    }
}

impl Read for SeekableSource {
    fn read(&mut self, mut buf: &mut [u8]) -> IoResult<usize> {
        let mut locked = self.source.lock().unwrap();
        let max_read = self.len - self.source_offset;
        if max_read == 0 {
            return Ok(0);
        } else if max_read < buf.len() as u64 {
            let max_read: usize = max_read.try_into().unwrap_or(usize::MAX);
            buf = &mut buf[..max_read];
        }
        let seek_pos = self.offset + self.source_offset;
        if Some(seek_pos) != locked.pos {
            locked.pos = Some(locked.source.seek(SeekFrom::Start(seek_pos))?);
        }
        let have_read = locked.source.read(buf)?;
        self.source_offset += have_read as u64;
        if let Some(ref mut pos) = locked.pos {
            *pos += have_read as u64;
        }
        Ok(have_read)
    }
}

impl Reset for SeekableSource {
    #[inline]
    fn reset(&mut self) -> IoResult<()> {
        self.source_offset = 0;
        Ok(())
    }
}

trait ReadSeek: Read + Seek + Send + Sync + Debug {}
impl<T: Read + Seek + Send + Sync + Debug> ReadSeek for T {}

#[derive(Debug)]
struct SeekableSourceInner<T: Read + Seek + Send + Sync + Debug + ?Sized> {
    pos: Option<u64>,
    source: T,
}

impl<T: Read + Seek + Send + Sync + Debug> SeekableSourceInner<T> {
    fn new(source: T) -> Self {
        Self { source, pos: None }
    }
}
