use super::{
    super::super::{
        regions::{DomainWithPort, IpAddrWithPort},
        spawn::spawn,
    },
    ChooseOptions, Chooser, ChooserFeedback, ChosenResults,
};
use dashmap::DashMap;
use log::{info, warn};
use std::{
    mem::take,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BlacklistKey {
    ip: IpAddrWithPort,
    domain: Option<DomainWithPort>,
}

#[derive(Debug, Clone)]
struct BlacklistValue {
    blocked_at: Instant,
}

type Blacklist = DashMap<BlacklistKey, BlacklistValue>;

#[derive(Debug, Clone)]
struct LockedData {
    last_shrink_at: Instant,
}

impl Default for LockedData {
    #[inline]
    fn default() -> Self {
        LockedData {
            last_shrink_at: Instant::now(),
        }
    }
}

const DEFAULT_BLOCK_DURATION: Duration = Duration::from_secs(30);
const DEFAULT_SHRINK_INTERVAL: Duration = Duration::from_secs(120);

/// IP 地址选择器
///
/// 包含 IP 地址黑名单，一旦被反馈 API 调用失败，则将所有相关 IP 地址冻结一段时间
#[derive(Debug, Clone)]
pub struct IpChooser {
    inner: Arc<IpChooserInner>,
}

#[derive(Debug, Default)]
struct IpChooserInner {
    blacklist: Blacklist,
    lock: Mutex<LockedData>,
    block_duration: Duration,
    shrink_interval: Duration,
}

impl Default for IpChooser {
    #[inline]
    fn default() -> Self {
        Self::builder().build()
    }
}

impl IpChooser {
    /// 创建 IP 地址选择构建器
    #[inline]
    pub fn builder() -> IpChooserBuilder {
        Default::default()
    }
}

impl Chooser for IpChooser {
    fn choose(&self, ips: &[IpAddrWithPort], opts: ChooseOptions) -> ChosenResults {
        let mut need_to_shrink = false;
        let filtered_ips: Vec<_> = ips
            .iter()
            .copied()
            .filter(|&ip| {
                self.inner
                    .blacklist
                    .get(&BlacklistKey {
                        ip,
                        domain: opts.domain().cloned(),
                    })
                    .map_or(true, |r| {
                        if r.value().blocked_at.elapsed() < self.inner.block_duration {
                            false
                        } else {
                            need_to_shrink = true;
                            true
                        }
                    })
            })
            .collect();
        do_some_work_async(&self.inner, need_to_shrink);
        filtered_ips.into()
    }

    fn feedback(&self, feedback: ChooserFeedback) {
        if feedback.error().is_some() {
            for &ip in feedback.ips().iter() {
                self.inner.blacklist.insert(
                    BlacklistKey {
                        ip,
                        domain: feedback.domain().cloned(),
                    },
                    BlacklistValue {
                        blocked_at: Instant::now(),
                    },
                );
            }
        } else {
            for &ip in feedback.ips().iter() {
                self.inner.blacklist.remove(&BlacklistKey {
                    ip,
                    domain: feedback.domain().cloned(),
                });
            }
        }
    }
}

impl IpChooser {
    #[allow(dead_code)]
    fn len(&self) -> usize {
        self.inner.blacklist.len()
    }
}

fn do_some_work_async(inner: &Arc<IpChooserInner>, need_to_shrink: bool) {
    if need_to_shrink && is_time_to_shrink(inner) {
        let cloned = inner.to_owned();
        if let Err(err) = spawn("qiniu.rust-sdk.http-client.chooser.IpChooser".into(), move || {
            if is_time_to_shrink_mut(&cloned) {
                info!("Ip Chooser spawns thread to do some housework");
                shrink_cache(&cloned.blacklist, cloned.block_duration);
            }
        }) {
            warn!("Ip Chooser was failed to spawn thread to do some housework: {}", err);
        }
    }

    return;

    fn is_time_to_shrink(inner: &Arc<IpChooserInner>) -> bool {
        if let Ok(locked_data) = inner.lock.try_lock() {
            _is_time_to_shrink(inner.shrink_interval, &locked_data)
        } else {
            false
        }
    }

    fn is_time_to_shrink_mut(inner: &Arc<IpChooserInner>) -> bool {
        if let Ok(mut locked_data) = inner.lock.try_lock() {
            if _is_time_to_shrink(inner.shrink_interval, &locked_data) {
                locked_data.last_shrink_at = Instant::now();
                return true;
            }
        }
        false
    }

    fn _is_time_to_shrink(shrink_interval: Duration, locked_data: &LockedData) -> bool {
        locked_data.last_shrink_at.elapsed() >= shrink_interval
    }

    fn shrink_cache(blacklist: &Blacklist, block_duration: Duration) {
        let old_size = blacklist.len();
        blacklist.retain(|_, value| value.blocked_at.elapsed() < block_duration);
        let new_size = blacklist.len();
        info!("Blacklist is shrunken, from {} to {} entries", old_size, new_size);
    }
}

/// IP 地址选择构建器
#[derive(Debug)]
pub struct IpChooserBuilder {
    inner: IpChooserInner,
}

impl Default for IpChooserBuilder {
    #[inline]
    fn default() -> Self {
        Self {
            inner: IpChooserInner {
                blacklist: Default::default(),
                lock: Default::default(),
                block_duration: DEFAULT_BLOCK_DURATION,
                shrink_interval: DEFAULT_SHRINK_INTERVAL,
            },
        }
    }
}

impl IpChooserBuilder {
    /// 设置屏蔽时长
    #[inline]
    pub fn block_duration(&mut self, block_duration: Duration) -> &mut Self {
        self.inner.block_duration = block_duration;
        self
    }

    /// 设置清理间隔时长
    #[inline]
    pub fn shrink_interval(&mut self, shrink_interval: Duration) -> &mut Self {
        self.inner.shrink_interval = shrink_interval;
        self
    }

    /// 构建 IP 地址选择器
    #[inline]
    pub fn build(&mut self) -> IpChooser {
        IpChooser {
            inner: Arc::new(take(&mut self.inner)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::super::{ChooseOptionsBuilder, ResponseError, ResponseErrorKind},
        *,
    };
    use std::net::{IpAddr, Ipv4Addr};

    const IPS_WITHOUT_PORT: &[IpAddrWithPort] = &[
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)), None),
    ];

    #[test]
    fn test_ip_chooser() {
        env_logger::builder().is_test(true).try_init().ok();

        let ip_chooser = IpChooser::default();
        let domain = DomainWithPort::new("fakedomain", None);
        assert_eq!(
            ip_chooser
                .choose(IPS_WITHOUT_PORT, ChooseOptionsBuilder::new().domain(&domain).build())
                .into_ip_addrs(),
            IPS_WITHOUT_PORT.to_vec()
        );
        ip_chooser.feedback(
            ChooserFeedback::builder(&[
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
            ])
            .domain(&domain)
            .error(&ResponseError::new_with_msg(
                ResponseErrorKind::ParseResponseError,
                "Test Error",
            ))
            .build(),
        );
        assert_eq!(
            ip_chooser
                .choose(IPS_WITHOUT_PORT, ChooseOptionsBuilder::new().domain(&domain).build())
                .into_ip_addrs(),
            vec![IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)), None)],
        );

        ip_chooser.feedback(
            ChooserFeedback::builder(IPS_WITHOUT_PORT)
                .domain(&domain)
                .error(&ResponseError::new_with_msg(
                    ResponseErrorKind::ParseResponseError,
                    "Test Error",
                ))
                .build(),
        );
        assert_eq!(
            ip_chooser
                .choose(IPS_WITHOUT_PORT, ChooseOptionsBuilder::new().domain(&domain).build())
                .into_ip_addrs(),
            vec![]
        );

        ip_chooser.feedback(
            ChooserFeedback::builder(&[
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
            ])
            .domain(&domain)
            .build(),
        );
        assert_eq!(
            ip_chooser
                .choose(IPS_WITHOUT_PORT, ChooseOptionsBuilder::new().domain(&domain).build())
                .into_ip_addrs(),
            vec![
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
            ]
        );
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_ip_chooser_expiration_and_shrink() {
        use futures_timer::Delay as AsyncDelay;

        env_logger::builder().is_test(true).try_init().ok();

        let ip_chooser = IpChooser::builder()
            .block_duration(Duration::from_secs(1))
            .shrink_interval(Duration::from_millis(500))
            .build();

        assert_eq!(
            ip_chooser
                .async_choose(IPS_WITHOUT_PORT, Default::default())
                .await
                .into_ip_addrs(),
            IPS_WITHOUT_PORT.to_vec()
        );
        ip_chooser
            .async_feedback(
                ChooserFeedback::builder(&[
                    IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
                    IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
                ])
                .error(&ResponseError::new_with_msg(
                    ResponseErrorKind::ParseResponseError,
                    "Test Error",
                ))
                .build(),
            )
            .await;
        assert_eq!(
            ip_chooser
                .async_choose(IPS_WITHOUT_PORT, Default::default())
                .await
                .into_ip_addrs(),
            vec![IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)), None)],
        );

        AsyncDelay::new(Duration::from_secs(1)).await;
        assert_eq!(
            ip_chooser
                .async_choose(IPS_WITHOUT_PORT, Default::default())
                .await
                .into_ip_addrs(),
            IPS_WITHOUT_PORT.to_vec()
        );

        AsyncDelay::new(Duration::from_millis(500)).await;
        assert_eq!(ip_chooser.len(), 0);
    }
}
