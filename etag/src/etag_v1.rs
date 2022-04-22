use super::sha1::{hash_sha1s, sha1};
use assert_impl::assert_impl;
use digest::generic_array::{typenum::U28, GenericArray};
use digest::{FixedOutput, Reset, Update};
use qiniu_utils::base64;
use std::io::{Result, Write};

pub(super) const DEFAULT_BLOCK_SIZE: usize = 1 << 22;

/// Etag V1 计算器，使用 Etag V1 算法计算七牛云存储上文件的 HASH 值
#[derive(Debug, Default)]
pub struct EtagV1 {
    buffer: Vec<u8>,
    sha1s: Vec<Vec<u8>>,
}

impl EtagV1 {
    /// 构建 Etag V1 计算器
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl Update for EtagV1 {
    /// 向 Etag V1 计算器输入数据，数据尺寸任意
    fn update(&mut self, data: impl AsRef<[u8]>) {
        self.buffer.extend_from_slice(data.as_ref());
        let mut iter = self.buffer.chunks_exact(DEFAULT_BLOCK_SIZE);
        for block in iter.by_ref() {
            self.sha1s.push(sha1(block));
        }
        self.buffer = {
            let mut new_buffer = Vec::new();
            new_buffer.extend_from_slice(iter.remainder());
            new_buffer
        };
    }
}

impl Write for EtagV1 {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.update(buf);
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(feature = "async")]
use {
    futures_lite::io::AsyncWrite,
    std::{
        pin::Pin,
        task::{Context, Poll},
    },
};

#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
impl AsyncWrite for EtagV1 {
    #[inline]
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        self.update(buf);
        Poll::Ready(Ok(buf.len()))
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl FixedOutput for EtagV1 {
    type OutputSize = U28;

    /// 计算 Etag V1，生成结果
    #[inline]
    fn finalize_into(mut self, out: &mut GenericArray<u8, Self::OutputSize>) {
        self.finalize_into_without_reset(out);
    }

    /// 计算 Etag V1，生成结果，然后重置该实例
    #[inline]
    fn finalize_into_reset(&mut self, out: &mut GenericArray<u8, Self::OutputSize>) {
        self.finalize_into_without_reset(out);
        self.reset();
    }
}

impl Reset for EtagV1 {
    /// 重置 Etag V1 计算器
    #[inline]
    fn reset(&mut self) {
        self.buffer.clear();
        self.sha1s.clear();
    }
}

impl EtagV1 {
    pub(super) fn finalize_into_without_reset(&mut self, out: &mut GenericArray<u8, U28>) {
        self.finish();
        self.calculate(out);
    }

    pub(super) fn finish(&mut self) {
        if !self.buffer.is_empty() {
            self.sha1s.push(sha1(&self.buffer));
            self.buffer.clear();
        }
    }

    fn calculate(&mut self, out: &mut GenericArray<u8, U28>) {
        base64::urlsafe_slice(&hash_sha1s(&self.sha1s), out);
    }

    pub(super) fn sha1s(&self) -> &[Vec<u8>] {
        &self.sha1s
    }
}
