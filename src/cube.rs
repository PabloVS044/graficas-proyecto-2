//! Cubo alineado a los ejes (AABB): la unica primitiva del diorama.
//!
//! Todo va a estar hecho de cubos, como en Minecraft. Eso simplifica la
//! interseccion, que se resuelve con el metodo de slabs sin resolver ninguna
//! ecuacion cuadratica.

use crate::material::MaterialId;
use crate::ray_intersect::{EPSILON, Intersect, RayIntersect};
use nalgebra_glm::Vec3;

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

    /// Cubo unitario cuya esquina minima es la celda entera `(x, y, z)`.
    pub fn unit(x: i32, y: i32, z: i32, material: MaterialId) -> Self {
        Cube::new(
            Vec3::new(x as f32, y as f32, z as f32),
            Vec3::new(1.0, 1.0, 1.0),
            material,
        )
    }
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, origin: &Vec3, direction: &Vec3) -> Intersect {
        // Metodo de slabs: el cubo es la interseccion de tres pares de planos
        // paralelos. Para cada eje se calcula en que intervalo del parametro t
        // el rayo esta dentro de ese par, y el rayo toca el cubo solo donde los
        // tres intervalos se traslapan.
        let mut t_near = f32::NEG_INFINITY;
        let mut t_far = f32::INFINITY;
        let mut near_axis = 0usize;
        let mut far_axis = 0usize;

        for axis in 0..3 {
            let o = origin[axis];
            let d = direction[axis];
            let (lo, hi) = (self.min[axis], self.max[axis]);

            if d.abs() < 1e-8 {
                // Rayo paralelo a este par de planos: o siempre esta dentro del
                // slab, o nunca toca el cubo.
                if o < lo || o > hi {
                    return Intersect::empty();
                }
                continue;
            }

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
        // detras del origen, el rayo nacio dentro del cubo y el impacto valido
        // es la salida.
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
            direction[axis] < 0.0
        } else {
            direction[axis] > 0.0
        };

        let mut normal = Vec3::zeros();
        normal[axis] = if positive { 1.0 } else { -1.0 };

        Intersect {
            is_intersecting: true,
            distance: t,
            point,
            normal,
            material: self.material,
        }
    }
}
