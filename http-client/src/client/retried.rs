use std::fmt;

/// 重试统计信息
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
    /// 提升当前终端地址的重试次数
    #[inline]
    pub fn increase_current_endpoint(&mut self) {
        self.retried_total += 1;
        self.retried_on_current_endpoint += 1;
        self.retried_on_current_ips += 1;
    }

    /// 提升放弃的终端地址的数量
    #[inline]
    pub fn increase_abandoned_endpoints(&mut self) {
        self.abandoned_endpoints += 1;
    }

    /// 提升放弃的终端的 IP 地址的数量
    #[inline]
    pub fn increase_abandoned_ips_of_current_endpoint(&mut self) {
        self.abandoned_ips_of_current_endpoint += 1;
    }

    /// 切换到备选终端地址
    #[inline]
    pub fn switch_to_alternative_endpoints(&mut self) {
        self.switched_to_alternative_endpoints = true;
        self.switch_endpoint();
    }

    /// 切换终端地址
    pub fn switch_endpoint(&mut self) {
        self.retried_on_current_endpoint = 0;
        self.abandoned_ips_of_current_endpoint = 0;
        self.switch_ips();
    }

    /// 切换当前 IP 地址
    pub fn switch_ips(&mut self) {
        self.retried_on_current_ips = 0;
    }

    /// 获取总共重试的次数
    #[inline]
    pub fn retried_total(&self) -> usize {
        self.retried_total
    }

    /// 获取当前终端地址的重试次数
    #[inline]
    pub fn retried_on_current_endpoint(&self) -> usize {
        self.retried_on_current_endpoint
    }

    /// 获取当前 IP 地址的重试次数
    #[inline]
    pub fn retried_on_current_ips(&self) -> usize {
        self.retried_on_current_ips
    }

    /// 获取放弃的终端地址的数量
    #[inline]
    pub fn abandoned_endpoints(&self) -> usize {
        self.abandoned_endpoints
    }

    /// 获取放弃的终端的 IP 地址的数量
    #[inline]
    pub fn abandoned_ips_of_current_endpoint(&self) -> usize {
        self.abandoned_ips_of_current_endpoint
    }

    /// 是否切换到了备选终端地址
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
