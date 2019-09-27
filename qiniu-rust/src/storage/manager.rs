use super::{
    bucket::BucketBuilder,
    region::{Region, RegionId},
    uploader::Uploader,
};
use crate::{config::Config, credential::Credential, http, utils::base64};
use error_chain::error_chain;
use qiniu_http::{Error as HTTPError, ErrorKind as HTTPErrorKind, Result as HTTPResult};
use std::borrow::Cow;

pub struct StorageManager {
    http_client: http::Client,
    credential: Credential,
    config: Config,
    rs_url: &'static str,
}

impl StorageManager {
    pub(crate) fn new(credential: Credential, config: Config) -> StorageManager {
        StorageManager {
            rs_url: Region::hua_dong().rs_url(config.use_https()),
            credential: credential.clone(),
            config: config.clone(),
            http_client: http::Client::new(config),
        }
    }

    pub fn rs_url(&mut self, rs_url: &'static str) -> &StorageManager {
        self.rs_url = rs_url;
        self
    }

    pub fn bucket_names(&self) -> HTTPResult<Vec<String>> {
        Ok(self
            .http_client
            .get("/buckets", &[self.rs_url])
            .token(http::Token::V1(self.credential.clone()))
            .accept_json()
            .no_body()
            .send()?
            .parse_json()?)
    }

    pub fn create_bucket<B: AsRef<str>>(&self, bucket: B, region_id: RegionId) -> HTTPResult<()> {
        Ok(self
            .http_client
            .post(
                &("/mkbucketv2/".to_owned()
                    + &base64::urlsafe(bucket.as_ref().as_bytes())
                    + "/region/"
                    + region_id.as_str()),
                &[self.rs_url],
            )
            .token(http::Token::V1(self.credential.clone()))
            .no_body()
            .send()?
            .ignore_body())
    }

    pub fn drop_bucket<B: AsRef<str>>(&self, bucket: B) -> Result<()> {
        match self
            .http_client
            .post(&("/drop/".to_owned() + bucket.as_ref()), &[self.rs_url])
            .token(http::Token::V1(self.credential.clone()))
            .no_body()
            .send()
        {
            Ok(ref mut response) => Ok(response.ignore_body()),
            Err(err) => {
                match err.error_kind() {
                    HTTPErrorKind::ResponseStatusCodeError(403, message) => {
                        if message.contains("drop non empty bucket is not allowed") {
                            return Err(ErrorKind::CannotDropNonEmptyBucket.into());
                        }
                    }
                    _ => {}
                }
                Err(err.into())
            }
        }
    }

    pub fn bucket<B: Into<Cow<'static, str>>>(&self, bucket: B) -> BucketBuilder {
        BucketBuilder::new(bucket, self.credential.clone(), self.config.clone())
    }

    pub fn uploader(&self) -> Uploader {
        Uploader::new(self.credential.clone(), self.config.clone())
    }
}

error_chain! {
    foreign_links {
        HTTPError(HTTPError);
    }

    errors {
        CannotDropNonEmptyBucket {
            description("Drop non empty bucket is not allowed")
            display("Drop non empty bucket is not allowed")
        }
    }
}
