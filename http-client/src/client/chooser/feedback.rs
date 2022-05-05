use super::super::{
    super::regions::{DomainWithPort, IpAddrWithPort},
    ResponseError, RetriedStatsInfo,
};
use qiniu_http::{Extensions, Metrics};

/// 选择器反馈
///
/// 用以修正选择器的选择逻辑，优化选择结果
#[derive(Debug)]
pub struct ChooserFeedback<'f> {
    ips: &'f [IpAddrWithPort],
    domain: Option<&'f DomainWithPort>,
    retried: &'f RetriedStatsInfo,
    extensions: &'f mut Extensions,
    metrics: Option<&'f Metrics>,
    error: Option<&'f ResponseError>,
}

impl<'f> ChooserFeedback<'f> {
    pub(in super::super::super) fn new(
        ips: &'f [IpAddrWithPort],
        domain: Option<&'f DomainWithPort>,
        retried: &'f RetriedStatsInfo,
        extensions: &'f mut Extensions,
        metrics: Option<&'f Metrics>,
        error: Option<&'f ResponseError>,
    ) -> Self {
        Self {
            ips,
            domain,
            retried,
            extensions,
            metrics,
            error,
        }
    }

    /// 获取 IP 地址列表
    #[inline]
    pub fn ips(&'f self) -> &'f [IpAddrWithPort] {
        self.ips
    }

    /// 获取域名
    ///
    /// 如果不存在域名的话，则返回 [`None`]
    #[inline]
    pub fn domain(&'f self) -> Option<&'f DomainWithPort> {
        self.domain
    }

    /// 获取重试统计信息
    #[inline]
    pub fn retried(&'f self) -> &'f RetriedStatsInfo {
        self.retried
    }

    /// 获取扩展信息
    #[inline]
    pub fn extensions(&'f self) -> &'f Extensions {
        self.extensions
    }

    /// 获取扩展信息的可变引用
    #[inline]
    pub fn extensions_mut(&'f mut self) -> &'f mut Extensions {
        self.extensions
    }

    /// 获取 HTTP 响应的指标信息
    #[inline]
    pub fn metrics(&'f self) -> Option<&'f Metrics> {
        self.metrics
    }

    /// 获取 HTTP 响应错误
    #[inline]
    pub fn error(&'f self) -> Option<&'f ResponseError> {
        self.error
    }
}
