//! 七牛认证信息模块
use crate::{
    http::{Headers, Method},
    storage::upload_policy::UploadPolicy,
    utils::{base64, mime},
};
use crypto_mac::Mac;
use hmac::Hmac;
use qiniu_http::Request;
use sha1::Sha1;
use std::{borrow::Cow, cmp::PartialEq, convert::TryFrom, fmt, result::Result, string::String, sync::Arc, time};
use url::Url;

#[derive(Clone, Eq, PartialEq)]
struct CredentialInner {
    access_key: Cow<'static, str>,
    secret_key: Cow<'static, str>,
}

/// 认证信息
///
/// 该结构体仅用于为其他 SDK 类提供认证信息，本身并不会验证认证信息的有效性
#[derive(Clone, Eq)]
pub struct Credential(Arc<CredentialInner>);

impl Credential {
    /// 创建认证，注意该方法不会验证 Access Key 和 Secret Key 的有效性。
    ///
    /// # Arguments
    ///
    /// * `access_key` - 七牛 Access Key
    /// * `secret_key` - 七牛 Secret Key
    ///
    /// # Example
    ///
    /// ```
    /// use qiniu_ng::Credential;
    /// let credential = Credential::new("[Access Key]", "[Secret Key]");
    /// ```
    pub fn new(access_key: impl Into<Cow<'static, str>>, secret_key: impl Into<Cow<'static, str>>) -> Credential {
        Credential(Arc::new(CredentialInner {
            access_key: access_key.into(),
            secret_key: secret_key.into(),
        }))
    }

    /// 获取七牛 Access Key
    pub fn access_key(&self) -> &str {
        self.0.access_key.as_ref()
    }

    fn secret_key(&self) -> &str {
        self.0.secret_key.as_ref()
    }

    pub(crate) fn sign(&self, data: &[u8]) -> String {
        self.access_key().to_owned() + ":" + &self.base64ed_hmac_digest(data)
    }

    pub(crate) fn sign_with_data(&self, data: &[u8]) -> String {
        let encoded_data = base64::urlsafe(data);
        self.sign(encoded_data.as_bytes()) + ":" + &encoded_data
    }

    pub(crate) fn authorization_v1_for_request(
        &self,
        url_string: impl AsRef<str>,
        content_type: Option<impl AsRef<str>>,
        body: Option<&[u8]>,
    ) -> Result<String, url::ParseError> {
        let authorization_token = self.sign_request_v1(url_string, content_type, body)?;
        Ok("QBox ".to_owned() + &authorization_token)
    }

    pub(crate) fn authorization_v2_for_request(
        &self,
        method: Method,
        url_string: impl AsRef<str>,
        headers: &Headers,
        body: Option<&[u8]>,
    ) -> Result<String, url::ParseError> {
        let authorization_token = self.sign_request_v2(method, url_string, headers, body)?;
        Ok("Qiniu ".to_owned() + &authorization_token)
    }

    pub(crate) fn sign_request_v1(
        &self,
        url_string: impl AsRef<str>,
        content_type: Option<impl AsRef<str>>,
        body: Option<&[u8]>,
    ) -> Result<String, url::ParseError> {
        let u = Url::parse(url_string.as_ref())?;
        let mut data_to_sign = Vec::with_capacity(1024);
        data_to_sign.extend_from_slice(u.path().as_bytes());
        if let Some(query) = u.query() {
            if !query.is_empty() {
                data_to_sign.extend_from_slice(b"?");
                data_to_sign.extend_from_slice(query.as_bytes());
            }
        }
        data_to_sign.extend_from_slice(b"\n");
        if let (Some(content_type), Some(body)) = (content_type, body) {
            if Self::will_push_body_v1(content_type) {
                data_to_sign.extend_from_slice(body);
            }
        }
        Ok(self.sign(&data_to_sign))
    }

    pub(crate) fn sign_request_v2(
        &self,
        method: Method,
        url_string: impl AsRef<str>,
        headers: &Headers,
        body: Option<&[u8]>,
    ) -> Result<String, url::ParseError> {
        let u = Url::parse(url_string.as_ref())?;
        let mut data_to_sign = Vec::with_capacity(1024);
        data_to_sign.extend_from_slice(method.as_bytes());
        data_to_sign.extend_from_slice(b" ");
        data_to_sign.extend_from_slice(u.path().as_bytes());
        if let Some(query) = u.query() {
            if !query.is_empty() {
                data_to_sign.extend_from_slice(b"?");
                data_to_sign.extend_from_slice(query.as_bytes());
            }
        }
        data_to_sign.extend_from_slice(b"\nHost: ");
        data_to_sign.extend_from_slice(u.host_str().expect("Host must be existed in URL").as_bytes());
        if let Some(port) = u.port() {
            data_to_sign.extend_from_slice(b":");
            data_to_sign.extend_from_slice(port.to_string().as_bytes());
        }
        data_to_sign.extend_from_slice(b"\n");

        if let Some(content_type) = headers.get(&"Content-Type".into()) {
            data_to_sign.extend_from_slice(b"Content-Type: ");
            data_to_sign.extend_from_slice(content_type.as_ref().as_bytes());
            data_to_sign.extend_from_slice(b"\n");
            sign_data_for_x_qiniu_headers(&mut data_to_sign, headers);
            data_to_sign.extend_from_slice(b"\n");
            if let Some(body) = body {
                if Self::will_push_body_v2(content_type) {
                    data_to_sign.extend_from_slice(body);
                }
            }
        } else {
            sign_data_for_x_qiniu_headers(&mut data_to_sign, &headers);
            data_to_sign.extend_from_slice(b"\n");
        }
        return Ok(self.sign(&data_to_sign));

        fn sign_data_for_x_qiniu_headers(data_to_sign: &mut Vec<u8>, headers: &Headers) {
            let mut x_qiniu_headers = headers
                .iter()
                .filter(|(key, _)| key.len() > "X-Qiniu-".len())
                .filter(|(key, _)| key.starts_with("X-Qiniu-"))
                .collect::<Vec<_>>();
            if x_qiniu_headers.is_empty() {
                return;
            }
            x_qiniu_headers.sort_unstable();
            for (header_key, header_value) in x_qiniu_headers {
                data_to_sign.extend_from_slice(header_key.as_bytes());
                data_to_sign.extend_from_slice(b": ");
                data_to_sign.extend_from_slice(header_value.as_bytes());
                data_to_sign.extend_from_slice(b"\n");
            }
        }
    }

    fn base64ed_hmac_digest(&self, data: &[u8]) -> String {
        let mut hmac = Hmac::<Sha1>::new_varkey(self.secret_key().as_bytes()).unwrap();
        hmac.input(data);
        base64::urlsafe(&hmac.result().code())
    }

    fn will_push_body_v1<ContentType: AsRef<str>>(content_type: ContentType) -> bool {
        mime::FORM_MIME.eq_ignore_ascii_case(content_type.as_ref())
    }

    fn will_push_body_v2<ContentType: AsRef<str>>(content_type: ContentType) -> bool {
        mime::FORM_MIME.eq_ignore_ascii_case(content_type.as_ref())
            || mime::JSON_MIME.eq_ignore_ascii_case(content_type.as_ref())
    }

    /// 验证七牛回调请求
    pub fn is_valid_request(&self, req: &Request) -> bool {
        self.is_valid_request_with_err(req).unwrap_or(false)
    }

    fn is_valid_request_with_err(&self, req: &Request) -> Result<bool, url::ParseError> {
        if let Some(original_authorization) = req.headers().get(&"Authorization".into()) {
            Ok(original_authorization
                == &self.authorization_v1_for_request(
                    req.url(),
                    req.headers().get(&"Content-Type".into()),
                    req.body().as_ref().map(|body| body.as_ref()),
                )?)
        } else {
            Ok(false)
        }
    }

    /// 对上传策略进行签名，将其转变为上传凭证
    pub fn sign_upload_policy(&self, upload_policy: &UploadPolicy) -> String {
        self.sign_with_data(upload_policy.as_json().as_bytes())
    }

    #[allow(dead_code)]
    pub(crate) fn sign_download_url_with_deadline(
        &self,
        url: Url,
        deadline: time::SystemTime,
        only_path: bool,
    ) -> Result<String, time::SystemTimeError> {
        let mut signed_url = {
            let mut s = String::with_capacity(2048);
            s.push_str(url.as_str());
            s
        };
        let mut to_sign = {
            let mut s = String::with_capacity(2048);
            if only_path {
                s.push_str(url.path());
                if let Some(query) = url.query() {
                    s.push('?');
                    s.push_str(query);
                }
            } else {
                s.push_str(url.as_str());
            }
            s
        };

        if to_sign.contains('?') {
            to_sign.push_str("&e=");
            signed_url.push_str("&e=");
        } else {
            to_sign.push_str("?e=");
            signed_url.push_str("?e=");
        }

        let deadline = u32::try_from(deadline.duration_since(time::UNIX_EPOCH)?.as_secs())
            .unwrap_or(std::u32::MAX)
            .to_string();
        to_sign.push_str(&deadline);
        signed_url.push_str(&deadline);
        signed_url.push_str("&token=");
        signed_url.push_str(&self.sign(to_sign.as_bytes()));
        Ok(signed_url)
    }

    #[allow(dead_code)]
    pub(crate) fn sign_download_url_with_lifetime(
        &self,
        url: Url,
        lifetime: time::Duration,
        only_path: bool,
    ) -> Result<String, time::SystemTimeError> {
        let deadline = time::SystemTime::now() + lifetime;
        self.sign_download_url_with_deadline(url, deadline, only_path)
    }
}

impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!(
            "Credential {{ access_key: {:?}, secret_key: CENSORED }}",
            &self.access_key()
        ))
    }
}

impl PartialEq for Credential {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl From<Credential> for Cow<'_, Credential> {
    fn from(credential: Credential) -> Self {
        Cow::Owned(credential)
    }
}

impl<'a> From<&'a Credential> for Cow<'a, Credential> {
    fn from(credential: &'a Credential) -> Self {
        Cow::Borrowed(credential)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qiniu_http::RequestBuilder;
    use std::{boxed::Box, error::Error, result::Result, thread};

    #[test]
    fn test_sign() -> Result<(), Box<dyn Error>> {
        let credential = get_credential();
        let mut threads = Vec::new();
        {
            let credential = credential.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(
                    credential.sign(b"hello"),
                    "abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0="
                );
                assert_eq!(
                    credential.sign(b"world"),
                    "abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ="
                );
            }));
        }
        {
            let credential = credential.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(
                    credential.sign(b"-test"),
                    "abcdefghklmnopq:vYKRLUoXRlNHfpMEQeewG0zylaw="
                );
                assert_eq!(
                    credential.sign(b"ba#a-"),
                    "abcdefghklmnopq:2d_Yr6H1GdTKg3RvMtpHOhi047M="
                );
            }));
        }
        threads.into_iter().for_each(|thread| thread.join().unwrap());
        Ok(())
    }

    #[test]
    fn test_sign_data() -> Result<(), Box<dyn Error>> {
        let credential = get_credential();
        let mut threads = Vec::new();
        {
            let credential = credential.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(
                    credential.sign_with_data(b"hello"),
                    "abcdefghklmnopq:BZYt5uVRy1RVt5ZTXbaIt2ROVMA=:aGVsbG8="
                );
                assert_eq!(
                    credential.sign_with_data(b"world"),
                    "abcdefghklmnopq:Wpe04qzPphiSZb1u6I0nFn6KpZg=:d29ybGQ="
                );
            }));
        }
        {
            let credential = credential.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(
                    credential.sign_with_data(b"-test"),
                    "abcdefghklmnopq:HlxenSSP_6BbaYNzx1fyeyw8v1Y=:LXRlc3Q="
                );
                assert_eq!(
                    credential.sign_with_data(b"ba#a-"),
                    "abcdefghklmnopq:kwzeJrFziPDMO4jv3DKVLDyqud0=:YmEjYS0="
                );
            }));
        }
        threads.into_iter().for_each(|thread| thread.join().unwrap());
        Ok(())
    }

    #[test]
    fn test_sign_request_v1() -> Result<(), Box<dyn Error>> {
        let credential = get_credential();
        assert_eq!(
            credential.sign_request_v1("http://upload.qiniup.com/", None::<&str>, Some(b"{\"name\":\"test\"}"))?,
            credential.sign(b"/\n")
        );
        assert_eq!(
            credential.sign_request_v1(
                "http://upload.qiniup.com/",
                Some("application/json"),
                Some(b"{\"name\":\"test\"}")
            )?,
            credential.sign(b"/\n")
        );
        assert_eq!(
            credential.sign_request_v1(
                "http://upload.qiniup.com/",
                Some("application/x-www-form-urlencoded"),
                Some(b"name=test&language=go")
            )?,
            credential.sign(b"/\nname=test&language=go")
        );
        assert_eq!(
            credential.sign_request_v1(
                "http://upload.qiniup.com/?v=2",
                Some("application/x-www-form-urlencoded"),
                Some(b"name=test&language=go")
            )?,
            credential.sign(b"/?v=2\nname=test&language=go")
        );
        assert_eq!(
            credential.sign_request_v1(
                "http://upload.qiniup.com/find/sdk?v=2",
                Some("application/x-www-form-urlencoded"),
                Some(b"name=test&language=go")
            )?,
            credential.sign(b"/find/sdk?v=2\nname=test&language=go")
        );
        Ok(())
    }

    #[test]
    fn test_sign_request_v2() -> Result<(), Box<dyn Error>> {
        let credential = get_credential();
        let empty_headers = {
            let mut headers = Headers::new();
            headers.insert("X-Qbox-Meta".into(), "value".into());
            headers
        };
        let json_headers = {
            let mut headers = Headers::new();
            headers.insert("Content-Type".into(), "application/json".into());
            headers.insert("X-Qbox-Meta".into(), "value".into());
            headers.insert("X-Qiniu-Cxxxx".into(), "valuec".into());
            headers.insert("X-Qiniu-Bxxxx".into(), "valueb".into());
            headers.insert("X-Qiniu-axxxx".into(), "valuea".into());
            headers.insert("X-Qiniu-e".into(), "value".into());
            headers.insert("X-Qiniu-".into(), "value".into());
            headers.insert("X-Qiniu".into(), "value".into());
            headers
        };
        let form_headers = {
            let mut headers = Headers::new();
            headers.insert("Content-Type".into(), "application/x-www-form-urlencoded".into());
            headers.insert("X-Qbox-Meta".into(), "value".into());
            headers.insert("X-Qiniu-Cxxxx".into(), "valuec".into());
            headers.insert("X-Qiniu-Bxxxx".into(), "valueb".into());
            headers.insert("X-Qiniu-axxxx".into(), "valuea".into());
            headers.insert("X-Qiniu-e".into(), "value".into());
            headers.insert("X-Qiniu-".into(), "value".into());
            headers.insert("X-Qiniu".into(), "value".into());
            headers
        };
        assert_eq!(
            credential.sign_request_v2(
                Method::GET,
                "http://upload.qiniup.com/",
                &json_headers,
                Some(b"{\"name\":\"test\"}")
            )?,
            credential.sign(
                concat!(
                    "GET /\n",
                    "Host: upload.qiniup.com\n",
                    "Content-Type: application/json\n",
                    "X-Qiniu-Axxxx: valuea\n",
                    "X-Qiniu-Bxxxx: valueb\n",
                    "X-Qiniu-Cxxxx: valuec\n",
                    "X-Qiniu-E: value\n\n",
                    "{\"name\":\"test\"}"
                )
                .as_bytes()
            )
        );
        assert_eq!(
            credential.sign_request_v2(
                Method::GET,
                "http://upload.qiniup.com/",
                &empty_headers,
                Some(b"{\"name\":\"test\"}")
            )?,
            credential.sign(concat!("GET /\n", "Host: upload.qiniup.com\n\n").as_bytes())
        );
        assert_eq!(
            credential.sign_request_v2(
                Method::POST,
                "http://upload.qiniup.com/",
                &json_headers,
                Some(b"{\"name\":\"test\"}")
            )?,
            credential.sign(
                concat!(
                    "POST /\n",
                    "Host: upload.qiniup.com\n",
                    "Content-Type: application/json\n",
                    "X-Qiniu-Axxxx: valuea\n",
                    "X-Qiniu-Bxxxx: valueb\n",
                    "X-Qiniu-Cxxxx: valuec\n",
                    "X-Qiniu-E: value\n\n",
                    "{\"name\":\"test\"}"
                )
                .as_bytes()
            )
        );
        assert_eq!(
            credential.sign_request_v2(
                Method::GET,
                "http://upload.qiniup.com/",
                &form_headers,
                Some(b"name=test&language=go")
            )?,
            credential.sign(
                concat!(
                    "GET /\n",
                    "Host: upload.qiniup.com\n",
                    "Content-Type: application/x-www-form-urlencoded\n",
                    "X-Qiniu-Axxxx: valuea\n",
                    "X-Qiniu-Bxxxx: valueb\n",
                    "X-Qiniu-Cxxxx: valuec\n",
                    "X-Qiniu-E: value\n\n",
                    "name=test&language=go"
                )
                .as_bytes()
            )
        );
        assert_eq!(
            credential.sign_request_v2(
                Method::GET,
                "http://upload.qiniup.com/?v=2",
                &form_headers,
                Some(b"name=test&language=go")
            )?,
            credential.sign(
                concat!(
                    "GET /?v=2\n",
                    "Host: upload.qiniup.com\n",
                    "Content-Type: application/x-www-form-urlencoded\n",
                    "X-Qiniu-Axxxx: valuea\n",
                    "X-Qiniu-Bxxxx: valueb\n",
                    "X-Qiniu-Cxxxx: valuec\n",
                    "X-Qiniu-E: value\n\n",
                    "name=test&language=go"
                )
                .as_bytes()
            )
        );
        assert_eq!(
            credential.sign_request_v2(
                Method::GET,
                "http://upload.qiniup.com/find/sdk?v=2",
                &form_headers,
                Some(b"name=test&language=go")
            )?,
            credential.sign(
                concat!(
                    "GET /find/sdk?v=2\n",
                    "Host: upload.qiniup.com\n",
                    "Content-Type: application/x-www-form-urlencoded\n",
                    "X-Qiniu-Axxxx: valuea\n",
                    "X-Qiniu-Bxxxx: valueb\n",
                    "X-Qiniu-Cxxxx: valuec\n",
                    "X-Qiniu-E: value\n\n",
                    "name=test&language=go"
                )
                .as_bytes()
            )
        );
        Ok(())
    }

    #[test]
    fn test_is_valid_request() -> Result<(), Box<dyn Error>> {
        let credential = get_credential();

        let json_body: &[u8] = b"{\"name\":\"test\"}";
        let form_body: &[u8] = b"name=test&language=go";
        assert!(credential.is_valid_request(
            &RequestBuilder::default()
                .url("http://upload.qiniup.com/")
                .header(
                    "Authorization",
                    credential.authorization_v1_for_request("http://upload.qiniup.com/", None::<&str>, None)?
                )
                .body(json_body)
                .build()
        ));
        assert!(credential.is_valid_request(
            &RequestBuilder::default()
                .url("http://upload.qiniup.com/")
                .header(
                    "Authorization",
                    credential.authorization_v1_for_request("http://upload.qiniup.com/", None::<&str>, None)?
                )
                .header("Content-Type", "application/json")
                .body(json_body)
                .build()
        ));
        assert!(credential.is_valid_request(
            &RequestBuilder::default()
                .url("http://upload.qiniup.com/find/sdk?v=2")
                .header(
                    "Authorization",
                    credential.authorization_v1_for_request(
                        "http://upload.qiniup.com/find/sdk?v=2",
                        Some("application/x-www-form-urlencoded"),
                        Some(b"name=test&language=go")
                    )?
                )
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(form_body)
                .build()
        ));
        Ok(())
    }

    #[test]
    fn test_sign_download_url_with_deadline() -> Result<(), Box<dyn Error>> {
        let credential = get_credential();
        assert_eq!(
            credential.sign_download_url_with_deadline(
                Url::parse("http://www.qiniu.com/?go=1")?,
                time::SystemTime::UNIX_EPOCH + time::Duration::from_secs(1_234_567_890 + 3600),
                false
            )?,
            "http://www.qiniu.com/?go=1&e=1234571490&token=abcdefghklmnopq:KjQtlGAkEOhSwtFjJfYtYa2-reE=",
        );
        assert_eq!(
            credential.sign_download_url_with_deadline(
                Url::parse("http://www.qiniu.com/?go=1")?,
                time::SystemTime::UNIX_EPOCH + time::Duration::from_secs(1_234_567_890 + 3600),
                true
            )?,
            "http://www.qiniu.com/?go=1&e=1234571490&token=abcdefghklmnopq:86uQeCB9GsFFvL2wA0mgBcOMsmk=",
        );
        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
