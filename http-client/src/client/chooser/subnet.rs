use super::{
    super::super::{
        regions::{DomainWithPort, IpAddrWithPort},
        spawn::spawn,
    },
    ChooseOptions, Chooser, ChooserFeedback, ChosenResults,
};
use dashmap::DashMap;
use ipnet::{Ipv4Net, Ipv6Net, PrefixLenError};
use log::{info, warn};
use std::{
    collections::HashMap,
    mem::take,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use typenum::{IsLess, Le, NonZero, Unsigned, U128, U32};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BlacklistKey {
    ip: IpAddrWithPort,
    domain: Option<DomainWithPort>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Subnet(IpAddrWithPort);

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
const DEFAULT_IPV4_NETMASK_PREFIX_LENGTH: u8 = 24;
const DEFAULT_IPV6_NETMASK_PREFIX_LENGTH: u8 = 64;

/// 子网选择器
///
/// 包含子网黑名单，一旦被反馈 API 调用失败，则将所有相关子网内 IP 地址冻结一段时间
#[derive(Debug, Clone)]
pub struct SubnetChooser {
    inner: Arc<SubnetChooserInner>,
}

#[derive(Debug, Default)]
struct SubnetChooserInner {
    blacklist: Blacklist,
    lock: Mutex<LockedData>,
    block_duration: Duration,
    shrink_interval: Duration,
    ipv4_netmask_prefix_length: u8,
    ipv6_netmask_prefix_length: u8,
}

impl Default for SubnetChooser {
    #[inline]
    fn default() -> Self {
        Self::builder().build()
    }
}

impl SubnetChooser {
    /// 创建子网选择构建器
    #[inline]
    pub fn builder() -> SubnetChooserBuilder {
        Default::default()
    }
}

impl Chooser for SubnetChooser {
    fn choose(&self, ips: &[IpAddrWithPort], opts: ChooseOptions) -> ChosenResults {
        let mut need_to_shrink = false;
        let mut subnets_map: HashMap<Subnet, Vec<IpAddrWithPort>> = Default::default();
        for &ip in ips.iter() {
            let is_blocked = self
                .inner
                .blacklist
                .get(&BlacklistKey {
                    ip,
                    domain: opts.domain().cloned(),
                })
                .map_or(false, |r| {
                    if r.value().blocked_at.elapsed() < self.inner.block_duration {
                        true
                    } else {
                        need_to_shrink = true;
                        false
                    }
                });
            if !is_blocked {
                let subnet = self.get_network_address(ip);
                if let Some(ips) = subnets_map.get_mut(&subnet) {
                    ips.push(ip);
                } else {
                    subnets_map.insert(subnet, vec![ip]);
                }
            }
        }
        let chosen_ips = choose_group(subnets_map.into_iter().map(|(_, ips)| ips)).unwrap_or_default();
        do_some_work_async(&self.inner, need_to_shrink);
        return chosen_ips.into();

        /// For production, choose any subnet by random
        #[cfg(not(test))]
        fn choose_group(iter: impl Iterator<Item = Vec<IpAddrWithPort>>) -> Option<Vec<IpAddrWithPort>> {
            use rand::prelude::IteratorRandom;

            iter.choose(&mut rand::thread_rng())
        }

        /// For Test cases, always choose the biggest subnet
        #[cfg(test)]
        fn choose_group(iter: impl Iterator<Item = Vec<IpAddrWithPort>>) -> Option<Vec<IpAddrWithPort>> {
            iter.max_by_key(|ips| ips.len())
        }
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

impl SubnetChooser {
    fn get_network_address(&self, addr: IpAddrWithPort) -> Subnet {
        let subnet = match addr.ip_addr() {
            IpAddr::V4(ipv4_addr) => {
                let ipv4_network_addr =
                    get_network_address_of_ipv4_addr(ipv4_addr, self.inner.ipv4_netmask_prefix_length);
                IpAddr::V4(ipv4_network_addr)
            }
            IpAddr::V6(ipv6_addr) => {
                let ipv6_network_addr =
                    get_network_address_of_ipv6_addr(ipv6_addr, self.inner.ipv6_netmask_prefix_length);
                IpAddr::V6(ipv6_network_addr)
            }
        };
        return Subnet(IpAddrWithPort::new(subnet, addr.port()));

        fn get_network_address_of_ipv4_addr(addr: Ipv4Addr, prefix: u8) -> Ipv4Addr {
            Ipv4Net::new(addr, prefix).unwrap().network()
        }

        fn get_network_address_of_ipv6_addr(addr: Ipv6Addr, prefix: u8) -> Ipv6Addr {
            Ipv6Net::new(addr, prefix).unwrap().network()
        }
    }

    #[allow(dead_code)]
    fn len(&self) -> usize {
        self.inner.blacklist.len()
    }
}

fn do_some_work_async(inner: &Arc<SubnetChooserInner>, need_to_shrink: bool) {
    if need_to_shrink && is_time_to_shrink(inner) {
        let cloned = inner.to_owned();
        if let Err(err) = spawn("qiniu.rust-sdk.http-client.chooser.SubnetChooser".into(), move || {
            if is_time_to_shrink_mut(&cloned) {
                info!("Subnet Chooser spawns thread to do some housework");
                shrink_cache(&cloned.blacklist, cloned.block_duration);
            }
        }) {
            warn!(
                "Subnet Chooser was failed to spawn thread to do some housework: {}",
                err
            );
        }
    }

    return;

    fn is_time_to_shrink(inner: &Arc<SubnetChooserInner>) -> bool {
        if let Ok(locked_data) = inner.lock.try_lock() {
            _is_time_to_shrink(inner.shrink_interval, &locked_data)
        } else {
            false
        }
    }

    fn is_time_to_shrink_mut(inner: &Arc<SubnetChooserInner>) -> bool {
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

/// 子网选择构建器
#[derive(Debug)]
pub struct SubnetChooserBuilder {
    inner: SubnetChooserInner,
}

impl Default for SubnetChooserBuilder {
    #[inline]
    fn default() -> Self {
        Self {
            inner: SubnetChooserInner {
                blacklist: Default::default(),
                lock: Default::default(),
                block_duration: DEFAULT_BLOCK_DURATION,
                shrink_interval: DEFAULT_SHRINK_INTERVAL,
                ipv4_netmask_prefix_length: DEFAULT_IPV4_NETMASK_PREFIX_LENGTH,
                ipv6_netmask_prefix_length: DEFAULT_IPV6_NETMASK_PREFIX_LENGTH,
            },
        }
    }
}

impl SubnetChooserBuilder {
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

    /// 用安全的方式设置 IPv4 地址子网掩码前缀长度
    #[inline]
    pub fn safe_ipv4_netmask_prefix_length<N>(&mut self) -> &mut Self
    where
        N: Unsigned + IsLess<U32>,
        Le<N, U32>: NonZero,
    {
        self.inner.ipv4_netmask_prefix_length = N::to_u8();
        self
    }

    /// 用安全的方式设置 IPv6 地址子网掩码前缀长度
    #[inline]
    pub fn safe_ipv6_netmask_prefix_length<N>(&mut self) -> &mut Self
    where
        N: Unsigned + IsLess<U128>,
        Le<N, U128>: NonZero,
    {
        self.inner.ipv6_netmask_prefix_length = N::to_u8();
        self
    }

    /// 设置 IPv4 地址子网掩码前缀长度
    #[inline]
    pub fn ipv4_netmask_prefix_length(&mut self, ipv4_netmask_prefix_length: u8) -> Result<&mut Self, PrefixLenError> {
        if ipv4_netmask_prefix_length > 32 {
            return Err(PrefixLenError);
        }
        self.inner.ipv4_netmask_prefix_length = ipv4_netmask_prefix_length;
        Ok(self)
    }

    /// 设置 IPv6 地址子网掩码前缀长度
    #[inline]
    pub fn ipv6_netmask_prefix_length(&mut self, ipv6_netmask_prefix_length: u8) -> Result<&mut Self, PrefixLenError> {
        if ipv6_netmask_prefix_length > 128 {
            return Err(PrefixLenError);
        }
        self.inner.ipv6_netmask_prefix_length = ipv6_netmask_prefix_length;
        Ok(self)
    }

    /// 构建子网选择器
    #[inline]
    pub fn build(&mut self) -> SubnetChooser {
        SubnetChooser {
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
    use std::{
        net::{IpAddr, Ipv4Addr},
        thread::sleep,
    };

    const SUBNET_1: &[IpAddrWithPort] = &[
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), None),
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), None),
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)), None),
    ];
    const SUBNET_2: &[IpAddrWithPort] = &[
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 2, 1)), None),
        IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 2, 2)), None),
    ];

    #[test]
    fn test_subnet_chooser() {
        env_logger::builder().is_test(true).try_init().ok();
        let all_ips = [SUBNET_1, SUBNET_2].concat();
        let domain = DomainWithPort::new("fakedomain", None);

        let subnet_chooser = SubnetChooser::default();
        assert_eq!(
            subnet_chooser
                .choose(&all_ips, ChooseOptionsBuilder::new().domain(&domain).build())
                .into_ip_addrs(),
            SUBNET_1.to_vec()
        );
        subnet_chooser.feedback(
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
            subnet_chooser
                .choose(&all_ips, ChooseOptionsBuilder::new().domain(&domain).build())
                .into_ip_addrs(),
            vec![
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 2, 1)), None),
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 2, 2)), None),
            ]
        );
        subnet_chooser.feedback(
            ChooserFeedback::builder(&[
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 2, 1)), None),
                IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 2, 2)), None),
            ])
            .domain(&domain)
            .error(&ResponseError::new_with_msg(
                ResponseErrorKind::ParseResponseError,
                "Test Error",
            ))
            .build(),
        );
        assert_eq!(
            subnet_chooser
                .choose(&all_ips, ChooseOptionsBuilder::new().domain(&domain).build())
                .into_ip_addrs(),
            vec![IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)), None)]
        );

        subnet_chooser.feedback(
            ChooserFeedback::builder(&[IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)), None)])
                .domain(&domain)
                .error(&ResponseError::new_with_msg(
                    ResponseErrorKind::ParseResponseError,
                    "Test Error",
                ))
                .build(),
        );
        subnet_chooser.feedback(
            ChooserFeedback::builder(&[IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 2, 1)), None)])
                .domain(&domain)
                .build(),
        );
        assert_eq!(
            subnet_chooser
                .choose(&all_ips, ChooseOptionsBuilder::new().domain(&domain).build())
                .into_ip_addrs(),
            vec![IpAddrWithPort::new(IpAddr::V4(Ipv4Addr::new(192, 168, 2, 1)), None)],
        );
    }

    #[test]
    fn test_subnet_chooser_expiration_and_shrink() {
        env_logger::builder().is_test(true).try_init().ok();
        let all_ips = [SUBNET_1, SUBNET_2].concat();

        let subnet_chooser = SubnetChooser::builder()
            .block_duration(Duration::from_secs(1))
            .shrink_interval(Duration::from_millis(500))
            .build();

        assert_eq!(
            subnet_chooser.choose(&all_ips, Default::default()).into_ip_addrs(),
            SUBNET_1.to_vec()
        );
        subnet_chooser.feedback(
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
            subnet_chooser.choose(&all_ips, Default::default()).into_ip_addrs(),
            SUBNET_2.to_vec(),
        );

        sleep(Duration::from_secs(1));
        assert_eq!(
            subnet_chooser.choose(&all_ips, Default::default()).into_ip_addrs(),
            SUBNET_1.to_vec()
        );

        sleep(Duration::from_millis(500));
        assert_eq!(subnet_chooser.len(), 0);
    }
}
