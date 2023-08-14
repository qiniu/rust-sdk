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
    fmt,
    iter::FromIterator,
    ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo},
};

mod sync_part;
pub use sync_part::{SyncMultipart, SyncPart, SyncPartBody};

#[cfg(feature = "async")]
mod async_part;

#[cfg(feature = "async")]
pub use async_part::{AsyncMultipart, AsyncPart, AsyncPartBody};

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
///                 AsyncPart::seekable(file).metadata(PartMetadata::default().file_name("fakefilename.bin")),
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
    use mime::APPLICATION_JSON;

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
}
