use super::{bucket::BucketBuilder, region::RegionId, uploader::UploadManager};
use crate::{
    config::Config,
    credential::Credential,
    http::{Client, Error as HTTPError, ErrorKind as HTTPErrorKind, Result as HTTPResult, TokenVersion},
};
use assert_impl::assert_impl;
use std::{
    borrow::{Borrow, Cow},
    result::Result,
};
use thiserror::Error;

#[derive(Clone)]
pub struct StorageManager {
    http_client: Client,
    credential: Credential,
    rs_url: Box<str>,
}

impl StorageManager {
    pub(crate) fn new(credential: Credential, config: Config) -> StorageManager {
        StorageManager {
            rs_url: config.rs_url().into(),
            credential,
            http_client: Client::new(config),
        }
    }

    pub fn bucket_names(&self) -> HTTPResult<Vec<String>> {
        Ok(self
            .http_client
            .get("/buckets", &[&self.rs_url])
            .token(TokenVersion::V2, self.credential.borrow().into())
            .accept_json()
            .no_body()
            .send()?
            .parse_json()?)
    }

    pub fn create_bucket(&self, bucket: impl AsRef<str>, region_id: RegionId) -> HTTPResult<()> {
        self.http_client
            .post(
                &("/mkbucketv3/".to_owned() + bucket.as_ref() + "/region/" + region_id.as_str()),
                &[&self.rs_url],
            )
            .token(TokenVersion::V2, self.credential.borrow().into())
            .no_body()
            .send()?
            .ignore_body();
        Ok(())
    }

    pub fn drop_bucket(&self, bucket: impl AsRef<str>) -> DropBucketResult<()> {
        match self
            .http_client
            .post(&("/drop/".to_owned() + bucket.as_ref()), &[&self.rs_url])
            .token(TokenVersion::V2, self.credential.borrow().into())
            .no_body()
            .send()
        {
            Ok(ref mut response) => {
                response.ignore_body();
                Ok(())
            }
            Err(err) => {
                if let HTTPErrorKind::ResponseStatusCodeError(403, message) = err.error_kind() {
                    if message.contains("drop non empty bucket is not allowed") {
                        return Err(DropBucketError::CannotDropNonEmptyBucket);
                    }
                }
                Err(err.into())
            }
        }
    }

    pub fn upload_manager(&self) -> UploadManager {
        UploadManager::new(self.http_client.config().to_owned())
    }

    pub fn bucket<'b>(&'b self, bucket: impl Into<Cow<'b, str>>) -> BucketBuilder<'b> {
        BucketBuilder::new(bucket.into(), self.credential.borrow().into(), self.upload_manager())
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

#[derive(Error, Debug)]
pub enum DropBucketError {
    #[error("Qiniu API call error: {0}")]
    HTTPError(#[from] HTTPError),
    #[error("Drop non empty bucket is not allowed")]
    CannotDropNonEmptyBucket,
}

pub type DropBucketResult<T> = Result<T, DropBucketError>;
