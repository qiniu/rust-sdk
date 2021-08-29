use super::super::{super::regions::IpAddrWithPort, ResponseError, RetriedStatsInfo};
use qiniu_http::Metrics;

#[derive(Clone, Debug)]
pub struct ChooserFeedback<'f> {
    ips: &'f [IpAddrWithPort],
    retried: &'f RetriedStatsInfo,
    metrics: Option<&'f dyn Metrics>,
    error: Option<&'f ResponseError>,
}

impl<'f> ChooserFeedback<'f> {
    #[inline]
    pub(in super::super::super) fn new(
        ips: &'f [IpAddrWithPort],
        retried: &'f RetriedStatsInfo,
        metrics: Option<&'f dyn Metrics>,
        error: Option<&'f ResponseError>,
    ) -> Self {
        Self {
            ips,
            retried,
            metrics,
            error,
        }
    }

    #[inline]
    pub fn ips(&self) -> &[IpAddrWithPort] {
        self.ips
    }

    #[inline]
    pub fn retried(&self) -> &RetriedStatsInfo {
        self.retried
    }

    #[inline]
    pub fn metrics(&self) -> Option<&dyn Metrics> {
        self.metrics
    }

    #[inline]
    pub fn error(&self) -> Option<&ResponseError> {
        self.error
    }
}
