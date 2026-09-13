//! Resultado de intersectar un rayo con un objeto y el rasgo que lo produce.

use crate::material::MaterialId;
use crate::vector::Vec3;

/// Desplazamiento minimo que se aplica al origen de los rayos secundarios.
///
/// Sin esto, un rayo de sombra o de reflexion que nace exactamente sobre la
/// superficie vuelve a chocar con ella por error de redondeo y la imagen se
/// llena de puntos negros ("shadow acne").
pub const EPSILON: f32 = 1e-3;

/// Todo lo que el sombreador necesita saber del punto donde pego el rayo.
#[derive(Debug, Clone, Copy)]
pub struct Intersect {
    pub is_intersecting: bool,

    /// Distancia desde el origen del rayo. Resuelve que objeto queda enfrente.
    pub distance: f32,

    /// Punto de impacto en coordenadas de mundo.
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
