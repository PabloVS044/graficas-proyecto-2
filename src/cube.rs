//! Cubo alineado a los ejes (AABB): la unica primitiva del diorama.
//!
//! Todo el diorama son cubos, como en Minecraft. Eso simplifica dos cosas:
//! la interseccion se resuelve con el metodo de slabs (muy barato) y las
//! coordenadas de textura salen directo de la cara golpeada, sin necesidad de
//! parametrizar una superficie curva.

use crate::material::MaterialId;
use crate::ray_intersect::{EPSILON, Intersect, RayIntersect};
use crate::vector::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Cube {
    pub min: Vec3,
    pub max: Vec3,
    pub material: MaterialId,
}

impl Cube {
    /// Cubo definido por su esquina minima y su tamaño en cada eje.
    pub fn new(min: Vec3, size: Vec3, material: MaterialId) -> Self {
        Cube {
            min,
            max: min + size,
            material,
        }
    }

    /// Calcula UV y base tangente para un punto sobre la cara `axis`
    /// (0 = X, 1 = Y, 2 = Z) con signo `positive`.
    ///
    /// Las orientaciones estan elegidas para que las texturas se vean derechas
    /// vistas desde afuera: `u` crece hacia la derecha y `v` crece hacia abajo,
    /// que es como se guardan los pixeles en un PNG.
    fn face_uv(&self, point: &Vec3, axis: usize, positive: bool) -> (f32, f32, Vec3, Vec3) {
        let size = self.max - self.min;
        let local = point - self.min;

        // Fraccion del punto dentro del cubo en cada eje, en [0, 1].
        let fx = if size.x != 0.0 { local.x / size.x } else { 0.0 };
        let fy = if size.y != 0.0 { local.y / size.y } else { 0.0 };
        let fz = if size.z != 0.0 { local.z / size.z } else { 0.0 };

        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let z = Vec3::new(0.0, 0.0, 1.0);

        // Para cada cara: (u, v, direccion de +u, direccion de -v = "arriba").
        match (axis, positive) {
            // Cara +X: se ve desde la derecha, Z crece hacia la izquierda visual.
            (0, true) => (fz, 1.0 - fy, z, y),
            (0, false) => (1.0 - fz, 1.0 - fy, -z, y),
            // Cara +Y: la de arriba. Se mira desde el cielo, asi que "arriba"
            // en la textura corresponde a -Z.
            (1, true) => (fx, fz, x, -z),
            (1, false) => (fx, 1.0 - fz, x, z),
            // Cara +Z: la que mira al frente en la orientacion por defecto.
            (2, true) => (fx, 1.0 - fy, x, y),
            _ => (1.0 - fx, 1.0 - fy, -x, y),
        }
    }
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, origin: &Vec3, direction: &Vec3) -> Intersect {
        // Metodo de slabs: el cubo es la interseccion de tres pares de planos
        // paralelos. Para cada eje se calcula en que intervalo de t el rayo
        // esta dentro de ese par, y el rayo toca el cubo solo donde los tres
        // intervalos se traslapan.
        let mut t_near = f32::NEG_INFINITY;
        let mut t_far = f32::INFINITY;
        let mut near_axis = 0usize;
        let mut far_axis = 0usize;

        for axis in 0..3 {
            let o = origin.get(axis);
            let d = direction.get(axis);
            let (lo, hi) = (self.min.get(axis), self.max.get(axis));

            if d.abs() < 1e-8 {
                // Rayo paralelo a este par de planos: o siempre esta dentro
                // del slab, o nunca toca el cubo.
                if o < lo || o > hi {
                    return Intersect::empty();
                }
                continue;
            }

            // 1/d puede ser infinito; los productos siguen dando el signo
            // correcto, por eso no hace falta un caso especial.
            let inv = 1.0 / d;
            let mut t1 = (lo - o) * inv;
            let mut t2 = (hi - o) * inv;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }

            if t1 > t_near {
                t_near = t1;
                near_axis = axis;
            }
            if t2 < t_far {
                t_far = t2;
                far_axis = axis;
            }
            if t_near > t_far {
                return Intersect::empty();
            }
        }

        // `t_near` es la entrada al cubo y `t_far` la salida. Si `t_near` quedo
        // detras del origen el rayo nacio adentro (pasa con los rayos de
        // refraccion que viajan dentro del cristal) y el impacto valido es la
        // salida.
        let (t, axis, entering) = if t_near > EPSILON {
            (t_near, near_axis, true)
        } else if t_far > EPSILON {
            (t_far, far_axis, false)
        } else {
            return Intersect::empty();
        };

        let point = origin + direction * t;

        // La cara golpeada es la del lado contrario al que apunta el rayo
        // cuando entra, y la del mismo lado cuando sale.
        let positive = if entering {
            direction.get(axis) < 0.0
        } else {
            direction.get(axis) > 0.0
        };

        let mut normal = Vec3::zeros();
        normal[axis] = if positive { 1.0 } else { -1.0 };

        let (u, v, tangent, bitangent) = self.face_uv(&point, axis, positive);

        Intersect {
            is_intersecting: true,
            distance: t,
            point,
            normal,
            tangent,
            bitangent,
            u,
            v,
            material: self.material,
        }
    }
}
