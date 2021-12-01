use embree_rust::{
    Device, Geometry, GeometryTriangle, Ray, Scene, SceneCommited, Triangle, Vec3, Vert,
    INVALID_GEOMETRY_ID,
};
use image::Pixel;

fn generate_cube() -> (Vec<Vert>, Vec<Triangle>) {
    let cube_verts = vec![
        Vert::new(Vec3::new(1.0, 1.0, 1.0)),
        Vert::new(Vec3::new(1.0, 1.0, -1.0)),
        Vert::new(Vec3::new(1.0, -1.0, 1.0)),
        Vert::new(Vec3::new(1.0, -1.0, -1.0)),
        Vert::new(Vec3::new(-1.0, 1.0, 1.0)),
        Vert::new(Vec3::new(-1.0, 1.0, -1.0)),
        Vert::new(Vec3::new(-1.0, -1.0, 1.0)),
        Vert::new(Vec3::new(-1.0, -1.0, -1.0)),
    ];

    let cube_indices = vec![
        Triangle::new(0, 3, 1),
        Triangle::new(0, 2, 3),
        Triangle::new(2, 7, 3),
        Triangle::new(2, 6, 3),
        Triangle::new(4, 2, 0),
        Triangle::new(4, 6, 2),
        Triangle::new(3, 5, 7),
        Triangle::new(3, 1, 5),
        Triangle::new(5, 7, 6),
        Triangle::new(5, 1, 4),
        Triangle::new(1, 5, 4),
        Triangle::new(1, 4, 0),
    ];

    (cube_verts, cube_indices)
}

fn trace_image(
    scene: &Scene<'_, SceneCommited>,
    width: usize,
    height: usize,
) -> image::DynamicImage {
    let camera_origin = (0.0, 0.0, -3.0);
    let camera_focal_length = 6.0;
    let camera_horizontal = 5.0;
    let camera_vertical = camera_horizontal * (height as f32 / width as f32);

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

            let ray_hit = scene.intersect(Ray::new(
                Vec3::new(camera_origin.0, camera_origin.1, camera_origin.2),
                0.001,
                1000.0,
                Vec3::new(ray_direction.0, ray_direction.1, ray_direction.2),
                0.0,
            ));

            let rgb = if ray_hit.hit.geomID != INVALID_GEOMETRY_ID {
                [ray_hit.hit.u, ray_hit.hit.v, 0.0]
            } else {
                [0.0, 0.0, 0.0]
            };

            let rgb = [
                (rgb[0].clamp(0.0, 1.0) * 255.0) as u8,
                (rgb[1].clamp(0.0, 1.0) * 255.0) as u8,
                (rgb[2].clamp(0.0, 1.0) * 255.0) as u8,
            ];
            *image::Rgb::from_slice(&rgb)
        },
    ))
}

fn main() {
    let device = Device::new();

    let mut scene = Scene::new(&device);

    let (cube_verts, cube_triangles) = generate_cube();
    let cube_geometry =
        Geometry::Triangle(GeometryTriangle::new(&device, &cube_verts, &cube_triangles));

    scene.attach_geometry(&cube_geometry);

    let scene = scene.commit();

    let viuer_config = viuer::Config {
        absolute_offset: false,
        ..Default::default()
    };

    let image = trace_image(&scene, 100, 100);

    viuer::print(&image, &viuer_config).unwrap();
}
