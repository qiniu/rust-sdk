use crate::{
    client::qiniu_ng_client_t,
    region::{qiniu_ng_region_t, qiniu_ng_regions_t},
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr},
    utils::{qiniu_ng_str_list_t, qiniu_ng_str_t},
};
use libc::{c_void, size_t};
use qiniu_ng::{
    storage::{bucket::Bucket, region::Region},
    Client,
};
use std::{
    borrow::Cow,
    mem::transmute,
    ptr::{null, null_mut},
};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_bucket_t(*mut c_void);

impl Default for qiniu_ng_bucket_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_bucket_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl<'r> From<qiniu_ng_bucket_t> for Option<Box<Bucket<'r>>> {
    fn from(bucket: qiniu_ng_bucket_t) -> Self {
        if bucket.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(bucket)) })
        }
    }
}

impl<'r> From<Option<Box<Bucket<'r>>>> for qiniu_ng_bucket_t {
    fn from(bucket: Option<Box<Bucket>>) -> Self {
        bucket.map(|bucket| bucket.into()).unwrap_or_default()
    }
}

impl<'r> From<Box<Bucket<'r>>> for qiniu_ng_bucket_t {
    fn from(bucket: Box<Bucket>) -> Self {
        unsafe { transmute(Box::into_raw(bucket)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_new(
    client: qiniu_ng_client_t,
    bucket_name: *const qiniu_ng_char_t,
) -> qiniu_ng_bucket_t {
    qiniu_ng_bucket_new2(client, bucket_name, null(), null(), 0)
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_new2(
    client: qiniu_ng_client_t,
    bucket_name: *const qiniu_ng_char_t,
    region: *const qiniu_ng_region_t,
    domains: *const *const qiniu_ng_char_t,
    domains_count: size_t,
) -> qiniu_ng_bucket_t {
    let client = Option::<Box<Client>>::from(client).unwrap();
    let bucket_name = unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap();
    let mut bucket_builder = client.storage().bucket(bucket_name);
    if let Some(region) = unsafe { region.as_ref() } {
        let region = Option::<Box<Cow<Region>>>::from(*region);
        if let Some(region) = region.as_ref() {
            bucket_builder = bucket_builder.region(region.to_owned().into_owned());
        }
        let _ = qiniu_ng_region_t::from(region);
    }
    for i in 0..domains_count {
        let domain = unsafe { *domains.add(i) };
        bucket_builder = bucket_builder.prepend_domain(unsafe { ucstr::from_ptr(domain) }.to_string().unwrap());
    }
    let bucket: qiniu_ng_bucket_t = Box::new(bucket_builder.build()).into();
    bucket.tap(|_| {
        let _ = qiniu_ng_client_t::from(client);
    })
}

// TODO: 设计 Bucket Builder

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_get_name(bucket: qiniu_ng_bucket_t) -> qiniu_ng_str_t {
    let bucket = Option::<Box<Bucket>>::from(bucket).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(bucket.name()) }.tap(|_| {
        let _ = qiniu_ng_bucket_t::from(bucket);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_free(bucket: *mut qiniu_ng_bucket_t) {
    if let Some(bucket) = unsafe { bucket.as_mut() } {
        let _ = Option::<Box<Bucket>>::from(*bucket);
        *bucket = qiniu_ng_bucket_t::default();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_is_freed(bucket: qiniu_ng_bucket_t) -> bool {
    bucket.is_null()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_get_region(
    bucket: qiniu_ng_bucket_t,
    region: *mut qiniu_ng_region_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let bucket = Option::<Box<Bucket>>::from(bucket).unwrap();
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
    let bucket = Option::<Box<Bucket>>::from(bucket).unwrap();
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
    let bucket = Option::<Box<Bucket>>::from(bucket).unwrap();
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
