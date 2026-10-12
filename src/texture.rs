//! Carga y muestreo de texturas en CPU.
//!
//! Basado en el `TextureManager` del profesor, con tres cambios:
//!
//! - No guarda la copia en GPU: aqui nadie dibuja con la GPU, solo se leen
//!   pixeles desde los hilos del raytracer.
//! - Se indexa por `TextureId` (un `usize`) en vez de por `String`. El color se
//!   pide millones de veces por frame; un acceso a `Vec` es inmediato mientras
//!   que hashear una ruta en el bucle interno se nota.
//! - El muestreo recibe coordenadas UV normalizadas y repite la textura
//!   (wrap), que es lo que hace falta para mapear caras de cubos.

// El modulo esta completo y probado, pero todavia no lo llama nadie: se conecta
// en `scene::build` cuando esten los PNG del diorama. Sin esto el compilador
// avisa de codigo muerto en cada compilacion y el aviso util se pierde entre el
// ruido.
#![allow(dead_code)]

use crate::vector::Vec3;
use std::collections::HashMap;

/// Indice de una textura dentro del `TextureManager`.
pub type TextureId = usize;

/// Una textura ya decodificada a RGB en punto flotante.
pub struct CpuTexture {
    pub width: usize,
    pub height: usize,
    /// RGB normalizado a [0, 1], fila por fila desde arriba.
    pub pixels: Vec<Vec3>,
}

impl CpuTexture {
    /// Lee el texel entero (x, y) repitiendo la textura fuera de rango.
    #[inline]
    fn texel(&self, x: i32, y: i32) -> Vec3 {
        let x = x.rem_euclid(self.width as i32) as usize;
        let y = y.rem_euclid(self.height as i32) as usize;
        self.pixels[y * self.width + x]
    }

    /// Muestreo nearest-neighbor. Es intencional: con texturas de 16x16 estilo
    /// Minecraft el filtrado bilineal las volveria una mancha borrosa; el
    /// escalonado duro es justo el look que se busca.
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> Vec3 {
        let x = (u * self.width as f32).floor() as i32;
        let y = (v * self.height as f32).floor() as i32;
        self.texel(x, y)
    }

    /// Muestreo bilineal sin repetir: en el borde se queda con el ultimo texel.
    ///
    /// Solo lo usa el skybox. Ahi el pixelado si estorba: el cielo cubre media
    /// pantalla y cada texel se estira sobre muchos pixeles. Repetir la textura
    /// tampoco sirve, porque el texel de enfrente pertenece al otro lado de la
    /// cara y en las aristas del cubo apareceria una linea de otro color.
    #[inline]
    pub fn sample_bilinear(&self, u: f32, v: f32) -> Vec3 {
        let max_x = (self.width - 1) as f32;
        let max_y = (self.height - 1) as f32;
        let x = (u * self.width as f32 - 0.5).clamp(0.0, max_x);
        let y = (v * self.height as f32 - 0.5).clamp(0.0, max_y);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i32, y0 as i32);
        let x1 = (x0 + 1).min(max_x as i32);
        let y1 = (y0 + 1).min(max_y as i32);
        let top = self.texel(x0, y0) * (1.0 - fx) + self.texel(x1, y0) * fx;
        let bottom = self.texel(x0, y1) * (1.0 - fx) + self.texel(x1, y1) * fx;
        top * (1.0 - fy) + bottom * fy
    }
}

/// Coleccion de texturas cargadas, compartida de solo lectura entre los hilos.
#[derive(Default)]
pub struct TextureManager {
    textures: Vec<CpuTexture>,
    by_path: HashMap<String, TextureId>,
}

impl TextureManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Carga un PNG desde disco y devuelve su id. Si ya estaba cargado
    /// devuelve el id existente en vez de volver a leer el archivo.
    pub fn load(&mut self, path: &str) -> Result<TextureId, String> {
        if let Some(&id) = self.by_path.get(path) {
            return Ok(id);
        }

        let image = raylib::prelude::Image::load_image(path)
            .map_err(|e| format!("no se pudo cargar la textura {path}: {e}"))?;

        let (width, height) = (image.width as usize, image.height as usize);
        // `get_image_data` normaliza el formato interno de raylib a RGBA8.
        let pixels = image
            .get_image_data()
            .iter()
            .map(|c| {
                Vec3::new(
                    srgb_to_linear(c.r as f32 / 255.0),
                    srgb_to_linear(c.g as f32 / 255.0),
                    srgb_to_linear(c.b as f32 / 255.0),
                )
            })
            .collect();

        let id = self.textures.len();
        self.textures.push(CpuTexture {
            width,
            height,
            pixels,
        });
        self.by_path.insert(path.to_string(), id);
        Ok(id)
    }

    /// Carga una textura sin convertir de sRGB a lineal. Los normal maps no son
    /// colores sino vectores codificados, aplicarles gamma los deformaria.
    pub fn load_raw(&mut self, path: &str) -> Result<TextureId, String> {
        let key = format!("raw:{path}");
        if let Some(&id) = self.by_path.get(&key) {
            return Ok(id);
        }

        let image = raylib::prelude::Image::load_image(path)
            .map_err(|e| format!("no se pudo cargar la textura {path}: {e}"))?;

        let (width, height) = (image.width as usize, image.height as usize);
        let pixels = image
            .get_image_data()
            .iter()
            .map(|c| {
                Vec3::new(
                    c.r as f32 / 255.0,
                    c.g as f32 / 255.0,
                    c.b as f32 / 255.0,
                )
            })
            .collect();

        let id = self.textures.len();
        self.textures.push(CpuTexture {
            width,
            height,
            pixels,
        });
        self.by_path.insert(key, id);
        Ok(id)
    }

    #[inline]
    pub fn sample(&self, id: TextureId, u: f32, v: f32) -> Vec3 {
        self.textures[id].sample(u, v)
    }

    #[inline]
    pub fn sample_bilinear(&self, id: TextureId, u: f32, v: f32) -> Vec3 {
        self.textures[id].sample_bilinear(u, v)
    }

    /// Decodifica un texel de normal map a un vector en espacio tangente.
    /// El formato estandar guarda el vector en [0, 1] por canal, asi que se
    /// remapea a [-1, 1] y se renormaliza.
    #[inline]
    pub fn sample_normal(&self, id: TextureId, u: f32, v: f32) -> Vec3 {
        let c = self.textures[id].sample(u, v);
        crate::vector::normalize(&Vec3::new(
            c.x * 2.0 - 1.0,
            c.y * 2.0 - 1.0,
            c.z * 2.0 - 1.0,
        ))
    }

    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}

/// Pasa un canal de color de sRGB a espacio lineal.
///
/// Las texturas vienen guardadas en sRGB (asi las ve el ojo), pero sumar y
/// multiplicar luz solo tiene sentido en espacio lineal. Se convierte al
/// cargar y se regresa a sRGB al escribir el pixel en `framebuffer::encode_pixel`.
#[inline]
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
