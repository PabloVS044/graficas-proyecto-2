//! Framebuffer por software.
//!
//! El buffer es un `Vec<u8>` plano en RGBA en vez de una `Image` de raylib.
//! `update_texture` de raylib sube exactamente este formato a la GPU, asi que
//! mostrar el resultado en pantalla es una sola llamada sin conversiones de por
//! medio.

/// Buffer de color RGBA8 con su resolucion.
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    /// `width * height * 4` bytes en orden R, G, B, A.
    pub pixels: Vec<u8>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            pixels: vec![0; width * height * 4],
        }
    }

    /// Bytes que ocupa una fila de la imagen.
    pub fn row_bytes(&self) -> usize {
        self.width * 4
    }

    /// Escribe un pixel ya codificado.
    pub fn set(&mut self, x: usize, y: usize, rgba: [u8; 4]) {
        let i = y * self.row_bytes() + x * 4;
        self.pixels[i..i + 4].copy_from_slice(&rgba);
    }

    /// Guarda el buffer como PNG.
    pub fn save_png(&self, path: &str) -> Result<(), String> {
        use raylib::prelude::*;

        let mut image = Image::gen_image_color(self.width as i32, self.height as i32, Color::BLACK);
        for y in 0..self.height {
            for x in 0..self.width {
                let i = (y * self.width + x) * 4;
                image.draw_pixel(
                    x as i32,
                    y as i32,
                    Color::new(
                        self.pixels[i],
                        self.pixels[i + 1],
                        self.pixels[i + 2],
                        self.pixels[i + 3],
                    ),
                );
            }
        }
        image.export_image(path);
        Ok(())
    }
}

/// Convierte un color en punto flotante a un pixel RGBA de 8 bits.
#[inline]
pub fn encode_pixel(color: nalgebra_glm::Vec3) -> [u8; 4] {
    let channel = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    [
        channel(color.x),
        channel(color.y),
        channel(color.z),
        255,
    ]
}
