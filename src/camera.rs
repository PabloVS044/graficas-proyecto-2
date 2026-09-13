//! Camara orbital.
//!
//! Para poder mirar el diorama desde cualquier lado hay que girarlo y poder
//! acercarse. En vez de mover los cubos, lo que se mueve es la camara alrededor
//! de un punto fijo: el resultado visual es el mismo (el diorama parece girar)
//! y la escena no hay que reconstruirla en cada frame.

use crate::vector::{Vec3, cross, normalize};

pub struct Camera {
    /// Punto alrededor del cual orbita la camara.
    pub center: Vec3,
    /// Angulo horizontal en radianes.
    pub yaw: f32,
    /// Angulo vertical en radianes. Se limita para no pasar por los polos,
    /// donde la base de la camara se vuelve degenerada.
    pub pitch: f32,
    /// Distancia al centro: el zoom.
    pub distance: f32,
    /// Campo de vision vertical en radianes.
    pub fov: f32,

    /// Se enciende cuando la camara se movio. `main` lo usa para decidir si
    /// renderiza rapido y feo o lento y bonito.
    pub dirty: bool,
}

/// Limites del zoom y de la inclinacion.
const MIN_DISTANCE: f32 = 7.0;
const MAX_DISTANCE: f32 = 70.0;

/// La camara no baja del horizonte. No es solo estetico: la cara inferior de la
/// isla no existe (se descarta al construir la escena, ver `voxel.rs`), asi que
/// mirar el diorama desde abajo mostraria el hueco.
const MIN_PITCH: f32 = 0.04;
const MAX_PITCH: f32 = 1.45; // ~83 grados

impl Camera {
    pub fn new(center: Vec3, distance: f32) -> Self {
        Camera {
            center,
            yaw: 0.7,
            pitch: 0.42,
            distance,
            fov: std::f32::consts::FRAC_PI_3,
            dirty: true,
        }
    }

    /// Posicion del ojo, deducida de los angulos y la distancia.
    pub fn eye(&self) -> Vec3 {
        let (sp, cp) = self.pitch.sin_cos();
        let (sy, cy) = self.yaw.sin_cos();
        self.center + Vec3::new(cp * sy, sp, cp * cy) * self.distance
    }

    /// Base ortonormal de la camara: (derecha, arriba, frente).
    pub fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let forward = normalize(&(self.center - self.eye()));
        let world_up = Vec3::new(0.0, 1.0, 0.0);
        let right = normalize(&cross(&forward, &world_up));
        let up = cross(&right, &forward);
        (right, up, forward)
    }

    pub fn orbit(&mut self, d_yaw: f32, d_pitch: f32) {
        if d_yaw == 0.0 && d_pitch == 0.0 {
            return;
        }
        self.yaw += d_yaw;
        self.pitch = (self.pitch + d_pitch).clamp(MIN_PITCH, MAX_PITCH);
        self.dirty = true;
    }

    /// Acerca o aleja la camara. Se multiplica en vez de sumar para que el
    /// zoom se sienta igual de rapido de cerca que de lejos.
    pub fn zoom(&mut self, factor: f32) {
        if factor == 1.0 {
            return;
        }
        self.distance = (self.distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
        self.dirty = true;
    }

    pub fn pan_vertical(&mut self, amount: f32) {
        if amount == 0.0 {
            return;
        }
        self.center.y += amount;
        self.dirty = true;
    }
}

/// Datos de camara ya precalculados para el frame, listos para repartir entre
/// los hilos.
///
/// Todo lo que no depende del pixel (la base, el ojo, la escala del FOV) se
/// calcula una sola vez aqui en vez de repetirlo millones de veces dentro del
/// bucle de rayos.
#[derive(Debug, Clone, Copy)]
pub struct CameraFrame {
    pub eye: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
    /// `tan(fov / 2)`: media altura del plano de proyeccion a distancia 1.
    pub half_height: f32,
    pub aspect: f32,
}

impl CameraFrame {
    pub fn new(camera: &Camera, width: usize, height: usize) -> Self {
        let (right, up, forward) = camera.basis();
        CameraFrame {
            eye: camera.eye(),
            right,
            up,
            forward,
            half_height: (camera.fov * 0.5).tan(),
            aspect: width as f32 / height as f32,
        }
    }

    /// Direccion del rayo primario que sale por el punto `(px, py)` del
    /// framebuffer, en pixeles con parte fraccionaria (para el supersampling).
    #[inline]
    pub fn ray_direction(&self, px: f32, py: f32, width: f32, height: f32) -> Vec3 {
        // Pixel -> espacio de pantalla [-1, 1], con Y hacia arriba.
        let sx = (2.0 * px / width - 1.0) * self.aspect * self.half_height;
        let sy = (1.0 - 2.0 * py / height) * self.half_height;
        normalize(&(self.right * sx + self.up * sy + self.forward))
    }
}
