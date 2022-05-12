use super::{
    super::{
        super::{EndpointsProvider, IpAddrWithPort, ServiceName},
        callbacks::Callbacks,
        Authorization, CallbackContext, ExtendedCallbackContext, HttpClient, ResolveAnswers, ResponseError,
        SimplifiedCallbackContext,
    },
    request_metadata::RequestMetadata,
    Idempotent, QueryPair,
};
use anyhow::Result as AnyResult;
use qiniu_http::{
    Extensions, HeaderMap, HeaderName, HeaderValue, Method, ResponseParts, StatusCode, TransferProgressInfo, UserAgent,
    Version,
};
use std::{fmt, time::Duration};

pub(in super::super) struct InnerRequest<'r, B: 'r, E: 'r> {
    http_client: &'r HttpClient,
    endpoints_provider: E,
    service_names: &'r [ServiceName],
    callbacks: Callbacks<'r>,
    metadata: RequestMetadata<'r>,
    body: B,
    appended_user_agent: UserAgent,
    extensions: Extensions,
}

impl<'r, B: 'r, E: 'r> InnerRequest<'r, B, E> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        http_client: &'r HttpClient,
        endpoints_provider: E,
        service_names: &'r [ServiceName],
        callbacks: Callbacks<'r>,
        data: RequestMetadata<'r>,
        body: B,
        appended_user_agent: UserAgent,
        extensions: Extensions,
    ) -> Self {
        Self {
            http_client,
            endpoints_provider,
            service_names,
            callbacks,
            metadata: data,
            body,
            appended_user_agent,
            extensions,
        }
    }

    pub(in super::super) fn split(self) -> (InnerRequestParts<'r>, B, E, &'r [ServiceName], Extensions) {
        (
            InnerRequestParts {
                http_client: self.http_client,
                callbacks: self.callbacks,
                data: self.metadata,
                appended_user_agent: self.appended_user_agent,
            },
            self.body,
            self.endpoints_provider,
            self.service_names,
            self.extensions,
        )
    }
}

#[derive(Debug)]
pub(in super::super) struct InnerRequestParts<'r> {
    http_client: &'r HttpClient,
    callbacks: Callbacks<'r>,
    data: RequestMetadata<'r>,
    appended_user_agent: UserAgent,
}

impl<'r> SimplifiedCallbackContext for InnerRequestParts<'r> {
    #[inline]
    fn use_https(&self) -> bool {
        self.data.use_https.unwrap_or_else(|| self.http_client.use_https())
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
    fn query_pairs(&self) -> &[QueryPair] {
        &self.data.query_pairs
    }

    #[inline]
    fn headers(&self) -> &HeaderMap {
        &self.data.headers
    }

    #[inline]
    fn appended_user_agent(&self) -> &UserAgent {
        &self.appended_user_agent
    }

    #[inline]
    fn authorization(&self) -> Option<&Authorization<'r>> {
        self.data.authorization.as_ref()
    }

    #[inline]
    fn idempotent(&self) -> Idempotent {
        self.data.idempotent
    }
}

impl InnerRequestParts<'_> {
    pub(in super::super) fn http_client(&self) -> &HttpClient {
        self.http_client
    }

    pub(in super::super) fn call_uploading_progress_callbacks(
        &self,
        context: &dyn SimplifiedCallbackContext,
        progress_info: TransferProgressInfo<'_>,
    ) -> AnyResult<()> {
        self.callbacks
            .call_uploading_progress_callbacks(context, progress_info)?;
        self.http_client
            .callbacks()
            .call_uploading_progress_callbacks(context, progress_info)?;
        Ok(())
    }

    pub(in super::super) fn uploading_progress_callbacks_count(&self) -> usize {
        self.callbacks.on_uploading_progress_callbacks().len()
            + self.http_client.callbacks().on_uploading_progress_callbacks().len()
    }

    pub(in super::super) fn call_receive_response_status_callbacks(
        &self,
        context: &dyn SimplifiedCallbackContext,
        status_code: StatusCode,
    ) -> AnyResult<()> {
        self.callbacks
            .call_receive_response_status_callbacks(context, status_code)?;
        self.http_client
            .callbacks()
            .call_receive_response_status_callbacks(context, status_code)?;
        Ok(())
    }

    pub(in super::super) fn receive_response_status_callbacks_count(&self) -> usize {
        self.callbacks.on_receive_response_status_callbacks().len()
            + self
                .http_client
                .callbacks()
                .on_receive_response_status_callbacks()
                .len()
    }

    pub(in super::super) fn call_receive_response_header_callbacks(
        &self,
        context: &dyn SimplifiedCallbackContext,
        header_name: &HeaderName,
        header_value: &HeaderValue,
    ) -> AnyResult<()> {
        self.callbacks
            .call_receive_response_header_callbacks(context, header_name, header_value)?;
        self.http_client
            .callbacks()
            .call_receive_response_header_callbacks(context, header_name, header_value)?;
        Ok(())
    }

    pub(in super::super) fn receive_response_header_callbacks_count(&self) -> usize {
        self.callbacks.on_receive_response_header_callbacks().len()
            + self
                .http_client
                .callbacks()
                .on_receive_response_header_callbacks()
                .len()
    }

    pub(in super::super) fn call_to_resolve_domain_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        domain: &str,
    ) -> AnyResult<()> {
        self.callbacks.call_to_resolve_domain_callbacks(context, domain)?;
        self.http_client
            .callbacks()
            .call_to_resolve_domain_callbacks(context, domain)?;
        Ok(())
    }

    pub(in super::super) fn call_domain_resolved_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        domain: &str,
        answers: &ResolveAnswers,
    ) -> AnyResult<()> {
        self.callbacks
            .call_domain_resolved_callbacks(context, domain, answers)?;
        self.http_client
            .callbacks()
            .call_domain_resolved_callbacks(context, domain, answers)?;
        Ok(())
    }

    pub(in super::super) fn call_to_choose_ips_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        ips: &[IpAddrWithPort],
    ) -> AnyResult<()> {
        self.callbacks.call_to_choose_ips_callbacks(context, ips)?;
        self.http_client
            .callbacks()
            .call_to_choose_ips_callbacks(context, ips)?;
        Ok(())
    }

    pub(in super::super) fn call_ips_chosen_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        ips: &[IpAddrWithPort],
        chosen: &[IpAddrWithPort],
    ) -> AnyResult<()> {
        self.callbacks.call_ips_chosen_callbacks(context, ips, chosen)?;
        self.http_client
            .callbacks()
            .call_ips_chosen_callbacks(context, ips, chosen)?;
        Ok(())
    }

    pub(in super::super) fn call_before_request_signed_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
    ) -> AnyResult<()> {
        self.callbacks.call_before_request_signed_callbacks(context)?;
        self.http_client
            .callbacks()
            .call_before_request_signed_callbacks(context)?;
        Ok(())
    }

    pub(in super::super) fn call_after_request_signed_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
    ) -> AnyResult<()> {
        self.callbacks.call_after_request_signed_callbacks(context)?;
        self.http_client
            .callbacks()
            .call_after_request_signed_callbacks(context)?;
        Ok(())
    }

    pub(in super::super) fn call_response_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        response: &ResponseParts,
    ) -> AnyResult<()> {
        self.callbacks.call_response_callbacks(context, response)?;
        self.http_client
            .callbacks()
            .call_response_callbacks(context, response)?;
        Ok(())
    }

    pub(in super::super) fn call_error_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        error: &ResponseError,
    ) -> AnyResult<()> {
        self.callbacks.call_error_callbacks(context, error)?;
        self.http_client.callbacks().call_error_callbacks(context, error)?;
        Ok(())
    }

    pub(in super::super) fn call_before_backoff_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        delay: Duration,
    ) -> AnyResult<()> {
        self.callbacks.call_before_backoff_callbacks(context, delay)?;
        self.http_client
            .callbacks()
            .call_before_backoff_callbacks(context, delay)?;
        Ok(())
    }

    pub(in super::super) fn call_after_backoff_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        delay: Duration,
    ) -> AnyResult<()> {
        self.callbacks.call_after_backoff_callbacks(context, delay)?;
        self.http_client
            .callbacks()
            .call_after_backoff_callbacks(context, delay)?;
        Ok(())
    }
}

impl<'r, B: 'r, E: EndpointsProvider + 'r> fmt::Debug for InnerRequest<'r, B, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("http_client", &self.http_client)
            .field("service_names", &self.service_names)
            .field("endpoints_provider", &self.endpoints_provider)
            .field("callbacks", &self.callbacks)
            .field("data", &self.metadata)
            .field("appended_user_agent", &self.appended_user_agent)
            .finish()
    }
}
