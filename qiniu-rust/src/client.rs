use super::{config::Config, http, utils::auth::Auth};
use std::{clone, fmt, sync::Arc};

pub struct Client {
    auth: Auth,
    config: Arc<Config>,
    http_client: http::Client, // TODO: 考虑移除
}

fn new<AccessKey: Into<String>, SecretKey: Into<Vec<u8>>>(
    access_key: AccessKey,
    secret_key: SecretKey,
    config: Config,
) -> Client {
    let auth = Auth::new(access_key, secret_key);
    let config_rc = Arc::new(config);
    Client {
        auth: auth.clone(),
        config: config_rc.clone(),
        http_client: http::Client::new(auth, config_rc.clone()),
    }
}

impl clone::Clone for Client {
    fn clone(&self) -> Self {
        Client {
            auth: self.auth.clone(),
            config: self.config.clone(),
            http_client: http::Client::new(self.auth.clone(), self.config.clone()),
        }
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Client")
            .field("auth", &self.auth)
            .field("config", &self.config)
            .finish()
    }
}
