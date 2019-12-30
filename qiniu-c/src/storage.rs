use crate::{
    client::qiniu_ng_client_t, region::qiniu_ng_region_id_t, result::qiniu_ng_err, utils::qiniu_ng_str_list_t,
};
use libc::c_char;
use qiniu_ng::Client;
use std::ffi::CStr;
use tap::TapOps;

#[no_mangle]
pub extern "C" fn qiniu_ng_storage_bucket_names(
    client: qiniu_ng_client_t,
    names: *mut qiniu_ng_str_list_t,
    error: *mut qiniu_ng_err,
) -> bool {
    let client = Box::<Client>::from(client);
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

#[no_mangle]
pub extern "C" fn qiniu_ng_storage_create_bucket(
    client: qiniu_ng_client_t,
    bucket_name: *const c_char,
    region_id: qiniu_ng_region_id_t,
    error: *mut qiniu_ng_err,
) -> bool {
    let client = Box::<Client>::from(client);
    match client
        .storage()
        .create_bucket(
            unsafe { CStr::from_ptr(bucket_name) }.to_str().unwrap().to_owned(),
            region_id.into(),
        )
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

#[no_mangle]
pub extern "C" fn qiniu_ng_storage_drop_bucket(
    client: qiniu_ng_client_t,
    bucket_name: *const c_char,
    error: *mut qiniu_ng_err,
) -> bool {
    let client = Box::<Client>::from(client);
    match client
        .storage()
        .drop_bucket(unsafe { CStr::from_ptr(bucket_name) }.to_str().unwrap().to_owned())
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
