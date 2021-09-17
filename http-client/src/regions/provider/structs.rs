use super::{
    super::{Endpoint, EndpointParseError},
    Region,
};
use serde::Deserialize;
use std::{convert::TryFrom, fmt::Debug};

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ResponseBody {
    #[serde(alias = "regions")]
    hosts: Vec<RegionResponseBody>,
}

impl ResponseBody {
    #[inline]
    pub(super) fn into_hosts(self) -> Vec<RegionResponseBody> {
        self.hosts
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RegionResponseBody {
    #[serde(alias = "id")]
    region: Box<str>,
    io: DomainsResponseBody,
    up: DomainsResponseBody,
    uc: DomainsResponseBody,
    rs: DomainsResponseBody,
    rsf: DomainsResponseBody,
    api: DomainsResponseBody,
    s3: DomainsResponseBody,
}

#[derive(Debug, Clone, Deserialize)]
struct DomainsResponseBody {
    domains: Box<[Box<str>]>,
    old: Option<Box<[Box<str>]>>,
}

impl TryFrom<RegionResponseBody> for Region {
    type Error = EndpointParseError;
    fn try_from(body: RegionResponseBody) -> Result<Self, Self::Error> {
        let RegionResponseBody {
            region,
            io,
            up,
            uc,
            rs,
            rsf,
            api,
            s3,
        } = body;
        let mut builder = Self::builder(region);

        macro_rules! push_to_builder {
            ($service_name:expr, $push_to_endpoint:ident, $push_to_old_endpoint:ident) => {
                for domain in $service_name.domains.iter() {
                    let endpoint: Endpoint = domain.as_ref().parse()?;
                    builder = builder.$push_to_endpoint(endpoint);
                }
                if let Some(old_domains) = &$service_name.old {
                    for old_domain in old_domains.iter() {
                        let endpoint: Endpoint = old_domain.as_ref().parse()?;
                        builder = builder.$push_to_old_endpoint(endpoint);
                    }
                }
            };
        }
        push_to_builder!(io, push_io_endpoint, push_io_old_endpoint);
        push_to_builder!(up, push_up_endpoint, push_up_old_endpoint);
        push_to_builder!(uc, push_uc_endpoint, push_uc_old_endpoint);
        push_to_builder!(rs, push_rs_endpoint, push_rs_old_endpoint);
        push_to_builder!(rsf, push_rsf_endpoint, push_rsf_old_endpoint);
        push_to_builder!(api, push_api_endpoint, push_api_old_endpoint);
        push_to_builder!(s3, push_s3_endpoint, push_s3_old_endpoint);

        Ok(builder.build())
    }
}
