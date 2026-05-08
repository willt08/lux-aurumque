//! `lux-aurumque` — a transient path tracer with a process-philosophical
//! spine. The renderer's modules and the abstractions named in
//! `NOTES_PROCESS.md` are re-exported here so downstream crates can
//! consume both: render light, or implement the same traits over a
//! different kind of flux (vision API, audio event stream, agent run)
//! to inherit the [`SpectralBudget`] bound and the rest of the spine.
//!
//! See `examples/receptacle.rs` for a runnable blueprint applying the
//! process traits to a vision-API substrate.

pub mod camera;
pub mod hit;
pub mod material;
pub mod process;
pub mod ray;
pub mod scene;
pub mod sphere;
pub mod transient;
pub mod vec3;

pub use process::{
    BudgetError, Concrescence, Occasion, PublicWorld, Society, SpectralBudget,
};
