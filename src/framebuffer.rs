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

/// Convierte un color lineal en punto flotante a un pixel RGBA de 8 bits.
///
/// Dos pasos, y ninguno es cosmetico:
///
/// - **Tone mapping de Reinhard**, `c / (1 + c)`. El calculo de luz no esta
///   acotado a 1: basta con que dos lamparas se solapen para pasarse, y los
///   materiales emisivos que vienen despues lo van a hacer siempre. Recortando
///   en 1, todo lo brillante se aplana a blanco puro y se pierde el detalle.
///   Reinhard comprime el rango alto de forma continua, asi que un valor de 5 y
///   uno de 20 siguen siendo distinguibles.
///
/// - **Correccion gamma**, exponente 1/2.2. Sumar y multiplicar luz solo tiene
///   sentido en espacio lineal, pero la pantalla interpreta los valores como
///   sRGB. Sin este paso la imagen se ve mucho mas oscura de lo que deberia,
///   sobre todo en los medios tonos.
#[inline]
pub fn encode_pixel(color: nalgebra_glm::Vec3) -> [u8; 4] {
    let channel = |c: f32| {
        let c = c.max(0.0);
        let mapped = c / (1.0 + c);
        (mapped.powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8
    };
    [
        channel(color.x),
        channel(color.y),
        channel(color.z),
        255,
    ]
}
