use super::{HeaderName, HeaderValue, Headers};
use derive_builder::Builder;
use getset::{CopyGetters, Getters, MutGetters};
use std::{
    boxed::Box,
    default::Default,
    fmt,
    io::{Cursor, Read, Result as IOResult},
};

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
    headers: Headers<'static>,

    #[get = "pub"]
    #[get_mut = "pub"]
    #[builder(private)]
    body: Option<Body>,
}

impl Response {
    fn new(status_code: StatusCode, headers: Headers<'static>, body: Option<Body>) -> Response {
        Response {
            status_code: status_code,
            headers: headers,
            body: body,
        }
    }

    pub fn header<HeaderNameT: AsRef<str>>(&self, header_name: HeaderNameT) -> Option<&HeaderValue> {
        self.headers.get(header_name.as_ref())
    }

    pub fn into_parts(self) -> (StatusCode, Headers<'static>, Option<Body>) {
        (self.status_code, self.headers, self.body)
    }

    pub fn into_body(self) -> Option<Body> {
        self.body
    }

    pub fn take_body(&mut self) -> Option<Body> {
        self.body.take()
    }

    pub fn copy_body(&mut self) -> IOResult<Option<Body>> {
        self.body.take().map_or_else(
            || Ok(None),
            |mut body| {
                let mut body_buf = Vec::new();
                body.read_to_end(&mut body_buf)?;
                let body_clone = body_buf.clone();
                self.body = Some(Box::new(Cursor::new(body_clone)));
                Ok(Some(Box::new(Cursor::new(body_buf)) as Body))
            },
        )
    }
}

impl ResponseBuilder {
    pub fn header<HeaderNameT: Into<HeaderName<'static>>, HeaderValueT: Into<HeaderValue<'static>>>(
        mut self,
        header_name: HeaderNameT,
        header_value: HeaderValueT,
    ) -> Self {
        match &mut self.headers {
            Some(headers) => {
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

    pub fn stream<B: Read + 'static>(self, body: B) -> Self {
        self.body(Box::new(body) as Body)
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
