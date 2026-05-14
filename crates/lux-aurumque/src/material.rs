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

/// Outcome of a material's `scatter` call: the outgoing ray and the
/// throughput multiplier applied to subsequent radiance along it.
pub struct Scatter {
    /// The outgoing ray. Origin sits at the hit point; direction is
    /// material-dependent (cosine-weighted for Lambertian, mirror for
    /// Metal, etc.). `path_length` carries the cumulative optical length
    /// forward so the transient integrator can time-bin correctly.
    pub scattered: Ray,
    /// Multiplicative attenuation applied to the radiance returned by
    /// the recursive trace. RGB albedo for diffuse/specular surfaces.
    pub attenuation: Vec3,
}

/// How a surface scatters or emits light. `scatter` produces an
/// outgoing ray and an attenuation; `emitted` returns surface emission
/// (zero for non-emissive materials).
pub trait Material: Send + Sync {
    /// Given an incoming ray and the hit it produced, return a
    /// scattered ray + attenuation. `None` terminates the path —
    /// e.g. a pure light source absorbs whatever strikes it.
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

/// Ideal diffuse material — cosine-weighted hemisphere scattering. The
/// `albedo` is the fraction of incoming radiance reflected (per
/// channel); values in `[0, 1]`.
pub struct Lambertian {
    /// Reflectance in linear RGB. `(1, 1, 1)` is perfectly diffuse
    /// white; `(0, 0, 0)` is a black hole.
    pub albedo: Vec3,
}

impl Lambertian {
    /// Construct a Lambertian surface with the given albedo.
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

/// Specular metal — mirror reflection with an optional fuzz term that
/// perturbs the reflected direction. `fuzz = 0` is a perfect mirror;
/// `fuzz = 1` is a fully blurred reflection.
pub struct Metal {
    /// Reflectance in linear RGB.
    pub albedo: Vec3,
    /// Reflection blur strength in `[0, 1]`. Constructor clamps inputs
    /// to that range.
    pub fuzz: f32,
}

impl Metal {
    /// Construct a metallic surface. `fuzz` is clamped to `[0, 1]`.
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

/// Emissive surface — the pulse source for transient rendering. The
/// emitter absorbs incoming rays (`scatter` returns `None`) and
/// contributes its `intensity` whenever a path terminates here. Time
/// gating against the pulse profile happens in
/// [`crate::transient::trace_path`], not on the material.
pub struct DiffuseLight {
    /// Peak emitted radiance in linear RGB. Magnitudes can exceed 1.0
    /// — they're scaled by the temporal pulse profile when binned.
    pub intensity: Vec3,
}

impl DiffuseLight {
    /// Construct an emitter at the given peak intensity.
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
