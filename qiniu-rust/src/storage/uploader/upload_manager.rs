//! 上传管理器
//!
//! 封装上传相关功能

use super::{
    super::{
        region::Region,
        uploader::{UploadPolicy, UploadToken, UploadTokenParseError},
    },
    BatchUploader, ObjectUploader,
};
use crate::{config::Config, credential::Credential, http::Client};
use assert_impl::assert_impl;
use rayon::{ThreadPool, ThreadPoolBuildError, ThreadPoolBuilder};
use std::{borrow::Cow, result::Result, sync::Arc};
use thiserror::Error;

/// 上传管理器
///
/// 上传管理器更接近于一个上传入口，提供上传文件的功能，并可以利用它构建批量上传器
/// 可以跨线程使用，但由于可能会自带线程池，请勿跨进程使用
#[derive(Clone)]
pub struct UploadManager {
    http_client: Client,
    thread_pool: Option<Arc<ThreadPool>>,
}

impl UploadManager {
    /// 创建新的上传管理器
    pub fn new(config: Config) -> Self {
        UploadManager {
            http_client: Client::new(config),
            thread_pool: None,
        }
    }

    /// 创建新的上传管理器，并使用指定的线程池
    pub fn new_with_thread_pool(config: Config, thread_pool: Arc<ThreadPool>) -> Self {
        UploadManager {
            http_client: Client::new(config),
            thread_pool: Some(thread_pool),
        }
    }

    /// 创建新的上传管理器，并使用专用线程池
    pub fn new_with_exclusive_thread_pool(
        config: Config,
        thread_pool_size: usize,
    ) -> Result<Self, ThreadPoolBuildError> {
        let upload_manager = UploadManager {
            http_client: Client::new(config),
            thread_pool: Some(Arc::new(
                ThreadPoolBuilder::new()
                    .num_threads(thread_pool_size)
                    .thread_name(move |index| format!("upload_manager_thread_{}_{}", thread_pool_size, index))
                    .build()?,
            )),
        };
        Ok(upload_manager)
    }

    /// 根据上传凭证创建对象上传器
    pub fn upload_for_upload_token<'a>(
        &'a self,
        upload_token: impl Into<Cow<'a, UploadToken>>,
    ) -> CreateUploaderResult<ObjectUploader<'a>> {
        let upload_token = upload_token.into();
        let access_key = upload_token.access_key()?;
        if let Some(bucket_name) = upload_token.policy()?.bucket() {
            let bucket_name = bucket_name.to_owned();
            let up_urls_list = Region::query(&bucket_name, access_key, self.config().to_owned())
                .map(|regions| extract_up_urls_list_from_regions(regions.iter(), self.config().use_https()))
                .unwrap_or_else(|_| all_possible_up_urls_list(self.config().use_https()));
            Ok(ObjectUploader::new(
                self,
                upload_token,
                bucket_name.into(),
                up_urls_list,
            ))
        } else {
            Err(CreateUploaderError::BucketIsMissingInUploadToken)
        }
    }

    pub(in super::super) fn upload_for_internal_generated_upload_token_with_regions<'a>(
        &'a self,
        bucket_name: Cow<'a, str>,
        upload_token: Cow<'a, UploadToken>,
        regions: Option<impl Iterator<Item = &'a Region>>,
    ) -> ObjectUploader<'a> {
        let up_urls_list = regions
            .map(|regions| extract_up_urls_list_from_regions(regions, self.config().use_https()))
            .unwrap_or_else(|| all_possible_up_urls_list(self.config().use_https()));
        ObjectUploader::new(self, upload_token, bucket_name, up_urls_list)
    }

    /// 根据上传策略和认证信息创建对象上传器
    pub fn upload_for_upload_policy(
        &self,
        upload_policy: UploadPolicy,
        credential: Credential,
    ) -> CreateUploaderResult<ObjectUploader> {
        self.upload_for_upload_token(UploadToken::new(upload_policy, credential))
    }

    /// 根据存储空间和认证信息创建对象上传器
    pub fn upload_for_bucket(&self, bucket: impl Into<Cow<'static, str>>, credential: Credential) -> ObjectUploader {
        self.upload_for_upload_token(UploadToken::new_from_bucket(bucket.into(), credential, self.config()))
            .unwrap()
    }

    /// 根据上传凭证创建批量对象上传器
    pub fn batch_uploader_for_upload_token(
        &self,
        upload_token: impl Into<UploadToken>,
    ) -> CreateUploaderResult<BatchUploader> {
        BatchUploader::new_for_upload_manager(self.to_owned(), upload_token.into())
    }

    /// 根据上传策略和认证信息创建批量上传器
    pub fn batch_uploader_for_upload_policy(
        &self,
        upload_policy: UploadPolicy,
        credential: Credential,
    ) -> CreateUploaderResult<BatchUploader> {
        self.batch_uploader_for_upload_token(UploadToken::new(upload_policy, credential))
    }

    /// 根据存储空间和认证信息创建批量上传器
    pub fn batch_uploader_for_bucket(
        &self,
        bucket: impl Into<Cow<'static, str>>,
        credential: Credential,
    ) -> BatchUploader {
        self.batch_uploader_for_upload_token(UploadToken::new_from_bucket(bucket, credential, self.config()))
            .unwrap()
    }

    #[inline]
    pub(crate) fn thread_pool(&self) -> Option<&Arc<ThreadPool>> {
        self.thread_pool.as_ref()
    }

    #[inline]
    pub(crate) fn http_client(&self) -> &Client {
        &self.http_client
    }

    #[inline]
    pub(crate) fn config(&self) -> &Config {
        self.http_client.config()
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// 创建上传器错误
#[derive(Error, Debug)]
pub enum CreateUploaderError {
    /// 上传凭证解析错误
    #[error("Failed to parse upload token: {0}")]
    UploadTokenParseError(#[from] UploadTokenParseError),
    /// 上传凭证中不包含存储空间信息
    #[error("Bucket is missing in upload token")]
    BucketIsMissingInUploadToken,
}

/// 创建上传器结果
pub type CreateUploaderResult<T> = Result<T, CreateUploaderError>;

fn extract_up_urls_list_from_regions<'a>(
    iter: impl Iterator<Item = &'a Region>,
    use_https: bool,
) -> Box<[Box<[Box<str>]>]> {
    iter.map(|region| {
        region
            .up_urls_owned(use_https)
            .into_iter()
            .map(|url| url.into_owned().into_boxed_str())
            .collect::<Box<[_]>>()
    })
    .collect()
}

fn all_possible_up_urls_list(use_https: bool) -> Box<[Box<[Box<str>]>]> {
    Region::all()
        .iter()
        .map(|region| {
            region
                .up_urls_owned(use_https)
                .into_iter()
                .map(|url| url.into_owned().into_boxed_str())
                .collect::<Box<[_]>>()
        })
        .collect()
}
