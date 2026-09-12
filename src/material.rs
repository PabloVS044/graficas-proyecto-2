//! Definicion de materiales.
//!
//! Un material describe como responde una superficie a la luz. La energia que
//! llega se reparte entre varios canales, y `albedo` dice cuanto va a cada uno.

use nalgebra_glm::Vec3;

/// Indice de un material dentro de `Scene::materials`.
pub type MaterialId = usize;

#[derive(Debug, Clone)]
pub struct Material {
    /// Color base de la superficie.
    pub diffuse: Vec3,

    /// `[difuso, especular, reflexion, refraccion]`.
    ///
    /// Idealmente suman 1 o menos, o el material estaria devolviendo mas luz de
    /// la que recibe. Los dos ultimos canales todavia no se usan: llegan con la
    /// reflexion y la refraccion.
    pub albedo: [f32; 4],

    /// Exponente de Phong. Valores altos dan un brillo pequeño y concentrado
    /// (superficie pulida); valores bajos, uno amplio y suave (superficie
    /// rugosa).
    pub specular: f32,
}

impl Material {
    pub fn new(diffuse: Vec3, albedo: [f32; 4], specular: f32) -> Self {
        Material {
            diffuse,
            albedo,
            specular,
        }
    }
}
