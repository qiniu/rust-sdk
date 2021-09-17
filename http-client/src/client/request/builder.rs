use super::{
    super::{
        super::{IntoEndpoints, ServiceName},
        callbacks::{
            OnDomainResolved, OnError, OnHeader, OnIPsChosen, OnProgress, OnRequest, OnRetry,
            OnStatusCode, OnSuccess, OnToChooseIPs, OnToResolveDomain,
        },
        request_call, APIResult, Authorization, CallbacksBuilder, HTTPClient, SyncResponse,
    },
    request_data::RequestData,
    Idempotent, QueryPairKey, QueryPairValue, QueryPairs, Request,
};
use mime::{Mime, APPLICATION_JSON, APPLICATION_OCTET_STREAM, APPLICATION_WWW_FORM_URLENCODED};
use qiniu_http::{
    header::{ACCEPT, CONTENT_TYPE},
    Extensions, HeaderMap, HeaderName, HeaderValue, Method, RequestBody, Version,
};
use serde::Serialize;
use serde_json::Result as JSONResult;
use std::borrow::{Borrow, Cow};

#[cfg(feature = "async")]
use super::super::{async_request_call, AsyncResponse};

#[derive(Debug)]
pub struct RequestBuilder<'r> {
    http_client: &'r HTTPClient,
    service_name: ServiceName,
    into_endpoints: IntoEndpoints<'r>,
    callbacks: CallbacksBuilder,
    data: RequestData<'r>,
    appended_user_agent: Cow<'r, str>,
    extensions: Extensions,
}

impl<'r> RequestBuilder<'r> {
    pub(in super::super) fn new(
        http_client: &'r HTTPClient,
        method: Method,
        into_endpoints: IntoEndpoints<'r>,
        service_name: ServiceName,
    ) -> Self {
        Self {
            http_client,
            service_name,
            into_endpoints,
            callbacks: Default::default(),
            appended_user_agent: Default::default(),
            extensions: Default::default(),
            data: RequestData {
                method,
                use_https: None,
                version: Default::default(),
                path: Default::default(),
                query: Default::default(),
                query_pairs: Default::default(),
                headers: Default::default(),
                body: Default::default(),
                authorization: None,
                idempotent: Default::default(),
            },
        }
    }

    #[inline]
    pub fn use_https(mut self, use_https: bool) -> Self {
        self.data.use_https = Some(use_https);
        self
    }

    #[inline]
    pub fn version(mut self, version: Version) -> Self {
        self.data.version = version;
        self
    }

    #[inline]
    pub fn path(mut self, path: impl Into<Cow<'r, str>>) -> Self {
        self.data.path = path.into();
        self
    }

    #[inline]
    pub fn headers(mut self, headers: impl Into<Cow<'r, HeaderMap>>) -> Self {
        self.data.headers = headers.into();
        self
    }

    #[inline]
    pub fn set_header(
        mut self,
        header_name: impl Into<HeaderName>,
        header_value: impl Into<HeaderValue>,
    ) -> Self {
        self.data
            .headers
            .to_mut()
            .insert(header_name.into(), header_value.into());
        self
    }

    #[inline]
    pub fn body(mut self, body: impl Into<RequestBody<'r>>, content_type: Option<Mime>) -> Self {
        self.data.body = body.into();
        self.set_header(
            CONTENT_TYPE,
            HeaderValue::from_str(content_type.unwrap_or(APPLICATION_OCTET_STREAM).as_ref())
                .unwrap(),
        )
    }

    #[inline]
    pub fn json(mut self, body: impl Serialize) -> JSONResult<Self> {
        self.data.body = serde_json::to_vec(&body)?.into();
        Ok(self.set_header(
            CONTENT_TYPE,
            HeaderValue::from_str(APPLICATION_JSON.as_ref()).unwrap(),
        ))
    }

    #[inline]
    pub fn post_form<I, K, V>(mut self, iter: I) -> JSONResult<Self>
    where
        I: IntoIterator,
        I::Item: Borrow<(K, Option<V>)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut form = form_urlencoded::Serializer::new(String::new());
        for pair in iter {
            let (k, v) = pair.borrow();
            if let Some(v) = v {
                form.append_pair(k.as_ref(), v.as_ref());
            } else {
                form.append_key_only(k.as_ref());
            }
        }
        self.data.body = form.finish().into_bytes().into();
        Ok(self.set_header(
            CONTENT_TYPE,
            HeaderValue::from_str(APPLICATION_WWW_FORM_URLENCODED.as_ref()).unwrap(),
        ))
    }

    #[inline]
    pub fn accept_json(self) -> Self {
        self.set_header(
            ACCEPT,
            HeaderValue::from_str(APPLICATION_JSON.as_ref()).unwrap(),
        )
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
    pub fn extensions(mut self, extensions: Extensions) -> Self {
        self.extensions = extensions;
        self
    }

    #[inline]
    pub fn add_extension<T: Send + Sync + 'static>(mut self, val: T) -> Self {
        self.extensions.insert(val);
        self
    }

    #[inline]
    pub fn on_uploading_progress(mut self, callback: OnProgress) -> Self {
        self.callbacks = self.callbacks.on_uploading_progress(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_status(mut self, callback: OnStatusCode) -> Self {
        self.callbacks = self.callbacks.on_receive_response_status(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_header(mut self, callback: OnHeader) -> Self {
        self.callbacks = self.callbacks.on_receive_response_header(callback);
        self
    }

    #[inline]
    pub fn on_to_resolve_domain(mut self, callback: OnToResolveDomain) -> Self {
        self.callbacks = self.callbacks.on_to_resolve_domain(callback);
        self
    }

    #[inline]
    pub fn on_domain_resolved(mut self, callback: OnDomainResolved) -> Self {
        self.callbacks = self.callbacks.on_domain_resolved(callback);
        self
    }

    #[inline]
    pub fn on_to_choose_ips(mut self, callback: OnToChooseIPs) -> Self {
        self.callbacks = self.callbacks.on_to_choose_ips(callback);
        self
    }

    #[inline]
    pub fn on_ips_chosen(mut self, callback: OnIPsChosen) -> Self {
        self.callbacks = self.callbacks.on_ips_chosen(callback);
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
    pub fn on_before_backoff(mut self, callback: OnRetry) -> Self {
        self.callbacks = self.callbacks.on_before_backoff(callback);
        self
    }

    #[inline]
    pub fn on_after_backoff(mut self, callback: OnRetry) -> Self {
        self.callbacks = self.callbacks.on_after_backoff(callback);
        self
    }

    #[inline]
    pub fn call(self) -> APIResult<SyncResponse> {
        request_call(self.build())
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub async fn async_call(self) -> APIResult<AsyncResponse> {
        async_request_call(self.build()).await
    }

    #[inline]
    pub(in super::super) fn build(self) -> Request<'r> {
        let appended_user_agent =
            self.http_client.appended_user_agent().to_owned() + &self.appended_user_agent;
        Request::new(
            self.http_client,
            self.service_name,
            self.into_endpoints,
            self.callbacks.build(),
            self.data,
            appended_user_agent.into_boxed_str(),
            self.extensions,
        )
    }
}
