use super::{
    super::{
        super::{APIResult, Authorization, HTTPClient, ResponseError, ResponseErrorKind},
        Endpoints, ServiceName,
    },
    structs::ResponseBody,
    Region, RegionProvider,
};
use qiniu_credential::CredentialProvider;
use std::{any::Any, convert::TryFrom, fmt::Debug, sync::Arc};

#[cfg(feature = "async")]
use {async_std::task::spawn, futures::future::BoxFuture};

#[derive(Debug, Clone)]
pub struct RegionsProvider {
    inner: Arc<RegionsProviderInner>,
}

#[derive(Debug)]
struct RegionsProviderInner {
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
            inner: Arc::new(RegionsProviderInner {
                http_client,
                credential_provider,
                uc_endpoints: uc_endpoints.into(),
            }),
        }
    }

    fn do_sync_query(&self) -> APIResult<Vec<Region>> {
        let body: ResponseBody = self
            .inner
            .http_client
            .get(ServiceName::Uc, self.inner.uc_endpoints.to_owned())
            .path("/regions")
            .authorization(Authorization::v2(self.inner.credential_provider.to_owned()))
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
        let ctx = self.to_owned();

        spawn(async move { ctx.do_sync_query() }).await
    }
}

impl RegionProvider for RegionsProvider {
    fn get(&self) -> APIResult<Region> {
        self.get_all().map(|regions| {
            regions
                .into_iter()
                .next()
                .expect("Regions API returns empty regions")
        })
    }

    #[inline]
    fn get_all(&self) -> APIResult<Vec<Region>> {
        self.do_sync_query()
    }

    /// 异步返回七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get(&self) -> BoxFuture<APIResult<Region>> {
        Box::pin(async move {
            self.async_get_all().await.map(|regions| {
                regions
                    .into_iter()
                    .next()
                    .expect("Regions API returns empty regions")
            })
        })
    }

    /// 异步返回多个七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get_all(&self) -> BoxFuture<APIResult<Vec<Region>>> {
        Box::pin(async move { self.do_async_query().await })
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_region_provider(&self) -> &dyn RegionProvider {
        self
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

            let regions = provider.async_get_all().await?;
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
