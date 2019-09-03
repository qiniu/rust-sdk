use qiniu_http::{HTTPCaller, Headers, Request, Response, Result, StatusCode};
use serde::ser::Serialize;
use std::{
    boxed::Box,
    io::{Cursor, Read},
};

pub struct JSONCallMock<T>
where
    T: Serialize,
{
    pub status_code: StatusCode,
    pub response_headers: Headers,
    pub response_body: T,
}

impl<T> HTTPCaller for JSONCallMock<T>
where
    T: Serialize,
{
    fn call(&self, _request: &Request) -> Result<Response> {
        let mut response_headers = self.response_headers.to_owned();
        response_headers.insert("Content-Type".into(), "application/json".into());
        let body: Box<dyn Read> = Box::new(Cursor::new(serde_json::to_string(&self.response_body).unwrap()));
        Ok(Response::new(self.status_code, response_headers, Some(body)))
    }
}
