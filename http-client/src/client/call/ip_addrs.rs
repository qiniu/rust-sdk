use super::super::super::IpAddrWithPort;
use std::fmt::{self, Display};

#[derive(Default, Debug)]
pub(super) struct IpAddrs(Vec<IpAddrWithPort>);

impl From<IpAddrs> for Vec<IpAddrWithPort> {
    fn from(ips: IpAddrs) -> Self {
        ips.0
    }
}

impl From<Vec<IpAddrWithPort>> for IpAddrs {
    fn from(ips: Vec<IpAddrWithPort>) -> Self {
        Self(ips)
    }
}

impl Display for IpAddrs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, ip) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", ip)?;
        }
        write!(f, "]")?;
        Ok(())
    }
}
