mod simple;

use super::{
    super::regions::{DomainWithPort, IpAddrWithPort},
    Resolver, ResponseError,
};
use std::{any::Any, fmt::Debug};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChosenResult {
    IPs(Vec<IpAddrWithPort>),
    UseThisDomainDirectly,
    TryAnotherDomain,
}

pub trait Chooser: Any + Debug + Sync + Send {
    fn choose(&self, domain: &DomainWithPort, ignore_frozen: bool) -> ChosenResult;

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

    fn choose_ips(&self, ips: &[IpAddrWithPort]) -> ChosenResult;
    fn freeze_domain(&self, domain: &DomainWithPort, error: &ResponseError);
    fn freeze_ips(&self, ips: &[IpAddrWithPort], error: &ResponseError);
    fn unfreeze_domain(&self, domain: &DomainWithPort);
    fn unfreeze_ips(&self, ips: &[IpAddrWithPort]);
    fn resolver(&self) -> &dyn Resolver;
    fn resolver_mut(&mut self) -> &mut dyn Resolver;
    fn as_any(&self) -> &dyn Any;
    fn as_chooser(&self) -> &dyn Chooser;
}

pub use simple::SimpleChooser;
// TODO: 提供一个 Default Chooser
