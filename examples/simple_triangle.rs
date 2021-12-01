use embree_rust::sys;

fn main() {
    let test_str = std::ffi::CString::new("test").unwrap();
    println!("{:?}", unsafe { sys::rtcNewDevice(test_str.as_ptr()) });
}
