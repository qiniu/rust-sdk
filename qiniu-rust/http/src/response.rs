use super::{HeaderNameOwned, HeaderValueOwned, HeadersOwned};
use std::{
    convert::TryInto,
    default::Default,
    fmt::Debug,
    fs::File,
    io::{copy as io_copy, Error as IOError, ErrorKind as IOErrorKind, Read, Result as IOResult},
    mem::take,
    net::IpAddr,
};
use tempfile::tempfile;

/// HTTP 响应状态码
pub type StatusCode = u16;

/// 实现了 Read 和 Debug 的 Trait
pub trait ReadDebug: Read + Debug {}

impl<T: Read + Debug> ReadDebug for T {}

/// HTTP 响应体
#[derive(Debug)]
pub enum Body {
    Reader(Box<dyn ReadDebug>),
    File(File),
    Bytes(Vec<u8>),
}

impl Default for Body {
    #[inline]
    fn default() -> Self {
        Self::Bytes(Default::default())
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
            body: Body::Bytes(Default::default()),
            server_ip: None,
            server_port: 80,
        }
    }
}

impl Response {
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
    pub fn server_ip(&self) -> Option<&IpAddr> {
        self.server_ip.as_ref()
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

    /// 取出响应体
    ///
    /// 原响应中的响应体为替换为空
    #[inline]
    pub fn take_body(&mut self) -> Body {
        take(&mut self.body)
    }

    /// 复制响应体
    ///
    /// 该方法将尝试读取响应体，然后复制其内容
    pub fn clone_body(&mut self) -> IOResult<Body> {
        let content_length = self.try_to_get_content_length();
        return match self.body_mut() {
            Body::Reader(reader) => {
                let [body1, body2] = clone_body_from_reader(reader, content_length)?;
                *self.body_mut() = body1;
                Ok(body2)
            }
            Body::File(file) => Ok(Body::File(file.try_clone()?)),
            Body::Bytes(body) => Ok(Body::Bytes(body.to_owned())),
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
                    return Ok([Body::Bytes(buf.to_owned()), Body::Bytes(buf)]);
                }
            }
            let mut file = tempfile()?;
            io_copy(body, &mut file)?;
            Ok([Body::File(file.try_clone()?), Body::File(file)])
        }
    }

    /// 获取响应体长度
    pub fn body_len(&mut self) -> IOResult<u64> {
        if let Some(content_length) = self.try_to_get_content_length() {
            return Ok(content_length);
        }
        match self.body() {
            Body::Bytes(body) => Ok(body.len().try_into().unwrap()),
            Body::File(file) => Ok(file.metadata()?.len().try_into().unwrap()),
            Body::Reader(_) => match self.clone_body()? {
                Body::Bytes(body) => Ok(body.len().try_into().unwrap()),
                Body::File(file) => Ok(file.metadata()?.len().try_into().unwrap()),
                _ => panic!("Unexpected body type"),
            },
        }
    }

    fn try_to_get_content_length(&self) -> Option<u64> {
        self.header("Content-Length")
            .and_then(|content_length| content_length.parse().ok())
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
    pub fn stream_as_body(mut self, body: impl Read + Debug + 'static) -> Self {
        self.inner.body = Body::Reader(Box::new(body));
        self
    }

    /// 设置二进制字节数组为 HTTP 响应体
    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.inner.body = Body::Bytes(body.into());
        self
    }

    /// 设置文件为 HTTP 响应体
    #[inline]
    pub fn file_as_body(mut self, body: File) -> Self {
        self.inner.body = Body::File(body);
        self
    }

    /// 构建 HTTP 请求
    #[inline]
    pub fn build(self) -> Response {
        self.inner
    }
}
