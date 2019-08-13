use super::{header::Headers, method::Method};
use getset::{Getters, MutGetters, Setters};

pub type URL = String;
pub type Body = Vec<u8>;

#[derive(Debug, Getters, Setters, MutGetters, Clone)]
#[get = "pub"]
#[set = "pub"]
#[get_mut = "pub"]
pub struct Request {
    url: URL,
    method: Method,
    headers: Headers,
    body: Option<Body>,
}

impl Request {
    pub fn new<U: Into<URL>>(
        method: Method,
        url: U,
        headers: Headers,
        body: Option<Body>,
    ) -> Request {
        Request {
            url: url.into(),
            method: method,
            headers: headers,
            body: body,
        }
    }

    pub fn into_parts(self) -> (URL, Method, Headers, Option<Body>) {
        (self.url, self.method, self.headers, self.body)
    }

    pub fn into_body(self) -> Option<Body> {
        self.body
    }
}

pub struct RequestBuilder {
    request: Request,
}

impl RequestBuilder {
    pub fn default() -> RequestBuilder {
        RequestBuilder {
            request: Default::default(),
        }
    }

    pub fn url<U: Into<URL>>(mut self, url: U) -> RequestBuilder {
        self.request.url = url.into();
        self
    }

    pub fn header<HeaderNameT: Into<String>, HeaderValueT: Into<String>>(
        mut self,
        header_name: HeaderNameT,
        header_value: HeaderValueT,
    ) -> RequestBuilder {
        self.request
            .headers
            .insert(header_name.into(), header_value.into());
        self
    }

    pub fn headers(mut self, headers: Headers) -> RequestBuilder {
        self.request.headers = headers;
        self
    }

    pub fn body<B: Into<Vec<u8>>>(mut self, body: B) -> RequestBuilder {
        self.request.body = Some(body.into());
        self
    }

    pub fn build(self) -> Request {
        self.request
    }
}

impl Default for Request {
    fn default() -> Self {
        Self::new(Method::GET, "http://localhost", Headers::new(), None)
    }
}
