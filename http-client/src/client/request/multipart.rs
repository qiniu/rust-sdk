use assert_impl::assert_impl;
use mime::Mime;
use once_cell::sync::Lazy;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use qiniu_http::{
    header::{HeaderName, IntoHeaderName, CONTENT_TYPE},
    HeaderMap, HeaderValue,
};
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
    ffi::OsStr,
    fmt,
    iter::FromIterator,
    ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo},
};

/// 文件名
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileName {
    inner: SmallString<[u8; 64]>,
}
wrap_smallstr!(FileName);

/// Multipart 字段名称
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

/// Multipart 表单
///
/// ### 发送 Mutlipart 表单代码实例
///
/// ```
/// async fn example() -> anyhow::Result<()> {
/// # use async_std::io::ReadExt;
/// use qiniu_credential::Credential;
/// use qiniu_http_client::{
///     prelude::*, AsyncMultipart, AsyncPart, BucketRegionsQueryer, HttpClient, Idempotent, PartMetadata, RegionsProviderEndpoints, ServiceName,
/// };
/// use qiniu_upload_token::UploadPolicy;
/// use serde_json::Value;
/// use std::time::Duration;
///
/// # let file = async_std::io::Cursor::new(vec![0u8; 1024]);
/// let credential = Credential::new("abcdefghklmnopq", "1234567890");
/// let bucket_name = "test-bucket";
/// let object_name = "test-key";
/// let provider = UploadPolicy::new_for_object(bucket_name, object_name, Duration::from_secs(3600))
///     .build()
///     .into_dynamic_upload_token_provider(&credential);
/// let upload_token = provider
///     .async_to_token_string(Default::default())
///     .await?;
/// let value: Value = HttpClient::default()
///     .async_post(
///         &[ServiceName::Up],
///         RegionsProviderEndpoints::new(
///             BucketRegionsQueryer::new().query(credential.access_key().to_owned(), bucket_name),
///         ),
///     )
///     .idempotent(Idempotent::Always)
///     .accept_json()
///     .multipart(
///         AsyncMultipart::new()
///             .add_part("token", AsyncPart::text(upload_token))
///             .add_part("key", AsyncPart::text(object_name))
///             .add_part(
///                 "file",
///                 AsyncPart::stream(file).metadata(PartMetadata::default().file_name("fakefilename.bin")),
///             ),
///     )
///     .await?
///     .call()
///     .await?
///     .parse_json()
///     .await?
///     .into_body();
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Multipart<P> {
    boundary: Boundary,
    fields: VecDeque<(FieldName, P)>,
}

/// Multipart 表单组件
#[derive(Debug)]
pub struct Part<B> {
    meta: PartMetadata,
    body: B,
}

impl<P> Default for Multipart<P> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<P> Multipart<P> {
    /// 创建 Multipart 表单
    #[inline]
    pub fn new() -> Self {
        Self {
            boundary: gen_boundary(),
            fields: Default::default(),
        }
    }

    pub(super) fn boundary(&self) -> &str {
        &self.boundary
    }

    /// 添加 Multipart 表单组件
    #[inline]
    #[must_use]
    pub fn add_part(mut self, name: impl Into<FieldName>, part: P) -> Self {
        self.fields.push_back((name.into(), part));
        self
    }
}

impl<P: Sync + Send> Multipart<P> {
    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// Multipart 表单组件元信息
#[derive(Default, Debug)]
pub struct PartMetadata {
    headers: HeaderMap,
    file_name: Option<FileName>,
}

impl PartMetadata {
    /// 设置表单组件的 MIME 类型
    #[inline]
    #[must_use]
    pub fn mime(self, mime: Mime) -> Self {
        self.add_header(CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap())
    }

    /// 添加表单组件的 HTTP 头
    #[inline]
    #[must_use]
    pub fn add_header(mut self, name: impl IntoHeaderName, value: impl Into<HeaderValue>) -> Self {
        self.headers.insert(name, value.into());
        self
    }

    /// 设置表单组件的文件名
    #[inline]
    #[must_use]
    pub fn file_name(mut self, file_name: impl Into<FileName>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }
}

impl Extend<(HeaderName, HeaderValue)> for PartMetadata {
    #[inline]
    fn extend<T: IntoIterator<Item = (HeaderName, HeaderValue)>>(&mut self, iter: T) {
        self.headers.extend(iter)
    }
}

impl Extend<(Option<HeaderName>, HeaderValue)> for PartMetadata {
    #[inline]
    fn extend<T: IntoIterator<Item = (Option<HeaderName>, HeaderValue)>>(&mut self, iter: T) {
        self.headers.extend(iter)
    }
}

impl<B> Part<B> {
    /// 设置 Multipart 表单组件的元信息
    #[inline]
    #[must_use]
    pub fn metadata(mut self, metadata: PartMetadata) -> Self {
        self.meta = metadata;
        self
    }
}

impl<B: Sync + Send> Part<B> {
    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

mod sync_part {
    use super::*;
    use std::{
        fmt::{self, Debug},
        fs::File,
        io::{Cursor, Read, Result as IoResult},
        mem::take,
        path::Path,
    };

    enum SyncPartBodyInner<'a> {
        Bytes(Cursor<Cow<'a, [u8]>>),
        Stream(Box<dyn Read + 'a>),
    }

    impl Debug for SyncPartBodyInner<'_> {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Bytes(bytes) => f.debug_tuple("Bytes").field(bytes).finish(),
                Self::Stream(_) => f.debug_tuple("Stream").finish(),
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

        /// 设置阻塞 Multipart 的请求体为输入流
        #[inline]
        #[must_use]
        pub fn stream(value: impl Read + 'a) -> Self {
            Self {
                body: SyncPartBody(SyncPartBodyInner::Stream(Box::new(value))),
                meta: Default::default(),
            }
        }

        /// 设置阻塞 Multipart 的请求体为文件
        pub fn file_path<S: AsRef<OsStr> + ?Sized>(path: &S) -> IoResult<Self> {
            let path = Path::new(path);
            let file = File::open(&path)?;
            let mut metadata = PartMetadata::default().mime(mime_guess::from_path(&path).first_or_octet_stream());
            if let Some(file_name) = path.file_name() {
                let file_name = match file_name.to_string_lossy() {
                    Cow::Borrowed(str) => FileName::from(str),
                    Cow::Owned(string) => FileName::from(string),
                };
                metadata = metadata.file_name(file_name);
            }
            Ok(SyncPart::stream(file).metadata(metadata))
        }
    }

    /// 阻塞 Multipart
    pub type SyncMultipart<'a> = Multipart<SyncPart<'a>>;

    impl<'a> SyncMultipart<'a> {
        pub(in super::super) fn into_read(mut self) -> Box<dyn Read + 'a> {
            if self.fields.is_empty() {
                return Box::new(Cursor::new([]));
            }

            let (name, part) = self.fields.pop_front().unwrap();
            let chain = Box::new(self.part_stream(&name, part)) as Box<dyn Read + 'a>;
            let fields = take(&mut self.fields);
            Box::new(
                fields
                    .into_iter()
                    .fold(chain, |readable, (name, part)| {
                        Box::new(readable.chain(self.part_stream(&name, part))) as Box<dyn Read + 'a>
                    })
                    .chain(Cursor::new(b"--"))
                    .chain(Cursor::new(self.boundary.to_owned()))
                    .chain(Cursor::new(b"--\r\n")),
            )
        }

        fn part_stream(&self, name: &str, part: SyncPart<'a>) -> impl Read + 'a {
            Cursor::new(b"--")
                .chain(Cursor::new(self.boundary.to_owned()))
                .chain(Cursor::new(b"\r\n"))
                .chain(Cursor::new(encode_headers(name, &part.meta)))
                .chain(Cursor::new(b"\r\n\r\n"))
                .chain(part.body)
                .chain(Cursor::new(b"\r\n"))
        }
    }

    impl Read for SyncPartBody<'_> {
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
    use futures::io::{AsyncRead, AsyncReadExt, Cursor};
    use std::{
        fmt::{self, Debug},
        io::Result as IoResult,
        mem::take,
        pin::Pin,
        task::{Context, Poll},
    };

    type AsyncStream<'a> = Box<dyn AsyncRead + Send + Unpin + 'a>;

    enum AsyncPartBodyInner<'a> {
        Bytes(Cursor<Cow<'a, [u8]>>),
        Stream(AsyncStream<'a>),
    }

    impl Debug for AsyncPartBodyInner<'_> {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Bytes(bytes) => f.debug_tuple("Bytes").field(bytes).finish(),
                Self::Stream(_) => f.debug_tuple("Stream").finish(),
            }
        }
    }

    /// 异步 Multipart 表单组件请求体
    #[derive(Debug)]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub struct AsyncPartBody<'a>(AsyncPartBodyInner<'a>);

    /// 异步 Multipart 表单组件
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
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

        /// 设置异步 Multipart 的请求体为异步输入流
        #[inline]
        #[must_use]
        pub fn stream(value: impl AsyncRead + Send + Unpin + 'a) -> Self {
            Self {
                body: AsyncPartBody(AsyncPartBodyInner::Stream(Box::new(value))),
                meta: Default::default(),
            }
        }

        /// 设置异步 Multipart 的请求体为文件
        pub async fn file_path<S: AsRef<OsStr> + ?Sized>(path: &S) -> IoResult<AsyncPart<'a>> {
            let path = Path::new(path);
            let file = File::open(&path).await?;
            let mut metadata = PartMetadata::default().mime(mime_guess::from_path(&path).first_or_octet_stream());
            if let Some(file_name) = path.file_name() {
                let file_name = match file_name.to_string_lossy() {
                    Cow::Borrowed(str) => FileName::from(str),
                    Cow::Owned(string) => FileName::from(string),
                };
                metadata = metadata.file_name(file_name);
            }
            Ok(AsyncPart::stream(file).metadata(metadata))
        }
    }

    /// 异步 Multipart
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub type AsyncMultipart<'a> = Multipart<AsyncPart<'a>>;

    impl<'a> AsyncMultipart<'a> {
        pub(in super::super) fn into_async_read(mut self) -> Box<dyn AsyncRead + Send + Unpin + 'a> {
            if self.fields.is_empty() {
                return Box::new(Cursor::new([]));
            }

            let (name, part) = self.fields.pop_front().unwrap();
            let chain = Box::new(self.part_stream(&name, part)) as Box<dyn AsyncRead + Send + Unpin + 'a>;
            let fields = take(&mut self.fields);
            Box::new(
                fields
                    .into_iter()
                    .fold(chain, |readable, (name, part)| {
                        Box::new(readable.chain(self.part_stream(&name, part)))
                            as Box<dyn AsyncRead + Send + Unpin + 'a>
                    })
                    .chain(Cursor::new(b"--"))
                    .chain(Cursor::new(self.boundary.to_owned()))
                    .chain(Cursor::new(b"--\r\n")),
            )
        }

        fn part_stream(&self, name: &str, part: AsyncPart<'a>) -> impl AsyncRead + Send + Unpin + 'a {
            Cursor::new(b"--")
                .chain(Cursor::new(self.boundary.to_owned()))
                .chain(Cursor::new(b"\r\n"))
                .chain(Cursor::new(encode_headers(name, &part.meta)))
                .chain(Cursor::new(b"\r\n\r\n"))
                .chain(part.body)
                .chain(Cursor::new(b"\r\n"))
        }
    }

    impl AsyncRead for AsyncPartBody<'_> {
        #[inline]
        fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<IoResult<usize>> {
            match &mut self.0 {
                AsyncPartBodyInner::Bytes(bytes) => Pin::new(bytes).poll_read(cx, buf),
                AsyncPartBodyInner::Stream(stream) => Pin::new(stream).poll_read(cx, buf),
            }
        }
    }
}

#[cfg(feature = "async")]
pub use async_part::{AsyncMultipart, AsyncPart, AsyncPartBody};

fn gen_boundary() -> Boundary {
    use std::fmt::Write;

    let mut b = Boundary::with_capacity(32);
    write!(b, "{:016x}{:016x}", random::<u64>(), random::<u64>()).unwrap();
    b
}

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

fn format_file_name(filename: &str) -> FileName {
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("\\\\|\"|\r|\n").unwrap());
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
                headers.insert(CONTENT_TYPE, HeaderValue::from_str(APPLICATION_JSON.as_ref()).unwrap());
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
            .add_part("bytes1", SyncPart::bytes(b"part1".as_slice()))
            .add_part("text1", SyncPart::text("value1"))
            .add_part(
                "text2",
                SyncPart::text("value1").metadata(PartMetadata::default().mime(IMAGE_BMP)),
            )
            .add_part("reader1", SyncPart::stream(Cursor::new(b"value1")))
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
            .add_part("bytes1", AsyncPart::bytes(b"part1".as_slice()))
            .add_part("text1", AsyncPart::text("value1"))
            .add_part(
                "text2",
                AsyncPart::text("value1").metadata(PartMetadata::default().mime(IMAGE_BMP)),
            )
            .add_part("reader1", AsyncPart::stream(AsyncCursor::new(b"value1")))
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
        multipart.into_async_read().read_to_string(&mut actual).await?;
        assert_eq!(EXPECTED, actual);

        tempdir.close()?;
        Ok(())
    }
}
