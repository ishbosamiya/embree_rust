use std::collections::HashMap;

use generational_arena::{Arena, Index};

pub mod sys;

pub const INVALID_GEOMETRY_ID: u32 = sys::RTC_INVALID_GEOMETRY_ID;

#[derive(Debug)]
pub struct Embree {
    device: Device,
    scenes: Arena<Scene>,
    geometries: Arena<Geometry>,
    /// Map from GeometrySceneID to GeometryID, useful for when embree
    /// gives the GeometrySceneID but the user must be provided with
    /// the GeometryID.
    geometry_id_map: HashMap<GeometrySceneID, GeometryID>,
}

impl Embree {
    pub fn new() -> Self {
        Self {
            device: Device::new(),
            scenes: Arena::new(),
            geometries: Arena::new(),
            geometry_id_map: HashMap::new(),
        }
    }

    pub fn add_scene(&mut self) -> SceneID {
        self.add_scene_uncommitted()
    }

    fn add_scene_uncommitted(&mut self) -> SceneID {
        SceneID(
            self.scenes
                .insert(Scene::Uncommitted(SceneUncommitted::new(&self.device))),
        )
    }

    fn add_scene_committed(&mut self, scene: SceneCommitted) -> SceneID {
        SceneID(self.scenes.insert(Scene::Committed(scene)))
    }

    // TODO: it might not make sense to have Scene available to the user, need to decide
    // pub fn get_scene(&self, id: SceneID) -> Option<&Scene> {
    //     self.scenes.get(id.0)
    // }

    // pub fn get_scene_mut(&mut self, id: SceneID) -> Option<&mut Scene> {
    //     self.scenes.get_mut(id.0)
    // }

    pub fn add_geometry_triangle(&mut self, verts: &[Vert], indices: &[Triangle]) -> GeometryID {
        GeometryID(
            self.geometries
                .insert(Geometry::Triangle(GeometryTriangle::new(
                    &self.device,
                    verts,
                    indices,
                ))),
        )
    }

    pub fn add_geometry_sphere(&mut self, spheres: &[Sphere]) -> GeometryID {
        GeometryID(
            self.geometries
                .insert(Geometry::Sphere(GeometrySphere::new(&self.device, spheres))),
        )
    }

    /// commits the scene of the given id and returns the new id of
    /// the scene
    #[must_use = "the scene id will change, capture the new one"]
    pub fn commit_scene(&mut self, id: SceneID) -> SceneID {
        // TODO: propagate the error to the user
        let scene = self
            .scenes
            .remove(id.0)
            .expect("scene of given id is not available");

        let scene = match scene {
            Scene::Uncommitted(scene) => scene.commit(),
            Scene::Committed(scene) => scene,
        };

        self.add_scene_committed(scene)
    }

    pub fn attach_geometry_to_scene(&mut self, geometry_id: GeometryID, scene_id: SceneID) {
        // TODO: propagate the error to the user
        let geometry_scene_id = match self
            .scenes
            .get_mut(scene_id.0)
            .expect("scene of given id is not available")
        {
            Scene::Committed(_) => unreachable!("scene is committed already"),
            Scene::Uncommitted(scene) => scene.attach_geometry(
                self.geometries
                    .get(geometry_id.0)
                    .expect("geometry of given id is not available"),
            ),
        };

        let old_id = self.geometry_id_map.insert(geometry_scene_id, geometry_id);
        assert!(
            old_id.is_none(),
            "geometry might have been attached already"
        )
    }

    pub fn intersect_scene(&self, scene_id: SceneID, ray: Ray) -> RayHit {
        // TODO: propagate the error to the user
        match self
            .scenes
            .get(scene_id.0)
            .expect("scene of given id is not available")
        {
            Scene::Uncommitted(_) => unreachable!("scene must be committed, currently uncommitted"),
            Scene::Committed(scene) => scene.intersect(ray),
        }
    }

    pub fn get_geometry_id_from_geometry_scene_id(
        &self,
        geometry_scene_id: &GeometrySceneID,
    ) -> Option<&GeometryID> {
        self.geometry_id_map.get(geometry_scene_id)
    }
}

impl Default for Embree {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub(crate) struct Device {
    device: sys::RTCDevice,
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            sys::rtcReleaseDevice(self.device);
        }
        self.device = std::ptr::null_mut();
    }
}

unsafe impl Sync for Device {}
unsafe impl Send for Device {}

impl Device {
    pub fn new() -> Self {
        // TODO: add device config support
        let device = unsafe { sys::rtcNewDevice(std::ptr::null()) };
        assert_ne!(device, std::ptr::null_mut());
        Self { device }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SceneID(Index);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeometryID(Index);

#[derive(Debug)]
pub(crate) struct SceneUncommitted {
    scene: sys::RTCScene,
}
#[derive(Debug)]
pub(crate) struct SceneCommitted {
    scene: sys::RTCScene,
}

#[derive(Debug)]
pub(crate) enum Scene {
    Uncommitted(SceneUncommitted),
    Committed(SceneCommitted),
}

unsafe impl Sync for Scene {}
unsafe impl Send for Scene {}

impl Drop for SceneUncommitted {
    fn drop(&mut self) {
        unsafe {
            sys::rtcReleaseScene(self.scene);
        }
        self.scene = std::ptr::null_mut();
    }
}

impl Drop for SceneCommitted {
    fn drop(&mut self) {
        unsafe {
            sys::rtcReleaseScene(self.scene);
        }
        self.scene = std::ptr::null_mut();
    }
}

impl SceneUncommitted {
    pub(crate) fn new(device: &Device) -> Self {
        let scene = unsafe { sys::rtcNewScene(device.get_device()) };
        assert_ne!(scene, std::ptr::null_mut());
        Self { scene }
    }

    pub fn attach_geometry(&mut self, geometry: &Geometry) -> GeometrySceneID {
        GeometrySceneID(unsafe {
            sys::rtcAttachGeometry(self.get_scene(), geometry.get_geometry())
        })
    }

    pub fn commit(self) -> SceneCommitted {
        unsafe {
            sys::rtcCommitScene(self.get_scene());
        }

        // retain the scene so it is not dropped at the end of this
        // function
        unsafe {
            sys::rtcRetainScene(self.get_scene());
        }

        SceneCommitted { scene: self.scene }
    }

    /// # Safety
    ///
    /// If not handled correctly, can lead to memory leaks or other
    /// memory problems. It is always better to use the Rust API
    /// instead of trying to get access to the FFI parts directly.
    pub unsafe fn get_scene(&self) -> sys::RTCScene {
        self.scene
    }
}

impl SceneCommitted {
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
pub struct GeometrySceneID(pub u32);

// TODO: add support for other geometries
#[derive(Debug)]
pub(crate) enum Geometry {
    Triangle(GeometryTriangle),
    Sphere(GeometrySphere),
}

unsafe impl Sync for Geometry {}
unsafe impl Send for Geometry {}

impl Geometry {
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

#[derive(Debug)]
pub(crate) struct GeometryTriangle {
    geometry: sys::RTCGeometry,
}

unsafe impl Sync for GeometryTriangle {}
unsafe impl Send for GeometryTriangle {}

impl Drop for GeometryTriangle {
    fn drop(&mut self) {
        unsafe {
            sys::rtcReleaseGeometry(self.geometry);
        }

        self.geometry = std::ptr::null_mut();
    }
}

impl GeometryTriangle {
    pub(crate) fn new(device: &Device, verts: &[Vert], indices: &[Triangle]) -> Self {
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

        Self { geometry }
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

#[derive(Debug)]
pub(crate) struct GeometrySphere {
    geometry: sys::RTCGeometry,
}

unsafe impl Sync for GeometrySphere {}
unsafe impl Send for GeometrySphere {}

impl Drop for GeometrySphere {
    fn drop(&mut self) {
        unsafe {
            sys::rtcReleaseGeometry(self.geometry);
        }

        self.geometry = std::ptr::null_mut();
    }
}

impl GeometrySphere {
    pub(crate) fn new(device: &Device, spheres: &[Sphere]) -> Self {
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

        Self { geometry }
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
