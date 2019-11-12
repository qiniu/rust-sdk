use super::{HeaderName, HeaderValue, Headers};
use derive_builder::Builder;
use getset::{CopyGetters, Getters, MutGetters};
use std::{boxed::Box, default::Default, fmt, io::Read, net::IpAddr};

pub type StatusCode = u16;
pub type Body = Box<dyn Read>;

#[derive(Getters, CopyGetters, MutGetters, Builder)]
#[builder(
    pattern = "owned",
    setter(into, strip_option),
    build_fn(name = "inner_build", private)
)]
pub struct Response {
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    #[builder(default = "200")]
    status_code: StatusCode,

    #[get = "pub"]
    #[get_mut = "pub"]
    #[builder(default)]
    headers: Headers<'static>,

    #[get = "pub"]
    #[get_mut = "pub"]
    #[builder(private, default)]
    body: Option<Body>,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    #[builder(default)]
    server_ip: Option<IpAddr>,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    #[builder(default)]
    server_port: u16,
}

impl Response {
    pub fn header<HeaderNameT: Into<HeaderName<'static>>>(&self, header_name: HeaderNameT) -> Option<&HeaderValue> {
        self.headers.get(&header_name.into())
    }

    pub fn into_body(self) -> Option<Body> {
        self.body
    }

    pub fn take_body(&mut self) -> Option<Body> {
        self.body.take()
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

    pub fn build(self) -> Response {
        self.inner_build().unwrap()
    }
}

impl Default for Response {
    fn default() -> Self {
        ResponseBuilder::default().build()
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
