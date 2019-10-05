use super::{
    super::{
        bucket::{Bucket, BucketBuilder},
        recorder::FileSystemRecorder,
        region::Region,
        upload_policy::UploadPolicy,
        upload_token::UploadToken,
    },
    BucketUploader, FileUploaderBuilder,
};
use crate::{config::Config, credential::Credential};
use std::borrow::Cow;

pub struct UploadManager {
    credential: Credential,
    config: Config,
}

impl UploadManager {
    pub(crate) fn new(credential: Credential, config: Config) -> UploadManager {
        UploadManager {
            credential: credential,
            config: config,
        }
    }

    // TODO: ADD CUSTOMIZED RECORDER METHOD
    pub fn for_bucket<'b>(&self, bucket: &Bucket) -> BucketUploader<'b, FileSystemRecorder<'b>> {
        BucketUploader::new(
            bucket.name().to_owned(),
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
        )
    }

    pub fn for_bucket_name<'b, B: Into<Cow<'b, str>>>(
        &self,
        bucket_name: B,
    ) -> BucketUploader<'b, FileSystemRecorder<'b>> {
        self.for_bucket(&BucketBuilder::new(bucket_name, self.credential.clone(), self.config.clone()).build())
    }

    pub fn for_upload_token<'u, U: Into<UploadToken<'u>>>(
        &self,
        upload_token: U,
    ) -> error::Result<FileUploaderBuilder<'u, FileSystemRecorder<'u>>> {
        let upload_token = upload_token.into();
        let policy = upload_token.policy()?;
        if let Some(bucket_name) = policy.bucket() {
            Ok(FileUploaderBuilder::new(
                Cow::Owned(self.for_bucket_name(bucket_name.to_owned())),
                upload_token,
            ))
        } else {
            Err(error::ErrorKind::BucketIsMissingInUploadToken.into())
        }
    }

    pub fn for_upload_policy<'u>(
        &self,
        upload_policy: UploadPolicy<'u>,
    ) -> error::Result<FileUploaderBuilder<'u, FileSystemRecorder<'u>>> {
        self.for_upload_token(UploadToken::from_policy(upload_policy, self.credential.clone()))
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
