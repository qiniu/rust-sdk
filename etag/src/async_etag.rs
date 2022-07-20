use super::{etag::can_use_etag_v1, EtagV1, EtagV2, ETAG_SIZE};
use digest::{
    generic_array::{typenum::U28, GenericArray},
    FixedOutput, Update,
};
use futures_lite::io::{copy, AsyncRead, AsyncReadExt, Result};

async fn _etag_of_reader(mut reader: impl AsyncRead + Unpin, out: &mut GenericArray<u8, U28>) -> Result<()> {
    let mut etag_v1 = EtagV1::new();
    copy(&mut reader, &mut etag_v1).await?;
    etag_v1.finalize_into(out);
    Ok(())
}

/// 异步读取 reader 中的数据并计算它的 Etag V1，生成结果
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub async fn etag_of(reader: impl AsyncRead + Unpin) -> Result<String> {
    let mut buf = GenericArray::default();
    _etag_of_reader(reader, &mut buf).await?;
    Ok(String::from_utf8(buf.to_vec()).unwrap())
}

/// 异步读取 reader 中的数据并计算它的 Etag V1，生成结果到指定的缓冲中
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub async fn etag_to_buf(reader: impl AsyncRead + Unpin, array: &mut [u8; ETAG_SIZE]) -> Result<()> {
    _etag_of_reader(reader, GenericArray::from_mut_slice(array)).await?;
    Ok(())
}

#[allow(clippy::read_zero_byte_vec)]
async fn _etag_of_reader_with_parts(
    mut reader: impl AsyncRead + Unpin,
    parts: &[usize],
    out: &mut GenericArray<u8, U28>,
) -> Result<()> {
    if can_use_etag_v1(parts) {
        return _etag_of_reader(reader, out).await;
    }

    let mut etag_v2 = EtagV2::new();
    let mut buf = Vec::new();
    for &part in parts.iter() {
        buf.resize(part, 0u8);
        reader.read_exact(&mut buf).await?;
        etag_v2.update(&buf);
    }
    etag_v2.finalize_into(out);
    Ok(())
}

/// 根据给出的数据块尺寸，异步读取 reader 中的数据并计算它的 Etag V2，生成结果
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub async fn etag_with_parts(reader: impl AsyncRead + Unpin, parts: &[usize]) -> Result<String> {
    let mut buf = GenericArray::default();
    _etag_of_reader_with_parts(reader, parts, &mut buf).await?;
    Ok(String::from_utf8(buf.to_vec()).unwrap())
}

/// 根据给出的数据块尺寸，异步读取 reader 中的数据并计算它的 Etag V2，生成结果到指定的数组中
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub async fn etag_with_parts_to_buf(
    reader: impl AsyncRead + Unpin,
    parts: &[usize],
    array: &mut [u8; ETAG_SIZE],
) -> Result<()> {
    _etag_of_reader_with_parts(reader, parts, GenericArray::from_mut_slice(array)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_lite::io::Cursor;
    use std::{error::Error, result::Result};

    #[async_std::test]
    async fn test_etag_of_reader() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            etag_of(Cursor::new(utils::data_of_size(1 << 20))).await?,
            "Foyl8onxBLWeRLL5oItRJphv6i4b",
        );
        assert_eq!(
            etag_of(&mut Cursor::new(utils::data_of_size(9 << 20))).await?,
            "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
        );
        Ok(())
    }

    #[async_std::test]
    async fn test_etag_of_reader_with_parts() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            etag_with_parts(Cursor::new(utils::data_of_size(1 << 20)), &[1 << 20]).await?,
            "Foyl8onxBLWeRLL5oItRJphv6i4b",
        );
        assert_eq!(
            etag_with_parts(
                &mut Cursor::new(utils::data_of_size(9 << 20)),
                &[1 << 22, 1 << 22, 1 << 20]
            )
            .await?,
            "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
        );
        assert_eq!(
            etag_with_parts(Cursor::new(utils::data_of_size(1 << 20)), &[1 << 19, 1 << 19]).await?,
            "nlF4JinKEDBChmFGYbEIsZt6Gxnw",
        );
        assert_eq!(
            etag_with_parts(
                &mut Cursor::new(utils::data_of_size((1 << 19) + (1 << 23))),
                &[1 << 19, 1 << 23]
            )
            .await?,
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
