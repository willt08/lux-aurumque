# lux-aurumque

*Lux Aurumque* — Latin, "Light and Gold." A minimal **transient path tracer**
in Rust, rendering a small gilded room as a wavefront of light sweeps through it.

![preview](https://raw.githubusercontent.com/willt08/lux-aurumque/main/preview.gif)

Standard renderers compute the steady-state radiance arriving at a sensor —
the equilibrium reached after light has bounced around forever. *Transient*
rendering refuses that simplification. Light propagates at a finite speed,
and every photon path has a definite **duration** equal to its total optical
length divided by *c*. By binning each path's contribution into a histogram
indexed by that duration, we render not a single image but a *movie of light
propagating through the scene*, frame by picosecond.

This project renders such a movie. Each output PNG is one slice of the time
histogram (40 ps wide by default), and stitching them produces a video where
you can watch a Gaussian pulse leave the source, reach the gold sphere on a
measurable delay, scatter off the warm walls, and finally return to the
camera by progressively longer paths.

## Quick start

Requirements: Rust 1.85+ (edition 2024), a C linker (`build-essential` on
Debian/Ubuntu), `ffmpeg` for the final encode.

```bash
cargo run --release
ffmpeg -framerate 30 -i frames/frame_%04d.png \
       -c:v libx264 -pix_fmt yuv420p -crf 18 lux-aurumque.mp4
```

Defaults (640×480, 256 spp, 200 time bins of 40 ps) take a few minutes on a
modern laptop and need ~800 MB of RAM. Tune `WIDTH`, `HEIGHT`, `SAMPLES`,
`NUM_BINS`, `DT` in `src/main.rs`.

## What you'll see

A Cornell-box-style room reskinned in *aurum* — cream floor / ceiling / back
wall, deep copper left wall, antique amber right wall. On the floor: a
polished gold sphere (R 1.00, G 0.78, B 0.35 — gold's spectral response in
linear sRGB) and a smaller satin-gold diffuse sphere. Near the ceiling sits
a small emitter that fires a single 50 ps Gaussian pulse, warm-tinted to
roughly 3000 K (R:G:B = 50:38:18).

Camera is at `(0, 0.27, 0.85)` m; light at `(0, 0.50, −0.30)` m, ~1.17 m apart.
Direct light therefore reaches the camera around `t ≈ 1.17 / c ≈ 3.9 ns`,
i.e. frame **~98** at `DT = 40 ps`. Things to look for:

1. **Direct light arrives first** at frame ~98 — the source disc itself.
2. **Specular highlight on the gold sphere** a few frames later, as the
   wavefront catches the polished surface.
3. **Diffuse satin-gold sphere illuminating** with a measurable delay
   relative to the gold mirror (cosine-weighted scatter, longer effective
   path).
4. **Warm color bleed onto the cream walls** arriving later still — each
   bounce adds a centimeter or two of path length.
5. **Late tails** in the last ~30 frames where multi-bounce paths trickle in.

## Architecture

```
src/
├── main.rs          entry: scene setup, tile-based parallel render, PNG writeout
├── transient.rs     time-binned framebuffer + time-aware path tracer
├── camera.rs        pinhole camera ray generation
├── hit.rs           HitRecord, Hittable trait, world list
├── sphere.rs        analytic sphere intersection
├── material.rs      Lambertian / Metal / DiffuseLight (pulse emitter)
├── ray.rs           Ray with cumulative path_length
└── vec3.rs          Vec3 conveniences over glam
```

The single conceptual delta from a textbook path tracer
([Ray Tracing in One Weekend](https://raytracing.github.io/) and friends)
lives in three places:

- **`Ray::path_length`** — every ray carries the cumulative optical length
  of its history. Each bounce adds the segment length to the next ray's
  starting `path_length`.
- **`trace_path` deposit step** — when a path terminates on an emitter, its
  contribution is deposited into a *window* of bins around
  `t_arrival = path_length / c`, weighted by the source's Gaussian temporal
  profile `p(τ) = exp(−τ² / 2σ²)`. The window spans ±4σ (~99.99% of the
  pulse mass); depositing into only the central bin would silently drop
  most of the pulse whenever `σ ≫ dt`.
- **`Pulse::weight`** — evaluates the unnormalized Gaussian at the
  bin-center offset. Energy is in arbitrary units; the global Reinhard
  exposure pass in `main.rs` normalizes brightness across all bins.

Everything else — BRDFs (Lambertian, Metal, DiffuseLight), tone mapping
(Reinhard + sRGB-ish 1/2.2 gamma), and parallelism — is conventional.

## Physics knobs

| Constant       | Default | What it controls                                                                                            |
|----------------|---------|-------------------------------------------------------------------------------------------------------------|
| `WIDTH × HEIGHT` | 640×480 | Image resolution. Memory scales as `W × H × NUM_BINS × 12 bytes`.                                         |
| `SAMPLES`      | 256     | Paths per pixel. Transient is sample-hungrier than steady-state — most paths deposit into few bins.        |
| `MAX_DEPTH`    | 8       | Hard cap on bounces; russian roulette starts terminating after depth 3.                                    |
| `DT`           | 40 ps   | Width of each time bin. Smaller = sharper wavefronts, more bins needed.                                    |
| `NUM_BINS`     | 200     | Number of time slices = video length. `NUM_BINS × DT × c` is the path-length window (default ~2.4 m).      |
| `PULSE_SIGMA`  | 50 ps   | Temporal width σ of the emitted Gaussian. Pulse spreads ±4σ across bins; `σ ≈ DT` gives smooth wavefronts. |
| `TILE_SIZE`    | 64      | Render tile edge in pixels. Per-tile partial frame is `TILE_SIZE² × NUM_BINS × 12 bytes` (≈ 9.8 MB).       |

## Parallelism

Tile-based with a single shared global frame:

1. The image is split into `TILE_SIZE × TILE_SIZE` tiles (32 tiles at
   640×480 / 64).
2. Tiles run in parallel via `rayon::par_iter`. Each thread renders its
   tile into a small private `TransientFrame` (~9.8 MB at defaults).
3. After finishing a tile, the thread briefly locks the global frame and
   merges its partial contribution at the tile's offset
   (`TransientFrame::merge_tile`).

Memory peaks at `1 × global frame + active_threads × tile frame` — about
**780 MB** at the defaults with 4 threads. The earlier `fold + reduce`
pattern allocated one full-size frame *per rayon work chunk* (16+ at high
parallelism), which OOM-killed the renderer at modest resolutions on
memory-constrained boxes (e.g. WSL2's default 7.5 GB). Tiles are the
correct primitive here.

Mutex contention is bounded by tile count, not pixel count, and the
critical section copies a few MB of floats — negligible vs. the per-tile
render time.

## What's deliberately missing

This is a teaching/portfolio implementation. The following are absent in
the name of clarity:

- **Next event estimation / explicit light sampling.** Pure brute-force
  path tracing. Convergence is correspondingly slow on small lights.
- **BVH acceleration.** With ~8 spheres, linear iteration is fine.
- **Spectral rendering.** RGB only. Real transient renderers track
  wavelength too — see Jarabo et al. for the spectral-transient framework.
- **Participating media.** No fog, no scattering volumes. Marco et al.
  (2017) extended transient rendering to media; a natural next step.
- **f16 framebuffer.** All deposits are `f32`. At 1080p the global frame
  reaches ~5 GB; halving the precision (or chunked rendering of bin ranges)
  would push the resolution ceiling further.
- **Inverse rendering.** The forward model here is the foundation for
  non-line-of-sight reconstruction — but the inverse problem is its own
  project.

## Further reading

- Jarabo, Marco, Muñoz, Buisan, Jarosz, Gutierrez. **A Framework for
  Transient Rendering.** ACM TOG 2014.
- Velten et al. **Femto-Photography: Capturing and Visualizing the
  Propagation of Light.** SIGGRAPH 2013.
- Jarabo's PhD thesis, *Femto-Photography: Visualizing Light in Motion*,
  2015 — the most readable single document on the subject.

## On what is rendered

What the renderer is *computing* — beyond the pixel-side description above —
is summarised in a companion note, [NOTES_PROCESS.md](NOTES_PROCESS.md):
the path-traced society of occasions in the sense of Whitehead's
*Process and Reality*, the radiosity operator as a continuous hypergraph,
and the Dirichlet spectrum of the scene as the bound on its becoming. That
last point yields a concrete heuristic: `NUM_BINS · DT ≳ 3 T_1`, where
`T_1 ≈ 2 · diam(Ω) / c` is the period of the room's lowest mode.
For the default scene `T_1 ≈ 6.4 ns`; the defaults sit at `1.25 T_1`,
so doubling `NUM_BINS` is the principled move when fidelity to the
ring-down matters.

## License

Dual MIT / Apache-2.0.
