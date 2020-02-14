use crate::{
    client::qiniu_ng_client_t,
    region::qiniu_ng_region_id_t,
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr},
    utils::qiniu_ng_str_list_t,
};
use qiniu_ng::Client;
use tap::TapOps;

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

#[no_mangle]
pub extern "C" fn qiniu_ng_storage_create_bucket2(
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
