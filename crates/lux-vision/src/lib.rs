//! `lux-vision` — a pluggable, multimodal vision pipeline with
//! token-budget admission.
//!
//! Implement [`VisionClient`] to plug any vision model into the pipeline;
//! [`MockVisionClient`] runs the full pipeline offline with no keys.
//!
//! ## Quick example
//!
//! ```no_run
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! use std::sync::Arc;
//! use lux_vision::{Input, MockVisionClient, OcrInput, VisionPipeline};
//! use spectral_budget::SpectralBudget;
//!
//! let client = Arc::new(MockVisionClient);
//! let budget = SpectralBudget { principal_period: 4_000.0, ring_down_factor: 3.0 };
//!
//! let scene = VisionPipeline::new(client, budget)
//!     .with(Input::Ocr(OcrInput {
//!         text: "Cornell box; gold sphere on cream floor".into(),
//!         confidence: 0.95,
//!     }))
//!     .run()
//!     .await?;
//!
//! println!("{}", scene.caption);
//! # Ok(()) }
//! ```
//!
//! Swap `MockVisionClient` for `anthropic::AnthropicVisionClient`
//! (feature `anthropic-vision`) or `runway::RunwayVideoClient`
//! (feature `runway-video`) for production use.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod pipeline;

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
pub use pipeline::{
    AudioInput, ExifInput, ImageInput, Input, MockVisionClient, OcrInput, Scene,
    SceneArchive, VisionClient, VisionError, VisionPipeline, load_image, stub_exif, stub_ocr,
};
