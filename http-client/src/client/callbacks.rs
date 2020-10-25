use super::{ChosenResult, ResponseError, RetriedStatsInfo};
use qiniu_http::{
    HeaderName, HeaderValue, HeadersOwned, Method, Request, StatusCode, SyncResponse,
};
use std::{fmt, iter::FromIterator, net::IpAddr, time::Duration};

#[cfg(any(feature = "async"))]
pub use qiniu_http::AsyncResponse;

#[derive(Clone, Debug)]
pub struct RequestInfo {
    method: Method,
    url: Box<str>,
    headers: HeadersOwned,
    body: Box<[u8]>,
}

impl RequestInfo {
    pub(super) fn new(request: &Request) -> Self {
        Self {
            method: request.method(),
            url: request.url().to_owned().into_boxed_str(),
            headers: HeadersOwned::from_iter(
                request
                    .headers()
                    .iter()
                    .map(|(name, value)| (name.to_owned().into(), value.to_owned().into_owned())),
            ),
            body: request.body().to_owned().into_boxed_slice(),
        }
    }

    #[inline]
    pub fn method(&self) -> Method {
        self.method
    }

    #[inline]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[inline]
    pub fn headers(&self) -> &HeadersOwned {
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
    headers: &'r HeadersOwned,
    server_ip: Option<IpAddr>,
    server_port: u16,
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
    pub fn headers(&self) -> &'r HeadersOwned {
        self.headers
    }

    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.server_ip
    }

    #[inline]
    pub fn server_port(&self) -> u16 {
        self.server_port
    }
}

#[derive(Debug)]
pub struct CallbackContext<'reqref, 'retried, 'req> {
    id: usize,
    request: &'reqref mut Request<'req>,
    retried: &'retried RetriedStatsInfo,
}

impl<'reqref, 'retried, 'req> CallbackContext<'reqref, 'retried, 'req> {
    pub(super) fn new(
        id: usize,
        request: &'reqref mut Request<'req>,
        retried: &'retried RetriedStatsInfo,
    ) -> Self {
        Self {
            id,
            request,
            retried,
        }
    }

    #[inline]
    pub fn id(&self) -> usize {
        self.id
    }

    #[inline]
    pub fn request(&self) -> &Request {
        self.request
    }

    #[inline]
    pub fn request_mut(&mut self) -> &mut Request<'req> {
        self.request
    }

    #[inline]
    pub fn retried(&self) -> &RetriedStatsInfo {
        self.retried
    }
}

pub(super) type OnProgress = Box<dyn Fn(&RequestInfo, u64, u64) -> bool + Send + Sync>;
pub(super) type OnBody = Box<dyn Fn(&RequestInfo, &[u8]) -> bool + Send + Sync>;
pub(super) type OnStatusCode = Box<dyn Fn(&RequestInfo, StatusCode) -> bool + Send + Sync>;
pub(super) type OnHeader =
    Box<dyn Fn(&RequestInfo, &HeaderName, &HeaderValue) -> bool + Send + Sync>;

pub(super) type OnToChooseDomain = Box<dyn Fn(&str) -> bool + Send + Sync>;
pub(super) type OnDomainChosen = Box<dyn Fn(&str, &ChosenResult) -> bool + Send + Sync>;
pub(super) type OnRequest = Box<dyn Fn(&mut CallbackContext) -> bool + Send + Sync>;
pub(super) type OnRetry = Box<dyn Fn(&mut CallbackContext, Duration) -> bool + Send + Sync>;
pub(super) type OnSuccess = Box<dyn Fn(&mut CallbackContext, &ResponseInfo) -> bool + Send + Sync>;
pub(super) type OnError = Box<dyn Fn(&mut CallbackContext, &ResponseError) -> bool + Send + Sync>;

#[derive(Default)]
pub struct Callbacks {
    on_uploading_progress: Box<[OnProgress]>,
    on_downloading_progress: Box<[OnProgress]>,
    on_send_request_body: Box<[OnBody]>,
    on_receive_response_status: Box<[OnStatusCode]>,
    on_receive_response_body: Box<[OnBody]>,
    on_receive_response_header: Box<[OnHeader]>,
    on_to_choose_domain: Box<[OnToChooseDomain]>,
    on_domain_chosen: Box<[OnDomainChosen]>,
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
    on_downloading_progress: Vec<OnProgress>,
    on_send_request_body: Vec<OnBody>,
    on_receive_response_status: Vec<OnStatusCode>,
    on_receive_response_body: Vec<OnBody>,
    on_receive_response_header: Vec<OnHeader>,
    on_to_choose_domain: Vec<OnToChooseDomain>,
    on_domain_chosen: Vec<OnDomainChosen>,
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
        uploaded: u64,
        total: u64,
    ) -> bool {
        !self
            .on_uploading_progress_callbacks()
            .iter()
            .any(|callback| !callback(request, uploaded, total))
    }

    #[inline]
    pub(super) fn call_downloading_progress_callbacks(
        &self,
        request: &RequestInfo,
        downloaded: u64,
        total: u64,
    ) -> bool {
        !self
            .on_downloading_progress_callbacks()
            .iter()
            .any(|callback| !callback(request, downloaded, total))
    }

    #[inline]
    pub(super) fn call_send_request_body_callbacks(
        &self,
        request: &RequestInfo,
        request_body: &[u8],
    ) -> bool {
        !self
            .on_send_request_body_callbacks()
            .iter()
            .any(|callback| !callback(request, request_body))
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
    pub(super) fn call_receive_response_body_callbacks(
        &self,
        request: &RequestInfo,
        response_body: &[u8],
    ) -> bool {
        !self
            .on_receive_response_body_callbacks()
            .iter()
            .any(|callback| !callback(request, response_body))
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
    pub(super) fn call_to_choose_domain_callbacks(&self, domain: &str) -> bool {
        !self
            .on_to_choose_domain_callbacks()
            .iter()
            .any(|callback| !callback(domain))
    }

    #[inline]
    pub(super) fn call_domain_chosen_callbacks(&self, domain: &str, result: &ChosenResult) -> bool {
        !self
            .on_domain_chosen_callbacks()
            .iter()
            .any(|callback| !callback(domain, result))
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
    pub fn on_downloading_progress_callbacks(&self) -> &[OnProgress] {
        &self.on_downloading_progress
    }

    #[inline]
    pub fn on_send_request_body_callbacks(&self) -> &[OnBody] {
        &self.on_send_request_body
    }

    #[inline]
    pub fn on_receive_response_status_callbacks(&self) -> &[OnStatusCode] {
        &self.on_receive_response_status
    }

    #[inline]
    pub fn on_receive_response_body_callbacks(&self) -> &[OnBody] {
        &self.on_receive_response_body
    }

    #[inline]
    pub fn on_receive_response_header_callbacks(&self) -> &[OnHeader] {
        &self.on_receive_response_header
    }

    #[inline]
    pub fn on_to_choose_domain_callbacks(&self) -> &[OnToChooseDomain] {
        &self.on_to_choose_domain
    }

    #[inline]
    pub fn on_domain_chosen_callbacks(&self) -> &[OnDomainChosen] {
        &self.on_domain_chosen
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
    pub fn on_downloading_progress(mut self, callback: OnProgress) -> Self {
        self.on_downloading_progress.push(callback);
        self
    }

    #[inline]
    pub fn on_send_request_body(mut self, callback: OnBody) -> Self {
        self.on_send_request_body.push(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_status(mut self, callback: OnStatusCode) -> Self {
        self.on_receive_response_status.push(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_body(mut self, callback: OnBody) -> Self {
        self.on_receive_response_body.push(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_header(mut self, callback: OnHeader) -> Self {
        self.on_receive_response_header.push(callback);
        self
    }

    #[inline]
    pub fn on_to_choose_domain(mut self, callback: OnToChooseDomain) -> Self {
        self.on_to_choose_domain.push(callback);
        self
    }

    #[inline]
    pub fn on_domain_chosen(mut self, callback: OnDomainChosen) -> Self {
        self.on_domain_chosen.push(callback);
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
            on_downloading_progress: self.on_downloading_progress.into(),
            on_send_request_body: self.on_send_request_body.into(),
            on_receive_response_status: self.on_receive_response_status.into(),
            on_receive_response_body: self.on_receive_response_body.into(),
            on_receive_response_header: self.on_receive_response_header.into(),
            on_to_choose_domain: self.on_to_choose_domain.into(),
            on_domain_chosen: self.on_domain_chosen.into(),
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
        field!(s, "on_downloading_progress", on_downloading_progress);
        field!(s, "on_send_request_body", on_send_request_body);
        field!(s, "on_receive_response_status", on_receive_response_status);
        field!(s, "on_receive_response_body", on_receive_response_body);
        field!(s, "on_receive_response_header", on_receive_response_header);
        field!(s, "on_to_choose_domain", on_to_choose_domain);
        field!(s, "on_domain_chosen", on_domain_chosen);
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
        field!(s, "on_downloading_progress", on_downloading_progress);
        field!(s, "on_send_request_body", on_send_request_body);
        field!(s, "on_receive_response_status", on_receive_response_status);
        field!(s, "on_receive_response_body", on_receive_response_body);
        field!(s, "on_receive_response_header", on_receive_response_header);
        field!(s, "on_to_choose_domain", on_to_choose_domain);
        field!(s, "on_domain_chosen", on_domain_chosen);
        field!(s, "on_before_request_signed", on_before_request_signed);
        field!(s, "on_after_request_signed", on_after_request_signed);
        field!(s, "on_success", on_success);
        field!(s, "on_error", on_error);
        field!(s, "on_before_retry_delay", on_before_retry_delay);
        field!(s, "on_after_retry_delay", on_after_retry_delay);
        s.finish()
    }
}
