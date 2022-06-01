use super::{mime::Mime, Bucket};
use anyhow::{Error as AnyError, Result as AnyResult};
use assert_impl::assert_impl;
use auto_impl::auto_impl;
use dyn_clonable::clonable;
use indexmap::IndexMap;
use qiniu_apis::{
    http::{ResponseError as HttpResponseError, ResponseErrorKind as HttpResponseErrorKind},
    http_client::{ApiResult, RegionsProviderEndpoints, RequestBuilderParts, Response},
    upload_token::FileType,
};
use qiniu_utils::base64::urlsafe;
use std::{
    fmt::{self, Debug, Display},
    mem::take,
    sync::{Arc, Mutex},
};

macro_rules! impl_call_methods {
    ($mod_name:ident) => {
        impl_call_methods!($mod_name, entry);
    };
    ($mod_name:ident, $entry:ident) => {
        /// 阻塞发起操作请求
        ///
        /// 该方法的异步版本为 [`Self::async_call`]。
        pub fn call(&mut self) -> ApiResult<Response<qiniu_apis::storage::$mod_name::ResponseBody>> {
            let op = self.build();
            let mut request = op
                .$entry
                .bucket
                .objects_manager()
                .client()
                .storage()
                .$mod_name()
                .new_request(
                    RegionsProviderEndpoints::new(op.$entry.bucket.region_provider()?),
                    op.to_path_params(),
                    op.$entry.bucket.objects_manager().credential(),
                );
            if let Some(callback) = &self.before_request_callback {
                let mut callback = callback.lock().unwrap();
                callback(request.parts_mut()).map_err(make_callback_error)?;
            }
            request.call()
        }

        #[cfg(feature = "async")]
        #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
        /// 异步发起操作请求
        pub async fn async_call(&mut self) -> ApiResult<Response<qiniu_apis::storage::$mod_name::ResponseBody>> {
            let op = self.build();
            let mut request = op
                .$entry
                .bucket
                .objects_manager()
                .client()
                .storage()
                .$mod_name()
                .new_async_request(
                    RegionsProviderEndpoints::new(op.$entry.bucket.region_provider()?),
                    op.to_path_params(),
                    op.$entry.bucket.objects_manager().credential(),
                );
            if let Some(callback) = &self.before_request_callback {
                let mut callback = callback.lock().unwrap();
                callback(request.parts_mut()).map_err(make_callback_error)?;
            }
            request.call().await
        }

        #[allow(dead_code)]
        fn assert() {
            assert_impl!(Send: Self);
            assert_impl!(Sync: Self);
        }
    };
}

type BeforeRequestCallback<'c> =
    Arc<Mutex<dyn FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'c>>;

#[derive(Clone, Debug)]
pub(super) struct Entry<'a> {
    bucket: &'a Bucket,
    object: &'a str,
}

impl<'a> Entry<'a> {
    pub(super) fn new(bucket: &'a Bucket, object: &'a str) -> Self {
        Self { bucket, object }
    }

    fn encode(&self) -> String {
        urlsafe(self.to_string().as_bytes())
    }
}

impl Display for Entry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.bucket.name(), self.object)
    }
}

#[derive(Clone, Debug)]
struct SimpleEntry<'a> {
    bucket: &'a str,
    object: &'a str,
}

impl<'a> SimpleEntry<'a> {
    pub(super) fn new(bucket: &'a str, object: &'a str) -> Self {
        Self { bucket, object }
    }

    fn encode(&self) -> String {
        urlsafe(self.to_string().as_bytes())
    }
}

impl Display for SimpleEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.bucket, self.object)
    }
}

/// 对象操作提供者接口
#[clonable]
#[auto_impl(&mut, Box)]
pub trait OperationProvider: Clone + Debug + Sync + Send {
    /// 转换为对象操作命令
    fn to_operation(&mut self) -> String;
}

#[derive(Clone, Debug)]
pub(super) struct StatObject<'a> {
    entry: Entry<'a>,
}

impl StatObject<'_> {
    pub(super) fn builder(entry: Entry) -> StatObjectBuilder {
        StatObjectBuilder {
            inner: StatObject { entry },
            before_request_callback: None,
        }
    }

    fn to_path_params(&self) -> qiniu_apis::storage::stat_object::PathParams {
        qiniu_apis::storage::stat_object::PathParams::default().set_entry_as_str(self.entry.to_string())
    }
}

impl Display for StatObject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stat/{}", self.entry.encode())
    }
}

/// 对象元信息获取操作构建器
///
/// 可以通过 [`crate::Bucket::stat_object`] 方法获取该构建器。
#[derive(Clone)]
pub struct StatObjectBuilder<'a> {
    inner: StatObject<'a>,
    before_request_callback: Option<BeforeRequestCallback<'a>>,
}

impl<'a> StatObjectBuilder<'a> {
    /// 设置请求前回调函数
    #[inline]
    pub fn before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callback = Some(Arc::new(Mutex::new(callback)));
        self
    }

    fn build(&mut self) -> StatObject<'a> {
        StatObject {
            entry: self.inner.entry.to_owned(),
        }
    }

    impl_call_methods!(stat_object);
}

impl OperationProvider for StatObjectBuilder<'_> {
    fn to_operation(&mut self) -> String {
        self.build().to_string()
    }
}

impl Debug for StatObjectBuilder<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StatObjectBuilder").field("inner", &self.inner).finish()
    }
}

#[derive(Clone, Debug)]
pub(super) struct MoveObject<'a> {
    from_entry: Entry<'a>,
    to_entry: SimpleEntry<'a>,
    is_force: bool,
}

impl MoveObject<'_> {
    pub(super) fn builder<'a>(from_entry: Entry<'a>, to_bucket: &'a str, to_object: &'a str) -> MoveObjectBuilder<'a> {
        MoveObjectBuilder {
            inner: MoveObject {
                from_entry,
                to_entry: SimpleEntry::new(to_bucket, to_object),
                is_force: false,
            },
            before_request_callback: None,
        }
    }

    fn to_path_params(&self) -> qiniu_apis::storage::move_object::PathParams {
        qiniu_apis::storage::move_object::PathParams::default()
            .set_src_entry_as_str(self.from_entry.to_string())
            .set_dest_entry_as_str(self.to_entry.to_string())
            .set_is_force_as_bool(self.is_force)
    }
}

impl Display for MoveObject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "move/{}/{}", self.from_entry.encode(), self.to_entry.encode())?;
        if self.is_force {
            write!(f, "/force/true")?;
        }
        Ok(())
    }
}

/// 对象移动操作构建器
///
/// 可以通过 [`crate::Bucket::move_object_to`] 方法获取该构建器。
#[derive(Clone)]
pub struct MoveObjectBuilder<'a> {
    inner: MoveObject<'a>,
    before_request_callback: Option<BeforeRequestCallback<'a>>,
}

impl<'a> MoveObjectBuilder<'a> {
    /// 是否强制移动
    #[inline]
    pub fn is_force(&mut self, is_force: bool) -> &mut Self {
        self.inner.is_force = is_force;
        self
    }

    /// 设置请求前回调函数
    #[inline]
    pub fn before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callback = Some(Arc::new(Mutex::new(callback)));
        self
    }

    fn build(&mut self) -> MoveObject<'a> {
        MoveObject {
            from_entry: self.inner.from_entry.to_owned(),
            to_entry: self.inner.to_entry.to_owned(),
            is_force: take(&mut self.inner.is_force),
        }
    }

    impl_call_methods!(move_object, from_entry);
}

impl OperationProvider for MoveObjectBuilder<'_> {
    fn to_operation(&mut self) -> String {
        self.build().to_string()
    }
}

impl Debug for MoveObjectBuilder<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MoveObjectBuilder").field("inner", &self.inner).finish()
    }
}

#[derive(Clone, Debug)]
pub(super) struct CopyObject<'a> {
    from_entry: Entry<'a>,
    to_entry: SimpleEntry<'a>,
    is_force: bool,
}

impl CopyObject<'_> {
    pub(super) fn builder<'a>(from_entry: Entry<'a>, to_bucket: &'a str, to_object: &'a str) -> CopyObjectBuilder<'a> {
        CopyObjectBuilder {
            inner: CopyObject {
                from_entry,
                to_entry: SimpleEntry::new(to_bucket, to_object),
                is_force: false,
            },
            before_request_callback: None,
        }
    }

    fn to_path_params(&self) -> qiniu_apis::storage::copy_object::PathParams {
        qiniu_apis::storage::copy_object::PathParams::default()
            .set_src_entry_as_str(self.from_entry.to_string())
            .set_dest_entry_as_str(self.to_entry.to_string())
            .set_is_force_as_bool(self.is_force)
    }
}

impl Display for CopyObject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "copy/{}/{}", self.from_entry.encode(), self.to_entry.encode())?;
        if self.is_force {
            write!(f, "/force/true")?;
        }
        Ok(())
    }
}

/// 对象复制操作构建器
///
/// 可以通过 [`crate::Bucket::copy_object_to`] 方法获取该构建器。
#[derive(Clone)]
pub struct CopyObjectBuilder<'a> {
    inner: CopyObject<'a>,
    before_request_callback: Option<BeforeRequestCallback<'a>>,
}

impl<'a> CopyObjectBuilder<'a> {
    /// 是否强制复制
    #[inline]
    pub fn is_force(&mut self, is_force: bool) -> &mut Self {
        self.inner.is_force = is_force;
        self
    }

    /// 设置请求前回调函数
    #[inline]
    pub fn before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callback = Some(Arc::new(Mutex::new(callback)));
        self
    }

    fn build(&mut self) -> CopyObject<'a> {
        CopyObject {
            from_entry: self.inner.from_entry.to_owned(),
            to_entry: self.inner.to_entry.to_owned(),
            is_force: take(&mut self.inner.is_force),
        }
    }

    impl_call_methods!(copy_object, from_entry);
}

impl OperationProvider for CopyObjectBuilder<'_> {
    fn to_operation(&mut self) -> String {
        self.build().to_string()
    }
}

impl Debug for CopyObjectBuilder<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CopyObjectBuilder").field("inner", &self.inner).finish()
    }
}

#[derive(Clone, Debug)]
pub(super) struct DeleteObject<'a> {
    entry: Entry<'a>,
}

impl DeleteObject<'_> {
    pub(super) fn builder(entry: Entry) -> DeleteObjectBuilder {
        DeleteObjectBuilder {
            inner: DeleteObject { entry },
            before_request_callback: None,
        }
    }

    fn to_path_params(&self) -> qiniu_apis::storage::delete_object::PathParams {
        qiniu_apis::storage::delete_object::PathParams::default().set_entry_as_str(self.entry.to_string())
    }
}

impl Display for DeleteObject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "delete/{}", self.entry.encode())
    }
}

/// 对象删除操作构建器
///
/// 可以通过 [`crate::Bucket::delete_object`] 方法获取该构建器。
#[derive(Clone)]
pub struct DeleteObjectBuilder<'a> {
    inner: DeleteObject<'a>,
    before_request_callback: Option<BeforeRequestCallback<'a>>,
}

impl<'a> DeleteObjectBuilder<'a> {
    fn build(&mut self) -> DeleteObject<'a> {
        self.inner.to_owned()
    }

    /// 设置请求前回调函数
    #[inline]
    pub fn before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callback = Some(Arc::new(Mutex::new(callback)));
        self
    }

    impl_call_methods!(delete_object);
}

impl OperationProvider for DeleteObjectBuilder<'_> {
    fn to_operation(&mut self) -> String {
        self.build().to_string()
    }
}

impl Debug for DeleteObjectBuilder<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeleteObjectBuilder")
            .field("inner", &self.inner)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(super) struct UnfreezeObject<'a> {
    entry: Entry<'a>,
    freeze_after_days: usize,
}

impl UnfreezeObject<'_> {
    pub(super) fn builder(entry: Entry, freeze_after_days: usize) -> UnfreezeObjectBuilder {
        UnfreezeObjectBuilder {
            inner: UnfreezeObject {
                entry,
                freeze_after_days,
            },
            before_request_callback: None,
        }
    }

    fn to_path_params(&self) -> qiniu_apis::storage::restore_archived_object::PathParams {
        qiniu_apis::storage::restore_archived_object::PathParams::default()
            .set_entry_as_str(self.entry.to_string())
            .set_freeze_after_days_as_usize(self.freeze_after_days)
    }
}

impl Display for UnfreezeObject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "restoreAr/{}/freezeAfterDays/{}",
            self.entry.encode(),
            self.freeze_after_days
        )
    }
}

/// 对象解冻操作构建器
#[derive(Clone)]
pub struct UnfreezeObjectBuilder<'a> {
    inner: UnfreezeObject<'a>,
    before_request_callback: Option<BeforeRequestCallback<'a>>,
}

impl<'a> UnfreezeObjectBuilder<'a> {
    /// 设置请求前回调函数
    #[inline]
    pub fn before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callback = Some(Arc::new(Mutex::new(callback)));
        self
    }

    fn build(&mut self) -> UnfreezeObject<'a> {
        self.inner.to_owned()
    }

    impl_call_methods!(restore_archived_object);
}

impl OperationProvider for UnfreezeObjectBuilder<'_> {
    fn to_operation(&mut self) -> String {
        self.build().to_string()
    }
}

impl Debug for UnfreezeObjectBuilder<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnfreezeObjectBuilder")
            .field("inner", &self.inner)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(super) struct SetObjectType<'a> {
    entry: Entry<'a>,
    object_type: FileType,
}

impl SetObjectType<'_> {
    pub(super) fn builder(entry: Entry, object_type: FileType) -> SetObjectTypeBuilder {
        SetObjectTypeBuilder {
            inner: SetObjectType { entry, object_type },
            before_request_callback: None,
        }
    }

    fn to_path_params(&self) -> qiniu_apis::storage::set_object_file_type::PathParams {
        qiniu_apis::storage::set_object_file_type::PathParams::default()
            .set_entry_as_str(self.entry.to_string())
            .set_type_as_usize(self.object_type.into())
    }
}

impl Display for SetObjectType<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chtype/{}/type/{}", self.entry.encode(), self.object_type)
    }
}

/// 对象类型设置操作构建器
///
/// 可以通过 [`crate::Bucket::set_object_type`] 方法获取该构建器。
#[derive(Clone)]
pub struct SetObjectTypeBuilder<'a> {
    inner: SetObjectType<'a>,
    before_request_callback: Option<BeforeRequestCallback<'a>>,
}

impl<'a> SetObjectTypeBuilder<'a> {
    /// 设置请求前回调函数
    #[inline]
    pub fn before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callback = Some(Arc::new(Mutex::new(callback)));
        self
    }

    fn build(&mut self) -> SetObjectType<'a> {
        self.inner.to_owned()
    }

    impl_call_methods!(set_object_file_type);
}

impl OperationProvider for SetObjectTypeBuilder<'_> {
    fn to_operation(&mut self) -> String {
        self.build().to_string()
    }
}

impl Debug for SetObjectTypeBuilder<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetObjectTypeBuilder")
            .field("inner", &self.inner)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(super) struct ModifyObjectStatus<'a> {
    entry: Entry<'a>,
    disabled: bool,
}

impl ModifyObjectStatus<'_> {
    pub(super) fn builder(entry: Entry, disabled: bool) -> ModifyObjectStatusBuilder {
        ModifyObjectStatusBuilder {
            inner: ModifyObjectStatus { entry, disabled },
            before_request_callback: None,
        }
    }

    fn to_path_params(&self) -> qiniu_apis::storage::modify_object_status::PathParams {
        qiniu_apis::storage::modify_object_status::PathParams::default()
            .set_entry_as_str(self.entry.to_string())
            .set_status_as_usize(if self.disabled { 1 } else { 0 })
    }
}

impl Display for ModifyObjectStatus<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let disabled = if self.disabled { "1" } else { "0" };
        write!(f, "chstatus/{}/status/{}", self.entry.encode(), disabled)
    }
}

/// 修改对象状态构建器
///
/// 可以通过 [`crate::Bucket::modify_object_status`] 方法获取该构建器。
#[derive(Clone)]
pub struct ModifyObjectStatusBuilder<'a> {
    inner: ModifyObjectStatus<'a>,
    before_request_callback: Option<BeforeRequestCallback<'a>>,
}

impl<'a> ModifyObjectStatusBuilder<'a> {
    /// 封禁对象
    #[inline]
    pub fn disable(&mut self, disable: bool) -> &mut Self {
        self.inner.disabled = disable;
        self
    }

    /// 设置请求前回调函数
    #[inline]
    pub fn before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callback = Some(Arc::new(Mutex::new(callback)));
        self
    }

    fn build(&mut self) -> ModifyObjectStatus<'a> {
        ModifyObjectStatus {
            entry: self.inner.entry.to_owned(),
            disabled: take(&mut self.inner.disabled),
        }
    }

    impl_call_methods!(modify_object_status);
}

impl OperationProvider for ModifyObjectStatusBuilder<'_> {
    fn to_operation(&mut self) -> String {
        self.build().to_string()
    }
}

impl Debug for ModifyObjectStatusBuilder<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModifyObjectStatusBuilder")
            .field("inner", &self.inner)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(super) struct ModifyObjectMetadata<'a> {
    entry: Entry<'a>,
    mime_type: Mime,
    metadata: IndexMap<String, String>,
    conditions: IndexMap<String, String>,
}

impl ModifyObjectMetadata<'_> {
    pub(super) fn builder(entry: Entry, mime_type: Mime) -> ModifyObjectMetadataBuilder {
        ModifyObjectMetadataBuilder {
            inner: ModifyObjectMetadata {
                entry,
                mime_type,
                metadata: Default::default(),
                conditions: Default::default(),
            },
            before_request_callback: None,
        }
    }

    fn to_path_params(&self) -> qiniu_apis::storage::modify_object_metadata::PathParams {
        let mut params = qiniu_apis::storage::modify_object_metadata::PathParams::default()
            .set_entry_as_str(self.entry.to_string())
            .set_mime_type_as_str(self.mime_type.to_string());
        if !self.conditions.is_empty() {
            params = params.set_condition_as_str(self.condition_string());
        }
        for (key, value) in self.metadata.iter() {
            params = params.append_meta_data_as_str(format!("x-qn-meta-{}", key), value.to_owned());
        }
        params
    }

    fn condition_string(&self) -> String {
        let conditions: Vec<_> = self
            .conditions
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect();
        conditions.join("&")
    }
}

impl Display for ModifyObjectMetadata<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "chgm/{}/mime/{}",
            self.entry.encode(),
            urlsafe(self.mime_type.as_ref().as_bytes())
        )?;
        for (key, value) in self.metadata.iter() {
            write!(f, "/x-qn-meta-{}/{}", key, urlsafe(value.as_bytes()))?;
        }
        if !self.conditions.is_empty() {
            write!(f, "/cond/{}", urlsafe(self.condition_string().as_bytes()))?;
        }
        Ok(())
    }
}

/// 修改对象元信息构建器
///
/// 可以通过 [`crate::Bucket::modify_object_metadata`] 方法获取该构建器。
#[derive(Clone)]
pub struct ModifyObjectMetadataBuilder<'a> {
    inner: ModifyObjectMetadata<'a>,
    before_request_callback: Option<BeforeRequestCallback<'a>>,
}

impl<'a> ModifyObjectMetadataBuilder<'a> {
    /// 添加元信息
    #[inline]
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.inner.metadata.insert(key.into(), value.into());
        self
    }

    /// 添加修改条件条件
    #[inline]
    pub fn add_condition(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.inner.conditions.insert(key.into(), value.into());
        self
    }

    /// 设置请求前回调函数
    #[inline]
    pub fn before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callback = Some(Arc::new(Mutex::new(callback)));
        self
    }

    fn build(&mut self) -> ModifyObjectMetadata<'a> {
        ModifyObjectMetadata {
            entry: self.inner.entry.to_owned(),
            mime_type: self.inner.mime_type.to_owned(),
            metadata: take(&mut self.inner.metadata),
            conditions: take(&mut self.inner.conditions),
        }
    }

    impl_call_methods!(modify_object_metadata);
}

impl OperationProvider for ModifyObjectMetadataBuilder<'_> {
    fn to_operation(&mut self) -> String {
        self.build().to_string()
    }
}

impl Debug for ModifyObjectMetadataBuilder<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModifyObjectMetadataBuilder")
            .field("inner", &self.inner)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(super) struct ModifyObjectLifeCycle<'a> {
    entry: Entry<'a>,
    to_ia_after_days: AfterDays,
    to_archive_after_days: AfterDays,
    to_deep_archive_after_days: AfterDays,
    delete_after_days: AfterDays,
}

impl ModifyObjectLifeCycle<'_> {
    pub(super) fn builder(entry: Entry) -> ModifyObjectLifeCycleBuilder {
        ModifyObjectLifeCycleBuilder {
            inner: ModifyObjectLifeCycle {
                entry,
                to_ia_after_days: Default::default(),
                to_archive_after_days: Default::default(),
                to_deep_archive_after_days: Default::default(),
                delete_after_days: Default::default(),
            },
            before_request_callback: None,
        }
    }

    fn to_path_params(&self) -> qiniu_apis::storage::modify_object_life_cycle::PathParams {
        let mut params = qiniu_apis::storage::modify_object_life_cycle::PathParams::default()
            .set_entry_as_str(self.entry.to_string());
        if !self.to_ia_after_days.is_unmodified() {
            params = params.set_to_ia_after_days_as_isize(self.to_ia_after_days.into());
        }
        if !self.to_archive_after_days.is_unmodified() {
            params = params.set_to_archive_after_days_as_isize(self.to_archive_after_days.into());
        }
        if !self.to_deep_archive_after_days.is_unmodified() {
            params = params.set_to_deep_archive_after_days_as_isize(self.to_deep_archive_after_days.into());
        }
        if !self.delete_after_days.is_unmodified() {
            params = params.set_delete_after_days_as_isize(self.delete_after_days.into());
        }
        params
    }
}

impl Display for ModifyObjectLifeCycle<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lifecycle/{}", self.entry.encode())?;
        if !self.to_ia_after_days.is_unmodified() {
            write!(f, "/toIAAfterDays/{}", self.to_ia_after_days)?;
        }
        if !self.to_archive_after_days.is_unmodified() {
            write!(f, "/toARCHIVEAfterDays/{}", self.to_archive_after_days)?;
        }
        if !self.delete_after_days.is_unmodified() {
            write!(f, "/deleteAfterDays/{}", self.delete_after_days)?;
        }
        Ok(())
    }
}

/// 修改对象生命周期构建器
///
/// 可以通过 [`crate::Bucket::modify_object_life_cycle`] 方法获取该构建器。
#[derive(Clone)]
pub struct ModifyObjectLifeCycleBuilder<'a> {
    inner: ModifyObjectLifeCycle<'a>,
    before_request_callback: Option<BeforeRequestCallback<'a>>,
}

impl<'a> ModifyObjectLifeCycleBuilder<'a> {
    /// 设置多少天后自动转换为低频文件
    #[inline]
    pub fn ia_after_days(&mut self, to_ia_after_days: AfterDays) -> &mut Self {
        self.inner.to_ia_after_days = to_ia_after_days;
        self
    }

    /// 设置多少天后自动转换为归档文件
    #[inline]
    pub fn archive_after_days(&mut self, to_archive_after_days: AfterDays) -> &mut Self {
        self.inner.to_archive_after_days = to_archive_after_days;
        self
    }

    /// 设置多少天后自动转换为深度归档文件
    #[inline]
    pub fn deep_archive_after_days(&mut self, to_deep_archive_after_days: AfterDays) -> &mut Self {
        self.inner.to_deep_archive_after_days = to_deep_archive_after_days;
        self
    }

    /// 设置多少天后自动删除
    #[inline]
    pub fn delete_after_days(&mut self, to_delete_after_days: AfterDays) -> &mut Self {
        self.inner.delete_after_days = to_delete_after_days;
        self
    }

    /// 设置请求前回调函数
    #[inline]
    pub fn before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callback = Some(Arc::new(Mutex::new(callback)));
        self
    }

    fn build(&mut self) -> ModifyObjectLifeCycle<'a> {
        ModifyObjectLifeCycle {
            entry: self.inner.entry.to_owned(),
            to_ia_after_days: take(&mut self.inner.to_ia_after_days),
            to_archive_after_days: take(&mut self.inner.to_archive_after_days),
            to_deep_archive_after_days: take(&mut self.inner.to_deep_archive_after_days),
            delete_after_days: take(&mut self.inner.delete_after_days),
        }
    }

    impl_call_methods!(modify_object_life_cycle);
}

impl OperationProvider for ModifyObjectLifeCycleBuilder<'_> {
    fn to_operation(&mut self) -> String {
        self.build().to_string()
    }
}

impl Debug for ModifyObjectLifeCycleBuilder<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModifyObjectLifeCycleBuilder")
            .field("inner", &self.inner)
            .finish()
    }
}

/// 设置对象生命周期天数
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AfterDays(isize);

impl AfterDays {
    /// 不设置生命周期
    #[inline]
    pub const fn unset() -> Self {
        Self(-1)
    }

    /// 是否没有设置生命周期
    #[inline]
    pub const fn is_unset(self) -> bool {
        self.0 == -1
    }

    /// 不修改生命周期
    #[inline]
    pub const fn unmodify() -> Self {
        Self(0)
    }

    /// 是否不修改生命周期
    #[inline]
    pub const fn is_unmodified(self) -> bool {
        self.0 == 0
    }

    /// 设置生命周期天数
    #[inline]
    pub const fn new(days: isize) -> Self {
        Self(days)
    }

    /// 是否已经设置生命周期天数
    #[inline]
    pub const fn is_set(self) -> bool {
        self.0 > 0
    }

    /// 获取生命周期的天数
    #[inline]
    pub const fn to_value(self) -> isize {
        self.0
    }
}

impl From<isize> for AfterDays {
    #[inline]
    fn from(num: isize) -> Self {
        Self(num)
    }
}

impl From<AfterDays> for isize {
    #[inline]
    fn from(t: AfterDays) -> Self {
        t.0
    }
}

impl Display for AfterDays {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

fn make_callback_error(err: AnyError) -> HttpResponseError {
    HttpResponseError::builder(HttpResponseErrorKind::CallbackError, err).build()
}

#[cfg(test)]
#[cfg(feature = "async")]
mod tests {
    use super::{
        super::{mime, ObjectsManager},
        *,
    };
    use futures::future::BoxFuture;
    use qiniu_apis::{
        credential::Credential,
        http::{
            AsyncRequest, AsyncResponse, AsyncResponseBody, AsyncResponseResult, HeaderValue, HttpCaller, StatusCode,
            SyncRequest, SyncResponseResult,
        },
        http_client::{BucketName, DirectChooser, HttpClient, NeverRetrier, Region, NO_BACKOFF},
    };
    use serde_json::{json, to_vec as json_to_vec};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[async_std::test]
    async fn test_async_stat() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller;

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    assert!(request
                        .url()
                        .to_string()
                        .ends_with(&format!("/stat/{}", &encode_entry("fakeobjectname"))));
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(AsyncResponseBody::from_bytes(
                            json_to_vec(&json!({
                                "fsize": 12345,
                                "hash": "fakehash",
                                "mimeType": "text/plain",
                                "type": 0,
                                "putTime": generate_put_time(),
                            }))
                            .unwrap(),
                        ))
                        .build())
                })
            }
        }

        let bucket = get_bucket(FakeHttpCaller::default());
        let object = bucket.stat_object("fakeobjectname").async_call().await?.into_body();
        assert_eq!(object.get_hash_as_str(), "fakehash");
        assert_eq!(object.get_size_as_u64(), 12345u64);

        Ok(())
    }

    #[async_std::test]
    async fn test_async_copy() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller;

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    assert!(request.url().to_string().ends_with(&format!(
                        "/copy/{}/{}/force/true",
                        &encode_entry("fakeobjectname"),
                        &encode_entry("fakeobjectname2")
                    )));
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(AsyncResponseBody::from_bytes(vec![]))
                        .build())
                })
            }
        }

        let bucket = get_bucket(FakeHttpCaller::default());
        bucket
            .copy_object_to("fakeobjectname", &get_bucket_name(), "fakeobjectname2")
            .is_force(true)
            .async_call()
            .await?;

        Ok(())
    }

    #[async_std::test]
    async fn test_async_move() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller;

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    assert!(request.url().to_string().ends_with(&format!(
                        "/move/{}/{}/force/false",
                        &encode_entry("fakeobjectname"),
                        &encode_entry("fakeobjectname2")
                    )));
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(AsyncResponseBody::from_bytes(vec![]))
                        .build())
                })
            }
        }

        let bucket = get_bucket(FakeHttpCaller::default());
        bucket
            .move_object_to("fakeobjectname", &get_bucket_name(), "fakeobjectname2")
            .async_call()
            .await?;

        Ok(())
    }

    #[async_std::test]
    async fn test_async_delete() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller;

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    assert!(request
                        .url()
                        .to_string()
                        .ends_with(&format!("/delete/{}", &encode_entry("fakeobjectname"))));
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(AsyncResponseBody::from_bytes(vec![]))
                        .build())
                })
            }
        }

        let bucket = get_bucket(FakeHttpCaller::default());
        bucket.delete_object("fakeobjectname").async_call().await?;

        Ok(())
    }

    #[async_std::test]
    async fn test_async_unfreeze() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller;

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    assert!(request.url().to_string().ends_with(&format!(
                        "/restoreAr/{}/freezeAfterDays/7",
                        &encode_entry("fakeobjectname")
                    )));
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(AsyncResponseBody::from_bytes(vec![]))
                        .build())
                })
            }
        }

        let bucket = get_bucket(FakeHttpCaller::default());
        bucket.restore_archived_object("fakeobjectname", 7).async_call().await?;

        Ok(())
    }

    #[async_std::test]
    async fn test_async_set_type() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller;

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    assert!(request
                        .url()
                        .to_string()
                        .ends_with(&format!("/chtype/{}/type/2", &encode_entry("fakeobjectname"))));
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(AsyncResponseBody::from_bytes(vec![]))
                        .build())
                })
            }
        }

        let bucket = get_bucket(FakeHttpCaller::default());
        bucket
            .set_object_type("fakeobjectname", FileType::Archive)
            .async_call()
            .await?;

        Ok(())
    }

    #[async_std::test]
    async fn test_async_modify_status() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller;

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    assert!(request
                        .url()
                        .to_string()
                        .ends_with(&format!("/chstatus/{}/status/1", &encode_entry("fakeobjectname"))));
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(AsyncResponseBody::from_bytes(vec![]))
                        .build())
                })
            }
        }

        let bucket = get_bucket(FakeHttpCaller::default());
        bucket.modify_object_status("fakeobjectname", true).async_call().await?;

        Ok(())
    }

    #[async_std::test]
    async fn test_async_modify_metadata() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller;

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    assert!(request.url().to_string().ends_with(&format!(
                        "/chgm/{}/mime/{}/cond/{}/x-qn-meta-MetaKey-1/{}",
                        &encode_entry("fakeobjectname"),
                        &urlsafe(b"text/plain".as_slice()),
                        &urlsafe(b"hash=fakehash&mime=text/html".as_slice()),
                        &urlsafe(b"MetaValue-1".as_slice()),
                    )));
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(AsyncResponseBody::from_bytes(vec![]))
                        .build())
                })
            }
        }

        let bucket = get_bucket(FakeHttpCaller::default());
        bucket
            .modify_object_metadata("fakeobjectname", mime::TEXT_PLAIN)
            .add_metadata("MetaKey-1", "MetaValue-1")
            .add_condition("hash", "fakehash")
            .add_condition("mime", "text/html")
            .async_call()
            .await?;

        Ok(())
    }

    #[async_std::test]
    async fn test_async_modify_life_cycle() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller;

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    assert!(request.url().to_string().ends_with(&format!(
                        "/lifecycle/{}/toIAAfterDays/1/toArchiveAfterDays/2/toDeepArchiveAfterDays/3/deleteAfterDays/4",
                        &encode_entry("fakeobjectname"),
                    )));
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(AsyncResponseBody::from_bytes(vec![]))
                        .build())
                })
            }
        }

        let bucket = get_bucket(FakeHttpCaller::default());
        bucket
            .modify_object_life_cycle("fakeobjectname")
            .ia_after_days(AfterDays::new(1))
            .archive_after_days(AfterDays::new(2))
            .deep_archive_after_days(AfterDays::new(3))
            .delete_after_days(AfterDays::new(4))
            .async_call()
            .await?;

        Ok(())
    }

    fn get_bucket(caller: impl HttpCaller + 'static) -> Bucket {
        let object_manager = ObjectsManager::builder(get_credential())
            .http_client(
                HttpClient::builder(caller)
                    .chooser(DirectChooser)
                    .request_retrier(NeverRetrier)
                    .backoff(NO_BACKOFF)
                    .build(),
            )
            .build();
        object_manager.bucket_with_region(get_bucket_name(), single_rs_domain_region())
    }

    fn get_credential() -> Credential {
        Credential::new("fakeaccesskey", "fakesecretkey")
    }

    fn get_bucket_name() -> BucketName {
        "fakebucketname".into()
    }

    fn encode_entry(object_name: &str) -> String {
        urlsafe(format!("{}:{}", get_bucket_name(), object_name).as_bytes())
    }

    fn generate_put_time() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64 / 100
    }

    fn single_rs_domain_region() -> Region {
        Region::builder("chaotic")
            .add_rs_preferred_endpoint(("fakers.example.com".to_owned(), 8080).into())
            .build()
    }
}
