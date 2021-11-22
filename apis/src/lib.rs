#![cfg_attr(feature = "docs", feature(doc_cfg))]
pub use qiniu_http_client as http_client;
pub use qiniu_http_client::credential;
pub use qiniu_http_client::http;
#[cfg(feature = "isahc")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "isahc")))]
pub use qiniu_http_client::isahc;
#[cfg(feature = "reqwest")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "reqwest")))]
pub use qiniu_http_client::reqwest;
pub use qiniu_http_client::upload_token;
#[cfg(feature = "ureq")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "ureq")))]
pub use qiniu_http_client::ureq;
#[doc = "七牛 API 所用的基础类型库"]
pub mod base_types;
pub(crate) mod base_utils;
pub mod storage;
#[derive(Debug, Clone, Default)]
pub struct Client(qiniu_http_client::HttpClient);
impl Client {
    #[inline]
    pub fn new(client: qiniu_http_client::HttpClient) -> Self {
        Self(client)
    }
    #[inline]
    pub fn storage(&self) -> storage::Client {
        storage::Client::new(&self.0)
    }
}
impl From<qiniu_http_client::HttpClient> for Client {
    #[inline]
    fn from(client: qiniu_http_client::HttpClient) -> Self {
        Self(client)
    }
}
