//! Vector aliases and small geometric utilities.
//!
//! We use `glam::Vec3` everywhere — it's SIMD-friendly and its operator
//! overloading reads cleanly. This module just re-exports it under a shorter
//! local name and adds a few helpers we'll need repeatedly.

pub use glam::Vec3;

use rand::Rng;
use rand_xoshiro::Xoshiro256PlusPlus;

/// Sample a unit vector uniformly on the sphere.
/// Used when generating diffuse bounce directions.
#[inline]
pub fn random_unit_vec(rng: &mut Xoshiro256PlusPlus) -> Vec3 {
    // Marsaglia's method: rejection-sample inside the unit cube,
    // then normalize. Cheap, branch-light, and exact.
    loop {
        let p = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        );
        let len_sq = p.length_squared();
        if (1e-6..=1.0).contains(&len_sq) {
            return p / len_sq.sqrt();
        }
    }
}

/// Reflect `v` about a unit normal `n`. Standard mirror formula.
#[inline]
pub fn reflect(v: Vec3, n: Vec3) -> Vec3 {
    v - 2.0 * v.dot(n) * n
}

/// Component-wise near-zero check — used to reject degenerate scatter dirs.
#[inline]
pub fn near_zero(v: Vec3) -> bool {
    const EPS: f32 = 1e-8;
    v.x.abs() < EPS && v.y.abs() < EPS && v.z.abs() < EPS
}
