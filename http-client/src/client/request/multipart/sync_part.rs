use super::{encode_headers, Boundary, FileName, Multipart, Part, PartMetadata};
use qiniu_http::Reset;
use std::{
    borrow::Cow,
    cmp::Ordering,
    ffi::OsStr,
    fmt::{self, Debug},
    fs::File,
    io::{copy, Cursor, Read, Result as IoResult, Seek, SeekFrom},
    mem::take,
    path::Path,
};

trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

enum SyncPartBodyInner<'a> {
    Bytes(Cursor<Cow<'a, [u8]>>),
    Seekable(Box<dyn ReadSeek + Send + Sync + 'a>),
    Unseekable(Box<dyn Read + Send + Sync + 'a>),
}

impl Debug for SyncPartBodyInner<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bytes(bytes) => f.debug_tuple("Bytes").field(bytes).finish(),
            Self::Seekable(_) => f.debug_tuple("Seekable").finish(),
            Self::Unseekable(_) => f.debug_tuple("Unseekable").finish(),
        }
    }
}

/// 阻塞 Multipart 表单组件请求体
#[derive(Debug)]
pub struct SyncPartBody<'a>(SyncPartBodyInner<'a>);

/// 阻塞 Multipart 表单组件
pub type SyncPart<'a> = Part<SyncPartBody<'a>>;

impl<'a> SyncPart<'a> {
    /// 设置阻塞 Multipart 的请求体为字符串
    #[inline]
    #[must_use]
    pub fn text(value: impl Into<Cow<'a, str>>) -> Self {
        let bytes = match value.into() {
            Cow::Borrowed(str) => Cow::Borrowed(str.as_bytes()),
            Cow::Owned(string) => Cow::Owned(string.into_bytes()),
        };
        Self {
            body: SyncPartBody(SyncPartBodyInner::Bytes(Cursor::new(bytes))),
            meta: Default::default(),
        }
    }

    /// 设置阻塞 Multipart 的请求体为内存数据
    #[inline]
    #[must_use]
    pub fn bytes(value: impl Into<Cow<'a, [u8]>>) -> Self {
        Self {
            body: SyncPartBody(SyncPartBodyInner::Bytes(Cursor::new(value.into()))),
            meta: Default::default(),
        }
    }

    /// 设置阻塞 Multipart 的请求体为可寻址的输入流
    #[inline]
    #[must_use]
    pub fn seekable(value: impl Read + Seek + Send + Sync + 'a) -> Self {
        Self {
            body: SyncPartBody(SyncPartBodyInner::Seekable(Box::new(value))),
            meta: Default::default(),
        }
    }

    /// 设置阻塞 Multipart 的请求体为不可寻址的输入流
    #[inline]
    #[must_use]
    pub fn unseekable(value: impl Read + Send + Sync + 'a) -> Self {
        Self {
            body: SyncPartBody(SyncPartBodyInner::Unseekable(Box::new(value))),
            meta: Default::default(),
        }
    }

    /// 设置阻塞 Multipart 的请求体为文件
    pub fn file_path<S: AsRef<OsStr> + ?Sized>(path: &S) -> IoResult<Self> {
        let path = Path::new(path);
        let file = File::open(path)?;
        let mut metadata = PartMetadata::default().mime(mime_guess::from_path(path).first_or_octet_stream());
        if let Some(file_name) = path.file_name() {
            let file_name = match file_name.to_string_lossy() {
                Cow::Borrowed(str) => FileName::from(str),
                Cow::Owned(string) => FileName::from(string),
            };
            metadata = metadata.file_name(file_name);
        }
        Ok(SyncPart::seekable(file).metadata(metadata))
    }
}

/// 阻塞 Multipart
pub type SyncMultipart<'a> = Multipart<SyncPart<'a>>;

impl<'a> SyncMultipart<'a> {
    pub(in super::super) fn into_read(mut self) -> IoResult<SyncMultipartReader<'a>> {
        let mut reader = SyncMultipartReader::default();
        for (name, mut part) in take(&mut self.fields) {
            part.body.reset()?;
            reader.append_part(&name, part, &self.boundary)?;
        }
        reader.append_ending(&self.boundary);
        Ok(reader)
    }
}

impl Read for SyncPartBody<'_> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        match &mut self.0 {
            SyncPartBodyInner::Bytes(bytes) => bytes.read(buf),
            SyncPartBodyInner::Seekable(reader) => reader.read(buf),
            SyncPartBodyInner::Unseekable(reader) => {
                let mut buffer = Vec::new();
                copy(reader, &mut buffer)?;
                self.0 = SyncPartBodyInner::Bytes(Cursor::new(Cow::Owned(buffer)));
                self.read(buf)
            }
        }
    }
}

impl Seek for SyncPartBody<'_> {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        match &mut self.0 {
            SyncPartBodyInner::Bytes(bytes) => bytes.seek(pos),
            SyncPartBodyInner::Seekable(seekable) => seekable.seek(pos),
            SyncPartBodyInner::Unseekable(_) if pos == SeekFrom::Start(0) || pos == SeekFrom::Current(0) => Ok(0),
            SyncPartBodyInner::Unseekable(unseekable) => {
                let mut buffer = Vec::new();
                copy(unseekable, &mut buffer)?;
                self.0 = SyncPartBodyInner::Bytes(Cursor::new(Cow::Owned(buffer)));
                self.seek(pos)
            }
        }
    }
}

impl SyncPartBody<'_> {
    fn len(&mut self) -> IoResult<u64> {
        match &mut self.0 {
            SyncPartBodyInner::Bytes(bytes) => Ok(bytes.get_ref().len() as u64),
            SyncPartBodyInner::Seekable(seekable) => {
                let old_pos = seekable.stream_position()?;
                let len = seekable.seek(SeekFrom::End(0))?;
                if old_pos != len {
                    self.seek(SeekFrom::Start(old_pos))?;
                }
                Ok(len)
            }
            SyncPartBodyInner::Unseekable(unseekable) => {
                let mut buffer = Vec::new();
                copy(unseekable, &mut buffer)?;
                let buf_len = buffer.len();
                self.0 = SyncPartBodyInner::Bytes(Cursor::new(Cow::Owned(buffer)));
                Ok(buf_len as u64)
            }
        }
    }
}

#[derive(Debug)]
struct SyncPartBodyReader<'a> {
    inner: SyncPartBody<'a>,
    pos: u64,
    size: u64,
}

#[derive(Default, Debug)]
pub(in super::super) struct SyncMultipartReader<'a>(Vec<SyncPartBodyReader<'a>>);

impl<'a> SyncMultipartReader<'a> {
    fn append_bytes(&mut self, bytes: impl Into<Cow<'a, [u8]>>) -> &mut Self {
        let bytes = bytes.into();
        self.0.push(SyncPartBodyReader {
            size: bytes.len() as u64,
            inner: SyncPartBody(SyncPartBodyInner::Bytes(Cursor::new(bytes))),
            pos: 0,
        });
        self
    }

    fn append_raw_part(&mut self, mut part: SyncPart<'a>) -> IoResult<&mut Self> {
        self.0.push(SyncPartBodyReader {
            size: part.body.len()?,
            inner: part.body,
            pos: 0,
        });
        Ok(self)
    }

    fn append_part(&mut self, name: &str, part: SyncPart<'a>, boundary: &Boundary) -> IoResult<&mut Self> {
        self.append_bytes(b"--".as_slice())
            .append_bytes(boundary.as_str().to_owned().into_bytes())
            .append_bytes(b"\r\n".as_slice())
            .append_bytes(encode_headers(name, &part.meta).into_vec())
            .append_bytes(b"\r\n\r\n".as_slice())
            .append_raw_part(part)?
            .append_bytes(b"\r\n".as_slice());
        Ok(self)
    }

    fn append_ending(&mut self, boundary: &Boundary) -> &mut Self {
        self.append_bytes(b"--".as_slice())
            .append_bytes(boundary.as_str().to_owned().into_bytes())
            .append_bytes(b"--\r\n".as_slice())
    }

    fn seek_start(&mut self, offset: u64) -> IoResult<u64> {
        let mut offset_now = 0u64;
        let mut is_sought = false;
        for part in &mut self.0 {
            if is_sought {
                if part.pos > 0 {
                    part.pos = part.inner.seek(SeekFrom::Start(0))?;
                }
            } else if offset_now + part.size > offset {
                if part.pos != offset - offset_now {
                    part.pos = part.inner.seek(SeekFrom::Start(offset - offset_now))?;
                }
                offset_now += part.pos;
                is_sought = true;
            } else {
                if part.pos < part.size {
                    part.pos = part.inner.seek(SeekFrom::Start(part.size))?;
                }
                offset_now += part.size;
            }
        }
        Ok(offset_now)
    }

    fn seek_end(&mut self, mut offset: i64) -> IoResult<u64> {
        if offset > 0 {
            offset = 0;
        }
        let len = self.len();
        let new_offset = len - offset.unsigned_abs();
        self.seek_start(new_offset)
    }

    fn seek_current(&mut self, offset: i64) -> IoResult<u64> {
        let pos = self.position();
        match offset.cmp(&0) {
            Ordering::Less => self.seek_start(pos - offset as u64),
            Ordering::Greater => self.seek_start(pos + offset as u64),
            Ordering::Equal => Ok(pos),
        }
    }

    pub(in super::super) fn len(&self) -> u64 {
        self.0.iter().map(|part| part.size).sum()
    }

    fn position(&self) -> u64 {
        let mut pos = 0u64;
        for part in &self.0 {
            pos += part.pos;
            if part.pos < part.size {
                break;
            }
        }
        pos
    }
}

impl<'a> Read for SyncMultipartReader<'a> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let mut have_read = 0usize;
        'read_done: for part in &mut self.0 {
            while part.pos < part.size {
                let n = (&mut part.inner)
                    .take(part.size - part.pos)
                    .read(&mut buf[have_read..])?;
                part.pos += n as u64;
                have_read += n;
                if have_read == buf.len() {
                    break 'read_done;
                }
            }
        }
        Ok(have_read)
    }
}

impl<'a> Seek for SyncMultipartReader<'a> {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        match pos {
            SeekFrom::Start(offset) => self.seek_start(offset),
            SeekFrom::End(offset) => self.seek_end(offset),
            SeekFrom::Current(offset) => self.seek_current(offset),
        }
    }

    #[inline]
    fn rewind(&mut self) -> IoResult<()> {
        for part in &mut self.0 {
            part.inner.seek(SeekFrom::Start(0))?;
            part.pos = 0;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mime::IMAGE_BMP;
    use std::io::{Read, Write};
    use tempfile::tempdir;

    #[test]
    fn test_multipart_into_read() -> IoResult<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let tempdir = tempdir()?;
        let temp_file_path = tempdir.path().join("fake-file.json");
        let mut file = File::create(&temp_file_path)?;
        file.write_all(b"{\"a\":\"b\"}\n")?;
        drop(file);

        let mut multipart = SyncMultipart::new()
            .add_part("bytes1", SyncPart::bytes(b"part1".as_slice()))
            .add_part("text1", SyncPart::text("value1"))
            .add_part(
                "text2",
                SyncPart::text("value1").metadata(PartMetadata::default().mime(IMAGE_BMP)),
            )
            .add_part("reader1", SyncPart::seekable(Cursor::new(b"value1")))
            .add_part("reader2", SyncPart::file_path(&temp_file_path)?);
        multipart.boundary = "boundary".into();

        const EXPECTED: &str = "--boundary\r\n\
        content-disposition: form-data; name=\"bytes1\"\r\n\r\n\
        part1\r\n\
        --boundary\r\n\
        content-disposition: form-data; name=\"text1\"\r\n\r\n\
        value1\r\n\
        --boundary\r\n\
        content-disposition: form-data; name=\"text2\"\r\n\
        content-type: image/bmp\r\n\r\n\
        value1\r\n\
        --boundary\r\n\
        content-disposition: form-data; name=\"reader1\"\r\n\r\n\
        value1\r\n\
        --boundary\r\n\
        content-disposition: form-data; name=\"reader2\"; filename=\"fake-file.json\"\r\n\
        content-type: application/json\r\n\r\n\
        {\"a\":\"b\"}\n\r\n\
        --boundary--\
        \r\n";

        let mut actual = String::new();
        multipart.into_read()?.read_to_string(&mut actual)?;
        assert_eq!(EXPECTED, actual);

        tempdir.close()?;
        Ok(())
    }
}
