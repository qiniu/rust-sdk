use super::super::{DomainOrIpAddr, ResponseError, RetriedStatsInfo};

#[derive(Clone, Debug)]
pub struct ChooserFeedback<'f> {
    domain_or_ip_addr: &'f DomainOrIpAddr,
    retried: &'f RetriedStatsInfo,
    error: Option<&'f ResponseError>,
}

impl<'f> ChooserFeedback<'f> {
    #[inline]
    pub(in super::super::super) fn new(
        domain_or_ip_addr: &'f DomainOrIpAddr,
        retried: &'f RetriedStatsInfo,
        error: Option<&'f ResponseError>,
    ) -> Self {
        Self {
            domain_or_ip_addr,
            retried,
            error,
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
    pub fn error(&self) -> Option<&ResponseError> {
        self.error
    }
}
