use super::{
    etag_v1::EtagV1,
    sha1::{hash_sha1s, sha1},
};
use digest::generic_array::{typenum::U28, GenericArray};
use digest::{FixedOutput, Reset, Update};
use qiniu_utils::base64;

/// Etag V2 计算器，使用 Etag V2 算法计算七牛云存储上文件的 HASH 值
#[derive(Default)]
pub struct EtagV2 {
    buffer: Vec<u8>,
    etag_v1: EtagV1,
}

impl EtagV2 {
    /// 构建 Etag V2 计算器
    #[inline]
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
            .extend_from_slice(&hash_sha1s(self.etag_v1.sha1s())[1..]);
        self.etag_v1.reset()
    }
}

impl Reset for EtagV2 {
    /// 重置 Etag V2 计算器
    #[inline]
    fn reset(&mut self) {
        self.buffer.clear();
    }
}

impl FixedOutput for EtagV2 {
    type OutputSize = U28;

    /// 计算 Etag V2，生成结果
    #[inline]
    fn finalize_into(mut self, out: &mut GenericArray<u8, Self::OutputSize>) {
        self.finalize_into_without_reset(out);
    }

    /// 计算 Etag V2，生成结果，然后重置该实例
    #[inline]
    fn finalize_into_reset(&mut self, out: &mut GenericArray<u8, Self::OutputSize>) {
        self.finalize_into_without_reset(out);
        self.reset();
    }
}

impl EtagV2 {
    fn finalize_into_without_reset(&mut self, out: &mut GenericArray<u8, U28>) {
        let mut buffer = Vec::with_capacity(21);
        buffer.push(0x9eu8);
        buffer.extend_from_slice(&sha1(&self.buffer));
        base64::urlsafe_slice(&buffer, out);
    }
}
