//! The process-philosophical spine of the crate.
//!
//! This module names the abstractions the renderer was already computing.
//! Existing types (`Ray`, `TransientFrame`) pick up trait impls; downstream
//! consumers can implement the same traits over their own kinds of flux —
//! a vision API, an audio synthesizer, an agent runtime — and inherit the
//! same coherence guarantees, most importantly the [`SpectralBudget`] bound
//! that keeps becoming finite.
//!
//! Cross-references of the form *§N* point to `NOTES_PROCESS.md`.

/// An actual occasion of becoming. Prehends antecedent data, concresces,
/// and perishes into a determinate satisfaction. The unit of inheritance
/// in a society. (§1.)
pub trait Occasion {
    /// What the occasion takes account of: the prehended data.
    type Datum;
    /// What the occasion perishes into: its objectified contribution.
    type Satisfaction;

    fn datum(&self) -> &Self::Datum;
    fn is_satisfied(&self) -> bool;
    fn satisfaction(&self) -> Option<&Self::Satisfaction>;
}

/// A nexus of occasions sharing a defining characteristic and inheriting
/// from one another. Personally ordered if inheritance is one-dimensional.
/// (§1.)
pub trait Society {
    type Member: Occasion;
    fn members(&self) -> &[Self::Member];
    /// Diameter in the natural metric of the society — path length, tokens,
    /// duration, samples — whatever the domain measures becoming with.
    /// This is the quantity bounded by [`SpectralBudget`].
    fn diameter(&self) -> f64;
}

/// A concrescence node: an occasion that prehends *many* antecedents — a
/// hyperedge — unifying them into one satisfaction. The radiosity operator
/// made explicit. Degenerate (single-antecedent) for the path tracer;
/// non-trivial for tool-fanning vision and agent substrates. (§2.)
pub trait Concrescence {
    type Antecedent;
    type Unified;
    fn prehensions(&self) -> &[Self::Antecedent];
    fn unify(self) -> Self::Unified;
}

/// The public world: append-only objectification of perished occasions.
/// New occasions prehend from it rather than reconstructing the past. (§1.)
pub trait PublicWorld {
    type Inhabitant;
    fn deposit(&mut self, x: Self::Inhabitant);
}

/// Bound on the diameter of a society before its becoming exceeds the
/// principal eigenvalue of the underlying domain. (§3.)
///
/// For a closed scene of diameter `d` with wave-speed `c`, the principal
/// period is `T_1 ≈ 2d/c` (Faber–Krahn). The renderer's recommendation is
/// that the total time horizon `NUM_BINS · DT` not exceed `3·T_1` —
/// capturing the first three rings of the lowest mode. The same arithmetic
/// ports directly to other domains: token windows for vision APIs,
/// duration windows for audio events, depth limits for agent reasoning
/// chains.
#[derive(Copy, Clone, Debug)]
pub struct SpectralBudget {
    /// `T_1` — period of the lowest mode of the bounded domain.
    pub principal_period: f64,
    /// Multiplier on `T_1` defining the admission ceiling. 3.0 by default.
    pub ring_down_factor: f64,
}

impl SpectralBudget {
    /// Continuous Dirichlet bound for a scene of diameter `diam_m` (meters)
    /// with propagation speed `c` (m/s). `T_1 = 2·diam/c`.
    pub fn for_scene_diameter(diam_m: f64, c: f64) -> Self {
        Self { principal_period: 2.0 * diam_m / c, ring_down_factor: 3.0 }
    }

    /// Strict admission. Returns `Ok(())` if the diameter sits within the
    /// `ring_down_factor · T_1` ceiling; otherwise returns the structured
    /// violation so the caller can decide whether to abort, downgrade, or
    /// surface the error.
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

    /// Boolean shortcut where the caller does not need violation detail.
    pub fn admits(&self, diameter: f64) -> bool {
        self.try_admit(diameter).is_ok()
    }
}

/// Failure modes of [`SpectralBudget`]. Designed to be informative on
/// `Display` so an end user sees a complete diagnosis at the prompt.
#[derive(Debug)]
pub enum BudgetError {
    Exceeded {
        diameter: f64,
        bound: f64,
        principal_period: f64,
        ring_down_factor: f64,
    },
}

impl std::fmt::Display for BudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetError::Exceeded { diameter, bound, principal_period, ring_down_factor } => {
                write!(
                    f,
                    "spectral budget exceeded: diameter {:.3e} > bound {:.3e} \
                     (T_1 = {:.3e}, ring-down factor = {:.2}); a closed domain \
                     has finite becoming — see NOTES_PROCESS.md §3",
                    diameter, bound, principal_period, ring_down_factor
                )
            }
        }
    }
}

impl std::error::Error for BudgetError {}
