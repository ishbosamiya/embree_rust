use std::marker::PhantomData;

pub mod sys;

pub const INVALID_GEOMETRY_ID: u32 = sys::RTC_INVALID_GEOMETRY_ID;

pub struct Device {
    device: sys::RTCDevice,
    _marker: std::marker::PhantomData<sys::RTCDevice>,
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            sys::rtcReleaseDevice(self.device);
        }
        self.device = std::ptr::null_mut();
    }
}

impl Device {
    pub fn new() -> Self {
        Self {
            // TODO: add device config support
            device: unsafe { sys::rtcNewDevice(std::ptr::null()) },
            _marker: PhantomData,
        }
    }

    /// # Safety
    ///
    /// If not handled correctly, can lead to memory leaks or other
    /// memory problems. It is always better to use the Rust API
    /// instead of trying to get access to the FFI parts directly.
    pub unsafe fn get_device(&self) -> sys::RTCDevice {
        self.device
    }
}

impl Default for Device {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SceneUncommited;
pub struct SceneCommited;

pub struct Scene<'a, CommitStatus = SceneUncommited> {
    scene: sys::RTCScene,
    /// Marker to make sure Scene lasts for as long as the [`Device`]
    /// used to create it
    _marker: std::marker::PhantomData<&'a Device>,
    commit_status: std::marker::PhantomData<CommitStatus>,
}

impl<CommitStatus> Drop for Scene<'_, CommitStatus> {
    fn drop(&mut self) {
        unsafe {
            sys::rtcReleaseScene(self.scene);
        }
        self.scene = std::ptr::null_mut();
    }
}

impl<'a, CommitStatus> Scene<'a, CommitStatus> {
    /// # Safety
    ///
    /// If not handled correctly, can lead to memory leaks or other
    /// memory problems. It is always better to use the Rust API
    /// instead of trying to get access to the FFI parts directly.
    pub unsafe fn get_scene(&self) -> sys::RTCScene {
        self.scene
    }
}

impl<'a> Scene<'a, SceneUncommited> {
    pub fn new(device: &'a Device) -> Self {
        Self {
            scene: unsafe { sys::rtcNewScene(device.get_device()) },
            _marker: PhantomData,
            commit_status: PhantomData,
        }
    }

    pub fn attach_geometry(&mut self, geometry: &Geometry) -> GeometryID {
        GeometryID(unsafe { sys::rtcAttachGeometry(self.get_scene(), geometry.get_geometry()) })
    }

    pub fn commit(self) -> Scene<'a, SceneCommited> {
        unsafe {
            sys::rtcCommitScene(self.get_scene());
        }

        // retain the scene so it is not dropped at the end of this
        // function
        unsafe {
            sys::rtcRetainScene(self.get_scene());
        }

        Scene {
            scene: self.scene,
            _marker: self._marker,
            commit_status: PhantomData,
        }
    }
}

impl<'a> Scene<'a, SceneCommited> {
    /// Intersect ray with the scene.
    ///
    /// TODO: add support for the other intersection types along with
    /// custom context creation.
    pub fn intersect(&self, ray: Ray) -> RayHit {
        let mut context = IntersectContext::default();

        let mut rayhit = RayHit {
            ray,
            hit: Hit::default(),
        };

        unsafe { sys::rtcIntersect1(self.scene, &mut context, &mut rayhit) }

        rayhit
    }
}

pub type Ray = sys::RTCRay;

impl Ray {
    pub fn new(origin: Vec3, tnear: f32, tfar: f32, direction: Vec3, time: f32) -> Self {
        Self {
            org_x: origin.x,
            org_y: origin.y,
            org_z: origin.z,
            tnear,
            dir_x: direction.x,
            dir_y: direction.y,
            dir_z: direction.z,
            time,
            tfar,
            mask: u32::MAX,
            id: 0,
            flags: 0,
        }
    }
}

pub type Hit = sys::RTCHit;

impl Default for Hit {
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

pub type RayHit = sys::RTCRayHit;

pub type IntersectContext = sys::RTCIntersectContext;

impl Default for IntersectContext {
    fn default() -> Self {
        Self {
            flags: sys::RTCIntersectContextFlags_RTC_INTERSECT_CONTEXT_FLAG_INCOHERENT,
            filter: None,
            instID: [sys::RTC_INVALID_GEOMETRY_ID],
        }
    }
}

/// 3 element vector
///
/// Do not add or remove elements!
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// Vertex that stores the position
///
/// Do not add or remove elements without making the necessary changes
/// everywhere else, such as places that use
/// `sys::rtcSetNewGeometryBuffer`, etc.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Vert {
    pos: Vec3,
}

impl Vert {
    pub fn new(pos: Vec3) -> Self {
        Self { pos }
    }
}

/// Stores the 3 indices from the [`Vert`] buffer that form the
/// triangle
///
/// Do not add or remove elements!
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct Triangle {
    pub i0: u32,
    pub i1: u32,
    pub i2: u32,
}

impl Triangle {
    pub fn new(i0: u32, i1: u32, i2: u32) -> Self {
        Self { i0, i1, i2 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeometryID(u32);

// TODO: add support for other geometries
pub enum Geometry<'a> {
    Triangle(GeometryTriangle<'a>),
    Sphere(GeometrySphere<'a>),
}

impl Geometry<'_> {
    /// # Safety
    ///
    /// If not handled correctly, can lead to memory leaks or other
    /// memory problems. It is always better to use the Rust API
    /// instead of trying to get access to the FFI parts directly.
    pub unsafe fn get_geometry(&self) -> sys::RTCGeometry {
        match self {
            Geometry::Triangle(geometry) => geometry.get_geometry(),
            Geometry::Sphere(geometry) => geometry.get_geometry(),
        }
    }
}

pub struct GeometryTriangle<'a> {
    geometry: sys::RTCGeometry,
    _marker: std::marker::PhantomData<&'a Device>,
}

impl Drop for GeometryTriangle<'_> {
    fn drop(&mut self) {
        unsafe {
            sys::rtcReleaseGeometry(self.geometry);
        }

        self.geometry = std::ptr::null_mut();
    }
}

impl<'a> GeometryTriangle<'a> {
    pub fn new(device: &'a Device, verts: &[Vert], indices: &[Triangle]) -> Self {
        let geometry = unsafe {
            sys::rtcNewGeometry(
                device.get_device(),
                sys::RTCGeometryType_RTC_GEOMETRY_TYPE_TRIANGLE,
            )
        };

        let verts_buffer: &mut [Vert] = unsafe {
            std::slice::from_raw_parts_mut(
                sys::rtcSetNewGeometryBuffer(
                    geometry,
                    sys::RTCBufferType_RTC_BUFFER_TYPE_VERTEX,
                    0,
                    sys::RTCFormat_RTC_FORMAT_FLOAT3,
                    std::mem::size_of::<Vert>().try_into().unwrap(),
                    verts.len().try_into().unwrap(),
                ) as *mut Vert,
                verts.len(),
            )
        };

        verts_buffer.copy_from_slice(verts);

        let indices_buffer: &mut [Triangle] = unsafe {
            std::slice::from_raw_parts_mut(
                sys::rtcSetNewGeometryBuffer(
                    geometry,
                    sys::RTCBufferType_RTC_BUFFER_TYPE_INDEX,
                    0,
                    sys::RTCFormat_RTC_FORMAT_UINT3,
                    std::mem::size_of::<Triangle>().try_into().unwrap(),
                    indices.len().try_into().unwrap(),
                ) as *mut Triangle,
                indices.len(),
            )
        };

        indices_buffer.copy_from_slice(indices);

        unsafe {
            sys::rtcSetGeometryBuildQuality(geometry, sys::RTCBuildQuality_RTC_BUILD_QUALITY_HIGH);
            sys::rtcCommitGeometry(geometry);
        }

        Self {
            geometry,
            _marker: PhantomData,
        }
    }

    /// # Safety
    ///
    /// If not handled correctly, can lead to memory leaks or other
    /// memory problems. It is always better to use the Rust API
    /// instead of trying to get access to the FFI parts directly.
    pub unsafe fn get_geometry(&self) -> sys::RTCGeometry {
        self.geometry
    }
}

/// Sphere, stores position and radius
///
/// Do not add or remove elements. Embree requires only position and
/// radius.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Sphere {
    pos: Vec3,
    radius: f32,
}

impl Sphere {
    pub fn new(pos: Vec3, radius: f32) -> Self {
        Self { pos, radius }
    }
}

pub struct GeometrySphere<'a> {
    geometry: sys::RTCGeometry,
    _marker: std::marker::PhantomData<&'a Device>,
}

impl Drop for GeometrySphere<'_> {
    fn drop(&mut self) {
        unsafe {
            sys::rtcReleaseGeometry(self.geometry);
        }

        self.geometry = std::ptr::null_mut();
    }
}

impl<'a> GeometrySphere<'a> {
    pub fn new(device: &'a Device, spheres: &[Sphere]) -> Self {
        let geometry = unsafe {
            sys::rtcNewGeometry(
                device.get_device(),
                sys::RTCGeometryType_RTC_GEOMETRY_TYPE_SPHERE_POINT,
            )
        };

        let spheres_buffer: &mut [Sphere] = unsafe {
            std::slice::from_raw_parts_mut(
                sys::rtcSetNewGeometryBuffer(
                    geometry,
                    sys::RTCBufferType_RTC_BUFFER_TYPE_VERTEX,
                    0,
                    sys::RTCFormat_RTC_FORMAT_FLOAT4,
                    std::mem::size_of::<Sphere>().try_into().unwrap(),
                    spheres.len().try_into().unwrap(),
                ) as *mut Sphere,
                spheres.len(),
            )
        };

        spheres_buffer.copy_from_slice(spheres);

        unsafe {
            sys::rtcSetGeometryBuildQuality(geometry, sys::RTCBuildQuality_RTC_BUILD_QUALITY_HIGH);
            sys::rtcCommitGeometry(geometry);
        }

        Self {
            geometry,
            _marker: PhantomData,
        }
    }

    /// # Safety
    ///
    /// If not handled correctly, can lead to memory leaks or other
    /// memory problems. It is always better to use the Rust API
    /// instead of trying to get access to the FFI parts directly.
    pub unsafe fn get_geometry(&self) -> sys::RTCGeometry {
        self.geometry
    }
}

#[cfg(test)]
mod tests {
    use std::os::raw::c_uint;

    use crate::{Sphere, Triangle, Vert};

    /// [`c_uint`] should never be smaller or larger than [`u32`]
    #[test]
    fn c_uint_size_contraint() {
        assert_eq!(std::mem::size_of::<c_uint>(), std::mem::size_of::<u32>());
    }

    /// [`Vert`] should never be smaller or larger
    #[test]
    fn vert_size_constraint() {
        assert_eq!(std::mem::size_of::<Vert>(), 4 + 4 + 4);
    }

    /// [`Triangle`] should never be smaller or larger
    #[test]
    fn triangle_size_constraint() {
        assert_eq!(std::mem::size_of::<Triangle>(), 4 + 4 + 4);
    }

    /// [`Sphere`] should never be smaller or larger
    #[test]
    fn sphere_size_constraint() {
        assert_eq!(std::mem::size_of::<Sphere>(), 4 + 4 + 4 + 4);
    }
}
