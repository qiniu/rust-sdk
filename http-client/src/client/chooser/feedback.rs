use super::super::{super::regions::IpAddrWithPort, ResponseError, RetriedStatsInfo};
use qiniu_http::{Extensions, Metrics};

#[derive(Debug)]
pub struct ChooserFeedback<'f> {
    ips: &'f [IpAddrWithPort],
    retried: &'f RetriedStatsInfo,
    extensions: &'f mut Extensions,
    metrics: Option<&'f dyn Metrics>,
    error: Option<&'f ResponseError>,
}

impl<'f> ChooserFeedback<'f> {
    #[inline]
    pub(in super::super::super) fn new(
        ips: &'f [IpAddrWithPort],
        retried: &'f RetriedStatsInfo,
        extensions: &'f mut Extensions,
        metrics: Option<&'f dyn Metrics>,
        error: Option<&'f ResponseError>,
    ) -> Self {
        Self {
            ips,
            retried,
            extensions,
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
    pub fn extensions(&self) -> &Extensions {
        self.extensions
    }

    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        self.extensions
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
