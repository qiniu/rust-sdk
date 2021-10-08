use lazy_static::lazy_static;
use mime::Mime;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use qiniu_utils::{smallstr::SmallString, wrap_smallstr};
use rand::random;
use regex::Regex;
use serde::{
    de::{Deserialize, Deserializer, Error, Visitor},
    ser::{Serialize, Serializer},
};
use std::{
    borrow::{Borrow, BorrowMut, Cow},
    collections::VecDeque,
    fmt,
    iter::FromIterator,
    ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo},
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileName {
    inner: SmallString<[u8; 64]>,
}
wrap_smallstr!(FileName);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldName {
    inner: SmallString<[u8; 16]>,
}
wrap_smallstr!(FieldName);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Boundary {
    inner: SmallString<[u8; 32]>,
}
wrap_smallstr!(Boundary);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct HeaderBuffer {
    inner: SmallString<[u8; 256]>,
}
wrap_smallstr!(HeaderBuffer);

pub struct Multipart<P> {
    boundary: Boundary,
    fields: VecDeque<(FieldName, P)>,
}

pub struct Part<B> {
    meta: Metadata,
    body: B,
}

#[derive(Default, Debug)]
struct Metadata {
    mime: Option<Mime>,
    file_name: Option<FileName>,
}

impl<P> Default for Multipart<P> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<P> Multipart<P> {
    #[inline]
    pub fn new() -> Self {
        Self {
            boundary: gen_boundary(),
            fields: Default::default(),
        }
    }

    #[inline]
    pub fn add_part(mut self, name: impl Into<FieldName>, part: P) -> Self {
        self.fields.push_back((name.into(), part));
        self
    }
}

impl<B> Part<B> {
    #[inline]
    pub fn mime(mut self, mime: Mime) -> Self {
        self.meta.mime = Some(mime);
        self
    }

    #[inline]
    pub fn file_name(mut self, file_name: impl Into<FileName>) -> Self {
        self.meta.file_name = Some(file_name.into());
        self
    }
}

mod sync_part {
    use super::*;
    use bytes::{buf::Reader as BytesReader, Bytes};
    use std::{
        fs::File,
        io::{Cursor, Read, Result as IOResult},
        mem::take,
        path::Path,
    };

    enum SyncBodyInner {
        Bytes(BytesReader<Bytes>),
        Stream(Box<dyn Read>),
    }
    pub struct SyncBody(SyncBodyInner);
    pub type SyncPart = Part<SyncBody>;

    impl SyncPart {
        #[inline]
        pub fn text(value: impl Into<Cow<'static, str>>) -> Self {
            use bytes::Buf;

            let bytes = match value.into() {
                Cow::Borrowed(slice) => Bytes::from_static(slice.as_bytes()),
                Cow::Owned(string) => Bytes::from(string),
            };
            Self {
                body: SyncBody(SyncBodyInner::Bytes(bytes.reader())),
                meta: Default::default(),
            }
        }

        #[inline]
        pub fn bytes(value: impl Into<Cow<'static, [u8]>>) -> Self {
            use bytes::Buf;

            let bytes = match value.into() {
                Cow::Borrowed(slice) => Bytes::from_static(slice),
                Cow::Owned(string) => Bytes::from(string),
            };
            Self {
                body: SyncBody(SyncBodyInner::Bytes(bytes.reader())),
                meta: Default::default(),
            }
        }

        #[inline]
        pub fn stream(value: Box<dyn Read>) -> Self {
            Self {
                body: SyncBody(SyncBodyInner::Stream(value)),
                meta: Default::default(),
            }
        }

        #[inline]
        pub fn file_path(path: impl AsRef<Path>) -> IOResult<Self> {
            let file = File::open(path.as_ref())?;
            let mut part = SyncPart::stream(Box::new(file))
                .mime(mime_guess::from_path(path.as_ref()).first_or_octet_stream());
            if let Some(file_name) = path.as_ref().file_name() {
                let file_name = match file_name.to_string_lossy() {
                    Cow::Borrowed(str) => FileName::from(str),
                    Cow::Owned(string) => FileName::from(string),
                };
                part = part.file_name(file_name);
            }
            Ok(part)
        }
    }

    pub type SyncMultipart = Multipart<SyncPart>;

    impl SyncMultipart {
        #[inline]
        pub(in super::super) fn into_read(mut self) -> Box<dyn Read> {
            if self.fields.is_empty() {
                return Box::new(Cursor::new([]));
            }

            let (name, part) = self.fields.pop_front().unwrap();
            let chain = Box::new(self.part_stream(&name, part)) as Box<dyn Read>;
            let fields = take(&mut self.fields);
            Box::new(
                fields
                    .into_iter()
                    .fold(chain, |readable, (name, part)| {
                        Box::new(readable.chain(self.part_stream(&name, part))) as Box<dyn Read>
                    })
                    .chain(Cursor::new(b"--"))
                    .chain(Cursor::new(self.boundary.to_owned()))
                    .chain(Cursor::new(b"--\r\n")),
            )
        }

        #[inline]
        fn part_stream(&self, name: &str, part: SyncPart) -> impl Read {
            Cursor::new(b"--")
                .chain(Cursor::new(self.boundary.to_owned()))
                .chain(Cursor::new(b"\r\n"))
                .chain(Cursor::new(encode_headers(name, &part.meta)))
                .chain(Cursor::new(b"\r\n\r\n"))
                .chain(part.body)
                .chain(Cursor::new(b"\r\n"))
        }
    }

    impl Read for SyncBody {
        #[inline]
        fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
            match &mut self.0 {
                SyncBodyInner::Bytes(bytes) => bytes.read(buf),
                SyncBodyInner::Stream(stream) => stream.read(buf),
            }
        }
    }
}
pub use sync_part::{SyncBody, SyncMultipart, SyncPart};

#[cfg(feature = "async")]
mod async_part {
    use super::*;
    use async_std::{fs::File, path::Path};
    use bytes::Bytes;
    use futures::io::{AsyncRead, AsyncReadExt, Cursor};
    use std::{
        io::Result as IOResult,
        mem::take,
        pin::Pin,
        task::{Context, Poll},
    };

    type AsyncStream = Box<dyn AsyncRead + Unpin>;

    enum AsyncBodyInner {
        Bytes(Cursor<Bytes>),
        Stream(AsyncStream),
    }

    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub struct AsyncBody(AsyncBodyInner);

    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub type AsyncPart = Part<AsyncBody>;

    impl AsyncPart {
        #[inline]
        pub fn text(value: impl Into<Cow<'static, str>>) -> Self {
            let bytes = match value.into() {
                Cow::Borrowed(slice) => Bytes::from_static(slice.as_bytes()),
                Cow::Owned(string) => Bytes::from(string),
            };
            Self {
                body: AsyncBody(AsyncBodyInner::Bytes(Cursor::new(bytes))),
                meta: Default::default(),
            }
        }

        #[inline]
        pub fn bytes(value: impl Into<Cow<'static, [u8]>>) -> Self {
            let bytes = match value.into() {
                Cow::Borrowed(slice) => Bytes::from_static(slice),
                Cow::Owned(string) => Bytes::from(string),
            };
            Self {
                body: AsyncBody(AsyncBodyInner::Bytes(Cursor::new(bytes))),
                meta: Default::default(),
            }
        }

        #[inline]
        pub fn stream(value: AsyncStream) -> Self {
            Self {
                body: AsyncBody(AsyncBodyInner::Stream(value)),
                meta: Default::default(),
            }
        }

        #[inline]
        pub async fn file_path(path: impl AsRef<Path>) -> IOResult<Self> {
            let file = File::open(path.as_ref()).await?;
            let mut part = AsyncPart::stream(Box::new(file))
                .mime(mime_guess::from_path(path.as_ref()).first_or_octet_stream());
            if let Some(file_name) = path.as_ref().file_name() {
                let file_name = match file_name.to_string_lossy() {
                    Cow::Borrowed(str) => FileName::from(str),
                    Cow::Owned(string) => FileName::from(string),
                };
                part = part.file_name(file_name);
            }
            Ok(part)
        }
    }

    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub type AsyncMultipart = Multipart<AsyncPart>;

    impl AsyncMultipart {
        #[inline]
        pub(in super::super) fn into_async_read(mut self) -> Box<dyn AsyncRead + Unpin> {
            if self.fields.is_empty() {
                return Box::new(Cursor::new([]));
            }

            let (name, part) = self.fields.pop_front().unwrap();
            let chain = Box::new(self.part_stream(&name, part)) as Box<dyn AsyncRead + Unpin>;
            let fields = take(&mut self.fields);
            Box::new(
                fields
                    .into_iter()
                    .fold(chain, |readable, (name, part)| {
                        Box::new(readable.chain(self.part_stream(&name, part)))
                            as Box<dyn AsyncRead + Unpin>
                    })
                    .chain(Cursor::new(b"--"))
                    .chain(Cursor::new(self.boundary.to_owned()))
                    .chain(Cursor::new(b"--\r\n")),
            )
        }

        #[inline]
        fn part_stream(&self, name: &str, part: AsyncPart) -> impl AsyncRead + Unpin {
            Cursor::new(b"--")
                .chain(Cursor::new(self.boundary.to_owned()))
                .chain(Cursor::new(b"\r\n"))
                .chain(Cursor::new(encode_headers(name, &part.meta)))
                .chain(Cursor::new(b"\r\n\r\n"))
                .chain(part.body)
                .chain(Cursor::new(b"\r\n"))
        }
    }

    impl AsyncRead for AsyncBody {
        #[inline]
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<IOResult<usize>> {
            match &mut self.0 {
                AsyncBodyInner::Bytes(bytes) => Pin::new(bytes).poll_read(cx, buf),
                AsyncBodyInner::Stream(stream) => Pin::new(stream).poll_read(cx, buf),
            }
        }
    }
}

#[cfg(feature = "async")]
pub use async_part::{AsyncBody, AsyncMultipart, AsyncPart};

#[inline]
fn gen_boundary() -> Boundary {
    use std::fmt::Write;

    let mut b = Boundary::with_capacity(32);
    write!(b, "{:016x}{:016x}", random::<u64>(), random::<u64>()).unwrap();
    b
}

#[inline]
fn encode_headers(name: &str, field: &Metadata) -> HeaderBuffer {
    let mut buf = HeaderBuffer::from("Content-Disposition: form-data; ");
    buf.push_str(&format_parameter("name", name));
    if let Some(file_name) = field.file_name.as_ref() {
        buf.push_str("; ");
        buf.push_str(&format_file_name(file_name));
    }
    if let Some(mime) = field.mime.as_ref() {
        buf.push_str("\r\nContent-Type: ");
        buf.push_str(mime.as_ref());
    }
    buf
}

#[inline]
fn format_file_name(filename: &str) -> FileName {
    lazy_static! {
        static ref REGEX: Regex = Regex::new("\\\\|\"|\r|\n").unwrap();
    }
    let mut formatted = FileName::from("filename=\"");
    let mut last_match = 0;
    for m in REGEX.find_iter(filename) {
        let begin = m.start();
        let end = m.end();
        formatted.push_str(&filename[last_match..begin]);
        match &filename[begin..end] {
            "\\" => formatted.push_str("\\\\"),
            "\"" => formatted.push_str("\\\""),
            "\r" => formatted.push_str("\\\r"),
            "\n" => formatted.push_str("\\\n"),
            _ => unreachable!(),
        }
        last_match = end;
    }
    formatted.push_str(&filename[last_match..]);
    formatted.push_str("\"");
    formatted
}

const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'#')
    .add(b'?')
    .add(b'{')
    .add(b'}')
    .add(b'/')
    .add(b'%');

#[inline]
fn format_parameter(name: &str, value: &str) -> HeaderBuffer {
    let legal_value = {
        let mut buf = HeaderBuffer::new();
        buf.extend(utf8_percent_encode(value, PATH_SEGMENT_ENCODE_SET));
        buf
    };
    let mut formatted = HeaderBuffer::new();
    if value.len() == legal_value.len() {
        formatted.push_str(name);
        formatted.push_str("=\"");
        formatted.push_str(value);
        formatted.push_str("\"");
    } else {
        formatted.push_str(name);
        formatted.push_str("*=utf-8''");
        formatted.push_str(&legal_value);
    };
    formatted
}

#[cfg(test)]
mod tests {
    use super::*;
    use mime::{APPLICATION_JSON, IMAGE_BMP};
    use std::{
        fs::File,
        io::{Cursor, Result as IOResult, Write},
    };
    use tempfile::tempdir;

    #[test]
    fn test_gen_boundary() {
        env_logger::builder().is_test(true).try_init().ok();

        for _ in 0..5 {
            assert_eq!(gen_boundary().len(), 32);
        }
    }

    #[test]
    fn test_header_percent_encoding() {
        env_logger::builder().is_test(true).try_init().ok();

        let name = "start%'\"\r\nßend";
        let metadata = Metadata {
            mime: Some(APPLICATION_JSON),
            file_name: Some(name.into()),
        };

        assert_eq!(
            encode_headers(name, &metadata).as_str(),
            "Content-Disposition: form-data; name*=utf-8''start%25'%22%0D%0A%C3%9Fend; filename=\"start%'\\\"\\\r\\\nßend\"\r\nContent-Type: application/json"
        );
    }

    #[test]
    fn test_multipart_into_read() -> IOResult<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let tempdir = tempdir()?;
        let temp_file_path = tempdir.path().join("fake-file.json");
        let mut file = File::create(&temp_file_path)?;
        file.write_all(b"{\"a\":\"b\"}\n")?;
        drop(file);

        let mut multipart = SyncMultipart::new()
            .add_part("bytes1", SyncPart::bytes(&b"part1"[..]))
            .add_part("text1", SyncPart::text("value1"))
            .add_part("text2", SyncPart::text("value1").mime(IMAGE_BMP))
            .add_part(
                "reader1",
                SyncPart::stream(Box::new(Cursor::new(b"value1"))),
            )
            .add_part("reader2", SyncPart::file_path(temp_file_path)?);
        multipart.boundary = "boundary".into();

        const EXPECTED: &str = "--boundary\r\n\
        Content-Disposition: form-data; name=\"bytes1\"\r\n\r\n\
        part1\r\n\
        --boundary\r\n\
        Content-Disposition: form-data; name=\"text1\"\r\n\r\n\
        value1\r\n\
        --boundary\r\n\
        Content-Disposition: form-data; name=\"text2\"\r\n\
        Content-Type: image/bmp\r\n\r\n\
        value1\r\n\
        --boundary\r\n\
        Content-Disposition: form-data; name=\"reader1\"\r\n\r\n\
        value1\r\n\
        --boundary\r\n\
        Content-Disposition: form-data; name=\"reader2\"; filename=\"fake-file.json\"\r\n\
        Content-Type: application/json\r\n\r\n\
        {\"a\":\"b\"}\n\r\n\
        --boundary--\
        \r\n";

        let mut actual = String::new();
        multipart.into_read().read_to_string(&mut actual)?;
        assert_eq!(EXPECTED, actual);

        tempdir.close()?;
        Ok(())
    }

    #[cfg(feature = "async")]
    #[async_std::test]
    async fn test_multipart_into_async_read() -> IOResult<()> {
        use async_std::{
            fs::File,
            io::{Cursor as AsyncCursor, ReadExt, WriteExt},
        };

        env_logger::builder().is_test(true).try_init().ok();

        let tempdir = tempdir()?;
        let temp_file_path = tempdir.path().join("fake-file.json");
        let mut file = File::create(&temp_file_path).await?;
        file.write_all(b"{\"a\":\"b\"}\n").await?;
        file.flush().await?;
        drop(file);

        let mut multipart = AsyncMultipart::new()
            .add_part("bytes1", AsyncPart::bytes(&b"part1"[..]))
            .add_part("text1", AsyncPart::text("value1"))
            .add_part("text2", AsyncPart::text("value1").mime(IMAGE_BMP))
            .add_part(
                "reader1",
                AsyncPart::stream(Box::new(AsyncCursor::new(b"value1"))),
            )
            .add_part("reader2", AsyncPart::file_path(temp_file_path).await?);
        multipart.boundary = "boundary".into();

        const EXPECTED: &str = "--boundary\r\n\
        Content-Disposition: form-data; name=\"bytes1\"\r\n\r\n\
        part1\r\n\
        --boundary\r\n\
        Content-Disposition: form-data; name=\"text1\"\r\n\r\n\
        value1\r\n\
        --boundary\r\n\
        Content-Disposition: form-data; name=\"text2\"\r\n\
        Content-Type: image/bmp\r\n\r\n\
        value1\r\n\
        --boundary\r\n\
        Content-Disposition: form-data; name=\"reader1\"\r\n\r\n\
        value1\r\n\
        --boundary\r\n\
        Content-Disposition: form-data; name=\"reader2\"; filename=\"fake-file.json\"\r\n\
        Content-Type: application/json\r\n\r\n\
        {\"a\":\"b\"}\n\r\n\
        --boundary--\
        \r\n";

        let mut actual = String::new();
        multipart
            .into_async_read()
            .read_to_string(&mut actual)
            .await?;
        assert_eq!(EXPECTED, actual);

        tempdir.close()?;
        Ok(())
    }
}
