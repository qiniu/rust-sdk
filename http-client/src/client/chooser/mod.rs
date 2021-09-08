mod direct;
mod feedback;
mod ip;
mod never_choose_none;
mod shuffled;
mod subnet;

use super::super::regions::IpAddrWithPort;
pub use feedback::ChooserFeedback;
use std::{any::Any, fmt::Debug};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

pub trait Chooser: Any + Debug + Sync + Send {
    fn choose(&self, ips: &[IpAddrWithPort]) -> Vec<IpAddrWithPort>;
    fn feedback(&self, feedback: ChooserFeedback);

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

    fn as_any(&self) -> &dyn Any;
    fn as_chooser(&self) -> &dyn Chooser;
}

pub use direct::DirectChooser;
pub use ip::{IpChooser, IpChooserBuilder};
pub use never_choose_none::NeverChooseNoneChooser;
pub use shuffled::ShuffledChooser;
pub use subnet::{SubnetChooser, SubnetChooserBuilder};
