use qiniu_http::{HeaderName, HeaderValue, Request, ResponseError, StatusCode};
use std::fmt;

pub(super) type OnProgress = Box<dyn Fn(&Request, u64, u64) -> bool + Send + Sync>;
pub(super) type OnBody = Box<dyn Fn(&Request, &[u8]) -> bool + Send + Sync>;
pub(super) type OnRequest = Box<dyn Fn(&Request) -> bool + Send + Sync>;
pub(super) type OnRetry = Box<dyn Fn(&Request, usize) -> bool + Send + Sync>;
pub(super) type OnStatusCode = Box<dyn Fn(&Request, StatusCode) -> bool + Send + Sync>;
pub(super) type OnHeader = Box<dyn Fn(&Request, &HeaderName, &HeaderValue) -> bool + Send + Sync>;
pub(super) type OnError = Box<dyn Fn(&Request, &ResponseError) -> bool + Send + Sync>;

#[derive(Default)]
pub struct Callbacks {
    on_uploading_progress: Box<[OnProgress]>,
    on_downloading_progress: Box<[OnProgress]>,
    on_request: Box<[OnRequest]>,
    on_send_request_body: Box<[OnBody]>,
    on_receive_response_status: Box<[OnStatusCode]>,
    on_receive_response_body: Box<[OnBody]>,
    on_receive_response_header: Box<[OnHeader]>,
    on_error: Box<[OnError]>,
    on_retry: Box<[OnRetry]>,
}

#[derive(Default)]
pub struct CallbacksBuilder {
    on_uploading_progress: Vec<OnProgress>,
    on_downloading_progress: Vec<OnProgress>,
    on_request: Vec<OnRequest>,
    on_send_request_body: Vec<OnBody>,
    on_receive_response_status: Vec<OnStatusCode>,
    on_receive_response_body: Vec<OnBody>,
    on_receive_response_header: Vec<OnHeader>,
    on_error: Vec<OnError>,
    on_retry: Vec<OnRetry>,
}

impl Callbacks {
    #[inline]
    pub(super) fn call_uploading_progress_callbacks(
        &self,
        request: &Request,
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
        request: &Request,
        downloaded: u64,
        total: u64,
    ) -> bool {
        !self
            .on_downloading_progress_callbacks()
            .iter()
            .any(|callback| !callback(request, downloaded, total))
    }

    #[inline]
    pub(super) fn call_request_callbacks(&self, request: &Request) -> bool {
        !self
            .on_request_callbacks()
            .iter()
            .any(|callback| !callback(request))
    }

    #[inline]
    pub(super) fn call_send_request_body_callbacks(
        &self,
        request: &Request,
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
        request: &Request,
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
        request: &Request,
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
        request: &Request,
        header_name: &HeaderName,
        header_value: &HeaderValue,
    ) -> bool {
        !self
            .on_receive_response_header_callbacks()
            .iter()
            .any(|callback| !callback(request, header_name, header_value))
    }

    #[inline]
    pub(super) fn call_error_callbacks(&self, request: &Request, error: &ResponseError) -> bool {
        !self
            .on_error_callbacks()
            .iter()
            .any(|callback| !callback(request, error))
    }

    #[inline]
    pub(super) fn call_retry_callbacks(&self, request: &Request, retried: usize) -> bool {
        !self
            .on_retry_callbacks()
            .iter()
            .any(|callback| !callback(request, retried))
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
    pub fn on_request_callbacks(&self) -> &[OnRequest] {
        &self.on_request
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
    pub fn on_error_callbacks(&self) -> &[OnError] {
        &self.on_error
    }

    #[inline]
    pub fn on_retry_callbacks(&self) -> &[OnRetry] {
        &self.on_retry
    }
}

impl CallbacksBuilder {
    #[inline]
    pub fn on_uploading_progress(mut self, callback: impl Into<OnProgress>) -> Self {
        self.on_uploading_progress.push(callback.into());
        self
    }

    #[inline]
    pub fn on_downloading_progress(mut self, callback: impl Into<OnProgress>) -> Self {
        self.on_downloading_progress.push(callback.into());
        self
    }

    #[inline]
    pub fn on_request(mut self, callback: impl Into<OnRequest>) -> Self {
        self.on_request.push(callback.into());
        self
    }

    #[inline]
    pub fn on_send_request_body(mut self, callback: impl Into<OnBody>) -> Self {
        self.on_send_request_body.push(callback.into());
        self
    }

    #[inline]
    pub fn on_receive_response_status(mut self, callback: impl Into<OnStatusCode>) -> Self {
        self.on_receive_response_status.push(callback.into());
        self
    }

    #[inline]
    pub fn on_receive_response_body(mut self, callback: impl Into<OnBody>) -> Self {
        self.on_receive_response_body.push(callback.into());
        self
    }

    #[inline]
    pub fn on_receive_response_header(mut self, callback: impl Into<OnHeader>) -> Self {
        self.on_receive_response_header.push(callback.into());
        self
    }

    #[inline]
    pub fn on_error(mut self, callback: impl Into<OnError>) -> Self {
        self.on_error.push(callback.into());
        self
    }

    #[inline]
    pub fn on_retry(mut self, callback: impl Into<OnRetry>) -> Self {
        self.on_retry.push(callback.into());
        self
    }

    #[inline]
    pub fn build(self) -> Callbacks {
        Callbacks {
            on_uploading_progress: self.on_uploading_progress.into(),
            on_downloading_progress: self.on_downloading_progress.into(),
            on_request: self.on_request.into(),
            on_send_request_body: self.on_send_request_body.into(),
            on_receive_response_status: self.on_receive_response_status.into(),
            on_receive_response_body: self.on_receive_response_body.into(),
            on_receive_response_header: self.on_receive_response_header.into(),
            on_error: self.on_error.into(),
            on_retry: self.on_retry.into(),
        }
    }
}

impl fmt::Debug for Callbacks {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        macro_rules! field {
            ($ctx:ident,$method:ident) => {
                $ctx.field("$method", &self.$method.len())
            };
        }
        let s = &mut f.debug_struct("Callbacks");
        field!(s, on_uploading_progress);
        field!(s, on_downloading_progress);
        field!(s, on_request);
        field!(s, on_send_request_body);
        field!(s, on_receive_response_status);
        field!(s, on_receive_response_body);
        field!(s, on_receive_response_header);
        field!(s, on_error);
        field!(s, on_retry);
        s.finish()
    }
}

impl fmt::Debug for CallbacksBuilder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        macro_rules! field {
            ($ctx:ident,$method:ident) => {
                $ctx.field("$method", &self.$method.len())
            };
        }
        let s = &mut f.debug_struct("CallbacksBuilder");
        field!(s, on_uploading_progress);
        field!(s, on_downloading_progress);
        field!(s, on_request);
        field!(s, on_send_request_body);
        field!(s, on_receive_response_status);
        field!(s, on_receive_response_body);
        field!(s, on_receive_response_header);
        field!(s, on_error);
        field!(s, on_retry);
        s.finish()
    }
}
