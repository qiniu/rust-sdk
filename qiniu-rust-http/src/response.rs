use super::header::{HeaderValue, Headers};
use derive_builder::Builder;
use getset::{CopyGetters, Getters, MutGetters};
use std::{default::Default, fmt, io::Read};

pub type StatusCode = u16;
pub type Body = Box<dyn Read>;

#[derive(Getters, CopyGetters, MutGetters, Builder)]
#[builder(pattern = "owned", setter(into, strip_option), default)]
pub struct Response {
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    status_code: StatusCode,

    #[get = "pub"]
    #[get_mut = "pub"]
    headers: Headers,

    #[get = "pub"]
    #[get_mut = "pub"]
    #[builder(private, setter(name = "boxed_body"))]
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

impl ResponseBuilder {
    pub fn header<HeaderNameT: Into<String>, HeaderValueT: Into<String>>(
        mut self,
        header_name: HeaderNameT,
        header_value: HeaderValueT,
    ) -> Self {
        match self.headers {
            Some(ref mut headers) => {
                headers.insert(header_name.into(), header_value.into());
            }
            None => {
                let mut headers = Headers::new();
                headers.insert(header_name.into(), header_value.into());
                self = self.headers(headers);
            }
        }
        self
    }

    pub fn body<B: Read + 'static>(self, body: B) -> Self {
        self.boxed_body(Box::new(body) as Body)
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
