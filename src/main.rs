mod accel;
mod camera;
mod cube;
mod framebuffer;
mod light;
mod material;
mod ray_intersect;
mod render;
mod scene;
mod skybox;
mod terrain;
mod texture;
mod vector;
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

/// Nivel tope mientras los liquidos se animan: resolucion completa con una
/// muestra (~30 ms por cuadro). El ultimo nivel, con 2x2 muestras, tarda unos
/// 110 ms y la animacion se veria a tirones.
const ANIMATION_LEVEL: usize = 3;

/// Velocidades de los controles de camara.
const ORBIT_SPEED: f32 = 1.4; // radianes por segundo
const ZOOM_SPEED: f32 = 1.9; // factor por segundo
const PAN_SPEED: f32 = 3.0;

/// Un nivel de calidad ya reservado: su framebuffer en CPU y su textura en GPU.
struct RenderTarget {
    framebuffer: Framebuffer,
    texture: Texture2D,
    scale: f32,
    samples: u32,
}

fn main() {
    let mut scene = match scene::build() {
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

    // Modo sin ventana: traza una orbita completa a una secuencia de PNG.
    if std::env::args().any(|a| a == "--orbit") {
        render_orbit(&mut scene, threads);
        return;
    }

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

    let mut camera = Camera::new(scene.focus, 48.0);
    let mut show_hud = true;
    // Animacion de los liquidos. Encendida por defecto. Mientras anima, cada
    // cuadro es distinto y la imagen se queda en `ANIMATION_LEVEL` en vez de
    // refinarse hasta la calidad maxima; con T se apaga y la imagen quieta
    // sube hasta 2x2 muestras.
    let mut animate = true;

    // Nivel que se va a dibujar este frame. Empieza en el mas burdo y sube
    // mientras nadie toque la camara.
    let mut level = 0usize;
    // Nivel que ya esta trazado y listo para mostrarse.
    let mut ready_level = 0usize;
    // `true` mientras quede calidad por ganar. Se apaga al llegar al ultimo
    // nivel y se vuelve a encender en cuanto la camara se mueve.
    let mut refining = true;

    println!("hilos de render: {threads}");
    println!("controles: flechas/AD = rotar | W,S o rueda = zoom | Q,E = subir/bajar | T = pausar animacion | H = HUD");

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

        if rl.is_key_pressed(KeyboardKey::KEY_H) {
            show_hud = !show_hud;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_T) {
            animate = !animate;
        }
        if animate {
            scene.time += dt;
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
        if animate {
            // Cada cuadro es distinto, asi que hay que seguir trazando aunque
            // la camara este quieta, pero sin pasar del nivel de animacion.
            refining = true;
            level = level.min(ANIMATION_LEVEL);
        }

        // Solo se traza cuando hay algo nuevo que trazar. Sin esta condicion el
        // programa seguiria relanzando el mismo cuadro de maxima calidad una y
        // otra vez con la camara quieta, dejando los 16 nucleos al 100% para
        // producir exactamente la misma imagen.
        if refining {
            let target = &mut targets[level];
            let frame =
                CameraFrame::new(&camera, target.framebuffer.width, target.framebuffer.height);

            render::render(
                &mut target.framebuffer,
                &scene,
                &frame,
                target.samples,
                threads,
            );

            target
                .texture
                .update_texture(&target.framebuffer.pixels)
                .expect("el tamaño del framebuffer no coincide con el de la textura");

            ready_level = level;
            // El siguiente cuadro sube un escalon de calidad, hasta llegar al
            // ultimo, donde el refinamiento se detiene.
            let top = if animate { ANIMATION_LEVEL } else { targets.len() - 1 };
            if level < top {
                level += 1;
            } else if !animate {
                refining = false;
            }
        }

        // --- Dibujar -------------------------------------------------------
        let shown = &targets[ready_level];
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
            d.draw_rectangle(0, 0, WINDOW_WIDTH, 30, Color::new(0, 0, 0, 150));
            d.draw_text(
                "flechas/AD rotar   W/S zoom   Q/E altura   T pausar animacion   H HUD",
                12,
                8,
                14,
                Color::LIGHTGRAY,
            );
        }
    }
}

/// Traza una vuelta completa alrededor del diorama y la guarda como una
/// secuencia de PNG en `frames/`.
///
/// Es lo que alimenta el video de la entrega. Se hace sin ventana a proposito:
/// grabando la pantalla se captura lo que el refinamiento progresivo alcance a
/// producir en cada instante, asi que los cuadros saldrian de calidades
/// distintas. Aqui cada cuadro se traza completo antes de pasar al siguiente.
///
/// La trayectoria combina los dos movimientos: una vuelta entera de rotacion y
/// dos ciclos de acercamiento y alejamiento, desfasados respecto a la rotacion
/// para que se vean como movimientos separados y no como uno solo.
fn render_orbit(scene: &mut scene::Scene, threads: usize) {
    use std::f32::consts::TAU;

    let env_num = |name: &str, fallback: f32| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(fallback)
    };

    let frames = env_num("ORBIT_FRAMES", 240.0) as usize;
    let width = env_num("ORBIT_WIDTH", WINDOW_WIDTH as f32) as usize;
    let height = env_num("ORBIT_HEIGHT", WINDOW_HEIGHT as f32) as usize;
    let samples = env_num("ORBIT_SAMPLES", 2.0) as u32;
    let near = env_num("ORBIT_NEAR", 27.0);
    let far = env_num("ORBIT_FAR", 53.0);

    if let Err(err) = std::fs::create_dir_all("frames") {
        eprintln!("no se pudo crear el directorio frames/: {err}");
        std::process::exit(1);
    }

    let mut framebuffer = Framebuffer::new(width, height);
    let mut camera = Camera::new(scene.focus, far);

    println!("trazando {frames} cuadros de {width}x{height} con {samples}x{samples} muestras");
    let started = std::time::Instant::now();

    for index in 0..frames {
        let t = index as f32 / frames as f32;

        camera.yaw = TAU * t;
        // Dos ciclos de zoom por vuelta. El coseno arranca y termina en el mismo
        // punto, asi que el video se puede repetir en bucle sin salto.
        let zoom = 0.5 - 0.5 * (TAU * 2.0 * t).cos();
        camera.distance = far + (near - far) * zoom;
        // La altura tambien se mueve, desfasada un cuarto de ciclo, para que la
        // camara no describa un circulo plano.
        camera.pitch = 0.42 + 0.24 * (TAU * t).sin();
        // El video va a 30 cuadros por segundo: asi el agua y la lava se
        // mueven a la velocidad real con la que se van a ver.
        scene.time = index as f32 / 30.0;

        let frame = CameraFrame::new(&camera, width, height);
        render::render(&mut framebuffer, scene, &frame, samples, threads);

        let path = format!("frames/{index:04}.png");
        if let Err(err) = framebuffer.save_png(&path) {
            eprintln!("{err}");
            std::process::exit(1);
        }

        if index % 10 == 0 || index + 1 == frames {
            let done = index + 1;
            let elapsed = started.elapsed().as_secs_f32();
            let remaining = elapsed / done as f32 * (frames - done) as f32;
            println!("  {done}/{frames}  ({elapsed:.0}s transcurridos, ~{remaining:.0}s restantes)");
        }
    }

    println!("listo en {:.1?}", started.elapsed());
    println!("para armar el video:");
    println!("  ffmpeg -framerate 30 -i frames/%04d.png -c:v libx264 -pix_fmt yuv420p -crf 18 diorama.mp4");
}
