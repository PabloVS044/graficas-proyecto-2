//! Rejilla uniforme para acelerar las intersecciones.
//!
//! # El problema
//!
//! `Scene::hit` en su version ingenua prueba el rayo contra **todos** los cubos.
//! Con 1400 cubos y 2.4 millones de rayos primarios eso son 3400 millones de
//! pruebas por cuadro, y todavia faltan los rayos de sombra, reflexion y
//! refraccion. Medido: ~4.5 s por cuadro. Inservible para algo interactivo.
//!
//! # La solucion
//!
//! El diorama es un mundo de cubos sobre una rejilla entera, asi que la
//! estructura que mejor le queda no es un BVH sino una **rejilla uniforme**:
//! el espacio se parte en celdas del tamaño de un cubo y cada celda guarda que
//! cubos la tocan. Un rayo entonces no recorre la escena, recorre **la linea de
//! celdas que atraviesa**, y solo prueba los cubos de esas celdas.
//!
//! El recorrido usa el algoritmo de Amanatides y Woo (*A Fast Voxel Traversal
//! Algorithm*, 1987): el mismo DDA de Bresenham pero en 3D y con distancias
//! reales. Visita las celdas **en orden de cercania**, lo que permite cortar en
//! cuanto se encuentra un impacto: si el mas cercano de esta celda cae dentro
//! de la celda, ninguna celda posterior puede tener uno mejor.
//!
//! # Por que CSR y no `Vec<Vec<u32>>`
//!
//! Lo obvio seria un vector de vectores, uno por celda. Pero eso son miles de
//! reservas dispersas por el heap, y el recorrido salta de celda en celda: cada
//! salto seria un fallo de cache. En su lugar se guarda en formato CSR
//! (*compressed sparse row*): **un solo** arreglo con todos los indices de cubo
//! pegados, y otro con donde empieza cada celda. Los cubos de celdas vecinas
//! quedan contiguos en memoria, que es justo el orden en que se van a leer.

use crate::cube::Cube;
use nalgebra_glm::Vec3;

/// Margen que se le resta al extremo maximo de un cubo al registrarlo, para que
/// uno que termina justo en el borde de una celda no se registre tambien en la
/// siguiente.
const PADDING: f32 = 1e-3;

pub struct Grid {
    /// Esquina minima de la caja que envuelve toda la escena.
    min: Vec3,
    max: Vec3,
    /// Lado de una celda.
    cell_size: f32,
    /// Numero de celdas en cada eje.
    dims: [usize; 3],

    /// Indices de cubo de todas las celdas, pegados uno tras otro.
    items: Vec<u32>,
    /// `offsets[c]..offsets[c + 1]` es el rango de `items` de la celda `c`.
    offsets: Vec<u32>,
}

impl Grid {
    /// Construye la rejilla a partir de los cubos de la escena.
    ///
    /// El tamaño de celda es 1.0 porque el diorama esta hecho de cubos
    /// unitarios: celdas mas chicas solo agregarian celdas vacias que recorrer,
    /// y mas grandes meterian demasiados cubos en cada una.
    pub fn build(cubes: &[Cube]) -> Self {
        if cubes.is_empty() {
            return Grid {
                min: Vec3::zeros(),
                max: Vec3::new(1.0, 1.0, 1.0),
                cell_size: 1.0,
                dims: [1, 1, 1],
                items: Vec::new(),
                offsets: vec![0, 0],
            };
        }

        // Caja que envuelve la escena.
        let mut min = cubes[0].min;
        let mut max = cubes[0].max;
        for cube in cubes {
            for axis in 0..3 {
                min[axis] = min[axis].min(cube.min[axis]);
                max[axis] = max[axis].max(cube.max[axis]);
            }
        }
        let cell_size = 1.0f32;

        // La esquina de la rejilla se alinea a un multiplo del tamaño de celda.
        // Es importante: el diorama esta hecho de cubos unitarios sobre
        // coordenadas enteras, asi que con la rejilla alineada cada cubo cae
        // dentro de exactamente una celda. Si el origen quedara desplazado una
        // fraccion, cada cubo se registraria en las ocho celdas que lo rodean y
        // el recorrido probaria ocho veces mas cubos de los necesarios.
        min = Vec3::new(
            (min.x / cell_size).floor() * cell_size,
            (min.y / cell_size).floor() * cell_size,
            (min.z / cell_size).floor() * cell_size,
        );
        max = Vec3::new(
            (max.x / cell_size).ceil() * cell_size,
            (max.y / cell_size).ceil() * cell_size,
            (max.z / cell_size).ceil() * cell_size,
        );
        let extent = max - min;
        let dims = [
            ((extent.x / cell_size).ceil() as usize).max(1),
            ((extent.y / cell_size).ceil() as usize).max(1),
            ((extent.z / cell_size).ceil() as usize).max(1),
        ];
        let cell_count = dims[0] * dims[1] * dims[2];

        // Construccion del CSR en dos pasadas, sin reservas intermedias:
        //   1. contar cuantos cubos cae en cada celda,
        //   2. suma prefija para saber donde empieza cada una,
        //   3. segunda pasada que los coloca en su sitio.
        let cell_range = |cube: &Cube| {
            let lo = [
                index_of(cube.min.x, min.x, cell_size, dims[0]),
                index_of(cube.min.y, min.y, cell_size, dims[1]),
                index_of(cube.min.z, min.z, cell_size, dims[2]),
            ];
            // Se resta un epsilon al maximo para que un cubo que termina justo
            // en el borde de una celda no se registre tambien en la siguiente.
            let hi = [
                index_of(cube.max.x - PADDING, min.x, cell_size, dims[0]),
                index_of(cube.max.y - PADDING, min.y, cell_size, dims[1]),
                index_of(cube.max.z - PADDING, min.z, cell_size, dims[2]),
            ];
            (lo, hi)
        };

        let mut counts = vec![0u32; cell_count + 1];
        for cube in cubes {
            let (lo, hi) = cell_range(cube);
            for z in lo[2]..=hi[2] {
                for y in lo[1]..=hi[1] {
                    for x in lo[0]..=hi[0] {
                        counts[flatten(x, y, z, dims) + 1] += 1;
                    }
                }
            }
        }

        let mut offsets = counts;
        for c in 1..offsets.len() {
            offsets[c] += offsets[c - 1];
        }

        let total = offsets[cell_count] as usize;
        let mut items = vec![0u32; total];
        let mut cursor = offsets.clone();
        for (index, cube) in cubes.iter().enumerate() {
            let (lo, hi) = cell_range(cube);
            for z in lo[2]..=hi[2] {
                for y in lo[1]..=hi[1] {
                    for x in lo[0]..=hi[0] {
                        let cell = flatten(x, y, z, dims);
                        items[cursor[cell] as usize] = index as u32;
                        cursor[cell] += 1;
                    }
                }
            }
        }

        Grid {
            min,
            max,
            cell_size,
            dims,
            items,
            offsets,
        }
    }

    /// Cubos registrados en una celda.
    #[inline]
    fn cell_items(&self, x: usize, y: usize, z: usize) -> &[u32] {
        let cell = flatten(x, y, z, self.dims);
        let start = self.offsets[cell] as usize;
        let end = self.offsets[cell + 1] as usize;
        &self.items[start..end]
    }

    /// Intervalo `[t_enter, t_exit]` en el que el rayo esta dentro de la caja
    /// de la escena, o `None` si no la toca. Es el mismo metodo de slabs de
    /// `Cube`, aplicado a la caja completa.
    #[inline]
    fn clip(&self, origin: &Vec3, direction: &Vec3) -> Option<(f32, f32)> {
        let mut t_enter = 0.0f32;
        let mut t_exit = f32::INFINITY;

        for axis in 0..3 {
            let o = origin[axis];
            let d = direction[axis];
            if d.abs() < 1e-8 {
                if o < self.min[axis] || o > self.max[axis] {
                    return None;
                }
                continue;
            }
            let inv = 1.0 / d;
            let mut t1 = (self.min[axis] - o) * inv;
            let mut t2 = (self.max[axis] - o) * inv;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            t_enter = t_enter.max(t1);
            t_exit = t_exit.min(t2);
            if t_enter > t_exit {
                return None;
            }
        }

        Some((t_enter, t_exit))
    }

    /// Recorre las celdas que atraviesa el rayo, de la mas cercana a la mas
    /// lejana, y le pasa a `visit` los cubos de cada una.
    ///
    /// `visit` devuelve la distancia del mejor impacto encontrado hasta ahora
    /// (o infinito). El recorrido se detiene en cuanto ese impacto queda dentro
    /// de la celda ya visitada, porque entonces ninguna celda posterior puede
    /// contener uno mas cercano.
    pub fn traverse<F>(&self, origin: &Vec3, direction: &Vec3, max_distance: f32, mut visit: F)
    where
        F: FnMut(&[u32]) -> f32,
    {
        let Some((t_enter, t_exit)) = self.clip(origin, direction) else {
            return;
        };
        if t_enter > max_distance {
            return;
        }

        // Celda en la que el rayo entra a la rejilla.
        let entry = origin + direction * t_enter;
        let mut cell = [
            index_of(entry.x, self.min.x, self.cell_size, self.dims[0]),
            index_of(entry.y, self.min.y, self.cell_size, self.dims[1]),
            index_of(entry.z, self.min.z, self.cell_size, self.dims[2]),
        ];

        let mut step = [0isize; 3];
        // `t_max[i]`: distancia a la que el rayo cruza al siguiente plano de
        // celda en el eje i. `t_delta[i]`: cuanto avanza entre plano y plano.
        let mut t_max = [f32::INFINITY; 3];
        let mut t_delta = [f32::INFINITY; 3];

        for axis in 0..3 {
            let d = direction[axis];
            if d.abs() < 1e-8 {
                continue;
            }
            let inv = 1.0 / d.abs();
            t_delta[axis] = self.cell_size * inv;

            if d > 0.0 {
                step[axis] = 1;
                let boundary = self.min[axis] + (cell[axis] + 1) as f32 * self.cell_size;
                t_max[axis] = t_enter + (boundary - entry[axis]) * inv;
            } else {
                step[axis] = -1;
                let boundary = self.min[axis] + cell[axis] as f32 * self.cell_size;
                t_max[axis] = t_enter + (entry[axis] - boundary) * inv;
            }
        }

        let limit = t_exit.min(max_distance);

        loop {
            let items = self.cell_items(cell[0], cell[1], cell[2]);
            if !items.is_empty() {
                let best = visit(items);

                // El eje con el `t_max` mas chico marca donde termina la celda
                // actual. Si el impacto cae antes de eso, ya es el definitivo.
                let cell_exit = t_max[0].min(t_max[1]).min(t_max[2]);
                if best <= cell_exit {
                    return;
                }
            }

            // Avanzar a la siguiente celda: se cruza primero el plano mas
            // cercano de los tres.
            let axis = if t_max[0] < t_max[1] {
                if t_max[0] < t_max[2] { 0 } else { 2 }
            } else if t_max[1] < t_max[2] {
                1
            } else {
                2
            };

            if t_max[axis] > limit {
                return;
            }

            let next = cell[axis] as isize + step[axis];
            if next < 0 || next >= self.dims[axis] as isize {
                return; // el rayo salio de la rejilla
            }
            cell[axis] = next as usize;
            t_max[axis] += t_delta[axis];
        }
    }

    /// Numero de celdas, para reportar en el HUD y el README.
    pub fn cell_count(&self) -> usize {
        self.dims[0] * self.dims[1] * self.dims[2]
    }

    pub fn dims(&self) -> [usize; 3] {
        self.dims
    }

    /// Promedio de cubos por celda ocupada: dice si el tamaño de celda es el
    /// adecuado. Muy alto = celdas demasiado grandes; cercano a 1 = bien.
    pub fn average_occupancy(&self) -> f32 {
        let occupied = (0..self.cell_count())
            .filter(|&c| self.offsets[c + 1] > self.offsets[c])
            .count();
        if occupied == 0 {
            0.0
        } else {
            self.items.len() as f32 / occupied as f32
        }
    }
}

/// Coordenada de mundo -> indice de celda, recortado al rango valido.
#[inline]
fn index_of(coord: f32, origin: f32, cell_size: f32, dim: usize) -> usize {
    let i = ((coord - origin) / cell_size).floor();
    if i < 0.0 {
        0
    } else if i >= dim as f32 {
        dim - 1
    } else {
        i as usize
    }
}

/// Indice 3D -> indice lineal, con X variando mas rapido.
#[inline]
fn flatten(x: usize, y: usize, z: usize, dims: [usize; 3]) -> usize {
    (z * dims[1] + y) * dims[0] + x
}
