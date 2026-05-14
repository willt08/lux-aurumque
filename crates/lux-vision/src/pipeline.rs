//! Core pipeline types: input modalities, the `VisionClient` trait, the
//! [`VisionPipeline`] builder, and the [`SpectralBudget`]-guarded `run`.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use spectral_budget::{BudgetError, SpectralBudget};

// ── Input modalities ─────────────────────────────────────────────────────────

/// Decoded image bytes packaged for inline upload to a vision model.
/// Produced by [`load_image`]; can also be built by hand when bytes come
/// from a non-file source (HTTP body, in-memory render, etc.).
#[derive(Clone, Debug)]
pub struct ImageInput {
    /// Original encoded bytes (PNG/JPEG/WebP/GIF). Shared via `Arc` so
    /// cloning the input is cheap; the underlying buffer is never
    /// mutated after construction.
    pub raw_bytes: Arc<Vec<u8>>,
    /// MIME type string sent on the wire. Accepted values:
    /// `"image/png"`, `"image/jpeg"`, `"image/webp"`, `"image/gif"`.
    pub media_type: &'static str,
    /// Width in pixels of the decoded image.
    pub width: u32,
    /// Height in pixels of the decoded image.
    pub height: u32,
    /// Coarse token budget estimate, `ceil(w·h / 750)`. Used by
    /// [`SpectralBudget`] admission before the inference call fires.
    /// Override if your model uses a different tokenisation rule.
    pub estimated_tokens: u32,
}

/// OCR text recovered from an image, with a confidence hint.
#[derive(Clone, Debug)]
pub struct OcrInput {
    /// Recognised text. May contain newlines; the synthesiser sees it
    /// verbatim.
    pub text: String,
    /// Confidence in `[0.0, 1.0]`. Producer-defined; `0.0` is the
    /// conventional placeholder when no real confidence is available
    /// (e.g. [`stub_ocr`]).
    pub confidence: f32,
}

/// Image metadata fields (typically EXIF, but any key/value pairs work).
#[derive(Clone, Debug)]
pub struct ExifInput {
    /// Ordered list of `(key, value)` pairs. Order is preserved in the
    /// rendered prompt so callers can prioritise important fields first.
    pub fields: Vec<(String, String)>,
}

/// Transcribed audio aligned with a visual scene (e.g. video soundtrack,
/// narration, or accompanying recording).
#[derive(Clone, Debug)]
pub struct AudioInput {
    /// Plain-text transcript of the audio segment.
    pub transcript: String,
    /// Duration of the source audio in seconds. Used by the synthesiser
    /// to weight temporal context, not for re-rendering.
    pub duration_secs: f32,
}

/// One heterogeneous input modality, packaged as a uniform enum so the
/// pipeline can iterate over them.
#[derive(Clone, Debug)]
pub enum Input {
    Image(ImageInput),
    Ocr(OcrInput),
    Exif(ExifInput),
    Audio(AudioInput),
}

impl Input {
    /// Token weight used by [`SpectralBudget`] admission. The image
    /// estimate is a proxy (`w·h / 750`); text estimates use `bytes/4`.
    pub fn token_weight(&self) -> u32 {
        match self {
            Input::Image(i) => i.estimated_tokens,
            Input::Ocr(o) => (o.text.len() as u32).div_ceil(4),
            Input::Exif(e) => e
                .fields
                .iter()
                .map(|(k, v)| ((k.len() + v.len()) as u32) / 4 + 1)
                .sum::<u32>()
                .max(8),
            Input::Audio(a) => (a.transcript.len() as u32).div_ceil(4),
        }
    }
}

// ── Scene (the synthesised output) ────────────────────────────────────────────

/// The output of one pipeline run: a single textual scene description
/// fused from all inputs. Deposit into a [`SceneArchive`] when chaining.
#[derive(Clone, Debug)]
pub struct Scene {
    /// The synthesised description. For [`MockVisionClient`] this is a
    /// deterministic summary of the inputs; for production clients
    /// (Anthropic, etc.) it is the model's caption verbatim.
    pub caption: String,
    /// Number of inputs that participated in this synthesis.
    pub contributing: usize,
    /// Total token cost: for the mock client this is the sum of
    /// [`Input::token_weight`]; for real clients it is the upstream
    /// usage report (`input_tokens + output_tokens`) when available,
    /// falling back to the same estimate.
    pub total_tokens: u32,
}

// ── Error type ────────────────────────────────────────────────────────────────

/// Failure modes of the vision pipeline. `#[non_exhaustive]` — match
/// with a wildcard arm if you want to remain forward-compatible.
#[derive(Debug)]
#[non_exhaustive]
pub enum VisionError {
    /// Aggregate input diameter exceeded the [`SpectralBudget`].
    Budget(BudgetError),
    /// The pipeline was run with no inputs.
    NoInputs,
    /// A required environment variable (API key, base URL override) is
    /// not set.
    MissingEnv {
        /// Name of the environment variable that was expected.
        var: &'static str,
    },
    /// I/O failure reading an input from disk.
    Io(std::io::Error),
    /// Could not decode an input image.
    ImageDecode {
        /// Call-site label, useful when one pipeline decodes multiple images.
        label: &'static str,
        /// Underlying decoder message, stringified.
        message: String,
    },
    /// Could not re-encode a downscaled image while capping its size.
    ImageEncode {
        /// Call-site label.
        label: &'static str,
        /// Underlying encoder message, stringified.
        message: String,
    },
    /// A specific input kind was required but not supplied — e.g. the
    /// Anthropic client needs an [`Input::Image`].
    MissingInput {
        /// Kind of input the client required (`"image"`, etc.).
        kind: &'static str,
    },
    /// An input payload exceeded its configured byte cap.
    OversizedInput {
        /// Which input was oversized (`"reference_video"`, etc.).
        kind: &'static str,
        /// Actual size in bytes.
        bytes: usize,
        /// Configured ceiling in bytes.
        cap: usize,
    },
    /// Upstream HTTP / transport-layer failure.
    Network(String),
}

impl std::fmt::Display for VisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisionError::Budget(e) => write!(f, "{e}"),
            VisionError::NoInputs => write!(f, "no inputs to run"),
            VisionError::MissingEnv { var } => write!(f, "environment variable not set: {var}"),
            VisionError::Io(e) => write!(f, "i/o: {e}"),
            VisionError::ImageDecode { label, message } => {
                write!(f, "image decode ({label}): {message}")
            }
            VisionError::ImageEncode { label, message } => {
                write!(f, "image encode ({label}): {message}")
            }
            VisionError::MissingInput { kind } => write!(f, "required input missing: {kind}"),
            VisionError::OversizedInput { kind, bytes, cap } => {
                write!(f, "{kind} oversize: {bytes} bytes > {cap} byte cap")
            }
            VisionError::Network(s) => write!(f, "network: {s}"),
        }
    }
}

impl std::error::Error for VisionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VisionError::Budget(e) => Some(e),
            VisionError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<BudgetError> for VisionError {
    fn from(e: BudgetError) -> Self { VisionError::Budget(e) }
}

impl From<std::io::Error> for VisionError {
    fn from(e: std::io::Error) -> Self { VisionError::Io(e) }
}

// ── VisionClient trait ────────────────────────────────────────────────────────

/// The mover that turns a slice of inputs into a synthesised [`Scene`].
/// Implement this trait to plug any vision model into the pipeline.
/// [`MockVisionClient`] is the network-free reference implementation.
#[async_trait]
pub trait VisionClient: Send + Sync {
    async fn synthesize(&self, inputs: &[Input]) -> Result<Scene, VisionError>;
}

// ── Mock client ───────────────────────────────────────────────────────────────

/// Deterministic, network-free client. Demonstrates the shape of a
/// synthesis without requiring API keys. Swap in
/// `anthropic::AnthropicVisionClient` (feature `anthropic-vision`) for
/// production use; the rest of the pipeline is untouched.
pub struct MockVisionClient;

#[async_trait]
impl VisionClient for MockVisionClient {
    async fn synthesize(&self, inputs: &[Input]) -> Result<Scene, VisionError> {
        if inputs.is_empty() {
            return Err(VisionError::NoInputs);
        }
        let mut parts: Vec<String> = Vec::with_capacity(inputs.len());
        let mut total_tokens: u32 = 0;
        for a in inputs {
            total_tokens = total_tokens.saturating_add(a.token_weight());
            parts.push(match a {
                Input::Image(i) => {
                    format!("[image {}x{} ~{}tok]", i.width, i.height, i.estimated_tokens)
                }
                Input::Ocr(o) => {
                    format!("[ocr conf={:.2}: {:?}]", o.confidence, truncate(&o.text, 64))
                }
                Input::Exif(e) => format!("[exif: {} fields]", e.fields.len()),
                Input::Audio(a) => format!(
                    "[audio {:.1}s: {:?}]",
                    a.duration_secs,
                    truncate(&a.transcript, 64)
                ),
            });
        }
        Ok(Scene {
            caption: format!(
                "synthesised from {} inputs: {}",
                inputs.len(),
                parts.join(" + "),
            ),
            contributing: inputs.len(),
            total_tokens,
        })
    }
}

// ── VisionPipeline ────────────────────────────────────────────────────────────

/// Builder-style pipeline: accumulate inputs, then `run` to synthesise
/// a [`Scene`]. Admission against the [`SpectralBudget`] happens before
/// the inference call fires, so over-budget runs fail cheaply.
pub struct VisionPipeline {
    inputs: Vec<Input>,
    client: Arc<dyn VisionClient>,
    budget: SpectralBudget,
}

impl VisionPipeline {
    /// Create a new pipeline bound to a client and a budget. The client
    /// is shared (`Arc`) so the same backend can serve many concurrent
    /// pipelines.
    #[must_use]
    pub fn new(client: Arc<dyn VisionClient>, budget: SpectralBudget) -> Self {
        Self { inputs: Vec::new(), client, budget }
    }

    /// Add one input. Chains for builder-style construction.
    #[must_use]
    pub fn with(mut self, input: Input) -> Self {
        self.inputs.push(input);
        self
    }

    /// View the inputs accumulated so far (e.g. for logging).
    pub fn inputs(&self) -> &[Input] { &self.inputs }

    /// Aggregate token diameter of the inputs — the quantity checked
    /// against the [`SpectralBudget`] in [`Self::run`].
    pub fn diameter(&self) -> f64 {
        self.inputs
            .iter()
            .map(|a| a.token_weight() as u64)
            .sum::<u64>() as f64
    }

    fn admit(&self) -> Result<(), VisionError> {
        self.budget.try_admit(self.diameter())?;
        Ok(())
    }

    /// Run the pipeline: admit against the budget, then call the
    /// client. Errors short-circuit before the network call when the
    /// budget rejects.
    pub async fn run(self) -> Result<Scene, VisionError> {
        self.admit()?;
        self.client.synthesize(&self.inputs).await
    }
}

// ── SceneArchive ──────────────────────────────────────────────────────────────

/// Append-only history of synthesised scenes — useful when chaining
/// pipelines (each run's caption seeds the next).
pub struct SceneArchive {
    scenes: Vec<Scene>,
}

impl SceneArchive {
    pub fn new() -> Self { Self { scenes: Vec::new() } }
    pub fn push(&mut self, scene: Scene) { self.scenes.push(scene); }
    pub fn len(&self) -> usize { self.scenes.len() }
    pub fn is_empty(&self) -> bool { self.scenes.is_empty() }
    pub fn last(&self) -> Option<&Scene> { self.scenes.last() }
    pub fn iter(&self) -> impl Iterator<Item = &Scene> { self.scenes.iter() }
}

impl Default for SceneArchive {
    fn default() -> Self { Self::new() }
}

// ── Loaders ───────────────────────────────────────────────────────────────────

/// Load an image from disk into an [`ImageInput`]. Decodes the bytes to
/// extract width/height for the token-weight estimate; the original
/// encoded bytes are sent on the wire.
pub fn load_image(path: &Path) -> Result<ImageInput, VisionError> {
    let raw = std::fs::read(path)?;
    let img = image::load_from_memory(&raw).map_err(|e| VisionError::ImageDecode {
        label: "load_image",
        message: e.to_string(),
    })?;
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
    Ok(ImageInput {
        raw_bytes: Arc::new(raw),
        media_type,
        width,
        height,
        estimated_tokens,
    })
}

/// Placeholder OCR for when real OCR isn't wired up — returns a marker
/// string at confidence 0.0 so the pipeline runs end-to-end without
/// special-casing the inputs list.
pub fn stub_ocr(_image: &ImageInput) -> OcrInput {
    OcrInput {
        text: "(stub OCR — wire tesseract-rs or remote OCR for real text)".into(),
        confidence: 0.0,
    }
}

/// Placeholder EXIF — emits a minimal `source_path` / `provenance`
/// pair so the metadata channel exercises end-to-end.
pub fn stub_exif(path: &Path) -> ExifInput {
    ExifInput {
        fields: vec![
            ("source_path".into(), path.display().to_string()),
            ("provenance".into(), "stub".into()),
        ],
    }
}

/// Cap an inline image payload under `max_bytes`. Returns the bytes
/// borrowed when they already fit (zero copy), or a JPEG re-encode
/// downscaled to `max_side` on the longest axis when they don't. Used
/// by the Anthropic and Runway clients before base64-encoding.
#[cfg(any(feature = "anthropic-vision", feature = "runway-video"))]
pub fn cap_image<'a>(
    bytes: &'a [u8],
    media_type: &'static str,
    max_bytes: usize,
    max_side: u32,
    label: &'static str,
) -> Result<(std::borrow::Cow<'a, [u8]>, &'static str), VisionError> {
    if bytes.len() <= max_bytes {
        return Ok((std::borrow::Cow::Borrowed(bytes), media_type));
    }
    let decoded = image::load_from_memory(bytes).map_err(|e| VisionError::ImageDecode {
        label,
        message: e.to_string(),
    })?;
    let thumb = decoded.thumbnail(max_side, max_side);
    let mut buf: Vec<u8> = Vec::new();
    thumb
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Jpeg)
        .map_err(|e| VisionError::ImageEncode {
            label,
            message: e.to_string(),
        })?;
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
