use super::{
    super::{
        super::regions::{DomainWithPort, IpAddrWithPort},
        Resolver, ResponseError,
    },
    Chooser, ChosenResult,
};
use rand::{seq::SliceRandom, thread_rng};
use std::any::Any;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug, Clone)]
pub struct ShuffledChooser<C: Chooser> {
    chooser: C,
    shuffle_resolved_ips: bool,
    shuffle_given_ips: bool,
}

impl<C: Chooser> ShuffledChooser<C> {
    #[inline]
    fn new(chooser: C, shuffle_resolved_ips: bool, shuffle_given_ips: bool) -> Self {
        Self {
            chooser,
            shuffle_resolved_ips,
            shuffle_given_ips,
        }
    }

    #[inline]
    pub fn builder(chooser: C) -> ShuffledChooserBuilder<C> {
        ShuffledChooserBuilder {
            inner: Self::new(chooser, false, true),
        }
    }
}

impl<C: Chooser + Default> Default for ShuffledChooser<C> {
    #[inline]
    fn default() -> Self {
        Self::builder(C::default()).build()
    }
}

impl<C: Chooser> Chooser for ShuffledChooser<C> {
    fn choose(&self, domain: &DomainWithPort, ignore_frozen: bool) -> ChosenResult {
        match self.chooser.choose(domain, ignore_frozen) {
            ChosenResult::IPs(mut ips) => {
                if self.shuffle_resolved_ips {
                    ips.shuffle(&mut thread_rng());
                }
                ChosenResult::IPs(ips)
            }
            result => result,
        }
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_choose<'a>(
        &'a self,
        domain: &'a DomainWithPort,
        ignore_frozen: bool,
    ) -> BoxFuture<'a, ChosenResult> {
        Box::pin(async move {
            match self.chooser.async_choose(domain, ignore_frozen).await {
                ChosenResult::IPs(mut ips) => {
                    if self.shuffle_resolved_ips {
                        ips.shuffle(&mut thread_rng());
                    }
                    ChosenResult::IPs(ips)
                }
                result => result,
            }
        })
    }

    #[inline]
    fn choose_ips(&self, ips: &[IpAddrWithPort]) -> ChosenResult {
        match self.chooser.choose_ips(ips) {
            ChosenResult::IPs(mut ips) => {
                if self.shuffle_given_ips {
                    ips.shuffle(&mut thread_rng());
                }
                ChosenResult::IPs(ips)
            }
            result => result,
        }
    }

    #[inline]
    fn freeze_domain(&self, domain: &DomainWithPort, error: &ResponseError) {
        self.chooser.freeze_domain(domain, error)
    }

    #[inline]
    fn freeze_ips(&self, ips: &[IpAddrWithPort], error: &ResponseError) {
        self.chooser.freeze_ips(ips, error)
    }

    #[inline]
    fn unfreeze_domain(&self, domain: &DomainWithPort) {
        self.chooser.unfreeze_domain(domain)
    }

    #[inline]
    fn unfreeze_ips(&self, ips: &[IpAddrWithPort]) {
        self.chooser.unfreeze_ips(ips)
    }

    #[inline]
    fn resolver(&self) -> &dyn Resolver {
        self.chooser.resolver()
    }

    #[inline]
    fn resolver_mut(&mut self) -> &mut dyn Resolver {
        self.chooser.resolver_mut()
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

#[derive(Debug, Clone, Default)]
pub struct ShuffledChooserBuilder<C: Chooser> {
    inner: ShuffledChooser<C>,
}

impl<C: Chooser> ShuffledChooserBuilder<C> {
    #[inline]
    pub fn shuffle_resolved_ips(mut self, shuffle_resolved_ips: bool) -> Self {
        self.inner.shuffle_resolved_ips = shuffle_resolved_ips;
        self
    }

    #[inline]
    pub fn shuffle_given_ips(mut self, shuffle_given_ips: bool) -> Self {
        self.inner.shuffle_given_ips = shuffle_given_ips;
        self
    }

    #[inline]
    pub fn build(self) -> ShuffledChooser<C> {
        self.inner
    }
}
