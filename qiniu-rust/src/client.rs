use super::{config::Config, credential::Credential, storage::manager::StorageManager};
use getset::Getters;
use std::borrow::Cow;

#[derive(Getters)]
pub struct Client {
    #[get = "pub"]
    storage_manager: StorageManager,
}

impl Client {
    pub fn new<AccessKey: Into<Cow<'static, str>>, SecretKey: Into<Cow<'static, str>>>(
        access_key: AccessKey,
        secret_key: SecretKey,
        config: Config,
    ) -> Client {
        let credential = Credential::new(access_key, secret_key);
        Client {
            storage_manager: StorageManager::new(credential, config),
        }
    }

    pub fn storage(&self) -> &StorageManager {
        self.storage_manager()
    }
}
