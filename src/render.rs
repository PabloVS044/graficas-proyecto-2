//! El raytracer: de un rayo a un color, y de la escena a un framebuffer.

use crate::camera::CameraFrame;
use crate::framebuffer::{Framebuffer, encode_pixel};
use crate::material::Material;
use crate::ray_intersect::{EPSILON, Intersect};
use crate::scene::Scene;
use crate::vector::{Vec3, dot, normalize, reflect_vec};

/// Rebotes maximos de un rayo. Cada nivel multiplica el trabajo, asi que se
/// mantiene bajo: con 3 ya se ve un reflejo dentro de un reflejo y el cristal
/// se atraviesa entrando y saliendo, que es todo lo que la escena necesita.
pub const MAX_DEPTH: u32 = 3;

/// Cuantos bloqueadores transparentes seguidos atraviesa un rayo de sombra
/// antes de rendirse y declarar sombra total.
const MAX_SHADOW_STEPS: u32 = 4;

// ---------------------------------------------------------------------------
// Optica
// ---------------------------------------------------------------------------

/// Aproximacion de Schlick a las ecuaciones de Fresnel.
///
/// Devuelve que fraccion de la luz se **refleja** en la frontera entre dos
/// medios. Es lo que hace que el cristal visto de frente sea casi transparente
/// y visto de canto se comporte como un espejo; sin esto el vidrio se ve como
/// plastico translucido.
#[inline]
fn fresnel(incident: &Vec3, normal: &Vec3, refractive_index: f32) -> f32 {
    let cos_i = dot(incident, normal).clamp(-1.0, 1.0);
    // Se decide de que lado de la superficie viene el rayo para ordenar los
    // indices: de aire a vidrio o de vidrio a aire.
    let (eta_i, eta_t) = if cos_i > 0.0 {
        (refractive_index, 1.0)
    } else {
        (1.0, refractive_index)
    };

    let sin_t = eta_i / eta_t * (1.0 - cos_i * cos_i).max(0.0).sqrt();
    if sin_t >= 1.0 {
        // Reflexion interna total: nada sale, todo se refleja.
        return 1.0;
    }

    let cos_t = (1.0 - sin_t * sin_t).max(0.0).sqrt();
    let cos_i = cos_i.abs();
    let r_s = ((eta_t * cos_i) - (eta_i * cos_t)) / ((eta_t * cos_i) + (eta_i * cos_t));
    let r_p = ((eta_i * cos_i) - (eta_t * cos_t)) / ((eta_i * cos_i) + (eta_t * cos_t));
    ((r_s * r_s + r_p * r_p) * 0.5).clamp(0.0, 1.0)
}

/// Ley de Snell. Devuelve `None` si hay reflexion interna total.
#[inline]
fn refract(incident: &Vec3, normal: &Vec3, refractive_index: f32) -> Option<Vec3> {
    let mut cos_i = dot(incident, normal).clamp(-1.0, 1.0);
    let mut n = *normal;
    let (eta_i, eta_t);

    if cos_i < 0.0 {
        // El rayo entra al material.
        cos_i = -cos_i;
        eta_i = 1.0;
        eta_t = refractive_index;
    } else {
        // El rayo sale: se invierte la normal y se intercambian los indices.
        n = -n;
        eta_i = refractive_index;
        eta_t = 1.0;
    }

    let eta = eta_i / eta_t;
    let k = 1.0 - eta * eta * (1.0 - cos_i * cos_i);
    if k < 0.0 {
        None
    } else {
        Some(normalize(&(incident * eta + n * (eta * cos_i - k.sqrt()))))
    }
}

/// Desplaza el origen de un rayo secundario para que no vuelva a chocar con la
/// superficie de la que sale.
#[inline]
fn offset_origin(point: &Vec3, normal: &Vec3, direction: &Vec3) -> Vec3 {
    let side = if dot(direction, normal) < 0.0 { -1.0 } else { 1.0 };
    point + normal * (EPSILON * side)
}

// ---------------------------------------------------------------------------
// Sombreado
// ---------------------------------------------------------------------------

/// Color difuso del material en el punto de impacto: textura si tiene, color
/// plano si no.
#[inline]
fn surface_color(scene: &Scene, material: &Material, hit: &Intersect) -> Vec3 {
    match material.texture {
        Some(id) => scene.textures.sample(id, hit.u, hit.v),
        None => material.diffuse,
    }
}

/// Luz que el material emite por si mismo en este punto.
#[inline]
fn surface_emission(scene: &Scene, material: &Material, hit: &Intersect) -> Vec3 {
    match material.emission_map {
        // La mascara modula la emision por pixel: donde la mascara es negra el
        // bloque no brilla, donde es blanca brilla al maximo.
        Some(id) => {
            let mask = scene.textures.sample(id, hit.u, hit.v);
            Vec3::new(
                material.emission.x * mask.x,
                material.emission.y * mask.y,
                material.emission.z * mask.z,
            )
        }
        None => material.emission,
    }
}

/// Normal sombreada: la de la cara, perturbada por el normal map si lo hay.
///
/// El normal map guarda un vector en "espacio tangente" (relativo a la cara).
/// La matriz TBN lo lleva a coordenadas de mundo. Como los cubos estan
/// alineados a los ejes, T, B y N salen directo de la cara golpeada y no hace
/// falta calcularlos por triangulo.
#[inline]
fn shading_normal(scene: &Scene, material: &Material, hit: &Intersect) -> Vec3 {
    match material.normal_map {
        Some(id) => {
            let n = scene.textures.sample_normal(id, hit.u, hit.v);
            normalize(&(hit.tangent * n.x + hit.bitangent * n.y + hit.normal * n.z))
        }
        None => hit.normal,
    }
}

/// Fraccion de la luz de `light` que llega a `point`.
///
/// Un bloqueador opaco la deja en cero. Uno transparente solo la tiñe y la
/// atenua: asi el cristal proyecta una sombra clara y con su color en vez de
/// una mancha negra.
fn light_transmittance(scene: &Scene, point: &Vec3, normal: &Vec3, light_pos: &Vec3) -> Vec3 {
    let to_light = light_pos - point;
    let mut remaining = crate::vector::length(&to_light);
    let direction = to_light / remaining;

    let mut origin = offset_origin(point, normal, &direction);
    let mut transmittance = Vec3::new(1.0, 1.0, 1.0);

    for _ in 0..MAX_SHADOW_STEPS {
        let hit = scene.hit_within(&origin, &direction, remaining);
        if !hit.is_intersecting {
            return transmittance;
        }

        let material = scene.material(hit.material);
        if !material.is_transparent() {
            return Vec3::zeros();
        }

        // Lo que pasa a traves del bloqueador, teñido por su color.
        let tint = surface_color(scene, material, &hit);
        let k = material.albedo[3];
        transmittance = Vec3::new(
            transmittance.x * tint.x * k,
            transmittance.y * tint.y * k,
            transmittance.z * tint.z * k,
        );

        if transmittance.x + transmittance.y + transmittance.z < 1e-3 {
            return Vec3::zeros();
        }

        // Se sigue avanzando desde justo despues del bloqueador.
        remaining -= hit.distance;
        origin = hit.point + direction * EPSILON;
    }

    Vec3::zeros()
}

// ---------------------------------------------------------------------------
// El rayo
// ---------------------------------------------------------------------------

/// Lanza un rayo y devuelve el color que trae de vuelta, en espacio lineal.
pub fn cast_ray(scene: &Scene, origin: &Vec3, direction: &Vec3, depth: u32) -> Vec3 {
    if depth > MAX_DEPTH {
        return scene.sky(direction);
    }

    let hit = scene.hit(origin, direction);
    if !hit.is_intersecting {
        return scene.sky(direction);
    }

    let material = scene.material(hit.material);
    let base_color = surface_color(scene, material, &hit);
    let normal = shading_normal(scene, material, &hit);
    let view = -direction;

    // --- Luz directa: Phong (difuso + especular) con sombras ---------------
    let mut diffuse = Vec3::zeros();
    let mut specular = Vec3::zeros();

    for light in &scene.lights {
        let light_dir = normalize(&(light.position - hit.point));

        let n_dot_l = dot(&normal, &light_dir);
        if n_dot_l <= 0.0 {
            continue; // la cara le da la espalda a la luz
        }

        let shadow = light_transmittance(scene, &hit.point, &hit.normal, &light.position);
        if shadow == Vec3::zeros() {
            continue;
        }

        let radiance = light.radiance_at(&hit.point);
        let contribution = Vec3::new(
            radiance.x * shadow.x,
            radiance.y * shadow.y,
            radiance.z * shadow.z,
        );

        diffuse += contribution * n_dot_l;

        // Especular de Phong: el brillo aparece donde el reflejo de la luz
        // apunta hacia el ojo.
        let reflected = reflect_vec(&(-light_dir), &normal);
        let spec = dot(&reflected, &view).max(0.0).powf(material.specular);
        specular += contribution * spec;
    }

    let ambient = scene.ambient;
    let lit = Vec3::new(
        base_color.x * (diffuse.x + ambient.x),
        base_color.y * (diffuse.y + ambient.y),
        base_color.z * (diffuse.z + ambient.z),
    );

    let mut color = lit * material.albedo[0] + specular * material.albedo[1];
    color += surface_emission(scene, material, &hit);

    // --- Rayos secundarios -------------------------------------------------
    let wants_reflection = material.albedo[2] > 0.0;
    let wants_refraction = material.albedo[3] > 0.0;

    if (wants_reflection || wants_refraction) && depth < MAX_DEPTH {
        // Fresnel reparte energia entre reflejo y transmision segun el angulo.
        // Solo aplica a materiales que refractan; un espejo opaco refleja lo
        // mismo desde cualquier angulo.
        let kr = if wants_refraction {
            fresnel(direction, &normal, material.refractive_index)
        } else {
            0.0
        };

        if wants_reflection || kr > 0.0 {
            let dir = normalize(&reflect_vec(direction, &normal));
            let start = offset_origin(&hit.point, &normal, &dir);
            let reflected = cast_ray(scene, &start, &dir, depth + 1);
            let weight = material.albedo[2] + material.albedo[3] * kr;
            color += reflected * weight;
        }

        if wants_refraction && kr < 1.0 {
            if let Some(dir) = refract(direction, &normal, material.refractive_index) {
                let start = offset_origin(&hit.point, &normal, &dir);
                let transmitted = cast_ray(scene, &start, &dir, depth + 1);
                let weight = material.albedo[3] * (1.0 - kr);
                // Se tiñe con el color del material: el cristal verde deja
                // pasar luz verde.
                color += Vec3::new(
                    transmitted.x * base_color.x,
                    transmitted.y * base_color.y,
                    transmitted.z * base_color.z,
                ) * weight;
            }
        }
    }

    color
}

// ---------------------------------------------------------------------------
// Render paralelo
// ---------------------------------------------------------------------------

/// Traza la imagen completa repartiendo las filas entre hilos.
///
/// Por que asi y no de otra forma: cada pixel es independiente de los demas
/// (no comparten estado, no hay orden), asi que basta con partir el buffer en
/// bandas horizontales disjuntas con `chunks_mut` y repartirlas. Cada banda es
/// una rebanada `&mut [u8]` distinta, por lo que el compilador garantiza que no
/// hay dos hilos escribiendo el mismo pixel: cero sincronizacion dentro del
/// bucle de pixeles.
///
/// Las bandas no se reparten de antemano sino que los hilos las van tomando de
/// una cola conforme se desocupan (*work stealing*). Importa porque el costo
/// por banda es muy desigual: la franja donde esta el cristal dispara rayos de
/// refraccion y reflexion, mientras que la del cielo vacio se resuelve al
/// primer rebote. Con reparto fijo el frame duraria lo que tarde el hilo mas
/// desafortunado. El unico candado se toma una vez por banda, no por pixel.
///
/// `std::thread::scope` permite que los hilos tomen prestada la `&Scene` sin
/// envolverla en un `Arc`, porque garantiza que todos terminan antes de que la
/// funcion regrese.
pub fn render(
    framebuffer: &mut Framebuffer,
    scene: &Scene,
    camera: &CameraFrame,
    samples_per_axis: u32,
    threads: usize,
) {
    let width = framebuffer.width;
    let height = framebuffer.height;
    let row_bytes = framebuffer.row_bytes();

    // Varias bandas por hilo: mientras mas finas, mejor se reparte la carga,
    // pero mas veces se toma el candado. ~8 por hilo es un punto intermedio.
    let target_bands = (threads * 8).max(1);
    let rows_per_band = height.div_ceil(target_bands).max(1);

    let fwidth = width as f32;
    let fheight = height as f32;
    let samples = samples_per_axis.max(1);
    let inv_samples = 1.0 / (samples * samples) as f32;

    // Cola de trabajo: (fila inicial, rebanada del buffer). En orden inverso
    // para que `pop` entregue las bandas de arriba hacia abajo.
    let queue: Vec<(usize, &mut [u8])> = framebuffer
        .pixels
        .chunks_mut(rows_per_band * row_bytes)
        .enumerate()
        .map(|(i, band)| (i * rows_per_band, band))
        .rev()
        .collect();
    let queue = std::sync::Mutex::new(queue);

    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                loop {
                    // Se toma la siguiente banda libre y se suelta el candado
                    // de inmediato: el trazado ocurre fuera de la seccion
                    // critica.
                    let next = queue.lock().unwrap().pop();
                    let Some((y0, band)) = next else { break };

                    let rows = band.len() / row_bytes;
                    for row in 0..rows {
                        let y = y0 + row;
                        for x in 0..width {
                            let mut accum = Vec3::zeros();

                            // Supersampling: varias muestras dentro del pixel,
                            // promediadas. Es lo que quita el escalonado en los
                            // bordes de los cubos.
                            for sy in 0..samples {
                                for sx in 0..samples {
                                    let px = x as f32 + (sx as f32 + 0.5) / samples as f32;
                                    let py = y as f32 + (sy as f32 + 0.5) / samples as f32;
                                    let dir = camera.ray_direction(px, py, fwidth, fheight);
                                    accum += cast_ray(scene, &camera.eye, &dir, 0);
                                }
                            }

                            let rgba = encode_pixel(accum * inv_samples);
                            let i = row * row_bytes + x * 4;
                            band[i..i + 4].copy_from_slice(&rgba);
                        }
                    }
                }
            });
        }
    });
}
