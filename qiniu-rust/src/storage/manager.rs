//! 存储管理模块
//!
//! 封装存储相关管理功能

use super::{bucket::BucketBuilder, uploader::UploadManager};
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

/// 存储管理器
///
/// 封装存储相关管理功能
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

    /// 列出所有存储空间名称
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

    /// 创建存储空间
    ///
    /// 这里的参数 `region_id` 建议传入枚举类 `RegionId`，
    /// 但如果使用的是私有云且区域 ID 不在 `RegionId` 定义的枚举类内，则使用字符串。
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use qiniu_ng::{Client, Config, storage::region::RegionId};
    /// # use std::{result::Result, error::Error};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let client = Client::new("[Access Key]", "[Secret Key]", Config::default());
    ///
    /// // 创建华东区存储空间
    /// client.storage().create_bucket("[Bucket name 1]", RegionId::Z0)?;
    ///
    /// // 在名为 z3 的区域创建存储空间
    /// client.storage().create_bucket("[Bucket name 2]", "z3")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// 在创建存储空间时，需要注意存储空间的名称必须遵守以下规则：
    /// - 存储空间名称不允许重复，遇到冲突请更换名称。
    /// - 名称由 3 ~ 63 个字符组成 ，可包含小写字母、数字和短划线，且必须以小写字母或者数字开头和结尾。
    pub fn create_bucket(&self, bucket: impl AsRef<str>, region_id: impl AsRef<str>) -> HTTPResult<()> {
        self.http_client
            .post(
                &("/mkbucketv3/".to_owned() + bucket.as_ref() + "/region/" + region_id.as_ref()),
                &[&self.rs_url],
            )
            .token(TokenVersion::V2, self.credential.borrow().into())
            .no_body()
            .send()?
            .ignore_body();
        Ok(())
    }

    /// 删除存储空间
    ///
    /// 删除存储空间前务必保证存储空间里已经没有任何文件，否则删除将会失败。
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

    /// 获取上传管理器
    pub fn upload_manager(&self) -> UploadManager {
        UploadManager::new(self.http_client.config().to_owned())
    }

    /// 获取存储空间实例生成器
    pub fn bucket<'b>(&'b self, bucket: impl Into<Cow<'b, str>>) -> BucketBuilder<'b> {
        BucketBuilder::new(bucket.into(), self.credential.borrow().into(), self.upload_manager())
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// 删除存储空间错误
#[derive(Error, Debug)]
pub enum DropBucketError {
    #[error("Qiniu API call error: {0}")]
    HTTPError(#[from] HTTPError),
    #[error("Drop non empty bucket is not allowed")]
    CannotDropNonEmptyBucket,
}

/// 删除存储空间结果
pub type DropBucketResult<T> = Result<T, DropBucketError>;
