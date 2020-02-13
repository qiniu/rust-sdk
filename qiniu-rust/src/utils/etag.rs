//! 七牛 Etag 计算库

use super::base64;
use digest::{
    generic_array::{typenum::U28, GenericArray},
    FixedOutput, Input, Reset,
};
use sha1::Sha1;
use std::{
    fs::File,
    io::{copy, sink, Read, Result},
    mem::replace,
    option::Option,
    path::Path,
};
use tap::TapResultOps;

const BLOCK_SIZE: usize = 1 << 22;

/// Etag 字符串固定长度
pub const ETAG_SIZE: usize = 28;

/// 七牛 Etag 计算器
pub struct Etag {
    buffer: Vec<u8>,
    sha1s: Vec<Vec<u8>>,
}

/// 创建一个 Etag 计算器
pub fn new() -> Etag {
    Etag {
        buffer: Vec::new(),
        sha1s: Vec::new(),
    }
}

impl Default for Etag {
    fn default() -> Self {
        new()
    }
}

impl Input for Etag {
    /// 向 Etag 计算器输入数据
    fn input<B: AsRef<[u8]>>(&mut self, data: B) {
        self.buffer.extend_from_slice(data.as_ref());
        let mut iter = self.buffer.chunks_exact(BLOCK_SIZE);
        for block in iter.by_ref() {
            self.sha1s.push(Self::sha1(block));
        }
        self.buffer = {
            let mut new_buffer = Vec::new();
            new_buffer.extend_from_slice(iter.remainder());
            new_buffer
        };
    }
}

impl FixedOutput for Etag {
    type OutputSize = U28;

    /// 从 Etag 计算器获取结果
    fn fixed_result(mut self) -> GenericArray<u8, Self::OutputSize> {
        if !self.buffer.is_empty() {
            self.sha1s.push(Self::sha1(&self.buffer));
            self.buffer.clear();
        }
        self.encode_sha1s()
    }
}

impl Reset for Etag {
    /// 重置 Etag 计算器
    fn reset(&mut self) {
        self.buffer.clear();
        self.sha1s.clear();
    }
}

impl Etag {
    fn sha1(bytes: &[u8]) -> Vec<u8> {
        let mut sha1 = Sha1::default();
        sha1.input(bytes);
        sha1.fixed_result().to_vec()
    }

    fn encode_sha1s(self) -> GenericArray<u8, U28> {
        let mut fixed_result = [0u8; ETAG_SIZE];
        match self.sha1s.len() {
            0 => {
                fixed_result.copy_from_slice(b"Fto5o-5ea0sNMlW_75VgGJCv2AcJ");
            }
            1 => {
                let mut buf = Vec::with_capacity(32);
                buf.push(0x16u8);
                buf.extend_from_slice(self.sha1s.first().unwrap());
                base64::urlsafe_slice(&buf, &mut fixed_result);
            }
            _ => {
                let mut buf = Vec::with_capacity(1024);
                for sha1 in self.sha1s.iter() {
                    buf.extend_from_slice(&sha1);
                }
                let sha1 = Self::sha1(&buf);
                buf.clear();
                buf.push(0x96u8);
                buf.extend_from_slice(&sha1);
                base64::urlsafe_slice(&buf, &mut fixed_result);
            }
        }
        fixed_result.into()
    }
}

/// 一个 Etag 读取器
///
/// Etag 读取器实现 `std::io::Read` 接口，能够边读取数据边计算 Etag。
/// 可以用于在数据流无法倒回的情况下，边读取数据流边计算 Etag
pub struct Reader<IO>
where
    IO: Read,
{
    io: IO,
    etag: Option<String>,
    have_read: usize,
    digest: Etag,
}

/// 创建一个 Etag 读取器
///
/// 封装输入流
pub fn new_reader<IO: Read>(io: IO) -> Reader<IO> {
    Reader {
        io,
        etag: None,
        have_read: 0,
        digest: new(),
    }
}

impl<IO> Read for Reader<IO>
where
    IO: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.io.read(buf).tap_ok(|&mut have_read| {
            if !buf.is_empty() {
                if have_read > 0 {
                    self.have_read += have_read;
                    self.digest.input(buf.get(..have_read).unwrap())
                } else {
                    let digest = replace(&mut self.digest, new());
                    self.etag = Some(String::from_utf8(digest.fixed_result().to_vec()).unwrap());
                }
            }
        })
    }
}

impl<IO> Reader<IO>
where
    IO: Read,
{
    /// 获取 Etag
    pub fn etag(&self) -> Option<&str> {
        self.etag.as_ref().map(|s| s.as_str())
    }

    /// 获取 Etag，并销毁自身
    pub fn into_etag(self) -> Option<String> {
        self.etag
    }
}

/// 读取输入流并计算 Etag
///
/// 该方法将从输入流中读出全部数据，直到读到 EOF 为止
pub fn from<IO: Read>(io: IO) -> Result<String> {
    let mut reader = new_reader(io);
    copy(&mut reader, &mut sink())?;
    Ok(reader.into_etag().unwrap())
}

/// 根据给出的数据计算 Etag
pub fn from_bytes<S: AsRef<[u8]>>(buf: S) -> String {
    let mut etag_digest = new();
    etag_digest.input(buf.as_ref());
    String::from_utf8(etag_digest.fixed_result().to_vec()).unwrap()
}

/// 根据给出的文件内容计算 Etag
pub fn from_file<P: AsRef<Path>>(path: P) -> Result<String> {
    from(File::open(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qiniu_test_utils::temp_file;
    use std::{
        boxed::Box,
        error::Error,
        io::{empty, Cursor},
        result::Result,
    };

    #[test]
    fn test_etag_from_data() -> Result<(), Box<dyn Error>> {
        assert_eq!(from(&mut empty())?, "Fto5o-5ea0sNMlW_75VgGJCv2AcJ",);
        assert_eq!(from(&mut Cursor::new("etag"))?, "FpLiADEaVoALPkdb8tJEJyRTXoe_");
        Ok(())
    }

    #[test]
    fn test_etag_from_file() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            from_file(temp_file::create_temp_file(1 << 20)?)?,
            "Foyl8onxBLWeRLL5oItRJphv6i4b",
        );
        assert_eq!(
            from_file(temp_file::create_temp_file(4 * (1 << 20))?)?,
            "FicHOveBNs5Kn9d74M3b9tI4D-8r",
        );
        assert_eq!(
            from_file(temp_file::create_temp_file(5 * (1 << 20))?)?,
            "lg-Eb5KFCuZn-cUfj_oS2PPOU9xy",
        );
        assert_eq!(
            from_file(temp_file::create_temp_file(8 * (1 << 20))?)?,
            "lkSKZOMToDp-EqLDVuT1pyjQssl-",
        );
        assert_eq!(
            from_file(temp_file::create_temp_file(9 * (1 << 20))?)?,
            "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
        );
        Ok(())
    }
}
