use lazy_static::lazy_static;
use mime::Mime;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use qiniu_http::{header::CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use qiniu_utils::{smallstr::SmallString, wrap_smallstr};
use rand::random;
use regex::Regex;
use serde::{
    de::{Deserialize, Deserializer, Error, Visitor},
    ser::{Serialize, Serializer},
};
use smallvec::SmallVec;
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

type HeaderBuffer = SmallVec<[u8; 256]>;

#[derive(Debug)]
pub struct Multipart<P> {
    boundary: Boundary,
    fields: VecDeque<(FieldName, P)>,
}

pub struct Part<B> {
    meta: PartMetadata,
    body: B,
}

impl<B> fmt::Debug for Part<B> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Part").field("meta", &self.meta).finish()
    }
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

#[derive(Default, Debug)]
pub struct PartMetadata {
    headers: HeaderMap,
    file_name: Option<FileName>,
}

impl PartMetadata {
    #[inline]
    pub fn mime(self, mime: Mime) -> Self {
        self.add_header(CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap())
    }

    #[inline]
    pub fn add_header(
        mut self,
        name: impl Into<HeaderName>,
        value: impl Into<HeaderValue>,
    ) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    #[inline]
    pub fn file_name(mut self, file_name: impl Into<FileName>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }
}

impl<B> Part<B> {
    #[inline]
    pub fn metadata(mut self, metadata: PartMetadata) -> Self {
        self.meta = metadata;
        self
    }
}

mod sync_part {
    use super::*;
    use bytes::{buf::Reader as BytesReader, Bytes};
    use std::{
        fs::File,
        io::{Cursor, Read, Result as IoResult},
        mem::take,
        path::Path,
    };

    enum SyncPartBodyInner {
        Bytes(BytesReader<Bytes>),
        Stream(Box<dyn Read>),
    }
    pub struct SyncPartBody(SyncPartBodyInner);
    pub type SyncPart = Part<SyncPartBody>;

    impl SyncPart {
        #[inline]
        pub fn text(value: impl Into<Cow<'static, str>>) -> Self {
            use bytes::Buf;

            let bytes = match value.into() {
                Cow::Borrowed(slice) => Bytes::from_static(slice.as_bytes()),
                Cow::Owned(string) => Bytes::from(string),
            };
            Self {
                body: SyncPartBody(SyncPartBodyInner::Bytes(bytes.reader())),
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
                body: SyncPartBody(SyncPartBodyInner::Bytes(bytes.reader())),
                meta: Default::default(),
            }
        }

        #[inline]
        pub fn stream(value: Box<dyn Read>) -> Self {
            Self {
                body: SyncPartBody(SyncPartBodyInner::Stream(value)),
                meta: Default::default(),
            }
        }

        #[inline]
        pub fn file_path(path: impl AsRef<Path>) -> IoResult<Self> {
            let file = File::open(path.as_ref())?;
            let mut metadata = PartMetadata::default()
                .mime(mime_guess::from_path(path.as_ref()).first_or_octet_stream());
            if let Some(file_name) = path.as_ref().file_name() {
                let file_name = match file_name.to_string_lossy() {
                    Cow::Borrowed(str) => FileName::from(str),
                    Cow::Owned(string) => FileName::from(string),
                };
                metadata = metadata.file_name(file_name);
            }
            Ok(SyncPart::stream(Box::new(file)).metadata(metadata))
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

    impl Read for SyncPartBody {
        #[inline]
        fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
            match &mut self.0 {
                SyncPartBodyInner::Bytes(bytes) => bytes.read(buf),
                SyncPartBodyInner::Stream(stream) => stream.read(buf),
            }
        }
    }
}
pub use sync_part::{SyncMultipart, SyncPart, SyncPartBody};

#[cfg(feature = "async")]
mod async_part {
    use super::*;
    use async_std::{fs::File, path::Path};
    use bytes::Bytes;
    use futures::io::{AsyncRead, AsyncReadExt, Cursor};
    use std::{
        io::Result as IoResult,
        mem::take,
        pin::Pin,
        task::{Context, Poll},
    };

    type AsyncStream = Box<dyn AsyncRead + Send + Unpin>;

    enum AsyncPartBodyInner {
        Bytes(Cursor<Bytes>),
        Stream(AsyncStream),
    }

    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub struct AsyncPartBody(AsyncPartBodyInner);

    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub type AsyncPart = Part<AsyncPartBody>;

    impl AsyncPart {
        #[inline]
        pub fn text(value: impl Into<Cow<'static, str>>) -> Self {
            let bytes = match value.into() {
                Cow::Borrowed(slice) => Bytes::from_static(slice.as_bytes()),
                Cow::Owned(string) => Bytes::from(string),
            };
            Self {
                body: AsyncPartBody(AsyncPartBodyInner::Bytes(Cursor::new(bytes))),
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
                body: AsyncPartBody(AsyncPartBodyInner::Bytes(Cursor::new(bytes))),
                meta: Default::default(),
            }
        }

        #[inline]
        pub fn stream(value: AsyncStream) -> Self {
            Self {
                body: AsyncPartBody(AsyncPartBodyInner::Stream(value)),
                meta: Default::default(),
            }
        }

        #[inline]
        pub async fn file_path(path: impl AsRef<Path>) -> IoResult<Self> {
            let file = File::open(path.as_ref()).await?;
            let mut metadata = PartMetadata::default()
                .mime(mime_guess::from_path(path.as_ref()).first_or_octet_stream());
            if let Some(file_name) = path.as_ref().file_name() {
                let file_name = match file_name.to_string_lossy() {
                    Cow::Borrowed(str) => FileName::from(str),
                    Cow::Owned(string) => FileName::from(string),
                };
                metadata = metadata.file_name(file_name);
            }
            Ok(AsyncPart::stream(Box::new(file)).metadata(metadata))
        }
    }

    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub type AsyncMultipart = Multipart<AsyncPart>;

    impl AsyncMultipart {
        #[inline]
        pub(in super::super) fn into_async_read(mut self) -> Box<dyn AsyncRead + Send + Unpin> {
            if self.fields.is_empty() {
                return Box::new(Cursor::new([]));
            }

            let (name, part) = self.fields.pop_front().unwrap();
            let chain =
                Box::new(self.part_stream(&name, part)) as Box<dyn AsyncRead + Send + Unpin>;
            let fields = take(&mut self.fields);
            Box::new(
                fields
                    .into_iter()
                    .fold(chain, |readable, (name, part)| {
                        Box::new(readable.chain(self.part_stream(&name, part)))
                            as Box<dyn AsyncRead + Send + Unpin>
                    })
                    .chain(Cursor::new(b"--"))
                    .chain(Cursor::new(self.boundary.to_owned()))
                    .chain(Cursor::new(b"--\r\n")),
            )
        }

        #[inline]
        fn part_stream(&self, name: &str, part: AsyncPart) -> impl AsyncRead + Send + Unpin {
            Cursor::new(b"--")
                .chain(Cursor::new(self.boundary.to_owned()))
                .chain(Cursor::new(b"\r\n"))
                .chain(Cursor::new(encode_headers(name, &part.meta)))
                .chain(Cursor::new(b"\r\n\r\n"))
                .chain(part.body)
                .chain(Cursor::new(b"\r\n"))
        }
    }

    impl AsyncRead for AsyncPartBody {
        #[inline]
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<IoResult<usize>> {
            match &mut self.0 {
                AsyncPartBodyInner::Bytes(bytes) => Pin::new(bytes).poll_read(cx, buf),
                AsyncPartBodyInner::Stream(stream) => Pin::new(stream).poll_read(cx, buf),
            }
        }
    }
}

#[cfg(feature = "async")]
pub use async_part::{AsyncMultipart, AsyncPart, AsyncPartBody};

#[inline]
fn gen_boundary() -> Boundary {
    use std::fmt::Write;

    let mut b = Boundary::with_capacity(32);
    write!(b, "{:016x}{:016x}", random::<u64>(), random::<u64>()).unwrap();
    b
}

#[inline]
fn encode_headers(name: &str, field: &PartMetadata) -> HeaderBuffer {
    let mut buf = HeaderBuffer::from_slice(b"content-disposition: form-data; ");
    buf.extend_from_slice(&format_parameter("name", name));
    if let Some(file_name) = field.file_name.as_ref() {
        buf.extend_from_slice(b"; ");
        buf.extend_from_slice(format_file_name(file_name).as_bytes());
    }
    for (name, value) in field.headers.iter() {
        buf.extend_from_slice(b"\r\n");
        buf.extend_from_slice(name.as_str().as_bytes());
        buf.extend_from_slice(b": ");
        buf.extend_from_slice(value.as_bytes());
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
        for chunk in utf8_percent_encode(value, PATH_SEGMENT_ENCODE_SET) {
            buf.extend_from_slice(chunk.as_bytes());
        }
        buf
    };
    let mut formatted = HeaderBuffer::from_slice(name.as_bytes());
    if value.len() == legal_value.len() {
        formatted.extend_from_slice(b"=\"");
        formatted.extend_from_slice(value.as_bytes());
        formatted.extend_from_slice(b"\"");
    } else {
        formatted.extend_from_slice(b"*=utf-8''");
        formatted.extend_from_slice(&legal_value);
    };
    formatted
}

#[cfg(test)]
mod tests {
    use super::*;
    use mime::{APPLICATION_JSON, IMAGE_BMP};
    use std::{
        fs::File,
        io::{Cursor, Result as IoResult, Write},
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
        let metadata = PartMetadata {
            headers: {
                let mut headers = HeaderMap::default();
                headers.insert(
                    CONTENT_TYPE,
                    HeaderValue::from_str(APPLICATION_JSON.as_ref()).unwrap(),
                );
                headers
            },
            file_name: Some(name.into()),
        };

        assert_eq!(
            encode_headers(name, &metadata).as_ref(),
            "content-disposition: form-data; name*=utf-8''start%25'%22%0D%0A%C3%9Fend; filename=\"start%'\\\"\\\r\\\nßend\"\r\ncontent-type: application/json".as_bytes()
        );
    }

    #[test]
    fn test_multipart_into_read() -> IoResult<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let tempdir = tempdir()?;
        let temp_file_path = tempdir.path().join("fake-file.json");
        let mut file = File::create(&temp_file_path)?;
        file.write_all(b"{\"a\":\"b\"}\n")?;
        drop(file);

        let mut multipart = SyncMultipart::new()
            .add_part("bytes1", SyncPart::bytes(&b"part1"[..]))
            .add_part("text1", SyncPart::text("value1"))
            .add_part(
                "text2",
                SyncPart::text("value1").metadata(PartMetadata::default().mime(IMAGE_BMP)),
            )
            .add_part(
                "reader1",
                SyncPart::stream(Box::new(Cursor::new(b"value1"))),
            )
            .add_part("reader2", SyncPart::file_path(temp_file_path)?);
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
        multipart.into_read().read_to_string(&mut actual)?;
        assert_eq!(EXPECTED, actual);

        tempdir.close()?;
        Ok(())
    }

    #[cfg(feature = "async")]
    #[async_std::test]
    async fn test_multipart_into_async_read() -> IoResult<()> {
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
            .add_part(
                "text2",
                AsyncPart::text("value1").metadata(PartMetadata::default().mime(IMAGE_BMP)),
            )
            .add_part(
                "reader1",
                AsyncPart::stream(Box::new(AsyncCursor::new(b"value1"))),
            )
            .add_part("reader2", AsyncPart::file_path(temp_file_path).await?);
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
        multipart
            .into_async_read()
            .read_to_string(&mut actual)
            .await?;
        assert_eq!(EXPECTED, actual);

        tempdir.close()?;
        Ok(())
    }
}
