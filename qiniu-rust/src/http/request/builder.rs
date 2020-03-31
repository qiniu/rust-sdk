use super::{
    super::{
        token::{Token, Version},
        Response,
    },
    HTTPError, HTTPResult, HeaderName, HeaderValue, Headers, Inner, Method, Request,
};
use crate::{utils::mime, Config, Credential};
use serde::Serialize;
use std::{borrow::Cow, collections::HashMap, time::Duration};

pub(crate) struct Builder<'a>(Inner<'a>);

impl<'a> Builder<'a> {
    pub(crate) fn new(config: Config, method: Method, path: &'a str, base_urls: &'a [&'a str]) -> Builder<'a> {
        Builder(Inner {
            config,
            method,
            base_urls,
            path,
            query: HashMap::new(),
            headers: Headers::new(),
            body: Vec::new(),
            token: None,
            read_body: false,
            idempotent: false,
            follow_redirection: false,
            on_uploading_progress: None,
            on_downloading_progress: None,
            on_response: None,
            on_error: None,
        })
    }

    pub(crate) fn header<K: Into<HeaderName<'a>>, V: Into<HeaderValue<'a>>>(mut self, key: K, value: V) -> Builder<'a> {
        self.0.headers.insert(key.into(), value.into());
        self
    }

    pub(crate) fn query<K: Into<Cow<'a, str>>, V: Into<Cow<'a, str>>>(mut self, key: K, value: V) -> Builder<'a> {
        self.0.query.insert(key.into(), value.into());
        self
    }

    pub(crate) fn token(mut self, version: Version, credential: Cow<'a, Credential>) -> Builder<'a> {
        self.0.token = Some(Token::new(version, credential));
        self
    }

    pub(crate) fn idempotent(mut self) -> Builder<'a> {
        self.0.idempotent = true;
        self
    }

    pub(crate) fn follow_redirection(mut self) -> Builder<'a> {
        self.0.follow_redirection = true;
        self
    }

    pub(crate) fn on_uploading_progress(mut self, callback: &'a dyn Fn(u64, u64)) -> Builder<'a> {
        self.0.on_uploading_progress = Some(callback);
        self
    }

    pub(crate) fn on_downloading_progress(mut self, callback: &'a dyn Fn(u64, u64)) -> Builder<'a> {
        self.0.on_downloading_progress = Some(callback);
        self
    }

    pub(crate) fn on_response(
        mut self,
        callback: &'a dyn Fn(&mut Response, Duration) -> HTTPResult<()>,
    ) -> Builder<'a> {
        self.0.on_response = Some(callback);
        self
    }

    pub(crate) fn on_error(mut self, callback: &'a dyn Fn(Option<&str>, &HTTPError, Duration)) -> Builder<'a> {
        self.0.on_error = Some(callback);
        self
    }

    pub(crate) fn accept_json(mut self) -> Builder<'a> {
        self = self.header("Accept", mime::JSON_MIME);
        self.0.read_body = true;
        self
    }

    pub(crate) fn no_body(mut self) -> Request<'a> {
        self = self.header("Content-Type", mime::FORM_MIME);
        self.build()
    }

    pub(crate) fn raw_body<T: Into<Vec<u8>>, S: Into<HeaderValue<'a>>>(
        mut self,
        content_type: S,
        body: T,
    ) -> Request<'a> {
        self = self.header("Content-Type", content_type);
        self.0.body = body.into();
        self.build()
    }

    pub(crate) fn json_body<T: Serialize>(mut self, body: &T) -> serde_json::Result<Request<'a>> {
        let serialized_body = serde_json::to_vec(body)?;
        self = self.header("Content-Type", mime::JSON_MIME);
        self.0.body = serialized_body;
        Ok(self.build())
    }

    fn build(self) -> Request<'a> {
        Request(self.0)
    }
}
