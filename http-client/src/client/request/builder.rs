use super::{
    super::{
        super::{IntoEndpoints, ServiceName},
        callbacks::{
            OnBody, OnDomainChosen, OnError, OnHeader, OnProgress, OnRequest, OnRetry,
            OnStatusCode, OnSuccess, OnToChooseDomain,
        },
        request_call, APIResult, Authorization, CallbacksBuilder, Client, ResponseError,
        ResponseErrorKind, SyncResponse,
    },
    request_data::RequestData,
    Idempotent, QueryPairKey, QueryPairValue, QueryPairs, Request,
};
use mime::{Mime, APPLICATION_JSON, APPLICATION_OCTET_STREAM};
use qiniu_http::{HeaderName, HeaderValue, Headers, Method, RequestBody};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{from_reader as parse_json_from_reader, Result as JSONResult};
use std::{borrow::Cow, fmt, time::Duration};

pub struct RequestBuilder<'r> {
    client: &'r Client,
    service_name: ServiceName,
    into_endpoints: IntoEndpoints<'r>,
    callbacks: CallbacksBuilder,
    data: RequestData<'r>,
    appended_user_agent: Cow<'r, str>,
}

impl<'r> RequestBuilder<'r> {
    pub(in super::super) fn new(
        client: &'r Client,
        method: Method,
        into_endpoints: IntoEndpoints<'r>,
        service_name: ServiceName,
    ) -> Self {
        Self {
            client,
            service_name,
            into_endpoints,
            callbacks: Default::default(),
            appended_user_agent: Default::default(),
            data: RequestData {
                method,
                use_https: None,
                path: Default::default(),
                query: Default::default(),
                query_pairs: Default::default(),
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
    pub fn path(mut self, path: impl Into<Cow<'r, str>>) -> Self {
        self.data.path = path.into();
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
    pub fn query(mut self, query: impl Into<Cow<'r, str>>) -> Self {
        self.data.query = query.into();
        self
    }

    #[inline]
    pub fn query_pairs(mut self, query_pairs: QueryPairs<'r>) -> Self {
        self.data.query_pairs = query_pairs;
        self
    }

    #[inline]
    pub fn append_query_pair(
        mut self,
        query_pair_key: impl Into<QueryPairKey<'r>>,
        query_pair_value: impl Into<QueryPairValue<'r>>,
    ) -> Self {
        self.data
            .query_pairs
            .push((query_pair_key.into(), query_pair_value.into()));
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
    pub fn on_uploading_progress(mut self, callback: OnProgress) -> Self {
        self.callbacks = self.callbacks.on_uploading_progress(callback);
        self
    }

    #[inline]
    pub fn on_downloading_progress(mut self, callback: OnProgress) -> Self {
        self.callbacks = self.callbacks.on_downloading_progress(callback);
        self
    }

    #[inline]
    pub fn on_send_request_body(mut self, callback: OnBody) -> Self {
        self.callbacks = self.callbacks.on_send_request_body(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_status(mut self, callback: OnStatusCode) -> Self {
        self.callbacks = self.callbacks.on_receive_response_status(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_body(mut self, callback: OnBody) -> Self {
        self.callbacks = self.callbacks.on_receive_response_body(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_header(mut self, callback: OnHeader) -> Self {
        self.callbacks = self.callbacks.on_receive_response_header(callback);
        self
    }

    #[inline]
    pub fn on_to_choose_domain(mut self, callback: OnToChooseDomain) -> Self {
        self.callbacks = self.callbacks.on_to_choose_domain(callback);
        self
    }

    #[inline]
    pub fn on_domain_chosen(mut self, callback: OnDomainChosen) -> Self {
        self.callbacks = self.callbacks.on_domain_chosen(callback);
        self
    }

    #[inline]
    pub fn on_before_request_signed(mut self, callback: OnRequest) -> Self {
        self.callbacks = self.callbacks.on_before_request_signed(callback);
        self
    }

    #[inline]
    pub fn on_after_request_signed(mut self, callback: OnRequest) -> Self {
        self.callbacks = self.callbacks.on_after_request_signed(callback);
        self
    }

    #[inline]
    pub fn on_success(mut self, callback: OnSuccess) -> Self {
        self.callbacks = self.callbacks.on_success(callback);
        self
    }

    #[inline]
    pub fn on_error(mut self, callback: OnError) -> Self {
        self.callbacks = self.callbacks.on_error(callback);
        self
    }

    #[inline]
    pub fn on_before_retry_delay(mut self, callback: OnRetry) -> Self {
        self.callbacks = self.callbacks.on_before_retry_delay(callback);
        self
    }

    #[inline]
    pub fn on_after_retry_delay(mut self, callback: OnRetry) -> Self {
        self.callbacks = self.callbacks.on_after_retry_delay(callback);
        self
    }

    #[inline]
    pub fn call(self) -> APIResult<SyncResponse> {
        request_call(self.build())
    }

    pub fn parse_json<T: DeserializeOwned>(mut self) -> APIResult<T> {
        self.data.read_body = true;
        self.data
            .headers
            .insert("Accept".into(), APPLICATION_JSON.to_string().into());
        let mut response = request_call(self.build())?;
        let body = parse_json_from_reader(response.body_mut())
            .map_err(|err| ResponseError::new(ResponseErrorKind::ParseResponseError, err))?;
        Ok(body)
    }

    #[inline]
    pub(in super::super) fn build(self) -> Request<'r> {
        let appended_user_agent =
            self.client.appended_user_agent().to_owned() + &self.appended_user_agent;
        Request::new(
            self.client,
            self.service_name,
            self.into_endpoints,
            self.callbacks.build(),
            self.data,
            appended_user_agent.into_boxed_str(),
        )
    }
}

impl fmt::Debug for RequestBuilder<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestBuilder")
            .field("service_name", &self.service_name)
            .field("into_endpoints", &self.into_endpoints)
            .field("callbacks", &self.callbacks)
            .field("data", &self.data)
            .field("appended_user_agent", &self.appended_user_agent)
            .finish()
    }
}
