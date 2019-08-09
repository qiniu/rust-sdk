use super::config::Config;
use super::http;
use super::utils::auth::Auth;
use std::rc::Rc;

pub struct Client {
    auth: Rc<Auth>,
    config: Rc<Config>,
    http_client: http::client::Client,
}

fn new<AccessKey: ToString, SecretKey: Into<Vec<u8>>>(
    access_key: AccessKey,
    secret_key: SecretKey,
    config: Config,
) -> Client {
    let auth_rc = Rc::new(Auth::new(access_key, secret_key));
    let config_rc = Rc::new(config);
    Client {
        auth: auth_rc.clone(),
        config: config_rc.clone(),
        http_client: http::client::Client::new(auth_rc.clone(), config_rc.clone()),
    }
}
