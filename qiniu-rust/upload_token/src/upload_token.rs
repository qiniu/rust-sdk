use super::{UploadPolicy, UploadPolicyBuilder};
use assert_impl::assert_impl;
use once_cell::sync::OnceCell;
use qiniu_credential::AsCredential;
use qiniu_utils::base64;
use std::{borrow::Cow, ffi::c_void, fmt, sync::Arc, time::Duration};
use thiserror::Error;

/// 上传凭证
///
/// 可以点击[这里](https://developer.qiniu.com/kodo/manual/1208/upload-token)了解七牛安全机制。
#[derive(Debug, Clone)]
pub struct UploadToken(Arc<UploadTokenInner>);

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
enum UploadTokenInner {
    Token {
        token: Box<str>,
        policy: OnceCell<UploadPolicy>,
        access_key: OnceCell<Box<str>>,
    },
    Policy {
        policy: UploadPolicy,
        credential: Arc<dyn AsCredential>,
        token: OnceCell<Box<str>>,
    },
    Bucket {
        bucket: Cow<'static, str>,
        upload_token_lifetime: Duration,
        credential: Arc<dyn AsCredential>,
    },
}

impl UploadToken {
    /// 根据上传策略创建新的上传凭证
    pub fn new(policy: UploadPolicy, credential: Arc<dyn AsCredential>) -> Self {
        Self(Arc::new(UploadTokenInner::Policy {
            policy,
            credential,
            token: OnceCell::new(),
        }))
    }

    /// 根据存储空间创建新的上传凭证
    pub fn new_from_bucket(
        bucket: impl Into<Cow<'static, str>>,
        credential: Arc<dyn AsCredential>,
        upload_token_lifetime: Duration,
    ) -> Self {
        Self(Arc::new(UploadTokenInner::Bucket {
            credential,
            upload_token_lifetime,
            bucket: bucket.into(),
        }))
    }

    /// 解析上传凭证，获取 `Access Key`
    pub fn access_key(&self) -> ParseResult<Cow<str>> {
        match self.0.as_ref() {
            UploadTokenInner::Token {
                token, access_key, ..
            } => access_key
                .get_or_try_init(|| {
                    token
                        .find(':')
                        .map(|i| token.split_at(i).0.to_owned().into())
                        .ok_or_else(|| ParseError::InvalidUploadTokenFormat)
                })
                .map(|access_key| access_key.as_ref().into()),
            UploadTokenInner::Policy { credential, .. }
            | UploadTokenInner::Bucket { credential, .. } => Ok(credential.get().access_key),
        }
    }

    /// 解析上传凭证，获取上传策略
    pub fn policy(&self) -> ParseResult<Cow<UploadPolicy>> {
        match self.0.as_ref() {
            UploadTokenInner::Token { token, policy, .. } => policy
                .get_or_try_init(|| {
                    let encoded_policy = token
                        .splitn(3, ':')
                        .last()
                        .ok_or(ParseError::InvalidUploadTokenFormat)?;
                    let decoded_policy = base64::decode(encoded_policy.as_bytes())
                        .map_err(ParseError::Base64DecodeError)?;
                    Ok(UploadPolicy::from_json(&decoded_policy)
                        .map_err(ParseError::JSONDecodeError)?)
                })
                .map(|policy| policy.into()),
            UploadTokenInner::Policy { policy, .. } => Ok(policy.into()),
            UploadTokenInner::Bucket {
                bucket,
                upload_token_lifetime,
                ..
            } => Ok(UploadPolicyBuilder::new_policy_for_bucket(
                bucket.to_string(),
                *upload_token_lifetime,
            )
            .build()
            .into()),
        }
    }

    #[doc(hidden)]
    pub fn into_raw(self) -> *const c_void {
        Arc::into_raw(self.0).cast()
    }

    #[doc(hidden)]
    pub unsafe fn from_raw(ptr: *const c_void) -> Self {
        Self(Arc::from_raw(ptr.cast::<UploadTokenInner>()))
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl fmt::Display for UploadToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0.as_ref() {
            UploadTokenInner::Token { token, .. } => token.fmt(f),
            UploadTokenInner::Policy {
                policy,
                credential,
                token,
            } => token
                .get_or_init(|| {
                    credential
                        .sign_with_data(policy.as_json().as_bytes())
                        .into()
                })
                .fmt(f),
            UploadTokenInner::Bucket {
                bucket,
                upload_token_lifetime,
                credential,
            } => credential
                .sign_with_data(
                    UploadPolicyBuilder::new_policy_for_bucket(
                        bucket.to_string(),
                        *upload_token_lifetime,
                    )
                    .build()
                    .as_json()
                    .as_bytes(),
                )
                .fmt(f),
        }
    }
}

impl<'p> From<Cow<'p, str>> for UploadToken {
    fn from(s: Cow<'p, str>) -> Self {
        Self(Arc::new(UploadTokenInner::Token {
            token: s.into_owned().into(),
            policy: OnceCell::new(),
            access_key: OnceCell::new(),
        }))
    }
}

impl From<String> for UploadToken {
    fn from(s: String) -> Self {
        Self(Arc::new(UploadTokenInner::Token {
            token: s.into(),
            policy: OnceCell::new(),
            access_key: OnceCell::new(),
        }))
    }
}

impl<'p> From<&'p str> for UploadToken {
    fn from(s: &'p str) -> Self {
        Self(Arc::new(UploadTokenInner::Token {
            token: s.into(),
            policy: OnceCell::new(),
            access_key: OnceCell::new(),
        }))
    }
}

impl<'p> From<&'p UploadToken> for Cow<'p, UploadToken> {
    #[inline]
    fn from(token: &'p UploadToken) -> Self {
        Cow::Borrowed(token)
    }
}

impl From<UploadToken> for Cow<'_, UploadToken> {
    #[inline]
    fn from(token: UploadToken) -> Self {
        Cow::Owned(token)
    }
}

impl From<UploadToken> for String {
    #[inline]
    fn from(upload_token: UploadToken) -> Self {
        upload_token.to_string()
    }
}

/// 上传凭证解析错误
#[derive(Error, Debug)]
pub enum ParseError {
    /// 上传凭证格式错误
    #[error("Invalid upload token format")]
    InvalidUploadTokenFormat,
    /// 上传凭证 Base64 解码错误
    #[error("Base64 decode error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
    /// 上传凭证 JSON 解析错误
    #[error("JSON decode error: {0}")]
    JSONDecodeError(#[from] serde_json::Error),
}

/// 上传凭证解析结果
pub type ParseResult<T> = Result<T, ParseError>;

#[cfg(test)]
mod tests {
    use super::{super::UploadPolicyBuilder, *};
    use qiniu_credential::StaticCredential;
    use std::{boxed::Box, error::Error, result::Result};

    #[test]
    fn test_build_upload_token_from_upload_policy() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_object(
            "test_bucket",
            "test:file",
            Duration::from_secs(3600),
        )
        .build();
        let token = UploadToken::new(policy, get_credential()).to_string();
        assert!(token.starts_with(get_credential().get().access_key.as_ref()));
        let token = UploadToken::from(token);
        let policy = token.policy()?;
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), Some("test:file"));
        accept_string(token.to_owned().into());
        accept_upload_token(&token.to_string().into());
        accept_upload_token(&token.to_string().as_str().into());
        Ok(())
    }

    fn accept_string(_: String) {}
    fn accept_upload_token(_: &UploadToken) {}

    fn get_credential() -> Arc<dyn AsCredential> {
        Arc::new(StaticCredential::new("abcdefghklmnopq", "1234567890"))
    }
}
