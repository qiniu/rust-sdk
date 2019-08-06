use super::base64;
use super::mime;
use crypto::hmac::Hmac;
use crypto::mac::Mac;
use crypto::sha1::Sha1;
use http::Method;
use std::boxed::Box;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::result::Result;
use std::string::String;
use std::string::ToString;
use std::time;
use std::u32;
use url::Url;

pub struct Auth {
    access_key: String,
    secret_key: Vec<u8>,
}

impl Auth {
    pub fn new<AccessKey: ToString, SecretKey: Into<Vec<u8>>>(
        access_key: AccessKey,
        secret_key: SecretKey,
    ) -> Auth {
        Auth {
            access_key: access_key.to_string(),
            secret_key: secret_key.into(),
        }
    }

    pub(crate) fn sign(&self, data: &[u8]) -> String {
        format!("{}:{}", self.access_key, self.base64ed_hmac_digest(data))
    }

    pub(crate) fn sign_with_data(&self, data: &[u8]) -> String {
        let encoded_data = base64::urlsafe(data);
        format!("{}:{}", self.sign(encoded_data.as_bytes()), encoded_data)
    }

    pub(crate) fn authorization_v1_for_request<URL: AsRef<str>, ContentType: AsRef<str>>(
        &self,
        url_string: URL,
        content_type: Option<ContentType>,
        body: Option<&[u8]>,
    ) -> Result<String, Box<Error>> {
        let authorization_token = self.sign_request_v1(url_string, content_type, body)?;
        Ok("QBox ".to_owned() + &authorization_token)
    }

    pub(crate) fn authorization_v2_for_request<URL: AsRef<str>, ContentType: AsRef<str>>(
        &self,
        method: Method,
        url_string: URL,
        content_type: Option<ContentType>,
        body: Option<&[u8]>,
    ) -> Result<String, Box<Error>> {
        let authorization_token = self.sign_request_v2(method, url_string, content_type, body)?;
        Ok("Qiniu ".to_owned() + &authorization_token)
    }

    pub(crate) fn sign_request_v1<URL: AsRef<str>, ContentType: AsRef<str>>(
        &self,
        url_string: URL,
        content_type: Option<ContentType>,
        body: Option<&[u8]>,
    ) -> Result<String, Box<Error>> {
        let u = Url::parse(url_string.as_ref())?;
        let mut data_to_sign = String::with_capacity(1024);
        data_to_sign.push_str(u.path());
        if let Some(query) = u.query() {
            data_to_sign.push('?');
            data_to_sign.push_str(query);
        }
        data_to_sign.push('\n');
        if let (Some(content_type), Some(body)) = (content_type, body) {
            if Self::will_push_body_v1(content_type) {
                data_to_sign.push_str(std::str::from_utf8(body)?);
            }
        }
        Ok(self.sign(data_to_sign.as_bytes()))
    }

    pub(crate) fn sign_request_v2<URL: AsRef<str>, ContentType: AsRef<str>>(
        &self,
        method: Method,
        url_string: URL,
        content_type: Option<ContentType>,
        body: Option<&[u8]>,
    ) -> Result<String, Box<Error>> {
        let u = Url::parse(url_string.as_ref())?;
        let mut data_to_sign = String::with_capacity(1024);
        data_to_sign.push_str(method.as_str());
        data_to_sign.push(' ');
        data_to_sign.push_str(u.path());
        if let Some(query) = u.query() {
            data_to_sign.push('?');
            data_to_sign.push_str(query);
        }
        data_to_sign.push_str("\nHost: ");
        data_to_sign.push_str(u.host_str().expect("Host must be existed in URL"));
        if let Some(port) = u.port() {
            data_to_sign.push(':');
            data_to_sign.push_str(&port.to_string());
        }
        data_to_sign.push('\n');

        if let Some(content_type) = content_type {
            data_to_sign.push_str("Content-Type: ");
            data_to_sign.push_str(content_type.as_ref());
            data_to_sign.push_str("\n\n");
            if let Some(body) = body {
                if Self::will_push_body_v2(content_type) {
                    data_to_sign.push_str(std::str::from_utf8(body)?);
                }
            }
        } else {
            data_to_sign.push('\n');
        }
        Ok(self.sign(data_to_sign.as_bytes()))
    }

    fn base64ed_hmac_digest(&self, data: &[u8]) -> String {
        let mut hmac = Hmac::new(Sha1::new(), &self.secret_key);
        hmac.input(data);
        base64::urlsafe(hmac.result().code())
    }

    fn will_push_body_v1<ContentType: AsRef<str>>(content_type: ContentType) -> bool {
        mime::FORM_MIME.eq_ignore_ascii_case(content_type.as_ref())
    }

    fn will_push_body_v2<ContentType: AsRef<str>>(content_type: ContentType) -> bool {
        mime::FORM_MIME.eq_ignore_ascii_case(content_type.as_ref())
            || mime::JSON_MIME.eq_ignore_ascii_case(content_type.as_ref())
    }

    pub fn is_valid_request(&self, req: &http::Request<&[u8]>) -> bool {
        self.is_valid_request_with_err(req).unwrap_or(false)
    }

    fn is_valid_request_with_err(&self, req: &http::Request<&[u8]>) -> Result<bool, Box<Error>> {
        let original_authorization = req.headers().get(http::header::AUTHORIZATION);
        if original_authorization.is_none() {
            return Ok(false);
        }
        let original_authorization = original_authorization.unwrap().to_str()?;
        let content_type = req
            .headers()
            .get(http::header::CONTENT_TYPE)
            .map(|value| value.to_str());
        let content_type = match content_type {
            Some(result) => Some(result?),
            None => None,
        };
        let expected_authorization = self.authorization_v1_for_request(
            &req.uri().to_string(),
            content_type,
            Some(req.body()),
        )?;
        Ok(original_authorization == expected_authorization)
    }

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

impl Clone for Auth {
    fn clone(&self) -> Auth {
        Auth {
            access_key: self.access_key.clone(),
            secret_key: self.secret_key.clone(),
        }
    }
}

impl fmt::Debug for Auth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!(
            "Auth {{ access_key: {:?}, secret_key: CENSORED }}",
            &self.access_key
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::http_utils;
    use super::*;
    use http::Method;
    use http::StatusCode;

    #[test]
    fn test_sign() {
        let auth = get_auth();
        let mut threads = Vec::new();
        {
            let auth_cloned = auth.clone();
            threads.push(std::thread::spawn(move || {
                assert_eq!(
                    auth_cloned.sign(b"hello"),
                    "abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0="
                );
                assert_eq!(
                    auth_cloned.sign(b"world"),
                    "abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ="
                );
            }));
        }
        {
            let auth_cloned = auth.clone();
            threads.push(std::thread::spawn(move || {
                assert_eq!(
                    auth_cloned.sign(b"-test"),
                    "abcdefghklmnopq:vYKRLUoXRlNHfpMEQeewG0zylaw="
                );
                assert_eq!(
                    auth_cloned.sign(b"ba#a-"),
                    "abcdefghklmnopq:2d_Yr6H1GdTKg3RvMtpHOhi047M="
                );
            }));
        }
        for thread in threads.into_iter() {
            thread.join().unwrap();
        }
    }

    #[test]
    fn test_sign_data() {
        let auth = get_auth();
        let mut threads = Vec::new();
        {
            let auth_cloned = auth.clone();
            threads.push(std::thread::spawn(move || {
                assert_eq!(
                    auth_cloned.sign_with_data(b"hello"),
                    "abcdefghklmnopq:BZYt5uVRy1RVt5ZTXbaIt2ROVMA=:aGVsbG8="
                );
                assert_eq!(
                    auth_cloned.sign_with_data(b"world"),
                    "abcdefghklmnopq:Wpe04qzPphiSZb1u6I0nFn6KpZg=:d29ybGQ="
                );
            }));
        }
        {
            let auth_cloned = auth.clone();
            threads.push(std::thread::spawn(move || {
                assert_eq!(
                    auth_cloned.sign_with_data(b"-test"),
                    "abcdefghklmnopq:HlxenSSP_6BbaYNzx1fyeyw8v1Y=:LXRlc3Q="
                );
                assert_eq!(
                    auth_cloned.sign_with_data(b"ba#a-"),
                    "abcdefghklmnopq:kwzeJrFziPDMO4jv3DKVLDyqud0=:YmEjYS0="
                );
            }));
        }
        for thread in threads.into_iter() {
            thread.join().unwrap();
        }
    }

    #[test]
    fn test_sign_request_v1() {
        let auth = get_auth();
        assert_eq!(
            auth.sign_request_v1(
                "http://upload.qiniup.com/",
                None::<&str>,
                Some(b"{\"name\":\"test\"}")
            )
            .unwrap(),
            auth.sign(b"/\n")
        );
        assert_eq!(
            auth.sign_request_v1(
                "http://upload.qiniup.com/",
                Some("application/json"),
                Some(b"{\"name\":\"test\"}")
            )
            .unwrap(),
            auth.sign(b"/\n")
        );
        assert_eq!(
            auth.sign_request_v1(
                "http://upload.qiniup.com/",
                Some("application/x-www-form-urlencoded"),
                Some(b"name=test&language=go")
            )
            .unwrap(),
            auth.sign(b"/\nname=test&language=go")
        );
        assert_eq!(
            auth.sign_request_v1(
                "http://upload.qiniup.com/?v=2",
                Some("application/x-www-form-urlencoded"),
                Some(b"name=test&language=go")
            )
            .unwrap(),
            auth.sign(b"/?v=2\nname=test&language=go")
        );
        assert_eq!(
            auth.sign_request_v1(
                "http://upload.qiniup.com/find/sdk?v=2",
                Some("application/x-www-form-urlencoded"),
                Some(b"name=test&language=go")
            )
            .unwrap(),
            auth.sign(b"/find/sdk?v=2\nname=test&language=go")
        );
    }

    #[test]
    fn test_sign_request_v2() {
        let auth = get_auth();
        assert_eq!(
            auth.sign_request_v2(
                Method::GET,
                "http://upload.qiniup.com/",
                Some("application/json"),
                Some(b"{\"name\":\"test\"}")
            )
            .unwrap(),
            auth.sign(b"GET /\nHost: upload.qiniup.com\nContent-Type: application/json\n\n{\"name\":\"test\"}")
        );
        assert_eq!(
            auth.sign_request_v2(
                Method::GET,
                "http://upload.qiniup.com/",
                None::<&str>,
                Some(b"{\"name\":\"test\"}")
            )
            .unwrap(),
            auth.sign(b"GET /\nHost: upload.qiniup.com\n\n")
        );
        assert_eq!(
            auth.sign_request_v2(
                Method::POST,
                "http://upload.qiniup.com/",
                Some("application/json"),
                Some(b"{\"name\":\"test\"}")
            )
            .unwrap(),
            auth.sign(b"POST /\nHost: upload.qiniup.com\nContent-Type: application/json\n\n{\"name\":\"test\"}")
        );
        assert_eq!(
            auth.sign_request_v2(
                Method::GET,
                "http://upload.qiniup.com/",
                Some("application/x-www-form-urlencoded"),
                Some(b"name=test&language=go")
            )
            .unwrap(),
            auth.sign(b"GET /\nHost: upload.qiniup.com\nContent-Type: application/x-www-form-urlencoded\n\nname=test&language=go")
        );
        assert_eq!(
            auth.sign_request_v2(
                Method::GET,
                "http://upload.qiniup.com/?v=2",
                Some("application/x-www-form-urlencoded"),
                Some(b"name=test&language=go")
            )
            .unwrap(),
            auth.sign(b"GET /?v=2\nHost: upload.qiniup.com\nContent-Type: application/x-www-form-urlencoded\n\nname=test&language=go")
        );
        assert_eq!(
            auth.sign_request_v2(
                Method::GET,
                "http://upload.qiniup.com/find/sdk?v=2",
                Some("application/x-www-form-urlencoded"),
                Some(b"name=test&language=go")
            )
            .unwrap(),
            auth.sign(b"GET /find/sdk?v=2\nHost: upload.qiniup.com\nContent-Type: application/x-www-form-urlencoded\n\nname=test&language=go")
        );
    }

    #[test]
    fn test_is_valid_request() {
        let auth = get_auth();

        let json_body: &[u8] = b"{\"name\":\"test\"}";
        let form_body: &[u8] = b"name=test&language=go";
        assert!(auth.is_valid_request(
            &http::Request::builder()
                .uri("http://upload.qiniup.com/")
                .header(
                    "Authorization",
                    auth.authorization_v1_for_request(
                        "http://upload.qiniup.com/",
                        None::<&str>,
                        None
                    )
                    .unwrap()
                )
                .body(json_body)
                .unwrap()
        ));
        assert!(auth.is_valid_request(
            &http::Request::builder()
                .uri("http://upload.qiniup.com/")
                .header(
                    "Authorization",
                    auth.authorization_v1_for_request(
                        "http://upload.qiniup.com/",
                        None::<&str>,
                        None
                    )
                    .unwrap()
                )
                .header("Content-Type", "application/json")
                .body(json_body)
                .unwrap()
        ));
        assert!(auth.is_valid_request(
            &http::Request::builder()
                .uri("http://upload.qiniup.com/find/sdk?v=2")
                .header(
                    "Authorization",
                    auth.authorization_v1_for_request(
                        "http://upload.qiniup.com/find/sdk?v=2",
                        Some("application/x-www-form-urlencoded"),
                        Some(b"name=test&language=go")
                    )
                    .unwrap()
                )
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(form_body)
                .unwrap()
        ));
    }

    #[test]
    fn test_sign_download_url_with_deadline() {
        let auth = get_auth();
        assert_eq!(
            auth.sign_download_url_with_deadline(
                Url::parse("http://www.qiniu.com/?go=1").unwrap(),
                time::SystemTime::UNIX_EPOCH + time::Duration::from_secs(1_234_567_890 + 3600),
                false).unwrap(),
            "http://www.qiniu.com/?go=1&e=1234571490&token=abcdefghklmnopq:KjQtlGAkEOhSwtFjJfYtYa2-reE=",
        );
        assert_eq!(
            auth.sign_download_url_with_deadline(
                Url::parse("http://www.qiniu.com/?go=1").unwrap(),
                time::SystemTime::UNIX_EPOCH + time::Duration::from_secs(1_234_567_890 + 3600),
                true).unwrap(),
            "http://www.qiniu.com/?go=1&e=1234571490&token=abcdefghklmnopq:86uQeCB9GsFFvL2wA0mgBcOMsmk=",
        );
    }

    #[test]
    fn test_sign_download_url_with_lifetime() {
        let auth = get_real_auth();
        let keys = ["4k", "1m", "4m", "8m", "16m", "64m", "1g"];
        for key in keys.iter() {
            let url = auth
                .sign_download_url_with_lifetime(
                    Url::parse(&format!(
                        "http://z1-bucket.kodo-test.qiniu-solutions.com/{}",
                        key
                    ))
                    .unwrap(),
                    time::Duration::from_secs(30),
                    false,
                )
                .unwrap();
            assert_eq!(http_utils::head(&url).unwrap().status(), StatusCode::OK);
        }
    }

    fn get_auth() -> Auth {
        Auth::new("abcdefghklmnopq", "1234567890")
    }

    fn get_real_auth() -> Auth {
        use super::super::variables;

        let vars = variables::load_variables();
        Auth::new(vars.access_key(), vars.secret_key().as_bytes())
    }
}
