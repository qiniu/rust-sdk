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
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications
)]

use hmac::{Hmac, Mac, NewMac};
use http::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    method::Method,
    uri::Uri,
};
use mime::{APPLICATION_OCTET_STREAM, APPLICATION_WWW_FORM_URLENCODED};
use once_cell::sync::Lazy;
use qiniu_utils::base64;
use sha1::Sha1;
use std::{
    any::Any,
    collections::VecDeque,
    env,
    fmt::{self, Debug},
    io::{copy, Cursor, Error, ErrorKind, Read, Result},
    ops::{Deref, DerefMut},
    sync::RwLock,
    time::Duration,
};

mod header_name;
use header_name::make_header_name;

mod key;
pub use key::{AccessKey, SecretKey};

pub mod preclude {
    pub use super::CredentialProvider;
}

/// 认证信息
///
/// 返回认证信息的 AccessKey 和 SecretKey
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credential {
    access_key: AccessKey,
    secret_key: SecretKey,
}

impl Credential {
    /// 创建认证信息
    #[inline]
    pub fn new(access_key: impl Into<AccessKey>, secret_key: impl Into<SecretKey>) -> Self {
        Self {
            access_key: access_key.into(),
            secret_key: secret_key.into(),
        }
    }

    /// 获取认证信息的 AccessKey
    #[inline]
    pub fn access_key(&self) -> &AccessKey {
        &self.access_key
    }

    /// 获取认证信息的 SecretKey
    #[inline]
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    /// 同时返回认证信息的 AccessKey 和 SecretKey
    #[inline]
    pub fn into_pair(self) -> (AccessKey, SecretKey) {
        (self.access_key, self.secret_key)
    }
}

impl Credential {
    /// 使用七牛签名算法对数据进行签名
    ///
    /// 参考[管理凭证的签名算法文档](https://developer.qiniu.com/kodo/manual/1201/access-token)
    #[inline]
    pub fn sign(&self, data: &[u8]) -> String {
        self.access_key.to_string() + ":" + &base64ed_hmac_digest(self.secret_key.as_ref(), data)
    }

    #[inline]
    pub fn sign_reader(&self, reader: &mut dyn Read) -> Result<String> {
        Ok(self.access_key.to_string()
            + ":"
            + &base64ed_hmac_digest_reader(self.secret_key.as_ref(), reader)?)
    }

    /// 使用七牛签名算法对数据进行签名，并同时给出签名和原数据
    ///
    /// 参考[上传凭证的签名算法文档](https://developer.qiniu.com/kodo/manual/1208/upload-token)
    #[inline]
    pub fn sign_with_data(&self, data: &[u8]) -> String {
        let encoded_data = base64::urlsafe(data);
        self.sign(encoded_data.as_bytes()) + ":" + &encoded_data
    }

    /// 使用七牛签名算法 V1 对 HTTP 请求进行签名，返回 Authorization 的值
    #[inline]
    pub fn authorization_v1_for_request(
        &self,
        url: &Uri,
        content_type: Option<&HeaderValue>,
        body: &[u8],
    ) -> String {
        let authorization_token = sign_request_v1(self, url, content_type, body);
        "QBox ".to_owned() + &authorization_token
    }

    #[inline]
    pub fn authorization_v1_for_request_with_body_reader(
        &self,
        url: &Uri,
        content_type: Option<&HeaderValue>,
        body: &mut dyn Read,
    ) -> Result<String> {
        let authorization_token = sign_request_v1_with_body_reader(self, url, content_type, body)?;
        Ok("QBox ".to_owned() + &authorization_token)
    }

    /// 使用七牛签名算法 V2 对 HTTP 请求进行签名，返回 Authorization 的值
    #[inline]
    pub fn authorization_v2_for_request(
        &self,
        method: &Method,
        url: &Uri,
        headers: &HeaderMap,
        body: &[u8],
    ) -> String {
        let authorization_token = sign_request_v2(self, method, url, headers, body);
        "Qiniu ".to_owned() + &authorization_token
    }

    #[inline]
    pub fn authorization_v2_for_request_with_body_reader(
        &self,
        method: &Method,
        url: &Uri,
        headers: &HeaderMap,
        body: &mut dyn Read,
    ) -> Result<String> {
        let authorization_token =
            sign_request_v2_with_body_reader(self, method, url, headers, body)?;
        Ok("Qiniu ".to_owned() + &authorization_token)
    }

    /// 对对象的下载 URL 签名，可以生成私有存储空间的下载地址
    #[inline]
    pub fn sign_download_url(&self, url: Uri, deadline: Duration) -> Uri {
        let deadline = deadline.as_secs().to_string();
        let to_sign = append_query_pairs_to_url(url, &[("e", &deadline)]);
        let signature = self.sign(to_sign.to_string().as_bytes());
        return append_query_pairs_to_url(to_sign, &[("token", &signature)]);

        #[inline]
        fn append_query_pairs_to_url(url: Uri, pairs: &[(&str, &str)]) -> Uri {
            let path_string = url.path().to_owned();
            let query_string = url.query().unwrap_or_default().to_owned();
            let mut serializer = form_urlencoded::Serializer::new(query_string);
            for (key, value) in pairs.iter() {
                serializer.append_pair(key, value);
            }
            let query_string = serializer.finish();
            let mut path_and_query = path_string;
            if !query_string.is_empty() {
                path_and_query.push('?');
                path_and_query.push_str(&query_string);
            }
            let parts = url.into_parts();
            let mut builder = Uri::builder();
            if let Some(scheme) = parts.scheme {
                builder = builder.scheme(scheme);
            }
            if let Some(authority) = parts.authority {
                builder = builder.authority(authority);
            }
            builder.path_and_query(&path_and_query).build().unwrap()
        }
    }
}

#[cfg(feature = "async")]
impl Credential {
    #[inline]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub async fn sign_async_reader(&self, reader: &mut (dyn AsyncRead + Unpin)) -> Result<String> {
        Ok(self.access_key.to_string()
            + ":"
            + &base64ed_hmac_digest_async_reader(self.secret_key.as_ref(), reader).await?)
    }

    #[inline]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub async fn authorization_v1_for_request_with_async_body_reader(
        &self,
        url: &Uri,
        content_type: Option<&HeaderValue>,
        body: &mut (dyn AsyncRead + Unpin),
    ) -> Result<String> {
        let authorization_token =
            sign_request_v1_with_async_body_reader(self, url, content_type, body).await?;
        Ok("QBox ".to_owned() + &authorization_token)
    }

    #[inline]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub async fn authorization_v2_for_request_with_async_body_reader(
        &self,
        method: &Method,
        url: &Uri,
        headers: &HeaderMap,
        body: &mut (dyn AsyncRead + Unpin),
    ) -> Result<String> {
        let authorization_token =
            sign_request_v2_with_async_body_reader(self, method, url, headers, body).await?;
        Ok("Qiniu ".to_owned() + &authorization_token)
    }
}

fn sign_request_v1(
    cred: &Credential,
    url: &Uri,
    content_type: Option<&HeaderValue>,
    body: &[u8],
) -> String {
    let mut data_to_sign = _sign_request_v1_without_body(url);
    if let Some(content_type) = content_type {
        if !body.is_empty() && will_push_body_v1(content_type) {
            data_to_sign.extend_from_slice(body);
        }
    }
    cred.sign(&data_to_sign)
}

fn sign_request_v1_with_body_reader(
    cred: &Credential,
    url: &Uri,
    content_type: Option<&HeaderValue>,
    body: &mut dyn Read,
) -> Result<String> {
    let data_to_sign = _sign_request_v1_without_body(url);
    if let Some(content_type) = content_type {
        if will_push_body_v1(content_type) {
            return cred.sign_reader(&mut Cursor::new(data_to_sign).chain(body));
        }
    }
    Ok(cred.sign(&data_to_sign))
}

#[inline]
fn _sign_request_v1_without_body(url: &Uri) -> Vec<u8> {
    let mut data_to_sign = Vec::with_capacity(1024);
    data_to_sign.extend_from_slice(url.path().as_bytes());
    if let Some(query) = url.query() {
        if !query.is_empty() {
            data_to_sign.extend_from_slice(b"?");
            data_to_sign.extend_from_slice(query.as_bytes());
        }
    }
    data_to_sign.extend_from_slice(b"\n");
    data_to_sign
}

fn sign_request_v2(
    cred: &Credential,
    method: &Method,
    url: &Uri,
    headers: &HeaderMap,
    body: &[u8],
) -> String {
    let mut data_to_sign = _sign_request_v2_without_body(method, url, headers);
    if let Some(content_type) = headers.get(CONTENT_TYPE) {
        if will_push_body_v2(content_type) {
            data_to_sign.extend_from_slice(body);
        }
    }
    cred.sign(&data_to_sign)
}

fn sign_request_v2_with_body_reader(
    cred: &Credential,
    method: &Method,
    url: &Uri,
    headers: &HeaderMap,
    body: &mut dyn Read,
) -> Result<String> {
    let data_to_sign = _sign_request_v2_without_body(method, url, headers);
    if let Some(content_type) = headers.get(CONTENT_TYPE) {
        if will_push_body_v2(content_type) {
            return cred.sign_reader(&mut Cursor::new(data_to_sign).chain(body));
        }
    }
    Ok(cred.sign(&data_to_sign))
}

fn _sign_request_v2_without_body(method: &Method, url: &Uri, headers: &HeaderMap) -> Vec<u8> {
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
    if let Some(host) = url.host() {
        data_to_sign.extend_from_slice(b"\nHost: ");
        data_to_sign.extend_from_slice(host.as_bytes());
    }
    if let Some(port) = url.port() {
        data_to_sign.extend_from_slice(b":");
        data_to_sign.extend_from_slice(port.to_string().as_bytes());
    }
    data_to_sign.extend_from_slice(b"\n");

    if let Some(content_type) = headers.get(CONTENT_TYPE) {
        data_to_sign.extend_from_slice(b"Content-Type: ");
        data_to_sign.extend_from_slice(content_type.as_bytes());
        data_to_sign.extend_from_slice(b"\n");
    }
    _sign_data_for_x_qiniu_headers(&mut data_to_sign, headers);
    data_to_sign.extend_from_slice(b"\n");
    data_to_sign
}

fn _sign_data_for_x_qiniu_headers(data_to_sign: &mut Vec<u8>, headers: &HeaderMap) {
    let mut x_qiniu_headers = headers
        .iter()
        .map(|(key, value)| (make_header_name(key.as_str().into()), value.as_bytes()))
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
        data_to_sign.extend_from_slice(header_value);
        data_to_sign.extend_from_slice(b"\n");
    }
}

#[inline]
fn base64ed_hmac_digest(secret_key: &str, data: &[u8]) -> String {
    let mut hmac = Hmac::<Sha1>::new_from_slice(secret_key.as_bytes()).unwrap();
    hmac.update(data);
    base64::urlsafe(&hmac.finalize().into_bytes())
}

#[inline]
fn base64ed_hmac_digest_reader(secret_key: &str, reader: &mut dyn Read) -> Result<String> {
    let mut hmac = Hmac::<Sha1>::new_from_slice(secret_key.as_bytes()).unwrap();
    copy(reader, &mut hmac)?;
    Ok(base64::urlsafe(&hmac.finalize().into_bytes()))
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
mod async_sign {
    use super::*;
    use futures_lite::{
        io::{copy, AsyncRead, AsyncReadExt, Cursor},
        AsyncWrite,
    };
    use hmac::digest::{
        generic_array::{ArrayLength, GenericArray},
        BlockInput, FixedOutput, Reset, Update,
    };
    use std::task::{Context, Poll};

    pub(super) async fn sign_request_v1_with_async_body_reader(
        cred: &Credential,
        url: &Uri,
        content_type: Option<&HeaderValue>,
        body: &mut (dyn AsyncRead + Unpin),
    ) -> Result<String> {
        let data_to_sign = _sign_request_v1_without_body(url);
        if let Some(content_type) = content_type {
            if will_push_body_v1(content_type) {
                return cred
                    .sign_async_reader(&mut Cursor::new(data_to_sign).chain(body))
                    .await;
            }
        }
        Ok(cred.sign(&data_to_sign))
    }

    pub(super) async fn sign_request_v2_with_async_body_reader(
        cred: &Credential,
        method: &Method,
        url: &Uri,
        headers: &HeaderMap,
        body: &mut (dyn AsyncRead + Unpin),
    ) -> Result<String> {
        let data_to_sign = _sign_request_v2_without_body(method, url, headers);
        if let Some(content_type) = headers.get(CONTENT_TYPE) {
            if will_push_body_v2(content_type) {
                return cred
                    .sign_async_reader(&mut Cursor::new(data_to_sign).chain(body))
                    .await;
            }
        }
        Ok(cred.sign(&data_to_sign))
    }

    #[inline]
    pub(super) async fn base64ed_hmac_digest_async_reader(
        secret_key: &str,
        reader: &mut (dyn AsyncRead + Unpin),
    ) -> Result<String> {
        let mut hmac =
            AsyncHmacWriter(Hmac::<Sha1>::new_from_slice(secret_key.as_bytes()).unwrap());
        copy(reader, &mut hmac).await?;
        return Ok(base64::urlsafe(&hmac.finalize()));

        #[derive(Clone)]
        struct AsyncHmacWriter<D>(Hmac<D>)
        where
            D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
            D::BlockSize: ArrayLength<u8>;

        impl<D> AsyncWrite for AsyncHmacWriter<D>
        where
            D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
            D::BlockSize: ArrayLength<u8>,
            D::OutputSize: ArrayLength<u8>,
        {
            #[inline]
            fn poll_write(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                buf: &[u8],
            ) -> Poll<Result<usize>> {
                unsafe { self.get_unchecked_mut() }.0.update(buf);
                Poll::Ready(Ok(buf.len()))
            }

            #[inline]
            fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
                Poll::Ready(Ok(()))
            }

            #[inline]
            fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
                Poll::Ready(Ok(()))
            }
        }

        impl<D> AsyncHmacWriter<D>
        where
            D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
            D::BlockSize: ArrayLength<u8>,
        {
            #[inline]
            fn finalize(self) -> GenericArray<u8, <D as FixedOutput>::OutputSize> {
                self.0.finalize().into_bytes()
            }
        }
    }
}

#[cfg(feature = "async")]
pub use futures_lite::AsyncRead;

#[cfg(feature = "async")]
use {
    async_sign::*,
    std::{future::Future, pin::Pin},
};

#[cfg(feature = "async")]
type AsyncResult<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + 'a + Send>>;

/// 认证信息提供者
///
/// 为认证信息提供者的实现提供接口支持
pub trait CredentialProvider: Any + Debug + Sync + Send {
    /// 返回七牛认证信息
    fn get(&self, opts: &GetOptions) -> Result<GotCredential>;

    /// 异步返回七牛认证信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get<'a>(&'a self, opts: &'a GetOptions) -> AsyncResult<'a, GotCredential> {
        Box::pin(async move { self.get(opts) })
    }

    fn as_any(&self) -> &dyn Any;
    fn as_credential_provider(&self) -> &dyn CredentialProvider;
}

#[derive(Clone, Debug, Default)]
pub struct GetOptions {}

#[derive(Debug)]
pub struct GotCredential(Credential);

impl From<GotCredential> for Credential {
    #[inline]
    fn from(result: GotCredential) -> Self {
        result.0
    }
}

impl From<Credential> for GotCredential {
    #[inline]
    fn from(credential: Credential) -> Self {
        Self(credential)
    }
}

impl GotCredential {
    #[inline]
    pub fn credential(&self) -> &Credential {
        &self.0
    }

    #[inline]
    pub fn credential_mut(&mut self) -> &mut Credential {
        &mut self.0
    }

    #[inline]
    pub fn into_credential(self) -> Credential {
        self.0
    }
}

impl Deref for GotCredential {
    type Target = Credential;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GotCredential {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// 静态认证信息提供者，包含一个静态的认证信息，一旦创建则不可修改
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticCredentialProvider {
    credential: Credential,
}

impl StaticCredentialProvider {
    /// 构建一个静态认证信息提供者，只需要传入静态的认证信息即可
    pub fn new(credential: Credential) -> Self {
        Self { credential }
    }
}

impl CredentialProvider for StaticCredentialProvider {
    #[inline]
    fn get(&self, _opts: &GetOptions) -> Result<GotCredential> {
        Ok(self.credential.to_owned().into())
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

/// 全局认证信息提供者，可以将认证信息配置在全局变量中。任何全局认证信息提供者实例都可以设置和访问全局认证信息。
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct GlobalCredentialProvider;

static GLOBAL_CREDENTIAL: Lazy<RwLock<Option<Credential>>> = Lazy::new(|| RwLock::new(None));

impl GlobalCredentialProvider {
    /// 配置全局认证信息
    #[inline]
    pub fn setup(credential: Credential) {
        let mut global_credential = GLOBAL_CREDENTIAL.write().unwrap();
        *global_credential = Some(credential);
    }

    /// 清空全局认证信息
    #[inline]
    pub fn clear() {
        let mut global_credential = GLOBAL_CREDENTIAL.write().unwrap();
        *global_credential = None;
    }
}

impl CredentialProvider for GlobalCredentialProvider {
    #[inline]
    fn get(&self, _opts: &GetOptions) -> Result<GotCredential> {
        if let Some(credential) = GLOBAL_CREDENTIAL.read().unwrap().as_ref() {
            Ok(credential.to_owned().into())
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
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = f.debug_struct("GlobalCredentialProvider");
        d.field("credential", &GLOBAL_CREDENTIAL.read().unwrap());
        d.finish()
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
    pub fn setup(credential: &Credential) {
        env::set_var(QINIU_ACCESS_KEY_ENV_KEY, credential.access_key().as_str());
        env::set_var(QINIU_SECRET_KEY_ENV_KEY, credential.secret_key().as_str());
    }
}

impl CredentialProvider for EnvCredentialProvider {
    fn get(&self, _opts: &GetOptions) -> Result<GotCredential> {
        match (
            env::var(QINIU_ACCESS_KEY_ENV_KEY),
            env::var(QINIU_SECRET_KEY_ENV_KEY),
        ) {
            (Ok(access_key), Ok(secret_key))
                if !access_key.is_empty() && !secret_key.is_empty() =>
            {
                Ok(Credential::new(access_key, secret_key).into())
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
        let mut d = f.debug_struct("EnvCredentialProvider");
        if let (Some(access_key), Some(secret_key)) = (
            env::var_os(QINIU_ACCESS_KEY_ENV_KEY),
            env::var_os(QINIU_SECRET_KEY_ENV_KEY),
        ) {
            d.field("access_key", &access_key)
                .field("secret_key", &secret_key);
        }
        d.finish()
    }
}

/// 认证信息串提供者
///
/// 将多个认证信息串联，遍历并找寻第一个可用认证信息
#[derive(Debug)]
pub struct ChainCredentialsProvider {
    credentials: Box<[Box<dyn CredentialProvider>]>,
}

impl ChainCredentialsProvider {
    #[inline]
    pub fn builder(credential: Box<dyn CredentialProvider>) -> ChainCredentialsProviderBuilder {
        ChainCredentialsProviderBuilder::new(credential)
    }
}

impl CredentialProvider for ChainCredentialsProvider {
    fn get(&self, opts: &GetOptions) -> Result<GotCredential> {
        if let Some(credential) = self.credentials.iter().find_map(|c| c.get(opts).ok()) {
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
    fn async_get<'a>(&'a self, opts: &'a GetOptions) -> AsyncResult<'a, GotCredential> {
        Box::pin(async move {
            for provider in self.credentials.iter() {
                if let Ok(credential) = provider.async_get(opts).await {
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
        ChainCredentialsProviderBuilder::new(Box::new(GlobalCredentialProvider))
            .append_credential(Box::new(EnvCredentialProvider))
            .build()
    }
}

/// 串联认证信息构建器
///
/// 接受多个认证信息提供者并将他们串联成串联认证信息
pub struct ChainCredentialsProviderBuilder {
    credentials: VecDeque<Box<dyn CredentialProvider>>,
}

impl ChainCredentialsProviderBuilder {
    /// 构建新的串联认证信息构建器
    #[inline]
    pub fn new(credential: Box<dyn CredentialProvider>) -> ChainCredentialsProviderBuilder {
        let mut credentials = VecDeque::with_capacity(1);
        credentials.push_back(credential);
        Self { credentials }
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
    use http::header::HeaderName;
    use mime::APPLICATION_JSON;
    use std::{boxed::Box, error::Error, result::Result, sync::Arc, thread, time::Duration};

    #[test]
    fn test_sign() -> Result<(), Box<dyn Error>> {
        let credential: Arc<dyn CredentialProvider> = Arc::new(get_static_credential());
        let mut threads = Vec::new();
        {
            let credential = credential.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(
                    credential.get(&Default::default()).unwrap().sign(b"hello"),
                    "abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0="
                );
                assert_eq!(
                    credential
                        .get(&Default::default())
                        .unwrap()
                        .sign_reader(&mut Cursor::new(b"world"))
                        .unwrap(),
                    "abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ="
                );
            }));
        }
        {
            let credential = credential.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(
                    credential.get(&Default::default()).unwrap().sign(b"-test"),
                    "abcdefghklmnopq:vYKRLUoXRlNHfpMEQeewG0zylaw="
                );
                assert_eq!(
                    credential
                        .get(&Default::default())
                        .unwrap()
                        .sign_reader(&mut Cursor::new(b"ba#a-"))
                        .unwrap(),
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
    fn test_sign_with_data() -> Result<(), Box<dyn Error>> {
        let credential: Arc<dyn CredentialProvider> = Arc::new(get_static_credential());
        let mut threads = Vec::new();
        {
            let credential = credential.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(
                    credential
                        .get(&Default::default())
                        .unwrap()
                        .sign_with_data(b"hello"),
                    "abcdefghklmnopq:BZYt5uVRy1RVt5ZTXbaIt2ROVMA=:aGVsbG8="
                );
                assert_eq!(
                    credential
                        .get(&Default::default())
                        .unwrap()
                        .sign_with_data(b"world"),
                    "abcdefghklmnopq:Wpe04qzPphiSZb1u6I0nFn6KpZg=:d29ybGQ="
                );
            }));
        }
        {
            let credential = credential.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(
                    credential
                        .get(&Default::default())
                        .unwrap()
                        .sign_with_data(b"-test"),
                    "abcdefghklmnopq:HlxenSSP_6BbaYNzx1fyeyw8v1Y=:LXRlc3Q="
                );
                assert_eq!(
                    credential
                        .get(&Default::default())
                        .unwrap()
                        .sign_with_data(b"ba#a-"),
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
    fn test_authorization_v1_with_body_reader() -> Result<(), Box<dyn Error>> {
        let credential = get_static_credential();
        assert_eq!(
            credential
                .get(&Default::default())?
                .authorization_v1_for_request_with_body_reader(
                    &"http://upload.qiniup.com/".parse()?,
                    None,
                    &mut Cursor::new(b"{\"name\":\"test\"}")
                )?,
            "QBox ".to_owned() + &credential.get(&Default::default())?.sign(b"/\n")
        );
        assert_eq!(
            credential
                .get(&Default::default())?
                .authorization_v1_for_request_with_body_reader(
                    &"http://upload.qiniup.com/".parse()?,
                    Some(&HeaderValue::from_str(APPLICATION_JSON.as_ref())?),
                    &mut Cursor::new(b"{\"name\":\"test\"}")
                )?,
            "QBox ".to_owned() + &credential.get(&Default::default())?.sign(b"/\n")
        );
        assert_eq!(
            credential
                .get(&Default::default())?
                .authorization_v1_for_request_with_body_reader(
                    &"http://upload.qiniup.com/".parse()?,
                    Some(&HeaderValue::from_str(
                        APPLICATION_WWW_FORM_URLENCODED.as_ref()
                    )?),
                    &mut Cursor::new(b"name=test&language=go")
                )?,
            "QBox ".to_owned()
                + &credential
                    .get(&Default::default())?
                    .sign(b"/\nname=test&language=go")
        );
        assert_eq!(
            credential
                .get(&Default::default())?
                .authorization_v1_for_request_with_body_reader(
                    &"http://upload.qiniup.com/?v=2".parse()?,
                    Some(&HeaderValue::from_str(
                        APPLICATION_WWW_FORM_URLENCODED.as_ref()
                    )?),
                    &mut Cursor::new(b"name=test&language=go")
                )?,
            "QBox ".to_owned()
                + &credential
                    .get(&Default::default())?
                    .sign(b"/?v=2\nname=test&language=go")
        );
        assert_eq!(
            credential
                .get(&Default::default())?
                .authorization_v1_for_request_with_body_reader(
                    &"http://upload.qiniup.com/find/sdk?v=2".parse()?,
                    Some(&HeaderValue::from_str(
                        APPLICATION_WWW_FORM_URLENCODED.as_ref()
                    )?),
                    &mut Cursor::new(b"name=test&language=go")
                )?,
            "QBox ".to_owned()
                + &credential
                    .get(&Default::default())?
                    .sign(b"/find/sdk?v=2\nname=test&language=go")
        );
        Ok(())
    }

    #[test]
    fn test_authorization_v2_with_body_reader() -> Result<(), Box<dyn Error>> {
        let credential = get_global_credential();
        let empty_headers = {
            let mut headers = HeaderMap::new();
            headers.insert(
                HeaderName::from_static("x-qbox-meta"),
                HeaderValue::from_str("value")?,
            );
            headers
        };
        let json_headers = {
            let mut headers = HeaderMap::new();
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_str(APPLICATION_JSON.as_ref())?,
            );
            headers.insert(
                HeaderName::from_static("x-qbox-meta"),
                HeaderValue::from_str("value")?,
            );
            headers.insert(
                HeaderName::from_static("x-qiniu-cxxxx"),
                HeaderValue::from_str("valuec")?,
            );
            headers.insert(
                HeaderName::from_static("x-qiniu-bxxxx"),
                HeaderValue::from_str("valueb")?,
            );
            headers.insert(
                HeaderName::from_static("x-qiniu-axxxx"),
                HeaderValue::from_str("valuea")?,
            );
            headers.insert(
                HeaderName::from_static("x-qiniu-e"),
                HeaderValue::from_str("value")?,
            );
            headers.insert(
                HeaderName::from_static("x-qiniu-"),
                HeaderValue::from_str("value")?,
            );
            headers.insert(
                HeaderName::from_static("x-qiniu"),
                HeaderValue::from_str("value")?,
            );
            headers
        };
        let form_headers = {
            let mut headers = HeaderMap::new();
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_str(APPLICATION_WWW_FORM_URLENCODED.as_ref())?,
            );
            headers.insert(
                HeaderName::from_static("x-qbox-meta"),
                HeaderValue::from_str("value")?,
            );
            headers.insert(
                HeaderName::from_static("x-qiniu-cxxxx"),
                HeaderValue::from_str("valuec")?,
            );
            headers.insert(
                HeaderName::from_static("x-qiniu-bxxxx"),
                HeaderValue::from_str("valueb")?,
            );
            headers.insert(
                HeaderName::from_static("x-qiniu-axxxx"),
                HeaderValue::from_str("valuea")?,
            );
            headers.insert(
                HeaderName::from_static("x-qiniu-e"),
                HeaderValue::from_str("value")?,
            );
            headers.insert(
                HeaderName::from_static("x-qiniu-"),
                HeaderValue::from_str("value")?,
            );
            headers.insert(
                HeaderName::from_static("x-qiniu"),
                HeaderValue::from_str("value")?,
            );
            headers
        };
        assert_eq!(
            credential
                .get(&Default::default())?
                .authorization_v2_for_request_with_body_reader(
                    &Method::GET,
                    &"http://upload.qiniup.com/".parse()?,
                    &json_headers,
                    &mut Cursor::new(b"{\"name\":\"test\"}")
                )?,
            "Qiniu ".to_owned()
                + &credential.get(&Default::default())?.sign(
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
            credential
                .get(&Default::default())?
                .authorization_v2_for_request_with_body_reader(
                    &Method::GET,
                    &"http://upload.qiniup.com/".parse()?,
                    &empty_headers,
                    &mut Cursor::new(b"{\"name\":\"test\"}")
                )?,
            "Qiniu ".to_owned()
                + &credential
                    .get(&Default::default())?
                    .sign(concat!("GET /\n", "Host: upload.qiniup.com\n\n").as_bytes())
        );
        assert_eq!(
            credential
                .get(&Default::default())?
                .authorization_v2_for_request_with_body_reader(
                    &Method::POST,
                    &"http://upload.qiniup.com/".parse()?,
                    &json_headers,
                    &mut Cursor::new(b"{\"name\":\"test\"}")
                )?,
            "Qiniu ".to_owned()
                + &credential.get(&Default::default())?.sign(
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
            credential
                .get(&Default::default())?
                .authorization_v2_for_request_with_body_reader(
                    &Method::GET,
                    &"http://upload.qiniup.com/".parse()?,
                    &form_headers,
                    &mut Cursor::new(b"name=test&language=go")
                )?,
            "Qiniu ".to_owned()
                + &credential.get(&Default::default())?.sign(
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
            credential
                .get(&Default::default())?
                .authorization_v2_for_request_with_body_reader(
                    &Method::GET,
                    &"http://upload.qiniup.com/?v=2".parse()?,
                    &form_headers,
                    &mut Cursor::new(b"name=test&language=go")
                )?,
            "Qiniu ".to_owned()
                + &credential.get(&Default::default())?.sign(
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
            credential
                .get(&Default::default())?
                .authorization_v2_for_request_with_body_reader(
                    &Method::GET,
                    &"http://upload.qiniup.com/find/sdk?v=2".parse()?,
                    &form_headers,
                    &mut Cursor::new(b"name=test&language=go")
                )?,
            "Qiniu ".to_owned()
                + &credential.get(&Default::default())?.sign(
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
        let url = "http://www.qiniu.com/?go=1".parse()?;
        let url = credential
            .get(&Default::default())?
            .sign_download_url(url, Duration::from_secs(1_234_567_890 + 3600));
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
            let cred = chain_credentials.get(&Default::default())?;
            assert_eq!(cred.access_key().as_str(), "TEST2");
        }
        GlobalCredentialProvider::setup(Credential::new("TEST1", "test1"));
        {
            let cred = chain_credentials.get(&Default::default())?;
            assert_eq!(cred.access_key().as_str(), "TEST1");
        }
        Ok(())
    }

    fn get_static_credential() -> impl CredentialProvider {
        StaticCredentialProvider::new(Credential::new("abcdefghklmnopq", "1234567890"))
    }

    fn get_global_credential() -> impl CredentialProvider {
        GlobalCredentialProvider::setup(Credential::new("abcdefghklmnopq", "1234567890"));
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
        use futures_lite::io::Cursor;

        #[async_std::test]
        async fn test_sign_async_reader() -> Result<(), Box<dyn Error>> {
            let credential = get_static_credential();
            assert_eq!(
                credential
                    .get(&Default::default())?
                    .sign_async_reader(&mut Cursor::new(b"hello"))
                    .await?,
                "abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0="
            );
            assert_eq!(
                credential
                    .get(&Default::default())?
                    .sign_async_reader(&mut Cursor::new(b"world"))
                    .await?,
                "abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ="
            );
            assert_eq!(
                credential
                    .get(&Default::default())?
                    .sign_async_reader(&mut Cursor::new(b"-test"))
                    .await?,
                "abcdefghklmnopq:vYKRLUoXRlNHfpMEQeewG0zylaw="
            );
            assert_eq!(
                credential
                    .get(&Default::default())?
                    .sign_async_reader(&mut Cursor::new(b"ba#a-"))
                    .await?,
                "abcdefghklmnopq:2d_Yr6H1GdTKg3RvMtpHOhi047M="
            );
            Ok(())
        }

        #[async_std::test]
        async fn test_async_authorization_v1() -> Result<(), Box<dyn Error>> {
            let credential = get_static_credential();
            assert_eq!(
                credential
                    .get(&Default::default())?
                    .authorization_v1_for_request_with_async_body_reader(
                        &"http://upload.qiniup.com/".parse()?,
                        None,
                        &mut Cursor::new(b"{\"name\":\"test\"}")
                    )
                    .await?,
                "QBox ".to_owned() + &credential.get(&Default::default())?.sign(b"/\n")
            );
            assert_eq!(
                credential
                    .get(&Default::default())?
                    .authorization_v1_for_request_with_async_body_reader(
                        &"http://upload.qiniup.com/".parse()?,
                        Some(&HeaderValue::from_str(APPLICATION_JSON.as_ref())?),
                        &mut Cursor::new(b"{\"name\":\"test\"}")
                    )
                    .await?,
                "QBox ".to_owned() + &credential.get(&Default::default())?.sign(b"/\n")
            );
            assert_eq!(
                credential
                    .get(&Default::default())?
                    .authorization_v1_for_request_with_async_body_reader(
                        &"http://upload.qiniup.com/".parse()?,
                        Some(&HeaderValue::from_str(
                            APPLICATION_WWW_FORM_URLENCODED.as_ref()
                        )?),
                        &mut Cursor::new(b"name=test&language=go")
                    )
                    .await?,
                "QBox ".to_owned()
                    + &credential
                        .get(&Default::default())?
                        .sign(b"/\nname=test&language=go")
            );
            assert_eq!(
                credential
                    .get(&Default::default())?
                    .authorization_v1_for_request_with_async_body_reader(
                        &"http://upload.qiniup.com/?v=2".parse()?,
                        Some(&HeaderValue::from_str(
                            APPLICATION_WWW_FORM_URLENCODED.as_ref()
                        )?),
                        &mut Cursor::new(b"name=test&language=go")
                    )
                    .await?,
                "QBox ".to_owned()
                    + &credential
                        .get(&Default::default())?
                        .sign(b"/?v=2\nname=test&language=go")
            );
            assert_eq!(
                credential
                    .get(&Default::default())?
                    .authorization_v1_for_request_with_async_body_reader(
                        &"http://upload.qiniup.com/find/sdk?v=2".parse()?,
                        Some(&HeaderValue::from_str(
                            APPLICATION_WWW_FORM_URLENCODED.as_ref()
                        )?),
                        &mut Cursor::new(b"name=test&language=go")
                    )
                    .await?,
                "QBox ".to_owned()
                    + &credential
                        .get(&Default::default())?
                        .sign(b"/find/sdk?v=2\nname=test&language=go")
            );
            Ok(())
        }

        #[async_std::test]
        async fn test_async_authorization_v2() -> Result<(), Box<dyn Error>> {
            let credential = get_global_credential();
            let empty_headers = {
                let mut headers = HeaderMap::new();
                headers.insert(
                    HeaderName::from_static("x-qbox-meta"),
                    HeaderValue::from_str("value")?,
                );
                headers
            };
            let json_headers = {
                let mut headers = HeaderMap::new();
                headers.insert(
                    CONTENT_TYPE,
                    HeaderValue::from_str(APPLICATION_JSON.as_ref())?,
                );
                headers.insert(
                    HeaderName::from_static("x-qbox-meta"),
                    HeaderValue::from_str("value")?,
                );
                headers.insert(
                    HeaderName::from_static("x-qiniu-cxxxx"),
                    HeaderValue::from_str("valuec")?,
                );
                headers.insert(
                    HeaderName::from_static("x-qiniu-bxxxx"),
                    HeaderValue::from_str("valueb")?,
                );
                headers.insert(
                    HeaderName::from_static("x-qiniu-axxxx"),
                    HeaderValue::from_str("valuea")?,
                );
                headers.insert(
                    HeaderName::from_static("x-qiniu-e"),
                    HeaderValue::from_str("value")?,
                );
                headers.insert(
                    HeaderName::from_static("x-qiniu-"),
                    HeaderValue::from_str("value")?,
                );
                headers.insert(
                    HeaderName::from_static("x-qiniu"),
                    HeaderValue::from_str("value")?,
                );
                headers
            };
            let form_headers = {
                let mut headers = HeaderMap::new();
                headers.insert(
                    CONTENT_TYPE,
                    HeaderValue::from_str(APPLICATION_WWW_FORM_URLENCODED.as_ref())?,
                );
                headers.insert(
                    HeaderName::from_static("x-qbox-meta"),
                    HeaderValue::from_str("value")?,
                );
                headers.insert(
                    HeaderName::from_static("x-qiniu-cxxxx"),
                    HeaderValue::from_str("valuec")?,
                );
                headers.insert(
                    HeaderName::from_static("x-qiniu-bxxxx"),
                    HeaderValue::from_str("valueb")?,
                );
                headers.insert(
                    HeaderName::from_static("x-qiniu-axxxx"),
                    HeaderValue::from_str("valuea")?,
                );
                headers.insert(
                    HeaderName::from_static("x-qiniu-e"),
                    HeaderValue::from_str("value")?,
                );
                headers.insert(
                    HeaderName::from_static("x-qiniu-"),
                    HeaderValue::from_str("value")?,
                );
                headers.insert(
                    HeaderName::from_static("x-qiniu"),
                    HeaderValue::from_str("value")?,
                );
                headers
            };
            assert_eq!(
                credential
                    .get(&Default::default())?
                    .authorization_v2_for_request_with_async_body_reader(
                        &Method::GET,
                        &"http://upload.qiniup.com/".parse()?,
                        &json_headers,
                        &mut Cursor::new(b"{\"name\":\"test\"}")
                    )
                    .await?,
                "Qiniu ".to_owned()
                    + &credential.get(&Default::default())?.sign(
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
                credential
                    .get(&Default::default())?
                    .authorization_v2_for_request_with_async_body_reader(
                        &Method::GET,
                        &"http://upload.qiniup.com/".parse()?,
                        &empty_headers,
                        &mut Cursor::new(b"{\"name\":\"test\"}")
                    )
                    .await?,
                "Qiniu ".to_owned()
                    + &credential
                        .get(&Default::default())?
                        .sign(concat!("GET /\n", "Host: upload.qiniup.com\n\n").as_bytes())
            );
            assert_eq!(
                credential
                    .get(&Default::default())?
                    .authorization_v2_for_request_with_async_body_reader(
                        &Method::POST,
                        &"http://upload.qiniup.com/".parse()?,
                        &json_headers,
                        &mut Cursor::new(b"{\"name\":\"test\"}")
                    )
                    .await?,
                "Qiniu ".to_owned()
                    + &credential.get(&Default::default())?.sign(
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
                credential
                    .get(&Default::default())?
                    .authorization_v2_for_request_with_async_body_reader(
                        &Method::GET,
                        &"http://upload.qiniup.com/".parse()?,
                        &form_headers,
                        &mut Cursor::new(b"name=test&language=go")
                    )
                    .await?,
                "Qiniu ".to_owned()
                    + &credential.get(&Default::default())?.sign(
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
                credential
                    .get(&Default::default())?
                    .authorization_v2_for_request_with_async_body_reader(
                        &Method::GET,
                        &"http://upload.qiniup.com/?v=2".parse()?,
                        &form_headers,
                        &mut Cursor::new(b"name=test&language=go")
                    )
                    .await?,
                "Qiniu ".to_owned()
                    + &credential.get(&Default::default())?.sign(
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
                credential
                    .get(&Default::default())?
                    .authorization_v2_for_request_with_async_body_reader(
                        &Method::GET,
                        &"http://upload.qiniup.com/find/sdk?v=2".parse()?,
                        &form_headers,
                        &mut Cursor::new(b"name=test&language=go")
                    )
                    .await?,
                "Qiniu ".to_owned()
                    + &credential.get(&Default::default())?.sign(
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

        #[async_std::test]
        async fn test_async_sign_download_url() -> Result<(), Box<dyn Error>> {
            let credential = get_env_credential();
            let url = "http://www.qiniu.com/?go=1".parse()?;
            let url = credential
                .async_get(&Default::default())
                .await?
                .sign_download_url(url, Duration::from_secs(1_234_567_890 + 3600));
            assert_eq!(
                url.to_string(),
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
                let cred = chain_credentials.async_get(&Default::default()).await?;
                assert_eq!(cred.access_key().as_str(), "TEST2");
            }
            GlobalCredentialProvider::setup(Credential::new("TEST1", "test1"));
            {
                let cred = chain_credentials.async_get(&Default::default()).await?;
                assert_eq!(cred.access_key().as_str(), "TEST1");
            }
            Ok(())
        }
    }
}
