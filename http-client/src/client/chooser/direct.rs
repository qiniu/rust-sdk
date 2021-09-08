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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::{
        net::{IpAddr, Ipv4Addr},
        num::NonZeroU16,
    };

    const IPS_WITHOUT_PORT: &[IpAddrWithPort] = &[
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)), None),
    ];
    const IPS_WITH_PORT: &[IpAddrWithPort] = &[
        IpAddrWithPort::new(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            NonZeroU16::new(443),
        ),
        IpAddrWithPort::new(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            NonZeroU16::new(443),
        ),
        IpAddrWithPort::new(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)),
            NonZeroU16::new(443),
        ),
    ];

    #[test]
    fn test_direct_chooser() -> Result<()> {
        let chooser = DirectChooser;
        assert_eq!(chooser.choose(IPS_WITHOUT_PORT), IPS_WITHOUT_PORT.to_vec(),);
        assert_eq!(chooser.choose(IPS_WITH_PORT), IPS_WITH_PORT.to_vec(),);
        Ok(())
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_test_direct_chooser() -> Result<()> {
        let chooser = DirectChooser;
        assert_eq!(
            chooser.async_choose(IPS_WITHOUT_PORT).await,
            IPS_WITHOUT_PORT.to_vec(),
        );
        assert_eq!(
            chooser.async_choose(IPS_WITH_PORT).await,
            IPS_WITH_PORT.to_vec(),
        );
        Ok(())
    }
}
