use super::config::Config;
use super::http;
use super::utils::auth::Auth;
use std::sync::Arc;

pub struct Client {
    auth: Arc<Auth>,
    config: Arc<Config>,
    http_client: http::client::Client,
}

fn new<AccessKey: ToString, SecretKey: Into<Vec<u8>>>(
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
