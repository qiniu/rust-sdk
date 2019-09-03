use super::Region;
use crate::{config::Config, http, utils::auth::Auth};
use qiniu_http::Result;

pub struct BucketManager {
    http_client: http::Client,
    rs_url: &'static str,
}

impl BucketManager {
    pub(crate) fn new(auth: Auth, config: Config) -> BucketManager {
        BucketManager {
            rs_url: Region::hua_dong().rs_url(config.use_https()),
            http_client: http::Client::new(auth, config),
        }
    }

    pub fn rs_url(&mut self, rs_url: &'static str) -> &BucketManager {
        self.rs_url = rs_url;
        self
    }

    pub fn bucket_names(&self) -> Result<Vec<String>> {
        Ok(self
            .http_client
            .get("/buckets", &[self.rs_url])
            .token(http::Token::V1)
            .accept_json()
            .no_body()
            .send()?
            .parse_json()
            .unwrap()?)
    }
}
