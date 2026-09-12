//! Resultado de intersectar un rayo con un objeto y el rasgo que lo produce.

use nalgebra_glm::Vec3;

/// Desplazamiento minimo que se aplicara al origen de los rayos secundarios
/// para que no vuelvan a chocar con la superficie de la que salen.
pub const EPSILON: f32 = 1e-3;

/// Todo lo que hace falta saber del punto donde pego el rayo.
#[derive(Debug, Clone, Copy)]
pub struct Intersect {
    pub is_intersecting: bool,

    /// Distancia desde el origen del rayo. Es lo que resuelve que objeto queda
    /// enfrente, sin z-buffer ni ordenar nada a mano.
    pub distance: f32,

    /// Normal de la cara, apuntando hacia afuera del cubo.
    pub normal: Vec3,

    pub color: Vec3,
}

impl Intersect {
    /// El "no choco con nada": distancia infinita para que cualquier impacto
    /// real gane la comparacion.
    pub fn empty() -> Self {
        Intersect {
            is_intersecting: false,
            distance: f32::INFINITY,
            normal: Vec3::zeros(),
            color: Vec3::zeros(),
        }
    }
}

pub trait RayIntersect {
    /// Intersecta el rayo `origin + t * direction` (con `direction` normalizada)
    /// y devuelve el impacto mas cercano con `t > EPSILON`.
    fn ray_intersect(&self, origin: &Vec3, direction: &Vec3) -> Intersect;
}
