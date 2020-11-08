use super::super::{DomainOrIpAddr, Response, ResponseError, RetriedStatsInfo};
use std::{result::Result, time::Duration};

#[derive(Clone, Debug)]
pub struct ChooserFeedback<'f> {
    domain_or_ip_addr: &'f DomainOrIpAddr,
    retried: &'f RetriedStatsInfo,
    result: Result<ResponseMetrics, &'f ResponseError>,
}

impl<'f> ChooserFeedback<'f> {
    #[inline]
    pub(in super::super::super) fn new(
        domain_or_ip_addr: &'f DomainOrIpAddr,
        retried: &'f RetriedStatsInfo,
        result: Result<ResponseMetrics, &'f ResponseError>,
    ) -> Self {
        Self {
            domain_or_ip_addr,
            retried,
            result,
        }
    }

    #[inline]
    pub fn domain_or_ip_addr(&self) -> &DomainOrIpAddr {
        self.domain_or_ip_addr
    }

    #[inline]
    pub fn retried(&self) -> &RetriedStatsInfo {
        self.retried
    }

    #[inline]
    pub fn result(&self) -> Result<&ResponseMetrics, &ResponseError> {
        self.result.as_ref().map_err(|err| *err)
    }
}

#[derive(Clone, Default, Debug)]
pub struct ResponseMetrics {
    total_duration: Option<Duration>,
    name_lookup_duration: Option<Duration>,
    connect_duration: Option<Duration>,
    secure_connect_duration: Option<Duration>,
    redirect_duration: Option<Duration>,
    transfer_duration: Option<Duration>,
}

impl ResponseMetrics {
    #[inline]
    pub(in super::super::super) fn new_from_response<B>(response: &Response<B>) -> Self {
        Self {
            total_duration: response.total_duration(),
            name_lookup_duration: response.name_lookup_duration(),
            connect_duration: response.connect_duration(),
            secure_connect_duration: response.secure_connect_duration(),
            redirect_duration: response.redirect_duration(),
            transfer_duration: response.transfer_duration(),
        }
    }

    #[inline]
    pub fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    #[inline]
    pub fn name_lookup_duration(&self) -> Option<Duration> {
        self.name_lookup_duration
    }

    #[inline]
    pub fn connect_duration(&self) -> Option<Duration> {
        self.connect_duration
    }

    #[inline]
    pub fn secure_connect_duration(&self) -> Option<Duration> {
        self.secure_connect_duration
    }

    #[inline]
    pub fn redirect_duration(&self) -> Option<Duration> {
        self.redirect_duration
    }

    #[inline]
    pub fn transfer_duration(&self) -> Option<Duration> {
        self.transfer_duration
    }
}
