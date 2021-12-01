use std::os::raw::c_uint;

use embree_rust::sys;
use image::Pixel;

fn generate_cube_verts_and_indices() -> (Vec<(f32, f32, f32)>, Vec<c_uint>) {
    let cube_verts = vec![
        (1.0, 1.0, 1.0),
        (1.0, 1.0, -1.0),
        (1.0, -1.0, 1.0),
        (1.0, -1.0, -1.0),
        (-1.0, 1.0, 1.0),
        (-1.0, 1.0, -1.0),
        (-1.0, -1.0, 1.0),
        (-1.0, -1.0, -1.0),
    ];

    let cube_indices = vec![
        0, 3, 1, 0, 2, 3, 2, 7, 3, 2, 6, 3, 4, 2, 0, 4, 6, 2, 3, 5, 7, 3, 1, 5, 5, 7, 6, 5, 1, 4,
        1, 5, 4, 1, 4, 0,
    ];

    (cube_verts, cube_indices)
}

fn embree_generate_cube(device: sys::RTCDevice) -> sys::RTCGeometry {
    let (cube_verts, cube_indices) = generate_cube_verts_and_indices();

    let geometry =
        unsafe { sys::rtcNewGeometry(device, sys::RTCGeometryType_RTC_GEOMETRY_TYPE_TRIANGLE) };

    let verts: &mut [(f32, f32, f32)] = unsafe {
        std::slice::from_raw_parts_mut(
            sys::rtcSetNewGeometryBuffer(
                geometry,
                sys::RTCBufferType_RTC_BUFFER_TYPE_VERTEX,
                0,
                sys::RTCFormat_RTC_FORMAT_FLOAT3,
                std::mem::size_of::<(f32, f32, f32)>().try_into().unwrap(),
                8,
            ) as *mut (f32, f32, f32),
            8,
        )
    };

    verts
        .iter_mut()
        .zip(cube_verts.iter())
        .for_each(|(v1, v2)| {
            v1.0 = v2.0;
            v1.1 = v2.1;
            v1.2 = v2.2;
        });

    let indices: &mut [(c_uint, c_uint, c_uint)] = unsafe {
        std::slice::from_raw_parts_mut(
            sys::rtcSetNewGeometryBuffer(
                geometry,
                sys::RTCBufferType_RTC_BUFFER_TYPE_INDEX,
                0,
                sys::RTCFormat_RTC_FORMAT_UINT3,
                std::mem::size_of::<(c_uint, c_uint, c_uint)>()
                    .try_into()
                    .unwrap(),
                12,
            ) as *mut (c_uint, c_uint, c_uint),
            12,
        )
    };

    indices
        .iter_mut()
        .zip(cube_indices.chunks(3))
        .for_each(|(i, ci)| {
            i.0 = ci[0];
            i.1 = ci[1];
            i.2 = ci[2];
        });

    unsafe {
        sys::rtcSetGeometryBuildQuality(geometry, sys::RTCBuildQuality_RTC_BUILD_QUALITY_HIGH);
    }

    unsafe {
        sys::rtcCommitGeometry(geometry);
    }

    geometry
}

fn trace_ray(ray: sys::RTCRay, scene: sys::RTCScene) -> sys::RTCRayHit {
    let mut context = sys::RTCIntersectContext::default();

    let mut rayhit = sys::RTCRayHit {
        ray,
        hit: sys::RTCHit::default(),
    };

    unsafe { sys::rtcIntersect1(scene, &mut context, &mut rayhit) }

    rayhit
}

fn trace_image(scene: sys::RTCScene, width: usize, height: usize) -> image::DynamicImage {
    let camera_origin = (0.0, 0.0, -3.0);
    let camera_focal_length = 6.0;
    let camera_horizontal = 5.0;
    let camera_vertical = 5.0;

    image::DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(
        width.try_into().unwrap(),
        height.try_into().unwrap(),
        |x, y| {
            let u = (x as f32 / width as f32) * 2.0 - 1.0;
            let v = -((y as f32 / height as f32) * 2.0 - 1.0);

            let ray_direction = (
                u * camera_horizontal,
                v * camera_vertical,
                camera_focal_length,
            );

            let ray_hit = trace_ray(
                sys::RTCRay::new(
                    camera_origin.0,
                    camera_origin.1,
                    camera_origin.2,
                    0.001,
                    1000.0,
                    ray_direction.0,
                    ray_direction.1,
                    ray_direction.2,
                    0.0,
                ),
                scene,
            );

            let rgb = if ray_hit.hit.geomID != sys::RTC_INVALID_GEOMETRY_ID {
                [ray_hit.hit.u, ray_hit.hit.v, 0.0]
            } else {
                [0.0, 0.0, 0.0]
            };

            let rgb = [
                (rgb[0] * 255.0) as u8,
                (rgb[1] * 255.0) as u8,
                (rgb[2] * 255.0) as u8,
            ];
            *image::Rgb::from_slice(&rgb)
        },
    ))
}

fn main() {
    let device = unsafe { sys::rtcNewDevice(std::ptr::null()) };

    let scene = unsafe { sys::rtcNewScene(device) };

    let cube_geometry = embree_generate_cube(device);

    let _cube_id = unsafe { sys::rtcAttachGeometry(scene, cube_geometry) };

    unsafe {
        sys::rtcReleaseGeometry(cube_geometry);
    }

    unsafe {
        sys::rtcCommitScene(scene);
    }

    let viuer_config = viuer::Config {
        absolute_offset: false,
        ..Default::default()
    };

    let image = trace_image(scene, 100, 100);

    viuer::print(&image, &viuer_config).unwrap();
}
