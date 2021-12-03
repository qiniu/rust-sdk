use std::fmt;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RetriedStatsInfo {
    retried_total: usize,
    retried_on_current_endpoint: usize,
    retried_on_current_ips: usize,
    abandoned_endpoints: usize,
    abandoned_ips_of_current_endpoint: usize,
    switched_to_alternative_endpoints: bool,
}

impl RetriedStatsInfo {
    pub(super) fn increase(&mut self) {
        self.retried_total += 1;
        self.retried_on_current_endpoint += 1;
        self.retried_on_current_ips += 1;
    }

    pub(super) fn increase_abandoned_endpoints(&mut self) {
        self.abandoned_endpoints += 1;
    }

    pub(super) fn increase_abandoned_ips_of_current_endpoint(&mut self) {
        self.abandoned_ips_of_current_endpoint += 1;
    }

    pub(super) fn switch_to_alternative_endpoints(&mut self) {
        self.switched_to_alternative_endpoints = true;
        self.switch_endpoint();
    }

    pub(super) fn switch_endpoint(&mut self) {
        self.retried_on_current_endpoint = 0;
        self.abandoned_ips_of_current_endpoint = 0;
        self.switch_ips();
    }

    pub(super) fn switch_ips(&mut self) {
        self.retried_on_current_ips = 0;
    }

    #[inline]
    pub fn retried_total(&self) -> usize {
        self.retried_total
    }

    #[inline]
    pub fn retried_on_current_endpoint(&self) -> usize {
        self.retried_on_current_endpoint
    }

    #[inline]
    pub fn retried_on_current_ips(&self) -> usize {
        self.retried_on_current_ips
    }

    #[inline]
    pub fn abandoned_endpoints(&self) -> usize {
        self.abandoned_endpoints
    }

    #[inline]
    pub fn abandoned_ips_of_current_endpoint(&self) -> usize {
        self.abandoned_ips_of_current_endpoint
    }

    #[inline]
    pub fn switched_to_alternative_endpoints(&self) -> bool {
        self.switched_to_alternative_endpoints
    }
}

impl fmt::Display for RetriedStatsInfo {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{},{},{},{},{},{}",
            self.retried_total,
            self.retried_on_current_endpoint,
            self.retried_on_current_ips,
            self.abandoned_endpoints,
            self.abandoned_ips_of_current_endpoint,
            if self.switched_to_alternative_endpoints {
                "a" // alternative
            } else {
                "p" // preferred
            }
        )
    }
}
