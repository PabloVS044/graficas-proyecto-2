//! El raytracer: de un rayo a un color, y de la escena a un framebuffer.

use crate::camera::CameraFrame;
use crate::framebuffer::{Framebuffer, encode_pixel};
use crate::ray_intersect::EPSILON;
use crate::scene::Scene;
use nalgebra_glm::{Vec3, dot, length, normalize, reflect_vec};

/// Desplaza el origen de un rayo secundario para que no vuelva a chocar con la
/// superficie de la que sale.
///
/// Sin esto, el rayo de sombra que nace exactamente sobre la superficie vuelve
/// a intersectarla por error de redondeo y la marca como si estuviera en su
/// propia sombra: la imagen se llena de puntos negros ("shadow acne").
#[inline]
fn offset_origin(point: &Vec3, normal: &Vec3, direction: &Vec3) -> Vec3 {
    let side = if dot(direction, normal) < 0.0 { -1.0 } else { 1.0 };
    point + normal * (EPSILON * side)
}

/// `true` si hay algo opaco entre el punto y la luz.
fn in_shadow(scene: &Scene, point: &Vec3, normal: &Vec3, light_position: &Vec3) -> bool {
    let to_light = light_position - point;
    let distance = length(&to_light);
    let direction = to_light / distance;
    let origin = offset_origin(point, normal, &direction);

    // El limite de distancia importa: un objeto que este por detras de la luz no
    // puede taparla.
    scene
        .hit_within(&origin, &direction, distance)
        .is_intersecting
}

/// Lanza un rayo y devuelve el color que trae de vuelta.
pub fn cast_ray(scene: &Scene, origin: &Vec3, direction: &Vec3) -> Vec3 {
    let hit = scene.hit(origin, direction);
    if !hit.is_intersecting {
        return scene.sky(direction);
    }

    let material = scene.material(hit.material);
    let view = -direction;

    let mut diffuse = Vec3::zeros();
    let mut specular = Vec3::zeros();

    for light in &scene.lights {
        let light_dir = normalize(&(light.position - hit.point));

        // Reflexion difusa segun la ley del coseno de Lambert: lo que ilumina
        // una superficie no es la potencia de la luz sino cuanto la encara.
        let n_dot_l = dot(&hit.normal, &light_dir);
        if n_dot_l <= 0.0 {
            continue; // la cara le da la espalda a la luz
        }

        if in_shadow(scene, &hit.point, &hit.normal, &light.position) {
            continue;
        }

        let radiance = light.radiance_at(&hit.point);
        diffuse += radiance * n_dot_l;

        // Reflexion especular de Phong: el brillo aparece donde el reflejo del
        // rayo de luz apunta hacia el ojo. A diferencia del difuso, depende de
        // donde esta la camara, por eso se mueve con ella.
        let reflected = reflect_vec(&(-light_dir), &hit.normal);
        specular += radiance * dot(&reflected, &view).max(0.0).powf(material.specular);
    }

    let lit = diffuse + scene.ambient;
    let base = Vec3::new(
        material.diffuse.x * lit.x,
        material.diffuse.y * lit.y,
        material.diffuse.z * lit.z,
    );

    // El brillo especular no se tiñe con el color del material: es luz que
    // rebota en la superficie sin penetrarla, asi que conserva el color de la
    // lampara. Es lo que hace que una bola roja tenga un punto de brillo
    // blanco.
    base * material.albedo[0] + specular * material.albedo[1]
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
