# lux-aurumque (workspace)

[![CI](https://github.com/willt08/lux-aurumque/actions/workflows/ci.yml/badge.svg)](https://github.com/willt08/lux-aurumque/actions/workflows/ci.yml)

*Lux Aurumque* — Light and Gold. A Rust workspace shipping three
focused crates that share a substrate but address different domains.

![preview](https://raw.githubusercontent.com/willt08/lux-aurumque/main/preview.gif)

## Crates

| Crate | What it does | Status |
|---|---|---|
| [`spectral-budget`](crates/spectral-budget/) | Faber–Krahn budget primitive for bounded sequential domains (token windows, time horizons, request counts). Zero deps, ~150 LoC. | [![Crates.io](https://img.shields.io/crates/v/spectral-budget.svg)](https://crates.io/crates/spectral-budget) |
| [`lux-vision`](crates/lux-vision/) | Pluggable multimodal vision pipeline with token-budget admission. Mock + Claude + Runway backends. | [![Crates.io](https://img.shields.io/crates/v/lux-vision.svg)](https://crates.io/crates/lux-vision) |
| [`lux-aurumque`](crates/lux-aurumque/) | A minimal transient path tracer: light propagating at finite speed, rendered frame by picosecond. | [![Crates.io](https://img.shields.io/crates/v/lux-aurumque.svg)](https://crates.io/crates/lux-aurumque) |

`spectral-budget` is the shared seam. Both `lux-vision` and
`lux-aurumque` depend on it. The traditional unification ("photons and
tokens bounded by the same eigenvalue") survives as a *theoretical*
register; in code, it's expressed by a single 80-line primitive crate
that each domain consumes on its own terms.

## Publish order

The three crates form a small dependency chain. Publish in this order:

```bash
cargo publish -p spectral-budget
# wait for crates.io to ingest, then
cargo publish -p lux-vision
cargo publish -p lux-aurumque
```

## Build everything

```bash
cargo check --workspace --all-targets
cargo test --workspace
cargo test -p lux-vision --features full      # exercises Anthropic + Runway code paths
```

## Repository layout

```
.
├── Cargo.toml                  # workspace manifest
├── README.md                   # this file
├── NOTES_PROCESS.md            # the philosophical register (Whitehead)
├── essays/
│   └── receptacle.rs           # conceptual artefact (Plato's χώρα), not compiled
├── .github/workflows/ci.yml
└── crates/
    ├── spectral-budget/        # Faber–Krahn budget primitive
    ├── lux-vision/             # multimodal vision pipeline
    └── lux-aurumque/           # transient path tracer
```

The split is deliberate. v0.2 of `lux-aurumque` shipped the path tracer
and the vision pipeline together, sharing one Cargo.toml because they
shared a *theory*, not a build dependency. v0.3 separates them so each
finds its own audience: the renderer lives in the graphics category;
the pipeline lives in API-bindings + asynchronous; the budget primitive
lives in mathematics.

## License

Dual MIT / Apache-2.0. © 3BSN LLC.
