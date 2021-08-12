#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![deny(
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    explicit_outlives_requirements,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    non_ascii_idents,
    indirect_structural_match,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unstable_features,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications
)]

use hmac::{Hmac, Mac, NewMac};
use http::{
    header::{HeaderMap, CONTENT_TYPE},
    method::Method,
    HeaderValue,
};
use mime::{APPLICATION_OCTET_STREAM, APPLICATION_WWW_FORM_URLENCODED};
use once_cell::sync::Lazy;
use qiniu_utils::base64;
use sha1::Sha1;
use std::{
    any::Any,
    borrow::Cow,
    collections::VecDeque,
    convert::TryFrom,
    env,
    ffi::OsStr,
    fmt::{self, Debug},
    io::{Error, ErrorKind, Result},
    sync::{Arc, RwLock},
    time::Duration,
};
pub use url::{ParseError as UrlParseError, Url};

pub mod preclude {
    pub use super::CredentialProvider;
}

/// 认证信息
///
/// 返回认证信息的 AccessKey 和 SecretKey
#[derive(Clone, Debug)]
pub struct Credential<'a> {
    access_key: Cow<'a, str>,
    secret_key: Cow<'a, str>,
}

impl<'a> Credential<'a> {
    /// 创建认证信息
    #[inline]
    pub fn new(access_key: impl Into<Cow<'a, str>>, secret_key: impl Into<Cow<'a, str>>) -> Self {
        Self {
            access_key: access_key.into(),
            secret_key: secret_key.into(),
        }
    }

    /// 获取认证信息的 AccessKey
    #[inline]
    pub fn access_key(&self) -> &str {
        self.access_key.as_ref()
    }

    /// 获取认证信息的 SecretKey
    #[inline]
    pub fn secret_key(&self) -> &str {
        self.secret_key.as_ref()
    }

    /// 同时返回认证信息的 AccessKey 和 SecretKey
    #[inline]
    pub fn into_pair(self) -> (Cow<'a, str>, Cow<'a, str>) {
        (self.access_key, self.secret_key)
    }

    /// 修改认证信息的 AccessKey
    #[inline]
    pub fn access_key_mut(&mut self) -> &mut Cow<'a, str> {
        &mut self.access_key
    }
    /// 修改认证信息的 SecretKey
    #[inline]
    pub fn secret_key_mut(&mut self) -> &mut Cow<'a, str> {
        &mut self.secret_key
    }
}

impl Credential<'_> {
    /// 使用七牛签名算法对数据进行签名
    ///
    /// 参考[管理凭证的签名算法文档](https://developer.qiniu.com/kodo/manual/1201/access-token)
    pub fn sign(&self, data: &[u8]) -> String {
        self.access_key.to_owned().into_owned()
            + ":"
            + &base64ed_hmac_digest(self.secret_key.as_ref(), data)
    }

    /// 使用七牛签名算法对数据进行签名，并同时给出签名和原数据
    ///
    /// 参考[上传凭证的签名算法文档](https://developer.qiniu.com/kodo/manual/1208/upload-token)
    pub fn sign_with_data(&self, data: &[u8]) -> String {
        let encoded_data = base64::urlsafe(data);
        self.sign(encoded_data.as_bytes()) + ":" + &encoded_data
    }

    /// 使用七牛签名算法 V1 对 HTTP 请求进行签名，返回 Authorization 的值
    pub fn authorization_v1_for_request(
        &self,
        url: &Url,
        content_type: &HeaderValue,
        body: &[u8],
    ) -> String {
        let authorization_token = sign_request_v1(self, url, content_type, body);
        "QBox ".to_owned() + &authorization_token
    }

    /// 使用七牛签名算法 V2 对 HTTP 请求进行签名，返回 Authorization 的值
    pub fn authorization_v2_for_request(
        &self,
        method: Method,
        url: &Url,
        headers: &HeaderMap,
        body: &[u8],
    ) -> String {
        let authorization_token = sign_request_v2(self, method, url, headers, body);
        "Qiniu ".to_owned() + &authorization_token
    }

    /// 对对象的下载 URL 签名，可以生成私有存储空间的下载地址
    pub fn sign_download_url(&self, url: &mut Url, deadline: Duration) {
        let mut to_sign = url.to_string();

        if to_sign.contains('?') {
            to_sign.push_str("&e=");
        } else {
            to_sign.push_str("?e=");
        }

        let deadline = deadline.as_secs().to_string();
        to_sign.push_str(&deadline);
        let mut query_pairs = url.query_pairs_mut();
        query_pairs.append_pair("e", &deadline);
        query_pairs.append_pair("token", &self.sign(to_sign.as_bytes()));
    }
}

fn sign_request_v1(
    cred: &Credential,
    url: &Url,
    content_type: &HeaderValue,
    body: &[u8],
) -> String {
    let mut data_to_sign = Vec::with_capacity(1024);
    data_to_sign.extend_from_slice(url.path().as_bytes());
    if let Some(query) = url.query() {
        if !query.is_empty() {
            data_to_sign.extend_from_slice(b"?");
            data_to_sign.extend_from_slice(query.as_bytes());
        }
    }
    data_to_sign.extend_from_slice(b"\n");
    if !content_type.is_empty() && !body.is_empty() && will_push_body_v1(content_type) {
        data_to_sign.extend_from_slice(body);
    }
    cred.sign(&data_to_sign)
}

fn sign_request_v2(
    cred: &Credential,
    method: Method,
    url: &Url,
    headers: &HeaderMap,
    body: &[u8],
) -> String {
    let mut data_to_sign = Vec::with_capacity(1024);
    data_to_sign.extend_from_slice(method.as_str().as_bytes());
    data_to_sign.extend_from_slice(b" ");
    data_to_sign.extend_from_slice(url.path().as_bytes());
    if let Some(query) = url.query() {
        if !query.is_empty() {
            data_to_sign.extend_from_slice(b"?");
            data_to_sign.extend_from_slice(query.as_bytes());
        }
    }
    data_to_sign.extend_from_slice(b"\nHost: ");
    data_to_sign.extend_from_slice(
        url.host_str()
            .expect("Host must be existed in URL")
            .as_bytes(),
    );
    if let Some(port) = url.port() {
        data_to_sign.extend_from_slice(b":");
        data_to_sign.extend_from_slice(port.to_string().as_bytes());
    }
    data_to_sign.extend_from_slice(b"\n");

    if let Some(content_type) = headers.get(CONTENT_TYPE) {
        data_to_sign.extend_from_slice(b"Content-Type: ");
        data_to_sign.extend_from_slice(content_type.as_bytes());
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
    return cred.sign(&data_to_sign);

    fn sign_data_for_x_qiniu_headers(data_to_sign: &mut Vec<u8>, headers: &HeaderMap) {
        let mut x_qiniu_headers = headers
            .iter()
            .filter(|(key, _)| key.as_str().len() > "X-Qiniu-".len())
            .filter(|(key, _)| key.as_str().starts_with("X-Qiniu-"))
            .map(|(key, value)| (key.as_str(), value.as_bytes()))
            .collect::<Vec<_>>();
        if x_qiniu_headers.is_empty() {
            return;
        }
        x_qiniu_headers.sort_unstable();
        for (header_key, header_value) in x_qiniu_headers {
            data_to_sign.extend_from_slice(header_key.as_bytes());
            data_to_sign.extend_from_slice(b": ");
            data_to_sign.extend_from_slice(header_value);
            data_to_sign.extend_from_slice(b"\n");
        }
    }
}

fn base64ed_hmac_digest(secret_key: &str, data: &[u8]) -> String {
    let mut hmac = Hmac::<Sha1>::new_from_slice(secret_key.as_bytes()).unwrap();
    hmac.update(data);
    base64::urlsafe(&hmac.finalize().into_bytes())
}

#[inline]
fn will_push_body_v1(content_type: &HeaderValue) -> bool {
    APPLICATION_WWW_FORM_URLENCODED.as_ref() == content_type
}

#[inline]
fn will_push_body_v2(content_type: &HeaderValue) -> bool {
    APPLICATION_OCTET_STREAM.as_ref() != content_type
}

#[cfg(feature = "async")]
use std::{future::Future, pin::Pin};

#[cfg(feature = "async")]
type AsyncResult<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + 'a + Send>>;

/// 认证信息提供者
///
/// 为认证信息提供者的实现提供接口支持
pub trait CredentialProvider: Any + Debug + Sync + Send {
    /// 返回七牛认证信息
    fn get(&self) -> Result<Credential>;

    /// 异步返回七牛认证信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get(&self) -> AsyncResult<Credential> {
        Box::pin(async move { self.get() })
    }

    fn as_any(&self) -> &dyn Any;
    fn as_credential_provider(&self) -> &dyn CredentialProvider;
}

/// 静态认证信息提供者，包含一个静态的 AccessKey 和 SecretKey，一旦创建则不可修改
#[derive(Clone, Eq, PartialEq)]
pub struct StaticCredentialProvider {
    access_key: Cow<'static, str>,
    secret_key: Cow<'static, str>,
}

impl StaticCredentialProvider {
    /// 构建一个静态认证信息提供者，只需要传入静态的 AccessKey 和 SecretKey 即可
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

impl CredentialProvider for StaticCredentialProvider {
    #[inline]
    fn get(&self) -> Result<Credential> {
        Ok(Credential::new(
            Cow::Borrowed(self.access_key.as_ref()),
            Cow::Borrowed(self.secret_key.as_ref()),
        ))
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_credential_provider(&self) -> &dyn CredentialProvider {
        self
    }
}

impl TryFrom<&dyn CredentialProvider> for StaticCredentialProvider {
    type Error = Error;
    fn try_from(cred: &dyn CredentialProvider) -> Result<Self> {
        let value = cred.get()?;
        Ok(StaticCredentialProvider::new(
            value.access_key.into_owned(),
            value.secret_key.into_owned(),
        ))
    }
}

impl AsRef<dyn CredentialProvider> for StaticCredentialProvider {
    #[inline]
    fn as_ref(&self) -> &dyn CredentialProvider {
        self.as_credential_provider()
    }
}

impl Debug for StaticCredentialProvider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!(
            "StaticCredentialProvider {{ access_key: {:?}, secret_key: CENSORED }}",
            self.access_key,
        ))
    }
}

/// 全局认证信息提供者，可以将认证信息配置在全局变量中。任何全局认证信息提供者实例都可以设置和访问全局认证信息。
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct GlobalCredentialProvider;

static GLOBAL_CREDENTIAL: Lazy<RwLock<Option<Credential<'static>>>> =
    Lazy::new(|| RwLock::new(None));

impl GlobalCredentialProvider {
    /// 配置全局认证信息
    pub fn setup(
        access_key: impl Into<Cow<'static, str>>,
        secret_key: impl Into<Cow<'static, str>>,
    ) {
        let mut global_credential = GLOBAL_CREDENTIAL.write().unwrap();
        *global_credential = Some(Credential::new(access_key, secret_key));
    }

    /// 清空全局认证信息
    pub fn clear() {
        let mut global_credential = GLOBAL_CREDENTIAL.write().unwrap();
        *global_credential = None;
    }
}

impl CredentialProvider for GlobalCredentialProvider {
    fn get(&self) -> Result<Credential> {
        if let Some(credential) = GLOBAL_CREDENTIAL.read().unwrap().as_ref() {
            Ok(credential.clone())
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "GlobalCredentialProvider is not setuped, please call GlobalCredentialProvider::setup() to do it",
            ))
        }
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_credential_provider(&self) -> &dyn CredentialProvider {
        self
    }
}

impl Debug for GlobalCredentialProvider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(credential) = GLOBAL_CREDENTIAL.read().unwrap().as_ref() {
            f.write_fmt(format_args!(
                "GlobalCredentialProvider {{ access_key: {:?}, secret_key: CENSORED }}",
                credential.access_key,
            ))
        } else {
            write!(f, "GlobalCredentialProvider {{ None }}")
        }
    }
}

/// 环境变量认证信息提供者，可以将认证信息配置在环境变量中。
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct EnvCredentialProvider;

/// 设置七牛 AccessKey 的环境变量
pub const QINIU_ACCESS_KEY_ENV_KEY: &str = "QINIU_ACCESS_KEY";
/// 设置七牛 SecretKey 的环境变量
pub const QINIU_SECRET_KEY_ENV_KEY: &str = "QINIU_SECRET_KEY";

impl EnvCredentialProvider {
    /// 配置环境变量认证信息提供者
    #[inline]
    pub fn setup(access_key: impl AsRef<OsStr>, secret_key: impl AsRef<OsStr>) {
        env::set_var(QINIU_ACCESS_KEY_ENV_KEY, access_key);
        env::set_var(QINIU_SECRET_KEY_ENV_KEY, secret_key);
    }
}

impl CredentialProvider for EnvCredentialProvider {
    fn get(&self) -> Result<Credential> {
        match (
            env::var(QINIU_ACCESS_KEY_ENV_KEY),
            env::var(QINIU_SECRET_KEY_ENV_KEY),
        ) {
            (Ok(access_key), Ok(secret_key))
                if !access_key.is_empty() && !secret_key.is_empty() =>
            {
                Ok(Credential::new(access_key, secret_key))
            }
            _ => {
                static ERROR_MESSAGE: Lazy<String> = Lazy::new(|| {
                    format!("EnvCredentialProvider is not setuped, please call EnvCredentialProvider::setup() to do it, or set environment variable `{}` and `{}`", QINIU_ACCESS_KEY_ENV_KEY, QINIU_SECRET_KEY_ENV_KEY)
                });
                Err(Error::new(ErrorKind::Other, ERROR_MESSAGE.as_str()))
            }
        }
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_credential_provider(&self) -> &dyn CredentialProvider {
        self
    }
}

impl Debug for EnvCredentialProvider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (
            env::var_os(QINIU_ACCESS_KEY_ENV_KEY),
            env::var_os(QINIU_SECRET_KEY_ENV_KEY),
        ) {
            (Some(access_key), Some(_)) => f.write_fmt(format_args!(
                "EnvCredentialProvider {{ access_key: {:?}, secret_key: CENSORED }}",
                access_key,
            )),
            _ => write!(f, "EnvCredentialProvider {{ None }}"),
        }
    }
}

/// 认证信息串提供者
///
/// 将多个认证信息串联，遍历并找寻第一个可用认证信息
#[derive(Clone, Debug)]
pub struct ChainCredentialsProvider {
    credentials: Arc<[Box<dyn CredentialProvider>]>,
}

impl CredentialProvider for ChainCredentialsProvider {
    fn get(&self) -> Result<Credential> {
        if let Some(credential) = self.credentials.iter().find_map(|c| c.get().ok()) {
            Ok(credential)
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "All credentials are failed to get",
            ))
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get(&self) -> AsyncResult<Credential> {
        Box::pin(async move {
            for provider in self.credentials.iter() {
                if let Ok(credential) = provider.async_get().await {
                    return Ok(credential);
                }
            }
            Err(Error::new(
                ErrorKind::Other,
                "All credentials are failed to get",
            ))
        })
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_credential_provider(&self) -> &dyn CredentialProvider {
        self
    }
}

impl Default for ChainCredentialsProvider {
    #[inline]
    fn default() -> Self {
        ChainCredentialsProviderBuilder::default()
            .append_credential(Box::new(GlobalCredentialProvider))
            .append_credential(Box::new(EnvCredentialProvider))
            .build()
    }
}

/// 串联认证信息构建器
///
/// 接受多个认证信息提供者并将他们串联成串联认证信息
#[derive(Default)]
pub struct ChainCredentialsProviderBuilder {
    credentials: VecDeque<Box<dyn CredentialProvider>>,
}

impl ChainCredentialsProviderBuilder {
    /// 构建新的串联认证信息构建器
    #[inline]
    pub fn new() -> ChainCredentialsProviderBuilder {
        Default::default()
    }

    /// 将认证信息提供者推送到认证串末端
    #[inline]
    pub fn append_credential(mut self, credential: Box<dyn CredentialProvider>) -> Self {
        self.credentials.push_back(credential);
        self
    }

    /// 将认证信息提供者推送到认证串顶端
    #[inline]
    pub fn prepend_credential(mut self, credential: Box<dyn CredentialProvider>) -> Self {
        self.credentials.push_front(credential);
        self
    }

    /// 串联认证信息
    #[inline]
    pub fn build(self) -> ChainCredentialsProvider {
        ChainCredentialsProvider {
            credentials: self.credentials.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std as _;
    use std::{boxed::Box, error::Error, result::Result, sync::Arc, thread, time::Duration};

    #[test]
    fn test_sign() -> Result<(), Box<dyn Error>> {
        let credential: Arc<dyn CredentialProvider> = Arc::new(get_static_credential());
        let mut threads = Vec::new();
        {
            let credential = credential.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(
                    credential.get().unwrap().sign(b"hello"),
                    "abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0="
                );
                assert_eq!(
                    credential.get().unwrap().sign(b"world"),
                    "abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ="
                );
            }));
        }
        {
            let credential = credential.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(
                    credential.get().unwrap().sign(b"-test"),
                    "abcdefghklmnopq:vYKRLUoXRlNHfpMEQeewG0zylaw="
                );
                assert_eq!(
                    credential.get().unwrap().sign(b"ba#a-"),
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
        let credential: Arc<dyn CredentialProvider> = Arc::new(get_static_credential());
        let mut threads = Vec::new();
        {
            let credential = credential.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(
                    credential.get().unwrap().sign_with_data(b"hello"),
                    "abcdefghklmnopq:BZYt5uVRy1RVt5ZTXbaIt2ROVMA=:aGVsbG8="
                );
                assert_eq!(
                    credential.get().unwrap().sign_with_data(b"world"),
                    "abcdefghklmnopq:Wpe04qzPphiSZb1u6I0nFn6KpZg=:d29ybGQ="
                );
            }));
        }
        {
            let credential = credential.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(
                    credential.get().unwrap().sign_with_data(b"-test"),
                    "abcdefghklmnopq:HlxenSSP_6BbaYNzx1fyeyw8v1Y=:LXRlc3Q="
                );
                assert_eq!(
                    credential.get().unwrap().sign_with_data(b"ba#a-"),
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
        let credential = get_static_credential();
        assert_eq!(
            credential.get().unwrap().authorization_v1_for_request(
                &Url::parse("http://upload.qiniup.com/")?,
                "",
                b"{\"name\":\"test\"}"
            ),
            "QBox ".to_owned() + &credential.get().unwrap().sign(b"/\n")
        );
        assert_eq!(
            credential.get().unwrap().authorization_v1_for_request(
                &Url::parse("http://upload.qiniup.com/")?,
                mime::JSON_MIME,
                b"{\"name\":\"test\"}"
            ),
            "QBox ".to_owned() + &credential.get().unwrap().sign(b"/\n")
        );
        assert_eq!(
            credential.get().unwrap().authorization_v1_for_request(
                &Url::parse("http://upload.qiniup.com/")?,
                APPLICATION_WWW_FORM_URLENCODED,
                b"name=test&language=go"
            ),
            "QBox ".to_owned() + &credential.get().unwrap().sign(b"/\nname=test&language=go")
        );
        assert_eq!(
            credential.get().unwrap().authorization_v1_for_request(
                &Url::parse("http://upload.qiniup.com/?v=2")?,
                APPLICATION_WWW_FORM_URLENCODED,
                b"name=test&language=go"
            ),
            "QBox ".to_owned()
                + &credential
                    .get()
                    .unwrap()
                    .sign(b"/?v=2\nname=test&language=go")
        );
        assert_eq!(
            credential.get().unwrap().authorization_v1_for_request(
                &Url::parse("http://upload.qiniup.com/find/sdk?v=2")?,
                APPLICATION_WWW_FORM_URLENCODED,
                b"name=test&language=go"
            ),
            "QBox ".to_owned()
                + &credential
                    .get()
                    .unwrap()
                    .sign(b"/find/sdk?v=2\nname=test&language=go")
        );
        Ok(())
    }

    #[test]
    fn test_authorization_v2() -> Result<(), Box<dyn Error>> {
        let credential = get_global_credential();
        let empty_headers = {
            let mut headers = HeaderMap::new();
            headers.insert("X-Qbox-Meta".into(), "value".into());
            headers
        };
        let json_headers = {
            let mut headers = HeaderMap::new();
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
            let mut headers = HeaderMap::new();
            headers.insert(
                "Content-Type".into(),
                APPLICATION_WWW_FORM_URLENCODED.into(),
            );
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
            credential.get().unwrap().authorization_v2_for_request(
                Method::GET,
                &Url::parse("http://upload.qiniup.com/")?,
                &json_headers,
                b"{\"name\":\"test\"}"
            ),
            "Qiniu ".to_owned()
                + &credential.get().unwrap().sign(
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
            credential.get().unwrap().authorization_v2_for_request(
                Method::GET,
                &Url::parse("http://upload.qiniup.com/")?,
                &empty_headers,
                b"{\"name\":\"test\"}"
            ),
            "Qiniu ".to_owned()
                + &credential
                    .get()
                    .unwrap()
                    .sign(concat!("GET /\n", "Host: upload.qiniup.com\n\n").as_bytes())
        );
        assert_eq!(
            credential.get().unwrap().authorization_v2_for_request(
                Method::POST,
                &Url::parse("http://upload.qiniup.com/")?,
                &json_headers,
                b"{\"name\":\"test\"}"
            ),
            "Qiniu ".to_owned()
                + &credential.get().unwrap().sign(
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
            credential.get().unwrap().authorization_v2_for_request(
                Method::GET,
                &Url::parse("http://upload.qiniup.com/")?,
                &form_headers,
                b"name=test&language=go"
            ),
            "Qiniu ".to_owned()
                + &credential.get().unwrap().sign(
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
            credential.get().unwrap().authorization_v2_for_request(
                Method::GET,
                &Url::parse("http://upload.qiniup.com/?v=2")?,
                &form_headers,
                b"name=test&language=go"
            ),
            "Qiniu ".to_owned()
                + &credential.get().unwrap().sign(
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
            credential.get().unwrap().authorization_v2_for_request(
                Method::GET,
                &Url::parse("http://upload.qiniup.com/find/sdk?v=2")?,
                &form_headers,
                b"name=test&language=go"
            ),
            "Qiniu ".to_owned()
                + &credential.get().unwrap().sign(
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
        let credential = get_env_credential();
        let mut url = Url::parse("http://www.qiniu.com/?go=1")?;
        credential
            .get()
            .unwrap()
            .sign_download_url(&mut url, Duration::from_secs(1_234_567_890 + 3600));
        assert_eq!(
                url.to_string(),
                "http://www.qiniu.com/?go=1&e=1234571490&token=abcdefghklmnopq%3AKjQtlGAkEOhSwtFjJfYtYa2-reE%3D",
            );
        Ok(())
    }

    #[test]
    fn test_chain_credentials() -> Result<(), Box<dyn Error>> {
        GlobalCredentialProvider::clear();
        let chain_credentials = ChainCredentialsProvider::default();
        env::set_var(QINIU_ACCESS_KEY_ENV_KEY, "TEST2");
        env::set_var(QINIU_SECRET_KEY_ENV_KEY, "test2");
        {
            let cred = chain_credentials.get()?;
            assert_eq!(cred.access_key(), "TEST2");
        }
        GlobalCredentialProvider::setup("TEST1", "test1");
        {
            let cred = chain_credentials.get()?;
            assert_eq!(cred.access_key(), "TEST1");
        }
        Ok(())
    }

    fn get_static_credential() -> impl CredentialProvider {
        StaticCredentialProvider::new("abcdefghklmnopq", "1234567890")
    }

    fn get_global_credential() -> impl CredentialProvider {
        GlobalCredentialProvider::setup("abcdefghklmnopq", "1234567890");
        GlobalCredentialProvider
    }

    fn get_env_credential() -> impl CredentialProvider {
        env::set_var(QINIU_ACCESS_KEY_ENV_KEY, "abcdefghklmnopq");
        env::set_var(QINIU_SECRET_KEY_ENV_KEY, "1234567890");
        EnvCredentialProvider
    }

    #[cfg(feature = "async")]
    mod async_test {
        use super::*;

        #[async_std::test]
        async fn test_async_sign_download_url() -> Result<(), Box<dyn Error>> {
            let credential = get_env_credential();
            let mut url = Url::parse("http://www.qiniu.com/?go=1")?;
            credential
                .async_get()
                .await
                .unwrap()
                .sign_download_url(&mut url, Duration::from_secs(1_234_567_890 + 3600));
            assert_eq!(
                url.into_string(),
                "http://www.qiniu.com/?go=1&e=1234571490&token=abcdefghklmnopq%3AKjQtlGAkEOhSwtFjJfYtYa2-reE%3D",
            );
            Ok(())
        }

        #[async_std::test]
        async fn test_async_chain_credentials() -> Result<(), Box<dyn Error>> {
            GlobalCredentialProvider::clear();
            let chain_credentials = ChainCredentialsProvider::default();
            env::set_var(QINIU_ACCESS_KEY_ENV_KEY, "TEST2");
            env::set_var(QINIU_SECRET_KEY_ENV_KEY, "test2");
            {
                let cred = chain_credentials.async_get().await?;
                assert_eq!(cred.access_key(), "TEST2");
            }
            GlobalCredentialProvider::setup("TEST1", "test1");
            {
                let cred = chain_credentials.async_get().await?;
                assert_eq!(cred.access_key(), "TEST1");
            }
            Ok(())
        }
    }
}
