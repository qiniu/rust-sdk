use super::{
    super::{Resolver, ResponseError},
    Chooser, ChosenResult,
};
use dashmap::DashMap;
use log::info;
use std::{
    any::Any,
    net::IpAddr,
    sync::{Arc, Mutex},
    thread::Builder as ThreadBuilder,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum BlacklistKey {
    Domain(Box<str>),
    IpAddr(IpAddr),
}
#[derive(Debug, Clone)]
struct BlacklistValue {
    block_until: SystemTime,
}

type Blacklist = DashMap<BlacklistKey, BlacklistValue>;

#[derive(Debug, Clone)]
struct LockedData {
    last_shrink_timestamp: SystemTime,
}

const DEFAULT_BLOCK_DURATION: Duration = Duration::from_secs(30);
const BLACKLIST_SIZE_TO_SHRINK: usize = 100;
const MIN_SHRINK_INTERVAL: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub struct SimpleChooser<R: Resolver> {
    resolver: R,
    inner: Arc<SimpleChooserInner>,
    block_duration: Duration,
}

#[derive(Debug, Default)]
struct SimpleChooserInner {
    blacklist: Blacklist,
    lock: Mutex<LockedData>,
}

impl<R: Resolver> SimpleChooser<R> {
    #[inline]
    pub fn new(resolver: R, block_duration: Duration) -> Self {
        Self {
            resolver,
            block_duration,
            inner: Default::default(),
        }
    }
}

impl<R: Resolver + Default> Default for SimpleChooser<R> {
    fn default() -> Self {
        Self::new(R::default(), DEFAULT_BLOCK_DURATION)
    }
}
macro_rules! choose {
    ($inner:expr, $domain:ident, $ignore_frozen:expr, $resolve:block) => {{
        if $ignore_frozen {
            return $resolve.map_or_else(
                |_| ChosenResult::UseThisDomainDirectly,
                |ips| ChosenResult::IPs(ips.into()),
            );
        }

        let mut need_to_shrink = false;
        let chosen_result = if $inner
            .blacklist
            .get(&BlacklistKey::Domain($domain.into()))
            .map_or(false, |r| {
                if r.value().block_until >= SystemTime::now() {
                    true
                } else {
                    need_to_shrink = true;
                    false
                }
            }) {
            ChosenResult::TryAnotherDomain
        } else {
            $resolve.map_or_else(
                |_| ChosenResult::UseThisDomainDirectly,
                |ips| {
                    if ips.is_empty() {
                        return ChosenResult::UseThisDomainDirectly;
                    } else {
                        let filtered_ips: Vec<_> = ips
                            .to_vec()
                            .into_iter()
                            .filter(|&ip| {
                                $inner
                                    .blacklist
                                    .get(&BlacklistKey::IpAddr(ip))
                                    .map_or(true, |r| {
                                        if r.value().block_until < SystemTime::now() {
                                            need_to_shrink = true;
                                            true
                                        } else {
                                            false
                                        }
                                    })
                            })
                            .collect();
                        if filtered_ips.is_empty() {
                            ChosenResult::TryAnotherDomain
                        } else {
                            ChosenResult::IPs(filtered_ips)
                        }
                    }
                },
            )
        };

        do_some_work_async(&$inner, need_to_shrink);

        chosen_result
    }};
}

impl<R: Resolver> Chooser for SimpleChooser<R> {
    fn choose(&self, domain: &str, ignore_frozen: bool) -> ChosenResult {
        choose!(self.inner, domain, ignore_frozen, {
            self.resolver.resolve(domain)
        })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_choose<'a>(
        &'a self,
        domain: &'a str,
        ignore_frozen: bool,
    ) -> BoxFuture<'a, ChosenResult> {
        Box::pin(async move {
            choose!(self.inner, domain, ignore_frozen, {
                self.resolver.async_resolve(domain).await
            })
        })
    }

    fn freeze(&self, domain: &str, ips: &[IpAddr], _error: ResponseError) {
        let block_until = SystemTime::now() + self.block_duration;
        if ips.is_empty() {
            self.inner.blacklist.insert(
                BlacklistKey::Domain(domain.into()),
                BlacklistValue { block_until },
            );
        } else {
            for &ip in ips.iter() {
                self.inner
                    .blacklist
                    .insert(BlacklistKey::IpAddr(ip), BlacklistValue { block_until });
            }
        }
    }

    fn unfreeze(&self, domain: &str, ips: &[IpAddr]) {
        self.inner
            .blacklist
            .remove(&BlacklistKey::Domain(domain.into()));
        for &ip in ips.iter() {
            self.inner.blacklist.remove(&BlacklistKey::IpAddr(ip));
        }
    }

    #[inline]
    fn resolver(&self) -> &dyn Resolver {
        &self.resolver
    }

    #[inline]
    fn resolver_mut(&mut self) -> &mut dyn Resolver {
        &mut self.resolver
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
        ThreadBuilder::new()
            .name("qiniu.rust-sdk.http-client.chooser.SimpleChooser".into())
            .spawn(move || {
                if is_time_to_shrink_mut(&cloned.blacklist, &cloned.lock) {
                    info!("Simple Chooser spawns thread to do some housework");
                    shrink_cache(&cloned.blacklist);
                }
            })
            .ok();
    }

    return;

    fn is_time_to_shrink(blacklist: &Blacklist, locked_data: &Mutex<LockedData>) -> bool {
        if let Ok(locked_data) = locked_data.try_lock() {
            _is_time_to_shrink_mut(blacklist, &*locked_data)
        } else {
            false
        }
    }

    fn is_time_to_shrink_mut(blacklist: &Blacklist, locked_data: &Mutex<LockedData>) -> bool {
        if let Ok(mut locked_data) = locked_data.try_lock() {
            if _is_time_to_shrink_mut(blacklist, &*locked_data) {
                locked_data.last_shrink_timestamp = SystemTime::now();
                return true;
            }
        }
        false
    }

    #[inline]
    fn _is_time_to_shrink_mut(blacklist: &Blacklist, locked_data: &LockedData) -> bool {
        locked_data.last_shrink_timestamp + MIN_SHRINK_INTERVAL < SystemTime::now()
            && blacklist.len() >= BLACKLIST_SIZE_TO_SHRINK
    }

    #[inline]
    fn shrink_cache(blacklist: &Blacklist) {
        blacklist.retain(|_, value| value.block_until >= SystemTime::now());
        info!("Blacklist is shrunken");
    }
}

impl Default for LockedData {
    #[inline]
    fn default() -> Self {
        LockedData {
            last_shrink_timestamp: UNIX_EPOCH,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::super::{ResolveResult, ResponseErrorKind},
        *,
    };
    use std::{collections::HashMap, error::Error, net::Ipv4Addr, result::Result, thread::sleep};

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
            chooser.choose("test_domain_1.com", false),
            ChosenResult::IPs(vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)),
            ])
        );

        chooser.freeze(
            "test_domain_1.com",
            &[IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3))],
            ResponseError::new(ResponseErrorKind::ParseResponseError, "Test Error"),
        );
        assert_eq!(
            chooser.choose("test_domain_1.com", false),
            ChosenResult::IPs(vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            ])
        );
        assert_eq!(
            chooser.choose("test_domain_1.com", true),
            ChosenResult::IPs(vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)),
            ])
        );

        chooser.freeze(
            "test_domain_1.com",
            &[
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            ],
            ResponseError::new(ResponseErrorKind::ParseResponseError, "Test Error"),
        );
        assert_eq!(
            chooser.choose("test_domain_1.com", false),
            ChosenResult::TryAnotherDomain,
        );

        chooser.freeze(
            "test_domain_2.com",
            &[],
            ResponseError::new(ResponseErrorKind::ParseResponseError, "Test Error"),
        );
        assert_eq!(
            chooser.choose("test_domain_2.com", false),
            ChosenResult::TryAnotherDomain,
        );

        assert_eq!(
            chooser.choose("test_domain_3.com", false),
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
            chooser.choose("test_domain_1.com", false),
            ChosenResult::IPs(vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)),
            ])
        );

        chooser.freeze(
            "test_domain_1.com",
            &[
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)),
            ],
            ResponseError::new(ResponseErrorKind::ParseResponseError, "Test Error"),
        );
        assert_eq!(
            chooser.choose("test_domain_1.com", false),
            ChosenResult::TryAnotherDomain,
        );

        sleep(Duration::from_secs(1));

        assert_eq!(
            chooser.choose("test_domain_1.com", false),
            ChosenResult::IPs(vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)),
            ])
        );

        chooser.freeze(
            "test_domain_1.com",
            &[
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)),
            ],
            ResponseError::new(ResponseErrorKind::ParseResponseError, "Test Error"),
        );
        assert_eq!(
            chooser.choose("test_domain_1.com", false),
            ChosenResult::TryAnotherDomain,
        );

        chooser.unfreeze(
            "test_domain_1.com",
            &[
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            ],
        );
        assert_eq!(
            chooser.choose("test_domain_1.com", false),
            ChosenResult::IPs(vec![
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            ])
        );

        Ok(())
    }
}
