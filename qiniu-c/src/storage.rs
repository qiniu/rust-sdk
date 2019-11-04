use crate::{
    client::qiniu_ng_client_t,
    region::region_id_t,
    result::qiniu_ng_err,
    utils::{make_string_list, qiniu_ng_string_list_t},
};
use libc::{c_char, c_void};
use qiniu::{storage::bucket::Bucket, Client};
use std::{ffi::CStr, mem::transmute};

#[no_mangle]
pub extern "C" fn qiniu_ng_storage_bucket_names(
    client: qiniu_ng_client_t,
    names: *mut qiniu_ng_string_list_t,
    error: *mut qiniu_ng_err,
) -> bool {
    let client: Box<Client> = client.into();
    let names_result = client.storage().bucket_names();
    let _: qiniu_ng_client_t = client.into();
    match names_result {
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
    region_id: region_id_t,
    error: *mut qiniu_ng_err,
) -> bool {
    let client: Box<Client> = client.into();
    let bucket_name = unsafe { CStr::from_ptr(bucket_name).to_string_lossy() };
    let result = client.storage().create_bucket(bucket_name, region_id.into());
    let _: qiniu_ng_client_t = client.into();
    match result {
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
    let bucket_name = unsafe { CStr::from_ptr(bucket_name).to_string_lossy() };
    let result = client.storage().drop_bucket(bucket_name);
    let _: qiniu_ng_client_t = client.into();
    match result {
        Ok(_) => true,
        Err(err) => {
            if !error.is_null() {
                unsafe { *error = (&err).into() };
            }
            false
        }
    }
}

#[repr(C)]
pub struct qiniu_ng_bucket_t(*mut c_void);

impl<'r> From<qiniu_ng_bucket_t> for Box<Bucket<'r>> {
    fn from(bucket: qiniu_ng_bucket_t) -> Self {
        unsafe { Box::from_raw(transmute::<_, *mut Bucket>(bucket)) }
    }
}

impl<'r> From<Box<Bucket<'r>>> for qiniu_ng_bucket_t {
    fn from(bucket: Box<Bucket>) -> Self {
        unsafe { transmute(Box::into_raw(bucket)) }
    }
}
