use libc::{c_int, c_void, ferror, fread, size_t, FILE};
use std::io::{Error, ErrorKind, Read, Result};

/// @brief 数据读取器结构体，用于实现数据读取
/// @details
///   该结构是个简单的开放结构体，其中的 `read_func` 字段需要您用来定义回调函数实现对数据的读取。
///
///   `context` 字段则是您用来传入到回调函数的上下文参数指针，您可以用来作为回调函数的上下文使用。
#[repr(C)]
#[derive(Clone)]
pub struct qiniu_ng_readable_t {
    /// @brief 该字段需要您用来定义回调函数实现对数据的读取
    /// @details
    ///   该函数的第一个参数 `context` 是个上下文参数指针，可以自由定义其数据作为上下文使用。
    ///   第二个参数 `buf` 是 SDK 提供给回调函数读取数据的缓冲区地址，第三个参数 `count` 则是缓冲区的长度，单位为字节。
    ///   您需要读取数据并将数据写入 `buf` 缓冲区，且写入数据的尺寸不能超过 `count`。
    ///   写入完毕后，需要您将实际写入的数据长度填充在第四个参数 `have_read` 内。如果 `have_read` 中填充 `0`，则数据读取结束。
    ///   如果发生无法处理的读取错误，则返回相应的操作系统错误号码。如果没有发生任何错误，则返回 `0`。
    pub read_func: extern "C" fn(
        context: *mut c_void,
        buf: *mut c_void,
        count: size_t,
        have_read: *mut size_t,
    ) -> c_int,
    /// @brief 上下文参数指针
    pub context: *mut c_void,
}

impl Read for qiniu_ng_readable_t {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut have_read: size_t = 0;
        match (self.read_func)(
            self.context,
            buf.as_mut_ptr().cast(),
            buf.len(),
            &mut have_read,
        ) {
            0 => Ok(have_read),
            code => Err(Error::from_raw_os_error(code)),
        }
    }
}
unsafe impl Send for qiniu_ng_readable_t {}

pub(super) struct FileReader(*mut FILE);

impl From<*mut FILE> for FileReader {
    fn from(file: *mut FILE) -> Self {
        Self::new(file)
    }
}

impl FileReader {
    pub fn new(file: *mut FILE) -> Self {
        Self(file)
    }
}

impl Read for FileReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let have_read = unsafe { fread(buf.as_mut_ptr().cast(), 1, buf.len(), self.0) };
        if have_read < buf.len() && unsafe { ferror(self.0) } != 0 {
            return Err(Error::new(ErrorKind::Other, "ferror() returns non-zero"));
        }
        Ok(have_read)
    }
}
unsafe impl Send for FileReader {}
