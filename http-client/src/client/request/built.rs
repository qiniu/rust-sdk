use super::{
    super::{
        super::{IntoEndpoints, ServiceName},
        Authorization, CallbackContext, Callbacks, ChosenResult, Client, RequestInfo,
        ResponseError, ResponseInfo,
    },
    request_data::RequestData,
    request_id::RequestId,
    Idempotent, QueryPairs,
};
use qiniu_http::{HeaderName, HeaderValue, Headers, Method, StatusCode};
use std::{fmt, time::Duration};

pub(in super::super) struct Request<'r> {
    client: &'r Client,
    service_name: ServiceName,
    into_endpoints: IntoEndpoints<'r>,
    callbacks: Callbacks,
    data: RequestData<'r>,
    appended_user_agent: Box<str>,
}

impl<'r> Request<'r> {
    #[inline]
    pub(super) fn new(
        client: &'r Client,
        service_name: ServiceName,
        into_endpoints: IntoEndpoints<'r>,
        callbacks: Callbacks,
        data: RequestData<'r>,
        appended_user_agent: Box<str>,
    ) -> Self {
        Self {
            client,
            service_name,
            into_endpoints,
            callbacks,
            data,
            appended_user_agent,
        }
    }

    #[inline]
    pub(in super::super) fn split(
        self,
    ) -> (RequestWithoutEndpoints<'r>, IntoEndpoints<'r>, ServiceName) {
        (
            RequestWithoutEndpoints {
                client: self.client,
                callbacks: self.callbacks,
                data: self.data,
                appended_user_agent: self.appended_user_agent,
                request_id: RequestId::new(),
            },
            self.into_endpoints,
            self.service_name,
        )
    }
}

pub(in super::super) struct RequestWithoutEndpoints<'r> {
    client: &'r Client,
    callbacks: Callbacks,
    data: RequestData<'r>,
    appended_user_agent: Box<str>,
    request_id: RequestId,
}

impl<'r> RequestWithoutEndpoints<'r> {
    #[inline]
    pub(in super::super) fn request_id(&self) -> usize {
        self.request_id.get()
    }

    #[inline]
    pub(in super::super) fn client(&self) -> &Client {
        &self.client
    }

    #[inline]
    pub(in super::super) fn use_https(&self) -> bool {
        self.data
            .use_https
            .unwrap_or_else(|| self.client.use_https())
    }

    #[inline]
    pub(in super::super) fn method(&self) -> Method {
        self.data.method
    }

    #[inline]
    pub(in super::super) fn path(&self) -> &str {
        &self.data.path
    }

    #[inline]
    pub(in super::super) fn query(&self) -> &str {
        &self.data.query
    }

    #[inline]
    pub(in super::super) fn query_pairs(&self) -> &QueryPairs {
        &self.data.query_pairs
    }

    #[inline]
    pub(in super::super) fn headers(&self) -> &Headers {
        &self.data.headers
    }

    #[inline]
    pub(in super::super) fn body(&self) -> &[u8] {
        &self.data.body
    }

    #[inline]
    pub(in super::super) fn appended_user_agent(&self) -> &str {
        &self.appended_user_agent
    }

    #[inline]
    pub(in super::super) fn authorization(&self) -> Option<&Authorization> {
        self.data.authorization.as_ref()
    }

    #[inline]
    pub(in super::super) fn idempotent(&self) -> Idempotent {
        self.data.idempotent
    }

    #[inline]
    pub(in super::super) fn read_body(&self) -> bool {
        self.data.read_body
    }

    #[inline]
    pub(in super::super) fn follow_redirection(&self) -> bool {
        self.data.follow_redirection
    }

    #[inline]
    pub(in super::super) fn connect_timeout(&self) -> Option<Duration> {
        self.data
            .connect_timeout
            .or_else(|| self.client.connect_timeout())
    }

    #[inline]
    pub(in super::super) fn request_timeout(&self) -> Option<Duration> {
        self.data
            .request_timeout
            .or_else(|| self.client.request_timeout())
    }

    #[inline]
    pub(in super::super) fn tcp_keepalive_idle_timeout(&self) -> Option<Duration> {
        self.data.tcp_keepalive_idle_timeout
    }

    #[inline]
    pub(in super::super) fn tcp_keepalive_probe_interval(&self) -> Option<Duration> {
        self.data.tcp_keepalive_probe_interval
    }

    #[inline]
    pub(in super::super) fn low_transfer_speed(&self) -> Option<u32> {
        self.data.low_transfer_speed
    }

    #[inline]
    pub(in super::super) fn low_transfer_speed_timeout(&self) -> Option<Duration> {
        self.data.low_transfer_speed_timeout
    }

    #[inline]
    pub(in super::super) fn call_uploading_progress_callbacks(
        &self,
        request: &RequestInfo,
        uploaded: u64,
        total: u64,
    ) -> bool {
        self.callbacks
            .call_uploading_progress_callbacks(request, uploaded, total)
            && self
                .client
                .callbacks()
                .call_uploading_progress_callbacks(request, uploaded, total)
    }

    #[inline]
    pub(in super::super) fn call_downloading_progress_callbacks(
        &self,
        request: &RequestInfo,
        downloaded: u64,
        total: u64,
    ) -> bool {
        self.callbacks
            .call_downloading_progress_callbacks(request, downloaded, total)
            && self
                .client
                .callbacks()
                .call_downloading_progress_callbacks(request, downloaded, total)
    }

    #[inline]
    pub(in super::super) fn call_send_request_body_callbacks(
        &self,
        request: &RequestInfo,
        request_body: &[u8],
    ) -> bool {
        self.callbacks
            .call_send_request_body_callbacks(request, request_body)
            && self
                .client
                .callbacks()
                .call_send_request_body_callbacks(request, request_body)
    }

    #[inline]
    pub(in super::super) fn call_receive_response_status_callbacks(
        &self,
        request: &RequestInfo,
        status_code: StatusCode,
    ) -> bool {
        self.callbacks
            .call_receive_response_status_callbacks(request, status_code)
            && self
                .client
                .callbacks()
                .call_receive_response_status_callbacks(request, status_code)
    }

    #[inline]
    pub(in super::super) fn call_receive_response_body_callbacks(
        &self,
        request: &RequestInfo,
        response_body: &[u8],
    ) -> bool {
        self.callbacks
            .call_receive_response_body_callbacks(request, response_body)
            && self
                .client
                .callbacks()
                .call_receive_response_body_callbacks(request, response_body)
    }

    #[inline]
    pub(in super::super) fn call_receive_response_header_callbacks(
        &self,
        request: &RequestInfo,
        header_name: &HeaderName,
        header_value: &HeaderValue,
    ) -> bool {
        self.callbacks
            .call_receive_response_header_callbacks(request, header_name, header_value)
            && self
                .client
                .callbacks()
                .call_receive_response_header_callbacks(request, header_name, header_value)
    }

    #[inline]
    pub(in super::super) fn call_to_choose_domain_callbacks(&self, domain: &str) -> bool {
        self.callbacks.call_to_choose_domain_callbacks(domain)
            && self
                .client
                .callbacks()
                .call_to_choose_domain_callbacks(domain)
    }

    #[inline]
    pub(in super::super) fn call_domain_chosen_callbacks(
        &self,
        domain: &str,
        result: &ChosenResult,
    ) -> bool {
        self.callbacks.call_domain_chosen_callbacks(domain, result)
            && self
                .client
                .callbacks()
                .call_domain_chosen_callbacks(domain, result)
    }

    #[inline]
    pub(in super::super) fn call_before_request_signed_callbacks(
        &self,
        context: &mut CallbackContext,
    ) -> bool {
        self.callbacks.call_before_request_signed_callbacks(context)
            && self
                .client
                .callbacks()
                .call_before_request_signed_callbacks(context)
    }

    #[inline]
    pub(in super::super) fn call_after_request_signed_callbacks(
        &self,
        context: &mut CallbackContext,
    ) -> bool {
        self.callbacks.call_after_request_signed_callbacks(context)
            && self
                .client
                .callbacks()
                .call_after_request_signed_callbacks(context)
    }

    #[inline]
    pub(in super::super) fn call_success_callbacks(
        &self,
        context: &mut CallbackContext,
        response: &ResponseInfo,
    ) -> bool {
        self.callbacks.call_success_callbacks(context, response)
            && self
                .client
                .callbacks()
                .call_success_callbacks(context, response)
    }

    #[inline]
    pub(in super::super) fn call_error_callbacks(
        &self,
        context: &mut CallbackContext,
        error: &ResponseError,
    ) -> bool {
        self.callbacks.call_error_callbacks(context, error)
            && self.client.callbacks().call_error_callbacks(context, error)
    }

    #[inline]
    pub(in super::super) fn call_before_retry_delay_callbacks(
        &self,
        context: &mut CallbackContext,
        delay: Duration,
    ) -> bool {
        self.callbacks
            .call_before_retry_delay_callbacks(context, delay)
            && self
                .client
                .callbacks()
                .call_before_retry_delay_callbacks(context, delay)
    }

    #[inline]
    pub(in super::super) fn call_after_retry_delay_callbacks(
        &self,
        context: &mut CallbackContext,
        delay: Duration,
    ) -> bool {
        self.callbacks
            .call_after_retry_delay_callbacks(context, delay)
            && self
                .client
                .callbacks()
                .call_after_retry_delay_callbacks(context, delay)
    }
}

impl fmt::Debug for Request<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("client", &self.client)
            .field("service_name", &self.service_name)
            .field("into_endpoints", &self.into_endpoints)
            .field("callbacks", &self.callbacks)
            .field("data", &self.data)
            .field("appended_user_agent", &self.appended_user_agent)
            .finish()
    }
}
