//! Framebuffer por software.
//!
//! A diferencia del de la tarea anterior, aqui el buffer es un `Vec<u8>` plano
//! en RGBA en vez de una `Image` de raylib. Dos razones:
//!
//! 1. Se puede partir en bandas horizontales con `chunks_mut` y repartirlas
//!    entre hilos: `Image` no se puede compartir entre hilos de forma segura.
//! 2. `update_texture` de raylib sube exactamente este formato a la GPU, asi
//!    que blitear el resultado cada frame es una sola llamada sin conversiones.

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

    /// Guarda el buffer como PNG. Se usa para la captura en alta resolucion
    /// que alimenta el video de la entrega.
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
/// Incluye dos pasos que no estaban en la tarea anterior:
/// - **Tone mapping Reinhard**: `c / (1 + c)` comprime valores mayores a 1 que
///   ahora si aparecen, porque los materiales emisivos y los brillos especulares
///   pueden pasarse del rango [0, 1]. Sin esto todo lo brillante se recorta a
///   blanco plano y se pierde el detalle.
/// - **Correccion gamma** (exponente 1/2.2): el calculo de luz se hace en
///   espacio lineal, pero la pantalla espera sRGB. Sin esto la escena se ve
///   mucho mas oscura de lo que deberia.
#[inline]
pub fn encode_pixel(color: nalgebra_glm::Vec3) -> [u8; 4] {
    let tonemap = |c: f32| {
        let c = c.max(0.0);
        let mapped = c / (1.0 + c);
        mapped.powf(1.0 / 2.2)
    };
    [
        (tonemap(color.x) * 255.0).round().clamp(0.0, 255.0) as u8,
        (tonemap(color.y) * 255.0).round().clamp(0.0, 255.0) as u8,
        (tonemap(color.z) * 255.0).round().clamp(0.0, 255.0) as u8,
        255,
    ]
}
