use crate::{
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr, UCString},
    utils::{qiniu_ng_readable_t, qiniu_ng_str_list_t, qiniu_ng_str_map_t, qiniu_ng_str_t},
};
use libc::{c_char, c_void, size_t};

#[cfg(windows)]
use winapi::shared::{
    in6addr::in6_addr,
    inaddr::in_addr,
    ws2def::{AF_INET, AF_INET6},
};

#[cfg(not(windows))]
use libc::{in6_addr, in_addr, AF_INET, AF_INET6};

use qiniu_http::{HeaderName, Method, Request, Response, ResponseBody};
use std::{
    borrow::Cow,
    collections::{hash_map::RandomState, HashMap},
    convert::TryInto,
    ffi::CStr,
    fs::{File, OpenOptions},
    io::{copy as io_copy, sink as io_sink, Read, Write},
    mem::transmute,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    ptr::{copy_nonoverlapping, null_mut},
    slice::{from_raw_parts, from_raw_parts_mut},
};
use tap::TapOps;

/// @brief HTTP 方法
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_http_method_t {
    /// GET 方法
    qiniu_ng_http_method_get,
    /// HEAD 方法
    qiniu_ng_http_method_head,
    /// POST 方法
    qiniu_ng_http_method_post,
    /// PUT 方法
    qiniu_ng_http_method_put,
}

impl qiniu_ng_http_method_t {
    pub fn as_cstr(self) -> &'static CStr {
        match self {
            qiniu_ng_http_method_t::qiniu_ng_http_method_get => unsafe {
                CStr::from_bytes_with_nul_unchecked(b"GET\0")
            },
            qiniu_ng_http_method_t::qiniu_ng_http_method_head => unsafe {
                CStr::from_bytes_with_nul_unchecked(b"HEAD\0")
            },
            qiniu_ng_http_method_t::qiniu_ng_http_method_post => unsafe {
                CStr::from_bytes_with_nul_unchecked(b"POST\0")
            },
            qiniu_ng_http_method_t::qiniu_ng_http_method_put => unsafe {
                CStr::from_bytes_with_nul_unchecked(b"PUT\0")
            },
        }
    }
}

impl From<Method> for qiniu_ng_http_method_t {
    fn from(method: Method) -> Self {
        match method {
            Method::GET => qiniu_ng_http_method_t::qiniu_ng_http_method_get,
            Method::HEAD => qiniu_ng_http_method_t::qiniu_ng_http_method_head,
            Method::POST => qiniu_ng_http_method_t::qiniu_ng_http_method_post,
            Method::PUT => qiniu_ng_http_method_t::qiniu_ng_http_method_put,
        }
    }
}

impl From<qiniu_ng_http_method_t> for Method {
    fn from(method: qiniu_ng_http_method_t) -> Self {
        match method {
            qiniu_ng_http_method_t::qiniu_ng_http_method_get => Method::GET,
            qiniu_ng_http_method_t::qiniu_ng_http_method_head => Method::HEAD,
            qiniu_ng_http_method_t::qiniu_ng_http_method_post => Method::POST,
            qiniu_ng_http_method_t::qiniu_ng_http_method_put => Method::PUT,
        }
    }
}

impl From<qiniu_ng_http_method_t> for *const c_char {
    fn from(method: qiniu_ng_http_method_t) -> Self {
        method.as_cstr().as_ptr()
    }
}

/// @brief HTTP 请求
/// @details 该结构体封装 HTTP 请求相关数据
/// @note 无需对该结构体进行内存释放
/// @note 该结构体不可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_http_request_t(*mut c_void);

impl From<qiniu_ng_http_request_t> for &Request<'_> {
    fn from(request: qiniu_ng_http_request_t) -> Self {
        unsafe { transmute(request) }
    }
}

impl From<qiniu_ng_http_request_t> for &mut Request<'_> {
    fn from(request: qiniu_ng_http_request_t) -> Self {
        unsafe { transmute(request) }
    }
}

impl From<&Request<'_>> for qiniu_ng_http_request_t {
    fn from(request: &Request<'_>) -> Self {
        unsafe { transmute(request) }
    }
}

impl From<&mut Request<'_>> for qiniu_ng_http_request_t {
    fn from(request: &mut Request<'_>) -> Self {
        unsafe { transmute(request) }
    }
}

/// @brief 获取 HTTP 请求的 URL
/// @param[in] request HTTP 请求实例
/// @retval qiniu_ng_str_t 返回 HTTP 请求的 URL
/// @warning 务必记得 `qiniu_ng_str_t` 需要在使用完毕后调用 `qiniu_ng_str_free()` 释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_url(request: qiniu_ng_http_request_t) -> qiniu_ng_str_t {
    let request: &Request = request.into();
    unsafe { qiniu_ng_str_t::from_str_unchecked(request.url()) }.tap(|_| {
        let _ = qiniu_ng_http_request_t::from(request);
    })
}

/// @brief 设置 HTTP 请求的 URL
/// @param[in] request HTTP 请求实例
/// @param[in] url URL 字符串
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_set_url(request: qiniu_ng_http_request_t, url: *const qiniu_ng_char_t) {
    let request: &mut Request = request.into();
    *request.url_mut() = unsafe { ucstr::from_ptr(url) }.to_string().unwrap().into();
    let _ = qiniu_ng_http_request_t::from(request);
}

/// @brief 获取 HTTP 请求的方法
/// @param[in] request HTTP 请求实例
/// @retval qiniu_ng_http_method_t 返回 HTTP 请求的方法
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_method(request: qiniu_ng_http_request_t) -> qiniu_ng_http_method_t {
    let request: &Request = request.into();
    request
        .method()
        .tap(|_| {
            let _ = qiniu_ng_http_request_t::from(request);
        })
        .into()
}

/// @brief 设置 HTTP 请求的方法
/// @param[in] request HTTP 请求实例
/// @param[in] method HTTP 请求的方法
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_set_method(request: qiniu_ng_http_request_t, method: qiniu_ng_http_method_t) {
    let request: &mut Request = request.into();
    *request.method_mut() = method.into();
    let _ = qiniu_ng_http_request_t::from(request);
}

/// @brief 获取 HTTP 请求的 Header 值
/// @param[in] request HTTP 请求实例
/// @param[in] header_name HTTP 请求的 Header 名称
/// @retval qiniu_ng_str_t 返回 HTTP 请求的 Header 值，如果对应的 Header 名称找不到，返回的 `qiniu_ng_str_t` 中将封装 `NULL`
/// @warning 务必记得 `qiniu_ng_str_t` 需要在使用完毕后调用 `qiniu_ng_str_free()` 释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_header(
    request: qiniu_ng_http_request_t,
    header_name: *const qiniu_ng_char_t,
) -> qiniu_ng_str_t {
    let request: &Request = request.into();
    unsafe {
        qiniu_ng_str_t::from_optional_str_unchecked(
            request
                .headers()
                .get(&HeaderName::new(ucstr::from_ptr(header_name).to_string().unwrap()))
                .as_ref()
                .map(|header_value| header_value.as_ref()),
        )
    }
    .tap(|_| {
        let _ = qiniu_ng_http_request_t::from(request);
    })
}

/// @brief 获取 HTTP 请求的 Headers 键值对
/// @param[in] request HTTP 请求实例
/// @retval qiniu_ng_str_map_t 返回 HTTP 请求的 Headers 键值对
/// @warning 务必记得 `qiniu_ng_str_map_t` 需要在使用完毕后调用 `qiniu_ng_str_map_free()` 释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_headers(request: qiniu_ng_http_request_t) -> qiniu_ng_str_map_t {
    let request: &Request = request.into();
    let src_headers = request.headers();
    let mut dest_headers = Box::new(HashMap::<Box<ucstr>, Box<ucstr>, RandomState>::with_capacity(
        src_headers.len(),
    ));
    src_headers.iter().for_each(|(header_name, header_value)| {
        dest_headers.insert(
            UCString::from_str(header_name.as_ref()).unwrap().into_boxed_ucstr(),
            UCString::from_str(header_value.as_ref()).unwrap().into_boxed_ucstr(),
        );
    });
    let _ = qiniu_ng_http_request_t::from(request);
    dest_headers.into()
}

/// @brief 设置 HTTP 请求的 Header
/// @param[in] request HTTP 请求实例
/// @param[in] header_name HTTP 请求的 Header 名称
/// @param[in] header_value HTTP 请求的 Header 值
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_set_header(
    request: qiniu_ng_http_request_t,
    header_name: *const qiniu_ng_char_t,
    header_value: *const qiniu_ng_char_t,
) {
    let request: &mut Request = request.into();
    if let Some(header_value) = unsafe { header_value.as_ref() } {
        request.headers_mut().insert(
            HeaderName::new(unsafe { ucstr::from_ptr(header_name) }.to_string().unwrap()),
            unsafe { ucstr::from_ptr(header_value) }.to_string().unwrap().into(),
        );
    } else {
        request.headers_mut().remove(&HeaderName::new(
            unsafe { ucstr::from_ptr(header_name) }.to_string().unwrap(),
        ));
    }
    let _ = qiniu_ng_http_request_t::from(request);
}

/// @brief 获取 HTTP 请求体
/// @param[in] request HTTP 请求实例
/// @param[out] body_ptr 用于返回 HTTP 请求体地址。如果传入 `NULL` 表示不获取 `body_ptr`，但不影响 `body_size` 的获取
/// @param[out] body_size 用于返回 HTTP 请求体长度，单位为字节。如果传入 `NULL` 表示不获取 `body_size`，但不影响 `body_ptr` 的获取
/// @warning 请勿修改其存储的请求体内容，您可以调用 `qiniu_ng_http_request_set_body()` 设置内容
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_body(
    request: qiniu_ng_http_request_t,
    body_ptr: *mut *const c_void,
    body_size: *mut size_t,
) {
    let request: &Request = request.into();
    if let Some(body) = request.body().as_ref() {
        if let Some(body_size) = unsafe { body_size.as_mut() } {
            *body_size = body.len();
        }
        if let Some(body_ptr) = unsafe { body_ptr.as_mut() } {
            *body_ptr = body.as_ref().as_ptr().cast();
        }
    } else {
        if let Some(body_size) = unsafe { body_size.as_mut() } {
            *body_size = 0;
        }
        if let Some(body_ptr) = unsafe { body_ptr.as_mut() } {
            *body_ptr = null_mut();
        }
    }
    let _ = qiniu_ng_http_request_t::from(request);
}

/// @brief 设置 HTTP 请求体
/// @param[in] request HTTP 请求实例
/// @param[in] body_ptr HTTP 请求体地址。
/// @param[in] body_size HTTP 请求体长度，单位为字节。
/// @note 设置请求体时，SDK 客户端会复制并存储输入的请求体内容，因此 `body_ptr` 在使用完毕后即可释放
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_set_body(
    request: qiniu_ng_http_request_t,
    body_ptr: *const c_void,
    body_size: size_t,
) {
    let request: &mut Request = request.into();
    *request.body_mut() = if body_size == 0 {
        None
    } else {
        let mut buf = Vec::new();
        buf.extend_from_slice(unsafe { from_raw_parts(body_ptr.cast(), body_size) });
        Some(buf.into())
    };
    let _ = qiniu_ng_http_request_t::from(request);
}

/// @brief HTTP 请求是否自动跟踪重定向
/// @param[in] request HTTP 请求实例
/// @retval bool 如果 HTTP 请求自动跟踪重定向，则返回 `true`
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_will_follow_redirection(request: qiniu_ng_http_request_t) -> bool {
    let request: &Request = request.into();
    request.follow_redirection().tap(|_| {
        let _ = qiniu_ng_http_request_t::from(request);
    })
}

/// @brief 设置 HTTP 请求的自动跟踪重定向
/// @param[in] request HTTP 请求实例
/// @param[in] follow_redirection 是否自动重定向
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_set_follow_redirection(
    request: qiniu_ng_http_request_t,
    follow_redirection: bool,
) {
    let request: &mut Request = request.into();
    *request.follow_redirection_mut() = follow_redirection;
    let _ = qiniu_ng_http_request_t::from(request);
}

/// @brief 以字符串列表的形式获取 HTTP 请求预解析的套接字地址列表（套接字地址即 IP 地址:端口号）
/// @param[in] request HTTP 请求实例
/// @retval qiniu_ng_str_list_t 返回包含预解析的套接字地址的字符串列表
/// @warning 务必记得 `qiniu_ng_str_list_t` 需要在使用完毕后调用 `qiniu_ng_str_list_free()` 释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_resolved_socket_addrs_as_str_list(
    request: qiniu_ng_http_request_t,
) -> qiniu_ng_str_list_t {
    let request: &Request = request.into();
    let resolved_socket_addrs = request
        .resolved_socket_addrs()
        .iter()
        .map(|addr| addr.to_string())
        .collect::<Vec<_>>();
    unsafe { qiniu_ng_str_list_t::from_string_vec_unchecked(resolved_socket_addrs) }.tap(|_| {
        let _ = qiniu_ng_http_request_t::from(request);
    })
}

/// @brief 获取 HTTP 请求预解析的套接字地址列表长度
/// @param[in] request HTTP 请求实例
/// @retval size_t 返回预解析的 套接字地址的数量
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_resolved_socket_addrs_len(request: qiniu_ng_http_request_t) -> size_t {
    let request: &Request = request.into();
    request.resolved_socket_addrs().len().tap(|_| {
        let _ = qiniu_ng_http_request_t::from(request);
    })
}

/// @brief 根据索引获取 HTTP 请求的预解析的套接字地址（套接字地址即 IP 地址:端口号）
/// @param[in] request HTTP 请求实例
/// @param[in] index 预解析的套接字地址列表索引
/// @param[out] sa_family 用于返回预解析的套接字地址所在地址族，对于 IPv4 地址，将返回 `AF_INET`，对于 IPv6 地址，将返回 `AF_INET6`。如果传入 `NULL` 表示不获取 `sa_family`，但不会影响 API 返回值和其他数据的返回
/// @param[out] ip_addr 用于返回预解析的套接字地址中的 IP 地址，对于 IPv4 地址，将返回 `struct in_addr` 类型的数据，对于 IPv6 地址，将返回 `struct in6_addr` 类型的数据。如果传入 `NULL` 表示不获取 `ip_addr`，但不会影响 API 返回值和其他数据的返回
/// @param[out] port 用于返回预解析的套接字地址中的端口号。如果传入 `NULL` 表示不获取 `port`，但不会影响 API 返回值和其他数据的返回
/// @retval bool 如果预解析的套接字地址列表中该索引对应的套接字地址存在，则返回 `true`，否则返回 `false`
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_resolved_socket_addr(
    request: qiniu_ng_http_request_t,
    index: size_t,
    sa_family: *mut u16,
    ip_addr: *mut c_void,
    port: *mut u16,
) -> bool {
    let request: &Request = request.into();
    return match request.resolved_socket_addrs().as_ref().get(index) {
        Some(SocketAddr::V4(socket_addr)) => set_resolved_socket_addr_as_ipv4(socket_addr, sa_family, ip_addr, port),
        Some(SocketAddr::V6(socket_addr)) => set_resolved_socket_addr_as_ipv6(socket_addr, sa_family, ip_addr, port),
        None => false,
    }
    .tap(|_| {
        let _ = qiniu_ng_http_request_t::from(request);
    });

    fn set_resolved_socket_addr_as_ipv4(
        socket_addr: &SocketAddrV4,
        sa_family: *mut u16,
        ip_addr: *mut c_void,
        port: *mut u16,
    ) -> bool {
        if let Some(sa_family) = unsafe { sa_family.as_mut() } {
            *sa_family = AF_INET as u16;
        }
        let ip_addr: *mut in_addr = ip_addr.cast();
        if let Some(ip_addr) = unsafe { ip_addr.as_mut() } {
            *ip_addr = from_ipv4_addr_to_in_addr(*socket_addr.ip());
        }
        if let Some(port) = unsafe { port.as_mut() } {
            *port = socket_addr.port();
        }
        true
    }
    fn set_resolved_socket_addr_as_ipv6(
        socket_addr: &SocketAddrV6,
        sa_family: *mut u16,
        ip_addr: *mut c_void,
        port: *mut u16,
    ) -> bool {
        if let Some(sa_family) = unsafe { sa_family.as_mut() } {
            *sa_family = AF_INET6 as u16;
        }
        let ip_addr: *mut in6_addr = ip_addr.cast();
        if let Some(ip_addr) = unsafe { ip_addr.as_mut() } {
            *ip_addr = from_ipv6_addr_to_in6_addr(*socket_addr.ip());
        }
        if let Some(port) = unsafe { port.as_mut() } {
            *port = socket_addr.port();
        }
        true
    }
}

/// @brief 清空 HTTP 请求预解析的套接字地址列表
/// @param[in] request HTTP 请求实例
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_clear_resolved_socket_addrs(request: qiniu_ng_http_request_t) {
    let request: &mut Request = request.into();
    *request.resolved_socket_addrs_mut() = Cow::Borrowed(&[]);
    let _ = qiniu_ng_http_request_t::from(request);
}

/// @brief 以字符串的格式追加 HTTP 请求的预解析的套接字地址（套接字地址即 IP 地址:端口号）
/// @param[in] request HTTP 请求实例
/// @param[in] socket_addr 字符串格式的预解析的套接字地址
/// @retval bool 如果套接字地址解析并追加成功，则返回 `true`。否则返回 `false`
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_append_resolved_socket_addr_as_str(
    request: qiniu_ng_http_request_t,
    socket_addr: *const qiniu_ng_char_t,
) -> bool {
    let request: &mut Request = request.into();
    let socket_addr: SocketAddr = {
        match unsafe { ucstr::from_ptr(socket_addr) }
            .to_string()
            .ok()
            .and_then(|s| s.parse().ok())
        {
            Some(socket_addr) => socket_addr,
            None => {
                return false;
            }
        }
    };
    match request.resolved_socket_addrs_mut() {
        Cow::Borrowed(resolved_socket_addrs_ref) => {
            let mut resolved_socket_addrs: Vec<SocketAddr> = (*resolved_socket_addrs_ref).to_owned();
            resolved_socket_addrs.push(socket_addr);
            *request.resolved_socket_addrs_mut() = Cow::Owned(resolved_socket_addrs);
        }
        Cow::Owned(resolved_socket_addrs) => {
            resolved_socket_addrs.push(socket_addr);
        }
    }
    let _ = qiniu_ng_http_request_t::from(request);
    true
}

/// @brief 以 `struct in_addr` 类型追加 HTTP 请求的预解析的 IPv4 套接字地址（套接字地址即 IP 地址:端口号）
/// @param[in] request HTTP 请求实例
/// @param[in] ip_addr 要追加的 IPv4 套接字地址中的 IP 地址
/// @param[in] port 要追加的 IPv4 套接字地址中的端口号
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_append_resolved_ipv4_socket_addr(
    request: qiniu_ng_http_request_t,
    ip_addr: in_addr,
    port: u16,
) {
    let request: &mut Request = request.into();
    let socket_addr = SocketAddr::V4(SocketAddrV4::new(from_in_addr_to_ipv4_addr(ip_addr), port));
    match request.resolved_socket_addrs_mut() {
        Cow::Borrowed(resolved_socket_addrs_ref) => {
            let mut resolved_socket_addrs: Vec<SocketAddr> = (*resolved_socket_addrs_ref).to_owned();
            resolved_socket_addrs.push(socket_addr);
            *request.resolved_socket_addrs_mut() = Cow::Owned(resolved_socket_addrs);
        }
        Cow::Owned(resolved_socket_addrs) => {
            resolved_socket_addrs.push(socket_addr);
        }
    }
    let _ = qiniu_ng_http_request_t::from(request);
}

/// @brief 以 `struct in6_addr` 类型追加 HTTP 请求的预解析的 IPv6 套接字地址（套接字地址即 IP 地址:端口号）
/// @param[in] request HTTP 请求实例
/// @param[in] ip_addr 要追加的 IPv6 套接字地址中的 IP 地址
/// @param[in] port 要追加的 IPv6 套接字地址中的端口号
/// @param[in] flowinfo 要追加的 IPv6 套接字地址中的流标识符
/// @param[in] scope_id 要追加的 IPv6 套接字地址中的范围标识符
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_append_resolved_ipv6_socket_addr(
    request: qiniu_ng_http_request_t,
    ip_addr: in6_addr,
    port: u16,
    flowinfo: u32,
    scope_id: u32,
) {
    let request: &mut Request = request.into();
    let socket_addr = SocketAddr::V6(SocketAddrV6::new(
        from_in6_addr_to_ipv6_addr(ip_addr),
        port,
        flowinfo,
        scope_id,
    ));
    match request.resolved_socket_addrs_mut() {
        Cow::Borrowed(resolved_socket_addrs_ref) => {
            let mut resolved_socket_addrs: Vec<SocketAddr> = (*resolved_socket_addrs_ref).to_owned();
            resolved_socket_addrs.push(socket_addr);
            *request.resolved_socket_addrs_mut() = Cow::Owned(resolved_socket_addrs);
        }
        Cow::Owned(resolved_socket_addrs) => {
            resolved_socket_addrs.push(socket_addr);
        }
    }
    let _ = qiniu_ng_http_request_t::from(request);
}

/// @brief 获取 HTTP 请求中的自定义数据
/// @param[in] request HTTP 请求实例
/// @retval *void 获取自定义数据，如果不曾设置过自定义数据，则返回 `NULL`
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_custom_data(request: qiniu_ng_http_request_t) -> *mut c_void {
    let request: &Request = request.into();
    request.custom_data().tap(|_| {
        let _ = qiniu_ng_http_request_t::from(request);
    })
}

/// @brief 设置 HTTP 请求中的自定义数据
/// @details 自定义数据这个字段只要是提供给 HTTP 中间件使用，可以在 HTTP 请求被处理前设置，在处理后获取
/// @param[in] request HTTP 请求实例
/// @param[in] data 自定义数据
#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_set_custom_data(request: qiniu_ng_http_request_t, data: *mut c_void) {
    let request: &mut Request = request.into();
    *request.custom_data_mut() = data;
    let _ = qiniu_ng_http_request_t::from(request);
}

/// @brief HTTP 响应
/// @details 该结构体封装 HTTP 响应相关数据
/// @note 无需对该结构体进行内存释放
/// @note 该结构体不可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_http_response_t(*mut c_void);

impl From<qiniu_ng_http_response_t> for &Response {
    fn from(response: qiniu_ng_http_response_t) -> Self {
        unsafe { transmute(response) }
    }
}

impl From<qiniu_ng_http_response_t> for &mut Response {
    fn from(response: qiniu_ng_http_response_t) -> Self {
        unsafe { transmute(response) }
    }
}

impl From<&Response> for qiniu_ng_http_response_t {
    fn from(response: &Response) -> Self {
        unsafe { transmute(response) }
    }
}

impl From<&mut Response> for qiniu_ng_http_response_t {
    fn from(response: &mut Response) -> Self {
        unsafe { transmute(response) }
    }
}

/// @brief 获取 HTTP 响应的状态码
/// @param[in] response HTTP 响应实例
/// @retval uint16_t 返回 HTTP 响应的状态码
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_status_code(response: qiniu_ng_http_response_t) -> u16 {
    let response: &Response = response.into();
    response.status_code().tap(|_| {
        let _ = qiniu_ng_http_response_t::from(response);
    })
}

/// @brief 设置 HTTP 响应的状态码
/// @param[in] response HTTP 响应实例
/// @param[in] status_code 状态码
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_status_code(response: qiniu_ng_http_response_t, status_code: u16) {
    let response: &mut Response = response.into();
    *response.status_code_mut() = status_code;
}

/// @brief 获取 HTTP 响应的 Header 值
/// @param[in] response HTTP 响应实例
/// @param[in] header_name HTTP 响应的 Header 名称
/// @retval qiniu_ng_str_t 返回 HTTP 响应的 Header 值，如果对应的 Header 名称找不到，返回的 `qiniu_ng_str_t` 中将封装 `NULL`
/// @warning 务必记得 `qiniu_ng_str_t` 需要在使用完毕后调用 `qiniu_ng_str_free()` 释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_header(
    response: qiniu_ng_http_response_t,
    header_name: *const qiniu_ng_char_t,
) -> qiniu_ng_str_t {
    let response: &Response = response.into();
    unsafe {
        qiniu_ng_str_t::from_optional_str_unchecked(
            response
                .headers()
                .get(&HeaderName::new(ucstr::from_ptr(header_name).to_string().unwrap()))
                .as_ref()
                .map(|header_value| header_value.as_ref()),
        )
    }
    .tap(|_| {
        let _ = qiniu_ng_http_response_t::from(response);
    })
}

/// @brief 获取 HTTP 响应的 Headers 键值对
/// @param[in] response HTTP 响应实例
/// @retval qiniu_ng_str_map_t 返回 HTTP 响应的 Headers 键值对
/// @warning 务必记得 `qiniu_ng_str_map_t` 需要在使用完毕后调用 `qiniu_ng_str_map_free()` 释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_headers(response: qiniu_ng_http_response_t) -> qiniu_ng_str_map_t {
    let response: &Response = response.into();
    let src_headers = response.headers();
    let mut dest_headers = Box::new(HashMap::<Box<ucstr>, Box<ucstr>, RandomState>::with_capacity(
        src_headers.len(),
    ));
    src_headers.iter().for_each(|(header_name, header_value)| {
        dest_headers.insert(
            UCString::from_str(header_name.as_ref()).unwrap().into_boxed_ucstr(),
            UCString::from_str(header_value.as_ref()).unwrap().into_boxed_ucstr(),
        );
    });
    let _ = qiniu_ng_http_response_t::from(response);
    dest_headers.into()
}

/// @brief 设置 HTTP 响应的 Header
/// @param[in] response HTTP 响应实例
/// @param[in] header_name HTTP 响应的 Header 名称
/// @param[in] header_value HTTP 响应的 Header 值
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_header(
    response: qiniu_ng_http_response_t,
    header_name: *const qiniu_ng_char_t,
    header_value: *const qiniu_ng_char_t,
) {
    let response: &mut Response = response.into();
    if let Some(header_value) = unsafe { header_value.as_ref() } {
        response.headers_mut().insert(
            HeaderName::new(unsafe { ucstr::from_ptr(header_name) }.to_string().unwrap()),
            unsafe { ucstr::from_ptr(header_value) }.to_string().unwrap().into(),
        );
    } else {
        response.headers_mut().remove(&HeaderName::new(
            unsafe { ucstr::from_ptr(header_name) }.to_string().unwrap(),
        ));
    }
    let _ = qiniu_ng_http_response_t::from(response);
}

/// @brief 获取 HTTP 响应体的尺寸
/// @details 该 API 将会首先尝试获取 HTTP 响应中 `Content-Type` 的值，如果不存在，则尝试获取整个响应体并计算其尺寸
/// @param[in] response HTTP 响应实例
/// @param[out] length 用于返回 HTTP 响应体的尺寸，单位为字节。如果传入 `NULL` 表示不获取 `length`。但如果运行正常，返回值将依然是 `true`
/// @param[out] err 用于返回错误，如果传入 `NULL` 表示不获取 `err`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `length` 获得结果，如果返回 `false`，则表示可以读取 `err` 获得错误信息
/// @warning 对于运行错误的情况，需要调用 `qiniu_ng_err_t` 系列的函数判定具体错误并释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_body_length(
    response: qiniu_ng_http_response_t,
    length: *mut u64,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let response: &mut Response = response.into();
    match response.body_len() {
        Ok(len) => {
            if let Some(length) = unsafe { length.as_mut() } {
                *length = len;
            }
            true
        }
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            false
        }
    }
    .tap(|_| {
        let _ = qiniu_ng_http_response_t::from(response);
    })
}

/// @brief 提供缓冲区以获取 HTTP 响应体
/// @details
///     该 API 将会尝试获取整个响应体并加载到内存中，建议您先调用 `qiniu_ng_http_response_get_body_length()` 获取响应体具体尺寸，
///     当尺寸大小可以接受全部被载入内存时，才调用该函数
/// @param[in] response HTTP 响应实例
/// @param[in] max_body_size 缓冲区最大尺寸，单位为字节
/// @param[out] body_ptr 缓冲区地址
/// @param[out] body_size 最终获取的响应体尺寸，单位为字节
/// @param[out] err 用于返回错误，如果传入 `NULL` 表示不获取 `err`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `length` 获得结果，如果返回 `false`，则表示可以读取 `err` 获得错误信息
/// @warning 对于运行错误的情况，需要调用 `qiniu_ng_err_t` 系列的函数判定具体错误并释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_dump_body(
    response: qiniu_ng_http_response_t,
    max_body_size: u64,
    body_ptr: *mut c_void,
    body_size: *mut u64,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let response: &mut Response = response.into();
    return match response.clone_body() {
        Ok(Some(ResponseBody::Bytes(bytes))) => {
            qiniu_ng_http_response_dump_body_from_bytes(bytes, max_body_size, body_ptr, body_size)
        }
        Ok(Some(ResponseBody::Reader(reader))) => {
            qiniu_ng_http_response_dump_body_from_reader(reader, max_body_size, body_ptr, body_size, err)
        }
        Ok(Some(ResponseBody::File(file))) => {
            qiniu_ng_http_response_dump_body_from_file(file, max_body_size, body_ptr, body_size, err)
        }
        Ok(None) => {
            if let Some(body_size) = unsafe { body_size.as_mut() } {
                *body_size = 0;
            }
            true
        }
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            false
        }
    }
    .tap(|_| {
        let _ = qiniu_ng_http_response_t::from(response);
    });

    fn qiniu_ng_http_response_dump_body_from_bytes(
        bytes: Vec<u8>,
        max_body_size: u64,
        body_ptr: *mut c_void,
        body_size: *mut u64,
    ) -> bool {
        let bs = u64::max(max_body_size, bytes.len().try_into().unwrap());
        if let Some(body_ptr) = unsafe { body_ptr.as_mut() } {
            unsafe { copy_nonoverlapping(bytes.as_ptr().cast(), body_ptr, bs.try_into().unwrap()) };
        }
        if let Some(body_size) = unsafe { body_size.as_mut() } {
            *body_size = bs;
        }
        true
    }

    fn qiniu_ng_http_response_dump_body_from_reader(
        mut reader: Box<dyn Read>,
        max_body_size: u64,
        body_ptr: *mut c_void,
        body_size: *mut u64,
        err: *mut qiniu_ng_err_t,
    ) -> bool {
        let r = if let Some(body_ptr) = unsafe { body_ptr.as_mut() } {
            io_copy(&mut reader, &mut unsafe {
                from_raw_parts_mut(body_ptr as *mut c_void as *mut u8, max_body_size.try_into().unwrap())
            })
        } else {
            io_copy(&mut reader.take(max_body_size), &mut io_sink())
        };
        match r {
            Ok(bs) => {
                if let Some(body_size) = unsafe { body_size.as_mut() } {
                    *body_size = bs;
                }
                true
            }
            Err(ref e) => {
                if let Some(err) = unsafe { err.as_mut() } {
                    *err = e.into();
                }
                false
            }
        }
    }

    fn qiniu_ng_http_response_dump_body_from_file(
        mut file: File,
        max_body_size: u64,
        body_ptr: *mut c_void,
        body_size: *mut u64,
        err: *mut qiniu_ng_err_t,
    ) -> bool {
        let r = if let Some(body_ptr) = unsafe { body_ptr.as_mut() } {
            io_copy(&mut file, &mut unsafe {
                from_raw_parts_mut(body_ptr as *mut c_void as *mut u8, max_body_size.try_into().unwrap())
            })
        } else {
            io_copy(&mut file.take(max_body_size), &mut io_sink())
        };
        match r {
            Ok(bs) => {
                if let Some(body_size) = unsafe { body_size.as_mut() } {
                    *body_size = bs;
                }
                true
            }
            Err(ref e) => {
                if let Some(err) = unsafe { err.as_mut() } {
                    *err = e.into();
                }
                false
            }
        }
    }
}

/// @brief 提供文件路径以获取 HTTP 响应体
/// @details
///     该 API 将会尝试整个响应体并将响应体全部写入到指定文件中，
///     该 API 虽然性能不及 `qiniu_ng_http_response_dump_body()`，但可以适应更大的响应体内容，
///     建议您先调用 `qiniu_ng_http_response_get_body_length()` 获取响应体具体尺寸，然后决定调用哪个 API 获取响应体
/// @param[in] response HTTP 响应实例
/// @param[in] path 文件路径
/// @param[out] err 用于返回错误，如果传入 `NULL` 表示不获取 `err`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `false`，则表示可以读取 `err` 获得错误信息
/// @warning 对于运行错误的情况，需要调用 `qiniu_ng_err_t` 系列的函数判定具体错误并释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_dump_body_to_file(
    response: qiniu_ng_http_response_t,
    path: *const qiniu_ng_char_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let response: &mut Response = response.into();
    if let Err(ref e) = response
        .clone_body()
        .and_then(|body| {
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(unsafe { ucstr::from_ptr(path) }.to_path_buf())
                .map(|file| (file, body))
        })
        .and_then(|(mut file, mut body)| match &mut body {
            Some(ResponseBody::Bytes(bytes)) => file.write_all(&bytes),
            Some(ResponseBody::Reader(reader)) => io_copy(reader, &mut file).map(|_| ()),
            Some(ResponseBody::File(f)) => io_copy(&mut file, f).map(|_| ()),
            None => Ok(()),
        })
    {
        if let Some(err) = unsafe { err.as_mut() } {
            *err = e.into();
        }
        false
    } else {
        true
    }
    .tap(|_| {
        let _ = qiniu_ng_http_response_t::from(response);
    })
}

/// @brief 设置 HTTP 响应体
/// @param[in] response HTTP 响应实例
/// @param[in] body_ptr HTTP 响应体地址
/// @param[in] body_size HTTP 响应体长度，单位为字节
/// @note 设置响应体时，SDK 客户端会复制并存储输入的响应体内容，因此 `body_ptr` 在使用完毕后即可释放
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_body(
    response: qiniu_ng_http_response_t,
    body_ptr: *const c_void,
    body_size: size_t,
) {
    let response: &mut Response = response.into();
    *response.body_mut() = if body_size == 0 {
        None
    } else {
        let mut buf = Vec::new();
        buf.extend_from_slice(unsafe { from_raw_parts(body_ptr.cast(), body_size) });
        Some(ResponseBody::Bytes(buf))
    };
    let _ = qiniu_ng_http_response_t::from(response);
}

/// @brief 将指定文件内容作为 HTTP 响应体
/// @param[in] response HTTP 响应实例
/// @param[in] path 文件路径
/// @param[out] err 用于返回错误，如果传入 `NULL` 表示不获取 `err`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示设置正确，如果返回 `false`，则表示可以读取 `err` 获得错误信息
/// @warning 对于运行错误的情况，需要调用 `qiniu_ng_err_t` 系列的函数判定具体错误并释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_body_to_file(
    response: qiniu_ng_http_response_t,
    path: *const qiniu_ng_char_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let response: &mut Response = response.into();
    match File::open(unsafe { ucstr::from_ptr(path) }.to_path_buf()) {
        Ok(file) => {
            *response.body_mut() = Some(ResponseBody::File(file));
            true
        }
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            false
        }
    }
    .tap(|_| {
        let _ = qiniu_ng_http_response_t::from(response);
    })
}

/// @brief 将指定的数据阅读器作为 HTTP 响应体
/// @param[in] response HTTP 响应实例
/// @param[in] readable 数据阅读器
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_body_to_reader(
    response: qiniu_ng_http_response_t,
    readable: qiniu_ng_readable_t,
) {
    let response: &mut Response = response.into();
    *response.body_mut() = Some(ResponseBody::Reader(Box::new(readable)));
    let _ = qiniu_ng_http_response_t::from(response);
}

/// @brief 以字符串的形式获取 HTTP 响应的服务器 IP 地址
/// @param[in] response HTTP 响应实例
/// @retval qiniu_ng_str_t 返回服务器 IP 地址
/// @warning 务必记得 `qiniu_ng_str_t` 需要在使用完毕后调用 `qiniu_ng_str_list_free()` 释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_server_ip_as_str(response: qiniu_ng_http_response_t) -> qiniu_ng_str_t {
    let response: &Response = response.into();
    let server_ip = response.server_ip().map(|server_ip| server_ip.to_string());
    unsafe { qiniu_ng_str_t::from_optional_string_unchecked(server_ip) }.tap(|_| {
        let _ = qiniu_ng_http_response_t::from(response);
    })
}

/// @brief 获取 HTTP 响应的服务器 IP 地址
/// @param[in] response HTTP 响应实例
/// @param[out] sa_family 用于返回 HTTP 响应的服务器 IP 地址所在地址族，对于 IPv4 地址，将返回 `AF_INET`，对于 IPv6 地址，将返回 `AF_INET6`。如果传入 `NULL` 表示不获取 `sa_family`，但不会影响 API 返回值和其他数据的返回
/// @param[out] ip_addr 用于返回 HTTP 响应的服务器 IP 地址，对于 IPv4 地址，将返回 `struct in_addr` 类型的数据，对于 IPv6 地址，将返回 `struct in6_addr` 类型的数据。如果传入 `NULL` 表示不获取 `ip_addr`，但不会影响 API 返回值和其他数据的返回
/// @retval bool 如果 HTTP 响应的服务器 IP 地址存在，则返回 `true`，否则返回 `false`
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_server_ip(
    response: qiniu_ng_http_response_t,
    sa_family: *mut u16,
    ip_addr: *mut c_void,
) -> bool {
    let response: &Response = response.into();
    return match response.server_ip() {
        Some(IpAddr::V4(server_ip)) => set_server_ip_as_ipv4(server_ip, sa_family, ip_addr),
        Some(IpAddr::V6(server_ip)) => set_server_ip_as_ipv6(server_ip, sa_family, ip_addr),
        None => false,
    }
    .tap(|_| {
        let _ = qiniu_ng_http_response_t::from(response);
    });

    fn set_server_ip_as_ipv4(server_ip: Ipv4Addr, sa_family: *mut u16, ip_addr: *mut c_void) -> bool {
        if let Some(sa_family) = unsafe { sa_family.as_mut() } {
            *sa_family = AF_INET as u16;
        }
        let ip_addr: *mut in_addr = ip_addr.cast();
        if let Some(ip_addr) = unsafe { ip_addr.as_mut() } {
            *ip_addr = from_ipv4_addr_to_in_addr(server_ip);
        }
        true
    }
    fn set_server_ip_as_ipv6(server_ip: Ipv6Addr, sa_family: *mut u16, ip_addr: *mut c_void) -> bool {
        if let Some(sa_family) = unsafe { sa_family.as_mut() } {
            *sa_family = AF_INET6 as u16;
        }
        let ip_addr: *mut in6_addr = ip_addr.cast();
        if let Some(ip_addr) = unsafe { ip_addr.as_mut() } {
            *ip_addr = from_ipv6_addr_to_in6_addr(server_ip);
        }
        true
    }
}

/// @brief 以字符串的形式设置 HTTP 响应的服务器 IP 地址
/// @param[in] response HTTP 响应实例
/// @param[in] ip_addr 字符串格式的服务器 IP 地址
/// @retval bool 如果 IP 地址解析并设置成功，则返回 `true`。否则返回 `false`
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_server_ip_as_str(
    response: qiniu_ng_http_response_t,
    ip_addr: *const qiniu_ng_char_t,
) -> bool {
    let response: &mut Response = response.into();
    let mut is_ok = true;
    if let Some(ip_addr) = unsafe { ucstr::from_ptr(ip_addr) }
        .to_string()
        .ok()
        .and_then(|s| s.parse().ok())
    {
        *response.server_ip_mut() = Some(ip_addr);
    } else {
        is_ok = false;
    }
    let _ = qiniu_ng_http_response_t::from(response);
    is_ok
}

/// @brief 以 `struct in_addr` 类型设置 HTTP 响应的服务器 IP 地址
/// @param[in] response HTTP 响应实例
/// @param[in] ip_addr 服务器 IP 地址
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_server_ip_v4(response: qiniu_ng_http_response_t, ip_addr: in_addr) {
    let response: &mut Response = response.into();
    *response.server_ip_mut() = Some(IpAddr::V4(from_in_addr_to_ipv4_addr(ip_addr)));
    let _ = qiniu_ng_http_response_t::from(response);
}

/// @brief 以 `struct in6_addr` 类型设置 HTTP 响应的服务器 IP 地址
/// @param[in] response HTTP 响应实例
/// @param[in] ip_addr 服务器 IP 地址
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_server_ip_v6(response: qiniu_ng_http_response_t, ip_addr: in6_addr) {
    let response: &mut Response = response.into();
    *response.server_ip_mut() = Some(IpAddr::V6(from_in6_addr_to_ipv6_addr(ip_addr)));
    let _ = qiniu_ng_http_response_t::from(response);
}

/// @brief 重置 HTTP 请求预解析的服务器 IP 地址
/// @param[in] response HTTP 响应实例
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_unset_server_ip(response: qiniu_ng_http_response_t) {
    let response: &mut Response = response.into();
    *response.server_ip_mut() = None;
    let _ = qiniu_ng_http_response_t::from(response);
}

/// @brief 获取 HTTP 响应的服务器端口号
/// @param[in] response HTTP 响应实例
/// @retval uint16_t 返回 HTTP 响应的服务器端口号
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_server_port(response: qiniu_ng_http_response_t) -> u16 {
    let response: &Response = response.into();
    response.server_port().tap(|_| {
        let _ = qiniu_ng_http_response_t::from(response);
    })
}

/// @brief 设置 HTTP 响应的端口号
/// @param[in] response HTTP 响应实例
/// @param[in] port HTTP 响应的端口号
#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_server_port(response: qiniu_ng_http_response_t, port: u16) {
    let response: &mut Response = response.into();
    *response.server_port_mut() = port;
    let _ = qiniu_ng_http_response_t::from(response);
}

#[cfg(not(windows))]
mod unix {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    #[inline]
    pub(crate) fn from_ipv4_addr_to_in_addr(addr: Ipv4Addr) -> in_addr {
        in_addr {
            s_addr: u32::from_be_bytes(addr.octets()).to_be(),
        }
    }

    #[inline]
    pub(crate) fn from_ipv6_addr_to_in6_addr(addr: Ipv6Addr) -> in6_addr {
        in6_addr { s6_addr: addr.octets() }
    }

    #[inline]
    pub(crate) fn from_in_addr_to_ipv4_addr(addr: in_addr) -> Ipv4Addr {
        addr.s_addr.into()
    }

    #[inline]
    pub(crate) fn from_in6_addr_to_ipv6_addr(addr: in6_addr) -> Ipv6Addr {
        addr.s6_addr.into()
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[inline]
    pub(crate) fn from_ipv4_addr_to_in_addr(addr: Ipv4Addr) -> in_addr {
        let mut ia = in_addr::default();
        unsafe { *ia.S_un.S_addr_mut() = u32::from_be_bytes(addr.octets()).to_be() };
        ia
    }

    #[inline]
    pub(crate) fn from_ipv6_addr_to_in6_addr(addr: Ipv6Addr) -> in6_addr {
        let mut i6a = in6_addr::default();
        unsafe { *i6a.u.Byte_mut() = addr.octets() };
        i6a
    }

    #[inline]
    pub(crate) fn from_in_addr_to_ipv4_addr(addr: in_addr) -> Ipv4Addr {
        unsafe { addr.S_un.S_addr() }.to_owned().into()
    }

    #[inline]
    pub(crate) fn from_in6_addr_to_ipv6_addr(addr: in6_addr) -> Ipv6Addr {
        unsafe { addr.u.Byte() }.to_owned().into()
    }
}

#[cfg(not(windows))]
use unix::*;

#[cfg(windows)]
use windows::*;
