//! Skybox: el cielo como un cubo de seis texturas alrededor de la escena.
//!
//! Un rayo que no choca con nada termina "en el infinito", asi que lo unico que
//! importa es su direccion. Se elige la cara del cubo por el eje dominante de la
//! direccion y se proyecta sobre ella para sacar las coordenadas (u, v). Es la
//! misma convencion que los cubemaps de OpenGL; las imagenes salen de
//! `tools/gen_skybox.py`, que usa las mismas formulas al reves.
//!
//! El sol no esta pintado en las texturas sino que se suma aqui: un PNG de 8
//! bits no puede guardar valores mayores a 1, y un sol que no se pasa de 1
//! despues del tone mapping se ve como un disco gris en vez de deslumbrar.

use crate::texture::{TextureId, TextureManager};
use crate::vector::{Vec3, dot, normalize};

/// Las texturas se guardaron divididas entre esta escala para que el
/// horizonte, que pasa de 1, cupiera en 8 bits. Al leer se deshace.
const SKY_SCALE: f32 = 2.0;

/// Coseno del radio angular del disco del sol (unos 2 grados).
const SUN_COS_OUTER: f32 = 0.99935;
/// Hasta aqui el disco brilla completo; entre los dos cosenos se difumina.
const SUN_COS_INNER: f32 = 0.99960;

pub struct Skybox {
    /// Caras en orden +X, -X, +Y, -Y, +Z, -Z.
    faces: [TextureId; 6],
    /// Direccion hacia el sol, normalizada.
    sun_direction: Vec3,
    /// Radiancia del disco del sol. Mayor a 1 a proposito.
    sun_radiance: Vec3,
}

impl Skybox {
    /// Carga las seis caras desde `dir/{px,nx,py,ny,pz,nz}.png`.
    pub fn load(
        textures: &mut TextureManager,
        dir: &str,
        sun_direction: Vec3,
        sun_radiance: Vec3,
    ) -> Result<Self, String> {
        let mut faces = [0; 6];
        for (slot, name) in faces.iter_mut().zip(["px", "nx", "py", "ny", "pz", "nz"]) {
            *slot = textures.load(&format!("{dir}/{name}.png"))?;
        }
        Ok(Skybox {
            faces,
            sun_direction: normalize(&sun_direction),
            sun_radiance,
        })
    }

    /// Color del cielo en la direccion dada (no hace falta que este
    /// normalizada).
    #[inline]
    pub fn sample(&self, textures: &TextureManager, direction: &Vec3) -> Vec3 {
        let (face, u, v) = Self::project(direction);
        let mut color = textures.sample_bilinear(self.faces[face], u, v) * SKY_SCALE;

        let d = normalize(direction);
        let c = dot(&d, &self.sun_direction);
        if c > SUN_COS_OUTER {
            let t = ((c - SUN_COS_OUTER) / (SUN_COS_INNER - SUN_COS_OUTER)).clamp(0.0, 1.0);
            color += self.sun_radiance * t;
        }
        color
    }

    /// Cara del cubo y coordenadas (u, v) en [0, 1] de una direccion.
    #[inline]
    fn project(d: &Vec3) -> (usize, f32, f32) {
        let (ax, ay, az) = (d.x.abs(), d.y.abs(), d.z.abs());
        let (face, a, b) = if ax >= ay && ax >= az {
            if d.x > 0.0 {
                (0, -d.z / ax, -d.y / ax)
            } else {
                (1, d.z / ax, -d.y / ax)
            }
        } else if ay >= az {
            if d.y > 0.0 {
                (2, d.x / ay, d.z / ay)
            } else {
                (3, d.x / ay, -d.z / ay)
            }
        } else if d.z > 0.0 {
            (4, d.x / az, -d.y / az)
        } else {
            (5, -d.x / az, -d.y / az)
        };
        (face, (a + 1.0) * 0.5, (b + 1.0) * 0.5)
    }
}
