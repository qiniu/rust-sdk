use super::{
    super::regions::IpAddrWithPort, RequestWithoutEndpoints, ResolveAnswers, ResponseError,
    RetriedStatsInfo, SyncResponse,
};
use qiniu_http::{
    HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode, TransferProgressInfo, Uri,
};
use std::{fmt, net::IpAddr, num::NonZeroU16, time::Duration};

#[cfg(any(feature = "async"))]
pub use super::AsyncResponse;

#[derive(Clone, Debug)]
pub struct RequestInfo {
    method: Method,
    url: Uri,
    headers: HeaderMap,
    body: Box<[u8]>,
}

impl RequestInfo {
    pub(super) fn new(request: &Request) -> Self {
        Self {
            method: request.method().to_owned(),
            url: request.url().to_owned(),
            headers: request.headers().to_owned(),
            body: request.body().to_owned().into_boxed_slice(),
        }
    }

    #[inline]
    pub fn method(&self) -> &Method {
        &self.method
    }

    #[inline]
    pub fn url(&self) -> &Uri {
        &self.url
    }

    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    #[inline]
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

#[derive(Clone, Debug)]
pub struct ResponseInfo<'r> {
    status_code: StatusCode,
    headers: &'r HeaderMap,
    server_ip: Option<IpAddr>,
    server_port: Option<NonZeroU16>,
}

impl<'r> ResponseInfo<'r> {
    pub(super) fn new_from_sync(response: &'r SyncResponse) -> Self {
        Self {
            status_code: response.status_code(),
            headers: &response.headers(),
            server_ip: response.server_ip(),
            server_port: response.server_port(),
        }
    }

    #[cfg(any(feature = "async"))]
    pub(super) fn new_from_async(response: &'r AsyncResponse) -> Self {
        Self {
            status_code: response.status_code(),
            headers: &response.headers(),
            server_ip: response.server_ip(),
            server_port: response.server_port(),
        }
    }

    #[inline]
    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    #[inline]
    pub fn headers(&self) -> &'r HeaderMap {
        self.headers
    }

    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.server_ip
    }

    #[inline]
    pub fn server_port(&self) -> Option<NonZeroU16> {
        self.server_port
    }
}

#[derive(Debug)]
pub struct CallbackContext<'reqref, 'req, 'retried, 'httpreqref, 'httpreq> {
    request: &'reqref RequestWithoutEndpoints<'req>,
    http_request: &'httpreqref mut Request<'httpreq>,
    retried: &'retried RetriedStatsInfo,
}

impl<'reqref, 'req, 'retried, 'httpreqref, 'httpreq>
    CallbackContext<'reqref, 'req, 'retried, 'httpreqref, 'httpreq>
{
    pub(super) fn new(
        request: &'reqref RequestWithoutEndpoints<'req>,
        http_request: &'httpreqref mut Request<'httpreq>,
        retried: &'retried RetriedStatsInfo,
    ) -> Self {
        Self {
            request,
            http_request,
            retried,
        }
    }

    #[inline]
    pub fn request(&self) -> &Request {
        self.http_request
    }

    #[inline]
    pub fn request_mut(&mut self) -> &mut Request<'httpreq> {
        self.http_request
    }

    #[inline]
    pub fn retried(&self) -> &RetriedStatsInfo {
        self.retried
    }
}

pub(super) type OnProgress = Box<dyn Fn(&RequestInfo, &TransferProgressInfo) -> bool + Send + Sync>;
pub(super) type OnStatusCode = Box<dyn Fn(&RequestInfo, StatusCode) -> bool + Send + Sync>;
pub(super) type OnHeader =
    Box<dyn Fn(&RequestInfo, &HeaderName, &HeaderValue) -> bool + Send + Sync>;

pub(super) type OnToResolveDomain = Box<dyn Fn(&str) -> bool + Send + Sync>;
pub(super) type OnDomainResolved = Box<dyn Fn(&str, &ResolveAnswers) -> bool + Send + Sync>;
pub(super) type OnToChooseIPs = Box<dyn Fn(&[IpAddrWithPort]) -> bool + Send + Sync>;
pub(super) type OnIPsChosen =
    Box<dyn Fn(&[IpAddrWithPort], &[IpAddrWithPort]) -> bool + Send + Sync>;
pub(super) type OnRequest = Box<dyn Fn(&mut CallbackContext) -> bool + Send + Sync>;
pub(super) type OnRetry = Box<dyn Fn(&mut CallbackContext, Duration) -> bool + Send + Sync>;
pub(super) type OnSuccess = Box<dyn Fn(&mut CallbackContext, &ResponseInfo) -> bool + Send + Sync>;
pub(super) type OnError = Box<dyn Fn(&mut CallbackContext, &ResponseError) -> bool + Send + Sync>;

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
    on_before_retry_delay: Box<[OnRetry]>,
    on_after_retry_delay: Box<[OnRetry]>,
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
    on_before_retry_delay: Vec<OnRetry>,
    on_after_retry_delay: Vec<OnRetry>,
}

impl Callbacks {
    #[inline]
    pub(super) fn call_uploading_progress_callbacks(
        &self,
        request: &RequestInfo,
        progress_info: &TransferProgressInfo,
    ) -> bool {
        !self
            .on_uploading_progress_callbacks()
            .iter()
            .any(|callback| !callback(request, progress_info))
    }

    #[inline]
    pub(super) fn call_receive_response_status_callbacks(
        &self,
        request: &RequestInfo,
        status_code: StatusCode,
    ) -> bool {
        !self
            .on_receive_response_status_callbacks()
            .iter()
            .any(|callback| !callback(request, status_code))
    }

    #[inline]
    pub(super) fn call_receive_response_header_callbacks(
        &self,
        request: &RequestInfo,
        header_name: &HeaderName,
        header_value: &HeaderValue,
    ) -> bool {
        !self
            .on_receive_response_header_callbacks()
            .iter()
            .any(|callback| !callback(request, header_name, header_value))
    }

    #[inline]
    pub(super) fn call_to_resolve_domain_callbacks(&self, domain: &str) -> bool {
        !self
            .on_to_resolve_domain_callbacks()
            .iter()
            .any(|callback| !callback(domain))
    }

    #[inline]
    pub(super) fn call_domain_resolved_callbacks(
        &self,
        domain: &str,
        answers: &ResolveAnswers,
    ) -> bool {
        !self
            .on_domain_resolved_callbacks()
            .iter()
            .any(|callback| !callback(domain, answers))
    }

    #[inline]
    pub(super) fn call_to_choose_ips_callbacks(&self, ips: &[IpAddrWithPort]) -> bool {
        !self
            .on_to_choose_ips_callbacks()
            .iter()
            .any(|callback| !callback(ips))
    }

    #[inline]
    pub(super) fn call_ips_chosen_callbacks(
        &self,
        ips: &[IpAddrWithPort],
        chosen: &[IpAddrWithPort],
    ) -> bool {
        !self
            .on_ips_chosen_callbacks()
            .iter()
            .any(|callback| !callback(ips, chosen))
    }

    #[inline]
    pub(super) fn call_before_request_signed_callbacks(
        &self,
        context: &mut CallbackContext,
    ) -> bool {
        !self
            .on_before_request_signed_callbacks()
            .iter()
            .any(|callback| !callback(context))
    }

    #[inline]
    pub(super) fn call_after_request_signed_callbacks(
        &self,
        context: &mut CallbackContext,
    ) -> bool {
        !self
            .on_after_request_signed_callbacks()
            .iter()
            .any(|callback| !callback(context))
    }

    #[inline]
    pub(super) fn call_success_callbacks(
        &self,
        context: &mut CallbackContext,
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
        context: &mut CallbackContext,
        error: &ResponseError,
    ) -> bool {
        !self
            .on_error_callbacks()
            .iter()
            .any(|callback| !callback(context, error))
    }

    #[inline]
    pub(super) fn call_before_retry_delay_callbacks(
        &self,
        context: &mut CallbackContext,
        delay: Duration,
    ) -> bool {
        !self
            .on_before_retry_delay_callbacks()
            .iter()
            .any(|callback| !callback(context, delay))
    }

    #[inline]
    pub(super) fn call_after_retry_delay_callbacks(
        &self,
        context: &mut CallbackContext,
        delay: Duration,
    ) -> bool {
        !self
            .on_after_retry_delay_callbacks()
            .iter()
            .any(|callback| !callback(context, delay))
    }

    #[inline]
    pub fn builder() -> CallbacksBuilder {
        CallbacksBuilder::default()
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
    pub fn on_before_retry_delay_callbacks(&self) -> &[OnRetry] {
        &self.on_before_retry_delay
    }

    #[inline]
    pub fn on_after_retry_delay_callbacks(&self) -> &[OnRetry] {
        &self.on_after_retry_delay
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
    pub fn on_before_retry_delay(mut self, callback: OnRetry) -> Self {
        self.on_before_retry_delay.push(callback);
        self
    }

    #[inline]
    pub fn on_after_retry_delay(mut self, callback: OnRetry) -> Self {
        self.on_after_retry_delay.push(callback);
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
            on_before_retry_delay: self.on_before_retry_delay.into(),
            on_after_retry_delay: self.on_after_retry_delay.into(),
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
        field!(s, "on_before_retry_delay", on_before_retry_delay);
        field!(s, "on_after_retry_delay", on_after_retry_delay);
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
        field!(s, "on_before_retry_delay", on_before_retry_delay);
        field!(s, "on_after_retry_delay", on_after_retry_delay);
        s.finish()
    }
}
