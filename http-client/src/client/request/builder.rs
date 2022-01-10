use super::{
    super::{
        super::{EndpointsProvider, IpAddrWithPort, ServiceName},
        request_call, ApiResult, Authorization, CallbackContext, CallbacksBuilder,
        ExtendedCallbackContext, HttpClient, ResolveAnswers, ResponseError,
        SimplifiedCallbackContext, SyncResponse,
    },
    multipart::SyncMultipart,
    request_metadata::RequestMetadata,
    Idempotent, QueryPairKey, QueryPairValue, QueryPairs, SyncRequest,
};
use mime::{Mime, APPLICATION_JSON, APPLICATION_OCTET_STREAM, APPLICATION_WWW_FORM_URLENCODED};
use qiniu_http::{
    header::{ACCEPT, CONTENT_TYPE},
    CallbackResult, Extensions, HeaderMap, HeaderName, HeaderValue, Method, Reset, ResponseParts,
    StatusCode, SyncRequestBody, TransferProgressInfo, UserAgent, Version,
};
use serde::Serialize;
use serde_json::Result as JsonResult;
use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    io::{Read, Result as IoResult},
    mem::take,
    time::Duration,
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

#[derive(Default, Debug)]
pub struct RequestBuilderParts<'r> {
    callbacks: CallbacksBuilder<'r>,
    metadata: RequestMetadata<'r>,
    extensions: Extensions,
    appended_user_agent: UserAgent,
}

impl<'r> RequestBuilderParts<'r> {
    #[inline]
    pub fn use_https(&mut self, use_https: bool) -> &mut Self {
        self.metadata.use_https = Some(use_https);
        self
    }

    #[inline]
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.metadata.version = version;
        self
    }

    #[inline]
    pub fn set_header(
        &mut self,
        header_name: impl Into<HeaderName>,
        header_value: impl Into<HeaderValue>,
    ) -> &mut Self {
        self.metadata
            .headers
            .to_mut()
            .insert(header_name.into(), header_value.into());
        self
    }

    fn set_content_type(&mut self, content_type: Option<Mime>) -> &mut Self {
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
    pub fn accept_json(&mut self) -> &mut Self {
        self.set_accept(APPLICATION_JSON)
    }

    #[inline]
    pub fn accept_application_octet_stream(&mut self) -> &mut Self {
        self.set_accept(APPLICATION_OCTET_STREAM)
    }

    fn set_accept(&mut self, accept: Mime) -> &mut Self {
        self.set_header(ACCEPT, HeaderValue::from_str(accept.as_ref()).unwrap())
    }

    #[inline]
    pub fn append_query_pair(
        &mut self,
        query_pair_key: impl Into<QueryPairKey<'r>>,
        query_pair_value: impl Into<QueryPairValue<'r>>,
    ) -> &mut Self {
        self.metadata
            .query_pairs
            .push((query_pair_key.into(), query_pair_value.into()));
        self
    }

    #[inline]
    pub fn appended_user_agent(&mut self, user_agent: impl Into<UserAgent>) -> &mut Self {
        self.appended_user_agent = user_agent.into();
        self
    }

    #[inline]
    pub fn extensions(&mut self, extensions: Extensions) -> &mut Self {
        self.extensions = extensions;
        self
    }

    #[inline]
    pub fn add_extension<T: Send + Sync + 'static>(&mut self, val: T) -> &mut Self {
        self.extensions.insert(val);
        self
    }

    #[inline]
    pub fn on_uploading_progress(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, &TransferProgressInfo) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.callbacks.on_uploading_progress(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_status(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, StatusCode) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.callbacks.on_receive_response_status(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_header(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, &HeaderName, &HeaderValue) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.callbacks.on_receive_response_header(callback);
        self
    }

    #[inline]
    pub fn on_to_resolve_domain(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str) -> CallbackResult + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_to_resolve_domain(callback);
        self
    }

    #[inline]
    pub fn on_domain_resolved(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str, &ResolveAnswers) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.callbacks.on_domain_resolved(callback);
        self
    }

    #[inline]
    pub fn on_to_choose_ips(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort]) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.callbacks.on_to_choose_ips(callback);
        self
    }

    #[inline]
    pub fn on_ips_chosen(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort], &[IpAddrWithPort]) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.callbacks.on_ips_chosen(callback);
        self
    }

    #[inline]
    pub fn on_before_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> CallbackResult + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_before_request_signed(callback);
        self
    }

    #[inline]
    pub fn on_after_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> CallbackResult + Send + Sync + 'r,
    ) -> &mut Self {
        self.callbacks.on_after_request_signed(callback);
        self
    }

    #[inline]
    pub fn on_response(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseParts) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.callbacks.on_response(callback);
        self
    }

    #[inline]
    pub fn on_error(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseError) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.callbacks.on_error(callback);
        self
    }

    #[inline]
    pub fn on_before_backoff(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.callbacks.on_before_backoff(callback);
        self
    }

    #[inline]
    pub fn on_after_backoff(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.callbacks.on_after_backoff(callback);
        self
    }
}

#[derive(Debug)]
pub struct RequestBuilder<'r, B: 'r, E: 'r> {
    http_client: &'r HttpClient,
    service_names: &'r [ServiceName],
    endpoints_provider: E,
    parts: RequestBuilderParts<'r>,
    body: B,
}

impl<'r, B: Default + 'r, E: EndpointsProvider + 'r> RequestBuilder<'r, B, E> {
    pub(in super::super) fn new(
        http_client: &'r HttpClient,
        method: Method,
        endpoints_provider: E,
        service_names: &'r [ServiceName],
    ) -> Self {
        Self {
            http_client,
            service_names,
            endpoints_provider,
            parts: RequestBuilderParts {
                metadata: RequestMetadata {
                    method,
                    ..Default::default()
                },
                ..Default::default()
            },
            body: Default::default(),
        }
    }
}

impl<'r, B: 'r, E: 'r> RequestBuilder<'r, B, E> {
    #[inline]
    pub fn use_https(&mut self, use_https: bool) -> &mut Self {
        self.parts.use_https(use_https);
        self
    }

    #[inline]
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.parts.version(version);
        self
    }

    #[inline]
    pub fn path(&mut self, path: impl Into<Cow<'r, str>>) -> &mut Self {
        self.parts.metadata.path = path.into();
        self
    }

    #[inline]
    pub fn headers(&mut self, headers: impl Into<Cow<'r, HeaderMap>>) -> &mut Self {
        self.parts.metadata.headers = headers.into();
        self
    }

    #[inline]
    pub fn set_header(
        &mut self,
        header_name: impl Into<HeaderName>,
        header_value: impl Into<HeaderValue>,
    ) -> &mut Self {
        self.parts.set_header(header_name, header_value);
        self
    }

    #[inline]
    pub fn accept_json(&mut self) -> &mut Self {
        self.parts.accept_json();
        self
    }

    #[inline]
    pub fn accept_application_octet_stream(&mut self) -> &mut Self {
        self.parts.accept_application_octet_stream();
        self
    }

    #[inline]
    pub fn query(&mut self, query: impl Into<Cow<'r, str>>) -> &mut Self {
        self.parts.metadata.query = query.into();
        self
    }

    #[inline]
    pub fn query_pairs(&mut self, query_pairs: impl Into<QueryPairs<'r>>) -> &mut Self {
        self.parts.metadata.query_pairs = query_pairs.into();
        self
    }

    #[inline]
    pub fn append_query_pair(
        &mut self,
        query_pair_key: impl Into<QueryPairKey<'r>>,
        query_pair_value: impl Into<QueryPairValue<'r>>,
    ) -> &mut Self {
        self.parts
            .append_query_pair(query_pair_key, query_pair_value);
        self
    }

    #[inline]
    pub fn appended_user_agent(&mut self, user_agent: impl Into<UserAgent>) -> &mut Self {
        self.parts.appended_user_agent(user_agent);
        self
    }

    #[inline]
    pub fn authorization(&mut self, authorization: Authorization<'r>) -> &mut Self {
        self.parts.metadata.authorization = Some(authorization);
        self
    }

    #[inline]
    pub fn idempotent(&mut self, idempotent: Idempotent) -> &mut Self {
        self.parts.metadata.idempotent = idempotent;
        self
    }

    #[inline]
    pub fn extensions(&mut self, extensions: Extensions) -> &mut Self {
        self.parts.extensions(extensions);
        self
    }

    #[inline]
    pub fn add_extension<T: Send + Sync + 'static>(&mut self, val: T) -> &mut Self {
        self.parts.add_extension(val);
        self
    }

    #[inline]
    pub fn on_uploading_progress(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, &TransferProgressInfo) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.parts.on_uploading_progress(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_status(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, StatusCode) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.parts.on_receive_response_status(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_header(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, &HeaderName, &HeaderValue) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.parts.on_receive_response_header(callback);
        self
    }

    #[inline]
    pub fn on_to_resolve_domain(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str) -> CallbackResult + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_to_resolve_domain(callback);
        self
    }

    #[inline]
    pub fn on_domain_resolved(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str, &ResolveAnswers) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.parts.on_domain_resolved(callback);
        self
    }

    #[inline]
    pub fn on_to_choose_ips(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort]) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.parts.on_to_choose_ips(callback);
        self
    }

    #[inline]
    pub fn on_ips_chosen(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort], &[IpAddrWithPort]) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.parts.on_ips_chosen(callback);
        self
    }

    #[inline]
    pub fn on_before_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> CallbackResult + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_before_request_signed(callback);
        self
    }

    #[inline]
    pub fn on_after_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> CallbackResult + Send + Sync + 'r,
    ) -> &mut Self {
        self.parts.on_after_request_signed(callback);
        self
    }

    #[inline]
    pub fn on_response(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseParts) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.parts.on_response(callback);
        self
    }

    #[inline]
    pub fn on_error(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseError) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.parts.on_error(callback);
        self
    }

    #[inline]
    pub fn on_before_backoff(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.parts.on_before_backoff(callback);
        self
    }

    #[inline]
    pub fn on_after_backoff(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> CallbackResult
            + Send
            + Sync
            + 'r,
    ) -> &mut Self {
        self.parts.on_after_backoff(callback);
        self
    }

    #[inline]
    pub fn parts(&self) -> &RequestBuilderParts<'r> {
        &self.parts
    }

    #[inline]
    pub fn parts_mut(&mut self) -> &mut RequestBuilderParts<'r> {
        &mut self.parts
    }

    fn get_appended_user_agent(&self) -> UserAgent {
        let mut appended_user_agent = self.http_client.appended_user_agent().to_owned();
        appended_user_agent.push_str(self.parts.appended_user_agent.as_str());
        appended_user_agent
    }
}

pub type SyncRequestBuilder<'r, E> = RequestBuilder<'r, SyncRequestBody<'r>, E>;

impl<'r, E: 'r> SyncRequestBuilder<'r, E> {
    #[inline]
    pub fn stream_as_body(
        &mut self,
        body: impl Read + Reset + Debug + Send + Sync + 'static,
        content_length: u64,
        content_type: Option<Mime>,
    ) -> &mut Self {
        self.body = SyncRequestBody::from_reader(body, content_length);
        self.parts.set_content_type(content_type);
        self
    }

    #[inline]
    pub fn referenced_stream_as_body<T: Read + Reset + Debug + Send + Sync>(
        &mut self,
        body: &'r mut T,
        content_length: u64,
        content_type: Option<Mime>,
    ) -> &mut Self {
        self.body = SyncRequestBody::from_referenced_reader(body, content_length);
        self.parts.set_content_type(content_type);
        self
    }

    #[inline]
    pub fn bytes_as_body(
        &mut self,
        body: impl Into<Vec<u8>>,
        content_type: Option<Mime>,
    ) -> &mut Self {
        self.body = SyncRequestBody::from_bytes(body.into());
        self.parts.set_content_type(content_type);
        self
    }

    #[inline]
    pub fn referenced_bytes_as_body(
        &mut self,
        body: &'r [u8],
        content_type: Option<Mime>,
    ) -> &mut Self {
        self.body = SyncRequestBody::from_referenced_bytes(body);
        self.parts.set_content_type(content_type);
        self
    }

    #[inline]
    pub fn json(&mut self, body: impl Serialize) -> JsonResult<&mut Self> {
        Ok(self.bytes_as_body(serde_json::to_vec(&body)?, Some(APPLICATION_JSON)))
    }

    #[inline]
    pub fn post_form<I, K, V>(&mut self, iter: I) -> &mut Self
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
    pub fn multipart(&mut self, multipart: impl Into<SyncMultipart>) -> IoResult<&mut Self> {
        let mut buf = Vec::new();
        let multipart = multipart.into();
        let mime = ("multipart/form-data; boundary=".to_owned() + multipart.boundary())
            .parse()
            .unwrap();
        multipart.into_read().read_to_end(&mut buf)?;
        Ok(self.bytes_as_body(buf, Some(mime)))
    }
}

impl<'r, E: EndpointsProvider + Clone + 'r> SyncRequestBuilder<'r, E> {
    #[inline]
    pub fn call(&mut self) -> ApiResult<SyncResponse> {
        request_call(self.build())
    }

    pub(in super::super) fn build(&mut self) -> SyncRequest<'r, E> {
        SyncRequest::new(
            self.http_client,
            self.endpoints_provider.to_owned(),
            self.service_names,
            self.parts.callbacks.build(),
            take(&mut self.parts.metadata),
            take(&mut self.body),
            self.get_appended_user_agent(),
            take(&mut self.parts.extensions),
        )
    }
}

#[cfg(feature = "async")]
pub type AsyncRequestBuilder<'r, E> = RequestBuilder<'r, AsyncRequestBody<'r>, E>;

#[cfg(feature = "async")]
impl<'r, E: 'r> AsyncRequestBuilder<'r, E> {
    #[inline]
    pub fn stream_as_body(
        &mut self,
        body: impl AsyncRead + AsyncReset + Unpin + Debug + Send + Sync + 'static,
        content_length: u64,
        content_type: Option<Mime>,
    ) -> &mut Self {
        self.body = AsyncRequestBody::from_reader(body, content_length);
        self.parts.set_content_type(content_type);
        self
    }

    #[inline]
    pub fn referenced_stream_as_body<T: AsyncRead + AsyncReset + Unpin + Debug + Send + Sync>(
        &mut self,
        body: &'r mut T,
        content_length: u64,
        content_type: Option<Mime>,
    ) -> &mut Self {
        self.body = AsyncRequestBody::from_referenced_reader(body, content_length);
        self.parts.set_content_type(content_type);
        self
    }

    #[inline]
    pub fn bytes_as_body(
        &mut self,
        body: impl Into<Vec<u8>>,
        content_type: Option<Mime>,
    ) -> &mut Self {
        self.body = AsyncRequestBody::from_bytes(body.into());
        self.parts.set_content_type(content_type);
        self
    }

    #[inline]
    pub fn referenced_bytes_as_body(
        &mut self,
        body: &'r [u8],
        content_type: Option<Mime>,
    ) -> &mut Self {
        self.body = AsyncRequestBody::from_referenced_bytes(body);
        self.parts.set_content_type(content_type);
        self
    }

    #[inline]
    pub fn json(&mut self, body: impl Serialize) -> JsonResult<&mut Self> {
        Ok(self.bytes_as_body(serde_json::to_vec(&body)?, Some(APPLICATION_JSON)))
    }

    #[inline]
    pub fn post_form<I, K, V>(&mut self, iter: I) -> &mut Self
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
        &mut self,
        multipart: impl Into<AsyncMultipart>,
    ) -> IoResult<&mut RequestBuilder<'r, AsyncRequestBody<'r>, E>> {
        use futures::AsyncReadExt;

        let mut buf = Vec::new();
        let multipart = multipart.into();
        let mime = ("multipart/form-data; boundary=".to_owned() + multipart.boundary())
            .parse()
            .unwrap();
        multipart.into_async_read().read_to_end(&mut buf).await?;
        Ok(self.bytes_as_body(buf, Some(mime)))
    }
}

#[cfg(feature = "async")]
impl<'r, E: EndpointsProvider + Clone + 'r> AsyncRequestBuilder<'r, E> {
    #[inline]
    pub async fn call(&mut self) -> ApiResult<AsyncResponse> {
        async_request_call(self.build()).await
    }

    pub(in super::super) fn build(&mut self) -> AsyncRequest<'r, E> {
        AsyncRequest::new(
            self.http_client,
            self.endpoints_provider.to_owned(),
            self.service_names,
            self.parts.callbacks.build(),
            take(&mut self.parts.metadata),
            take(&mut self.body),
            self.get_appended_user_agent(),
            take(&mut self.parts.extensions),
        )
    }
}
