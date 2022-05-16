use super::super::{
    super::regions::{DomainWithPort, IpAddrWithPort},
    ResponseError, RetriedStatsInfo,
};
use maybe_owned::{MaybeOwned, MaybeOwnedMut};
use qiniu_http::{Extensions, Metrics};
use std::mem::replace;

/// 选择器反馈
///
/// 用以修正选择器的选择逻辑，优化选择结果
#[derive(Debug)]
pub struct ChooserFeedback<'f> {
    ips: &'f [IpAddrWithPort],
    domain: Option<&'f DomainWithPort>,
    retried: MaybeOwned<'f, RetriedStatsInfo>,
    extensions: MaybeOwnedMut<'f, Extensions>,
    metrics: Option<&'f Metrics>,
    error: Option<&'f ResponseError>,
}

impl<'f> ChooserFeedback<'f> {
    /// 创建选择器反馈构建器
    #[inline]
    pub fn builder(ips: &'f [IpAddrWithPort]) -> ChooserFeedbackBuilder<'f> {
        ChooserFeedbackBuilder::new(ips)
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
        &self.retried
    }

    /// 获取扩展信息
    #[inline]
    pub fn extensions(&'f self) -> &'f Extensions {
        &self.extensions
    }

    /// 获取扩展信息的可变引用
    #[inline]
    pub fn extensions_mut(&'f mut self) -> &'f mut Extensions {
        &mut self.extensions
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

/// 选择器反馈构建器
#[derive(Debug)]
pub struct ChooserFeedbackBuilder<'f>(ChooserFeedback<'f>);

impl<'f> ChooserFeedbackBuilder<'f> {
    /// 创建选择器反馈构建器
    #[inline]
    pub fn new(ips: &'f [IpAddrWithPort]) -> Self {
        Self(ChooserFeedback {
            ips,
            domain: None,
            metrics: None,
            error: None,
            retried: Default::default(),
            extensions: Default::default(),
        })
    }

    /// 设置域名
    #[inline]
    pub fn domain(&mut self, domain: &'f DomainWithPort) -> &mut Self {
        self.0.domain = Some(domain);
        self
    }

    /// 设置重试统计信息
    #[inline]
    pub fn retried(&mut self, retried: &'f RetriedStatsInfo) -> &mut Self {
        self.0.retried = MaybeOwned::from(retried);
        self
    }

    /// 获取扩展信息的可变引用
    #[inline]
    pub fn extensions(&mut self, extensions: &'f mut Extensions) -> &mut Self {
        self.0.extensions = MaybeOwnedMut::from(extensions);
        self
    }

    /// 设置 HTTP 响应的指标信息
    #[inline]
    pub fn metrics(&mut self, metrics: &'f Metrics) -> &mut Self {
        self.0.metrics = Some(metrics);
        self
    }

    /// 设置 HTTP 响应错误
    #[inline]
    pub fn error(&mut self, error: &'f ResponseError) -> &mut Self {
        self.0.error = Some(error);
        self
    }

    /// 构建选择器反馈器
    #[inline]
    pub fn build(&mut self) -> ChooserFeedback<'f> {
        let ips = self.0.ips;
        replace(
            &mut self.0,
            ChooserFeedback {
                ips,
                domain: None,
                metrics: None,
                error: None,
                retried: Default::default(),
                extensions: Default::default(),
            },
        )
    }
}
