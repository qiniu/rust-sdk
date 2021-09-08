use super::{
    super::{super::regions::IpAddrWithPort, spawn::spawn},
    Chooser, ChooserFeedback,
};
use dashmap::DashMap;
use log::{info, warn};
use std::{
    any::Any,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

type BlacklistKey = IpAddrWithPort;

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
const DEFAULT_MIN_SHRINK_INTERVAL: Duration = Duration::from_secs(120);

#[derive(Debug, Clone)]
pub struct IpChooser {
    inner: Arc<IpChooserInner>,
}

#[derive(Debug, Default)]
struct IpChooserInner {
    blacklist: Blacklist,
    lock: Mutex<LockedData>,
    block_duration: Duration,
    min_shrink_interval: Duration,
}

impl Default for IpChooser {
    #[inline]
    fn default() -> Self {
        Self {
            inner: Arc::new(IpChooserInner {
                blacklist: Default::default(),
                lock: Default::default(),
                block_duration: DEFAULT_BLOCK_DURATION,
                min_shrink_interval: DEFAULT_MIN_SHRINK_INTERVAL,
            }),
        }
    }
}

impl IpChooser {
    #[inline]
    pub fn builder() -> IpChooserBuilder {
        IpChooserBuilder {
            inner: IpChooserInner {
                blacklist: Default::default(),
                lock: Default::default(),
                block_duration: DEFAULT_BLOCK_DURATION,
                min_shrink_interval: DEFAULT_MIN_SHRINK_INTERVAL,
            },
        }
    }
}

impl Chooser for IpChooser {
    #[inline]
    fn choose(&self, ips: &[IpAddrWithPort]) -> Vec<IpAddrWithPort> {
        let mut need_to_shrink = false;
        let filtered_ips: Vec<_> = ips
            .to_vec()
            .into_iter()
            .filter(|&ip| {
                self.inner
                    .blacklist
                    .get(&BlacklistKey::from(ip))
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
        filtered_ips
    }

    fn feedback(&self, feedback: ChooserFeedback) {
        if feedback.error().is_some() {
            for &ip in feedback.ips().iter() {
                self.inner.blacklist.insert(
                    ip,
                    BlacklistValue {
                        blocked_at: Instant::now(),
                    },
                );
            }
        } else {
            for ip in feedback.ips().iter() {
                self.inner.blacklist.remove(ip);
            }
        }
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

fn do_some_work_async(inner: &Arc<IpChooserInner>, need_to_shrink: bool) {
    if need_to_shrink && is_time_to_shrink(inner) {
        let cloned = inner.to_owned();
        if let Err(err) = spawn(
            "qiniu.rust-sdk.http-client.chooser.IpChooser".into(),
            move || {
                if is_time_to_shrink_mut(&cloned) {
                    info!("Ip Chooser spawns thread to do some housework");
                    shrink_cache(&cloned.blacklist, cloned.block_duration);
                }
            },
        ) {
            warn!(
                "Ip Chooser was failed to spawn thread to do some housework: {}",
                err
            );
        }
    }

    return;

    #[inline]
    fn is_time_to_shrink(inner: &Arc<IpChooserInner>) -> bool {
        if let Ok(locked_data) = inner.lock.try_lock() {
            _is_time_to_shrink(inner.min_shrink_interval, &*locked_data)
        } else {
            false
        }
    }

    #[inline]
    fn is_time_to_shrink_mut(inner: &Arc<IpChooserInner>) -> bool {
        if let Ok(mut locked_data) = inner.lock.try_lock() {
            if _is_time_to_shrink(inner.min_shrink_interval, &*locked_data) {
                locked_data.last_shrink_at = Instant::now();
                return true;
            }
        }
        false
    }

    #[inline]
    fn _is_time_to_shrink(min_shrink_interval: Duration, locked_data: &LockedData) -> bool {
        locked_data.last_shrink_at.elapsed() >= min_shrink_interval
    }

    #[inline]
    fn shrink_cache(blacklist: &Blacklist, block_duration: Duration) {
        let old_size = blacklist.len();
        blacklist.retain(|_, value| value.blocked_at.elapsed() >= block_duration);
        let new_size = blacklist.len();
        info!(
            "Blacklist is shrunken, from {} to {} entries",
            old_size, new_size
        );
    }
}

#[derive(Debug)]
pub struct IpChooserBuilder {
    inner: IpChooserInner,
}

impl IpChooserBuilder {
    #[inline]
    pub fn block_duration(mut self, block_duration: Duration) -> Self {
        self.inner.block_duration = block_duration;
        self
    }

    #[inline]
    pub fn min_shrink_interval(mut self, min_shrink_interval: Duration) -> Self {
        self.inner.min_shrink_interval = min_shrink_interval;
        self
    }

    #[inline]
    pub fn build(self) -> IpChooser {
        IpChooser {
            inner: Arc::new(self.inner),
        }
    }
}

#[cfg(all(test, foo))]
mod tests {
    use super::{
        super::super::{ResolveResult, ResponseError, ResponseErrorKind, RetriedStatsInfo},
        *,
    };
    use std::{
        collections::HashMap,
        error::Error,
        net::{IpAddr, Ipv4Addr},
        result::Result,
        thread::sleep,
    };

    #[derive(Debug, Clone, Default)]
    struct ResolverFromTable {
        table: HashMap<Box<str>, Box<[IpAddr]>>,
    }

    impl ResolverFromTable {
        fn add(&mut self, domain: impl Into<String>, ip_addrs: Vec<IpAddr>) {
            self.table
                .insert(domain.into().into_boxed_str(), ip_addrs.into_boxed_slice());
        }
    }

    impl Resolver for ResolverFromTable {
        #[inline]
        fn resolve(&self, domain: &str) -> ResolveResult {
            let key = domain.to_owned().into_boxed_str();
            Ok(self
                .table
                .get(&key)
                .cloned()
                .unwrap_or(vec![].into_boxed_slice()))
        }

        #[inline]
        fn as_any(&self) -> &dyn Any {
            self
        }

        #[inline]
        fn as_resolver(&self) -> &dyn Resolver {
            self
        }
    }

    #[test]
    fn test_simple_chooser() -> Result<(), Box<dyn Error>> {
        let mut backend = ResolverFromTable::default();
        backend.add(
            "test_domain_1.com",
            vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)),
            ],
        );
        backend.add(
            "test_domain_2.com",
            vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 2, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 2, 2)),
            ],
        );
        backend.add("test_domain_3.com", vec![]);
        let chooser = IpChooser::new(backend, Duration::from_secs(30));

        assert_eq!(
            chooser.choose(&DomainWithPort::new("test_domain_1.com"), false),
            ChosenResult::IPs(vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into(),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)).into(),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)).into(),
            ])
        );

        chooser.feedback(ChooserFeedback::new(
            &DomainOrIpAddr::new_from_domain(
                DomainWithPort::new("test_domain_1.com"),
                vec![Ipv4Addr::new(192, 168, 1, 3).into()],
            ),
            &RetriedStatsInfo::default(),
            Err(&ResponseError::new(
                ResponseErrorKind::ParseResponseError,
                "Test Error",
            )),
        ));
        assert_eq!(
            chooser.choose(&DomainWithPort::new("test_domain_1.com"), false),
            ChosenResult::IPs(vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into(),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)).into(),
            ])
        );
        assert_eq!(
            chooser.choose(&DomainWithPort::new("test_domain_1.com"), true),
            ChosenResult::IPs(vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into(),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)).into(),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)).into(),
            ])
        );

        chooser.feedback(ChooserFeedback::new(
            &DomainOrIpAddr::new_from_domain(
                DomainWithPort::new("test_domain_1.com"),
                vec![
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into(),
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)).into(),
                ],
            ),
            &RetriedStatsInfo::default(),
            Err(&ResponseError::new(
                ResponseErrorKind::ParseResponseError,
                "Test Error",
            )),
        ));
        assert_eq!(
            chooser.choose(&DomainWithPort::new("test_domain_1.com"), false),
            ChosenResult::TryAnotherDomain,
        );

        chooser.feedback(ChooserFeedback::new(
            &DomainOrIpAddr::new_from_domain(DomainWithPort::new("test_domain_2.com"), vec![]),
            &RetriedStatsInfo::default(),
            Err(&ResponseError::new(
                ResponseErrorKind::ParseResponseError,
                "Test Error",
            )),
        ));
        assert_eq!(
            chooser.choose(&DomainWithPort::new("test_domain_2.com"), false),
            ChosenResult::TryAnotherDomain,
        );

        assert_eq!(
            chooser.choose(&DomainWithPort::new("test_domain_3.com"), false),
            ChosenResult::UseThisDomainDirectly,
        );

        Ok(())
    }

    #[test]
    fn test_simple_chooser_expiration() -> Result<(), Box<dyn Error>> {
        let mut backend = ResolverFromTable::default();
        backend.add(
            "test_domain_1.com",
            vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)),
            ],
        );
        let chooser = IpChooser::new(backend, Duration::from_secs(1));
        assert_eq!(
            chooser.choose(&DomainWithPort::new("test_domain_1.com"), false),
            ChosenResult::IPs(vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into(),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)).into(),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)).into(),
            ])
        );

        chooser.feedback(ChooserFeedback::new(
            &DomainOrIpAddr::new_from_domain(
                DomainWithPort::new("test_domain_1.com"),
                vec![
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into(),
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)).into(),
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)).into(),
                ],
            ),
            &RetriedStatsInfo::default(),
            Err(&ResponseError::new(
                ResponseErrorKind::ParseResponseError,
                "Test Error",
            )),
        ));
        assert_eq!(
            chooser.choose(&DomainWithPort::new("test_domain_1.com"), false),
            ChosenResult::TryAnotherDomain,
        );

        sleep(Duration::from_secs(1));

        assert_eq!(
            chooser.choose(&DomainWithPort::new("test_domain_1.com"), false),
            ChosenResult::IPs(vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into(),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)).into(),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)).into(),
            ])
        );

        chooser.feedback(ChooserFeedback::new(
            &DomainOrIpAddr::new_from_domain(
                DomainWithPort::new("test_domain_1.com"),
                vec![
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into(),
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)).into(),
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)).into(),
                ],
            ),
            &RetriedStatsInfo::default(),
            Err(&ResponseError::new(
                ResponseErrorKind::ParseResponseError,
                "Test Error",
            )),
        ));
        assert_eq!(
            chooser.choose(&DomainWithPort::new("test_domain_1.com"), false),
            ChosenResult::TryAnotherDomain,
        );

        chooser.feedback(ChooserFeedback::new(
            &DomainOrIpAddr::new_from_domain(
                DomainWithPort::new("test_domain_1.com"),
                vec![
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into(),
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)).into(),
                ],
            ),
            &RetriedStatsInfo::default(),
            Ok(Default::default()),
        ));
        assert_eq!(
            chooser.choose(&DomainWithPort::new("test_domain_1.com"), false),
            ChosenResult::IPs(vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into(),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)).into(),
            ])
        );

        Ok(())
    }
}
