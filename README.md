# lux-aurumque

*Lux Aurumque* — Light and Gold.

A **transient path tracer** and **pluggable vision-pipeline toolkit** in Rust,
unified by a process-philosophical spine. Two substrates, one coherence guarantee:
the [`SpectralBudget`] bound that keeps becoming finite — whether the domain is
photons propagating through a gilded room or tokens propagating through a vision model.

![preview](https://raw.githubusercontent.com/willt08/lux-aurumque/main/preview.gif)

---

## What it does (v0.3.0)

Two independent capabilities ship in one crate:

**1. Transient path tracer** — renders a physics-correct wavefront movie.
Standard path tracers compute steady-state radiance (light bounced until
equilibrium). This one refuses that simplification: every photon path carries
its cumulative optical length, which determines its arrival time. Binning by
arrival time produces a time-resolved movie of a Gaussian pulse propagating
through a Cornell-box scene reskinned in aurum.

**2. Vision pipeline** — a model-agnostic, multimodal inference chain with
pluggable backends. In v0.3.0 this is a first-class library API:

```rust
use lux_aurumque::{VisionConcrescence, Antecedent, MockVisionClient, SpectralBudget};
use lux_aurumque::anthropic::AnthropicVisionClient;  // feature: anthropic-vision
use lux_aurumque::runway::RunwayVideoClient;          // feature: runway-video
use lux_aurumque::shape::PromptShape;                 // feature: runway-video
```

---

## Vision pipeline — architecture

### Backbone: pluggable `VisionClient`

```
┌──────────────────────────────────────────────────────────────────┐
│  VisionConcrescence                                              │
│  ┌──────────┐  ┌──────┐  ┌──────┐  ┌───────┐                   │
│  │ Image    │  │ OCR  │  │ EXIF │  │ Audio │  ← Antecedents     │
│  └──────────┘  └──────┘  └──────┘  └───────┘                   │
│        │            │         │         │                        │
│        └────────────┴─────────┴─────────┘                       │
│                              │                                   │
│                    SpectralBudget::try_admit                      │
│                    (token-window guard)                          │
│                              │                                   │
│                     VisionClient::synthesize                      │
│                     (MockVisionClient | AnthropicVisionClient)   │
│                              │                                   │
│                         UnifiedScene  →  SceneArchive            │
└──────────────────────────────────────────────────────────────────┘
                               │
                      RunwayVideoClient
                      (text_to_video | image_to_video
                       | character_performance)
```

**Implement `VisionClient` to swap any vision model in.** The `MockVisionClient`
runs the full pipeline offline with no API keys — useful for testing the pipeline
shape before paying for inference.

### Multimodal feature fusion

`VisionConcrescence` accepts heterogeneous antecedents in a single forward pass:

| Antecedent | Token weight estimate | Role |
|---|---|---|
| `Image` | `w × h / 750` | Primary visual feature |
| `Ocr` | `bytes / 4` | Textual overlay / caption signal |
| `Exif` | field count × avg chars / 4 | Metadata conditioning |
| `Audio` | `bytes / 4` | Temporal / semantic alignment |

The aggregate token diameter is checked against `SpectralBudget` before any
inference fires. Over-budget prehensions are rejected with a structured error
that carries the diameter, bound, `T_1`, and ring-down factor — diagnosable
at the call site without inspecting internals.

### Chained inference (video continuation)

Each video segment is one `Occasion` in a personally-ordered society:

```
frame_N.mp4
    │
    ├─ extract_final_frame (ffmpeg)
    │
    ├─ Claude: describe_image × 2 (parallel tokio::try_join!)
    │     ├─ CHARACTER_DIRECTIVE  → character embedding (≤300 chars)
    │     └─ LOCI_DIRECTIVE       → spatial context embedding (≤300 chars)
    │
    ├─ compose (character + loci + PromptShape directive)
    │
    ├─ translate_nexus (Claude: JSON nexus → caption prose, ≤950 chars)
    │
    └─ RunwayVideoClient::image_to_video  →  task_id (frame_N+1)
```

The two Claude extraction calls are **independent prehensions of the same datum**
— they run in parallel. The structured JSON nexus is translated into caption-shaped
video prompt prose by a second Claude call before submission to Runway, stripping
JSON syntax that would otherwise leak as on-screen text in the video output.

### PromptShape — structured prompt engineering layer

Six presets ship as library types in `lux_aurumque::shape::PromptShape`:

| Shape | Description |
|---|---|
| `json` | Electron-orbital society, three-phase camera |
| `prose` | Explicit causal linkages, Newtonian |
| `bare` | No structural markers |
| `fly` | Owl breaks the picture-plane (essentia + deictic reductio); dolly-zoom terminates on iris at t=8s |
| `reverence` | Backflip → kneel reverence, locked-off, Greco-Roman |
| `lux` | Holomorphic prism: light in the frame decomposes into spectral arcs, conformal warp, no object motion |

Shapes with a glossary (`json`, `fly`, `reverence`, `lux`) route through Claude's
`translate_nexus` — the translation seam converts the structured nexus into
caption prose grounded in the relational glossary, enforcing visual commitments
the video encoder's text backbone is trained to consume.

Prose and bare shapes bypass translation and go directly to Runway.

---

## Quick start

### Path tracer

```bash
cargo run --release
ffmpeg -framerate 30 -i frames/frame_%04d.png \
       -c:v libx264 -pix_fmt yuv420p -crf 18 lux-aurumque.mp4
```

### Vision pipeline

```bash
# Set keys
export ANTHROPIC_API_KEY=...
export RUNWAY_API_KEY=...

# Describe an image → image-to-video (Claude caption seeds Runway)
cargo run --example concrescence --features runaway-hackathon -- image.png

# Image-anchored shape mode (skip describe; shape drives motion)
cargo run --example concrescence --features runaway-hackathon -- \
  image.png --prompt-shape fly

# Text-to-video, shape preset
cargo run --example concrescence --features runaway-hackathon -- \
  --prompt-shape lux

# Chain from a prior Runway task (parallel character + loci extraction)
cargo run --example concrescence --features runaway-hackathon -- \
  --continue-from <task-id> --prompt-shape fly

# Chain from a local mp4 — image_to_video (prompt-shape present)
LUX_VIDEO_DURATION_SECS=8 \
cargo run --example concrescence --features runaway-hackathon -- \
  --continue-from-file prior.mp4 --prompt-shape lux

# Chain from a local mp4 — character_performance (no prompt-shape)
cargo run --example concrescence --features runaway-hackathon -- \
  --continue-from-file prior.mp4
```

### Environment variables

| Variable | Default | Effect |
|---|---|---|
| `ANTHROPIC_API_KEY` | — | Required for real synthesis and nexus translation |
| `RUNWAY_API_KEY` | — | Required for any video generation |
| `LUX_VISION_MODEL` | `claude-sonnet-4-6` | Vision model for synthesis and translation |
| `LUX_VIDEO_MODEL` | endpoint-dependent | Runway model (text/image paths only) |
| `LUX_CHARACTER_PERFORMANCE_MODEL` | `act_two` | Model for character_performance endpoint |
| `LUX_VIDEO_DURATION_SECS` | `4` | Output video duration |
| `LUX_PUBLIC_FIGURE_THRESHOLD` | `low` | Runway content moderation gate |
| `LUX_RUNWAY_VIDEO_MAX_BYTES` | `15728640` (15 MB) | Reference video cap for character_performance |
| `LUX_SKIP_CLAUDE_INHERITANCE` | `0` | Skip Claude extraction on no-shape continuation |

---

## Feature flags

```toml
[dependencies]
lux-aurumque = { version = "0.3", features = ["runaway-hackathon"] }
```

| Feature | Enables |
|---|---|
| *(none)* | Path tracer + vision spine (`VisionClient`, `MockVisionClient`, `VisionConcrescence`, `SceneArchive`, `SpectralBudget`) |
| `anthropic-vision` | `lux_aurumque::anthropic::AnthropicVisionClient` |
| `runway-video` | `lux_aurumque::runway::RunwayVideoClient` + `lux_aurumque::shape::PromptShape` |
| `runaway-hackathon` | Alias: enables both `anthropic-vision` and `runway-video` |

The base crate (no features) has no network dependencies. Pull in only what you need.

---

## Practical applications

**Generative video continuation** — chain any video through Claude feature
extraction + Runway generation. Each segment inherits character and spatial
context from the prior frame; the `PromptShape` preset steers the motion without
re-prompting from scratch.

**Multimodal scene understanding pipeline** — fuse image, OCR, EXIF, and audio
transcripts into a single unified caption with token-budget enforcement.
Swap `AnthropicVisionClient` for your own `VisionClient` impl to run against any
vision backbone (GPT-4o, Gemini Vision, local BLIP-2, etc.).

**Prompt ablation framework** — the `PromptShape` system was designed for
controlled ablations: fix the image anchor, vary the shape, measure temporal
coherence in the latent walk. `json` vs `prose` vs `bare` isolate the effect
of structural encoding on the video model's motion generation.

**Physics-grounded token budgeting** — `SpectralBudget` enforces the same
Faber–Krahn arithmetic that bounds the path tracer's render horizon, ported to
token windows. The principal period `T_1` is your model's context window;
`ring_down_factor = 3` gives a `3 · T_1` admission ceiling. The same struct
works for any bounded sequential domain.

**Light-field phenomena visualization** — the `--prompt-shape lux` preset
drives a holomorphic prism effect: locked camera, no object motion, only the
light field bends and fans into spectral arcs following conformal complex-plane
geometry. Built on the same process spine as the path tracer.

---

## Library structure (v0.3.0)

```
src/
├── lib.rs          re-exports; module declarations
├── process.rs      Occasion, Society, Concrescence, PublicWorld, SpectralBudget
├── vision.rs       VisionClient, MockVisionClient, VisionConcrescence,
│                   SceneArchive, Antecedent, ImagePrehension, load_image, …
├── anthropic.rs    AnthropicVisionClient (feature: anthropic-vision)
├── runway.rs       RunwayVideoClient, TaskHandle (feature: runway-video)
├── shape.rs        PromptShape + nexus constants (feature: runway-video)
├── tests.rs        28 unit tests (SpectralBudget, vision core, PromptShape, loaders)
│
│   Path tracer:
├── transient.rs    time-binned framebuffer + transient path tracer
├── camera.rs       pinhole ray generation
├── hit.rs          HitRecord, Hittable
├── sphere.rs       analytic sphere intersection
├── material.rs     Lambertian / Metal / DiffuseLight
├── ray.rs          Ray with cumulative path_length
├── scene.rs        scene graph
└── vec3.rs         Vec3 over glam

examples/
├── concrescence.rs   vision pipeline CLI (~320 lines; imports from library)
├── receptacle.rs     process-spine blueprint (Plato's χώρα as vision substrate)
└── spectral_vibe.rs  Faber–Krahn tempo derivation → 6/8 audio bed
```

---

## Path tracer physics knobs

| Constant | Default | Effect |
|---|---|---|
| `WIDTH × HEIGHT` | 640×480 | Resolution — memory scales as `W × H × NUM_BINS × 12 bytes` |
| `SAMPLES` | 256 | Paths per pixel |
| `DT` | 40 ps | Time-bin width — smaller = sharper wavefronts |
| `NUM_BINS` | 200 | Time slices; `NUM_BINS × DT × c` is the path-length window |
| `PULSE_SIGMA` | 50 ps | Gaussian pulse temporal width |
| `TILE_SIZE` | 64 | Render tile edge; controls peak memory |

The renderer enforces `NUM_BINS · DT ≤ 3 · T_1` at startup (Faber–Krahn bound,
`T_1 ≈ 2 · diam(Ω) / c`). Parameters that violate the budget are refused with
a structured error before any computation runs.

---

## Further reading

- Jarabo et al. **A Framework for Transient Rendering.** ACM TOG 2014.
- Velten et al. **Femto-Photography.** SIGGRAPH 2013.
- Whitehead, **Process and Reality** (1929) — the philosophical substrate for
  the `process.rs` spine. [`NOTES_PROCESS.md`](NOTES_PROCESS.md) maps the
  renderer's data structures onto the ontology directly.

---

## License

Dual MIT / Apache-2.0. © 3BSN LLC.
