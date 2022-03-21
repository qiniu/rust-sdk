use auto_impl::auto_impl;
use dyn_clonable::clonable;
use qiniu_credential::{Credential, CredentialProvider, GetOptions};
use qiniu_http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, RequestParts, SyncRequest,
};
use qiniu_upload_token::UploadTokenProvider;
use std::{fmt::Debug, io::Error as IoError, mem::take, result::Result};
use tap::Tap;
use thiserror::Error;
use url::ParseError as UrlParseError;

#[cfg(feature = "async")]
use {futures::future::BoxFuture, qiniu_http::AsyncRequest};

#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait AuthorizationProvider: Clone + Debug + Sync + Send {
    /// 使用指定的鉴权方式对 HTTP 请求进行签名
    fn sign(&self, request: &mut SyncRequest) -> AuthorizationResult<()>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    /// 使用指定的鉴权方式对 HTTP 请求进行异步签名
    fn async_sign<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AuthorizationResult<()>>;
}

#[derive(Clone, Debug)]
pub struct UploadTokenAuthorization<P: ?Sized>(P);

impl<P> From<P> for UploadTokenAuthorization<P> {
    #[inline]
    fn from(provider: P) -> Self {
        Self(provider)
    }
}

impl<P: UploadTokenProvider + Clone> AuthorizationProvider for UploadTokenAuthorization<P> {
    fn sign(&self, request: &mut SyncRequest) -> AuthorizationResult<()> {
        let authorization = uptoken_authorization(&self.0.to_token_string(&Default::default())?);
        set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
        Ok(())
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_sign<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AuthorizationResult<()>> {
        Box::pin(async move {
            let authorization = uptoken_authorization(&self.0.async_to_token_string(&Default::default()).await?);
            set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
            Ok(())
        })
    }
}

#[derive(Clone, Debug)]
pub struct CredentialAuthorizationV1<P: ?Sized>(P);

impl<P> From<P> for CredentialAuthorizationV1<P> {
    #[inline]
    fn from(provider: P) -> Self {
        Self(provider)
    }
}

impl<P: CredentialProvider + Clone> AuthorizationProvider for CredentialAuthorizationV1<P> {
    fn sign(&self, request: &mut SyncRequest) -> AuthorizationResult<()> {
        _sign(&self.0, request, &Default::default())?;
        return Ok(());

        fn _sign(
            credential_provider: impl CredentialProvider + Clone,
            request: &mut SyncRequest,
            get_options: &GetOptions,
        ) -> AuthorizationResult<()> {
            let authorization = authorization_v1_for_request(&*credential_provider.get(get_options)?, request)?;
            set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
            Ok(())
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_sign<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AuthorizationResult<()>> {
        return Box::pin(async move {
            _sign(&self.0, request, &Default::default()).await?;
            Ok(())
        });

        async fn _sign(
            credential_provider: impl CredentialProvider + Clone,
            request: &mut AsyncRequest<'_>,
            get_options: &GetOptions,
        ) -> AuthorizationResult<()> {
            let authorization =
                authorization_v1_for_async_request(&*credential_provider.async_get(get_options).await?, request)
                    .await?;
            set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
            Ok(())
        }
    }
}

fn authorization_v1_for_request(credential: &Credential, request: &mut SyncRequest) -> AuthorizationResult<String> {
    let (parts, mut body) = take(request).into_parts();
    credential
        .authorization_v1_for_request_with_body_reader(parts.url(), parts.headers().get(CONTENT_TYPE), &mut body)
        .tap(|_| {
            *request = SyncRequest::from_parts(parts, body);
        })
        .map_err(|err| err.into())
}

#[cfg(feature = "async")]
async fn authorization_v1_for_async_request(
    credential: &Credential,
    request: &mut AsyncRequest<'_>,
) -> AuthorizationResult<String> {
    let (parts, mut body) = take(request).into_parts();
    credential
        .authorization_v1_for_request_with_async_body_reader(parts.url(), parts.headers().get(CONTENT_TYPE), &mut body)
        .await
        .tap(|_| {
            *request = AsyncRequest::from_parts(parts, body);
        })
        .map_err(|err| err.into())
}

#[derive(Clone, Debug)]
pub struct CredentialAuthorizationV2<P: ?Sized>(P);

impl<P> From<P> for CredentialAuthorizationV2<P> {
    #[inline]
    fn from(provider: P) -> Self {
        Self(provider)
    }
}

impl<P: CredentialProvider + Clone> AuthorizationProvider for CredentialAuthorizationV2<P> {
    fn sign(&self, request: &mut SyncRequest) -> AuthorizationResult<()> {
        _sign(&self.0, request, &Default::default())?;
        return Ok(());

        fn _sign(
            credential_provider: impl CredentialProvider + Clone,
            request: &mut SyncRequest,
            get_options: &GetOptions,
        ) -> AuthorizationResult<()> {
            let authorization = authorization_v2_for_request(&*credential_provider.get(get_options)?, request)?;
            set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
            Ok(())
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_sign<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AuthorizationResult<()>> {
        return Box::pin(async move {
            _sign(&self.0, request, &Default::default()).await?;
            Ok(())
        });

        async fn _sign(
            credential_provider: impl CredentialProvider + Clone,
            request: &mut AsyncRequest<'_>,
            get_options: &GetOptions,
        ) -> AuthorizationResult<()> {
            let authorization =
                authorization_v2_for_async_request(&*credential_provider.async_get(get_options).await?, request)
                    .await?;
            set_authorization(request, HeaderValue::from_str(&authorization).unwrap());
            Ok(())
        }
    }
}

fn authorization_v2_for_request(credential: &Credential, request: &mut SyncRequest) -> AuthorizationResult<String> {
    let (parts, mut body) = take(request).into_parts();
    credential
        .authorization_v2_for_request_with_body_reader(parts.method(), parts.url(), parts.headers(), &mut body)
        .tap(|_| {
            *request = SyncRequest::from_parts(parts, body);
        })
        .map_err(|err| err.into())
}

#[cfg(feature = "async")]
async fn authorization_v2_for_async_request(
    credential: &Credential,
    request: &mut AsyncRequest<'_>,
) -> AuthorizationResult<String> {
    let (parts, mut body) = take(request).into_parts();
    credential
        .authorization_v2_for_request_with_async_body_reader(parts.method(), parts.url(), parts.headers(), &mut body)
        .await
        .tap(|_| {
            *request = AsyncRequest::from_parts(parts, body);
        })
        .map_err(|err| err.into())
}

fn set_authorization(request: &mut RequestParts, authorization: HeaderValue) {
    request.headers_mut().insert(AUTHORIZATION, authorization);
}

fn uptoken_authorization(upload_token: &str) -> String {
    "UpToken ".to_owned() + upload_token
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

#[derive(Clone, Debug)]
pub enum Authorization<'a> {
    Owned(Box<dyn AuthorizationProvider + 'a>),
    Borrowed(&'a dyn AuthorizationProvider),
}

impl<'a> Authorization<'a> {
    #[inline]
    pub fn from_owned<T: AuthorizationProvider + 'a>(provider: T) -> Self {
        Self::Owned(Box::new(provider))
    }

    #[inline]
    pub fn from_referenced(provider: &'a dyn AuthorizationProvider) -> Self {
        Self::Borrowed(provider)
    }

    #[inline]
    pub fn uptoken(provider: impl UploadTokenProvider + Clone + 'a) -> Self {
        Self::from_owned(UploadTokenAuthorization::from(provider))
    }

    #[inline]
    pub fn v1(provider: impl CredentialProvider + Clone + 'a) -> Self {
        Self::from_owned(CredentialAuthorizationV1::from(provider))
    }

    #[inline]
    pub fn v2(provider: impl CredentialProvider + Clone + 'a) -> Self {
        Self::from_owned(CredentialAuthorizationV2::from(provider))
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
