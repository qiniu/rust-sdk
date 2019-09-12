use super::{
    super::{bucket::Bucket, region::Region, upload_policy::UploadPolicy, upload_token::UploadToken},
    BucketUploader, FileUploaderBuilder,
};
use crate::{config::Config, utils::auth::Auth};
use std::borrow::Cow;

pub struct Uploader {
    auth: Auth,
    config: Config,
}

impl Uploader {
    pub(crate) fn new(auth: Auth, config: Config) -> Uploader {
        Uploader {
            auth: auth,
            config: config,
        }
    }

    pub fn for_bucket<'b>(&self, bucket: &Bucket) -> qiniu_http::Result<BucketUploader<'b>> {
        Ok(BucketUploader::new(
            bucket.name().to_owned(),
            bucket
                .regions()?
                .map(|region| {
                    region
                        .up_urls(self.config.use_https())
                        .into_iter()
                        .map(|url| url.into())
                        .collect::<Vec<Box<str>>>()
                        .into()
                })
                .collect::<Vec<Box<[Box<str>]>>>(),
            self.auth.clone(),
            self.config.clone(),
        ))
    }

    pub fn for_bucket_name<'b, B: Into<Cow<'b, str>>>(&self, bucket_name: B) -> qiniu_http::Result<BucketUploader<'b>> {
        let bucket_name: Cow<'b, str> = bucket_name.into();
        let uc_urls = Region::query_uc_urls(bucket_name.as_ref(), self.auth.clone(), self.config.clone())?;
        Ok(BucketUploader::new(
            bucket_name,
            uc_urls,
            self.auth.clone(),
            self.config.clone(),
        ))
    }

    pub fn for_upload_token<'u>(&self, upload_token: UploadToken<'u>) -> error::Result<FileUploaderBuilder<'u>> {
        let policy = upload_token.clone().policy()?;
        if let Some(bucket_name) = policy.bucket() {
            Ok(FileUploaderBuilder::new(
                Cow::Owned(self.for_bucket_name(bucket_name.to_owned())?),
                upload_token,
            ))
        } else {
            Err(error::ErrorKind::BucketIsMissingInUploadToken.into())
        }
    }

    pub fn for_upload_policy<'u>(&self, upload_policy: UploadPolicy<'u>) -> error::Result<FileUploaderBuilder<'u>> {
        self.for_upload_token(UploadToken::from_policy(upload_policy, self.auth.clone()))
    }
}

pub mod error {
    use super::super::super::upload_token;
    use error_chain::error_chain;

    error_chain! {
        links {
            UploadTokenParseError(upload_token::Error, upload_token::ErrorKind);
        }

        foreign_links {
            QiniuAPIError(qiniu_http::Error);
        }

        errors {
            BucketIsMissingInUploadToken {
                description("bucket is missing in upload token")
                display("bucket is missing in upload token")
            }
        }
    }
}
