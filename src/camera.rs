//! Camara y generacion de los rayos primarios.

use nalgebra_glm::{Vec3, cross, normalize};

pub struct Camera {
    pub eye: Vec3,
    /// Punto al que mira la camara.
    pub target: Vec3,
    /// Campo de vision vertical en radianes.
    pub fov: f32,
}

impl Camera {
    pub fn new(eye: Vec3, target: Vec3) -> Self {
        Camera {
            eye,
            target,
            fov: std::f32::consts::FRAC_PI_3,
        }
    }
}

/// Datos de camara ya precalculados para el cuadro.
///
/// Todo lo que no depende del pixel (la base, el ojo, la escala del campo de
/// vision) se calcula una sola vez aqui en vez de repetirlo dentro del bucle de
/// rayos, que se ejecuta cientos de miles de veces.
#[derive(Debug, Clone, Copy)]
pub struct CameraFrame {
    pub eye: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    /// `tan(fov / 2)`: media altura del plano de proyeccion a distancia 1.
    half_height: f32,
    aspect: f32,
}

impl CameraFrame {
    pub fn new(camera: &Camera, width: usize, height: usize) -> Self {
        let forward = normalize(&(camera.target - camera.eye));
        let world_up = Vec3::new(0.0, 1.0, 0.0);
        let right = normalize(&cross(&forward, &world_up));
        let up = cross(&right, &forward);

        CameraFrame {
            eye: camera.eye,
            right,
            up,
            forward,
            half_height: (camera.fov * 0.5).tan(),
            aspect: width as f32 / height as f32,
        }
    }

    /// Direccion del rayo primario que sale por el punto `(px, py)` del
    /// framebuffer.
    #[inline]
    pub fn ray_direction(&self, px: f32, py: f32, width: f32, height: f32) -> Vec3 {
        // Pixel -> espacio de pantalla [-1, 1], con Y hacia arriba.
        let sx = (2.0 * px / width - 1.0) * self.aspect * self.half_height;
        let sy = (1.0 - 2.0 * py / height) * self.half_height;
        normalize(&(self.right * sx + self.up * sy + self.forward))
    }
}
