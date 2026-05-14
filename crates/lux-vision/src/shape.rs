//! Prompt-shape presets: structured JSON nexus + relational glossary pairs
//! that drive Runway video generation via Claude-mediated translation.
//!
//! Each variant encodes a society of occasions (Whitehead-style causal chain)
//! as a JSON string. The companion glossary binds vocabulary to precise visual
//! commitments so the translator (Claude) cannot default to generic readings.
//! Requires feature `runway-video`.

// ── Orbital (bouncing-spheres ablation) ──────────────────────────────────────

const BARE_PROMPT: &str = "Two glossy spheres collide and rebound in a black void — each strike compressing briefly, then springing back along its incoming axis. A third sphere orbits the encounter on a slow elliptical path, never touching. Steady camera. Volumetric light, dark backdrop. Pure motion: contact, rebound, orbit.";

const PROSE_PROMPT: &str = "A society of five linked occasions: [1] ENTER: two glossy spheres appear at opposite frame edges in a void. [2] APPROACH (prehends 1): both translate inward at constant velocity. [3] IMPACT (prehends 2): trajectories converge at frame center; mutual compression. [4] REBOUND (prehends 3): each reverses along its incoming axis. [5] SEPARATE (prehends 4): drift to opposite edges. Parallel orbit-society: a third sphere traces an elliptical path around the encounter throughout, never touching. Camera: locked-off. Backdrop: void. Physics: Newtonian only.";

const RELATIONS_ORBITAL: &str = "\
- sphere: small glowing orb, roughly 5% of frame width
- nucleus: invisible central point at origin, indicated only by a faint glow
- orbital: spatial path/region a sphere traces; never literally drawn as a line
- p_x orbital: motion confined to two dumbbell-shaped lobes along the x-axis
- p_y orbital: same dumbbell shape along the y-axis
- d_z2 orbital: torus path around the z-axis with two cap-lobes top and bottom
- quantum_jump: instantaneous spatial discontinuity, no smooth interpolation
- 90deg phase-shifted: when A reaches its +x extremum, B reaches its +y extremum
- \"no classical collisions\": the spheres never visually touch each other
- camera \"wide hold\": locked-off shot, all three spheres in frame, no motion
- camera \"push-in on jump\": slow dolly toward the nucleus, motivated by the quantum_jump
- backdrop \"void with faint nucleus glow\": near-black field with a subtle radial glow at frame center
";

const JSON_PROMPT: &str = r#"{"society":[{"id":0,"act":"appear","prehends":[],"sat":"three glowing spheres at orbital radii around invisible central nucleus, void"},{"id":1,"act":"swing_px","prehends":[0],"sat":"sphere A swings between two lobes along x-axis (p_x orbital), full cycle per second"},{"id":2,"act":"swing_py","prehends":[0],"sat":"sphere B swings along y-axis (p_y orbital), 90deg phase-shifted from A"},{"id":3,"act":"orbit_dz2","prehends":[0],"sat":"sphere C moves continuously along a d_z2 torus path, one revolution per second"},{"id":4,"act":"quantum_jump","prehends":[1,2,3],"sat":"all three blink to higher orbitals at t=4s, return at t=5s, discrete energy transition"},{"id":5,"act":"resume","prehends":[4],"sat":"orbital motion continues smoothly through to t=8s"}],"camera":[{"t":"0-3s","act":"wide hold"},{"t":"3-5s","act":"slow push-in on jump"},{"t":"5-8s","act":"wide hold"}],"backdrop":"void with faint nucleus glow","physics":"quantum-mechanical orbital motion, no classical collisions"}"#;

// ── Fly (owl breaks the picture-plane) ───────────────────────────────────────

/// Essentia reductio on the owl (reduced to its eidos) + deictic reductio on
/// the background (bare indexicals only). The picture-plane cracks like rice
/// paper at t≈1s; the owl flies forward through the rupture and the camera
/// terminates in a full dolly-zoom to the iris at t=8s.
const RELATIONS_FLY: &str = "\
- owl: the owl of Athena from the prior segment; appearance MUST match exactly — same plumage, same posture, same luminous gaze; do not redescribe the body, the visual anchor (init image) carries it
- essentia reductio: the owl reduced to what it must be to remain owl — folded geometry of feathers, articulated talons, twin apertures of nous; nothing decorative, no embellishment
- deictic reductio: the background is stripped of representational scenery; only bare indexicals — a faint here, a fainter there, axial markers of orientation suspended in graphite-on-vellum void; no horizon, no architecture, no atmosphere
- picture-plane / 2D wall: the visible flat surface of the frame itself, treated as a real physical material that can be cracked and peeled
- crack like rice paper: a fault-line propagates along the leading edge of the primary feathers as the wings unfold; thin paper fibres tear visibly along the flight edge
- peel in paper-tearing curls: the cracked 2D surface lifts and curls slowly outward in two or three folds, revealing volumetric depth behind it
- fly forward through the rupture: the owl exits the picture-plane along the camera axis (Z-emergence into our space), NOT a 2D parallax pan
- deictic marks scatter: the bare here/there indexicals of the background, freed from their plane, drift outward like punctuation knocked from a page
- dolly-zoom on the right eye: from t≈4.5s, a single uninterrupted forward dolly with corresponding zoom-in onto the owl's right eye; pass the beak, pass the brow-feathers; do not cut, do not break the move
- iris-only at t=8s: at the final frame the entire viewport is the iris — amber striations, pupil as a black coin, the scene reflected curved and tiny on the cornea; hold one frame, then cut
- lighting gold-on-graphite: single directional low-key key light, warm gold on the owl's facing side, deep graphite shadow on the off side; no ambient fill, no colored gels
- style ink-and-gilt, hieratic: restrained, sacral, no flourish, no text, no overlays; gold leaf on dark vellum register
";

const JSON_FLY_PROMPT: &str = r#"{"society":[{"id":0,"act":"hold_threshold","prehends":[],"sat":"the inherited owl held one beat at frame center, plumage and posture identical to the prior segment, eyes lit, nothing decorative — pure eidos of the bird against a deictic-reduced background of bare here/there axial markers in graphite-on-vellum void"},{"id":1,"act":"unfold_push","prehends":[0],"sat":"at t≈1.0s the wings unfold and the owl pushes off; the leading edge of the primary feathers traces a fault-line and the picture-plane itself cracks like rice paper along that edge"},{"id":2,"act":"peel_rupture","prehends":[1],"sat":"the 2D surface peels back in slow paper-tearing curls revealing volumetric depth behind what was a wall — the flat backdrop becomes a torn aperture"},{"id":3,"act":"fly_through","prehends":[2,0],"sat":"the owl flies forward through the rupture into our space along the camera axis, eyes still lit, eidos preserved from id=0; the deictic marks of the background scatter outward like punctuation knocked from a page"},{"id":4,"act":"commit_dolly","prehends":[3],"sat":"at t≈4.5s the camera commits to a single uninterrupted dolly-zoom toward the owl's right eye, passing the beak, passing the brow-feathers, all else falling into bokeh"},{"id":5,"act":"iris_only","prehends":[4],"sat":"at t=8.0s the frame is entirely the iris — amber striations, pupil as a black coin, the world reflected curved and tiny on the cornea — hold one frame and cut"}],"camera":[{"t":"0-1s","act":"locked-off, owl held at threshold"},{"t":"1-4.5s","act":"slow forward push as the picture-plane tears and the owl emerges through the rupture"},{"t":"4.5-8s","act":"continuous dolly-zoom into the right eye, terminating fully on iris at t=8s"}],"backdrop":"deictic-reduced void: graphite-on-vellum, only faint here/there axial markers, no representational scenery; ruptures into volumetric depth from t≈1s","physics":"single continuous 8s take, directional low-key gold-on-graphite light, ink-and-gilt hieratic style; owl appearance MUST match the prior segment exactly — same bird, continuing"}"#;

// ── Reverence ─────────────────────────────────────────────────────────────────

const RELATIONS_REVERENCE: &str = "\
- figure: the bronze-armored Roman/Greek warrior visible in the init image; do not redescribe armor, plume, or cape
- coil: pre-flip wind-up — knees bent deeply, both arms swing downward and back to load momentum, weight on balls of feet
- backflip: backward somersault, exactly one full rotation, body tucked, head leads the rotation, plume traces a single vertical arc
- land: balls of both feet contact ground simultaneously, knees absorb impact, pteruges and cape settle
- kneel: right foot forward, left knee lowered to ground, right forearm laid across the bent right knee — formal salute pose
- reverence: head bowed once and held still, gaze cast downward
- locked-off: stationary camera, no pan, no dolly, no zoom; medium-wide framing keeps the full flight arc and the final kneel in frame
- backdrop \"shallow ground\": packed earth or stone underfoot, soft volumetric overhead light, dust motes settling
- physics: single continuous take, photoreal Greco-Roman epic, no cuts, no slow motion, no replays
";

const JSON_REVERENCE_PROMPT: &str = r#"{"society":[{"id":0,"act":"stand","prehends":[],"sat":"figure rooted at frame center, cape resting, gaze level, half-second hold"},{"id":1,"act":"coil","prehends":[0],"sat":"deep knee bend, both arms swing downward and back, weight loads onto balls of feet"},{"id":2,"act":"backflip","prehends":[1],"sat":"explosive vertical launch, body tucked, one full backward rotation completed by t=3.5s, plume traces a single vertical arc"},{"id":3,"act":"land","prehends":[2],"sat":"balls of both feet contact ground, knees absorb impact, pteruges and cape settle, brief stabilization"},{"id":4,"act":"kneel","prehends":[3],"sat":"right foot steps forward, left knee lowers to ground, right forearm rests across bent right knee"},{"id":5,"act":"reverence","prehends":[4],"sat":"head bows once, gaze cast down by t=7s, posture held motionless through to t=8s"}],"camera":"locked-off medium-wide that keeps the full flight arc and final kneel in frame","backdrop":"shallow stone or earth ground, soft volumetric overhead light, dust motes settling","physics":"single continuous take, photoreal Greco-Roman, no cuts, no slow motion"}"#;

// ── Lux (holomorphic prism) ───────────────────────────────────────────────────

/// Camera locked-off. No object motion. The light field alone bends and fans
/// into spectral arcs following conformal complex-plane geometry, with a
/// Möbius-like warp at peak dispersion. The scene substrate stays legible
/// through the decomposed light at the terminal rest state.
const RELATIONS_LUX: &str = "\
- holomorphic prism: a transparent crystalline solid overlaid on the frame whose refractive geometry obeys conformal complex-plane rules — it bends light along curved arcs, not straight lines, and preserves local angles at every point
- spectral decomposition: white light separated into wavelength components; long wavelengths (red ~700nm) deviate least, short wavelengths (violet ~400nm) deviate most; the full visible spectrum fans between them
- chromatic arc: the smoothly curved path that a single wavelength band traces through the holomorphic medium; each band follows its own arc, separated from its neighbours by a small angular increment
- conformal warp: image distortion that is locally angle-preserving but globally distance-distorting; objects look undistorted up close but bent at large scale — the visual signature of a holomorphic map (Möbius-like)
- wavelength copy: the same scene rendered at a slightly displaced position for each spectral band, producing rainbow-fringed halation around every high-contrast edge; six or more bands visible simultaneously
- chromatic interference: where two wavelength arcs cross, they produce additive colour mixing — a brief bright node that is neither band's hue alone
- luminous equilibrium: the terminal state — all bands fully open and held, scene substrate legible through the decomposed light field, nothing moving, a still radiating stasis
- no object motion: only the light field bends and fans; every object, figure, and surface in the scene stays exactly in place; all temporal change is in the light alone
- init image carries the scene: do not introduce new objects, figures, or backgrounds; the prism works on whatever geometry and tone is already in the frame
- lighting: the existing illumination in the frame is the source material; the prism reveals spectral content already latent in those lights and shadows
";

const JSON_LUX_PROMPT: &str = r#"{"society":[{"id":0,"act":"hold_intact","prehends":[],"sat":"the init-image frame held for one beat — every object, every light, every shadow exactly as given; no effect yet; a precise record of the scene as substrate"},{"id":1,"act":"prism_coalesces","prehends":[0],"sat":"a transparent holomorphic crystal — faces obeying conformal geometry — materialises as a luminous overlay across the frame; its presence is felt in a faint angle-preserving shimmer, not yet decomposing light"},{"id":2,"act":"first_arcs","prehends":[1],"sat":"light entering the prism begins to separate by wavelength; deep red arcs deviate barely, violet arcs curve sharply; the first thin rainbow halation appears around high-contrast edges of objects in the scene"},{"id":3,"act":"full_spectrum","prehends":[2],"sat":"maximum dispersion: the scene is visible simultaneously in six or more wavelength-shifted copies, each offset along its chromatic arc — rainbow-banded holographic overlay, the scene substrate still legible beneath"},{"id":4,"act":"conformal_warp","prehends":[3],"sat":"the wavelength bands themselves begin to curve holomorphically following complex-plane conformal rules — locally angle-preserving, globally bending; where two arcs cross, chromatic interference nodes flash briefly; the light field folds like a Möbius veil over the intact scene"},{"id":5,"act":"luminous_equilibrium","prehends":[4,0],"sat":"bands hold at full open dispersion; the original scene from id=0 is legible as substrate through the spectral veil; nothing moves; a still radiating stasis held through to t=8s"}],"camera":"locked-off throughout — no pan, no dolly, no zoom; all temporal change is in the light field alone","backdrop":"the init image, unchanged except for the light transformation; no new objects introduced","physics":"geometric optics + conformal complex-plane warp; single continuous 8s take; no object motion; no text, no overlays, no lens flare clichés — pure spectral physics"}"#;

// ── PromptShape ───────────────────────────────────────────────────────────────

#[derive(Copy, Clone, Debug)]
pub enum PromptShape {
    Json,
    Prose,
    Bare,
    Fly,
    Reverence,
    Lux,
}

impl PromptShape {
    /// Raw prompt text (JSON or prose) sent as the directive.
    pub fn text(self) -> &'static str {
        match self {
            PromptShape::Json => JSON_PROMPT,
            PromptShape::Prose => PROSE_PROMPT,
            PromptShape::Bare => BARE_PROMPT,
            PromptShape::Fly => JSON_FLY_PROMPT,
            PromptShape::Reverence => JSON_REVERENCE_PROMPT,
            PromptShape::Lux => JSON_LUX_PROMPT,
        }
    }

    /// Relational glossary paired with this shape. `Some` → the translation
    /// seam fires (Claude translates the JSON nexus into caption prose);
    /// `None` → the text is sent straight to Runway.
    pub fn glossary(self) -> Option<&'static str> {
        match self {
            PromptShape::Json => Some(RELATIONS_ORBITAL),
            PromptShape::Fly => Some(RELATIONS_FLY),
            PromptShape::Reverence => Some(RELATIONS_REVERENCE),
            PromptShape::Lux => Some(RELATIONS_LUX),
            PromptShape::Prose | PromptShape::Bare => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PromptShape::Json => "json",
            PromptShape::Prose => "prose",
            PromptShape::Bare => "bare",
            PromptShape::Fly => "fly",
            PromptShape::Reverence => "reverence",
            PromptShape::Lux => "lux",
        }
    }

    pub fn from_arg(s: &str) -> Result<Self, String> {
        match s {
            "json" => Ok(Self::Json),
            "prose" => Ok(Self::Prose),
            "bare" => Ok(Self::Bare),
            "fly" => Ok(Self::Fly),
            "reverence" => Ok(Self::Reverence),
            "lux" => Ok(Self::Lux),
            other => Err(format!(
                "--prompt-shape: unknown value '{other}', \
                 expected json|prose|bare|fly|reverence|lux"
            )),
        }
    }
}
