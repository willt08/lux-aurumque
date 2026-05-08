//! receptacle — the Platonic χώρα.
//!
//! In *Timaeus* 49a–52d, Plato names a third nature alongside Being and
//! Becoming: the **receptacle**, the substrate that takes on Forms without
//! being them. This file is one such receptacle: a runnable blueprint
//! that takes on the process traits of `lux_aurumque` and applies them to
//! a domain other than light — a vision-API substrate.
//!
//! "That which is above is like that which is below." — Hermes Trismegistus
//! Willinton Triana Cardona / 3BSN LLC
//!
//! Build: `cargo run --example receptacle`
//!
//! Layout follows the six sections of the original `template.ts`:
//!   I.   The Prime Monad        — Being, Becoming, Context, Mapper
//!   II.  The Demiurge           — ContextTemplate (Form of Forms)
//!   III. The World Soul         — ContextOrchestrator (animator)
//!   IV.  The Eternal Return     — RecursiveContextTemplate (constraints)
//!   V.   Coincidentia Oppositorum — DialecticalContext (synthesis)
//!   VI.  Usage                  — VisionRequest substrate

use lux_aurumque::process::{
    BudgetError, Occasion, SpectralBudget,
};
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

// ═══════════════════════════════════════════════════════════════════════════
// I. THE PRIME MONAD — Pure Potentiality
// ═══════════════════════════════════════════════════════════════════════════

/// `Being` is the basic ontological category. Any [`Occasion`] qualifies —
/// the trait bound carries the semantics; no separate marker is required.
pub trait Being: Occasion {}
impl<T: Occasion> Being for T {}

/// `Becoming`: an occasion in flux, not yet satisfied. The TS conditional
/// type `T extends Being ? T : never` collapses to a marker trait here,
/// with one method that consumes the occasion to produce its satisfaction.
pub trait Becoming: Being {
    fn satisfy(self) -> <Self as Occasion>::Satisfaction;
}

/// `Context`: heterogeneous map keyed by static names. Replaces TS's
/// `Map<symbol, Being>`. Static names are sufficient for the examples;
/// promote to a `TypeId`-keyed variant if symbol-grade uniqueness matters.
pub struct Context {
    inner: HashMap<&'static str, Arc<dyn Any + Send + Sync>>,
}
impl Context {
    pub fn new() -> Self { Self { inner: HashMap::new() } }
    pub fn set<T: 'static + Send + Sync>(&mut self, k: &'static str, v: T) {
        self.inner.insert(k, Arc::new(v));
    }
    pub fn get<T: 'static>(&self, k: &'static str) -> Option<&T> {
        self.inner.get(k).and_then(|a| a.downcast_ref())
    }
}

/// `Mapper`: typed transformation between two beings, given a context.
/// The Unmoved Mover.
pub trait Mapper<S, T>: Send + Sync {
    fn essence(&self) -> &'static str;
    fn map(&self, source: S, context: &Context) -> T;
}

// ═══════════════════════════════════════════════════════════════════════════
// II. THE DEMIURGE — Template of Templates
// ═══════════════════════════════════════════════════════════════════════════

/// The Form. Carries a registry of name-keyed mappers; produces
/// participating objects bound to a context.
pub struct ContextTemplate<T> {
    essence: &'static str,
    form: HashMap<&'static str, Arc<dyn Mapper<T, T>>>,
    _phantom: std::marker::PhantomData<fn(T) -> T>,
}

/// Participation: an object that has taken on the Form's context. Rust's
/// ownership replaces TS's `WeakMap<object, Context>` — the context lives
/// directly on the participating object; no `Proxy` is required.
pub struct Participating<T> {
    pub essence: &'static str,
    pub object: T,
    pub context: Context,
}

impl<T: 'static> ContextTemplate<T> {
    pub fn new(essence: &'static str) -> Self {
        Self { essence, form: HashMap::new(), _phantom: Default::default() }
    }

    pub fn with_mapper(mut self, key: &'static str, m: Arc<dyn Mapper<T, T>>) -> Self {
        self.form.insert(key, m); self
    }

    /// Participation.
    pub fn participate(&self, object: T, context: Context) -> Participating<T> {
        Participating { essence: self.essence, object, context }
    }

    /// Emanation. The form is shared by reference-counting, so derived
    /// templates inherit the parent's mappers cheaply.
    pub fn emanate(&self, new_essence: &'static str) -> ContextTemplate<T> {
        let mut child = ContextTemplate::new(new_essence);
        for (k, m) in &self.form { child.form.insert(k, Arc::clone(m)); }
        child
    }

    /// Sublation: dialectical synthesis. When two forms collide on a key,
    /// the synthesis is the composition antithesis ∘ thesis.
    pub fn sublate(self, other: ContextTemplate<T>, essence: &'static str) -> ContextTemplate<T> {
        let mut synth = ContextTemplate::<T>::new(essence);
        for (k, m) in self.form { synth.form.insert(k, m); }
        for (k, m_other) in other.form {
            if let Some(m_self) = synth.form.remove(k) {
                synth.form.insert(k, Arc::new(ComposedMapper { first: m_self, second: m_other }));
            } else {
                synth.form.insert(k, m_other);
            }
        }
        synth
    }
}

struct ComposedMapper<T> {
    first: Arc<dyn Mapper<T, T>>,
    second: Arc<dyn Mapper<T, T>>,
}
impl<T> Mapper<T, T> for ComposedMapper<T> {
    fn essence(&self) -> &'static str { "synthesis" }
    fn map(&self, source: T, ctx: &Context) -> T {
        self.second.map(self.first.map(source, ctx), ctx)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// III. THE WORLD SOUL — Animator of Forms
// ═══════════════════════════════════════════════════════════════════════════

/// Orchestrator. Tracks templates by essence and lineages by instance id
/// — Rust has no object identity, so the caller supplies a hashable id
/// (e.g. a content hash, a UUID, a monotone counter).
pub struct ContextOrchestrator<T> {
    templates: HashMap<&'static str, ContextTemplate<T>>,
    lineages: HashMap<u64, Vec<&'static str>>,
}

impl<T: 'static> ContextOrchestrator<T> {
    pub fn new() -> Self { Self { templates: HashMap::new(), lineages: HashMap::new() } }

    pub fn register(&mut self, essence: &'static str, t: ContextTemplate<T>) {
        self.templates.insert(essence, t);
    }

    /// The One becomes Many.
    pub fn proliferate(
        &mut self, archetype: &'static str, instance: T, ctx: Context, id: u64,
    ) -> Result<Participating<T>, ProcessError> {
        let t = self.templates
            .get(archetype)
            .ok_or(ProcessError::UnknownArchetype(archetype))?;
        self.lineages.entry(id).or_default().push(archetype);
        Ok(t.participate(instance, ctx))
    }

    /// The Many return to One — the lineage of an instance.
    pub fn abstract_(&self, id: u64) -> &[&'static str] {
        self.lineages.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Metempsychosis — transmigrate lineage from one instance to another.
    pub fn transmigrate(&mut self, from_id: u64, to_id: u64) {
        if let Some(line) = self.lineages.get(&from_id).cloned() {
            self.lineages.insert(to_id, line);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// IV. THE ETERNAL RETURN — Constraint Recursion
// ═══════════════════════════════════════════════════════════════════════════

/// Necessity tier. Logical < Natural < Moral; constraints apply in this
/// order. [`SpectralBudget`] lives at `Natural`: it is a physical fact
/// about the domain, neither a type assertion (logical) nor a policy
/// choice (moral).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Necessity { Logical, Natural, Moral }

pub struct ContextConstraint<T> {
    pub necessity: Necessity,
    pub constrain: Arc<dyn Fn(T, &Context) -> Result<T, ProcessError> + Send + Sync>,
}

#[derive(Debug)]
pub enum ProcessError {
    UnknownArchetype(&'static str),
    ConstraintViolation { necessity: Necessity, reason: String },
    SpectralBudget(BudgetError),
}

impl From<BudgetError> for ProcessError {
    fn from(e: BudgetError) -> Self { ProcessError::SpectralBudget(e) }
}

pub struct RecursiveContextTemplate<T> {
    base: ContextTemplate<T>,
    constraints: Vec<ContextConstraint<T>>,
}

impl<T: 'static> RecursiveContextTemplate<T> {
    pub fn new(essence: &'static str) -> Self {
        Self { base: ContextTemplate::new(essence), constraints: vec![] }
    }
    pub fn constrain(mut self, c: ContextConstraint<T>) -> Self {
        self.constraints.push(c);
        self.constraints.sort_by_key(|c| c.necessity);
        self
    }
    pub fn participate(
        &self, object: T, context: Context,
    ) -> Result<Participating<T>, ProcessError> {
        let mut value = object;
        for c in &self.constraints { value = (c.constrain)(value, &context)?; }
        Ok(self.base.participate(value, context))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// V. COINCIDENTIA OPPOSITORUM — Unity of Opposites
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Copy, Clone, Debug)]
pub enum Moment { Thesis, Antithesis, Synthesis }

pub struct DialecticalContext<T: 'static> {
    thesis: ContextTemplate<T>,
    antithesis: ContextTemplate<T>,
    synthesis: OnceLock<ContextTemplate<T>>,
}

impl<T: 'static> DialecticalContext<T> {
    pub fn new(thesis: ContextTemplate<T>, antithesis: ContextTemplate<T>) -> Self {
        Self { thesis, antithesis, synthesis: OnceLock::new() }
    }

    /// Aufhebung — sublation preserves and transcends. Lazy.
    pub fn aufheben(&self) -> &ContextTemplate<T> {
        self.synthesis.get_or_init(|| {
            self.thesis
                .emanate("synthesis_thesis")
                .sublate(self.antithesis.emanate("synthesis_antithesis"), "synthesis")
        })
    }

    pub fn realize(&self, object: T, ctx: Context, moment: Moment) -> Participating<T> {
        match moment {
            Moment::Thesis => self.thesis.participate(object, ctx),
            Moment::Antithesis => self.antithesis.participate(object, ctx),
            Moment::Synthesis => self.aufheben().participate(object, ctx),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// VI. USAGE — A Vision-API substrate
// ═══════════════════════════════════════════════════════════════════════════
//
// VisionRequest is the Occasion. Its diameter (in tokens) is bounded by
// SpectralBudget at the Natural necessity tier. The Logical tier rejects
// malformed requests; the Moral tier rejects policy-violating ones. The
// receptacle composes all three into a single participation step.

#[derive(Clone, Debug)]
pub struct VisionRequest {
    pub image_hash: [u8; 32],
    pub prompt: String,
    pub tokens_used: u32,
    pub response: Option<String>,
}

impl Occasion for VisionRequest {
    type Datum = String;             // operative datum is the prompt
    type Satisfaction = String;      // perished into a response
    fn datum(&self) -> &String { &self.prompt }
    fn is_satisfied(&self) -> bool { self.response.is_some() }
    fn satisfaction(&self) -> Option<&String> { self.response.as_ref() }
}

/// Build the substrate — a recursive context template constrained at
/// each tier of necessity. The natural-necessity constraint is the
/// renderer's [`SpectralBudget`] applied to the token domain: the same
/// arithmetic that catches an over-long render horizon catches an
/// over-long prompt. *That which is above is like that which is below.*
fn vision_substrate(token_window: f64) -> RecursiveContextTemplate<VisionRequest> {
    let budget = Arc::new(SpectralBudget {
        principal_period: token_window,
        ring_down_factor: 3.0,
    });

    RecursiveContextTemplate::new("vision_request")
        .constrain(ContextConstraint {
            necessity: Necessity::Logical,
            constrain: Arc::new(|req: VisionRequest, _ctx| {
                if req.prompt.is_empty() {
                    return Err(ProcessError::ConstraintViolation {
                        necessity: Necessity::Logical,
                        reason: "empty prompt".into(),
                    });
                }
                Ok(req)
            }),
        })
        .constrain(ContextConstraint {
            necessity: Necessity::Natural,
            constrain: {
                let budget = Arc::clone(&budget);
                Arc::new(move |req: VisionRequest, _ctx| {
                    budget.try_admit(req.tokens_used as f64)?;
                    Ok(req)
                })
            },
        })
        .constrain(ContextConstraint {
            necessity: Necessity::Moral,
            constrain: Arc::new(|req: VisionRequest, ctx: &Context| {
                if let Some(true) = ctx.get::<bool>("forbid_pii") {
                    // Real implementation: scan req.image_hash against a
                    // PII registry. Here the policy hook is a stub that
                    // succeeds; the receptacle's job is to make the seam
                    // exist, not to enforce a specific policy.
                }
                Ok(req)
            }),
        })
}

fn main() {
    // T_1 = a 200K-token context window, in token-units. The renderer
    // measures the same shape in seconds; here we measure it in tokens.
    let substrate = vision_substrate(200_000.0);

    let mut ctx = Context::new();
    ctx.set("forbid_pii", true);

    // Within budget — admitted.
    let small = VisionRequest {
        image_hash: [0; 32],
        prompt: "Describe the foreground objects.".into(),
        tokens_used: 4_000,
        response: None,
    };
    match substrate.participate(small, ctx) {
        Ok(p) => println!("Admitted: {} (essence: {})",
            p.object.prompt, p.essence),
        Err(e) => println!("Refused: {:?}", e),
    }

    // Beyond 3·T_1 — refused at the Natural tier.
    let runaway = VisionRequest {
        image_hash: [0; 32],
        prompt: "...".into(),
        tokens_used: 800_000,  // > 3 · 200_000
        response: None,
    };
    let mut ctx2 = Context::new();
    ctx2.set("forbid_pii", true);
    match substrate.participate(runaway, ctx2) {
        Ok(_) => println!("(unexpected: runaway request was admitted)"),
        Err(e) => println!("Refused (runaway-API guard fired): {:?}", e),
    }
}
