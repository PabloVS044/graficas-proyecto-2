mod camera;
mod cube;
mod framebuffer;
mod light;
mod material;
mod ray_intersect;
mod render;
mod scene;
mod texture;
mod voxel;

use camera::{Camera, CameraFrame};
use framebuffer::Framebuffer;
use raylib::prelude::*;

const WINDOW_WIDTH: i32 = 960;
const WINDOW_HEIGHT: i32 = 640;

/// Niveles de calidad, de mas rapido a mas lento.
///
/// `(divisor, muestras_por_eje)`: el divisor reduce la resolucion interna del
/// render y el resultado se estira al tamaño de la ventana. Mientras la camara
/// se mueve se usa el primer nivel, que es borroso pero corre a decenas de
/// frames por segundo; en cuanto la camara se queda quieta el render va
/// subiendo de nivel frame a frame hasta llegar a la imagen final.
///
/// Este refinamiento progresivo es lo que permite tener un raytracer por
/// software interactivo: nunca se espera la imagen buena para poder moverse.
const QUALITY_LEVELS: &[(i32, u32)] = &[(8, 1), (4, 1), (2, 1), (1, 1), (1, 2)];

/// Velocidades de los controles de camara.
const ORBIT_SPEED: f32 = 1.4; // radianes por segundo
const ZOOM_SPEED: f32 = 1.9; // factor por segundo
const PAN_SPEED: f32 = 3.0;
const AUTO_ROTATE_SPEED: f32 = 0.35;

/// Un nivel de calidad ya reservado: su framebuffer en CPU y su textura en GPU.
struct RenderTarget {
    framebuffer: Framebuffer,
    texture: Texture2D,
    scale: f32,
    samples: u32,
}

fn main() {
    let scene = match scene::build() {
        Ok(scene) => scene,
        Err(err) => {
            eprintln!("error al construir la escena: {err}");
            std::process::exit(1);
        }
    };

    // Un hilo por nucleo. Si el sistema no lo sabe decir, 4 es una apuesta
    // razonable.
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let (mut rl, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Diorama raytracing - proyecto 2")
        .build();
    rl.set_target_fps(60);

    let mut targets: Vec<RenderTarget> = QUALITY_LEVELS
        .iter()
        .map(|&(divisor, samples)| {
            let w = WINDOW_WIDTH / divisor;
            let h = WINDOW_HEIGHT / divisor;
            let image = Image::gen_image_color(w, h, Color::BLACK);
            let texture = rl
                .load_texture_from_image(&thread, &image)
                .expect("no se pudo crear la textura de destino");
            RenderTarget {
                framebuffer: Framebuffer::new(w as usize, h as usize),
                texture,
                scale: divisor as f32,
                samples,
            }
        })
        .collect();

    let mut camera = Camera::new(scene.focus, 30.0);
    let mut auto_rotate = true;
    let mut show_hud = true;

    // Nivel que se va a dibujar este frame. Empieza en el mas burdo y sube
    // mientras nadie toque la camara.
    let mut level = 0usize;
    // Nivel que ya esta trazado y listo para mostrarse.
    let mut ready_level = 0usize;
    // `true` mientras quede calidad por ganar. Se apaga al llegar al ultimo
    // nivel y se vuelve a encender en cuanto la camara se mueve.
    let mut refining = true;
    let mut last_ms = 0.0f64;

    println!("hilos de render: {threads}");
    println!("controles: flechas/AD = rotar | W,S o rueda = zoom | Q,E = subir/bajar | R = auto-giro | P = captura PNG | H = HUD");

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();

        // --- Entrada -------------------------------------------------------
        let mut d_yaw = 0.0;
        let mut d_pitch = 0.0;

        if rl.is_key_down(KeyboardKey::KEY_LEFT) || rl.is_key_down(KeyboardKey::KEY_A) {
            d_yaw -= ORBIT_SPEED * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_RIGHT) || rl.is_key_down(KeyboardKey::KEY_D) {
            d_yaw += ORBIT_SPEED * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_UP) {
            d_pitch += ORBIT_SPEED * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_DOWN) {
            d_pitch -= ORBIT_SPEED * dt;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            auto_rotate = !auto_rotate;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_H) {
            show_hud = !show_hud;
        }

        if auto_rotate {
            d_yaw += AUTO_ROTATE_SPEED * dt;
        }

        camera.orbit(d_yaw, d_pitch);

        let mut zoom = 1.0;
        if rl.is_key_down(KeyboardKey::KEY_W) {
            zoom /= 1.0 + ZOOM_SPEED * dt;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            zoom *= 1.0 + ZOOM_SPEED * dt;
        }
        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 {
            zoom *= (1.0 - wheel * 0.12).clamp(0.5, 2.0);
        }
        camera.zoom(zoom);

        if rl.is_key_down(KeyboardKey::KEY_E) {
            camera.pan_vertical(PAN_SPEED * dt);
        }
        if rl.is_key_down(KeyboardKey::KEY_Q) {
            camera.pan_vertical(-PAN_SPEED * dt);
        }

        // --- Decidir que nivel trazar --------------------------------------
        if camera.dirty {
            // La camara se movio: se tira el refinamiento y se vuelve al nivel
            // rapido.
            camera.dirty = false;
            level = 0;
            refining = true;
        }

        // Solo se traza cuando hay algo nuevo que trazar. Sin esta condicion el
        // programa seguiria relanzando el mismo cuadro de maxima calidad una y
        // otra vez con la camara quieta, dejando los 16 nucleos al 100% para
        // producir exactamente la misma imagen.
        if refining {
            let target = &mut targets[level];
            let frame =
                CameraFrame::new(&camera, target.framebuffer.width, target.framebuffer.height);

            let started = std::time::Instant::now();
            render::render(
                &mut target.framebuffer,
                &scene,
                &frame,
                target.samples,
                threads,
            );
            last_ms = started.elapsed().as_secs_f64() * 1000.0;

            target
                .texture
                .update_texture(&target.framebuffer.pixels)
                .expect("el tamaño del framebuffer no coincide con el de la textura");

            ready_level = level;
            // El siguiente cuadro sube un escalon de calidad, hasta llegar al
            // ultimo, donde el refinamiento se detiene.
            if level + 1 < targets.len() {
                level += 1;
            } else {
                refining = false;
            }
        }

        // --- Captura en alta resolucion ------------------------------------
        if rl.is_key_pressed(KeyboardKey::KEY_P) {
            let path = "captura.png";
            let mut hires =
                Framebuffer::new((WINDOW_WIDTH * 2) as usize, (WINDOW_HEIGHT * 2) as usize);
            let hires_frame = CameraFrame::new(&camera, hires.width, hires.height);
            render::render(&mut hires, &scene, &hires_frame, 3, threads);
            match hires.save_png(path) {
                Ok(()) => println!("captura guardada en {path}"),
                Err(err) => eprintln!("{err}"),
            }
        }

        // --- Dibujar -------------------------------------------------------
        let shown = &targets[ready_level];
        let hud = format!(
            "{}x{}  x{} muestras{}  |  {:.1} ms  |  {} hilos  |  {} cubos",
            shown.framebuffer.width,
            shown.framebuffer.height,
            shown.samples,
            if refining { "  (refinando)" } else { "" },
            last_ms,
            threads,
            scene.cubes.len()
        );

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        d.draw_texture_ex(
            &shown.texture,
            Vector2::zero(),
            0.0,
            shown.scale,
            Color::WHITE,
        );

        if show_hud {
            d.draw_rectangle(0, 0, WINDOW_WIDTH, 52, Color::new(0, 0, 0, 150));
            d.draw_text(&hud, 12, 10, 18, Color::RAYWHITE);
            d.draw_text(
                "flechas/AD rotar  W/S zoom  Q/E altura  R auto-giro  P captura  H HUD",
                12,
                32,
                14,
                Color::LIGHTGRAY,
            );
        }
    }
}
