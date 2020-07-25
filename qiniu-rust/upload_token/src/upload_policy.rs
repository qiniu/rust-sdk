use super::{FileType, InvalidFileType};
use assert_impl::assert_impl;
use serde_json::{
    json,
    map::{Keys as JSONMapKeys, Values as JSONMapValues},
    value::Index as JSONValueIndex,
    Value as JSONValue,
};
use std::{
    borrow::{Borrow, Cow},
    convert::{TryFrom, TryInto},
    fmt,
    hash::Hash,
    ops::{Bound, RangeBounds},
    str::Split,
    time::{Duration, SystemTime},
};

const SCOPE_KEY: &str = "scope";
const IS_PREFIXAL_SCOPE_KEY: &str = "isPrefixalScope";
const DEADLINE_KEY: &str = "deadline";
const INSERT_ONLY_KEY: &str = "insertOnly";
const RETURN_URL_KEY: &str = "returnUrl";
const RETURN_BODY_KEY: &str = "returnBody";
const CALLBACK_URL_KEY: &str = "callbackUrl";
const CALLBACK_HOST_KEY: &str = "callbackHost";
const CALLBACK_BODY_KEY: &str = "callbackBody";
const CALLBACK_BODY_TYPE_KEY: &str = "callbackBodyType";
const SAVE_KEY_KEY: &str = "saveKey";
const FORCE_SAVE_KEY_KEY: &str = "forceSaveKey";
const FSIZE_MIN_KEY: &str = "fsizeMin";
const FSIZE_LIMIT_KEY: &str = "fsizeLimit";
const DETECT_MIME_KEY: &str = "detectMime";
const MIME_LIMIT_KEY: &str = "mimeLimit";
const FILE_TYPE_KEY: &str = "fileType";
const DELETE_AFTER_DAYS_KEY: &str = "deleteAfterDays";

/// 上传策略
///
/// 可以点击[这里](https://developer.qiniu.com/kodo/manual/1206/put-policy)了解七牛安全机制。
#[derive(Clone, Eq, PartialEq)]
pub struct UploadPolicy {
    inner: JSONValue,
}

impl UploadPolicy {
    /// 存储空间约束
    pub fn bucket(&self) -> Option<&str> {
        self.get(SCOPE_KEY)
            .as_ref()
            .and_then(|s| s.as_str())
            .and_then(|s| s.splitn(2, ':').next())
    }

    /// 对象名称约束或对象名称前缀约束
    pub fn key(&self) -> Option<&str> {
        self.get(SCOPE_KEY)
            .as_ref()
            .and_then(|v| v.as_str())
            .and_then(|s| s.splitn(2, ':').nth(1))
    }

    /// 是否是对象名称前缀约束
    pub fn use_prefixal_object_key(&self) -> bool {
        self.get(IS_PREFIXAL_SCOPE_KEY)
            .and_then(|v| v.as_u64())
            .is_some()
    }

    /// 是否仅允许新增对象，不允许覆盖对象
    pub fn is_insert_only(&self) -> bool {
        self.get(INSERT_ONLY_KEY).and_then(|v| v.as_u64()).is_some()
    }

    /// 是否启用 MIME 类型自动检测
    pub fn mime_detection_enabled(&self) -> bool {
        self.get(DETECT_MIME_KEY).and_then(|v| v.as_u64()).is_some()
    }

    /// 上传凭证过期时间
    pub fn token_deadline(&self) -> Option<SystemTime> {
        self.get(DEADLINE_KEY).and_then(|v| v.as_u64()).map(|t| {
            SystemTime::UNIX_EPOCH
                .checked_add(Duration::from_secs(t))
                .unwrap()
        })
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
        self.get(RETURN_URL_KEY).and_then(|v| v.as_str())
    }

    /// 上传成功后，自定义七牛云最终返回给上传端的数据
    pub fn return_body(&self) -> Option<&str> {
        self.get(RETURN_BODY_KEY).and_then(|v| v.as_str())
    }

    /// 上传成功后，七牛云向业务服务器发送 POST 请求的 URL 列表
    pub fn callback_urls(&self) -> Option<Split<char>> {
        self.get(CALLBACK_URL_KEY)
            .and_then(|v| v.as_str())
            .map(|s| s.split(';'))
    }

    /// 上传成功后，七牛云向业务服务器发送回调请求时的 `Host`
    pub fn callback_host(&self) -> Option<&str> {
        self.get(CALLBACK_HOST_KEY).and_then(|v| v.as_str())
    }

    /// 上传成功后，七牛云向业务服务器发送回调请求时的内容
    ///
    /// 支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
    pub fn callback_body(&self) -> Option<&str> {
        self.get(CALLBACK_BODY_KEY).and_then(|v| v.as_str())
    }

    /// 上传成功后，七牛云向业务服务器发送回调通知 `callback_body()` 的 `Content-Type`
    ///
    /// 默认为 `application/x-www-form-urlencoded` ，也可设置为 `application/json` 。
    pub fn callback_body_type(&self) -> Option<&str> {
        self.get(CALLBACK_BODY_TYPE_KEY).and_then(|v| v.as_str())
    }

    /// 自定义对象名称
    ///
    /// 支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
    pub fn save_key(&self) -> Option<&str> {
        self.get(SAVE_KEY_KEY).and_then(|v| v.as_str())
    }

    /// 是否忽略客户端指定的对象名称，强制使用自定义对象名称 `save_key()` 进行文件命名
    pub fn is_save_key_forced(&self) -> bool {
        self.get(FORCE_SAVE_KEY_KEY)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// 限定上传文件尺寸的范围
    ///
    /// 返回的第一个元素为最小尺寸，第二个元素为最大尺寸，如果为 `None` 表示不限制，单位为字节
    pub fn file_size_limitation(&self) -> (Option<u64>, Option<u64>) {
        (
            self.get(FSIZE_MIN_KEY).and_then(|v| v.as_u64()),
            self.get(FSIZE_LIMIT_KEY).and_then(|v| v.as_u64()),
        )
    }

    /// 限定用户上传的文件类型
    ///
    /// 指定本字段值，七牛服务器会侦测文件内容以判断 MIME 类型，再用判断值跟指定值进行匹配，
    /// 匹配成功则允许上传，匹配失败则返回 403 状态码
    pub fn mime_types(&self) -> Option<Split<char>> {
        self.get(MIME_LIMIT_KEY)
            .and_then(|v| v.as_str())
            .map(|s| s.split(';'))
    }

    /// 文件类型
    pub fn file_type(&self) -> Result<FileType, InvalidFileType> {
        self.get(FILE_TYPE_KEY)
            .and_then(|v| v.as_u64())
            .and_then(|v| u8::try_from(v).ok())
            .unwrap_or(0)
            .try_into()
    }

    /// 对象生命周期
    ///
    /// 精确到天
    pub fn object_lifetime(&self) -> Option<Duration> {
        self.get(DELETE_AFTER_DAYS_KEY)
            .and_then(|v| v.as_u64())
            .map(|d| Duration::from_secs((d * 60 * 60 * 24).try_into().unwrap_or(u64::max_value())))
    }

    /// 对象生命结束时间
    ///
    /// 精确到天
    pub fn object_deadline(&self) -> Option<SystemTime> {
        self.object_lifetime().map(|t| SystemTime::now() + t)
    }

    /// 获取 JSON 格式的上传凭证
    pub fn as_json(&self) -> String {
        serde_json::to_string(&self.inner).unwrap()
    }

    /// 解析 JSON 格式的上传凭证
    pub fn from_json(json: impl AsRef<[u8]>) -> serde_json::Result<UploadPolicy> {
        serde_json::from_slice(json.as_ref()).map(|inner| UploadPolicy { inner })
    }

    /// 根据指定的上传凭证字段获取相应的值
    #[inline]
    pub fn get(&self, key: impl JSONValueIndex) -> Option<&JSONValue> {
        self.inner.get(key)
    }

    /// 获取上传凭证的字段迭代器
    #[inline]
    pub fn keys(&self) -> JSONMapKeys {
        self.inner.as_object().unwrap().keys()
    }

    /// 获取上传凭证的字段值的迭代器
    #[inline]
    pub fn values(&self) -> JSONMapValues {
        self.inner.as_object().unwrap().values()
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl fmt::Debug for UploadPolicy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// 上传策略生成器
///
/// 用于生成上传策略，一旦生成完毕，上传策略将无法被修改
#[derive(Clone)]
pub struct UploadPolicyBuilder {
    inner: JSONValue,
}

impl From<UploadPolicy> for UploadPolicyBuilder {
    fn from(policy: UploadPolicy) -> Self {
        Self {
            inner: policy.inner,
        }
    }
}

impl UploadPolicyBuilder {
    /// 为指定的存储空间生成的上传策略
    ///
    /// 允许用户上传文件到指定的存储空间，不限制上传客户端指定对象名称。
    ///
    /// 上传策略根据给出的客户端配置指定上传凭证有效期
    pub fn new_policy_for_bucket(
        bucket: impl Into<String>,
        upload_token_lifetime: Duration,
    ) -> Self {
        let mut policy = Self {
            inner: json!({
                SCOPE_KEY: bucket.into().into_boxed_str(),
            }),
        };
        policy.token_lifetime(upload_token_lifetime);
        policy
    }

    /// 为指定的存储空间和对象名称生成的上传策略
    ///
    /// 允许用户以指定的对象名称上传文件到指定的存储空间。
    /// 上传客户端不能指定与上传策略冲突的对象名称。
    ///
    /// 上传策略根据给出的客户端配置指定上传凭证有效期
    pub fn new_policy_for_object(
        bucket: impl Into<String>,
        key: impl AsRef<str>,
        upload_token_lifetime: Duration,
    ) -> Self {
        let mut policy = Self {
            inner: json!({
                SCOPE_KEY: bucket.into() + ":" + key.as_ref(),
            }),
        };
        policy.token_lifetime(upload_token_lifetime);
        policy
    }

    /// 为指定的存储空间和对象名称前缀生成的上传策略
    ///
    /// 允许用户以指定的对象名称前缀上传文件到指定的存储空间。
    /// 上传客户端指定包含该前缀的对象名称。
    ///
    /// 上传策略根据给出的客户端配置指定上传凭证有效期
    pub fn new_policy_for_objects_with_prefix(
        bucket: impl Into<String>,
        prefix: impl AsRef<str>,
        upload_token_lifetime: Duration,
    ) -> Self {
        let mut policy = Self {
            inner: json!({
                SCOPE_KEY: bucket.into() + ":" + prefix.as_ref(),
                IS_PREFIXAL_SCOPE_KEY: 1,
            }),
        };
        policy.token_lifetime(upload_token_lifetime);
        policy
    }

    /// 指定上传凭证有效期
    pub fn token_lifetime(&mut self, lifetime: Duration) -> &mut Self {
        self.set(
            DEADLINE_KEY.into(),
            JSONValue::Number(
                SystemTime::now()
                    .checked_add(lifetime)
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .and_then(|t| u64::try_from(t.as_secs()).ok())
                    .unwrap_or(u64::max_value())
                    .into(),
            ),
        )
    }

    /// 指定上传凭证过期时间
    pub fn token_deadline(&mut self, deadline: SystemTime) -> &mut Self {
        self.set(
            DEADLINE_KEY.into(),
            JSONValue::Number(
                deadline
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .ok()
                    .and_then(|t| u64::try_from(t.as_secs()).ok())
                    .unwrap_or(u64::max_value())
                    .into(),
            ),
        )
    }

    /// 仅允许创建新的对象，不允许覆盖和修改同名对象
    pub fn insert_only(&mut self) -> &mut Self {
        self.set(INSERT_ONLY_KEY.into(), JSONValue::Number(1.into()))
    }

    /// 启用 MIME 类型自动检测
    pub fn enable_mime_detection(&mut self) -> &mut Self {
        self.set(DETECT_MIME_KEY.into(), JSONValue::Number(1.into()))
    }

    /// 禁用 MIME 类型自动检测
    pub fn disable_mime_detection(&mut self) -> &mut Self {
        self.unset(DETECT_MIME_KEY)
    }

    /// 设置文件类型
    pub fn file_type(&mut self, file_type: FileType) -> &mut Self {
        self.set(
            FILE_TYPE_KEY.into(),
            JSONValue::Number(u8::from(file_type).into()),
        )
    }

    /// Web 端文件上传成功后，浏览器执行 303 跳转的 URL
    ///
    /// 通常用于表单上传。
    /// 文件上传成功后会跳转到 `<return_url>?upload_ret=<queryString>`，
    /// `<queryString>` 包含 `return_body()` 内容。
    /// 如不设置 `return_url`，则直接将 `return_body()` 的内容返回给客户端
    pub fn return_url(&mut self, url: impl Into<String>) -> &mut Self {
        self.set(RETURN_URL_KEY.into(), JSONValue::String(url.into()))
    }

    /// 上传成功后，自定义七牛云最终返回给上传端（在指定 `return_url()` 时是携带在跳转路径参数中）的数据
    ///
    /// 支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)。
    /// `return_body` 要求是合法的 JSON 文本。
    /// 例如 `{"key": $(key), "hash": $(etag), "w": $(imageInfo.width), "h": $(imageInfo.height)}`
    pub fn return_body(&mut self, body: impl Into<String>) -> &mut Self {
        self.set(RETURN_BODY_KEY.into(), JSONValue::String(body.into()))
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
        host: impl Into<String>,
        body: impl Into<String>,
        body_type: impl Into<String>,
    ) -> &mut Self {
        self.set(
            CALLBACK_URL_KEY.into(),
            JSONValue::String(urls.as_ref().join(";")),
        );
        {
            let callback_host = host.into();
            if callback_host.is_empty() {
                self.unset(CALLBACK_HOST_KEY);
            } else {
                self.set(CALLBACK_HOST_KEY.into(), JSONValue::String(callback_host));
            }
        }
        self.set(CALLBACK_BODY_KEY.into(), JSONValue::String(body.into()));
        {
            let callback_body_type = body_type.into();
            if callback_body_type.is_empty() {
                self.unset(CALLBACK_BODY_TYPE_KEY);
            } else {
                self.set(
                    CALLBACK_BODY_TYPE_KEY.into(),
                    JSONValue::String(callback_body_type),
                );
            }
        }
        self
    }

    /// 自定义对象名称
    ///
    /// 支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)。
    /// `force` 为 `false` 时，`save_as` 字段仅当用户上传的时候没有主动指定对象名时起作用，
    /// `force` 为 `true` 时，将强制按 `save_as` 字段的内容命名
    pub fn save_as(&mut self, save_as: impl Into<String>, force: bool) -> &mut Self {
        self.set(SAVE_KEY_KEY.into(), JSONValue::String(save_as.into()));
        if force {
            self.set(FORCE_SAVE_KEY_KEY.into(), JSONValue::Bool(true));
        } else {
            self.unset(FORCE_SAVE_KEY_KEY);
        }
        self
    }

    /// 限定上传文件尺寸的范围
    ///
    /// 单位为字节
    pub fn file_size_limitation(&mut self, size: impl RangeBounds<u64>) -> &mut Self {
        match size.start_bound() {
            Bound::Included(&s) => {
                self.set(FSIZE_MIN_KEY.into(), JSONValue::Number(s.into()));
            }
            Bound::Excluded(&s) => {
                self.set(FSIZE_MIN_KEY.into(), JSONValue::Number((s + 1).into()));
            }
            Bound::Unbounded => {
                self.unset(FSIZE_MIN_KEY);
            }
        }
        match size.end_bound() {
            Bound::Included(&s) => {
                self.set(FSIZE_LIMIT_KEY.into(), JSONValue::Number(s.into()));
            }
            Bound::Excluded(&s) => {
                self.set(FSIZE_LIMIT_KEY.into(), JSONValue::Number((s - 1).into()));
            }
            Bound::Unbounded => {
                self.unset(FSIZE_LIMIT_KEY);
            }
        }
        self
    }

    /// 限定用户上传的文件类型
    ///
    /// 指定本字段值，七牛服务器会侦测文件内容以判断 MIME 类型，再用判断值跟指定值进行匹配，
    /// 匹配成功则允许上传，匹配失败则返回 403 状态码
    pub fn mime_types<'a>(&mut self, content_types: impl AsRef<[&'a str]>) -> &mut Self {
        self.set(
            MIME_LIMIT_KEY.into(),
            JSONValue::String(content_types.as_ref().join(";")),
        )
    }

    /// 对象生命周期
    ///
    /// 精确到天
    pub fn object_lifetime(&mut self, lifetime: Duration) -> &mut Self {
        let lifetime_secs = lifetime.as_secs();
        let secs_one_day = 60 * 60 * 24;

        self.set(
            DELETE_AFTER_DAYS_KEY.into(),
            lifetime_secs
                .checked_add(secs_one_day)
                .and_then(|s| s.checked_sub(1))
                .and_then(|s| s.checked_div(secs_one_day))
                .and_then(|s| s.try_into().ok())
                .unwrap_or_else(|| JSONValue::Number(u64::max_value().into())),
        )
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

    #[inline]
    pub fn set(&mut self, k: String, v: JSONValue) -> &mut Self {
        self.inner.as_object_mut().unwrap().insert(k, v);
        self
    }

    #[inline]
    pub fn unset<Q>(&mut self, k: &Q) -> &mut Self
    where
        String: Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.inner.as_object_mut().unwrap().remove(k);
        self
    }

    /// 生成上传策略
    pub fn build(&self) -> UploadPolicy {
        UploadPolicy {
            inner: self.inner.clone(),
        }
    }

    /// 重置上传策略生成器
    ///
    /// 重置生成器使得生成器可以被多次复用
    pub fn reset(&mut self) {
        let immutable_keys = [SCOPE_KEY, DEADLINE_KEY, IS_PREFIXAL_SCOPE_KEY];
        self.inner = JSONValue::Object(
            self.inner
                .as_object()
                .unwrap()
                .iter()
                .filter_map(|(k, v)| {
                    immutable_keys
                        .iter()
                        .find(|&ik| k == ik)
                        .map(|_| (k.to_owned(), v.to_owned()))
                })
                .collect(),
        );
    }
}

impl fmt::Debug for UploadPolicyBuilder {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<'p> From<&'p UploadPolicy> for Cow<'p, UploadPolicy> {
    #[inline]
    fn from(policy: &'p UploadPolicy) -> Self {
        Cow::Borrowed(policy)
    }
}

impl From<UploadPolicy> for Cow<'_, UploadPolicy> {
    #[inline]
    fn from(policy: UploadPolicy) -> Self {
        Cow::Owned(policy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qiniu_utils::mime;
    use serde_json::json;
    use std::{boxed::Box, error::Error, result::Result};

    #[test]
    fn test_build_upload_policy_for_bucket() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .build();
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

        assert_eq!(policy.keys().len(), 2);
        assert_eq!(policy.get("scope"), Some(&json!("test_bucket")));
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH)?
                - Duration::from_secs(policy.get("deadline").unwrap().as_u64().unwrap())
                < Duration::from_secs(5)
        );
        assert!(policy.get("isPrefixalScope").is_none());
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_for_object() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_object(
            "test_bucket",
            "test:object",
            Duration::from_secs(3600),
        )
        .build();
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

        assert_eq!(policy.keys().len(), 2);
        assert_eq!(policy.get("scope"), Some(&json!("test_bucket:test:object")));
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH)?
                - Duration::from_secs(policy.get("deadline").unwrap().as_u64().unwrap())
                < Duration::from_secs(5)
        );
        assert!(policy.get("isPrefixalScope").is_none());
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_for_objects_with_prefix() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_objects_with_prefix(
            "test_bucket",
            "test:object",
            Duration::from_secs(3600),
        )
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

        assert_eq!(policy.keys().len(), 3);
        assert_eq!(policy.get("scope"), Some(&json!("test_bucket:test:object")));
        assert!(
            one_hour_later.duration_since(SystemTime::UNIX_EPOCH)?
                - Duration::from_secs(policy.get("deadline").unwrap().as_u64().unwrap())
                < Duration::from_secs(5)
        );
        assert_eq!(policy.get("isPrefixalScope"), Some(&json!(1)));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_deadline() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
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

        assert!(
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?
                - Duration::from_secs(policy.get("deadline").unwrap().as_u64().unwrap())
                < Duration::from_secs(5)
        );
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_lifetime() -> Result<(), Box<dyn Error>> {
        let one_day = Duration::from_secs(60 * 60 * 24);
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
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

        assert!(
            tomorrow.duration_since(SystemTime::UNIX_EPOCH)?
                - Duration::from_secs(policy.get("deadline").unwrap().as_u64().unwrap())
                < Duration::from_secs(5)
        );
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_insert_only() -> Result<(), Box<dyn Error>> {
        let policy = UploadPolicyBuilder::new_policy_for_object(
            "test_bucket",
            "test",
            Duration::from_secs(3600),
        )
        .insert_only()
        .build();
        assert_eq!(policy.is_insert_only(), true);
        assert_eq!(policy.get("insertOnly"), Some(&json!(1)));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_mime_detection() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .enable_mime_detection()
                .build();
        assert_eq!(policy.mime_detection_enabled(), true);
        assert_eq!(policy.get("detectMime"), Some(&json!(1)));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_normal_storage() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .file_type(FileType::Normal)
                .build();
        assert_eq!(policy.file_type()?, FileType::Normal);
        assert_eq!(policy.get("fileType"), Some(&json!(0)));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_infrequent_storage() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .file_type(FileType::InfrequentAccess)
                .build();
        assert_eq!(policy.file_type()?, FileType::InfrequentAccess);
        assert_eq!(policy.get("fileType"), Some(&json!(1)));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_return_url() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .return_url("http://www.qiniu.io/test")
                .build();
        assert_eq!(policy.return_url(), Some("http://www.qiniu.io/test"));
        assert_eq!(
            policy.get("returnUrl"),
            Some(&json!("http://www.qiniu.io/test"))
        );
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_return_body() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .return_body("datadatadata")
                .build();
        assert_eq!(policy.return_body(), Some("datadatadata"));
        assert_eq!(policy.get("returnBody"), Some(&json!("datadatadata")));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_callback() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .callback(
                    &["https://1.1.1.1", "https://2.2.2.2", "https://3.3.3.3"],
                    "www.qiniu.com",
                    "a=b&c=d",
                    "",
                )
                .build();
        assert_eq!(
            policy
                .callback_urls()
                .map(|urls| urls.collect::<Vec<&str>>()),
            Some(vec![
                "https://1.1.1.1",
                "https://2.2.2.2",
                "https://3.3.3.3"
            ])
        );
        assert_eq!(policy.callback_host(), Some("www.qiniu.com"));
        assert_eq!(policy.callback_body(), Some("a=b&c=d"));
        assert_eq!(policy.callback_body_type(), None);
        assert_eq!(
            policy.get("callbackUrl"),
            Some(&json!("https://1.1.1.1;https://2.2.2.2;https://3.3.3.3"))
        );
        assert_eq!(policy.get("callbackHost"), Some(&json!("www.qiniu.com")));
        assert_eq!(policy.get("callbackBody"), Some(&json!("a=b&c=d")));
        assert!(policy.get("callbackBodyType").is_none());
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_callback_body_with_body_type() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .callback(
                    &["https://1.1.1.1", "https://2.2.2.2", "https://3.3.3.3"],
                    "www.qiniu.com",
                    "a=b&c=d",
                    mime::FORM_MIME,
                )
                .build();
        assert_eq!(
            policy
                .callback_urls()
                .map(|urls| urls.collect::<Vec<&str>>()),
            Some(vec![
                "https://1.1.1.1",
                "https://2.2.2.2",
                "https://3.3.3.3"
            ])
        );
        assert_eq!(policy.callback_host(), Some("www.qiniu.com"));
        assert_eq!(policy.callback_body(), Some("a=b&c=d"));
        assert_eq!(policy.callback_body_type(), Some(mime::FORM_MIME));
        assert_eq!(
            policy.get("callbackUrl"),
            Some(&json!("https://1.1.1.1;https://2.2.2.2;https://3.3.3.3"))
        );
        assert_eq!(policy.get("callbackHost"), Some(&json!("www.qiniu.com")));
        assert_eq!(policy.get("callbackBody"), Some(&json!("a=b&c=d")));
        assert_eq!(
            policy.get("callbackBodyType"),
            Some(&json!(mime::FORM_MIME))
        );
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_save_key() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .save_as("target_file", false)
                .build();
        assert_eq!(policy.save_key(), Some("target_file"));
        assert_eq!(policy.is_save_key_forced(), false);
        assert_eq!(policy.get("saveKey"), Some(&json!("target_file")));
        assert!(policy.get("forceSaveKey").is_none());
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_save_key_by_force() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .save_as("target_file", true)
                .build();
        assert_eq!(policy.save_key(), Some("target_file"));
        assert_eq!(policy.is_save_key_forced(), true);
        assert_eq!(policy.get("saveKey"), Some(&json!("target_file")));
        assert_eq!(policy.get("forceSaveKey"), Some(&json!(true)));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_file_size_exclusive_limit() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .file_size_limitation(15..20)
                .build();
        assert_eq!(policy.file_size_limitation(), (Some(15), Some(19)));
        assert_eq!(policy.get("fsizeMin"), Some(&json!(15)));
        assert_eq!(policy.get("fsizeLimit"), Some(&json!(19)));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_file_size_inclusive_limit() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .file_size_limitation(15..=20)
                .build();
        assert_eq!(policy.file_size_limitation(), (Some(15), Some(20)));
        assert_eq!(policy.get("fsizeMin"), Some(&json!(15)));
        assert_eq!(policy.get("fsizeLimit"), Some(&json!(20)));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_file_size_max_limit() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .file_size_limitation(..20)
                .build();
        assert_eq!(policy.file_size_limitation(), (None, Some(19)));
        assert!(policy.get("fsizeMin").is_none());
        assert_eq!(policy.get("fsizeLimit"), Some(&json!(19)));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_file_size_min_limit() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .file_size_limitation(15..)
                .build();
        assert_eq!(policy.file_size_limitation(), (Some(15), None));
        assert_eq!(policy.get("fsizeMin"), Some(&json!(15)));
        assert!(policy.get("fsizeLimit").is_none());
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_mime() -> Result<(), Box<dyn Error>> {
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .mime_types(&["image/jpeg", "image/png"])
                .build();
        assert_eq!(
            policy.mime_types().map(|ops| ops.collect::<Vec<&str>>()),
            Some(vec!["image/jpeg", "image/png"])
        );
        assert_eq!(
            policy.get("mimeLimit"),
            Some(&json!("image/jpeg;image/png"))
        );
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_object_lifetime() -> Result<(), Box<dyn Error>> {
        let one_hundred_days = Duration::from_secs(100 * 24 * 60 * 60);
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .object_lifetime(one_hundred_days)
                .build();
        assert_eq!(policy.object_lifetime(), Some(one_hundred_days));

        assert_eq!(policy.get("deleteAfterDays"), Some(&json!(100)));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_short_object_lifetime() -> Result<(), Box<dyn Error>> {
        let one_hundred_secs = Duration::from_secs(100);
        let one_day = Duration::from_secs(24 * 60 * 60);
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
                .object_lifetime(one_hundred_secs)
                .build();
        assert_eq!(policy.object_lifetime(), Some(one_day));

        assert_eq!(policy.get("deleteAfterDays"), Some(&json!(1)));
        Ok(())
    }

    #[test]
    fn test_build_upload_policy_with_object_deadline() -> Result<(), Box<dyn Error>> {
        let one_hundred_days = Duration::from_secs(100 * 24 * 60 * 60);
        let after_one_hundred_days = SystemTime::now() + one_hundred_days;
        let policy =
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", Duration::from_secs(3600))
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
