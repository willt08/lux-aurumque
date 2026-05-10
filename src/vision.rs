//! The vision-pipeline layer: pluggable multi-modal prehension → unified
//! scene synthesis, bounded by [`SpectralBudget`].
//!
//! Core types are unconditional. Feature-gated clients (`AnthropicVisionClient`,
//! `RunwayVideoClient`) live in their own modules and implement [`VisionClient`]
//! as a drop-in swap for [`MockVisionClient`].

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;

use crate::process::{BudgetError, Concrescence, Occasion, PublicWorld, Society, SpectralBudget};

// ── Prehensions ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct ImagePrehension {
    pub raw_bytes: Arc<Vec<u8>>,
    pub media_type: &'static str,
    pub width: u32,
    pub height: u32,
    pub estimated_tokens: u32,
}

#[derive(Clone, Debug)]
pub struct OcrPrehension {
    pub text: String,
    pub confidence: f32,
}

#[derive(Clone, Debug)]
pub struct ExifPrehension {
    pub fields: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub struct AudioPrehension {
    pub transcript: String,
    pub duration_secs: f32,
}

/// One heterogeneous antecedent, packaged as a uniform enum so the
/// concrescence can iterate over them. Each variant is already
/// objectified data — pre-perished facts entering from the public world.
#[derive(Clone, Debug)]
pub enum Antecedent {
    Image(ImagePrehension),
    Ocr(OcrPrehension),
    Exif(ExifPrehension),
    Audio(AudioPrehension),
}

impl Antecedent {
    /// Token weight used by [`SpectralBudget`] admission. The image
    /// estimate is a proxy (`w·h / 750`); text estimates use `bytes/4`.
    pub fn token_weight(&self) -> u32 {
        match self {
            Antecedent::Image(i) => i.estimated_tokens,
            Antecedent::Ocr(o) => (o.text.len() as u32).div_ceil(4),
            Antecedent::Exif(e) => e
                .fields
                .iter()
                .map(|(k, v)| ((k.len() + v.len()) as u32) / 4 + 1)
                .sum::<u32>()
                .max(8),
            Antecedent::Audio(a) => (a.transcript.len() as u32).div_ceil(4),
        }
    }
}

impl Occasion for Antecedent {
    type Datum = Self;
    type Satisfaction = Self;
    fn datum(&self) -> &Self { self }
    fn is_satisfied(&self) -> bool { true }
    fn satisfaction(&self) -> Option<&Self> { Some(self) }
}

// ── Unified satisfaction ──────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct UnifiedScene {
    pub caption: String,
    pub contributing: usize,
    pub total_tokens: u32,
}

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug)]
#[non_exhaustive]
pub enum ConcrescenceError {
    Budget(BudgetError),
    NoAntecedents,
    Network(String),
    Refused(String),
}

impl std::fmt::Display for ConcrescenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConcrescenceError::Budget(e) => write!(f, "{e}"),
            ConcrescenceError::NoAntecedents => write!(f, "no antecedents to concresce"),
            ConcrescenceError::Network(s) => write!(f, "network: {s}"),
            ConcrescenceError::Refused(r) => write!(f, "refused: {r}"),
        }
    }
}

impl std::error::Error for ConcrescenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConcrescenceError::Budget(e) => Some(e),
            _ => None,
        }
    }
}

impl From<BudgetError> for ConcrescenceError {
    fn from(e: BudgetError) -> Self { ConcrescenceError::Budget(e) }
}

// ── VisionClient trait ────────────────────────────────────────────────────────

/// The mover that turns a slice of prehensions into a unified satisfaction.
/// Implement this trait to plug any vision model into the pipeline.
/// [`MockVisionClient`] is the network-free reference implementation.
#[async_trait]
pub trait VisionClient: Send + Sync {
    async fn synthesize(
        &self,
        antecedents: &[Antecedent],
    ) -> Result<UnifiedScene, ConcrescenceError>;
}

// ── Mock client ───────────────────────────────────────────────────────────────

/// Deterministic, network-free client. Demonstrates the shape of
/// satisfaction without requiring API keys. Swap in [`AnthropicVisionClient`]
/// (feature `anthropic-vision`) for production use; the rest of the pipeline
/// is untouched by which client is plugged in.
pub struct MockVisionClient;

#[async_trait]
impl VisionClient for MockVisionClient {
    async fn synthesize(
        &self,
        antecedents: &[Antecedent],
    ) -> Result<UnifiedScene, ConcrescenceError> {
        if antecedents.is_empty() {
            return Err(ConcrescenceError::NoAntecedents);
        }
        let mut parts: Vec<String> = Vec::with_capacity(antecedents.len());
        let mut total_tokens: u32 = 0;
        for a in antecedents {
            total_tokens = total_tokens.saturating_add(a.token_weight());
            parts.push(match a {
                Antecedent::Image(i) => {
                    format!("[image {}x{} ~{}tok]", i.width, i.height, i.estimated_tokens)
                }
                Antecedent::Ocr(o) => {
                    format!("[ocr conf={:.2}: {:?}]", o.confidence, truncate(&o.text, 64))
                }
                Antecedent::Exif(e) => format!("[exif: {} fields]", e.fields.len()),
                Antecedent::Audio(a) => format!(
                    "[audio {:.1}s: {:?}]",
                    a.duration_secs,
                    truncate(&a.transcript, 64)
                ),
            });
        }
        Ok(UnifiedScene {
            caption: format!(
                "concresced from {} prehensions: {}",
                antecedents.len(),
                parts.join(" ⊕ "),
            ),
            contributing: antecedents.len(),
            total_tokens,
        })
    }
}

// ── VisionConcrescence ────────────────────────────────────────────────────────

pub struct VisionConcrescence {
    antecedents: Vec<Antecedent>,
    client: Arc<dyn VisionClient>,
    budget: SpectralBudget,
}

impl VisionConcrescence {
    #[must_use]
    pub fn new(client: Arc<dyn VisionClient>, budget: SpectralBudget) -> Self {
        Self { antecedents: Vec::new(), client, budget }
    }

    #[must_use]
    pub fn prehend(mut self, a: Antecedent) -> Self {
        self.antecedents.push(a);
        self
    }

    fn admit(&self) -> Result<(), ConcrescenceError> {
        let total: u32 = self
            .antecedents
            .iter()
            .map(|a| a.token_weight())
            .fold(0u32, |acc, x| acc.saturating_add(x));
        self.budget.try_admit(total as f64)?;
        Ok(())
    }
}

impl Society for VisionConcrescence {
    type Member = Antecedent;
    fn members(&self) -> &[Antecedent] { &self.antecedents }
    fn diameter(&self) -> f64 {
        self.antecedents
            .iter()
            .map(|a| a.token_weight() as u64)
            .sum::<u64>() as f64
    }
}

impl Concrescence for VisionConcrescence {
    type Antecedent = Antecedent;
    type Unified = Pin<Box<dyn Future<Output = Result<UnifiedScene, ConcrescenceError>> + Send>>;

    fn prehensions(&self) -> &[Antecedent] { &self.antecedents }

    fn unify(self) -> Self::Unified {
        Box::pin(async move {
            self.admit()?;
            self.client.synthesize(&self.antecedents).await
        })
    }
}

// ── SceneArchive (public world) ───────────────────────────────────────────────

pub struct SceneArchive {
    scenes: Vec<UnifiedScene>,
}

impl SceneArchive {
    pub fn new() -> Self { Self { scenes: Vec::new() } }
    pub fn len(&self) -> usize { self.scenes.len() }
    pub fn is_empty(&self) -> bool { self.scenes.is_empty() }
    pub fn last(&self) -> Option<&UnifiedScene> { self.scenes.last() }
    pub fn iter(&self) -> impl Iterator<Item = &UnifiedScene> { self.scenes.iter() }
}

impl Default for SceneArchive {
    fn default() -> Self { Self::new() }
}

impl PublicWorld for SceneArchive {
    type Inhabitant = UnifiedScene;
    fn deposit(&mut self, x: UnifiedScene) { self.scenes.push(x); }
}

// ── Loaders ───────────────────────────────────────────────────────────────────

pub fn load_image(path: &Path) -> Result<ImagePrehension, ConcrescenceError> {
    let raw = std::fs::read(path)
        .map_err(|e| ConcrescenceError::Refused(format!("read failed: {e}")))?;
    let img = image::load_from_memory(&raw)
        .map_err(|e| ConcrescenceError::Refused(format!("image decode failed: {e}")))?;
    let (width, height) = (img.width(), img.height());
    let media_type = match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/png",
    };
    let estimated_tokens =
        ((width as u64 * height as u64) as f32 / 750.0).ceil().max(1.0) as u32;
    Ok(ImagePrehension {
        raw_bytes: Arc::new(raw),
        media_type,
        width,
        height,
        estimated_tokens,
    })
}

pub fn stub_ocr(_image: &ImagePrehension) -> OcrPrehension {
    OcrPrehension {
        text: "(stub OCR — wire tesseract-rs or remote OCR for real text)".into(),
        confidence: 0.0,
    }
}

pub fn stub_exif(path: &Path) -> ExifPrehension {
    ExifPrehension {
        fields: vec![
            ("source_path".into(), path.display().to_string()),
            ("provenance".into(), "stub".into()),
        ],
    }
}

/// Cap an inline image payload under `max_bytes`. Returns the bytes
/// borrowed when they already fit (zero copy), or a JPEG re-encode
/// downscaled to `max_side` on the longest axis when they don't.
/// Used by the Anthropic and Runway clients before base64-encoding.
#[cfg(any(feature = "anthropic-vision", feature = "runway-video"))]
pub fn cap_image<'a>(
    bytes: &'a [u8],
    media_type: &'static str,
    max_bytes: usize,
    max_side: u32,
    label: &str,
) -> Result<(std::borrow::Cow<'a, [u8]>, &'static str), ConcrescenceError> {
    if bytes.len() <= max_bytes {
        return Ok((std::borrow::Cow::Borrowed(bytes), media_type));
    }
    let decoded = image::load_from_memory(bytes)
        .map_err(|e| ConcrescenceError::Refused(format!("{label} decode: {e}")))?;
    let thumb = decoded.thumbnail(max_side, max_side);
    let mut buf: Vec<u8> = Vec::new();
    thumb
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Jpeg)
        .map_err(|e| ConcrescenceError::Refused(format!("{label} encode: {e}")))?;
    eprintln!(
        "[lux-vision] {label} cap: {} bytes → {}×{} jpeg ({} bytes)",
        bytes.len(),
        thumb.width(),
        thumb.height(),
        buf.len(),
    );
    Ok((std::borrow::Cow::Owned(buf), "image/jpeg"))
}

// ── Internal helpers ──────────────────────────────────────────────────────────

pub(crate) fn truncate(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}
