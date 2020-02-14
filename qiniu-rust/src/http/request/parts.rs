use super::{
    super::{response::Response, token::Token},
    HTTPError, HTTPResult, Headers, Method,
};
use crate::config::Config;
use std::{borrow::Cow, collections::HashMap, fmt, time::Duration};

pub(crate) struct Parts<'a> {
    pub(super) method: Method,
    pub(super) base_urls: &'a [&'a str],
    pub(super) path: &'a str,
    pub(super) query: Option<HashMap<Cow<'a, str>, Cow<'a, str>>>,
    pub(super) headers: Option<Headers<'a>>,
    pub(super) body: Option<Vec<u8>>,
    pub(super) config: Config,
    pub(super) token: Option<Token<'a>>,
    pub(super) read_body: bool,
    pub(super) idempotent: bool,
    pub(super) follow_redirection: bool,
    pub(super) on_uploading_progress: Option<&'a dyn Fn(u64, u64)>,
    pub(super) on_downloading_progress: Option<&'a dyn Fn(u64, u64)>,
    pub(super) on_response: Option<&'a dyn Fn(&mut Response, Duration) -> HTTPResult<()>>,
    pub(super) on_error: Option<&'a dyn Fn(Option<&str>, &HTTPError, Duration)>,
}

impl fmt::Debug for Parts<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Parts")
            .field("method", &self.method)
            .field("base_urls", &self.base_urls)
            .field("path", &self.path)
            .field("query", &self.query)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .field("config", &self.config)
            .field("token", &self.token)
            .field("read_body", &self.read_body)
            .field("idempotent", &self.idempotent)
            .field("follow_redirection", &self.follow_redirection)
            .field(
                "on_uploading_progress",
                if self.on_uploading_progress.is_some() {
                    &"Installed"
                } else {
                    &"Not Installed"
                },
            )
            .field(
                "on_downloading_progress",
                if self.on_downloading_progress.is_some() {
                    &"Installed"
                } else {
                    &"Not Installed"
                },
            )
            .field(
                "on_response",
                if self.on_response.is_some() {
                    &"Installed"
                } else {
                    &"Not Installed"
                },
            )
            .field(
                "on_error",
                if self.on_error.is_some() {
                    &"Installed"
                } else {
                    &"Not Installed"
                },
            )
            .finish()
    }
}
