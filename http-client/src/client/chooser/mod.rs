mod direct;
mod feedback;
mod ip;
mod never_empty_handed;
mod shuffled;
mod subnet;

use super::super::regions::IpAddrWithPort;
pub use feedback::ChooserFeedback;
use std::{
    any::Any,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

pub trait Chooser: Any + Debug + Sync + Send {
    fn choose(&self, ips: &[IpAddrWithPort], opts: &ChooseOptions) -> ChosenResults;
    fn feedback(&self, feedback: ChooserFeedback);

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_choose<'a>(
        &'a self,
        ips: &'a [IpAddrWithPort],
        opts: &'a ChooseOptions,
    ) -> BoxFuture<'a, ChosenResults> {
        Box::pin(async move { self.choose(ips, opts) })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_feedback<'a>(&'a self, feedback: ChooserFeedback<'a>) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.feedback(feedback) })
    }

    fn as_any(&self) -> &dyn Any;
    fn as_chooser(&self) -> &dyn Chooser;
}

#[derive(Debug, Clone, Default)]
pub struct ChooseOptions {}

#[derive(Debug, Clone, Default)]
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
