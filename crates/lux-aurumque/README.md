# lux-aurumque

[![Crates.io](https://img.shields.io/crates/v/lux-aurumque.svg)](https://crates.io/crates/lux-aurumque)
[![docs.rs](https://img.shields.io/docsrs/lux-aurumque)](https://docs.rs/lux-aurumque)
[![License](https://img.shields.io/crates/l/lux-aurumque.svg)](https://crates.io/crates/lux-aurumque)
[![MSRV](https://img.shields.io/crates/msrv/lux-aurumque.svg)](https://github.com/willt08/lux-aurumque)

*Lux Aurumque* — Light and Gold. A minimal **transient path tracer** in
Rust, rendering a small gilded room as a wavefront of light sweeps
through it.

![preview](https://raw.githubusercontent.com/willt08/lux-aurumque/main/preview.gif)

Standard renderers compute the steady-state radiance arriving at a
sensor — the equilibrium reached after light has bounced around forever.
*Transient* rendering refuses that simplification. Light propagates at a
finite speed, and every photon path has a definite **duration** equal
to its total optical length divided by *c*. By binning each path's
contribution into a histogram indexed by that duration, we render not a
single image but a *movie of light propagating through the scene*,
frame by picosecond.

## Run the demo

Requirements: Rust 1.88+ (edition 2024), a C linker (`build-essential`
on Debian/Ubuntu), `ffmpeg` for the final encode.

```bash
cargo run --release -p lux-aurumque
ffmpeg -framerate 30 -i frames/frame_%04d.png \
       -c:v libx264 -pix_fmt yuv420p -crf 18 lux-aurumque.mp4
```

Defaults (640×480, 256 spp, 475 time bins of 40 ps) take a few minutes
on a modern laptop and need ~800 MB of RAM. Tune `WIDTH`, `HEIGHT`,
`SAMPLES`, `NUM_BINS`, `DT` at the top of `src/main.rs`.

## What you'll see

A Cornell-box-style room reskinned in *aurum* — cream floor / ceiling /
back wall, deep copper left wall, antique amber right wall. On the
floor: a polished gold sphere and a smaller satin-gold diffuse sphere.
Near the ceiling sits a small emitter that fires a single 50 ps
Gaussian pulse, warm-tinted to roughly 3000 K.

- **Direct light arrives first** at frame ~98 — the source disc itself.
- **Specular highlight on the gold sphere** a few frames later, as the
  wavefront catches the polished surface.
- **Diffuse satin-gold sphere illuminating** with a measurable delay
  (cosine-weighted scatter, longer effective path).
- **Warm colour bleed onto the cream walls** arriving later still —
  each bounce adds a centimetre or two of path length.
- **Late tails** in the last frames where multi-bounce paths trickle in.

## Library use

The crate re-exports
[`SpectralBudget`](https://docs.rs/spectral-budget) — the Faber–Krahn
bound that refuses parameter combinations whose total path-length
horizon would exceed `3 · T_1` for the bounded scene. The renderer
enforces this at startup; combinations that violate the budget are
refused with a structured error before any computation runs.

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
├── scene.rs         Cornell-box scene setup
└── vec3.rs          Vec3 conveniences over glam
```

The single conceptual delta from a textbook path tracer
([Ray Tracing in One Weekend](https://raytracing.github.io/) and
friends) lives in three places:

- **`Ray::path_length`** — every ray carries the cumulative optical
  length of its history. Each bounce adds the segment length to the
  next ray's starting `path_length`.
- **`trace_path` deposit step** — when a path terminates on an emitter,
  its contribution is deposited into a *window* of bins around
  `t_arrival = path_length / c`, weighted by the source's Gaussian
  temporal profile.
- **`TransientFrame`** — a 3D `(height × width × num_bins)` buffer,
  saved out as a per-bin PNG stack.

## Physics knobs

| Constant | Default | Effect |
|---|---|---|
| `WIDTH × HEIGHT` | 640×480 | Resolution — memory scales as `W × H × NUM_BINS × 12 bytes` |
| `SAMPLES` | 256 | Paths per pixel |
| `DT` | 40 ps | Time-bin width — smaller = sharper wavefronts |
| `NUM_BINS` | 475 | Time slices; `NUM_BINS × DT × c` is the path-length window |
| `PULSE_SIGMA` | 50 ps | Gaussian pulse temporal width |
| `TILE_SIZE` | 64 | Render tile edge; controls peak memory |

## Further reading

- Jarabo et al. **A Framework for Transient Rendering.** ACM TOG 2014.
- Velten et al. **Femto-Photography.** SIGGRAPH 2013.
- Whitehead, **Process and Reality** (1929) — the philosophical
  substrate for the workspace's process register. See
  [`NOTES_PROCESS.md`](../../NOTES_PROCESS.md) in the workspace root for
  the mapping onto Whitehead's ontology.

## License

Dual MIT / Apache-2.0. © 3BSN LLC.
