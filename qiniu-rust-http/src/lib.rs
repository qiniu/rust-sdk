use http::{Request, Response};
use std::{boxed::Box, error::Error, io::Read, result::Result};

pub trait HTTPCaller {
    fn call(&self, request: Request<Vec<u8>>) -> Result<Response<Box<Read>>, Box<Error>>;
}
