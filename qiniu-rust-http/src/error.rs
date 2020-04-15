use super::{
    method::Method,
    request::Request,
    response::{Response, StatusCode},
};
use getset::{CopyGetters, Getters};
use std::{boxed::Box, error::Error as StdError, fmt, io, marker::Send, ops::Deref, result};

/// 出现请求错误时的 URL
pub type URL = Box<str>;

/// 出现请求错误时的七牛请求 ID
pub type RequestID = Box<str>;

/// HTTP 请求结果
pub type Result<T> = result::Result<T, Error>;

/// 错误可重试性类型
///
/// 对于可重试性的概念，可以参见主页中 [重试策略，幂等性与重试安全之间的关系](index.html#重试策略幂等性与重试安全之间的关系)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RetryKind {
    /// 总是可重试的错误
    RetryableError,
    /// 区域不可重试错误
    ZoneUnretryableError,
    /// 主机不可重试错误
    HostUnretryableError,
    /// 不可重试错误
    UnretryableError,
}

/// JSON 错误
#[derive(Debug)]
pub struct JSONError(_JSONError);

#[derive(Debug)]
enum _JSONError {
    SerdeJSONError(serde_json::Error),
    Description(Box<str>),
}

/// HTTP 错误类型
#[derive(Debug)]
pub enum ErrorKind {
    /// HTTP 调用错误
    HTTPCallerError(HTTPCallerError),
    /// JSON 错误
    JSONError(JSONError),
    /// 恶意响应
    //
    /// 响应可能并非来自七牛
    MaliciousResponse,
    /// 非预期的重定向
    UnexpectedRedirect,
    /// IO 错误
    IOError(io::Error),
    /// 未知错误
    UnknownError(Box<dyn StdError + Send>),
    /// 响应状态码错误
    ResponseStatusCodeError(StatusCode, Box<str>),
    /// 用户取消
    UserCanceled,
}

/// HTTP 调用错误
///
/// 由于与 HTTP 客户端实现解耦合，但外层依然需要根据错误类型判断处理方式。因此需要在赋值错误时给出错误类型。
#[derive(Debug, Getters, CopyGetters)]
pub struct HTTPCallerError {
    /// 错误类型
    #[get_copy = "pub"]
    kind: HTTPCallerErrorKind,

    /// 错误内容
    #[get = "pub"]
    inner: Box<dyn StdError + Send>,
}

impl ErrorKind {
    /// 创建 HTTP 调用错误
    pub fn new_http_caller_error_kind(kind: HTTPCallerErrorKind, error: impl StdError + Send + 'static) -> Self {
        ErrorKind::HTTPCallerError(HTTPCallerError {
            kind,
            inner: Box::new(error),
        })
    }
}

/// HTTP 调用错误类型
#[derive(Debug, Copy, Clone)]
pub enum HTTPCallerErrorKind {
    /// DNS 解析错误
    ResolveError,
    /// 代理错误
    ProxyError,
    /// SSL 错误
    SSLError,
    /// 连接错误
    ConnectionError,
    /// 请求错误
    RequestError,
    /// 响应错误
    ResponseError,
    /// 超时错误
    TimeoutError,
    /// 未知错误
    UnknownError,
}

/// HTTP 错误
#[derive(Getters, CopyGetters)]
pub struct Error {
    /// 可重试类型
    #[get_copy = "pub"]
    retry_kind: RetryKind,

    /// 是否重试安全
    #[get_copy = "pub"]
    is_retry_safe: bool,

    /// 错误类型
    #[get = "pub"]
    error_kind: ErrorKind,

    /// HTTP 请求方法
    #[get_copy = "pub"]
    method: Option<Method>,

    /// HTTP 请求 ID
    #[get = "pub"]
    request_id: Option<RequestID>,

    /// HTTP 请求 URL
    #[get = "pub"]
    url: Option<URL>,
}

impl Error {
    /// 通过请求和响应创建 HTTP 错误
    pub fn new_from_req_resp(
        retry_kind: RetryKind,
        error_kind: ErrorKind,
        is_retry_safe: bool,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Error {
            retry_kind,
            error_kind,
            is_retry_safe,
            method: Some(request.method()),
            request_id: Self::extract_req_id_from_response(response),
            url: Some(request.url().into()),
        }
    }

    /// 通过请求和响应创建可重试的 HTTP 错误
    pub fn new_retryable_error_from_req_resp(
        error_kind: ErrorKind,
        is_retry_safe: bool,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new_from_req_resp(RetryKind::RetryableError, error_kind, is_retry_safe, request, response)
    }

    /// 通过请求和响应创建区域不可重试的 HTTP 错误
    pub fn new_zone_unretryable_error_from_req_resp(
        error_kind: ErrorKind,
        is_retry_safe: bool,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new_from_req_resp(
            RetryKind::ZoneUnretryableError,
            error_kind,
            is_retry_safe,
            request,
            response,
        )
    }

    /// 通过请求和响应创建主机不可重试的 HTTP 错误
    pub fn new_host_unretryable_error_from_req_resp(
        error_kind: ErrorKind,
        is_retry_safe: bool,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new_from_req_resp(
            RetryKind::HostUnretryableError,
            error_kind,
            is_retry_safe,
            request,
            response,
        )
    }

    /// 通过请求和响应创建不可重试的 HTTP 错误
    ///
    /// 由于是不可重试的，因此总被认为是重试不安全的
    pub fn new_unretryable_error_from_req_resp(
        error_kind: ErrorKind,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new_from_req_resp(RetryKind::UnretryableError, error_kind, false, request, response)
    }

    /// 通过直接赋值的方式创建 HTTP 错误
    ///
    /// 此类错误通常发生的 HTTP 请求发生前，因此 `#new` 方法的 `request` 参数无法给出，此时，可以调用该方法，直接传入 URL，HTTP 和 RequestID 方法即可
    pub fn new(
        retry_kind: RetryKind,
        error_kind: ErrorKind,
        is_retry_safe: bool,
        method: Option<Method>,
        url: Option<URL>,
        request_id: Option<RequestID>,
    ) -> Error {
        Error {
            retry_kind,
            error_kind,
            is_retry_safe,
            method,
            url,
            request_id,
        }
    }

    /// 创建可重试的 HTTP 错误
    ///
    /// 此类错误通常发生的 HTTP 请求发生前，因此 `#new` 方法的 `request` 参数无法给出，此时，可以调用该方法，直接传入 URL，HTTP 和 RequestID 方法即可
    pub fn new_retryable_error(
        error_kind: ErrorKind,
        is_retry_safe: bool,
        method: Option<Method>,
        url: Option<URL>,
        request_id: Option<RequestID>,
    ) -> Error {
        Self::new(
            RetryKind::RetryableError,
            error_kind,
            is_retry_safe,
            method,
            url,
            request_id,
        )
    }

    /// 创建区域不可重试的 HTTP 错误
    ///
    /// 此类错误通常发生的 HTTP 请求发生前，因此 `#new` 方法的 `request` 参数无法给出，此时，可以调用该方法，直接传入 URL，HTTP 和 RequestID 方法即可
    pub fn new_zone_unretryable_error(
        error_kind: ErrorKind,
        is_retry_safe: bool,
        method: Option<Method>,
        url: Option<URL>,
        request_id: Option<RequestID>,
    ) -> Error {
        Self::new(
            RetryKind::ZoneUnretryableError,
            error_kind,
            is_retry_safe,
            method,
            url,
            request_id,
        )
    }

    /// 创建主机不可重试的 HTTP 错误
    ///
    /// 此类错误通常发生的 HTTP 请求发生前，因此 `#new` 方法的 `request` 参数无法给出，此时，可以调用该方法，直接传入 URL，HTTP 和 RequestID 方法即可
    pub fn new_host_unretryable_error(
        error_kind: ErrorKind,
        is_retry_safe: bool,
        method: Option<Method>,
        url: Option<URL>,
        request_id: Option<RequestID>,
    ) -> Error {
        Self::new(
            RetryKind::HostUnretryableError,
            error_kind,
            is_retry_safe,
            method,
            url,
            request_id,
        )
    }

    /// 创建不可重试的 HTTP 错误
    ///
    /// 此类错误通常发生的 HTTP 请求发生前，因此 `#new` 方法的 `request` 参数无法给出，此时，可以调用该方法，直接传入 URL，HTTP 和 RequestID 方法即可。
    /// 由于是不可重试的，因此总被认为是重试不安全的
    pub fn new_unretryable_error(
        error_kind: ErrorKind,
        method: Option<Method>,
        url: Option<URL>,
        request_id: Option<RequestID>,
    ) -> Error {
        Self::new(RetryKind::UnretryableError, error_kind, false, method, url, request_id)
    }

    fn extract_req_id_from_response(response: Option<&Response>) -> Option<RequestID> {
        response.and_then(|resp| {
            resp.headers()
                .get(&"X-Reqid".into())
                .map(|v| v.to_owned().into_boxed_str())
        })
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Error")
            .field("retry_kind", &self.retry_kind)
            .field("error_kind", &self.error_kind)
            .field("method", &self.method)
            .field("url", &self.url)
            .field("request_id", &self.request_id)
            .field("is_retry_safe", &self.is_retry_safe)
            .finish()
    }
}

impl fmt::Display for JSONError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            _JSONError::SerdeJSONError(err) => err.fmt(f),
            _JSONError::Description(err) => err.fmt(f),
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::HTTPCallerError(err) => write!(f, "HTTPCallerError({})", err.inner),
            Self::JSONError(err) => write!(f, "JSONError({})", err),
            Self::MaliciousResponse => write!(f, "MaliciousResponse"),
            Self::UnexpectedRedirect => write!(f, "UnexpectedRedirect"),
            Self::UserCanceled => write!(f, "UserCanceled"),
            Self::IOError(err) => write!(f, "IOError({})", err),
            Self::UnknownError(err) => write!(f, "UnknownError({})", err),
            Self::ResponseStatusCodeError(status_code, error_message) => write!(
                f,
                "ResponseStatusCodeError(status_code = {}, error_message = {})",
                status_code, error_message
            ),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "retry_kind: {:?}, error_kind: {}, method: {}, url: {}, request_id: {}, is_retry_safe: {}",
            self.retry_kind,
            self.error_kind,
            self.method.as_ref().map(|m| m.as_str()).unwrap_or("None"),
            self.url.as_ref().map(|u| &u as &str).unwrap_or("None"),
            self.request_id.as_ref().map(|u| &u as &str).unwrap_or("None"),
            if self.is_retry_safe { "True" } else { "False" },
        )
    }
}

impl StdError for JSONError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        match &self.0 {
            _JSONError::SerdeJSONError(err) => err.description(),
            _JSONError::Description(err) => err.as_ref(),
        }
    }
    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn StdError> {
        match &self.0 {
            _JSONError::SerdeJSONError(err) => Some(err),
            _JSONError::Description(_) => None,
        }
    }
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match &self.0 {
            _JSONError::SerdeJSONError(err) => Some(err),
            _JSONError::Description(_) => None,
        }
    }
}

impl StdError for Error {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        match &self.error_kind {
            ErrorKind::HTTPCallerError(err) => err.description(),
            ErrorKind::JSONError(err) => err.description(),
            ErrorKind::MaliciousResponse => "Malicious response",
            ErrorKind::UnexpectedRedirect => "Unexpected redirect",
            ErrorKind::UserCanceled => "User canceled",
            ErrorKind::IOError(err) => err.description(),
            ErrorKind::UnknownError(err) => err.description(),
            ErrorKind::ResponseStatusCodeError(_, error_message) => &error_message,
        }
    }

    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn StdError> {
        match &self.error_kind {
            ErrorKind::HTTPCallerError(err) => err.cause(),
            ErrorKind::JSONError(err) => err.cause(),
            ErrorKind::IOError(err) => Some(err),
            ErrorKind::UnknownError(err) => Some(err.deref()),
            _ => None,
        }
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match &self.error_kind {
            ErrorKind::HTTPCallerError(err) => err.source(),
            ErrorKind::JSONError(err) => err.source(),
            ErrorKind::IOError(err) => Some(err),
            ErrorKind::UnknownError(err) => Some(err.deref()),
            _ => None,
        }
    }
}

impl fmt::Display for HTTPCallerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl StdError for HTTPCallerError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        self.inner.description()
    }

    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn StdError> {
        Some(self.inner.deref())
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.inner.deref())
    }
}

impl From<serde_json::Error> for JSONError {
    fn from(err: serde_json::Error) -> Self {
        Self(_JSONError::SerdeJSONError(err))
    }
}

impl From<Box<str>> for JSONError {
    fn from(err: Box<str>) -> Self {
        Self(_JSONError::Description(err))
    }
}

impl From<String> for JSONError {
    fn from(err: String) -> Self {
        Self(_JSONError::Description(err.into_boxed_str()))
    }
}
