use super::config::Config;
use super::http;
use super::utils::auth::Auth;
use std::{clone, fmt, sync::Arc};

pub struct Client {
    auth: Arc<Auth>,
    config: Arc<Config>,
    http_client: http::client::Client,
}

fn new<AccessKey: Into<String>, SecretKey: Into<Vec<u8>>>(
    access_key: AccessKey,
    secret_key: SecretKey,
    config: Config,
) -> Client {
    let auth_rc = Arc::new(Auth::new(access_key, secret_key));
    let config_rc = Arc::new(config);
    Client {
        auth: auth_rc.clone(),
        config: config_rc.clone(),
        http_client: http::client::Client::new(auth_rc.clone(), config_rc.clone()),
    }
}

impl clone::Clone for Client {
    fn clone(&self) -> Self {
        Client {
            auth: self.auth.clone(),
            config: self.config.clone(),
            http_client: http::client::Client::new(self.auth.clone(), self.config.clone()),
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
