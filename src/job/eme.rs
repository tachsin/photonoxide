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
//! Bloch modes ([`crate::eme::Bloch`]) at each wavelength it solves as a sweep point. A section
//! with `radius_um` and `angle_deg` is a bend by the conformal map ([`crate::eme::Slice::bent`]),
//! drawn turning in the scene. `[task.plane]` makes the sections 3D ([`three`]): strips on a
//! plane, a `[device]`'s geometry cut along x, or the task's own drawing (a `stack`, its
//! `layer`, its shapes and `x_um`) cut the same way.
//!
//! The run records its scene (the cores seen from above, z along x), the solver's settings,
//! the S-parameters between the guided modes at the two ends (at each wavelength), and each
//! convergence study.

use num_complex::Complex64 as c64;
use schemars::{JsonSchema, SchemaGenerator};
use serde::{Deserialize, Serialize};

use super::execute::solver_run;
use super::kind::TaskKind;
use super::scene::{SceneLayer, SceneMedium, SceneShape};
use super::{Event, check_finite, check_sweep, task_error};
use crate::Result;
use crate::convergence::{Convergence, Refined, ToTolerance, Tolerance, Verdict};
use crate::eme::{
    Beam, Eme, Piece, Scattering, Section, Slice, Spectrum, Taper, Wall,
    entries as spectrum_entries, power,
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

/// A `[[task.section]]`: a straight, a taper or a bend, in order along z.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SectionSpec {
    /// Its name, for the record: `"section k"` by default.
    name: Option<String>,
    /// Its length along z, µm: 0 for a port's reference plane. A bend's is its arc, from
    /// `radius_um` and `angle_deg`.
    length_um: Option<f64>,
    /// A bend's radius, µm, to the core's centre (to the window's middle for layers): positive
    /// turning towards the lower wall (the bend's centre beyond it), negative towards the upper.
    /// With `angle_deg` the section is a bend in the plane of the window, by the conformal map
    /// ([`crate::eme::Slice::bent`]).
    radius_um: Option<f64>,
    /// A bend's angle, degrees.
    angle_deg: Option<f64>,
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

impl SectionSpec {
    /// Whether it is a bend.
    fn is_bend(&self) -> bool {
        self.radius_um.is_some() || self.angle_deg.is_some()
    }

    /// Its length along z (for a bend along the arc at the window's middle, where its modes'
    /// propagation constants are taken), µm; NaN where it has none.
    fn length(&self) -> f64 {
        match (self.middle_radius(), self.angle_deg) {
            (Some(r), Some(a)) => r.abs() * a.to_radians(),
            _ => self.length_um.unwrap_or(f64::NAN),
        }
    }

    /// A bend's radius to the window's middle, µm: its core's centre `center_um` from it.
    fn middle_radius(&self) -> Option<f64> {
        self.radius_um.map(|r| r - self.center_um.unwrap_or(0.0))
    }
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

/// A `[task.beam]`: a Gaussian across the first section's plane ([`crate::eme::Beam`]).
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BeamSpec {
    /// Its waist, µm: the field falls to 1/e at the waist from its centre.
    waist_um: f64,
    /// Its centre, µm from the window's middle (0 by default).
    center_um: Option<f64>,
    /// The tilt of its phase fronts, degrees, toward +x (0 by default).
    angle_deg: Option<f64>,
    /// The index of the medium the tilt is in: the cladding's by default.
    index: Option<f64>,
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

#[derive(Clone, Deserialize, JsonSchema)]
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
    /// The sections in order along z (a `[device]`'s or a drawing's are cut from its geometry).
    #[serde(default)]
    section: Vec<SectionSpec>,
    /// The sections in 3D: each cross-section a plane of Yee's grid ([`three`]).
    plane: Option<three::PlaneSpec>,
    /// A drawing's stack, its shapes drawn on it and cut into 3D sections along x
    /// ([`crate::device::Cut`]): a named one, `soi_220`, or `soi` / `nitride` with `core_nm`
    /// and `bottom_oxide_um`; or a `[task.stack]` table of layers.
    stack: Option<super::StackField>,
    /// A named stack's core thickness, nm.
    core_nm: Option<f64>,
    /// A named stack's bottom oxide, µm.
    bottom_oxide_um: Option<f64>,
    /// A drawing's core layer: the plane spans its shapes across and its thickness up, with
    /// `[task.plane]`'s margins.
    layer: Option<String>,
    /// Where a drawing is cut, from and to along x, µm: its guides must run on past both ends,
    /// whose cross-sections are the ports'.
    x_um: Option<[f64; 2]>,
    /// A drawing's rectangles, as a structure job's.
    #[serde(default)]
    rect: Vec<super::RectSpec>,
    /// A drawing's circles.
    #[serde(default)]
    circle: Vec<super::CircleSpec>,
    /// A drawing's rings.
    #[serde(default)]
    ring: Vec<super::RingSpec>,
    /// A drawing's paths: straights, arcs and tapers of a waveguide.
    #[serde(default)]
    path: Vec<super::PathSpec>,
    /// A drawing's shapes defined by functions, masks, polygons and booleans, and a GDSII file's
    /// cell on one of its layers (`type = "gds"`): a layout's sections cut from its file.
    #[serde(default)]
    shape: Vec<super::shapes::ShapeSpec>,
    adaptive: Option<AdaptiveSpec>,
    /// A bend's thin layers' largest thickness in the mapped coordinate, nm (10 by default,
    /// [`crate::eme::Eme::with_bend_step`]): where `[task.convergence]` starts the bends' layers
    /// from, halving them until its quantity changes by less than its tolerance.
    bend_step_nm: Option<f64>,
    convergence: Option<ConvergenceSpec>,
    sweep: Option<WavelengthSweep>,
    /// S over the sweep's wavelengths interpolated from a few solves, with its error stated.
    interpolate: Option<InterpolateSpec>,
    /// Some sections repeated: a grating, solved by repeated squaring of one period's S-matrix,
    /// and its Bloch modes recorded.
    period: Option<PeriodSpec>,
    /// A Gaussian beam launched on the first section's modes (2D, between walls without a PML):
    /// the shares of its power reflected, transmitted and carried by each guided mode at the
    /// last end, recorded at each wavelength.
    beam: Option<BeamSpec>,
}

/// A section resolved at a wavelength: its name, length, how its cross-section is made, and a
/// bend's radius to the window's middle and angle (radians).
#[derive(Clone, Debug)]
struct Resolved {
    name: String,
    length: f64,
    shape: Shape,
    radius: Option<f64>,
    angle: f64,
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
        let mut eme = Eme::new(self.polarization()?, wavelength, modes)?.with_walls(lower, upper);
        if let Some(nm) = self.bend_step_nm {
            eme = eme.with_bend_step(Length::um(nm * 1e-3))?;
        }
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
        if let Some(nm) = self.bend_step_nm
            && !(nm.is_finite() && nm > 0.0)
        {
            return Err(task_error(format!(
                "bend_step_nm must be above 0, got {nm}"
            )));
        }
        if let Some(plane) = &self.plane {
            if self.beam.is_some() {
                return Err(task_error(
                    "[task.beam] is a 2D job's: a beam on a 3D section's plane isn't launched yet",
                ));
            }
            self.validate_common()?;
            return three::validate(self, plane, device);
        }
        if let Some(name) = self.drawing_field() {
            return Err(task_error(format!(
                "{name} is a drawing's, which is cut into 3D sections: give [task.plane]"
            )));
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

    /// The first of a drawing's fields the task gives, if any.
    fn drawing_field(&self) -> Option<&'static str> {
        [
            ("stack", self.stack.is_some()),
            ("core_nm", self.core_nm.is_some()),
            ("bottom_oxide_um", self.bottom_oxide_um.is_some()),
            ("layer", self.layer.is_some()),
            ("x_um", self.x_um.is_some()),
            ("[[task.rect]]", !self.rect.is_empty()),
            ("[[task.circle]]", !self.circle.is_empty()),
            ("[[task.ring]]", !self.ring.is_empty()),
            ("[[task.path]]", !self.path.is_empty()),
            ("[[task.shape]]", !self.shape.is_empty()),
        ]
        .into_iter()
        .find_map(|(name, given)| given.then_some(name))
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
                    "[task.period] repeats some of the [[task.section]]: a [device]'s or a \
                     drawing's sections are cut from its geometry",
                ));
            }
            if !(a >= 2 && a <= b && b < n) {
                return Err(task_error(format!(
                    "[task.period] sections are the first and last of one period, counted from 1, \
                     between the ports (sections 2 to {}), got [{a}, {b}]",
                    n.saturating_sub(1)
                )));
            }
            if p.count == 0 {
                return Err(task_error("[task.period] count must be at least 1"));
            }
            if self.adaptive.is_some() {
                return Err(task_error(
                    "[task.adaptive] places the steps of the whole structure: a job with a \
                     [task.period] takes equal steps",
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
        if let Some(b) = &self.beam {
            self.validate_beam(b)?;
        }
        self.validate_sections(self.window(), self.symmetric(), true)
    }

    /// A `[task.beam]`'s settings: a beam needs the whole window between walls (its power is
    /// the propagating modes' there) and the first section straight.
    fn validate_beam(&self, b: &BeamSpec) -> Result<()> {
        let at = "[task.beam]";
        if !(b.waist_um.is_finite() && b.waist_um > 0.0) {
            return Err(task_error(format!("{at}: waist_um must be positive")));
        }
        let half = 0.5 * self.window();
        let c = b.center_um.unwrap_or(0.0);
        if !(c.is_finite() && c.abs() < half) {
            return Err(task_error(format!(
                "{at}: center_um must be inside the window, within {half} um of its middle"
            )));
        }
        if !b.angle_deg.unwrap_or(0.0).is_finite() || b.angle_deg.unwrap_or(0.0).abs() >= 90.0 {
            return Err(task_error(format!("{at}: angle_deg must be within ±90")));
        }
        if b.index.is_some_and(|n| !(n.is_finite() && n > 0.0)) {
            return Err(task_error(format!("{at}: index must be positive")));
        }
        if self.pml.is_some() || self.symmetric() {
            return Err(task_error(format!(
                "{at}: a beam's shares of power are its propagating modes' between walls: leave \
                 out [task.pml] and symmetric"
            )));
        }
        if self.interpolate.is_some() {
            return Err(task_error(format!(
                "{at}: the beam's shares are solved at each wavelength: leave out \
                 [task.interpolate]"
            )));
        }
        if self.section.first().is_some_and(SectionSpec::is_bend) {
            return Err(task_error(format!(
                "{at}: the beam is launched on the first section, which must be straight"
            )));
        }
        Ok(())
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
            let taper = matches!(s.core_um, Some(Thickness::Taper(_)));
            if s.is_bend() {
                self.validate_bend(&at, s, k, window, half, layers, taper)?;
            } else {
                let length = s.length_um.ok_or_else(|| {
                    task_error(format!(
                        "{at} needs length_um (a bend radius_um and angle_deg instead)"
                    ))
                })?;
                if !(length.is_finite() && length >= 0.0) {
                    return Err(task_error(format!(
                        "{at}: length_um must be at least 0, got {length}"
                    )));
                }
            }
            let length = s.length();
            if taper && (k == 0 || k + 1 == self.section.len()) {
                return Err(task_error(format!(
                    "{at}: the first and last sections are the ports, straights, not tapers"
                )));
            }
            if taper && length <= 0.0 {
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

    /// A bend's own settings: `s` is section `k` of a window `window` µm wide (the half's when
    /// `half`), in 2D where `layers`.
    #[allow(clippy::too_many_arguments)]
    fn validate_bend(
        &self,
        at: &str,
        s: &SectionSpec,
        k: usize,
        window: f64,
        half: bool,
        layers: bool,
        taper: bool,
    ) -> Result<()> {
        let (Some(r), Some(a)) = (s.radius_um, s.angle_deg) else {
            return Err(task_error(format!(
                "{at}: a bend needs both radius_um and angle_deg"
            )));
        };
        if s.length_um.is_some() {
            return Err(task_error(format!(
                "{at}: a bend's length is its arc, radius_um times angle_deg: leave length_um out"
            )));
        }
        if !layers {
            return Err(task_error(format!(
                "{at}: a bend is a 2D job's: the conformal map is exact only in the plane of the \
                 bend"
            )));
        }
        if half {
            return Err(task_error(format!(
                "{at}: a bend has no plane of symmetry across it: a job with bends isn't symmetric"
            )));
        }
        if k == 0 || k + 1 == self.section.len() {
            return Err(task_error(format!(
                "{at}: the first and last sections are the ports, straights, not bends"
            )));
        }
        if taper {
            return Err(task_error(format!(
                "{at}: a bend's core is one thickness, not [from, to]"
            )));
        }
        if !(a.is_finite() && a > 0.0) {
            return Err(task_error(format!(
                "{at}: angle_deg must be above 0, got {a}"
            )));
        }
        let middle = s.middle_radius().unwrap_or(r);
        if !(r.is_finite() && middle.abs() > 0.5 * window) {
            return Err(task_error(format!(
                "{at}: radius_um must keep the window ({window} um wide) on one side of the \
                 bend's centre, got {r}"
            )));
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
                    length: s.length(),
                    shape,
                    radius: s.middle_radius().filter(|_| s.is_bend()),
                    angle: s.angle_deg.map_or(0.0, f64::to_radians),
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

/// A section of an `"eme"` job's structure as its run records it ([`Event::EmeSections`]):
/// what it is and where it lies along the structure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmeSection {
    /// Its name: the job's own (`"section 2"` where it gives none), or a cut's, `"cut 3"`.
    pub name: String,
    /// `"straight"`, `"taper"`, `"bend"`, or `"cut"` for a plane cut from a `[device]` or the
    /// job's own drawing.
    pub kind: String,
    /// Where it starts along the structure, µm: its first period's place for a section of
    /// `[task.period]`, the periods' whole length before the sections after them.
    pub start_um: f64,
    /// Its length, µm; a bend's along the arc at the window's middle.
    pub length_um: f64,
    /// The cross-sections it is solved in at the job's own settings: a taper's steps, 1
    /// otherwise (`[task.adaptive]` and `[task.convergence]` may place more).
    pub steps: usize,
    /// A bend's radius to the window's middle, µm.
    pub radius_um: Option<f64>,
    /// A bend's angle, degrees.
    pub angle_deg: Option<f64>,
    /// How many times it is repeated: `[task.period]`'s count for a section of the period, 1
    /// otherwise.
    pub repeats: usize,
}

/// The sections of a 2D job (or a 3D job's strips) as its run records them.
fn sections_event(task: &EmeTask, resolved: &[Resolved]) -> Event {
    let period = task
        .period
        .map(|p| (p.sections[0] - 1..p.sections[1], p.count));
    let mut z = 0.0;
    let mut sections = Vec::with_capacity(resolved.len());
    for (k, r) in resolved.iter().enumerate() {
        let repeats = match &period {
            Some((range, count)) if range.contains(&k) => *count,
            _ => 1,
        };
        sections.push(EmeSection {
            name: r.name.clone(),
            kind: if r.radius.is_some() {
                "bend"
            } else if r.is_taper() {
                "taper"
            } else {
                "straight"
            }
            .into(),
            start_um: z,
            length_um: r.length,
            steps: r.steps(),
            radius_um: r.radius,
            angle_deg: r.radius.map(|_| r.angle.to_degrees()),
            repeats,
        });
        z += r.length;
        // after a period's last section, the other periods
        if let Some((range, count)) = &period
            && k + 1 == range.end
        {
            let one: f64 = resolved[range.clone()].iter().map(|r| r.length).sum();
            z += one * (*count as f64 - 1.0);
        }
    }
    Event::EmeSections { sections }
}

impl Resolved {
    fn is_taper(&self) -> bool {
        matches!(self.shape, Shape::Core { from, to, .. } if from != to)
    }

    /// Its cross-section a share `u` along it, bent if it is a bend.
    fn slice_at(&self, u: f64, w: f64) -> Result<Slice> {
        let slice = self.straight_at(u, w)?;
        match self.radius {
            Some(r) => slice.bent(Length::um(r)),
            None => Ok(slice),
        }
    }

    /// Its cross-section a share `u` along it, straight.
    fn straight_at(&self, u: f64, w: f64) -> Result<Slice> {
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

/// One solve: the S-matrix with `modes` modes and each taper's steps times `scale`, each
/// bend's thin layers at most `bend_um` thick (the task's `bend_step_nm` when `None`).
fn solve(
    task: &EmeTask,
    resolved: &[Resolved],
    wavelength: Wavelength,
    modes: usize,
    scale: usize,
    bend_um: Option<f64>,
) -> Result<Solved> {
    let mut eme = task.eme(wavelength, modes)?;
    if let Some(h) = bend_um {
        eme = eme.with_bend_step(Length::um(h))?;
    }
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
        return Ok(solve(&plain, &resolved, lam, modes, scale, None)?.0);
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
/// when it has one, and with bends their thin layers after them, each study recorded: where it
/// ended, (modes, the steps' scale), the solve there, and the bends' layers' thickness the
/// study refined them to (µm; `None` when it didn't study them).
fn solve_at(
    task: &EmeTask,
    resolved: &[Resolved],
    wl: f64,
    run: &mut Run,
) -> Result<((usize, usize), Solved, Option<f64>)> {
    let lam = Wavelength::um(wl)?;
    let Some(c) = &task.convergence else {
        return Ok((
            (task.modes, 1),
            solve(task, resolved, lam, task.modes, 1, None)?,
            None,
        ));
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
            let s = solve(task, resolved, lam, m, n / base_steps, None)?;
            let v = read(&s.0);
            solved.push(((m, n / base_steps), s));
            Ok(v)
        })?;
        let at = (r.modes, r.steps / base_steps);
        (vec![r.mode_study, r.step_study, r.together], at)
    } else {
        let r = driver.counts(task.modes, |m| {
            let s = solve(task, resolved, lam, m, 1, None)?;
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
        None => solve(task, resolved, lam, at.0, at.1, None)?,
    };
    if !resolved.iter().any(|r| r.radius.is_some()) {
        return Ok((at, s, None));
    }
    // the bends' mapped profile has its own discretisation beside the modes': its thin layers
    // halved, at the modes and steps found, until the quantity changes by less than the
    // tolerance (a staircase, second order in the layers' thickness)
    let start = task.eme(lam, at.0)?.bend_step().to_um();
    let layers = ToTolerance::new(
        &format!("{name} of the fundamental modes at {wl} um, the bends' layers"),
        Tolerance::Relative(c.tolerance),
    )?;
    let mut by_step: Vec<(f64, Solved)> = vec![(start, s)];
    let r = layers.grids(start, |h| {
        let found = by_step.iter().position(|(k, _)| *k == h);
        let s = match found {
            Some(i) => by_step.swap_remove(i).1,
            None => solve(task, resolved, lam, at.0, at.1, Some(h))?,
        };
        let v = read(&s.0);
        by_step.push((h, s));
        Ok(v)
    })?;
    run.record(&Event::GridConvergence {
        label: r.quantity.clone(),
        wavelength_um: wl,
        check: check_of(&r),
    })?;
    let finest = r.grids[r.grids.len() - 1];
    let s = match by_step.into_iter().find(|(k, _)| *k == finest) {
        Some((_, s)) => s,
        None => solve(task, resolved, lam, at.0, at.1, Some(finest))?,
    };
    Ok((at, s, Some(finest)))
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
    let length: f64 = task.section[a - 1..b].iter().map(SectionSpec::length).sum();
    Some([
        "period".into(),
        format!(
            "sections {a} to {b} ({length} um) {} times, by repeated squaring of its S-matrix; \
             its Bloch modes recorded as a sweep point at each wavelength solved",
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
    // where each section starts and its heading (radians from x): a straight goes on along it, a
    // bend turns it about its centre
    let mut pen = Pen {
        at: [0.0, 0.0],
        heading: 0.0,
    };
    // the window's edges at each section's ends and along each bend, for the scene's extent
    let mut edges = vec![pen.place(0.0, -0.5 * w), pen.place(0.0, 0.5 * w)];
    for r in order(task).into_iter().map(|k| &resolved[k]) {
        let l = r.length;
        // a point of the section, a share u along it and y across it from the window's middle;
        // and how many shares a straight edge takes (a bend's 32 a quarter turn)
        let bend = r.radius.map(|radius| (radius, r.angle));
        let point = |u: f64, y: f64| match bend {
            Some((radius, angle)) => pen.arc(radius, u * angle, y),
            None => pen.place(u * l, y),
        };
        let arc_points = bend.map_or(1, |(_, a)| {
            (32.0 * a / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize
        });
        // a band from a to b across, along the whole section
        let band = |a: f64, b: f64, k: usize, width: &dyn Fn(f64) -> (f64, f64)| -> SceneShape {
            let edge = |lower: bool| -> Vec<[f64; 2]> {
                (0..=k)
                    .map(|i| {
                        let u = i as f64 / k as f64;
                        let (lo, hi) = width(u);
                        point(u, if lower { a + lo } else { b + hi })
                    })
                    .collect()
            };
            let mut outline = edge(true);
            outline.extend(edge(false).into_iter().rev());
            SceneShape {
                layer: "core".into(),
                outline,
            }
        };
        match &r.shape {
            Shape::Layers(layers) => {
                let mut x = if half { 0.0 } else { -0.5 * w };
                for &(n, d) in layers {
                    if n != clad_n && l > 0.0 {
                        let rect = |a: f64, b: f64| {
                            if bend.is_some() {
                                band(a, b, arc_points, &|_| (0.0, 0.0))
                            } else {
                                SceneShape {
                                    layer: "core".into(),
                                    outline: vec![
                                        point(0.0, a),
                                        point(1.0, a),
                                        point(1.0, b),
                                        point(0.0, b),
                                    ],
                                }
                            }
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
                    let k = if from == to { arc_points } else { 32 };
                    let half_at = |u: f64| 0.5 * profile.at(from, to, u);
                    shapes.push(band(center, center, k, &|u| (-half_at(u), half_at(u))));
                }
            }
        }
        for i in 1..=arc_points {
            let u = i as f64 / arc_points as f64;
            edges.extend([point(u, -0.5 * w), point(u, 0.5 * w)]);
        }
        pen = match bend {
            Some((radius, angle)) => pen.turned(radius, angle),
            None => Pen {
                at: pen.place(l, 0.0),
                ..pen
            },
        };
    }
    let span = |axis: usize| {
        let lo = edges.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min);
        let hi = edges
            .iter()
            .map(|p| p[axis])
            .fold(f64::NEG_INFINITY, f64::max);
        [lo, hi.max(lo + 1e-3)]
    };
    Ok(Event::Scene {
        x_um: span(0),
        y_um: span(1),
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

/// Where a section starts in the scene, and its heading (radians from x).
#[derive(Clone, Copy, Debug)]
struct Pen {
    at: [f64; 2],
    heading: f64,
}

impl Pen {
    /// The point `along` ahead and `y` across (to the left).
    fn place(&self, along: f64, y: f64) -> [f64; 2] {
        let (s, c) = self.heading.sin_cos();
        [
            self.at[0] + along * c - y * s,
            self.at[1] + along * s + y * c,
        ]
    }

    /// The point a bend of radius `radius` (to the pen; positive with its centre on the right,
    /// at −y) has reached after `angle` radians, `y` across from its middle.
    fn arc(&self, radius: f64, angle: f64, y: f64) -> [f64; 2] {
        let centre = self.place(0.0, -radius);
        let turn = radius.signum() * angle;
        let (s, c) = turn.sin_cos();
        let (hs, hc) = self.heading.sin_cos();
        // n cos + t sin, with t the heading and n to its left
        let out = [-hs * c + hc * s, hc * c + hs * s];
        [
            centre[0] + (radius + y) * out[0],
            centre[1] + (radius + y) * out[1],
        ]
    }

    /// The pen after a bend of `radius` and `angle`.
    fn turned(&self, radius: f64, angle: f64) -> Pen {
        Pen {
            at: self.arc(radius, angle, 0.0),
            heading: self.heading - radius.signum() * angle,
        }
    }
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
    let bends = first.iter().filter(|r| r.radius.is_some()).count();
    if bends > 0 {
        details.push([
            "bends".into(),
            format!(
                "{bends}, by the conformal map, each one's profile in thin layers at most {} um \
                 thick",
                crate::eme::BEND_STEP
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
    run.record(&sections_event(&task, &first))?;
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
        let (at, (_, error, steps), bend) = solve_at(&task, &resolved, task.wavelength_um, run)?;
        // the bends' layers as the study left them, kept at every solve too
        let task = EmeTask {
            bend_step_nm: bend.map(|h| h * 1e3).or(task.bend_step_nm),
            ..task.clone()
        };
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
        let (at, (s, error, steps), _) = solve_at(&task, &resolved, wl, run)?;
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
        if let Some(b) = &task.beam {
            run.record(&beam_event(&task, b, wl, at.0, &s, &ends)?)?;
        }
    }
    Ok(())
}

/// A `[task.beam]` launched on the first section's `modes` modes at `wavelength` µm and carried
/// by `s`: the shares of its power reflected, transmitted and in each guided mode at the last
/// end.
fn beam_event(
    task: &EmeTask,
    b: &BeamSpec,
    wavelength: f64,
    modes: usize,
    s: &Scattering,
    (first, last): &(Slice, Slice),
) -> Result<Event> {
    let lam = Wavelength::um(wavelength)?;
    let index = match b.index {
        Some(n) => n,
        None => {
            let m: Material = crate::device::spec::task_medium(task.cladding()?, "cladding")
                .map_err(|e| task_error(format!("cladding: {e}")))?;
            m.refractive_index(lam)
                .map_err(|e| task_error(format!("cladding: {e}")))?
                .re
        }
    };
    let beam = Beam::gaussian(
        Length::um(b.waist_um),
        Length::um(b.center_um.unwrap_or(0.0)),
    )?
    .tilted(b.angle_deg.unwrap_or(0.0).to_radians(), index)?;
    let launched = task.eme(lam, modes)?.launch(first, &beam)?;
    let p = launched.power();
    let (r, t) = s.forward(&launched.arriving())?;
    let n = guided(&s.last, last, false);
    let pol = match task.polarization()? {
        Polarization::Te => "TE",
        Polarization::Tm => "TM",
    };
    Ok(Event::EmeBeam {
        wavelength_um: wavelength,
        reflected: power(&r, &s.first) / p,
        transmitted: power(&t, &s.last) / p,
        ports: (0..n).map(|m| format!("2 {pol}{m}")).collect(),
        shares: t.iter().take(n).map(|a| a.norm_sqr() / p).collect(),
    })
}
