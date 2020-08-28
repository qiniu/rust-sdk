use std::{error, fmt, result};

/// HTTP 响应错误类型
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorType {
    /// 协议错误，该协议不能支持
    ProtocolError,

    /// 非法的 URL 错误
    InvalidURLError,

    /// 网络连接失败
    ConnectError,

    /// 代理连接失败
    ProxyError,

    /// 域名解析失败
    UnknownHostError,

    /// 传输失败
    TransmissionError,

    /// 超时失败
    TimeoutError,

    /// SSL 错误
    SSLError,

    /// 重定向次数过多
    TooManyRedirect,

    /// 未知错误
    UnknownError,

    /// 用户取消
    UserCancelled,
}

/// HTTP 响应错误
#[derive(Clone, Debug)]
pub struct Error {
    error_type: ErrorType,
    description: Box<str>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.description.fmt(f)
    }
}

impl error::Error for Error {}

pub type Result<T> = result::Result<T, Error>;
