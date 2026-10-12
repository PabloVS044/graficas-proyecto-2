//! Mundo de voxeles: el andamio con el que se construye el diorama.
//!
//! Escribir el diorama como una lista de `Cube` directamente es incomodo y
//! desperdicia geometria. En su lugar se llena una rejilla de voxeles (una
//! celda entera = un bloque, como en Minecraft) y al final se convierte a
//! cubos. Eso da dos cosas gratis:
//!
//! - **Poner un bloque encima de otro lo reemplaza**, que es justo lo que uno
//!   quiere al esculpir: primero el terreno, luego se le excava el estanque
//!   encima sin tener que buscar y borrar cubos a mano.
//! - **Se descartan los bloques enterrados**. Un bloque rodeado por los seis
//!   lados de bloques opacos no se ve nunca, pero si se emitiera ocuparia
//!   celdas de la rejilla de aceleracion y se probaria contra rayos que jamas
//!   lo van a alcanzar. Quitarlos no cambia ni un pixel de la imagen.

use crate::cube::Cube;
use crate::material::{Material, MaterialId};
use crate::vector::Vec3;
use std::collections::HashMap;

pub struct VoxelWorld {
    blocks: HashMap<(i32, i32, i32), MaterialId>,
    /// Altura por debajo de la cual se considera que todo es roca maciza. Sirve
    /// para que la cara inferior de la isla, que la camara nunca ve, tambien se
    /// descarte.
    floor_y: i32,
}

impl VoxelWorld {
    pub fn new(floor_y: i32) -> Self {
        VoxelWorld {
            blocks: HashMap::new(),
            floor_y,
        }
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, material: MaterialId) {
        self.blocks.insert((x, y, z), material);
    }

    /// Quita un bloque. Se usa para excavar el estanque y el lago de lava.
    pub fn clear(&mut self, x: i32, y: i32, z: i32) {
        self.blocks.remove(&(x, y, z));
    }

    /// Llena una caja de bloques, con los dos extremos incluidos.
    pub fn fill(
        &mut self,
        xs: (i32, i32),
        ys: (i32, i32),
        zs: (i32, i32),
        material: MaterialId,
    ) {
        for x in xs.0..=xs.1 {
            for y in ys.0..=ys.1 {
                for z in zs.0..=zs.1 {
                    self.set(x, y, z, material);
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Convierte el mundo a la lista de cubos que consume el raytracer,
    /// descartando los que no se pueden ver.
    ///
    /// Devuelve `(cubos, descartados)`.
    pub fn into_cubes(self, materials: &[Material]) -> (Vec<Cube>, usize) {
        // Un bloque tapa a su vecino solo si es opaco: a traves del agua y del
        // cristal si se ve lo de atras, asi que esos no cuentan como tapadera.
        let is_opaque = |id: MaterialId| {
            let m = &materials[id];
            !m.is_transparent()
        };

        let hidden = |x: i32, y: i32, z: i32| {
            // Un bloque por debajo del suelo solo puede estar ahi a proposito,
            // colgando de la isla (la cascada). La regla de "debajo todo es
            // roca" lo daria por enterrado, asi que esos nunca se descartan.
            if y < self.floor_y {
                return false;
            }
            const NEIGHBOURS: [(i32, i32, i32); 6] = [
                (1, 0, 0),
                (-1, 0, 0),
                (0, 1, 0),
                (0, -1, 0),
                (0, 0, 1),
                (0, 0, -1),
            ];
            NEIGHBOURS.iter().all(|(dx, dy, dz)| {
                let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                // Todo lo que este por debajo del suelo cuenta como roca: la
                // camara no puede pasar por debajo del diorama.
                if ny < self.floor_y {
                    return true;
                }
                self.blocks
                    .get(&(nx, ny, nz))
                    .is_some_and(|&id| is_opaque(id))
            })
        };

        // Se ordenan las celdas antes de emitirlas para que la escena sea
        // siempre identica entre ejecuciones: un `HashMap` no garantiza orden,
        // y sin esto la rejilla de aceleracion cambiaria de contenido en cada
        // arranque y las mediciones no serian comparables.
        let mut keys: Vec<(i32, i32, i32)> = self.blocks.keys().copied().collect();
        keys.sort_unstable();

        let mut cubes = Vec::with_capacity(keys.len());
        let mut dropped = 0usize;

        for (x, y, z) in keys {
            if hidden(x, y, z) {
                dropped += 1;
                continue;
            }
            let material = self.blocks[&(x, y, z)];
            cubes.push(Cube::new(
                Vec3::new(x as f32, y as f32, z as f32),
                Vec3::new(1.0, 1.0, 1.0),
                material,
            ));
        }

        (cubes, dropped)
    }
}
