use crate::{
    client::qiniu_ng_client_t,
    result::{make_qiniu_ng_err_from_qiniu_http_error, qiniu_ng_err},
    utils::{make_string_list, qiniu_ng_string_list_t},
};
use qiniu::Client;
use std::mem;

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_bucket_names(
    client: qiniu_ng_client_t,
    names: *mut qiniu_ng_string_list_t,
    error: *mut qiniu_ng_err,
) -> bool {
    let boxed_client = Box::from_raw(mem::transmute::<_, *mut Client>(client));
    match boxed_client.bucket_manager().bucket_names() {
        Ok(bucket_names) => {
            Box::into_raw(boxed_client);
            *names = make_string_list(&bucket_names);
            true
        }
        Err(err) => {
            Box::into_raw(boxed_client);
            if !error.is_null() {
                *error = make_qiniu_ng_err_from_qiniu_http_error(&err);
            }
            false
        }
    }
}
