use crate::{utils::bool as bool_utils, Config};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    convert::TryInto,
    default::Default,
    ops::{Bound, RangeBounds},
    str::Split,
    time::{Duration, SystemTime},
};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UploadPolicy<'p> {
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<Cow<'p, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deadline: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    is_prefixal_scope: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    insert_only: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    return_url: Option<Cow<'p, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    return_body: Option<Cow<'p, str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    callback_url: Option<Cow<'p, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_host: Option<Cow<'p, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_body: Option<Cow<'p, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_body_type: Option<Cow<'p, str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    save_key: Option<Cow<'p, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    force_save_key: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    fsize_min: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fsize_limit: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    detect_mime: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mime_limit: Option<Cow<'p, str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    file_type: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    delete_after_days: Option<usize>,
}

impl<'p> UploadPolicy<'p> {
    pub fn bucket(&self) -> Option<&str> {
        self.scope.as_ref().and_then(|s| s.splitn(2, ':').nth(0))
    }

    pub fn key(&self) -> Option<&str> {
        self.scope.as_ref().and_then(|s| s.splitn(2, ':').nth(1))
    }

    pub fn use_prefixal_object_key(&self) -> bool {
        bool_utils::int_to_bool(self.is_prefixal_scope.unwrap_or(0))
    }

    pub fn is_insert_only(&self) -> bool {
        bool_utils::int_to_bool(self.insert_only.unwrap_or(0))
    }

    pub fn is_overwritable(&self) -> bool {
        !self.is_insert_only()
    }

    pub fn mime_detection_enabled(&self) -> bool {
        bool_utils::int_to_bool(self.detect_mime.unwrap_or(0))
    }

    pub fn token_deadline(&self) -> Option<SystemTime> {
        self.deadline
            .map(|t| SystemTime::UNIX_EPOCH + Duration::from_secs(t.into()))
    }

    pub fn token_lifetime(&self) -> Option<Duration> {
        self.token_deadline().map(|t| {
            t.duration_since(SystemTime::now())
                .unwrap_or_else(|_| Duration::from_secs(0))
        })
    }

    pub fn return_url(&self) -> Option<&str> {
        Self::convert_to_optional_str(&self.return_url)
    }

    pub fn return_body(&self) -> Option<&str> {
        Self::convert_to_optional_str(&self.return_body)
    }

    pub fn callback_urls(&self) -> Option<Split<char>> {
        Self::convert_to_optional_splited_str(&self.callback_url, ';')
    }

    pub fn callback_host(&self) -> Option<&str> {
        Self::convert_to_optional_str(&self.callback_host)
    }

    pub fn callback_body(&self) -> Option<&str> {
        Self::convert_to_optional_str(&self.callback_body)
    }

    pub fn callback_body_type(&self) -> Option<&str> {
        Self::convert_to_optional_str(&self.callback_body_type)
    }

    pub fn save_key(&self) -> Option<&str> {
        Self::convert_to_optional_str(&self.save_key)
    }

    pub fn is_save_key_forced(&self) -> bool {
        self.force_save_key.unwrap_or(false)
    }

    pub fn file_size_limitation(&self) -> (Option<usize>, Option<usize>) {
        (self.fsize_min, self.fsize_limit)
    }

    pub fn mime_types(&self) -> Option<Split<char>> {
        Self::convert_to_optional_splited_str(&self.mime_limit, ';')
    }

    pub fn is_normal_storage_used(&self) -> bool {
        !self.is_infrequent_storage_used()
    }

    pub fn is_infrequent_storage_used(&self) -> bool {
        bool_utils::int_to_bool(self.file_type.unwrap_or(0))
    }

    pub fn object_lifetime(&self) -> Option<Duration> {
        self.delete_after_days
            .map(|d| Duration::from_secs((d * 60 * 60 * 24).try_into().unwrap_or(u64::max_value())))
    }

    pub fn object_deadline(&self) -> Option<SystemTime> {
        self.object_lifetime().map(|t| SystemTime::now() + t)
    }

    fn convert_to_optional_str<'a>(s: &'a Option<Cow<'p, str>>) -> Option<&'a str> {
        s.as_ref().map(|s| s.as_ref())
    }

    fn convert_to_optional_splited_str<'a>(s: &'a Option<Cow<'p, str>>, pat: char) -> Option<Split<'a, char>> {
        s.as_ref().map(|x| x.split(pat))
    }

    pub fn as_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    pub fn from_json(json: &'p str) -> serde_json::Result<UploadPolicy<'p>> {
        serde_json::from_str(json)
    }

    pub fn from_json_owned<J: AsRef<str>>(json: J) -> serde_json::Result<UploadPolicy<'static>> {
        serde_json::from_str(json.as_ref())
    }

    pub fn from_json_slice(json: &'p [u8]) -> serde_json::Result<UploadPolicy<'p>> {
        serde_json::from_slice(json)
    }

    pub fn from_json_slice_owned<J: AsRef<[u8]>>(json: J) -> serde_json::Result<UploadPolicy<'static>> {
        serde_json::from_slice(json.as_ref())
    }
}

impl Default for UploadPolicy<'_> {
    fn default() -> Self {
        UploadPolicy {
            scope: None,
            is_prefixal_scope: None,
            deadline: None,
            insert_only: None,
            return_url: None,
            return_body: None,
            callback_url: None,
            callback_host: None,
            callback_body: None,
            callback_body_type: None,
            save_key: None,
            force_save_key: None,
            fsize_min: None,
            fsize_limit: None,
            detect_mime: None,
            mime_limit: None,
            file_type: None,
            delete_after_days: None,
        }
    }
}

#[derive(Debug)]
pub struct UploadPolicyBuilder<'p> {
    inner: UploadPolicy<'p>,
}

impl<'p> UploadPolicyBuilder<'p> {
    pub fn from(policy: UploadPolicy<'p>) -> UploadPolicyBuilder<'p> {
        UploadPolicyBuilder { inner: policy }
    }

    pub fn new_policy_for_bucket(bucket: impl Into<Cow<'p, str>>, config: &Config) -> UploadPolicyBuilder<'p> {
        UploadPolicyBuilder {
            inner: UploadPolicy {
                scope: Some(bucket.into()),
                ..Default::default()
            },
        }
        .token_lifetime(config.upload_token_lifetime())
    }

    pub fn new_policy_for_object(
        bucket: impl Into<String>,
        key: impl AsRef<str>,
        config: &Config,
    ) -> UploadPolicyBuilder<'p> {
        UploadPolicyBuilder {
            inner: UploadPolicy {
                scope: Some((bucket.into() + ":" + key.as_ref()).into()),
                ..Default::default()
            },
        }
        .token_lifetime(config.upload_token_lifetime())
    }

    pub fn new_policy_for_objects_with_prefix(
        bucket: impl Into<String>,
        prefix: impl AsRef<str>,
        config: &Config,
    ) -> UploadPolicyBuilder<'p> {
        UploadPolicyBuilder {
            inner: UploadPolicy {
                scope: Some((bucket.into() + ":" + prefix.as_ref()).into()),
                is_prefixal_scope: Some(1),
                ..Default::default()
            },
        }
        .token_lifetime(config.upload_token_lifetime())
    }

    pub fn token_lifetime(mut self, lifetime: Duration) -> UploadPolicyBuilder<'p> {
        self.inner.deadline = Some(
            SystemTime::now()
                .checked_add(lifetime)
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .and_then(|t| t.as_secs().try_into().ok())
                .unwrap_or(u32::max_value()),
        );
        self
    }

    pub fn token_deadline(mut self, deadline: SystemTime) -> UploadPolicyBuilder<'p> {
        self.inner.deadline = Some(
            deadline
                .duration_since(SystemTime::UNIX_EPOCH)
                .ok()
                .and_then(|t| t.as_secs().try_into().ok())
                .unwrap_or(u32::max_value()),
        );
        self
    }

    pub fn insert_only(mut self) -> UploadPolicyBuilder<'p> {
        self.inner.insert_only = Some(bool_utils::bool_to_int(true));
        self
    }

    pub fn overwritable(mut self) -> UploadPolicyBuilder<'p> {
        self.inner.insert_only = None;
        self
    }

    pub fn enable_mime_detection(mut self) -> UploadPolicyBuilder<'p> {
        self.inner.detect_mime = Some(bool_utils::bool_to_int(true));
        self
    }

    pub fn disable_mime_detection(mut self) -> UploadPolicyBuilder<'p> {
        self.inner.detect_mime = None;
        self
    }

    pub fn infrequent_storage(mut self) -> UploadPolicyBuilder<'p> {
        self.inner.file_type = Some(bool_utils::bool_to_int(true));
        self
    }

    pub fn normal_storage(mut self) -> UploadPolicyBuilder<'p> {
        self.inner.file_type = None;
        self
    }

    pub fn return_url(mut self, url: impl Into<Cow<'p, str>>) -> UploadPolicyBuilder<'p> {
        self.inner.return_url = Some(url.into());
        self
    }

    pub fn return_body(mut self, body: impl Into<Cow<'p, str>>) -> UploadPolicyBuilder<'p> {
        self.inner.return_body = Some(body.into());
        self
    }

    pub fn callback_urls<'a>(
        mut self,
        urls: impl AsRef<[&'a str]>,
        host: impl Into<Cow<'p, str>>,
    ) -> UploadPolicyBuilder<'p> {
        self.inner.callback_url = Some(urls.as_ref().join(";").into());
        self.inner.callback_host = {
            let callback_host = host.into();
            if callback_host.is_empty() {
                None
            } else {
                Some(callback_host)
            }
        };
        self
    }

    pub fn callback_body(
        mut self,
        body: impl Into<Cow<'p, str>>,
        body_type: impl Into<Cow<'p, str>>,
    ) -> UploadPolicyBuilder<'p> {
        self.inner.callback_body = Some(body.into());
        self.inner.callback_body_type = {
            let callback_body_type = body_type.into();
            if callback_body_type.is_empty() {
                None
            } else {
                Some(callback_body_type)
            }
        };
        self
    }

    pub fn save_as(mut self, key: impl Into<Cow<'p, str>>, force: bool) -> UploadPolicyBuilder<'p> {
        self.inner.save_key = Some(key.into());
        if force {
            self.inner.force_save_key = Some(true);
        }
        self
    }

    pub fn file_size_limitation(mut self, size: impl RangeBounds<usize>) -> UploadPolicyBuilder<'p> {
        self.inner.fsize_min = match size.start_bound() {
            Bound::Included(&s) => Some(s),
            Bound::Excluded(&s) => Some(s + 1),
            Bound::Unbounded => None,
        };
        self.inner.fsize_limit = match size.end_bound() {
            Bound::Included(&s) => Some(s),
            Bound::Excluded(&s) => Some(s - 1),
            Bound::Unbounded => None,
        };
        self
    }

    pub fn mime_types<'a>(mut self, content_types: impl AsRef<[&'a str]>) -> UploadPolicyBuilder<'p> {
        self.inner.mime_limit = Some(content_types.as_ref().join(";").into());
        self
    }

    pub fn object_lifetime(mut self, lifetime: Duration) -> UploadPolicyBuilder<'p> {
        let lifetime_secs = lifetime.as_secs();
        let secs_one_day = 60 * 60 * 24;

        self.inner.delete_after_days = Some(
            lifetime_secs
                .checked_add(secs_one_day)
                .and_then(|s| s.checked_sub(1))
                .and_then(|s| s.checked_div(secs_one_day))
                .and_then(|s| s.try_into().ok())
                .unwrap_or(usize::max_value()),
        );
        self
    }

    pub fn object_deadline(self, deadline: SystemTime) -> UploadPolicyBuilder<'p> {
        self.object_lifetime(
            deadline
                .duration_since(SystemTime::now())
                .unwrap_or_else(|_| Duration::from_secs(0)),
        )
    }

    pub fn build(self) -> UploadPolicy<'p> {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::{boxed::Box, error::Error, result::Result};

    #[test]
    fn test_build_upload_policy_for_bucket() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default()).build();
        let now = SystemTime::now();
        let one_hour_later = now + Duration::from_secs(60 * 60);
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), None);
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH)?
                - policy
                    .token_deadline()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                < Duration::from_secs(5)
        );

        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v.as_object().unwrap().len(), 2);
        assert_eq!(v["scope"], "test_bucket");
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH)?
                - Duration::from_secs(v["deadline"].as_u64().unwrap())
                < Duration::from_secs(5)
        );
        assert_eq!(v["isPrefixalScope"], json!(null));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_for_object() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_object("test_bucket", "test:object", &Config::default()).build();
        let now = SystemTime::now();
        let one_hour_later = now + Duration::from_secs(60 * 60);
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), Some("test:object"));
        assert!(!policy.use_prefixal_object_key());
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH)?
                - policy
                    .token_deadline()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                < Duration::from_secs(5)
        );

        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v.as_object().unwrap().len(), 2);
        assert_eq!(v["scope"], "test_bucket:test:object");
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH)?
                - Duration::from_secs(v["deadline"].as_u64().unwrap())
                < Duration::from_secs(5)
        );
        assert_eq!(v["isPrefixalScope"], json!(null));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_for_objects_with_prefix() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_objects_with_prefix("test_bucket", "test:object", &Config::default())
                .build();
        let now = SystemTime::now();
        let one_hour_later = now + Duration::from_secs(60 * 60);
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), Some("test:object"));
        assert!(policy.use_prefixal_object_key());
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH)?
                - policy
                    .token_deadline()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                < Duration::from_secs(5)
        );

        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v.as_object().unwrap().len(), 3);
        assert_eq!(v["scope"], "test_bucket:test:object");
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH)?
                - Duration::from_secs(v["deadline"].as_u64().unwrap())
                < Duration::from_secs(5)
        );
        assert_eq!(v["isPrefixalScope"], json!(1));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_deadline() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .token_deadline(SystemTime::now())
            .build();
        assert!(
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?
                - policy
                    .token_deadline()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                < Duration::from_secs(5)
        );

        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert!(
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?
                - Duration::from_secs(v["deadline"].as_u64().unwrap())
                < Duration::from_secs(5)
        );
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_lifetime() -> Result<(), Box<dyn Error>> {
        let one_day = Duration::from_secs(60 * 60 * 24);
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .token_lifetime(one_day)
            .build();
        let now = SystemTime::now();
        let tomorrow = now + one_day;
        assert!(
            tomorrow.duration_since(SystemTime::UNIX_EPOCH)?
                - policy
                    .token_deadline()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                < Duration::from_secs(5)
        );

        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert!(
            tomorrow.duration_since(SystemTime::UNIX_EPOCH)? - Duration::from_secs(v["deadline"].as_u64().unwrap())
                < Duration::from_secs(5)
        );
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_lifetime_overflow() -> Result<(), Box<dyn Error>> {
        let future = Duration::from_secs(u64::max_value());
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .token_lifetime(future)
            .build();
        assert!(
            policy
                .token_deadline()
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)?
                > SystemTime::now()
                    .checked_add(Duration::from_secs(50 * 365 * 24 * 60 * 60))
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)?
        );
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_insert_only() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .insert_only()
            .build();
        assert_eq!(policy.is_insert_only(), true);
        assert_eq!(policy.is_overwritable(), false);
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["insertOnly"], 1);
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_overwritable() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .overwritable()
            .build();
        assert_eq!(policy.is_insert_only(), false);
        assert_eq!(policy.is_overwritable(), true);
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["insertOnly"], json!(null));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_mime_detection() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .enable_mime_detection()
            .build();
        assert_eq!(policy.mime_detection_enabled(), true);
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["detectMime"], 1);
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_normal_storage() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .normal_storage()
            .build();
        assert_eq!(policy.is_normal_storage_used(), true);
        assert_eq!(policy.is_infrequent_storage_used(), false);
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["fileType"], json!(null));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_infrequent_storage() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .infrequent_storage()
            .build();
        assert_eq!(policy.is_normal_storage_used(), false);
        assert_eq!(policy.is_infrequent_storage_used(), true);
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["fileType"], 1);
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_return_url() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .return_url("http://www.qiniu.io/test")
            .build();
        assert_eq!(policy.return_url(), Some("http://www.qiniu.io/test"));
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["returnUrl"], "http://www.qiniu.io/test");
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_return_body() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .return_body("datadatadata")
            .build();
        assert_eq!(policy.return_body(), Some("datadatadata"));
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["returnBody"], "datadatadata");
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_callback_urls() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .callback_urls(
                &["https://1.1.1.1", "https://2.2.2.2", "https://3.3.3.3"],
                "www.qiniu.com",
            )
            .build();
        assert_eq!(
            policy.callback_urls().map(|urls| urls.collect::<Vec<&str>>()),
            Some(vec!["https://1.1.1.1", "https://2.2.2.2", "https://3.3.3.3"])
        );
        assert_eq!(policy.callback_host(), Some("www.qiniu.com"));
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["callbackUrl"], "https://1.1.1.1;https://2.2.2.2;https://3.3.3.3");
        assert_eq!(v["callbackHost"], "www.qiniu.com");
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_callback_body() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .callback_body("a=b&c=d", "")
            .build();
        assert_eq!(policy.callback_body(), Some("a=b&c=d"));
        assert_eq!(policy.callback_body_type(), None);
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["callbackBody"], "a=b&c=d");
        assert_eq!(v["callbackBodyType"], json!(null));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_callback_body_with_body_type() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .callback_body("a=b&c=d", "application/x-www-form-urlencoded")
            .build();
        assert_eq!(policy.callback_body(), Some("a=b&c=d"));
        assert_eq!(policy.callback_body_type(), Some("application/x-www-form-urlencoded"));
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["callbackBody"], "a=b&c=d");
        assert_eq!(v["callbackBodyType"], "application/x-www-form-urlencoded");
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_save_key() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .save_as("target_file", false)
            .build();
        assert_eq!(policy.save_key(), Some("target_file"));
        assert_eq!(policy.is_save_key_forced(), false);
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["saveKey"], "target_file");
        assert_eq!(v["forceSaveKey"], json!(null));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_save_key_by_force() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .save_as("target_file", true)
            .build();
        assert_eq!(policy.save_key(), Some("target_file"));
        assert_eq!(policy.is_save_key_forced(), true);
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["saveKey"], "target_file");
        assert_eq!(v["forceSaveKey"], true);
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_file_size_exclusive_limit() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .file_size_limitation(15..20)
            .build();
        assert_eq!(policy.file_size_limitation(), (Some(15), Some(19)));
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["fsizeMin"], 15);
        assert_eq!(v["fsizeLimit"], 19);
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_file_size_inclusive_limit() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .file_size_limitation(15..=20)
            .build();
        assert_eq!(policy.file_size_limitation(), (Some(15), Some(20)));
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["fsizeMin"], 15);
        assert_eq!(v["fsizeLimit"], 20);
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_file_size_max_limit() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .file_size_limitation(..20)
            .build();
        assert_eq!(policy.file_size_limitation(), (None, Some(19)));
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["fsizeMin"], json!(null));
        assert_eq!(v["fsizeLimit"], 19);
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_file_size_min_limit() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .file_size_limitation(15..)
            .build();
        assert_eq!(policy.file_size_limitation(), (Some(15), None));
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["fsizeMin"], 15);
        assert_eq!(v["fsizeLimit"], json!(null));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_mime() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .mime_types(&["image/jpeg", "image/png"])
            .build();
        assert_eq!(
            policy.mime_types().map(|ops| ops.collect::<Vec<&str>>()),
            Some(vec!["image/jpeg", "image/png"])
        );
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["mimeLimit"], "image/jpeg;image/png");
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_object_lifetime() -> Result<(), Box<dyn Error>> {
        let one_hundred_days = Duration::from_secs(100 * 24 * 60 * 60);
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .object_lifetime(one_hundred_days)
            .build();
        assert_eq!(policy.object_lifetime(), Some(one_hundred_days));

        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["deleteAfterDays"], 100);
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_short_object_lifetime() -> Result<(), Box<dyn Error>> {
        let one_hundred_secs = Duration::from_secs(100);
        let one_day = Duration::from_secs(24 * 60 * 60);
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .object_lifetime(one_hundred_secs)
            .build();
        assert_eq!(policy.object_lifetime(), Some(one_day));

        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["deleteAfterDays"], 1);
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_object_deadline() -> Result<(), Box<dyn Error>> {
        let one_hundred_days = Duration::from_secs(100 * 24 * 60 * 60);
        let after_one_hundred_days = SystemTime::now() + one_hundred_days;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .object_lifetime(one_hundred_days)
            .build();
        assert!(
            policy
                .object_deadline()
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)?
                - after_one_hundred_days.duration_since(SystemTime::UNIX_EPOCH)?
                < Duration::from_secs(5)
        );
        Ok(())
    }
}
