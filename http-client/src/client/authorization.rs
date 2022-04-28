use anyhow::Error as AnyError;
use auto_impl::auto_impl;
use chrono::Utc;
use dyn_clonable::clonable;
use qiniu_credential::{Credential, CredentialProvider, GetOptions, Uri};
use qiniu_http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, RequestParts, SyncRequest,
};
use qiniu_upload_token::{ToStringError, UploadTokenProvider};
use std::{
    env::{remove_var, set_var, var_os},
    fmt::Debug,
    io::Error as IoError,
    mem::take,
    time::Duration,
};
use tap::Tap;
use thiserror::Error;
use url::ParseError as UrlParseError;

#[cfg(feature = "async")]
use {futures::future::BoxFuture, qiniu_http::AsyncRequest};

/// 七牛鉴权签名接口
///
/// 对 HTTP 请求进行签名
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait AuthorizationProvider: Clone + Debug + Sync + Send {
    /// 使用指定的鉴权方式对 HTTP 请求进行签名
    ///
    /// 该方法的异步版本为 [`Self::async_sign`]。
    fn sign(&self, request: &mut SyncRequest) -> AuthorizationResult<()>;

    /// 使用指定的鉴权方式对异步 HTTP 请求进行签名
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_sign<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AuthorizationResult<()>>;
}

/// 上传凭证鉴权签名
#[derive(Clone, Debug)]
pub struct UploadTokenAuthorization<P: ?Sized>(P);

impl<P> UploadTokenAuthorization<P> {
    /// 创建上传凭证鉴权签名
    #[inline]
    pub fn new(provider: P) -> Self {
        Self(provider)
    }
}

impl<P> From<P> for UploadTokenAuthorization<P> {
    #[inline]
    fn from(provider: P) -> Self {
        Self::new(provider)
    }
}

impl<P: UploadTokenProvider + Clone> AuthorizationProvider for UploadTokenAuthorization<P> {
    fn sign(&self, request: &mut SyncRequest) -> AuthorizationResult<()> {
        let authorization = uptoken_authorization(&self.0.to_token_string(Default::default())?);
        set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
        Ok(())
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_sign<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AuthorizationResult<()>> {
        Box::pin(async move {
            let authorization = uptoken_authorization(&self.0.async_to_token_string(Default::default()).await?);
            set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
            Ok(())
        })
    }
}

/// 七牛签名算法 V1 鉴权签名
#[derive(Clone, Debug)]
pub struct CredentialAuthorizationV1<P: ?Sized>(P);

impl<P> CredentialAuthorizationV1<P> {
    /// 创建七牛签名算法 V1 鉴权签名
    #[inline]
    pub fn new(provider: P) -> Self {
        Self(provider)
    }
}

impl<P> From<P> for CredentialAuthorizationV1<P> {
    #[inline]
    fn from(provider: P) -> Self {
        Self::new(provider)
    }
}

impl<P: CredentialProvider + Clone> AuthorizationProvider for CredentialAuthorizationV1<P> {
    fn sign(&self, request: &mut SyncRequest) -> AuthorizationResult<()> {
        _sign(&self.0, request, Default::default())?;
        return Ok(());

        fn _sign(
            credential_provider: impl CredentialProvider + Clone,
            request: &mut SyncRequest,
            get_options: GetOptions,
        ) -> AuthorizationResult<()> {
            let authorization =
                authorization_v1_for_request(credential_provider.get(get_options)?.credential(), request)?;
            set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
            Ok(())
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_sign<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AuthorizationResult<()>> {
        return Box::pin(async move {
            _sign(&self.0, request, Default::default()).await?;
            Ok(())
        });

        async fn _sign(
            credential_provider: impl CredentialProvider + Clone,
            request: &mut AsyncRequest<'_>,
            get_options: GetOptions,
        ) -> AuthorizationResult<()> {
            let authorization = authorization_v1_for_async_request(
                credential_provider.async_get(get_options).await?.credential(),
                request,
            )
            .await?;
            set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
            Ok(())
        }
    }
}

fn authorization_v1_for_request(credential: &Credential, request: &mut SyncRequest) -> AuthorizationResult<String> {
    let (parts, mut body) = take(request).into_parts_and_body();
    credential
        .authorization_v1_for_request_with_body_reader(parts.url(), parts.headers().get(CONTENT_TYPE), &mut body)
        .tap(|_| {
            *request = SyncRequest::from_parts_and_body(parts, body);
        })
        .map_err(|err| err.into())
}

#[cfg(feature = "async")]
async fn authorization_v1_for_async_request(
    credential: &Credential,
    request: &mut AsyncRequest<'_>,
) -> AuthorizationResult<String> {
    let (parts, mut body) = take(request).into_parts_and_body();
    credential
        .authorization_v1_for_request_with_async_body_reader(parts.url(), parts.headers().get(CONTENT_TYPE), &mut body)
        .await
        .tap(|_| {
            *request = AsyncRequest::from_parts_and_body(parts, body);
        })
        .map_err(|err| err.into())
}

/// 全局禁用时间戳签名
pub fn global_disable_timestamp_signature() {
    set_var(DISABLE_QINIU_TIMESTAMP_SIGNATURE, "1");
}

/// 全局启用时间戳签名
pub fn global_enable_timestamp_signature() {
    remove_var(DISABLE_QINIU_TIMESTAMP_SIGNATURE);
}

/// 七牛签名算法 V2 鉴权签名
#[derive(Clone, Debug)]
pub struct CredentialAuthorizationV2<P: ?Sized> {
    timestamp_signature_enabled: bool,
    provider: P,
}

const DISABLE_QINIU_TIMESTAMP_SIGNATURE: &str = "DISABLE_QINIU_TIMESTAMP_SIGNATURE";

impl<P> CredentialAuthorizationV2<P> {
    /// 创建七牛签名算法 V2 鉴权签名
    ///
    /// 可以通过 `DISABLE_QINIU_TIMESTAMP_SIGNATURE` 环境变量禁用时间戳签名，或是调用 [`Self::disable_timestamp_signature`] 来禁用时间戳签名
    #[inline]
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            timestamp_signature_enabled: var_os(DISABLE_QINIU_TIMESTAMP_SIGNATURE).is_none(),
        }
    }

    /// 禁用时间戳签名
    ///
    /// 该方法将覆盖 `DISABLE_QINIU_TIMESTAMP_SIGNATURE` 环境变量的设置
    #[inline]
    pub fn disable_timestamp_signature(&mut self) -> &mut Self {
        self.timestamp_signature_enabled = false;
        self
    }

    /// 启用时间戳签名
    ///
    /// 该方法将覆盖 `DISABLE_QINIU_TIMESTAMP_SIGNATURE` 环境变量的设置
    #[inline]
    pub fn enable_timestamp_signature(&mut self) -> &mut Self {
        self.timestamp_signature_enabled = true;
        self
    }
}

impl<P> From<P> for CredentialAuthorizationV2<P> {
    #[inline]
    fn from(provider: P) -> Self {
        Self::new(provider)
    }
}

impl<P: CredentialProvider + Clone> AuthorizationProvider for CredentialAuthorizationV2<P> {
    fn sign(&self, request: &mut SyncRequest) -> AuthorizationResult<()> {
        _sign(
            &self.provider,
            self.timestamp_signature_enabled,
            request,
            Default::default(),
        )?;
        return Ok(());

        fn _sign(
            credential_provider: impl CredentialProvider + Clone,
            timestamp_signature_enabled: bool,
            request: &mut SyncRequest,
            get_options: GetOptions,
        ) -> AuthorizationResult<()> {
            if timestamp_signature_enabled {
                request.headers_mut().insert(X_QINIU_DATE, make_x_qiniu_date_value());
            }
            let authorization =
                authorization_v2_for_request(credential_provider.get(get_options)?.credential(), request)?;
            set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
            Ok(())
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_sign<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AuthorizationResult<()>> {
        return Box::pin(async move {
            _sign(
                &self.provider,
                self.timestamp_signature_enabled,
                request,
                Default::default(),
            )
            .await?;
            Ok(())
        });

        async fn _sign(
            credential_provider: impl CredentialProvider + Clone,
            timestamp_signature_enabled: bool,
            request: &mut AsyncRequest<'_>,
            get_options: GetOptions,
        ) -> AuthorizationResult<()> {
            if timestamp_signature_enabled {
                request.headers_mut().insert(X_QINIU_DATE, make_x_qiniu_date_value());
            }
            let authorization = authorization_v2_for_async_request(
                credential_provider.async_get(get_options).await?.credential(),
                request,
            )
            .await?;
            set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
            Ok(())
        }
    }
}

fn authorization_v2_for_request(credential: &Credential, request: &mut SyncRequest) -> AuthorizationResult<String> {
    let (parts, mut body) = take(request).into_parts_and_body();
    credential
        .authorization_v2_for_request_with_body_reader(parts.method(), parts.url(), parts.headers(), &mut body)
        .tap(|_| {
            *request = SyncRequest::from_parts_and_body(parts, body);
        })
        .map_err(|err| err.into())
}

#[cfg(feature = "async")]
async fn authorization_v2_for_async_request(
    credential: &Credential,
    request: &mut AsyncRequest<'_>,
) -> AuthorizationResult<String> {
    let (parts, mut body) = take(request).into_parts_and_body();
    credential
        .authorization_v2_for_request_with_async_body_reader(parts.method(), parts.url(), parts.headers(), &mut body)
        .await
        .tap(|_| {
            *request = AsyncRequest::from_parts_and_body(parts, body);
        })
        .map_err(|err| err.into())
}

fn set_authorization(request: &mut RequestParts, authorization: HeaderValue) {
    request.headers_mut().insert(AUTHORIZATION, authorization);
}

fn uptoken_authorization(upload_token: &str) -> String {
    "UpToken ".to_owned() + upload_token
}

const X_QINIU_DATE: &str = "X-Qiniu-Date";

fn make_x_qiniu_date_value() -> HeaderValue {
    HeaderValue::from_str(&Utc::now().format("%Y%m%dT%H%M%SZ").to_string()).unwrap()
}

/// 七牛下载地址鉴权签名
#[derive(Clone, Debug)]
pub struct DownloadUrlCredentialAuthorization<P: ?Sized> {
    lifetime: Duration,
    provider: P,
}

impl<P> DownloadUrlCredentialAuthorization<P> {
    /// 创建七牛下载地址鉴权签名
    #[inline]
    pub fn new(provider: P, lifetime: Duration) -> Self {
        Self { provider, lifetime }
    }
}

impl<P> From<P> for DownloadUrlCredentialAuthorization<P> {
    #[inline]
    fn from(provider: P) -> Self {
        Self::new(provider, Duration::from_secs(3600))
    }
}

impl<P: CredentialProvider + Clone> AuthorizationProvider for DownloadUrlCredentialAuthorization<P> {
    fn sign(&self, request: &mut SyncRequest) -> AuthorizationResult<()> {
        let credential = self.provider.get(Default::default())?;
        let url = sign_download_url(&credential, self.lifetime, take(request.url_mut()));
        *request.url_mut() = url;
        Ok(())
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_sign<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AuthorizationResult<()>> {
        Box::pin(async move {
            let credential = self.provider.async_get(Default::default()).await?;
            let url = sign_download_url(&credential, self.lifetime, take(request.url_mut()));
            *request.url_mut() = url;
            Ok(())
        })
    }
}

fn sign_download_url(credential: &Credential, lifetime: Duration, url: Uri) -> Uri {
    credential.sign_download_url(url, lifetime)
}

/// 鉴权签名错误
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AuthorizationError {
    /// 获取认证信息或上传凭证错误
    #[error("Get Upload Token or Credential error: {0}")]
    IoError(#[from] IoError),

    /// 生成上传凭证回调函数错误
    #[error("Generate Upload Policy Callback error: {0}")]
    CallbackError(#[from] AnyError),

    /// URL 解析错误
    #[error("Parse URL error: {0}")]
    UrlParseError(#[from] UrlParseError),
}

/// 鉴权签名结果
pub type AuthorizationResult<T> = Result<T, AuthorizationError>;

impl From<ToStringError> for AuthorizationError {
    fn from(err: ToStringError) -> Self {
        match err {
            ToStringError::CredentialGetError(err) => Self::IoError(err),
            ToStringError::CallbackError(err) => Self::CallbackError(err),
            err => unimplemented!("Unexpected ToStringError: {:?}", err),
        }
    }
}

/// 七牛鉴权签名
///
/// 该类型是个枚举类型，引用或拥有七牛鉴权签名接口的实例
#[derive(Clone, Debug)]
pub enum Authorization<'a> {
    /// 拥有七牛鉴权签名接口的实例
    Owned(Box<dyn AuthorizationProvider + 'a>),

    /// 引用七牛鉴权签名接口的实例
    Borrowed(&'a dyn AuthorizationProvider),
}

impl<'a> Authorization<'a> {
    /// 根据一个拥有的七牛鉴权签名接口的实例创建一个鉴权签名
    #[inline]
    pub fn from_owned<T: AuthorizationProvider + 'a>(provider: T) -> Self {
        Self::Owned(Box::new(provider))
    }

    /// 根据一个引用的七牛鉴权签名接口的实例创建一个鉴权签名
    #[inline]
    pub fn from_referenced(provider: &'a dyn AuthorizationProvider) -> Self {
        Self::Borrowed(provider)
    }

    /// 根据上传凭证获取接口创建一个上传凭证签名算法的签名
    #[inline]
    pub fn uptoken(provider: impl UploadTokenProvider + Clone + 'a) -> Self {
        Self::from_owned(UploadTokenAuthorization::from(provider))
    }

    /// 根据认证信息获取接口创建一个使用七牛鉴权 v1 签名算法的签名
    #[inline]
    pub fn v1(provider: impl CredentialProvider + Clone + 'a) -> Self {
        Self::from_owned(CredentialAuthorizationV1::from(provider))
    }

    /// 根据认证信息获取接口创建一个使用七牛鉴权 v2 签名算法的签名
    #[inline]
    pub fn v2(provider: impl CredentialProvider + Clone + 'a) -> Self {
        Self::from_owned(CredentialAuthorizationV2::from(provider))
    }

    /// 根据认证信息获取接口创建一个使用七牛鉴权 v2 签名算法的签名，并且禁用时间戳签名
    #[inline]
    pub fn v2_without_timestamp_signature(provider: impl CredentialProvider + Clone + 'a) -> Self {
        let mut auth = CredentialAuthorizationV2::from(provider);
        auth.disable_timestamp_signature();
        Self::from_owned(auth)
    }

    /// 根据认证信息获取接口创建一个下载凭证签名算法的签名
    #[inline]
    pub fn download(provider: impl CredentialProvider + Clone + 'a) -> Self {
        Self::from_owned(DownloadUrlCredentialAuthorization::from(provider))
    }
}

impl<'a> AsRef<dyn AuthorizationProvider + 'a> for Authorization<'a> {
    #[inline]
    fn as_ref(&self) -> &(dyn AuthorizationProvider + 'a) {
        match self {
            Authorization::Owned(owned) => owned.as_ref(),
            Authorization::Borrowed(borrowed) => borrowed,
        }
    }
}

impl AuthorizationProvider for Authorization<'_> {
    fn sign(&self, request: &mut SyncRequest) -> AuthorizationResult<()> {
        self.as_ref().sign(request)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_sign<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AuthorizationResult<()>> {
        self.as_ref().async_sign(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result as AnyResult;
    use qiniu_credential::HeaderMap;
    use qiniu_http::SyncRequestBody;

    #[test]
    fn test_credential_authorition_v2() -> AnyResult<()> {
        let credential = Credential::new("ak", "sk");
        let headers = {
            let mut headers = HeaderMap::new();
            headers.insert("x-qiniu-", HeaderValue::from_static("a"));
            headers.insert("x-qiniu", HeaderValue::from_static("b"));
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-www-form-urlencoded"),
            );
            headers
        };
        let body = b"{\"name\": \"test\"}";
        let mut request = SyncRequest::builder()
            .url("http://upload.qiniup.com".parse()?)
            .headers(headers.to_owned())
            .body(SyncRequestBody::from_bytes(body.to_vec()))
            .build();
        global_enable_timestamp_signature();
        Authorization::v2(credential.to_owned()).sign(&mut request)?;
        assert!(request
            .headers()
            .get(AUTHORIZATION)
            .unwrap()
            .to_str()?
            .starts_with("Qiniu ak:"));
        assert!(request.headers().get("x-qiniu-date").is_some());

        global_disable_timestamp_signature();
        request = SyncRequest::builder()
            .url("http://upload.qiniup.com".parse()?)
            .headers(headers)
            .body(SyncRequestBody::from_bytes(body.to_vec()))
            .build();
        Authorization::v2(credential.to_owned()).sign(&mut request)?;
        global_enable_timestamp_signature();
        assert!(request
            .headers()
            .get(AUTHORIZATION)
            .unwrap()
            .to_str()?
            .starts_with("Qiniu ak:"));
        assert!(request.headers().get("x-qiniu-date").is_none());

        let headers = {
            let mut headers = HeaderMap::new();
            headers.insert("x-qiniu-bbb", HeaderValue::from_static("AAA"));
            headers.insert("x-qiniu-aaa", HeaderValue::from_static("CCC"));
            headers
        };
        let body = b"name=test&language=go}";
        global_disable_timestamp_signature();
        request = SyncRequest::builder()
            .url("http://upload.qiniup.com/mkfile/sdf.jpg".parse()?)
            .headers(headers)
            .body(SyncRequestBody::from_bytes(body.to_vec()))
            .build();
        Authorization::v2(credential).sign(&mut request)?;
        global_enable_timestamp_signature();
        assert_eq!(
            request.headers().get(AUTHORIZATION).unwrap(),
            HeaderValue::from_static("Qiniu ak:arPKqUn6T6DrnHhygbFS40PGBgY=")
        );
        assert!(request.headers().get("x-qiniu-date").is_none());
        Ok(())
    }
}
