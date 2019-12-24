use crate::{
    config::qiniu_ng_config_t,
    result::qiniu_ng_err,
    utils::{qiniu_ng_string_list_t, qiniu_ng_string_t},
};
use libc::{c_char, c_void, size_t};
use qiniu_ng::storage::region::{Region, RegionId};
use std::{borrow::Cow, ffi::CStr, mem::transmute};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
pub enum qiniu_ng_region_id_t {
    Z0,
    Z1,
    Z2,
    AS0,
    NA0,
}

impl qiniu_ng_region_id_t {
    pub fn as_cstr(self) -> &'static CStr {
        match self {
            qiniu_ng_region_id_t::Z0 => unsafe { CStr::from_bytes_with_nul_unchecked(b"z0\0") },
            qiniu_ng_region_id_t::Z1 => unsafe { CStr::from_bytes_with_nul_unchecked(b"z1\0") },
            qiniu_ng_region_id_t::Z2 => unsafe { CStr::from_bytes_with_nul_unchecked(b"z2\0") },
            qiniu_ng_region_id_t::AS0 => unsafe { CStr::from_bytes_with_nul_unchecked(b"as0\0") },
            qiniu_ng_region_id_t::NA0 => unsafe { CStr::from_bytes_with_nul_unchecked(b"na0\0") },
        }
    }
}

impl From<RegionId> for qiniu_ng_region_id_t {
    fn from(region_id: RegionId) -> Self {
        match region_id {
            RegionId::Z0 => qiniu_ng_region_id_t::Z0,
            RegionId::Z1 => qiniu_ng_region_id_t::Z1,
            RegionId::Z2 => qiniu_ng_region_id_t::Z2,
            RegionId::AS0 => qiniu_ng_region_id_t::AS0,
            RegionId::NA0 => qiniu_ng_region_id_t::NA0,
        }
    }
}

impl From<qiniu_ng_region_id_t> for RegionId {
    fn from(region_id: qiniu_ng_region_id_t) -> Self {
        match region_id {
            qiniu_ng_region_id_t::Z0 => RegionId::Z0,
            qiniu_ng_region_id_t::Z1 => RegionId::Z1,
            qiniu_ng_region_id_t::Z2 => RegionId::Z2,
            qiniu_ng_region_id_t::AS0 => RegionId::AS0,
            qiniu_ng_region_id_t::NA0 => RegionId::NA0,
        }
    }
}

impl From<qiniu_ng_region_id_t> for *const c_char {
    fn from(region_id: qiniu_ng_region_id_t) -> Self {
        region_id.as_cstr().as_ptr()
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_id_name(region_id: qiniu_ng_region_id_t) -> *const c_char {
    region_id.into()
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_region_t(*mut c_void);

impl From<qiniu_ng_region_t> for Box<Cow<'static, Region>> {
    fn from(region: qiniu_ng_region_t) -> Self {
        unsafe { Box::from_raw(transmute::<_, *mut Cow<'static, Region>>(region)) }
    }
}

impl From<Box<Cow<'static, Region>>> for qiniu_ng_region_t {
    fn from(region: Box<Cow<'static, Region>>) -> Self {
        unsafe { transmute(Box::into_raw(region)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_region_id(
    region: qiniu_ng_region_t,
    region_id: *mut qiniu_ng_region_id_t,
) -> bool {
    let region: Box<Cow<'static, Region>> = region.into();
    match region.region_id().tap(|_| {
        let _: qiniu_ng_region_t = region.into();
    }) {
        Some(rid) => {
            if !region_id.is_null() {
                unsafe { *region_id = rid.into() };
            }
            true
        }
        None => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_region_by_id(region_id: qiniu_ng_region_id_t) -> qiniu_ng_region_t {
    match region_id {
        qiniu_ng_region_id_t::Z0 => Box::new(Cow::Borrowed(Region::z0())).into(),
        qiniu_ng_region_id_t::Z1 => Box::new(Cow::Borrowed(Region::z1())).into(),
        qiniu_ng_region_id_t::Z2 => Box::new(Cow::Borrowed(Region::z2())).into(),
        qiniu_ng_region_id_t::AS0 => Box::new(Cow::Borrowed(Region::as0())).into(),
        qiniu_ng_region_id_t::NA0 => Box::new(Cow::Borrowed(Region::na0())).into(),
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_up_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_string_list_t {
    let region: Box<Cow<'static, Region>> = region.into();
    unsafe { qiniu_ng_string_list_t::from_str_slice_unchecked(&region.up_urls(use_https)) }.tap(|_| {
        let _: qiniu_ng_region_t = region.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_io_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_string_list_t {
    let region: Box<Cow<'static, Region>> = region.into();
    unsafe { qiniu_ng_string_list_t::from_str_slice_unchecked(&region.io_urls(use_https)) }.tap(|_| {
        let _: qiniu_ng_region_t = region.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_rs_url(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_string_t {
    let region: Box<Cow<'static, Region>> = region.into();
    unsafe { qiniu_ng_string_t::from_str_unchecked(region.rs_url(use_https)) }.tap(|_| {
        let _: qiniu_ng_region_t = region.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_rsf_url(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_string_t {
    let region: Box<Cow<'static, Region>> = region.into();
    unsafe { qiniu_ng_string_t::from_str_unchecked(region.rsf_url(use_https)) }.tap(|_| {
        let _: qiniu_ng_region_t = region.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_api_url(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_string_t {
    let region: Box<Cow<'static, Region>> = region.into();
    unsafe { qiniu_ng_string_t::from_str_unchecked(region.api_url(use_https)) }.tap(|_| {
        let _: qiniu_ng_region_t = region.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_get_uc_url(use_https: bool) -> *const c_char {
    if use_https {
        unsafe { CStr::from_bytes_with_nul_unchecked(b"https://uc.qbox.me\0") }.as_ptr()
    } else {
        unsafe { CStr::from_bytes_with_nul_unchecked(b"http://uc.qbox.me\0") }.as_ptr()
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_get_uplog_url() -> *const c_char {
    unsafe { CStr::from_bytes_with_nul_unchecked(b"https://uplog.qbox.me\0") }.as_ptr()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_query(
    bucket_name: *const c_char,
    access_key: *const c_char,
    config: qiniu_ng_config_t,
    regions: *mut qiniu_ng_regions_t,
    error: *mut qiniu_ng_err,
) -> bool {
    match Region::query(
        unsafe { CStr::from_ptr(bucket_name) }.to_str().unwrap().to_owned(),
        unsafe { CStr::from_ptr(access_key) }.to_str().unwrap().to_owned(),
        config.get_clone(),
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
    let _: Box<Cow<'static, Region>> = region.into();
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_regions_t(*mut c_void, *mut c_void);

impl From<qiniu_ng_regions_t> for Box<[Region]> {
    fn from(regions: qiniu_ng_regions_t) -> Self {
        unsafe { Box::from_raw(transmute::<_, *mut [Region]>(regions)) }
    }
}

impl From<Box<[Region]>> for qiniu_ng_regions_t {
    fn from(regions: Box<[Region]>) -> Self {
        unsafe { transmute(Box::into_raw(regions)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_regions_len(regions: qiniu_ng_regions_t) -> size_t {
    let regions: Box<[Region]> = regions.into();
    regions.len().tap(|_| {
        let _: qiniu_ng_regions_t = regions.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_regions_get(
    regions: qiniu_ng_regions_t,
    index: size_t,
    region: *mut qiniu_ng_region_t,
) -> bool {
    let regions: Box<[Region]> = regions.into();
    let mut got = false;
    if let Some(r) = regions.get(index) {
        if !region.is_null() {
            let r: Box<Cow<Region>> = Box::new(Cow::Owned(r.to_owned()));
            unsafe { *region = r.into() };
        }
        got = true;
    }
    let _: qiniu_ng_regions_t = regions.into();
    got
}

#[no_mangle]
pub extern "C" fn qiniu_ng_regions_free(regions: qiniu_ng_regions_t) {
    let _: Box<[Region]> = regions.into();
}
