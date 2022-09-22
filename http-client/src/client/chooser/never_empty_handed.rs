use super::{super::super::regions::IpAddrWithPort, ChooseOptions, Chooser, ChooserFeedback, ChosenResults};
use num_rational::Ratio;
use rand::{prelude::*, thread_rng};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

const DEFAULT_RANDOM_CHOOSE_RATIO: Ratio<usize> = Ratio::new_raw(1, 2);

/// 永不空手的选择器
///
/// 确保 [`Chooser`] 实例不会因为所有可选择的 IP 地址都被屏蔽而导致 HTTP 客户端直接返回错误，
/// 在内置的 [`Chooser`] 没有返回结果时，将会随机返回一定比例的 IP 地址供 HTTP 客户端做一轮尝试。
#[derive(Debug, Clone)]
pub struct NeverEmptyHandedChooser<C: ?Sized> {
    random_choose_ratio: Ratio<usize>,
    inner_chooser: C,
}

impl<C> NeverEmptyHandedChooser<C> {
    /// 创建永不空手的选择器
    ///
    /// 需要提供在所有 IP 地址都被屏蔽的情况下，随机返回的 IP 地址的比例
    ///
    /// 需要注意，提供的随机比例的分母必须大于 0，且比值小于 1。
    #[inline]
    pub fn new(chooser: C, random_choose_ratio: Ratio<usize>) -> Self {
        assert!(random_choose_ratio.numer() <= random_choose_ratio.denom());
        assert!(*random_choose_ratio.denom() > 0);
        Self {
            inner_chooser: chooser,
            random_choose_ratio,
        }
    }
}

impl<C: Default> Default for NeverEmptyHandedChooser<C> {
    #[inline]
    fn default() -> Self {
        Self::new(Default::default(), DEFAULT_RANDOM_CHOOSE_RATIO)
    }
}

impl<C: Chooser + Clone> Chooser for NeverEmptyHandedChooser<C> {
    fn choose(&self, ips: &[IpAddrWithPort], opts: ChooseOptions) -> ChosenResults {
        let chosen = self.inner_chooser.choose(ips, opts);
        if chosen.is_empty() {
            self.random_choose(ips).into()
        } else {
            chosen
        }
    }

    #[inline]
    fn feedback(&self, feedback: ChooserFeedback) {
        self.inner_chooser.feedback(feedback)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_choose<'a>(&'a self, ips: &'a [IpAddrWithPort], opts: ChooseOptions<'a>) -> BoxFuture<'a, ChosenResults> {
        Box::pin(async move {
            let chosen = self.inner_chooser.async_choose(ips, opts).await;
            if chosen.is_empty() {
                self.random_choose(ips).into()
            } else {
                chosen
            }
        })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_feedback<'a>(&'a self, feedback: ChooserFeedback<'a>) -> BoxFuture<'a, ()> {
        self.inner_chooser.async_feedback(feedback)
    }
}

impl<C> NeverEmptyHandedChooser<C> {
    fn random_choose(&self, ips: &[IpAddrWithPort]) -> Vec<IpAddrWithPort> {
        let chosen_len = (self.random_choose_ratio * ips.len()).ceil().to_integer();
        ips.choose_multiple(&mut thread_rng(), chosen_len).copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{
            super::{ResponseError, ResponseErrorKind},
            IpChooser,
        },
        *,
    };
    use std::net::{IpAddr, Ipv4Addr};

    const IPS_WITHOUT_PORT: &[IpAddrWithPort] = &[
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)), None),
    ];

    #[test]
    fn test_never_empty_handed_chooser() {
        env_logger::builder().is_test(true).try_init().ok();

        let ip_chooser: NeverEmptyHandedChooser<IpChooser> = Default::default();
        assert_eq!(
            ip_chooser.choose(IPS_WITHOUT_PORT, Default::default()).into_ip_addrs(),
            IPS_WITHOUT_PORT.to_vec()
        );
        ip_chooser.feedback(
            ChooserFeedback::builder(&[
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
            ])
            .error(&ResponseError::new_with_msg(
                ResponseErrorKind::ParseResponseError,
                "Test Error",
            ))
            .build(),
        );
        assert_eq!(
            ip_chooser.choose(IPS_WITHOUT_PORT, Default::default()).into_ip_addrs(),
            [IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)), None)].to_vec(),
        );

        ip_chooser.feedback(
            ChooserFeedback::builder(IPS_WITHOUT_PORT)
                .error(&ResponseError::new_with_msg(
                    ResponseErrorKind::ParseResponseError,
                    "Test Error",
                ))
                .build(),
        );

        assert_eq!(
            ip_chooser
                .choose(IPS_WITHOUT_PORT, Default::default())
                .into_ip_addrs()
                .len(),
            2
        );
    }
}
