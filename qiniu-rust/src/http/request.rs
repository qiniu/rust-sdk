use super::super::config::Config;
use super::super::utils::auth::Auth;
use super::token::Token;
use qiniu_http::{HeaderName, HeaderValue, Headers, Method};
use serde::Serialize;
use std::{boxed::Box, error::Error, result, string::ToString, sync::Arc};

struct Parts {
    method: Method,
    hosts: Vec<String>,
    path: String,
    headers: Headers,
    body: Vec<u8>,
    auth: Arc<Auth>,
    config: Arc<Config>,
    token: Token,
}

pub struct Request {
    parts: Parts,
}

pub struct Builder {
    parts: Parts,
    error: Option<Box<Error>>,
}

impl Request {
    // fn send(mut self) -> Result<Request> {
    // let mut http_request_builder = http::Request::builder()::method(self.method);
    // }
}

impl Builder {
    pub(super) fn new<Hosts, Host, Path>(
        auth: Arc<Auth>,
        config: Arc<Config>,
        method: Method,
        path: Path,
        hosts: Hosts,
    ) -> Builder
    where
        Path: ToString,
        Host: ToString,
        Hosts: AsRef<[Host]>,
    {
        Builder {
            parts: Parts {
                auth: auth,
                config: config,
                method: method,
                hosts: hosts
                    .as_ref()
                    .into_iter()
                    .map(|host| host.to_string())
                    .collect(),
                path: path.to_string(),
                headers: Headers::new(),
                body: Vec::<u8>::new(),
                token: Token::None,
            },
            error: None,
        }
    }

    pub fn header<K: Into<HeaderName>, V: Into<HeaderValue>>(
        mut self,
        key: K,
        value: V,
    ) -> Builder {
        self.parts.headers.insert(key.into(), value.into());
        self
    }

    pub fn token(mut self, token: Token) -> Builder {
        if self.error.is_none() {
            self.parts.token = token;
        }
        self
    }

    pub fn no_body(self) -> result::Result<Request, Box<Error>> {
        self.build()
    }

    pub fn raw_body<T: Into<Vec<u8>>>(mut self, body: T) -> result::Result<Request, Box<Error>> {
        if self.error.is_none() {
            self.parts.body = body.into();
        }
        self.build()
    }

    pub fn json_body<T: Serialize>(mut self, body: &T) -> result::Result<Request, Box<Error>> {
        if self.error.is_none() {
            match serde_json::to_vec(body) {
                Ok(serialized_body) => {
                    self.parts.body = serialized_body;
                }
                Err(e) => {
                    self.error = Some(Box::new(e));
                }
            }
        }
        self.build()
    }

    pub fn form_body<T: Serialize>(mut self, body: &T) -> result::Result<Request, Box<Error>> {
        if self.error.is_none() {
            match serde_urlencoded::to_string(body) {
                Ok(serialized_body) => {
                    self.parts.body = serialized_body.into_bytes();
                }
                Err(e) => {
                    self.error = Some(Box::new(e));
                }
            }
        }
        self.build()
    }

    fn build(self) -> result::Result<Request, Box<Error>> {
        match self.error {
            Some(err) => Err(err),
            None => Ok(Request { parts: self.parts }),
        }
    }
}
