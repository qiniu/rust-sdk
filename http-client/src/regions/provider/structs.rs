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
    #[serde(rename = "domains")]
    preferred: Box<[Box<str>]>,
    #[serde(rename = "old")]
    alternative: Option<Box<[Box<str>]>>,
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
            ($service_name:expr, $push_to_preferred_endpoint:ident, $push_to_alternative_endpoint:ident) => {
                for preferred_domain in $service_name.preferred.iter() {
                    let endpoint: Endpoint = preferred_domain.as_ref().parse()?;
                    builder.$push_to_preferred_endpoint(endpoint);
                }
                if let Some(alternative_domains) = &$service_name.alternative {
                    for alternative_domain in alternative_domains.iter() {
                        let endpoint: Endpoint = alternative_domain.as_ref().parse()?;
                        builder.$push_to_alternative_endpoint(endpoint);
                    }
                }
            };
        }
        push_to_builder!(io, push_io_preferred_endpoint, push_io_alternative_endpoint);
        push_to_builder!(up, push_up_preferred_endpoint, push_up_alternative_endpoint);
        push_to_builder!(uc, push_uc_preferred_endpoint, push_uc_alternative_endpoint);
        push_to_builder!(rs, push_rs_preferred_endpoint, push_rs_alternative_endpoint);
        push_to_builder!(
            rsf,
            push_rsf_preferred_endpoint,
            push_rsf_alternative_endpoint
        );
        push_to_builder!(
            api,
            push_api_preferred_endpoint,
            push_api_alternative_endpoint
        );
        push_to_builder!(s3, push_s3_preferred_endpoint, push_s3_alternative_endpoint);

        Ok(builder.build())
    }
}
