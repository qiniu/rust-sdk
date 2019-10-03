use super::super::response::Response;
use crate::{config::Config, http::token::Token};
use qiniu_http::{Headers, Method, Request as HTTPRequest, Result as HTTPResult};
use std::{borrow::Cow, collections::HashMap, fmt};

pub(crate) struct Parts<'a> {
    pub(super) method: Method,
    pub(super) hosts: &'a [&'a str],
    pub(super) path: &'a str,
    pub(super) query: Option<HashMap<Cow<'a, str>, Cow<'a, str>>>,
    pub(super) headers: Option<Headers<'a>>,
    pub(super) body: Option<Vec<u8>>,
    pub(super) config: Config,
    pub(super) token: Token,
    pub(super) read_body: bool,
    pub(super) idempotent: bool,
    pub(super) follow_redirection: bool,
    pub(super) response_callback: Option<&'a dyn ResponseCallback>,
}

pub(crate) trait ResponseCallback {
    fn on_response_callback(&self, response: &mut Response, request: &HTTPRequest) -> HTTPResult<()>;
}

impl fmt::Debug for Parts<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Parts")
            .field("method", &self.method)
            .field("hosts", &self.hosts)
            .field("path", &self.path)
            .field("query", &self.query)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .field("config", &self.config)
            .field("token", &self.token)
            .field("read_body", &self.read_body)
            .field("idempotent", &self.idempotent)
            .field("follow_redirection", &self.follow_redirection)
            .finish()
    }
}
