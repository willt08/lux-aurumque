//! spectral_vibe — the spectral-tempo link from `NOTES_PROCESS.md` §3.
//!
//! The renderer's room of diameter `d` has principal period `T_1 = 2·d/c`
//! that bounds its becoming. Take that *same Faber–Krahn arithmetic* and
//! apply it at an audible time scale: rescale the propagation speed `c`
//! from light-speed to a "felt-speed" (factor `1e-9` is natural —
//! nanoseconds become seconds), and `T_1` lands in the seconds range,
//! where it plays the role of *bar length* in a 6/8 audio bed.
//!
//! For the renderer's default scene (diameter ≈ 0.95 m) the optical
//! `T_1 ≈ 6.34 ns`. Rescaled to audio, `T_1_audio ≈ 6.34 s`. With six
//! beats per bar, the implied tempo is `6 · 60 / 6.34 ≈ 56.8 BPM` —
//! within rounding of `vibe.rs`'s heartbeat-paced `56 BPM` setting. The
//! coincidence is no coincidence: both substrates are bounded resonant
//! systems and Faber–Krahn fixes their slowest mode the same way.
//!
//! Build:
//!     cargo run --example spectral_vibe -- 0.95
//!
//! Renders `spectral_vibe.wav` (stereo, 44.1 kHz, 32-bit float).
//!
//! Willinton Triana Cardona / 3BSN LLC

use hound::{SampleFormat, WavSpec, WavWriter};
use spectral_budget::SpectralBudget;
use std::f32::consts::TAU;

/// Speed of light in vacuum, m/s — the renderer's `c`.
const C_LIGHT: f64 = 2.998e8;

/// Time-rescale factor mapping light-speed nanoseconds to audio-speed
/// seconds. Equivalent to dividing `c` by `1e9`: a "tempo speed" of
/// roughly `0.3 m/s` (slow walking pace, breath-paced).
const TEMPO_RESCALE: f64 = 1.0e-9;

/// Beats per bar in 6/8 meter — vibe.rs's signature.
const BEATS_PER_BAR: f64 = 6.0;

/// Total length of the rendered bed, in bars.
const RENDER_BARS: f64 = 4.0;

const SR: f32 = 44_100.0;

fn main() -> hound::Result<()> {
    // 1. Take the room diameter from CLI, default to lux-aurumque's scene.
    let diameter_m: f64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.95);

    // 2. Optical reading: light-speed propagation, principal period in seconds
    //    but extremely tiny — the value the renderer constructs.
    let optical = SpectralBudget::for_scene_diameter(diameter_m, C_LIGHT);

    // 3. Audio reading: rescale `c` by 1e-9 (felt-speed). Same diameter,
    //    same Faber–Krahn arithmetic, bar-scale period.
    let audio = SpectralBudget::for_scene_diameter(diameter_m, C_LIGHT * TEMPO_RESCALE);

    // 4. The audio principal period is the bar length. BPM follows.
    let bar_secs = audio.principal_period;
    let bpm = BEATS_PER_BAR * 60.0 / bar_secs;
    let beat_secs = bar_secs / BEATS_PER_BAR;

    // 5. Print the parallel — the substrate-crossing identity.
    println!("─── spectral-tempo link ──────────────────────────────");
    println!("Room diameter         : {:.3} m", diameter_m);
    println!();
    println!("Optical (light-speed):");
    println!("  c                   : {:.3e} m/s", C_LIGHT);
    println!("  T_1                 : {:.3} ns ({:.3e} s)",
        optical.principal_period * 1e9, optical.principal_period);
    println!("  3·T_1 ceiling       : {:.3} ns",
        optical.principal_period * optical.ring_down_factor * 1e9);
    println!();
    println!("Audio (felt-speed, c rescaled by {:.0e}):", TEMPO_RESCALE);
    println!("  c_audio             : {:.4} m/s", C_LIGHT * TEMPO_RESCALE);
    println!("  T_1 (bar length)    : {:.3} s", bar_secs);
    println!("  beat (6/8)          : {:.3} s", beat_secs);
    println!("  tempo               : {:.1} BPM", bpm);
    println!();

    // 6. Render `RENDER_BARS` bars at the derived tempo.
    let total_secs = bar_secs * RENDER_BARS;
    let path = "spectral_vibe.wav";
    render_pad_with_bells(path, bpm as f32, total_secs as f32)?;
    println!("Wrote {path} ({:.2} s, {:.0} bars).", total_secs, RENDER_BARS);
    Ok(())
}

/// Minimal three-voice synth: sustained sine pad with a slow breath
/// modulation, a bell hit on every beat, and a low heartbeat on the
/// downbeat of each bar (beats 1 and 4 in 6/8).
fn render_pad_with_bells(path: &str, bpm: f32, total_secs: f32) -> hound::Result<()> {
    let spec = WavSpec {
        channels: 2,
        sample_rate: SR as u32,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(path, spec)?;

    let beat_secs = 60.0 / bpm;
    let bar_secs = beat_secs * BEATS_PER_BAR as f32;
    let total_samples = (total_secs * SR) as usize;

    for i in 0..total_samples {
        let t = i as f32 / SR;

        // Pad: A2 (110 Hz) + perfect fifth (165 Hz), breath modulation 1/8 Hz.
        let breath = 0.55 + 0.30 * (TAU * t / 8.0).sin();
        let pad = breath * (
            0.55 * (TAU * 110.0 * t).sin()
          + 0.35 * (TAU * 165.0 * t).sin()
        ) * 0.18;

        // Bell: brief 880 Hz with exponential decay at every beat onset.
        let beat_phase = (t / beat_secs).fract() * beat_secs;
        let bell_env = (-beat_phase / 0.18).exp();
        let bell = bell_env * (TAU * 880.0 * beat_phase).sin() * 0.22;

        // Heartbeat: a low 55 Hz thud on bar onsets and on beat 4 (the
        // 6/8 secondary stress). Two short hits per bar.
        let bar_phase = (t / bar_secs).fract() * bar_secs;
        let downbeat_t = bar_phase;
        let secondary_t = (bar_phase - 3.0 * beat_secs).abs().min(bar_secs - bar_phase);
        let heartbeat_env = (-downbeat_t / 0.10).exp().max((-secondary_t / 0.10).exp());
        let heartbeat = heartbeat_env * (TAU * 55.0 * t).sin() * 0.30;

        let mix = (pad + bell + heartbeat).tanh();

        writer.write_sample(mix)?; // L
        writer.write_sample(mix)?; // R
    }

    writer.finalize()
}
