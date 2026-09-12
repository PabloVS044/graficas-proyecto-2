//! El raytracer: de un rayo a un color, y de la escena a un framebuffer.

use crate::camera::CameraFrame;
use crate::framebuffer::{Framebuffer, encode_pixel};
use crate::scene::Scene;
use nalgebra_glm::Vec3;

/// Tinte fijo por orientacion de la cara.
///
/// Todavia no hay luces: esto solo sirve para comprobar a ojo que las normales
/// salen bien orientadas. Si una cara lateral se viera igual de clara que la de
/// arriba, o el cubo se viera plano, seria senal de que el metodo de slabs esta
/// devolviendo el eje o el signo equivocados. Lo sustituye la iluminacion de
/// verdad en cuanto haya materiales y luces.
#[inline]
fn face_tint(normal: &Vec3) -> f32 {
    if normal.y > 0.5 {
        1.00 // arriba
    } else if normal.y < -0.5 {
        0.50 // abajo
    } else if normal.x.abs() > 0.5 {
        0.72 // este y oeste
    } else {
        0.86 // norte y sur
    }
}

/// Lanza un rayo y devuelve el color que trae de vuelta.
pub fn cast_ray(scene: &Scene, origin: &Vec3, direction: &Vec3) -> Vec3 {
    let hit = scene.hit(origin, direction);
    if hit.is_intersecting {
        hit.color * face_tint(&hit.normal)
    } else {
        scene.sky(direction)
    }
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
