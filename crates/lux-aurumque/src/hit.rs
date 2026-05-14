//! Hit records and the Hittable trait — the abstract interface every
//! scene primitive must implement.

use crate::material::Material;
use crate::ray::Ray;
use crate::vec3::Vec3;

use std::sync::Arc;

/// Information returned by [`Hittable::hit`] when a ray strikes a
/// surface: the world-space hit point, surface normal, parametric `t`,
/// front/back flag, and the material to shade with.
#[derive(Clone)]
pub struct HitRecord {
    /// World-space hit point.
    pub p: Vec3,
    /// Outward-facing surface normal, always normalized.
    pub normal: Vec3,
    /// Ray parameter at hit.
    pub t: f32,
    /// Whether the ray struck the front face. Lets materials know if they're
    /// inside or outside an object.
    pub front_face: bool,
    /// Material reference. Arc so primitives can share materials cheaply.
    pub material: Arc<dyn Material>,
}

impl HitRecord {
    /// Set the normal so it always points *against* the incoming ray.
    /// This convention simplifies BRDF evaluation.
    #[inline]
    pub fn set_face_normal(&mut self, ray: &Ray, outward_normal: Vec3) {
        self.front_face = ray.dir.dot(outward_normal) < 0.0;
        self.normal = if self.front_face { outward_normal } else { -outward_normal };
    }
}

/// Abstract intersection target. Implemented by every scene primitive
/// (sphere, plane, mesh, etc.) and by [`HittableList`] which composes
/// them. `Send + Sync` because tile rendering hits objects from worker
/// threads.
pub trait Hittable: Send + Sync {
    /// Return a hit record if the ray strikes this object within
    /// `(t_min, t_max)`. `None` if the ray misses or the closest hit is
    /// outside the bounds — those bounds let callers (e.g.
    /// [`HittableList`]) tighten the search interval after each hit.
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord>;
}

/// A list of hittables that returns the closest hit. The world's root
/// container.
pub struct HittableList {
    /// Owned scene primitives. Each entry is `Arc` so primitives can
    /// share materials and be addressed from multiple lists cheaply.
    pub objects: Vec<Arc<dyn Hittable>>,
}

impl Default for HittableList {
    fn default() -> Self { Self::new() }
}

impl HittableList {
    /// Construct an empty list.
    pub fn new() -> Self { Self { objects: Vec::new() } }
    /// Add a primitive to the list.
    pub fn push(&mut self, obj: Arc<dyn Hittable>) { self.objects.push(obj); }
}

impl Hittable for HittableList {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let mut closest = t_max;
        let mut best = None;
        for obj in &self.objects {
            if let Some(rec) = obj.hit(ray, t_min, closest) {
                closest = rec.t;
                best = Some(rec);
            }
        }
        best
    }
}
