pub use base64::DecodeError;
use std::result::Result;

pub fn urlsafe(data: &[u8]) -> String {
    base64::encode_config(data, base64::URL_SAFE)
}

pub fn urlsafe_buf(data: &[u8], mut encoded: &mut String) {
    base64::encode_config_buf(data, base64::URL_SAFE, &mut encoded)
}

pub fn urlsafe_slice(data: &[u8], mut encoded: &mut [u8]) -> usize {
    base64::encode_config_slice(data, base64::URL_SAFE, &mut encoded)
}

pub fn decode(data: &[u8]) -> Result<Vec<u8>, DecodeError> {
    base64::decode_config(data, base64::URL_SAFE)
}

pub fn decode_buf(data: &[u8], mut decoded: &mut Vec<u8>) -> Result<(), DecodeError> {
    base64::decode_config_buf(data, base64::URL_SAFE, &mut decoded)
}

pub fn decode_slice(data: &[u8], mut decoded: &mut [u8]) -> Result<usize, DecodeError> {
    base64::decode_config_slice(data, base64::URL_SAFE, &mut decoded)
}
