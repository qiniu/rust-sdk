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

pub struct Builder {
    parts: Parts,
    domains_manager: DomainsManager,
    host_freeze_duration: Duration,
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
            domains_manager: config.domains_manager().clone(),
            host_freeze_duration: *config.host_freeze_duration(),
            parts: Parts {
                auth: auth,
                config: config,
                method: method,
                hosts: hosts.into().into_iter().map(|host| host.into()).collect(),
                path: path.into(),
                query: HashMap::new(),
                headers: Headers::new(),
                body: Vec::<u8>::new(),
                token: Token::None,
            },
        }
    }

    pub fn header<K: Into<HeaderName>, V: Into<HeaderValue>>(mut self, key: K, value: V) -> Builder {
        self.parts.headers.insert(key.into(), value.into());
        self
    }

    pub fn query<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Builder {
        self.parts.query.insert(key.into(), value.into());
        self
    }

    pub fn token(mut self, token: Token) -> Builder {
        self.parts.token = token;
        self
    }

    pub fn no_body(self) -> Request {
        self.build()
    }

    pub fn raw_body<T: Into<Vec<u8>>, S: Into<String>>(mut self, content_type: S, body: T) -> Request {
        self.parts
            .headers
            .insert("Content-Type".to_string(), content_type.into());
        self.parts.body = body.into();
        self.build()
    }

    pub fn json_body<T: Serialize>(mut self, body: &T) -> BuildResult {
        let serialized_body = serde_json::to_vec(body)?;
        self.parts
            .headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        self.parts.body = serialized_body;
        Ok(self.build())
    }

    pub fn form_body<T: Serialize>(mut self, body: &T) -> BuildResult {
        let serialized_body = serde_urlencoded::to_string(body)?;
        self.parts.headers.insert(
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );
        self.parts.body = serialized_body.into_bytes();
        Ok(self.build())
    }

    fn build(self) -> Request {
        Request {
            parts: self.parts,
            domains_manager: self.domains_manager,
            host_freeze_duration: self.host_freeze_duration,
        }
    }
}
