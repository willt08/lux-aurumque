//! Runway video client: text-to-video, image-to-video, character-performance.
//!
//! Requires feature `runway-video` and `RUNWAY_API_KEY` in env.
//! Override the base URL with `LUX_RUNWAY_BASE_URL` (default: api.dev.runwayml.com).

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use crate::pipeline::{VisionError, cap_image};

/// Runway video client. Talks to three endpoints: `text_to_video`,
/// `image_to_video`, and `character_performance`. Each submission
/// returns a [`TaskHandle` ] that the caller polls (or hands to
/// [`Self::wait_for_completion`]).
pub struct RunwayVideoClient {
    api_key: String,
    model: String,
    base_url: String,
    http: reqwest::Client,
}

impl RunwayVideoClient {
    /// `default_model` is used when `LUX_VIDEO_MODEL` is unset.
    /// Image paths default to `gen4_turbo`; text-only paths to `veo3.1_fast`.
    pub fn from_env(default_model: &str) -> Result<Self, VisionError> {
        let api_key = std::env::var("RUNWAY_API_KEY")
            .map_err(|_| VisionError::MissingEnv { var: "RUNWAY_API_KEY" })?;
        let model =
            std::env::var("LUX_VIDEO_MODEL").unwrap_or_else(|_| default_model.into());
        let base_url = std::env::var("LUX_RUNWAY_BASE_URL")
            .unwrap_or_else(|_| "https://api.dev.runwayml.com".into());
        Ok(Self { api_key, model, base_url, http: reqwest::Client::new() })
    }

    /// Submit a text-to-video task. Returns the queued [`TaskHandle`];
    /// poll `GET /v1/tasks/{id}` or call [`Self::wait_for_completion`]
    /// for the result.
    pub async fn text_to_video(
        &self,
        prompt: &str,
        ratio: &str,
        duration_secs: u32,
        audio: bool,
    ) -> Result<TaskHandle, VisionError> {
        const MAX: usize = 1000;
        let prompt = truncate_prompt(prompt, MAX);
        let body = TextRequest {
            prompt_text: prompt.as_ref(),
            ratio,
            audio,
            duration: duration_secs,
            model: &self.model,
        };
        self.post_task("/v1/text_to_video", &body).await
    }

    /// Submit an image-to-video task. The image anchors the visual
    /// identity; the prompt guides motion. Image is inlined as a
    /// `data:` URI, capped at 3.33 MB and downscaled when over.
    pub async fn image_to_video(
        &self,
        prompt: &str,
        image_bytes: &[u8],
        media_type: &'static str,
        ratio: &str,
        duration_secs: u32,
        audio: bool,
    ) -> Result<TaskHandle, VisionError> {
        const IMG_LIMIT: usize = (3.33 * 1024.0 * 1024.0) as usize;
        const IMG_MAX_SIDE: u32 = 2048;
        let (capped, mime) =
            cap_image(image_bytes, media_type, IMG_LIMIT, IMG_MAX_SIDE, "runway")?;
        let prompt_image =
            format!("data:{};base64,{}", mime, STANDARD.encode(capped.as_ref()));

        const PROMPT_MAX: usize = 1000;
        let prompt = truncate_prompt(prompt, PROMPT_MAX);
        let body = ImageRequest {
            prompt_text: prompt.as_ref(),
            prompt_image: &prompt_image,
            ratio,
            audio,
            duration: duration_secs,
            model: &self.model,
        };
        self.post_task("/v1/image_to_video", &body).await
    }

    /// Submit a character-performance task. Transfers the motion and
    /// expressions of `cfg.reference_video_bytes` onto the appearance
    /// of `cfg.character_image_bytes`. The model is supplied via
    /// [`CharacterPerformanceConfig`] — this endpoint uses a disjoint
    /// model namespace (`act_two`) from text/image routes, so
    /// `LUX_VIDEO_MODEL` must not bleed in here. Reference video is
    /// capped at 15 MB by default; override with
    /// `LUX_RUNWAY_VIDEO_MAX_BYTES`.
    pub async fn character_performance(
        &self,
        cfg: CharacterPerformanceConfig<'_>,
    ) -> Result<TaskHandle, VisionError> {
        const IMG_LIMIT: usize = (3.33 * 1024.0 * 1024.0) as usize;
        const IMG_MAX_SIDE: u32 = 2048;
        let (cap_char, cap_mime) = cap_image(
            cfg.character_image_bytes,
            cfg.character_media_type,
            IMG_LIMIT,
            IMG_MAX_SIDE,
            "runway",
        )?;
        let character_uri =
            format!("data:{};base64,{}", cap_mime, STANDARD.encode(cap_char.as_ref()));

        let max_video: usize = std::env::var("LUX_RUNWAY_VIDEO_MAX_BYTES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(15 * 1024 * 1024);
        if cfg.reference_video_bytes.len() > max_video {
            return Err(VisionError::OversizedInput {
                kind: "reference_video",
                bytes: cfg.reference_video_bytes.len(),
                cap: max_video,
            });
        }
        let reference_uri = format!(
            "data:{};base64,{}",
            cfg.reference_media_type,
            STANDARD.encode(cfg.reference_video_bytes),
        );

        let body = CharacterPerformanceRequest {
            model: cfg.model,
            character: Asset { kind: "image", uri: &character_uri },
            reference: Asset { kind: "video", uri: &reference_uri },
            ratio: cfg.ratio,
            body_control: cfg.body_control,
            expression_intensity: cfg.expression_intensity,
            content_moderation: ContentModeration {
                public_figure_threshold: cfg.public_figure_threshold,
            },
        };
        self.post_task("/v1/character_performance", &body).await
    }

    /// Poll a task once by id.
    pub async fn get_task(&self, task_id: &str) -> Result<TaskHandle, VisionError> {
        let resp = self
            .http
            .get(format!("{}/v1/tasks/{}", self.base_url, task_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("X-Runway-Version", "2024-11-06")
            .send()
            .await
            .map_err(|e| VisionError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(VisionError::Network(format!("{status}: {body}")));
        }
        resp.json::<TaskHandle>()
            .await
            .map_err(|e| VisionError::Network(format!("decode: {e}")))
    }

    /// Block until the task reaches a terminal state (SUCCEEDED, FAILED,
    /// or CANCELLED). Polls every 5 s; times out after 10 min.
    pub async fn wait_for_completion(
        &self,
        task_id: &str,
    ) -> Result<TaskHandle, VisionError> {
        let deadline = std::time::Duration::from_secs(600);
        let interval = std::time::Duration::from_secs(5);
        let started = std::time::Instant::now();
        loop {
            if started.elapsed() > deadline {
                return Err(VisionError::Network(format!(
                    "task {task_id}: timeout after {deadline:?}"
                )));
            }
            let task = self.get_task(task_id).await?;
            match task.status.as_deref().unwrap_or("") {
                "SUCCEEDED" => return Ok(task),
                "FAILED" | "CANCELLED" => {
                    return Err(VisionError::Network(format!(
                        "task {task_id}: {} ({})",
                        task.status.as_deref().unwrap_or(""),
                        task.failure.unwrap_or_default(),
                    )));
                }
                other => {
                    let pct = task.progress.unwrap_or(0.0) * 100.0;
                    eprintln!("[runway] task {task_id}: {other} ({pct:.0}%)");
                    tokio::time::sleep(interval).await;
                }
            }
        }
    }

    async fn post_task<B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<TaskHandle, VisionError> {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("X-Runway-Version", "2024-11-06")
            .json(body)
            .send()
            .await
            .map_err(|e| VisionError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(VisionError::Network(format!("{status}: {text}")));
        }
        resp.json::<TaskHandle>()
            .await
            .map_err(|e| VisionError::Network(format!("decode: {e}")))
    }
}

/// Configuration for [`RunwayVideoClient::character_performance`].
///
/// `model` must be from the character-performance namespace (currently
/// `act_two`). Use `LUX_CHARACTER_PERFORMANCE_MODEL` to override at
/// runtime rather than `LUX_VIDEO_MODEL`, which addresses a disjoint
/// endpoint.
pub struct CharacterPerformanceConfig<'a> {
    /// Runway model id for the character-performance endpoint. Use
    /// `"act_two"` unless Runway publishes a newer namespace entry.
    pub model: &'a str,
    /// Encoded bytes of the still image whose subject's appearance is
    /// transferred onto the reference video's motion. Capped at 3.33 MB
    /// and downscaled to ≤ 2048 px on the long axis before upload.
    pub character_image_bytes: &'a [u8],
    /// MIME type of `character_image_bytes`. Accepted values:
    /// `"image/png"`, `"image/jpeg"`, `"image/webp"`, `"image/gif"`.
    pub character_media_type: &'static str,
    /// Encoded bytes of the reference video supplying motion and
    /// expression. Capped at 15 MB by default; override via the
    /// `LUX_RUNWAY_VIDEO_MAX_BYTES` environment variable.
    pub reference_video_bytes: &'a [u8],
    /// MIME type of `reference_video_bytes`, e.g. `"video/mp4"`.
    pub reference_media_type: &'static str,
    /// Output aspect ratio as `"WIDTH:HEIGHT"` in pixels, e.g.
    /// `"1280:720"`, `"768:1280"`, `"960:960"`.
    pub ratio: &'a str,
    /// `true` to transfer body movement from the reference video;
    /// `false` to limit transfer to facial expression only.
    pub body_control: bool,
    /// Expression intensity in the range `0..=10`. Higher values
    /// amplify facial motion from the reference video.
    pub expression_intensity: u8,
    /// Content-moderation gate for public-figure resemblance. Accepted
    /// values: `"low"`, `"auto"`, `"high"`. Defaults to the
    /// `LUX_PUBLIC_FIGURE_THRESHOLD` environment variable when set.
    pub public_figure_threshold: &'a str,
}

/// A Runway task returned by `POST /v1/{endpoint}` and `GET /v1/tasks/{id}`.
/// All fields beyond `id` are optional so the client tolerates schema
/// drift.
#[derive(Debug, Deserialize)]
pub struct TaskHandle {
    /// Server-assigned task identifier.
    pub id: String,
    /// Current task state. Known values: `"PENDING"`, `"RUNNING"`,
    /// `"THROTTLED"`, `"SUCCEEDED"`, `"FAILED"`, `"CANCELLED"`. Treat
    /// unknown values as "still running".
    #[serde(default)]
    pub status: Option<String>,
    /// Output artefact URLs (typically a single signed video URL) once
    /// `status == "SUCCEEDED"`. Empty while in progress.
    #[serde(default)]
    pub output: Vec<String>,
    /// Progress in `[0.0, 1.0]` while running. `None` outside running.
    #[serde(default)]
    pub progress: Option<f32>,
    /// Human-readable failure reason when `status` is `"FAILED"` or
    /// `"CANCELLED"`; otherwise `None`.
    #[serde(default)]
    pub failure: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn truncate_prompt(s: &str, max_chars: usize) -> std::borrow::Cow<'_, str> {
    let total = s.chars().count();
    if total <= max_chars {
        return std::borrow::Cow::Borrowed(s);
    }
    let take = max_chars.saturating_sub(1);
    let prefix: String = s.chars().take(take).collect();
    let cut = prefix.rfind(char::is_whitespace).unwrap_or(prefix.len());
    let mut out = prefix[..cut].to_string();
    out.push('…');
    eprintln!(
        "[runway] prompt: {total} chars > {max_chars} char limit; truncated to {} chars",
        out.chars().count(),
    );
    std::borrow::Cow::Owned(out)
}

// ── Wire types ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct TextRequest<'a> {
    #[serde(rename = "promptText")]
    prompt_text: &'a str,
    ratio: &'a str,
    audio: bool,
    duration: u32,
    model: &'a str,
}

#[derive(Serialize)]
struct ImageRequest<'a> {
    #[serde(rename = "promptText")]
    prompt_text: &'a str,
    #[serde(rename = "promptImage")]
    prompt_image: &'a str,
    ratio: &'a str,
    audio: bool,
    duration: u32,
    model: &'a str,
}

#[derive(Serialize)]
struct CharacterPerformanceRequest<'a> {
    model: &'a str,
    character: Asset<'a>,
    reference: Asset<'a>,
    ratio: &'a str,
    #[serde(rename = "bodyControl")]
    body_control: bool,
    #[serde(rename = "expressionIntensity")]
    expression_intensity: u8,
    #[serde(rename = "contentModeration")]
    content_moderation: ContentModeration<'a>,
}

#[derive(Serialize)]
struct ContentModeration<'a> {
    #[serde(rename = "publicFigureThreshold")]
    public_figure_threshold: &'a str,
}

#[derive(Serialize)]
struct Asset<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    uri: &'a str,
}
