use super::config;
use super::utils::auth::Auth;

pub struct Client {
    auth: Auth,
    config: config::Config,
}

fn new<AccessKey: ToString, SecretKey: Into<Vec<u8>>>(
    access_key: AccessKey,
    secret_key: SecretKey,
    config: config::Config,
) -> Client {
    Client {
        auth: Auth::new(access_key, secret_key),
        config: config,
    }
}
