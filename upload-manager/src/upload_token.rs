use anyhow::Result as AnyResult;
use assert_impl::assert_impl;
use qiniu_apis::{
    credential::{AccessKey, CredentialProvider},
    http::ResponseErrorKind as HttpResponseErrorKind,
    http_client::{ApiResult, ResponseError},
};
use qiniu_upload_token::{
    BucketName, BucketUploadTokenProvider, ObjectName, ObjectUploadTokenProvider, UploadPolicyBuilder,
    UploadTokenProvider, UploadTokenProviderExt,
};
use std::{
    fmt::{self, Debug},
    sync::Arc,
    time::Duration,
};

/// 上传凭证签发器
#[derive(Clone, Debug)]
pub struct UploadTokenSigner(UploadTokenSignerInner);

#[derive(Clone, Debug)]
enum UploadTokenSignerInner {
    UploadTokenProvider(Arc<dyn UploadTokenProvider>),
    CredentialProvider(UploadTokenCredentialSigner),
}

type OnPolicyGeneratedCallback = Arc<dyn Fn(&mut UploadPolicyBuilder) -> AnyResult<()> + Sync + Send + 'static>;

#[derive(Clone)]
struct UploadTokenCredentialSigner {
    credential: Arc<dyn CredentialProvider>,
    bucket_name: BucketName,
    lifetime: Duration,
    on_policy_generated: Option<OnPolicyGeneratedCallback>,
}

impl Debug for UploadTokenCredentialSigner {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UploadTokenCredentialSigner")
            .field("credential", &self.credential)
            .field("bucket_name", &self.bucket_name)
            .field("lifetime", &self.lifetime)
            .finish()
    }
}

impl UploadTokenSigner {
    /// 根据上传凭证提供者创建上传凭证签发器
    #[inline]
    pub fn new_upload_token_provider(upload_token_provider: impl UploadTokenProvider + 'static) -> Self {
        Self(UploadTokenSignerInner::UploadTokenProvider(Arc::new(
            upload_token_provider,
        )))
    }

    /// 根据认证信息提供者和存储空间名称创建上传凭证签发器
    #[inline]
    pub fn new_credential_provider(
        credential: impl CredentialProvider + 'static,
        bucket_name: impl Into<BucketName>,
        lifetime: Duration,
    ) -> Self {
        Self(UploadTokenSignerInner::CredentialProvider(
            UploadTokenCredentialSigner {
                credential: Arc::new(credential),
                bucket_name: bucket_name.into(),
                lifetime,
                on_policy_generated: None,
            },
        ))
    }

    /// 根据认证信息提供者和存储空间名称创建上传凭证签发构建器
    #[inline]
    pub fn new_credential_provider_builder(
        credential: impl CredentialProvider + 'static,
        bucket_name: impl Into<BucketName>,
        lifetime: Duration,
    ) -> UploadTokenSignerBuilder {
        UploadTokenSignerBuilder::new_credential_provider(credential, bucket_name, lifetime)
    }

    /// 获取上传凭证提供者
    ///
    /// 如果没有设置则返回 [`None`]
    #[inline]
    pub fn upload_token_provider(&self) -> Option<&dyn UploadTokenProvider> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(provider) => Some(provider.as_ref()),
            UploadTokenSignerInner::CredentialProvider { .. } => None,
        }
    }

    /// 获取认证信息提供者
    ///
    /// 如果没有设置则返回 [`None`]
    #[inline]
    pub fn credential_provider(&self) -> Option<&dyn CredentialProvider> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(_) => None,
            UploadTokenSignerInner::CredentialProvider(UploadTokenCredentialSigner { credential, .. }) => {
                Some(credential.as_ref())
            }
        }
    }

    pub(super) fn access_key(&self) -> ApiResult<AccessKey> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(provider) => provider
                .access_key(Default::default())
                .map(|ak| ak.into())
                .map_err(|err| ResponseError::new(HttpResponseErrorKind::InvalidRequestResponse.into(), err)),
            UploadTokenSignerInner::CredentialProvider(UploadTokenCredentialSigner { credential, .. }) => {
                Ok(credential.get(Default::default())?.access_key().to_owned())
            }
        }
    }

    pub(super) fn bucket_name(&self) -> ApiResult<BucketName> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(provider) => provider
                .bucket_name(Default::default())
                .map_err(|err| ResponseError::new(HttpResponseErrorKind::InvalidRequestResponse.into(), err)),
            UploadTokenSignerInner::CredentialProvider(UploadTokenCredentialSigner { bucket_name, .. }) => {
                Ok(bucket_name.to_owned())
            }
        }
    }

    #[cfg(feature = "async")]
    pub(super) async fn async_access_key(&self) -> ApiResult<AccessKey> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(provider) => provider
                .async_access_key(Default::default())
                .await
                .map(|ak| ak.into())
                .map_err(|err| ResponseError::new(HttpResponseErrorKind::InvalidRequestResponse.into(), err)),
            UploadTokenSignerInner::CredentialProvider(UploadTokenCredentialSigner { credential, .. }) => {
                Ok(credential.async_get(Default::default()).await?.access_key().to_owned())
            }
        }
    }

    #[cfg(feature = "async")]
    pub(super) async fn async_bucket_name(&self) -> ApiResult<BucketName> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(provider) => provider
                .async_bucket_name(Default::default())
                .await
                .map_err(|err| ResponseError::new(HttpResponseErrorKind::InvalidRequestResponse.into(), err)),
            UploadTokenSignerInner::CredentialProvider(UploadTokenCredentialSigner { bucket_name, .. }) => {
                Ok(bucket_name.to_owned())
            }
        }
    }

    pub(super) fn make_upload_token_provider(
        &self,
        object_name: Option<ObjectName>,
    ) -> OwnedUploadTokenProviderOrReferenced<'_> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(provider) => {
                OwnedUploadTokenProviderOrReferenced::Referenced(provider.as_ref())
            }
            UploadTokenSignerInner::CredentialProvider(UploadTokenCredentialSigner {
                credential,
                bucket_name,
                lifetime,
                on_policy_generated,
            }) => {
                if let Some(object_name) = object_name {
                    OwnedUploadTokenProviderOrReferenced::Owned(Box::new(make_object_upload_token_provider(
                        bucket_name,
                        object_name,
                        *lifetime,
                        credential,
                        on_policy_generated.to_owned(),
                    )))
                } else {
                    OwnedUploadTokenProviderOrReferenced::Owned(Box::new(make_bucket_upload_token_provider(
                        bucket_name,
                        *lifetime,
                        credential,
                        on_policy_generated.to_owned(),
                    )))
                }
            }
        }
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

pub(super) enum OwnedUploadTokenProviderOrReferenced<'r> {
    Owned(Box<dyn UploadTokenProvider + 'r>),
    Referenced(&'r dyn UploadTokenProvider),
}

impl OwnedUploadTokenProviderOrReferenced<'_> {
    pub(super) fn as_ref(&self) -> &dyn UploadTokenProvider {
        match self {
            OwnedUploadTokenProviderOrReferenced::Owned(provider) => provider.as_ref(),
            OwnedUploadTokenProviderOrReferenced::Referenced(referenced) => referenced,
        }
    }
}

fn make_object_upload_token_provider<C: CredentialProvider + Clone>(
    bucket_name: &BucketName,
    object_name: ObjectName,
    lifetime: Duration,
    credential: C,
    on_policy_generated: Option<OnPolicyGeneratedCallback>,
) -> ObjectUploadTokenProvider<C> {
    let mut builder = ObjectUploadTokenProvider::builder(bucket_name.to_owned(), object_name, lifetime, credential);
    if let Some(on_policy_generated) = on_policy_generated {
        builder = builder.on_policy_generated(move |builder| on_policy_generated(builder));
    }
    builder.build()
}

fn make_bucket_upload_token_provider<C: CredentialProvider + Clone>(
    bucket_name: &BucketName,
    lifetime: Duration,
    credential: C,
    on_policy_generated: Option<OnPolicyGeneratedCallback>,
) -> BucketUploadTokenProvider<C> {
    let mut builder = BucketUploadTokenProvider::builder(bucket_name.to_owned(), lifetime, credential);
    if let Some(on_policy_generated) = on_policy_generated {
        builder = builder.on_policy_generated(move |builder| on_policy_generated(builder));
    }
    builder.build()
}

impl<T: UploadTokenProvider + 'static> From<T> for UploadTokenSigner {
    #[inline]
    fn from(upload_token_provider: T) -> Self {
        Self(UploadTokenSignerInner::UploadTokenProvider(Arc::new(
            upload_token_provider,
        )))
    }
}

/// 上传凭证签发构建器
#[derive(Clone, Debug)]
pub struct UploadTokenSignerBuilder(UploadTokenCredentialSigner);

impl UploadTokenSignerBuilder {
    /// 根据认证信息提供者和存储空间名称创建上传凭证签发构建器
    #[inline]
    pub fn new_credential_provider(
        credential: impl CredentialProvider + 'static,
        bucket_name: impl Into<BucketName>,
        lifetime: Duration,
    ) -> Self {
        Self(UploadTokenCredentialSigner {
            credential: Arc::new(credential),
            bucket_name: bucket_name.into(),
            lifetime,
            on_policy_generated: None,
        })
    }

    /// 设置上传凭证回调函数
    #[inline]
    #[must_use]
    pub fn on_policy_generated(
        mut self,
        callback: impl Fn(&mut UploadPolicyBuilder) -> AnyResult<()> + Sync + Send + 'static,
    ) -> Self {
        self.0.on_policy_generated = Some(Arc::new(callback));
        self
    }

    /// 构造存储空间上传凭证
    #[inline]
    pub fn build(self) -> UploadTokenSigner {
        UploadTokenSigner(UploadTokenSignerInner::CredentialProvider(self.0))
    }
}
