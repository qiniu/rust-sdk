use super::{
    config::Config,
    credential::Credential,
    storage::{manager::StorageManager, uploader::UploadManager},
};
use assert_impl::assert_impl;
use getset::Getters;
use std::borrow::Cow;

#[derive(Getters, Clone)]
pub struct Client {
    #[get = "pub"]
    storage_manager: StorageManager,

    #[get = "pub"]
    upload_manager: UploadManager,
}

impl Client {
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

    pub fn storage(&self) -> &StorageManager {
        self.storage_manager()
    }

    pub fn upload(&self) -> &UploadManager {
        self.upload_manager()
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}
