pub fn urlsafe(data: &[u8]) -> String {
    base64::encode_config(data, base64::URL_SAFE)
}
