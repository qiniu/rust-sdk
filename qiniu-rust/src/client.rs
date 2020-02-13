//! 七牛客户端模块
use super::{
    config::Config,
    credential::Credential,
    storage::{manager::StorageManager, uploader::UploadManager},
};
use assert_impl::assert_impl;
use getset::Getters;
use std::borrow::Cow;

/// 七牛 SDK 客户端
///
/// 这里的客户端是针对七牛服务器而言，而并非指该结构体是运行在客户端应用程序上。
/// 实际上，该结构体由于会存储用户的 SecretKey，因此不推荐在客户端应用程序上使用，而应该只在服务器端应用程序上使用。
#[derive(Getters, Clone)]
pub struct Client {
    #[get = "pub"]
    storage_manager: StorageManager,

    #[get = "pub"]
    upload_manager: UploadManager,
}

impl Client {
    /// 构建 SDK 客户端
    ///
    /// # Arguments
    ///
    /// * `access_key` - 七牛 Access Key
    /// * `secret_key` - 七牛 Secret Key
    /// * `config` - 七牛客户端配置
    ///
    /// # Example
    ///
    /// ```
    /// use qiniu_ng::{Client, Config};
    /// let client = Client::new("[Access Key]", "[Secret Key]", Config::default());
    /// ```
    pub fn new(
        access_key: impl Into<Cow<'static, str>>,
        secret_key: impl Into<Cow<'static, str>>,
        config: Config,
    ) -> Client {
        let credential = Credential::new(access_key, secret_key);
        Client {
            upload_manager: UploadManager::new(config.clone()),
            storage_manager: StorageManager::new(credential, config),
        }
    }

    /// 获取存储空间管理器
    pub fn storage(&self) -> &StorageManager {
        self.storage_manager()
    }

    /// 获取上传管理器
    pub fn upload(&self) -> &UploadManager {
        self.upload_manager()
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}
