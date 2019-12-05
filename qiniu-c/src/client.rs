use crate::{config::qiniu_ng_config_t, upload::qiniu_ng_upload_manager_t, utils::convert_c_char_to_string};
use libc::{c_char, c_void};
use qiniu_ng::Client;
use std::mem::transmute;
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_client_t(*mut c_void);

impl From<qiniu_ng_client_t> for Box<Client> {
    fn from(client: qiniu_ng_client_t) -> Self {
        unsafe { Box::from_raw(transmute::<_, *mut Client>(client)) }
    }
}

impl From<Box<Client>> for qiniu_ng_client_t {
    fn from(client: Box<Client>) -> Self {
        unsafe { transmute(Box::into_raw(client)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_client_new(
    access_key: *const c_char,
    secret_key: *const c_char,
    config: qiniu_ng_config_t,
) -> qiniu_ng_client_t {
    Box::new(Client::new(
        convert_c_char_to_string(access_key),
        convert_c_char_to_string(secret_key),
        config.get_clone(),
    ))
    .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_client_free(client: qiniu_ng_client_t) {
    let _: Box<Client> = client.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_client_get_upload_manager(client: qiniu_ng_client_t) -> qiniu_ng_upload_manager_t {
    let client: Box<Client> = client.into();
    Box::new(client.upload().to_owned())
        .tap(|_| {
            let _: qiniu_ng_client_t = client.into();
        })
        .into()
}
