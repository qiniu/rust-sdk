use qiniu_apis::{
    credential::{AccessKey, CredentialProvider},
    http::ResponseErrorKind as HttpResponseErrorKind,
    http_client::{ApiResult, ResponseError},
};
use qiniu_upload_token::{
    BucketName, BucketUploadTokenProvider, ObjectName, ObjectUploadTokenProvider, UploadTokenProvider,
    UploadTokenProviderExt,
};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct UploadTokenSigner(UploadTokenSignerInner);

#[derive(Clone, Debug)]
enum UploadTokenSignerInner {
    UploadTokenProvider(Box<dyn UploadTokenProvider>),
    CredentialProvider {
        credential: Box<dyn CredentialProvider>,
        bucket_name: BucketName,
        lifetime: Duration,
    },
}

impl UploadTokenSigner {
    #[inline]
    pub fn new_upload_token_provider(upload_token_provider: impl UploadTokenProvider + 'static) -> Self {
        Self(UploadTokenSignerInner::UploadTokenProvider(Box::new(
            upload_token_provider,
        )))
    }

    #[inline]
    pub fn new_credential_provider(
        credential: impl CredentialProvider + 'static,
        bucket_name: impl Into<BucketName>,
        lifetime: Duration,
    ) -> Self {
        Self(UploadTokenSignerInner::CredentialProvider {
            credential: Box::new(credential),
            bucket_name: bucket_name.into(),
            lifetime,
        })
    }

    #[inline]
    pub fn upload_token_provider(&self) -> Option<&dyn UploadTokenProvider> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(provider) => Some(provider.as_ref()),
            UploadTokenSignerInner::CredentialProvider { .. } => None,
        }
    }

    #[inline]
    pub fn credential_provider(&self) -> Option<&dyn CredentialProvider> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(_) => None,
            UploadTokenSignerInner::CredentialProvider { credential, .. } => Some(credential.as_ref()),
        }
    }

    pub(super) fn access_key(&self) -> ApiResult<AccessKey> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(provider) => provider
                .access_key(&Default::default())
                .map(|ak| ak.into())
                .map_err(|err| ResponseError::new(HttpResponseErrorKind::InvalidRequestResponse.into(), err)),
            UploadTokenSignerInner::CredentialProvider { credential, .. } => {
                Ok(credential.get(&Default::default())?.access_key().to_owned())
            }
        }
    }

    pub(super) fn bucket_name(&self) -> ApiResult<BucketName> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(provider) => provider
                .bucket_name(&Default::default())
                .map_err(|err| ResponseError::new(HttpResponseErrorKind::InvalidRequestResponse.into(), err)),
            UploadTokenSignerInner::CredentialProvider { bucket_name, .. } => Ok(bucket_name.to_owned()),
        }
    }

    #[cfg(feature = "async")]
    pub(super) async fn async_access_key(&self) -> ApiResult<AccessKey> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(provider) => provider
                .async_access_key(&Default::default())
                .await
                .map(|ak| ak.into())
                .map_err(|err| ResponseError::new(HttpResponseErrorKind::InvalidRequestResponse.into(), err)),
            UploadTokenSignerInner::CredentialProvider { credential, .. } => {
                Ok(credential.async_get(&Default::default()).await?.access_key().to_owned())
            }
        }
    }

    #[cfg(feature = "async")]
    pub(super) async fn async_bucket_name(&self) -> ApiResult<BucketName> {
        match &self.0 {
            UploadTokenSignerInner::UploadTokenProvider(provider) => provider
                .async_bucket_name(&Default::default())
                .await
                .map_err(|err| ResponseError::new(HttpResponseErrorKind::InvalidRequestResponse.into(), err)),
            UploadTokenSignerInner::CredentialProvider { bucket_name, .. } => Ok(bucket_name.to_owned()),
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
            UploadTokenSignerInner::CredentialProvider {
                credential,
                bucket_name,
                lifetime,
            } => {
                if let Some(object_name) = object_name {
                    OwnedUploadTokenProviderOrReferenced::Owned(Box::new(make_object_upload_token_provider(
                        bucket_name,
                        object_name,
                        *lifetime,
                        credential,
                    )))
                } else {
                    OwnedUploadTokenProviderOrReferenced::Owned(Box::new(make_bucket_upload_token_provider(
                        bucket_name,
                        *lifetime,
                        credential,
                    )))
                }
            }
        }
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
) -> ObjectUploadTokenProvider<C> {
    ObjectUploadTokenProvider::new(bucket_name.to_owned(), object_name, lifetime, credential)
}

fn make_bucket_upload_token_provider<C: CredentialProvider + Clone>(
    bucket_name: &BucketName,
    lifetime: Duration,
    credential: C,
) -> BucketUploadTokenProvider<C> {
    BucketUploadTokenProvider::new(bucket_name.to_owned(), lifetime, credential)
}

impl<T: UploadTokenProvider + 'static> From<T> for UploadTokenSigner {
    #[inline]
    fn from(upload_token_provider: T) -> Self {
        Self(UploadTokenSignerInner::UploadTokenProvider(Box::new(
            upload_token_provider,
        )))
    }
}
