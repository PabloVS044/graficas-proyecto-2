mod camera;
mod cube;
mod framebuffer;
mod ray_intersect;
mod render;
mod scene;

use camera::{Camera, CameraFrame};
use framebuffer::Framebuffer;
use nalgebra_glm::Vec3;
use raylib::prelude::*;

const WINDOW_WIDTH: i32 = 960;
const WINDOW_HEIGHT: i32 = 640;

fn main() {
    let scene = scene::build();

    let (mut rl, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Diorama raytracing - proyecto 2")
        .build();
    rl.set_target_fps(60);

    // La escena no cambia todavia, asi que se traza una sola vez al arrancar.
    let mut fb = Framebuffer::new(WINDOW_WIDTH as usize, WINDOW_HEIGHT as usize);
    let camera = Camera::new(Vec3::new(9.0, 7.0, 12.0), Vec3::new(0.0, 0.0, 0.0));
    let frame = CameraFrame::new(&camera, fb.width, fb.height);

    let started = std::time::Instant::now();
    render::render(&mut fb, &scene, &frame);
    println!("trazado en {:.2?}", started.elapsed());

    let image = Image::gen_image_color(WINDOW_WIDTH, WINDOW_HEIGHT, Color::BLACK);
    let mut texture = rl
        .load_texture_from_image(&thread, &image)
        .expect("no se pudo crear la textura de destino");
    texture
        .update_texture(&fb.pixels)
        .expect("el tamaño del framebuffer no coincide con el de la textura");

    while !rl.window_should_close() {
        if rl.is_key_pressed(KeyboardKey::KEY_P) {
            match fb.save_png("captura.png") {
                Ok(()) => println!("captura guardada en captura.png"),
                Err(err) => eprintln!("{err}"),
            }
        }

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture(&texture, 0, 0, Color::WHITE);
    }
}
