//! Base64 计算库
//!
//! 提供简单的 URLSafe Base64 计算库

pub use base64::DecodeError;
use std::result::Result;

/// 以 URL 安全的方式，将指定的二进制数据编码为 Base64 字符串
pub fn urlsafe(data: &[u8]) -> String {
    base64::encode_config(data, base64::URL_SAFE)
}

/// 以 URL 安全的方式，将指定的二进制数据编码为 Base64 字符串
pub fn urlsafe_buf(data: &[u8], encoded: &mut String) {
    base64::encode_config_buf(data, base64::URL_SAFE, encoded)
}

/// 以 URL 安全的方式，将指定的二进制数据编码为 Base64 字符串
pub fn urlsafe_slice(data: &[u8], encoded: &mut [u8]) -> usize {
    base64::encode_config_slice(data, base64::URL_SAFE, encoded)
}

/// 以 URL 安全的方式，将指定的 Base64 字符串解码为二进制数据
pub fn decode(data: &[u8]) -> Result<Vec<u8>, DecodeError> {
    base64::decode_config(data, base64::URL_SAFE)
}

/// 以 URL 安全的方式，将指定的 Base64 字符串解码为二进制数据
pub fn decode_buf(data: &[u8], decoded: &mut Vec<u8>) -> Result<(), DecodeError> {
    base64::decode_config_buf(data, base64::URL_SAFE, decoded)
}

/// 以 URL 安全的方式，将指定的 Base64 字符串解码为二进制数据
pub fn decode_slice(data: &[u8], decoded: &mut [u8]) -> Result<usize, DecodeError> {
    base64::decode_config_slice(data, base64::URL_SAFE, decoded)
}
