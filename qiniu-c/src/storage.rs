use crate::{
    client::qiniu_ng_client_t,
    region::qiniu_ng_region_id_t,
    result::qiniu_ng_err,
    utils::{convert_c_char_to_string, make_string_list, qiniu_ng_string_list_t},
};
use libc::c_char;
use qiniu_ng::Client;
use tap::TapOps;

#[no_mangle]
pub extern "C" fn qiniu_ng_storage_bucket_names(
    client: qiniu_ng_client_t,
    names: *mut qiniu_ng_string_list_t,
    error: *mut qiniu_ng_err,
) -> bool {
    let client: Box<Client> = client.into();
    match client.storage().bucket_names().tap(|_| {
        let _: qiniu_ng_client_t = client.into();
    }) {
        Ok(bucket_names) => {
            if !names.is_null() {
                unsafe { *names = make_string_list(&bucket_names) };
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
    let bucket_name = convert_c_char_to_string(bucket_name.cast());
    match client.storage().create_bucket(bucket_name, region_id.into()).tap(|_| {
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
    let bucket_name = convert_c_char_to_string(bucket_name.cast());
    match client.storage().drop_bucket(bucket_name).tap(|_| {
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
