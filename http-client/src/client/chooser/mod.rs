mod simple;

use super::Resolver;
use qiniu_http::ResponseError;
use std::{any::Any, fmt::Debug, net::IpAddr};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

pub enum ChosenResult {
    IPs(Vec<IpAddr>),
    UseThisDomainDirectly,
    TryAnotherDomain,
}

pub trait Chooser: Any + Debug + Sync + Send {
    fn choose(&self, domain: &str) -> ChosenResult;

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_choose<'a>(&'a self, domain: &'a str) -> BoxFuture<'a, ChosenResult> {
        Box::pin(async move { self.choose(domain) })
    }

    fn mark_as_failed(&self, domain: &str, ip: IpAddr, error: ResponseError);
    fn resolver(&self) -> &dyn Resolver;
    fn resolver_mut(&mut self) -> &mut dyn Resolver;
    fn as_any(&self) -> &dyn Any;
    fn as_chooser(&self) -> &dyn Chooser;
}

pub use simple::SimpleChooser;
// TODO: 提供一个 Default Chooser
