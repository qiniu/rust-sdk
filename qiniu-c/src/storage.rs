use crate::{
    client::qiniu_ng_client_t,
    region::qiniu_ng_region_id_t,
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr},
    utils::qiniu_ng_str_list_t,
};
use qiniu_ng::Client;
use tap::TapOps;

/// @brief 列出所有存储空间名称
/// @param[in] client 七牛客户端
/// @param[out] names 用于返回存储空间名称列表，如果传入 `NULL` 表示不获取 `names`。但如果运行正常，返回值将依然是 `true`
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `names` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `names` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_storage_bucket_names(
    client: qiniu_ng_client_t,
    names: *mut qiniu_ng_str_list_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let client = Option::<Box<Client>>::from(client).unwrap();
    match client.storage().bucket_names().tap(|_| {
        let _ = qiniu_ng_client_t::from(client);
    }) {
        Ok(bucket_names) => {
            if let Some(names) = unsafe { names.as_mut() } {
                *names = unsafe { qiniu_ng_str_list_t::from_string_vec_unchecked(bucket_names) };
            }
            true
        }
        Err(ref err) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = err.into();
            }
            false
        }
    }
}

/// @brief 创建存储空间
/// @param[in] client 七牛客户端
/// @param[in] bucket_name 存储空间名称
/// @param[in] region_id 区域 ID，如果您的区域 ID 不是 `qiniu_ng_region_id_t` 所能提供，则调用 `qiniu_ng_storage_create_bucket_with_customized_region_id()` 函数创建存储空间
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示创建成功，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @details
///     在创建存储空间时，需要注意存储空间的名称必须遵守以下规则：
///     - 存储空间名称不允许重复，遇到冲突请更换名称。
///     - 名称由 3 ~ 63 个字符组成 ，可包含小写字母、数字和短划线，且必须以小写字母或者数字开头和结尾。
#[no_mangle]
pub extern "C" fn qiniu_ng_storage_create_bucket(
    client: qiniu_ng_client_t,
    bucket_name: *const qiniu_ng_char_t,
    region_id: qiniu_ng_region_id_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let client = Option::<Box<Client>>::from(client).unwrap();
    _qiniu_ng_storage_create_bucket(
        &client,
        &unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap(),
        region_id.as_ref(),
        error,
    )
    .tap(|_| {
        let _ = qiniu_ng_client_t::from(client);
    })
}

/// @brief 使用自定义区域 ID 创建存储空间
/// @param[in] client 七牛客户端
/// @param[in] bucket_name 存储空间名称
/// @param[in] region_id 区域 ID 字符串，参考[官方文档](https://developer.qiniu.com/kodo/manual/1671/region-endpoint)
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示创建成功，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @details
///     在创建存储空间时，需要注意存储空间的名称必须遵守以下规则：
///     - 存储空间名称不允许重复，遇到冲突请更换名称。
///     - 名称由 3 ~ 63 个字符组成 ，可包含小写字母、数字和短划线，且必须以小写字母或者数字开头和结尾。
#[no_mangle]
pub extern "C" fn qiniu_ng_storage_create_bucket_with_customized_region_id(
    client: qiniu_ng_client_t,
    bucket_name: *const qiniu_ng_char_t,
    region_id: *const qiniu_ng_char_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let client = Option::<Box<Client>>::from(client).unwrap();
    _qiniu_ng_storage_create_bucket(
        &client,
        &unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap(),
        &unsafe { ucstr::from_ptr(region_id) }.to_string().unwrap(),
        error,
    )
    .tap(|_| {
        let _ = qiniu_ng_client_t::from(client);
    })
}

fn _qiniu_ng_storage_create_bucket(
    client: &Client,
    bucket_name: &str,
    region_id: &str,
    error: *mut qiniu_ng_err_t,
) -> bool {
    match client.storage().create_bucket(bucket_name, region_id) {
        Ok(_) => true,
        Err(ref err) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = err.into();
            }
            false
        }
    }
}

/// @brief 删除存储空间
/// @param[in] client 七牛客户端
/// @param[in] bucket_name 即将删除的存储空间名称
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示删除成功，如果返回 `false`，则表示可以读取 `error` 获得错误信息
#[no_mangle]
pub extern "C" fn qiniu_ng_storage_drop_bucket(
    client: qiniu_ng_client_t,
    bucket_name: *const qiniu_ng_char_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let client = Option::<Box<Client>>::from(client).unwrap();
    match client
        .storage()
        .drop_bucket(unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap())
        .tap(|_| {
            let _ = qiniu_ng_client_t::from(client);
        }) {
        Ok(_) => true,
        Err(ref err) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = err.into();
            }
            false
        }
    }
}
