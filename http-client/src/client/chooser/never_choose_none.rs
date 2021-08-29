use super::{super::super::regions::IpAddrWithPort, Chooser, ChooserFeedback};
use num_rational::Ratio;
use rand::{prelude::*, thread_rng};
use std::any::Any;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug, Clone)]
pub struct NeverChooseNoneChooser<C: Chooser> {
    inner_chooser: C,
    random_choose_ratio: Ratio<usize>,
}

impl<C: Chooser + Default> Default for NeverChooseNoneChooser<C> {
    fn default() -> Self {
        Self {
            inner_chooser: Default::default(),
            random_choose_ratio: Ratio::new(1, 2),
        }
    }
}

impl<C: Chooser> Chooser for NeverChooseNoneChooser<C> {
    #[inline]
    fn choose(&self, ips: &[IpAddrWithPort]) -> Vec<IpAddrWithPort> {
        let chosen = self.inner_chooser.choose(ips);
        if chosen.is_empty() {
            self.random_choose(ips)
        } else {
            chosen
        }
    }

    #[inline]
    fn feedback(&self, feedback: ChooserFeedback) {
        self.inner_chooser.feedback(feedback)
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_choose<'a>(&'a self, ips: &'a [IpAddrWithPort]) -> BoxFuture<'a, Vec<IpAddrWithPort>> {
        Box::pin(async move {
            let chosen = self.inner_chooser.async_choose(ips).await;
            if chosen.is_empty() {
                self.random_choose(ips)
            } else {
                chosen
            }
        })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_feedback<'a>(&'a self, feedback: ChooserFeedback<'a>) -> BoxFuture<'a, ()> {
        self.inner_chooser.async_feedback(feedback)
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

impl<C: Chooser> NeverChooseNoneChooser<C> {
    #[inline]
    fn random_choose(&self, ips: &[IpAddrWithPort]) -> Vec<IpAddrWithPort> {
        let chosen_len = (self.random_choose_ratio * ips.len()).to_integer();
        ips.choose_multiple(&mut thread_rng(), chosen_len)
            .cloned()
            .collect()
    }
}
