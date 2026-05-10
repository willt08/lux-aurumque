//! `lux-aurumque` — a transient path tracer and pluggable vision-pipeline
//! toolkit, unified by a process-philosophical spine.
//!
//! ## Path tracer
//! `camera`, `hit`, `material`, `ray`, `scene`, `sphere`, `transient`, `vec3`
//! render light propagating at finite speed, frame by picosecond.
//!
//! ## Vision pipeline
//! `vision` — core types: [`VisionClient`], [`VisionConcrescence`],
//! [`Antecedent`], [`SceneArchive`]. Implement [`VisionClient`] to plug in
//! any vision model; [`MockVisionClient`] works without network access.
//!
//! `anthropic` (feature `anthropic-vision`) — [`anthropic::AnthropicVisionClient`]:
//! real Claude vision synthesis and nexus translation.
//!
//! `runway` (feature `runway-video`) — [`runway::RunwayVideoClient`]:
//! text-to-video, image-to-video, and character-performance generation.
//!
//! `shape` (feature `runway-video`) — [`shape::PromptShape`]: structured JSON
//! nexus presets with relational glossaries, translated by Claude before submission.
//!
//! ## Process spine
//! `process` — [`Occasion`], [`Society`], [`Concrescence`], [`PublicWorld`],
//! [`SpectralBudget`]: the invariants both the renderer and the vision
//! pipeline satisfy.

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
