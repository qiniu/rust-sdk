use super::{
    super::{super::regions::IpAddrWithPort, spawn::spawn},
    Chooser, ChooserFeedback,
};
use dashmap::DashMap;
pub use ipnet::PrefixLenError;
use ipnet::{Ipv4Net, Ipv6Net};
use log::{info, warn};
use std::{
    any::Any,
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
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
const DEFAULT_IPV4_NETMASK_PREFIX_LENGTH: u8 = 24;
const DEFAULT_IPV6_NETMASK_PREFIX_LENGTH: u8 = 64;

#[derive(Debug, Clone)]
pub struct SubnetChooser {
    inner: Arc<SubnetChooserInner>,
}

#[derive(Debug, Default)]
struct SubnetChooserInner {
    blacklist: Blacklist,
    lock: Mutex<LockedData>,
    block_duration: Duration,
    min_shrink_interval: Duration,
    ipv4_netmask_prefix_length: u8,
    ipv6_netmask_prefix_length: u8,
}

impl Default for SubnetChooser {
    #[inline]
    fn default() -> Self {
        Self {
            inner: Arc::new(SubnetChooserInner {
                blacklist: Default::default(),
                lock: Default::default(),
                block_duration: DEFAULT_BLOCK_DURATION,
                min_shrink_interval: DEFAULT_MIN_SHRINK_INTERVAL,
                ipv4_netmask_prefix_length: DEFAULT_IPV4_NETMASK_PREFIX_LENGTH,
                ipv6_netmask_prefix_length: DEFAULT_IPV6_NETMASK_PREFIX_LENGTH,
            }),
        }
    }
}

impl SubnetChooser {
    #[inline]
    pub fn builder() -> SubnetChooserBuilder {
        SubnetChooserBuilder {
            inner: SubnetChooserInner {
                blacklist: Default::default(),
                lock: Default::default(),
                block_duration: DEFAULT_BLOCK_DURATION,
                min_shrink_interval: DEFAULT_MIN_SHRINK_INTERVAL,
                ipv4_netmask_prefix_length: DEFAULT_IPV4_NETMASK_PREFIX_LENGTH,
                ipv6_netmask_prefix_length: DEFAULT_IPV6_NETMASK_PREFIX_LENGTH,
            },
        }
    }
}

impl Chooser for SubnetChooser {
    #[inline]
    fn choose(&self, ips: &[IpAddrWithPort]) -> Vec<IpAddrWithPort> {
        let mut need_to_shrink = false;
        let mut ip_network_map: HashMap<IpAddrWithPort, Vec<IpAddrWithPort>> = Default::default();
        for &ip in ips.iter() {
            let network_address = self.get_network_address(ip);
            let is_blocked = self
                .inner
                .blacklist
                .get(&BlacklistKey::from(network_address))
                .map_or(false, |r| {
                    if r.value().blocked_at.elapsed() < self.inner.block_duration {
                        true
                    } else {
                        need_to_shrink = true;
                        false
                    }
                });
            if !is_blocked {
                if let Some(ips) = ip_network_map.get_mut(&BlacklistKey::from(network_address)) {
                    ips.push(ip);
                } else {
                    ip_network_map.insert(BlacklistKey::from(network_address), vec![ip]);
                }
            }
        }
        let chosen_ips = ip_network_map
            .into_iter()
            .next()
            .map(|(_, ips)| ips)
            .unwrap_or_default();
        do_some_work_async(&self.inner, need_to_shrink);
        chosen_ips
    }

    fn feedback(&self, feedback: ChooserFeedback) {
        if feedback.error().is_some() {
            for &ip in feedback.ips().iter() {
                self.inner.blacklist.insert(
                    self.get_network_address(ip),
                    BlacklistValue {
                        blocked_at: Instant::now(),
                    },
                );
            }
        } else {
            for &ip in feedback.ips().iter() {
                self.inner.blacklist.remove(&self.get_network_address(ip));
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

impl SubnetChooser {
    fn get_network_address(&self, addr: IpAddrWithPort) -> IpAddrWithPort {
        match addr.ip_addr() {
            IpAddr::V4(ipv4_addr) => {
                let ipv4_network_addr = get_network_address_of_ipv4_addr(
                    ipv4_addr,
                    self.inner.ipv4_netmask_prefix_length,
                );
                IpAddrWithPort::new_with_port(IpAddr::V4(ipv4_network_addr), addr.port())
            }
            IpAddr::V6(ipv6_addr) => {
                let ipv6_network_addr = get_network_address_of_ipv6_addr(
                    ipv6_addr,
                    self.inner.ipv6_netmask_prefix_length,
                );
                IpAddrWithPort::new_with_port(IpAddr::V6(ipv6_network_addr), addr.port())
            }
        }
    }
}

fn do_some_work_async(inner: &Arc<SubnetChooserInner>, need_to_shrink: bool) {
    if need_to_shrink && is_time_to_shrink(inner) {
        let cloned = inner.to_owned();
        if let Err(err) = spawn(
            "qiniu.rust-sdk.http-client.chooser.SubnetChooser".into(),
            move || {
                if is_time_to_shrink_mut(&cloned) {
                    info!("Subnet Chooser spawns thread to do some housework");
                    shrink_cache(&cloned.blacklist, cloned.block_duration);
                }
            },
        ) {
            warn!(
                "Subnet Chooser was failed to spawn thread to do some housework: {}",
                err
            );
        }
    }

    return;

    #[inline]
    fn is_time_to_shrink(inner: &Arc<SubnetChooserInner>) -> bool {
        if let Ok(locked_data) = inner.lock.try_lock() {
            _is_time_to_shrink(inner.min_shrink_interval, &*locked_data)
        } else {
            false
        }
    }

    #[inline]
    fn is_time_to_shrink_mut(inner: &Arc<SubnetChooserInner>) -> bool {
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
pub struct SubnetChooserBuilder {
    inner: SubnetChooserInner,
}

impl SubnetChooserBuilder {
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
    pub fn ipv4_netmask_prefix_length(mut self, ipv4_netmask_prefix_length: u8) -> Self {
        self.inner.ipv4_netmask_prefix_length = ipv4_netmask_prefix_length;
        self
    }

    #[inline]
    pub fn ipv6_netmask_prefix_length(mut self, ipv6_netmask_prefix_length: u8) -> Self {
        self.inner.ipv6_netmask_prefix_length = ipv6_netmask_prefix_length;
        self
    }

    #[inline]
    pub fn build(self) -> SubnetChooser {
        SubnetChooser {
            inner: Arc::new(self.inner),
        }
    }
}

#[inline]
fn get_network_address_of_ipv4_addr(addr: Ipv4Addr, prefix: u8) -> Ipv4Addr {
    Ipv4Net::new(addr, prefix).unwrap().network()
}

#[inline]
fn get_network_address_of_ipv6_addr(addr: Ipv6Addr, prefix: u8) -> Ipv6Addr {
    Ipv6Net::new(addr, prefix).unwrap().network()
}
