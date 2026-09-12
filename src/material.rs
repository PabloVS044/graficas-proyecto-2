//! Definicion de materiales.
//!
//! Un material describe como responde una superficie a la luz. Por ahora solo
//! contempla la reflexion difusa, que es la que mas influye en el color: la luz
//! rebota en todas direcciones por igual porque la superficie es irregular a
//! escala microscopica.

use nalgebra_glm::Vec3;

/// Indice de un material dentro de `Scene::materials`.
pub type MaterialId = usize;

#[derive(Debug, Clone)]
pub struct Material {
    /// Color base de la superficie.
    pub diffuse: Vec3,

    /// Fraccion de la luz incidente que la superficie devuelve de forma difusa.
    /// Por encima de 1 estaria creando energia de la nada.
    pub albedo: f32,
}

impl Material {
    pub fn new(diffuse: Vec3, albedo: f32) -> Self {
        Material { diffuse, albedo }
    }
}
