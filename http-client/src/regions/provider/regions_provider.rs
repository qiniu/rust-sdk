use super::{
    super::{
        super::{APIResult, Authorization, HTTPClient, ResponseError, ResponseErrorKind},
        Endpoints, ServiceName,
    },
    structs::ResponseBody,
    GetOptions, GotRegion, GotRegions, Region, RegionProvider,
};
use qiniu_credential::CredentialProvider;
use std::{convert::TryFrom, fmt::Debug, sync::Arc};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug)]
pub struct RegionsProvider {
    credential_provider: Arc<dyn CredentialProvider>,
    http_client: HTTPClient,
    uc_endpoints: Endpoints,
}

impl RegionsProvider {
    #[inline]
    pub fn new(
        http_client: HTTPClient,
        uc_endpoints: impl Into<Endpoints>,
        credential_provider: Arc<dyn CredentialProvider>,
    ) -> Self {
        Self {
            http_client,
            credential_provider,
            uc_endpoints: uc_endpoints.into(),
        }
    }

    fn do_sync_query(&self) -> APIResult<Vec<Region>> {
        let body: ResponseBody = self
            .http_client
            .get(&[ServiceName::Uc], self.uc_endpoints.to_owned())
            .path("/regions")
            .authorization(Authorization::v2(self.credential_provider.to_owned()))
            .accept_json()
            .call()?
            .parse_json()?
            .into_body();
        body.into_hosts()
            .into_iter()
            .map(|host| {
                Region::try_from(host)
                    .map_err(|err| ResponseError::new(ResponseErrorKind::ParseResponseError, err))
            })
            .collect()
    }

    #[cfg(feature = "async")]
    async fn do_async_query(&self) -> APIResult<Vec<Region>> {
        let body: ResponseBody = self
            .http_client
            .async_get(&[ServiceName::Uc], self.uc_endpoints.to_owned())
            .path("/regions")
            .authorization(Authorization::v2(self.credential_provider.to_owned()))
            .accept_json()
            .call()
            .await?
            .parse_json()
            .await?
            .into_body();
        body.into_hosts()
            .into_iter()
            .map(|host| {
                Region::try_from(host)
                    .map_err(|err| ResponseError::new(ResponseErrorKind::ParseResponseError, err))
            })
            .collect()
    }
}

impl RegionProvider for RegionsProvider {
    fn get(&self, opts: &GetOptions) -> APIResult<GotRegion> {
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
    fn get_all(&self, _opts: &GetOptions) -> APIResult<GotRegions> {
        self.do_sync_query().map(GotRegions::from)
    }

    /// 异步返回七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get<'a>(&'a self, opts: &'a GetOptions) -> BoxFuture<'a, APIResult<GotRegion>> {
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
    fn async_get_all<'a>(&'a self, _opts: &'a GetOptions) -> BoxFuture<APIResult<GotRegions>> {
        Box::pin(async move { self.do_async_query().await.map(GotRegions::from) })
    }
}

#[cfg(all(test, feature = "isahc", feature = "async"))]
mod tests {
    use crate::HTTPClient;

    use super::{super::super::Endpoint, *};
    use futures::channel::oneshot::channel;
    use qiniu_credential::{Credential, StaticCredentialProvider};
    use serde_json::{json, Value as JSONValue};
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
            let provider = RegionsProvider::new(
                HTTPClient::build_isahc()?.use_https(false).build(),
                vec![Endpoint::from(addr)],
                Arc::new(StaticCredentialProvider::new(Credential::new(
                    ACCESS_KEY, SECRET_KEY,
                ))),
            );

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

    fn get_response_json_body() -> JSONValue {
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
