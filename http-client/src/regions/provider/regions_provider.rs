use super::{
    super::{
        super::{ApiResult, Authorization, HttpClient, ResponseError},
        Endpoints, ServiceName,
    },
    structs::ResponseBody,
    GetOptions, GotRegion, GotRegions, Region, RegionProvider,
};
use qiniu_credential::CredentialProvider;
use std::{convert::TryFrom, fmt::Debug};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug, Clone)]
pub struct RegionsProvider {
    credential_provider: Box<dyn CredentialProvider>,
    http_client: HttpClient,
    uc_endpoints: Endpoints,
}

impl RegionsProvider {
    #[inline]
    pub fn builder(
        credential_provider: impl CredentialProvider + 'static,
    ) -> RegionsProviderBuilder {
        RegionsProviderBuilder::new(credential_provider)
    }

    #[inline]
    pub fn new(credential_provider: impl CredentialProvider + 'static) -> Self {
        Self::builder(credential_provider).build()
    }

    fn do_sync_query(&self) -> ApiResult<Vec<Region>> {
        let (parts, body) = self
            .http_client
            .get(&[ServiceName::Uc], &self.uc_endpoints)
            .path("/regions")
            .authorization(Authorization::v2(self.credential_provider.to_owned()))
            .accept_json()
            .call()?
            .parse_json::<ResponseBody>()?
            .into_parts();
        body.into_hosts()
            .into_iter()
            .map(|host| {
                Region::try_from(host)
                    .map_err(|err| ResponseError::from_endpoint_parse_error(err, &parts))
            })
            .collect()
    }

    #[cfg(feature = "async")]
    async fn do_async_query(&self) -> ApiResult<Vec<Region>> {
        let (parts, body) = self
            .http_client
            .async_get(&[ServiceName::Uc], &self.uc_endpoints)
            .path("/regions")
            .authorization(Authorization::v2(self.credential_provider.to_owned()))
            .accept_json()
            .call()
            .await?
            .parse_json::<ResponseBody>()
            .await?
            .into_parts();
        body.into_hosts()
            .into_iter()
            .map(|host| {
                Region::try_from(host)
                    .map_err(|err| ResponseError::from_endpoint_parse_error(err, &parts))
            })
            .collect()
    }
}

impl RegionProvider for RegionsProvider {
    fn get(&self, opts: &GetOptions) -> ApiResult<GotRegion> {
        self.get_all(opts).map(|regions| {
            regions
                .into_regions()
                .into_iter()
                .next()
                .expect("Regions API returns empty regions")
                .into()
        })
    }

    #[inline]
    fn get_all(&self, _opts: &GetOptions) -> ApiResult<GotRegions> {
        self.do_sync_query().map(GotRegions::from)
    }

    /// 异步返回七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get<'a>(&'a self, opts: &'a GetOptions) -> BoxFuture<'a, ApiResult<GotRegion>> {
        Box::pin(async move {
            self.async_get_all(opts).await.map(|regions| {
                regions
                    .into_regions()
                    .into_iter()
                    .next()
                    .expect("Regions API returns empty regions")
                    .into()
            })
        })
    }

    /// 异步返回多个七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get_all<'a>(&'a self, _opts: &'a GetOptions) -> BoxFuture<ApiResult<GotRegions>> {
        Box::pin(async move { self.do_async_query().await.map(GotRegions::from) })
    }
}

#[derive(Debug, Clone)]
pub struct RegionsProviderBuilder {
    credential_provider: Box<dyn CredentialProvider>,
    http_client: Option<HttpClient>,
    uc_endpoints: Option<Endpoints>,
}

impl RegionsProviderBuilder {
    #[inline]
    pub fn new(credential_provider: impl CredentialProvider + 'static) -> Self {
        Self {
            credential_provider: Box::new(credential_provider),
            http_client: None,
            uc_endpoints: None,
        }
    }

    #[inline]
    pub fn http_client(mut self, http_client: HttpClient) -> Self {
        self.http_client = Some(http_client);
        self
    }

    #[inline]
    pub fn uc_endpoints(mut self, uc_endpoints: impl Into<Endpoints>) -> Self {
        self.uc_endpoints = Some(uc_endpoints.into());
        self
    }

    #[inline]
    pub fn build(self) -> RegionsProvider {
        RegionsProvider {
            credential_provider: self.credential_provider,
            http_client: self.http_client.unwrap_or_default(),
            uc_endpoints: self
                .uc_endpoints
                .unwrap_or_else(|| Endpoints::public_uc_endpoints().to_owned()),
        }
    }
}

#[cfg(all(test, feature = "isahc", feature = "async"))]
mod tests {
    use crate::HttpClient;

    use super::{super::super::Endpoint, *};
    use futures::channel::oneshot::channel;
    use qiniu_credential::Credential;
    use serde_json::{json, Value as JsonValue};
    use std::{error::Error, result::Result};
    use tokio::task::spawn;
    use warp::{http::header::HeaderValue, path, reply::Response, Filter};

    macro_rules! starts_with_server {
        ($addr:ident, $routes:ident, $code:block) => {{
            let (tx, rx) = channel();
            let ($addr, server) =
                warp::serve($routes).bind_with_graceful_shutdown(([127, 0, 0, 1], 0), async move {
                    rx.await.ok();
                });
            let handler = spawn(server);
            $code;
            tx.send(()).ok();
            handler.await.ok();
        }};
    }

    #[tokio::test]
    async fn test_get_all_regions() -> Result<(), Box<dyn Error>> {
        const ACCESS_KEY: &str = "0123456789001234567890";
        const SECRET_KEY: &str = "secret01234567890";
        let routes = path!("regions")
            .and(warp::header::value("Authorization"))
            .map(move |authorization: HeaderValue| {
                assert!(authorization
                    .to_str()
                    .unwrap()
                    .starts_with("Qiniu 0123456789001234567890:"));
                let mut response =
                    Response::new(get_response_json_body().to_string().into_bytes().into());
                response
                    .headers_mut()
                    .insert("X-Reqid", HeaderValue::from_static("FAKE_REQ_ID"));
                response
            });

        starts_with_server!(addr, routes, {
            let provider = RegionsProvider::builder(Credential::new(ACCESS_KEY, SECRET_KEY))
                .http_client(HttpClient::build_isahc()?.use_https(false).build())
                .uc_endpoints(vec![Endpoint::from(addr)])
                .build();

            let regions = provider.async_get_all(&Default::default()).await?;
            assert_eq!(regions.len(), 5);
            assert_eq!(
                regions
                    .iter()
                    .map(|region| region.region_id())
                    .collect::<Vec<_>>(),
                &["z0", "z1", "z2", "as0", "na0"]
            )
        });
        Ok(())
    }

    fn get_response_json_body() -> JsonValue {
        json!({
            "regions": [
               {
                 "id": "z0",
                 "description": "East China",
                 "io": {
                   "domains": [
                     "iovip.qbox.me"
                   ]
                 },
                 "up": {
                   "domains": [
                     "upload.qiniup.com",
                     "up.qiniup.com"
                   ],
                   "old": [
                     "upload.qbox.me",
                     "up.qbox.me"
                   ]
                 },
                 "uc": {
                   "domains": [
                     "uc.qbox.me"
                   ]
                 },
                 "rs": {
                   "domains": [
                     "rs-z0.qbox.me"
                   ]
                 },
                 "rsf": {
                   "domains": [
                     "rsf-z0.qbox.me"
                   ]
                 },
                 "api": {
                   "domains": [
                     "api.qiniu.com"
                   ]
                 },
                 "s3": {
                   "domains": [
                     "s3-cn-east-1.qiniucs.com"
                   ],
                   "region_alias": "cn-east-1"
                 }
               },
               {
                 "id": "z1",
                 "description": "North China",
                 "io": {
                   "domains": [
                     "iovip-z1.qbox.me"
                   ]
                 },
                 "up": {
                   "domains": [
                     "upload-z1.qiniup.com",
                     "up-z1.qiniup.com"
                   ],
                   "old": [
                     "upload-z1.qbox.me",
                     "up-z1.qbox.me"
                   ]
                 },
                 "uc": {
                   "domains": [
                     "uc.qbox.me"
                   ]
                 },
                 "rs": {
                   "domains": [
                     "rs-z1.qbox.me"
                   ]
                 },
                 "rsf": {
                   "domains": [
                     "rsf-z1.qbox.me"
                   ]
                 },
                 "api": {
                   "domains": [
                     "api.qiniu.com"
                   ]
                 },
                 "s3": {
                   "domains": [
                     "s3-cn-north-1.qiniucs.com"
                   ],
                   "region_alias": "cn-north-1"
                 }
               },
               {
                 "id": "z2",
                 "description": "South China",
                 "io": {
                   "domains": [
                     "iovip-z2.qbox.me"
                   ]
                 },
                 "up": {
                   "domains": [
                     "upload-z2.qiniup.com",
                     "up-z2.qiniup.com"
                   ],
                   "old": [
                     "upload-z2.qbox.me",
                     "up-z2.qbox.me"
                   ]
                 },
                 "uc": {
                   "domains": [
                     "uc.qbox.me"
                   ]
                 },
                 "rs": {
                   "domains": [
                     "rs-z2.qbox.me"
                   ]
                 },
                 "rsf": {
                   "domains": [
                     "rsf-z2.qbox.me"
                   ]
                 },
                 "api": {
                   "domains": [
                     "api.qiniu.com"
                   ]
                 },
                 "s3": {
                   "domains": [
                     "s3-cn-south-1.qiniucs.com"
                   ],
                   "region_alias": "cn-south-1"
                 }
               },
               {
                 "id": "as0",
                 "description": "Southeast Asia",
                 "io": {
                   "domains": [
                     "iovip-as0.qbox.me"
                   ]
                 },
                 "up": {
                   "domains": [
                     "upload-as0.qiniup.com",
                     "up-as0.qiniup.com"
                   ],
                   "old": [
                     "upload-as0.qbox.me",
                     "up-as0.qbox.me"
                   ]
                 },
                 "uc": {
                   "domains": [
                     "uc.qbox.me"
                   ]
                 },
                 "rs": {
                   "domains": [
                     "rs-na0.qbox.me"
                   ]
                 },
                 "rsf": {
                   "domains": [
                     "rsf-na0.qbox.me"
                   ]
                 },
                 "api": {
                   "domains": [
                     "api.qiniu.com"
                   ]
                 },
                 "s3": {
                   "domains": [
                     "s3-ap-southeast-1.qiniucs.com"
                   ],
                   "region_alias": "ap-southeast-1"
                 }
               },
               {
                 "id": "na0",
                 "description": "North America",
                 "io": {
                   "domains": [
                     "iovip-na0.qbox.me"
                   ]
                 },
                 "up": {
                   "domains": [
                     "upload-na0.qiniup.com",
                     "up-na0.qiniup.com"
                   ],
                   "old": [
                     "upload-na0.qbox.me",
                     "up-na0.qbox.me"
                   ]
                 },
                 "uc": {
                   "domains": [
                     "uc.qbox.me"
                   ]
                 },
                 "rs": {
                   "domains": [
                     "rs-na0.qbox.me"
                   ]
                 },
                 "rsf": {
                   "domains": [
                     "rsf-na0.qbox.me"
                   ]
                 },
                 "api": {
                   "domains": [
                     "api.qiniu.com"
                   ]
                 },
                 "s3": {
                   "domains": [
                     "s3-us-north-1.qiniucs.com"
                   ],
                   "region_alias": "us-north-1"
                 }
               }
            ]
        })
    }
}
