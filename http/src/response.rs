use super::{HeaderNameOwned, HeaderValueOwned, HeadersOwned, ResponseError};
use assert_impl::assert_impl;
use std::{
    convert::TryInto,
    default::Default,
    fmt::Debug,
    fs::File,
    io::{
        copy as io_copy, Cursor, Error as IOError, ErrorKind as IOErrorKind, Read,
        Result as IOResult, Seek, SeekFrom,
    },
    net::IpAddr,
    result,
};
use tempfile::tempfile;

/// HTTP 响应状态码
pub type StatusCode = u16;

trait ReadSeekDebug: Read + Seek + Debug + Send + Sync {}
impl<T: Read + Seek + Debug + Send + Sync> ReadSeekDebug for T {}

/// HTTP 响应体
#[derive(Debug)]
pub struct Body(BodyInner);

#[derive(Debug)]
enum BodyInner {
    SeekableReader(Box<dyn ReadSeekDebug>),
    File(File),
    Bytes(Cursor<Vec<u8>>),
}

impl Default for Body {
    #[inline]
    fn default() -> Self {
        Self(BodyInner::Bytes(Default::default()))
    }
}

impl Read for Body {
    fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
        match &mut self.0 {
            BodyInner::SeekableReader(reader) => reader.read(buf),
            BodyInner::File(file) => file.read(buf),
            BodyInner::Bytes(bytes) => bytes.read(buf),
        }
    }
}

impl Seek for Body {
    fn seek(&mut self, pos: SeekFrom) -> IOResult<u64> {
        match &mut self.0 {
            BodyInner::SeekableReader(reader) => reader.seek(pos),
            BodyInner::File(file) => file.seek(pos),
            BodyInner::Bytes(bytes) => bytes.seek(pos),
        }
    }
}

/// HTTP 响应
///
/// 封装 HTTP 响应相关字段
#[derive(Debug)]
pub struct Response {
    status_code: StatusCode,
    headers: HeadersOwned,
    body: Body,
    server_ip: Option<IpAddr>,
    server_port: u16,
}

impl Default for Response {
    #[inline]
    fn default() -> Self {
        Self {
            status_code: 200,
            headers: Default::default(),
            body: Default::default(),
            server_ip: None,
            server_port: 80,
        }
    }
}

impl Response {
    // 返回 HTTP 响应构建器
    #[inline]
    pub fn builder() -> ResponseBuilder {
        ResponseBuilder::default()
    }

    /// HTTP 状态码
    #[inline]
    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    /// 修改 HTTP 状态码
    #[inline]
    pub fn status_code_mut(&mut self) -> &mut StatusCode {
        &mut self.status_code
    }

    /// HTTP Headers
    #[inline]
    pub fn headers(&self) -> &HeadersOwned {
        &self.headers
    }

    /// 修改 HTTP Headers
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeadersOwned {
        &mut self.headers
    }

    /// HTTP 响应体
    #[inline]
    pub fn body(&self) -> &Body {
        &self.body
    }

    /// 修改 HTTP 响应体
    #[inline]
    pub fn body_mut(&mut self) -> &mut Body {
        &mut self.body
    }

    /// HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.server_ip
    }

    /// 修改 HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip_mut(&mut self) -> Option<&mut IpAddr> {
        self.server_ip.as_mut()
    }

    /// HTTP 服务器端口号
    #[inline]
    pub fn server_port(&self) -> u16 {
        self.server_port
    }

    /// 修改 HTTP 服务器端口号
    #[inline]
    pub fn server_port_mut(&mut self) -> &mut u16 {
        &mut self.server_port
    }
}

impl Response {
    /// 获取 HTTP 响应 Header
    #[inline]
    pub fn header(&self, header_name: impl Into<HeaderNameOwned>) -> Option<&HeaderValueOwned> {
        self.headers.get(&header_name.into())
    }

    /// 取出响应体
    #[inline]
    pub fn into_body(self) -> Body {
        self.body
    }

    /// 复制响应体
    ///
    /// 该方法将尝试读取响应体，然后复制其内容
    pub fn clone_body(&mut self) -> IOResult<Body> {
        let content_length = self.try_to_get_content_length();
        return match &mut self.body_mut().0 {
            BodyInner::SeekableReader(reader) => {
                let [body1, body2] = clone_body_from_reader(reader, content_length)?;
                *self.body_mut() = body1;
                Ok(body2)
            }
            BodyInner::File(file) => Ok(Body(BodyInner::File(file.try_clone()?))),
            BodyInner::Bytes(body) => Ok(Body(BodyInner::Bytes(body.to_owned()))),
        };

        fn clone_body_from_reader(
            body: &mut dyn Read,
            content_length: Option<u64>,
        ) -> IOResult<[Body; 2]> {
            if let Some(content_length) = content_length {
                if content_length < 1 << 12 {
                    let mut buf = Vec::new();
                    if content_length as usize != body.read_to_end(&mut buf)? {
                        return Err(IOError::from(IOErrorKind::UnexpectedEof));
                    }
                    return Ok([
                        Body(BodyInner::Bytes(Cursor::new(buf.to_owned()))),
                        Body(BodyInner::Bytes(Cursor::new(buf))),
                    ]);
                }
            }
            let mut file = tempfile()?;
            io_copy(body, &mut file)?;
            Ok([
                Body(BodyInner::File(file.try_clone()?)),
                Body(BodyInner::File(file)),
            ])
        }
    }

    /// 获取响应体长度
    pub fn body_len(&mut self) -> IOResult<u64> {
        if let Some(content_length) = self.try_to_get_content_length() {
            return Ok(content_length);
        }
        return match &mut self.body_mut().0 {
            BodyInner::Bytes(body) => Ok(body.get_ref().len().try_into().unwrap()),
            BodyInner::File(file) => Ok(file.metadata()?.len()),
            BodyInner::SeekableReader(body) => get_len_from_seekable(body),
        };

        fn get_len_from_seekable(body: &mut dyn Seek) -> IOResult<u64> {
            let cur_offset = body.seek(SeekFrom::Current(0))?;
            let len = body.seek(SeekFrom::End(0))?;
            body.seek(SeekFrom::Start(cur_offset))?;
            Ok(len)
        }
    }

    fn try_to_get_content_length(&self) -> Option<u64> {
        self.header("Content-Length")
            .and_then(|content_length| content_length.parse().ok())
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

#[derive(Debug, Default)]
pub struct ResponseBuilder {
    inner: Response,
}

impl ResponseBuilder {
    /// 设置 HTTP 状态码
    #[inline]
    pub fn status_code(mut self, status_code: StatusCode) -> Self {
        self.inner.status_code = status_code;
        self
    }

    /// 设置 HTTP Headers
    #[inline]
    pub fn headers(mut self, headers: HeadersOwned) -> Self {
        self.inner.headers = headers;
        self
    }

    /// 设置 HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip(mut self, server_ip: Option<IpAddr>) -> Self {
        self.inner.server_ip = server_ip;
        self
    }

    /// 设置 HTTP 服务器端口号
    #[inline]
    pub fn server_port(mut self, server_port: u16) -> Self {
        self.inner.server_port = server_port;
        self
    }

    /// 添加 HTTP Header
    #[inline]
    pub fn header(
        mut self,
        header_name: impl Into<HeaderNameOwned>,
        header_value: impl Into<HeaderValueOwned>,
    ) -> Self {
        self.inner
            .headers
            .insert(header_name.into(), header_value.into());
        self
    }

    /// 设置数据流为 HTTP 响应体
    #[inline]
    pub fn seekable_stream_as_body(
        mut self,
        body: impl Read + Seek + Debug + Send + Sync + 'static,
    ) -> Self {
        self.inner.body = Body(BodyInner::SeekableReader(Box::new(body)));
        self
    }

    /// 设置二进制字节数组为 HTTP 响应体
    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.inner.body = Body(BodyInner::Bytes(Cursor::new(body.into())));
        self
    }

    /// 设置文件为 HTTP 响应体
    #[inline]
    pub fn file_as_body(mut self, mut body: File) -> IOResult<Self> {
        body.seek(SeekFrom::Start(0))?;
        self.inner.body = Body(BodyInner::File(body));
        Ok(self)
    }

    /// 构建 HTTP 请求
    #[inline]
    pub fn build(self) -> Response {
        self.inner
    }
}

/// HTTP 响应结果
pub type Result = result::Result<Response, ResponseError>;
