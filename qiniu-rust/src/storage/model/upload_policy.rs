use crate::{config::Config, utils::bool as bool_utils};
use serde::{Deserialize, Serialize};
use std::{
    convert::TryInto,
    default::Default,
    ops::{Bound, RangeBounds},
    str::Split,
    time::{Duration, SystemTime},
};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UploadPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deadline: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    is_prefixal_scope: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    insert_only: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    end_user: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    return_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    return_body: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    callback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_body_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    persistent_ops: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    persistent_notify_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    persistent_pipeline: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    save_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    force_save_key: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    fsize_min: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fsize_limit: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    detect_mime: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mime_limit: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    file_type: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    delete_after_days: Option<usize>,
}

impl UploadPolicy {
    pub fn bucket(&self) -> Option<&str> {
        self.scope.as_ref().and_then(|s| s.splitn(2, ':').nth(0))
    }

    pub fn key(&self) -> Option<&str> {
        self.scope.as_ref().and_then(|s| s.splitn(2, ':').nth(1))
    }

    pub fn prefixal(&self) -> bool {
        bool_utils::int_to_bool(self.is_prefixal_scope.unwrap_or(0))
    }

    pub fn insert_only(&self) -> bool {
        bool_utils::int_to_bool(self.insert_only.unwrap_or(0))
    }

    pub fn overwritable(&self) -> bool {
        !self.insert_only()
    }

    pub fn auto_detect_mime(&self) -> bool {
        bool_utils::int_to_bool(self.detect_mime.unwrap_or(0))
    }

    pub fn deadline(&self) -> Option<SystemTime> {
        self.deadline
            .map(|t| SystemTime::UNIX_EPOCH + Duration::from_secs(t.into()))
    }

    pub fn lifetime(&self) -> Option<Duration> {
        self.deadline().map(|t| {
            t.duration_since(SystemTime::now())
                .unwrap_or_else(|_| Duration::from_secs(0))
        })
    }

    pub fn end_user(&self) -> Option<&str> {
        Self::to_optional_str(&self.end_user)
    }

    pub fn return_url(&self) -> Option<&str> {
        Self::to_optional_str(&self.return_url)
    }

    pub fn return_body(&self) -> Option<&str> {
        Self::to_optional_str(&self.return_body)
    }

    pub fn callback_urls(&self) -> Option<Split<char>> {
        Self::to_optional_splited_str(&self.callback_url, ';')
    }

    pub fn callback_host(&self) -> Option<&str> {
        Self::to_optional_str(&self.callback_host)
    }

    pub fn callback_body(&self) -> Option<&str> {
        Self::to_optional_str(&self.callback_body)
    }

    pub fn callback_body_type(&self) -> Option<&str> {
        Self::to_optional_str(&self.callback_body_type)
    }

    pub fn persistent_ops(&self) -> Option<Split<char>> {
        Self::to_optional_splited_str(&self.persistent_ops, ';')
    }

    pub fn persistent_notify_url(&self) -> Option<&str> {
        Self::to_optional_str(&self.persistent_notify_url)
    }

    pub fn persistent_pipeline(&self) -> Option<&str> {
        Self::to_optional_str(&self.persistent_pipeline)
    }

    pub fn save_key(&self) -> Option<&str> {
        Self::to_optional_str(&self.save_key)
    }

    pub fn force_save_key(&self) -> bool {
        self.force_save_key.unwrap_or(false)
    }

    pub fn file_size(&self) -> (Option<usize>, Option<usize>) {
        (self.fsize_min, self.fsize_limit)
    }

    pub fn mime(&self) -> Option<Split<char>> {
        Self::to_optional_splited_str(&self.mime_limit, ';')
    }

    pub fn normal_storage(&self) -> bool {
        !self.infrequent_storage()
    }

    pub fn infrequent_storage(&self) -> bool {
        bool_utils::int_to_bool(self.file_type.unwrap_or(0))
    }

    pub fn file_lifetime(&self) -> Option<Duration> {
        self.delete_after_days
            .map(|d| Duration::from_secs((d * 60 * 60 * 24).try_into().unwrap_or_else(|_| u64::max_value())))
    }

    pub fn file_deadline(&self) -> Option<SystemTime> {
        self.file_lifetime().map(|t| SystemTime::now() + t)
    }

    fn to_optional_str(s: &Option<String>) -> Option<&str> {
        s.as_ref().map(|s| s.as_str())
    }

    fn to_optional_splited_str(s: &Option<String>, pat: char) -> Option<Split<char>> {
        s.as_ref().map(|x| x.split(pat))
    }

    pub fn as_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    pub fn from_json<S: AsRef<str>>(json: S) -> serde_json::Result<UploadPolicy> {
        serde_json::from_str(json.as_ref())
    }

    pub fn from_json_slice<S: AsRef<[u8]>>(json: S) -> serde_json::Result<UploadPolicy> {
        serde_json::from_slice(json.as_ref())
    }
}

impl Default for UploadPolicy {
    fn default() -> Self {
        UploadPolicy {
            scope: None,
            is_prefixal_scope: None,
            deadline: None,
            insert_only: None,
            end_user: None,
            return_url: None,
            return_body: None,
            callback_url: None,
            callback_host: None,
            callback_body: None,
            callback_body_type: None,
            persistent_ops: None,
            persistent_notify_url: None,
            persistent_pipeline: None,
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
pub struct UploadPolicyBuilder {
    inner: UploadPolicy,
}

impl UploadPolicyBuilder {
    pub fn from(policy: UploadPolicy) -> UploadPolicyBuilder {
        UploadPolicyBuilder { inner: policy }
    }

    pub fn new_policy_for_bucket<B: Into<String>>(bucket: B, config: &Config) -> UploadPolicyBuilder {
        let builder = UploadPolicyBuilder {
            inner: UploadPolicy {
                scope: Some(bucket.into()),
                ..Default::default()
            },
        };
        builder.token_lifetime(config.upload_token_lifetime())
    }

    pub fn new_policy_for_file<B: Into<String>, K: AsRef<str>>(
        bucket: B,
        key: K,
        config: &Config,
    ) -> UploadPolicyBuilder {
        let builder = UploadPolicyBuilder {
            inner: UploadPolicy {
                scope: Some(bucket.into() + ":" + key.as_ref()),
                ..Default::default()
            },
        };
        builder.token_lifetime(config.upload_token_lifetime())
    }

    pub fn new_policy_for_file_name_with_prefix<B: Into<String>, K: AsRef<str>>(
        bucket: B,
        prefix: K,
        config: &Config,
    ) -> UploadPolicyBuilder {
        let builder = UploadPolicyBuilder {
            inner: UploadPolicy {
                scope: Some(bucket.into() + ":" + prefix.as_ref()),
                is_prefixal_scope: Some(1),
                ..Default::default()
            },
        };
        builder.token_lifetime(config.upload_token_lifetime())
    }

    pub fn token_lifetime(mut self, lifetime: Duration) -> UploadPolicyBuilder {
        self.inner.deadline = Some(
            SystemTime::now()
                .checked_add(lifetime)
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .and_then(|t| t.as_secs().try_into().ok())
                .unwrap_or_else(|| u32::max_value()),
        );
        self
    }

    pub fn token_deadline(mut self, deadline: SystemTime) -> UploadPolicyBuilder {
        self.inner.deadline = Some(
            deadline
                .duration_since(SystemTime::UNIX_EPOCH)
                .ok()
                .and_then(|t| t.as_secs().try_into().ok())
                .unwrap_or_else(|| u32::max_value()),
        );
        self
    }

    pub fn insert_only(mut self) -> UploadPolicyBuilder {
        self.inner.insert_only = Some(bool_utils::bool_to_int(true));
        self
    }

    pub fn overwritable(mut self) -> UploadPolicyBuilder {
        self.inner.insert_only = Some(bool_utils::bool_to_int(false));
        self
    }

    pub fn auto_detect_mime(mut self) -> UploadPolicyBuilder {
        self.inner.detect_mime = Some(bool_utils::bool_to_int(true));
        self
    }

    pub fn normal_storage(mut self) -> UploadPolicyBuilder {
        self.inner.file_type = Some(bool_utils::bool_to_int(false));
        self
    }

    pub fn infrequent_storage(mut self) -> UploadPolicyBuilder {
        self.inner.file_type = Some(bool_utils::bool_to_int(true));
        self
    }

    pub fn return_url<U: Into<String>>(mut self, url: U) -> UploadPolicyBuilder {
        self.inner.return_url = Some(url.into());
        self
    }

    pub fn return_body<B: Into<String>>(mut self, body: B) -> UploadPolicyBuilder {
        self.inner.return_body = Some(body.into());
        self
    }

    pub fn callback_urls<US: AsRef<[U]>, U: AsRef<str>, H: Into<String>>(
        mut self,
        urls: US,
        host: Option<H>,
    ) -> UploadPolicyBuilder {
        self.inner.callback_url = Some(
            urls.as_ref()
                .iter()
                .map(|u| u.as_ref())
                .collect::<Vec<&str>>()
                .join(";"),
        );
        self.inner.callback_host = host.map(|h| h.into());
        self
    }

    pub fn callback_body<B: Into<String>, BT: Into<String>>(
        mut self,
        body: B,
        body_type: Option<BT>,
    ) -> UploadPolicyBuilder {
        self.inner.callback_body = Some(body.into());
        self.inner.callback_body_type = body_type.map(|bt| bt.into());
        self
    }

    pub fn persistent_ops<Ops: AsRef<[Op]>, Op: AsRef<str>, U: Into<String>, P: Into<String>>(
        mut self,
        ops: Ops,
        notify_url: Option<U>,
        pipeline: Option<P>,
    ) -> UploadPolicyBuilder {
        self.inner.persistent_ops = Some(ops.as_ref().iter().map(|u| u.as_ref()).collect::<Vec<&str>>().join(";"));
        self.inner.persistent_notify_url = notify_url.map(|u| u.into());
        self.inner.persistent_pipeline = pipeline.map(|p| p.into());
        self
    }

    pub fn save_as<K: Into<String>>(mut self, key: K, force: bool) -> UploadPolicyBuilder {
        self.inner.save_key = Some(key.into());
        if force {
            self.inner.force_save_key = Some(true);
        }
        self
    }

    pub fn file_size<R: RangeBounds<usize>>(mut self, size: R) -> UploadPolicyBuilder {
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

    pub fn mime<Ts: AsRef<[T]>, T: AsRef<str>>(mut self, content_types: Ts) -> UploadPolicyBuilder {
        self.inner.mime_limit = Some(
            content_types
                .as_ref()
                .iter()
                .map(|u| u.as_ref())
                .collect::<Vec<&str>>()
                .join(";"),
        );
        self
    }

    pub fn file_lifetime(mut self, lifetime: Duration) -> UploadPolicyBuilder {
        let lifetime_secs = lifetime.as_secs();
        let secs_one_day = 60 * 60 * 24;

        self.inner.delete_after_days = Some(
            lifetime_secs
                .checked_add(secs_one_day)
                .and_then(|s| s.checked_sub(1))
                .and_then(|s| s.checked_div(secs_one_day))
                .and_then(|s| s.try_into().ok())
                .unwrap_or_else(|| usize::max_value()),
        );
        self
    }

    pub fn file_deadline(self, deadline: SystemTime) -> UploadPolicyBuilder {
        self.file_lifetime(
            deadline
                .duration_since(SystemTime::now())
                .unwrap_or_else(|_| Duration::from_secs(0)),
        )
    }

    pub fn build(self) -> UploadPolicy {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn test_build_upload_policy_for_bucket() {
        let one_hour = Duration::from_secs(60 * 60);
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default()).build();
        let now = SystemTime::now();
        let one_hour_later = now + one_hour;
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), None);
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH).unwrap()
                - policy
                    .deadline()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                < Duration::from_secs(5)
        );

        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v.as_object().unwrap().len(), 2);
        assert_eq!(v["scope"], "test_bucket");
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH).unwrap()
                - Duration::from_secs(v["deadline"].as_u64().unwrap())
                < Duration::from_secs(5)
        );
        assert_eq!(v["isPrefixalScope"], json!(null));
    }

    #[test]
    fn test_build_upload_policy_for_file() {
        let one_hour = Duration::from_secs(60 * 60);
        let policy = UploadPolicyBuilder::new_policy_for_file("test_bucket", "test:file", &Config::default()).build();
        let now = SystemTime::now();
        let one_hour_later = now + one_hour;
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), Some("test:file"));
        assert!(!policy.prefixal());
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH).unwrap()
                - policy
                    .deadline()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                < Duration::from_secs(5)
        );

        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v.as_object().unwrap().len(), 2);
        assert_eq!(v["scope"], "test_bucket:test:file");
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH).unwrap()
                - Duration::from_secs(v["deadline"].as_u64().unwrap())
                < Duration::from_secs(5)
        );
        assert_eq!(v["isPrefixalScope"], json!(null));
    }

    #[test]
    fn test_build_upload_policy_for_file_name_with_prefix() {
        let one_hour = Duration::from_secs(60 * 60);
        let policy =
            UploadPolicyBuilder::new_policy_for_file_name_with_prefix("test_bucket", "test:file", &Config::default())
                .build();
        let now = SystemTime::now();
        let one_hour_later = now + one_hour;
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), Some("test:file"));
        assert!(policy.prefixal());
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH).unwrap()
                - policy
                    .deadline()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                < Duration::from_secs(5)
        );

        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v.as_object().unwrap().len(), 3);
        assert_eq!(v["scope"], "test_bucket:test:file");
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH).unwrap()
                - Duration::from_secs(v["deadline"].as_u64().unwrap())
                < Duration::from_secs(5)
        );
        assert_eq!(v["isPrefixalScope"], json!(1));
    }

    #[test]
    fn test_build_upload_policy_with_deadline() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .token_deadline(SystemTime::now())
            .build();
        assert!(
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap()
                - policy
                    .deadline()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                < Duration::from_secs(5)
        );

        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert!(
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap()
                - Duration::from_secs(v["deadline"].as_u64().unwrap())
                < Duration::from_secs(5)
        );
    }

    #[test]
    fn test_build_upload_policy_with_lifetime() {
        let one_day = Duration::from_secs(60 * 60 * 24);
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .token_lifetime(one_day)
            .build();
        let now = SystemTime::now();
        let tomorrow = now + one_day;
        assert!(
            tomorrow.duration_since(SystemTime::UNIX_EPOCH).unwrap()
                - policy
                    .deadline()
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                < Duration::from_secs(5)
        );

        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert!(
            tomorrow.duration_since(SystemTime::UNIX_EPOCH).unwrap()
                - Duration::from_secs(v["deadline"].as_u64().unwrap())
                < Duration::from_secs(5)
        );
    }

    #[test]
    fn test_build_upload_policy_with_lifetime_overflow() {
        let future = Duration::from_secs(u64::max_value());
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .token_lifetime(future)
            .build();
        assert!(
            policy
                .deadline()
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                > SystemTime::now()
                    .checked_add(Duration::from_secs(50 * 365 * 24 * 60 * 60))
                    .unwrap()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
        );
    }

    #[test]
    fn test_build_upload_policy_with_insert_only() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .insert_only()
            .build();
        assert_eq!(policy.insert_only(), true);
        assert_eq!(policy.overwritable(), false);
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["insertOnly"], 1);
    }

    #[test]
    fn test_build_upload_policy_with_overwritable() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .overwritable()
            .build();
        assert_eq!(policy.insert_only(), false);
        assert_eq!(policy.overwritable(), true);
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["insertOnly"], 0);
    }

    #[test]
    fn test_build_upload_policy_with_auto_detect_mime() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .auto_detect_mime()
            .build();
        assert_eq!(policy.auto_detect_mime(), true);
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["detectMime"], 1);
    }

    #[test]
    fn test_build_upload_policy_with_normal_storage() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .normal_storage()
            .build();
        assert_eq!(policy.normal_storage(), true);
        assert_eq!(policy.infrequent_storage(), false);
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["fileType"], 0);
    }

    #[test]
    fn test_build_upload_policy_with_infrequent_storage() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .infrequent_storage()
            .build();
        assert_eq!(policy.normal_storage(), false);
        assert_eq!(policy.infrequent_storage(), true);
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["fileType"], 1);
    }

    #[test]
    fn test_build_upload_policy_with_return_url() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .return_url("http://www.qiniu.io/test")
            .build();
        assert_eq!(policy.return_url(), Some("http://www.qiniu.io/test"));
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["returnUrl"], "http://www.qiniu.io/test");
    }

    #[test]
    fn test_build_upload_policy_with_return_body() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .return_body("datadatadata")
            .build();
        assert_eq!(policy.return_body(), Some("datadatadata"));
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["returnBody"], "datadatadata");
    }

    #[test]
    fn test_build_upload_policy_with_callback_urls() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .callback_urls(
                &["https://1.1.1.1", "https://2.2.2.2", "https://3.3.3.3"],
                Some("www.qiniu.com"),
            )
            .build();
        assert_eq!(
            policy.callback_urls().map(|urls| urls.collect::<Vec<&str>>()),
            Some(vec!["https://1.1.1.1", "https://2.2.2.2", "https://3.3.3.3"])
        );
        assert_eq!(policy.callback_host(), Some("www.qiniu.com"));
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["callbackUrl"], "https://1.1.1.1;https://2.2.2.2;https://3.3.3.3");
        assert_eq!(v["callbackHost"], "www.qiniu.com");
    }

    #[test]
    fn test_build_upload_policy_with_callback_body() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .callback_body("a=b&c=d", None::<String>)
            .build();
        assert_eq!(policy.callback_body(), Some("a=b&c=d"));
        assert_eq!(policy.callback_body_type(), None);
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["callbackBody"], "a=b&c=d");
        assert_eq!(v["callbackBodyType"], json!(null));
    }

    #[test]
    fn test_build_upload_policy_with_callback_body_with_body_type() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .callback_body("a=b&c=d", Some("application/x-www-form-urlencoded"))
            .build();
        assert_eq!(policy.callback_body(), Some("a=b&c=d"));
        assert_eq!(policy.callback_body_type(), Some("application/x-www-form-urlencoded"));
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["callbackBody"], "a=b&c=d");
        assert_eq!(v["callbackBodyType"], "application/x-www-form-urlencoded");
    }

    #[test]
    fn test_build_upload_policy_with_persistent_ops() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .persistent_ops(&["avthumb/mp4", "avthumb/m3u8"], None::<String>, None::<String>)
            .build();
        assert_eq!(
            policy.persistent_ops().map(|ops| ops.collect::<Vec<&str>>()),
            Some(vec!["avthumb/mp4", "avthumb/m3u8"])
        );
        assert_eq!(policy.persistent_notify_url(), None);
        assert_eq!(policy.persistent_pipeline(), None);
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["persistentOps"], "avthumb/mp4;avthumb/m3u8");
        assert_eq!(v["persistentNotifyUrl"], json!(null));
        assert_eq!(v["persistentPipeline"], json!(null));
    }

    #[test]
    fn test_build_upload_policy_with_persistent_ops_with_notify_url() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .persistent_ops(
                &["avthumb/mp4", "avthumb/m3u8"],
                Some("http://www.qiniu.com/pfop"),
                None::<String>,
            )
            .build();
        assert_eq!(
            policy.persistent_ops().map(|ops| ops.collect::<Vec<&str>>()),
            Some(vec!["avthumb/mp4", "avthumb/m3u8"])
        );
        assert_eq!(policy.persistent_notify_url(), Some("http://www.qiniu.com/pfop"));
        assert_eq!(policy.persistent_pipeline(), None);
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["persistentOps"], "avthumb/mp4;avthumb/m3u8");
        assert_eq!(v["persistentNotifyUrl"], "http://www.qiniu.com/pfop");
        assert_eq!(v["persistentPipeline"], json!(null));
    }

    #[test]
    fn test_build_upload_policy_with_persistent_ops_with_pipeline() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .persistent_ops(&["avthumb/mp4", "avthumb/m3u8"], None::<String>, Some("pipeline"))
            .build();
        assert_eq!(
            policy.persistent_ops().map(|ops| ops.collect::<Vec<&str>>()),
            Some(vec!["avthumb/mp4", "avthumb/m3u8"])
        );
        assert_eq!(policy.persistent_notify_url(), None);
        assert_eq!(policy.persistent_pipeline(), Some("pipeline"));
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["persistentOps"], "avthumb/mp4;avthumb/m3u8");
        assert_eq!(v["persistentNotifyUrl"], json!(null));
        assert_eq!(v["persistentPipeline"], "pipeline");
    }

    #[test]
    fn test_build_upload_policy_with_save_key() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .save_as("target_file", false)
            .build();
        assert_eq!(policy.save_key(), Some("target_file"));
        assert_eq!(policy.force_save_key(), false);
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["saveKey"], "target_file");
        assert_eq!(v["forceSaveKey"], json!(null));
    }

    #[test]
    fn test_build_upload_policy_with_save_key_by_force() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .save_as("target_file", true)
            .build();
        assert_eq!(policy.save_key(), Some("target_file"));
        assert_eq!(policy.force_save_key(), true);
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["saveKey"], "target_file");
        assert_eq!(v["forceSaveKey"], true);
    }

    #[test]
    fn test_build_upload_policy_with_file_size_exclusive_limit() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .file_size(15..20)
            .build();
        assert_eq!(policy.file_size(), (Some(15), Some(19)));
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["fsizeMin"], 15);
        assert_eq!(v["fsizeLimit"], 19);
    }

    #[test]
    fn test_build_upload_policy_with_file_size_inclusive_limit() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .file_size(15..=20)
            .build();
        assert_eq!(policy.file_size(), (Some(15), Some(20)));
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["fsizeMin"], 15);
        assert_eq!(v["fsizeLimit"], 20);
    }

    #[test]
    fn test_build_upload_policy_with_file_size_max_limit() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .file_size(..20)
            .build();
        assert_eq!(policy.file_size(), (None, Some(19)));
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["fsizeMin"], json!(null));
        assert_eq!(v["fsizeLimit"], 19);
    }

    #[test]
    fn test_build_upload_policy_with_file_size_min_limit() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .file_size(15..)
            .build();
        assert_eq!(policy.file_size(), (Some(15), None));
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["fsizeMin"], 15);
        assert_eq!(v["fsizeLimit"], json!(null));
    }

    #[test]
    fn test_build_upload_policy_with_mime() {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .mime(&["image/jpeg", "image/png"])
            .build();
        assert_eq!(
            policy.mime().map(|ops| ops.collect::<Vec<&str>>()),
            Some(vec!["image/jpeg", "image/png"])
        );
        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["mimeLimit"], "image/jpeg;image/png");
    }

    #[test]
    fn test_build_upload_policy_with_file_lifetime() {
        let one_hundred_days = Duration::from_secs(100 * 24 * 60 * 60);
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .file_lifetime(one_hundred_days)
            .build();
        assert_eq!(policy.file_lifetime(), Some(one_hundred_days));

        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["deleteAfterDays"], 100);
    }

    #[test]
    fn test_build_upload_policy_with_short_file_lifetime() {
        let one_hundred_secs = Duration::from_secs(100);
        let one_day = Duration::from_secs(24 * 60 * 60);
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .file_lifetime(one_hundred_secs)
            .build();
        assert_eq!(policy.file_lifetime(), Some(one_day));

        let v: Value = serde_json::from_str(policy.as_json().as_str()).unwrap();
        assert_eq!(v["deleteAfterDays"], 1);
    }

    #[test]
    fn test_build_upload_policy_with_file_deadline() {
        let one_hundred_days = Duration::from_secs(100 * 24 * 60 * 60);
        let after_one_hundred_days = SystemTime::now() + one_hundred_days;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .file_lifetime(one_hundred_days)
            .build();
        assert!(
            policy
                .file_deadline()
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                - after_one_hundred_days.duration_since(SystemTime::UNIX_EPOCH).unwrap()
                < Duration::from_secs(5)
        );
    }
}
