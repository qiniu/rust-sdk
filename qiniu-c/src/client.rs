use crate::{
    config::qiniu_ng_config_t,
    string::{qiniu_ng_char_t, ucstr},
    upload::qiniu_ng_upload_manager_t,
};
use libc::c_void;
use qiniu_ng::Client;
use std::{mem::transmute, ptr::null_mut};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_client_t(*mut c_void);

// TODO: 提供 Client 的 clone() 方法

impl Default for qiniu_ng_client_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_client_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_client_t> for Option<Box<Client>> {
    fn from(client: qiniu_ng_client_t) -> Self {
        if client.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(client)) })
        }
    }
}

impl From<Option<Box<Client>>> for qiniu_ng_client_t {
    fn from(client: Option<Box<Client>>) -> Self {
        client.map(|client| client.into()).unwrap_or_default()
    }
}

impl From<Box<Client>> for qiniu_ng_client_t {
    fn from(client: Box<Client>) -> Self {
        unsafe { transmute(Box::into_raw(client)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_client_new(
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
    config: qiniu_ng_config_t,
) -> qiniu_ng_client_t {
    Box::new(Client::new(
        unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
        unsafe { ucstr::from_ptr(secret_key) }.to_string().unwrap(),
        config.get_clone().unwrap(),
    ))
    .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_client_free(client: *mut qiniu_ng_client_t) {
    if let Some(client) = unsafe { client.as_mut() } {
        let _ = Option::<Box<Client>>::from(*client);
        *client = qiniu_ng_client_t::default();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_client_is_freed(client: qiniu_ng_client_t) -> bool {
    client.is_null()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_client_get_upload_manager(client: qiniu_ng_client_t) -> qiniu_ng_upload_manager_t {
    let client = Option::<Box<Client>>::from(client).unwrap();
    Box::new(client.upload().to_owned())
        .tap(|_| {
            let _ = qiniu_ng_client_t::from(client);
        })
        .into()
}
