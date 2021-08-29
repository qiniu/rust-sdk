use super::{Chooser, ChooserFeedback, IpAddrWithPort};
use std::any::Any;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Clone, Copy, Debug, Default)]
pub struct DirectChooser;

impl Chooser for DirectChooser {
    #[inline]
    fn choose(&self, ips: &[IpAddrWithPort]) -> Vec<IpAddrWithPort> {
        ips.to_owned()
    }

    #[inline]
    fn feedback(&self, _feedback: ChooserFeedback) {}

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_choose<'a>(&'a self, ips: &'a [IpAddrWithPort]) -> BoxFuture<'a, Vec<IpAddrWithPort>> {
        Box::pin(async move { self.choose(ips) })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_feedback<'a>(&'a self, feedback: ChooserFeedback<'a>) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.feedback(feedback) })
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_chooser(&self) -> &dyn Chooser {
        self
    }
}
