use super::{
    super::{super::regions::IntoDomains, Authorization, Callbacks, Client, ResponseError},
    request_data::RequestData,
    Idempotent, Queries,
};
use qiniu_http::{HeaderName, HeaderValue, Headers, Method, Request as HTTPRequest, StatusCode};
use std::{fmt, time::Duration};

pub struct Request<'r> {
    client: &'r Client,
    into_domains: IntoDomains<'r>,
    callbacks: Callbacks,
    data: RequestData<'r>,
    appended_user_agent: Box<str>,
}

impl<'r> Request<'r> {
    #[inline]
    pub(super) fn new(
        client: &'r Client,
        into_domains: IntoDomains<'r>,
        callbacks: Callbacks,
        data: RequestData<'r>,
        appended_user_agent: Box<str>,
    ) -> Self {
        Self {
            client,
            into_domains,
            callbacks,
            data,
            appended_user_agent,
        }
    }

    #[inline]
    pub fn use_https(&self) -> bool {
        self.data
            .use_https
            .unwrap_or_else(|| self.client.use_https())
    }

    #[inline]
    pub fn method(&self) -> Method {
        self.data.method
    }

    #[inline]
    pub fn queries(&self) -> &Queries {
        &self.data.queries
    }

    #[inline]
    pub fn headers(&self) -> &Headers {
        &self.data.headers
    }

    #[inline]
    pub fn body(&self) -> &[u8] {
        &self.data.body
    }

    #[inline]
    pub fn appended_user_agent(&self) -> &str {
        &self.appended_user_agent
    }

    #[inline]
    pub fn authorization(&self) -> Option<&Authorization> {
        self.data.authorization.as_ref()
    }

    #[inline]
    pub fn idempotent(&self) -> Idempotent {
        self.data.idempotent
    }

    #[inline]
    pub fn read_body(&self) -> bool {
        self.data.read_body
    }

    #[inline]
    pub fn follow_redirection(&self) -> bool {
        self.data.follow_redirection
    }

    #[inline]
    pub fn connect_timeout(&self) -> Option<Duration> {
        self.data
            .connect_timeout
            .or_else(|| self.client.connect_timeout())
    }

    #[inline]
    pub fn request_timeout(&self) -> Option<Duration> {
        self.data
            .request_timeout
            .or_else(|| self.client.request_timeout())
    }

    #[inline]
    pub fn tcp_keepalive_idle_timeout(&self) -> Option<Duration> {
        self.data.tcp_keepalive_idle_timeout
    }

    #[inline]
    pub fn tcp_keepalive_probe_interval(&self) -> Option<Duration> {
        self.data.tcp_keepalive_probe_interval
    }

    #[inline]
    pub fn low_transfer_speed(&self) -> Option<u32> {
        self.data.low_transfer_speed
    }

    #[inline]
    pub fn low_transfer_speed_timeout(&self) -> Option<Duration> {
        self.data.low_transfer_speed_timeout
    }

    #[inline]
    pub fn call_uploading_progress_callbacks(
        &self,
        request: &HTTPRequest,
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
        request: &HTTPRequest,
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
    pub(in super::super) fn call_request_callbacks(&self, request: &HTTPRequest) -> bool {
        self.callbacks.call_request_callbacks(request)
            && self.client.callbacks().call_request_callbacks(request)
    }

    #[inline]
    pub(in super::super) fn call_send_request_body_callbacks(
        &self,
        request: &HTTPRequest,
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
        request: &HTTPRequest,
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
        request: &HTTPRequest,
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
        request: &HTTPRequest,
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
    pub(in super::super) fn call_success_callbacks(
        &self,
        request: &HTTPRequest,
        status_code: StatusCode,
        headers: &Headers,
    ) -> bool {
        self.callbacks
            .call_success_callbacks(request, status_code, headers)
            && self
                .client
                .callbacks()
                .call_success_callbacks(request, status_code, headers)
    }

    #[inline]
    pub(in super::super) fn call_error_callbacks(
        &self,
        request: &HTTPRequest,
        error: &ResponseError,
    ) -> bool {
        self.callbacks.call_error_callbacks(request, error)
            && self.client.callbacks().call_error_callbacks(request, error)
    }

    #[inline]
    pub(in super::super) fn call_retry_callbacks(
        &self,
        request: &HTTPRequest,
        retried: usize,
    ) -> bool {
        self.callbacks.call_retry_callbacks(request, retried)
            && self
                .client
                .callbacks()
                .call_retry_callbacks(request, retried)
    }
}

impl fmt::Debug for Request<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("into_domains", &self.into_domains)
            .field("callbacks", &self.callbacks)
            .field("data", &self.data)
            .field("appended_user_agent", &self.appended_user_agent)
            .finish()
    }
}
