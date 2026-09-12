mod framebuffer;

use framebuffer::{Framebuffer, encode_pixel};
use nalgebra_glm::Vec3;
use raylib::prelude::*;

const WINDOW_WIDTH: i32 = 960;
const WINDOW_HEIGHT: i32 = 640;

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Diorama raytracing - proyecto 2")
        .build();
    rl.set_target_fps(60);

    // Degradado de prueba: sirve para comprobar que el orden de los canales y
    // el de las filas coinciden con lo que espera raylib antes de meter nada de
    // raytracing.
    let mut fb = Framebuffer::new(WINDOW_WIDTH as usize, WINDOW_HEIGHT as usize);
    for y in 0..fb.height {
        for x in 0..fb.width {
            let u = x as f32 / fb.width as f32;
            let v = y as f32 / fb.height as f32;
            fb.set(x, y, encode_pixel(Vec3::new(u, v, 0.35)));
        }
    }

    let image = Image::gen_image_color(WINDOW_WIDTH, WINDOW_HEIGHT, Color::BLACK);
    let mut texture = rl
        .load_texture_from_image(&thread, &image)
        .expect("no se pudo crear la textura de destino");
    texture
        .update_texture(&fb.pixels)
        .expect("el tamaño del framebuffer no coincide con el de la textura");

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture(&texture, 0, 0, Color::WHITE);
    }
}
