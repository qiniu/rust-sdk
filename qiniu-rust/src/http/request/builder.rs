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
use std::{collections::HashMap, result, sync::Arc, time::Duration};

error_chain! {
    foreign_links {
        JSONError(serde_json::error::Error);
        FormError(serde_urlencoded::ser::Error);
    }
}

pub struct Builder<'a> {
    parts: Parts<'a>,
    domains_manager: DomainsManager,
    host_freeze_duration: Duration,
}

pub type BuildResult<'a> = result::Result<Request<'a>, Error>;

impl<'a> Builder<'a> {
    pub fn new(
        auth: Arc<Auth>,
        config: Arc<Config>,
        method: Method,
        path: &'a str,
        hosts: &'a [&'a str],
    ) -> Builder<'a> {
        Builder {
            domains_manager: config.domains_manager().clone(),
            host_freeze_duration: *config.host_freeze_duration(),
            parts: Parts {
                auth: auth,
                config: config,
                method: method,
                hosts: hosts,
                path: path,
                query: HashMap::new(),
                headers: Headers::new(),
                body: None,
                token: Token::None,
            },
        }
    }

    pub fn header<K: Into<HeaderName>, V: Into<HeaderValue>>(mut self, key: K, value: V) -> Builder<'a> {
        self.parts.headers.insert(key.into(), value.into());
        self
    }

    pub fn query<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Builder<'a> {
        self.parts.query.insert(key.into(), value.into());
        self
    }

    pub fn token(mut self, token: Token) -> Builder<'a> {
        self.parts.token = token;
        self
    }

    pub fn no_body(self) -> Request<'a> {
        self.build()
    }

    pub fn raw_body<T: Into<Vec<u8>>, S: Into<String>>(mut self, content_type: S, body: T) -> Request<'a> {
        self.parts
            .headers
            .insert("Content-Type".to_string(), content_type.into());
        self.parts.body = Some(body.into());
        self.build()
    }

    pub fn json_body<T: Serialize>(mut self, body: &T) -> BuildResult<'a> {
        let serialized_body = serde_json::to_vec(body)?;
        self.parts
            .headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        self.parts.body = Some(serialized_body);
        Ok(self.build())
    }

    pub fn form_body<T: Serialize>(mut self, body: &T) -> BuildResult<'a> {
        let serialized_body = serde_urlencoded::to_string(body)?;
        self.parts.headers.insert(
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );
        self.parts.body = Some(serialized_body.into_bytes());
        Ok(self.build())
    }

    fn build(self) -> Request<'a> {
        Request {
            parts: self.parts,
            domains_manager: self.domains_manager,
            host_freeze_duration: self.host_freeze_duration,
        }
    }
}
