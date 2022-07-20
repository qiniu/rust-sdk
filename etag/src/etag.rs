use super::{etag_v1::DEFAULT_BLOCK_SIZE, EtagV1, EtagV2};
use assert_impl::assert_impl;
use digest::{
    generic_array::{typenum::U28, GenericArray},
    FixedOutput, Reset, Update,
};
use std::io::{copy, Read, Result};

/// Etag 字符串固定长度
pub const ETAG_SIZE: usize = 28;

/// 兼容 Etag 兼容计算器，可以为不同版本的 Etag 提供相同的接口
#[derive(Debug)]
#[non_exhaustive]
pub enum Etag {
    /// Etag V1 计算器
    V1(EtagV1),

    /// Etag V2 计算器
    V2(EtagV2),
}

impl Etag {
    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// Etag 版本
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtagVersion {
    /// Etag V1
    V1,
    /// Etag V2
    V2,
}

impl Etag {
    /// 创建指定版本的 Etag 兼容计算器
    pub fn new(version: EtagVersion) -> Etag {
        match version {
            EtagVersion::V1 => Etag::V1(EtagV1::new()),
            EtagVersion::V2 => Etag::V2(EtagV2::new()),
        }
    }
}

impl Update for Etag {
    /// 向 Etag 兼容计算器输入数据，数据尺寸任意
    fn update(&mut self, data: impl AsRef<[u8]>) {
        match self {
            Self::V1(etag_v1) => etag_v1.update(data),
            Self::V2(etag_v2) => etag_v2.update(data),
        }
    }
}

impl FixedOutput for Etag {
    type OutputSize = U28;

    /// 计算 Etag，生成结果
    fn finalize_into(self, out: &mut GenericArray<u8, Self::OutputSize>) {
        match self {
            Self::V1(etag_v1) => etag_v1.finalize_into(out),
            Self::V2(etag_v2) => etag_v2.finalize_into(out),
        }
    }

    /// 计算 Etag，生成结果，然后重置该实例
    fn finalize_into_reset(&mut self, out: &mut GenericArray<u8, Self::OutputSize>) {
        match self {
            Self::V1(etag_v1) => etag_v1.finalize_into_reset(out),
            Self::V2(etag_v2) => etag_v2.finalize_into_reset(out),
        }
    }
}

impl Reset for Etag {
    /// 重置 Etag 兼容计算器
    fn reset(&mut self) {
        match self {
            Self::V1(etag_v1) => etag_v1.reset(),
            Self::V2(etag_v2) => etag_v2.reset(),
        }
    }
}

fn _etag_of_reader(mut reader: impl Read, out: &mut GenericArray<u8, U28>) -> Result<()> {
    let mut etag_v1 = EtagV1::new();
    copy(&mut reader, &mut etag_v1)?;
    etag_v1.finalize_into(out);
    Ok(())
}

/// 读取 reader 中的数据并计算它的 Etag V1，生成结果
pub fn etag_of(reader: impl Read) -> Result<String> {
    let mut buf = GenericArray::default();
    _etag_of_reader(reader, &mut buf)?;
    Ok(String::from_utf8(buf.to_vec()).unwrap())
}

/// 读取 reader 中的数据并计算它的 Etag V1，生成结果到指定的缓冲中
pub fn etag_to_buf(reader: impl Read, array: &mut [u8; ETAG_SIZE]) -> Result<()> {
    _etag_of_reader(reader, GenericArray::from_mut_slice(array))?;
    Ok(())
}

#[allow(clippy::read_zero_byte_vec)]
fn _etag_of_reader_with_parts(mut reader: impl Read, parts: &[usize], out: &mut GenericArray<u8, U28>) -> Result<()> {
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

/// 根据给出的数据块尺寸，读取 reader 中的数据并计算它的 Etag V2，生成结果
pub fn etag_with_parts(reader: impl Read, parts: &[usize]) -> Result<String> {
    let mut buf = GenericArray::default();
    _etag_of_reader_with_parts(reader, parts, &mut buf)?;
    Ok(String::from_utf8(buf.to_vec()).unwrap())
}

/// 根据给出的数据块尺寸，读取 reader 中的数据并计算它的 Etag V2，生成结果到指定的数组中
pub fn etag_with_parts_to_buf(reader: impl Read, parts: &[usize], array: &mut [u8; ETAG_SIZE]) -> Result<()> {
    _etag_of_reader_with_parts(reader, parts, GenericArray::from_mut_slice(array))?;
    Ok(())
}

pub(super) fn can_use_etag_v1(parts: &[usize]) -> bool {
    !parts
        .iter()
        .enumerate()
        .any(|(i, &part)| i != parts.len() - 1 && part != DEFAULT_BLOCK_SIZE || part > DEFAULT_BLOCK_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std as _;
    use std::{error::Error, io::Cursor, result::Result};

    #[test]
    fn test_etag_v1() -> Result<(), Box<dyn Error>> {
        {
            let etag_v1 = EtagV1::new();
            assert_eq!(etag_v1.finalize_fixed().as_slice(), b"Fto5o-5ea0sNMlW_75VgGJCv2AcJ");
        }
        {
            let mut etag_v1 = EtagV1::new();
            etag_v1.update(b"etag");
            assert_eq!(etag_v1.finalize_fixed().as_slice(), b"FpLiADEaVoALPkdb8tJEJyRTXoe_");
        }
        {
            let mut etag_v1 = EtagV1::new();
            etag_v1.update(&utils::data_of_size(1 << 20));
            assert_eq!(etag_v1.finalize_fixed().as_slice(), b"Foyl8onxBLWeRLL5oItRJphv6i4b");
        }
        {
            let mut etag_v1 = EtagV1::new();
            etag_v1.update(&utils::data_of_size(4 * (1 << 20)));
            assert_eq!(etag_v1.finalize_fixed().as_slice(), b"FicHOveBNs5Kn9d74M3b9tI4D-8r");
        }
        {
            let mut etag_v1 = EtagV1::new();
            etag_v1.update(&utils::data_of_size(5 * (1 << 20)));
            assert_eq!(etag_v1.finalize_fixed().as_slice(), b"lg-Eb5KFCuZn-cUfj_oS2PPOU9xy");
        }
        {
            let mut etag_v1 = EtagV1::new();
            etag_v1.update(&utils::data_of_size(8 * (1 << 20)));
            assert_eq!(etag_v1.finalize_fixed().as_slice(), b"lkSKZOMToDp-EqLDVuT1pyjQssl-");
        }
        {
            let mut etag_v1 = EtagV1::new();
            etag_v1.update(&utils::data_of_size(9 * (1 << 20)));
            assert_eq!(etag_v1.finalize_fixed().as_slice(), b"ljgVjMtyMsOgIySv79U8Qz4TrUO4");
        }
        Ok(())
    }

    #[test]
    fn test_etag_v2() -> Result<(), Box<dyn Error>> {
        {
            let mut etag_v2 = EtagV2::new();
            etag_v2.update(b"hello");
            etag_v2.update(b"world");
            assert_eq!(etag_v2.finalize_fixed().as_slice(), b"ns56DcSIfBFUENXjdhsJTIvl3Rcu");
        }
        {
            let mut etag_v2 = EtagV2::new();
            etag_v2.update(&utils::data_of_size(1 << 19));
            etag_v2.update(&utils::data_of_size(1 << 19));
            assert_eq!(etag_v2.finalize_fixed().as_slice(), b"nlF4JinKEDBChmFGYbEIsZt6Gxnw");
        }
        {
            let mut etag_v2 = EtagV2::new();
            etag_v2.update(&utils::data_of_size(1 << 19));
            etag_v2.update(&utils::data_of_size(1 << 23));
            assert_eq!(etag_v2.finalize_fixed().as_slice(), b"nt82yvMNHlNgZ4H8_A_4de84mr2f");
        }
        {
            let mut etag_v1 = EtagV1::new();
            let mut etag_v2 = EtagV2::new();
            etag_v1.update(&utils::data_of_size(1 << 22));
            etag_v2.update(&utils::data_of_size(1 << 22));
            etag_v1.update(&utils::data_of_size(1 << 22));
            etag_v2.update(&utils::data_of_size(1 << 22));
            etag_v1.update(&utils::data_of_size(1 << 20));
            etag_v2.update(&utils::data_of_size(1 << 20));
            assert_eq!(etag_v1.finalize_fixed().as_slice(), etag_v2.finalize_fixed().as_slice(),);
        }
        {
            let mut etag_v1 = EtagV1::new();
            let mut etag_v2 = EtagV2::new();
            etag_v1.update(&utils::data_of_size(1 << 22));
            etag_v2.update(&utils::data_of_size(1 << 22));
            etag_v1.update(&utils::data_of_size(1 << 22));
            etag_v2.update(&utils::data_of_size(1 << 22));
            etag_v1.update(&utils::data_of_size(1 << 20));
            etag_v2.update(&utils::data_of_size(1 << 20));
            etag_v1.update(&utils::data_of_size(1 << 22));
            etag_v2.update(&utils::data_of_size(1 << 22));
            assert_ne!(etag_v1.finalize_fixed().as_slice(), etag_v2.finalize_fixed().as_slice(),);
        }
        Ok(())
    }

    #[test]
    fn test_etag_of_reader() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            etag_of(Cursor::new(utils::data_of_size(1 << 20)))?,
            "Foyl8onxBLWeRLL5oItRJphv6i4b",
        );
        assert_eq!(
            etag_of(&mut Cursor::new(utils::data_of_size(9 << 20)))?,
            "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
        );
        Ok(())
    }

    #[test]
    fn test_etag_of_reader_with_parts() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            etag_with_parts(Cursor::new(utils::data_of_size(1 << 20)), &[1 << 20])?,
            "Foyl8onxBLWeRLL5oItRJphv6i4b",
        );
        assert_eq!(
            etag_with_parts(
                &mut Cursor::new(utils::data_of_size(9 << 20)),
                &[1 << 22, 1 << 22, 1 << 20]
            )?,
            "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
        );
        assert_eq!(
            etag_with_parts(Cursor::new(utils::data_of_size(1 << 20)), &[1 << 19, 1 << 19])?,
            "nlF4JinKEDBChmFGYbEIsZt6Gxnw",
        );
        assert_eq!(
            etag_with_parts(
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
