use super::{config::Config, storage::BucketManager, utils::auth::Auth};
use getset::Getters;

#[derive(Getters)]
pub struct Client {
    #[get = "pub"]
    bucket_manager: BucketManager,
}

impl Client {
    pub fn new<AccessKey: Into<String>, SecretKey: Into<Vec<u8>>>(
        access_key: AccessKey,
        secret_key: SecretKey,
        config: Config,
    ) -> Client {
        let auth = Auth::new(access_key, secret_key);
        Client {
            bucket_manager: BucketManager::new(auth, config),
        }
    }
}
