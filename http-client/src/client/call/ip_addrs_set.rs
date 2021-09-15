use super::super::super::IpAddrWithPort;
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct IpAddrsSet {
    set: HashSet<IpAddrWithPort>,
    ordered: Vec<IpAddrWithPort>,
}

impl IpAddrsSet {
    #[inline]
    pub(super) fn new(ips: &[IpAddrWithPort]) -> Self {
        Self {
            set: ips.iter().cloned().collect(),
            ordered: ips.to_vec(),
        }
    }

    #[inline]
    pub(super) fn difference_slice(&mut self, ips: &[IpAddrWithPort]) {
        for ip in ips.iter() {
            self.set.remove(ip);
        }
    }

    #[inline]
    pub(super) fn union_slice(&mut self, ips: &[IpAddrWithPort]) {
        for &ip in ips.iter() {
            self.set.insert(ip);
            self.ordered.push(ip);
        }
    }

    #[inline]
    pub(super) fn difference_set(&mut self, ips: &Self) {
        for ip in ips.set.iter() {
            self.set.remove(ip);
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub(super) fn union_set(&mut self, ips: &Self) {
        for &ip in ips.set.iter() {
            self.set.insert(ip);
            self.ordered.push(ip);
        }
    }

    #[inline]
    pub(super) fn remains(&self) -> Vec<IpAddrWithPort> {
        self.ordered
            .iter()
            .cloned()
            .filter(|ip| self.set.contains(ip))
            .collect()
    }
}
