#[cfg(test)]
mod pipeline_core {
    use std::sync::Arc;

    use spectral_budget::SpectralBudget;

    use crate::pipeline::{
        AudioInput, ExifInput, ImageInput, Input, MockVisionClient, OcrInput, Scene,
        SceneArchive, VisionClient, VisionError, VisionPipeline,
    };

    fn image_input(w: u32, h: u32) -> ImageInput {
        ImageInput {
            raw_bytes: Arc::new(vec![0u8; 16]),
            media_type: "image/png",
            width: w,
            height: h,
            estimated_tokens: (w as u64 * h as u64 / 750) as u32 + 1,
        }
    }

    fn open_budget() -> SpectralBudget {
        SpectralBudget { principal_period: 1_000_000.0, ring_down_factor: 3.0 }
    }

    // ── Input::token_weight ───────────────────────────────────────────────────

    #[test]
    fn image_token_weight_equals_estimated_tokens() {
        let a = Input::Image(image_input(100, 100));
        let img = match &a {
            Input::Image(i) => i,
            _ => unreachable!(),
        };
        assert_eq!(a.token_weight(), img.estimated_tokens);
    }

    #[test]
    fn ocr_token_weight_is_bytes_over_four() {
        let a = Input::Ocr(OcrInput { text: "abcd".into(), confidence: 1.0 });
        assert_eq!(a.token_weight(), 1);
        let b = Input::Ocr(OcrInput { text: "a".repeat(400), confidence: 0.5 });
        assert_eq!(b.token_weight(), 100);
    }

    #[test]
    fn exif_token_weight_has_floor_of_eight() {
        let a = Input::Exif(ExifInput { fields: vec![] });
        assert_eq!(a.token_weight(), 8);
    }

    #[test]
    fn audio_token_weight_is_bytes_over_four() {
        let a = Input::Audio(AudioInput {
            transcript: "word".repeat(100),
            duration_secs: 5.0,
        });
        assert_eq!(a.token_weight(), 100);
    }

    // ── MockVisionClient ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn mock_synthesize_empty_inputs_errors() {
        let client = Arc::new(MockVisionClient);
        let err = client.synthesize(&[]).await.unwrap_err();
        assert!(matches!(err, VisionError::NoInputs));
    }

    #[tokio::test]
    async fn mock_synthesize_produces_caption_with_count() {
        let client = Arc::new(MockVisionClient);
        let inputs = vec![
            Input::Image(image_input(64, 64)),
            Input::Ocr(OcrInput { text: "hello".into(), confidence: 0.9 }),
        ];
        let scene = client.synthesize(&inputs).await.unwrap();
        assert_eq!(scene.contributing, 2);
        assert!(scene.caption.contains("2 inputs"));
    }

    // ── VisionPipeline ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn pipeline_runs_within_budget() {
        let client = Arc::new(MockVisionClient);
        let p = VisionPipeline::new(client, open_budget())
            .with(Input::Image(image_input(32, 32)));
        assert_eq!(p.inputs().len(), 1);
        let scene = p.run().await.unwrap();
        assert_eq!(scene.contributing, 1);
    }

    #[tokio::test]
    async fn pipeline_rejected_over_budget() {
        let client = Arc::new(MockVisionClient);
        let tight = SpectralBudget { principal_period: 1.0, ring_down_factor: 1.0 };
        let p = VisionPipeline::new(client, tight)
            .with(Input::Image(image_input(3000, 3000)));
        let err = p.run().await.unwrap_err();
        assert!(matches!(err, VisionError::Budget(_)));
    }

    #[tokio::test]
    async fn pipeline_diameter_equals_sum_of_token_weights() {
        let client = Arc::new(MockVisionClient);
        let img = image_input(100, 100);
        let expected = img.estimated_tokens as f64;
        let p = VisionPipeline::new(client, open_budget())
            .with(Input::Image(img));
        assert!((p.diameter() - expected).abs() < 1.0);
    }

    // ── SceneArchive ──────────────────────────────────────────────────────────

    #[test]
    fn scene_archive_is_append_only() {
        let mut archive = SceneArchive::new();
        assert!(archive.is_empty());
        archive.push(Scene {
            caption: "first".into(),
            contributing: 1,
            total_tokens: 10,
        });
        archive.push(Scene {
            caption: "second".into(),
            contributing: 2,
            total_tokens: 20,
        });
        assert_eq!(archive.len(), 2);
        assert_eq!(archive.last().unwrap().caption, "second");
        let captions: Vec<_> = archive.iter().map(|s| s.caption.as_str()).collect();
        assert_eq!(captions, ["first", "second"]);
    }
}

// ── shape::PromptShape ────────────────────────────────────────────────────────

#[cfg(all(test, feature = "runway-video"))]
mod prompt_shape {
    use crate::shape::PromptShape;

    const ALL_LABELS: &[&str] = &["json", "prose", "bare", "fly", "reverence", "lux"];

    #[test]
    fn from_arg_round_trips_all_variants() {
        for label in ALL_LABELS {
            let shape = PromptShape::from_arg(label)
                .unwrap_or_else(|e| panic!("from_arg({label}): {e}"));
            assert_eq!(shape.label(), *label);
        }
    }

    #[test]
    fn from_arg_rejects_unknown() {
        assert!(PromptShape::from_arg("unknown_xyz").is_err());
    }

    #[test]
    fn text_is_nonempty_for_all_variants() {
        for label in ALL_LABELS {
            let shape = PromptShape::from_arg(label).unwrap();
            assert!(
                !shape.text().is_empty(),
                "shape {label} has empty text()"
            );
        }
    }

    #[test]
    fn json_shaped_variants_have_glossary() {
        for label in &["json", "fly", "reverence", "lux"] {
            let shape = PromptShape::from_arg(label).unwrap();
            assert!(
                shape.glossary().is_some(),
                "shape {label} should have a glossary"
            );
        }
    }

    #[test]
    fn prose_and_bare_have_no_glossary() {
        for label in &["prose", "bare"] {
            let shape = PromptShape::from_arg(label).unwrap();
            assert!(
                shape.glossary().is_none(),
                "shape {label} should not have a glossary"
            );
        }
    }

    #[test]
    fn prose_and_bare_have_no_json_syntax() {
        for label in &["prose", "bare"] {
            let shape = PromptShape::from_arg(label).unwrap();
            let text = shape.text();
            assert!(!text.contains('{'), "prose/bare shape {label} contains '{{'");
        }
    }

    #[test]
    fn fly_text_references_owl() {
        let shape = PromptShape::from_arg("fly").unwrap();
        assert!(shape.text().to_ascii_lowercase().contains("owl"));
    }

    #[test]
    fn lux_text_references_prism_or_spectral() {
        let shape = PromptShape::from_arg("lux").unwrap();
        let text = shape.text().to_ascii_lowercase();
        assert!(text.contains("prism") || text.contains("spectral"));
    }
}

// ── loaders ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod loaders {
    use std::path::Path;
    use std::sync::Arc;

    use crate::pipeline::{ImageInput, stub_exif, stub_ocr};

    fn dummy_image() -> ImageInput {
        ImageInput {
            raw_bytes: Arc::new(vec![]),
            media_type: "image/png",
            width: 10,
            height: 10,
            estimated_tokens: 1,
        }
    }

    #[test]
    fn stub_ocr_returns_zero_confidence() {
        let img = dummy_image();
        let ocr = stub_ocr(&img);
        assert_eq!(ocr.confidence, 0.0);
        assert!(!ocr.text.is_empty());
    }

    #[test]
    fn stub_exif_records_source_path() {
        let path = Path::new("/some/image.png");
        let exif = stub_exif(path);
        let source = exif.fields.iter().find(|(k, _)| k == "source_path");
        assert!(source.is_some());
        assert!(source.unwrap().1.contains("image.png"));
    }

    #[test]
    fn load_image_rejects_nonexistent_file() {
        let err = crate::pipeline::load_image(Path::new("/no/such/file_xyz.png"));
        assert!(err.is_err());
    }
}
