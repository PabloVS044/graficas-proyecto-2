//! La escena: geometria y cielo.

use crate::cube::Cube;
use crate::ray_intersect::{Intersect, RayIntersect};
use nalgebra_glm::Vec3;

pub struct Scene {
    pub cubes: Vec<Cube>,
    /// Colores del degradado del cielo: lo que ve un rayo que no choca con nada.
    pub sky_top: Vec3,
    pub sky_bottom: Vec3,
}

impl Scene {
    /// Impacto mas cercano del rayo contra la escena.
    pub fn hit(&self, origin: &Vec3, direction: &Vec3) -> Intersect {
        let mut closest = Intersect::empty();
        for cube in &self.cubes {
            let hit = cube.ray_intersect(origin, direction);
            if hit.is_intersecting && hit.distance < closest.distance {
                closest = hit;
            }
        }
        closest
    }

    #[inline]
    pub fn sky(&self, direction: &Vec3) -> Vec3 {
        let t = (direction.y * 0.5 + 0.5).clamp(0.0, 1.0);
        self.sky_bottom * (1.0 - t) + self.sky_top * t
    }
}

/// Escena de prueba: una plataforma y unas torres, para comprobar que las
/// intersecciones resuelven bien que cubo tapa a cual y que las caras salen
/// orientadas como deben.
pub fn build() -> Scene {
    let grey = Vec3::new(0.45, 0.47, 0.45);
    let red = Vec3::new(0.75, 0.25, 0.20);
    let blue = Vec3::new(0.25, 0.40, 0.75);
    let sand = Vec3::new(0.80, 0.70, 0.45);

    let mut cubes = Vec::new();

    for x in -4..5 {
        for z in -4..5 {
            let color = if (x + z) % 2 == 0 { grey } else { sand };
            cubes.push(Cube::unit(x, -1, z, color));
        }
    }

    cubes.push(Cube::unit(-2, 0, -2, red));
    cubes.push(Cube::unit(-2, 1, -2, red));
    cubes.push(Cube::unit(2, 0, -1, blue));
    cubes.push(Cube::unit(0, 0, 2, red));

    Scene {
        cubes,
        sky_top: Vec3::new(0.20, 0.32, 0.60),
        sky_bottom: Vec3::new(0.70, 0.75, 0.85),
    }
}
