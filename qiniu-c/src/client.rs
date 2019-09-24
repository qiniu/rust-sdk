// use qiniu::utils::Client;
// use std::os::raw::c_char;

// #[no_mangle]
// pub extern "C" fn qiniu_ng_client_new(access_key: *const c_char, secret_key: *const c_char) -> *mut c_void {
//     let etag = Box::new(Client::new());
//     Box::into_raw(etag) as usize as *mut c_void
// }
