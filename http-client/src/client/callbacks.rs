use crate::SimplifiedCallbackContext;

use super::{
    super::regions::IpAddrWithPort,
    callback::{CallbackContext, ExtendedCallbackContext},
    ResolveAnswers, ResponseError, ResponseInfo,
};
use qiniu_http::{HeaderName, HeaderValue, StatusCode, TransferProgressInfo};
use std::{fmt, time::Duration};

pub type OnProgress =
    Box<dyn Fn(&dyn SimplifiedCallbackContext, &TransferProgressInfo) -> bool + Send + Sync>;
pub type OnStatusCode =
    Box<dyn Fn(&dyn SimplifiedCallbackContext, StatusCode) -> bool + Send + Sync>;
pub type OnHeader =
    Box<dyn Fn(&dyn SimplifiedCallbackContext, &HeaderName, &HeaderValue) -> bool + Send + Sync>;

pub type OnToResolveDomain = Box<dyn Fn(&mut dyn CallbackContext, &str) -> bool + Send + Sync>;
pub type OnDomainResolved =
    Box<dyn Fn(&mut dyn CallbackContext, &str, &ResolveAnswers) -> bool + Send + Sync>;
pub type OnToChooseIPs =
    Box<dyn Fn(&mut dyn CallbackContext, &[IpAddrWithPort]) -> bool + Send + Sync>;
pub type OnIPsChosen = Box<
    dyn Fn(&mut dyn CallbackContext, &[IpAddrWithPort], &[IpAddrWithPort]) -> bool + Send + Sync,
>;
pub type OnRequest = Box<dyn Fn(&mut dyn ExtendedCallbackContext) -> bool + Send + Sync>;
pub type OnRetry = Box<dyn Fn(&mut dyn ExtendedCallbackContext, Duration) -> bool + Send + Sync>;
pub type OnSuccess =
    Box<dyn Fn(&mut dyn ExtendedCallbackContext, &ResponseInfo) -> bool + Send + Sync>;
pub type OnError =
    Box<dyn Fn(&mut dyn ExtendedCallbackContext, &ResponseError) -> bool + Send + Sync>;

#[derive(Default)]
pub struct Callbacks {
    on_uploading_progress: Box<[OnProgress]>,
    on_receive_response_status: Box<[OnStatusCode]>,
    on_receive_response_header: Box<[OnHeader]>,
    on_to_resolve_domain: Box<[OnToResolveDomain]>,
    on_domain_resolved: Box<[OnDomainResolved]>,
    on_to_choose_ips: Box<[OnToChooseIPs]>,
    on_ips_chosen: Box<[OnIPsChosen]>,
    on_before_request_signed: Box<[OnRequest]>,
    on_after_request_signed: Box<[OnRequest]>,
    on_success: Box<[OnSuccess]>,
    on_error: Box<[OnError]>,
    on_before_backoff: Box<[OnRetry]>,
    on_after_backoff: Box<[OnRetry]>,
}

#[derive(Default)]
pub struct CallbacksBuilder {
    on_uploading_progress: Vec<OnProgress>,
    on_receive_response_status: Vec<OnStatusCode>,
    on_receive_response_header: Vec<OnHeader>,
    on_to_resolve_domain: Vec<OnToResolveDomain>,
    on_domain_resolved: Vec<OnDomainResolved>,
    on_to_choose_ips: Vec<OnToChooseIPs>,
    on_ips_chosen: Vec<OnIPsChosen>,
    on_before_request_signed: Vec<OnRequest>,
    on_after_request_signed: Vec<OnRequest>,
    on_success: Vec<OnSuccess>,
    on_error: Vec<OnError>,
    on_before_backoff: Vec<OnRetry>,
    on_after_backoff: Vec<OnRetry>,
}

impl Callbacks {
    #[inline]
    pub(super) fn call_uploading_progress_callbacks(
        &self,
        context: &dyn SimplifiedCallbackContext,
        progress_info: &TransferProgressInfo,
    ) -> bool {
        !self
            .on_uploading_progress_callbacks()
            .iter()
            .any(|callback| !callback(context, progress_info))
    }

    #[inline]
    pub(super) fn call_receive_response_status_callbacks(
        &self,
        context: &dyn SimplifiedCallbackContext,
        status_code: StatusCode,
    ) -> bool {
        !self
            .on_receive_response_status_callbacks()
            .iter()
            .any(|callback| !callback(context, status_code))
    }

    #[inline]
    pub(super) fn call_receive_response_header_callbacks(
        &self,
        context: &dyn SimplifiedCallbackContext,
        header_name: &HeaderName,
        header_value: &HeaderValue,
    ) -> bool {
        !self
            .on_receive_response_header_callbacks()
            .iter()
            .any(|callback| !callback(context, header_name, header_value))
    }

    #[inline]
    pub(super) fn call_to_resolve_domain_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        domain: &str,
    ) -> bool {
        !self
            .on_to_resolve_domain_callbacks()
            .iter()
            .any(|callback| !callback(context, domain))
    }

    #[inline]
    pub(super) fn call_domain_resolved_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        domain: &str,
        answers: &ResolveAnswers,
    ) -> bool {
        !self
            .on_domain_resolved_callbacks()
            .iter()
            .any(|callback| !callback(context, domain, answers))
    }

    #[inline]
    pub(super) fn call_to_choose_ips_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        ips: &[IpAddrWithPort],
    ) -> bool {
        !self
            .on_to_choose_ips_callbacks()
            .iter()
            .any(|callback| !callback(context, ips))
    }

    #[inline]
    pub(super) fn call_ips_chosen_callbacks(
        &self,
        context: &mut dyn CallbackContext,
        ips: &[IpAddrWithPort],
        chosen: &[IpAddrWithPort],
    ) -> bool {
        !self
            .on_ips_chosen_callbacks()
            .iter()
            .any(|callback| !callback(context, ips, chosen))
    }

    #[inline]
    pub(super) fn call_before_request_signed_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
    ) -> bool {
        !self
            .on_before_request_signed_callbacks()
            .iter()
            .any(|callback| !callback(context))
    }

    #[inline]
    pub(super) fn call_after_request_signed_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
    ) -> bool {
        !self
            .on_after_request_signed_callbacks()
            .iter()
            .any(|callback| !callback(context))
    }

    #[inline]
    pub(super) fn call_success_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        response: &ResponseInfo,
    ) -> bool {
        !self
            .on_success_callbacks()
            .iter()
            .any(|callback| !callback(context, response))
    }

    #[inline]
    pub(super) fn call_error_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        error: &ResponseError,
    ) -> bool {
        !self
            .on_error_callbacks()
            .iter()
            .any(|callback| !callback(context, error))
    }

    #[inline]
    pub(super) fn call_before_backoff_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        delay: Duration,
    ) -> bool {
        !self
            .on_before_backoff_callbacks()
            .iter()
            .any(|callback| !callback(context, delay))
    }

    #[inline]
    pub(super) fn call_after_backoff_callbacks(
        &self,
        context: &mut dyn ExtendedCallbackContext,
        delay: Duration,
    ) -> bool {
        !self
            .on_after_backoff_callbacks()
            .iter()
            .any(|callback| !callback(context, delay))
    }

    #[inline]
    pub fn builder() -> CallbacksBuilder {
        Default::default()
    }

    #[inline]
    pub fn on_uploading_progress_callbacks(&self) -> &[OnProgress] {
        &self.on_uploading_progress
    }

    #[inline]
    pub fn on_receive_response_status_callbacks(&self) -> &[OnStatusCode] {
        &self.on_receive_response_status
    }

    #[inline]
    pub fn on_receive_response_header_callbacks(&self) -> &[OnHeader] {
        &self.on_receive_response_header
    }

    #[inline]
    pub fn on_to_resolve_domain_callbacks(&self) -> &[OnToResolveDomain] {
        &self.on_to_resolve_domain
    }

    #[inline]
    pub fn on_domain_resolved_callbacks(&self) -> &[OnDomainResolved] {
        &self.on_domain_resolved
    }

    #[inline]
    pub fn on_to_choose_ips_callbacks(&self) -> &[OnToChooseIPs] {
        &self.on_to_choose_ips
    }

    #[inline]
    pub fn on_ips_chosen_callbacks(&self) -> &[OnIPsChosen] {
        &self.on_ips_chosen
    }

    #[inline]
    pub fn on_before_request_signed_callbacks(&self) -> &[OnRequest] {
        &self.on_before_request_signed
    }

    #[inline]
    pub fn on_after_request_signed_callbacks(&self) -> &[OnRequest] {
        &self.on_after_request_signed
    }

    #[inline]
    pub fn on_success_callbacks(&self) -> &[OnSuccess] {
        &self.on_success
    }

    #[inline]
    pub fn on_error_callbacks(&self) -> &[OnError] {
        &self.on_error
    }

    #[inline]
    pub fn on_before_backoff_callbacks(&self) -> &[OnRetry] {
        &self.on_before_backoff
    }

    #[inline]
    pub fn on_after_backoff_callbacks(&self) -> &[OnRetry] {
        &self.on_after_backoff
    }
}

impl CallbacksBuilder {
    #[inline]
    pub fn on_uploading_progress(mut self, callback: OnProgress) -> Self {
        self.on_uploading_progress.push(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_status(mut self, callback: OnStatusCode) -> Self {
        self.on_receive_response_status.push(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_header(mut self, callback: OnHeader) -> Self {
        self.on_receive_response_header.push(callback);
        self
    }

    #[inline]
    pub fn on_to_resolve_domain(mut self, callback: OnToResolveDomain) -> Self {
        self.on_to_resolve_domain.push(callback);
        self
    }

    #[inline]
    pub fn on_domain_resolved(mut self, callback: OnDomainResolved) -> Self {
        self.on_domain_resolved.push(callback);
        self
    }

    #[inline]
    pub fn on_to_choose_ips(mut self, callback: OnToChooseIPs) -> Self {
        self.on_to_choose_ips.push(callback);
        self
    }

    #[inline]
    pub fn on_ips_chosen(mut self, callback: OnIPsChosen) -> Self {
        self.on_ips_chosen.push(callback);
        self
    }

    #[inline]
    pub fn on_before_request_signed(mut self, callback: OnRequest) -> Self {
        self.on_before_request_signed.push(callback);
        self
    }

    #[inline]
    pub fn on_after_request_signed(mut self, callback: OnRequest) -> Self {
        self.on_after_request_signed.push(callback);
        self
    }

    #[inline]
    pub fn on_success(mut self, callback: OnSuccess) -> Self {
        self.on_success.push(callback);
        self
    }

    #[inline]
    pub fn on_error(mut self, callback: OnError) -> Self {
        self.on_error.push(callback);
        self
    }

    #[inline]
    pub fn on_before_backoff(mut self, callback: OnRetry) -> Self {
        self.on_before_backoff.push(callback);
        self
    }

    #[inline]
    pub fn on_after_backoff(mut self, callback: OnRetry) -> Self {
        self.on_after_backoff.push(callback);
        self
    }

    #[inline]
    pub fn build(self) -> Callbacks {
        Callbacks {
            on_uploading_progress: self.on_uploading_progress.into(),
            on_receive_response_status: self.on_receive_response_status.into(),
            on_receive_response_header: self.on_receive_response_header.into(),
            on_to_resolve_domain: self.on_to_resolve_domain.into(),
            on_domain_resolved: self.on_domain_resolved.into(),
            on_to_choose_ips: self.on_to_choose_ips.into(),
            on_ips_chosen: self.on_ips_chosen.into(),
            on_before_request_signed: self.on_before_request_signed.into(),
            on_after_request_signed: self.on_after_request_signed.into(),
            on_success: self.on_success.into(),
            on_error: self.on_error.into(),
            on_before_backoff: self.on_before_backoff.into(),
            on_after_backoff: self.on_after_backoff.into(),
        }
    }
}

impl fmt::Debug for Callbacks {
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
        field!(s, "on_success", on_success);
        field!(s, "on_error", on_error);
        field!(s, "on_before_backoff", on_before_backoff);
        field!(s, "on_after_backoff", on_after_backoff);
        s.finish()
    }
}

impl fmt::Debug for CallbacksBuilder {
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
        field!(s, "on_success", on_success);
        field!(s, "on_error", on_error);
        field!(s, "on_before_backoff", on_before_backoff);
        field!(s, "on_after_backoff", on_after_backoff);
        s.finish()
    }
}
