# lux-vision

[![Crates.io](https://img.shields.io/crates/v/lux-vision.svg)](https://crates.io/crates/lux-vision)
[![docs.rs](https://img.shields.io/docsrs/lux-vision)](https://docs.rs/lux-vision)
[![License](https://img.shields.io/crates/l/lux-vision.svg)](https://crates.io/crates/lux-vision)

A pluggable, multimodal vision pipeline with token-budget admission.
Implement `VisionClient` to plug any vision model in; `MockVisionClient`
runs the full pipeline offline with no API keys.

```toml
[dependencies]
lux-vision = "0.1"
spectral-budget = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use std::sync::Arc;
use lux_vision::{Input, MockVisionClient, OcrInput, VisionPipeline};
use spectral_budget::SpectralBudget;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(MockVisionClient);
    let budget = SpectralBudget { principal_period: 4_000.0, ring_down_factor: 3.0 };

    let scene = VisionPipeline::new(client, budget)
        .with(Input::Ocr(OcrInput {
            text: "Cornell box; gold sphere on cream floor".into(),
            confidence: 0.95,
        }))
        .run()
        .await?;

    println!("{}", scene.caption);
    Ok(())
}
```

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  VisionPipeline                                                  │
│  ┌──────────┐  ┌──────┐  ┌──────┐  ┌───────┐                   │
│  │ Image    │  │ OCR  │  │ EXIF │  │ Audio │  ← Input variants  │
│  └──────────┘  └──────┘  └──────┘  └───────┘                   │
│        │            │         │         │                        │
│        └────────────┴─────────┴─────────┘                       │
│                              │                                   │
│                    SpectralBudget::try_admit                      │
│                    (token-window guard)                          │
│                              │                                   │
│                    VisionClient::synthesize                       │
│                    (MockVisionClient | AnthropicVisionClient)    │
│                              │                                   │
│                            Scene  →  SceneArchive                 │
└──────────────────────────────────────────────────────────────────┘
                               │
                       RunwayVideoClient
                       (text_to_video | image_to_video
                        | character_performance)
```

The base crate (no features) has no network dependencies. Pull in only
what you need:

| Feature | Enables |
|---|---|
| *(none)* | Pipeline core: `VisionClient`, `MockVisionClient`, `VisionPipeline`, `SceneArchive`, all input types |
| `anthropic-vision` | `lux_vision::anthropic::AnthropicVisionClient` — real Claude vision synthesis |
| `runway-video` | `lux_vision::runway::RunwayVideoClient` + `lux_vision::shape::PromptShape` |
| `full` | Both backend clients |

## Token-budget admission

Aggregate input diameter (token weight, summed across modalities) is
checked against the [`spectral-budget`](https://docs.rs/spectral-budget)
ceiling before any inference call fires. Over-budget pipelines fail
cheaply with a structured `VisionError::Budget` carrying the diameter,
the bound, `T_1`, and the ring-down factor.

| Input | Token weight estimate |
|---|---|
| `Image` | `ceil(w × h / 750)` |
| `Ocr` | `bytes / 4` |
| `Exif` | `field count × avg chars / 4`, floor of 8 |
| `Audio` | `transcript bytes / 4` |

## Environment variables (feature `anthropic-vision` / `runway-video`)

| Variable | Default | Effect |
|---|---|---|
| `ANTHROPIC_API_KEY` | — | Required for Anthropic synthesis |
| `RUNWAY_API_KEY` | — | Required for Runway video tasks |
| `LUX_VISION_MODEL` | `claude-sonnet-4-6` | Anthropic model id |
| `LUX_VIDEO_MODEL` | endpoint-dependent | Runway model id (text/image routes) |
| `LUX_RUNWAY_BASE_URL` | `https://api.dev.runwayml.com` | Runway endpoint override |
| `LUX_RUNWAY_VIDEO_MAX_BYTES` | `15728640` (15 MB) | character_performance reference video cap |

## License

Dual MIT / Apache-2.0. © 3BSN LLC.
