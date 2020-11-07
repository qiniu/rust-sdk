mod feedback;
mod shuffled;
mod simple;

use super::{
    super::regions::{DomainWithPort, IpAddrWithPort},
    Resolver,
};
pub use feedback::ChooserFeedback;
use std::{any::Any, fmt::Debug};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

pub trait Chooser: Any + Debug + Sync + Send {
    fn choose(&self, domain: &DomainWithPort, ignore_frozen: bool) -> ChosenResult;
    fn choose_ips(&self, ips: &[IpAddrWithPort]) -> ChosenResult;
    fn feedback(&self, feedback: ChooserFeedback);

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_choose<'a>(
        &'a self,
        domain: &'a DomainWithPort,
        ignore_frozen: bool,
    ) -> BoxFuture<'a, ChosenResult> {
        Box::pin(async move { self.choose(domain, ignore_frozen) })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_choose_ips<'a>(&'a self, ips: &'a [IpAddrWithPort]) -> BoxFuture<'a, ChosenResult> {
        Box::pin(async move { self.choose_ips(ips) })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_feedback<'a>(&'a self, feedback: ChooserFeedback<'a>) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.feedback(feedback) })
    }

    fn resolver(&self) -> &dyn Resolver;
    fn resolver_mut(&mut self) -> &mut dyn Resolver;
    fn as_any(&self) -> &dyn Any;
    fn as_chooser(&self) -> &dyn Chooser;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChosenResult {
    IPs(Vec<IpAddrWithPort>),
    UseThisDomainDirectly,
    TryAnotherDomain,
}

pub use shuffled::{ShuffledChooser, ShuffledChooserBuilder};
pub use simple::SimpleChooser;
// TODO: 提供一个 Default Chooser
