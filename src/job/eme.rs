//! The `"eme"` job: a 2D structure of sections along z, straights and tapers, by eigenmode
//! expansion ([`crate::eme`]).
//!
//! Each section is a stack of layers across x between the window's two walls: a core of some
//! thickness centred in the cladding (a slab), or layers given one by one. A taper's core
//! changes thickness along it, linearly, parabolically (its square linear) or exponentially,
//! and is cut into steps: equal ones, or placed where the response changes until an estimated
//! error is within `[task.adaptive]`'s tolerance ([`crate::eme::Eme::adaptive`]). The ports are
//! the first and last sections, straights. `[task.convergence]` drives the number of modes (and
//! the equal steps) until the transmission or the reflection of the fundamental modes changes by
//! less than its tolerance ([`crate::convergence::ToTolerance`]). A `[task.sweep]` over the
//! wavelength repeats the whole at each point; with `[task.interpolate]` it solves only where
//! its tolerance needs, the modes and steps settled once at `wavelength_um`, and records S at
//! the sweep's points from the interpolant ([`crate::eme::Spectrum`]) with its estimated error.
//! `[task.period]` repeats some of the sections `count` times, the structure solved by repeated
//! squaring of one period's S-matrix ([`crate::eme::Eme::grating`]), and records the period's
//! Bloch modes ([`crate::eme::Bloch`]) at each wavelength it solves as a sweep point.
//!
//! The run records its scene (the cores seen from above, z along x), the solver's settings,
//! the S-parameters between the guided modes at the two ends (at each wavelength), and each
//! convergence study.

use num_complex::Complex64 as c64;
use schemars::{JsonSchema, SchemaGenerator};
use serde::Deserialize;

use super::execute::solver_run;
use super::kind::TaskKind;
use super::scene::{SceneLayer, SceneMedium, SceneShape};
use super::{Event, check_finite, check_sweep, task_error};
use crate::Result;
use crate::convergence::{Convergence, Refined, ToTolerance, Tolerance, Verdict};
use crate::eme::{
    Eme, Piece, Scattering, Section, Slice, Spectrum, Taper, Wall, entries as spectrum_entries,
};
use crate::material::Material;
use crate::mode::Polarization;
use crate::run::{Job, Run, Stop};
use crate::sweep::Options;
use crate::units::{Length, Wavelength};

mod three;

/// A section's core thickness: one for a straight, [from, to] for a taper.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
enum Thickness {
    Straight(f64),
    Taper([f64; 2]),
}

/// A layer of a section given layer by layer.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct LayerSpec {
    /// Its medium: a refractive index, or a catalogue entry.
    #[schemars(with = "crate::job::schema::Medium")]
    medium: toml::Value,
    /// Its thickness, µm.
    thickness_um: f64,
}

/// A `[[task.section]]`: a straight or a taper, in order along z.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SectionSpec {
    /// Its name, for the record: `"section k"` by default.
    name: Option<String>,
    /// Its length along z, µm: 0 for a port's reference plane.
    length_um: f64,
    /// The core's thickness across the window, µm: one number for a straight, `[from, to]` for a
    /// taper.
    core_um: Option<Thickness>,
    /// The core's centre, µm from the window's middle: 0 by default.
    center_um: Option<f64>,
    /// The core's medium: the task's `core` by default.
    #[schemars(with = "Option<crate::job::schema::Medium>")]
    core: Option<toml::Value>,
    /// Instead of a core, a straight's layers from the lower wall, filling the window.
    layers: Option<Vec<LayerSpec>>,
    /// A taper's profile: `"linear"` (the default), `"parabolic"` (the thickness squared
    /// linear) or `"exponential"`.
    profile: Option<String>,
    /// A taper's steps: equal ones, or the first ones `[task.adaptive]` refines; 10 by default.
    steps: Option<usize>,
}

/// A `[task.pml]`: a perfectly matched layer inside the walls.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PmlSpec {
    /// Its thickness, µm.
    thickness_um: f64,
    /// Its strength σ, the imaginary part of its thickness over the real part.
    strength: f64,
    /// `"both"` walls (the default) or the `"upper"` one only, the lower being a plane of
    /// symmetry.
    sides: Option<String>,
}

/// A `[task.adaptive]`: the tapers' steps placed where the response changes.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AdaptiveSpec {
    /// The tolerance on the fundamental modes' entries of S (absolute; they are at most 1).
    tolerance: f64,
}

/// A `[task.convergence]`: the number of modes (and the equal steps) driven to a tolerance.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ConvergenceSpec {
    /// The tolerance, relative to the quantity.
    tolerance: f64,
    /// `"transmission"` (the default), |S21|² between the fundamental modes, or `"reflection"`,
    /// |S11|².
    quantity: Option<String>,
}

/// A `[task.interpolate]`: S over `[task.sweep]`'s wavelengths from as few solves as a
/// tolerance needs ([`crate::eme::Spectrum`]), the discretisation settled at `wavelength_um`.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct InterpolateSpec {
    /// The tolerance on every entry of S between the ports' guided modes, relative to the
    /// largest |S_qp| solved (they are at most 1: about an absolute one); at least 1e-13.
    tolerance: f64,
    /// The most solves: 200 by default, at least 5.
    max_solves: Option<usize>,
}

/// A `[task.period]`: some of the sections, in order, repeated as a grating.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PeriodSpec {
    /// The first and the last of the sections that make one period, counted from 1 in the
    /// order of `[[task.section]]`; not the ports (the first and last sections).
    sections: [usize; 2],
    /// How many periods, at least 1.
    count: usize,
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct WavelengthSweep {
    #[allow(dead_code)] // only the wavelength is swept; checked below
    parameter: String,
    from: f64,
    to: f64,
    points: usize,
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct EmeTask {
    #[allow(dead_code)] // looked up by the registry (job::kind); kept for serde and the schema
    kind: String,
    /// The vacuum wavelength, µm.
    wavelength_um: f64,
    /// `"te"` (the default; E along y, the direction the structure is uniform in) or `"tm"`.
    polarization: Option<String>,
    /// The modes each section keeps (in 3D at most the plane's unknowns: more keeps them all).
    modes: usize,
    /// The window's width across, wall to wall inside any PML, µm (the upper half's, when
    /// `symmetric`): a 2D job's.
    window_um: Option<f64>,
    /// The lower and upper walls, `"electric"` (the default) or `"magnetic"`.
    walls: Option<[String; 2]>,
    /// The structure is symmetric about its middle and only its even modes are wanted: its
    /// upper half is solved, the plane of symmetry the lower wall (magnetic for TE, electric
    /// for TM) and a PML at the upper wall only, with half the modes for the same result. The
    /// cores are centred, `window_um` is the half's width, and layers are the half's.
    symmetric: Option<bool>,
    pml: Option<PmlSpec>,
    /// What surrounds the cores: a refractive index or a catalogue entry (a `[device]`'s
    /// stack has its own).
    #[schemars(with = "Option<crate::job::schema::Medium>")]
    cladding: Option<toml::Value>,
    /// The cores' medium, unless a section gives its own.
    #[schemars(with = "Option<crate::job::schema::Medium>")]
    core: Option<toml::Value>,
    /// The sections in order along z (a `[device]`'s are cut from its geometry).
    #[serde(default)]
    section: Vec<SectionSpec>,
    /// The sections in 3D: each cross-section a plane of Yee's grid ([`three`]).
    plane: Option<three::PlaneSpec>,
    adaptive: Option<AdaptiveSpec>,
    convergence: Option<ConvergenceSpec>,
    sweep: Option<WavelengthSweep>,
    /// S over the sweep's wavelengths interpolated from a few solves, with its error stated.
    interpolate: Option<InterpolateSpec>,
    /// Some sections repeated: a grating, solved by repeated squaring of one period's S-matrix,
    /// and its Bloch modes recorded.
    period: Option<PeriodSpec>,
}

/// A section resolved at a wavelength: its name, length, and how its cross-section is made.
#[derive(Clone, Debug)]
struct Resolved {
    name: String,
    length: f64,
    shape: Shape,
}

#[derive(Clone, Debug)]
enum Shape {
    /// Layers from the lower wall.
    Layers(Vec<(c64, f64)>),
    /// A core in the cladding: its index, centre, and thickness at each end with the profile.
    Core {
        core: c64,
        cladding: c64,
        center: f64,
        half: bool,
        from: f64,
        to: f64,
        profile: Profile,
        steps: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Profile {
    Linear,
    Parabolic,
    Exponential,
}

impl Profile {
    fn named(name: Option<&str>) -> Result<Profile> {
        match name.unwrap_or("linear") {
            "linear" => Ok(Profile::Linear),
            "parabolic" => Ok(Profile::Parabolic),
            "exponential" => Ok(Profile::Exponential),
            other => Err(task_error(format!(
                "a taper's profile is \"linear\", \"parabolic\" or \"exponential\", got \"{other}\""
            ))),
        }
    }

    /// The thickness a share `u` (0 to 1) along a taper from `a` to `b`.
    fn at(self, a: f64, b: f64, u: f64) -> f64 {
        match self {
            Profile::Linear => a + (b - a) * u,
            Profile::Parabolic => (a * a + (b * b - a * a) * u).sqrt(),
            Profile::Exponential => a * (b / a).powf(u),
        }
    }
}

impl EmeTask {
    /// A 2D job's window, wall to wall (checked by [`EmeTask::validate`]).
    fn window(&self) -> f64 {
        self.window_um.unwrap_or(f64::NAN)
    }

    /// The cladding, which a task alone must give.
    fn cladding(&self) -> Result<&toml::Value> {
        self.cladding
            .as_ref()
            .ok_or_else(|| task_error("an eme job needs a cladding: an index or a catalogue entry"))
    }

    fn polarization(&self) -> Result<Polarization> {
        match self.polarization.as_deref().unwrap_or("te") {
            "te" | "TE" => Ok(Polarization::Te),
            "tm" | "TM" => Ok(Polarization::Tm),
            other => Err(task_error(format!(
                "polarization is \"te\" or \"tm\", got \"{other}\""
            ))),
        }
    }

    fn walls(&self) -> Result<[Wall; 2]> {
        let wall = |w: &str| match w {
            "electric" => Ok(Wall::Electric),
            "magnetic" => Ok(Wall::Magnetic),
            other => Err(task_error(format!(
                "a wall is \"electric\" or \"magnetic\", got \"{other}\""
            ))),
        };
        if self.symmetric() {
            // the plane of symmetry, where the even modes' tangential H vanishes (TE) or their
            // tangential E does (TM)
            let plane = match self.polarization()? {
                Polarization::Te => Wall::Magnetic,
                Polarization::Tm => Wall::Electric,
            };
            return Ok([plane, Wall::Electric]);
        }
        match &self.walls {
            None => Ok([Wall::Electric; 2]),
            Some([a, b]) => Ok([wall(a)?, wall(b)?]),
        }
    }

    fn symmetric(&self) -> bool {
        self.symmetric == Some(true)
    }

    /// The wavelengths it solves at: the sweep's, else its own.
    fn wavelengths(&self) -> Vec<f64> {
        match &self.sweep {
            Some(s) => (0..s.points)
                .map(|k| s.from + (s.to - s.from) * k as f64 / (s.points - 1) as f64)
                .collect(),
            None => vec![self.wavelength_um],
        }
    }

    /// The expansion's settings at `wavelength` with `modes` modes.
    fn eme(&self, wavelength: Wavelength, modes: usize) -> Result<Eme> {
        let [lower, upper] = self.walls()?;
        let eme = Eme::new(self.polarization()?, wavelength, modes)?.with_walls(lower, upper);
        match &self.pml {
            None => Ok(eme),
            Some(p) if self.symmetric() => {
                eme.with_upper_pml(Length::um(p.thickness_um), p.strength)
            }
            Some(p) => match p.sides.as_deref().unwrap_or("both") {
                "both" => eme.with_pml(Length::um(p.thickness_um), p.strength),
                "upper" => eme.with_upper_pml(Length::um(p.thickness_um), p.strength),
                other => Err(task_error(format!(
                    "[task.pml] sides is \"both\" or \"upper\", got \"{other}\""
                ))),
            },
        }
    }

    /// What [`EmeTask::sections`] and the check both refuse before solving anything; `device`
    /// when the job has a `[device]`, whose sections are cut from it.
    fn validate(&self, device: bool) -> Result<()> {
        Wavelength::um(self.wavelength_um).map_err(|e| task_error(e.to_string()))?;
        if self.modes == 0 {
            return Err(task_error("modes must be at least 1"));
        }
        if let Some(plane) = &self.plane {
            self.validate_common()?;
            return three::validate(self, plane, device);
        }
        if device {
            return Err(task_error(
                "an eme job beside a [device] cuts it into 3D sections: give [task.plane]",
            ));
        }
        let window = self.window();
        if !(window.is_finite() && window > 0.0) {
            return Err(task_error(format!(
                "window_um must be a positive width, got {}",
                self.window_um
                    .map_or_else(|| "none".to_owned(), |w| w.to_string())
            )));
        }
        self.cladding()?;
        self.validate_common()?;
        self.validate_2d()
    }

    /// What 2D and 3D jobs both refuse: the PML's, the adaptive steps', the convergence's and the
    /// sweep's settings.
    fn validate_common(&self) -> Result<()> {
        if let Some(p) = &self.pml
            && !(p.thickness_um.is_finite()
                && p.thickness_um > 0.0
                && p.strength.is_finite()
                && p.strength > 0.0)
        {
            return Err(task_error(format!(
                "[task.pml] needs a positive thickness_um and strength, got {} and {}",
                p.thickness_um, p.strength
            )));
        }
        if let Some(a) = &self.adaptive
            && !(a.tolerance.is_finite() && a.tolerance > 0.0)
        {
            return Err(task_error(format!(
                "[task.adaptive] tolerance must be above 0, got {}",
                a.tolerance
            )));
        }
        if let Some(c) = &self.convergence {
            if !(c.tolerance.is_finite() && c.tolerance > 0.0) {
                return Err(task_error(format!(
                    "[task.convergence] tolerance must be above 0, got {}",
                    c.tolerance
                )));
            }
            quantity(c.quantity.as_deref())?;
        }
        if let Some(s) = &self.sweep {
            if s.parameter != "wavelength" {
                return Err(task_error(format!(
                    "an eme job sweeps the wavelength, not \"{}\"",
                    s.parameter
                )));
            }
            check_sweep(s.from, s.to, s.points)?;
            for w in [s.from, s.to] {
                Wavelength::um(w).map_err(|e| task_error(e.to_string()))?;
            }
        }
        if let Some(p) = &self.period {
            let [a, b] = p.sections;
            let n = self.section.len();
            if n == 0 {
                return Err(task_error(
                    "[task.period] repeats some of the [[task.section]]: a [device]'s sections                      are cut from its geometry",
                ));
            }
            if !(a >= 2 && a <= b && b < n) {
                return Err(task_error(format!(
                    "[task.period] sections are the first and last of one period, counted from 1,                      between the ports (sections 2 to {}), got [{a}, {b}]",
                    n.saturating_sub(1)
                )));
            }
            if p.count == 0 {
                return Err(task_error("[task.period] count must be at least 1"));
            }
            if self.adaptive.is_some() {
                return Err(task_error(
                    "[task.adaptive] places the steps of the whole structure: a job with a                      [task.period] takes equal steps",
                ));
            }
        }
        if let Some(i) = &self.interpolate {
            if self.sweep.is_none() {
                return Err(task_error(
                    "[task.interpolate] interpolates S over [task.sweep]'s wavelengths: give a \
                     [task.sweep]",
                ));
            }
            if !(i.tolerance.is_finite() && i.tolerance >= crate::sweep::DEFAULT_TOLERANCE) {
                return Err(task_error(format!(
                    "[task.interpolate] tolerance must be at least 1e-13, got {}",
                    i.tolerance
                )));
            }
            if i.max_solves.is_some_and(|n| n < 5) {
                return Err(task_error(
                    "[task.interpolate] max_solves must be at least 5, the solves it starts from",
                ));
            }
        }
        Ok(())
    }

    /// What a 2D job refuses besides.
    fn validate_2d(&self) -> Result<()> {
        self.polarization()?;
        self.walls()?;
        if self.symmetric() {
            if self.walls.is_some() {
                return Err(task_error(
                    "a symmetric job's walls are its plane of symmetry and an electric wall: \
                     leave walls out",
                ));
            }
            if self.pml.as_ref().and_then(|p| p.sides.as_deref()) == Some("both") {
                return Err(task_error(
                    "a symmetric job's PML is at its upper wall only: leave sides out",
                ));
            }
            if self.section.iter().any(|s| s.center_um.is_some()) {
                return Err(task_error(
                    "a symmetric job's cores are centred on its plane of symmetry: leave \
                     center_um out",
                ));
            }
        }
        self.validate_sections(self.window(), self.symmetric(), true)
    }

    /// The sections' own settings, their cores inside a window `window` µm wide (the half's
    /// when `half`), their layers given one by one where `layers` (2D only).
    fn validate_sections(&self, window: f64, half: bool, layers: bool) -> Result<()> {
        if self.section.len() < 2 {
            return Err(task_error(
                "an eme job needs at least two [[task.section]]: its ports are the first and \
                 the last",
            ));
        }
        for (k, s) in self.section.iter().enumerate() {
            let at = format!("[[task.section]] #{}", k + 1);
            check_finite(&format!("{at} center_um"), s.center_um)?;
            if !(s.length_um.is_finite() && s.length_um >= 0.0) {
                return Err(task_error(format!(
                    "{at}: length_um must be at least 0, got {}",
                    s.length_um
                )));
            }
            let taper = matches!(s.core_um, Some(Thickness::Taper(_)));
            if taper && (k == 0 || k + 1 == self.section.len()) {
                return Err(task_error(format!(
                    "{at}: the first and last sections are the ports, straights, not tapers"
                )));
            }
            if taper && s.length_um <= 0.0 {
                return Err(task_error(format!("{at}: a taper needs a length above 0")));
            }
            if !taper && (s.profile.is_some() || s.steps.is_some()) {
                return Err(task_error(format!(
                    "{at}: profile and steps are a taper's, whose core_um is [from, to]"
                )));
            }
            if s.steps == Some(0) {
                return Err(task_error(format!("{at}: steps must be at least 1")));
            }
            Profile::named(s.profile.as_deref())?;
            match (&s.core_um, &s.layers) {
                (Some(_), Some(_)) | (None, None) => {
                    return Err(task_error(format!(
                        "{at} needs either core_um or layers, not both"
                    )));
                }
                (Some(t), None) => {
                    let (a, b) = match *t {
                        Thickness::Straight(a) => (a, a),
                        Thickness::Taper([a, b]) => (a, b),
                    };
                    let c = s.center_um.unwrap_or(0.0);
                    for t in [a, b] {
                        if !(t.is_finite() && t > 0.0) {
                            return Err(task_error(format!(
                                "{at}: a core's thickness must be positive, got {t} um"
                            )));
                        }
                        let share = if half { 1.0 } else { 0.5 };
                        if c.abs() + 0.5 * t >= share * window {
                            return Err(task_error(format!(
                                "{at}: the core ({t} um about {c} um) must lie inside the window \
                                 ({window} um wide)"
                            )));
                        }
                    }
                    if s.core.is_none() && self.core.is_none() {
                        return Err(task_error(format!(
                            "{at}: a core needs a medium: core here or in [task]"
                        )));
                    }
                }
                (None, Some(_)) if !layers => {
                    return Err(task_error(format!(
                        "{at}: a 3D section is a strip, its width core_um: layers are a 2D \
                         job's"
                    )));
                }
                (None, Some(layers)) => {
                    if s.center_um.is_some() || s.core.is_some() {
                        return Err(task_error(format!(
                            "{at}: center_um and core are a core's, not layers'"
                        )));
                    }
                    let total: f64 = layers.iter().map(|l| l.thickness_um).sum();
                    if layers
                        .iter()
                        .any(|l| !(l.thickness_um.is_finite() && l.thickness_um > 0.0))
                        || (total - window).abs() > 1e-9 * window
                    {
                        return Err(task_error(format!(
                            "{at}: the layers' thicknesses must be positive and fill the window \
                             ({window} um), got {total} um"
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// The sections resolved at `wavelength` (µm): every medium's index there.
    fn sections(&self, wavelength: f64) -> Result<Vec<Resolved>> {
        let lam = Wavelength::um(wavelength)?;
        let index = |v: &toml::Value, at: &str| -> Result<c64> {
            let m: Material = crate::device::spec::task_medium(v, at)
                .map_err(|e| task_error(format!("{at}: {e}")))?;
            m.refractive_index(lam)
                .map_err(|e| task_error(format!("{at}: {e}")))
        };
        let cladding = index(self.cladding()?, "cladding")?;
        self.section
            .iter()
            .enumerate()
            .map(|(k, s)| {
                let at = format!("[[task.section]] #{}", k + 1);
                let shape = match (&s.core_um, &s.layers) {
                    (_, Some(layers)) => Shape::Layers(
                        layers
                            .iter()
                            .map(|l| Ok((index(&l.medium, &at)?, l.thickness_um)))
                            .collect::<Result<_>>()?,
                    ),
                    (Some(t), None) => {
                        let core = match (&s.core, &self.core) {
                            (Some(c), _) | (None, Some(c)) => index(c, &format!("{at} core"))?,
                            (None, None) => return Err(task_error(format!("{at}: no core"))),
                        };
                        let (from, to) = match *t {
                            Thickness::Straight(a) => (a, a),
                            Thickness::Taper([a, b]) => (a, b),
                        };
                        Shape::Core {
                            core,
                            cladding,
                            center: s.center_um.unwrap_or(0.0),
                            half: self.symmetric(),
                            from,
                            to,
                            profile: Profile::named(s.profile.as_deref())?,
                            steps: s.steps.unwrap_or(10),
                        }
                    }
                    (None, None) => return Err(task_error(format!("{at}: no core or layers"))),
                };
                Ok(Resolved {
                    name: s
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("section {}", k + 1)),
                    length: s.length_um,
                    shape,
                })
            })
            .collect()
    }
}

/// The cross-section of a core `t` µm thick about `center` (from the window's middle) in a
/// window `w` µm wide; or, `half`, the upper half of one centred, against the lower wall.
fn core_slice(core: c64, cladding: c64, center: f64, t: f64, w: f64, half: bool) -> Result<Slice> {
    if half {
        return Slice::new(&[
            (core, Length::um(0.5 * t)),
            (cladding, Length::um(w - 0.5 * t)),
        ]);
    }
    if center == 0.0 {
        return Slice::slab(core, Length::um(t), cladding, Length::um(w));
    }
    let below = 0.5 * w + center - 0.5 * t;
    let above = 0.5 * w - center - 0.5 * t;
    Slice::new(&[
        (cladding, Length::um(below)),
        (core, Length::um(t)),
        (cladding, Length::um(above)),
    ])
}

impl Resolved {
    fn is_taper(&self) -> bool {
        matches!(self.shape, Shape::Core { from, to, .. } if from != to)
    }

    /// Its cross-section a share `u` along it.
    fn slice_at(&self, u: f64, w: f64) -> Result<Slice> {
        match &self.shape {
            Shape::Layers(layers) => Slice::new(
                &layers
                    .iter()
                    .map(|&(n, d)| (n, Length::um(d)))
                    .collect::<Vec<_>>(),
            ),
            &Shape::Core {
                core,
                cladding,
                center,
                half,
                from,
                to,
                profile,
                ..
            } => core_slice(core, cladding, center, profile.at(from, to, u), w, half),
        }
    }

    fn steps(&self) -> usize {
        match self.shape {
            Shape::Core { steps, .. } if self.is_taper() => steps,
            _ => 1,
        }
    }
}

/// A quantity `[task.convergence]` drives: its name and how it is read off S.
type Quantity = (&'static str, fn(&Scattering) -> f64);

/// What `[task.convergence]` drives.
fn quantity(name: Option<&str>) -> Result<Quantity> {
    match name.unwrap_or("transmission") {
        "transmission" => Ok(("|S21|^2", |s| s.s21[0][0].norm_sqr())),
        "reflection" => Ok(("|S11|^2", |s| s.s11[0][0].norm_sqr())),
        other => Err(task_error(format!(
            "[task.convergence] quantity is \"transmission\" or \"reflection\", got \"{other}\""
        ))),
    }
}

/// A solve's S-matrix, the adaptive placement's estimated error if it placed the steps, and
/// the tapers' steps.
type Solved = (Scattering, Option<f64>, usize);

/// One solve: the S-matrix with `modes` modes and each taper's steps times `scale`.
fn solve(
    task: &EmeTask,
    resolved: &[Resolved],
    wavelength: Wavelength,
    modes: usize,
    scale: usize,
) -> Result<Solved> {
    let eme = task.eme(wavelength, modes)?;
    let w = task.window();
    if let Some(a) = &task.adaptive {
        let out = eme.adaptive(&pieces(resolved, w, scale)?, a.tolerance)?;
        let steps = out.edges.iter().map(|e| e.len() - 1).sum();
        return Ok((out.scattering, Some(out.error), steps));
    }
    let (sections, steps) = sections_of(resolved, w, scale)?;
    let s = match period_range(task, resolved, scale) {
        Some((range, count)) => eme.grating(&sections, range, count)?,
        None => eme.s_matrix(&sections)?,
    };
    Ok((s, None, steps))
}

/// The sections of `resolved` in a window `w` µm wide, each taper's equal steps times `scale`,
/// and the tapers' steps; one period of a `[task.period]`.
fn sections_of(resolved: &[Resolved], w: f64, scale: usize) -> Result<(Vec<Section>, usize)> {
    let mut sections = Vec::new();
    let mut steps = 0;
    for r in resolved {
        if r.is_taper() {
            let n = r.steps() * scale;
            steps += n;
            let l = r.length;
            sections.extend(Section::sampled(Length::um(l), n, |z| {
                r.slice_at(z.to_um() / l, w)
            })?);
        } else {
            sections.push(Section::new(r.slice_at(0.0, w)?, Length::um(r.length)));
        }
    }
    Ok((sections, steps))
}

/// A `[task.period]`'s sections among those [`sections_of`] makes (each taper's steps times
/// `scale`), and its count.
fn period_range(
    task: &EmeTask,
    resolved: &[Resolved],
    scale: usize,
) -> Option<(std::ops::Range<usize>, usize)> {
    let p = task.period?;
    let [a, b] = p.sections;
    let made = |r: &Resolved| if r.is_taper() { r.steps() * scale } else { 1 };
    let start: usize = resolved.iter().take(a - 1).map(made).sum();
    let end: usize = resolved.iter().take(b).map(made).sum();
    Some((start..end, p.count))
}

/// The Bloch modes of a `[task.period]`'s period at `wavelength` µm with `modes` modes and its
/// tapers' steps times `scale`, as a sweep point over the wavelength: the least decaying first,
/// at most 4, each K/k₀ in the reduced zone (its real part at most λ/2Λ) with Im K/k₀ its decay.
fn bloch_event(
    task: &EmeTask,
    resolved: &[Resolved],
    wavelength: f64,
    (modes, scale): (usize, usize),
) -> Result<Option<Event>> {
    let Some((range, _)) = period_range(task, resolved, scale) else {
        return Ok(None);
    };
    let (sections, _) = sections_of(resolved, task.window(), scale)?;
    let eme = task.eme(Wavelength::um(wavelength)?, modes)?;
    let bloch = eme.bloch(&sections[range])?;
    let te = match task.polarization()? {
        Polarization::Te => 1.0,
        Polarization::Tm => 0.0,
    };
    Ok(Some(bloch_point(&bloch, wavelength, Some(te))))
}

/// A period's Bloch modes as a sweep point over the wavelength (see [`bloch_event`]), with
/// their TE fraction `te` where it is one for all (a 2D job's: 1 for TE, 0 for TM), none for a
/// 3D job's hybrid modes.
pub(super) fn bloch_point(bloch: &crate::eme::Bloch, wavelength: f64, te: Option<f64>) -> Event {
    let indices: Vec<[f64; 2]> = bloch
        .effective_indices
        .iter()
        .filter(|n| n.re.is_finite() && n.im.is_finite())
        .take(4)
        .map(|n| [n.re, n.im])
        .collect();
    Event::SweepPoint {
        parameter: "wavelength".into(),
        value: wavelength,
        wavelength_um: wavelength,
        te_fractions: te.map_or_else(Vec::new, |t| vec![t; indices.len()]),
        effective_indices: indices,
    }
}

/// The structure as [`Eme::adaptive`] takes it: its tapers from each one's steps times `scale`.
fn pieces(resolved: &[Resolved], w: f64, scale: usize) -> Result<Vec<Piece<'static>>> {
    resolved
        .iter()
        .map(|r| {
            if r.is_taper() {
                let r2 = r.clone();
                let l = r.length;
                Ok(Piece::Taper(Taper::new(
                    Length::um(l),
                    r.steps() * scale,
                    move |z| r2.slice_at(z.to_um() / l, w),
                )?))
            } else {
                Ok(Piece::Section(Section::new(
                    r.slice_at(0.0, w)?,
                    Length::um(r.length),
                )))
            }
        })
        .collect()
}

/// The S-matrix at `wavelength` µm with `modes` modes and the tapers' steps fixed: each
/// taper's `edges` (µm from its start; each step the cross-section at its middle, as
/// [`Eme::adaptive`] places them), or without them its steps times `scale`, equal.
fn fixed(
    task: &EmeTask,
    wavelength: f64,
    (modes, scale): (usize, usize),
    edges: Option<&[Vec<f64>]>,
) -> Result<Scattering> {
    let resolved = task.sections(wavelength)?;
    let lam = Wavelength::um(wavelength)?;
    let Some(edges) = edges else {
        let plain = EmeTask {
            adaptive: None,
            ..task.clone()
        };
        return Ok(solve(&plain, &resolved, lam, modes, scale)?.0);
    };
    let w = task.window();
    let mut sections = Vec::new();
    let mut taper = 0;
    for r in &resolved {
        if r.is_taper() {
            for step in edges[taper].windows(2) {
                let (a, b) = (step[0], step[1]);
                sections.push(Section::new(
                    r.slice_at(0.5 * (a + b) / r.length, w)?,
                    Length::um(b - a),
                ));
            }
            taper += 1;
        } else {
            sections.push(Section::new(r.slice_at(0.0, w)?, Length::um(r.length)));
        }
    }
    task.eme(lam, modes)?.s_matrix(&sections)
}

/// A wavelength's solve, its modes (and a task's equal steps) driven by `[task.convergence]`
/// when it has one, each study recorded: where it ended, (modes, the steps' scale), and the
/// solve there.
fn solve_at(
    task: &EmeTask,
    resolved: &[Resolved],
    wl: f64,
    run: &mut Run,
) -> Result<((usize, usize), Solved)> {
    let lam = Wavelength::um(wl)?;
    let Some(c) = &task.convergence else {
        return Ok(((task.modes, 1), solve(task, resolved, lam, task.modes, 1)?));
    };
    let tapers = resolved.iter().filter(|r| r.is_taper()).count();
    let base_steps: usize = resolved
        .iter()
        .filter(|r| r.is_taper())
        .map(Resolved::steps)
        .sum();
    let (name, read) = quantity(c.quantity.as_deref())?;
    let label = format!("{name} of the fundamental modes at {wl} um");
    let driver = ToTolerance::new(&label, Tolerance::Relative(c.tolerance))?;
    // every solve, by its modes and its steps' scale
    let mut solved: Vec<((usize, usize), Solved)> = Vec::new();
    let (studies, at): (Vec<Refined>, (usize, usize)) = if tapers > 0 && task.adaptive.is_none() {
        let r = driver.eme(task.modes, base_steps, |m, n| {
            // n is the base steps doubled: each taper's own doubled alike
            let s = solve(task, resolved, lam, m, n / base_steps)?;
            let v = read(&s.0);
            solved.push(((m, n / base_steps), s));
            Ok(v)
        })?;
        let at = (r.modes, r.steps / base_steps);
        (vec![r.mode_study, r.step_study, r.together], at)
    } else {
        let r = driver.counts(task.modes, |m| {
            let s = solve(task, resolved, lam, m, 1)?;
            let v = read(&s.0);
            solved.push(((m, 1), s));
            Ok(v)
        })?;
        let m = (1.0 / r.grids[r.grids.len() - 1]).round() as usize;
        (vec![r], (m, 1))
    };
    for r in &studies {
        run.record(&Event::GridConvergence {
            label: r.quantity.clone(),
            wavelength_um: wl,
            check: check_of(r),
        })?;
    }
    let s = match solved.into_iter().find(|(k, _)| *k == at) {
        Some((_, s)) => s,
        None => solve(task, resolved, lam, at.0, at.1)?,
    };
    Ok((at, s))
}

/// S over `[task.sweep]`'s wavelengths from as few solves as `[task.interpolate]`'s tolerance
/// needs ([`Spectrum::run`]), `solve(λ)` the same discretisation at every wavelength and
/// `ports(λ, S)` the guided modes at each end there: the entries between the modes guided at
/// both ends of the range, each sweep point's S-parameters from the interpolant, and its
/// estimated error with the wavelengths solved.
pub(super) fn interpolated(
    task: &EmeTask,
    run: &mut Run,
    stop: &Stop,
    mut solve: impl FnMut(f64) -> Result<Scattering>,
    ports: impl Fn(f64, &Scattering) -> Result<(usize, usize)>,
    name: impl Fn(usize) -> String,
) -> Result<()> {
    let (Some(spec), Some(sweep)) = (&task.interpolate, &task.sweep) else {
        return Err(task_error("[task.interpolate] needs a [task.sweep]"));
    };
    if stop.reason().is_some() {
        return Ok(());
    }
    // the ports' guided modes at both ends of the range, those solves kept for the sweep's
    let mut kept: Vec<(f64, Scattering)> = Vec::new();
    let mut counts = (usize::MAX, usize::MAX);
    for wl in [sweep.from, sweep.to] {
        let s = solve(wl)?;
        let (a, b) = ports(wl, &s)?;
        counts = (counts.0.min(a), counts.1.min(b));
        kept.push((wl, s));
    }
    let options = Options::new(spec.tolerance).with_max_samples(spec.max_solves.unwrap_or(200));
    let spectrum = Spectrum::run(
        |l| {
            let l = l.to_um();
            match kept.iter().position(|k| k.0 == l) {
                Some(i) => Ok(kept.swap_remove(i).1),
                None => solve(l),
            }
        },
        Wavelength::um(sweep.from)?,
        Wavelength::um(sweep.to)?,
        counts,
        &options,
    )?;
    for k in 0..sweep.points {
        if stop.reason().is_some() {
            return Ok(());
        }
        let wl = sweep.from + (sweep.to - sweep.from) * k as f64 / (sweep.points - 1) as f64;
        let lam = Wavelength::um(wl)?;
        run.record(&matrix_event(
            &spectrum.at(lam),
            &spectrum.effective_indices(lam),
            wl,
            counts,
            &name,
        ))?;
    }
    let solved: Vec<String> = spectrum
        .wavelengths()
        .iter()
        .map(|l| format!("{l:.5}"))
        .collect();
    run.record(&Event::SolveError {
        point: None,
        value: task.wavelength_um,
        measure: format!(
            "S over {} to {} um interpolated from {} solves (set-valued AAA, one set of \
             wavelengths for every entry), {} its tolerance {:e}: the estimated error of its \
             worst entry; solved at {} um",
            sweep.from,
            sweep.to,
            spectrum.solves(),
            if spectrum.converged() {
                "within"
            } else {
                "not within"
            },
            spec.tolerance,
            solved.join(", ")
        ),
        error: spectrum.error(),
    })?;
    Ok(())
}

/// A study's record as a run's grid check.
fn check_of(r: &Refined) -> Convergence {
    Convergence {
        quantity: r.quantity.clone(),
        value: r.value,
        step_um: r.grids.last().copied().unwrap_or(f64::NAN),
        steps_um: r.grids.clone(),
        values: r.values.clone(),
        tolerance: Some(r.tolerance),
        error: r.error,
        order: r.observed_order,
        limit: r.order.map(|_| r.value),
        verdict: if r.reached {
            Verdict::Converged
        } else {
            Verdict::NotConverged
        },
        reason: r.reason.clone(),
    }
}

/// The guided modes of a port: those with a real index above its outer layers' (at least the
/// first mode), up to 4; of a half, above its upper layer's (the lower is on the plane of
/// symmetry).
fn guided(indices: &[c64], slice: &Slice, half: bool) -> usize {
    let layers = slice.layers();
    let upper = layers[layers.len() - 1].0.re;
    let outer = if half {
        upper
    } else {
        layers[0].0.re.max(upper)
    };
    indices
        .iter()
        .take(4)
        .take_while(|n| n.re > outer && n.im.abs() <= 1e-9 * n.re)
        .count()
        .max(1)
}

/// The S-parameters event of `s` between the ports' guided modes.
fn s_event(
    s: &Scattering,
    wavelength: f64,
    first: &Slice,
    last: &Slice,
    polarization: Polarization,
    half: bool,
) -> Event {
    let (n1, n2) = (guided(&s.first, first, half), guided(&s.last, last, half));
    let pol = match polarization {
        Polarization::Te => "TE",
        Polarization::Tm => "TM",
    };
    // a half's modes are the even ones: TE0, TE2, ...
    ports_event(s, wavelength, (n1, n2), |m| {
        format!("{pol}{}", if half { 2 * m } else { m })
    })
}

/// The S-parameters event of `s` between the first `n1` modes at its first end and `n2` at its
/// last, the m-th named `name(m)` after its port's number.
fn ports_event(
    s: &Scattering,
    wavelength: f64,
    (n1, n2): (usize, usize),
    name: impl Fn(usize) -> String,
) -> Event {
    let n = n1 + n2;
    let entries = spectrum_entries(s, (n1, n2));
    let matrix: Vec<Vec<c64>> = entries.chunks(n).map(<[c64]>::to_vec).collect();
    let indices: Vec<c64> = s
        .first
        .iter()
        .take(n1)
        .chain(s.last.iter().take(n2))
        .copied()
        .collect();
    matrix_event(&matrix, &indices, wavelength, (n1, n2), &name)
}

/// The S-parameters event of the S-matrix `s` between `n1` modes at its first end and `n2` at
/// its last (rows and columns port 1's modes, then port 2's), their effective `indices` in the
/// same order, the m-th named `name(m)` after its port's number.
fn matrix_event(
    s: &[Vec<c64>],
    indices: &[c64],
    wavelength: f64,
    (n1, n2): (usize, usize),
    name: &impl Fn(usize) -> String,
) -> Event {
    let mut ports = Vec::new();
    for (p, n) in [n1, n2].into_iter().enumerate() {
        for m in 0..n {
            ports.push(format!("{} {}", p + 1, name(m)));
        }
    }
    Event::SParameters {
        wavelength_um: wavelength,
        ports,
        effective_indices: indices.iter().map(|n| n.re).collect(),
        s: s.iter()
            .map(|row| row.iter().map(|v| [v.re, v.im]).collect())
            .collect(),
    }
}

/// The solver's detail of a `[task.period]`.
fn period_details(task: &EmeTask) -> Option<[String; 2]> {
    let p = task.period?;
    let [a, b] = p.sections;
    let length: f64 = task.section[a - 1..b].iter().map(|s| s.length_um).sum();
    Some([
        "period".into(),
        format!(
            "sections {a} to {b} ({length} um) {} times, by repeated squaring of its S-matrix;              its Bloch modes recorded as a sweep point at each wavelength solved",
            p.count
        ),
    ])
}

/// The order the sections come in along z: a `[task.period]`'s repeated in place.
fn order(task: &EmeTask) -> Vec<usize> {
    let n = task.section.len();
    match task.period {
        None => (0..n).collect(),
        Some(p) => {
            let [a, b] = p.sections;
            let mut out: Vec<usize> = (0..a - 1).collect();
            for _ in 0..p.count {
                out.extend(a - 1..b);
            }
            out.extend(b..n);
            out
        }
    }
}

/// The scene: the cores (and layers other than the cladding) seen from above, z along x and the
/// window across y, on one layer 1 µm thick.
fn eme_scene(task: &EmeTask) -> Result<Event> {
    scene_across(task, task.window(), task.symmetric())
}

/// [`eme_scene`] for a window `window` µm across, the half's when `half` (drawn whole).
fn scene_across(task: &EmeTask, window: f64, half: bool) -> Result<Event> {
    let resolved = task.sections(task.wavelength_um)?;
    let lam = Wavelength::um(task.wavelength_um)?;
    let cladding = crate::device::spec::task_medium(task.cladding()?, "cladding")?;
    let eps = |n: c64| (n * n).re;
    let clad_n = cladding.refractive_index(lam)?;
    let medium = |name: &str, n: c64| SceneMedium {
        material: name.to_owned(),
        eps: eps(n),
    };
    // a symmetric job's half drawn whole, mirrored about its plane of symmetry
    let w = if half { 2.0 * window } else { window };
    let mut shapes = Vec::new();
    let mut core_n = clad_n;
    let mut z = 0.0;
    for r in order(task).into_iter().map(|k| &resolved[k]) {
        let l = r.length;
        match &r.shape {
            Shape::Layers(layers) => {
                let mut x = if half { 0.0 } else { -0.5 * w };
                for &(n, d) in layers {
                    if n != clad_n && l > 0.0 {
                        let rect = |a: f64, b: f64| SceneShape {
                            layer: "core".into(),
                            outline: vec![[z, a], [z + l, a], [z + l, b], [z, b]],
                        };
                        shapes.push(rect(x, x + d));
                        if half {
                            shapes.push(rect(-x - d, -x));
                        }
                        core_n = n;
                    }
                    x += d;
                }
            }
            &Shape::Core {
                core,
                center,
                from,
                to,
                profile,
                ..
            } => {
                core_n = core;
                if l > 0.0 {
                    // the outline sampled at 32 points along a taper, 1 along a straight
                    let k = if from == to { 1 } else { 32 };
                    let edge = |sign: f64| -> Vec<[f64; 2]> {
                        (0..=k)
                            .map(|i| {
                                let u = i as f64 / k as f64;
                                [z + u * l, center + sign * 0.5 * profile.at(from, to, u)]
                            })
                            .collect()
                    };
                    let mut outline = edge(-1.0);
                    outline.extend(edge(1.0).into_iter().rev());
                    shapes.push(SceneShape {
                        layer: "core".into(),
                        outline,
                    });
                }
            }
        }
        z += l;
    }
    Ok(Event::Scene {
        x_um: [0.0, z.max(1e-3)],
        y_um: [-0.5 * w, 0.5 * w],
        z_um: [0.0, 1.0],
        wavelength_um: task.wavelength_um,
        substrate: medium(cladding.name(), clad_n),
        cladding: medium(cladding.name(), clad_n),
        layers: vec![SceneLayer {
            name: "core".into(),
            z_um: [0.0, 1.0],
            material: medium("core", core_n),
            background: medium(cladding.name(), clad_n),
        }],
        shapes,
    })
}

/// The `"eme"` kind: a 2D structure of sections by eigenmode expansion.
pub(super) struct EmeKind;

impl TaskKind for EmeKind {
    fn name(&self) -> &'static str {
        "eme"
    }

    fn schema(&self, g: &mut SchemaGenerator) -> serde_json::Value {
        super::schema::task::<EmeTask>(g)
    }

    fn check(&self, job: &Job) -> Result<()> {
        let task: EmeTask = super::params::task(job)?;
        task.validate(job.device().is_some())?;
        if task.plane.is_some() {
            return three::check(job, &task);
        }
        for w in task.wavelengths() {
            task.sections(w)?;
            task.eme(Wavelength::um(w)?, task.modes)?;
        }
        Ok(())
    }

    fn preview(&self, job: &Job) -> Result<Event> {
        if job.device().is_some() {
            // a device's design drawn as it is edited, as its other tasks draw it
            return super::device_scene(job);
        }
        super::check(job)?;
        let task: EmeTask = super::params::task(job)?;
        match &task.plane {
            Some(plane) => three::scene(&task, plane),
            None => eme_scene(&task),
        }
    }

    fn execute(&self, job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
        if job.device().is_some() {
            return super::device::computed(job, run, stop, |job, run, stop, scene| {
                run.record(&scene)?;
                eme(job, run, stop)
            });
        }
        solver_run(job, run, stop, self.name(), eme)
    }

    fn farms(&self, _: &Job) -> bool {
        false
    }

    fn exports(&self, _: &Job) -> (&'static str, &'static [&'static str]) {
        ("an eme job", &[])
    }
}

fn eme(job: &Job, run: &mut Run, stop: &Stop) -> Result<()> {
    let task: EmeTask = super::params::task(job)?;
    task.validate(job.device().is_some())?;
    if let Some(plane) = &task.plane {
        return three::run(job, &task, plane, run, stop);
    }
    run.record(&eme_scene(&task)?)?;
    let wavelengths = task.wavelengths();
    let first = task.sections(wavelengths[0])?;
    let tapers = first.iter().filter(|r| r.is_taper()).count();
    let base_steps: usize = first
        .iter()
        .filter(|r| r.is_taper())
        .map(Resolved::steps)
        .sum();
    let mut details = vec![
        [
            "polarization".to_owned(),
            task.polarization.clone().unwrap_or_else(|| "te".into()),
        ],
        ["window".to_owned(), format!("{} um", task.window())],
        [
            "sections".to_owned(),
            first
                .iter()
                .map(|r| format!("{} ({} um)", r.name, r.length))
                .collect::<Vec<_>>()
                .join(", "),
        ],
    ];
    if let Some(p) = &task.pml {
        details.push([
            "PML".into(),
            format!(
                "{} um of strength {} at the {} wall(s)",
                p.thickness_um,
                p.strength,
                p.sides.as_deref().unwrap_or("both")
            ),
        ]);
    }
    if tapers > 0 {
        details.push([
            "taper steps".into(),
            match &task.adaptive {
                Some(a) => format!(
                    "placed where the response changes, from {base_steps}, to {:e} on S's fundamental entries",
                    a.tolerance
                ),
                None => format!("{base_steps}, equal"),
            },
        ]);
    }
    if let Some(d) = period_details(&task) {
        details.push(d);
    }
    if let Some(i) = &task.interpolate {
        details.push([
            "interpolation".into(),
            format!(
                "S over the sweep from as few solves as {:e} needs, the modes and steps settled \
                 at {} um",
                i.tolerance, task.wavelength_um
            ),
        ]);
    }
    run.record(&Event::Solver {
        module: "eme".into(),
        cells: [task.modes, base_steps.max(1)],
        step_um: first
            .iter()
            .filter(|r| r.is_taper())
            .map(|r| r.length / r.steps() as f64)
            .fold(0.0, f64::max),
        unknowns: task.modes * (task.section.len() + base_steps),
        details,
    })?;
    if let Some(s) = &task.sweep {
        run.record(&Event::Sweep {
            parameter: "wavelength".into(),
            from: s.from,
            to: s.to,
            points: s.points,
        })?;
    }
    let w = task.window();
    let ends_at = |wl: f64| -> Result<(Slice, Slice)> {
        let resolved = task.sections(wl)?;
        Ok((
            resolved[0].slice_at(0.0, w)?,
            resolved[resolved.len() - 1].slice_at(0.0, w)?,
        ))
    };
    let pol = match task.polarization()? {
        Polarization::Te => "TE",
        Polarization::Tm => "TM",
    };
    if task.interpolate.is_some() {
        // the discretisation settled once, at the job's wavelength, and kept at every solve
        let resolved = task.sections(task.wavelength_um)?;
        let (at, (_, error, steps)) = solve_at(&task, &resolved, task.wavelength_um, run)?;
        let edges = match &task.adaptive {
            Some(a) => {
                if let Some(e) = error {
                    run.record(&Event::SolveError {
                        point: None,
                        value: task.wavelength_um,
                        measure: format!(
                            "the tapers' {steps} adaptive steps at {} um, kept at every \
                             wavelength: the estimated error of S's fundamental entries",
                            task.wavelength_um
                        ),
                        error: e,
                    })?;
                }
                let eme = task.eme(Wavelength::um(task.wavelength_um)?, at.0)?;
                Some(
                    eme.adaptive(&pieces(&resolved, w, at.1)?, a.tolerance)?
                        .edges,
                )
            }
            None => None,
        };
        if let Some(e) = bloch_event(&task, &resolved, task.wavelength_um, at)? {
            run.record(&e)?;
        }
        let half = task.symmetric();
        return interpolated(
            &task,
            run,
            stop,
            |wl| fixed(&task, wl, at, edges.as_deref()),
            |wl, s| {
                let (first, last) = ends_at(wl)?;
                Ok((guided(&s.first, &first, half), guided(&s.last, &last, half)))
            },
            |m| format!("{pol}{}", if half { 2 * m } else { m }),
        );
    }
    for (point, &wl) in wavelengths.iter().enumerate() {
        if stop.reason().is_some() {
            return Ok(());
        }
        let resolved = task.sections(wl)?;
        let ends = ends_at(wl)?;
        let (at, (s, error, steps)) = solve_at(&task, &resolved, wl, run)?;
        if let Some(e) = error {
            run.record(&Event::SolveError {
                point: task.sweep.as_ref().map(|_| point),
                value: wl,
                measure: format!(
                    "the tapers' {steps} adaptive steps: the estimated error of S's fundamental \
                     entries"
                ),
                error: e,
            })?;
        }
        run.record(&s_event(
            &s,
            wl,
            &ends.0,
            &ends.1,
            task.polarization()?,
            task.symmetric(),
        ))?;
        if let Some(e) = bloch_event(&task, &resolved, wl, at)? {
            run.record(&e)?;
        }
    }
    Ok(())
}
