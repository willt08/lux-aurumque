//! A small demo scene: a Cornell-box-style room of large spheres approximating
//! walls, plus a small sphere pulse-emitter near the ceiling and a diffuse
//! sphere on the floor that the wavefront will sweep across.
//!
//! We use spheres for walls (giant radii flatten visually into planes) so
//! the whole scene needs only one primitive type — keeping the codebase
//! compact and the intersection routine tight.

use std::sync::Arc;

use crate::hit::HittableList;
use crate::material::{DiffuseLight, Lambertian, Metal};
use crate::sphere::Sphere;
use crate::vec3::Vec3;

/// Build the scene. Units are in METERS — important because the speed of
/// light c is in m/s and our time bins are in seconds. A 30 cm scene at
/// 10 ps bins gives one bin of light-travel = 3 mm of geometry, so a
/// half-meter scene fills ~150 bins, which is what we want.
pub fn cornell_room() -> HittableList {
    let mut world = HittableList::new();

    // --- walls: 1000-meter spheres, flat at the scale we view -----------
    // Palette is warm: cream, burnished copper, antique amber. The point is
    // for the wavefront to look like it's gilding surfaces as it sweeps.
    let r_wall  = 1000.0;
    let cream   = Arc::new(Lambertian::new(Vec3::new(0.82, 0.76, 0.62)));
    let copper  = Arc::new(Lambertian::new(Vec3::new(0.55, 0.25, 0.10)));
    let amber   = Arc::new(Lambertian::new(Vec3::new(0.50, 0.35, 0.10)));

    // Floor / ceiling / back wall: warm cream.
    world.push(Arc::new(Sphere::new(
        Vec3::new(0.0, -r_wall, 0.0), r_wall, cream.clone(),
    )));
    world.push(Arc::new(Sphere::new(
        Vec3::new(0.0, r_wall + 0.55, 0.0), r_wall, cream.clone(),
    )));
    world.push(Arc::new(Sphere::new(
        Vec3::new(0.0, 0.0, -r_wall - 0.55), r_wall, cream,
    )));
    // Left wall: deep copper.
    world.push(Arc::new(Sphere::new(
        Vec3::new(-r_wall - 0.275, 0.0, 0.0), r_wall, copper,
    )));
    // Right wall: antique amber.
    world.push(Arc::new(Sphere::new(
        Vec3::new(r_wall + 0.275, 0.0, 0.0), r_wall, amber,
    )));

    // --- gold sphere — polished, mirror-like. Reflectance is the spectral
    // response of physical gold (R high, G medium, B low) in linear sRGB,
    // approximately. Low fuzz keeps the wavefront's reflection legible.
    let gold = Arc::new(Metal::new(Vec3::new(1.00, 0.78, 0.35), 0.02));
    world.push(Arc::new(Sphere::new(
        Vec3::new(-0.10, 0.10, -0.30), 0.10, gold,
    )));

    // --- diffuse satin-gold sphere for warm bleed across the cream floor.
    let satin_gold = Arc::new(Lambertian::new(Vec3::new(0.78, 0.58, 0.22)));
    world.push(Arc::new(Sphere::new(
        Vec3::new(0.13, 0.08, -0.20), 0.08, satin_gold,
    )));

    // --- the pulse emitter: warm-tinted (aurum) rather than neutral white.
    // R:G:B ≈ 50:38:18 gives a 3000K-ish candleflame cast. Intensity stays
    // high so per-bin Gaussian-weighted contributions read clearly.
    let light = Arc::new(DiffuseLight::new(Vec3::new(50.0, 38.0, 18.0)));
    world.push(Arc::new(Sphere::new(
        Vec3::new(0.0, 0.50, -0.30), 0.04, light,
    )));

    world
}
