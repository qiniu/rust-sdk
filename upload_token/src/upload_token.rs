use super::{UploadPolicy, UploadPolicyBuilder};
use once_cell::sync::OnceCell;
use qiniu_credential::CredentialProvider;
use qiniu_utils::base64;
use std::{
    any::Any,
    borrow::Cow,
    fmt::{self, Debug},
    io::Error as IOError,
    time::Duration,
};
use thiserror::Error;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// 上传凭证提供者
///
/// 可以点击[这里](https://developer.qiniu.com/kodo/manual/1208/upload-token)了解七牛安全机制。
pub trait UploadTokenProvider: Any + Debug + Sync + Send {
    /// 从上传凭证内获取 AccessKey
    fn access_key(&self) -> ParseResult<Cow<str>>;

    /// 异步从上传凭证内获取 AccessKey
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_access_key(&self) -> BoxFuture<ParseResult<Cow<str>>> {
        Box::pin(async move { self.access_key() })
    }

    /// 从上传凭证内获取上传策略
    fn policy(&self) -> ParseResult<Cow<UploadPolicy>>;

    /// 异步从上传凭证内获取上传策略
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_policy(&self) -> BoxFuture<ParseResult<Cow<UploadPolicy>>> {
        Box::pin(async move { self.policy() })
    }

    /// 生成字符串
    fn to_string(&self) -> GenerateResult<Cow<str>>;

    /// 异步生成字符串
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_to_string(&self) -> BoxFuture<GenerateResult<Cow<str>>> {
        Box::pin(async move { self.to_string() })
    }

    fn as_upload_token_provider(&self) -> &dyn UploadTokenProvider;
    fn as_any(&self) -> &dyn Any;
}

/// 静态上传凭证提供者
///
/// 根据已经被生成好的上传凭证字符串生成上传凭证提供者实例，可以将上传凭证解析为 Access Token 和上传策略
pub struct StaticUploadTokenProvider {
    upload_token: Box<str>,
    policy: OnceCell<UploadPolicy>,
    access_key: OnceCell<Box<str>>,
}

impl StaticUploadTokenProvider {
    /// 构建一个静态上传凭证，只需要传入静态的上传凭证字符串即可
    pub fn new(upload_token: impl Into<String>) -> Self {
        Self {
            upload_token: upload_token.into().into_boxed_str(),
            policy: OnceCell::new(),
            access_key: OnceCell::new(),
        }
    }
}

impl fmt::Debug for StaticUploadTokenProvider {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("StaticUploadTokenProvider")
            .field("upload_token", &self.upload_token)
            .finish()
    }
}

impl UploadTokenProvider for StaticUploadTokenProvider {
    fn access_key(&self) -> ParseResult<Cow<str>> {
        self.access_key
            .get_or_try_init(|| {
                self.upload_token
                    .find(':')
                    .map(|i| self.upload_token.split_at(i).0.to_owned().into())
                    .ok_or_else(|| ParseError::InvalidUploadTokenFormat)
            })
            .map(|access_key| access_key.as_ref().into())
    }

    fn policy(&self) -> ParseResult<Cow<UploadPolicy>> {
        self.policy
            .get_or_try_init(|| {
                let encoded_policy = self
                    .upload_token
                    .splitn(3, ':')
                    .last()
                    .ok_or(ParseError::InvalidUploadTokenFormat)?;
                let decoded_policy = base64::decode(encoded_policy.as_bytes())
                    .map_err(ParseError::Base64DecodeError)?;
                Ok(
                    UploadPolicy::from_json(&decoded_policy)
                        .map_err(ParseError::JSONDecodeError)?,
                )
            })
            .map(|policy| policy.into())
    }

    #[inline]
    fn to_string(&self) -> GenerateResult<Cow<str>> {
        Ok(Cow::Borrowed(&self.upload_token))
    }

    #[inline]
    fn as_upload_token_provider(&self) -> &dyn UploadTokenProvider {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: Into<String>> From<T> for StaticUploadTokenProvider {
    #[inline]
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

pub(super) struct FromUploadPolicy {
    upload_policy: UploadPolicy,
    credential: Box<dyn CredentialProvider>,
    upload_token: OnceCell<Box<str>>,
}

impl FromUploadPolicy {
    /// 基于上传策略和认证信息生成上传凭证实例
    pub fn new(upload_policy: UploadPolicy, credential: Box<dyn CredentialProvider>) -> Self {
        Self {
            upload_policy,
            credential,
            upload_token: OnceCell::new(),
        }
    }
}

impl fmt::Debug for FromUploadPolicy {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FromUploadPolicy")
            .field("upload_policy", &self.upload_policy)
            .finish()
    }
}

impl UploadTokenProvider for FromUploadPolicy {
    #[inline]
    fn access_key(&self) -> ParseResult<Cow<str>> {
        Ok(self.credential.get()?.into_pair().0)
    }

    #[inline]
    fn policy(&self) -> ParseResult<Cow<UploadPolicy>> {
        Ok(Cow::Borrowed(&self.upload_policy))
    }

    fn to_string(&self) -> GenerateResult<Cow<str>> {
        let upload_token = self.upload_token.get_or_try_init::<_, IOError>(|| {
            Ok(self
                .credential
                .get()?
                .sign_with_data(self.upload_policy.as_json().as_bytes())
                .into_boxed_str())
        })?;
        Ok(Cow::Borrowed(upload_token))
    }

    #[inline]
    fn as_upload_token_provider(&self) -> &dyn UploadTokenProvider {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// 基于存储空间动态生成
///
/// 根据存储空间快速生成上传凭证实例
pub struct BucketUploadTokenProvider {
    bucket: Cow<'static, str>,
    upload_token_lifetime: Duration,
    credential: Box<dyn CredentialProvider>,
}

impl BucketUploadTokenProvider {
    /// 基于存储空间名称和认证信息动态生成上传凭证实例
    pub fn new(
        bucket: impl Into<Cow<'static, str>>,
        upload_token_lifetime: Duration,
        credential: Box<dyn CredentialProvider>,
    ) -> Self {
        Self {
            bucket: bucket.into(),
            upload_token_lifetime,
            credential,
        }
    }
}

impl fmt::Debug for BucketUploadTokenProvider {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BucketUploadTokenProvider")
            .field("bucket", &self.bucket)
            .field("upload_token_lifetime", &self.upload_token_lifetime)
            .finish()
    }
}

impl UploadTokenProvider for BucketUploadTokenProvider {
    #[inline]
    fn access_key(&self) -> ParseResult<Cow<str>> {
        Ok(self.credential.get()?.into_pair().0)
    }

    fn policy(&self) -> ParseResult<Cow<UploadPolicy>> {
        Ok(UploadPolicyBuilder::new_policy_for_bucket(
            self.bucket.to_string(),
            self.upload_token_lifetime,
        )
        .build()
        .into())
    }

    fn to_string(&self) -> GenerateResult<Cow<str>> {
        let upload_token = self.credential.get()?.sign_with_data(
            UploadPolicyBuilder::new_policy_for_bucket(
                self.bucket.to_string(),
                self.upload_token_lifetime,
            )
            .build()
            .as_json()
            .as_bytes(),
        );
        Ok(upload_token.into())
    }

    #[inline]
    fn as_upload_token_provider(&self) -> &dyn UploadTokenProvider {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
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
    /// 上传凭证获取认证信息错误
    #[error("Credential get error: {0}")]
    CredentialGetError(#[from] IOError),
}

/// 上传凭证解析结果
pub type ParseResult<T> = Result<T, ParseError>;

/// 上传凭证生成错误
#[derive(Error, Debug)]
pub enum GenerateError {
    /// 上传凭证获取认证信息错误
    #[error("Credential get error: {0}")]
    CredentialGetError(#[from] IOError),
}

/// 上传凭证解析结果
pub type GenerateResult<T> = Result<T, GenerateError>;

#[cfg(test)]
mod tests {
    use super::{super::UploadPolicyBuilder, *};
    use qiniu_credential::StaticCredentialProvider;
    use std::{boxed::Box, error::Error, result::Result};

    #[test]
    fn test_build_upload_token_from_upload_policy() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_object(
            "test_bucket",
            "test:file",
            Duration::from_secs(3600),
        )
        .build();
        let token = FromUploadPolicy::new(policy, get_credential())
            .to_string()?
            .into_owned();
        assert!(token.starts_with(get_credential().get()?.access_key()));
        let token = StaticUploadTokenProvider::from(token);
        let policy = token.policy()?;
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), Some("test:file"));
        Ok(())
    }

    #[cfg(feature = "async")]
    mod async_test {
        use super::*;

        #[async_std::test]
        async fn test_async_build_upload_token_from_upload_policy() -> Result<(), Box<dyn Error>> {
            let policy = UploadPolicyBuilder::new_policy_for_object(
                "test_bucket",
                "test:file",
                Duration::from_secs(3600),
            )
            .build();
            let token = FromUploadPolicy::new(policy, get_credential())
                .async_to_string()
                .await?
                .into_owned();
            assert!(token.starts_with(get_credential().async_get().await?.access_key()));
            let token = StaticUploadTokenProvider::from(token);
            let policy = token.async_policy().await?;
            assert_eq!(policy.bucket(), Some("test_bucket"));
            assert_eq!(policy.key(), Some("test:file"));
            Ok(())
        }
    }

    fn get_credential() -> Box<dyn CredentialProvider> {
        Box::new(StaticCredentialProvider::new(
            "abcdefghklmnopq",
            "1234567890",
        ))
    }
}
