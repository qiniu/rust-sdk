use super::request;
use crate::{config::Config, utils::auth::Auth};
use qiniu_http::Method;
use std::sync::Arc;

pub struct Client {
    auth: Arc<Auth>,
    config: Arc<Config>,
}

impl Client {
    pub fn new(auth: Arc<Auth>, config: Arc<Config>) -> Client {
        Client {
            auth: auth,
            config: config,
        }
    }

    pub fn get<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::GET, path, hosts)
    }

    pub fn post<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::POST, path, hosts)
    }

    pub fn put<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::PUT, path, hosts)
    }

    pub fn delete<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::DELETE, path, hosts)
    }

    pub fn patch<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::PATCH, path, hosts)
    }

    pub fn head<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::HEAD, path, hosts)
    }

    fn request_builder<'a>(&self, method: Method, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        request::Builder::new(self.auth.clone(), self.config.clone(), method, path, hosts)
    }
}
