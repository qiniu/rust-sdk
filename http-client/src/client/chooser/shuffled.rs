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

#[cfg(test)]
mod tests {
    use super::{
        super::{
            super::{ResponseError, ResponseErrorKind, RetriedStatsInfo},
            IpChooser,
        },
        *,
    };
    use std::{
        collections::HashSet,
        net::{IpAddr, Ipv4Addr},
    };

    const IPS_WITHOUT_PORT: &[IpAddrWithPort] = &[
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)), None),
    ];

    #[test]
    fn test_shuffled_chooser() {
        env_logger::builder().is_test(true).try_init().ok();

        let ip_chooser: ShuffledChooser<IpChooser> = Default::default();
        assert_eq!(
            make_set(ip_chooser.choose(IPS_WITHOUT_PORT)),
            make_set(IPS_WITHOUT_PORT)
        );
        ip_chooser.feedback(ChooserFeedback::new(
            &[
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
            ],
            &RetriedStatsInfo::default(),
            None,
            Some(&ResponseError::new(
                ResponseErrorKind::ParseResponseError,
                "Test Error",
            )),
        ));
        assert_eq!(
            make_set(ip_chooser.choose(IPS_WITHOUT_PORT)),
            make_set(&[IpAddrWithPort::new(
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)),
                None
            )]),
        );

        ip_chooser.feedback(ChooserFeedback::new(
            IPS_WITHOUT_PORT,
            &RetriedStatsInfo::default(),
            None,
            Some(&ResponseError::new(
                ResponseErrorKind::ParseResponseError,
                "Test Error",
            )),
        ));
        assert_eq!(ip_chooser.choose(IPS_WITHOUT_PORT), vec![]);

        ip_chooser.feedback(ChooserFeedback::new(
            &[
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
            ],
            &RetriedStatsInfo::default(),
            None,
            None,
        ));
        assert_eq!(
            make_set(ip_chooser.choose(IPS_WITHOUT_PORT)),
            make_set(&[
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
            ])
        );
    }

    #[inline]
    fn make_set(ips: impl AsRef<[IpAddrWithPort]>) -> HashSet<IpAddrWithPort> {
        let mut h = HashSet::new();
        h.extend(ips.as_ref());
        h
    }
}
