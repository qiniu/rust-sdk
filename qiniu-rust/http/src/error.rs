use std::result;

pub enum Error {
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
    UnknownError(Box<str>),

    /// JSON 解析错误
    ParseJSONError,

    /// 响应码错误
    ResponseError,

    /// 用户取消
    UserCancelled,

    /// 恶意响应
    MaliciousResponse,
}

pub type Result<T> = result::Result<T, Error>;
