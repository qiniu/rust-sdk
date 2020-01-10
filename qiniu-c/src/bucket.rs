use crate::{
    client::qiniu_ng_client_t,
    region::{qiniu_ng_region_t, qiniu_ng_regions_t},
    result::qiniu_ng_err_t,
    utils::{qiniu_ng_str_list_t, qiniu_ng_str_t},
};
use libc::{c_char, c_void, size_t};
use qiniu_ng::{
    storage::{bucket::Bucket, region::Region},
    Client,
};
use std::{borrow::Cow, ffi::CStr, mem::transmute, ptr::null};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_bucket_t(*mut c_void);

impl<'r> From<qiniu_ng_bucket_t> for Box<Bucket<'r>> {
    fn from(bucket: qiniu_ng_bucket_t) -> Self {
        unsafe { Box::from_raw(transmute(bucket)) }
    }
}

impl<'r> From<Box<Bucket<'r>>> for qiniu_ng_bucket_t {
    fn from(bucket: Box<Bucket>) -> Self {
        unsafe { transmute(Box::into_raw(bucket)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_new(client: qiniu_ng_client_t, bucket_name: *const c_char) -> qiniu_ng_bucket_t {
    qiniu_ng_bucket_new2(client, bucket_name, null(), null(), 0)
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_new2(
    client: qiniu_ng_client_t,
    bucket_name: *const c_char,
    region: *const qiniu_ng_region_t,
    domains: *const *const c_char,
    domains_count: size_t,
) -> qiniu_ng_bucket_t {
    let client = Box::<Client>::from(client);
    let bucket_name = unsafe { CStr::from_ptr(bucket_name) }.to_str().unwrap().to_owned();
    let mut bucket_builder = client.storage().bucket(bucket_name);
    if let Some(region) = unsafe { region.as_ref() } {
        let region = Box::<Cow<Region>>::from(region.to_owned());
        bucket_builder = bucket_builder.region(region.to_owned().into_owned());
        let _ = qiniu_ng_region_t::from(region);
    }
    for i in 0..domains_count {
        let domain = unsafe { *domains.add(i) };
        bucket_builder = bucket_builder.domain(unsafe { CStr::from_ptr(domain) }.to_str().unwrap().to_owned());
    }
    let bucket: qiniu_ng_bucket_t = Box::new(bucket_builder.build()).into();
    bucket.tap(|_| {
        let _ = qiniu_ng_client_t::from(client);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_get_name(bucket: qiniu_ng_bucket_t) -> qiniu_ng_str_t {
    let bucket = Box::<Bucket>::from(bucket);
    unsafe { qiniu_ng_str_t::from_str_unchecked(bucket.name()) }.tap(|_| {
        let _ = qiniu_ng_bucket_t::from(bucket);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_free(bucket: qiniu_ng_bucket_t) {
    let _ = Box::<Bucket>::from(bucket);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_get_region(
    bucket: qiniu_ng_bucket_t,
    region: *mut qiniu_ng_region_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let bucket = Box::<Bucket>::from(bucket);
    match bucket.region().map(|region| region.to_owned()).tap(|_| {
        let _ = qiniu_ng_bucket_t::from(bucket);
    }) {
        Ok(r) => {
            if let Some(region) = unsafe { region.as_mut() } {
                *region = Box::<Cow<Region>>::new(Cow::Owned(r)).into();
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
pub extern "C" fn qiniu_ng_bucket_get_regions(
    bucket: qiniu_ng_bucket_t,
    regions: *mut qiniu_ng_regions_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let bucket: Box<Bucket> = bucket.into();
    match bucket
        .regions()
        .map(|iter| iter.map(|r| r.to_owned()).collect::<Box<[Region]>>())
        .tap(|_| {
            let _ = qiniu_ng_bucket_t::from(bucket);
        }) {
        Ok(r) => {
            if let Some(regions) = unsafe { regions.as_mut() } {
                *regions = r.into();
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
pub extern "C" fn qiniu_ng_bucket_get_domains(
    bucket: qiniu_ng_bucket_t,
    domains: *mut qiniu_ng_str_list_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let bucket: Box<Bucket> = bucket.into();
    match bucket
        .domains()
        .map(|domains| unsafe { qiniu_ng_str_list_t::from_str_slice_unchecked(&domains) })
        .tap(|_| {
            let _ = qiniu_ng_bucket_t::from(bucket);
        }) {
        Ok(ds) => {
            if let Some(domains) = unsafe { domains.as_mut() } {
                *domains = ds;
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
