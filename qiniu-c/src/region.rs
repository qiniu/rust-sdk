use super::{
    config::qiniu_ng_config_t,
    result::qiniu_ng_err,
    utils::{make_string, make_string_list, qiniu_ng_string_list_t, qiniu_ng_string_t},
};
use libc::{c_char, c_void, size_t};
use qiniu::storage::region;
use std::{ffi::CStr, mem};

#[repr(C)]
pub enum region_id_t {
    Z0,
    Z1,
    Z2,
    AS0,
    NA0,
}

impl region_id_t {
    pub fn as_cstr(&self) -> &'static CStr {
        match self {
            region_id_t::Z0 => CStr::from_bytes_with_nul(b"z0\0").unwrap(),
            region_id_t::Z1 => CStr::from_bytes_with_nul(b"z1\0").unwrap(),
            region_id_t::Z2 => CStr::from_bytes_with_nul(b"z2\0").unwrap(),
            region_id_t::AS0 => CStr::from_bytes_with_nul(b"as0\0").unwrap(),
            region_id_t::NA0 => CStr::from_bytes_with_nul(b"na0\0").unwrap(),
        }
    }
}

impl From<region::RegionId> for region_id_t {
    fn from(region_id: region::RegionId) -> Self {
        match region_id {
            region::RegionId::Z0 => region_id_t::Z0,
            region::RegionId::Z1 => region_id_t::Z1,
            region::RegionId::Z2 => region_id_t::Z2,
            region::RegionId::AS0 => region_id_t::AS0,
            region::RegionId::NA0 => region_id_t::NA0,
        }
    }
}

impl From<region_id_t> for region::RegionId {
    fn from(region_id: region_id_t) -> Self {
        match region_id {
            region_id_t::Z0 => region::RegionId::Z0,
            region_id_t::Z1 => region::RegionId::Z1,
            region_id_t::Z2 => region::RegionId::Z2,
            region_id_t::AS0 => region::RegionId::AS0,
            region_id_t::NA0 => region::RegionId::NA0,
        }
    }
}

impl From<region_id_t> for *const c_char {
    fn from(region_id: region_id_t) -> Self {
        region_id.as_cstr().as_ptr()
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_id_to_name(region_id: region_id_t) -> *const c_char {
    region_id.into()
}

#[repr(C)]
pub struct qiniu_ng_region_t(*mut c_void);

impl From<qiniu_ng_region_t> for Box<region::Region> {
    fn from(region: qiniu_ng_region_t) -> Self {
        unsafe { Box::from_raw(mem::transmute::<_, *mut region::Region>(region)) }
    }
}

impl From<Box<region::Region>> for qiniu_ng_region_t {
    fn from(region: Box<region::Region>) -> Self {
        unsafe { mem::transmute(Box::into_raw(region)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_up_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_string_list_t {
    let region: Box<region::Region> = region.into();
    let up_urls = make_string_list(region.up_urls(use_https));
    let _: qiniu_ng_region_t = region.into();
    up_urls
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_io_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_string_list_t {
    let region: Box<region::Region> = region.into();
    let up_urls = make_string_list(region.io_urls(use_https));
    let _: qiniu_ng_region_t = region.into();
    up_urls
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_rs_url(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_string_t {
    let region: Box<region::Region> = region.into();
    let rs_url = make_string(region.rs_url(use_https));
    let _: qiniu_ng_region_t = region.into();
    rs_url
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_rsf_url(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_string_t {
    let region: Box<region::Region> = region.into();
    let rsf_url = make_string(region.rsf_url(use_https));
    let _: qiniu_ng_region_t = region.into();
    rsf_url
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_api_url(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_string_t {
    let region: Box<region::Region> = region.into();
    let api_url = make_string(region.api_url(use_https));
    let _: qiniu_ng_region_t = region.into();
    api_url
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_query(
    bucket_name: *const c_char,
    access_key: *const c_char,
    config: *const qiniu_ng_config_t,
    regions: *mut qiniu_ng_regions_t,
    error: *mut qiniu_ng_err,
) -> bool {
    match region::Region::query(
        unsafe { CStr::from_ptr(bucket_name).to_string_lossy() },
        unsafe { CStr::from_ptr(access_key).to_string_lossy() },
        unsafe { config.as_ref() }.unwrap().into(),
    ) {
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
pub extern "C" fn qiniu_ng_region_free(region: qiniu_ng_region_t) {
    let _: Box<region::Region> = region.into();
}

#[repr(C)]
pub struct qiniu_ng_regions_t(*mut c_void, *mut c_void);

impl From<qiniu_ng_regions_t> for Box<[region::Region]> {
    fn from(regions: qiniu_ng_regions_t) -> Self {
        unsafe { Box::from_raw(mem::transmute::<_, *mut [region::Region]>(regions)) }
    }
}

impl From<Box<[region::Region]>> for qiniu_ng_regions_t {
    fn from(regions: Box<[region::Region]>) -> Self {
        unsafe { mem::transmute(Box::into_raw(regions)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_regions_len(regions: qiniu_ng_regions_t) -> size_t {
    let regions: Box<[region::Region]> = regions.into();
    let len = regions.len();
    let _: qiniu_ng_regions_t = regions.into();
    len
}

#[no_mangle]
pub extern "C" fn qiniu_ng_regions_get(
    regions: qiniu_ng_regions_t,
    index: size_t,
    region: *mut qiniu_ng_region_t,
) -> bool {
    let regions: Box<[region::Region]> = regions.into();
    let mut got = false;
    if let Some(r) = regions.get(index) {
        if !region.is_null() {
            unsafe { *region = Box::new(r.to_owned()).into() };
        }
        got = true;
    }
    let _: qiniu_ng_regions_t = regions.into();
    got
}

#[no_mangle]
pub extern "C" fn qiniu_ng_regions_free(regions: qiniu_ng_regions_t) {
    let _: Box<[region::Region]> = regions.into();
}
