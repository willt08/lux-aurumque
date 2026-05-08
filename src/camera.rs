//! Pinhole camera. Generates rays through pixels, with optional sub-pixel jitter.

use crate::ray::Ray;
use crate::vec3::Vec3;

use rand::Rng;
use rand_xoshiro::Xoshiro256PlusPlus;

pub struct Camera {
    origin: Vec3,
    lower_left: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
}

impl Camera {
    /// `lookfrom` -> `lookat`, `vup` is "up" in world space, `vfov_deg` is
    /// vertical field of view in degrees, `aspect` is width/height.
    pub fn new(lookfrom: Vec3, lookat: Vec3, vup: Vec3, vfov_deg: f32, aspect: f32) -> Self {
        let theta = vfov_deg.to_radians();
        let h = (theta / 2.0).tan();
        let viewport_h = 2.0 * h;
        let viewport_w = aspect * viewport_h;

        let w = (lookfrom - lookat).normalize();
        let u = vup.cross(w).normalize();
        let v = w.cross(u);

        let origin = lookfrom;
        let horizontal = viewport_w * u;
        let vertical = viewport_h * v;
        let lower_left = origin - horizontal / 2.0 - vertical / 2.0 - w;

        Self { origin, lower_left, horizontal, vertical }
    }

    /// `s, t` in [0, 1] across the image. We jitter inside the pixel for
    /// antialiasing; jitter is generated outside this method so the caller
    /// can keep control of the RNG stream.
    pub fn ray_through(&self, s: f32, t: f32, _rng: &mut Xoshiro256PlusPlus) -> Ray {
        let dir = self.lower_left + s * self.horizontal + t * self.vertical - self.origin;
        Ray::new(self.origin, dir, 0.0)
    }

    /// Sub-pixel jitter convenience.
    pub fn jitter(rng: &mut Xoshiro256PlusPlus) -> (f32, f32) {
        (rng.r#gen::<f32>(), rng.r#gen::<f32>())
    }
}
