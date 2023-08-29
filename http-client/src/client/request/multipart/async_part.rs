use super::{encode_headers, Boundary, FileName, Multipart, Part, PartMetadata};
use futures::{
    io::{copy, AsyncRead, Cursor, SeekFrom},
    AsyncSeek, AsyncSeekExt,
};
use qiniu_http::AsyncReset;
use qiniu_utils::async_fs::File;
use std::{
    borrow::Cow,
    cmp::Ordering,
    ffi::OsStr,
    fmt::{self, Debug},
    io::Result as IoResult,
    mem::take,
    path::Path,
    pin::Pin,
    task::{ready, Context, Poll},
};

trait AsyncReadSeek: AsyncRead + AsyncSeek {}
impl<T: AsyncRead + AsyncSeek> AsyncReadSeek for T {}

type AsyncSeekable<'a> = Box<dyn AsyncReadSeek + Send + Sync + Unpin + 'a>;
type AsyncUnseekable<'a> = Box<dyn AsyncRead + Send + Sync + Unpin + 'a>;

enum AsyncPartBodyInner<'a> {
    Bytes(Cursor<Cow<'a, [u8]>>),
    Seekable(AsyncSeekable<'a>),
    Unseekable {
        buffer: Vec<u8>,
        unseekable: AsyncUnseekable<'a>,
    },
}

impl Debug for AsyncPartBodyInner<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bytes(bytes) => f.debug_tuple("Bytes").field(bytes).finish(),
            Self::Seekable { .. } => f.debug_tuple("Seekable").finish(),
            Self::Unseekable { .. } => f.debug_tuple("Unseekable").finish(),
        }
    }
}

/// 异步 Multipart 表单组件请求体
#[derive(Debug)]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
)]
pub struct AsyncPartBody<'a>(AsyncPartBodyInner<'a>);

/// 异步 Multipart 表单组件
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
)]
pub type AsyncPart<'a> = Part<AsyncPartBody<'a>>;

impl<'a> AsyncPart<'a> {
    /// 设置异步 Multipart 的请求体为字符串
    #[inline]
    #[must_use]
    pub fn text(value: impl Into<Cow<'a, str>>) -> Self {
        let bytes = match value.into() {
            Cow::Borrowed(slice) => Cow::Borrowed(slice.as_bytes()),
            Cow::Owned(string) => Cow::Owned(string.into_bytes()),
        };
        Self {
            body: AsyncPartBody(AsyncPartBodyInner::Bytes(Cursor::new(bytes))),
            meta: Default::default(),
        }
    }

    /// 设置异步 Multipart 的请求体为内存数据
    #[inline]
    #[must_use]
    pub fn bytes(value: impl Into<Cow<'a, [u8]>>) -> Self {
        Self {
            body: AsyncPartBody(AsyncPartBodyInner::Bytes(Cursor::new(value.into()))),
            meta: Default::default(),
        }
    }

    /// 设置异步 Multipart 的请求体为可寻址的异步输入流
    #[inline]
    #[must_use]
    pub fn seekable(value: impl AsyncRead + AsyncSeek + Send + Sync + Unpin + 'a) -> Self {
        Self {
            body: AsyncPartBody(AsyncPartBodyInner::Seekable(Box::new(value))),
            meta: Default::default(),
        }
    }

    /// 设置异步 Multipart 的请求体为不可寻址的异步输入流
    #[inline]
    #[must_use]
    pub fn unseekable(value: impl AsyncRead + Send + Sync + Unpin + 'a) -> Self {
        Self {
            body: AsyncPartBody(AsyncPartBodyInner::Unseekable {
                unseekable: Box::new(value),
                buffer: Default::default(),
            }),
            meta: Default::default(),
        }
    }

    /// 设置异步 Multipart 的请求体为文件
    pub async fn file_path<S: AsRef<OsStr> + ?Sized>(path: &S) -> IoResult<AsyncPart<'a>> {
        let path = Path::new(path);
        let file = File::open(&path).await?;
        let mut metadata = PartMetadata::default().mime(mime_guess::from_path(path).first_or_octet_stream());
        if let Some(file_name) = path.file_name() {
            let file_name = match file_name.to_string_lossy() {
                Cow::Borrowed(str) => FileName::from(str),
                Cow::Owned(string) => FileName::from(string),
            };
            metadata = metadata.file_name(file_name);
        }
        Ok(AsyncPart::seekable(file).metadata(metadata))
    }
}

/// 异步 Multipart
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
)]
pub type AsyncMultipart<'a> = Multipart<AsyncPart<'a>>;

impl<'a> AsyncMultipart<'a> {
    pub(in super::super) async fn into_async_read(mut self) -> IoResult<AsyncMultipartReader<'a>> {
        let mut reader = AsyncMultipartReader::default();
        for (name, mut part) in take(&mut self.fields) {
            part.body.reset().await?;
            reader.append_part(&name, part, &self.boundary).await?;
        }
        reader.append_ending(&self.boundary);
        Ok(reader)
    }
}

impl AsyncRead for AsyncPartBody<'_> {
    #[inline]
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<IoResult<usize>> {
        match &mut self.0 {
            AsyncPartBodyInner::Bytes(bytes) => Pin::new(bytes).poll_read(cx, buf),
            AsyncPartBodyInner::Seekable(reader) => Pin::new(reader).poll_read(cx, buf),
            AsyncPartBodyInner::Unseekable { buffer, unseekable } => {
                let n = ready!(Pin::new(unseekable).poll_read(cx, buf))?;
                if n > 0 {
                    buffer.extend_from_slice(&buf[..n]);
                } else {
                    self.0 = AsyncPartBodyInner::Bytes(Cursor::new(Cow::Owned(take(buffer))));
                }
                Poll::Ready(Ok(n))
            }
        }
    }
}

impl AsyncSeek for AsyncPartBody<'_> {
    #[inline]
    fn poll_seek(mut self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<IoResult<u64>> {
        match &mut self.0 {
            AsyncPartBodyInner::Bytes(bytes) => Pin::new(bytes).poll_seek(cx, pos),
            AsyncPartBodyInner::Seekable(stream) => Pin::new(stream).poll_seek(cx, pos),
            AsyncPartBodyInner::Unseekable { .. } if pos == SeekFrom::Start(0) || pos == SeekFrom::Current(0) => {
                Poll::Ready(Ok(0))
            }
            AsyncPartBodyInner::Unseekable { buffer, unseekable } => {
                let mut buf = [0u8; 4096];
                let mut unseekable = Pin::new(unseekable);
                loop {
                    let n = ready!(Pin::new(&mut unseekable).poll_read(cx, &mut buf))?;
                    if n > 0 {
                        buffer.extend_from_slice(&buf[..n]);
                    } else {
                        self.0 = AsyncPartBodyInner::Bytes(Cursor::new(Cow::Owned(take(buffer))));
                        break;
                    }
                }
                self.poll_seek(cx, pos)
            }
        }
    }
}

impl AsyncPartBody<'_> {
    async fn len(&mut self) -> IoResult<u64> {
        match &mut self.0 {
            AsyncPartBodyInner::Bytes(bytes) => Ok(bytes.get_ref().len() as u64),
            AsyncPartBodyInner::Seekable(stream) => {
                let mut pin_stream = Pin::new(stream);
                let old_pos = pin_stream.stream_position().await?;
                let len = pin_stream.seek(SeekFrom::End(0)).await?;
                if old_pos != len {
                    pin_stream.seek(SeekFrom::Start(old_pos)).await?;
                }
                Ok(len)
            }
            AsyncPartBodyInner::Unseekable { buffer, unseekable } => {
                let mut buffer = take(buffer);
                copy(unseekable, &mut buffer).await?;
                let buf_len = buffer.len();
                self.0 = AsyncPartBodyInner::Bytes(Cursor::new(Cow::Owned(buffer)));
                Ok(buf_len as u64)
            }
        }
    }
}

#[derive(Debug)]
struct AsyncPartBodyReader<'a> {
    inner: AsyncPartBody<'a>,
    pos: u64,
    size: u64,
}

#[derive(Default, Debug)]
pub(in super::super) struct AsyncMultipartReader<'a>(Vec<AsyncPartBodyReader<'a>>);

impl<'a> AsyncMultipartReader<'a> {
    fn append_bytes(&mut self, bytes: impl Into<Cow<'a, [u8]>>) -> &mut Self {
        let bytes = bytes.into();
        self.0.push(AsyncPartBodyReader {
            size: bytes.len() as u64,
            inner: AsyncPartBody(AsyncPartBodyInner::Bytes(Cursor::new(bytes))),
            pos: 0,
        });
        self
    }

    async fn append_raw_part(&mut self, mut part: AsyncPart<'a>) -> IoResult<&mut AsyncMultipartReader<'a>> {
        self.0.push(AsyncPartBodyReader {
            size: part.body.len().await?,
            inner: part.body,
            pos: 0,
        });
        Ok(self)
    }

    async fn append_part(
        &mut self,
        name: &str,
        part: AsyncPart<'a>,
        boundary: &Boundary,
    ) -> IoResult<&mut AsyncMultipartReader<'a>> {
        self.append_bytes(b"--".as_slice())
            .append_bytes(boundary.as_str().to_owned().into_bytes())
            .append_bytes(b"\r\n".as_slice())
            .append_bytes(encode_headers(name, &part.meta).into_vec())
            .append_bytes(b"\r\n\r\n".as_slice())
            .append_raw_part(part)
            .await?
            .append_bytes(b"\r\n".as_slice());
        Ok(self)
    }

    fn append_ending(&mut self, boundary: &Boundary) -> &mut Self {
        self.append_bytes(b"--".as_slice())
            .append_bytes(boundary.as_str().to_owned().into_bytes())
            .append_bytes(b"--\r\n".as_slice())
    }

    fn seek_start(mut self: Pin<&mut Self>, cx: &mut Context<'_>, offset: u64) -> Poll<IoResult<u64>> {
        let mut offset_now = 0u64;
        let mut is_sought = false;
        for part in &mut self.0 {
            let pin_body = Pin::new(&mut part.inner);
            if is_sought {
                if part.pos > 0 {
                    part.pos = ready!(pin_body.poll_seek(cx, SeekFrom::Start(0)))?;
                }
            } else if offset_now + part.size > offset {
                if offset - offset_now != part.pos {
                    part.pos = ready!(pin_body.poll_seek(cx, SeekFrom::Start(offset - offset_now)))?;
                }
                offset_now += part.pos;
                is_sought = true;
            } else {
                if part.pos < part.size {
                    part.pos = ready!(pin_body.poll_seek(cx, SeekFrom::Start(part.size)))?;
                }
                offset_now += part.size;
            }
        }
        Poll::Ready(Ok(offset_now))
    }

    fn seek_end(self: Pin<&mut Self>, cx: &mut Context<'_>, mut offset: i64) -> Poll<IoResult<u64>> {
        if offset > 0 {
            offset = 0;
        }
        let len = self.len();
        let new_offset = len - offset.unsigned_abs();
        self.seek_start(cx, new_offset)
    }

    fn seek_current(self: Pin<&mut Self>, cx: &mut Context<'_>, offset: i64) -> Poll<IoResult<u64>> {
        let pos = self.position();
        match offset.cmp(&0) {
            Ordering::Less => self.seek_start(cx, pos - offset as u64),
            Ordering::Greater => self.seek_start(cx, pos + offset as u64),
            Ordering::Equal => Poll::Ready(Ok(pos)),
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

impl<'a> AsyncRead for AsyncMultipartReader<'a> {
    #[inline]
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<IoResult<usize>> {
        if let Some(part) = self.0.iter_mut().find(|part| part.pos < part.size) {
            let max_read = buf
                .len()
                .min(usize::try_from(part.size - part.pos).unwrap_or(usize::MAX));
            let n = ready!(Pin::new(&mut part.inner).poll_read(cx, &mut buf[..max_read]))?;
            part.pos += n as u64;
            Poll::Ready(Ok(n))
        } else {
            Poll::Ready(Ok(0))
        }
    }
}

impl<'a> AsyncSeek for AsyncMultipartReader<'a> {
    #[inline]
    fn poll_seek(self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<IoResult<u64>> {
        match pos {
            SeekFrom::Start(offset) => self.seek_start(cx, offset),
            SeekFrom::End(offset) => self.seek_end(cx, offset),
            SeekFrom::Current(offset) => self.seek_current(cx, offset),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::io::{AsyncReadExt, AsyncWriteExt, Cursor as AsyncCursor};
    use mime::IMAGE_BMP;
    use tempfile::tempdir;

    #[qiniu_utils::async_runtime::test]
    async fn test_multipart_into_async_read() -> IoResult<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let tempdir = tempdir()?;
        let temp_file_path = tempdir.path().join("fake-file.json");
        let mut file = File::create(&temp_file_path).await?;
        file.write_all(b"{\"a\":\"b\"}\n").await?;
        file.flush().await?;
        drop(file);

        let mut multipart = AsyncMultipart::new()
            .add_part("bytes1", AsyncPart::bytes(b"part1".as_slice()))
            .add_part("text1", AsyncPart::text("value1"))
            .add_part(
                "text2",
                AsyncPart::text("value1").metadata(PartMetadata::default().mime(IMAGE_BMP)),
            )
            .add_part("reader1", AsyncPart::seekable(AsyncCursor::new(b"value1")))
            .add_part("reader2", AsyncPart::file_path(&temp_file_path).await?);
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
        multipart.into_async_read().await?.read_to_string(&mut actual).await?;
        assert_eq!(EXPECTED, actual);

        tempdir.close()?;
        Ok(())
    }
}
