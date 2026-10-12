//! Definicion de materiales.
//!
//! Un material describe como responde una superficie a la luz. Cada material
//! del diorama tiene su propia textura y sus propios parametros, de modo que
//! dos bloques distintos nunca reaccionan igual ante la misma lampara.

use crate::texture::TextureId;
use crate::vector::Vec3;

/// Indice de un material dentro de `Scene::materials`.
pub type MaterialId = usize;

/// Como se reparte la energia que llega a una superficie.
///
/// Los cuatro pesos de `albedo` corresponden a los cuatro canales que se suman
/// para formar el color final. Idealmente suman <= 1 para no crear energia de
/// la nada, aunque no se fuerza.
#[derive(Debug, Clone)]
pub struct Material {
    /// Nombre legible, solo para depurar y para el HUD.
    pub name: &'static str,

    /// `[difuso, especular, reflexion, refraccion]`.
    ///
    /// - difuso: cuanto del color de la textura se ve bajo luz directa.
    /// - especular: intensidad del brillo tipo Phong.
    /// - reflexion: cuanto del rayo se rebota como espejo.
    /// - refraccion: cuanto del rayo atraviesa el material.
    pub albedo: [f32; 4],

    /// Exponente de Phong. Valores altos = brillo pequeño y concentrado
    /// (superficie pulida); valores bajos = brillo amplio y suave (superficie
    /// rugosa).
    pub specular: f32,

    /// Indice de refraccion. 1.0 = vacio, 1.33 = agua, 1.5 = vidrio.
    /// Solo importa si `albedo[3] > 0`.
    pub refractive_index: f32,

    /// Luz que la superficie emite por si misma, sin depender de ninguna
    /// lampara. Es lo que hace que un bloque se vea encendido en la oscuridad.
    pub emission: Vec3,

    /// Color base cuando el material no tiene textura.
    pub diffuse: Vec3,

    /// Textura de color de los **costados**. Si es `None` se usa `diffuse`.
    pub texture: Option<TextureId>,

    /// Textura de la cara de arriba. Si es `None` se usa la de los costados.
    ///
    /// Existe porque un bloque de pasto no se ve igual por todos lados: hierba
    /// arriba, tierra abajo y tierra con una franja de hierba en los costados.
    /// Con una sola textura los acantilados se ven como una alfombra verde.
    pub texture_top: Option<TextureId>,

    /// Textura de la cara de abajo. Si es `None` se usa la de los costados.
    pub texture_bottom: Option<TextureId>,

    /// Mapa de normales en espacio tangente. Perturba la normal de la cara
    /// plana del cubo para simular relieve sin agregar geometria.
    pub normal_map: Option<TextureId>,

    /// Mascara de emision: permite que solo algunos pixeles de la textura
    /// brillen (por ejemplo, las vetas de un bloque de mineral luminoso)
    /// en vez de que emita la cara entera.
    pub emission_map: Option<TextureId>,

    /// Velocidad con la que la textura se desplaza, en texturas por segundo
    /// sobre (u, v). Anima los liquidos sin agregar geometria: el agua de la
    /// cascada baja y la lava se arrastra. `[0, 0]` para lo que no se mueve.
    pub uv_scroll: [f32; 2],
}

impl Default for Material {
    fn default() -> Self {
        Material {
            name: "sin nombre",
            albedo: [1.0, 0.0, 0.0, 0.0],
            specular: 1.0,
            refractive_index: 1.0,
            emission: Vec3::zeros(),
            diffuse: Vec3::new(0.8, 0.8, 0.8),
            texture: None,
            texture_top: None,
            texture_bottom: None,
            normal_map: None,
            emission_map: None,
            uv_scroll: [0.0, 0.0],
        }
    }
}

impl Material {
    /// `true` si el material deja pasar algo de luz. Se consulta al trazar
    /// sombras: un bloque de cristal no debe proyectar una sombra negra solida.
    #[inline]
    pub fn is_transparent(&self) -> bool {
        self.albedo[3] > 0.0
    }

}
