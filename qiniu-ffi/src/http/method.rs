use qiniu_http::Method;
use std::fmt;

/// @brief 七牛 HTTP 方法
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug, Eq)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_http_method_t {
    qiniu_ng_http_method_get,
    qiniu_ng_http_method_head,
    qiniu_ng_http_method_post,
    qiniu_ng_http_method_put,
}

impl From<Method> for qiniu_ng_http_method_t {
    fn from(m: Method) -> Self {
        match m {
            Method::GET => Self::qiniu_ng_http_method_get,
            Method::HEAD => Self::qiniu_ng_http_method_head,
            Method::POST => Self::qiniu_ng_http_method_post,
            Method::PUT => Self::qiniu_ng_http_method_put,
        }
    }
}

impl From<qiniu_ng_http_method_t> for Method {
    fn from(m: qiniu_ng_http_method_t) -> Self {
        match m {
            qiniu_ng_http_method_t::qiniu_ng_http_method_get => Method::GET,
            qiniu_ng_http_method_t::qiniu_ng_http_method_head => Method::HEAD,
            qiniu_ng_http_method_t::qiniu_ng_http_method_post => Method::POST,
            qiniu_ng_http_method_t::qiniu_ng_http_method_put => Method::PUT,
        }
    }
}

impl fmt::Display for qiniu_ng_http_method_t {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Method::from(*self).fmt(f)
    }
}
