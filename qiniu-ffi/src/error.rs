use super::ucstring::UCString;
use libc::{c_char, c_int, c_void, fprintf, fputs, FILE};
use qiniu_credential::UrlParseError;
use std::{
    error::Error,
    fmt,
    io::{Error as IOError, ErrorKind as IOErrorKind},
};

/// @brief SDK 错误类型
/// @note 请通过调用 `qiniu_ng_err_t` 相关的函数来判定错误具体类型
#[repr(C)]
#[derive(Copy, Debug, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_err_t {
    /// 没有错误
    qiniu_ng_err_none,
    /// 系统调用异常
    qiniu_ng_err_os_error(i32),
    /// IO 调用异常
    qiniu_ng_err_io_error(qiniu_ng_err_io_error_t),
    /// URL 解析异常
    qiniu_ng_err_url_parse_error(qiniu_ng_err_url_parse_error_t),
}

/// @brief SDK 输入输出错误类型
#[repr(C)]
#[derive(Copy, Debug, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_err_io_error_t {
    /// It's OK
    qiniu_ng_err_io_ok = 0,
    /// An entity was not found, often a file.
    qiniu_ng_err_io_not_found_err = 1,
    /// The operation lacked the necessary privileges to complete.
    qiniu_ng_err_io_permission_denied_err = 2,
    /// The connection was refused by the remote server.
    qiniu_ng_err_io_connection_refused_err = 3,
    /// The connection was reset by the remote server.
    qiniu_ng_err_io_connection_reset_err = 4,
    /// The connection was aborted (terminated) by the remote server.
    qiniu_ng_err_io_connection_aborted_err = 5,
    /// The network operation failed because it was not connected yet.
    qiniu_ng_err_io_not_connected_err = 6,
    /// A socket address could not be bound because the address is already in use elsewhere.
    qiniu_ng_err_io_addr_in_use_err = 7,
    /// A nonexistent interface was requested or the requested address was not local.
    qiniu_ng_err_io_addr_not_available_err = 8,
    /// The operation failed because a pipe was closed.
    qiniu_ng_err_io_broken_pipe_err = 9,
    /// An entity already exists, often a file.
    qiniu_ng_err_io_already_exists_err = 10,
    /// The operation needs to block to complete, but the blocking operation was requested to not occur.
    qiniu_ng_err_io_would_block_err = 11,
    /// A parameter was incorrect.
    qiniu_ng_err_io_invalid_input_err = 12,
    /// Data not valid for the operation were encountered.
    qiniu_ng_err_io_invalid_data_err = 13,
    /// The I/O operation's timeout expired, causing it to be canceled.
    qiniu_ng_err_io_timed_out_err = 14,
    /// An operation could only succeed if it wrote a particular number of bytes but only a smaller number of bytes could be written.
    qiniu_ng_err_io_write_zero_err = 15,
    /// This operation was interrupted.
    qiniu_ng_err_io_interrupted_err = 16,
    /// Any I/O error not part of this list.
    qiniu_ng_err_io_other_err = 17,
    /// An error returned when an operation could not be completed because an "end of file" was reached prematurely.
    qiniu_ng_err_io_unexpected_eof_err = 18,
}

/// @brief URL 解析错误类型
#[repr(C)]
#[derive(Copy, Debug, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_err_url_parse_error_t {
    qiniu_ng_err_url_parse_empty_host_err = 1,
    qiniu_ng_err_url_parse_idna_err = 2,
    qiniu_ng_err_url_parse_invalid_port_err = 3,
    qiniu_ng_err_url_parse_invalid_ipv4_addr_err = 4,
    qiniu_ng_err_url_parse_invalid_ipv6_addr_err = 5,
    qiniu_ng_err_url_parse_invalid_domain_char_err = 6,
    qiniu_ng_err_url_parse_relative_url_without_base_err = 7,
    qiniu_ng_err_url_parse_relative_url_with_cannot_be_a_base_err = 8,
    qiniu_ng_err_url_parse_set_host_on_cannot_be_a_base_url_err = 9,
    qiniu_ng_err_url_parse_overflow_err = 10,
    qiniu_ng_err_url_parse_unknown_err = 11,
}

impl Default for qiniu_ng_err_t {
    #[inline]
    fn default() -> Self {
        qiniu_ng_err_t::qiniu_ng_err_none
    }
}

/// @brief 判定错误是否确实存在
/// @param[in] err SDK 错误实例
/// @retval bool 当错误确实存在时返回 `true`
#[no_mangle]
pub extern "C" fn qiniu_ng_err_any_error(err: &qiniu_ng_err_t) -> bool {
    !matches!(err, qiniu_ng_err_t::qiniu_ng_err_none)
}

/// @brief 判定错误是否是系统调用异常
/// @param[in] err SDK 错误实例
/// @param[out] code 用于返回系统调用异常号码，如果传入 `NULL` 表示不获取 `code`，但如果错误确实是系统调用异常，返回值依然是 `true`
/// @retval bool 当错误确实是系统调用异常时返回 `true`
#[no_mangle]
pub extern "C" fn qiniu_ng_err_os_error_extract(err: &qiniu_ng_err_t, code: *mut i32) -> bool {
    match err {
        qiniu_ng_err_t::qiniu_ng_err_os_error(os_error_code) => {
            if let Some(code) = unsafe { code.as_mut() } {
                *code = *os_error_code;
            }
            true
        }
        _ => false,
    }
}

/// @brief 判定错误是否是输入输出调用异常
/// @param[in] err SDK 错误实例
/// @param[out] code 用于返回输出调用异常号码，如果传入 `NULL` 表示不获取 `code`，但如果错误确实是输出调用异常，返回值依然是 `true`
/// @retval bool 当错误确实是输出调用异常时返回 `true`
#[no_mangle]
pub extern "C" fn qiniu_ng_err_io_error_extract(err: &qiniu_ng_err_t, code: *mut i32) -> bool {
    match err {
        qiniu_ng_err_t::qiniu_ng_err_io_error(io_error_code) => {
            if let Some(code) = unsafe { code.as_mut() } {
                *code = (*io_error_code) as i32;
            }
            true
        }
        _ => false,
    }
}

/// @brief 判定错误是否是 URL 解析异常
/// @param[in] err SDK 错误实例
/// @param[out] code 用于返回 URL 解析异常号码，如果传入 `NULL` 表示不获取 `code`，但如果错误确实是 URL 解析异常，返回值依然是 `true`
/// @retval bool 当错误确实是 URL 解析异常时返回 `true`
#[no_mangle]
pub extern "C" fn qiniu_ng_err_url_parse_error_extract(
    err: &qiniu_ng_err_t,
    code: *mut i32,
) -> bool {
    match err {
        qiniu_ng_err_t::qiniu_ng_err_url_parse_error(url_parse_error_code) => {
            if let Some(code) = unsafe { code.as_mut() } {
                *code = (*url_parse_error_code) as i32;
            }
            true
        }
        _ => false,
    }
}

impl fmt::Display for qiniu_ng_err_io_error_t {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(error_kind) = Option::<IOErrorKind>::from(*self) {
            IOError::from(error_kind).fmt(f)
        } else {
            use std::fmt::Debug;
            None::<IOError>.fmt(f)
        }
    }
}

impl Error for qiniu_ng_err_io_error_t {}

impl From<IOError> for qiniu_ng_err_t {
    fn from(err: IOError) -> Self {
        err.raw_os_error()
            .map(qiniu_ng_err_t::qiniu_ng_err_os_error)
            .unwrap_or_else(|| qiniu_ng_err_io_error_t::from(err.kind()).into())
    }
}

impl From<IOErrorKind> for qiniu_ng_err_io_error_t {
    fn from(err: IOErrorKind) -> Self {
        match err {
            IOErrorKind::NotFound => Self::qiniu_ng_err_io_not_found_err,
            IOErrorKind::PermissionDenied => Self::qiniu_ng_err_io_permission_denied_err,
            IOErrorKind::ConnectionRefused => Self::qiniu_ng_err_io_connection_refused_err,
            IOErrorKind::ConnectionReset => Self::qiniu_ng_err_io_connection_reset_err,
            IOErrorKind::ConnectionAborted => Self::qiniu_ng_err_io_connection_aborted_err,
            IOErrorKind::NotConnected => Self::qiniu_ng_err_io_not_connected_err,
            IOErrorKind::AddrInUse => Self::qiniu_ng_err_io_addr_in_use_err,
            IOErrorKind::AddrNotAvailable => Self::qiniu_ng_err_io_addr_not_available_err,
            IOErrorKind::BrokenPipe => Self::qiniu_ng_err_io_broken_pipe_err,
            IOErrorKind::AlreadyExists => Self::qiniu_ng_err_io_already_exists_err,
            IOErrorKind::WouldBlock => Self::qiniu_ng_err_io_would_block_err,
            IOErrorKind::InvalidInput => Self::qiniu_ng_err_io_invalid_input_err,
            IOErrorKind::InvalidData => Self::qiniu_ng_err_io_invalid_data_err,
            IOErrorKind::TimedOut => Self::qiniu_ng_err_io_timed_out_err,
            IOErrorKind::WriteZero => Self::qiniu_ng_err_io_write_zero_err,
            IOErrorKind::Interrupted => Self::qiniu_ng_err_io_interrupted_err,
            IOErrorKind::UnexpectedEof => Self::qiniu_ng_err_io_unexpected_eof_err,
            _ => Self::qiniu_ng_err_io_other_err,
        }
    }
}

impl Default for qiniu_ng_err_io_error_t {
    #[inline]
    fn default() -> Self {
        Self::qiniu_ng_err_io_ok
    }
}

#[allow(non_camel_case_types)]
type io_err_t = qiniu_ng_err_io_error_t;

impl From<io_err_t> for Option<IOErrorKind> {
    fn from(err: io_err_t) -> Self {
        type Kind = IOErrorKind;
        match err {
            io_err_t::qiniu_ng_err_io_ok => None,
            io_err_t::qiniu_ng_err_io_not_found_err => Some(Kind::NotFound),
            io_err_t::qiniu_ng_err_io_permission_denied_err => Some(Kind::PermissionDenied),
            io_err_t::qiniu_ng_err_io_connection_refused_err => Some(Kind::ConnectionRefused),
            io_err_t::qiniu_ng_err_io_connection_reset_err => Some(Kind::ConnectionReset),
            io_err_t::qiniu_ng_err_io_connection_aborted_err => Some(Kind::ConnectionAborted),
            io_err_t::qiniu_ng_err_io_not_connected_err => Some(Kind::NotConnected),
            io_err_t::qiniu_ng_err_io_addr_in_use_err => Some(Kind::AddrInUse),
            io_err_t::qiniu_ng_err_io_addr_not_available_err => Some(Kind::AddrNotAvailable),
            io_err_t::qiniu_ng_err_io_broken_pipe_err => Some(Kind::BrokenPipe),
            io_err_t::qiniu_ng_err_io_already_exists_err => Some(Kind::AlreadyExists),
            io_err_t::qiniu_ng_err_io_would_block_err => Some(Kind::WouldBlock),
            io_err_t::qiniu_ng_err_io_invalid_input_err => Some(Kind::InvalidInput),
            io_err_t::qiniu_ng_err_io_invalid_data_err => Some(Kind::InvalidData),
            io_err_t::qiniu_ng_err_io_timed_out_err => Some(Kind::TimedOut),
            io_err_t::qiniu_ng_err_io_write_zero_err => Some(Kind::WriteZero),
            io_err_t::qiniu_ng_err_io_interrupted_err => Some(Kind::Interrupted),
            io_err_t::qiniu_ng_err_io_unexpected_eof_err => Some(Kind::UnexpectedEof),
            io_err_t::qiniu_ng_err_io_other_err => Some(Kind::Other),
        }
    }
}

impl From<qiniu_ng_err_io_error_t> for qiniu_ng_err_t {
    #[inline]
    fn from(err: qiniu_ng_err_io_error_t) -> Self {
        qiniu_ng_err_t::qiniu_ng_err_io_error(err)
    }
}

impl From<UrlParseError> for qiniu_ng_err_url_parse_error_t {
    fn from(err: UrlParseError) -> Self {
        match err {
            UrlParseError::EmptyHost => Self::qiniu_ng_err_url_parse_empty_host_err,
            UrlParseError::IdnaError => Self::qiniu_ng_err_url_parse_idna_err,
            UrlParseError::InvalidPort => Self::qiniu_ng_err_url_parse_invalid_port_err,
            UrlParseError::InvalidIpv4Address => Self::qiniu_ng_err_url_parse_invalid_ipv4_addr_err,
            UrlParseError::InvalidIpv6Address => Self::qiniu_ng_err_url_parse_invalid_ipv6_addr_err,
            UrlParseError::InvalidDomainCharacter => {
                Self::qiniu_ng_err_url_parse_invalid_domain_char_err
            }
            UrlParseError::RelativeUrlWithoutBase => {
                Self::qiniu_ng_err_url_parse_relative_url_without_base_err
            }
            UrlParseError::RelativeUrlWithCannotBeABaseBase => {
                Self::qiniu_ng_err_url_parse_relative_url_with_cannot_be_a_base_err
            }
            UrlParseError::SetHostOnCannotBeABaseUrl => {
                Self::qiniu_ng_err_url_parse_set_host_on_cannot_be_a_base_url_err
            }
            UrlParseError::Overflow => Self::qiniu_ng_err_url_parse_overflow_err,
            _ => Self::qiniu_ng_err_url_parse_unknown_err,
        }
    }
}

impl From<UrlParseError> for qiniu_ng_err_t {
    #[inline]
    fn from(err: UrlParseError) -> Self {
        qiniu_ng_err_url_parse_error_t::from(err).into()
    }
}

impl From<qiniu_ng_err_url_parse_error_t> for qiniu_ng_err_t {
    #[inline]
    fn from(err: qiniu_ng_err_url_parse_error_t) -> Self {
        qiniu_ng_err_t::qiniu_ng_err_url_parse_error(err)
    }
}

impl fmt::Display for qiniu_ng_err_url_parse_error_t {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::qiniu_ng_err_url_parse_empty_host_err => UrlParseError::EmptyHost.fmt(f),
            Self::qiniu_ng_err_url_parse_idna_err => UrlParseError::IdnaError.fmt(f),
            Self::qiniu_ng_err_url_parse_invalid_port_err => UrlParseError::InvalidPort.fmt(f),
            Self::qiniu_ng_err_url_parse_invalid_ipv4_addr_err => {
                UrlParseError::InvalidIpv4Address.fmt(f)
            }
            Self::qiniu_ng_err_url_parse_invalid_ipv6_addr_err => {
                UrlParseError::InvalidIpv6Address.fmt(f)
            }
            Self::qiniu_ng_err_url_parse_invalid_domain_char_err => {
                UrlParseError::InvalidDomainCharacter.fmt(f)
            }
            Self::qiniu_ng_err_url_parse_relative_url_without_base_err => {
                UrlParseError::RelativeUrlWithoutBase.fmt(f)
            }
            Self::qiniu_ng_err_url_parse_relative_url_with_cannot_be_a_base_err => {
                UrlParseError::RelativeUrlWithCannotBeABaseBase.fmt(f)
            }
            Self::qiniu_ng_err_url_parse_set_host_on_cannot_be_a_base_url_err => {
                UrlParseError::SetHostOnCannotBeABaseUrl.fmt(f)
            }
            Self::qiniu_ng_err_url_parse_overflow_err => UrlParseError::Overflow.fmt(f),
            _ => write!(f, "Unknown URL Parse Error"),
        }
    }
}

impl Error for qiniu_ng_err_url_parse_error_t {}

impl fmt::Display for qiniu_ng_err_t {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::qiniu_ng_err_none => write!(f, "Ok"),
            Self::qiniu_ng_err_os_error(errno) => IOError::from_raw_os_error(*errno).fmt(f),
            Self::qiniu_ng_err_io_error(err) => err.fmt(f),
            Self::qiniu_ng_err_url_parse_error(err) => err.fmt(f),
        }
    }
}

/// @brief 当错误存在时，调用 `fputs()` 输出错误信息
/// @param[in] stream 输出流
/// @param[in] err SDK 错误实例
#[no_mangle]
pub extern "C" fn qiniu_ng_err_fputs(err: qiniu_ng_err_t, stream: *mut FILE) -> c_int {
    let error_description = unsafe { UCString::from_string_unchecked(err.to_string()) };
    unsafe { fputs(error_description.as_ptr().cast(), stream) }
}

/// @brief 当错误存在时，调用 `fprintf()` 输出错误信息
/// @param[in] stream 输出流
/// @param[in] format 输出格式，采用 `fprintf` 语法，本函数向该格式输出一个字符串类型的参数作为错误信息，因此，如果该参数设置为 `"%s"` 将会直接输出错误信息，而 `"%s\n"` 将会输出错误信息并换行
/// @param[in] err SDK 错误实例
#[cfg(not(windows))]
#[no_mangle]
pub extern "C" fn qiniu_ng_err_fprintf(
    stream: *mut FILE,
    format: *const c_char,
    err: qiniu_ng_err_t,
) -> c_int {
    let error_description = unsafe { UCString::from_string_unchecked(err.to_string()) };
    unsafe { fprintf(stream, format, error_description.as_ptr() as *mut c_void) }
}
