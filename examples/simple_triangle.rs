use embree_rust::sys;

fn main() {
    let device = unsafe { sys::rtcNewDevice(std::ptr::null()) };

    let scene = unsafe { sys::rtcNewScene(device) };

    unsafe {
        sys::rtcCommitScene(scene);
    }

    let mut context = sys::RTCIntersectContext::default();

    let mut rayhit = sys::RTCRayHit {
        ray: sys::RTCRay::new(0.0, 0.0, 0.0, 0.001, 1000.0, 0.0, 0.0, 1.0, 0.0),
        hit: sys::RTCHit::default(),
    };

    unsafe { sys::rtcIntersect1(scene, &mut context, &mut rayhit) }

    assert_eq!(rayhit.hit.geomID, sys::RTC_INVALID_GEOMETRY_ID);
}
