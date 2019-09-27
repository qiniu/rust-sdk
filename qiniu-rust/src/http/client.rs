use super::request;
use crate::{config::Config, credential::Credential};
use qiniu_http::Method;

pub(crate) struct Client {
    credential: Credential,
    config: Config,
}

impl Client {
    pub(crate) fn new(credential: Credential, config: Config) -> Client {
        Client {
            credential: credential,
            config: config,
        }
    }

    pub(crate) fn get<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::GET, path, hosts)
    }

    pub(crate) fn post<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::POST, path, hosts)
    }

    pub(crate) fn put<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::PUT, path, hosts)
    }

    pub(crate) fn delete<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::DELETE, path, hosts)
    }

    pub(crate) fn patch<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::PATCH, path, hosts)
    }

    pub(crate) fn head<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::HEAD, path, hosts)
    }

    fn request_builder<'a>(&self, method: Method, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        request::Builder::new(self.credential.clone(), self.config.clone(), method, path, hosts)
    }
}
