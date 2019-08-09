use http::{Request, Response};
use qiniu_http::HTTPCaller;
use std::{boxed::Box, error::Error, io::Read, result::Result};

pub struct PanickedHTTPCaller(pub(crate) &'static str);

impl HTTPCaller for PanickedHTTPCaller {
    fn call(&self, _request: Request<Vec<u8>>) -> Result<Response<Box<Read>>, Box<Error>> {
        panic!(self.0);
    }
}
