//! Materials: how a surface scatters or emits light.
//!
//! Every material implements:
//!   - `scatter`: given an incoming ray and a hit, optionally produce a
//!     scattered ray + an attenuation (the path's color throughput).
//!   - `emitted`: light produced *at* the surface (for emissive materials).
//!
//! Returning `None` from `scatter` means the path terminates here (e.g.
//! a pure light source absorbs anything that hits it from the back).

use crate::hit::HitRecord;
use crate::ray::Ray;
use crate::vec3::{near_zero, random_unit_vec, reflect, Vec3};

use rand_xoshiro::Xoshiro256PlusPlus;

pub struct Scatter {
    pub scattered: Ray,
    pub attenuation: Vec3,
}

pub trait Material: Send + Sync {
    fn scatter(
        &self,
        ray_in: &Ray,
        rec: &HitRecord,
        rng: &mut Xoshiro256PlusPlus,
    ) -> Option<Scatter>;

    /// Default: most materials don't emit.
    fn emitted(&self) -> Vec3 { Vec3::ZERO }
}

// ---------------------------------------------------------------------------
// Lambertian — ideal diffuse. Cosine-weighted hemisphere sampling.
// ---------------------------------------------------------------------------

pub struct Lambertian { pub albedo: Vec3 }

impl Lambertian {
    pub fn new(albedo: Vec3) -> Self { Self { albedo } }
}

impl Material for Lambertian {
    fn scatter(
        &self,
        ray_in: &Ray,
        rec: &HitRecord,
        rng: &mut Xoshiro256PlusPlus,
    ) -> Option<Scatter> {
        // Cosine-weighted bounce: normal + random unit vec in the sphere
        // is the simplest unbiased way to get cos-weighted directions.
        let mut dir = rec.normal + random_unit_vec(rng);
        if near_zero(dir) { dir = rec.normal; }
        // Carry forward the path length traveled to this hit; the *new* segment
        // adds to it as the ray traces further.
        let scattered = Ray::new(rec.p, dir, ray_in.path_length + rec.t);
        Some(Scatter { scattered, attenuation: self.albedo })
    }
}

// ---------------------------------------------------------------------------
// Metal — perfect reflection with optional fuzz.
// ---------------------------------------------------------------------------

pub struct Metal { pub albedo: Vec3, pub fuzz: f32 }

impl Metal {
    pub fn new(albedo: Vec3, fuzz: f32) -> Self {
        Self { albedo, fuzz: fuzz.clamp(0.0, 1.0) }
    }
}

impl Material for Metal {
    fn scatter(
        &self,
        ray_in: &Ray,
        rec: &HitRecord,
        rng: &mut Xoshiro256PlusPlus,
    ) -> Option<Scatter> {
        let reflected = reflect(ray_in.dir, rec.normal);
        let dir = reflected + self.fuzz * random_unit_vec(rng);
        if dir.dot(rec.normal) <= 0.0 { return None; }
        let scattered = Ray::new(rec.p, dir, ray_in.path_length + rec.t);
        Some(Scatter { scattered, attenuation: self.albedo })
    }
}

// ---------------------------------------------------------------------------
// Emissive — a pulsed light source.
//
// In transient rendering the source is *temporally localized*: it emits
// only during a short window (e.g. a femtosecond pulse). Any photon that
// terminates on this surface contributes radiance scaled by the temporal
// profile evaluated at (t - path_length / c).
//
// We don't need a full temporal profile per material — the emitted()
// method just returns the peak intensity, and the transient integrator
// handles the time gating when binning the contribution.
// ---------------------------------------------------------------------------

pub struct DiffuseLight { pub intensity: Vec3 }

impl DiffuseLight {
    pub fn new(intensity: Vec3) -> Self { Self { intensity } }
}

impl Material for DiffuseLight {
    fn scatter(
        &self,
        _ray_in: &Ray,
        _rec: &HitRecord,
        _rng: &mut Xoshiro256PlusPlus,
    ) -> Option<Scatter> {
        // Light absorbs incoming rays — the bounce terminates here.
        None
    }

    fn emitted(&self) -> Vec3 { self.intensity }
}
