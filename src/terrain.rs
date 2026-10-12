//! Generacion procedural del terreno.
//!
//! Todo escrito a mano, sin librerias: un hash entero para sacar numeros
//! pseudoaleatorios deterministas, ruido de valor encima del hash, y ruido
//! fractal (varias octavas del ruido de valor sumadas) encima del ruido.
//!
//! La semilla cambia el mundo completo. Con la misma semilla el resultado es
//! identico bit a bit, que es lo que permite comparar mediciones entre
//! ejecuciones; con otra semilla salen otras colinas y otros arboles.

pub struct Terrain {
    seed: u32,
}

impl Terrain {
    pub fn new(seed: u32) -> Self {
        Terrain { seed }
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// Numero pseudoaleatorio en [0, 1) para la celda (x, z).
    ///
    /// `salt` separa usos distintos sobre la misma celda: sin el, "hay arbol
    /// aqui" y "que tipo de arbol" saldrian del mismo numero y todos los
    /// arboles posibles serian del mismo tipo.
    #[inline]
    pub fn hash01(&self, x: i32, z: i32, salt: u32) -> f32 {
        // Mezcla entera tipo xorshift-multiply. Determinista y sin estado.
        let mut h = (x as u32).wrapping_mul(0x27D4_EB2D)
            ^ (z as u32).wrapping_mul(0x1656_67B1)
            ^ self.seed.wrapping_mul(0x9E37_79B9)
            ^ salt.wrapping_mul(0x85EB_CA6B);
        h ^= h >> 15;
        h = h.wrapping_mul(0x2C1B_3C6D);
        h ^= h >> 12;
        h = h.wrapping_mul(0x2974_5B7D);
        h ^= h >> 16;
        (h >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Ruido de valor: se hashean las esquinas de una rejilla y se interpola
    /// suavemente entre ellas. Devuelve valores en [0, 1].
    pub fn value_noise(&self, x: f32, z: f32, salt: u32) -> f32 {
        let (x0, z0) = (x.floor(), z.floor());
        let (fx, fz) = (x - x0, z - z0);
        // Curva suave: quita los rombos que deja la interpolacion lineal pura.
        let (ux, uz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));

        let (ix, iz) = (x0 as i32, z0 as i32);
        let c00 = self.hash01(ix, iz, salt);
        let c10 = self.hash01(ix + 1, iz, salt);
        let c01 = self.hash01(ix, iz + 1, salt);
        let c11 = self.hash01(ix + 1, iz + 1, salt);

        let a = c00 + (c10 - c00) * ux;
        let b = c01 + (c11 - c01) * ux;
        a + (b - a) * uz
    }

    /// Ruido fractal: octavas de ruido de valor, cada una al doble de
    /// frecuencia y la mitad de amplitud que la anterior. La primera da la
    /// forma de las colinas y las siguientes les rompen el contorno, que con
    /// una sola octava sale demasiado redondo. Devuelve valores en [0, 1].
    pub fn fbm(&self, x: f32, z: f32, octaves: u32) -> f32 {
        let (mut sum, mut amplitude, mut frequency, mut total) = (0.0, 1.0, 1.0, 0.0);
        for octave in 0..octaves {
            sum += self.value_noise(x * frequency, z * frequency, octave) * amplitude;
            total += amplitude;
            amplitude *= 0.5;
            frequency *= 2.0;
        }
        sum / total
    }
}
