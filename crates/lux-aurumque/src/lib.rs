//! `lux-aurumque` — a minimal **transient path tracer** in Rust.
//!
//! Standard renderers compute steady-state radiance (light bounced
//! until equilibrium). This one refuses that simplification: every
//! photon path carries its cumulative optical length, which determines
//! its arrival time. Binning by arrival time produces a time-resolved
//! movie of a Gaussian pulse propagating through a Cornell-box scene
//! reskinned in *aurum* — gold, copper, amber.
//!
//! ## Run the demo render
//!
//! Clone the repo and run:
//!
//! ```bash
//! cargo run --release -p lux-aurumque
//! ffmpeg -framerate 30 -i frames/frame_%04d.png \
//!        -c:v libx264 -pix_fmt yuv420p -crf 18 lux-aurumque.mp4
//! ```
//!
//! ## Library use
//!
//! The crate exposes the path-tracing primitives ([`transient`],
//! [`camera`], [`material`], [`sphere`], etc.) plus a re-export of
//! [`SpectralBudget`] — the Faber–Krahn bound that refuses parameter
//! combinations whose total path-length horizon would exceed `3 · T_1`
//! for the bounded scene.
//!
//! ```
//! use lux_aurumque::{SpectralBudget, transient::C};
//!
//! // 0.95 m room → T_1 ≈ 6.34 ns; admit up to 3·T_1 ≈ 19.0 ns.
//! let budget = SpectralBudget::for_scene_diameter(0.95, C as f64);
//! assert!(budget.admits(15e-9));
//! assert!(!budget.admits(25e-9));
//! ```
//!
//! See `NOTES_PROCESS.md` in the repository for the process-philosophical
//! framing — Whitehead's ontology mapped onto the renderer's data flow.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod camera;
pub mod hit;
pub mod material;
pub mod ray;
pub mod scene;
pub mod sphere;
pub mod transient;
pub mod vec3;

#[doc(inline)]
pub use spectral_budget::{BudgetError, SpectralBudget};
