use super::{super::super::regions::IpAddrWithPort, Chooser, ChooserFeedback};
use dashmap::DashMap;
use log::{info, warn};
use std::{
    any::Any,
    sync::{Arc, Mutex},
    thread::Builder as ThreadBuilder,
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
const BLACKLIST_SIZE_TO_SHRINK: usize = 100;
const MIN_SHRINK_INTERVAL: Duration = Duration::from_secs(120);

#[derive(Debug, Clone)]
pub struct SimpleChooser {
    inner: Arc<SimpleChooserInner>,
}

#[derive(Debug, Default)]
struct SimpleChooserInner {
    blacklist: Blacklist,
    lock: Mutex<LockedData>,
    block_duration: Duration,
}

impl SimpleChooser {
    #[inline]
    pub fn new(block_duration: Duration) -> Self {
        Self {
            inner: Arc::new(SimpleChooserInner {
                block_duration,
                blacklist: Default::default(),
                lock: Default::default(),
            }),
        }
    }
}

impl Default for SimpleChooser {
    #[inline]
    fn default() -> Self {
        Self::new(DEFAULT_BLOCK_DURATION)
    }
}

impl Chooser for SimpleChooser {
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

fn do_some_work_async(inner: &Arc<SimpleChooserInner>, need_to_shrink: bool) {
    if need_to_shrink && is_time_to_shrink(&inner.blacklist, &inner.lock) {
        let cloned = inner.to_owned();
        if let Err(err) = ThreadBuilder::new()
            .name("qiniu.rust-sdk.http-client.chooser.SimpleChooser".into())
            .spawn(move || {
                if is_time_to_shrink_mut(&cloned.blacklist, &cloned.lock) {
                    info!("Simple Chooser spawns thread to do some housework");
                    shrink_cache(&cloned.blacklist, cloned.block_duration);
                }
            })
        {
            warn!(
                "Simple Chooser was failed to spawn thread to do some housework: {}",
                err
            );
        }
    }

    return;

    #[inline]
    fn is_time_to_shrink(blacklist: &Blacklist, locked_data: &Mutex<LockedData>) -> bool {
        if let Ok(locked_data) = locked_data.try_lock() {
            _is_time_to_shrink(blacklist, &*locked_data)
        } else {
            false
        }
    }

    #[inline]
    fn is_time_to_shrink_mut(blacklist: &Blacklist, locked_data: &Mutex<LockedData>) -> bool {
        if let Ok(mut locked_data) = locked_data.try_lock() {
            if _is_time_to_shrink(blacklist, &*locked_data) {
                locked_data.last_shrink_at = Instant::now();
                return true;
            }
        }
        false
    }

    #[inline]
    fn _is_time_to_shrink(blacklist: &Blacklist, locked_data: &LockedData) -> bool {
        locked_data.last_shrink_at.elapsed() >= MIN_SHRINK_INTERVAL
            && blacklist.len() >= BLACKLIST_SIZE_TO_SHRINK
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
        let chooser = SimpleChooser::new(backend, Duration::from_secs(30));

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
        let chooser = SimpleChooser::new(backend, Duration::from_secs(1));
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
