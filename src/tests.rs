// ── process::SpectralBudget ───────────────────────────────────────────────────

#[cfg(test)]
mod spectral_budget {
    use crate::process::{BudgetError, SpectralBudget};

    fn budget(t1: f64, factor: f64) -> SpectralBudget {
        SpectralBudget { principal_period: t1, ring_down_factor: factor }
    }

    #[test]
    fn admits_at_bound() {
        let b = budget(100.0, 3.0);
        assert!(b.admits(300.0));
    }

    #[test]
    fn admits_below_bound() {
        let b = budget(100.0, 3.0);
        assert!(b.admits(0.0));
        assert!(b.admits(299.99));
    }

    #[test]
    fn rejects_above_bound() {
        let b = budget(100.0, 3.0);
        assert!(!b.admits(300.001));
        assert!(!b.admits(1_000_000.0));
    }

    #[test]
    fn try_admit_ok_returns_unit() {
        let b = budget(100.0, 3.0);
        assert!(b.try_admit(150.0).is_ok());
    }

    #[test]
    fn try_admit_err_carries_values() {
        let b = budget(100.0, 3.0);
        match b.try_admit(500.0) {
            Err(BudgetError::Exceeded { diameter, bound, principal_period, ring_down_factor }) => {
                assert_eq!(diameter, 500.0);
                assert_eq!(bound, 300.0);
                assert_eq!(principal_period, 100.0);
                assert_eq!(ring_down_factor, 3.0);
            }
            Ok(()) => panic!("expected Exceeded"),
        }
    }

    #[test]
    fn for_scene_diameter_sets_t1_as_2d_over_c() {
        let b = SpectralBudget::for_scene_diameter(1.5, 3.0e8);
        let expected_t1 = 2.0 * 1.5 / 3.0e8;
        assert!((b.principal_period - expected_t1).abs() < 1e-20);
        assert_eq!(b.ring_down_factor, 3.0);
    }

    #[test]
    fn display_contains_key_numbers() {
        let b = budget(100.0, 3.0);
        let msg = b.try_admit(500.0).unwrap_err().to_string();
        assert!(msg.contains("5.000e2"), "diameter in message: {msg}");
        assert!(msg.contains("3.000e2"), "bound in message: {msg}");
    }
}

// ── vision core ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod vision_core {
    use std::sync::Arc;

    use crate::process::SpectralBudget;
    use crate::vision::{
        Antecedent, AudioPrehension, ConcrescenceError, ExifPrehension, ImagePrehension,
        MockVisionClient, OcrPrehension, SceneArchive, UnifiedScene, VisionConcrescence,
    };
    use crate::{Concrescence, PublicWorld, Society};

    fn image_prehension(w: u32, h: u32) -> ImagePrehension {
        ImagePrehension {
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

    // ── Antecedent::token_weight ──────────────────────────────────────────────

    #[test]
    fn image_token_weight_equals_estimated_tokens() {
        let a = Antecedent::Image(image_prehension(100, 100));
        let img = match &a {
            Antecedent::Image(i) => i,
            _ => unreachable!(),
        };
        assert_eq!(a.token_weight(), img.estimated_tokens);
    }

    #[test]
    fn ocr_token_weight_is_bytes_over_four() {
        let a = Antecedent::Ocr(OcrPrehension { text: "abcd".into(), confidence: 1.0 });
        assert_eq!(a.token_weight(), 1);
        let b = Antecedent::Ocr(OcrPrehension { text: "a".repeat(400), confidence: 0.5 });
        assert_eq!(b.token_weight(), 100);
    }

    #[test]
    fn exif_token_weight_has_floor_of_eight() {
        let a = Antecedent::Exif(ExifPrehension { fields: vec![] });
        assert_eq!(a.token_weight(), 8);
    }

    #[test]
    fn audio_token_weight_is_bytes_over_four() {
        let a = Antecedent::Audio(AudioPrehension {
            transcript: "word".repeat(100),
            duration_secs: 5.0,
        });
        assert_eq!(a.token_weight(), 100);
    }

    // ── MockVisionClient ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn mock_synthesize_empty_antecedents_errors() {
        let client = Arc::new(MockVisionClient);
        use crate::vision::VisionClient;
        let err = client.synthesize(&[]).await.unwrap_err();
        assert!(matches!(err, ConcrescenceError::NoAntecedents));
    }

    #[tokio::test]
    async fn mock_synthesize_produces_caption_with_count() {
        let client = Arc::new(MockVisionClient);
        use crate::vision::VisionClient;
        let ants = vec![
            Antecedent::Image(image_prehension(64, 64)),
            Antecedent::Ocr(OcrPrehension { text: "hello".into(), confidence: 0.9 }),
        ];
        let scene = client.synthesize(&ants).await.unwrap();
        assert_eq!(scene.contributing, 2);
        assert!(scene.caption.contains("2 prehensions"));
    }

    // ── VisionConcrescence ────────────────────────────────────────────────────

    #[tokio::test]
    async fn concrescence_unifies_within_budget() {
        let client = Arc::new(MockVisionClient);
        let c = VisionConcrescence::new(client, open_budget())
            .prehend(Antecedent::Image(image_prehension(32, 32)));
        assert_eq!(Concrescence::prehensions(&c).len(), 1);
        let scene = c.unify().await.unwrap();
        assert_eq!(scene.contributing, 1);
    }

    #[tokio::test]
    async fn concrescence_rejected_over_budget() {
        let client = Arc::new(MockVisionClient);
        let tight = SpectralBudget { principal_period: 1.0, ring_down_factor: 1.0 };
        let c = VisionConcrescence::new(client, tight)
            .prehend(Antecedent::Image(image_prehension(3000, 3000)));
        let err = c.unify().await.unwrap_err();
        assert!(matches!(err, ConcrescenceError::Budget(_)));
    }

    #[tokio::test]
    async fn society_diameter_equals_sum_of_token_weights() {
        let client = Arc::new(MockVisionClient);
        let img = image_prehension(100, 100);
        let expected = img.estimated_tokens as f64;
        let c = VisionConcrescence::new(client, open_budget())
            .prehend(Antecedent::Image(img));
        assert!((Society::diameter(&c) - expected).abs() < 1.0);
    }

    // ── SceneArchive ──────────────────────────────────────────────────────────

    #[test]
    fn scene_archive_is_append_only() {
        let mut archive = SceneArchive::new();
        assert!(archive.is_empty());
        archive.deposit(UnifiedScene {
            caption: "first".into(),
            contributing: 1,
            total_tokens: 10,
        });
        archive.deposit(UnifiedScene {
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
    fn json_shapes_contain_no_raw_json_syntax_after_label() {
        // Sanity: the raw text for json-shaped presets contains JSON (it should —
        // translation strips it before Runway sees it). Prose/bare should not.
        for label in &["prose", "bare"] {
            let shape = PromptShape::from_arg(label).unwrap();
            let text = shape.text();
            assert!(!text.contains('{'), "prose/bare shape {label} contains '{{': {text:.80}...");
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

// ── vision loaders ────────────────────────────────────────────────────────────

#[cfg(test)]
mod vision_loaders {
    use std::path::Path;

    use crate::vision::{stub_exif, stub_ocr, ImagePrehension};
    use std::sync::Arc;

    fn dummy_image() -> ImagePrehension {
        ImagePrehension {
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
        let err = crate::vision::load_image(Path::new("/no/such/file_xyz.png"));
        assert!(err.is_err());
    }
}
