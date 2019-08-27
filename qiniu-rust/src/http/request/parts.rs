use crate::{config::Config, http::token::Token, utils::auth::Auth};
use qiniu_http::{Headers, Method};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Parts {
    pub method: Method,
    pub hosts: Vec<String>,
    pub path: String,
    pub headers: Headers,
    pub body: Vec<u8>,
    pub auth: Arc<Auth>,
    pub config: Arc<Config>,
    pub token: Token,
}
