use super::{
    super::{
        super::{config::Config, utils::auth::Auth},
        token::Token,
        DomainsManager,
    },
    Parts, Request,
};
use error_chain::error_chain;
use qiniu_http::{HeaderName, HeaderValue, Headers, Method};
use serde::Serialize;
use std::{result, sync::Arc, time::Duration};

error_chain! {
    foreign_links {
        JSONError(serde_json::error::Error);
        FormError(serde_urlencoded::ser::Error);
    }
}

pub struct Builder {
    parts: Parts,
    domains_manager: DomainsManager,
    host_freeze_duration: Duration,
    error: Option<Error>,
}

pub type BuildResult = result::Result<Request, Error>;

impl Builder {
    pub fn new<Hosts, Host, Path>(
        auth: Arc<Auth>,
        config: Arc<Config>,
        method: Method,
        path: Path,
        hosts: Hosts,
    ) -> Builder
    where
        Path: Into<String>,
        Host: Into<String>,
        Hosts: Into<Vec<Host>>,
    {
        Builder {
            parts: Parts {
                auth: auth,
                config: config.clone(),
                method: method,
                hosts: hosts.into().into_iter().map(|host| host.into()).collect(),
                path: path.into(),
                headers: Headers::new(),
                body: Vec::<u8>::new(),
                token: Token::None,
            },
            domains_manager: config.domains_manager().clone(),
            host_freeze_duration: *config.host_freeze_duration(),
            error: None,
        }
    }

    pub fn header<K: Into<HeaderName>, V: Into<HeaderValue>>(mut self, key: K, value: V) -> Builder {
        self.parts.headers.insert(key.into(), value.into());
        self
    }

    pub fn token(mut self, token: Token) -> Builder {
        if self.error.is_none() {
            self.parts.token = token;
        }
        self
    }

    pub fn no_body(self) -> BuildResult {
        self.build()
    }

    pub fn raw_body<T: Into<Vec<u8>>, S: Into<String>>(mut self, content_type: S, body: T) -> BuildResult {
        if self.error.is_none() {
            self.parts
                .headers
                .insert("Content-Type".to_string(), content_type.into());
            self.parts.body = body.into();
        }
        self.build()
    }

    pub fn json_body<T: Serialize>(mut self, body: &T) -> BuildResult {
        if self.error.is_none() {
            match serde_json::to_vec(body) {
                Ok(serialized_body) => {
                    self.parts
                        .headers
                        .insert("Content-Type".to_string(), "application/json".to_string());
                    self.parts.body = serialized_body;
                }
                Err(e) => {
                    self.error = Some(e.into());
                }
            }
        }
        self.build()
    }

    pub fn form_body<T: Serialize>(mut self, body: &T) -> BuildResult {
        if self.error.is_none() {
            match serde_urlencoded::to_string(body) {
                Ok(serialized_body) => {
                    self.parts.headers.insert(
                        "Content-Type".to_string(),
                        "application/x-www-form-urlencoded".to_string(),
                    );
                    self.parts.body = serialized_body.into_bytes();
                }
                Err(e) => {
                    self.error = Some(e.into());
                }
            }
        }
        self.build()
    }

    fn build(self) -> BuildResult {
        match self.error {
            Some(err) => Err(err),
            None => Ok(Request {
                parts: self.parts,
                domains_manager: self.domains_manager,
                host_freeze_duration: self.host_freeze_duration,
            }),
        }
    }
}
