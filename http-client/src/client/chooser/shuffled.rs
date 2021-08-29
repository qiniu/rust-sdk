use super::{super::super::regions::IpAddrWithPort, Chooser, ChooserFeedback};
use rand::{seq::SliceRandom, thread_rng};
use std::any::Any;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug, Clone)]
pub struct ShuffledChooser<C: Chooser> {
    chooser: C,
}

impl<C: Chooser> ShuffledChooser<C> {
    #[inline]
    pub fn new(chooser: C) -> Self {
        Self { chooser }
    }
}

impl<C: Chooser + Default> Default for ShuffledChooser<C> {
    #[inline]
    fn default() -> Self {
        Self::new(C::default())
    }
}

impl<C: Chooser> Chooser for ShuffledChooser<C> {
    #[inline]
    fn choose(&self, ips: &[IpAddrWithPort]) -> Vec<IpAddrWithPort> {
        let mut ips = self.chooser.choose(ips);
        ips.shuffle(&mut thread_rng());
        ips
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_choose<'a>(&'a self, ips: &'a [IpAddrWithPort]) -> BoxFuture<'a, Vec<IpAddrWithPort>> {
        Box::pin(async move {
            let mut ips = self.chooser.async_choose(ips).await;
            ips.shuffle(&mut thread_rng());
            ips
        })
    }

    #[inline]
    fn feedback(&self, feedback: ChooserFeedback) {
        self.chooser.feedback(feedback)
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_feedback<'a>(&'a self, feedback: ChooserFeedback<'a>) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.chooser.async_feedback(feedback).await })
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
