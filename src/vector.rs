//! Vector de tres componentes y las operaciones que necesita el raytracer.
//!
//! Escrito a mano a proposito: el proyecto no puede apoyarse en librerias
//! externas al lenguaje, asi que aqui no se usa nada fuera de `std`. Son las
//! seis operaciones que el trazado necesita de verdad, ni una mas.

use std::ops::{Add, AddAssign, Div, Index, IndexMut, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    pub const fn zeros() -> Self {
        Vec3::new(0.0, 0.0, 0.0)
    }

    /// Producto componente a componente.
    ///
    /// No es una operacion vectorial estandar, pero en el sombreado aparece
    /// todo el rato: multiplicar el color de una superficie por el color de la
    /// luz es exactamente esto, canal por canal.
    #[inline]
    pub fn mul_elementwise(self, other: Vec3) -> Vec3 {
        Vec3::new(self.x * other.x, self.y * other.y, self.z * other.z)
    }

    /// Componente por indice, devuelta **por valor**: 0 = X, 1 = Y, 2 = Z.
    ///
    /// Existe ademas del operador `[]` por una razon de rendimiento medida. El
    /// operador tiene que devolver una referencia, y eso obliga al vector a
    /// vivir en memoria en lugar de en registros. En los bucles calientes (el
    /// metodo de slabs y el recorrido de la rejilla, que se ejecutan millones de
    /// veces por cuadro) eso costaba un 30% del tiempo de render.
    #[inline]
    pub fn get(self, i: usize) -> f32 {
        match i {
            0 => self.x,
            1 => self.y,
            _ => self.z,
        }
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }
}

/// Producto punto: cuanto apuntan dos vectores en la misma direccion.
///
/// Es la operacion central del sombreado. Para vectores unitarios vale el
/// coseno del angulo que forman, que es justo lo que pide la ley de Lambert.
#[inline]
pub fn dot(a: &Vec3, b: &Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// Producto cruz: un vector perpendicular a los dos de entrada.
/// Se usa para construir la base de la camara.
#[inline]
pub fn cross(a: &Vec3, b: &Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

#[inline]
pub fn length(v: &Vec3) -> f32 {
    v.length_squared().sqrt()
}

/// Distancia al cuadrado. Se evita la raiz porque para comparar distancias, o
/// para la caida cuadratica de una luz, el cuadrado ya sirve.
#[inline]
pub fn distance2(a: &Vec3, b: &Vec3) -> f32 {
    (*a - *b).length_squared()
}

/// Devuelve el vector con longitud 1.
///
/// Si el vector es nulo se devuelve tal cual en vez de dividir entre cero, que
/// daria NaN y lo propagaria a toda la imagen.
#[inline]
pub fn normalize(v: &Vec3) -> Vec3 {
    let len = length(v);
    if len == 0.0 { *v } else { *v / len }
}

/// Refleja `incident` respecto a la normal `n`, que se supone unitaria.
///
/// La componente del rayo paralela a la normal se invierte y la paralela a la
/// superficie se conserva, que es lo que hace un espejo.
#[inline]
pub fn reflect_vec(incident: &Vec3, n: &Vec3) -> Vec3 {
    *incident - *n * (2.0 * dot(incident, n))
}

// ---------------------------------------------------------------------------
// Operadores
// ---------------------------------------------------------------------------

impl Add for Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, k: f32) -> Vec3 {
        Vec3::new(self.x * k, self.y * k, self.z * k)
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, k: f32) -> Vec3 {
        Vec3::new(self.x / k, self.y / k, self.z / k)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    #[inline]
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

impl AddAssign for Vec3 {
    #[inline]
    fn add_assign(&mut self, o: Vec3) {
        self.x += o.x;
        self.y += o.y;
        self.z += o.z;
    }
}

// Referencias: el raytracer pasa vectores prestados constantemente y sin estas
// implementaciones habria que sembrar el codigo de asteriscos.
impl Add<&Vec3> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, o: &Vec3) -> Vec3 {
        *self + *o
    }
}

impl Sub<&Vec3> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, o: &Vec3) -> Vec3 {
        *self - *o
    }
}

impl Mul<f32> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, k: f32) -> Vec3 {
        *self * k
    }
}

impl Div<f32> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, k: f32) -> Vec3 {
        *self / k
    }
}

impl Add<Vec3> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, o: Vec3) -> Vec3 {
        *self + o
    }
}

impl Sub<Vec3> for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, o: Vec3) -> Vec3 {
        *self - o
    }
}

impl Add<&Vec3> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, o: &Vec3) -> Vec3 {
        self + *o
    }
}

impl Sub<&Vec3> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, o: &Vec3) -> Vec3 {
        self - *o
    }
}

impl Neg for &Vec3 {
    type Output = Vec3;
    #[inline]
    fn neg(self) -> Vec3 {
        -*self
    }
}

/// Acceso por indice: 0 = X, 1 = Y, 2 = Z.
///
/// Lo necesita el metodo de slabs de `Cube` y el recorrido de la rejilla, que
/// tratan los tres ejes en un bucle en vez de repetir el codigo tres veces.
impl Index<usize> for Vec3 {
    type Output = f32;
    #[inline]
    fn index(&self, i: usize) -> &f32 {
        match i {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("indice de Vec3 fuera de rango: {i}"),
        }
    }
}

impl IndexMut<usize> for Vec3 {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut f32 {
        match i {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            _ => panic!("indice de Vec3 fuera de rango: {i}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_da_longitud_uno() {
        let v = normalize(&Vec3::new(3.0, -4.0, 12.0));
        assert!((length(&v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalize_no_produce_nan_con_vector_nulo() {
        let v = normalize(&Vec3::zeros());
        assert!(v.x.is_finite() && v.y.is_finite() && v.z.is_finite());
    }

    /// Un rayo que baja en diagonal debe rebotar hacia arriba conservando la
    /// componente horizontal.
    #[test]
    fn reflejo_invierte_solo_la_componente_normal() {
        let incidente = normalize(&Vec3::new(1.0, -1.0, 0.0));
        let n = Vec3::new(0.0, 1.0, 0.0);
        let r = reflect_vec(&incidente, &n);
        assert!((r.x - incidente.x).abs() < 1e-6);
        assert!((r.y + incidente.y).abs() < 1e-6);
    }

    #[test]
    fn cross_es_perpendicular_a_sus_operandos() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(-4.0, 5.0, 6.0);
        let c = cross(&a, &b);
        assert!(dot(&c, &a).abs() < 1e-4);
        assert!(dot(&c, &b).abs() < 1e-4);
    }

    #[test]
    fn indexar_recorre_los_tres_ejes() {
        let mut v = Vec3::zeros();
        v[0] = 1.0;
        v[1] = 2.0;
        v[2] = 3.0;
        assert_eq!((v[0], v[1], v[2]), (v.x, v.y, v.z));
    }
}
