use super::{
    etag_v1::{EtagV1, DEFAULT_BLOCK_SIZE},
    sha1::{hash_sha1s, sha1},
};
use assert_impl::assert_impl;
use digest::generic_array::{typenum::U28, GenericArray};
use digest::{FixedOutput, Reset, Update};
use qiniu_utils::base64;

/// Etag V2 计算器，使用 Etag V2 算法计算七牛云存储上文件的 HASH 值
#[derive(Debug, Default)]
pub struct EtagV2 {
    buffer: Vec<u8>,
    partial_etag: EtagV1,
    etag_v1: Option<EtagV1>,
    encounter_non_4m_block: bool,
}

impl EtagV2 {
    /// 构建 Etag V2 计算器
    #[inline]
    pub fn new() -> Self {
        Self {
            buffer: Default::default(),
            partial_etag: Default::default(),
            etag_v1: Some(Default::default()),
            encounter_non_4m_block: false,
        }
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl Update for EtagV2 {
    /// 向 Etag V2 计算器输入数据，数据尺寸任意，但是一次输入的数据将被视为一个数据块
    fn update(&mut self, data: impl AsRef<[u8]>) {
        if self.encounter_non_4m_block {
            self.etag_v1 = None;
        }
        if let Some(etag_v1) = self.etag_v1.as_mut() {
            etag_v1.update(data.as_ref());
        }
        if data.as_ref().len() != DEFAULT_BLOCK_SIZE {
            self.encounter_non_4m_block = true;
        }
        self.partial_etag.reset();
        self.partial_etag.update(data.as_ref());
        self.partial_etag.finish();
        self.buffer
            .extend_from_slice(&hash_sha1s(self.partial_etag.sha1s())[1..]);
    }
}

impl Reset for EtagV2 {
    /// 重置 Etag V2 计算器
    fn reset(&mut self) {
        self.buffer.clear();
        if let Some(etag_v1) = &mut self.etag_v1 {
            etag_v1.reset();
        } else {
            self.etag_v1 = Some(Default::default());
        }
        self.encounter_non_4m_block = false;
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
        if let Some(etag_v1) = &mut self.etag_v1 {
            etag_v1.finalize_into_without_reset(out);
        } else {
            let mut buffer = Vec::with_capacity(21);
            buffer.push(0x9eu8);
            buffer.extend_from_slice(&sha1(&self.buffer));
            base64::urlsafe_slice(&buffer, out);
        }
    }
}
