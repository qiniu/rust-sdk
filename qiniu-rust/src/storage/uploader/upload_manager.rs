use super::{
    super::{bucket::Bucket, region::Region, upload_policy::UploadPolicy, upload_token::UploadToken},
    BucketUploaderBuilder, FileUploaderBuilder, UploadLoggerBuilder,
};
use crate::{config::Config, credential::Credential, utils::ron::Ron};
use assert_impl::assert_impl;
use std::{borrow::Cow, io::Result as IOResult};

#[derive(Clone)]
pub struct UploadManager {
    config: Config,
    upload_logger_builder: Option<UploadLoggerBuilder>,
}

impl UploadManager {
    pub(in super::super) fn config(&self) -> &Config {
        &self.config
    }

    pub(crate) fn new(config: Config) -> UploadManager {
        UploadManager {
            upload_logger_builder: UploadLoggerBuilder::new(config.clone()),
            config,
        }
    }

    pub fn for_bucket(&self, bucket: &Bucket) -> IOResult<BucketUploaderBuilder> {
        BucketUploaderBuilder::new(
            bucket.name().into(),
            bucket
                .regions()
                .map(|iter| Self::extract_up_urls_list_from_regions(iter, self.config.use_https()))
                .unwrap_or_else(|_| Self::all_possible_up_urls_list(self.config.use_https())),
            self.config.to_owned(),
            self.upload_logger_builder.as_ref().cloned(),
        )
    }

    pub fn for_bucket_name<'b, B: Into<Cow<'b, str>>, AK: AsRef<str>>(
        &self,
        bucket_name: B,
        access_key: AK,
    ) -> IOResult<BucketUploaderBuilder> {
        let bucket_name = bucket_name.into();
        let up_urls_list = Region::query(bucket_name.as_ref(), access_key.as_ref(), self.config.to_owned())
            .map(|regions| Self::extract_up_urls_list_from_regions(regions.iter(), self.config.use_https()))
            .unwrap_or_else(|_| Self::all_possible_up_urls_list(self.config.use_https()));
        BucketUploaderBuilder::new(
            bucket_name.into_owned().into(),
            up_urls_list,
            self.config.to_owned(),
            self.upload_logger_builder.as_ref().cloned(),
        )
    }

    fn extract_up_urls_list_from_regions<'a, IT: Iterator<Item = &'a Region>>(
        iter: IT,
        use_https: bool,
    ) -> Box<[Box<[Box<str>]>]> {
        iter.map(|region| {
            region
                .up_urls(use_https)
                .into_iter()
                .map(|url| url.into())
                .collect::<Box<[Box<str>]>>()
        })
        .collect::<_>()
    }

    fn all_possible_up_urls_list(use_https: bool) -> Box<[Box<[Box<str>]>]> {
        Region::all()
            .iter()
            .map(|region| {
                region
                    .up_urls(use_https)
                    .into_iter()
                    .map(|url| url.into())
                    .collect::<Box<[Box<str>]>>()
            })
            .collect::<_>()
    }

    pub fn for_upload_token<'u, U: Into<UploadToken<'u>>>(
        &self,
        upload_token: U,
    ) -> error::Result<FileUploaderBuilder<'u>> {
        let upload_token = upload_token.into();
        let access_key = upload_token.access_key()?;
        let policy = upload_token.policy()?;
        if let Some(bucket_name) = policy.bucket() {
            Ok(FileUploaderBuilder::new(
                Ron::Owned(self.for_bucket_name(bucket_name.to_owned(), access_key)?.build()),
                upload_token.token().into(),
            ))
        } else {
            Err(error::ErrorKind::BucketIsMissingInUploadToken.into())
        }
    }

    pub fn for_upload_policy<'u>(
        &self,
        upload_policy: UploadPolicy<'u>,
        credential: Credential,
    ) -> error::Result<FileUploaderBuilder<'u>> {
        self.for_upload_token(UploadToken::from_policy(upload_policy, credential))
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
