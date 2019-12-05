use crate::{
    client::qiniu_ng_client_t,
    region::{qiniu_ng_region_t, qiniu_ng_regions_t},
    result::qiniu_ng_err,
    utils::{convert_c_char_to_string, make_string, make_string_list, qiniu_ng_string_list_t, qiniu_ng_string_t},
};
use libc::{c_char, c_void};
use qiniu_ng::{
    storage::{bucket::Bucket, region::Region},
    Client,
};
use std::{borrow::Cow, mem::transmute, ptr::null};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
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
    domains_count: usize,
) -> qiniu_ng_bucket_t {
    let client: Box<Client> = client.into();
    let bucket_name = convert_c_char_to_string(bucket_name);
    let mut bucket_builder = client.storage().bucket(bucket_name);
    if let Some(region) = unsafe { region.as_ref() } {
        let region: Box<Cow<Region>> = region.to_owned().into();
        bucket_builder = bucket_builder.region(region.to_owned().into_owned());
        let _: qiniu_ng_region_t = region.into();
    }
    for i in 0..domains_count {
        bucket_builder = bucket_builder.domain(convert_c_char_to_string(unsafe { *domains.add(i) }));
    }
    let bucket: qiniu_ng_bucket_t = Box::new(bucket_builder.build()).into();
    bucket.tap(|_| {
        let _: qiniu_ng_client_t = client.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_get_name(bucket: qiniu_ng_bucket_t) -> qiniu_ng_string_t {
    let bucket: Box<Bucket> = bucket.into();
    make_string(bucket.name()).tap(|_| {
        let _: qiniu_ng_bucket_t = bucket.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_free(bucket: qiniu_ng_bucket_t) {
    let _: Box<Bucket> = bucket.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_get_region(
    bucket: qiniu_ng_bucket_t,
    region: *mut qiniu_ng_region_t,
    error: *mut qiniu_ng_err,
) -> bool {
    let bucket: Box<Bucket> = bucket.into();
    match bucket.region().map(|region| region.to_owned()).tap(|_| {
        let _: qiniu_ng_bucket_t = bucket.into();
    }) {
        Ok(r) => {
            if !region.is_null() {
                let r: Box<Cow<Region>> = Box::new(Cow::Owned(r));
                unsafe { *region = r.into() };
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
pub extern "C" fn qiniu_ng_bucket_get_regions(
    bucket: qiniu_ng_bucket_t,
    regions: *mut qiniu_ng_regions_t,
    error: *mut qiniu_ng_err,
) -> bool {
    let bucket: Box<Bucket> = bucket.into();
    match bucket
        .regions()
        .map(|iter| iter.map(|r| r.to_owned()).collect::<Box<[Region]>>())
        .tap(|_| {
            let _: qiniu_ng_bucket_t = bucket.into();
        }) {
        Ok(r) => {
            if !regions.is_null() {
                unsafe { *regions = r.into() };
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
pub extern "C" fn qiniu_ng_bucket_get_domains(
    bucket: qiniu_ng_bucket_t,
    domains: *mut qiniu_ng_string_list_t,
    error: *mut qiniu_ng_err,
) -> bool {
    let bucket: Box<Bucket> = bucket.into();
    match bucket.domains().map(|domains| make_string_list(&domains)).tap(|_| {
        let _: qiniu_ng_bucket_t = bucket.into();
    }) {
        Ok(ds) => {
            if !domains.is_null() {
                unsafe { *domains = ds };
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
