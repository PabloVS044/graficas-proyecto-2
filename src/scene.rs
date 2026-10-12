//! La escena: geometria, materiales, luces y cielo.
//!
//! `Scene` se construye una sola vez al arrancar y despues solo se lee, por eso
//! se puede compartir entre todos los hilos del render con una simple `&Scene`.

use crate::accel::Grid;
use crate::cube::Cube;
use crate::light::Light;
use crate::material::{Material, MaterialId};
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::skybox::Skybox;
use crate::terrain::Terrain;
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

    /// El cielo: lo que ve un rayo que no choca con nada.
    pub skybox: Skybox,

    /// Punto alrededor del cual conviene orbitar la camara.
    pub focus: Vec3,

    /// Segundos transcurridos. Mueve las texturas de los liquidos.
    pub time: f32,

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

    /// Color del cielo para un rayo que se escapa de la escena. Los reflejos
    /// y la refraccion terminan aqui tambien, asi que el agua y la obsidiana
    /// reflejan las nubes sin codigo extra.
    #[inline]
    pub fn sky(&self, direction: &Vec3) -> Vec3 {
        self.skybox.sample(&self.textures, direction)
    }
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
pub const BIRCH_LOG: MaterialId = 12;
pub const BIRCH_LEAVES: MaterialId = 13;
pub const SPRUCE_LOG: MaterialId = 14;
pub const SPRUCE_LEAVES: MaterialId = 15;
pub const DIRT: MaterialId = 16;
pub const COAL_ORE: MaterialId = 17;
pub const IRON_ORE: MaterialId = 18;
pub const DIAMOND_ORE: MaterialId = 19;

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
    let t_grass_side = color(textures, "grass_side")?;
    let t_dirt = color(textures, "dirt")?;
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
    let t_birch_log = color(textures, "birch_log")?;
    let t_birch_leaves = color(textures, "birch_leaves")?;
    let t_spruce_log = color(textures, "spruce_log")?;
    let t_spruce_leaves = color(textures, "spruce_leaves")?;

    let n_stone = normal(textures, "stone")?;
    let n_cobble = normal(textures, "cobblestone")?;
    let n_planks = normal(textures, "planks")?;
    let n_lava = normal(textures, "lava")?;
    let n_sand = normal(textures, "sand")?;
    let n_log = normal(textures, "log")?;
    let n_birch_log = normal(textures, "birch_log")?;
    let n_spruce_log = normal(textures, "spruce_log")?;
    let t_coal = color(textures, "coal_ore")?;
    let t_iron = color(textures, "iron_ore")?;
    let t_diamond = color(textures, "diamond_ore")?;
    let n_coal = normal(textures, "coal_ore")?;
    let n_iron = normal(textures, "iron_ore")?;
    let n_diamond = normal(textures, "diamond_ore")?;

    Ok(vec![
        Material {
            name: "pasto",
            albedo: [0.95, 0.02, 0.00, 0.00],
            specular: 3.0,
            diffuse: Vec3::new(0.29, 0.52, 0.20),
            texture: Some(t_grass_side),
            texture_top: Some(t_grass),
            texture_bottom: Some(t_dirt),
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
            // Corre hacia +v: en la superficie del estanque va hacia el canal
            // de la cascada, y en las caras verticales de la cascada, hacia
            // abajo.
            uv_scroll: [0.0, 0.6],
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
            // Mucho mas lenta que el agua: la lava se arrastra.
            uv_scroll: [0.03, 0.08],
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
        Material {
            name: "abedul",
            albedo: [0.90, 0.06, 0.00, 0.00],
            specular: 8.0,
            diffuse: Vec3::new(0.85, 0.83, 0.78),
            texture: Some(t_birch_log),
            normal_map: Some(n_birch_log),
            ..Default::default()
        },
        Material {
            name: "hojas abedul",
            albedo: [0.85, 0.04, 0.00, 0.00],
            specular: 4.0,
            diffuse: Vec3::new(0.30, 0.48, 0.20),
            texture: Some(t_birch_leaves),
            ..Default::default()
        },
        Material {
            name: "pino",
            albedo: [0.90, 0.06, 0.00, 0.00],
            specular: 8.0,
            diffuse: Vec3::new(0.25, 0.16, 0.09),
            texture: Some(t_spruce_log),
            normal_map: Some(n_spruce_log),
            ..Default::default()
        },
        Material {
            name: "hojas pino",
            albedo: [0.85, 0.04, 0.00, 0.00],
            specular: 4.0,
            diffuse: Vec3::new(0.12, 0.25, 0.12),
            texture: Some(t_spruce_leaves),
            ..Default::default()
        },
        Material {
            name: "tierra",
            // Solo se ve donde una colina corta el terreno en vertical: el
            // escalon deja a la vista la capa de debajo del pasto.
            albedo: [0.92, 0.02, 0.00, 0.00],
            specular: 2.0,
            diffuse: Vec3::new(0.40, 0.28, 0.18),
            texture: Some(t_dirt),
            ..Default::default()
        },
        Material {
            name: "carbon",
            albedo: [0.88, 0.06, 0.00, 0.00],
            specular: 10.0,
            diffuse: Vec3::new(0.35, 0.35, 0.35),
            texture: Some(t_coal),
            normal_map: Some(n_coal),
            ..Default::default()
        },
        Material {
            name: "hierro",
            // Las vetas son metal: algo mas de brillo que la roca que las
            // rodea, con el reflejo concentrado.
            albedo: [0.85, 0.18, 0.00, 0.00],
            specular: 40.0,
            diffuse: Vec3::new(0.50, 0.45, 0.42),
            texture: Some(t_iron),
            normal_map: Some(n_iron),
            ..Default::default()
        },
        Material {
            name: "diamante",
            // Las gemas destellan: brillo muy concentrado y un poco de reflejo.
            albedo: [0.80, 0.40, 0.08, 0.00],
            specular: 120.0,
            diffuse: Vec3::new(0.45, 0.60, 0.62),
            texture: Some(t_diamond),
            normal_map: Some(n_diamond),
            ..Default::default()
        },
    ])
}

// ---------------------------------------------------------------------------
// Trazado del diorama
// ---------------------------------------------------------------------------

/// Semiejes de la isla. La forma no es un rectangulo sino un "squircle"
/// (`|x/a|^4 + |z/b|^4 <= 1`): tiene las esquinas redondeadas, que se ve mucho
/// mas natural que un bloque cuadrado, sin necesidad de dibujar el contorno a
/// mano. El exponente 4 (y no 3) llena mas las esquinas, que es donde cabe la
/// zona de colinas de `WILD_X` x `WILD_Z`.
const ISLAND_RX: f32 = 20.0;
const ISLAND_RZ: f32 = 16.0;

/// Profundidad a la que llega la roca de la isla.
const BEDROCK_Y: i32 = -5;

// Caja de la cabaña (extremos incluidos).
const CABIN_X: (i32, i32) = (-11, -5);
const CABIN_Z: (i32, i32) = (-7, -2);

// Estanque, franja de obsidiana y lago de lava.
//
// Van uno al lado del otro a lo ancho, no uno detras del otro: como la camara
// orbita, si estuvieran alineados en profundidad habria angulos desde los que
// uno taparia completamente al otro. Asi los tres se ven juntos desde el frente
// y desde los dos costados.
const POND_X: (i32, i32) = (-13, -6);
const POND_Z: (i32, i32) = (4, 10);
const OBSIDIAN_X: (i32, i32) = (-2, -1);
const OBSIDIAN_Z: (i32, i32) = (4, 11);
const LAVA_X: (i32, i32) = (1, 7);
const LAVA_Z: (i32, i32) = (4, 9);

/// Zona de 16x16 sin nada construido: aqui el terreno es puro ruido, sin
/// aplanar. La rubrica pide generacion procedural en al menos 16x16 cubos; el
/// resto de la isla tambien sale del ruido, pero tiene zonas aplanadas encima
/// para la cabaña, el agua y la lava. Lo construido se agrupo al frente y a la
/// izquierda justamente para dejar este cuadrante libre. La prueba
/// `wild_zone_is_procedural` comprueba que siga asi si se mueve algo.
const WILD_X: (i32, i32) = (0, 15);
const WILD_Z: (i32, i32) = (-13, 2);

/// Fila del andador de adoquin que corre a lo largo de la orilla.
const WALK_Z: i32 = 2;

#[inline]
fn inside_island(x: i32, z: i32) -> bool {
    let fx = (x as f32 + 0.5) / ISLAND_RX;
    let fz = (z as f32 + 0.5) / ISLAND_RZ;
    fx.abs().powi(4) + fz.abs().powi(4) <= 1.0
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

/// Columnas del canal por donde el estanque se desborda hacia la cascada.
const CHANNEL_X: (i32, i32) = (-8, -7);

/// Altura maxima de las colinas, en bloques sobre el nivel del agua.
const MAX_HILL: f32 = 6.0;

/// Ancho, en celdas, de la rampa entre una zona aplanada y el terreno libre.
const BLEND_CELLS: f32 = 4.0;

/// Zonas que tienen que quedar a la altura 0 porque algo se apoya en ellas: la
/// cabaña, el estanque, la lava, el camino y el canal de la cascada. Cada una
/// es una caja `(xs, zs)` con los extremos incluidos.
const FLAT_PADS: [((i32, i32), (i32, i32)); 6] = [
    ((CABIN_X.0 - 1, CABIN_X.1 + 1), (CABIN_Z.0 - 1, CABIN_Z.1 + 1)),
    ((POND_X.0 - 1, POND_X.1 + 1), (POND_Z.0 - 1, POND_Z.1 + 1)),
    (OBSIDIAN_X, OBSIDIAN_Z),
    ((LAVA_X.0 - 1, LAVA_X.1 + 1), (LAVA_Z.0 - 1, LAVA_Z.1 + 1)),
    ((POND_X.0 - 1, OBSIDIAN_X.1), (CABIN_Z.1 + 1, WALK_Z)),
    ((CHANNEL_X.0 - 1, CHANNEL_X.1 + 1), (POND_Z.1 + 1, 99)),
];

/// Distancia (en celdas, tipo ajedrez) de la columna a la zona aplanada mas
/// cercana. 0 si esta dentro de una.
fn pad_distance(x: i32, z: i32) -> i32 {
    FLAT_PADS
        .iter()
        .map(|&((x0, x1), (z0, z1))| {
            let dx = (x0 - x).max(x - x1).max(0);
            let dz = (z0 - z).max(z - z1).max(0);
            dx.max(dz)
        })
        .min()
        .unwrap_or(i32::MAX)
}

#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Altura del bloque mas alto de terreno en esa columna.
///
/// Tres pasos:
///
/// 1. **Ruido fractal** de tres octavas a escala 1/11: colinas de unas once
///    celdas de ancho con el contorno roto por las octavas finas.
/// 2. **Curva de altura**: el ruido se recorta abajo para que haya llano entre
///    colinas (con el ruido crudo toda la isla seria una ondulacion pareja) y
///    se eleva a 1.4 para que las cimas sean mas escarpadas que las faldas.
/// 3. **Aplanado suave** cerca de lo construido: la altura se multiplica por
///    un factor que vale 0 dentro de las zonas planas y sube a 1 en
///    `BLEND_CELLS` celdas. Asi las colinas bajan en rampa hasta la cabaña o el
///    estanque en vez de cortarse en un muro.
fn terrain_top(terrain: &Terrain, x: i32, z: i32) -> i32 {
    let distance = pad_distance(x, z);
    if distance == 0 {
        return 0;
    }

    let n = terrain.fbm(x as f32 / 11.0, z as f32 / 11.0, 3);
    let shaped = ((n - 0.30) / 0.45).clamp(0.0, 1.0).powf(1.4);

    // Cerca del borde de la isla el terreno baja un poco: una colina de seis
    // bloques justo en la orilla deja un muro sin transicion.
    let fx = (x as f32 + 0.5) / ISLAND_RX;
    let fz = (z as f32 + 0.5) / ISLAND_RZ;
    let edge = 1.0 - (fx.abs().powi(4) + fz.abs().powi(4)).clamp(0.0, 1.0);
    let edge = smoothstep(0.0, 0.30, edge);

    let blend = smoothstep(0.0, BLEND_CELLS, distance as f32);

    (shaped * MAX_HILL * edge * blend).round() as i32
}

/// Posicion de la luz principal del atardecer.
const SUN_POSITION: Vec3 = Vec3::new(40.0, 23.0, 26.0);

/// Punto alrededor del cual orbita la camara.
const FOCUS: Vec3 = Vec3::new(-1.0, 1.5, 0.0);

/// Semilla por defecto: la que da la composicion que se ve en el video.
const DEFAULT_SEED: u32 = 3;

/// Construye el diorama completo.
pub fn build() -> Result<Scene, String> {
    let mut textures = TextureManager::new();
    let materials = build_materials(&mut textures)?;
    let mut world = VoxelWorld::new(BEDROCK_Y);

    // Semilla del terreno. Se puede cambiar con la variable de entorno SEED
    // para comprobar que el terreno se genera y no esta dibujado a mano.
    let seed = std::env::var("SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SEED);
    let terrain = Terrain::new(seed);

    carve_island(&mut world, &terrain);
    let ores = place_ores(&mut world, &terrain);
    carve_water_and_lava(&mut world);
    build_path(&mut world);
    build_cabin(&mut world);
    build_pier(&mut world);
    build_waterfall(&mut world);
    let trees = plant_trees(&mut world, &terrain);

    let lamparas = glowstone_blocks();
    for &(x, y, z) in &lamparas {
        world.set(x, y, z, GLOWSTONE);
    }

    let voxels = world.len();
    let (cubes, culled) = world.into_cubes(&materials);

    let lights = build_lights(&lamparas);

    // El disco del sol va en la direccion de la luz principal, vista desde el
    // centro del diorama: asi las sombras salen del lado donde se ve el sol.
    let skybox = Skybox::load(
        &mut textures,
        "assets/skybox",
        SUN_POSITION - FOCUS,
        Vec3::new(14.0, 8.0, 4.0),
    )?;

    let grid = Grid::build(&cubes);

    let wild: Vec<i32> = (WILD_X.0..=WILD_X.1)
        .flat_map(|x| (WILD_Z.0..=WILD_Z.1).map(move |z| (x, z)))
        .map(|(x, z)| terrain_top(&terrain, x, z))
        .collect();
    println!("minerales: {ores} bloques");
    println!(
        "terreno: semilla {}, {} arboles, zona libre {}x{} con alturas {}..{}",
        terrain.seed(),
        trees,
        WILD_X.1 - WILD_X.0 + 1,
        WILD_Z.1 - WILD_Z.0 + 1,
        wild.iter().min().unwrap_or(&0),
        wild.iter().max().unwrap_or(&0),
    );
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
        skybox,
        focus: FOCUS,
        time: 0.0,
        grid,
    })
}

/// Isla: pasto en la superficie, dos capas de tierra y roca hasta el fondo,
/// como una columna de Minecraft. La tierra se ve donde una colina corta el
/// terreno en vertical y en el contorno del acantilado.
fn carve_island(world: &mut VoxelWorld, terrain: &Terrain) {
    let rx = ISLAND_RX.ceil() as i32;
    let rz = ISLAND_RZ.ceil() as i32;

    for x in -rx..=rx {
        for z in -rz..=rz {
            if !inside_island(x, z) {
                continue;
            }
            let top = terrain_top(terrain, x, z);
            world.set(x, top, z, GRASS);
            world.set(x, top - 1, z, DIRT);
            world.set(x, top - 2, z, DIRT);
            for y in BEDROCK_Y..=(top - 3) {
                world.set(x, y, z, STONE);
            }
        }
    }
}

/// Vetas de mineral en la roca, como en Minecraft: carbon comun, hierro menos
/// comun y diamante raro y solo en lo mas hondo.
///
/// Se reparten con el mismo hash del terreno, asi que cambian con la semilla.
/// Se siembran en toda la roca, no solo en la superficie: las que quedan
/// enterradas las descarta el `VoxelWorld` y no cuestan nada, y las que caen en
/// el acantilado son las que se ven.
fn place_ores(world: &mut VoxelWorld, terrain: &Terrain) -> usize {
    let rx = ISLAND_RX.ceil() as i32;
    let rz = ISLAND_RZ.ceil() as i32;
    let mut placed = 0;

    for x in -rx..=rx {
        for z in -rz..=rz {
            if !inside_island(x, z) {
                continue;
            }
            let top = terrain_top(terrain, x, z);
            for y in BEDROCK_Y..=(top - 3) {
                let roll = terrain.hash01(x, z, 200 + (y - BEDROCK_Y) as u32);
                let ore = if y <= BEDROCK_Y + 1 && roll < 0.025 {
                    DIAMOND_ORE
                } else if roll < 0.055 {
                    IRON_ORE
                } else if roll < 0.11 {
                    COAL_ORE
                } else {
                    continue;
                };
                world.set(x, y, z, ore);
                placed += 1;
            }
        }
    }
    placed
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
    let (ox, oz) = (OBSIDIAN_X.0, OBSIDIAN_Z.0);
    for (x, z, height) in [
        (ox, oz, 2),
        (ox + 1, oz + 2, 4),
        (ox, oz + 4, 3),
        (ox + 1, oz + 6, 2),
        (ox + 1, oz - 3, 3),
        (ox, oz + 7, 2),
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
    // Tramo que baja de la puerta al borde del agua. Las dos columnas son las
    // mismas que las de la puerta, para que el camino salga de ella y no de un
    // punto cualquiera de la fachada.
    let (x0, _) = CABIN_X;
    for z in (CABIN_Z.1 + 1)..=WALK_Z {
        world.set(x0 + 3, 0, z, COBBLE);
        world.set(x0 + 4, 0, z, COBBLE);
    }
    // Andador a lo largo de la orilla, de punta a punta, que conecta el
    // estanque con la franja de obsidiana.
    for x in (POND_X.0 - 1)..=OBSIDIAN_X.1 {
        world.set(x, 0, WALK_Z, COBBLE);
    }
}

/// La cabaña: tablones, esquinas de tronco, ventanas de cristal, puerta abierta
/// y techo a dos aguas.
fn build_cabin(world: &mut VoxelWorld) {
    let (x0, x1) = CABIN_X;
    let (z0, z1) = CABIN_Z;

    // Piso y paredes.
    //
    // El piso ocupa la misma capa que el pasto, no la de encima: asi se entra
    // sin escalon y el hueco de la puerta queda a la altura a la que se camina.
    // Un bloque mas arriba, la casa se leia como una tarima con paredes.
    world.fill((x0, x1), (0, 0), (z0, z1), PLANKS);
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
        world.set(x0 + 1, y, z1, GLASS);
        world.set(x0 + 2, y, z1, GLASS);
        world.set(x0 + 5, y, z1, GLASS);
        world.set(x0, y, z0 + 2, GLASS);
        world.set(x0, y, z0 + 3, GLASS);
    }

    // Puerta: un hueco de dos bloques en la pared del frente.
    world.clear(x0 + 3, 1, z1);
    world.clear(x0 + 3, 2, z1);
    world.clear(x0 + 4, 1, z1);
    world.clear(x0 + 4, 2, z1);

    // Lampara dentro de la cabaña. No se ve directamente, pero su luz sale por
    // la ventana: es lo que hace que el cristal se lea como cristal y no como
    // un agujero.
    world.set(x0 + 2, 3, z0 + 2, GLOWSTONE);

    // Ventana tambien en la pared de atras. No es simetria por simetria: el
    // diorama gira, y sin ella la cabaña vista desde atras es una losa lisa.
    for y in 2..=3 {
        world.set(x0 + 2, y, z0, GLASS);
        world.set(x0 + 3, y, z0, GLASS);
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

    // Chimenea de adoquin atravesando el faldon trasero del techo. Le da a la
    // silueta de la cabaña un punto vertical, que sin ella es un bloque con
    // tejado y nada mas.
    for y in 5..=9 {
        world.set(x1 - 1, y, z0 + 1, COBBLE);
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
    //
    // Va a la altura del suelo, no encima: la hierba y el camino se pisan a y=1
    // y la tarima ocupa esa misma capa, de modo que se sale de la puerta y se
    // entra al muelle sin escalon. Un bloque mas arriba se veia como una tarima
    // montada sobre el estanque en vez de como una continuacion del camino.
    //
    // Se detiene antes de la orilla opuesta: un tablero que cruza el estanque
    // de lado a lado es un puente, y lo que se busca es un muelle, que tiene
    // que terminar en agua abierta.
    //
    // Al quedar al ras desaparecen los pilotes: la capa de debajo es la arena
    // del fondo, asi que quedarian enterrados y no se verian desde ningun
    // angulo.
    for x in (POND_X.0 + 3)..=(POND_X.0 + 4) {
        for z in POND_Z.0..=(POND_Z.0 + 4) {
            world.set(x, 0, z, PLANKS);
        }
    }
}


/// Cascada: el estanque se desborda por un canal hacia el borde de la isla y
/// el agua cae por el acantilado hasta perderse por debajo de la roca.
///
/// Sale por el frente de la isla, el lado al que mira la cabaña, para que se
/// vea en la toma principal. La caida sigue mas alla del fondo de la isla:
/// una isla flotante con agua que cae al vacio es la imagen que se busca, y
/// cortarla a ras de la roca la haria ver como una mancha azul pegada a la
/// pared.
fn build_waterfall(world: &mut VoxelWorld) {
    // Canal: del estanque a la orilla. Se limpia lo que haya encima por si el
    // terreno ondula ahi, asi el agua pasa por una zanja en vez de enterrarse.
    for x in CHANNEL_X.0..=CHANNEL_X.1 {
        for z in (POND_Z.1 + 1)..=(ISLAND_RZ as i32) {
            if !inside_island(x, z) {
                continue;
            }
            for y in 1..=(MAX_HILL as i32 + 1) {
                world.clear(x, y, z);
            }
            world.set(x, 0, z, WATER);
            world.set(x, -1, z, SAND);
        }
    }

    // Caida: la columna justo afuera del borde, pegada al acantilado. Las dos
    // columnas terminan a distinta altura para que el final no sea un corte
    // recto.
    for (x, bottom) in [(CHANNEL_X.0, BEDROCK_Y - 5), (CHANNEL_X.1, BEDROCK_Y - 4)] {
        let mut z = POND_Z.1 + 1;
        while inside_island(x, z) {
            z += 1;
        }
        for y in bottom..=0 {
            world.set(x, y, z, WATER);
        }
    }
}

/// Arboles colocados de forma procedural.
///
/// La isla se divide en celdas de 6x6 y cada celda tira un dado: si sale,
/// prueba hasta tres posiciones al azar dentro de ella y planta el arbol en la
/// primera que sirva. Repartirlos por celdas, en vez de tirar el dado columna
/// por columna, evita que dos arboles queden pegados y que se formen claros
/// enormes; los reintentos evitan que una celda con un solo sitio malo se
/// quede vacia.
///
/// El tipo depende de la altura, como los biomas de Minecraft: pinos en las
/// cimas y robles o abedules en el llano. Se descartan las posiciones donde un
/// arbol estorbaria: junto a lo construido (la copa se comeria el tejado),
/// en pendientes fuertes (el tronco quedaria flotando sobre un escalon) y en
/// la orilla (media copa colgando sobre el vacio).
fn plant_trees(world: &mut VoxelWorld, terrain: &Terrain) -> usize {
    const CELL: i32 = 6;
    const DENSITY: f32 = 0.55;
    // En la zona de colinas hay menos: es donde se luce el terreno, y un
    // bosque cerrado lo tapaba por completo.
    const WILD_DENSITY: f32 = 0.30;
    const ATTEMPTS: u32 = 3;

    let rx = ISLAND_RX.ceil() as i32;
    let rz = ISLAND_RZ.ceil() as i32;
    let mut planted: Vec<(i32, i32)> = Vec::new();

    for cx in (-rx).div_euclid(CELL)..=rx.div_euclid(CELL) {
        for cz in (-rz).div_euclid(CELL)..=rz.div_euclid(CELL) {
            let (mid_x, mid_z) = (cx * CELL + CELL / 2, cz * CELL + CELL / 2);
            let density = if in_box(mid_x, mid_z, WILD_X, WILD_Z) {
                WILD_DENSITY
            } else {
                DENSITY
            };
            if terrain.hash01(cx, cz, 100) > density {
                continue;
            }
            let spot = (0..ATTEMPTS).find_map(|attempt| {
                let salt = 101 + attempt * 2;
                let x = cx * CELL + (terrain.hash01(cx, cz, salt) * CELL as f32) as i32;
                let z = cz * CELL + (terrain.hash01(cx, cz, salt + 1) * CELL as f32) as i32;
                // Dos celdas vecinas pueden elegir puntos pegados a su borde
                // comun; con menos de 3 de separacion una copa se come el
                // tronco del otro.
                let spaced = planted
                    .iter()
                    .all(|&(px, pz)| (px - x).abs().max((pz - z).abs()) >= 3);
                (spaced && tree_fits(terrain, x, z)).then_some((x, z))
            });
            let Some((x, z)) = spot else {
                continue;
            };
            let h = terrain_top(terrain, x, z);

            let pick = terrain.hash01(x, z, 103);
            if h >= 3 {
                plant_spruce(world, terrain, x, z, 5 + (pick * 2.0) as i32);
            } else if pick < 0.55 {
                plant_round_tree(world, terrain, x, z, 4, LOG, LEAVES);
            } else {
                plant_round_tree(world, terrain, x, z, 5, BIRCH_LOG, BIRCH_LEAVES);
            }
            planted.push((x, z));
        }
    }
    planted.len()
}

/// `true` si en la columna (x, z) cabe un arbol sin estorbar.
fn tree_fits(terrain: &Terrain, x: i32, z: i32) -> bool {
    // 4 de distancia a lo construido: con menos, desde algun angulo de la
    // orbita la copa tapa el estanque o la lava.
    if !inside_island_margin(x, z, 1) || pad_distance(x, z) < 4 {
        return false;
    }
    let h = terrain_top(terrain, x, z);
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .all(|(dx, dz)| (terrain_top(terrain, x + dx, z + dz) - h).abs() <= 1)
}

/// Arbol de copa redonda (roble o abedul): copa de hojas sin las esquinas,
/// para que la silueta no se vea cuadrada.
fn plant_round_tree(
    world: &mut VoxelWorld,
    terrain: &Terrain,
    x: i32,
    z: i32,
    trunk: i32,
    log: MaterialId,
    leaves: MaterialId,
) {
    let base = terrain_top(terrain, x, z) + 1;

    for y in base..base + trunk {
        world.set(x, y, z, log);
    }

    let crown = base + trunk - 1;
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
                world.set(x + dx, crown + dy, z + dz, leaves);
            }
        }
    }
    world.set(x, crown + 3, z, leaves);
}

/// Pino: tronco alto y copa en capas que se van cerrando hacia la punta, como
/// los de la taiga de Minecraft.
fn plant_spruce(world: &mut VoxelWorld, terrain: &Terrain, x: i32, z: i32, height: i32) {
    let base = terrain_top(terrain, x, z) + 1;
    let top = base + height;

    for y in base..top {
        world.set(x, y, z, SPRUCE_LOG);
    }

    // Capas de abajo hacia arriba, alternando anchas y angostas para que se
    // vean los escalones de la copa.
    let layers: [i32; 6] = [2, 1, 2, 1, 1, 0];
    let first = top - layers.len() as i32 + 1;
    for (i, &radius) in layers.iter().enumerate() {
        let y = first + i as i32;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if radius > 1 && dx.abs() == radius && dz.abs() == radius {
                    continue;
                }
                if dx == 0 && dz == 0 && y < top {
                    continue;
                }
                world.set(x + dx, y, z + dz, SPRUCE_LEAVES);
            }
        }
    }
    world.set(x, top + 1, z, SPRUCE_LEAVES);
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
        (CABIN_X.0 + 2, 1, CABIN_Z.1 + 1),
        (CABIN_X.0 + 5, 1, CABIN_Z.1 + 1),
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
        SUN_POSITION,
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
    let lava_x = LAVA_X.0 as f32;
    for (i, x) in [lava_x - 1.0, lava_x + 1.5, lava_x + 3.5].iter().enumerate() {
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
        Vec3::new(
            CABIN_X.0 as f32 + 0.5,
            3.6,
            CABIN_Z.0 as f32 + 2.5,
        ),
        Vec3::new(1.00, 0.70, 0.38),
        9.0,
    ));

    lights
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La zona libre mide al menos 16x16, cae entera dentro de la isla y no la
    /// toca ninguna zona aplanada: todas sus alturas salen del ruido.
    #[test]
    fn wild_zone_is_procedural() {
        assert!(WILD_X.1 - WILD_X.0 + 1 >= 16);
        assert!(WILD_Z.1 - WILD_Z.0 + 1 >= 16);
        for x in WILD_X.0..=WILD_X.1 {
            for z in WILD_Z.0..=WILD_Z.1 {
                assert!(inside_island(x, z), "({x}, {z}) queda fuera de la isla");
                assert!(pad_distance(x, z) > 0, "({x}, {z}) esta aplanada");
            }
        }
    }

    /// La zona libre tiene relieve de verdad con la semilla por defecto: si el
    /// ruido saliera plano ahi, la caracteristica no se veria.
    #[test]
    fn wild_zone_has_hills() {
        let terrain = Terrain::new(DEFAULT_SEED);
        let heights: Vec<i32> = (WILD_X.0..=WILD_X.1)
            .flat_map(|x| (WILD_Z.0..=WILD_Z.1).map(move |z| (x, z)))
            .map(|(x, z)| terrain_top(&terrain, x, z))
            .collect();
        let max = *heights.iter().max().unwrap();
        let distinct = heights.iter().collect::<std::collections::HashSet<_>>().len();
        assert!(max >= 4, "la colina mas alta mide {max}");
        assert!(distinct >= 4, "solo hay {distinct} alturas distintas");
    }
}
