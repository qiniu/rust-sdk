use qiniu_credential::{Credential, CredentialProvider};
use qiniu_http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, RequestParts, SyncRequest,
};
use qiniu_upload_token::UploadTokenProvider;
use std::{fmt, io::Error as IoError, mem::take, result::Result};
use tap::Tap;
use thiserror::Error;
use url::ParseError as UrlParseError;

#[cfg(feature = "async")]
use qiniu_http::AsyncRequest;

/// API 鉴权方式
#[derive(Clone)]
pub struct Authorization {
    inner: AuthorizationInner,
}

#[derive(Clone, Debug)]
enum AuthorizationInner {
    UpToken(Box<dyn UploadTokenProvider>),
    V1(Box<dyn CredentialProvider>),
    V2(Box<dyn CredentialProvider>),
}

impl Authorization {
    #[inline]
    pub fn uptoken(provider: impl UploadTokenProvider + 'static) -> Self {
        Self {
            inner: AuthorizationInner::UpToken(Box::new(provider)),
        }
    }

    #[inline]
    pub fn v1(provider: impl CredentialProvider + 'static) -> Self {
        Self {
            inner: AuthorizationInner::V1(Box::new(provider)),
        }
    }

    #[inline]
    pub fn v2(provider: impl CredentialProvider + 'static) -> Self {
        Self {
            inner: AuthorizationInner::V2(Box::new(provider)),
        }
    }

    /// 使用指定的鉴权方式对 HTTP 请求进行签名
    pub fn sign(&self, request: &mut SyncRequest) -> AuthorizationResult<()> {
        let authorization = match &self.inner {
            AuthorizationInner::UpToken(provider) => {
                uptoken_authorization(&provider.to_token_string(&Default::default())?)
            }
            AuthorizationInner::V1(provider) => authorization_v1_for_request(
                provider.get(&Default::default())?.credential(),
                request,
            )?,
            AuthorizationInner::V2(provider) => authorization_v2_for_request(
                provider.get(&Default::default())?.credential(),
                request,
            )?,
        };
        set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
        return Ok(());

        fn authorization_v1_for_request(
            credential: &Credential,
            request: &mut SyncRequest,
        ) -> AuthorizationResult<String> {
            let (parts, mut body) = take(request).into_parts();
            credential
                .authorization_v1_for_request_with_body_reader(
                    parts.url(),
                    parts.headers().get(CONTENT_TYPE),
                    &mut body,
                )
                .tap(|_| {
                    *request = SyncRequest::from_parts(parts, body);
                })
                .map_err(|err| err.into())
        }

        fn authorization_v2_for_request(
            credential: &Credential,
            request: &mut SyncRequest,
        ) -> AuthorizationResult<String> {
            let (parts, mut body) = take(request).into_parts();
            credential
                .authorization_v2_for_request_with_body_reader(
                    parts.method(),
                    parts.url(),
                    parts.headers(),
                    &mut body,
                )
                .tap(|_| {
                    *request = SyncRequest::from_parts(parts, body);
                })
                .map_err(|err| err.into())
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    /// 使用指定的鉴权方式对 HTTP 请求进行异步签名
    pub async fn async_sign(&self, request: &mut AsyncRequest<'_>) -> AuthorizationResult<()> {
        let authorization = match &self.inner {
            AuthorizationInner::UpToken(provider) => {
                uptoken_authorization(&provider.async_to_token_string(&Default::default()).await?)
            }
            AuthorizationInner::V1(provider) => {
                authorization_v1_for_request(
                    provider.async_get(&Default::default()).await?.credential(),
                    request,
                )
                .await?
            }
            AuthorizationInner::V2(provider) => {
                authorization_v2_for_request(
                    provider.async_get(&Default::default()).await?.credential(),
                    request,
                )
                .await?
            }
        };
        set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
        return Ok(());

        async fn authorization_v1_for_request(
            credential: &Credential,
            request: &mut AsyncRequest<'_>,
        ) -> AuthorizationResult<String> {
            let (parts, mut body) = take(request).into_parts();
            credential
                .authorization_v1_for_request_with_async_body_reader(
                    parts.url(),
                    parts.headers().get(CONTENT_TYPE),
                    &mut body,
                )
                .await
                .tap(|_| {
                    *request = AsyncRequest::from_parts(parts, body);
                })
                .map_err(|err| err.into())
        }

        async fn authorization_v2_for_request(
            credential: &Credential,
            request: &mut AsyncRequest<'_>,
        ) -> AuthorizationResult<String> {
            let (parts, mut body) = take(request).into_parts();
            credential
                .authorization_v2_for_request_with_async_body_reader(
                    parts.method(),
                    parts.url(),
                    parts.headers(),
                    &mut body,
                )
                .await
                .tap(|_| {
                    *request = AsyncRequest::from_parts(parts, body);
                })
                .map_err(|err| err.into())
        }
    }
}

fn set_authorization(request: &mut RequestParts, authorization: HeaderValue) {
    request.headers_mut().insert(AUTHORIZATION, authorization);
}

fn uptoken_authorization(upload_token: &str) -> String {
    "UpToken ".to_owned() + upload_token
}

impl From<Box<dyn UploadTokenProvider>> for Authorization {
    #[inline]
    fn from(provider: Box<dyn UploadTokenProvider>) -> Self {
        Self::uptoken(provider)
    }
}

impl From<Box<dyn CredentialProvider>> for Authorization {
    #[inline]
    fn from(provider: Box<dyn CredentialProvider>) -> Self {
        Self::v2(provider)
    }
}

/// API 鉴权错误
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AuthorizationError {
    /// 获取认证信息或上传凭证错误
    #[error("Get Upload Token or Credential error: {0}")]
    IoError(#[from] IoError),
    /// URL 解析错误
    #[error("Parse URL error: {0}")]
    UrlParseError(#[from] UrlParseError),
}
/// API 鉴权结果
pub type AuthorizationResult<T> = Result<T, AuthorizationError>;

impl fmt::Debug for Authorization {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}
