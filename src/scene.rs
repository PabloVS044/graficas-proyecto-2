//! La escena: geometria, materiales, luces y cielo.

use crate::cube::Cube;
use crate::light::Light;
use crate::material::{Material, MaterialId};
use crate::ray_intersect::{Intersect, RayIntersect};
use nalgebra_glm::Vec3;

pub struct Scene {
    pub cubes: Vec<Cube>,
    pub materials: Vec<Material>,
    pub lights: Vec<Light>,

    /// Luz de relleno que llega de todas partes por igual.
    pub ambient: Vec3,

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

    /// Version acotada para los rayos de sombra: solo interesa si hay algo
    /// entre el punto y la luz, no que hay mas alla de ella.
    pub fn hit_within(&self, origin: &Vec3, direction: &Vec3, max_distance: f32) -> Intersect {
        let mut closest = Intersect::empty();
        for cube in &self.cubes {
            let hit = cube.ray_intersect(origin, direction);
            if hit.is_intersecting && hit.distance < closest.distance && hit.distance < max_distance
            {
                closest = hit;
            }
        }
        closest
    }

    #[inline]
    pub fn material(&self, id: MaterialId) -> &Material {
        &self.materials[id]
    }

    #[inline]
    pub fn sky(&self, direction: &Vec3) -> Vec3 {
        let t = (direction.y * 0.5 + 0.5).clamp(0.0, 1.0);
        self.sky_bottom * (1.0 - t) + self.sky_top * t
    }
}

// Indices de los materiales dentro de `Scene::materials`.
const STONE: MaterialId = 0;
const SAND: MaterialId = 1;
const RED: MaterialId = 2;
const BLUE: MaterialId = 3;
const MIRROR: MaterialId = 4;

/// Escena de prueba: una plataforma y unas torres.
pub fn build() -> Scene {
    let materials = vec![
        // Piedra mate: apenas brilla y el brillo es ancho.
        Material::new(Vec3::new(0.32, 0.35, 0.33), [0.90, 0.05, 0.0, 0.0], 8.0),
        // Arena, todavia mas mate.
        Material::new(Vec3::new(0.78, 0.62, 0.30), [0.90, 0.02, 0.0, 0.0], 4.0),
        // Ceramica roja pulida: brillo pequeño y marcado.
        Material::new(Vec3::new(0.80, 0.12, 0.08), [0.80, 0.45, 0.0, 0.0], 90.0),
        Material::new(Vec3::new(0.10, 0.28, 0.80), [0.80, 0.45, 0.0, 0.0], 90.0),
        // Metal pulido: casi no tiene color propio, lo que se ve en el es lo
        // que refleja. De ahi que el difuso sea minimo y la reflexion domine.
        Material::new(Vec3::new(0.85, 0.87, 0.90), [0.05, 0.60, 0.75, 0.0], 220.0),
    ];

    let mut cubes = Vec::new();

    for x in -4..5 {
        for z in -4..5 {
            let material = if (x + z) % 2 == 0 { STONE } else { SAND };
            cubes.push(Cube::unit(x, -1, z, material));
        }
    }

    cubes.push(Cube::unit(-2, 0, -2, RED));
    cubes.push(Cube::unit(-2, 1, -2, RED));
    cubes.push(Cube::unit(2, 0, -1, BLUE));
    cubes.push(Cube::unit(0, 0, 2, RED));

    // Columna de espejo: refleja las torres y el ajedrezado del suelo, que es
    // lo que permite comprobar de un vistazo que el rayo reflejado esta
    // saliendo con el angulo correcto.
    for y in 0..3 {
        cubes.push(Cube::unit(-1, y, -4, MIRROR));
        cubes.push(Cube::unit(0, y, -4, MIRROR));
    }

    let lights = vec![
        // Luz principal, lejana y sin caida: hace de sol.
        Light::sun(
            Vec3::new(12.0, 14.0, 8.0),
            Vec3::new(1.0, 0.96, 0.90),
            1.5,
        ),
        // Lampara calida y cercana, esta si con caida cuadratica. Sirve para
        // comprobar que la atenuacion se nota: solo ilumina lo que tiene al
        // lado.
        Light::new(
            Vec3::new(1.5, 1.5, 2.5),
            Vec3::new(1.0, 0.55, 0.25),
            14.0,
        ),
    ];

    Scene {
        cubes,
        materials,
        lights,
        ambient: Vec3::new(0.05, 0.06, 0.10),
        sky_top: Vec3::new(0.08, 0.22, 0.80),
        sky_bottom: Vec3::new(0.55, 0.68, 0.95),
    }
}
