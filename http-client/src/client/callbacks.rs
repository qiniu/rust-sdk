use super::{
    super::regions::IpAddrWithPort,
    callback::{CallbackContext, ExtendedCallbackContext},
    ResolveAnswers, ResponseError, SimplifiedCallbackContext,
};
use anyhow::Result as AnyResult;
use qiniu_http::{HeaderName, HeaderValue, ResponseParts, StatusCode, TransferProgressInfo};
use std::{fmt, mem::take, time::Duration};

type OnProgress<'f> =
    Box<dyn Fn(&dyn SimplifiedCallbackContext, TransferProgressInfo<'_>) -> AnyResult<()> + Send + Sync + 'f>;
type OnStatusCode<'f> = Box<dyn Fn(&dyn SimplifiedCallbackContext, StatusCode) -> AnyResult<()> + Send + Sync + 'f>;
type OnHeader<'f> =
    Box<dyn Fn(&dyn SimplifiedCallbackContext, &HeaderName, &HeaderValue) -> AnyResult<()> + Send + Sync + 'f>;

type OnToResolveDomain<'f> = Box<dyn Fn(&mut dyn CallbackContext, &str) -> AnyResult<()> + Send + Sync + 'f>;
type OnDomainResolved<'f> =
    Box<dyn Fn(&mut dyn CallbackContext, &str, &ResolveAnswers) -> AnyResult<()> + Send + Sync + 'f>;
type OnToChooseIPs<'f> = Box<dyn Fn(&mut dyn CallbackContext, &[IpAddrWithPort]) -> AnyResult<()> + Send + Sync + 'f>;
type OnIPsChosen<'f> =
    Box<dyn Fn(&mut dyn CallbackContext, &[IpAddrWithPort], &[IpAddrWithPort]) -> AnyResult<()> + Send + Sync + 'f>;
type OnRequest<'f> = Box<dyn Fn(&mut dyn ExtendedCallbackContext) -> AnyResult<()> + Send + Sync + 'f>;
type OnRetry<'f> = Box<dyn Fn(&mut dyn ExtendedCallbackContext, Duration) -> AnyResult<()> + Send + Sync + 'f>;
type OnResponse<'f> = Box<dyn Fn(&mut dyn ExtendedCallbackContext, &ResponseParts) -> AnyResult<()> + Send + Sync + 'f>;
type OnError<'f> = Box<dyn Fn(&mut dyn ExtendedCallbackContext, &ResponseError) -> AnyResult<()> + Send + Sync + 'f>;

#[derive(Default)]
pub(super) struct Callbacks<'f> {
    on_uploading_progress: Box<[OnProgress<'f>]>,
    on_receive_response_status: Box<[OnStatusCode<'f>]>,
    on_receive_response_header: Box<[OnHeader<'f>]>,
    on_to_resolve_domain: Box<[OnToResolveDomain<'f>]>,
    on_domain_resolved: Box<[OnDomainResolved<'f>]>,
    on_to_choose_ips: Box<[OnToChooseIPs<'f>]>,
    on_ips_chosen: Box<[OnIPsChosen<'f>]>,
    on_before_request_signed: Box<[OnRequest<'f>]>,
    on_after_request_signed: Box<[OnRequest<'f>]>,
    on_response: Box<[OnResponse<'f>]>,
    on_error: Box<[OnError<'f>]>,
    on_before_backoff: Box<[OnRetry<'f>]>,
    on_after_backoff: Box<[OnRetry<'f>]>,
}

#[derive(Default)]
pub(super) struct CallbacksBuilder<'f> {
    on_uploading_progress: Vec<OnProgress<'f>>,
    on_receive_response_status: Vec<OnStatusCode<'f>>,
    on_receive_response_header: Vec<OnHeader<'f>>,
    on_to_resolve_domain: Vec<OnToResolveDomain<'f>>,
    on_domain_resolved: Vec<OnDomainResolved<'f>>,
    on_to_choose_ips: Vec<OnToChooseIPs<'f>>,
    on_ips_chosen: Vec<OnIPsChosen<'f>>,
    on_before_request_signed: Vec<OnRequest<'f>>,
    on_after_request_signed: Vec<OnRequest<'f>>,
    on_response: Vec<OnResponse<'f>>,
    on_error: Vec<OnError<'f>>,
    on_before_backoff: Vec<OnRetry<'f>>,
    on_after_backoff: Vec<OnRetry<'f>>,
}

impl<'f> Callbacks<'f> {
    pub(super) fn call_uploading_progress_callbacks(
        &self,
        context: &dyn SimplifiedCallbackContext,
        progress_info: TransferProgressInfo<'_>,
    ) -> AnyResult<()> {
        self.on_uploading_progress_callbacks()
            .iter()
            .try_for_each(|callback| callback(context, progress_info))
    }

    pub(super) fn call_receive_response_status_callbacks(
        &self,
        context: &dyn SimplifiedCallbackContext,
        status_code: StatusCode,
    ) -> AnyResult<()> {
        self.on_receive_response_status_callbacks()
            .iter()
            .try_for_each(|callback| callback(context, status_code))
    }

    pub(super) fn call_receive_response_header_callbacks(
        &self,
        context: &dyn SimplifiedCallbackContext,
        header_name: &HeaderName,
        header_value: &HeaderValue,
    ) -> AnyResult<()> {
        self.on_receive_response_header_callbacks()
            .iter()
            .try_for_each(|callback| callback(context, header_name, header_value))
    }

    pub(super) fn call_to_resolve_domain_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        domain: &str,
    ) -> AnyResult<()> {
        self.on_to_resolve_domain_callbacks()
            .iter()
            .try_for_each(|callback| callback(context, domain))
    }

    pub(super) fn call_domain_resolved_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        domain: &str,
        answers: &ResolveAnswers,
    ) -> AnyResult<()> {
        self.on_domain_resolved_callbacks()
            .iter()
            .try_for_each(|callback| callback(context, domain, answers))
    }

    pub(super) fn call_to_choose_ips_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        ips: &[IpAddrWithPort],
    ) -> AnyResult<()> {
        self.on_to_choose_ips_callbacks()
            .iter()
            .try_for_each(|callback| callback(context, ips))
    }

    pub(super) fn call_ips_chosen_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        ips: &[IpAddrWithPort],
        chosen: &[IpAddrWithPort],
    ) -> AnyResult<()> {
        self.on_ips_chosen_callbacks()
            .iter()
            .try_for_each(|callback| callback(context, ips, chosen))
    }

    pub(super) fn call_before_request_signed_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
    ) -> AnyResult<()> {
        self.on_before_request_signed_callbacks()
            .iter()
            .try_for_each(|callback| callback(context))
    }

    pub(super) fn call_after_request_signed_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
    ) -> AnyResult<()> {
        self.on_after_request_signed_callbacks()
            .iter()
            .try_for_each(|callback| callback(context))
    }

    pub(super) fn call_response_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        response: &ResponseParts,
    ) -> AnyResult<()> {
        self.on_response_callbacks()
            .iter()
            .try_for_each(|callback| callback(context, response))
    }

    pub(super) fn call_error_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        error: &ResponseError,
    ) -> AnyResult<()> {
        self.on_error_callbacks()
            .iter()
            .try_for_each(|callback| callback(context, error))
    }

    pub(super) fn call_before_backoff_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        delay: Duration,
    ) -> AnyResult<()> {
        self.on_before_backoff_callbacks()
            .iter()
            .try_for_each(|callback| callback(context, delay))
    }

    pub(super) fn call_after_backoff_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        delay: Duration,
    ) -> AnyResult<()> {
        self.on_after_backoff_callbacks()
            .iter()
            .try_for_each(|callback| callback(context, delay))
    }

    #[inline]
    pub(super) fn on_uploading_progress_callbacks(&self) -> &[OnProgress<'f>] {
        &self.on_uploading_progress
    }

    #[inline]
    pub(super) fn on_receive_response_status_callbacks(&self) -> &[OnStatusCode<'f>] {
        &self.on_receive_response_status
    }

    #[inline]
    pub(super) fn on_receive_response_header_callbacks(&self) -> &[OnHeader<'f>] {
        &self.on_receive_response_header
    }

    #[inline]
    pub(super) fn on_to_resolve_domain_callbacks(&self) -> &[OnToResolveDomain<'f>] {
        &self.on_to_resolve_domain
    }

    #[inline]
    pub(super) fn on_domain_resolved_callbacks(&self) -> &[OnDomainResolved<'f>] {
        &self.on_domain_resolved
    }

    #[inline]
    pub(super) fn on_to_choose_ips_callbacks(&self) -> &[OnToChooseIPs<'f>] {
        &self.on_to_choose_ips
    }

    #[inline]
    pub(super) fn on_ips_chosen_callbacks(&self) -> &[OnIPsChosen<'f>] {
        &self.on_ips_chosen
    }

    #[inline]
    pub(super) fn on_before_request_signed_callbacks(&self) -> &[OnRequest<'f>] {
        &self.on_before_request_signed
    }

    #[inline]
    pub(super) fn on_after_request_signed_callbacks(&self) -> &[OnRequest<'f>] {
        &self.on_after_request_signed
    }

    #[inline]
    pub(super) fn on_response_callbacks(&self) -> &[OnResponse<'f>] {
        &self.on_response
    }

    #[inline]
    pub(super) fn on_error_callbacks(&self) -> &[OnError<'f>] {
        &self.on_error
    }

    #[inline]
    pub(super) fn on_before_backoff_callbacks(&self) -> &[OnRetry<'f>] {
        &self.on_before_backoff
    }

    #[inline]
    pub(super) fn on_after_backoff_callbacks(&self) -> &[OnRetry<'f>] {
        &self.on_after_backoff
    }
}

impl<'f> CallbacksBuilder<'f> {
    #[inline]
    pub(super) fn on_uploading_progress(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, TransferProgressInfo<'_>) -> AnyResult<()> + Send + Sync + 'f,
    ) -> &mut Self {
        self.on_uploading_progress.push(Box::new(callback));
        self
    }

    #[inline]
    pub(super) fn on_receive_response_status(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, StatusCode) -> AnyResult<()> + Send + Sync + 'f,
    ) -> &mut Self {
        self.on_receive_response_status.push(Box::new(callback));
        self
    }

    #[inline]
    pub(super) fn on_receive_response_header(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, &HeaderName, &HeaderValue) -> AnyResult<()> + Send + Sync + 'f,
    ) -> &mut Self {
        self.on_receive_response_header.push(Box::new(callback));
        self
    }

    #[inline]
    pub(super) fn on_to_resolve_domain(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str) -> AnyResult<()> + Send + Sync + 'f,
    ) -> &mut Self {
        self.on_to_resolve_domain.push(Box::new(callback));
        self
    }

    #[inline]
    pub(super) fn on_domain_resolved(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str, &ResolveAnswers) -> AnyResult<()> + Send + Sync + 'f,
    ) -> &mut Self {
        self.on_domain_resolved.push(Box::new(callback));
        self
    }

    #[inline]
    pub(super) fn on_to_choose_ips(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort]) -> AnyResult<()> + Send + Sync + 'f,
    ) -> &mut Self {
        self.on_to_choose_ips.push(Box::new(callback));
        self
    }

    #[inline]
    pub(super) fn on_ips_chosen(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort], &[IpAddrWithPort]) -> AnyResult<()>
            + Send
            + Sync
            + 'f,
    ) -> &mut Self {
        self.on_ips_chosen.push(Box::new(callback));
        self
    }

    #[inline]
    pub(super) fn on_before_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> AnyResult<()> + Send + Sync + 'f,
    ) -> &mut Self {
        self.on_before_request_signed.push(Box::new(callback));
        self
    }

    #[inline]
    pub(super) fn on_after_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> AnyResult<()> + Send + Sync + 'f,
    ) -> &mut Self {
        self.on_after_request_signed.push(Box::new(callback));
        self
    }

    #[inline]
    pub(super) fn on_response(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseParts) -> AnyResult<()> + Send + Sync + 'f,
    ) -> &mut Self {
        self.on_response.push(Box::new(callback));
        self
    }

    #[inline]
    pub(super) fn on_error(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseError) -> AnyResult<()> + Send + Sync + 'f,
    ) -> &mut Self {
        self.on_error.push(Box::new(callback));
        self
    }

    #[inline]
    pub(super) fn on_before_backoff(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> AnyResult<()> + Send + Sync + 'f,
    ) -> &mut Self {
        self.on_before_backoff.push(Box::new(callback));
        self
    }

    #[inline]
    pub(super) fn on_after_backoff(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> AnyResult<()> + Send + Sync + 'f,
    ) -> &mut Self {
        self.on_after_backoff.push(Box::new(callback));
        self
    }

    pub(super) fn build(&mut self) -> Callbacks<'f> {
        let owned = take(self);
        Callbacks {
            on_uploading_progress: owned.on_uploading_progress.into(),
            on_receive_response_status: owned.on_receive_response_status.into(),
            on_receive_response_header: owned.on_receive_response_header.into(),
            on_to_resolve_domain: owned.on_to_resolve_domain.into(),
            on_domain_resolved: owned.on_domain_resolved.into(),
            on_to_choose_ips: owned.on_to_choose_ips.into(),
            on_ips_chosen: owned.on_ips_chosen.into(),
            on_before_request_signed: owned.on_before_request_signed.into(),
            on_after_request_signed: owned.on_after_request_signed.into(),
            on_response: owned.on_response.into(),
            on_error: owned.on_error.into(),
            on_before_backoff: owned.on_before_backoff.into(),
            on_after_backoff: owned.on_after_backoff.into(),
        }
    }
}

impl fmt::Debug for Callbacks<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        macro_rules! field {
            ($ctx:ident, $method_name:expr, $method:ident) => {
                $ctx.field($method_name, &self.$method.len())
            };
        }
        let s = &mut f.debug_struct("Callbacks");
        field!(s, "on_uploading_progress", on_uploading_progress);
        field!(s, "on_receive_response_status", on_receive_response_status);
        field!(s, "on_receive_response_header", on_receive_response_header);
        field!(s, "on_to_resolve_domain", on_to_resolve_domain);
        field!(s, "on_domain_resolved", on_domain_resolved);
        field!(s, "on_to_choose_ips", on_to_choose_ips);
        field!(s, "on_ips_chosen", on_ips_chosen);
        field!(s, "on_before_request_signed", on_before_request_signed);
        field!(s, "on_after_request_signed", on_after_request_signed);
        field!(s, "on_response", on_response);
        field!(s, "on_error", on_error);
        field!(s, "on_before_backoff", on_before_backoff);
        field!(s, "on_after_backoff", on_after_backoff);
        s.finish()
    }
}

impl fmt::Debug for CallbacksBuilder<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        macro_rules! field {
            ($ctx:ident, $method_name:expr, $method:ident) => {
                $ctx.field($method_name, &self.$method.len())
            };
        }
        let s = &mut f.debug_struct("CallbacksBuilder");
        field!(s, "on_uploading_progress", on_uploading_progress);
        field!(s, "on_receive_response_status", on_receive_response_status);
        field!(s, "on_receive_response_header", on_receive_response_header);
        field!(s, "on_to_resolve_domain", on_to_resolve_domain);
        field!(s, "on_domain_resolved", on_domain_resolved);
        field!(s, "on_to_choose_ips", on_to_choose_ips);
        field!(s, "on_ips_chosen", on_ips_chosen);
        field!(s, "on_before_request_signed", on_before_request_signed);
        field!(s, "on_after_request_signed", on_after_request_signed);
        field!(s, "on_response", on_response);
        field!(s, "on_error", on_error);
        field!(s, "on_before_backoff", on_before_backoff);
        field!(s, "on_after_backoff", on_after_backoff);
        s.finish()
    }
}
