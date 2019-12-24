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
    let client: Box<Client> = client.into();
    match client.storage().bucket_names().tap(|_| {
        let _: qiniu_ng_client_t = client.into();
    }) {
        Ok(bucket_names) => {
            if !names.is_null() {
                unsafe { *names = qiniu_ng_str_list_t::from_string_vec_unchecked(bucket_names) };
            }
            true
        }
        Err(err) => {
            if !error.is_null() {
                unsafe { *error = (&err).into() };
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
    let client: Box<Client> = client.into();
    match client
        .storage()
        .create_bucket(
            unsafe { CStr::from_ptr(bucket_name) }.to_str().unwrap().to_owned(),
            region_id.into(),
        )
        .tap(|_| {
            let _: qiniu_ng_client_t = client.into();
        }) {
        Ok(_) => true,
        Err(err) => {
            if !error.is_null() {
                unsafe { *error = (&err).into() };
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
    let client: Box<Client> = client.into();
    match client
        .storage()
        .drop_bucket(unsafe { CStr::from_ptr(bucket_name) }.to_str().unwrap().to_owned())
        .tap(|_| {
            let _: qiniu_ng_client_t = client.into();
        }) {
        Ok(_) => true,
        Err(err) => {
            if !error.is_null() {
                unsafe { *error = (&err).into() };
            }
            false
        }
    }
}
