//! A budget primitive for bounded sequential domains: token windows,
//! time horizons, request counts, anything where you need to refuse
//! work that would exceed a principled ceiling.
//!
//! The framing comes from the Faber–Krahn inequality on the principal
//! eigenvalue of a bounded Laplacian: a closed domain of diameter `d`
//! with propagation speed `c` has a slowest mode with period
//! `T_1 ≈ 2·d/c`. Capping aggregate diameter at `k · T_1` (default
//! `k = 3`) keeps becoming finite. The same arithmetic applies whether
//! `d` is photon path-length, an LLM token window, or an agent
//! reasoning depth.
//!
//! ## Example
//!
//! ```
//! use spectral_budget::SpectralBudget;
//!
//! // 200 000-token context window; admit up to 3·T_1.
//! let budget = SpectralBudget {
//!     principal_period: 200_000.0,
//!     ring_down_factor: 3.0,
//! };
//!
//! assert!(budget.admits(400_000.0));
//! assert!(budget.try_admit(700_000.0).is_err());
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]

use std::fmt;

/// A ceiling on the diameter a bounded sequential domain may reach
/// before its becoming exceeds the principal eigenvalue of the
/// underlying Laplacian.
///
/// For a closed scene of diameter `d` with wave-speed `c`, the principal
/// period is `T_1 ≈ 2·d/c` (Faber–Krahn). The recommended ceiling is
/// `ring_down_factor · T_1` — capturing the first few rings of the
/// lowest mode.
#[derive(Copy, Clone, Debug)]
pub struct SpectralBudget {
    /// `T_1` — period of the lowest mode of the bounded domain.
    pub principal_period: f64,
    /// Multiplier on `T_1` defining the admission ceiling. `3.0` by
    /// default — three ring-down cycles of the principal mode.
    pub ring_down_factor: f64,
}

impl SpectralBudget {
    /// Continuous Dirichlet bound for a scene of diameter `diam_m`
    /// (metres) with propagation speed `c` (m/s). `T_1 = 2·diam/c`.
    /// Useful for physical (path-tracer-style) budgets.
    pub fn for_scene_diameter(diam_m: f64, c: f64) -> Self {
        Self {
            principal_period: 2.0 * diam_m / c,
            ring_down_factor: 3.0,
        }
    }

    /// Strict admission. Returns `Ok(())` if the diameter sits within
    /// the `ring_down_factor · principal_period` ceiling; otherwise
    /// returns the structured violation so the caller can decide to
    /// abort, downgrade, or surface the error.
    #[must_use = "handle the Err variant to surface budget violations"]
    pub fn try_admit(&self, diameter: f64) -> Result<(), BudgetError> {
        let bound = self.ring_down_factor * self.principal_period;
        if diameter <= bound {
            Ok(())
        } else {
            Err(BudgetError::Exceeded {
                diameter,
                bound,
                principal_period: self.principal_period,
                ring_down_factor: self.ring_down_factor,
            })
        }
    }

    /// Boolean shortcut where the caller does not need violation
    /// detail. Prefer [`Self::try_admit`] when you need to surface the
    /// reason.
    #[must_use]
    pub fn admits(&self, diameter: f64) -> bool {
        self.try_admit(diameter).is_ok()
    }
}

/// Failure modes of [`SpectralBudget`]. The `Display` impl is
/// informative enough to surface directly at a user-facing prompt.
#[derive(Debug)]
#[non_exhaustive]
pub enum BudgetError {
    /// The candidate diameter exceeded `ring_down_factor · principal_period`.
    Exceeded {
        /// The diameter that was rejected.
        diameter: f64,
        /// The admission ceiling at the time of the call.
        bound: f64,
        /// `T_1`, the principal period of the bounded domain.
        principal_period: f64,
        /// The multiplier applied to `T_1` to compute `bound`.
        ring_down_factor: f64,
    },
}

impl fmt::Display for BudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BudgetError::Exceeded {
                diameter,
                bound,
                principal_period,
                ring_down_factor,
            } => write!(
                f,
                "spectral budget exceeded: diameter {diameter:.3e} > \
                 bound {bound:.3e} (T_1 = {principal_period:.3e}, \
                 ring-down factor = {ring_down_factor:.2})"
            ),
        }
    }
}

impl std::error::Error for BudgetError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_within_bound() {
        let b = SpectralBudget { principal_period: 100.0, ring_down_factor: 3.0 };
        assert!(b.admits(0.0));
        assert!(b.admits(100.0));
        assert!(b.admits(300.0));
        assert!(!b.admits(300.1));
    }

    #[test]
    fn exceeded_carries_diagnostic_fields() {
        let b = SpectralBudget { principal_period: 100.0, ring_down_factor: 3.0 };
        match b.try_admit(500.0) {
            Err(BudgetError::Exceeded { diameter, bound, principal_period, ring_down_factor }) => {
                assert_eq!(diameter, 500.0);
                assert_eq!(bound, 300.0);
                assert_eq!(principal_period, 100.0);
                assert_eq!(ring_down_factor, 3.0);
            }
            other => panic!("expected Exceeded, got {other:?}"),
        }
    }

    #[test]
    fn for_scene_diameter_matches_faber_krahn() {
        const C: f64 = 2.998e8;
        let b = SpectralBudget::for_scene_diameter(0.95, C);
        assert!((b.principal_period - 2.0 * 0.95 / C).abs() < 1e-18);
        assert_eq!(b.ring_down_factor, 3.0);
    }
}
