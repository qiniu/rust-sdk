use std::{error, fmt};

/// HTTP 响应错误类型
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorKind {
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

    /// 发送失败
    SendError,

    /// 接受失败
    ReceiveError,

    /// 本地 IO 失败
    LocalIOError,

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
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    error: Box<dyn error::Error + Send + Sync>,
}

impl Error {
    /// 创建 HTTP 响应错误
    #[inline]
    pub fn new(kind: ErrorKind, err: impl Into<Box<dyn error::Error + Send + Sync>>) -> Self {
        Error {
            kind,
            error: err.into(),
        }
    }

    /// 获取 HTTP 响应错误类型
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.error.fmt(f)
    }
}

impl error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(self.error.as_ref())
    }
}
