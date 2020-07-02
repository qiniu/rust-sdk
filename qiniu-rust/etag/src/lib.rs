use digest::{FixedOutput, Reset, Update};
use generic_array::{typenum::U28, GenericArray};
use qiniu_utils::base64;
use std::{
    fs::File,
    io::{copy, Read, Result, Write},
};

const DEFAULT_BLOCK_SIZE: usize = 1 << 22;

/// Etag 字符串固定长度
pub const ETAG_SIZE: usize = 28;

/// Etag V1 计算器，使用 Etag V1 算法计算七牛云存储上文件的 HASH 值
#[derive(Default)]
pub struct EtagV1 {
    buffer: Vec<u8>,
    sha1s: Vec<Vec<u8>>,
}

/// Etag V2 计算器，使用 Etag V2 算法计算七牛云存储上文件的 HASH 值
#[derive(Default)]
pub struct EtagV2 {
    buffer: Vec<u8>,
    etag_v1: EtagV1,
}

impl EtagV1 {
    /// 构建 Etag V1 计算器
    pub fn new() -> Self {
        Default::default()
    }
}

impl Update for EtagV1 {
    /// 向 Etag V1 计算器输入数据，数据尺寸任意
    fn update(&mut self, data: impl AsRef<[u8]>) {
        self.buffer.extend_from_slice(data.as_ref());
        let mut iter = self.buffer.chunks_exact(DEFAULT_BLOCK_SIZE);
        for block in iter.by_ref() {
            self.sha1s.push(sha1::sha1(block));
        }
        self.buffer = {
            let mut new_buffer = Vec::new();
            new_buffer.extend_from_slice(iter.remainder());
            new_buffer
        };
    }
}

impl Write for EtagV1 {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl FixedOutput for EtagV1 {
    type OutputSize = U28;

    /// 计算 Etag V1，生成结果
    fn finalize_into(mut self, out: &mut GenericArray<u8, Self::OutputSize>) {
        self.finalize_into_without_reset(out);
    }

    /// 计算 Etag V1，生成结果，然后重置该实例
    fn finalize_into_reset(&mut self, out: &mut GenericArray<u8, Self::OutputSize>) {
        self.finalize_into_without_reset(out);
        self.reset();
    }
}

impl Reset for EtagV1 {
    /// 重置 Etag V1 计算器
    fn reset(&mut self) {
        self.buffer.clear();
        self.sha1s.clear();
    }
}

impl EtagV1 {
    fn finalize_into_without_reset(&mut self, out: &mut GenericArray<u8, U28>) {
        self.finish();
        self.calculate(out);
    }

    fn finish(&mut self) {
        if !self.buffer.is_empty() {
            self.sha1s.push(sha1::sha1(&self.buffer));
            self.buffer.clear();
        }
    }

    fn calculate(&mut self, out: &mut GenericArray<u8, U28>) {
        base64::urlsafe_slice(&sha1_encoder::hash_sha1s(&self.sha1s), out);
    }
}

impl EtagV2 {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Update for EtagV2 {
    /// 向 Etag V2 计算器输入数据，数据尺寸任意，但是一次输入的数据将被视为一个数据块
    fn update(&mut self, data: impl AsRef<[u8]>) {
        self.etag_v1.update(data);
        self.etag_v1.finish();
        self.buffer
            .extend_from_slice(&sha1_encoder::hash_sha1s(&self.etag_v1.sha1s)[1..]);
        self.etag_v1.reset()
    }
}

impl Reset for EtagV2 {
    /// 重置 Etag V2 计算器
    fn reset(&mut self) {
        self.buffer.clear();
    }
}

impl FixedOutput for EtagV2 {
    type OutputSize = U28;

    /// 计算 Etag V2，生成结果
    fn finalize_into(mut self, out: &mut GenericArray<u8, Self::OutputSize>) {
        self.finalize_into_without_reset(out);
    }

    /// 计算 Etag V2，生成结果，然后重置该实例
    fn finalize_into_reset(&mut self, out: &mut GenericArray<u8, Self::OutputSize>) {
        self.finalize_into_without_reset(out);
        self.reset();
    }
}

impl EtagV2 {
    fn finalize_into_without_reset(&mut self, out: &mut GenericArray<u8, U28>) {
        let mut buffer = Vec::with_capacity(21);
        buffer.push(0x9eu8);
        buffer.extend_from_slice(&sha1::sha1(&self.buffer));
        base64::urlsafe_slice(&buffer, out);
    }
}

fn _etag_of_reader(mut reader: impl Read, out: &mut GenericArray<u8, U28>) -> Result<()> {
    let mut etag_v1 = EtagV1::new();
    copy(&mut reader, &mut etag_v1)?;
    etag_v1.finalize_into(out);
    Ok(())
}

/// 读取 reader 中的数据并计算它的 Etag，生成结果
pub fn etag_of_reader(reader: impl Read) -> Result<String> {
    let mut buf = GenericArray::default();
    _etag_of_reader(reader, &mut buf)?;
    Ok(String::from_utf8(buf.to_vec()).unwrap())
}

/// 读取文件并计算它的 Etag，生成结果
pub fn etag_of_file(file: &mut File) -> Result<String> {
    etag_of_reader(file)
}

/// 读取 reader 中的数据并计算它的 Etag，生成结果到指定的数组中
pub fn etag_of_reader_to_array(reader: impl Read, array: &mut [u8; ETAG_SIZE]) -> Result<()> {
    _etag_of_reader(reader, GenericArray::from_mut_slice(array))?;
    Ok(())
}

/// 读取文件并计算它的 Etag，生成结果到指定的数组中
pub fn etag_of_file_to_array(file: &mut File, array: &mut [u8; ETAG_SIZE]) -> Result<()> {
    etag_of_reader_to_array(file, array)
}

fn _etag_of_reader_with_parts(
    mut reader: impl Read,
    parts: &[usize],
    out: &mut GenericArray<u8, U28>,
) -> Result<()> {
    if can_use_etag_v1(parts) {
        return _etag_of_reader(reader, out);
    }

    let mut etag_v2 = EtagV2::new();
    let mut buf = Vec::new();
    for &part in parts.iter() {
        buf.resize(part, 0u8);
        reader.read_exact(&mut buf)?;
        etag_v2.update(&buf);
    }
    etag_v2.finalize_into(out);
    Ok(())
}

/// 根据给出的数据块尺寸，读取 reader 中的数据并计算它的 Etag，生成结果
pub fn etag_of_reader_with_parts(reader: impl Read, parts: &[usize]) -> Result<String> {
    let mut buf = GenericArray::default();
    _etag_of_reader_with_parts(reader, parts, &mut buf)?;
    Ok(String::from_utf8(buf.to_vec()).unwrap())
}

/// 根据给出的数据块尺寸，读取文件并计算它的 Etag，生成结果
pub fn etag_of_file_with_parts(file: &mut File, parts: &[usize]) -> Result<String> {
    etag_of_reader_with_parts(file, parts)
}

/// 根据给出的数据块尺寸，读取 reader 中的数据并计算它的 Etag，生成结果到指定的数组中
pub fn etag_of_reader_with_parts_to_array(
    reader: impl Read,
    parts: &[usize],
    array: &mut [u8; ETAG_SIZE],
) -> Result<()> {
    _etag_of_reader_with_parts(reader, parts, GenericArray::from_mut_slice(array))?;
    Ok(())
}

/// 根据给出的数据块尺寸，读取文件并计算它的 Etag，生成结果到指定的数组中
pub fn etag_of_file_with_parts_to_array(
    file: &mut File,
    parts: &[usize],
    array: &mut [u8; ETAG_SIZE],
) -> Result<()> {
    etag_of_reader_with_parts_to_array(file, parts, array)
}

mod sha1_encoder {
    use super::sha1;

    pub(super) fn hash_sha1s(sha1s: &[Vec<u8>]) -> Vec<u8> {
        match sha1s.len() {
            0 => vec![
                0x16, 0xda, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0xd, 0x32, 0x55, 0xbf, 0xef, 0x95,
                0x60, 0x18, 0x90, 0xaf, 0xd8, 0x7, 0x9,
            ],
            1 => {
                let mut buf = Vec::with_capacity(32);
                buf.push(0x16u8);
                buf.extend_from_slice(sha1s.first().unwrap());
                buf
            }
            _ => {
                let mut buf = Vec::with_capacity(sha1s.iter().map(|s| s.len()).sum());
                for sha1 in sha1s.iter() {
                    buf.extend_from_slice(&sha1);
                }
                let sha1 = sha1::sha1(&buf);
                buf.clear();
                buf.push(0x96u8);
                buf.extend_from_slice(&sha1);
                buf
            }
        }
    }
}

mod sha1 {
    use sha1::{Digest, Sha1};

    pub(super) fn sha1(bytes: &[u8]) -> Vec<u8> {
        let mut sha1 = Sha1::default();
        sha1.update(bytes);
        sha1.finalize().to_vec()
    }
}

fn can_use_etag_v1(parts: &[usize]) -> bool {
    for (i, &part) in parts.iter().enumerate() {
        if i != parts.len() - 1 && part != DEFAULT_BLOCK_SIZE || part > DEFAULT_BLOCK_SIZE {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{error::Error, io::Cursor, result::Result};

    #[test]
    fn test_etag_v1() -> Result<(), Box<dyn Error>> {
        {
            let etag_v1 = EtagV1::new();
            assert_eq!(
                etag_v1.finalize_fixed().as_slice(),
                b"Fto5o-5ea0sNMlW_75VgGJCv2AcJ"
            );
        }
        {
            let mut etag_v1 = EtagV1::new();
            etag_v1.update(b"etag");
            assert_eq!(
                etag_v1.finalize_fixed().as_slice(),
                b"FpLiADEaVoALPkdb8tJEJyRTXoe_"
            );
        }
        {
            let mut etag_v1 = EtagV1::new();
            etag_v1.update(&utils::data_of_size(1 << 20));
            assert_eq!(
                etag_v1.finalize_fixed().as_slice(),
                b"Foyl8onxBLWeRLL5oItRJphv6i4b"
            );
        }
        {
            let mut etag_v1 = EtagV1::new();
            etag_v1.update(&utils::data_of_size(4 * (1 << 20)));
            assert_eq!(
                etag_v1.finalize_fixed().as_slice(),
                b"FicHOveBNs5Kn9d74M3b9tI4D-8r"
            );
        }
        {
            let mut etag_v1 = EtagV1::new();
            etag_v1.update(&utils::data_of_size(5 * (1 << 20)));
            assert_eq!(
                etag_v1.finalize_fixed().as_slice(),
                b"lg-Eb5KFCuZn-cUfj_oS2PPOU9xy"
            );
        }
        {
            let mut etag_v1 = EtagV1::new();
            etag_v1.update(&utils::data_of_size(8 * (1 << 20)));
            assert_eq!(
                etag_v1.finalize_fixed().as_slice(),
                b"lkSKZOMToDp-EqLDVuT1pyjQssl-"
            );
        }
        {
            let mut etag_v1 = EtagV1::new();
            etag_v1.update(&utils::data_of_size(9 * (1 << 20)));
            assert_eq!(
                etag_v1.finalize_fixed().as_slice(),
                b"ljgVjMtyMsOgIySv79U8Qz4TrUO4"
            );
        }
        Ok(())
    }

    #[test]
    fn test_etag_v2() -> Result<(), Box<dyn Error>> {
        {
            let mut etag_v2 = EtagV2::new();
            etag_v2.update(b"hello");
            etag_v2.update(b"world");
            assert_eq!(
                etag_v2.finalize_fixed().as_slice(),
                b"ns56DcSIfBFUENXjdhsJTIvl3Rcu"
            );
        }
        {
            let mut etag_v2 = EtagV2::new();
            etag_v2.update(&utils::data_of_size(1 << 19));
            etag_v2.update(&utils::data_of_size(1 << 19));
            assert_eq!(
                etag_v2.finalize_fixed().as_slice(),
                b"nlF4JinKEDBChmFGYbEIsZt6Gxnw"
            );
        }
        {
            let mut etag_v2 = EtagV2::new();
            etag_v2.update(&utils::data_of_size(1 << 19));
            etag_v2.update(&utils::data_of_size(1 << 23));
            assert_eq!(
                etag_v2.finalize_fixed().as_slice(),
                b"nt82yvMNHlNgZ4H8_A_4de84mr2f"
            );
        }
        Ok(())
    }

    #[test]
    fn test_etag_of_reader() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            etag_of_reader(Cursor::new(utils::data_of_size(1 << 20)))?,
            "Foyl8onxBLWeRLL5oItRJphv6i4b",
        );
        assert_eq!(
            etag_of_reader(&mut Cursor::new(utils::data_of_size(9 << 20)))?,
            "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
        );
        Ok(())
    }

    #[test]
    fn test_etag_of_reader_with_parts() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            etag_of_reader_with_parts(Cursor::new(utils::data_of_size(1 << 20)), &[1 << 20])?,
            "Foyl8onxBLWeRLL5oItRJphv6i4b",
        );
        assert_eq!(
            etag_of_reader_with_parts(
                &mut Cursor::new(utils::data_of_size(9 << 20)),
                &[1 << 22, 1 << 22, 1 << 20]
            )?,
            "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
        );
        assert_eq!(
            etag_of_reader_with_parts(
                Cursor::new(utils::data_of_size(1 << 20)),
                &[1 << 19, 1 << 19]
            )?,
            "nlF4JinKEDBChmFGYbEIsZt6Gxnw",
        );
        assert_eq!(
            etag_of_reader_with_parts(
                &mut Cursor::new(utils::data_of_size((1 << 19) + (1 << 23))),
                &[1 << 19, 1 << 23]
            )?,
            "nt82yvMNHlNgZ4H8_A_4de84mr2f",
        );
        Ok(())
    }

    mod utils {
        const FAKE_DATA: [u8; 4096] = make_fake_data();

        pub(super) fn data_of_size(size: usize) -> Vec<u8> {
            let mut buffer = Vec::with_capacity(size);
            let mut rest = size;

            while rest > 0 {
                let add_size = rest.min(FAKE_DATA.len());
                buffer.extend_from_slice(&FAKE_DATA[..add_size]);
                rest -= add_size;
            }
            buffer
        }

        const fn make_fake_data() -> [u8; 4096] {
            let mut buf = [b'b'; 4096];
            buf[0] = b'A';
            buf[4094] = b'\r';
            buf[4095] = b'\n';
            buf
        }
    }
}
