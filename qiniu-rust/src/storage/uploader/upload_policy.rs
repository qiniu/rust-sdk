//! 上传策略模块
//!
//! 负责解析和生成上传策略

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
use thiserror::Error;

/// 上传策略
///
/// 可以点击[这里](https://developer.qiniu.com/kodo/manual/1206/put-policy)了解七牛安全机制。
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
    /// 存储空间约束
    pub fn bucket(&self) -> Option<&str> {
        self.scope.as_ref().and_then(|s| s.splitn(2, ':').nth(0))
    }

    /// 对象名称约束或对象名称前缀约束
    pub fn key(&self) -> Option<&str> {
        self.scope.as_ref().and_then(|s| s.splitn(2, ':').nth(1))
    }

    /// 是否是对象名称前缀约束
    pub fn use_prefixal_object_key(&self) -> bool {
        bool_utils::int_to_bool(self.is_prefixal_scope.unwrap_or(0))
    }

    /// 是否仅允许新增对象，不允许覆盖对象
    pub fn is_insert_only(&self) -> bool {
        bool_utils::int_to_bool(self.insert_only.unwrap_or(0))
    }

    /// 允许覆盖对象
    ///
    /// 相当于 `!is_insert_only()`
    pub fn is_overwritable(&self) -> bool {
        !self.is_insert_only()
    }

    /// 是否启用 MIME 类型自动检测
    pub fn mime_detection_enabled(&self) -> bool {
        bool_utils::int_to_bool(self.detect_mime.unwrap_or(0))
    }

    /// 上传凭证过期时间
    pub fn token_deadline(&self) -> Option<SystemTime> {
        self.deadline
            .map(|t| SystemTime::UNIX_EPOCH + Duration::from_secs(t.into()))
    }

    /// 上传凭证有效期
    pub fn token_lifetime(&self) -> Option<Duration> {
        self.token_deadline().map(|t| {
            t.duration_since(SystemTime::now())
                .unwrap_or_else(|_| Duration::from_secs(0))
        })
    }

    /// Web 端文件上传成功后，浏览器执行 303 跳转的 URL
    pub fn return_url(&self) -> Option<&str> {
        Self::convert_to_optional_str(&self.return_url)
    }

    /// 上传成功后，自定义七牛云最终返回给上传端的数据
    pub fn return_body(&self) -> Option<&str> {
        Self::convert_to_optional_str(&self.return_body)
    }

    /// 上传成功后，七牛云向业务服务器发送 POST 请求的 URL 列表
    pub fn callback_urls(&self) -> Option<Split<char>> {
        Self::convert_to_optional_splited_str(&self.callback_url, ';')
    }

    /// 上传成功后，七牛云向业务服务器发送回调请求时的 `Host`
    pub fn callback_host(&self) -> Option<&str> {
        Self::convert_to_optional_str(&self.callback_host)
    }

    /// 上传成功后，七牛云向业务服务器发送回调请求时的内容
    ///
    /// 支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
    pub fn callback_body(&self) -> Option<&str> {
        Self::convert_to_optional_str(&self.callback_body)
    }

    /// 上传成功后，七牛云向业务服务器发送回调通知 `callback_body()` 的 `Content-Type`
    ///
    /// 默认为 `application/x-www-form-urlencoded` ，也可设置为 `application/json` 。
    pub fn callback_body_type(&self) -> Option<&str> {
        Self::convert_to_optional_str(&self.callback_body_type)
    }

    /// 自定义对象名称
    ///
    /// 支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
    pub fn save_key(&self) -> Option<&str> {
        Self::convert_to_optional_str(&self.save_key)
    }

    /// 是否忽略客户端指定的对象名称，强制使用自定义对象名称 `save_key()` 进行文件命名
    pub fn is_save_key_forced(&self) -> bool {
        self.force_save_key.unwrap_or(false)
    }

    /// 限定上传文件尺寸的范围
    ///
    /// 返回的第一个元素为最小尺寸，第二个元素为最大尺寸，如果为 `None` 表示不限制，单位为字节
    pub fn file_size_limitation(&self) -> (Option<usize>, Option<usize>) {
        (self.fsize_min, self.fsize_limit)
    }

    /// 限定用户上传的文件类型
    ///
    /// 指定本字段值，七牛服务器会侦测文件内容以判断 MIME 类型，再用判断值跟指定值进行匹配，
    /// 匹配成功则允许上传，匹配失败则返回 403 状态码
    pub fn mime_types(&self) -> Option<Split<char>> {
        Self::convert_to_optional_splited_str(&self.mime_limit, ';')
    }

    /// 是否会使用标准存储
    pub fn is_normal_storage_used(&self) -> bool {
        !self.is_infrequent_storage_used()
    }

    /// 是否会使用低频存储
    pub fn is_infrequent_storage_used(&self) -> bool {
        bool_utils::int_to_bool(self.file_type.unwrap_or(0))
    }

    /// 对象生命周期
    ///
    /// 精确到天
    pub fn object_lifetime(&self) -> Option<Duration> {
        self.delete_after_days
            .map(|d| Duration::from_secs((d * 60 * 60 * 24).try_into().unwrap_or(u64::max_value())))
    }

    /// 对象生命结束时间
    ///
    /// 精确到天
    pub fn object_deadline(&self) -> Option<SystemTime> {
        self.object_lifetime().map(|t| SystemTime::now() + t)
    }

    fn convert_to_optional_str<'a>(s: &'a Option<Cow<'p, str>>) -> Option<&'a str> {
        s.as_ref().map(|s| s.as_ref())
    }

    fn convert_to_optional_splited_str<'a>(s: &'a Option<Cow<'p, str>>, pat: char) -> Option<Split<'a, char>> {
        s.as_ref().map(|x| x.split(pat))
    }

    /// 获取 JSON 格式的上传凭证
    pub fn as_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    /// 解析 JSON 格式的上传凭证
    pub fn from_json(json: impl AsRef<[u8]>) -> serde_json::Result<UploadPolicy<'static>> {
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

/// 上传策略生成器
///
/// 用于生成上传策略，一旦生成完毕，上传策略将无法被修改
#[derive(Debug)]
pub struct UploadPolicyBuilder<'p> {
    inner: UploadPolicy<'p>,
}

impl<'p> From<UploadPolicy<'p>> for UploadPolicyBuilder<'p> {
    fn from(policy: UploadPolicy<'p>) -> Self {
        Self { inner: policy }
    }
}

impl<'p> UploadPolicyBuilder<'p> {
    /// 为指定的存储空间生成的上传策略
    ///
    /// 允许用户上传文件到指定的存储空间，不限制上传客户端指定对象名称。
    /// 且这种模式下生成的上传策略将被自动指定 `insert_only()`，且不允许指定 `overwritable()`，
    /// 因此上传时不能通过覆盖的方式修改同名对象。
    ///
    /// 上传策略根据给出的客户端配置指定上传凭证有效期
    pub fn new_policy_for_bucket(bucket: impl Into<Cow<'p, str>>, config: &Config) -> Self {
        let mut policy = Self {
            inner: UploadPolicy {
                scope: Some(bucket.into()),
                ..Default::default()
            },
        };
        policy.token_lifetime(config.upload_token_lifetime()).insert_only();
        policy
    }

    /// 为指定的存储空间和对象名称生成的上传策略
    ///
    /// 允许用户以指定的对象名称上传文件到指定的存储空间。
    /// 上传客户端不能指定与上传策略冲突的对象名称。
    /// 且这种模式下生成的上传策略将被自动指定 `overwritable()`，
    /// 如果不希望允许同名对象被覆盖和修改，则应该调用 `insert_only()`。
    ///
    /// 上传策略根据给出的客户端配置指定上传凭证有效期
    pub fn new_policy_for_object(bucket: impl Into<String>, key: impl AsRef<str>, config: &Config) -> Self {
        let mut policy = Self {
            inner: UploadPolicy {
                scope: Some((bucket.into() + ":" + key.as_ref()).into()),
                ..Default::default()
            },
        };
        policy.token_lifetime(config.upload_token_lifetime());
        policy
    }

    /// 为指定的存储空间和对象名称前缀生成的上传策略
    ///
    /// 允许用户以指定的对象名称前缀上传文件到指定的存储空间。
    /// 上传客户端指定包含该前缀的对象名称。
    /// 且这种模式下生成的上传策略将被自动指定 `overwritable()`，
    /// 如果不希望允许同名对象被覆盖和修改，则应该调用 `insert_only()`。
    ///
    /// 上传策略根据给出的客户端配置指定上传凭证有效期
    pub fn new_policy_for_objects_with_prefix(
        bucket: impl Into<String>,
        prefix: impl AsRef<str>,
        config: &Config,
    ) -> Self {
        let mut policy = Self {
            inner: UploadPolicy {
                scope: Some((bucket.into() + ":" + prefix.as_ref()).into()),
                is_prefixal_scope: Some(1),
                ..Default::default()
            },
        };
        policy.token_lifetime(config.upload_token_lifetime());
        policy
    }

    /// 指定上传凭证有效期
    pub fn token_lifetime(&mut self, lifetime: Duration) -> &mut Self {
        self.inner.deadline = Some(
            SystemTime::now()
                .checked_add(lifetime)
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .and_then(|t| t.as_secs().try_into().ok())
                .unwrap_or(u32::max_value()),
        );
        self
    }

    /// 指定上传凭证过期时间
    pub fn token_deadline(&mut self, deadline: SystemTime) -> &mut Self {
        self.inner.deadline = Some(
            deadline
                .duration_since(SystemTime::UNIX_EPOCH)
                .ok()
                .and_then(|t| t.as_secs().try_into().ok())
                .unwrap_or(u32::max_value()),
        );
        self
    }

    /// 仅允许创建新的对象，不允许覆盖和修改同名对象
    pub fn insert_only(&mut self) -> &mut Self {
        self.inner.insert_only = Some(bool_utils::bool_to_int(true));
        self
    }

    /// 允许覆盖和修改同名对象
    pub fn overwritable(&mut self) -> UploadPolicyResult<&mut Self> {
        if !self
            .inner
            .scope
            .as_ref()
            .map(|scope| scope.contains(':'))
            .unwrap_or(false)
        {
            return Err(UploadPolicyError::OverwritableIsForbidden);
        }
        self.inner.insert_only = None;
        Ok(self)
    }

    /// 启用 MIME 类型自动检测
    pub fn enable_mime_detection(&mut self) -> &mut Self {
        self.inner.detect_mime = Some(bool_utils::bool_to_int(true));
        self
    }

    /// 禁用 MIME 类型自动检测
    pub fn disable_mime_detection(&mut self) -> &mut Self {
        self.inner.detect_mime = None;
        self
    }

    /// 使用低频存储
    pub fn infrequent_storage(&mut self) -> &mut Self {
        self.inner.file_type = Some(bool_utils::bool_to_int(true));
        self
    }

    /// 使用标准存储
    pub fn normal_storage(&mut self) -> &mut Self {
        self.inner.file_type = None;
        self
    }

    /// Web 端文件上传成功后，浏览器执行 303 跳转的 URL
    ///
    /// 通常用于表单上传。
    /// 文件上传成功后会跳转到 `<return_url>?upload_ret=<queryString>`，
    /// `<queryString>` 包含 `return_body()` 内容。
    /// 如不设置 `return_url`，则直接将 `return_body()` 的内容返回给客户端
    pub fn return_url(&mut self, url: impl Into<Cow<'p, str>>) -> &mut Self {
        self.inner.return_url = Some(url.into());
        self
    }

    /// 上传成功后，自定义七牛云最终返回给上传端（在指定 `return_url()` 时是携带在跳转路径参数中）的数据
    ///
    /// 支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)。
    /// `return_body` 要求是合法的 JSON 文本。
    /// 例如 `{"key": $(key), "hash": $(etag), "w": $(imageInfo.width), "h": $(imageInfo.height)}`
    pub fn return_body(&mut self, body: impl Into<Cow<'p, str>>) -> &mut Self {
        self.inner.return_body = Some(body.into());
        self
    }

    /// 上传成功后，七牛云向业务服务器发送 POST 请求的 URL 列表，`Host`，回调请求的内容以及其 `Content-Type`
    ///
    /// 七牛服务器会在上传成功后逐一回调 URL 直到有一个成功为止
    ///
    /// 如果给出的 `host` 为空字符串，则使用默认的 `Host`
    ///
    /// `body` 参数必须不能为空，支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
    //
    /// `body_type` 参数表示 `body` 参数的 `Content-Type`，如果为空，则为默认的 `application/x-www-form-urlencoded`
    pub fn callback<'a>(
        &mut self,
        urls: impl AsRef<[&'a str]>,
        host: impl Into<Cow<'p, str>>,
        body: impl Into<Cow<'p, str>>,
        body_type: impl Into<Cow<'p, str>>,
    ) -> &mut Self {
        self.inner.callback_url = Some(urls.as_ref().join(";").into());
        self.inner.callback_host = {
            let callback_host = host.into();
            if callback_host.is_empty() {
                None
            } else {
                Some(callback_host)
            }
        };
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

    /// 自定义对象名称
    ///
    /// 支持支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)。
    /// `force` 为 `false` 时，`save_as` 字段仅当用户上传的时候没有主动指定对象名时起作用，
    /// `force` 为 `true` 时，将强制按 `save_as` 字段的格式命名
    pub fn save_as(&mut self, key: impl Into<Cow<'p, str>>, force: bool) -> &mut Self {
        self.inner.save_key = Some(key.into());
        if force {
            self.inner.force_save_key = Some(true);
        }
        self
    }

    /// 限定上传文件尺寸的范围
    ///
    /// 单位为字节
    pub fn file_size_limitation(&mut self, size: impl RangeBounds<usize>) -> &mut Self {
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

    /// 限定用户上传的文件类型
    ///
    /// 指定本字段值，七牛服务器会侦测文件内容以判断 MIME 类型，再用判断值跟指定值进行匹配，
    /// 匹配成功则允许上传，匹配失败则返回 403 状态码
    pub fn mime_types<'a>(&mut self, content_types: impl AsRef<[&'a str]>) -> &mut Self {
        self.inner.mime_limit = Some(content_types.as_ref().join(";").into());
        self
    }

    /// 对象生命周期
    ///
    /// 精确到天
    pub fn object_lifetime(&mut self, lifetime: Duration) -> &mut Self {
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

    /// 对象的生命到期时间
    ///
    /// 精确到天
    pub fn object_deadline(&mut self, deadline: SystemTime) -> &mut Self {
        self.object_lifetime(
            deadline
                .duration_since(SystemTime::now())
                .unwrap_or_else(|_| Duration::from_secs(0)),
        )
    }

    /// 生成上传策略
    pub fn build(&mut self) -> UploadPolicy<'p> {
        self.inner.clone()
    }
}

#[derive(Error, Debug)]
pub enum UploadPolicyError {
    #[error("Overwritable is forbidden for the policy")]
    OverwritableIsForbidden,
}

pub type UploadPolicyResult<T> = Result<T, UploadPolicyError>;

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
        assert_eq!(v.as_object().unwrap().len(), 3);
        assert_eq!(v["scope"], "test_bucket");
        assert_eq!(v["insertOnly"], 1);
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
        let policy = UploadPolicyBuilder::new_policy_for_object("test_bucket", "test", &Config::default())
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
        let policy = UploadPolicyBuilder::new_policy_for_object("test_bucket", "test", &Config::default())
            .overwritable()?
            .build();
        assert_eq!(policy.is_insert_only(), false);
        assert_eq!(policy.is_overwritable(), true);
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["insertOnly"], json!(null));
        Ok(())
    }

    #[test]
    fn test_build_bucket_level_upload_policy_with_overwritable() -> Result<(), Box<dyn Error>> {
        UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .overwritable()
            .unwrap_err();
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
    fn test_build_upload_policy_with_callback() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .callback(
                &["https://1.1.1.1", "https://2.2.2.2", "https://3.3.3.3"],
                "www.qiniu.com",
                "a=b&c=d",
                "",
            )
            .build();
        assert_eq!(
            policy.callback_urls().map(|urls| urls.collect::<Vec<&str>>()),
            Some(vec!["https://1.1.1.1", "https://2.2.2.2", "https://3.3.3.3"])
        );
        assert_eq!(policy.callback_host(), Some("www.qiniu.com"));
        assert_eq!(policy.callback_body(), Some("a=b&c=d"));
        assert_eq!(policy.callback_body_type(), None);
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["callbackUrl"], "https://1.1.1.1;https://2.2.2.2;https://3.3.3.3");
        assert_eq!(v["callbackHost"], "www.qiniu.com");
        assert_eq!(v["callbackBody"], "a=b&c=d");
        assert_eq!(v["callbackBodyType"], json!(null));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_callback_body_with_body_type() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &Config::default())
            .callback(
                &["https://1.1.1.1", "https://2.2.2.2", "https://3.3.3.3"],
                "www.qiniu.com",
                "a=b&c=d",
                "application/x-www-form-urlencoded",
            )
            .build();
        assert_eq!(
            policy.callback_urls().map(|urls| urls.collect::<Vec<&str>>()),
            Some(vec!["https://1.1.1.1", "https://2.2.2.2", "https://3.3.3.3"])
        );
        assert_eq!(policy.callback_host(), Some("www.qiniu.com"));
        assert_eq!(policy.callback_body(), Some("a=b&c=d"));
        assert_eq!(policy.callback_body_type(), Some("application/x-www-form-urlencoded"));
        let v: Value = serde_json::from_str(policy.as_json().as_str())?;
        assert_eq!(v["callbackUrl"], "https://1.1.1.1;https://2.2.2.2;https://3.3.3.3");
        assert_eq!(v["callbackHost"], "www.qiniu.com");
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
