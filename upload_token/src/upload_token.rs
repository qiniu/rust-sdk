use super::{UploadPolicy, UploadPolicyBuilder};
use once_cell::sync::OnceCell;
use qiniu_credential::{AccessKey, CredentialProvider};
use qiniu_utils::{base64, BucketName, ObjectName};
use std::{
    any::Any,
    borrow::Cow,
    fmt::{self, Debug},
    io::{Error as IOError, Result as IOResult},
    sync::RwLock,
    time::{Duration, Instant},
};
use tap::Tap;
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
    fn access_key(&self) -> ParseResult<AccessKey>;

    /// 异步从上传凭证内获取 AccessKey
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_access_key(&self) -> AsyncParseResult<AccessKey> {
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
    fn access_key(&self) -> ParseResult<AccessKey> {
        self.access_key
            .get_or_try_init(|| {
                self.upload_token
                    .find(':')
                    .map(|i| self.upload_token.split_at(i).0.to_owned().into())
                    .ok_or(ParseError::InvalidUploadTokenFormat)
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
                UploadPolicy::from_json(&decoded_policy).map_err(ParseError::JSONDecodeError)
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

#[derive(Debug)]
pub(super) struct FromUploadPolicy<C> {
    upload_policy: UploadPolicy,
    credential: C,
}

impl<C> FromUploadPolicy<C> {
    /// 基于上传策略和认证信息生成上传凭证实例
    pub(super) fn new(upload_policy: UploadPolicy, credential: C) -> Self {
        Self {
            upload_policy,
            credential,
        }
    }
}

impl<C: CredentialProvider> UploadTokenProvider for FromUploadPolicy<C> {
    #[inline]
    fn access_key(&self) -> ParseResult<AccessKey> {
        Ok(self.credential.get()?.into_pair().0)
    }

    #[inline]
    fn policy(&self) -> ParseResult<Cow<UploadPolicy>> {
        Ok(Cow::Borrowed(&self.upload_policy))
    }

    fn to_string(&self) -> IOResult<Cow<str>> {
        Ok(Cow::Owned(
            self.credential
                .get()?
                .sign_with_data(self.upload_policy.as_json().as_bytes()),
        ))
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

pub type OnPolicyGeneratedCallback = Box<dyn Fn(&mut UploadPolicyBuilder) + Sync + Send + 'static>;

/// 基于存储空间的动态生成
///
/// 根据存储空间的快速生成上传凭证实例
pub struct BucketUploadTokenProvider<C> {
    bucket: BucketName,
    upload_token_lifetime: Duration,
    credential: C,
    on_policy_generated: Option<OnPolicyGeneratedCallback>,
}

impl<C> BucketUploadTokenProvider<C> {
    /// 基于存储空间和认证信息动态生成上传凭证实例
    #[inline]
    pub fn new(
        bucket: impl Into<BucketName>,
        upload_token_lifetime: Duration,
        credential: C,
    ) -> Self {
        Self::builder(bucket, upload_token_lifetime, credential).build()
    }

    #[inline]
    pub fn builder(
        bucket: impl Into<BucketName>,
        upload_token_lifetime: Duration,
        credential: C,
    ) -> BucketUploadTokenProviderBuilder<C> {
        BucketUploadTokenProviderBuilder {
            inner: Self {
                bucket: bucket.into(),
                upload_token_lifetime,
                credential,
                on_policy_generated: None,
            },
        }
    }

    #[inline]
    fn make_policy(&self) -> UploadPolicy {
        UploadPolicyBuilder::new_policy_for_bucket(
            self.bucket.to_string(),
            self.upload_token_lifetime,
        )
        .tap_mut(|policy| {
            if let Some(on_policy_generated) = self.on_policy_generated.as_ref() {
                on_policy_generated(policy);
            }
        })
        .build()
    }
}

impl<C> Debug for BucketUploadTokenProvider<C> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BucketUploadTokenProvider")
            .field("bucket", &self.bucket)
            .field("upload_token_lifetime", &self.upload_token_lifetime)
            .finish()
    }
}

impl<C: CredentialProvider> UploadTokenProvider for BucketUploadTokenProvider<C> {
    #[inline]
    fn access_key(&self) -> ParseResult<AccessKey> {
        Ok(self.credential.get()?.into_pair().0)
    }

    fn policy(&self) -> ParseResult<Cow<UploadPolicy>> {
        Ok(self.make_policy().into())
    }

    fn to_string(&self) -> IOResult<Cow<str>> {
        let upload_token = self
            .credential
            .get()?
            .sign_with_data(self.make_policy().as_json().as_bytes());
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

pub struct BucketUploadTokenProviderBuilder<C> {
    inner: BucketUploadTokenProvider<C>,
}

impl<C> BucketUploadTokenProviderBuilder<C> {
    #[inline]
    pub fn on_policy_generated(mut self, callback: OnPolicyGeneratedCallback) -> Self {
        self.inner.on_policy_generated = Some(callback);
        self
    }

    #[inline]
    pub fn build(self) -> BucketUploadTokenProvider<C> {
        self.inner
    }
}

impl<C> Debug for BucketUploadTokenProviderBuilder<C> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BucketUploadTokenProviderBuilder")
            .field("bucket", &self.inner.bucket)
            .field("upload_token_lifetime", &self.inner.upload_token_lifetime)
            .finish()
    }
}

/// 基于对象的动态生成
///
/// 根据对象的快速生成上传凭证实例
pub struct ObjectUploadTokenProvider<C> {
    bucket: BucketName,
    object: ObjectName,
    upload_token_lifetime: Duration,
    credential: C,
    on_policy_generated: Option<OnPolicyGeneratedCallback>,
}

impl<C> ObjectUploadTokenProvider<C> {
    /// 基于存储空间和对象名称和认证信息动态生成上传凭证实例
    #[inline]
    pub fn new(
        bucket: impl Into<BucketName>,
        object: impl Into<ObjectName>,
        upload_token_lifetime: Duration,
        credential: C,
    ) -> Self {
        Self::builder(bucket, object, upload_token_lifetime, credential).build()
    }

    #[inline]
    pub fn builder(
        bucket: impl Into<BucketName>,
        object: impl Into<ObjectName>,
        upload_token_lifetime: Duration,
        credential: C,
    ) -> ObjectUploadTokenProviderBuilder<C> {
        ObjectUploadTokenProviderBuilder {
            inner: Self {
                bucket: bucket.into(),
                object: object.into(),
                upload_token_lifetime,
                credential,
                on_policy_generated: None,
            },
        }
    }

    #[inline]
    fn make_policy(&self) -> UploadPolicy {
        UploadPolicyBuilder::new_policy_for_object(
            self.bucket.to_string(),
            self.object.to_string(),
            self.upload_token_lifetime,
        )
        .tap_mut(|policy| {
            if let Some(on_policy_generated) = self.on_policy_generated.as_ref() {
                on_policy_generated(policy);
            }
        })
        .build()
    }
}

impl<C> Debug for ObjectUploadTokenProvider<C> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ObjectUploadTokenProvider")
            .field("bucket", &self.bucket)
            .field("object", &self.object)
            .field("upload_token_lifetime", &self.upload_token_lifetime)
            .finish()
    }
}

impl<C: CredentialProvider> UploadTokenProvider for ObjectUploadTokenProvider<C> {
    #[inline]
    fn access_key(&self) -> ParseResult<AccessKey> {
        Ok(self.credential.get()?.into_pair().0)
    }

    fn policy(&self) -> ParseResult<Cow<UploadPolicy>> {
        Ok(self.make_policy().into())
    }

    fn to_string(&self) -> IOResult<Cow<str>> {
        let upload_token = self
            .credential
            .get()?
            .sign_with_data(self.make_policy().as_json().as_bytes());
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

pub struct ObjectUploadTokenProviderBuilder<C> {
    inner: ObjectUploadTokenProvider<C>,
}

impl<C> ObjectUploadTokenProviderBuilder<C> {
    #[inline]
    pub fn on_policy_generated(mut self, callback: OnPolicyGeneratedCallback) -> Self {
        self.inner.on_policy_generated = Some(callback);
        self
    }

    #[inline]
    pub fn build(self) -> ObjectUploadTokenProvider<C> {
        self.inner
    }
}

impl<C> Debug for ObjectUploadTokenProviderBuilder<C> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ObjectUploadTokenProviderBuilder")
            .field("bucket", &self.inner.bucket)
            .field("object", &self.inner.object)
            .field("upload_token_lifetime", &self.inner.upload_token_lifetime)
            .finish()
    }
}

#[derive(Debug)]
struct Cache<T> {
    cached_at: Instant,
    value: T,
}

#[derive(Debug, Default)]
struct SyncCache {
    access_key: RwLock<Option<Cache<AccessKey>>>,
    upload_policy: RwLock<Option<Cache<UploadPolicy>>>,
    upload_token: RwLock<Option<Cache<String>>>,
}

#[cfg(feature = "async")]
#[derive(Debug, Default)]
struct AsyncCache {
    access_key: AsyncMutex<Option<Cache<AccessKey>>>,
    upload_policy: AsyncMutex<Option<Cache<UploadPolicy>>>,
    upload_token: AsyncMutex<Option<Cache<String>>>,
}

#[derive(Debug)]
pub struct CachedUploadTokenProvider<P> {
    inner_provider: P,
    cache_lifetime: Duration,
    sync_cache: SyncCache,

    #[cfg(feature = "async")]
    async_cache: AsyncCache,
}

impl<P> CachedUploadTokenProvider<P> {
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
        let guard = $provider.sync_cache.$cache_field.read().unwrap();
        return if let Some(cache) = &*guard {
            if cache.cached_at.elapsed() < $provider.cache_lifetime {
                Ok(cache.value.to_owned().into())
            } else {
                drop(guard);
                update_cache(&$provider)
            }
        } else {
            drop(guard);
            update_cache(&$provider)
        };

        fn update_cache(
            provider: &CachedUploadTokenProvider<impl UploadTokenProvider>,
        ) -> $return_type {
            let mut guard = provider.sync_cache.$cache_field.write().unwrap();
            if let Some(cache) = &*guard {
                if cache.cached_at.elapsed() < provider.cache_lifetime {
                    return Ok(cache.value.to_owned().into());
                }
            }
            match provider.inner_provider.$method_name() {
                Ok(value) => {
                    *guard = Some(Cache {
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
    fn access_key(&self) -> ParseResult<AccessKey> {
        sync_method!(self, access_key, access_key, ParseResult<AccessKey>)
    }

    fn policy(&self) -> ParseResult<Cow<UploadPolicy>> {
        sync_method!(self, upload_policy, policy, ParseResult<Cow<UploadPolicy>>)
    }

    fn to_string(&self) -> IOResult<Cow<str>> {
        sync_method!(self, upload_token, to_string, IOResult<Cow<str>>)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_access_key(&self) -> AsyncParseResult<AccessKey> {
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
    use qiniu_credential::{Credential, StaticCredentialProvider};
    use std::{boxed::Box, error::Error, result::Result};
    use structopt as _;

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
        assert!(token.starts_with(get_credential().get()?.access_key().as_str()));
        let token = StaticUploadTokenProvider::from(token);
        let policy = token.policy()?;
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), Some("test:file"));
        Ok(())
    }

    #[test]
    fn test_build_upload_token_for_bucket() -> Result<(), Box<dyn Error>> {
        let provider = BucketUploadTokenProvider::builder(
            "test_bucket",
            Duration::from_secs(3600),
            get_credential(),
        )
        .on_policy_generated(Box::new(|policy| {
            policy.return_body("{\"key\":$(key)}");
        }))
        .build();

        let token = provider.to_string()?.into_owned();
        assert!(token.starts_with(get_credential().get()?.access_key().as_str()));

        let policy = provider.policy()?;
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), None);
        assert_eq!(policy.return_body(), Some("{\"key\":$(key)}"));

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
            assert!(token.starts_with(get_credential().async_get().await?.access_key().as_str()));
            let token = StaticUploadTokenProvider::from(token);
            let policy = token.async_policy().await?;
            assert_eq!(policy.bucket(), Some("test_bucket"));
            assert_eq!(policy.key(), Some("test:file"));
            Ok(())
        }
    }

    fn get_credential() -> impl CredentialProvider {
        StaticCredentialProvider::new(Credential::new("abcdefghklmnopq", "1234567890"))
    }
}
