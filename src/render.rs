//! El raytracer: de un rayo a un color, y de la escena a un framebuffer.

use crate::camera::CameraFrame;
use crate::framebuffer::{Framebuffer, encode_pixel};
use crate::scene::Scene;
use nalgebra_glm::{Vec3, dot, normalize};

/// Lanza un rayo y devuelve el color que trae de vuelta.
pub fn cast_ray(scene: &Scene, origin: &Vec3, direction: &Vec3) -> Vec3 {
    let hit = scene.hit(origin, direction);
    if !hit.is_intersecting {
        return scene.sky(direction);
    }

    let material = scene.material(hit.material);

    // Reflexion difusa segun la ley del coseno de Lambert: lo que ilumina una
    // superficie no es la potencia de la luz sino cuanto la encara. Una cara
    // perpendicular al rayo de luz recibe toda la energia; una inclinada reparte
    // la misma energia sobre mas superficie y se ve mas oscura.
    let mut diffuse = Vec3::zeros();
    for light in &scene.lights {
        let light_dir = normalize(&(light.position - hit.point));
        let n_dot_l = dot(&hit.normal, &light_dir);
        if n_dot_l <= 0.0 {
            continue; // la cara le da la espalda a la luz
        }
        diffuse += light.radiance_at(&hit.point) * n_dot_l;
    }

    // La luz ambiente evita que lo que no recibe luz directa quede en negro
    // absoluto, que se ve plano y artificial.
    let total = diffuse + scene.ambient;

    Vec3::new(
        material.diffuse.x * total.x,
        material.diffuse.y * total.y,
        material.diffuse.z * total.z,
    ) * material.albedo
}

/// Un raytracer no es mas que un bucle de dos dimensiones que recorre la
/// pantalla lanzando un rayo por pixel.
pub fn render(framebuffer: &mut Framebuffer, scene: &Scene, camera: &CameraFrame) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            // Se muestrea el centro del pixel, de ahi el 0.5.
            let direction =
                camera.ray_direction(x as f32 + 0.5, y as f32 + 0.5, width, height);
            let color = cast_ray(scene, &camera.eye, &direction);
            framebuffer.set(x, y, encode_pixel(color));
        }
    }
}
