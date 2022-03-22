use super::{
    super::{Endpoint, EndpointParseError},
    Region,
};
use serde::Deserialize;
use std::{convert::TryFrom, fmt::Debug, time::Duration};

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ResponseBody {
    #[serde(alias = "regions")]
    hosts: Vec<RegionResponseBody>,
}

impl ResponseBody {
    pub(super) fn into_hosts(self) -> Vec<RegionResponseBody> {
        self.hosts
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RegionResponseBody {
    #[serde(alias = "id")]
    region: Box<str>,
    #[serde(default = "default_ttl")]
    ttl: u64,
    io: DomainsResponseBody,
    up: DomainsResponseBody,
    uc: DomainsResponseBody,
    rs: DomainsResponseBody,
    rsf: DomainsResponseBody,
    api: DomainsResponseBody,
    s3: DomainsResponseBody,
}

impl RegionResponseBody {
    pub(super) fn lifetime(&self) -> Duration {
        Duration::from_secs(self.ttl)
    }
}

fn default_ttl() -> u64 {
    86400
}

#[derive(Debug, Clone, Deserialize)]
struct DomainsResponseBody {
    #[serde(rename = "domains")]
    preferred: Box<[Box<str>]>,

    #[serde(rename = "old")]
    alternative: Option<Box<[Box<str>]>>,

    #[serde(rename = "region_alias")]
    s3_region_id: Option<Box<str>>,
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
            ..
        } = body;
        let mut builder = Self::builder(region);

        macro_rules! push_to_builder {
            ($service_name:expr, $add_to_preferred_endpoint:ident, $add_to_alternative_endpoint:ident) => {
                for preferred_domain in $service_name.preferred.iter() {
                    let endpoint: Endpoint = preferred_domain.as_ref().parse()?;
                    builder.$add_to_preferred_endpoint(endpoint);
                }
                if let Some(alternative_domains) = &$service_name.alternative {
                    for alternative_domain in alternative_domains.iter() {
                        let endpoint: Endpoint = alternative_domain.as_ref().parse()?;
                        builder.$add_to_alternative_endpoint(endpoint);
                    }
                }
            };
        }
        push_to_builder!(io, add_io_preferred_endpoint, add_io_alternative_endpoint);
        push_to_builder!(up, add_up_preferred_endpoint, add_up_alternative_endpoint);
        push_to_builder!(uc, add_uc_preferred_endpoint, add_uc_alternative_endpoint);
        push_to_builder!(rs, add_rs_preferred_endpoint, add_rs_alternative_endpoint);
        push_to_builder!(rsf, add_rsf_preferred_endpoint, add_rsf_alternative_endpoint);
        push_to_builder!(api, add_api_preferred_endpoint, add_api_alternative_endpoint);
        push_to_builder!(s3, add_s3_preferred_endpoint, add_s3_alternative_endpoint);
        if let Some(s3_region_id) = s3.s3_region_id {
            builder.s3_region_id(s3_region_id);
        }

        Ok(builder.build())
    }
}
