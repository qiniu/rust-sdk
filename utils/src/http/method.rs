use std::{error::Error, fmt, str::FromStr};

/// HTTP 方法
///
/// 这里的 HTTP 方法并不完整，但已经满足了七牛 SDK 的需要
#[derive(Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Method {
    /// GET 方法
    GET,
    /// HEAD 方法
    HEAD,
    /// POST 方法
    POST,
    /// PUT 方法
    PUT,
}

impl Method {
    /// 将 HTTP 方法转换成字符串
    pub fn as_str(&self) -> &str {
        match self {
            Method::GET => "GET",
            Method::HEAD => "HEAD",
            Method::POST => "POST",
            Method::PUT => "PUT",
        }
    }

    /// 将 HTTP 方法转换成二进制字节数组
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Method::GET => b"GET",
            Method::HEAD => b"HEAD",
            Method::POST => b"POST",
            Method::PUT => b"PUT",
        }
    }
}

impl FromStr for Method {
    type Err = InvalidMethod;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "GET" => Ok(Method::GET),
            "HEAD" => Ok(Method::HEAD),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            method => Err(InvalidMethod(method.into())),
        }
    }
}

impl AsRef<str> for Method {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> PartialEq<&'a Method> for Method {
    #[inline]
    fn eq(&self, other: &&'a Method) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<Method> for &'a Method {
    #[inline]
    fn eq(&self, other: &Method) -> bool {
        *self == other
    }
}

impl PartialEq<str> for Method {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_ref() == other
    }
}

impl PartialEq<Method> for str {
    #[inline]
    fn eq(&self, other: &Method) -> bool {
        self == other.as_ref()
    }
}

impl<'a> PartialEq<&'a str> for Method {
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        self.as_ref() == *other
    }
}

impl<'a> PartialEq<Method> for &'a str {
    #[inline]
    fn eq(&self, other: &Method) -> bool {
        *self == other.as_ref()
    }
}

impl fmt::Debug for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl fmt::Display for Method {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.as_ref())
    }
}

impl Default for Method {
    #[inline]
    fn default() -> Method {
        Method::GET
    }
}

impl<'a> From<&'a Method> for Method {
    #[inline]
    fn from(t: &'a Method) -> Self {
        *t
    }
}

/// 非法的 HTTP 方法错误
#[derive(Clone, Debug)]
pub struct InvalidMethod(Box<str>);

impl fmt::Display for InvalidMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid HTTP method")
    }
}

impl Error for InvalidMethod {}
