//! Fuentes de luz.

use nalgebra_glm::Vec3;

/// Luz puntual: irradia en todas direcciones desde un punto.
///
/// Es la que usa el diorama porque las antorchas, lamparas y focos de la escena
/// son objetos pequeños con posicion concreta, y porque es la unica que produce
/// sombras que se abren con la distancia.
#[derive(Debug, Clone, Copy)]
pub struct Light {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,

    /// Si es `true`, la intensidad cae con el cuadrado de la distancia
    /// (comportamiento fisico). Para la luz principal que hace de sol conviene
    /// `false`, porque a esa escala la caida solo oscurece la escena entera.
    pub attenuates: bool,
}

impl Light {
    pub fn new(position: Vec3, color: Vec3, intensity: f32) -> Self {
        Light {
            position,
            color,
            intensity,
            attenuates: true,
        }
    }

    /// Luz sin caida, pensada para hacer de iluminacion global de la escena.
    pub fn sun(position: Vec3, color: Vec3, intensity: f32) -> Self {
        Light {
            position,
            color,
            intensity,
            attenuates: false,
        }
    }

    /// Radiancia que llega a `point`, ya con la caida aplicada.
    #[inline]
    pub fn radiance_at(&self, point: &Vec3) -> Vec3 {
        if !self.attenuates {
            return self.color * self.intensity;
        }
        let d2 = nalgebra_glm::distance2(&self.position, point).max(1.0);
        self.color * (self.intensity / d2)
    }
}
