use qiniu_http::{HTTPCaller, Request, Response, Result};

pub enum HTTPHandler {
    Dynamic(Box<dyn HTTPCaller + Send + Sync>),
    Static(fn(request: &Request) -> Result<Response>),
}

impl HTTPCaller for HTTPHandler {
    fn call(&self, request: &Request) -> Result<Response> {
        match self {
            HTTPHandler::Dynamic(endpoint) => endpoint.call(request),
            HTTPHandler::Static(f) => (f)(request),
        }
    }
}

#[allow(dead_code)]
pub struct PanickedHTTPCaller(pub(crate) &'static str);

impl HTTPCaller for PanickedHTTPCaller {
    fn call(&self, _request: &Request) -> Result<Response> {
        panic!(self.0);
    }
}
