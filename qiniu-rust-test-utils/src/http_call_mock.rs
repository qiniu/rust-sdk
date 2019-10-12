use qiniu_http::{HTTPCaller, Headers, Method, Request, Response, ResponseBuilder, Result, StatusCode};
use regex::Regex;
use serde::Serialize;
use std::{
    borrow::Cow,
    boxed::Box,
    io::Cursor,
    marker::{Send, Sync},
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
    response_headers: Headers<'static>,
    response_body: T,
}

impl<T> JSONCallMock<T>
where
    T: Serialize,
{
    pub fn new(status_code: StatusCode, headers: Headers<'static>, response_body: T) -> JSONCallMock<T> {
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
        let mut headers = self.response_headers.to_owned();
        headers.insert("Content-Type".into(), "application/json".into());
        Ok(ResponseBuilder::default()
            .status_code(self.status_code)
            .headers(headers)
            .stream(Cursor::new(serde_json::to_string(&self.response_body).unwrap()))
            .build()
            .unwrap())
    }
}

struct CounterCallMockInner<T>
where
    T: HTTPCaller,
{
    caller: T,
    call_counter: AtomicUsize,
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
            }),
        }
    }

    pub fn call_called(&self) -> usize {
        self.inner.call_counter.load(Ordering::SeqCst)
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
        let mut headers = Headers::with_capacity(1);
        headers.insert("Content-Type".into(), "application/json".into());

        let body = serde_json::to_string(&ErrorResponse {
            error: self.error_message.clone(),
        })
        .unwrap();

        Ok(ResponseBuilder::default()
            .status_code(self.status_code)
            .headers(headers)
            .stream(Cursor::new(body))
            .build()
            .unwrap())
    }
}

struct CallHandler {
    method: Method,
    url_regexp: regex::Regex,
    called: AtomicUsize,
    handler: Box<dyn Fn(&Request, usize) -> Result<Response> + Send + Sync>,
}

pub struct CallHandlers {
    handlers: Vec<CallHandler>,
    default: Box<dyn Fn(&Request) -> Result<Response> + Send + Sync>,
}

impl CallHandlers {
    pub fn new<R: Fn(&Request) -> Result<Response> + Send + Sync + 'static>(default_handler: R) -> Self {
        CallHandlers {
            handlers: Vec::new(),
            default: Box::new(default_handler),
        }
    }

    pub fn install<S: AsRef<str>, R: Fn(&Request, usize) -> Result<Response> + Send + Sync + 'static>(
        mut self,
        method: Method,
        url_regexp: S,
        handler: R,
    ) -> Self {
        self.handlers.push(CallHandler {
            method: method,
            url_regexp: Regex::new(url_regexp.as_ref()).unwrap(),
            handler: Box::new(handler),
            called: AtomicUsize::new(0),
        });
        self
    }
}

impl HTTPCaller for CallHandlers {
    fn call(&self, request: &Request) -> Result<Response> {
        for handler in self.handlers.iter() {
            if handler.method == request.method() && handler.url_regexp.is_match(request.url()) {
                let called = handler.called.fetch_add(1, Ordering::SeqCst);
                return (handler.handler)(request, called + 1);
            }
        }
        (self.default)(request)
    }
}
