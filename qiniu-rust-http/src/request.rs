use super::{header::Headers, method::Method};
use getset::{Getters, MutGetters};

pub type URL = String;
pub type Body = [u8];

#[derive(Debug, Getters, MutGetters, Clone)]
pub struct Request<'b> {
    #[get = "pub"]
    #[get_mut = "pub"]
    url: URL,

    #[get = "pub"]
    #[get_mut = "pub"]
    method: Method,

    #[get = "pub"]
    #[get_mut = "pub"]
    headers: Headers,

    body: Option<&'b Body>,
}

impl<'b> Request<'b> {
    pub fn new<U: Into<URL>>(method: Method, url: U, headers: Headers, body: Option<&'b Body>) -> Request {
        Request {
            url: url.into(),
            method: method,
            headers: headers,
            body: body,
        }
    }

    pub fn body(&self) -> Option<&'b Body> {
        self.body
    }
}

pub struct RequestBuilder<'r> {
    request: Request<'r>,
}

impl<'r> RequestBuilder<'r> {
    pub fn default() -> RequestBuilder<'r> {
        RequestBuilder {
            request: Default::default(),
        }
    }

    pub fn method<M: Into<Method>>(mut self, method: M) -> RequestBuilder<'r> {
        self.request.method = method.into();
        self
    }

    pub fn url<U: Into<URL>>(mut self, url: U) -> RequestBuilder<'r> {
        self.request.url = url.into();
        self
    }

    pub fn header<HeaderNameT: Into<String>, HeaderValueT: Into<String>>(
        mut self,
        header_name: HeaderNameT,
        header_value: HeaderValueT,
    ) -> RequestBuilder<'r> {
        self.request.headers.insert(header_name.into(), header_value.into());
        self
    }

    pub fn headers(mut self, headers: Headers) -> RequestBuilder<'r> {
        self.request.headers = headers;
        self
    }

    pub fn body(mut self, body: &'r [u8]) -> RequestBuilder<'r> {
        self.request.body = Some(body);
        self
    }

    pub fn build(self) -> Request<'r> {
        self.request
    }
}

impl Default for Request<'_> {
    fn default() -> Self {
        Self::new(Method::GET, "http://localhost", Headers::new(), None)
    }
}
