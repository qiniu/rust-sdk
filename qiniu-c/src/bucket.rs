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
use std::{borrow::Cow, mem::transmute};
use tap::TapOps;

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

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket(client: qiniu_ng_client_t, bucket_name: *const c_char) -> qiniu_ng_bucket_t {
    let client: Box<Client> = client.into();
    let bucket_name = convert_c_char_to_string(bucket_name);
    let bucket: qiniu_ng_bucket_t = Box::new(client.storage().bucket(bucket_name).build()).into();
    bucket.tap(|_| {
        let _: qiniu_ng_client_t = client.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_name(bucket: qiniu_ng_bucket_t) -> qiniu_ng_string_t {
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
pub extern "C" fn qiniu_ng_bucket_region(
    bucket: qiniu_ng_bucket_t,
    region: *mut qiniu_ng_region_t,
    error: *mut qiniu_ng_err,
) -> bool {
    let bucket: Box<Bucket> = bucket.into();
    match bucket
        .region()
        .map(|region| Box::new(Cow::Owned(region.to_owned())) as Box<Cow<Region>>)
        .tap(|_| {
            let _: qiniu_ng_bucket_t = bucket.into();
        }) {
        Ok(r) => {
            if !region.is_null() {
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
pub extern "C" fn qiniu_ng_bucket_regions(
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
pub extern "C" fn qiniu_ng_bucket_domains(
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
