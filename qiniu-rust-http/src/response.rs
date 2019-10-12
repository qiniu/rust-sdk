use super::{HeaderName, HeaderValue, Headers};
use derive_builder::Builder;
use getset::{CopyGetters, Getters, MutGetters};
use std::{
    boxed::Box,
    default::Default,
    fmt,
    io::{Cursor, Read, Result as IOResult},
    net::IpAddr,
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

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    server_ip: Option<IpAddr>,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    server_port: u16,
}

impl Response {
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
        Response {
            status_code: 200,
            headers: Headers::new(),
            body: None,
            server_ip: None,
            server_port: 0,
        }
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Response")
            .field("status_code", &self.status_code())
            .field("headers", self.headers())
            .field("server_ip", &self.server_ip())
            .field("server_port", &self.server_port())
            .finish()
    }
}
