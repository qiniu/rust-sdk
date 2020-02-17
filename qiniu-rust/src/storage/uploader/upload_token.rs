use super::upload_policy::UploadPolicy;
use crate::{credential::Credential, utils::base64};
use std::{borrow::Cow, convert::From, fmt, result::Result};
use thiserror::Error;

/// 上传凭证
///
/// 可以点击[这里](https://developer.qiniu.com/kodo/manual/1208/upload-token)了解七牛安全机制。
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UploadToken<'p>(UploadTokenInner<'p>);

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Eq, PartialEq)]
enum UploadTokenInner<'p> {
    Token(Cow<'p, str>),
    Policy {
        policy: UploadPolicy<'p>,
        credential: Cow<'p, Credential>,
    },
}

impl<'p> UploadToken<'p> {
    /// 根据上传策略创建新的上传凭证
    pub fn new(policy: UploadPolicy<'p>, credential: impl Into<Cow<'p, Credential>>) -> Self {
        Self(UploadTokenInner::Policy {
            policy,
            credential: credential.into(),
        })
    }

    /// 解析上传凭证，获取 `Access Key`
    pub fn access_key(&self) -> UploadTokenParseResult<&str> {
        match &self.0 {
            UploadTokenInner::Token(token) => token
                .find(':')
                .map(|i| token.split_at(i).0)
                .ok_or_else(|| UploadTokenParseError::InvalidUploadTokenFormat),
            UploadTokenInner::Policy { credential, .. } => Ok(credential.access_key()),
        }
    }

    /// 解析上传凭证，获取上传策略
    pub fn policy<'a>(&'a self) -> UploadTokenParseResult<Cow<'a, UploadPolicy<'p>>> {
        match &self.0 {
            UploadTokenInner::Token(token) => {
                let encoded_policy = token
                    .splitn(3, ':')
                    .last()
                    .ok_or(UploadTokenParseError::InvalidUploadTokenFormat)?;
                let decoded_policy =
                    base64::decode(encoded_policy.as_bytes()).map_err(UploadTokenParseError::Base64DecodeError)?;
                Ok(Cow::Owned(
                    UploadPolicy::from_json(&decoded_policy).map_err(UploadTokenParseError::JSONDecodeError)?,
                ))
            }
            UploadTokenInner::Policy { policy, .. } => Ok(Cow::Borrowed(policy)),
        }
    }
}

impl fmt::Display for UploadToken<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            UploadTokenInner::Token(token) => fmt::Display::fmt(token, f),
            UploadTokenInner::Policy { policy, credential } => {
                fmt::Display::fmt(&credential.sign_upload_policy(policy), f)
            }
        }
    }
}

impl<'p> From<Cow<'p, str>> for UploadToken<'p> {
    fn from(s: Cow<'p, str>) -> Self {
        Self(UploadTokenInner::Token(s))
    }
}

impl From<String> for UploadToken<'_> {
    fn from(s: String) -> Self {
        Self(UploadTokenInner::Token(s.into()))
    }
}

impl<'p> From<&'p str> for UploadToken<'p> {
    fn from(s: &'p str) -> Self {
        Self(UploadTokenInner::Token(s.into()))
    }
}

impl<'p> From<UploadToken<'p>> for String {
    fn from(upload_token: UploadToken<'p>) -> Self {
        upload_token.to_string()
    }
}

/// 上传凭证解析错误
#[derive(Error, Debug)]
pub enum UploadTokenParseError {
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
pub type UploadTokenParseResult<T> = Result<T, UploadTokenParseError>;

#[cfg(test)]
mod tests {
    use super::{super::upload_policy::UploadPolicyBuilder, *};
    use crate::Config;
    use std::{borrow::Cow, boxed::Box, error::Error, result::Result};

    #[test]
    fn test_build_upload_token_from_upload_policy() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_object("test_bucket", "test:file", &Config::default()).build();
        let token = UploadToken::new(policy, Cow::Owned(get_credential())).to_string();
        assert!(token.starts_with(get_credential().access_key()));
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

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
