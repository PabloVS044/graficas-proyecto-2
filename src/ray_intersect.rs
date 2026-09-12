//! Resultado de intersectar un rayo con un objeto y el rasgo que lo produce.

use crate::material::MaterialId;
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

    /// Punto de impacto en coordenadas de mundo. Hace falta para medir la
    /// direccion y la distancia a cada luz.
    pub point: Vec3,

    /// Normal geometrica de la cara, siempre apuntando hacia **afuera** del
    /// cubo. Si el rayo viene de adentro (caso de la refraccion) apunta en el
    /// mismo sentido que el rayo y el sombreador la invierte.
    pub normal: Vec3,

    /// Direccion en la que crece `u` sobre la cara golpeada.
    pub tangent: Vec3,

    /// Direccion en la que **decrece** `v` sobre la cara, o sea "hacia arriba"
    /// en la textura. Junto con `tangent` y `normal` forma la base TBN que
    /// convierte la normal leida del normal map a coordenadas de mundo.
    pub bitangent: Vec3,

    /// Coordenadas de textura de la cara, en [0, 1].
    pub u: f32,
    pub v: f32,

    pub material: MaterialId,
}

impl Intersect {
    /// El "no choco con nada": distancia infinita para que cualquier impacto
    /// real gane la comparacion.
    pub fn empty() -> Self {
        Intersect {
            is_intersecting: false,
            distance: f32::INFINITY,
            point: Vec3::zeros(),
            normal: Vec3::zeros(),
            tangent: Vec3::zeros(),
            bitangent: Vec3::zeros(),
            u: 0.0,
            v: 0.0,
            material: 0,
        }
    }
}

pub trait RayIntersect {
    /// Intersecta el rayo `origin + t * direction` (con `direction` normalizada)
    /// y devuelve el impacto mas cercano con `t > EPSILON`.
    fn ray_intersect(&self, origin: &Vec3, direction: &Vec3) -> Intersect;
}
