use crypto_mac::Mac;
use hmac::Hmac;
use qiniu_utils::{
    base64,
    http::{header::Headers, method::Method},
    mime,
};
use sha1::Sha1;
use std::{
    any::Any,
    borrow::Cow,
    convert::TryFrom,
    fmt::{self, Debug},
    time::Duration,
};
use url::Url;

/// 认证信息
///
/// 返回认证信息的 AccessKey 和 SecretKey
pub struct Credential<'a> {
    pub access_key: Cow<'a, str>,
    pub secret_key: Cow<'a, str>,
}

/// 认证信息
///
/// 为认证信息的实现提供接口支持
pub trait AsCredential: Any + Debug + Sync + Send {
    /// 返回七牛认证信息
    fn get(&self) -> Credential;

    fn as_any(&self) -> &dyn Any;
    fn as_credential(&self) -> &dyn AsCredential;

    /// 使用七牛签名算法对数据进行签名
    ///
    /// 参考[管理凭证的签名算法文档](https://developer.qiniu.com/kodo/manual/1201/access-token)
    fn sign(&self, data: &[u8]) -> String {
        let value = self.get();
        value.access_key.into_owned() + ":" + &base64ed_hmac_digest(value.secret_key.as_ref(), data)
    }

    /// 使用七牛签名算法对数据进行签名，并同时给出签名和原数据
    ///
    /// 参考[上传凭证的签名算法文档](https://developer.qiniu.com/kodo/manual/1208/upload-token)
    fn sign_with_data(&self, data: &[u8]) -> String {
        let encoded_data = base64::urlsafe(data);
        self.sign(encoded_data.as_bytes()) + ":" + &encoded_data
    }

    /// 使用七牛签名算法 V1 对 HTTP 请求进行签名，返回 Authorization 的值
    fn authorization_v1_for_request(
        &self,
        url_string: &str,
        content_type: &str,
        body: &[u8],
    ) -> Result<String, url::ParseError> {
        let authorization_token =
            sign_request_v1(self.as_credential(), url_string, content_type, body)?;
        Ok("QBox ".to_owned() + &authorization_token)
    }

    /// 使用七牛签名算法 V2 对 HTTP 请求进行签名，返回 Authorization 的值
    fn authorization_v2_for_request(
        &self,
        method: Method,
        url_string: &str,
        headers: &Headers,
        body: &[u8],
    ) -> Result<String, url::ParseError> {
        let authorization_token =
            sign_request_v2(self.as_credential(), method, url_string, headers, body)?;
        Ok("Qiniu ".to_owned() + &authorization_token)
    }

    /// 对对象的下载 URL 签名，可以生成私有存储空间的下载地址或带有时间戳鉴权的下载地址
    fn sign_download_url(&self, url: &mut Url, deadline: Duration, only_path: bool) {
        let mut to_sign = {
            let mut s = String::with_capacity(1 << 10);
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
        } else {
            to_sign.push_str("?e=");
        }

        let deadline = u32::try_from(deadline.as_secs())
            .unwrap_or(std::u32::MAX)
            .to_string();
        to_sign.push_str(&deadline);
        let mut query_pairs = url.query_pairs_mut();
        query_pairs.append_pair("e", &deadline);
        query_pairs.append_pair("token", &&self.sign(to_sign.as_bytes()));
    }
}

fn sign_request_v1(
    cred: &dyn AsCredential,
    url_string: &str,
    content_type: &str,
    body: &[u8],
) -> Result<String, url::ParseError> {
    let u = Url::parse(url_string)?;
    let mut data_to_sign = Vec::with_capacity(1024);
    data_to_sign.extend_from_slice(u.path().as_bytes());
    if let Some(query) = u.query() {
        if !query.is_empty() {
            data_to_sign.extend_from_slice(b"?");
            data_to_sign.extend_from_slice(query.as_bytes());
        }
    }
    data_to_sign.extend_from_slice(b"\n");
    if !content_type.is_empty() && !body.is_empty() && will_push_body_v1(content_type) {
        data_to_sign.extend_from_slice(body);
    }
    Ok(cred.sign(&data_to_sign))
}

fn sign_request_v2(
    cred: &dyn AsCredential,
    method: Method,
    url_string: impl AsRef<str>,
    headers: &Headers,
    body: &[u8],
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
    data_to_sign.extend_from_slice(
        u.host_str()
            .expect("Host must be existed in URL")
            .as_bytes(),
    );
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
        if !body.is_empty() && will_push_body_v2(content_type) {
            data_to_sign.extend_from_slice(body);
        }
    } else {
        sign_data_for_x_qiniu_headers(&mut data_to_sign, &headers);
        data_to_sign.extend_from_slice(b"\n");
    }
    return Ok(cred.sign(&data_to_sign));

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

fn base64ed_hmac_digest(secret_key: &str, data: &[u8]) -> String {
    let mut hmac = Hmac::<Sha1>::new_varkey(secret_key.as_bytes()).unwrap();
    hmac.input(data);
    base64::urlsafe(&hmac.result().code())
}

#[inline]
fn will_push_body_v1(content_type: &str) -> bool {
    mime::FORM_MIME.eq_ignore_ascii_case(content_type)
}

#[inline]
fn will_push_body_v2(content_type: &str) -> bool {
    !mime::BINARY_MIME.eq_ignore_ascii_case(content_type)
}

/// 静态认证信息，包含一个静态的 AccessKey 和 SecretKey，一旦创建则不可修改
#[derive(Eq, PartialEq)]
pub struct StaticCredential {
    access_key: Cow<'static, str>,
    secret_key: Cow<'static, str>,
}

impl StaticCredential {
    /// 构建一个静态认证信息，只需要传入静态的 AccessKey 和 SecretKey 即可
    pub fn new(
        access_key: impl Into<Cow<'static, str>>,
        secret_key: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            access_key: access_key.into(),
            secret_key: secret_key.into(),
        }
    }
}

impl AsCredential for StaticCredential {
    #[inline]
    fn get(&self) -> Credential {
        Credential {
            access_key: Cow::Borrowed(&self.access_key),
            secret_key: Cow::Borrowed(&self.secret_key),
        }
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_credential(&self) -> &dyn AsCredential {
        self
    }
}

impl From<&dyn AsCredential> for StaticCredential {
    fn from(cred: &dyn AsCredential) -> Self {
        let value = cred.get();
        StaticCredential::new(value.access_key.into_owned(), value.secret_key.into_owned())
    }
}

impl AsRef<dyn AsCredential> for StaticCredential {
    #[inline]
    fn as_ref(&self) -> &dyn AsCredential {
        self.as_credential()
    }
}

impl fmt::Debug for StaticCredential {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!(
            "StaticCredential {{ access_key: {:?}, secret_key: CENSORED }}",
            self.get().access_key.as_ref(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{boxed::Box, error::Error, result::Result, sync::Arc, thread, time::Duration};

    #[test]
    fn test_sign() -> Result<(), Box<dyn Error>> {
        let credential: Arc<dyn AsCredential> = Arc::new(get_credential());
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
        threads
            .into_iter()
            .for_each(|thread| thread.join().unwrap());
        Ok(())
    }

    #[test]
    fn test_sign_data() -> Result<(), Box<dyn Error>> {
        let credential: Arc<dyn AsCredential> = Arc::new(get_credential());
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
        threads
            .into_iter()
            .for_each(|thread| thread.join().unwrap());
        Ok(())
    }

    #[test]
    fn test_authorization_v1() -> Result<(), Box<dyn Error>> {
        let credential = get_credential();
        assert_eq!(
            credential.authorization_v1_for_request(
                "http://upload.qiniup.com/",
                "",
                b"{\"name\":\"test\"}"
            )?,
            "QBox ".to_owned() + &credential.sign(b"/\n")
        );
        assert_eq!(
            credential.authorization_v1_for_request(
                "http://upload.qiniup.com/",
                mime::JSON_MIME,
                b"{\"name\":\"test\"}"
            )?,
            "QBox ".to_owned() + &credential.sign(b"/\n")
        );
        assert_eq!(
            credential.authorization_v1_for_request(
                "http://upload.qiniup.com/",
                mime::FORM_MIME,
                b"name=test&language=go"
            )?,
            "QBox ".to_owned() + &credential.sign(b"/\nname=test&language=go")
        );
        assert_eq!(
            credential.authorization_v1_for_request(
                "http://upload.qiniup.com/?v=2",
                mime::FORM_MIME,
                b"name=test&language=go"
            )?,
            "QBox ".to_owned() + &credential.sign(b"/?v=2\nname=test&language=go")
        );
        assert_eq!(
            credential.authorization_v1_for_request(
                "http://upload.qiniup.com/find/sdk?v=2",
                mime::FORM_MIME,
                b"name=test&language=go"
            )?,
            "QBox ".to_owned() + &credential.sign(b"/find/sdk?v=2\nname=test&language=go")
        );
        Ok(())
    }

    #[test]
    fn test_authorization_v2() -> Result<(), Box<dyn Error>> {
        let credential = get_credential();
        let empty_headers = {
            let mut headers = Headers::new();
            headers.insert("X-Qbox-Meta".into(), "value".into());
            headers
        };
        let json_headers = {
            let mut headers = Headers::new();
            headers.insert("Content-Type".into(), mime::JSON_MIME.into());
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
            headers.insert("Content-Type".into(), mime::FORM_MIME.into());
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
            credential.authorization_v2_for_request(
                Method::GET,
                "http://upload.qiniup.com/",
                &json_headers,
                b"{\"name\":\"test\"}"
            )?,
            "Qiniu ".to_owned()
                + &credential.sign(
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
            credential.authorization_v2_for_request(
                Method::GET,
                "http://upload.qiniup.com/",
                &empty_headers,
                b"{\"name\":\"test\"}"
            )?,
            "Qiniu ".to_owned()
                + &credential.sign(concat!("GET /\n", "Host: upload.qiniup.com\n\n").as_bytes())
        );
        assert_eq!(
            credential.authorization_v2_for_request(
                Method::POST,
                "http://upload.qiniup.com/",
                &json_headers,
                b"{\"name\":\"test\"}"
            )?,
            "Qiniu ".to_owned()
                + &credential.sign(
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
            credential.authorization_v2_for_request(
                Method::GET,
                "http://upload.qiniup.com/",
                &form_headers,
                b"name=test&language=go"
            )?,
            "Qiniu ".to_owned()
                + &credential.sign(
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
            credential.authorization_v2_for_request(
                Method::GET,
                "http://upload.qiniup.com/?v=2",
                &form_headers,
                b"name=test&language=go"
            )?,
            "Qiniu ".to_owned()
                + &credential.sign(
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
            credential.authorization_v2_for_request(
                Method::GET,
                "http://upload.qiniup.com/find/sdk?v=2",
                &form_headers,
                b"name=test&language=go"
            )?,
            "Qiniu ".to_owned()
                + &credential.sign(
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
    fn test_sign_download_url() -> Result<(), Box<dyn Error>> {
        let credential = get_credential();
        {
            let mut url = Url::parse("http://www.qiniu.com/?go=1")?;
            credential.sign_download_url(
                &mut url,
                Duration::from_secs(1_234_567_890 + 3600),
                false,
            );
            assert_eq!(
                url.into_string(),
                "http://www.qiniu.com/?go=1&e=1234571490&token=abcdefghklmnopq%3AKjQtlGAkEOhSwtFjJfYtYa2-reE%3D",
            );
        }
        {
            let mut url = Url::parse("http://www.qiniu.com/?go=1")?;
            credential.sign_download_url(&mut url, Duration::from_secs(1_234_567_890 + 3600), true);
            assert_eq!(
                url.into_string(),
                "http://www.qiniu.com/?go=1&e=1234571490&token=abcdefghklmnopq%3A86uQeCB9GsFFvL2wA0mgBcOMsmk%3D",
            );
        }
        Ok(())
    }

    fn get_credential() -> impl AsCredential {
        StaticCredential::new("abcdefghklmnopq", "1234567890")
    }
}
