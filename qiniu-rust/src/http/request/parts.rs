use crate::{config::Config, http::token::Token, utils::auth::Auth};
use qiniu_http::{Headers, Method};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone)]
pub struct Parts {
    pub(super) method: Method,
    pub(super) hosts: Vec<String>, // TODO: 尝试让其接受 &[&str] 或 std::slice::Iter<'a, &str>
    pub(super) path: String,       // TODO: 尝试让其接受 &str
    pub(super) query: HashMap<String, String>,
    pub(super) headers: Headers,
    pub(super) body: Vec<u8>,       // TODO: 尝试让其接受 &[u8]
    pub(super) auth: Arc<Auth>,     // TODO: 尝试让其接受 &Auth
    pub(super) config: Arc<Config>, // TODO: 尝试让其接受 &Config
    pub(super) token: Token,
}
