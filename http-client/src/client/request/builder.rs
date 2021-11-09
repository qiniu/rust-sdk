use super::{
    super::{
        super::{IntoEndpoints, ServiceName},
        callbacks::{
            OnDomainResolved, OnError, OnHeader, OnIPsChosen, OnProgress, OnRequest, OnRetry,
            OnStatusCode, OnSuccess, OnToChooseIPs, OnToResolveDomain,
        },
        request_call, ApiResult, Authorization, CallbacksBuilder, HttpClient, SyncResponse,
    },
    multipart::SyncMultipart,
    request_metadata::RequestMetadata,
    Idempotent, QueryPairKey, QueryPairValue, QueryPairs, SyncRequest,
};
use mime::{
    Mime, APPLICATION_JSON, APPLICATION_OCTET_STREAM, APPLICATION_WWW_FORM_URLENCODED,
    MULTIPART_FORM_DATA,
};
use qiniu_http::{
    header::{ACCEPT, CONTENT_TYPE},
    Extensions, HeaderMap, HeaderName, HeaderValue, Method, Reset, SyncRequestBody, UserAgent,
    Version,
};
use serde::Serialize;
use serde_json::Result as JsonResult;
use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    io::{Read, Result as IoResult},
};

#[cfg(feature = "async")]
use {
    super::{
        super::{async_request_call, AsyncResponse},
        multipart::AsyncMultipart,
        AsyncRequest,
    },
    futures::io::AsyncRead,
    qiniu_http::{AsyncRequestBody, AsyncReset},
};

#[derive(Debug)]
pub struct RequestBuilder<'r, B: 'r> {
    http_client: &'r HttpClient,
    service_names: &'r [ServiceName],
    into_endpoints: IntoEndpoints<'r>,
    callbacks: CallbacksBuilder,
    metadata: RequestMetadata<'r>,
    body: B,
    appended_user_agent: UserAgent,
    extensions: Extensions,
}

impl<'r, B: Default> RequestBuilder<'r, B> {
    pub(in super::super) fn new(
        http_client: &'r HttpClient,
        method: Method,
        into_endpoints: IntoEndpoints<'r>,
        service_names: &'r [ServiceName],
    ) -> Self {
        Self {
            http_client,
            service_names,
            into_endpoints,
            callbacks: Default::default(),
            appended_user_agent: Default::default(),
            extensions: Default::default(),
            body: Default::default(),
            metadata: RequestMetadata {
                method,
                use_https: None,
                version: Default::default(),
                path: Default::default(),
                query: Default::default(),
                query_pairs: Default::default(),
                headers: Default::default(),
                authorization: None,
                idempotent: Default::default(),
            },
        }
    }
}

impl<'r, B> RequestBuilder<'r, B> {
    #[inline]
    pub fn use_https(mut self, use_https: bool) -> Self {
        self.metadata.use_https = Some(use_https);
        self
    }

    #[inline]
    pub fn version(mut self, version: Version) -> Self {
        self.metadata.version = version;
        self
    }

    #[inline]
    pub fn path(mut self, path: impl Into<Cow<'r, str>>) -> Self {
        self.metadata.path = path.into();
        self
    }

    #[inline]
    pub fn headers(mut self, headers: impl Into<Cow<'r, HeaderMap>>) -> Self {
        self.metadata.headers = headers.into();
        self
    }

    #[inline]
    pub fn set_header(
        mut self,
        header_name: impl Into<HeaderName>,
        header_value: impl Into<HeaderValue>,
    ) -> Self {
        self.metadata
            .headers
            .to_mut()
            .insert(header_name.into(), header_value.into());
        self
    }

    #[inline]
    fn set_content_type(self, content_type: Option<Mime>) -> Self {
        self.set_header(
            CONTENT_TYPE,
            HeaderValue::from_str(
                content_type
                    .as_ref()
                    .unwrap_or(&APPLICATION_OCTET_STREAM)
                    .as_ref(),
            )
            .unwrap(),
        )
    }

    #[inline]
    pub fn accept_json(self) -> Self {
        self.set_accept(APPLICATION_JSON)
    }

    #[inline]
    pub fn accept_application_octet_stream(self) -> Self {
        self.set_accept(APPLICATION_OCTET_STREAM)
    }

    #[inline]
    fn set_accept(self, accept: Mime) -> Self {
        self.set_header(ACCEPT, HeaderValue::from_str(accept.as_ref()).unwrap())
    }

    #[inline]
    pub fn query(mut self, query: impl Into<Cow<'r, str>>) -> Self {
        self.metadata.query = query.into();
        self
    }

    #[inline]
    pub fn query_pairs(mut self, query_pairs: QueryPairs<'r>) -> Self {
        self.metadata.query_pairs = query_pairs;
        self
    }

    #[inline]
    pub fn append_query_pair(
        mut self,
        query_pair_key: impl Into<QueryPairKey<'r>>,
        query_pair_value: impl Into<QueryPairValue<'r>>,
    ) -> Self {
        self.metadata
            .query_pairs
            .push((query_pair_key.into(), query_pair_value.into()));
        self
    }

    #[inline]
    pub fn appended_user_agent(mut self, user_agent: impl Into<UserAgent>) -> Self {
        self.appended_user_agent = user_agent.into();
        self
    }

    #[inline]
    pub fn authorization(mut self, authorization: Authorization) -> Self {
        self.metadata.authorization = Some(authorization);
        self
    }

    #[inline]
    pub fn idempotent(mut self, idempotent: Idempotent) -> Self {
        self.metadata.idempotent = idempotent;
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
    fn get_appended_user_agent(&self) -> UserAgent {
        let mut appended_user_agent = self.http_client.appended_user_agent().to_owned();
        appended_user_agent.push_str(self.appended_user_agent.as_str());
        appended_user_agent
    }
}

pub type SyncRequestBuilder<'r> = RequestBuilder<'r, SyncRequestBody<'r>>;

impl<'r> SyncRequestBuilder<'r> {
    #[inline]
    pub fn stream_as_body(
        mut self,
        body: impl Read + Reset + Debug + Send + Sync + 'static,
        content_length: u64,
        content_type: Option<Mime>,
    ) -> Self {
        self.body = SyncRequestBody::from_reader(body, content_length);
        self.set_content_type(content_type)
    }

    #[inline]
    pub fn referenced_stream_as_body<T: Read + Reset + Debug + Send + Sync>(
        mut self,
        body: &'r mut T,
        content_length: u64,
        content_type: Option<Mime>,
    ) -> Self {
        self.body = SyncRequestBody::from_referenced_reader(body, content_length);
        self.set_content_type(content_type)
    }

    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>, content_type: Option<Mime>) -> Self {
        self.body = SyncRequestBody::from_bytes(body.into());
        self.set_content_type(content_type)
    }

    #[inline]
    pub fn referenced_bytes_as_body(mut self, body: &'r [u8], content_type: Option<Mime>) -> Self {
        self.body = SyncRequestBody::from_referenced_bytes(body);
        self.set_content_type(content_type)
    }

    #[inline]
    pub fn json(self, body: impl Serialize) -> JsonResult<Self> {
        Ok(self.bytes_as_body(serde_json::to_vec(&body)?, Some(APPLICATION_JSON)))
    }

    #[inline]
    pub fn post_form<I, K, V>(self, iter: I) -> Self
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
        self.bytes_as_body(
            form.finish().into_bytes(),
            Some(APPLICATION_WWW_FORM_URLENCODED),
        )
    }

    #[inline]
    pub fn multipart(self, multipart: SyncMultipart) -> IoResult<Self> {
        let mut buf = Vec::new();
        multipart.into_read().read_to_end(&mut buf)?;
        Ok(self.bytes_as_body(buf, Some(MULTIPART_FORM_DATA)))
    }

    #[inline]
    pub fn call(self) -> ApiResult<SyncResponse> {
        request_call(self.build())
    }

    #[inline]
    pub(in super::super) fn build(self) -> SyncRequest<'r> {
        let appended_user_agent = self.get_appended_user_agent();
        SyncRequest::new(
            self.http_client,
            self.service_names,
            self.into_endpoints,
            self.callbacks.build(),
            self.metadata,
            self.body,
            appended_user_agent,
            self.extensions,
        )
    }
}

#[cfg(feature = "async")]
pub type AsyncRequestBuilder<'r> = RequestBuilder<'r, AsyncRequestBody<'r>>;

#[cfg(feature = "async")]
impl<'r> AsyncRequestBuilder<'r> {
    #[inline]
    pub fn stream_as_body(
        mut self,
        body: impl AsyncRead + AsyncReset + Unpin + Debug + Send + Sync + 'static,
        content_length: u64,
        content_type: Option<Mime>,
    ) -> Self {
        self.body = AsyncRequestBody::from_reader(body, content_length);
        self.set_content_type(content_type)
    }

    #[inline]
    pub fn referenced_stream_as_body<T: AsyncRead + AsyncReset + Unpin + Debug + Send + Sync>(
        mut self,
        body: &'r mut T,
        content_length: u64,
        content_type: Option<Mime>,
    ) -> Self {
        self.body = AsyncRequestBody::from_referenced_reader(body, content_length);
        self.set_content_type(content_type)
    }

    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>, content_type: Option<Mime>) -> Self {
        self.body = AsyncRequestBody::from_bytes(body.into());
        self.set_content_type(content_type)
    }

    #[inline]
    pub fn referenced_bytes_as_body(mut self, body: &'r [u8], content_type: Option<Mime>) -> Self {
        self.body = AsyncRequestBody::from_referenced_bytes(body);
        self.set_content_type(content_type)
    }

    #[inline]
    pub fn json(self, body: impl Serialize) -> JsonResult<Self> {
        Ok(self.bytes_as_body(serde_json::to_vec(&body)?, Some(APPLICATION_JSON)))
    }

    #[inline]
    pub fn post_form<I, K, V>(self, iter: I) -> Self
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
        self.bytes_as_body(
            form.finish().into_bytes(),
            Some(APPLICATION_WWW_FORM_URLENCODED),
        )
    }

    #[inline]
    pub async fn multipart(
        self,
        multipart: AsyncMultipart,
    ) -> IoResult<RequestBuilder<'r, AsyncRequestBody<'r>>> {
        use futures::AsyncReadExt;

        let mut buf = Vec::new();
        multipart.into_async_read().read_to_end(&mut buf).await?;
        Ok(self.bytes_as_body(buf, Some(MULTIPART_FORM_DATA)))
    }

    #[inline]
    pub async fn call(self) -> ApiResult<AsyncResponse> {
        async_request_call(self.build()).await
    }

    #[inline]
    pub(in super::super) fn build(self) -> AsyncRequest<'r> {
        let appended_user_agent = self.get_appended_user_agent();
        AsyncRequest::new(
            self.http_client,
            self.service_names,
            self.into_endpoints,
            self.callbacks.build(),
            self.metadata,
            self.body,
            appended_user_agent,
            self.extensions,
        )
    }
}
