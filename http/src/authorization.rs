use super::Request;
use qiniu_credential::{Credential, CredentialProvider, Url, UrlParseError};
use qiniu_upload_token::UploadTokenProvider;
use std::{fmt, io::Error as IOError, result::Result, sync::Arc};
use thiserror::Error;

/// API 鉴权方式
#[derive(Clone)]
pub struct Authorization {
    inner: AuthorizationInner,
}

#[derive(Clone, Debug)]
enum AuthorizationInner {
    UpToken(Arc<dyn UploadTokenProvider>),
    V1(Arc<dyn CredentialProvider>),
    V2(Arc<dyn CredentialProvider>),
}

impl Authorization {
    #[inline]
    pub(super) fn uptoken(provider: Arc<dyn UploadTokenProvider>) -> Self {
        Self {
            inner: AuthorizationInner::UpToken(provider),
        }
    }

    #[inline]
    pub(super) fn v1(provider: Arc<dyn CredentialProvider>) -> Self {
        Self {
            inner: AuthorizationInner::V1(provider),
        }
    }

    #[inline]
    pub(super) fn v2(provider: Arc<dyn CredentialProvider>) -> Self {
        Self {
            inner: AuthorizationInner::V2(provider),
        }
    }

    /// 使用指定的鉴权方式对 HTTP 请求进行签名
    pub fn sign(&self, request: &mut Request) -> AuthorizationResult<()> {
        let authorization = match &self.inner {
            AuthorizationInner::UpToken(provider) => uptoken_authorization(&provider.to_string()?),
            AuthorizationInner::V1(provider) => {
                authorization_v1_for_request(&provider.get()?, request)?
            }
            AuthorizationInner::V2(provider) => {
                authorization_v2_for_request(&provider.get()?, request)?
            }
        };
        set_authorization(request, authorization);
        Ok(())
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    /// 使用指定的鉴权方式对 HTTP 请求进行异步签名
    pub async fn async_sign(&self, request: &mut Request<'_>) -> AuthorizationResult<()> {
        let authorization = match &self.inner {
            AuthorizationInner::UpToken(provider) => {
                uptoken_authorization(&provider.async_to_string().await?)
            }
            AuthorizationInner::V1(provider) => {
                authorization_v1_for_request(&provider.async_get().await?, request)?
            }
            AuthorizationInner::V2(provider) => {
                authorization_v2_for_request(&provider.async_get().await?, request)?
            }
        };
        set_authorization(request, authorization);
        Ok(())
    }
}

#[inline]
fn set_authorization(request: &mut Request, authorization: String) {
    request
        .headers_mut()
        .insert("Authorization".into(), authorization.into());
}

#[inline]
fn uptoken_authorization(upload_token: &str) -> String {
    "UpToken ".to_owned() + upload_token
}

#[inline]
fn authorization_v1_for_request(
    credential: &Credential,
    request: &Request,
) -> AuthorizationResult<String> {
    Ok(credential.authorization_v1_for_request(
        &Url::parse(request.url())?,
        request
            .headers()
            .get(&"Content-Type".into())
            .unwrap_or(&"".into()),
        request.body(),
    ))
}

#[inline]
fn authorization_v2_for_request(
    credential: &Credential,
    request: &Request,
) -> AuthorizationResult<String> {
    Ok(credential.authorization_v2_for_request(
        request.method(),
        &Url::parse(request.url())?,
        request.headers(),
        request.body(),
    ))
}

impl From<Arc<dyn UploadTokenProvider>> for Authorization {
    #[inline]
    fn from(provider: Arc<dyn UploadTokenProvider>) -> Self {
        Self::uptoken(provider)
    }
}

impl From<Arc<dyn CredentialProvider>> for Authorization {
    #[inline]
    fn from(provider: Arc<dyn CredentialProvider>) -> Self {
        Self::v2(provider)
    }
}

/// API 鉴权错误
#[derive(Error, Debug)]
pub enum AuthorizationError {
    /// 获取认证信息或上传凭证错误
    #[error("Get Upload Token or Credential error: {0}")]
    IOError(#[from] IOError),
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
