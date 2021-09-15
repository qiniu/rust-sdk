use super::{
    super::{
        super::{IntoEndpoints, IpAddrWithPort, ServiceName},
        Authorization, CallbackContext, Callbacks, ExtendedCallbackContext, HTTPClient,
        ResolveAnswers, ResponseError, ResponseInfo, SimplifiedCallbackContext,
    },
    request_data::RequestData,
    Idempotent, QueryPairs,
};
use qiniu_http::{
    Extensions, HeaderMap, HeaderName, HeaderValue, Method, StatusCode, TransferProgressInfo,
    Version,
};
use std::{fmt, time::Duration};

pub(in super::super) struct Request<'r> {
    http_client: &'r HTTPClient,
    service_name: ServiceName,
    into_endpoints: IntoEndpoints<'r>,
    callbacks: Callbacks,
    data: RequestData<'r>,
    appended_user_agent: Box<str>,
    extensions: Extensions,
}

impl<'r> Request<'r> {
    #[inline]
    pub(super) fn new(
        http_client: &'r HTTPClient,
        service_name: ServiceName,
        into_endpoints: IntoEndpoints<'r>,
        callbacks: Callbacks,
        data: RequestData<'r>,
        appended_user_agent: Box<str>,
        extensions: Extensions,
    ) -> Self {
        Self {
            http_client,
            service_name,
            into_endpoints,
            callbacks,
            data,
            appended_user_agent,
            extensions,
        }
    }

    #[inline]
    pub(in super::super) fn split(
        self,
    ) -> (
        RequestWithoutEndpoints<'r>,
        IntoEndpoints<'r>,
        ServiceName,
        Extensions,
    ) {
        (
            RequestWithoutEndpoints {
                http_client: self.http_client,
                callbacks: self.callbacks,
                data: self.data,
                appended_user_agent: self.appended_user_agent,
            },
            self.into_endpoints,
            self.service_name,
            self.extensions,
        )
    }
}

#[derive(Debug)]
pub(in super::super) struct RequestWithoutEndpoints<'r> {
    http_client: &'r HTTPClient,
    callbacks: Callbacks,
    data: RequestData<'r>,
    appended_user_agent: Box<str>,
}

impl<'r> SimplifiedCallbackContext for RequestWithoutEndpoints<'r> {
    #[inline]
    fn use_https(&self) -> bool {
        self.data
            .use_https
            .unwrap_or_else(|| self.http_client.use_https())
    }

    #[inline]
    fn method(&self) -> &Method {
        &self.data.method
    }

    #[inline]
    fn version(&self) -> Version {
        self.data.version
    }

    #[inline]
    fn path(&self) -> &str {
        &self.data.path
    }

    #[inline]
    fn query(&self) -> &str {
        &self.data.query
    }

    #[inline]
    fn query_pairs(&self) -> &QueryPairs {
        &self.data.query_pairs
    }

    #[inline]
    fn headers(&self) -> &HeaderMap {
        &self.data.headers
    }

    #[inline]
    fn body(&self) -> &[u8] {
        &self.data.body
    }

    #[inline]
    fn appended_user_agent(&self) -> &str {
        &self.appended_user_agent
    }

    #[inline]
    fn authorization(&self) -> Option<&Authorization> {
        self.data.authorization.as_ref()
    }

    #[inline]
    fn idempotent(&self) -> Idempotent {
        self.data.idempotent
    }
}

impl<'r> RequestWithoutEndpoints<'r> {
    #[inline]
    pub(in super::super) fn http_client(&self) -> &HTTPClient {
        self.http_client
    }

    #[inline]
    pub(in super::super) fn call_uploading_progress_callbacks(
        &self,
        context: &dyn SimplifiedCallbackContext,
        progress_info: &TransferProgressInfo,
    ) -> bool {
        self.callbacks
            .call_uploading_progress_callbacks(context, progress_info)
            && self
                .http_client
                .callbacks()
                .call_uploading_progress_callbacks(context, progress_info)
    }

    #[inline]
    pub(in super::super) fn uploading_progress_callbacks_count(&self) -> usize {
        self.callbacks.on_uploading_progress_callbacks().len()
            + self
                .http_client
                .callbacks()
                .on_uploading_progress_callbacks()
                .len()
    }

    #[inline]
    pub(in super::super) fn call_receive_response_status_callbacks(
        &self,
        context: &dyn SimplifiedCallbackContext,
        status_code: StatusCode,
    ) -> bool {
        self.callbacks
            .call_receive_response_status_callbacks(context, status_code)
            && self
                .http_client
                .callbacks()
                .call_receive_response_status_callbacks(context, status_code)
    }

    #[inline]
    pub(in super::super) fn receive_response_status_callbacks_count(&self) -> usize {
        self.callbacks.on_receive_response_status_callbacks().len()
            + self
                .http_client
                .callbacks()
                .on_receive_response_status_callbacks()
                .len()
    }

    #[inline]
    pub(in super::super) fn call_receive_response_header_callbacks(
        &self,
        context: &dyn SimplifiedCallbackContext,
        header_name: &HeaderName,
        header_value: &HeaderValue,
    ) -> bool {
        self.callbacks
            .call_receive_response_header_callbacks(context, header_name, header_value)
            && self
                .http_client
                .callbacks()
                .call_receive_response_header_callbacks(context, header_name, header_value)
    }

    #[inline]
    pub(in super::super) fn receive_response_header_callbacks_count(&self) -> usize {
        self.callbacks.on_receive_response_header_callbacks().len()
            + self
                .http_client
                .callbacks()
                .on_receive_response_header_callbacks()
                .len()
    }

    #[inline]
    pub(in super::super) fn call_to_resolve_domain_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        domain: &str,
    ) -> bool {
        self.callbacks
            .call_to_resolve_domain_callbacks(context, domain)
            && self
                .http_client
                .callbacks()
                .call_to_resolve_domain_callbacks(context, domain)
    }

    #[inline]
    pub(in super::super) fn call_domain_resolved_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        domain: &str,
        answers: &ResolveAnswers,
    ) -> bool {
        self.callbacks
            .call_domain_resolved_callbacks(context, domain, answers)
            && self
                .http_client
                .callbacks()
                .call_domain_resolved_callbacks(context, domain, answers)
    }

    #[inline]
    pub(in super::super) fn call_to_choose_ips_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        ips: &[IpAddrWithPort],
    ) -> bool {
        self.callbacks.call_to_choose_ips_callbacks(context, ips)
            && self
                .http_client
                .callbacks()
                .call_to_choose_ips_callbacks(context, ips)
    }

    #[inline]
    pub(in super::super) fn call_ips_chosen_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        ips: &[IpAddrWithPort],
        chosen: &[IpAddrWithPort],
    ) -> bool {
        self.callbacks
            .call_ips_chosen_callbacks(context, ips, chosen)
            && self
                .http_client
                .callbacks()
                .call_ips_chosen_callbacks(context, ips, chosen)
    }

    #[inline]
    pub(in super::super) fn call_before_request_signed_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
    ) -> bool {
        self.callbacks.call_before_request_signed_callbacks(context)
            && self
                .http_client
                .callbacks()
                .call_before_request_signed_callbacks(context)
    }

    #[inline]
    pub(in super::super) fn call_after_request_signed_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
    ) -> bool {
        self.callbacks.call_after_request_signed_callbacks(context)
            && self
                .http_client
                .callbacks()
                .call_after_request_signed_callbacks(context)
    }

    #[inline]
    pub(in super::super) fn call_success_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        response: &ResponseInfo,
    ) -> bool {
        self.callbacks.call_success_callbacks(context, response)
            && self
                .http_client
                .callbacks()
                .call_success_callbacks(context, response)
    }

    #[inline]
    pub(in super::super) fn call_error_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        error: &ResponseError,
    ) -> bool {
        self.callbacks.call_error_callbacks(context, error)
            && self
                .http_client
                .callbacks()
                .call_error_callbacks(context, error)
    }

    #[inline]
    pub(in super::super) fn call_before_retry_delay_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        delay: Duration,
    ) -> bool {
        self.callbacks
            .call_before_retry_delay_callbacks(context, delay)
            && self
                .http_client
                .callbacks()
                .call_before_retry_delay_callbacks(context, delay)
    }

    #[inline]
    pub(in super::super) fn call_after_retry_delay_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        delay: Duration,
    ) -> bool {
        self.callbacks
            .call_after_retry_delay_callbacks(context, delay)
            && self
                .http_client
                .callbacks()
                .call_after_retry_delay_callbacks(context, delay)
    }
}

impl fmt::Debug for Request<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("http_client", &self.http_client)
            .field("service_name", &self.service_name)
            .field("into_endpoints", &self.into_endpoints)
            .field("callbacks", &self.callbacks)
            .field("data", &self.data)
            .field("appended_user_agent", &self.appended_user_agent)
            .finish()
    }
}
