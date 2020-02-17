use qiniu_http::{
    Error as HTTPError, ErrorKind as HTTPErrorKind, HTTPCaller, HTTPCallerErrorKind, Headers, Method, Request,
    Response, ResponseBuilder, Result, StatusCode,
};
use rand::{thread_rng, Rng};
use rand_core::RngCore;
use regex::Regex;
use serde::Serialize;
use std::{
    borrow::Cow,
    boxed::Box,
    convert::TryInto,
    io::{Error as IOError, ErrorKind as IOErrorKind},
    marker::{Send, Sync},
    sync::{
        atomic::{AtomicUsize, Ordering::Relaxed},
        Arc,
    },
};

pub fn fake_req_id() -> String {
    let mut rng = thread_rng();
    let mut buf = vec![0; 12];
    rng.fill_bytes(&mut buf);
    base64::encode_config(&buf, base64::URL_SAFE)
}

pub struct JSONCallMock<T: Serialize + Send + Sync> {
    status_code: StatusCode,
    response_headers: Headers<'static>,
    response_body: T,
}

impl<T: Serialize + Send + Sync> JSONCallMock<T> {
    pub fn new(status_code: StatusCode, response_headers: Headers<'static>, response_body: T) -> JSONCallMock<T> {
        JSONCallMock {
            status_code,
            response_headers,
            response_body,
        }
    }
}

impl<T: Serialize + Send + Sync> HTTPCaller for JSONCallMock<T> {
    fn call(&self, _request: &Request) -> Result<Response> {
        let mut headers = self.response_headers.to_owned();
        headers.insert("Content-Type".into(), "application/json".into());
        headers.insert("X-Reqid".into(), fake_req_id().into());
        Ok(ResponseBuilder::default()
            .status_code(self.status_code)
            .headers(headers)
            .bytes_as_body(serde_json::to_string(&self.response_body).unwrap())
            .build())
    }
}

struct CounterCallMockInner<T: HTTPCaller> {
    caller: T,
    call_counter: AtomicUsize,
}

pub struct CounterCallMock<T: HTTPCaller> {
    inner: Arc<CounterCallMockInner<T>>,
}

impl<T: HTTPCaller> CounterCallMock<T> {
    pub fn new(caller: T) -> CounterCallMock<T> {
        CounterCallMock {
            inner: Arc::new(CounterCallMockInner {
                caller,
                call_counter: AtomicUsize::new(0),
            }),
        }
    }

    pub fn call_called(&self) -> usize {
        self.inner.call_counter.load(Relaxed)
    }
}

impl<T: HTTPCaller> HTTPCaller for CounterCallMock<T> {
    fn call(&self, request: &Request) -> Result<Response> {
        self.inner.call_counter.fetch_add(1, Relaxed);
        self.inner.caller.call(request)
    }
}

impl<T: HTTPCaller> Clone for CounterCallMock<T> {
    fn clone(&self) -> Self {
        CounterCallMock {
            inner: self.inner.clone(),
        }
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
            status_code,
            error_message: error_message.into(),
        }
    }
}

impl<'e> HTTPCaller for ErrorResponseMock<'e> {
    fn call(&self, _request: &Request) -> Result<Response> {
        let mut headers = Headers::with_capacity(1);
        headers.insert("Content-Type".into(), "application/json".into());
        headers.insert("X-Reqid".into(), fake_req_id().into());

        let body = serde_json::to_string(&ErrorResponse {
            error: self.error_message.clone(),
        })
        .unwrap();

        Ok(ResponseBuilder::default()
            .status_code(self.status_code)
            .headers(headers)
            .bytes_as_body(body)
            .build())
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
            method,
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
                let called = handler.called.fetch_add(1, Relaxed);
                return (handler.handler)(request, called + 1);
            }
        }
        (self.default)(request)
    }
}

pub struct UploadingProgressErrorMock<T: HTTPCaller> {
    caller: T,
    packet_size: u32,
    uploading_failure_probability: f64,
}

impl<T: HTTPCaller> UploadingProgressErrorMock<T> {
    pub fn new(caller: T, packet_size: u32, uploading_failure_probability: f64) -> UploadingProgressErrorMock<T> {
        UploadingProgressErrorMock {
            caller,
            packet_size,
            uploading_failure_probability,
        }
    }
}

impl<T: HTTPCaller> HTTPCaller for UploadingProgressErrorMock<T> {
    fn call(&self, request: &Request) -> Result<Response> {
        let mut rng = thread_rng();
        let total_size: u64 = request
            .body()
            .as_ref()
            .map(|body| body.len().try_into().unwrap_or(u64::max_value()))
            .unwrap_or(0u64);
        let packet_size: u64 = self.packet_size.into();
        for i in 1..=total_size {
            if i % packet_size != total_size % packet_size {
                continue;
            }
            if rng.gen_range(
                0u64,
                ((1.max(total_size / packet_size) as f64) / self.uploading_failure_probability) as u64,
            ) == 0
            {
                return Err(HTTPError::new_retryable_error(
                    HTTPErrorKind::new_http_caller_error_kind(
                        HTTPCallerErrorKind::RequestError,
                        IOError::new(IOErrorKind::TimedOut, "Custom error"),
                    ),
                    true,
                    request,
                    None,
                ));
            }
            if let Some(on_uploading_progress) = request.on_uploading_progress() {
                on_uploading_progress.call(i, total_size);
            }
        }
        self.caller.call(request)
    }
}
