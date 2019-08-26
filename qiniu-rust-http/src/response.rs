use super::header::{HeaderValue, Headers};
use getset::{Getters, MutGetters};
use std::{default::Default, fmt, io::Read};

pub type StatusCode = u16;
pub type Body = Box<dyn Read>;

#[derive(Getters, MutGetters)]
pub struct Response {
    status_code: StatusCode,

    #[get = "pub"]
    #[get_mut = "pub"]
    headers: Headers,

    #[get = "pub"]
    #[get_mut = "pub"]
    body: Option<Body>,
}

impl Response {
    pub fn new(status_code: StatusCode, headers: Headers, body: Option<Body>) -> Response {
        Response {
            status_code: status_code,
            headers: headers,
            body: body,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    pub fn header<HeaderNameT: AsRef<str>>(&self, header_name: HeaderNameT) -> Option<&HeaderValue> {
        self.headers.get(header_name.as_ref())
    }

    pub fn into_parts(self) -> (StatusCode, Headers, Option<Body>) {
        (self.status_code, self.headers, self.body)
    }

    pub fn into_body(self) -> Option<Body> {
        self.body
    }
}

pub struct ResponseBuilder {
    response: Response,
}

impl ResponseBuilder {
    pub fn default() -> ResponseBuilder {
        ResponseBuilder {
            response: Default::default(),
        }
    }

    pub fn status_code<S: Into<StatusCode>>(mut self, status_code: S) -> ResponseBuilder {
        self.response.status_code = status_code.into();
        self
    }

    pub fn header<HeaderNameT: Into<String>, HeaderValueT: Into<String>>(
        mut self,
        header_name: HeaderNameT,
        header_value: HeaderValueT,
    ) -> ResponseBuilder {
        self.response.headers.insert(header_name.into(), header_value.into());
        self
    }

    pub fn headers(mut self, headers: Headers) -> ResponseBuilder {
        self.response.headers = headers;
        self
    }

    pub fn body<B: Read + 'static>(mut self, body: B) -> ResponseBuilder {
        self.response.body = Some(Box::new(body));
        self
    }

    pub fn build(self) -> Response {
        self.response
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new(200, Headers::new(), None)
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Response")
            .field("status_code", &self.status_code())
            .field("headers", self.headers())
            .finish()
    }
}
