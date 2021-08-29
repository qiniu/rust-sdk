use super::super::super::IpAddrWithPort;
use std::collections::HashSet;

pub(super) struct IpAddrsSet {
    set: HashSet<IpAddrWithPort>,
}

impl IpAddrsSet {
    #[inline]
    pub(super) fn new(ips: &[IpAddrWithPort]) -> Self {
        Self {
            set: ips.iter().cloned().collect(),
        }
    }

    #[inline]
    pub(super) fn difference(&mut self, ips: &[IpAddrWithPort]) {
        for ip in ips.iter() {
            self.set.remove(ip);
        }
    }

    #[inline]
    pub(super) fn remains(&self) -> Vec<IpAddrWithPort> {
        self.set.iter().cloned().collect()
    }
}
