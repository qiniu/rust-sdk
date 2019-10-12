use super::super::response::Response;
use crate::{config::Config, http::token::Token};
use qiniu_http::{Error, Headers, Method, Result};
use std::{borrow::Cow, collections::HashMap, fmt, time::Duration};

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
    pub(super) on_uploading_progress: Option<&'a dyn Fn(usize, usize)>,
    pub(super) on_downloading_progress: Option<&'a dyn Fn(usize, usize)>,
    pub(super) on_retry_request: Option<&'a dyn Fn(&str, &Error, usize, usize, Duration)>,
    pub(super) on_host_failed: Option<&'a dyn Fn(&str, &Error, Duration)>,
    pub(super) on_response: Option<&'a dyn Fn(&mut Response, Duration) -> Result<()>>,
    pub(super) on_failed: Option<&'a dyn Fn(&Error, Duration)>,
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
                "on_retry_request",
                if self.on_retry_request.is_some() {
                    &"Installed"
                } else {
                    &"Not Installed"
                },
            )
            .field(
                "on_host_failed",
                if self.on_host_failed.is_some() {
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
                "on_failed",
                if self.on_failed.is_some() {
                    &"Installed"
                } else {
                    &"Not Installed"
                },
            )
            .finish()
    }
}
