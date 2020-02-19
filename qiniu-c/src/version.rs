use lazy_static::lazy_static;
use libc::c_char;
use tap::TapOps;

lazy_static! {
    static ref VERSION_C_STRING: Vec<u8> = {
        env!("CARGO_PKG_VERSION")
            .to_string()
            .into_bytes()
            .tap(|version| version.push(b'\0'))
    };
    static ref BUILD_FLAGS_C_STRING: Vec<u8> = {
        Vec::new()
            .tap(|features| {
                #[cfg(any(feature = "use-libcurl"))]
                {
                    features.push("use-libcurl");
                }
            })
            .join(",")
            .into_bytes()
            .tap(|features| features.push(b'\0'))
    };
}

/// @brief 获取 qiniu_ng 库版本号
/// @retval *char 版本号字符串
/// @warning 请勿修改其存储的字符串内容
#[no_mangle]
pub extern "C" fn qiniu_ng_version() -> *const c_char {
    VERSION_C_STRING.as_ptr().cast()
}

/// @brief 获取 qiniu_ng 编译的功能列表
/// @retval *char 功能列表字符串
/// @warning 请勿修改其存储的字符串内容
#[no_mangle]
pub extern "C" fn qiniu_ng_features() -> *const c_char {
    BUILD_FLAGS_C_STRING.as_ptr().cast()
}
