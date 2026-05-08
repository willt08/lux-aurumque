//! Entry point.
//!
//! Renders a Cornell-box-style scene with a pulsed light source, producing
//! a stack of PNGs — one per time bin — in `frames/`. Pipe through ffmpeg
//! to produce a video of light propagating through the scene.
//!
//! Run:
//!     cargo run --release
//!
//! Then:
//!     ffmpeg -framerate 30 -i frames/frame_%04d.png \
//!            -c:v libx264 -pix_fmt yuv420p -crf 18 transient.mp4

mod camera;
mod hit;
mod material;
mod ray;
mod scene;
mod sphere;
mod transient;
mod vec3;

use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use rand::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use rayon::prelude::*;

use camera::Camera;
use transient::{trace_path, Pulse, TransientFrame, C};
use vec3::Vec3;

// ---- render parameters ------------------------------------------------------

const WIDTH:        usize = 640;
const HEIGHT:       usize = 480;
const SAMPLES:      u32   = 256;     // paths per pixel
const MAX_DEPTH:    u32   = 8;
const NUM_BINS:     usize = 200;     // how many time slices in the output video
const DT:           f32   = 4.0e-11; // 40 ps/bin × 200 = 8 ns -> ~2.4 m path window
const PULSE_SIGMA:  f32   = 5.0e-11; // 50 ps pulse — fat enough to read clearly
const TILE_SIZE:    usize = 64;      // render tile edge in pixels
const FRAMES_DIR:   &str  = "frames";

// ---- tone mapping for output -----------------------------------------------

/// Reinhard tone map + sRGB-ish gamma. Cheap and robust for a wide range of
/// energies across bins.
#[inline]
fn tonemap_pixel(c: Vec3, exposure: f32) -> [u8; 3] {
    let x = c * exposure;
    let mapped = Vec3::new(
        x.x / (1.0 + x.x),
        x.y / (1.0 + x.y),
        x.z / (1.0 + x.z),
    );
    let g = Vec3::new(mapped.x.powf(1.0 / 2.2), mapped.y.powf(1.0 / 2.2), mapped.z.powf(1.0 / 2.2));
    [
        (g.x.clamp(0.0, 1.0) * 255.0) as u8,
        (g.y.clamp(0.0, 1.0) * 255.0) as u8,
        (g.z.clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(FRAMES_DIR)?;

    eprintln!("Building scene...");
    let world = scene::cornell_room();

    let camera = Camera::new(
        Vec3::new(0.0, 0.27, 0.85),     // lookfrom
        Vec3::new(0.0, 0.27, 0.0),      // lookat — center of room
        Vec3::Y,                         // up
        40.0,                            // vfov degrees
        WIDTH as f32 / HEIGHT as f32,
    );
    let pulse = Pulse { sigma: PULSE_SIGMA };

    eprintln!(
        "Rendering {}x{} @ {} spp, {} bins of {} ps -> {:.2} ns total",
        WIDTH, HEIGHT, SAMPLES, NUM_BINS, DT * 1e12, NUM_BINS as f32 * DT * 1e9
    );
    eprintln!(
        "Path-length window: 0 -> {:.3} m",
        NUM_BINS as f32 * DT * C
    );

    let start = Instant::now();

    // ---- parallel render: tile-based, one shared global frame ---------
    //
    // Memory rationale: the previous fold/reduce pattern allocated one
    // full TransientFrame *per rayon work chunk* — easily 16+ partials at
    // ~hundreds of MB each. We replace that with a single shared global
    // frame plus tiny per-tile partials (TILE_SIZE × TILE_SIZE × NUM_BINS).
    //
    // Tiles never overlap, so writes within a tile are unsynchronized. Only
    // the brief merge at the end of each tile takes the global mutex, and
    // those locks are short and bounded by tile count, not pixel count.
    let mut tiles: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut y0 = 0;
    while y0 < HEIGHT {
        let th = (HEIGHT - y0).min(TILE_SIZE);
        let mut x0 = 0;
        while x0 < WIDTH {
            let tw = (WIDTH - x0).min(TILE_SIZE);
            tiles.push((x0, y0, tw, th));
            x0 += TILE_SIZE;
        }
        y0 += TILE_SIZE;
    }
    eprintln!("{} tiles ({}×{} px each)", tiles.len(), TILE_SIZE, TILE_SIZE);

    let frame_mu = Mutex::new(TransientFrame::new(WIDTH, HEIGHT, NUM_BINS, DT));

    tiles.par_iter().for_each(|&(tx0, ty0, tw, th)| {
        let mut local = TransientFrame::new(tw, th, NUM_BINS, DT);
        for ly in 0..th {
            for lx in 0..tw {
                let gx = tx0 + lx;
                let gy = ty0 + ly;
                let pixel_idx = gy * WIDTH + gx;
                // Camera convention: origin at bottom-left.
                let y_camera = HEIGHT - 1 - gy;

                let mut rng = Xoshiro256PlusPlus::seed_from_u64(
                    0xCAFEBABE_DEADBEEF
                        ^ (pixel_idx as u64).wrapping_mul(0x9E3779B97F4A7C15),
                );

                for _ in 0..SAMPLES {
                    let (jx, jy) = Camera::jitter(&mut rng);
                    let s = (gx as f32 + jx) / (WIDTH  as f32 - 1.0);
                    let t = (y_camera as f32 + jy) / (HEIGHT as f32 - 1.0);
                    let ray = camera.ray_through(s, t, &mut rng);
                    // The local frame uses tile-relative coordinates.
                    trace_path(ray, &world, &pulse, MAX_DEPTH, lx, ly,
                               &mut local, &mut rng);
                }
            }
        }
        frame_mu.lock().unwrap().merge_tile(&local, tx0, ty0);
    });

    let frame = frame_mu.into_inner().unwrap();

    let render_secs = start.elapsed().as_secs_f32();
    eprintln!("Render done in {:.1}s.", render_secs);

    // Normalize by samples per pixel so brightness is independent of SPP.
    let mut frame = frame;
    frame.scale(1.0 / SAMPLES as f32);

    // ---- find a single global exposure across all bins -----------------
    //
    // Using one exposure for every PNG keeps the brightness consistent
    // through the video, so you can SEE the wavefront moving. Picking the
    // exposure from the brightest bin's 99th percentile prevents single
    // hot pixels from washing everything out.
    let mut all_brightnesses: Vec<f32> = Vec::with_capacity(NUM_BINS);
    for bin in 0..NUM_BINS {
        let s = frame.slice(bin);
        let mut max_local = 0.0_f32;
        for px in s.chunks_exact(3) {
            let lum = 0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2];
            if lum > max_local { max_local = lum; }
        }
        all_brightnesses.push(max_local);
    }
    let global_max = all_brightnesses.iter().cloned().fold(0.0_f32, f32::max);
    let exposure = if global_max > 0.0 { 0.6 / global_max } else { 1.0 };
    eprintln!("Global max luminance = {:.4}, exposure = {:.4}", global_max, exposure);

    // ---- write each bin as a PNG --------------------------------------
    eprintln!("Writing {} PNGs to {}/", NUM_BINS, FRAMES_DIR);
    (0..NUM_BINS).into_par_iter().for_each(|bin| {
        let s = frame.slice(bin);
        let mut buf = vec![0u8; WIDTH * HEIGHT * 3];
        for (i, px) in s.chunks_exact(3).enumerate() {
            let c = Vec3::new(px[0], px[1], px[2]);
            let rgb = tonemap_pixel(c, exposure);
            buf[i * 3]     = rgb[0];
            buf[i * 3 + 1] = rgb[1];
            buf[i * 3 + 2] = rgb[2];
        }
        let path = format!("{}/frame_{:04}.png", FRAMES_DIR, bin);
        image::save_buffer(
            Path::new(&path), &buf,
            WIDTH as u32, HEIGHT as u32,
            image::ColorType::Rgb8,
        ).expect("PNG write failed");
    });

    eprintln!("Done. To make a video:");
    eprintln!("  ffmpeg -framerate 30 -i frames/frame_%04d.png \\");
    eprintln!("         -c:v libx264 -pix_fmt yuv420p -crf 18 transient.mp4");
    Ok(())
}
