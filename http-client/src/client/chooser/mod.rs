mod direct;
mod feedback;
mod ip;
mod never_empty_handed;
mod shuffled;
mod subnet;

use super::super::regions::IpAddrWithPort;
use auto_impl::auto_impl;
pub use feedback::ChooserFeedback;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait Chooser: Debug + Sync + Send {
    fn choose(&self, ips: &[IpAddrWithPort], opts: &ChooseOptions) -> ChosenResults;
    fn feedback(&self, feedback: ChooserFeedback);

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_choose<'a>(
        &'a self,
        ips: &'a [IpAddrWithPort],
        opts: &'a ChooseOptions,
    ) -> BoxFuture<'a, ChosenResults> {
        Box::pin(async move { self.choose(ips, opts) })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_feedback<'a>(&'a self, feedback: ChooserFeedback<'a>) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.feedback(feedback) })
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChooseOptions {}

#[derive(Debug)]
pub struct ChosenResults(Vec<IpAddrWithPort>);

impl ChosenResults {
    #[inline]
    pub fn ip_addrs(&self) -> &[IpAddrWithPort] {
        &self.0
    }

    #[inline]
    pub fn ip_addrs_mut(&mut self) -> &mut Vec<IpAddrWithPort> {
        &mut self.0
    }

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
