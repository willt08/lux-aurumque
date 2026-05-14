//! The transient core.
//!
//! ## Concept
//!
//! In a *steady-state* renderer, each pixel is one RGB triple — the sum of
//! radiance over all light paths that reach it, with no notion of *when*
//! each contribution arrived. Transient rendering refuses that collapse:
//! every pixel becomes a *time histogram* of arrivals, indexed by the
//! cumulative optical path length each photon traveled before reaching
//! the sensor.
//!
//! Because light propagates at a finite speed `c`, the time of arrival of
//! a photon path of total length `L` is simply `t = L / c`. We bin those
//! arrivals into discrete time bins of width `dt`, building a 3D framebuffer
//! of shape `(height, width, num_bins)`. Each time bin can then be saved as
//! a separate PNG, and ffmpeg stitches them into a video that shows light
//! literally propagating through the scene.
//!
//! ## What's a Whitehead would call this
//!
//! Each path is an *actual occasion of illumination*: it has a definite
//! beginning (emission), a satisfaction (deposition at the sensor), a
//! definite duration (`L / c`), and contributes to the public world (the
//! framebuffer) before perishing. The renderer is a society of such
//! occasions, summed.
//!
//! ## Implementation
//!
//! - Path tracing accumulates `path_length` segment by segment on each `Ray`.
//! - When a path terminates by striking an emissive surface, we deposit
//!   `throughput * emitted` into the time bin corresponding to its total
//!   length — *if* the source's pulse profile admits it at that time.
//! - The pulse is modeled here as a Gaussian centered at `t = 0` (the moment
//!   of emission) with a configurable temporal width sigma. The contribution
//!   at the sensor is therefore weighted by `exp(-((t - L/c) / sigma)^2 / 2)`,
//!   integrated over the bin.
//! - For simplicity and clarity we skip explicit light sampling — pure path
//!   tracing with russian-roulette termination. This is slow per sample but
//!   short to read, and parallelization across pixels recovers the cost.

use crate::hit::Hittable;
use crate::ray::Ray;
use crate::vec3::Vec3;

use rand::Rng;
use rand_xoshiro::Xoshiro256PlusPlus;

// ---- physical constants -----------------------------------------------------

/// Speed of light in meters per second. We work in *meters* in scene space.
pub const C: f32 = 2.998e8;

// ---- transient framebuffer --------------------------------------------------

/// A 3-D time-resolved framebuffer: `(num_bins, height, width)` of
/// linear-RGB radiance, addressable as a histogram of arrival times per
/// pixel. The headline data structure for transient rendering.
pub struct TransientFrame {
    /// Image width in pixels.
    pub width: usize,
    /// Image height in pixels.
    pub height: usize,
    /// Number of time bins along the temporal axis.
    pub num_bins: usize,
    /// Bin width in seconds. With `dt = 1e-11` (10 ps) and 256 bins, we cover
    /// 2.56 ns of propagation — enough for ~76 cm of total path length, which
    /// is about right for a ~30 cm scene with a couple of bounces.
    pub dt: f32,
    /// Stored as `[bin][y][x][channel]` flattened. We separate by bin first
    /// so that writing a single time slice is one contiguous slab.
    data: Vec<f32>,
}

impl TransientFrame {
    /// Allocate a zeroed framebuffer of `width × height × num_bins` with
    /// time-bin width `dt` seconds.
    pub fn new(width: usize, height: usize, num_bins: usize, dt: f32) -> Self {
        Self {
            width, height, num_bins, dt,
            data: vec![0.0; width * height * num_bins * 3],
        }
    }

    #[inline]
    fn idx(&self, bin: usize, x: usize, y: usize, ch: usize) -> usize {
        ((bin * self.height + y) * self.width + x) * 3 + ch
    }

    /// Accumulate a contribution into bin `bin` at pixel (x, y). The
    /// inherent fast path — used by `trace_path` and `merge_tile`.
    /// See [`Deposit`] and [`TransientFrame::deposit`] for the typed
    /// API when integrating downstream.
    #[inline]
    pub fn accumulate(&mut self, bin: usize, x: usize, y: usize, color: Vec3) {
        if bin >= self.num_bins { return; }
        let i = self.idx(bin, x, y, 0);
        self.data[i]     += color.x;
        self.data[i + 1] += color.y;
        self.data[i + 2] += color.z;
    }

    /// Borrow one time slice as a flat slice of `width * height * 3` floats.
    pub fn slice(&self, bin: usize) -> &[f32] {
        let base = bin * self.height * self.width * 3;
        &self.data[base..base + self.height * self.width * 3]
    }

    /// Merge a small tile-sized frame into a region of `self` starting at
    /// `(x0, y0)`. The tile must fit inside `self`. Used by the tile-based
    /// renderer to fold a thread's local result into the global frame.
    pub fn merge_tile(&mut self, tile: &TransientFrame, x0: usize, y0: usize) {
        debug_assert_eq!(self.num_bins, tile.num_bins);
        debug_assert!(x0 + tile.width  <= self.width);
        debug_assert!(y0 + tile.height <= self.height);
        for bin in 0..self.num_bins {
            for ly in 0..tile.height {
                let li = tile.idx(bin, 0, ly, 0);
                let gi = self.idx(bin, x0, y0 + ly, 0);
                let n = tile.width * 3;
                let (lhs, rhs) = (
                    &mut self.data[gi..gi + n],
                    &tile.data[li..li + n],
                );
                for (a, b) in lhs.iter_mut().zip(rhs) { *a += *b; }
            }
        }
    }

    /// Multiply every stored radiance value by `k`. Used for SPP
    /// normalisation: divide the accumulated buffer by the number of
    /// samples per pixel before tone-mapping.
    pub fn scale(&mut self, k: f32) { for v in &mut self.data { *v *= k; } }
}

// ---- the pulse --------------------------------------------------------------

/// Gaussian temporal pulse profile.
///
/// `sigma_seconds` controls the width of the emitted pulse. A real ultrafast
/// laser is hundreds of femtoseconds; we use something coarser by default
/// (a few picoseconds) so it spans multiple bins and produces visibly smooth
/// wavefronts at our temporal resolution.
pub struct Pulse {
    /// Standard deviation of the Gaussian envelope, in seconds.
    pub sigma: f32,
}

impl Pulse {
    /// Integrate the Gaussian over a single bin of width `dt` centered at
    /// time `t_center`. Closed-form via the error function would be cleaner;
    /// we use the midpoint approximation since `dt << sigma` typically.
    #[inline]
    pub fn weight(&self, t_center: f32) -> f32 {
        let z = t_center / self.sigma;
        (-0.5 * z * z).exp()
    }
}

// ---- the time-aware tracer --------------------------------------------------

/// Trace a single path and deposit its contribution into the frame's
/// per-pixel time histogram.
///
/// Returns nothing — the side effect is deposition. Each emissive hit
/// contributes once, weighted by:
///   - the accumulated material throughput along the path,
///   - the surface's intrinsic emitted radiance,
///   - the temporal pulse profile evaluated at the path's arrival time.
#[allow(clippy::too_many_arguments)]
pub fn trace_path(
    ray_in: Ray,
    world: &dyn Hittable,
    pulse: &Pulse,
    max_depth: u32,
    px: usize,
    py: usize,
    frame: &mut TransientFrame,
    rng: &mut Xoshiro256PlusPlus,
) {
    let mut ray = ray_in;
    let mut throughput = Vec3::ONE;

    for depth in 0..max_depth {
        // 1e-3 epsilon avoids self-intersection acne.
        let Some(rec) = world.hit(&ray, 1e-3, f32::INFINITY) else {
            // Ray escaped to the environment — no contribution. (We could
            // add a sky model here; for a closed Cornell-box-like scene we
            // don't need one.)
            return;
        };

        let emitted = rec.material.emitted();

        // Hit something emissive: deposit and terminate.
        if emitted.length_squared() > 0.0 {
            let total_length = ray.path_length + rec.t;
            let t_arrival = total_length / C;

            if t_arrival.is_finite() && t_arrival >= 0.0 {
                // A path of length L is a delta arrival at t = L/c. Convolving
                // with the source's Gaussian pulse spreads its energy across a
                // window of bins around L/c. ±4σ captures ~99.99% of the mass;
                // depositing into only the central bin (the previous behavior)
                // silently dropped most of it whenever σ >> dt.
                let half_window = (4.0 * pulse.sigma / frame.dt).ceil() as isize;
                let center_bin = (t_arrival / frame.dt) as isize;
                let first = (center_bin - half_window).max(0) as usize;
                let last_excl = ((center_bin + half_window + 1).max(0) as usize)
                    .min(frame.num_bins);

                for bin in first..last_excl {
                    let t_bin_center = (bin as f32 + 0.5) * frame.dt;
                    let w = pulse.weight(t_bin_center - t_arrival);
                    let contribution = throughput * emitted * w;
                    frame.accumulate(bin, px, py, contribution);
                }
            }
            return;
        }

        // Otherwise, scatter and continue.
        let Some(scatter) = rec.material.scatter(&ray, &rec, rng) else {
            return;
        };
        throughput *= scatter.attenuation;

        // Russian roulette after a few bounces: terminate with probability
        // proportional to (1 - throughput's max component), keeping the
        // estimator unbiased by scaling survivors.
        if depth > 3 {
            let p_continue = throughput.max_element().min(0.95);
            if rng.r#gen::<f32>() > p_continue { return; }
            throughput /= p_continue;
        }

        ray = scatter.scattered;
    }
}

// ----------------------------------------------------------------------------
// Process-spine impls. (See NOTES_PROCESS.md §1.)
// ----------------------------------------------------------------------------

/// One contribution to the time-resolved framebuffer: a path's
/// satisfaction, addressed by pixel and bin.
#[derive(Clone, Copy, Debug)]
pub struct Deposit {
    /// `(x, y)` pixel coordinate.
    pub pixel: (usize, usize),
    /// Time bin to credit.
    pub bin: usize,
    /// Linear-RGB radiance to add (already scaled by throughput and
    /// pulse weight).
    pub color: Vec3,
}

impl TransientFrame {
    /// Typed deposit point — wraps [`Self::accumulate`] with a structured
    /// [`Deposit`] value. Useful when threading the framebuffer through
    /// a generic consumer.
    pub fn deposit(&mut self, d: Deposit) {
        self.accumulate(d.bin, d.pixel.0, d.pixel.1, d.color);
    }
}
