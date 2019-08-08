pub fn urlsafe(data: &[u8]) -> String {
    base64::encode_config(data, base64::URL_SAFE)
}

pub fn urlsafe_buf(data: &[u8], mut encoded: &mut String) {
    base64::encode_config_buf(data, base64::URL_SAFE, &mut encoded)
}

pub fn urlsafe_slice(data: &[u8], mut encoded: &mut [u8]) -> usize {
    base64::encode_config_slice(data, base64::URL_SAFE, &mut encoded)
}
