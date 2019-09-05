use qiniu_http::{Error, HTTPCaller, Headers, Request, Response, Result, StatusCode};
use serde::Serialize;
use std::{
    borrow::Cow,
    boxed::Box,
    io::{Cursor, Read},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

pub struct JSONCallMock<T>
where
    T: Serialize,
{
    status_code: StatusCode,
    response_headers: Headers,
    response_body: T,
}

impl<T> JSONCallMock<T>
where
    T: Serialize,
{
    pub fn new(status_code: StatusCode, headers: Headers, response_body: T) -> JSONCallMock<T> {
        JSONCallMock {
            status_code: status_code,
            response_headers: headers,
            response_body: response_body,
        }
    }
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

struct CounterCallMockInner<T>
where
    T: HTTPCaller,
{
    caller: T,
    call_counter: AtomicUsize,
    on_retry_request_counter: AtomicUsize,
    on_host_failed_counter: AtomicUsize,
    on_request_built_counter: AtomicUsize,
    on_response_counter: AtomicUsize,
    on_error_counter: AtomicUsize,
}

#[derive(Clone)]
pub struct CounterCallMock<T>
where
    T: HTTPCaller,
{
    inner: Arc<CounterCallMockInner<T>>,
}

impl<T> CounterCallMock<T>
where
    T: HTTPCaller,
{
    pub fn new(caller: T) -> CounterCallMock<T> {
        CounterCallMock {
            inner: Arc::new(CounterCallMockInner {
                caller: caller,
                call_counter: AtomicUsize::new(0),
                on_retry_request_counter: AtomicUsize::new(0),
                on_host_failed_counter: AtomicUsize::new(0),
                on_request_built_counter: AtomicUsize::new(0),
                on_response_counter: AtomicUsize::new(0),
                on_error_counter: AtomicUsize::new(0),
            }),
        }
    }

    pub fn call_called(&self) -> usize {
        self.inner.call_counter.load(Ordering::SeqCst)
    }

    pub fn on_retry_request_called(&self) -> usize {
        self.inner.on_retry_request_counter.load(Ordering::SeqCst)
    }

    pub fn on_host_failed_called(&self) -> usize {
        self.inner.on_host_failed_counter.load(Ordering::SeqCst)
    }

    pub fn on_request_built_called(&self) -> usize {
        self.inner.on_request_built_counter.load(Ordering::SeqCst)
    }

    pub fn on_response_called(&self) -> usize {
        self.inner.on_response_counter.load(Ordering::SeqCst)
    }

    pub fn on_error_called(&self) -> usize {
        self.inner.on_error_counter.load(Ordering::SeqCst)
    }

    pub fn as_boxed(&self) -> Box<Self> {
        Box::new(CounterCallMock {
            inner: self.inner.clone(),
        })
    }
}

impl<T> HTTPCaller for CounterCallMock<T>
where
    T: HTTPCaller,
{
    fn call(&self, request: &Request) -> Result<Response> {
        self.inner.call_counter.fetch_add(1, Ordering::SeqCst);
        self.inner.caller.call(request)
    }

    fn on_retry_request(&self, request: &Request, error: &Error, retried: usize, retries: usize) {
        self.inner.on_retry_request_counter.fetch_add(1, Ordering::SeqCst);
        self.inner.caller.on_retry_request(request, error, retried, retries)
    }
    fn on_host_failed(&self, failed_host: &str, error: &Error) {
        self.inner.on_host_failed_counter.fetch_add(1, Ordering::SeqCst);
        self.inner.caller.on_host_failed(failed_host, error)
    }
    fn on_request_built(&self, request: &mut Request) {
        self.inner.on_request_built_counter.fetch_add(1, Ordering::SeqCst);
        self.inner.caller.on_request_built(request)
    }
    fn on_response(&self, request: &Request, response: &Response) {
        self.inner.on_response_counter.fetch_add(1, Ordering::SeqCst);
        self.inner.caller.on_response(request, response)
    }
    fn on_error(&self, err: &Error) {
        self.inner.on_error_counter.fetch_add(1, Ordering::SeqCst);
        self.inner.caller.on_error(err)
    }
}

#[derive(Serialize)]
struct ErrorResponse<'e> {
    error: Cow<'e, str>,
}

pub struct ErrorResponseMock<'e> {
    status_code: StatusCode,
    error_message: Cow<'e, str>,
}

impl<'e> ErrorResponseMock<'e> {
    pub fn new<E: Into<Cow<'e, str>>>(status_code: StatusCode, error_message: E) -> ErrorResponseMock<'e> {
        ErrorResponseMock {
            status_code: status_code,
            error_message: error_message.into(),
        }
    }
}

impl<'e> HTTPCaller for ErrorResponseMock<'e> {
    fn call(&self, _request: &Request) -> Result<Response> {
        let mut headers = Headers::new();
        headers.insert("Content-Type".into(), "application/json".into());

        let body = serde_json::to_string(&ErrorResponse {
            error: self.error_message.clone(),
        })
        .unwrap();

        Ok(Response::new(
            self.status_code,
            headers,
            Some(Box::new(Cursor::new(body))),
        ))
    }
}
