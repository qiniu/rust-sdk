use crate::{
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr},
    utils::{qiniu_ng_optional_str_t, qiniu_ng_readable_t, qiniu_ng_str_map_t, qiniu_ng_str_t},
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
    ffi::{CStr, CString},
    fs::File,
    io::{copy as io_copy, sink as io_sink, Read},
    mem::transmute,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    ptr::{copy_nonoverlapping, null_mut},
    slice::{from_raw_parts, from_raw_parts_mut},
};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_http_method_t {
    qiniu_ng_http_method_get,
    qiniu_ng_http_method_head,
    qiniu_ng_http_method_post,
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

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_http_request_t(*mut c_void);

impl From<qiniu_ng_http_request_t> for Box<&Request<'_>> {
    fn from(request: qiniu_ng_http_request_t) -> Self {
        unsafe { Box::from_raw(transmute(request)) }
    }
}

impl From<Box<&Request<'_>>> for qiniu_ng_http_request_t {
    fn from(request: Box<&Request<'_>>) -> Self {
        unsafe { transmute(Box::into_raw(request)) }
    }
}

impl From<qiniu_ng_http_request_t> for Box<&mut Request<'_>> {
    fn from(request: qiniu_ng_http_request_t) -> Self {
        unsafe { Box::from_raw(transmute(request)) }
    }
}

impl From<Box<&mut Request<'_>>> for qiniu_ng_http_request_t {
    fn from(request: Box<&mut Request<'_>>) -> Self {
        unsafe { transmute(Box::into_raw(request)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_url(request: qiniu_ng_http_request_t) -> qiniu_ng_str_t {
    let request = Box::<&Request>::from(request);
    unsafe { qiniu_ng_str_t::from_str_unchecked(request.url()) }.tap(|_| {
        let _ = qiniu_ng_http_request_t::from(request);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_set_url(request: qiniu_ng_http_request_t, url: *const c_char) {
    let request = Box::<&mut Request>::from(request);
    *request.url_mut() = unsafe { CStr::from_ptr(url) }.to_str().unwrap().to_owned().into();
    let _ = qiniu_ng_http_request_t::from(request);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_method(request: qiniu_ng_http_request_t) -> qiniu_ng_http_method_t {
    let request = Box::<&Request>::from(request);
    request
        .method()
        .tap(|_| {
            let _ = qiniu_ng_http_request_t::from(request);
        })
        .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_set_method(request: qiniu_ng_http_request_t, method: qiniu_ng_http_method_t) {
    let request = Box::<&mut Request>::from(request);
    *request.method_mut() = method.into();
    let _ = qiniu_ng_http_request_t::from(request);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_header(
    request: qiniu_ng_http_request_t,
    header_name: *const c_char,
) -> qiniu_ng_optional_str_t {
    let request = Box::<&Request>::from(request);
    unsafe {
        qiniu_ng_optional_str_t::from_str_unchecked(
            request
                .headers()
                .get(&HeaderName::new(CStr::from_ptr(header_name).to_str().unwrap()))
                .as_ref()
                .map(|header_value| header_value.as_ref()),
        )
    }
    .tap(|_| {
        let _ = qiniu_ng_http_request_t::from(request);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_headers(request: qiniu_ng_http_request_t) -> qiniu_ng_str_map_t {
    let request = Box::<&Request>::from(request);
    let src_headers = request.headers();
    let mut dest_headers = Box::new(HashMap::<Box<CStr>, Box<CStr>, RandomState>::with_capacity(
        src_headers.len(),
    ));
    src_headers.iter().for_each(|(header_name, header_value)| {
        dest_headers.insert(
            unsafe { CString::from_vec_unchecked(header_name.as_ref().as_bytes().to_owned()) }.into_boxed_c_str(),
            unsafe { CString::from_vec_unchecked(header_value.as_ref().as_bytes().to_owned()) }.into_boxed_c_str(),
        );
    });
    let _ = qiniu_ng_http_request_t::from(request);
    dest_headers.into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_set_header(
    request: qiniu_ng_http_request_t,
    header_name: *const c_char,
    header_value: *const c_char,
) {
    let request = Box::<&mut Request>::from(request);
    if let Some(header_value) = unsafe { header_value.as_ref() } {
        request.headers_mut().insert(
            HeaderName::new(unsafe { CStr::from_ptr(header_name) }.to_str().unwrap()),
            unsafe { CStr::from_ptr(header_value) }.to_str().unwrap().into(),
        );
    } else {
        request.headers_mut().remove(&HeaderName::new(
            unsafe { CStr::from_ptr(header_name) }.to_str().unwrap(),
        ));
    }
    let _ = qiniu_ng_http_request_t::from(request);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_body(
    request: qiniu_ng_http_request_t,
    body_ptr: *mut *const c_void,
    body_size: *mut size_t,
) {
    let request = Box::<&Request>::from(request);
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

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_set_body(
    request: qiniu_ng_http_request_t,
    body_ptr: *const c_void,
    body_size: size_t,
) {
    let request = Box::<&mut Request>::from(request);
    *request.body_mut() = if body_size == 0 {
        None
    } else {
        let mut buf = Vec::with_capacity(body_size);
        buf.copy_from_slice(unsafe { from_raw_parts(body_ptr.cast(), body_size) });
        Some(buf.into())
    };
    let _ = qiniu_ng_http_request_t::from(request);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_will_follow_redirection(request: qiniu_ng_http_request_t) -> bool {
    let request = Box::<&Request>::from(request);
    request.follow_redirection().tap(|_| {
        let _ = qiniu_ng_http_request_t::from(request);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_set_follow_redirection(
    request: qiniu_ng_http_request_t,
    follow_redirection: bool,
) {
    let request = Box::<&mut Request>::from(request);
    *request.follow_redirection_mut() = follow_redirection;
    let _ = qiniu_ng_http_request_t::from(request);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_resolved_socket_addrs_len(request: qiniu_ng_http_request_t) -> size_t {
    let request = Box::<&Request>::from(request);
    request.resolved_socket_addrs().len().tap(|_| {
        let _ = qiniu_ng_http_request_t::from(request);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_resolved_socket_addr(
    request: qiniu_ng_http_request_t,
    index: size_t,
    sa_family: *mut u16,
    ip_addr: *mut c_void,
    port: *mut u16,
) -> bool {
    let request = Box::<&Request>::from(request);
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

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_clear_resolved_socket_addrs(request: qiniu_ng_http_request_t) {
    let request = Box::<&mut Request>::from(request);
    *request.resolved_socket_addrs_mut() = Cow::Borrowed(&[]);
    let _ = qiniu_ng_http_request_t::from(request);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_append_resolved_ipv4_socket_addr(
    request: qiniu_ng_http_request_t,
    ip_addr: in_addr,
    port: u16,
) {
    let request = Box::<&mut Request>::from(request);
    match request.resolved_socket_addrs_mut() {
        Cow::Borrowed(resolved_socket_addrs_ref) => {
            let mut resolved_socket_addrs: Vec<SocketAddr> = (*resolved_socket_addrs_ref).to_owned();
            resolved_socket_addrs.push(SocketAddr::V4(SocketAddrV4::new(
                from_in_addr_to_ipv4_addr(ip_addr),
                port,
            )));
            *request.resolved_socket_addrs_mut() = Cow::Owned(resolved_socket_addrs);
        }
        Cow::Owned(resolved_socket_addrs) => {
            resolved_socket_addrs.push(SocketAddr::V4(SocketAddrV4::new(
                from_in_addr_to_ipv4_addr(ip_addr),
                port,
            )));
        }
    }
    let _ = qiniu_ng_http_request_t::from(request);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_append_resolved_ipv6_socket_addr(
    request: qiniu_ng_http_request_t,
    ip_addr: in6_addr,
    port: u16,
    flowinfo: u32,
    scope_id: u32,
) {
    let request = Box::<&mut Request>::from(request);
    match request.resolved_socket_addrs_mut() {
        Cow::Borrowed(resolved_socket_addrs_ref) => {
            let mut resolved_socket_addrs: Vec<SocketAddr> = (*resolved_socket_addrs_ref).to_owned();
            resolved_socket_addrs.push(SocketAddr::V6(SocketAddrV6::new(
                from_in6_addr_to_ipv6_addr(ip_addr),
                port,
                flowinfo,
                scope_id,
            )));
            *request.resolved_socket_addrs_mut() = Cow::Owned(resolved_socket_addrs);
        }
        Cow::Owned(resolved_socket_addrs) => {
            resolved_socket_addrs.push(SocketAddr::V6(SocketAddrV6::new(
                from_in6_addr_to_ipv6_addr(ip_addr),
                port,
                flowinfo,
                scope_id,
            )));
        }
    }
    let _ = qiniu_ng_http_request_t::from(request);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_get_custom_data(request: qiniu_ng_http_request_t) -> *mut c_void {
    let request = Box::<&Request>::from(request);
    request.custom_data().tap(|_| {
        let _ = qiniu_ng_http_request_t::from(request);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_request_set_custom_data(request: qiniu_ng_http_request_t, data: *mut c_void) {
    let request = Box::<&mut Request>::from(request);
    *request.custom_data_mut() = data;
    let _ = qiniu_ng_http_request_t::from(request);
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_http_response_t(*mut c_void);

impl From<qiniu_ng_http_response_t> for Box<&Response> {
    fn from(response: qiniu_ng_http_response_t) -> Self {
        unsafe { Box::from_raw(transmute(response)) }
    }
}

impl From<Box<&Response>> for qiniu_ng_http_response_t {
    fn from(response: Box<&Response>) -> Self {
        unsafe { transmute(Box::into_raw(response)) }
    }
}

impl From<qiniu_ng_http_response_t> for Box<&mut Response> {
    fn from(response: qiniu_ng_http_response_t) -> Self {
        unsafe { Box::from_raw(transmute(response)) }
    }
}

impl From<Box<&mut Response>> for qiniu_ng_http_response_t {
    fn from(response: Box<&mut Response>) -> Self {
        unsafe { transmute(Box::into_raw(response)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_status_code(response: qiniu_ng_http_response_t) -> u16 {
    let response = Box::<&Response>::from(response);
    response.status_code().tap(|_| {
        let _ = qiniu_ng_http_response_t::from(response);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_status_code(response: qiniu_ng_http_response_t, status_code: u16) {
    let response = Box::<&mut Response>::from(response);
    *response.status_code_mut() = status_code;
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_header(
    response: qiniu_ng_http_response_t,
    header_name: *const c_char,
) -> qiniu_ng_optional_str_t {
    let response = Box::<&Response>::from(response);
    unsafe {
        qiniu_ng_optional_str_t::from_str_unchecked(
            response
                .headers()
                .get(&HeaderName::new(CStr::from_ptr(header_name).to_str().unwrap()))
                .as_ref()
                .map(|header_value| header_value.as_ref()),
        )
    }
    .tap(|_| {
        let _ = qiniu_ng_http_response_t::from(response);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_headers(response: qiniu_ng_http_response_t) -> qiniu_ng_str_map_t {
    let response = Box::<&Response>::from(response);
    let src_headers = response.headers();
    let mut dest_headers = Box::new(HashMap::<Box<CStr>, Box<CStr>, RandomState>::with_capacity(
        src_headers.len(),
    ));
    src_headers.iter().for_each(|(header_name, header_value)| {
        dest_headers.insert(
            unsafe { CString::from_vec_unchecked(header_name.as_ref().as_bytes().to_owned()) }.into_boxed_c_str(),
            unsafe { CString::from_vec_unchecked(header_value.as_ref().as_bytes().to_owned()) }.into_boxed_c_str(),
        );
    });
    let _ = qiniu_ng_http_response_t::from(response);
    dest_headers.into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_header(
    response: qiniu_ng_http_response_t,
    header_name: *const c_char,
    header_value: *const c_char,
) {
    let response = Box::<&mut Response>::from(response);
    if let Some(header_value) = unsafe { header_value.as_ref() } {
        response.headers_mut().insert(
            HeaderName::new(unsafe { CStr::from_ptr(header_name) }.to_str().unwrap()),
            unsafe { CStr::from_ptr(header_value) }.to_str().unwrap().into(),
        );
    } else {
        response.headers_mut().remove(&HeaderName::new(
            unsafe { CStr::from_ptr(header_name) }.to_str().unwrap(),
        ));
    }
    let _ = qiniu_ng_http_response_t::from(response);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_body_length(
    response: qiniu_ng_http_response_t,
    length: *mut u64,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let response = Box::<&mut Response>::from(response);
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

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_dump_body(
    response: qiniu_ng_http_response_t,
    max_body_size: u64,
    body_ptr: *mut c_void,
    body_size: *mut u64,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let response = Box::<&mut Response>::from(response);
    return match response.clone_body() {
        Ok(Some(ResponseBody::Bytes(bytes))) => {
            qiniu_ng_http_response_dump_body_as_bytes(bytes, max_body_size, body_ptr, body_size)
        }
        Ok(Some(ResponseBody::Reader(reader))) => {
            qiniu_ng_http_response_dump_body_as_reader(reader, max_body_size, body_ptr, body_size, err)
        }
        Ok(Some(ResponseBody::File(file))) => {
            qiniu_ng_http_response_dump_body_as_file(file, max_body_size, body_ptr, body_size, err)
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

    fn qiniu_ng_http_response_dump_body_as_bytes(
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

    fn qiniu_ng_http_response_dump_body_as_reader(
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

    fn qiniu_ng_http_response_dump_body_as_file(
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

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_body(
    response: qiniu_ng_http_response_t,
    body_ptr: *const c_void,
    body_size: size_t,
) {
    let response = Box::<&mut Response>::from(response);
    *response.body_mut() = if body_size == 0 {
        None
    } else {
        let mut buf = Vec::with_capacity(body_size);
        buf.copy_from_slice(unsafe { from_raw_parts(body_ptr.cast(), body_size) });
        Some(ResponseBody::Bytes(buf))
    };
    let _ = qiniu_ng_http_response_t::from(response);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_body_to_file(
    response: qiniu_ng_http_response_t,
    path: *const qiniu_ng_char_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let response = Box::<&mut Response>::from(response);
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

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_body_to_reader(
    response: qiniu_ng_http_response_t,
    readable: qiniu_ng_readable_t,
) {
    let response = Box::<&mut Response>::from(response);
    *response.body_mut() = Some(ResponseBody::Reader(Box::new(readable)));
    let _ = qiniu_ng_http_response_t::from(response);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_server_ip(
    response: qiniu_ng_http_response_t,
    sa_family: *mut u16,
    ip_addr: *mut c_void,
) -> bool {
    let response = Box::<&Response>::from(response);
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

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_server_ip_v4(response: qiniu_ng_http_response_t, ip_addr: in_addr) {
    let response = Box::<&mut Response>::from(response);
    *response.server_ip_mut() = Some(IpAddr::V4(from_in_addr_to_ipv4_addr(ip_addr)));
    let _ = qiniu_ng_http_response_t::from(response);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_server_ip_v6(response: qiniu_ng_http_response_t, ip_addr: in6_addr) {
    let response = Box::<&mut Response>::from(response);
    *response.server_ip_mut() = Some(IpAddr::V6(from_in6_addr_to_ipv6_addr(ip_addr)));
    let _ = qiniu_ng_http_response_t::from(response);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_unset_server_ip(response: qiniu_ng_http_response_t) {
    let response = Box::<&mut Response>::from(response);
    *response.server_ip_mut() = None;
    let _ = qiniu_ng_http_response_t::from(response);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_get_server_port(response: qiniu_ng_http_response_t) -> u16 {
    let response = Box::<&Response>::from(response);
    response.server_port().tap(|_| {
        let _ = qiniu_ng_http_response_t::from(response);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_http_response_set_server_port(response: qiniu_ng_http_response_t, port: u16) {
    let response = Box::<&mut Response>::from(response);
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
