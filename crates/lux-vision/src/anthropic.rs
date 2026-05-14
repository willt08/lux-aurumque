//! Anthropic Claude vision client.
//!
//! Requires feature `anthropic-vision` and `ANTHROPIC_API_KEY` in env.
//! Override the model with `LUX_VISION_MODEL` (default: `claude-sonnet-4-6`).

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use crate::pipeline::{Input, Scene, VisionClient, VisionError, cap_image};

/// Anthropic Claude vision client. Implements [`VisionClient`] and adds
/// two helpers (`describe_image`, `translate_nexus`) used by the
/// `lux-vision` example pipeline for chained video continuation.
pub struct AnthropicVisionClient {
    api_key: String,
    model: String,
    max_tokens: u32,
    http: reqwest::Client,
}

impl AnthropicVisionClient {
    pub fn from_env() -> Result<Self, VisionError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| VisionError::MissingEnv { var: "ANTHROPIC_API_KEY" })?;
        let model = std::env::var("LUX_VISION_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-6".into());
        Ok(Self { api_key, model, max_tokens: 1024, http: reqwest::Client::new() })
    }

    /// Describe an image according to `directive`. Used for parallel
    /// character + loci extraction from a prior segment's final frame.
    pub async fn describe_image(
        &self,
        image_bytes: &[u8],
        media_type: &'static str,
        directive: &str,
    ) -> Result<String, VisionError> {
        const LIMIT: usize = 5 * 1024 * 1024;
        const MAX_SIDE: u32 = 1568;
        let (payload, mime) = cap_image(image_bytes, media_type, LIMIT, MAX_SIDE, "anthropic")?;
        let b64 = STANDARD.encode(payload.as_ref());

        let body = Request {
            model: &self.model,
            max_tokens: 400,
            messages: vec![Message {
                role: "user",
                content: vec![
                    ContentBlock::Image {
                        source: ImageSource { kind: "base64", media_type: mime, data: b64 },
                    },
                    ContentBlock::Text { text: directive },
                ],
            }],
        };
        let text = self.post_text(&body).await?;
        Ok(text)
    }

    /// Translate a structured JSON nexus + relational glossary into a
    /// caption-shaped video prompt (≤ 950 chars, no JSON syntax).
    pub async fn translate_nexus(
        &self,
        nexus_json: &str,
        relations: &str,
    ) -> Result<String, VisionError> {
        let prompt = format!(
            "Translate this structured scene description into a single-paragraph \
             video-generation prompt for a text-conditioned video diffusion model.\n\n\
             REQUIREMENTS:\n\
             - Strip ALL JSON syntax: no braces, no quoted field names, no `prehends:[]`. \
             The output must look like natural caption prose, not code.\n\
             - Preserve every physical commitment and temporal causality from the nexus.\n\
             - Use continuous motion verbs and explicit cadence (e.g. \"swings once per second\").\n\
             - Use cinematographic vocabulary for camera moves.\n\
             - Output a SINGLE PARAGRAPH of 600-900 characters. No preamble. No JSON. No bullet lists.\n\n\
             RELATIONAL CONTEXT (treat as ground-truth disambiguation):\n{relations}\n\n\
             NEXUS (the scene to render):\n{nexus_json}\n\n\
             Output the caption only, starting immediately with the scene description.",
        );
        let body = Request {
            model: &self.model,
            max_tokens: 512,
            messages: vec![Message {
                role: "user",
                content: vec![ContentBlock::Text { text: &prompt }],
            }],
        };
        self.post_text(&body).await
    }

    async fn post_text(&self, body: &Request<'_>) -> Result<String, VisionError> {
        let resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| VisionError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(VisionError::Network(format!("{status}: {body}")));
        }

        let parsed: Response =
            resp.json().await.map_err(|e| VisionError::Network(format!("decode: {e}")))?;

        Ok(parsed
            .content
            .iter()
            .filter(|b| b.kind == "text")
            .filter_map(|b| b.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string())
    }

    fn render_prompt(inputs: &[Input]) -> String {
        let mut p = String::from(
            "You receive an image alongside auxiliary inputs \
             (OCR text, EXIF metadata, optional audio transcript). \
             Synthesize one concise scene description that incorporates \
             all sources. Note any tensions between them.\n\n",
        );
        for a in inputs {
            match a {
                Input::Image(_) => {}
                Input::Ocr(o) => {
                    p.push_str(&format!("OCR (confidence {:.2}):\n{}\n\n", o.confidence, o.text));
                }
                Input::Exif(e) => {
                    p.push_str("EXIF:\n");
                    for (k, v) in &e.fields {
                        p.push_str(&format!("  {k} = {v}\n"));
                    }
                    p.push('\n');
                }
                Input::Audio(au) => {
                    p.push_str(&format!(
                        "Audio transcript ({:.1}s):\n{}\n\n",
                        au.duration_secs, au.transcript
                    ));
                }
            }
        }
        p
    }
}

#[async_trait]
impl VisionClient for AnthropicVisionClient {
    async fn synthesize(&self, inputs: &[Input]) -> Result<Scene, VisionError> {
        if inputs.is_empty() {
            return Err(VisionError::NoInputs);
        }
        let image = inputs
            .iter()
            .find_map(|a| match a {
                Input::Image(i) => Some(i),
                _ => None,
            })
            .ok_or(VisionError::MissingInput { kind: "image" })?;

        const LIMIT: usize = 5 * 1024 * 1024;
        const MAX_SIDE: u32 = 1568;
        let (payload, mime) =
            cap_image(image.raw_bytes.as_slice(), image.media_type, LIMIT, MAX_SIDE, "anthropic")?;
        let b64 = STANDARD.encode(payload.as_ref());
        let prompt = Self::render_prompt(inputs);

        let body = Request {
            model: &self.model,
            max_tokens: self.max_tokens,
            messages: vec![Message {
                role: "user",
                content: vec![
                    ContentBlock::Image {
                        source: ImageSource { kind: "base64", media_type: mime, data: b64 },
                    },
                    ContentBlock::Text { text: &prompt },
                ],
            }],
        };

        let resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| VisionError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(VisionError::Network(format!("{status}: {body}")));
        }

        let parsed: Response =
            resp.json().await.map_err(|e| VisionError::Network(format!("decode: {e}")))?;

        let caption = parsed
            .content
            .iter()
            .filter(|b| b.kind == "text")
            .filter_map(|b| b.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n");

        let total_tokens = parsed
            .usage
            .map(|u| u.input_tokens.saturating_add(u.output_tokens))
            .unwrap_or_else(|| inputs.iter().map(|a| a.token_weight()).sum());

        Ok(Scene { caption, contributing: inputs.len(), total_tokens })
    }
}

// ── Wire types ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: Vec<ContentBlock<'a>>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock<'a> {
    Text { text: &'a str },
    Image { source: ImageSource<'a> },
}

#[derive(Serialize)]
struct ImageSource<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    media_type: &'static str,
    data: String,
}

#[derive(Deserialize)]
struct Response {
    content: Vec<ResponseBlock>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct ResponseBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}
