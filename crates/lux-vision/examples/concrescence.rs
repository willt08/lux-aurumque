//! concrescence — pipeline orchestrator example.
//!
//! CLI driving `lux-vision`. All types and clients live in the library
//! (`lux_vision::{pipeline, anthropic, runway, shape}`); this file
//! wires them into a runnable program.
//!
//! Modes (selected automatically by the arguments provided):
//!
//!   <image.png>                          describe → Runway image_to_video
//!   <image.png> --prompt-shape <shape>   image-anchored shape mode (skip describe)
//!   --prompt-shape <shape>               text_to_video with preset
//!   (no args, feature enabled)           text_to_video, stdin prompt
//!   --continue-from <task-id>            chain from a prior Runway task
//!   --continue-from-file <path.mp4>      chain from a local video file
//!     (with --prompt-shape)              → image_to_video + Claude extraction
//!     (without --prompt-shape)           → character_performance
//!
//! Build (mock, no network):
//!     cargo run --example concrescence -- <image.png>
//!
//! Build (full pipeline):
//!     cargo run --example concrescence --features full -- <image.png>

use std::path::PathBuf;
#[cfg(feature = "runway-video")]
use std::path::Path;
use std::sync::Arc;

use spectral_budget::SpectralBudget;
use lux_vision::{
    Input, MockVisionClient, SceneArchive, VisionClient, VisionPipeline,
    load_image, stub_exif, stub_ocr,
};

#[cfg(feature = "anthropic-vision")]
use lux_vision::anthropic::AnthropicVisionClient;

#[cfg(feature = "runway-video")]
use lux_vision::runway::{CharacterPerformanceConfig, RunwayVideoClient, TaskHandle};

#[cfg(feature = "runway-video")]
use lux_vision::shape::PromptShape;

// ── Directives for parallel character + loci extraction ──────────────────────

#[cfg(feature = "anthropic-vision")]
const CHARACTER_DIRECTIVE: &str =
    "List each visible character (person, animal, etc.) in this frame. \
     For each: appearance, posture, current action, what they're attending to. \
     Stay literal — describe what is shown, not what might happen next. \
     Output a single concise paragraph (≤300 chars), no list markers.";

#[cfg(feature = "anthropic-vision")]
const LOCI_DIRECTIVE: &str =
    "Describe the spatial setting of this frame: room/space layout, key \
     objects and their positions, lighting (source, color, mood), camera \
     position relative to subjects, environmental atmosphere. Stay literal — \
     no narrative interpretation. Output a single concise paragraph (≤300 chars).";

// ── Client selection ──────────────────────────────────────────────────────────

fn build_client() -> Arc<dyn VisionClient> {
    #[cfg(feature = "anthropic-vision")]
    {
        match AnthropicVisionClient::from_env() {
            Ok(c) => {
                eprintln!("[vision] AnthropicVisionClient ready.");
                return Arc::new(c);
            }
            Err(e) => eprintln!("[vision] Anthropic init failed ({e}); falling back to mock."),
        }
    }
    Arc::new(MockVisionClient)
}

// ── Nexus translation seam ────────────────────────────────────────────────────

/// If `shape` has a glossary, translate the JSON nexus into caption prose via
/// Claude. Falls back to the raw JSON (with a warning) when ANTHROPIC_API_KEY
/// is absent — the video model will see JSON, risking text overlay.
#[cfg(feature = "runway-video")]
async fn maybe_translate_nexus(
    prompt: String,
    shape: Option<PromptShape>,
) -> Result<String, Box<dyn std::error::Error>> {
    let Some(shape) = shape else { return Ok(prompt); };
    let Some(glossary) = shape.glossary() else { return Ok(prompt); };

    #[cfg(not(feature = "anthropic-vision"))]
    {
        eprintln!(
            "[vision] no anthropic-vision feature: skipping {} nexus translation",
            shape.label()
        );
        return Ok(prompt);
    }

    #[cfg(feature = "anthropic-vision")]
    {
        if std::env::var("ANTHROPIC_API_KEY").is_err() {
            eprintln!(
                "[vision] no ANTHROPIC_API_KEY: skipping {} nexus translation \
                 (sending raw JSON — text-overlay risk)",
                shape.label()
            );
            return Ok(prompt);
        }
        let claude = AnthropicVisionClient::from_env()?;
        eprintln!("[vision] translating {} nexus via Claude…", shape.label());
        let translated = claude.translate_nexus(&prompt, glossary).await?;
        eprintln!(
            "[vision] {} nexus: {} → {} chars",
            shape.label(),
            prompt.len(),
            translated.len()
        );
        Ok(translated)
    }
}

// ── Continuation helpers ──────────────────────────────────────────────────────

#[cfg(feature = "runway-video")]
fn video_duration_secs() -> u32 {
    std::env::var("LUX_VIDEO_DURATION_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4)
}

/// Download a Runway video URL to a temp file, return local path.
#[cfg(feature = "runway-video")]
async fn download_video(url: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!(
        "runway_{}_{}.mp4",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    let resp = reqwest::get(url).await?;
    if !resp.status().is_success() {
        return Err(format!("download status {}", resp.status()).into());
    }
    std::fs::write(&path, resp.bytes().await?)?;
    Ok(path)
}

/// Use ffmpeg to extract the final frame of an mp4 as JPEG.
#[cfg(feature = "runway-video")]
fn extract_final_frame(mp4_path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let frame_path = mp4_path.with_extension("final.jpg");
    let out = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-sseof", "-0.1",
            "-i", mp4_path.to_str().ok_or("non-utf8 path")?,
            "-update", "1",
            "-q:v", "2",
            frame_path.to_str().ok_or("non-utf8 path")?,
        ])
        .output()?;
    if !out.status.success() {
        return Err(format!(
            "ffmpeg failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    Ok(frame_path)
}

#[cfg(feature = "runway-video")]
fn extract_and_load_final_frame(
    mp4_path: &Path,
) -> Result<lux_vision::ImageInput, Box<dyn std::error::Error>> {
    eprintln!("[vision] extracting final frame via ffmpeg…");
    let frame_path = extract_final_frame(mp4_path)?;
    let frame = load_image(&frame_path)?;
    eprintln!(
        "[vision] final frame: {} bytes, {}×{}, {}",
        frame.raw_bytes.len(),
        frame.width,
        frame.height,
        frame.media_type,
    );
    Ok(frame)
}

// ── Pipeline branches ─────────────────────────────────────────────────────────

/// `--continue-from <task-id>`: download the prior Runway task's output,
/// extract the final frame, then call `continue_from_frame`.
#[cfg(feature = "full")]
async fn continue_from_run(
    prior_task_id: String,
    prompt_shape: Option<PromptShape>,
) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RUNWAY_API_KEY").is_err() {
        return Err("--continue-from requires RUNWAY_API_KEY".into());
    }
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        return Err("--continue-from requires ANTHROPIC_API_KEY for character/loci extraction".into());
    }
    eprintln!("[vision] polling prior task: {prior_task_id}");
    let rw = RunwayVideoClient::from_env("gen4.5")?;
    let task = rw.wait_for_completion(&prior_task_id).await?;
    let video_url = task
        .output
        .into_iter()
        .next()
        .ok_or("task SUCCEEDED but output is empty")?;
    eprintln!("[vision] downloading prior video: {video_url}");
    let mp4_path = download_video(&video_url).await?;
    let frame = extract_and_load_final_frame(&mp4_path)?;
    continue_from_frame(frame, &prior_task_id, prompt_shape).await
}

/// Core continuation: extract character + loci from `frame` via Claude,
/// compose with `prompt_shape` directive, submit image_to_video.
#[cfg(feature = "full")]
async fn continue_from_frame(
    frame: lux_vision::ImageInput,
    prior_label: &str,
    prompt_shape: Option<PromptShape>,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[vision] extracting character + loci in parallel…");
    let claude = AnthropicVisionClient::from_env()?;
    let (character, loci) = tokio::try_join!(
        claude.describe_image(&frame.raw_bytes, frame.media_type, CHARACTER_DIRECTIVE),
        claude.describe_image(&frame.raw_bytes, frame.media_type, LOCI_DIRECTIVE),
    )?;
    eprintln!("[vision] character: {character}");
    eprintln!("[vision] loci: {loci}");

    let directive = if let Some(shape) = prompt_shape {
        eprintln!("[vision] using --prompt-shape={} preset", shape.label());
        shape.text().to_string()
    } else {
        println!();
        println!("─── Continuation directive ───────────────────────────");
        println!("Inherited character : {character}");
        println!("Inherited loci      : {loci}");
        println!();
        println!("Enter directive for the next segment (one line):");
        print!("> ");
        use std::io::{BufRead, Write};
        std::io::stdout().flush()?;
        let mut buf = String::new();
        std::io::stdin().lock().read_line(&mut buf)?;
        let line = buf.trim().to_string();
        if line.is_empty() {
            return Err("empty directive".into());
        }
        line
    };

    let composed = format!(
        "Continuation segment. Inherit visual continuity from the prior frame.\n\n\
         Inherited character: {character}\n\n\
         Inherited loci: {loci}\n\n\
         Directive for this segment: {directive}"
    );
    let final_prompt = maybe_translate_nexus(composed, prompt_shape).await?;

    let rw = RunwayVideoClient::from_env("gen4.5")?;
    eprintln!("[vision] queueing continuation segment via image_to_video…");
    let dur = video_duration_secs();
    let task = rw
        .image_to_video(&final_prompt, &frame.raw_bytes, frame.media_type, "1280:720", dur, true)
        .await?;
    println!("Continuation task queued:");
    println!("  prior  : {prior_label}");
    println!("  next   : {}", task.id);
    println!("  status : {:?}", task.status);
    println!("  poll   : GET /v1/tasks/{}", task.id);
    Ok(())
}

/// `--continue-from-file <path.mp4>`:
///   with `--prompt-shape` → image_to_video (Claude extraction + shape directive)
///   without               → character_performance (motion transfer, no prompt)
#[cfg(feature = "full")]
async fn continue_from_file_run(
    mp4_path: PathBuf,
    prompt_shape: Option<PromptShape>,
) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RUNWAY_API_KEY").is_err() {
        return Err("--continue-from-file requires RUNWAY_API_KEY".into());
    }
    if !mp4_path.exists() {
        return Err(format!("--continue-from-file: no such file: {}", mp4_path.display()).into());
    }

    let skip_claude = std::env::var("LUX_SKIP_CLAUDE_INHERITANCE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if prompt_shape.is_some() && skip_claude {
        return Err(
            "--prompt-shape composes with Anthropic extraction; \
             unset LUX_SKIP_CLAUDE_INHERITANCE to use a shape preset"
                .into(),
        );
    }
    if !skip_claude && std::env::var("ANTHROPIC_API_KEY").is_err() {
        return Err(
            "--continue-from-file requires ANTHROPIC_API_KEY \
             (or set LUX_SKIP_CLAUDE_INHERITANCE=1 — note: a --prompt-shape preset \
             requires the Anthropic leg)"
                .into(),
        );
    }

    let frame = extract_and_load_final_frame(&mp4_path)?;

    // Prompt-shape set → image_to_video: the prompt is load-bearing.
    if prompt_shape.is_some() {
        eprintln!(
            "[vision] --continue-from-file + --prompt-shape → image_to_video: {}",
            mp4_path.display()
        );
        return continue_from_frame(frame, &mp4_path.display().to_string(), prompt_shape).await;
    }

    // No shape → character_performance: motion transferred from the reference.
    eprintln!(
        "[vision] --continue-from-file → character_performance: {}",
        mp4_path.display()
    );

    let video_bytes = std::fs::read(&mp4_path)
        .map_err(|e| format!("read {}: {e}", mp4_path.display()))?;
    eprintln!("[vision] reference video: {} bytes", video_bytes.len());

    if !skip_claude {
        eprintln!("[vision] extracting character + loci (informational)…");
        let claude = AnthropicVisionClient::from_env()?;
        let (character, loci) = tokio::try_join!(
            claude.describe_image(&frame.raw_bytes, frame.media_type, CHARACTER_DIRECTIVE),
            claude.describe_image(&frame.raw_bytes, frame.media_type, LOCI_DIRECTIVE),
        )?;
        println!();
        println!("─── Inheritance metadata (informational) ─────────────");
        println!("Character : {character}");
        println!("Loci      : {loci}");
        println!();
    }

    let cp_model = std::env::var("LUX_CHARACTER_PERFORMANCE_MODEL")
        .unwrap_or_else(|_| "act_two".into());
    if std::env::var("LUX_VIDEO_MODEL").is_ok() {
        eprintln!(
            "[vision] note: LUX_VIDEO_MODEL is ignored on this path; \
             character_performance uses LUX_CHARACTER_PERFORMANCE_MODEL (default: act_two)"
        );
    }
    let rw = RunwayVideoClient::from_env(&cp_model)?;
    let public_figure = std::env::var("LUX_PUBLIC_FIGURE_THRESHOLD")
        .unwrap_or_else(|_| "low".into());

    eprintln!(
        "[vision] queueing character_performance ({cp_model}, \
         publicFigureThreshold={public_figure})…"
    );
    let task = rw
        .character_performance(CharacterPerformanceConfig {
            model: &cp_model,
            character_image_bytes: &frame.raw_bytes,
            character_media_type: frame.media_type,
            reference_video_bytes: &video_bytes,
            reference_media_type: "video/mp4",
            ratio: "1280:720",
            body_control: true,
            expression_intensity: 3,
            public_figure_threshold: &public_figure,
        })
        .await?;

    println!("Character-performance task queued:");
    println!("  prior  : {}", mp4_path.display());
    println!("  next   : {}", task.id);
    println!("  status : {:?}", task.status);
    println!("  poll   : GET /v1/tasks/{}", task.id);
    Ok(())
}

/// `--prompt-shape <shape>` with a path arg: image anchors the visual
/// identity; the shape preset drives motion. Claude describe leg skipped.
#[cfg(feature = "full")]
async fn image_shape_run(
    path: PathBuf,
    shape: PromptShape,
) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RUNWAY_API_KEY").is_err() {
        return Err("--prompt-shape with an image requires RUNWAY_API_KEY".into());
    }
    let image = load_image(&path)?;
    eprintln!("[vision] image-anchored shape mode: --prompt-shape={}", shape.label());
    let prompt = maybe_translate_nexus(shape.text().to_string(), Some(shape)).await?;
    let rw = RunwayVideoClient::from_env("gen4_turbo")?;
    let dur = video_duration_secs();
    eprintln!("[vision] queueing image_to_video — frame as init, shape as guide…");
    let task = rw
        .image_to_video(&prompt, &image.raw_bytes, image.media_type, "1280:720", dur, true)
        .await?;
    print_task("Runway task queued", &task);
    Ok(())
}

/// No path arg: pure text-to-video. Prompt from stdin or shape preset.
#[cfg(feature = "runway-video")]
async fn text_only_run(
    prompt_shape: Option<PromptShape>,
) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RUNWAY_API_KEY").is_err() {
        return Err("text-only mode requires RUNWAY_API_KEY".into());
    }
    let prompt = if let Some(shape) = prompt_shape {
        eprintln!("[vision] using --prompt-shape={} preset", shape.label());
        shape.text().to_string()
    } else {
        let p = read_text_prompt()?;
        if p.is_empty() {
            return Err("empty prompt".into());
        }
        p
    };
    let prompt = maybe_translate_nexus(prompt, prompt_shape).await?;
    let rw = RunwayVideoClient::from_env("veo3.1_fast")?;
    let dur = video_duration_secs();
    eprintln!("[vision] queueing text_to_video…");
    let task = rw.text_to_video(&prompt, "1280:720", dur, true).await?;
    print_task("Runway task queued", &task);
    Ok(())
}

// ── CLI helpers ───────────────────────────────────────────────────────────────

#[cfg(feature = "runway-video")]
fn print_task(label: &str, task: &TaskHandle) {
    println!("{label}:");
    println!("  id     : {}", task.id);
    println!("  status : {:?}", task.status);
    println!("  poll   : GET /v1/tasks/{}", task.id);
}

#[cfg(feature = "runway-video")]
fn read_text_prompt() -> std::io::Result<String> {
    use std::io::{self, BufRead, Write};
    println!();
    println!("─── Text-only mode ───────────────────────────────────");
    println!("Enter video prompt (one line, paste-friendly).");
    print!("> ");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().lock().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

#[cfg(feature = "runway-video")]
fn read_runway_prompt(caption: &str) -> std::io::Result<String> {
    use std::io::{self, BufRead, Write};
    let preview: String = caption.chars().take(200).collect();
    let elided = if caption.chars().count() > 200 { "…" } else { "" };
    println!();
    println!("─── Runway prompt ────────────────────────────────────");
    println!("Claude caption preview:");
    println!("  {preview}{elided}");
    println!();
    println!("Enter prompt (one line). Press enter alone to use the caption above.");
    print!("> ");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().lock().read_line(&mut buf)?;
    let typed = buf.trim();
    Ok(if typed.is_empty() { caption.to_string() } else { typed.to_string() })
}

fn parse_cli() -> Result<(Option<String>, Option<String>, Option<String>, Option<String>), String> {
    let mut path: Option<String> = None;
    let mut shape: Option<String> = None;
    let mut continue_from: Option<String> = None;
    let mut continue_from_file: Option<String> = None;
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--prompt-shape" => {
                shape = Some(iter.next().ok_or_else(|| {
                    "--prompt-shape requires a value: json|prose|bare|fly|reverence|lux"
                        .to_string()
                })?);
            }
            "--continue-from" => {
                continue_from = Some(iter.next().ok_or_else(|| {
                    "--continue-from requires a Runway task id".to_string()
                })?);
            }
            "--continue-from-file" => {
                continue_from_file = Some(iter.next().ok_or_else(|| {
                    "--continue-from-file requires a path to a local .mp4".to_string()
                })?);
            }
            "--help" | "-h" => {
                println!(
                    "usage: concrescence [<image.png>] \
                     [--prompt-shape json|prose|bare|fly|reverence|lux] \
                     [--continue-from <task-id>] \
                     [--continue-from-file <path.mp4>]\n\
                     \n  <image.png>                    describe → image_to_video\
                     \n  <image.png> + --prompt-shape   image-anchored shape mode\
                     \n  --prompt-shape                 text_to_video with preset\
                     \n  (no args)                      text_to_video, stdin prompt\
                     \n  --continue-from <id>           chain from prior Runway task\
                     \n  --continue-from-file <mp4>     chain from local video\
                     \n    + --prompt-shape fly          → image_to_video (prompt drives)\
                     \n    + --prompt-shape lux          → image_to_video (light decomp)\
                     \n    (no shape)                    → character_performance"
                );
                std::process::exit(0);
            }
            other if !other.starts_with('-') => path = Some(other.to_string()),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok((path, shape, continue_from, continue_from_file))
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let (path_arg, shape_arg, continue_from, continue_from_file) =
        parse_cli().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    #[cfg(feature = "runway-video")]
    let prompt_shape: Option<PromptShape> = match shape_arg {
        Some(s) => Some(PromptShape::from_arg(&s)?),
        None => None,
    };
    #[cfg(not(feature = "runway-video"))]
    if shape_arg.is_some() {
        return Err("--prompt-shape requires --features runway-video (or full)".into());
    }

    #[cfg(not(feature = "full"))]
    if continue_from.is_some() || continue_from_file.is_some() {
        return Err(
            "--continue-from / --continue-from-file require --features full".into(),
        );
    }

    if continue_from.is_some() && continue_from_file.is_some() {
        return Err("--continue-from and --continue-from-file are mutually exclusive".into());
    }

    #[cfg(feature = "full")]
    if let Some(prior_id) = continue_from {
        if path_arg.is_some() {
            return Err("--continue-from takes its frame from the prior task; omit <image.png>".into());
        }
        return continue_from_run(prior_id, prompt_shape).await;
    }

    #[cfg(feature = "full")]
    if let Some(file) = continue_from_file {
        if path_arg.is_some() {
            return Err("--continue-from-file takes its frame from the mp4; omit <image.png>".into());
        }
        return continue_from_file_run(PathBuf::from(file), prompt_shape).await;
    }

    #[cfg(feature = "runway-video")]
    if path_arg.is_none() {
        return text_only_run(prompt_shape).await;
    }

    #[cfg(feature = "full")]
    if let (Some(p), Some(shape)) = (path_arg.as_ref(), prompt_shape) {
        return image_shape_run(PathBuf::from(p), shape).await;
    }

    // ── Standard path: describe → archive → Runway ────────────────────────

    let path = path_arg.ok_or("usage: concrescence <image.png>")?;
    let path = PathBuf::from(path);

    let image = load_image(&path)?;
    #[cfg(feature = "runway-video")]
    let image_for_runway = image.clone();
    #[cfg(not(feature = "runway-video"))]
    let _ = &image;
    let ocr = stub_ocr(&image);
    let exif = stub_exif(&path);

    let budget = SpectralBudget { principal_period: 200_000.0, ring_down_factor: 3.0 };
    let client = build_client();
    let mut archive = SceneArchive::new();

    let pipeline = VisionPipeline::new(client, budget)
        .with(Input::Image(image))
        .with(Input::Ocr(ocr))
        .with(Input::Exif(exif));

    println!(
        "Pipeline prepared: {} inputs, diameter = {:.0} tokens",
        pipeline.inputs().len(),
        pipeline.diameter(),
    );

    match pipeline.run().await {
        Ok(scene) => {
            println!("Satisfied:");
            println!("  contributing : {}", scene.contributing);
            println!("  total_tokens : {}", scene.total_tokens);
            println!("  caption      : {}", scene.caption);
            archive.push(scene);
            println!("Public world holds {} scene(s).", archive.len());

            #[cfg(feature = "runway-video")]
            if std::env::var("RUNWAY_API_KEY").is_ok() {
                if let Ok(rw) = RunwayVideoClient::from_env("gen4_turbo") {
                    if let Some(scene) = archive.last() {
                        let prompt = read_runway_prompt(&scene.caption)?;
                        eprintln!("[vision] queueing image_to_video…");
                        let dur = video_duration_secs();
                        match rw
                            .image_to_video(
                                &prompt,
                                &image_for_runway.raw_bytes,
                                image_for_runway.media_type,
                                "1280:720",
                                dur,
                                true,
                            )
                            .await
                        {
                            Ok(task) => print_task("Runway task queued", &task),
                            Err(e) => eprintln!("[vision] Runway submit failed: {e}"),
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("Refused: {e}");
            return Err(e.into());
        }
    }

    Ok(())
}
