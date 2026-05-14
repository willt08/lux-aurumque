//! `lux-aurumque` — a transient path tracer and pluggable vision-pipeline
//! toolkit, unified by a process-philosophical spine.
//!
//! ## Quick example
//!
//! Build a vision pipeline with no features and no API keys. The
//! [`MockVisionClient`] demonstrates the pipeline shape — swap it for
//! `AnthropicVisionClient` (feature `anthropic-vision`) for real synthesis.
//!
//! ```no_run
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! use std::sync::Arc;
//! use lux_aurumque::{
//!     Antecedent, Concrescence, MockVisionClient, OcrPrehension,
//!     SpectralBudget, VisionConcrescence,
//! };
//!
//! let client = Arc::new(MockVisionClient);
//! let budget = SpectralBudget { principal_period: 4_000.0, ring_down_factor: 3.0 };
//!
//! let scene = VisionConcrescence::new(client, budget)
//!     .prehend(Antecedent::Ocr(OcrPrehension {
//!         text: "Cornell box; gold sphere on cream floor".into(),
//!         confidence: 0.95,
//!     }))
//!     .unify()
//!     .await?;
//!
//! println!("{}", scene.caption);
//! # Ok(()) }
//! ```
//!
//! ## Vision pipeline
//! `vision` — core types: [`VisionClient`], [`VisionConcrescence`],
//! [`Antecedent`], [`SceneArchive`]. Implement [`VisionClient`] to plug in
//! any vision model; [`MockVisionClient`] works without network access.
//!
//! `anthropic` (feature `anthropic-vision`) — `AnthropicVisionClient`:
//! real Claude vision synthesis and nexus translation.
//!
//! `runway` (feature `runway-video`) — `RunwayVideoClient`:
//! text-to-video, image-to-video, and character-performance generation.
//!
//! `shape` (feature `runway-video`) — `PromptShape`: structured JSON
//! nexus presets with relational glossaries, translated by Claude before submission.
//!
//! ## Path tracer
//! `camera`, `hit`, `material`, `ray`, `scene`, `sphere`, `transient`, `vec3`
//! render light propagating at finite speed, frame by picosecond. Clone the
//! repo and run `cargo run --release` to produce the gilded-room movie.
//!
//! ## Process spine
//! `process` — [`Occasion`], [`Society`], [`Concrescence`], [`PublicWorld`],
//! [`SpectralBudget`]: the invariants both the renderer and the vision
//! pipeline satisfy. See `NOTES_PROCESS.md` in the repository for the full
//! mapping onto Whitehead's ontology.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod camera;
pub mod hit;
pub mod material;
pub mod process;
pub mod ray;
pub mod scene;
pub mod sphere;
pub mod transient;
pub mod vec3;
pub mod vision;

#[cfg(feature = "anthropic-vision")]
#[cfg_attr(docsrs, doc(cfg(feature = "anthropic-vision")))]
pub mod anthropic;

#[cfg(feature = "runway-video")]
#[cfg_attr(docsrs, doc(cfg(feature = "runway-video")))]
pub mod runway;

#[cfg(feature = "runway-video")]
#[cfg_attr(docsrs, doc(cfg(feature = "runway-video")))]
pub mod shape;

#[cfg(test)]
mod tests;

#[doc(inline)]
pub use process::{
    BudgetError, Concrescence, Occasion, PublicWorld, Society, SpectralBudget,
};

#[doc(inline)]
pub use vision::{
    Antecedent, AudioPrehension, ConcrescenceError, ExifPrehension, ImagePrehension,
    MockVisionClient, OcrPrehension, SceneArchive, UnifiedScene, VisionClient,
    VisionConcrescence, load_image, stub_exif, stub_ocr,
};
