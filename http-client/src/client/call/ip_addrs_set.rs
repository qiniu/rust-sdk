use super::super::super::IpAddrWithPort;
use std::{
    collections::HashSet,
    fmt::{self, Display},
};

#[derive(Default, Debug)]
pub(super) struct IpAddrsSet {
    set: HashSet<IpAddrWithPort>,
    ordered: Vec<IpAddrWithPort>,
}

impl IpAddrsSet {
    pub(super) fn new(ips: &[IpAddrWithPort]) -> Self {
        Self {
            set: ips.iter().copied().collect(),
            ordered: ips.to_vec(),
        }
    }

    pub(super) fn difference_slice(&mut self, ips: &[IpAddrWithPort]) {
        for ip in ips.iter() {
            self.set.remove(ip);
        }
    }

    pub(super) fn union_slice(&mut self, ips: &[IpAddrWithPort]) {
        for &ip in ips.iter() {
            self.set.insert(ip);
            self.ordered.push(ip);
        }
    }

    pub(super) fn difference_set(&mut self, ips: &Self) {
        for ip in ips.set.iter() {
            self.set.remove(ip);
        }
    }

    #[allow(dead_code)]
    pub(super) fn union_set(&mut self, ips: &Self) {
        for &ip in ips.set.iter() {
            self.set.insert(ip);
            self.ordered.push(ip);
        }
    }

    pub(super) fn remains(&self) -> Vec<IpAddrWithPort> {
        self.ordered
            .iter()
            .copied()
            .filter(|ip| self.set.contains(ip))
            .collect()
    }
}

impl Display for IpAddrsSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, ip) in self.ordered.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", ip)?;
        }
        write!(f, "]")?;
        Ok(())
    }
}
