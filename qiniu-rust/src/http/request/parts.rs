use crate::{config::Config, http::token::Token, utils::auth::Auth};
use qiniu_http::{Headers, Method};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone)]
pub struct Parts<'a> {
    pub(super) method: Method,
    pub(super) hosts: &'a [&'a str], // TODO: 尝试让其接受 &[&str] 或 std::slice::Iter<'a, &str>
    pub(super) path: &'a str,        // TODO: 尝试让其接受 &str
    pub(super) query: Option<HashMap<String, String>>,
    pub(super) headers: Option<Headers>,
    pub(super) body: Option<Vec<u8>>,
    pub(super) auth: Arc<Auth>,
    pub(super) config: Arc<Config>,
    pub(super) token: Token,
    pub(super) read_body: bool,
}
