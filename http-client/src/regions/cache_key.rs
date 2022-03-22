use super::super::Endpoints;
use md5::{
    digest::{generic_array::GenericArray, FixedOutputDirty},
    Md5,
};
use qiniu_credential::AccessKey;
use qiniu_upload_token::BucketName;
use serde::{
    de::{Error as DeError, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

type Md5Value = GenericArray<u8, <Md5 as FixedOutputDirty>::OutputSize>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(super) struct CacheKey {
    uc_md5_hex: String,
    ak_and_bucket: Option<AkAndBucket>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
struct AkAndBucket {
    bucket_name: BucketName,
    access_key: AccessKey,
}

impl CacheKey {
    #[inline]
    fn new(uc_md5: &Md5Value, ak_and_bucket: Option<(BucketName, AccessKey)>) -> Self {
        Self {
            uc_md5_hex: hex::encode(uc_md5),
            ak_and_bucket: ak_and_bucket.map(|(bucket_name, access_key)| AkAndBucket {
                bucket_name,
                access_key,
            }),
        }
    }

    #[inline]
    pub(super) fn new_from_endpoint(uc_endpoints: &Endpoints, ak_and_bucket: Option<(BucketName, AccessKey)>) -> Self {
        Self::new(uc_endpoints.md5(), ak_and_bucket)
    }

    #[inline]
    pub(super) fn new_from_endpoint_and_ak_and_bucket(
        uc_endpoints: &Endpoints,
        bucket_name: BucketName,
        access_key: AccessKey,
    ) -> Self {
        Self::new_from_endpoint(uc_endpoints, Some((bucket_name, access_key)))
    }
}

impl Serialize for CacheKey {
    #[inline]
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if let Some(ak_and_bucket) = &self.ak_and_bucket {
            s.collect_str(&format!(
                "qiniu-cache-key-v1:{}:{}:{}",
                self.uc_md5_hex, ak_and_bucket.access_key, ak_and_bucket.bucket_name,
            ))
        } else {
            s.collect_str(&format!("qiniu-cache-key-v1:{}", self.uc_md5_hex))
        }
    }
}

struct CacheKeyVisitor;

impl Visitor<'_> for CacheKeyVisitor {
    type Value = CacheKey;

    #[inline]
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Key of cache")
    }

    fn visit_str<E: DeError>(self, value: &str) -> Result<Self::Value, E> {
        if let Some(value) = value.strip_prefix("qiniu-cache-key-v1:") {
            let mut iter = value.splitn(3, ':');
            match (iter.next(), iter.next(), iter.next()) {
                (Some(uc_md5_hex), None, None) => Ok(CacheKey {
                    uc_md5_hex: uc_md5_hex.to_owned(),
                    ak_and_bucket: None,
                }),
                (Some(uc_md5_hex), Some(ak), Some(bucket)) => Ok(CacheKey {
                    uc_md5_hex: uc_md5_hex.to_owned(),
                    ak_and_bucket: Some(AkAndBucket {
                        bucket_name: bucket.into(),
                        access_key: ak.into(),
                    }),
                }),
                _ => Err(E::custom(format!("Invalid cache_key: {}", value))),
            }
        } else {
            Err(E::custom(format!("Unrecognized version of cache_key: {}", value)))
        }
    }
}

impl<'de> Deserialize<'de> for CacheKey {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(CacheKeyVisitor)
    }
}
