use super::{UploadPolicy, UploadPolicyBuilder};
use once_cell::sync::OnceCell;
use qiniu_credential::CredentialProvider;
use qiniu_utils::base64;
use std::{
    any::Any,
    borrow::Cow,
    fmt::{self, Debug},
    io::{Error as IOError, Result as IOResult},
    sync::RwLock,
    time::{Duration, Instant},
};
use thiserror::Error;

#[cfg(feature = "async")]
use {
    futures::lock::Mutex as AsyncMutex,
    std::{future::Future, pin::Pin},
};

#[cfg(feature = "async")]
type AsyncParseResult<'a, T> = Pin<Box<dyn Future<Output = ParseResult<T>> + 'a + Send>>;

#[cfg(feature = "async")]
type AsyncIOResult<'a, T> = Pin<Box<dyn Future<Output = IOResult<T>> + 'a + Send>>;

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
    fn async_access_key(&self) -> AsyncParseResult<Cow<str>> {
        Box::pin(async move { self.access_key() })
    }

    /// 从上传凭证内获取上传策略
    fn policy(&self) -> ParseResult<Cow<UploadPolicy>>;

    /// 异步从上传凭证内获取上传策略
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_policy(&self) -> AsyncParseResult<Cow<UploadPolicy>> {
        Box::pin(async move { self.policy() })
    }

    /// 生成字符串
    fn to_string(&self) -> IOResult<Cow<str>>;

    /// 异步生成字符串
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_to_string(&self) -> AsyncIOResult<Cow<str>> {
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

impl Debug for StaticUploadTokenProvider {
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
    fn to_string(&self) -> IOResult<Cow<str>> {
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
    pub(super) fn new(
        upload_policy: UploadPolicy,
        credential: Box<dyn CredentialProvider>,
    ) -> Self {
        Self {
            upload_policy,
            credential,
            upload_token: OnceCell::new(),
        }
    }
}

impl Debug for FromUploadPolicy {
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

    fn to_string(&self) -> IOResult<Cow<str>> {
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

/// 基于对象的动态生成
///
/// 根据对象的快速生成上传凭证实例
pub struct ObjectUploadTokenProvider {
    bucket: Cow<'static, str>,
    object: Cow<'static, str>,
    upload_token_lifetime: Duration,
    credential: Box<dyn CredentialProvider>,
}

impl ObjectUploadTokenProvider {
    /// 基于存储空间和对象名称和认证信息动态生成上传凭证实例
    #[inline]
    pub fn new(
        bucket: impl Into<Cow<'static, str>>,
        object: impl Into<Cow<'static, str>>,
        upload_token_lifetime: Duration,
        credential: Box<dyn CredentialProvider>,
    ) -> Self {
        Self {
            bucket: bucket.into(),
            object: object.into(),
            upload_token_lifetime,
            credential,
        }
    }
}

impl Debug for ObjectUploadTokenProvider {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ObjectUploadTokenProvider")
            .field("bucket", &self.bucket)
            .field("object", &self.object)
            .field("upload_token_lifetime", &self.upload_token_lifetime)
            .finish()
    }
}

impl UploadTokenProvider for ObjectUploadTokenProvider {
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

    fn to_string(&self) -> IOResult<Cow<str>> {
        let upload_token = self.credential.get()?.sign_with_data(
            UploadPolicyBuilder::new_policy_for_object(
                self.bucket.to_string(),
                self.object.to_string(),
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

#[derive(Debug)]
struct Cache<T> {
    cached_at: Instant,
    value: T,
}

#[derive(Debug, Default)]
struct SyncCache {
    access_key: RwLock<Option<Cache<String>>>,
    upload_policy: RwLock<Option<Cache<UploadPolicy>>>,
    upload_token: RwLock<Option<Cache<String>>>,
}

#[cfg(feature = "async")]
#[derive(Debug, Default)]
struct AsyncCache {
    access_key: AsyncMutex<Option<Cache<String>>>,
    upload_policy: AsyncMutex<Option<Cache<UploadPolicy>>>,
    upload_token: AsyncMutex<Option<Cache<String>>>,
}

#[derive(Debug)]
pub struct CachedUploadTokenProvider<P: UploadTokenProvider> {
    inner_provider: P,
    cache_lifetime: Duration,
    sync_cache: SyncCache,

    #[cfg(feature = "async")]
    async_cache: AsyncCache,
}

impl<P: UploadTokenProvider> CachedUploadTokenProvider<P> {
    #[inline]
    pub fn new(inner_provider: P, cache_lifetime: Duration) -> Self {
        Self {
            inner_provider,
            cache_lifetime,
            sync_cache: Default::default(),

            #[cfg(feature = "async")]
            async_cache: Default::default(),
        }
    }
}

macro_rules! sync_method {
    ($provider:expr, $cache_field:ident, $method_name:ident, $return_type:ty) => {{
        let cache = $provider.sync_cache.$cache_field.read().unwrap();
        return if let Some(cache) = &*cache {
            if cache.cached_at.elapsed() < $provider.cache_lifetime {
                Ok(cache.value.to_owned().into())
            } else {
                drop(cache);
                update_cache(&$provider)
            }
        } else {
            drop(cache);
            update_cache(&$provider)
        };

        fn update_cache(
            provider: &CachedUploadTokenProvider<impl UploadTokenProvider>,
        ) -> $return_type {
            let mut cache = provider.sync_cache.$cache_field.write().unwrap();
            if let Some(cache) = &*cache {
                if cache.cached_at.elapsed() < provider.cache_lifetime {
                    return Ok(cache.value.to_owned().into());
                }
            }
            match provider.inner_provider.$method_name() {
                Ok(value) => {
                    *cache = Some(Cache {
                        cached_at: Instant::now(),
                        value: value.to_owned().into(),
                    });
                    Ok(value)
                }
                Err(err) => Err(err),
            }
        }
    }};
}

#[cfg(feature = "async")]
macro_rules! async_method {
    ($provider:expr, $cache_field:ident, $method_name:ident) => {{
        Box::pin(async move {
            let mut cache = $provider.async_cache.$cache_field.lock().await;
            if let Some(cache) = &*cache {
                if cache.cached_at.elapsed() < $provider.cache_lifetime {
                    return Ok(cache.value.to_owned().into());
                }
            }
            match $provider.inner_provider.$method_name().await {
                Ok(value) => {
                    *cache = Some(Cache {
                        cached_at: Instant::now(),
                        value: value.to_owned().into(),
                    });
                    Ok(value)
                }
                Err(err) => Err(err),
            }
        })
    }};
}

impl<P: UploadTokenProvider> UploadTokenProvider for CachedUploadTokenProvider<P> {
    fn access_key(&self) -> ParseResult<Cow<str>> {
        sync_method!(self, access_key, access_key, ParseResult<Cow<str>>)
    }

    fn policy(&self) -> ParseResult<Cow<UploadPolicy>> {
        sync_method!(self, upload_policy, policy, ParseResult<Cow<UploadPolicy>>)
    }

    fn to_string(&self) -> IOResult<Cow<str>> {
        sync_method!(self, upload_token, to_string, IOResult<Cow<str>>)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_access_key(&self) -> AsyncParseResult<Cow<str>> {
        async_method!(self, access_key, async_access_key)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_policy(&self) -> AsyncParseResult<Cow<UploadPolicy>> {
        async_method!(self, upload_policy, async_policy)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_to_string(&self) -> AsyncIOResult<Cow<str>> {
        async_method!(self, upload_token, async_to_string)
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
#[non_exhaustive]
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

#[cfg(test)]
mod tests {
    use super::{super::UploadPolicyBuilder, *};
    use async_std as _;
    use clap as _;
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
