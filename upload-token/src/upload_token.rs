use super::{UploadPolicy, UploadPolicyBuilder};
use anyhow::{Error as AnyError, Result as AnyResult};
use auto_impl::auto_impl;
use dyn_clonable::clonable;
use once_cell::sync::OnceCell;
use qiniu_credential::{AccessKey, CredentialProvider};
use qiniu_utils::{base64, BucketName, ObjectName};
use std::{
    borrow::Cow,
    convert::Infallible,
    fmt::{self, Debug, Display},
    io::Error as IoError,
    ops::{Deref, DerefMut},
    str::FromStr,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use thiserror::Error;

#[cfg(feature = "async")]
use {
    futures::lock::Mutex as AsyncMutex,
    std::{future::Future, pin::Pin},
};

/// 上传凭证获取接口
///
/// 可以阅读 <https://developer.qiniu.com/kodo/manual/1208/upload-token> 了解七牛安全机制。
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait UploadTokenProvider: Clone + Debug + Sync + Send {
    /// 从上传凭证内获取 AccessKey
    ///
    /// 该方法的异步版本为 [`Self::async_access_key`]。
    fn access_key(&self, opts: GetAccessKeyOptions) -> ParseResult<GotAccessKey>;

    /// 异步从上传凭证内获取 AccessKey
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_access_key(&self, opts: GetAccessKeyOptions) -> AsyncParseResult<'_, GotAccessKey> {
        Box::pin(async move { self.access_key(opts) })
    }

    /// 从上传凭证内获取上传策略
    ///
    /// 该方法的异步版本为 [`Self::async_policy`]。
    fn policy(&self, opts: GetPolicyOptions) -> ParseResult<GotUploadPolicy>;

    /// 异步从上传凭证内获取上传策略
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_policy(&self, opts: GetPolicyOptions) -> AsyncParseResult<'_, GotUploadPolicy> {
        Box::pin(async move { self.policy(opts) })
    }

    /// 生成字符串
    ///
    /// 该方法的异步版本为 [`Self::async_to_token_string`]。
    fn to_token_string(&self, opts: ToStringOptions) -> ToStringResult<Cow<'_, str>>;

    /// 异步生成字符串
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_to_token_string(&self, opts: ToStringOptions) -> AsyncToStringResult<'_, Cow<'_, str>> {
        Box::pin(async move { self.to_token_string(opts) })
    }
}

/// 获取 Access Key 的选项
#[derive(Copy, Clone, Debug, Default)]
pub struct GetAccessKeyOptions {}

impl GetAccessKeyOptions {}

/// 获取上传策略的选项
#[derive(Copy, Clone, Debug, Default)]
pub struct GetPolicyOptions {}

impl GetPolicyOptions {}

/// 获取上传凭证的选项
#[derive(Copy, Clone, Debug, Default)]
pub struct ToStringOptions {}

impl ToStringOptions {}

/// 获取的 Access Key
///
/// 该数据结构目前和 Access Key 相同，可以和 Access Key 相互转换，但之后可能会添加更多字段
#[derive(Debug)]
pub struct GotAccessKey(AccessKey);

impl From<GotAccessKey> for AccessKey {
    #[inline]
    fn from(result: GotAccessKey) -> Self {
        result.into_access_key()
    }
}

impl From<AccessKey> for GotAccessKey {
    #[inline]
    fn from(result: AccessKey) -> Self {
        Self(result)
    }
}

impl GotAccessKey {
    /// 获取 Access Key
    #[inline]
    pub fn access_key(&self) -> &AccessKey {
        &self.0
    }

    /// 获取 Access Key 的可变引用
    #[inline]
    pub fn access_key_mut(&mut self) -> &mut AccessKey {
        &mut self.0
    }

    /// 转换为 Access Key
    #[inline]
    pub fn into_access_key(self) -> AccessKey {
        self.0
    }
}

impl Deref for GotAccessKey {
    type Target = AccessKey;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GotAccessKey {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// 获取的上传策略
///
/// 该数据结构目前和上传策略相同，可以和上传策略相互转换，但之后可能会添加更多字段
#[derive(Debug, Clone)]
pub struct GotUploadPolicy<'a>(Cow<'a, UploadPolicy>);

impl<'a> From<GotUploadPolicy<'a>> for Cow<'a, UploadPolicy> {
    #[inline]
    fn from(result: GotUploadPolicy<'a>) -> Self {
        result.0
    }
}

impl From<GotUploadPolicy<'_>> for UploadPolicy {
    #[inline]
    fn from(result: GotUploadPolicy<'_>) -> Self {
        result.into_upload_policy()
    }
}

impl<'a> From<Cow<'a, UploadPolicy>> for GotUploadPolicy<'a> {
    #[inline]
    fn from(policy: Cow<'a, UploadPolicy>) -> Self {
        Self(policy)
    }
}

impl<'a> From<&'a UploadPolicy> for GotUploadPolicy<'a> {
    #[inline]
    fn from(policy: &'a UploadPolicy) -> Self {
        Self::from(Cow::Borrowed(policy))
    }
}

impl From<UploadPolicy> for GotUploadPolicy<'_> {
    #[inline]
    fn from(policy: UploadPolicy) -> Self {
        Self::from(Cow::Owned(policy))
    }
}

impl GotUploadPolicy<'_> {
    /// 获取上传策略
    #[inline]
    pub fn upload_policy(&self) -> &UploadPolicy {
        &self.0
    }

    /// 获取上传策略的可变引用
    #[inline]
    pub fn upload_policy_mut(&mut self) -> &mut UploadPolicy {
        self.0.to_mut()
    }

    /// 转换为上传策略
    #[inline]
    pub fn into_upload_policy(self) -> UploadPolicy {
        self.0.into_owned()
    }
}

impl Deref for GotUploadPolicy<'_> {
    type Target = UploadPolicy;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GotUploadPolicy<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.to_mut()
    }
}

/// 上传凭证获取接口扩展
///
/// 提供存储空间名称解析方法
pub trait UploadTokenProviderExt: UploadTokenProvider {
    /// 获取上传凭证中的存储空间名称
    ///
    /// 该方法的异步版本为 [`Self::async_bucket_name`]。
    fn bucket_name(&self, opts: GetPolicyOptions) -> ParseResult<BucketName> {
        self.policy(opts).and_then(|policy| {
            policy
                .bucket()
                .map_or(Err(ParseError::InvalidUploadTokenFormat), |bucket_name| {
                    Ok(BucketName::from(bucket_name))
                })
        })
    }

    /// 异步获取上传凭证中的存储空间名称
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_bucket_name(&self, opts: GetPolicyOptions) -> AsyncParseResult<'_, BucketName> {
        Box::pin(async move {
            self.async_policy(opts).await.and_then(|policy| {
                policy
                    .bucket()
                    .map_or(Err(ParseError::InvalidUploadTokenFormat), |bucket_name| {
                        Ok(BucketName::from(bucket_name))
                    })
            })
        })
    }
}

impl<T: UploadTokenProvider> UploadTokenProviderExt for T {}

/// 静态上传凭证提供者
///
/// 根据已经被生成好的上传凭证字符串生成上传凭证获取接口的实例，可以将上传凭证解析为 Access Token 和上传策略
#[derive(Clone)]
pub struct StaticUploadTokenProvider {
    upload_token: Box<str>,
    policy: OnceCell<UploadPolicy>,
    access_key: OnceCell<AccessKey>,
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

    /// 获取上传凭证字符串
    #[inline]
    pub fn into_string(self) -> String {
        self.upload_token.into_string()
    }

    pub(super) fn set_policy(&self, policy: UploadPolicy) {
        self.policy.set(policy).ok();
    }

    pub(super) fn set_access_key(&self, access_key: AccessKey) {
        self.access_key.set(access_key).ok();
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

impl Display for StaticUploadTokenProvider {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.upload_token, f)
    }
}

impl UploadTokenProvider for StaticUploadTokenProvider {
    fn access_key(&self, _opts: GetAccessKeyOptions) -> ParseResult<GotAccessKey> {
        self.access_key
            .get_or_try_init(|| {
                self.upload_token
                    .find(':')
                    .map(|i| self.upload_token.split_at(i).0.to_owned().into())
                    .ok_or(ParseError::InvalidUploadTokenFormat)
            })
            .map(|access_key| access_key.to_owned())
            .map(GotAccessKey::from)
    }

    fn policy(&self, _opts: GetPolicyOptions) -> ParseResult<GotUploadPolicy<'_>> {
        self.policy
            .get_or_try_init(|| {
                let encoded_policy = self
                    .upload_token
                    .splitn(3, ':')
                    .last()
                    .ok_or(ParseError::InvalidUploadTokenFormat)?;
                let decoded_policy =
                    base64::decode(encoded_policy.as_bytes()).map_err(ParseError::Base64DecodeError)?;
                UploadPolicy::from_json(decoded_policy).map_err(ParseError::JsonDecodeError)
            })
            .map(Cow::Borrowed)
            .map(GotUploadPolicy::from)
    }

    #[inline]
    fn to_token_string(&self, _opts: ToStringOptions) -> ToStringResult<Cow<'_, str>> {
        Ok(Cow::Borrowed(self.upload_token.as_ref()))
    }
}

impl<T: Into<String>> From<T> for StaticUploadTokenProvider {
    #[inline]
    fn from(s: T) -> Self {
        Self::new(s.into())
    }
}

impl FromStr for StaticUploadTokenProvider {
    type Err = Infallible;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

/// 根据上传凭证生成上传策略
///
/// 通过 [`UploadPolicy::into_dynamic_upload_token_provider()`] 创建
#[derive(Debug, Clone)]
pub struct FromUploadPolicy<C: Clone> {
    upload_policy: UploadPolicy,
    credential: C,
}

impl<C: Clone> FromUploadPolicy<C> {
    /// 基于上传策略和认证信息生成上传凭证实例
    #[inline]
    pub fn new(upload_policy: UploadPolicy, credential: C) -> Self {
        Self {
            upload_policy,
            credential,
        }
    }

    /// 同时返回构建时提供的上传策略和认证信息提供者
    #[inline]
    pub fn split(self) -> (UploadPolicy, C) {
        (self.upload_policy, self.credential)
    }
}

impl<C: CredentialProvider + Clone> UploadTokenProvider for FromUploadPolicy<C> {
    fn access_key(&self, _opts: GetAccessKeyOptions) -> ParseResult<GotAccessKey> {
        Ok(self
            .credential
            .get(Default::default())?
            .into_credential()
            .split()
            .0
            .into())
    }

    #[inline]
    fn policy(&self, _opts: GetPolicyOptions) -> ParseResult<GotUploadPolicy<'_>> {
        Ok(Cow::Borrowed(&self.upload_policy).into())
    }

    fn to_token_string(&self, _opts: ToStringOptions) -> ToStringResult<Cow<'_, str>> {
        Ok(Cow::Owned(
            self.credential
                .get(Default::default())?
                .sign_with_data(self.upload_policy.as_json().as_bytes()),
        ))
    }
}

type OnPolicyGeneratedCallback = Arc<dyn Fn(&mut UploadPolicyBuilder) -> AnyResult<()> + Sync + Send + 'static>;

/// 基于存储空间的动态生成
///
/// 根据存储空间的快速生成上传凭证实例
#[derive(Clone)]
pub struct BucketUploadTokenProvider<C: Clone> {
    bucket: BucketName,
    upload_token_lifetime: Duration,
    credential: C,
    on_policy_generated: Option<OnPolicyGeneratedCallback>,
}

impl<C: Clone> BucketUploadTokenProvider<C> {
    /// 基于存储空间和认证信息动态生成上传凭证实例
    #[inline]
    pub fn new(bucket: impl Into<BucketName>, upload_token_lifetime: Duration, credential: C) -> Self {
        Self::builder(bucket, upload_token_lifetime, credential).build()
    }

    /// 创建存储空间上传凭证构建器
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

    fn make_policy(&self) -> AnyResult<UploadPolicy> {
        let mut builder =
            UploadPolicyBuilder::new_policy_for_bucket(self.bucket.to_string(), self.upload_token_lifetime);
        if let Some(on_policy_generated) = self.on_policy_generated.as_ref() {
            on_policy_generated(&mut builder)?;
        }
        Ok(builder.build())
    }
}

impl<C: Clone> Debug for BucketUploadTokenProvider<C> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BucketUploadTokenProvider")
            .field("bucket", &self.bucket)
            .field("upload_token_lifetime", &self.upload_token_lifetime)
            .finish()
    }
}

impl<C: CredentialProvider + Clone> UploadTokenProvider for BucketUploadTokenProvider<C> {
    #[inline]
    fn access_key(&self, _opts: GetAccessKeyOptions) -> ParseResult<GotAccessKey> {
        Ok(self
            .credential
            .get(Default::default())?
            .into_credential()
            .split()
            .0
            .into())
    }

    fn policy(&self, _opts: GetPolicyOptions) -> ParseResult<GotUploadPolicy<'_>> {
        Ok(self.make_policy()?.into())
    }

    fn to_token_string(&self, _opts: ToStringOptions) -> ToStringResult<Cow<'_, str>> {
        Ok(Cow::Owned(
            self.credential
                .get(Default::default())?
                .sign_with_data(self.make_policy()?.as_json().as_bytes()),
        ))
    }
}

/// 存储空间上传凭证构建器
#[derive(Clone)]
pub struct BucketUploadTokenProviderBuilder<C: Clone> {
    inner: BucketUploadTokenProvider<C>,
}

impl<C: Clone> BucketUploadTokenProviderBuilder<C> {
    /// 设置上传凭证回调函数
    #[inline]
    #[must_use]
    pub fn on_policy_generated(
        mut self,
        callback: impl Fn(&mut UploadPolicyBuilder) -> AnyResult<()> + Sync + Send + 'static,
    ) -> Self {
        self.inner.on_policy_generated = Some(Arc::new(callback));
        self
    }

    /// 构造存储空间上传凭证
    #[inline]
    pub fn build(self) -> BucketUploadTokenProvider<C> {
        self.inner
    }
}

impl<C: Clone> Debug for BucketUploadTokenProviderBuilder<C> {
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
#[derive(Clone)]
pub struct ObjectUploadTokenProvider<C: Clone> {
    bucket: BucketName,
    object: ObjectName,
    upload_token_lifetime: Duration,
    credential: C,
    on_policy_generated: Option<OnPolicyGeneratedCallback>,
}

impl<C: Clone> ObjectUploadTokenProvider<C> {
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

    /// 创建对象上传凭证构建器
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

    fn make_policy(&self) -> AnyResult<UploadPolicy> {
        let mut builder = UploadPolicyBuilder::new_policy_for_object(
            self.bucket.to_string(),
            self.object.to_string(),
            self.upload_token_lifetime,
        );
        if let Some(on_policy_generated) = self.on_policy_generated.as_ref() {
            on_policy_generated(&mut builder)?;
        }
        Ok(builder.build())
    }
}

impl<C: Clone> Debug for ObjectUploadTokenProvider<C> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ObjectUploadTokenProvider")
            .field("bucket", &self.bucket)
            .field("object", &self.object)
            .field("upload_token_lifetime", &self.upload_token_lifetime)
            .finish()
    }
}

impl<C: CredentialProvider + Clone> UploadTokenProvider for ObjectUploadTokenProvider<C> {
    fn access_key(&self, _opts: GetAccessKeyOptions) -> ParseResult<GotAccessKey> {
        Ok(self
            .credential
            .get(Default::default())?
            .into_credential()
            .split()
            .0
            .into())
    }

    fn policy(&self, _opts: GetPolicyOptions) -> ParseResult<GotUploadPolicy<'_>> {
        Ok(self.make_policy()?.into())
    }

    fn to_token_string(&self, _opts: ToStringOptions) -> ToStringResult<Cow<'_, str>> {
        Ok(Cow::Owned(
            self.credential
                .get(Default::default())?
                .sign_with_data(self.make_policy()?.as_json().as_bytes()),
        ))
    }
}

/// 对象上传凭证构建器
#[derive(Clone)]
pub struct ObjectUploadTokenProviderBuilder<C: Clone> {
    inner: ObjectUploadTokenProvider<C>,
}

impl<C: Clone> ObjectUploadTokenProviderBuilder<C> {
    /// 设置上传凭证回调函数
    #[inline]
    #[must_use]
    pub fn on_policy_generated(
        mut self,
        callback: impl Fn(&mut UploadPolicyBuilder) -> AnyResult<()> + Sync + Send + 'static,
    ) -> Self {
        self.inner.on_policy_generated = Some(Arc::new(callback));
        self
    }

    /// 构建对象上传凭证
    #[inline]
    pub fn build(self) -> ObjectUploadTokenProvider<C> {
        self.inner
    }
}

impl<C: Clone> Debug for ObjectUploadTokenProviderBuilder<C> {
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

#[derive(Debug, Default, Clone)]
struct SyncCache(Arc<SyncCacheInner>);

#[derive(Debug, Default)]
struct SyncCacheInner {
    access_key: RwLock<Option<Cache<AccessKey>>>,
    upload_policy: RwLock<Option<Cache<UploadPolicy>>>,
    upload_token: RwLock<Option<Cache<String>>>,
}

#[cfg(feature = "async")]
#[derive(Debug, Default, Clone)]
struct AsyncCache(Arc<AsyncCacheInner>);

#[cfg(feature = "async")]
#[derive(Debug, Default)]
struct AsyncCacheInner {
    access_key: AsyncMutex<Option<Cache<AccessKey>>>,
    upload_policy: AsyncMutex<Option<Cache<UploadPolicy>>>,
    upload_token: AsyncMutex<Option<Cache<String>>>,
}

/// 缓存生成的上传凭证
///
/// 内部存储另一个上传凭证获取接口的实例，该结构为之提供指定时间内的缓存，避免每次都要重新生成新的上传凭证。
#[derive(Debug, Clone)]
pub struct CachedUploadTokenProvider<P: Clone> {
    inner_provider: P,
    cache_lifetime: Duration,
    sync_cache: SyncCache,

    #[cfg(feature = "async")]
    async_cache: AsyncCache,
}

impl<P: Clone> CachedUploadTokenProvider<P> {
    /// 创建上传凭证缓存，需要提供另一个上传凭证获取接口的实例，和需要缓存的时长
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
    ($provider:expr, $cache_field:ident, $opts_field:ident, $opts_type:ty, $method_name:ident, $return_type:ty) => {{
        let guard = $provider.sync_cache.0.$cache_field.read().unwrap();
        return if let Some(cache) = &*guard {
            if cache.cached_at.elapsed() < $provider.cache_lifetime {
                Ok(cache.value.to_owned().into())
            } else {
                drop(guard);
                update_cache(&$provider, $opts_field)
            }
        } else {
            drop(guard);
            update_cache(&$provider, $opts_field)
        };

        #[allow(unused_lifetimes)]
        fn update_cache(
            provider: &CachedUploadTokenProvider<impl UploadTokenProvider + Clone>,
            opts: $opts_type,
        ) -> $return_type {
            let mut guard = provider.sync_cache.0.$cache_field.write().unwrap();
            if let Some(cache) = &*guard {
                if cache.cached_at.elapsed() < provider.cache_lifetime {
                    return Ok(cache.value.to_owned().into());
                }
            }
            match provider.inner_provider.$method_name(opts) {
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
    ($provider:expr, $cache_field:ident, $opts_field:ident, $method_name:ident) => {{
        Box::pin(async move {
            let mut cache = $provider.async_cache.0.$cache_field.lock().await;
            if let Some(cache) = &*cache {
                if cache.cached_at.elapsed() < $provider.cache_lifetime {
                    return Ok(cache.value.to_owned().into());
                }
            }
            match $provider.inner_provider.$method_name($opts_field).await {
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

impl<P: UploadTokenProvider + Clone> UploadTokenProvider for CachedUploadTokenProvider<P> {
    fn access_key(&self, opts: GetAccessKeyOptions) -> ParseResult<GotAccessKey> {
        sync_method!(
            self,
            access_key,
            opts,
            GetAccessKeyOptions,
            access_key,
            ParseResult<GotAccessKey>
        )
    }

    fn policy(&self, opts: GetPolicyOptions) -> ParseResult<GotUploadPolicy<'_>> {
        sync_method!(
            self,
            upload_policy,
            opts,
            GetPolicyOptions,
            policy,
            ParseResult<GotUploadPolicy<'_>>
        )
    }

    fn to_token_string(&self, opts: ToStringOptions) -> ToStringResult<Cow<'_, str>> {
        sync_method!(
            self,
            upload_token,
            opts,
            ToStringOptions,
            to_token_string,
            ToStringResult<Cow<str>>
        )
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_access_key(&self, opts: GetAccessKeyOptions) -> AsyncParseResult<'_, GotAccessKey> {
        async_method!(self, access_key, opts, async_access_key)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_policy(&self, opts: GetPolicyOptions) -> AsyncParseResult<'_, GotUploadPolicy<'_>> {
        async_method!(self, upload_policy, opts, async_policy)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_to_token_string(&self, opts: ToStringOptions) -> AsyncToStringResult<'_, Cow<'_, str>> {
        async_method!(self, upload_token, opts, async_to_token_string)
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
    JsonDecodeError(#[from] serde_json::Error),
    /// 上传凭证获取认证信息错误
    #[error("Credential get error: {0}")]
    CredentialGetError(#[from] IoError),
    /// `on_policy_generated` 回调函数错误
    #[error("on_policy_generated callback error: {0}")]
    CallbackError(#[from] AnyError),
}

/// 上传凭证解析结果
pub type ParseResult<T> = Result<T, ParseError>;

/// 生成上传凭证字符串错误
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ToStringError {
    /// 上传凭证获取认证信息错误
    #[error("Credential get error: {0}")]
    CredentialGetError(#[from] IoError),
    /// 生成上传凭证回调函数错误
    #[error("Generate Upload Policy Callback error: {0}")]
    CallbackError(#[from] AnyError),
}

/// 生成上传凭证字符串结果
pub type ToStringResult<T> = Result<T, ToStringError>;

#[cfg(feature = "async")]
type AsyncParseResult<'a, T> = Pin<Box<dyn Future<Output = ParseResult<T>> + 'a + Send>>;

#[cfg(feature = "async")]
type AsyncToStringResult<'a, T> = Pin<Box<dyn Future<Output = ToStringResult<T>> + 'a + Send>>;

#[cfg(test)]
mod tests {
    use super::{super::UploadPolicyBuilder, *};
    use async_std as _;
    use qiniu_credential::Credential;
    use std::{boxed::Box, error::Error, result::Result};
    use structopt as _;

    #[test]
    fn test_build_upload_token_from_upload_policy() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_object("test_bucket", "test:file", Duration::from_secs(3600)).build();
        let provider = FromUploadPolicy::new(policy, get_credential());
        let token = provider.to_token_string(Default::default())?;
        assert!(token.starts_with(get_credential().get(Default::default())?.access_key().as_str()));
        let token: StaticUploadTokenProvider = token.parse()?;
        let policy = token.policy(Default::default())?;
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), Some("test:file"));
        Ok(())
    }

    #[test]
    fn test_build_upload_token_for_bucket() -> Result<(), Box<dyn Error>> {
        let provider = BucketUploadTokenProvider::builder("test_bucket", Duration::from_secs(3600), get_credential())
            .on_policy_generated(|policy| {
                policy.return_body("{\"key\":$(key)}");
                Ok(())
            })
            .build();

        let token = provider.to_token_string(Default::default())?;
        assert!(token.starts_with(get_credential().get(Default::default())?.access_key().as_str()));

        let policy = provider.policy(Default::default())?;
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
            let policy =
                UploadPolicyBuilder::new_policy_for_object("test_bucket", "test:file", Duration::from_secs(3600))
                    .build();
            let provider = FromUploadPolicy::new(policy, get_credential());
            let token = provider.async_to_token_string(Default::default()).await?;
            assert!(token.starts_with(
                get_credential()
                    .async_get(Default::default())
                    .await?
                    .access_key()
                    .as_str()
            ));
            let token: StaticUploadTokenProvider = token.parse()?;
            let get_policy_from_size_options = Default::default();
            let policy = token.async_policy(get_policy_from_size_options).await?;
            assert_eq!(policy.bucket(), Some("test_bucket"));
            assert_eq!(policy.key(), Some("test:file"));
            Ok(())
        }
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
