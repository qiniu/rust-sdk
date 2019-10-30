use super::{
    super::{
        bucket::{Bucket, BucketBuilder},
        region::Region,
        upload_policy::UploadPolicy,
        upload_token::UploadToken,
    },
    BucketUploaderBuilder, FileUploaderBuilder, UploadLoggerBuilder,
};
use crate::{config::Config, credential::Credential, utils::ron::Ron};
use assert_impl::assert_impl;
use std::{borrow::Cow, io::Result as IOResult};

#[derive(Clone)]
pub struct UploadManager {
    credential: Credential,
    config: Config,
    upload_logger_builder: Option<UploadLoggerBuilder>,
}

impl UploadManager {
    pub(in super::super) fn credential(&self) -> &Credential {
        &self.credential
    }

    pub(in super::super) fn config(&self) -> &Config {
        &self.config
    }

    pub(in super::super) fn upload_logger_builder(&self) -> Option<&UploadLoggerBuilder> {
        self.upload_logger_builder.as_ref()
    }

    pub(crate) fn new(credential: Credential, config: Config) -> UploadManager {
        UploadManager {
            upload_logger_builder: UploadLoggerBuilder::new(config.clone()),
            credential,
            config,
        }
    }

    pub fn for_bucket(&self, bucket: &Bucket) -> IOResult<BucketUploaderBuilder> {
        BucketUploaderBuilder::new(
            bucket.name().into(),
            bucket
                .regions()
                .map(|iter| {
                    iter.map(|region| {
                        region
                            .up_urls(self.config.use_https())
                            .into_iter()
                            .map(|url| url.into())
                            .collect::<Box<[Box<str>]>>()
                    })
                    .collect::<Box<[Box<[Box<str>]>]>>()
                })
                .unwrap_or_else(|_| {
                    Region::all()
                        .iter()
                        .map(|region| {
                            region
                                .up_urls(self.config.use_https())
                                .into_iter()
                                .map(|url| url.into())
                                .collect::<Box<[Box<str>]>>()
                        })
                        .collect::<Box<[Box<[Box<str>]>]>>()
                }),
            self.credential.clone(),
            self.config.clone(),
            self.upload_logger_builder.as_ref().cloned(),
        )
    }

    pub fn for_bucket_name<'b, B: Into<Cow<'b, str>>>(&self, bucket_name: B) -> IOResult<BucketUploaderBuilder> {
        self.for_bucket(&BucketBuilder::new(bucket_name.into(), self.to_owned()).build())
    }

    pub fn for_upload_token<'u, U: Into<UploadToken<'u>>>(
        &self,
        upload_token: U,
    ) -> error::Result<FileUploaderBuilder<'u>> {
        let upload_token = upload_token.into();
        let policy = upload_token.policy()?;
        if let Some(bucket_name) = policy.bucket() {
            Ok(FileUploaderBuilder::new(
                Ron::Owned(self.for_bucket_name(bucket_name.to_owned())?.build()),
                upload_token.token().into(),
            ))
        } else {
            Err(error::ErrorKind::BucketIsMissingInUploadToken.into())
        }
    }

    pub fn for_upload_policy<'u>(&self, upload_policy: UploadPolicy<'u>) -> error::Result<FileUploaderBuilder<'u>> {
        self.for_upload_token(UploadToken::from_policy(upload_policy, self.credential.clone()))
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

pub mod error {
    use super::super::super::upload_token;
    use error_chain::error_chain;
    use std::io::Error as IOError;

    error_chain! {
        links {
            UploadTokenParseError(upload_token::Error, upload_token::ErrorKind);
        }

        foreign_links {
            QiniuAPIError(qiniu_http::Error);
            IOError(IOError);
        }

        errors {
            BucketIsMissingInUploadToken {
                description("bucket is missing in upload token")
                display("bucket is missing in upload token")
            }
        }
    }
}
