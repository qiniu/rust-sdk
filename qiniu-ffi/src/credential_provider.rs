use super::{
    credential::qiniu_ng_credential_t,
    error::{qiniu_ng_err_io_error_t, qiniu_ng_err_t},
    ucstring::{convert_optional_c_string_to_rust_string, qiniu_ng_char_t},
};
use libc::c_void;
use qiniu_credential::{
    ChainCredentialsProvider, ChainCredentialsProviderBuilder, Credential, CredentialProvider,
    EnvCredentialProvider, GlobalCredentialProvider, StaticCredentialProvider,
};
use qiniu_ffi_struct_macros::FFIStruct;
use std::{
    any::Any,
    io::{Error as IOError, ErrorKind as IOErrorKind, Result as IOResult},
    mem::{replace, transmute, ManuallyDrop},
};

/// @brief 认证信息提供者
/// @details 为认证信息提供者的实现提供接口支持
///   * 调用 `qiniu_ng_credential_provider_new_*()` 函数创建 `qiniu_ng_credential_provider_t` 实例。
///   * 当 `qiniu_ng_credential_provider_t` 使用完毕后，请务必调用 `qiniu_ng_credential_provider_free()` 方法释放内存。
#[repr(C)]
#[derive(Copy, Clone, PartialEq, FFIStruct)]
#[ffi_wrap(Box, dyn, CredentialProvider)]
pub struct qiniu_ng_credential_provider_t(*mut c_void, *mut c_void);

/// @brief 从认证信息提供者获取认证信息
/// @param[in] credential_provider 七牛认证信息提供者实例
/// @param[out] credential 七牛认证信息实例
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `signed` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_provider_get(
    credential_provider: qiniu_ng_credential_provider_t,
    credential: *mut qiniu_ng_credential_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let credential_provider = ManuallyDrop::new(
        Option::<Box<dyn CredentialProvider>>::from(credential_provider).unwrap(),
    );
    match credential_provider.get() {
        Ok(got) => {
            let got: Credential<'static> = unsafe { transmute(got) };
            if let Some(credential) = unsafe { credential.as_mut() } {
                *credential = Box::new(got).into();
            }
            true
        }
        Err(err) => {
            if let Some(e) = unsafe { error.as_mut() } {
                *e = err.into();
            }
            false
        }
    }
}

/// @brief 释放 七牛认证信息提供者实例
/// @param[in,out] credential 七牛认证信息提供者实例地址，释放完毕后该认证信息提供者实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_provider_free(
    credential_provider: *mut qiniu_ng_credential_provider_t,
) {
    if let Some(credential_provider) = unsafe { credential_provider.as_mut() } {
        let _ = Option::<Box<dyn CredentialProvider>>::from(*credential_provider);
        *credential_provider = Default::default();
    }
}

/// @brief 判断七牛认证信息提供者实例是否是 NULL
/// @param[in] credential 七牛认证信息提供者实例
/// @retval bool 如果返回 `true` 则表示七牛认证信息提供者实例是 `NULL`
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_provider_is_null(
    credential_provider: qiniu_ng_credential_provider_t,
) -> bool {
    credential_provider.is_null()
}

/// @brief 构建一个静态认证信息提供者，只需要传入静态的 AccessKey 和 SecretKey 即可
/// @param[in] access_key 七牛 AccessKey
/// @param[in] secret_key 七牛 SecretKey
/// @retval qiniu_ng_credential_provider_t 获取创建的七牛认证信息提供者实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_credential_provider_free()` 方法释放 `qiniu_ng_credential_provider_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_provider_new_static(
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
) -> qiniu_ng_credential_provider_t {
    let credential_provider: Box<dyn CredentialProvider> = Box::new(StaticCredentialProvider::new(
        unsafe { convert_optional_c_string_to_rust_string(access_key) },
        unsafe { convert_optional_c_string_to_rust_string(secret_key) },
    ));
    credential_provider.into()
}

/// @brief 构建一个全局认证信息提供者，可以将认证信息配置在全局变量中
/// @details 任何全局认证信息提供者实例都可以设置和访问全局认证信息。
/// @retval qiniu_ng_credential_provider_t 获取创建的七牛认证信息提供者实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_credential_provider_free()` 方法释放 `qiniu_ng_credential_provider_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_provider_new_global() -> qiniu_ng_credential_provider_t {
    let credential_provider: Box<dyn CredentialProvider> = Box::new(GlobalCredentialProvider);
    credential_provider.into()
}

/// @brief 配置全局认证信息
/// @param[in] access_key 七牛 AccessKey
/// @param[in] secret_key 七牛 SecretKey
/// @warning 应该首先调用该方法，然后再调用 `qiniu_ng_credential_provider_new_global()` 方法创建全局认证信息提供者实例
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_provider_global_setup(
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
) {
    GlobalCredentialProvider::setup(
        unsafe { convert_optional_c_string_to_rust_string(access_key) },
        unsafe { convert_optional_c_string_to_rust_string(secret_key) },
    );
}

/// @brief 清空全局认证信息
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_provider_global_clear() {
    GlobalCredentialProvider::clear();
}

/// @brief 构建一个环境变量认证信息提供者，可以将认证信息配置在环境变量中。
/// @retval qiniu_ng_credential_provider_t 获取创建的七牛认证信息提供者实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_credential_provider_free()` 方法释放 `qiniu_ng_credential_provider_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_provider_new_env() -> qiniu_ng_credential_provider_t {
    let credential_provider: Box<dyn CredentialProvider> = Box::new(EnvCredentialProvider);
    credential_provider.into()
}

/// @brief 配置环境变量认证信息
/// @param[in] access_key 七牛 AccessKey
/// @param[in] secret_key 七牛 SecretKey
/// @warning 与 `qiniu_ng_credential_provider_global_setup()` 不同，该方法的调用并非必要，可以通过在外部直接设置环境变量 `QINIU_ACCESS_KEY` 和 `QINIU_SECRET_KEY` 来配置环境变量认证信息
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_provider_env_setup(
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
) {
    EnvCredentialProvider::setup(
        unsafe { convert_optional_c_string_to_rust_string(access_key) },
        unsafe { convert_optional_c_string_to_rust_string(secret_key) },
    );
}

/// @brief 构建一个默认的认证信息串提供者
/// @retval qiniu_ng_credential_provider_t 获取创建的七牛认证信息提供者实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_credential_provider_free()` 方法释放 `qiniu_ng_credential_provider_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_chain_credential_provider_default() -> qiniu_ng_credential_provider_t {
    let credential_provider: Box<dyn CredentialProvider> =
        Box::new(ChainCredentialsProvider::default());
    credential_provider.into()
}

/// @brief 串联认证信息构建器
/// @details 接受多个认证信息提供者并将他们串联成串联认证信息
#[repr(C)]
#[derive(Copy, Clone, PartialEq, FFIStruct)]
#[ffi_wrap(Box, ChainCredentialsProviderBuilder)]
pub struct qiniu_ng_chain_credential_provider_builder_t(*mut c_void);

/// @brief 构建新的串联认证信息构建器
/// @retval qiniu_ng_chain_credential_provider_builder_t 获取创建的串联认证信息构建器实例
#[no_mangle]
pub extern "C" fn qiniu_ng_chain_credential_provider_builder_new(
) -> qiniu_ng_chain_credential_provider_builder_t {
    Box::new(ChainCredentialsProviderBuilder::new()).into()
}

/// @brief 将认证信息提供者推送到认证串末端
/// @params[in,out] builder 串联认证信息构建器地址
/// @params[in,out] credential_provider 认证信息提供者地址
/// @warning 注意，传入的 `credential_provider` 参数后，该提供者将被该串联认证信息构建器占有，因此不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_chain_credential_provider_builder_append_credential(
    builder: *mut qiniu_ng_chain_credential_provider_builder_t,
    credential_provider: *mut qiniu_ng_credential_provider_t,
) {
    let mut chain_credential_provider_builder =
        Option::<Box<ChainCredentialsProviderBuilder>>::from(unsafe { builder.read() }).unwrap();
    {
        let credential_provider =
            Option::<Box<dyn CredentialProvider>>::from(unsafe { credential_provider.read() })
                .unwrap();
        *chain_credential_provider_builder =
            chain_credential_provider_builder.append_credential(credential_provider);
    }
    unsafe { builder.write(chain_credential_provider_builder.into()) };
    unsafe { *credential_provider = Default::default() };
}

/// @brief 将认证信息提供者推送到认证串顶端
/// @params[in,out] builder 串联认证信息构建器地址
/// @params[in,out] credential_provider 认证信息提供者地址
/// @warning 注意，传入的 `credential_provider` 参数后，该提供者将被该串联认证信息构建器占有，因此不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_chain_credential_provider_builder_prepend_credential(
    builder: *mut qiniu_ng_chain_credential_provider_builder_t,
    credential_provider: *mut qiniu_ng_credential_provider_t,
) {
    let mut chain_credential_provider_builder =
        Option::<Box<ChainCredentialsProviderBuilder>>::from(unsafe { builder.read() }).unwrap();
    {
        let credential_provider =
            Option::<Box<dyn CredentialProvider>>::from(unsafe { credential_provider.read() })
                .unwrap();
        *chain_credential_provider_builder =
            chain_credential_provider_builder.prepend_credential(credential_provider);
    }
    unsafe { builder.write(chain_credential_provider_builder.into()) };
    unsafe { *credential_provider = Default::default() };
}

/// @brief 串联认证信息，生成新的七牛认证信息提供者实例
/// @params[in,out] builder 串联认证信息构建器地址
/// @retval qiniu_ng_credential_provider_t 获取创建的七牛认证信息提供者实例
/// @param[in,out] credential 七牛认证信息实例地址，释放完毕后该认证信息实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_chain_credential_provider_build(
    builder: *mut qiniu_ng_chain_credential_provider_builder_t,
) -> qiniu_ng_credential_provider_t {
    unsafe { builder.as_mut() }
        .map(|builder| {
            let credential_provider =
                Option::<Box<ChainCredentialsProviderBuilder>>::from(*builder).unwrap();
            *builder = Default::default();
            let credential_provider: Box<dyn CredentialProvider> =
                Box::new(credential_provider.build());
            credential_provider.into()
        })
        .unwrap_or_default()
}

/// @brief 释放 串联认证信息构建器实例
/// @param[in,out] builder 串联认证信息构建器实例地址，释放完毕后该串联认证信息构建器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_chain_credential_provider_free(
    builder: *mut qiniu_ng_chain_credential_provider_builder_t,
) {
    if let Some(builder) = unsafe { builder.as_mut() } {
        let _ = Option::<Box<ChainCredentialsProviderBuilder>>::from(*builder);
        *builder = Default::default();
    }
}

#[repr(C)]
#[derive(Copy, Default, Clone, PartialEq)]
pub struct qiniu_ng_user_defined_credential_t {
    pub credential: qiniu_ng_credential_t,
    pub error: qiniu_ng_err_io_error_t,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct UserDefinedCredentialProvider {
    f: extern "C" fn(
        init: qiniu_ng_user_defined_credential_t,
    ) -> qiniu_ng_user_defined_credential_t,
}

impl CredentialProvider for UserDefinedCredentialProvider {
    #[inline]
    fn get(&self) -> IOResult<Credential> {
        type StaticCredential = Credential<'static>;
        let retval = (self.f)(Default::default());
        Option::<IOErrorKind>::from(retval.error)
            .map(|kind| IOError::new(kind, "Returned by qiniu_ng_user_defined_credential_t"))
            .map(Err)
            .unwrap_or_else(|| {
                let mut credential = Option::<Box<StaticCredential>>::from(retval.credential)
                    .expect("Either field of qiniu_ng_user_defined_credential_t must be set");
                let credential: StaticCredential =
                    replace(&mut credential, Credential::new("", ""));
                Ok(credential)
            })
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_credential_provider(&self) -> &dyn CredentialProvider {
        self
    }
}

/// @brief 构建一个用户自定义认证信息提供者
/// @details 接受一个用户自定义认证信息提供函数，该函数接受一个返回值的默认值，经过修改后再将该值返回。
///   * 对于获取认证信息成功的情况，通过 `qiniu_ng_credential_new()` 创建 qiniu_ng_credential_t 类型的实例后设置为 `init.credential` 字段，返回将 `init` 返回。该 qiniu_ng_credential_t 的内存将由 SDK 自行回收
///   * 对于获取认证信息失败的情况，根据错误类型从 `qiniu_ng_err_io_error_t` 中取出一个枚举值后设置为 `init.error` 字段，返回将 `init` 返回
///   * 在任何时候都无需同时设置两个字段，也不能一个字段都不设置
/// @param[in] f 用户自定义认证信息提供函数
/// @retval qiniu_ng_credential_provider_t 获取创建的七牛认证信息提供者实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_credential_provider_free()` 方法释放 `qiniu_ng_credential_provider_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_provider_new_user_defined(
    f: extern "C" fn(
        init: qiniu_ng_user_defined_credential_t,
    ) -> qiniu_ng_user_defined_credential_t,
) -> qiniu_ng_credential_provider_t {
    let credential_provider: Box<dyn CredentialProvider> =
        Box::new(UserDefinedCredentialProvider { f });
    credential_provider.into()
}
