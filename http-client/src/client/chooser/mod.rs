mod direct;
mod feedback;
mod ip;
mod never_empty_handed;
mod shuffled;
mod subnet;

use super::super::regions::{DomainWithPort, IpAddrWithPort};
use auto_impl::auto_impl;
use dyn_clonable::clonable;
pub use feedback::ChooserFeedback;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// 选择 IP 地址接口
///
/// 还提供了对选择结果的反馈接口，用以修正自身选择逻辑，优化选择结果
///
/// 同时提供阻塞接口和异步接口，异步接口则需要启用 `async` 功能
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait Chooser: Clone + Debug + Sync + Send {
    /// 选择 IP 地址列表
    fn choose(&self, ips: &[IpAddrWithPort], opts: ChooseOptions) -> ChosenResults;

    /// 反馈选择的 IP 地址列表的结果
    fn feedback(&self, feedback: ChooserFeedback);

    /// 异步选择 IP 地址列表
    ///
    /// 该方法的异步版本为 [`Self::async_choose`]。
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_choose<'a>(&'a self, ips: &'a [IpAddrWithPort], opts: ChooseOptions<'a>) -> BoxFuture<'a, ChosenResults> {
        Box::pin(async move { self.choose(ips, opts) })
    }

    /// 异步反馈选择的 IP 地址列表的结果
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_feedback<'a>(&'a self, feedback: ChooserFeedback<'a>) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.feedback(feedback) })
    }
}

/// 选择 IP 地址列表的选项
#[derive(Debug, Copy, Clone, Default)]
pub struct ChooseOptions<'a> {
    domain: Option<&'a DomainWithPort>,
}

impl<'a> ChooseOptions<'a> {
    /// 获取 IP 地址的域名
    #[inline]
    pub fn domain(&'a self) -> Option<&'a DomainWithPort> {
        self.domain
    }

    /// 创建选择 IP 地址列表的选项构建器
    #[inline]
    pub fn builder() -> ChooseOptionsBuilder<'a> {
        ChooseOptionsBuilder::new()
    }
}

/// 选择 IP 地址列表的选项构建器
#[derive(Debug, Clone, Default)]
pub struct ChooseOptionsBuilder<'a>(ChooseOptions<'a>);

impl<'a> ChooseOptionsBuilder<'a> {
    /// 创建选择 IP 地址列表的选项构建器
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 IP 地址的域名
    #[inline]
    pub fn domain(&mut self, domain: &'a DomainWithPort) -> &mut Self {
        self.0.domain = Some(domain);
        self
    }

    /// 构建选择 IP 地址列表的选项
    #[inline]
    pub fn build(&self) -> ChooseOptions<'a> {
        self.0
    }
}

/// 经过选择的 IP 地址列表
#[derive(Debug)]
pub struct ChosenResults(Vec<IpAddrWithPort>);

impl ChosenResults {
    /// 获取 IP 地址列表
    #[inline]
    pub fn ip_addrs(&self) -> &[IpAddrWithPort] {
        &self.0
    }

    /// 获取 IP 地址列表的可变引用
    #[inline]
    pub fn ip_addrs_mut(&mut self) -> &mut Vec<IpAddrWithPort> {
        &mut self.0
    }

    /// 转换为 IP 地址列表
    #[inline]
    pub fn into_ip_addrs(self) -> Vec<IpAddrWithPort> {
        self.0
    }
}

impl From<Vec<IpAddrWithPort>> for ChosenResults {
    #[inline]
    fn from(ip_addrs: Vec<IpAddrWithPort>) -> Self {
        Self(ip_addrs)
    }
}

impl FromIterator<IpAddrWithPort> for ChosenResults {
    #[inline]
    fn from_iter<T: IntoIterator<Item = IpAddrWithPort>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl<'a> IntoIterator for &'a ChosenResults {
    type Item = &'a IpAddrWithPort;
    type IntoIter = std::slice::Iter<'a, IpAddrWithPort>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl IntoIterator for ChosenResults {
    type Item = IpAddrWithPort;
    type IntoIter = std::vec::IntoIter<IpAddrWithPort>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<ChosenResults> for Vec<IpAddrWithPort> {
    #[inline]
    fn from(answers: ChosenResults) -> Self {
        answers.0
    }
}

impl AsRef<[IpAddrWithPort]> for ChosenResults {
    #[inline]
    fn as_ref(&self) -> &[IpAddrWithPort] {
        &self.0
    }
}

impl AsMut<[IpAddrWithPort]> for ChosenResults {
    #[inline]
    fn as_mut(&mut self) -> &mut [IpAddrWithPort] {
        &mut self.0
    }
}

impl Deref for ChosenResults {
    type Target = [IpAddrWithPort];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChosenResults {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub use direct::DirectChooser;
pub use ip::{IpChooser, IpChooserBuilder};
pub use never_empty_handed::NeverEmptyHandedChooser;
pub use shuffled::ShuffledChooser;
pub use subnet::{SubnetChooser, SubnetChooserBuilder};
