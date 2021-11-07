use super::{ChooseOptions, Chooser, ChooserFeedback, ChosenResults, IpAddrWithPort};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Clone, Copy, Debug, Default)]
pub struct DirectChooser;

impl Chooser for DirectChooser {
    #[inline]
    fn choose(&self, ips: &[IpAddrWithPort], _opts: &ChooseOptions) -> ChosenResults {
        ips.to_owned().into()
    }

    #[inline]
    fn feedback(&self, _feedback: ChooserFeedback) {}

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_choose<'a>(
        &'a self,
        ips: &'a [IpAddrWithPort],
        opts: &'a ChooseOptions,
    ) -> BoxFuture<'a, ChosenResults> {
        Box::pin(async move { self.choose(ips, opts) })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_feedback<'a>(&'a self, feedback: ChooserFeedback<'a>) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.feedback(feedback) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use once_cell::sync::Lazy;
    use std::{
        net::{IpAddr, Ipv4Addr},
        num::NonZeroU16,
    };

    static IPS_WITHOUT_PORT: Lazy<Vec<IpAddrWithPort>> = Lazy::new(|| {
        vec![
            IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
            IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
            IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)), None),
        ]
    });

    static IPS_WITH_PORT: Lazy<Vec<IpAddrWithPort>> = Lazy::new(|| {
        vec![
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
        ]
    });

    #[test]
    fn test_direct_chooser() -> Result<()> {
        let chooser = DirectChooser;
        assert_eq!(
            chooser
                .choose(&IPS_WITHOUT_PORT, &Default::default())
                .into_ip_addrs(),
            IPS_WITHOUT_PORT.to_vec(),
        );
        assert_eq!(
            chooser
                .choose(&IPS_WITH_PORT, &Default::default())
                .into_ip_addrs(),
            IPS_WITH_PORT.to_vec(),
        );
        Ok(())
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_test_direct_chooser() -> Result<()> {
        let chooser = DirectChooser;
        assert_eq!(
            chooser
                .async_choose(&IPS_WITHOUT_PORT, &Default::default())
                .await
                .into_ip_addrs(),
            IPS_WITHOUT_PORT.to_vec(),
        );
        assert_eq!(
            chooser
                .async_choose(&IPS_WITH_PORT, &Default::default())
                .await
                .into_ip_addrs(),
            IPS_WITH_PORT.to_vec(),
        );
        Ok(())
    }
}
