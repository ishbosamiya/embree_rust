#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::c_uint;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// must keep consistent with RTC_INVALID_GEOMETRY_ID
pub const RTC_INVALID_GEOMETRY_ID: c_uint = c_uint::MAX;
