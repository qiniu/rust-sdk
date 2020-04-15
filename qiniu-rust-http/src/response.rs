use super::{HeaderNameOwned, HeaderValueOwned, HeadersOwned};
use derive_builder::Builder;
use getset::{CopyGetters, Getters, MutGetters};
use std::{
    boxed::Box,
    convert::TryInto,
    default::Default,
    fmt,
    fs::File,
    io::{copy as io_copy, Error as IOError, ErrorKind as IOErrorKind, Read, Result as IOResult, Seek, SeekFrom},
    net::IpAddr,
};
use tempfile::tempfile;

/// HTTP 响应状态码
pub type StatusCode = u16;

/// HTTP 响应体
pub enum Body {
    Reader(Box<dyn Read>),
    Bytes(Vec<u8>),
    File(File),
}

/// HTTP 响应
///
/// 封装 HTTP 响应相关字段
#[derive(Getters, CopyGetters, MutGetters, Builder)]
#[builder(
    pattern = "owned",
    setter(into, strip_option),
    build_fn(name = "inner_build", private)
)]
pub struct Response {
    /// HTTP 状态码
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    #[builder(default = "200")]
    status_code: StatusCode,

    /// HTTP Headers
    #[get = "pub"]
    #[get_mut = "pub"]
    #[builder(default)]
    headers: HeadersOwned,

    /// HTTP 响应体
    #[get = "pub"]
    #[get_mut = "pub"]
    #[builder(private, default)]
    body: Option<Body>,

    /// HTTP 服务器 IP 地址
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    #[builder(default)]
    server_ip: Option<IpAddr>,

    /// HTTP 服务器端口号
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    #[builder(default)]
    server_port: u16,
}

impl Response {
    /// 获取 HTTP 响应 Header
    pub fn header(&self, header_name: impl Into<HeaderNameOwned>) -> Option<&HeaderValueOwned> {
        self.headers.get(&header_name.into())
    }

    /// 取出响应体
    pub fn into_body(self) -> Option<Body> {
        self.body
    }

    /// 取出响应体
    ///
    /// 原响应中的响应体为替换为 `NULL`
    pub fn take_body(&mut self) -> Option<Body> {
        self.body.take()
    }

    /// 复制响应体
    ///
    /// 该方法将尝试读取响应体，然后复制其内容
    pub fn clone_body(&mut self) -> IOResult<Option<Body>> {
        let content_length = self.try_to_get_content_length();
        return match self.body_mut() {
            Some(Body::Reader(reader)) => {
                let [body1, body2] = clone_body_from_reader(reader, content_length)?;
                *self.body_mut() = Some(body1);
                Ok(Some(body2))
            }
            Some(Body::File(file)) => {
                let [body1, body2] = clone_body_from_reader(file, content_length)?;
                *self.body_mut() = Some(body1);
                Ok(Some(body2))
            }
            Some(Body::Bytes(body)) => Ok(Some(Body::Bytes(body.to_owned()))),
            None => Ok(None),
        };

        fn clone_body_from_reader(body: &mut dyn Read, content_length: Option<u64>) -> IOResult<[Body; 2]> {
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
            Some(Body::Bytes(body)) => Ok(body.len().try_into().unwrap()),
            None => Ok(0),
            _ => match self.clone_body()? {
                Some(Body::Bytes(body)) => Ok(body.len().try_into().unwrap()),
                Some(Body::File(file)) => Ok(file.metadata()?.len().try_into().unwrap()),
                _ => panic!("Unexpected body type"),
            },
        }
    }

    fn try_to_get_content_length(&self) -> Option<u64> {
        self.header("Content-Length")
            .and_then(|content_length| content_length.parse().ok())
    }
}

impl ResponseBuilder {
    /// 添加 HTTP 响应 Header
    pub fn header(
        mut self,
        header_name: impl Into<HeaderNameOwned>,
        header_value: impl Into<HeaderValueOwned>,
    ) -> Self {
        match &mut self.headers {
            Some(headers) => {
                headers.insert(header_name.into(), header_value.into());
            }
            None => {
                let mut headers = HeadersOwned::new();
                headers.insert(header_name.into(), header_value.into());
                self = self.headers(headers);
            }
        }
        self
    }

    /// 设置数据流为 HTTP 响应体
    pub fn stream_as_body(self, body: impl Read + 'static) -> Self {
        self.body(Body::Reader(Box::new(body)))
    }

    /// 设置二进制字节数组为 HTTP 响应体
    pub fn bytes_as_body(self, body: impl Into<Vec<u8>>) -> Self {
        self.body(Body::Bytes(body.into()))
    }

    /// 设置文件为 HTTP 响应体
    pub fn file_as_body(self, mut body: File) -> IOResult<Self> {
        body.seek(SeekFrom::Start(0))?;
        Ok(self.body(Body::File(body)))
    }

    /// 生成 HTTP 响应
    pub fn build(self) -> Response {
        self.inner_build().unwrap()
    }
}

impl Default for Response {
    fn default() -> Self {
        ResponseBuilder::default().build()
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Response")
            .field("status_code", &self.status_code())
            .field("headers", self.headers())
            .field("server_ip", &self.server_ip())
            .field("server_port", &self.server_port())
            .finish()
    }
}
