use super::{
    super::{super::config::Config, token::Token, DomainsManager},
    Parts, Request, ResponseCallback,
};
use error_chain::error_chain;
use qiniu_http::{HeaderName, HeaderValue, Headers, Method};
use serde::Serialize;
use std::{borrow::Cow, collections::HashMap, net::SocketAddr, result::Result as StdResult};

error_chain! {
    foreign_links {
        JSONError(serde_json::error::Error);
        FormError(serde_urlencoded::ser::Error);
    }
}

pub(crate) struct Builder<'a> {
    parts: Parts<'a>,
    domains_manager: DomainsManager,
}

impl<'a> Builder<'a> {
    pub(crate) fn new(config: Config, method: Method, path: &'a str, hosts: &'a [&'a str]) -> Builder<'a> {
        Builder {
            domains_manager: config.domains_manager().clone(),
            parts: Parts {
                config: config,
                method: method,
                hosts: hosts,
                path: path,
                query: None,
                headers: None,
                body: None,
                token: Token::None,
                read_body: false,
                idempotent: false,
                follow_redirection: false,
                response_callback: None,
                resolved_socket_addrs: &[],
                on_uploading_progress: None,
                on_downloading_progress: None,
            },
        }
    }

    pub(crate) fn header<K: Into<HeaderName<'a>>, V: Into<HeaderValue<'a>>>(mut self, key: K, value: V) -> Builder<'a> {
        match &mut self.parts.headers {
            Some(headers) => {
                headers.insert(key.into(), value.into());
            }
            None => {
                self.parts.headers = Some({
                    let mut h = Headers::with_capacity(4);
                    h.insert(key.into(), value.into());
                    h
                });
            }
        }
        self
    }

    pub(crate) fn query<K: Into<Cow<'a, str>>, V: Into<Cow<'a, str>>>(mut self, key: K, value: V) -> Builder<'a> {
        match self.parts.query {
            Some(ref mut query) => {
                query.insert(key.into(), value.into());
            }
            None => {
                self.parts.query = Some({
                    let mut q = HashMap::with_capacity(4);
                    q.insert(key.into(), value.into());
                    q
                });
            }
        }
        self
    }

    pub(crate) fn token(mut self, token: Token) -> Builder<'a> {
        self.parts.token = token;
        self
    }

    pub(crate) fn idempotent(mut self) -> Builder<'a> {
        self.parts.idempotent = true;
        self
    }

    pub(crate) fn follow_redirection(mut self) -> Builder<'a> {
        self.parts.follow_redirection = true;
        self
    }

    pub(crate) fn resolved_socket_addrs(mut self, resolved_socket_addrs: &'a [SocketAddr]) -> Builder<'a> {
        self.parts.resolved_socket_addrs = resolved_socket_addrs;
        self
    }

    pub(crate) fn response_callback(mut self, callback: &'a dyn ResponseCallback) -> Builder<'a> {
        self.parts.response_callback = Some(callback);
        self
    }

    pub(crate) fn on_uploading_progress(mut self, callback: &'a dyn Fn(usize, usize)) -> Builder<'a> {
        self.parts.on_uploading_progress = Some(callback);
        self
    }

    pub(crate) fn on_downloading_progress(mut self, callback: &'a dyn Fn(usize, usize)) -> Builder<'a> {
        self.parts.on_downloading_progress = Some(callback);
        self
    }

    pub(crate) fn accept_json(mut self) -> Builder<'a> {
        self = self.header("Accept", "application/json");
        self.parts.read_body = true;
        self
    }

    pub(crate) fn no_body(self) -> Request<'a> {
        self.build()
    }

    pub(crate) fn raw_body<T: Into<Vec<u8>>, S: Into<HeaderValue<'a>>>(
        mut self,
        content_type: S,
        body: T,
    ) -> Request<'a> {
        self = self.header("Content-Type", content_type);
        self.parts.body = Some(body.into());
        self.build()
    }

    pub(crate) fn json_body<T: Serialize>(mut self, body: &T) -> StdResult<Request<'a>, Error> {
        let serialized_body = serde_json::to_vec(body)?;
        self = self.header("Content-Type", "application/json");
        self.parts.body = Some(serialized_body);
        Ok(self.build())
    }

    pub(crate) fn form_body<T: Serialize>(mut self, body: &T) -> StdResult<Request<'a>, Error> {
        let serialized_body = serde_urlencoded::to_string(body)?;
        self = self.header("Content-Type", "application/x-www-form-urlencoded");
        self.parts.body = Some(serialized_body.into_bytes());
        Ok(self.build())
    }

    fn build(self) -> Request<'a> {
        Request {
            parts: self.parts,
            domains_manager: self.domains_manager,
        }
    }
}
