use etag::EtagV1;

/// Etag 字符串固定长度
pub const ETAG_SIZE: usize = 28;

/// @brief 七牛 Etag 计算器
/// @details 可以多次接受输入数据以计算七牛 Etag
/// @note
///   * 调用 `qiniu_ng_etag_v1_new()` 函数创建 `qiniu_ng_etag_v1_t` 实例。
///   * 随即可以多次调用 `qiniu_ng_etag_v1_update()` 函数输入数据。
///   * 最终调用 `qiniu_ng_etag_v1_result()` 函数获取计算结果。
///   * 当 `qiniu_ng_etag_v1_t` 使用完毕后，请务必调用 `qiniu_ng_etag_v1_free()` 方法释放内存。
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_etag_v1_t(*mut c_void);

impl Default for qiniu_ng_etag_v1_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_etag_v1_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_etag_v1_t> for Option<Box<EtagV1>> {
    fn from(etag: qiniu_ng_etag_v1_t) -> Self {
        if etag.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(etag)) })
        }
    }
}

impl From<Box<EtagV1>> for qiniu_ng_etag_v1_t {
    #[inline]
    fn from(etag: Box<EtagV1>) -> Self {
        unsafe { transmute(Box::into_raw(etag)) }
    }
}

impl From<Option<Box<EtagV1>>> for qiniu_ng_etag_v1_t {
    #[inline]
    fn from(etag: Option<Box<EtagV1>>) -> Self {
        etag.map(|etag| etag.into()).unwrap_or_default()
    }
}

/// @brief 创建 七牛 Etag V1 计算器实例
/// @retval qiniu_ng_etag_v1_t 获取创建的七牛 Etag V1 计算器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_etag_free()` 方法释放 `qiniu_ng_etag_v1_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v1_new() -> qiniu_ng_etag_v1_t {
    Box::new(EtagV1::new()).into()
}

/// @brief 向七牛 Etag V1 计算器实例输入数据
/// @param[in] etag 七牛 Etag V1 计算器实例
/// @param[in] data 输入数据地址
/// @param[in] data_len 输入数据长度
/// @note 多次调用该方法可以多次输入数据
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v1_update(
    etag: qiniu_ng_etag_v1_t,
    data: *const c_void,
    data_len: size_t,
) {
    let mut etag = ManuallyDrop::new(Option::<Box<EtagV1>>::from(etag).unwrap());
    etag.input(unsafe { from_raw_parts(data.cast(), data_len) });
}

/// @brief 从七牛 Etag V1 计算器获取结果
/// @param[in] etag 七牛 Etag V1 计算器实例
/// @param[out] result_ptr 用于返回 Etag 的内存地址，这里 `result_ptr` 必须不能为 `NULL`
/// @warning 保证提供给 `result_ptr` 至少 ETAG_SIZE 长度的内存
/// @note 该函数总是返回正确的结果
/// @note 该方法调用后，七牛 Etag V1 计算器实例将被自动重置，可以重新输入新的数据
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v1_result(etag: qiniu_ng_etag_v1_t, result_ptr: *mut c_void) {
    let mut etag = ManuallyDrop::new(Option::<Box<EtagV1>>::from(etag).unwrap());
    let result_ptr: &mut [u8; ETAG_SIZE] = unsafe { transmute(result_ptr) };
    etag.finalize_into_reset(result_ptr);
}

/// @brief 重置七牛 Etag V1 计算器实例
/// @param[in] etag 七牛 Etag V1 计算器实例
/// @note 该函数总是返回正确的结果
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v1_reset(etag: qiniu_ng_etag_v1_t) {
    let mut etag = ManuallyDrop::new(Option::<Box<EtagV1>>::from(etag).unwrap());
    etag.reset();
}

/// @brief 释放 七牛 Etag V1 计算器实例
/// @param[in,out] etag 七牛 Etag V1 计算器实例地址，释放完毕后该计算器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v1_free(etag: *mut qiniu_ng_etag_v1_t) {
    if let Some(etag) = unsafe { etag.as_mut() } {
        let _ = Option::<Box<EtagV1>>::from(*etag);
        *etag = qiniu_ng_etag_v1_t::default();
    }
}

/// @brief 判断 七牛 Etag V1 计算器实例是否已经被释放
/// @param[in] etag 七牛 Etag V1 计算器实例
/// @retval bool 如果返回 `true` 则表示七牛 Etag V1 计算器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v1_is_freed(etag: qiniu_ng_etag_v1_t) -> bool {
    etag.is_null()
}

/// @brief 创建 七牛 Etag V2 计算器实例
/// @retval qiniu_ng_etag_v2_t 获取创建的七牛 Etag V2 计算器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_etag_free()` 方法释放 `qiniu_ng_etag_v2_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v2_new() -> qiniu_ng_etag_v2_t {
    Box::new(EtagV2::new()).into()
}

/// @brief 向七牛 Etag V2 计算器实例输入数据
/// @param[in] etag 七牛 Etag V2 计算器实例
/// @param[in] data 输入数据地址
/// @param[in] data_len 输入数据长度
/// @note 多次调用该方法可以多次输入数据
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v2_update(
    etag: qiniu_ng_etag_v2_t,
    data: *const c_void,
    data_len: size_t,
) {
    let mut etag = ManuallyDrop::new(Option::<Box<EtagV2>>::from(etag).unwrap());
    etag.input(unsafe { from_raw_parts(data.cast(), data_len) });
}

/// @brief 从七牛 Etag V2 计算器获取结果
/// @param[in] etag 七牛 Etag V2 计算器实例
/// @param[out] result_ptr 用于返回 Etag 的内存地址，这里 `result_ptr` 必须不能为 `NULL`
/// @warning 保证提供给 `result_ptr` 至少 ETAG_SIZE 长度的内存
/// @note 该函数总是返回正确的结果
/// @note 该方法调用后，七牛 Etag V2 计算器实例将被自动重置，可以重新输入新的数据
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v2_result(etag: qiniu_ng_etag_v2_t, result_ptr: *mut c_void) {
    let mut etag = ManuallyDrop::new(Option::<Box<EtagV2>>::from(etag).unwrap());
    let result_ptr: &mut [u8; ETAG_SIZE] = unsafe { transmute(result_ptr) };
    etag.finalize_into_reset(result_ptr);
}

/// @brief 重置七牛 Etag V2 计算器实例
/// @param[in] etag 七牛 Etag V2 计算器实例
/// @note 该函数总是返回正确的结果
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v2_reset(etag: qiniu_ng_etag_v2_t) {
    let mut etag = ManuallyDrop::new(Option::<Box<EtagV2>>::from(etag).unwrap());
    etag.reset();
}

/// @brief 释放 七牛 Etag V2 计算器实例
/// @param[in,out] etag 七牛 Etag V2 计算器实例地址，释放完毕后该计算器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v2_free(etag: *mut qiniu_ng_etag_v2_t) {
    if let Some(etag) = unsafe { etag.as_mut() } {
        let _ = Option::<Box<EtagV2>>::from(*etag);
        *etag = qiniu_ng_etag_v2_t::default();
    }
}

/// @brief 判断 七牛 Etag V2 计算器实例是否已经被释放
/// @param[in] etag 七牛 Etag V2 计算器实例
/// @retval bool 如果返回 `true` 则表示七牛 Etag V2 计算器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v2_is_freed(etag: qiniu_ng_etag_v2_t) -> bool {
    etag.is_null()
}
