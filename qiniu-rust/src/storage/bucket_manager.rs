use super::{bucket::BucketBuilder, Region, RegionId};
use crate::{
    config::Config,
    http::{
        self,
        error::{Error as QiniuError, ErrorKind as QiniuErrorKind},
    },
    utils::{auth::Auth, base64},
};
use error_chain::error_chain;
use qiniu_http::{Error as HTTPError, Result as HTTPResult};
use std::{borrow::Cow, error::Error as StdError};

pub struct BucketManager {
    http_client: http::Client,
    auth: Auth,
    config: Config,
    rs_url: &'static str,
}

impl BucketManager {
    pub(crate) fn new(auth: Auth, config: Config) -> BucketManager {
        BucketManager {
            rs_url: Region::hua_dong().rs_url(config.use_https()),
            auth: auth.clone(),
            config: config.clone(),
            http_client: http::Client::new(auth, config),
        }
    }

    pub fn rs_url(&mut self, rs_url: &'static str) -> &BucketManager {
        self.rs_url = rs_url;
        self
    }

    pub fn bucket_names(&self) -> HTTPResult<Vec<String>> {
        Ok(self
            .http_client
            .get("/buckets", &[self.rs_url])
            .token(http::Token::V1)
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
            .token(http::Token::V1)
            .no_body()
            .send()?
            .ignore_body())
    }

    pub fn drop_bucket<B: AsRef<str>>(&self, bucket: B) -> Result<()> {
        match self
            .http_client
            .post(&("/drop/".to_owned() + bucket.as_ref()), &[self.rs_url])
            .token(http::Token::V1)
            .no_body()
            .send()
        {
            Ok(ref mut response) => Ok(response.ignore_body()),
            Err(err) => {
                if let Some(source) = err.source() {
                    if let Some(err) = source.downcast_ref::<QiniuError>() {
                        match err.kind() {
                            QiniuErrorKind::ForbiddenError(_, message) => {
                                if message.contains("drop non empty bucket is not allowed") {
                                    return Err(ErrorKind::CannotDropNonEmptyBucket.into());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(err.into())
            }
        }
    }

    pub fn bucket<B: Into<Cow<'static, str>>>(&self, bucket: B) -> BucketBuilder {
        BucketBuilder::new(bucket, self.auth.clone(), self.config.clone())
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
