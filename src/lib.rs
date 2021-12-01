use std::os::raw::c_uint;

pub mod sys;

impl Default for sys::RTCIntersectContext {
    fn default() -> Self {
        Self {
            flags: sys::RTCIntersectContextFlags_RTC_INTERSECT_CONTEXT_FLAG_INCOHERENT,
            filter: None,
            instID: [sys::RTC_INVALID_GEOMETRY_ID],
        }
    }
}

impl sys::RTCRay {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        org_x: f32,
        org_y: f32,
        org_z: f32,
        tnear: f32,
        tfar: f32,
        dir_x: f32,
        dir_y: f32,
        dir_z: f32,
        time: f32,
    ) -> Self {
        Self {
            org_x,
            org_y,
            org_z,
            tnear,
            dir_x,
            dir_y,
            dir_z,
            time,
            tfar,
            mask: c_uint::MAX,
            id: 0,
            flags: 0,
        }
    }
}

impl Default for sys::RTCHit {
    fn default() -> Self {
        Self {
            Ng_x: 0.0,
            Ng_y: 0.0,
            Ng_z: 0.0,
            u: 0.0,
            v: 0.0,
            primID: sys::RTC_INVALID_GEOMETRY_ID,
            geomID: sys::RTC_INVALID_GEOMETRY_ID,
            instID: [sys::RTC_INVALID_GEOMETRY_ID],
        }
    }
}
