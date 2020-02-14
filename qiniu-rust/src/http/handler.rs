use qiniu_http::{HTTPCaller, Request, Response, Result};

#[allow(dead_code)]
pub(crate) struct PanickedHTTPCaller(pub(crate) &'static str);

impl HTTPCaller for PanickedHTTPCaller {
    fn call(&self, _request: &Request) -> Result<Response> {
        panic!(self.0);
    }
}
