//! La escena: geometria, materiales, luces y cielo.
//!
//! `Scene` se construye una sola vez al arrancar y despues solo se lee, por eso
//! se puede compartir entre todos los hilos del render con una simple `&Scene`.

use crate::accel::Grid;
use crate::cube::Cube;
use crate::light::Light;
use crate::material::{Material, MaterialId};
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::texture::TextureManager;
use crate::voxel::VoxelWorld;
use crate::vector::Vec3;

pub struct Scene {
    pub cubes: Vec<Cube>,
    pub materials: Vec<Material>,
    pub lights: Vec<Light>,
    pub textures: TextureManager,

    /// Luz de relleno que llega de todas partes. Evita que lo que no recibe luz
    /// directa quede en negro absoluto, que se ve plano y artificial.
    pub ambient: Vec3,

    /// Colores del cielo: lo que ve un rayo que no choca con nada.
    pub sky_top: Vec3,
    pub sky_horizon: Vec3,
    pub sky_bottom: Vec3,

    /// Punto alrededor del cual conviene orbitar la camara.
    pub focus: Vec3,

    /// Rejilla de aceleracion. Se construye una vez a partir de `cubes` y es lo
    /// que evita probar el rayo contra la escena entera.
    pub grid: Grid,
}

impl Scene {
    /// Impacto mas cercano del rayo contra la escena.
    ///
    /// Recorre solo las celdas que el rayo atraviesa, de cerca a lejos, y
    /// prueba unicamente los cubos registrados en ellas. La rejilla corta el
    /// recorrido en cuanto el mejor impacto queda dentro de la celda actual.
    pub fn hit(&self, origin: &Vec3, direction: &Vec3) -> Intersect {
        let mut closest = Intersect::empty();
        self.grid
            .traverse(origin, direction, f32::INFINITY, |items| {
                for &index in items {
                    let hit = self.cubes[index as usize].ray_intersect(origin, direction);
                    if hit.is_intersecting && hit.distance < closest.distance {
                        closest = hit;
                    }
                }
                closest.distance
            });
        closest
    }

    /// Version acotada para rayos de sombra: solo interesa lo que este entre el
    /// punto y la luz. El limite de distancia se le pasa a la rejilla, que asi
    /// deja de recorrer celdas mas alla de la fuente.
    pub fn hit_within(&self, origin: &Vec3, direction: &Vec3, max_distance: f32) -> Intersect {
        let mut closest = Intersect::empty();
        self.grid.traverse(origin, direction, max_distance, |items| {
            for &index in items {
                let hit = self.cubes[index as usize].ray_intersect(origin, direction);
                if hit.is_intersecting
                    && hit.distance < closest.distance
                    && hit.distance < max_distance
                {
                    closest = hit;
                }
            }
            closest.distance
        });
        closest
    }

    #[inline]
    pub fn material(&self, id: MaterialId) -> &Material {
        &self.materials[id]
    }

    /// Color del cielo para un rayo que se escapa de la escena.
    ///
    /// Tres franjas en vez de un degradado simple: el naranja del atardecer
    /// tiene que quedarse pegado al horizonte, no subir hasta el cenit. La
    /// transicion usa `smoothstep` para que no se vea la linea de corte.
    #[inline]
    pub fn sky(&self, direction: &Vec3) -> Vec3 {
        let y = direction.y;
        if y >= 0.0 {
            let t = smoothstep(0.0, 0.45, y);
            self.sky_horizon * (1.0 - t) + self.sky_top * t
        } else {
            let t = smoothstep(0.0, 0.25, -y);
            self.sky_horizon * (1.0 - t) + self.sky_bottom * t
        }
    }
}

#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ---------------------------------------------------------------------------
// Materiales
// ---------------------------------------------------------------------------

// Indices de los materiales dentro de `Scene::materials`.
pub const GRASS: MaterialId = 0;
pub const STONE: MaterialId = 1;
pub const COBBLE: MaterialId = 2;
pub const PLANKS: MaterialId = 3;
pub const LOG: MaterialId = 4;
pub const LEAVES: MaterialId = 5;
pub const WATER: MaterialId = 6;
pub const LAVA: MaterialId = 7;
pub const OBSIDIAN: MaterialId = 8;
pub const GLASS: MaterialId = 9;
pub const GLOWSTONE: MaterialId = 10;
pub const SAND: MaterialId = 11;

/// Los materiales del diorama.
///
/// Cada uno tiene su propia textura y sus propios parametros. Los parametros
/// estan elegidos para que se distingan a simple vista:
///
/// - el pasto y la arena no brillan nada,
/// - el adoquin brilla poco y ancho, como corresponde a una superficie rugosa,
/// - la madera brilla algo mas,
/// - el agua y el cristal refractan, pero con indices distintos (1.33 y 1.50),
///   asi que deforman de forma diferente,
/// - la obsidiana pulida es el unico espejo de verdad,
/// - la lava no depende de ninguna lampara: se ilumina sola.
///
/// Cinco materiales llevan ademas mapa de normales. Se cargan con `load_raw` y
/// no con `load` porque no son colores sino vectores codificados: pasarlos de
/// sRGB a lineal los deformaria.
fn build_materials(textures: &mut TextureManager) -> Result<Vec<Material>, String> {
    let color = |t: &mut TextureManager, n: &str| t.load(&format!("assets/textures/{n}.png"));
    let normal = |t: &mut TextureManager, n: &str| t.load_raw(&format!("assets/normals/{n}.png"));

    let t_grass = color(textures, "grass")?;
    let t_stone = color(textures, "stone")?;
    let t_cobble = color(textures, "cobblestone")?;
    let t_planks = color(textures, "planks")?;
    let t_log = color(textures, "log")?;
    let t_leaves = color(textures, "leaves")?;
    let t_water = color(textures, "water")?;
    let t_lava = color(textures, "lava")?;
    let t_obsidian = color(textures, "obsidian")?;
    let t_glass = color(textures, "glass")?;
    let t_sand = color(textures, "sand")?;
    let t_glowstone = color(textures, "glowstone")?;

    let n_stone = normal(textures, "stone")?;
    let n_cobble = normal(textures, "cobblestone")?;
    let n_planks = normal(textures, "planks")?;
    let n_lava = normal(textures, "lava")?;
    let n_sand = normal(textures, "sand")?;
    let n_log = normal(textures, "log")?;

    Ok(vec![
        Material {
            name: "pasto",
            albedo: [0.95, 0.02, 0.00, 0.00],
            specular: 3.0,
            diffuse: Vec3::new(0.29, 0.52, 0.20),
            texture: Some(t_grass),
            ..Default::default()
        },
        Material {
            name: "piedra",
            albedo: [0.88, 0.06, 0.00, 0.00],
            specular: 10.0,
            diffuse: Vec3::new(0.42, 0.42, 0.45),
            texture: Some(t_stone),
            normal_map: Some(n_stone),
            ..Default::default()
        },
        Material {
            name: "adoquin",
            albedo: [0.90, 0.05, 0.00, 0.00],
            specular: 6.0,
            diffuse: Vec3::new(0.36, 0.36, 0.38),
            texture: Some(t_cobble),
            normal_map: Some(n_cobble),
            ..Default::default()
        },
        Material {
            name: "tablones",
            albedo: [0.90, 0.10, 0.02, 0.00],
            specular: 12.0,
            diffuse: Vec3::new(0.56, 0.38, 0.22),
            texture: Some(t_planks),
            normal_map: Some(n_planks),
            ..Default::default()
        },
        Material {
            name: "tronco",
            albedo: [0.90, 0.06, 0.00, 0.00],
            specular: 8.0,
            diffuse: Vec3::new(0.36, 0.25, 0.15),
            texture: Some(t_log),
            normal_map: Some(n_log),
            ..Default::default()
        },
        Material {
            name: "hojas",
            albedo: [0.85, 0.04, 0.00, 0.00],
            specular: 4.0,
            diffuse: Vec3::new(0.18, 0.40, 0.14),
            texture: Some(t_leaves),
            ..Default::default()
        },
        Material {
            name: "agua",
            // Mas reflexion que el vidrio y menos transmision: al atardecer el
            // agua devuelve el naranja del cielo, y ese reflejo es lo que la
            // hace legible como agua en vez de como un hueco oscuro. El canal
            // difuso deja de ser testimonial una vez hay textura: sin el, la
            // superficie solo mostraba reflejos de una escena oscura y el
            // estanque se leia negro.
            albedo: [0.30, 0.60, 0.26, 0.50],
            specular: 180.0,
            refractive_index: 1.33,
            diffuse: Vec3::new(0.55, 0.78, 0.88),
            texture: Some(t_water),
            ..Default::default()
        },
        Material {
            name: "lava",
            // Difuso muy bajo a proposito: la lava se ilumina sola, y si ademas
            // respondiera a las luces que ella misma proyecta se lavaria hasta
            // quedar de un amarillo blancuzco plano.
            albedo: [0.12, 0.08, 0.00, 0.00],
            specular: 4.0,
            // Brilla sola: es la fuente de luz principal de la mitad derecha
            // del diorama. El valor pasa de 1, que es lo que el tone mapping
            // comprime; con la proporcion R:G:B tan separada el naranja
            // sobrevive a la compresion en vez de irse a blanco.
            emission: Vec3::new(1.70, 0.30, 0.035),
            diffuse: Vec3::new(0.95, 0.30, 0.06),
            texture: Some(t_lava),
            normal_map: Some(n_lava),
            emission_map: Some(t_lava),
            ..Default::default()
        },
        Material {
            name: "obsidiana",
            // Piedra negra pulida, no espejo de plata.
            //
            // El reflejo se suma sin teñirse con el color de la superficie, asi
            // que con un peso alto un material negro devolvia la hierba y el
            // cielo a brillo completo y las columnas se leian como paneles de
            // agua. Ademas 0.34 es irreal: un dielectrico pulido refleja en
            // torno al 5% de frente. Con 0.12 y un especular estrecho manda la
            // textura negra, la superficie se ve pulida, y lo poco que refleja
            // sigue viendose porque lo que tiene enfrente es lava, que brilla.
            albedo: [0.60, 0.35, 0.12, 0.00],
            specular: 320.0,
                        diffuse: Vec3::new(0.09, 0.07, 0.13),
            texture: Some(t_obsidian),
            ..Default::default()
        },
        Material {
            name: "cristal",
            albedo: [0.05, 0.50, 0.10, 0.85],
            specular: 250.0,
            refractive_index: 1.50,
            diffuse: Vec3::new(0.88, 0.94, 0.92),
            texture: Some(t_glass),
            ..Default::default()
        },
        Material {
            name: "piedra luminosa",
            albedo: [0.45, 0.10, 0.00, 0.00],
            specular: 12.0,
            // La textura hace tambien de mascara de emision: brillan las vetas
            // y la roca que las separa no. Es lo que distingue una piedra
            // luminosa de un cubo amarillo encendido.
            emission: Vec3::new(2.60, 1.65, 0.45),
            emission_map: Some(t_glowstone),
            texture: Some(t_glowstone),
            diffuse: Vec3::new(0.90, 0.78, 0.40),
            ..Default::default()
        },
        Material {
            name: "arena",
            albedo: [0.95, 0.03, 0.00, 0.00],
            specular: 4.0,
            diffuse: Vec3::new(0.78, 0.70, 0.50),
            texture: Some(t_sand),
            normal_map: Some(n_sand),
            ..Default::default()
        },
    ])
}

// ---------------------------------------------------------------------------
// Trazado del diorama
// ---------------------------------------------------------------------------

/// Semiejes de la isla. La forma no es un rectangulo sino un "squircle"
/// (`|x/a|^3 + |z/b|^3 <= 1`): tiene las esquinas redondeadas, que se ve mucho
/// mas natural que un bloque cuadrado, sin necesidad de dibujar el contorno a
/// mano.
const ISLAND_RX: f32 = 12.0;
const ISLAND_RZ: f32 = 10.0;

/// Profundidad a la que llega la roca de la isla.
const BEDROCK_Y: i32 = -5;

// Caja de la cabaña (extremos incluidos).
const CABIN_X: (i32, i32) = (-9, -3);
const CABIN_Z: (i32, i32) = (-7, -2);

// Estanque, franja de obsidiana y lago de lava.
//
// Van uno al lado del otro a lo ancho, no uno detras del otro: como la camara
// orbita, si estuvieran alineados en profundidad habria angulos desde los que
// uno taparia completamente al otro. Asi los tres se ven juntos desde el frente
// y desde los dos costados.
const POND_X: (i32, i32) = (-6, 0);
const POND_Z: (i32, i32) = (1, 6);
const OBSIDIAN_X: (i32, i32) = (1, 2);
const OBSIDIAN_Z: (i32, i32) = (1, 7);
const LAVA_X: (i32, i32) = (3, 9);
const LAVA_Z: (i32, i32) = (1, 6);

#[inline]
fn inside_island(x: i32, z: i32) -> bool {
    let fx = (x as f32 + 0.5) / ISLAND_RX;
    let fz = (z as f32 + 0.5) / ISLAND_RZ;
    fx.abs().powi(3) + fz.abs().powi(3) <= 1.0
}

/// `true` si la celda y todo lo que la rodea a `margin` de distancia siguen
/// dentro de la isla.
///
/// Se usa para que el lago de lava no llegue al borde: si una celda de lava
/// quedara en la orilla, el bloque de al lado no existiria y se veria el cielo
/// por el agujero, ademas de que la lava se derramaria por el acantilado.
#[inline]
fn inside_island_margin(x: i32, z: i32, margin: i32) -> bool {
    for dx in -margin..=margin {
        for dz in -margin..=margin {
            if !inside_island(x + dx, z + dz) {
                return false;
            }
        }
    }
    true
}

#[inline]
fn in_box(x: i32, z: i32, xs: (i32, i32), zs: (i32, i32)) -> bool {
    x >= xs.0 && x <= xs.1 && z >= zs.0 && z <= zs.1
}

/// Zonas que tienen que quedar perfectamente planas a la altura 0: donde se
/// apoyan la cabaña, el agua, la lava y el camino.
fn is_flat_zone(x: i32, z: i32) -> bool {
    in_box(x, z, (CABIN_X.0 - 1, CABIN_X.1 + 1), (CABIN_Z.0 - 1, CABIN_Z.1 + 1))
        || in_box(x, z, (POND_X.0 - 1, POND_X.1 + 1), (POND_Z.0 - 1, POND_Z.1 + 1))
        || in_box(x, z, OBSIDIAN_X, OBSIDIAN_Z)
        || in_box(x, z, (LAVA_X.0 - 1, LAVA_X.1 + 1), (LAVA_Z.0 - 1, LAVA_Z.1 + 1))
        || in_box(x, z, (-7, 2), (-1, 0))
}

/// Ruido de valor escrito a mano (sin librerias): se hashean las esquinas de
/// una rejilla gruesa y se interpola suavemente entre ellas.
///
/// Se usa solo para ondular un poco el terreno; con altura constante la isla se
/// ve como una mesa.
fn value_noise(x: f32, z: f32) -> f32 {
    #[inline]
    fn hash(ix: i32, iz: i32) -> f32 {
        // Mezcla entera tipo xorshift. Determinista y sin estado.
        let mut h = (ix as u32).wrapping_mul(0x27D4_EB2D) ^ (iz as u32).wrapping_mul(0x1656_67B1);
        h ^= h >> 15;
        h = h.wrapping_mul(0x2C1B_3C6D);
        h ^= h >> 12;
        h = h.wrapping_mul(0x2974_5B7D);
        h ^= h >> 16;
        (h >> 8) as f32 / (1u32 << 24) as f32
    }

    let (x0, z0) = (x.floor(), z.floor());
    let (fx, fz) = (x - x0, z - z0);
    // Curva suave: quita la apariencia de rombos que deja la interpolacion
    // lineal pura.
    let (ux, uz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));

    let (ix, iz) = (x0 as i32, z0 as i32);
    let c00 = hash(ix, iz);
    let c10 = hash(ix + 1, iz);
    let c01 = hash(ix, iz + 1);
    let c11 = hash(ix + 1, iz + 1);

    let a = c00 + (c10 - c00) * ux;
    let b = c01 + (c11 - c01) * ux;
    a + (b - a) * uz
}

/// Altura del bloque mas alto de terreno en esa columna.
fn terrain_top(x: i32, z: i32) -> i32 {
    if is_flat_zone(x, z) {
        return 0;
    }
    // Escala 1/7: colinas de unas siete celdas de ancho.
    let n = value_noise(x as f32 / 7.0, z as f32 / 7.0);

    // Cerca del borde de la isla el terreno baja, para que no termine en un
    // acantilado recto por todos lados.
    let fx = (x as f32 + 0.5) / ISLAND_RX;
    let fz = (z as f32 + 0.5) / ISLAND_RZ;
    let edge = 1.0 - (fx.abs().powi(3) + fz.abs().powi(3)).clamp(0.0, 1.0);

    ((n * 3.4 * edge.powf(0.45)) - 0.45).round().clamp(0.0, 3.0) as i32
}

/// Construye el diorama completo.
pub fn build() -> Result<Scene, String> {
    let mut textures = TextureManager::new();
    let materials = build_materials(&mut textures)?;
    let mut world = VoxelWorld::new(BEDROCK_Y);

    carve_island(&mut world);
    raise_outcrop(&mut world);
    carve_water_and_lava(&mut world);
    build_path(&mut world);
    build_cabin(&mut world);
    build_pier(&mut world);
    plant_trees(&mut world);

    let lamparas = glowstone_blocks();
    for &(x, y, z) in &lamparas {
        world.set(x, y, z, GLOWSTONE);
    }

    let voxels = world.len();
    let (cubes, culled) = world.into_cubes(&materials);

    let lights = build_lights(&lamparas);

    let grid = Grid::build(&cubes);

    println!(
        "diorama: {} bloques -> {} cubos visibles ({} enterrados descartados)",
        voxels,
        cubes.len(),
        culled
    );
    println!(
        "rejilla: {:?} celdas ({} en total), ocupacion media {:.2} cubos por celda",
        grid.dims(),
        grid.cell_count(),
        grid.average_occupancy()
    );

    println!("\nmateriales:");
    println!(
        "  {:<10} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}  {}",
        "nombre", "dif", "spec", "refl", "refr", "exp", "ior", "extras"
    );
    for material in &materials {
        let mut extras = Vec::new();
        if material.emission != Vec3::zeros() {
            extras.push("emisivo");
        }
        if material.texture.is_some() {
            extras.push("textura");
        }
        if material.normal_map.is_some() {
            extras.push("normal map");
        }
        println!(
            "  {:<10} {:>6.2} {:>6.2} {:>6.2} {:>6.2} {:>6.0} {:>6.2}  {}",
            material.name,
            material.albedo[0],
            material.albedo[1],
            material.albedo[2],
            material.albedo[3],
            material.specular,
            material.refractive_index,
            extras.join(", ")
        );
    }
    println!();

    Ok(Scene {
        cubes,
        materials,
        lights,
        textures,
        // Relleno azulado: es la luz del cielo del atardecer rebotando. Que sea
        // frio hace que el naranja de la lava resalte por contraste.
        ambient: Vec3::new(0.070, 0.088, 0.140),
        sky_top: Vec3::new(0.06, 0.10, 0.30),
        sky_horizon: Vec3::new(0.95, 0.42, 0.20),
        sky_bottom: Vec3::new(0.10, 0.08, 0.10),
        focus: Vec3::new(-1.0, 1.5, 0.0),
        grid,
    })
}

/// Isla: pasto en la superficie y roca hasta el fondo.
fn carve_island(world: &mut VoxelWorld) {
    let rx = ISLAND_RX.ceil() as i32;
    let rz = ISLAND_RZ.ceil() as i32;

    for x in -rx..=rx {
        for z in -rz..=rz {
            if !inside_island(x, z) {
                continue;
            }
            let top = terrain_top(x, z);
            world.set(x, top, z, GRASS);
            world.set(x, top - 1, z, STONE);
            world.set(x, top - 2, z, STONE);
            for y in BEDROCK_Y..=(top - 3) {
                world.set(x, y, z, STONE);
            }
        }
    }
}

/// Afloramiento de roca al fondo de la isla.
///
/// Rompe la silueta, que sin el es un disco plano, y le da al arbol de la
/// izquierda un sitio elevado desde donde proyectar sombra sobre la cabaña.
fn raise_outcrop(world: &mut VoxelWorld) {
    for x in -12..=-7 {
        for z in 2..=8 {
            if !inside_island(x, z) {
                continue;
            }
            // Altura en forma de loma: maxima en el centro del afloramiento y
            // bajando hacia los bordes.
            let dx = (x + 10) as f32 / 3.0;
            let dz = (z - 5) as f32 / 3.5;
            let bump = (1.0 - (dx * dx + dz * dz)).max(0.0);
            let top = (bump * 3.0).round() as i32;
            if top <= 0 {
                continue;
            }
            let base = terrain_top(x, z);
            let cima = base + top;

            // Roca en el cuerpo de la loma y pasto en la cima.
            //
            // Sin la capa de pasto, el afloramiento salia de la hierba como un
            // pegote de roca oscura sin ninguna transicion: se leia como un
            // monton de carbon, no como terreno. Con la cima cubierta, la roca
            // solo asoma en las caras inclinadas, que es donde asoma de verdad.
            for y in base..cima {
                world.set(x, y, z, STONE);
            }
            world.set(x, cima, z, GRASS);
        }
    }
}

/// Excava el estanque y el lago de lava, y levanta la franja de obsidiana que
/// los separa.
///
/// La obsidiana esta ahi por una razon, no de adorno: es lo que aparece cuando
/// la lava toca el agua. Ademas es el unico material reflectante del diorama y
/// al estar justo entre los dos refleja la lava, que es lo que hace que el
/// reflejo se note en vez de ser un espejo de cielo vacio.
fn carve_water_and_lava(world: &mut VoxelWorld) {
    // Estanque: dos bloques de agua sobre un fondo de adoquin. El fondo se ve
    // a traves del agua, deformado por la refraccion.
    for x in POND_X.0..=POND_X.1 {
        for z in POND_Z.0..=POND_Z.1 {
            // Fondo de arena clara con piedras sueltas encima. El fondo
            // importa por dos razones: tiene que ser claro para que el estanque
            // no se lea como un agujero negro, y tiene que tener un patron
            // reconocible, porque sin lineas rectas debajo la refraccion no se
            // aprecia. Solo dos bloques de profundidad, para que le llegue luz.
            world.set(x, -2, z, SAND);
            if (x * 3 + z * 5).rem_euclid(6) == 0 {
                world.set(x, -1, z, STONE);
            } else {
                world.set(x, -1, z, SAND);
            }
            world.set(x, 0, z, WATER);
        }
    }

    // Orilla de arena alrededor del estanque.
    for x in (POND_X.0 - 1)..=(POND_X.1 + 1) {
        for z in (POND_Z.0 - 1)..=(POND_Z.1 + 1) {
            if in_box(x, z, POND_X, POND_Z) || !inside_island(x, z) {
                continue;
            }
            world.set(x, 0, z, SAND);
        }
    }

    // Franja de obsidiana entre el agua y la lava.
    for x in OBSIDIAN_X.0..=OBSIDIAN_X.1 {
        for z in OBSIDIAN_Z.0..=OBSIDIAN_Z.1 {
            if !inside_island(x, z) {
                continue;
            }
            world.set(x, 0, z, OBSIDIAN);
            world.set(x, -1, z, OBSIDIAN);
        }
    }

    // Lago de lava, a ras de suelo y rodeado de un reborde de obsidiana.
    //
    // El reborde no es decorativo: sin el, una celda de lava en la orilla de la
    // isla dejaria un agujero por el que se veria el cielo. El margen de 1
    // garantiza que toda celda de lava tenga vecinos, y las celdas del anillo
    // que queda alrededor se rellenan de obsidiana, que ademas es donde el
    // reflejo naranja se ve mejor.
    let pool: Vec<(i32, i32)> = (LAVA_X.0..=LAVA_X.1)
        .flat_map(|x| (LAVA_Z.0..=LAVA_Z.1).map(move |z| (x, z)))
        .filter(|&(x, z)| inside_island_margin(x, z, 1))
        .collect();

    for &(x, z) in &pool {
        world.set(x, -2, z, OBSIDIAN);
        world.set(x, -1, z, LAVA);
        world.set(x, 0, z, LAVA);
    }

    // Anillo: las celdas pegadas al lago que no son lago.
    //
    // El borde es de adoquin y no de obsidiana. Como superficie plana, la
    // obsidiana refleja hacia arriba, o sea el cielo, que al atardecer es azul:
    // un anillo de obsidiana de este tamaño rodeaba la lava de una banda azul
    // que se leia como un foso de agua. El adoquin ahi se lee como un bordillo
    // de piedra, que es lo que un borde de lava deberia parecer, y deja la
    // obsidiana solo donde su reflejo si aporta: las columnas verticales.
    for &(x, z) in &pool {
        for dx in -1..=1 {
            for dz in -1..=1 {
                let (nx, nz) = (x + dx, z + dz);
                if pool.contains(&(nx, nz)) || !inside_island(nx, nz) {
                    continue;
                }
                world.set(nx, 0, nz, COBBLE);
                world.set(nx, -1, nz, OBSIDIAN);
            }
        }
    }

    // Columnas de obsidiana entre el agua y la lava.
    //
    // Son la razon de ser del material reflectante. Una superficie de obsidiana
    // plana en el suelo refleja hacia arriba, o sea el cielo, y el reflejo pasa
    // desapercibido. Estas columnas tienen caras **verticales** mirando al lago,
    // asi que reflejan la lava: franjas naranjas moviendose sobre cristal negro
    // conforme gira la camara. Narrativamente son lo que queda donde la lava
    // toco el agua.
    for (x, z, height) in [
        (1, 1, 2),
        (2, 3, 4),
        (1, 5, 3),
        (2, 7, 2),
        (2, -1, 3),
        (1, 8, 2),
    ] {
        if !inside_island(x, z) {
            continue;
        }
        for y in 0..height {
            world.set(x, y, z, OBSIDIAN);
        }
    }
}

/// Camino de adoquin de la puerta de la cabaña a la orilla del estanque.
fn build_path(world: &mut VoxelWorld) {
    // Tramo que baja de la puerta al borde del agua.
    for z in -1..=0 {
        world.set(-6, 0, z, COBBLE);
        world.set(-5, 0, z, COBBLE);
    }
    // Andador a lo largo de la orilla, de punta a punta, que conecta el
    // estanque con la franja de obsidiana.
    for x in -7..=2 {
        world.set(x, 0, 0, COBBLE);
    }
}

/// La cabaña: tablones, esquinas de tronco, ventanas de cristal, puerta abierta
/// y techo a dos aguas.
fn build_cabin(world: &mut VoxelWorld) {
    let (x0, x1) = CABIN_X;
    let (z0, z1) = CABIN_Z;

    // Piso y paredes. El piso va sobre el pasto, las paredes cuatro bloques.
    world.fill((x0, x1), (1, 1), (z0, z1), PLANKS);
    for y in 1..=4 {
        for x in x0..=x1 {
            world.set(x, y, z0, PLANKS);
            world.set(x, y, z1, PLANKS);
        }
        for z in z0..=z1 {
            world.set(x0, y, z, PLANKS);
            world.set(x1, y, z, PLANKS);
        }
    }

    // Troncos en las cuatro esquinas: rompen la monotonia de la pared y son un
    // material distinto al de los tablones.
    for y in 1..=4 {
        world.set(x0, y, z0, LOG);
        world.set(x1, y, z0, LOG);
        world.set(x0, y, z1, LOG);
        world.set(x1, y, z1, LOG);
    }

    // Ventanas de cristal en la pared del frente (la que mira al estanque) y
    // una lateral, a la altura de los ojos.
    for y in 2..=3 {
        world.set(-8, y, z1, GLASS);
        world.set(-7, y, z1, GLASS);
        world.set(-4, y, z1, GLASS);
        world.set(x0, y, -5, GLASS);
        world.set(x0, y, -4, GLASS);
    }

    // Puerta: un hueco de dos bloques en la pared del frente.
    world.clear(-6, 1, z1);
    world.clear(-6, 2, z1);
    world.clear(-5, 1, z1);
    world.clear(-5, 2, z1);

    // Lampara dentro de la cabaña. No se ve directamente, pero su luz sale por
    // la ventana: es lo que hace que el cristal se lea como cristal y no como
    // un agujero.
    world.set(-7, 3, -5, GLOWSTONE);

    // Ventana tambien en la pared de atras. No es simetria por simetria: el
    // diorama gira, y sin ella la cabaña vista desde atras es una losa lisa.
    for y in 2..=3 {
        world.set(-7, y, z0, GLASS);
        world.set(-6, y, z0, GLASS);
    }

    // Techo a dos aguas: tres escalones que se van cerrando hacia el centro,
    // con un bloque de alero sobresaliendo por cada lado.
    let roof = [
        (5, (x0 - 1, x1 + 1), (z0 - 1, z1 + 1)),
        (6, (x0 - 1, x1 + 1), (z0, z1)),
        (7, (x0 - 1, x1 + 1), (z0 + 1, z1 - 1)),
    ];
    for (y, xs, zs) in roof {
        world.fill(xs, (y, y), zs, PLANKS);
    }

    // Chimenea de piedra que atraviesa el techo, con brasas encendidas arriba.
    //
    // Cumple dos funciones: rompe la silueta del techo, que desde atras era una
    // losa rectangular, y pone una fuente de luz calida en lo alto que se ve
    // desde cualquier angulo de la orbita.
    let (cx, cz) = (-8, -6);
    for y in 1..=8 {
        world.set(cx, y, cz, COBBLE);
    }
}

/// Muelle de madera sobre el estanque.
///
/// Una pasarela recta que sale de la orilla, justo donde termina el camino de
/// adoquin, y avanza sobre el agua. Antes era una losa de 5x2 flotando en medio
/// del estanque, sin tocar tierra: parecia una tabla suelta, no un muelle.
///
/// Tres cosas lo hacen leerse como muelle y no como suelo:
///
/// - **Arranca en la orilla.** Una pasarela que empieza en el agua no se
///   entiende; hay que ver de donde sale.
/// - **Los pilotes bajan hasta la arena.** Se ven a traves del agua y explican
///   que sostiene la pasarela. Sin ellos la madera parece flotar.
/// - **Se detiene antes de la orilla opuesta.** Cruzarlo de lado a lado lo
///   convertiria en un puente; un muelle termina en agua abierta.
fn build_pier(world: &mut VoxelWorld) {
    // Pasarela recta de dos bloques de ancho, desde la orilla hacia el agua.
    // Se detiene antes de la orilla opuesta: un tablero que cruza el estanque
    // de lado a lado es un puente, y lo que se busca es un muelle, que tiene
    // que terminar en agua abierta.
    for x in -4..=-3 {
        for z in 1..=4 {
            world.set(x, 1, z, PLANKS);
        }
    }

    // Pilotes: del nivel del agua hasta hundirse en la arena del fondo.
    for (x, z) in [(-4, 2), (-3, 2), (-4, 4), (-3, 4)] {
        for y in -1..=0 {
            world.set(x, y, z, LOG);
        }
    }
}

/// Arboles: tronco de cuatro bloques y copa de hojas sin las esquinas, para que
/// la silueta no se vea cuadrada.
///
/// Las posiciones importan: esta funcion corre despues de levantar la cabaña y
/// los bloques se sobrescriben, asi que una copa que alcance el tejado se lo
/// come. El arbol de la izquierda se planta sobre el afloramiento rocoso, que
/// ademas lo aleja de la cabaña y le da altura para proyectar sombra sobre ella.
fn plant_trees(world: &mut VoxelWorld) {
    for (x, z) in [(-11, 5), (10, -4), (-4, 9)] {
        let base = terrain_top(x, z) + 1;

        for y in base..base + 4 {
            world.set(x, y, z, LOG);
        }

        let crown = base + 3;
        for dy in 0..3 {
            let radius: i32 = if dy == 2 { 1 } else { 2 };
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    // Se recortan las esquinas del cubo de hojas.
                    if dx.abs() == radius && dz.abs() == radius {
                        continue;
                    }
                    // No tapar el tronco con hojas en su propia columna.
                    if dx == 0 && dz == 0 && dy < 2 {
                        continue;
                    }
                    world.set(x + dx, crown + dy, z + dz, LEAVES);
                }
            }
        }
        world.set(x, crown + 3, z, LEAVES);
    }
}

/// Celdas donde va un bloque de piedra luminosa.
///
/// Antes eran piezas pequeñas de 0.28 x 0.5 colocadas fuera de la rejilla, a
/// modo de antorchas. No funcionaba: al no tener textura salian como tubitos
/// amarillos lisos, el unico material de la escena sin detalle, y su forma
/// alargada desentonaba entre puros cubos. Como bloque entero encajan con el
/// resto y pueden llevar textura, que es lo que les da las vetas encendidas.
fn glowstone_blocks() -> Vec<(i32, i32, i32)> {
    vec![
        // A los lados de la puerta, apoyadas en el suelo.
        (-7, 1, -1),
        (-4, 1, -1),
    ]
}

/// Las luces de la escena.
///
/// El atardecer se arma con tres grupos:
///
/// 1. **El sol**, bajo y naranja, sin atenuacion: da la direccion general de
///    las sombras y la luz rasante que hace visible el relieve del adoquin.
/// 2. **La lava**, varias luces repartidas sobre el lago para que la iluminacion
///    no salga de un solo punto y las sombras no se vean como de reflector.
/// 3. **Las antorchas**, una por cada cubo emisivo, para que ademas de brillar
///    alumbren lo que tienen alrededor.
fn build_lights(lamparas: &[(i32, i32, i32)]) -> Vec<Light> {
    let mut lights = vec![Light::sun(
        // Lejos y bajo: rayos rasantes y sombras largas, pero no tanto como
        // para que las caras horizontales se queden sin luz (con el sol casi en
        // el horizonte el techo y el pasto reciben `n . l` casi cero y el
        // diorama se ve iluminado solo de costado).
        Vec3::new(40.0, 23.0, 26.0),
        Vec3::new(1.00, 0.60, 0.33),
        1.05,
    )];

    // Luz de relleno, fria y desde el lado opuesto al sol.
    //
    // Hace falta porque el diorama **gira**: con una sola fuente lateral, media
    // vuelta de la camara transcurre mirando la cara en sombra y la escena se
    // vuelve una silueta negra. El relleno representa la luz azul del cielo que
    // rebota por el lado opuesto al atardecer, y ademas separa por color lo
    // iluminado (naranja) de lo que esta en sombra (azul), que es lo que le da
    // profundidad a la imagen.
    lights.push(Light::sun(
        Vec3::new(-34.0, 20.0, -28.0),
        Vec3::new(0.34, 0.46, 0.78),
        0.55,
    ));

    // Tres focos sobre el lago de lava.
    let lava_centre_z = (LAVA_Z.0 + LAVA_Z.1) as f32 * 0.5;
    for (i, x) in [2.0f32, 4.5, 6.5].iter().enumerate() {
        let z = lava_centre_z + (i as f32 - 1.0) * 1.8;
        lights.push(Light::new(
            Vec3::new(*x, 1.6, z),
            Vec3::new(1.00, 0.33, 0.07),
            26.0,
        ));
    }

    // Una luz en el centro de cada piedra luminosa. Un bloque que brilla pero
    // no alumbra lo que tiene alrededor se lee como una pegatina.
    for &(x, y, z) in lamparas {
        lights.push(Light::new(
            Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5),
            Vec3::new(1.00, 0.62, 0.28),
            13.0,
        ));
    }

    // La lampara de dentro de la cabaña, la que sale por la ventana.
    lights.push(Light::new(
        Vec3::new(-6.5, 3.6, -4.5),
        Vec3::new(1.00, 0.70, 0.38),
        9.0,
    ));

    lights
}
