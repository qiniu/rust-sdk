use super::super::SyncResponse;
use qiniu_http::{HeaderMap, StatusCode};
use std::{net::IpAddr, num::NonZeroU16};

#[cfg(any(feature = "async"))]
use super::super::AsyncResponse;

#[derive(Clone, Debug)]
pub struct ResponseInfo<'r> {
    status_code: StatusCode,
    headers: &'r HeaderMap,
    server_ip: Option<IpAddr>,
    server_port: Option<NonZeroU16>,
}

impl<'r> ResponseInfo<'r> {
    pub(in super::super) fn new_from_sync(response: &'r SyncResponse) -> Self {
        Self {
            status_code: response.status_code(),
            headers: &response.headers(),
            server_ip: response.server_ip(),
            server_port: response.server_port(),
        }
    }

    #[cfg(any(feature = "async"))]
    pub(in super::super) fn new_from_async(response: &'r AsyncResponse) -> Self {
        Self {
            status_code: response.status_code(),
            headers: &response.headers(),
            server_ip: response.server_ip(),
            server_port: response.server_port(),
        }
    }

    #[inline]
    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    #[inline]
    pub fn headers(&self) -> &'r HeaderMap {
        self.headers
    }

    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.server_ip
    }

    #[inline]
    pub fn server_port(&self) -> Option<NonZeroU16> {
        self.server_port
    }
}
