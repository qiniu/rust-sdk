use crate::{
    config::qiniu_ng_config_t,
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr},
    utils::qiniu_ng_str_list_t,
};
use libc::{c_char, c_void, size_t};
use qiniu_ng::storage::region::{Region, RegionId};
use std::{borrow::Cow, ffi::CStr, mem::transmute, ptr::null_mut};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_region_id_t {
    qiniu_ng_region_z0,
    qiniu_ng_region_z1,
    qiniu_ng_region_z2,
    qiniu_ng_region_as0,
    qiniu_ng_region_na0,
}

impl qiniu_ng_region_id_t {
    pub fn as_cstr(self) -> &'static CStr {
        match self {
            qiniu_ng_region_id_t::qiniu_ng_region_z0 => unsafe { CStr::from_bytes_with_nul_unchecked(b"z0\0") },
            qiniu_ng_region_id_t::qiniu_ng_region_z1 => unsafe { CStr::from_bytes_with_nul_unchecked(b"z1\0") },
            qiniu_ng_region_id_t::qiniu_ng_region_z2 => unsafe { CStr::from_bytes_with_nul_unchecked(b"z2\0") },
            qiniu_ng_region_id_t::qiniu_ng_region_as0 => unsafe { CStr::from_bytes_with_nul_unchecked(b"as0\0") },
            qiniu_ng_region_id_t::qiniu_ng_region_na0 => unsafe { CStr::from_bytes_with_nul_unchecked(b"na0\0") },
        }
    }
}

impl AsRef<str> for qiniu_ng_region_id_t {
    fn as_ref(&self) -> &str {
        match self {
            qiniu_ng_region_id_t::qiniu_ng_region_z0 => "z0",
            qiniu_ng_region_id_t::qiniu_ng_region_z1 => "z1",
            qiniu_ng_region_id_t::qiniu_ng_region_z2 => "z2",
            qiniu_ng_region_id_t::qiniu_ng_region_as0 => "as0",
            qiniu_ng_region_id_t::qiniu_ng_region_na0 => "na0",
        }
    }
}

impl From<RegionId> for qiniu_ng_region_id_t {
    fn from(region_id: RegionId) -> Self {
        match region_id {
            RegionId::Z0 => qiniu_ng_region_id_t::qiniu_ng_region_z0,
            RegionId::Z1 => qiniu_ng_region_id_t::qiniu_ng_region_z1,
            RegionId::Z2 => qiniu_ng_region_id_t::qiniu_ng_region_z2,
            RegionId::AS0 => qiniu_ng_region_id_t::qiniu_ng_region_as0,
            RegionId::NA0 => qiniu_ng_region_id_t::qiniu_ng_region_na0,
        }
    }
}

impl From<qiniu_ng_region_id_t> for RegionId {
    fn from(region_id: qiniu_ng_region_id_t) -> Self {
        match region_id {
            qiniu_ng_region_id_t::qiniu_ng_region_z0 => RegionId::Z0,
            qiniu_ng_region_id_t::qiniu_ng_region_z1 => RegionId::Z1,
            qiniu_ng_region_id_t::qiniu_ng_region_z2 => RegionId::Z2,
            qiniu_ng_region_id_t::qiniu_ng_region_as0 => RegionId::AS0,
            qiniu_ng_region_id_t::qiniu_ng_region_na0 => RegionId::NA0,
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

// TODO: 提供 Region Builder
// TODO: 提供 Region 的 clone() 方法

impl Default for qiniu_ng_region_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_region_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_region_t> for Option<Box<Cow<'static, Region>>> {
    fn from(region: qiniu_ng_region_t) -> Self {
        if region.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(region)) })
        }
    }
}

impl From<Option<Box<Cow<'static, Region>>>> for qiniu_ng_region_t {
    fn from(region: Option<Box<Cow<'static, Region>>>) -> Self {
        region.map(|region| region.into()).unwrap_or_default()
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
    let region = Option::<Box<Cow<'static, Region>>>::from(region).unwrap();
    match region.region_id().tap(|_| {
        let _ = qiniu_ng_region_t::from(region);
    }) {
        Some(rid) => {
            if let Some(region_id) = unsafe { region_id.as_mut() } {
                *region_id = rid.into();
            }
            true
        }
        None => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_region_by_id(region_id: qiniu_ng_region_id_t) -> qiniu_ng_region_t {
    match region_id {
        qiniu_ng_region_id_t::qiniu_ng_region_z0 => Box::new(Cow::Borrowed(Region::z0())).into(),
        qiniu_ng_region_id_t::qiniu_ng_region_z1 => Box::new(Cow::Borrowed(Region::z1())).into(),
        qiniu_ng_region_id_t::qiniu_ng_region_z2 => Box::new(Cow::Borrowed(Region::z2())).into(),
        qiniu_ng_region_id_t::qiniu_ng_region_as0 => Box::new(Cow::Borrowed(Region::as0())).into(),
        qiniu_ng_region_id_t::qiniu_ng_region_na0 => Box::new(Cow::Borrowed(Region::na0())).into(),
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_up_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_str_list_t {
    let region = Option::<Box<Cow<'static, Region>>>::from(region).unwrap();
    unsafe { qiniu_ng_str_list_t::from_str_slice_unchecked(&region.up_urls_ref(use_https)) }.tap(|_| {
        let _ = qiniu_ng_region_t::from(region);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_io_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_str_list_t {
    let region = Option::<Box<Cow<'static, Region>>>::from(region).unwrap();
    unsafe { qiniu_ng_str_list_t::from_str_slice_unchecked(&region.io_urls_ref(use_https)) }.tap(|_| {
        let _ = qiniu_ng_region_t::from(region);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_rs_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_str_list_t {
    let region = Option::<Box<Cow<'static, Region>>>::from(region).unwrap();
    unsafe { qiniu_ng_str_list_t::from_str_slice_unchecked(&region.rs_urls_ref(use_https)) }.tap(|_| {
        let _ = qiniu_ng_region_t::from(region);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_rsf_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_str_list_t {
    let region = Option::<Box<Cow<'static, Region>>>::from(region).unwrap();
    unsafe { qiniu_ng_str_list_t::from_str_slice_unchecked(&region.rsf_urls_ref(use_https)) }.tap(|_| {
        let _ = qiniu_ng_region_t::from(region);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_api_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_str_list_t {
    let region = Option::<Box<Cow<'static, Region>>>::from(region).unwrap();
    unsafe { qiniu_ng_str_list_t::from_str_slice_unchecked(&region.api_urls_ref(use_https)) }.tap(|_| {
        let _ = qiniu_ng_region_t::from(region);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_query(
    bucket_name: *const qiniu_ng_char_t,
    access_key: *const qiniu_ng_char_t,
    config: qiniu_ng_config_t,
    regions: *mut qiniu_ng_regions_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    match Region::query(
        unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap(),
        unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
        config.get_clone().unwrap(),
    ) {
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
pub extern "C" fn qiniu_ng_region_free(region: *mut qiniu_ng_region_t) {
    if let Some(region) = unsafe { region.as_mut() } {
        let _ = Option::<Box<Cow<'static, Region>>>::from(*region);
        *region = qiniu_ng_region_t::default();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_region_is_freed(region: qiniu_ng_region_t) -> bool {
    region.is_null()
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_regions_t(*mut c_void, *mut c_void);

impl Default for qiniu_ng_regions_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut(), null_mut())
    }
}

impl qiniu_ng_regions_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null() && self.1.is_null()
    }
}

impl From<qiniu_ng_regions_t> for Option<Box<[Region]>> {
    fn from(regions: qiniu_ng_regions_t) -> Self {
        if regions.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(regions)) })
        }
    }
}

impl From<Option<Box<[Region]>>> for qiniu_ng_regions_t {
    fn from(regions: Option<Box<[Region]>>) -> Self {
        regions.map(|regions| regions.into()).unwrap_or_default()
    }
}

impl From<Box<[Region]>> for qiniu_ng_regions_t {
    fn from(regions: Box<[Region]>) -> Self {
        unsafe { transmute(Box::into_raw(regions)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_regions_len(regions: qiniu_ng_regions_t) -> size_t {
    let regions = Option::<Box<[Region]>>::from(regions).unwrap();
    regions.len().tap(|_| {
        let _ = qiniu_ng_regions_t::from(regions);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_regions_get(
    regions: qiniu_ng_regions_t,
    index: size_t,
    region: *mut qiniu_ng_region_t,
) -> bool {
    let regions = Option::<Box<[Region]>>::from(regions).unwrap();
    let mut got = false;
    if let Some(r) = regions.get(index) {
        if let Some(region) = unsafe { region.as_mut() } {
            *region = Box::<Cow<Region>>::new(Cow::Owned(r.to_owned())).into();
        }
        got = true;
    }
    let _ = qiniu_ng_regions_t::from(regions);
    got
}

#[no_mangle]
pub extern "C" fn qiniu_ng_regions_free(regions: *mut qiniu_ng_regions_t) {
    if let Some(regions) = unsafe { regions.as_mut() } {
        let _ = Option::<Box<[Region]>>::from(*regions);
        *regions = qiniu_ng_regions_t::default();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_regions_is_freed(regions: qiniu_ng_regions_t) -> bool {
    regions.is_null()
}
