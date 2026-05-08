//! Ray primitive.
//!
//! A ray is an origin plus a direction, parametrized by `t >= 0`.
//! In transient rendering we *also* care about the cumulative optical path
//! length the ray has accumulated so far — this is what gives every photon
//! contribution a definite arrival time at the sensor.

use crate::vec3::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
    /// Optical path length already traveled BEFORE this ray segment starts.
    /// In vacuum this equals geometric distance; in a medium of refractive
    /// index n, it would be n * geometric distance. We assume vacuum.
    pub path_length: f32,
}

impl Ray {
    #[inline]
    pub fn new(origin: Vec3, dir: Vec3, path_length: f32) -> Self {
        Self { origin, dir: dir.normalize(), path_length }
    }

    /// Position along the ray at parameter `t`.
    #[inline]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + t * self.dir
    }
}
