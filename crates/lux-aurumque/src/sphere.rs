//! Analytic sphere intersection.
//!
//! Solving |o + t*d - c|^2 = r^2  gives a quadratic in t:
//!     a t^2 + 2 h t + c_term = 0
//! where a = d.d, h = (o-c).d, c_term = (o-c).(o-c) - r^2.
//! With d normalized, a = 1 and we save a few ops.

use std::sync::Arc;

use crate::hit::{HitRecord, Hittable};
use crate::material::Material;
use crate::ray::Ray;
use crate::vec3::Vec3;

/// Analytic sphere primitive. The scene's only primitive type — walls
/// are large spheres flattened at the camera's scale, which keeps the
/// intersection routine tight and the code compact.
pub struct Sphere {
    /// World-space centre.
    pub center: Vec3,
    /// Radius in metres.
    pub radius: f32,
    /// Shared material. `Arc` so many spheres can reuse one material
    /// (cream walls, gold sphere, etc.).
    pub material: Arc<dyn Material>,
}

impl Sphere {
    /// Construct a sphere with the given centre, radius, and material.
    pub fn new(center: Vec3, radius: f32, material: Arc<dyn Material>) -> Self {
        Self { center, radius, material }
    }
}

impl Hittable for Sphere {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let oc = ray.origin - self.center;
        // Ray dir is normalized -> a = 1.
        let half_b = oc.dot(ray.dir);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = half_b * half_b - c;
        if discriminant < 0.0 { return None; }

        let sqrt_d = discriminant.sqrt();
        // Try the nearer root first.
        let mut root = -half_b - sqrt_d;
        if root <= t_min || root >= t_max {
            root = -half_b + sqrt_d;
            if root <= t_min || root >= t_max { return None; }
        }

        let p = ray.at(root);
        let outward_normal = (p - self.center) / self.radius;
        let mut rec = HitRecord {
            p,
            normal: outward_normal,
            t: root,
            front_face: true,
            material: self.material.clone(),
        };
        rec.set_face_normal(ray, outward_normal);
        Some(rec)
    }
}
