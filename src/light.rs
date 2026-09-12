//! Fuentes de luz.

use nalgebra_glm::Vec3;

/// Luz puntual: irradia en todas direcciones desde un punto.
#[derive(Debug, Clone, Copy)]
pub struct Light {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,

    /// Si es `true`, la intensidad cae con el cuadrado de la distancia, que es
    /// el comportamiento fisico. Para la luz que hace de sol conviene `false`:
    /// a esa distancia la caida solo oscurece la escena entera por igual.
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

    /// Luz sin caida, pensada para hacer de iluminacion general de la escena.
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
        // El minimo de 1 evita que la intensidad se dispare cuando la luz esta
        // practicamente pegada a la superficie.
        let d2 = nalgebra_glm::distance2(&self.position, point).max(1.0);
        self.color * (self.intensity / d2)
    }
}
