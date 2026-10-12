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
use vector::Vec3;

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

    // Modo sin ventana: traza el recorrido del video a una secuencia de PNG.
    if std::env::args().any(|a| a == "--video") {
        render_video(&mut scene, threads);
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

/// Una toma del recorrido: hacia donde mira la camara y desde donde.
struct Shot {
    focus: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    /// Segundos para llegar a esta toma desde la anterior.
    travel: f32,
    /// Segundos que se queda en la toma.
    hold: f32,
    /// Giro (radianes) mientras se queda: una toma completamente quieta se ve
    /// como una foto, y con un poco de giro el relieve y los reflejos se mueven.
    drift: f32,
}

/// El recorrido del video. Abre con una vuelta completa a la isla, se acerca a
/// cada efecto uno por uno y cierra alejandose.
///
/// Los angulos estan desenrollados (siempre crecen o casi) para que la
/// interpolacion no de vueltas de mas entre dos tomas.
fn tour() -> Vec<Shot> {
    use std::f32::consts::TAU;
    let shot = |focus: (f32, f32, f32), yaw: f32, pitch: f32, distance: f32, travel: f32, hold: f32, drift: f32| Shot {
        focus: Vec3::new(focus.0, focus.1, focus.2),
        yaw,
        pitch,
        distance,
        travel,
        hold,
        drift,
    };
    vec![
        // Vuelta completa a la isla: skybox, terreno, arboles, cascada.
        shot((-1.0, 1.5, 0.0), 0.7, 0.42, 45.0, 0.0, 9.0, TAU),
        // Estanque desde arriba: el fondo de arena deformado por la refraccion.
        shot((-9.5, 0.0, 7.0), TAU + 0.3, 0.75, 15.0, 3.0, 4.0, 0.25),
        // Cascada de frente, con el mar de nubes detras.
        shot((-7.5, -3.0, 15.0), TAU + 0.15, 0.12, 19.0, 3.0, 4.0, 0.35),
        // Acantilado con minerales: relieve de los mapas normales.
        shot((-1.0, -3.0, 15.0), TAU + 0.55, 0.08, 13.0, 2.5, 4.0, 0.25),
        // Columnas de obsidiana reflejando la lava, desde el lado de la lava.
        shot((-1.5, 1.5, 7.0), TAU + 1.35, 0.28, 15.0, 3.0, 4.5, 0.35),
        // Ventanas de la cabaña: cristal que refracta y la lampara de dentro.
        shot((-8.0, 2.5, -2.0), TAU + 0.15, 0.18, 14.0, 3.0, 4.0, 0.25),
        // Zona de colinas procedural.
        shot((8.0, 2.0, -5.0), TAU + 2.4, 0.38, 27.0, 3.5, 4.5, 0.4),
        // A contraluz, mirando hacia el sol del skybox.
        shot((-1.0, 3.0, 0.0), TAU + 4.15, 0.06, 50.0, 3.5, 4.0, 0.3),
        // Cierre: se aleja y sube.
        shot((-1.0, 1.5, 0.0), 2.0 * TAU + 0.7, 0.55, 60.0, 4.0, 2.0, 0.2),
    ]
}

/// Cuanto sube (radianes de inclinacion) y se aleja la camara a la mitad de
/// cada transicion.
const TRAVEL_LIFT: f32 = 0.45;
const TRAVEL_BACKOFF: f32 = 12.0;

#[inline]
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Camara del recorrido en el segundo `t`, o `None` si ya termino.
fn tour_camera(shots: &[Shot], t: f32, camera: &mut Camera) -> Option<()> {
    let mut start = 0.0;
    // Estado al terminar la toma anterior (incluye su giro).
    let mut prev = (shots[0].focus, shots[0].yaw, shots[0].pitch, shots[0].distance);
    for shot in shots {
        let travel_end = start + shot.travel;
        let hold_end = travel_end + shot.hold;
        if t < hold_end {
            if t < travel_end {
                let k = smooth((t - start) / shot.travel);
                // Arco: a medio camino la camara sube y se aleja. Yendo en
                // linea recta entre dos tomas cercanas al suelo atravesaba
                // colinas y copas de arboles.
                let arc = (std::f32::consts::PI * k).sin();
                camera.center = prev.0 * (1.0 - k) + shot.focus * k;
                camera.yaw = prev.1 + (shot.yaw - prev.1) * k;
                camera.pitch = prev.2 + (shot.pitch - prev.2) * k + TRAVEL_LIFT * arc;
                camera.distance = prev.3 + (shot.distance - prev.3) * k + TRAVEL_BACKOFF * arc;
            } else {
                let k = (t - travel_end) / shot.hold;
                camera.center = shot.focus;
                camera.yaw = shot.yaw + shot.drift * k;
                camera.pitch = shot.pitch;
                camera.distance = shot.distance;
            }
            return Some(());
        }
        prev = (shot.focus, shot.yaw + shot.drift, shot.pitch, shot.distance);
        start = hold_end;
    }
    None
}

/// Traza el recorrido del video y lo guarda como una secuencia de PNG en
/// `frames/`.
///
/// Se hace sin ventana a proposito: grabando la pantalla se captura lo que el
/// refinamiento progresivo alcance a producir en cada instante, asi que los
/// cuadros saldrian de calidades distintas. Aqui cada cuadro se traza completo
/// antes de pasar al siguiente, y el tiempo de la animacion avanza a 30 cuadros
/// por segundo, que es la velocidad a la que se va a reproducir.
fn render_video(scene: &mut scene::Scene, threads: usize) {
    const FPS: f32 = 30.0;

    let env_num = |name: &str, fallback: f32| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(fallback)
    };

    let width = env_num("VIDEO_WIDTH", 1280.0) as usize;
    let height = env_num("VIDEO_HEIGHT", 720.0) as usize;
    let samples = env_num("VIDEO_SAMPLES", 2.0) as u32;
    // Trazar solo uno de cada N cuadros: sirve para revisar el recorrido
    // rapido antes de trazarlo completo.
    let step = env_num("VIDEO_STEP", 1.0).max(1.0) as usize;

    if let Err(err) = std::fs::create_dir_all("frames") {
        eprintln!("no se pudo crear el directorio frames/: {err}");
        std::process::exit(1);
    }

    let shots = tour();
    let total_secs: f32 = shots.iter().map(|s| s.travel + s.hold).sum();
    let frames = (total_secs * FPS) as usize;

    let mut framebuffer = Framebuffer::new(width, height);
    let mut camera = Camera::new(scene.focus, shots[0].distance);

    println!(
        "trazando {} de {frames} cuadros ({total_secs:.0} s) de {width}x{height} con {samples}x{samples} muestras",
        frames.div_ceil(step)
    );
    let started = std::time::Instant::now();

    for index in (0..frames).step_by(step) {
        let t = index as f32 / FPS;
        if tour_camera(&shots, t, &mut camera).is_none() {
            break;
        }
        scene.time = t;

        let frame = CameraFrame::new(&camera, width, height);
        render::render(&mut framebuffer, scene, &frame, samples, threads);

        let path = format!("frames/{index:04}.png");
        if let Err(err) = framebuffer.save_png(&path) {
            eprintln!("{err}");
            std::process::exit(1);
        }

        let done = index / step + 1;
        if done % 30 == 0 || index + step >= frames {
            let todo = frames.div_ceil(step);
            let elapsed = started.elapsed().as_secs_f32();
            let remaining = elapsed / done as f32 * (todo - done.min(todo)) as f32;
            println!("  {done}/{todo}  ({elapsed:.0}s transcurridos, ~{remaining:.0}s restantes)");
        }
    }

    println!("listo en {:.1?}", started.elapsed());
    println!("para armar el video:");
    println!("  ffmpeg -framerate 30 -i frames/%04d.png -c:v libx264 -pix_fmt yuv420p -crf 18 diorama.mp4");
}
