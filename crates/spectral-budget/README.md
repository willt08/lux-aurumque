# spectral-budget

[![Crates.io](https://img.shields.io/crates/v/spectral-budget.svg)](https://crates.io/crates/spectral-budget)
[![docs.rs](https://img.shields.io/docsrs/spectral-budget)](https://docs.rs/spectral-budget)
[![License](https://img.shields.io/crates/l/spectral-budget.svg)](https://crates.io/crates/spectral-budget)

A budget primitive for bounded sequential domains: token windows, time
horizons, request counts — anything where you need to refuse work that
would exceed a principled ceiling, and surface a structured reason when
you do.

```rust
use spectral_budget::SpectralBudget;

// 200 000-token context window; admit up to 3 ring-down periods.
let budget = SpectralBudget {
    principal_period: 200_000.0,
    ring_down_factor: 3.0,
};

assert!(budget.admits(400_000.0));
assert!(budget.try_admit(700_000.0).is_err());
```

## Why this shape

The framing comes from the Faber–Krahn inequality on the principal
eigenvalue of a bounded Laplacian: a closed domain of diameter `d` with
propagation speed `c` has a slowest mode with period `T_1 ≈ 2·d/c`.
Capping aggregate diameter at `k · T_1` (default `k = 3`) keeps becoming
finite. The same arithmetic applies whether `d` is photon path-length,
an LLM token window, an audio duration, or an agent reasoning depth.

In practice this means:

- `principal_period` is *your domain's natural period*: context-window
  size for LLM calls, total path-length horizon for a renderer, max
  audio bar length for an audio bed.
- `ring_down_factor` is *how many of those periods you'll tolerate*
  before refusing. `3.0` is the well-tempered default — three rings of
  the slowest mode.
- `try_admit` returns a structured `BudgetError::Exceeded` carrying the
  diameter, the bound, `T_1`, and the factor — diagnosable at the call
  site without inspecting internals.

## License

Dual MIT / Apache-2.0. © 3BSN LLC.
