# Notes on Process

A companion to `README_LUX.md`. The README explains *what the renderer does*;
this document explains the metaphysical and spectral-geometric structure of
*what it is computing*. References are to Whitehead's *Process and Reality*
(corrected edition, Griffin & Sherburne 1978; cited as **PR**) and to standard
results in spectral geometry.

## 1. A path is a personally ordered society of occasions

In *Process and Reality* the basic unit of becoming is the **actual occasion**:
a definite, atomic act of experience that prehends antecedent occasions, fuses
them into a single subjective unification, and **perishes into objectivity** at
the moment its concrescence is complete. Whitehead calls this terminus the
**satisfaction** of the occasion (PR III.III).

A *society* of occasions is a nexus of occasions sharing a defining
characteristic and inheriting from one another (PR II.III). A **personally
ordered society** is one-dimensional: each occasion inherits from a single
predecessor. Whitehead's prototypes are the electron, the photon, the
"enduring object" of common sense — none of them substances, all of them
sequences.

A backward-traced path in this renderer is a personally ordered society of
exactly this form:

- Each scattering event is one occasion. It prehends the prior segment
  (direction, throughput, accumulated `path_length`) and the surface
  (BRDF, normal, emission). It satisfies into a determinate next ray.
- The path as a whole is the society. Its **defining characteristic** is the
  Markov inheritance pattern: each occasion's data is a function only of its
  immediate predecessor and the surface it strikes (`material::scatter`).
- The path **terminates** when one occasion satisfies on an emissive surface.
  At that moment the cumulative `path_length` becomes determinate, the
  contribution is binned (`transient::trace_path` deposit step), and the
  society perishes into the framebuffer.

The framebuffer is therefore not a *picture* but a **public world**: the
objectified record of all paths that have completed satisfaction. Every entry
is a perished occasion's contribution to "the public matter of fact"
(PR II.IX). What we visualise is the becoming of an objective datum out of a
multitude of subjective unifications.

## 2. Concrescence at a surface is hypergraph-shaped

A pure path graph is too narrow. Whitehead's term **concrescence** (PR III.I)
names the process by which **many become one**: a single occasion prehends a
multitude of antecedent data and fuses them into one subjective form.

In a single-bounce path tracer the concrescence at a hit point is degenerate:
exactly one incoming ray, one scattered ray. But the natural object encoding
the full geometric transfer is the **radiosity operator** $K$:

$$
K[B](x) = \int_{\partial \Omega} f_r(x, \omega_i \to \omega_o)\, V(x, y)\, \cos\theta\, B(y)\, dy
$$

— a kernel on the scene's surface manifold that takes "outgoing radiance at
$y$" to "incoming-then-reflected radiance at $x$". Equilibrium is the fixed
point $B = E + K B$; multi-bounce light is the Neumann series
$B = \sum_{n=0}^\infty K^n E$.

Read structurally, $K$ is the adjacency operator of a **continuous hypergraph**:
every surface point is a node, and the kernel weights all `(in, out, point)`
hyperedges by their geometric and material throughput. Each hyperedge is a
locus of concrescence — a place where many incoming photon flows can prehend
the surface and emit a single feeling-toned outgoing flow. Path tracing
samples this hypergraph stochastically; the renderer is one Monte Carlo
realisation of the operator's action.

Discrete radiosity (Goral et al. 1984, and the later finite-element
formulations) makes this exact: surfaces are partitioned into patches, $K$
becomes a matrix, and the hyperedge structure is literal. The path-graph
picture and the radiosity-operator picture are the same object viewed from
two scales.

## 3. The Dirichlet spectrum bounds the becoming

In spectral geometry, every bounded domain $\Omega \subset \mathbb{R}^3$ with
sufficiently regular boundary admits a discrete Dirichlet spectrum:

$$
-\Delta \varphi_n = \lambda_n \varphi_n,\quad \varphi_n|_{\partial \Omega} = 0,
\quad 0 < \lambda_1 \le \lambda_2 \le \cdots \to \infty
$$

The eigenfunctions are the room's **vibrational modes**; the eigenvalues are
its **vibrational spectrum**. Weyl's law gives the asymptotic counting
$N(\lambda) \sim (|\Omega| / 6\pi^2)\, \lambda^{3/2}$. The **smallest**
eigenvalue $\lambda_1$ is the one that matters for becoming: it is the
*slowest* mode the room admits, and its scale is set by Faber–Krahn,
$\lambda_1 \gtrsim (\pi / \mathrm{diam}\,\Omega)^2$.

The Green's function of the scalar wave equation $u_{tt} = c^2 \Delta u$ on
$\Omega$ with Dirichlet boundary decomposes in this basis:

$$
G(x, y, t) = \sum_{n \ge 1} \varphi_n(x) \varphi_n(y)\,
\frac{\sin(c \sqrt{\lambda_n}\, t)}{c \sqrt{\lambda_n}}
$$

so the longest characteristic time of the room is

$$
T_1 \;=\; \frac{2\pi}{c\sqrt{\lambda_1}} \;\approx\; \frac{2\, \mathrm{diam}\,\Omega}{c}.
$$

For the present scene (cube of edge $\sim 0.55$ m, diameter $\sim 0.95$ m):
$T_1 \approx 6.4$ ns. **The becoming of the scene as a wave-mechanical object
is bounded by this scale.** Concrescence beyond $T_1$ adds no qualitatively
new modes — only the exponentially decaying tails of those already present.

This gives a principled rule for the renderer's time horizon. Writing
$N_{\text{bins}}$ for `NUM_BINS` and $\Delta t$ for `DT`:

$$
N_{\text{bins}} \cdot \Delta t \;\gtrsim\; 3\, T_1.
$$

The factor of 3 captures the first three rings of the lowest mode, after
which higher-mode interference is the only structure remaining. With
$c = 2.998 \times 10^8$ m/s and the present scene's diameter
$\approx 0.95$ m, the principal period is $T_1 \approx 6.34$ ns and the
ceiling is $3\, T_1 \approx 19.0$ ns. The v0.1.0 defaults
($N_{\text{bins}} = 200$, $\Delta t = 40$ ps, total $8$ ns) captured only
$\approx 1.26\, T_1$ — enough to see the wavefront establish but not its
ring-down. The v0.2.0 defaults ($N_{\text{bins}} = 475$, $\Delta t = 40$ ps,
total $19.0$ ns $\le 3.00\, T_1$) sit at the threshold and admit the
spectral budget exactly; smaller scenes use proportionally fewer bins,
larger scenes proportionally more.

Light is electromagnetic and incoherent in path tracing, so the literal
Helmholtz problem differs from the scalar Dirichlet idealisation above.
But the *bound* survives: the spectral radius $\rho(K) < 1$ of the radiosity
operator gives an exactly analogous bound on bounce count,
$N_{\max} \sim |\log \rho(K)|^{-1}$.

The discrete picture is sharper still. The path society *is* a graph —
vertices are scattering events, edges are ray segments — and the renderer
samples it directly. Chung's spectral graph theory (Chung 1997, Ch. 8,
*Eigenvalues of subgraphs with boundary conditions*) gives a Dirichlet
Laplacian on any finite subgraph $S$ whose boundary $\partial S$ is held at
zero, with smallest eigenvalue $\lambda_1(S)$ that controls relaxation on
$S$ exactly as in the continuum. The closed scene picks out a subgraph; its
$\lambda_1$ is the combinatorial bound on becoming. All three pictures —
continuous Helmholtz, radiosity operator, Chung subgraph Dirichlet — agree
in their qualitative content: **a closed scene has finite becoming**, set
by its geometry.

## 4. What is rendered, then, in three sentences

Each pixel of each time slice records the public objectification of a society
of occasions whose satisfaction occurred at a definite moment within the
scene's spectral horizon. The video shows the society growing into
determinate fact, frame by 40 ps. What perishes is the path; what endures —
because the public world preserves all that is determinate (PR I.III) — is
the wavefront.

## 5. From metaphysics to API surface

In v0.2.0 the abstractions developed above are named in the type system,
in `src/process.rs`:

- **`Occasion`** (§1) — the basic unit. `Ray` implements it: its datum is
  its direction; its satisfaction (the cumulative `path_length` at the
  terminal emissive hit) is recorded by the `trace_path` loop, not on the
  ray itself.
- **`Society`** (§1) — the personally ordered nexus. `Path` implements
  it: its members are the segments; its `diameter()` reads the final
  segment's `path_length`, the quantity bounded by the budget.
- **`PublicWorld`** (§1) — the append-only objectification. `TransientFrame`
  implements it: each `Deposit` is a perished occasion's contribution.
- **`Concrescence`** (§2) — the hyperedge. Degenerate in the renderer
  (one antecedent per scatter); the example `examples/receptacle.rs`
  exercises the non-degenerate case in the vision-API setting.
- **`SpectralBudget`** (§3) — the bound on becoming, exported as both
  `try_admit(diameter) -> Result<(), BudgetError>` and `admits(d) -> bool`.
  At startup the renderer checks `NUM_BINS · DT ≤ 3 · T_1`; the default
  parameters of v0.2.0 satisfy this with a small margin.

The translation of `template.ts` in `examples/receptacle.rs` exhibits
the same five abstractions in a context where the principal period is
not seconds-of-light-travel but tokens-of-context. The renderer's bound
becomes a runaway-API guard: a vision request whose token budget exceeds
`3 · T_1(context_window)` is refused before the model is called. *That
which is above is like that which is below.*

## References

- Whitehead, A. N. *Process and Reality*. Corrected edition (Griffin &
  Sherburne, eds.). Free Press, 1978. Cited as **PR** with part-section.
- Plato. *Timaeus*. Trans. Donald J. Zeyl. Hackett, 2000. — 49a–52d for
  the **receptacle** (χώρα), the third nature alongside Being and
  Becoming, after which `examples/receptacle.rs` is named.
- Goral, C. M.; Torrance, K. E.; Greenberg, D. P.; Battaile, B. *Modeling the
  interaction of light between diffuse surfaces.* SIGGRAPH 1984. — the
  original radiosity formulation as a finite hypergraph operator.
- Chavel, I. *Eigenvalues in Riemannian Geometry.* Academic Press, 1984. —
  Faber–Krahn, Weyl's law, the heat-kernel proof.
- Chung, F. R. K. *Spectral Graph Theory.* CBMS Regional Conference Series in
  Mathematics, **No. 92**. American Mathematical Society, 1997. — Chapter 8,
  *Eigenvalues of subgraphs with boundary conditions*, gives the discrete
  Dirichlet bound for the path-society picture of §1 and §3.
- Jarabo, A.; Marco, J.; Muñoz, A.; Buisan, R.; Jarosz, W.; Gutierrez, D.
  *A Framework for Transient Rendering.* ACM TOG 2014. — the Monte Carlo
  realisation cited in the README.
