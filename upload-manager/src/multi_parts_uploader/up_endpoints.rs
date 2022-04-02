use qiniu_apis::http_client::{ApiResult, Endpoint, Endpoints, EndpointsGetOptions, EndpointsProvider, ServiceName};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(from = "Endpoints", into = "Endpoints")]
pub(super) struct UpEndpoints {
    inner: Endpoints,
    set: HashSet<Endpoint>,
    vec: Vec<Endpoint>,
}

impl From<Endpoints> for UpEndpoints {
    fn from(endpoints: Endpoints) -> Self {
        Self {
            set: to_hash_set(&endpoints),
            vec: to_vec(&endpoints),
            inner: endpoints,
        }
    }
}

impl From<UpEndpoints> for Endpoints {
    fn from(endpoints: UpEndpoints) -> Self {
        endpoints.inner
    }
}

impl UpEndpoints {
    pub(super) fn from_endpoints_provider(endpoints_provider: &impl EndpointsProvider) -> ApiResult<Self> {
        let inner = endpoints_provider
            .get_endpoints(EndpointsGetOptions::builder().service_names(&[ServiceName::Up]).build())?
            .into_owned();
        let set = to_hash_set(&inner);
        let vec = to_vec(&inner);
        Ok(Self { inner, set, vec })
    }

    #[cfg(feature = "async")]
    pub(super) async fn async_from_endpoints_provider(endpoints_provider: &impl EndpointsProvider) -> ApiResult<Self> {
        let inner = endpoints_provider
            .async_get_endpoints(EndpointsGetOptions::builder().service_names(&[ServiceName::Up]).build())
            .await?
            .into_owned();
        let set = to_hash_set(&inner);
        let vec = to_vec(&inner);
        Ok(Self { inner, set, vec })
    }

    pub(super) fn any_intersection(&self, others: &[Endpoint]) -> bool {
        for other in others {
            if self.set.contains(other) {
                return true;
            }
        }
        false
    }

    pub(super) fn as_slice(&self) -> &[Endpoint] {
        &self.vec
    }
}

impl EndpointsProvider for UpEndpoints {
    #[inline]
    fn get_endpoints<'e>(&'e self, _options: EndpointsGetOptions<'_>) -> ApiResult<Cow<'e, Endpoints>> {
        Ok(Cow::Borrowed(&self.inner))
    }
}

fn to_hash_set(endpoints: &Endpoints) -> HashSet<Endpoint> {
    endpoints
        .preferred()
        .iter()
        .chain(endpoints.alternative().iter())
        .cloned()
        .collect()
}

fn to_vec(endpoints: &Endpoints) -> Vec<Endpoint> {
    endpoints
        .preferred()
        .iter()
        .chain(endpoints.alternative().iter())
        .cloned()
        .collect()
}
