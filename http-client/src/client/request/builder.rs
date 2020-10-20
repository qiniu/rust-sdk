use super::{
    super::{
        super::regions::IntoDomains,
        callbacks::{
            OnBody, OnError, OnHeader, OnProgress, OnRequest, OnRetry, OnStatusCode, OnSuccess,
        },
        Authorization, CallbacksBuilder, Client,
    },
    request_data::RequestData,
    Idempotent, Queries, QueryKey, QueryValue, Request,
};
use mime::{Mime, APPLICATION_JSON, APPLICATION_OCTET_STREAM};
use qiniu_http::{HeaderName, HeaderValue, Headers, Method, RequestBody};
use serde::Serialize;
use serde_json::Result as JSONResult;
use std::{borrow::Cow, fmt, time::Duration};

pub struct RequestBuilder<'r> {
    client: &'r Client,
    into_domains: IntoDomains<'r>,
    callbacks: CallbacksBuilder,
    data: RequestData<'r>,
    appended_user_agent: Cow<'r, str>,
}

impl<'r> RequestBuilder<'r> {
    pub(super) fn new(client: &'r Client, method: Method, into_domains: IntoDomains<'r>) -> Self {
        Self {
            client,
            into_domains,
            callbacks: Default::default(),
            appended_user_agent: Default::default(),
            data: RequestData {
                method,
                use_https: None,
                queries: Default::default(),
                headers: Default::default(),
                body: Default::default(),
                authorization: None,
                idempotent: Default::default(),
                read_body: false,
                follow_redirection: false,
                connect_timeout: None,
                request_timeout: None,
                tcp_keepalive_idle_timeout: None,
                tcp_keepalive_probe_interval: None,
                low_transfer_speed: None,
                low_transfer_speed_timeout: None,
            },
        }
    }

    #[inline]
    pub fn use_https(mut self, use_https: bool) -> Self {
        self.data.use_https = Some(use_https);
        self
    }

    #[inline]
    pub fn headers(mut self, headers: Headers<'r>) -> Self {
        self.data.headers = headers;
        self
    }

    #[inline]
    pub fn set_header(
        mut self,
        header_name: impl Into<HeaderName<'r>>,
        header_value: impl Into<HeaderValue<'r>>,
    ) -> Self {
        self.data
            .headers
            .insert(header_name.into(), header_value.into());
        self
    }

    #[inline]
    pub fn body(mut self, body: impl Into<RequestBody<'r>>, content_type: Option<Mime>) -> Self {
        self.data.body = body.into();
        self.data.headers.insert(
            "Content-Type".into(),
            content_type
                .unwrap_or(APPLICATION_OCTET_STREAM)
                .to_string()
                .into(),
        );
        self
    }

    #[inline]
    pub fn json(mut self, body: impl Serialize) -> JSONResult<Self> {
        self.data.body = serde_json::to_vec(&body)?.into();
        self.data
            .headers
            .insert("Content-Type".into(), APPLICATION_JSON.to_string().into());
        Ok(self)
    }

    #[inline]
    pub fn queries(mut self, queries: Queries<'r>) -> Self {
        self.data.queries = queries;
        self
    }

    #[inline]
    pub fn set_query(
        mut self,
        query_key: impl Into<QueryKey<'r>>,
        query_value: impl Into<QueryValue<'r>>,
    ) -> Self {
        self.data
            .queries
            .insert(query_key.into(), query_value.into());
        self
    }

    #[inline]
    pub fn appended_user_agent(mut self, user_agent: impl Into<Cow<'r, str>>) -> Self {
        self.appended_user_agent = user_agent.into();
        self
    }

    #[inline]
    pub fn authorization(mut self, authorization: Authorization) -> Self {
        self.data.authorization = Some(authorization);
        self
    }

    #[inline]
    pub fn idempotent(mut self, idempotent: Idempotent) -> Self {
        self.data.idempotent = idempotent;
        self
    }

    #[inline]
    pub fn accept_json(mut self) -> Self {
        self.data.read_body = true;
        self.data
            .headers
            .insert("Accept".into(), APPLICATION_JSON.to_string().into());
        self
    }

    #[inline]
    pub fn follow_redirection(mut self, follow_redirection: bool) -> Self {
        self.data.follow_redirection = follow_redirection;
        self
    }

    #[inline]
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.data.connect_timeout = Some(timeout);
        self
    }

    #[inline]
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.data.request_timeout = Some(timeout);
        self
    }

    #[inline]
    pub fn tcp_keepalive_idle_timeout(mut self, timeout: Duration) -> Self {
        self.data.tcp_keepalive_idle_timeout = Some(timeout);
        self
    }

    #[inline]
    pub fn tcp_keepalive_probe_interval(mut self, interval: Duration) -> Self {
        self.data.tcp_keepalive_probe_interval = Some(interval);
        self
    }

    #[inline]
    pub fn low_transfer_speed(mut self, speed: u32) -> Self {
        self.data.low_transfer_speed = Some(speed);
        self
    }

    #[inline]
    pub fn low_transfer_speed_timeout(mut self, timeout: Duration) -> Self {
        self.data.low_transfer_speed_timeout = Some(timeout);
        self
    }

    #[inline]
    pub fn on_uploading_progress(mut self, callback: impl Into<OnProgress>) -> Self {
        self.callbacks = self.callbacks.on_uploading_progress(callback);
        self
    }

    #[inline]
    pub fn on_downloading_progress(mut self, callback: impl Into<OnProgress>) -> Self {
        self.callbacks = self.callbacks.on_downloading_progress(callback);
        self
    }

    #[inline]
    pub fn on_request(mut self, callback: impl Into<OnRequest>) -> Self {
        self.callbacks = self.callbacks.on_request(callback);
        self
    }

    #[inline]
    pub fn on_send_request_body(mut self, callback: impl Into<OnBody>) -> Self {
        self.callbacks = self.callbacks.on_send_request_body(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_status(mut self, callback: impl Into<OnStatusCode>) -> Self {
        self.callbacks = self.callbacks.on_receive_response_status(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_body(mut self, callback: impl Into<OnBody>) -> Self {
        self.callbacks = self.callbacks.on_receive_response_body(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_header(mut self, callback: impl Into<OnHeader>) -> Self {
        self.callbacks = self.callbacks.on_receive_response_header(callback);
        self
    }

    #[inline]
    pub fn on_success(mut self, callback: impl Into<OnSuccess>) -> Self {
        self.callbacks = self.callbacks.on_success(callback);
        self
    }

    #[inline]
    pub fn on_error(mut self, callback: impl Into<OnError>) -> Self {
        self.callbacks = self.callbacks.on_error(callback);
        self
    }

    #[inline]
    pub fn on_retry(mut self, callback: impl Into<OnRetry>) -> Self {
        self.callbacks = self.callbacks.on_retry(callback);
        self
    }

    #[inline]
    pub fn build(self) -> Request<'r> {
        let appended_user_agent =
            self.client.appended_user_agent().to_owned() + &self.appended_user_agent;
        Request::new(
            self.client,
            self.into_domains,
            self.callbacks.build(),
            self.data,
            appended_user_agent.into_boxed_str(),
        )
    }
}

impl fmt::Debug for RequestBuilder<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestBuilder")
            .field("into_domains", &self.into_domains)
            .field("callbacks", &self.callbacks)
            .field("data", &self.data)
            .field("appended_user_agent", &self.appended_user_agent)
            .finish()
    }
}
