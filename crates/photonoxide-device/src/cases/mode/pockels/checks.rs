//! The validation cases of the Pockels effect in a mode ([`crate::mode::pockels`]): a uniform
//! field in each cut of bulk lithium niobate against the textbook Δn = −½ n³ r E, a film between
//! plates against its slab solved again, the perturbation against the re-solve on a thin-film
//! modulator's cross-section and its convergence on both grids, and the published thin-film
//! modulators' Vπ·L.
//!
//! The thin-film cross-sections ([`Tfln`]): x-cut lithium niobate (its c axis across the guide,
//! so a TE mode and the field across the gap meet through r₃₃) on a buried oxide over silicon,
//! a rib etched into the film, two electrodes on the slab on either side of it, under an oxide
//! cladding. The electrostatic problem holds one gap: the ground electrode to the window's left
//! edge, the signal electrode to its centre line, a mirror on the right (a ground–signal–ground
//! line's symmetry), every other side insulated, far away. The optics leave the metal out (its
//! absorption is the papers' < 0.03 dB/cm, not the index's change).

use faer::c64;

use crate::electro::{
    Boundary, Conductor, ElectricField, Electrostatic, Features, Grading, Grid, Section, Side,
    Tensor as Static,
};
use crate::geometry::{Point, Polygon, Shape};
use crate::material::catalogue::{Conditions, Tensor, entry};
use crate::material::{CrystalAxis, Material, Orientation};
use crate::mode::pockels::{
    Along, Drive, Pockels, half_wave_voltage_length, overlap_factor, section_change,
};
use crate::mode::thermo::{index_change, index_change_resolved};
use crate::mode::vector::{self, CrossSection, Permittivity, VectorMode, graded_nodes};
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

#[path = "checks/hybrid.rs"]
mod hybrid;

const OK: &str = "a valid model";

/// The wavelength of the thin-film cases, µm.
const LAMBDA: f64 = 1.55;

fn wavelength() -> Wavelength {
    Wavelength::um(LAMBDA).expect(OK)
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The catalogue's lithium niobate tensor labelled `label`.
fn linbo3_tensor(label: &str) -> Tensor {
    entry("linbo3")
        .expect(OK)
        .tensors
        .into_iter()
        .find(|t| t.label == label)
        .expect("a tensor of the catalogue")
}

/// Which coefficients a case takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Coefficients {
    /// Jazbinšek & Zgonik's clamped r^S and ε^S (633 nm).
    Clamped,
    /// Jazbinšek & Zgonik's unclamped r^T and ε^T (633 nm): the low-frequency values, below the
    /// crystal's acoustic resonances.
    Unclamped,
    /// The thin film's r₃₃ at 1550 nm (Chelladurai et al., clamped, 100 MHz to 330 GHz) with
    /// the bulk crystal's clamped other elements and ε^S.
    Film,
}

impl Coefficients {
    /// The tensor in the crystal's axes.
    pub(crate) fn pockels(self) -> Pockels {
        let bulk = |label| Pockels::from_tensor(&linbo3_tensor(label)).expect(OK);
        match self {
            Coefficients::Clamped => bulk("bulk, clamped, 633 nm"),
            Coefficients::Unclamped => bulk("bulk, unclamped, 633 nm"),
            Coefficients::Film => Pockels::completed(
                &linbo3_tensor("thin film (x-cut LNOI), clamped, 1550 nm"),
                &linbo3_tensor("bulk, clamped, 633 nm"),
            )
            .expect(OK),
        }
    }

    /// The static relative permittivity perpendicular to c and along it.
    pub(crate) fn static_permittivity(self) -> (f64, f64) {
        let symbol = |s: &str| {
            entry("linbo3")
                .expect(OK)
                .constants
                .into_iter()
                .find(|c| c.symbol == s)
                .expect("a constant of the catalogue")
                .value
        };
        match self {
            Coefficients::Clamped | Coefficients::Film => {
                (symbol(r"\varepsilon^S_{11}"), symbol(r"\varepsilon^S_{33}"))
            }
            Coefficients::Unclamped => {
                (symbol(r"\varepsilon^T_{11}"), symbol(r"\varepsilon^T_{33}"))
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Bulk crystals
// ---------------------------------------------------------------------------------------------

/// Lithium niobate (Zelmon's, 1.55 µm) oriented `orientation`.
fn oriented_linbo3(orientation: Orientation) -> Material {
    entry("linbo3")
        .expect(OK)
        .oriented(None, Conditions::default(), orientation)
        .expect(OK)
}

/// A uniform field of `volts_per_m` along the chip's axis `field_axis` in bulk lithium niobate
/// cut as `orientation` says, a guide along x: each transverse principal index's change from the
/// rotated tensor's Δε (the exact square root of ε + Δε on the axis) against the textbook
/// −½ n³ r E with the coefficient `textbook[a]` (pm/V, signed) for cross-section axis a (0
/// across, 1 up): the larger relative difference.
fn bulk(orientation: Orientation, field_axis: usize, textbook: [f64; 2]) -> f64 {
    let e = 1e5;
    let m = oriented_linbo3(orientation);
    let eps = Along::X.tensor(m.tensor(wavelength()).expect(OK));
    let r = Coefficients::Clamped
        .pockels()
        .rotated(&orientation)
        .in_section(Along::X);
    let mut chip = [0.0; 3];
    chip[field_axis] = e;
    let m3 = Along::X.matrix();
    let field: [f64; 3] = std::array::from_fn(|a| (0..3).map(|k| m3[a][k] * chip[k]).sum());
    let d = r.permittivity_change(eps, field);
    let mut worst = 0.0f64;
    for a in 0..2 {
        let n = eps[a][a].re.sqrt();
        let moved = (eps[a][a] + d[a][a]).re.sqrt() - n;
        let expected = -0.5 * n.powi(3) * textbook[a] * 1e-12 * e;
        worst = worst.max((moved / expected - 1.0).abs());
    }
    worst
}

/// An x-cut, its c axis across a guide along the chip's x (in the plane, along the chip's y),
/// the field across: Δn_e by r₃₃ across, Δn_o by r₁₃ up.
fn bulk_x_cut() -> Outcome {
    let o = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Y)).expect(OK);
    // x-cut, the crystal's y along the chip's x: its z (c) along the chip's y, across
    outcome(bulk(o, 1, [31.2, 9.10]), 0.0, 1e-4)
}

/// A z-cut, the field up along c: Δn_o by r₁₃ across, Δn_e by r₃₃ up.
fn bulk_z_cut() -> Outcome {
    let o = Orientation::cut(CrystalAxis::Z, None).expect(OK);
    outcome(bulk(o, 2, [9.10, 31.2]), 0.0, 1e-4)
}

/// A y-cut, the field up along the crystal's y: the ordinary indices split by ∓r₂₂, the one
/// along the crystal's x (across) by +½ n_o³ r₂₂ E, the c axis along the guide.
fn bulk_y_cut() -> Outcome {
    // y-cut, the crystal's x along the chip's y (across a guide along x), its z along the
    // chip's −x (along the guide)
    let o = Orientation::cut(CrystalAxis::Y, Some(CrystalAxis::Z)).expect(OK);
    // across is the crystal's x: Δη₁₁ = r₁₂ E_y = −r₂₂ E_y; up is its y: Δη₂₂ = r₂₂ E_y
    outcome(bulk(o, 2, [-3.40, 3.40]), 0.0, 1e-4)
}

// ---------------------------------------------------------------------------------------------
// Thin-film cross-sections
// ---------------------------------------------------------------------------------------------

/// A thin-film lithium niobate modulator's cross-section (µm): see the module's docs. y = 0 at
/// the film's bottom; the rib is centred on x = 0, the gap from −gap/2 to gap/2.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tfln {
    /// The film's thickness.
    pub film: f64,
    /// The slab left beside the rib.
    pub slab: f64,
    /// The rib's top width.
    pub top: f64,
    /// The sidewall's angle from the plane, degrees (90 vertical).
    pub angle_deg: f64,
    /// The electrodes' gap.
    pub gap: f64,
    /// The signal electrode's width to the mirror (half its width).
    pub signal: f64,
    /// The ground electrode's width.
    pub ground: f64,
    /// The electrodes' thickness.
    pub metal: f64,
    /// The buried oxide's thickness.
    pub buried: f64,
    /// The cladding's thickness over the slab (the oxide's top at slab + clad).
    pub clad: f64,
    /// The cladding's static relative permittivity.
    pub clad_eps: f64,
    /// Whether the cladding is the buried oxide's silica optically (else air above the film).
    pub clad_oxide: bool,
}

/// Thermal oxide's static relative permittivity.
const OXIDE_EPS: f64 = 3.9;
/// Silicon's.
const SILICON_EPS: f64 = 11.7;
/// The silicon under the buried oxide in the electrostatic window, µm.
const SUBSTRATE: f64 = 20.0;
/// The air over the cladding in the electrostatic window, µm.
const AIR: f64 = 20.0;

impl Tfln {
    /// The rib's half width at height y (in the rib's part of the film).
    fn half_width(&self, y: f64) -> f64 {
        0.5 * self.top + (self.film - y) / self.angle_deg.to_radians().tan()
    }

    /// Whether (x, y) is in the film.
    fn in_crystal(&self, x: f64, y: f64) -> bool {
        (0.0..self.slab).contains(&y)
            || ((self.slab..self.film).contains(&y) && x.abs() < self.half_width(y))
    }

    /// The rib's corners (µm): its foot on the slab, left and right, then its top, right and
    /// left.
    pub(crate) fn rib_corners(&self) -> [[f64; 2]; 4] {
        let (b, t) = (self.half_width(self.slab), 0.5 * self.top);
        [
            [-b, self.slab],
            [b, self.slab],
            [t, self.film],
            [-t, self.film],
        ]
    }

    /// The rib as a polygon.
    fn rib(&self) -> Shape {
        Shape::Polygon(
            Polygon::new(
                self.rib_corners()
                    .iter()
                    .map(|c| Point::um(c[0], c[1]))
                    .collect(),
            )
            .expect(OK),
        )
    }

    fn rect(x: [f64; 2], y: [f64; 2]) -> Shape {
        Shape::rect(
            Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
            Length::um(x[1] - x[0]),
            Length::um(y[1] - y[0]),
        )
        .expect(OK)
    }

    /// The electrostatic problem at `volts` on the signal, the ground at 0, its grid graded by
    /// `grading` from the rib's and the electrodes' corners.
    pub(crate) fn electrostatic(
        &self,
        coefficients: Coefficients,
        grading: &Grading,
        volts: f64,
    ) -> Electrostatic {
        let (perpendicular, along_c) = coefficients.static_permittivity();
        let left = -0.5 * self.gap - self.ground;
        let right = 0.5 * self.gap + self.signal;
        let (bottom, top) = (-self.buried - SUBSTRATE, self.slab + self.clad + AIR);
        let mut section = Section::new("air");
        section
            .layer(bottom, -self.buried, "Si")
            .and_then(|s| s.layer(-self.buried, 0.0, "box"))
            .and_then(|s| s.layer(self.slab, self.slab + self.clad, "clad"))
            .and_then(|s| s.layer(0.0, self.slab, "LN"))
            .expect(OK);
        section.draw(self.rib(), "LN");
        let ground = Tfln::rect(
            [left - 1.0, -0.5 * self.gap],
            [self.slab, self.slab + self.metal],
        );
        let signal = Tfln::rect(
            [0.5 * self.gap, right + 1.0],
            [self.slab, self.slab + self.metal],
        );
        let mut features: Features = section.features().clone();
        features
            .shape(&ground)
            .shape(&signal)
            .line_x(left)
            .line_x(right);
        // the optical region's corners, where the mode samples the field
        for x in [-1.5 * self.top, 1.5 * self.top] {
            features.corner(x, 0.0).corner(x, self.film);
        }
        let grid = Grid::fitted([left, right], [bottom, top], grading, &features).expect(OK);
        let mut problem = Electrostatic::new(section, grid);
        problem
            .permittivity("air", Static::isotropic(1.0))
            .permittivity("Si", Static::isotropic(SILICON_EPS))
            .permittivity("box", Static::isotropic(OXIDE_EPS))
            .permittivity("clad", Static::isotropic(self.clad_eps))
            // x-cut, c across: across ε₃₃, up ε₁₁
            .permittivity("LN", Static::diagonal(along_c, perpendicular))
            .conductor(Conductor::shape(ground, 0.0))
            .conductor(Conductor::shape(signal, volts))
            .boundary(Side::Left, Boundary::INSULATED)
            .boundary(Side::Right, Boundary::INSULATED);
        problem
    }

    /// The static field at `volts` on the signal.
    pub(crate) fn field(
        &self,
        coefficients: Coefficients,
        grading: &Grading,
        volts: f64,
    ) -> ElectricField {
        let p = self.electrostatic(coefficients, grading, volts);
        let phi = p.solve().expect(OK);
        p.electric_field(&phi).expect(OK)
    }

    /// The optical cross-section on a grid `h` fine over the film and the rib, growing away
    /// from them, and the material's permittivities: the crystal x-cut with c across.
    pub(crate) fn optics(&self, h: f64) -> (CrossSection, Optics) {
        self.optics_at(h, wavelength())
    }

    /// The same at the wavelength `w`: the crystal's and the oxide's permittivities there.
    pub(crate) fn optics_at(&self, h: f64, w: Wavelength) -> (CrossSection, Optics) {
        let x_cut = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z)).expect(OK);
        // along y: the chip's x across, so the crystal's c across
        let crystal = Along::Y.tensor(oriented_linbo3(x_cut).tensor(w).expect(OK));
        let oxide = entry("sio2")
            .expect(OK)
            .default_model()
            .expect(OK)
            .materials(Conditions::default())
            .expect(OK)
            .remove(0)
            .permittivity(w)
            .expect(OK);
        let optics = Optics {
            crystal,
            oxide,
            tfln: *self,
        };
        let edge = self.half_width(self.slab);
        let xs = graded_nodes(
            -3.0,
            3.0,
            (-edge - 0.3, edge + 0.3),
            h,
            0.08,
            0.1,
            &[-edge, edge, -0.5 * self.top, 0.5 * self.top],
        );
        let top = if self.clad_oxide {
            self.film + 1.5
        } else {
            (self.slab + self.clad).max(self.film) + 1.0
        };
        let mut fixed = vec![0.0, self.slab, self.film];
        if !self.clad_oxide {
            fixed.push(self.slab + self.clad);
        }
        let ys = graded_nodes(-1.5, top, (-0.2, self.film + 0.2), h, 0.08, 0.1, &fixed);
        let cells = optics.cells(&xs, &ys);
        (CrossSection::new(xs, ys, cells).expect(OK), optics)
    }
}

/// The samples a side of a cell takes for its fill.
const SAMPLES: usize = 4;

/// The materials of a thin-film cross-section, in the cross-section's axes.
pub(crate) struct Optics {
    crystal: [[c64; 3]; 3],
    oxide: c64,
    tfln: Tfln,
}

impl Optics {
    fn at(&self, x: f64, y: f64) -> Option<[[c64; 3]; 3]> {
        let t = &self.tfln;
        if t.in_crystal(x, y) {
            return None;
        }
        let zero = c64::new(0.0, 0.0);
        let e = if y < 0.0 || t.clad_oxide || y < t.slab + t.clad {
            self.oxide
        } else {
            c64::new(1.0, 0.0)
        };
        Some([[e, zero, zero], [zero, e, zero], [zero, zero, e]])
    }

    /// The points sampling cell (x0..x1, y0..y1).
    fn samples(x0: f64, x1: f64, y0: f64, y1: f64) -> impl Iterator<Item = (f64, f64)> {
        (0..SAMPLES * SAMPLES).map(move |s| {
            let (a, b) = (s / SAMPLES, s % SAMPLES);
            (
                x0 + (x1 - x0) * (a as f64 + 0.5) / SAMPLES as f64,
                y0 + (y1 - y0) * (b as f64 + 0.5) / SAMPLES as f64,
            )
        })
    }

    /// Each cell's permittivity, the mean of its samples' (cells x-major, as the solver has
    /// them).
    fn cells(&self, xs: &[f64], ys: &[f64]) -> Vec<Permittivity> {
        let mut cells = Vec::with_capacity((xs.len() - 1) * (ys.len() - 1));
        for i in 0..xs.len() - 1 {
            for j in 0..ys.len() - 1 {
                let mut sum = [[c64::new(0.0, 0.0); 3]; 3];
                for (x, y) in Optics::samples(xs[i], xs[i + 1], ys[j], ys[j + 1]) {
                    let t = self.at(x, y).unwrap_or(self.crystal);
                    for a in 0..3 {
                        for b in 0..3 {
                            sum[a][b] += t[a][b];
                        }
                    }
                }
                let n = (SAMPLES * SAMPLES) as f64;
                let mean = sum.map(|row| row.map(|v| v / n));
                cells.push(Permittivity::from_tensor(mean).expect(OK));
            }
        }
        cells
    }

    /// Each cell's change of permittivity in the static field `field` (V/m at a point, across
    /// and up): the mean over its samples in the crystal of −ε Δη ε, r the crystal's tensor in
    /// the cross-section's axes.
    pub(crate) fn change(
        &self,
        cs: &CrossSection,
        r: &Pockels,
        field: impl Fn(f64, f64) -> [f64; 2],
    ) -> Vec<Permittivity> {
        let (xs, ys) = (cs.x(), cs.y());
        let mut out = Vec::with_capacity(cs.cells().len());
        for i in 0..xs.len() - 1 {
            for j in 0..ys.len() - 1 {
                let mut sum = [[c64::new(0.0, 0.0); 3]; 3];
                for (x, y) in Optics::samples(xs[i], xs[i + 1], ys[j], ys[j + 1]) {
                    if self.at(x, y).is_none() {
                        let e = field(x, y);
                        let d = r.permittivity_change(self.crystal, [e[0], e[1], 0.0]);
                        for a in 0..3 {
                            for b in 0..3 {
                                sum[a][b] += d[a][b];
                            }
                        }
                    }
                }
                let n = (SAMPLES * SAMPLES) as f64;
                out.push(section_change(sum.map(|row| row.map(|v| v / n))));
            }
        }
        out
    }
}

/// The crystal's tensor in a thin-film cross-section's axes: x-cut, c across a guide along y.
pub(crate) fn section_pockels(coefficients: Coefficients) -> Pockels {
    let x_cut = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z)).expect(OK);
    coefficients.pockels().rotated(&x_cut).in_section(Along::Y)
}

/// The TE-like fundamental mode of `cs`: of the two highest, the one with the larger TE share.
fn te_mode(cs: &CrossSection) -> VectorMode {
    te_mode_at(cs, wavelength()).expect(OK)
}

/// The same at the wavelength `w`.
///
/// # Errors
///
/// The solver's, and [`crate::Error::InvalidValue`] when it finds no mode.
pub(crate) fn te_mode_at(cs: &CrossSection, w: Wavelength) -> crate::Result<VectorMode> {
    let modes = vector::modes(cs, w, 2, None)?;
    modes
        .into_iter()
        .max_by(|a, b| a.te_fraction().total_cmp(&b.te_fraction()))
        .ok_or_else(|| {
            crate::Error::invalid("thin-film modulator", "its cross-section guides no mode")
        })
}

/// `tfln`'s push–pull Vπ·L (V·cm) with `coefficients`, the static grid graded by `grading`,
/// the optical grid `h` fine.
pub(crate) fn modulation(
    tfln: &Tfln,
    coefficients: Coefficients,
    grading: &Grading,
    h: f64,
) -> f64 {
    let field = tfln.field(coefficients, grading, 1.0);
    let (cs, optics) = tfln.optics(h);
    let mode = te_mode(&cs);
    let delta = optics.change(&cs, &section_pockels(coefficients), |x, y| field.at(x, y));
    let per_volt = index_change(&mode, &cs, &delta).expect(OK).re;
    100.0 * half_wave_voltage_length(per_volt, wavelength(), Drive::PushPull).expect(OK)
}

/// Wang et al. 2018's cross-section (Opt. Express 26, 1547): a 700 nm x-cut film etched 400 nm
/// to a rib 900 nm wide on top on 2 µm of silica, a 3.5 µm gap between electrodes of 15 nm of Ti
/// and 300 nm of Au, 1.5 µm of silica over them (its Section 2).
pub(crate) fn wang_2018(angle_deg: f64) -> Tfln {
    Tfln {
        film: 0.7,
        slab: 0.3,
        top: 0.9,
        angle_deg,
        gap: 3.5,
        signal: 5.0,
        ground: 15.0,
        metal: 0.315,
        buried: 2.0,
        clad: 1.5,
        clad_eps: OXIDE_EPS,
        clad_oxide: true,
    }
}

/// Zhang et al. 2022's (Photon. Res. 10, 2380): a 600 nm x-cut film etched 300 nm to a rib 1.2
/// µm wide on top, 2 µm of thermal oxide on silicon, a 5 µm gap, a 20 µm signal, 0.83 µm of
/// metal, 0.6 µm of PECVD oxide (ε 5.5, as their model has it) and air above.
pub(crate) fn zhang_2022(angle_deg: f64) -> Tfln {
    Tfln {
        film: 0.6,
        slab: 0.3,
        top: 1.2,
        angle_deg,
        gap: 5.0,
        signal: 10.0,
        ground: 15.0,
        metal: 0.83,
        buried: 2.0,
        clad: 0.6,
        clad_eps: 5.5,
        clad_oxide: false,
    }
}

/// The re-solve's Δn_eff over the perturbation's at `volts`, less 1, on `tfln`.
pub(crate) fn resolve_difference(tfln: &Tfln, volts: f64, h: f64) -> f64 {
    let grading = Grading::new(1.0, 0.02, 1.15).expect(OK);
    let field = tfln.field(Coefficients::Film, &grading, volts);
    let (cs, optics) = tfln.optics(h);
    let mode = te_mode(&cs);
    let delta = optics.change(&cs, &section_pockels(Coefficients::Film), |x, y| {
        field.at(x, y)
    });
    let first = index_change(&mode, &cs, &delta).expect(OK).re;
    let resolved = index_change_resolved(&mode, &cs, &delta, wavelength())
        .expect(OK)
        .re;
    resolved / first - 1.0
}

// ---------------------------------------------------------------------------------------------
// A film between plates, in closed form
// ---------------------------------------------------------------------------------------------

/// The film's thickness in the plates cases, µm.
const FILM: f64 = 0.6;
/// The oxide on either side of it in the optics, µm.
const BEYOND: f64 = 2.0;

/// Silica's index at 1.55 µm (Malitson's, the catalogue's default).
fn oxide_index() -> f64 {
    entry("sio2")
        .expect(OK)
        .default_model()
        .expect(OK)
        .materials(Conditions::default())
        .expect(OK)
        .remove(0)
        .refractive_index(wavelength())
        .expect(OK)
        .re
}

/// A `FILM` µm film of lithium niobate cut as `cut` says (x: c across; z: c up) centred on y =
/// 0 in silica, a static field from plates: for the x-cut on the window's sides 10 µm apart at 0
/// and 10 V, a uniform field across; for the z-cut above and below it, 1 µm of oxide (ε 3.9)
/// between each and the film, at 0 and 2 V, the film's field D / ε₃₃ by D's continuity. Its TE₀
/// mode (E across) on the full-vector solver, 5 nm cells, a quarter between mirror walls, by
/// first-order perturbation in the static solve's field, against the slab solved again with the
/// film's index moved by the textbook −½ n³ r E: (the larger of) the relative differences of
/// Δn_eff, and for the z-cut of the film's field from its closed form.
fn plates(cut: CrystalAxis) -> f64 {
    let c = Coefficients::Clamped;
    let (perpendicular, along_c) = c.static_permittivity();
    let x_cut = cut == CrystalAxis::X;
    let orientation = if x_cut {
        Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Z))
    } else {
        Orientation::cut(CrystalAxis::Z, None)
    }
    .expect(OK);
    // the static problem
    let half = 0.5 * FILM;
    let mut section = Section::new("oxide");
    section.layer(-half, half, "LN").expect(OK);
    let (window_x, window_y, volts) = if x_cut {
        ([-5.0, 5.0], [-3.0, 3.0], 10.0)
    } else {
        ([-1.0, 1.0], [-half - 1.0, half + 1.0], 2.0)
    };
    let features = section.features().clone();
    let grid = Grid::fitted(
        window_x,
        window_y,
        &Grading::uniform(0.05).expect(OK),
        &features,
    )
    .expect(OK);
    let mut problem = Electrostatic::new(section, grid);
    problem
        .permittivity("oxide", Static::isotropic(OXIDE_EPS))
        .permittivity(
            "LN",
            if x_cut {
                Static::diagonal(along_c, perpendicular)
            } else {
                Static::diagonal(perpendicular, along_c)
            },
        );
    let (low, high) = if x_cut {
        (Side::Left, Side::Right)
    } else {
        (Side::Bottom, Side::Top)
    };
    problem
        .boundary(low, Boundary::Fixed(0.0))
        .boundary(high, Boundary::Fixed(volts));
    let phi = problem.solve().expect(OK);
    let field = problem.electric_field(&phi).expect(OK);
    // the film's field in closed form: E = -V/W across, or -D/ε₃₃ with D = V / Σ t/ε up
    let (axis, exact_field) = if x_cut {
        (0, -volts / ((window_x[1] - window_x[0]) * 1e-6))
    } else {
        let d = volts / ((FILM / along_c + 2.0 / OXIDE_EPS) * 1e-6);
        (1, -d / along_c)
    };
    let film_field = field.at(0.0, 0.0)[axis];
    // the optics: the quarter x, y ≥ 0, TE between electric walls, a magnetic wall at y = 0
    let h = 0.005;
    let crystal = Along::Y.tensor(oriented_linbo3(orientation).tensor(wavelength()).expect(OK));
    let ox = oxide_index();
    let oxide = Permittivity::isotropic(c64::new(ox * ox, 0.0));
    let ny = ((half + BEYOND) / h).round() as usize;
    let cs = CrossSection::uniform((0.0, 4.0 * h, 4), (0.0, ny as f64 * h, ny), |_, y| {
        if y < half {
            Permittivity::from_tensor(crystal).expect(OK)
        } else {
            oxide
        }
    })
    .and_then(|cs| {
        cs.with_boundaries(vector::Boundaries {
            west: vector::Boundary::ElectricWall,
            east: vector::Boundary::ElectricWall,
            south: vector::Boundary::MagneticWall,
            ..vector::Boundaries::default()
        })
    })
    .expect(OK);
    let r = c.pockels().rotated(&orientation).in_section(Along::Y);
    let ys = cs.y().to_vec();
    let mut delta = Vec::with_capacity(cs.cells().len());
    for _ in 0..4 {
        for j in 0..ny {
            let y = 0.5 * (ys[j] + ys[j + 1]);
            delta.push(if y < half {
                let e = field.at(0.0, y);
                section_change(r.permittivity_change(crystal, [e[0], e[1], 0.0]))
            } else {
                Permittivity::isotropic(c64::new(0.0, 0.0))
            });
        }
    }
    let mode = vector::modes(&cs, wavelength(), 1, None)
        .expect(OK)
        .remove(0);
    let first = index_change(&mode, &cs, &delta).expect(OK).re;
    // the slab, its film's index moved by the textbook change along E (the cross-section's x)
    let n = crystal[0][0].re.sqrt();
    let coefficient = r.element(0, 0, axis);
    let moved = n - 0.5 * n.powi(3) * coefficient * exact_field;
    let slab = |core: f64| {
        crate::mode::slab::Slab::new(ox, core, ox, Length::um(FILM))
            .expect(OK)
            .modes(crate::mode::Polarization::Te, wavelength())
            .remove(0)
            .effective_index()
    };
    let exact = slab(moved) - slab(n);
    (first / exact - 1.0)
        .abs()
        .max((film_field / exact_field - 1.0).abs())
}

fn plates_x_cut() -> Outcome {
    outcome(plates(CrystalAxis::X), 0.0, 1e-3)
}

fn plates_z_cut() -> Outcome {
    outcome(plates(CrystalAxis::Z), 0.0, 1e-3)
}

// ---------------------------------------------------------------------------------------------
// The perturbation against the re-solve, and the grids
// ---------------------------------------------------------------------------------------------

/// The static grading of the thin-film cases: `smallest` at the corners, growing by 15% a cell
/// to 1 µm.
fn grading(smallest: f64) -> Grading {
    Grading::new(1.0, smallest, 1.15).expect(OK)
}

/// The sidewall angle the thin-film cases take, degrees: the papers don't give theirs.
const ANGLE: f64 = 70.0;

/// Wang et al.'s cross-section at 10 V, 20 nm optical cells: the re-solve's Δn_eff against the
/// perturbation's (relative difference).
fn resolve_tfln() -> Outcome {
    outcome(resolve_difference(&wang_2018(ANGLE), 10.0, 0.02), 0.0, 2e-3)
}

/// The same at 10, 20 and 40 V: the difference's part that grows with V, second order in Δε,
/// doubles as V does (log₂ of the ratio of its two changes).
fn resolve_order() -> Outcome {
    let d = [10.0, 20.0, 40.0].map(|v| resolve_difference(&wang_2018(ANGLE), v, 0.02));
    outcome(((d[2] - d[1]) / (d[1] - d[0])).log2(), 1.0, 0.1)
}

/// Vπ·L of Wang et al.'s cross-section (film coefficients) on the static grid's smallest cells
/// of 20, 10 and 5 nm, 20 nm optical cells: the order of convergence.
fn static_order() -> Outcome {
    let v = [0.02, 0.01, 0.005]
        .map(|s| modulation(&wang_2018(ANGLE), Coefficients::Film, &grading(s), 0.02));
    outcome(((v[0] - v[1]) / (v[1] - v[2])).log2(), 1.0, 0.5)
}

/// Vπ·L of the same on optical cells of 40, 20 and 10 nm, the static grid's smallest 10 nm: the
/// grid study's estimate of the finest one's error, relative.
fn optical_convergence() -> Outcome {
    let mut study = crate::convergence::Study::new("Vpi L");
    for h in [0.04, 0.02, 0.01] {
        let v = modulation(&wang_2018(ANGLE), Coefficients::Film, &grading(0.01), h);
        study = study.on(h, v).expect(OK);
    }
    let c = study.check(1e-3).expect(OK);
    outcome(c.error.unwrap_or(f64::INFINITY) / c.value, 0.0, 1e-3)
}

/// Push–pull Vπ·L (V·cm) of `tfln` with the film's coefficients, both grids refined together
/// (the static grid's smallest cells and the optical cells 40, 20 and 10 nm), gated by the grid
/// study at 0.5%: the finest grid's value.
fn published(tfln: &Tfln) -> f64 {
    let mut study = crate::convergence::Study::new("Vpi L");
    for h in [0.04, 0.02, 0.01] {
        let v = modulation(tfln, Coefficients::Film, &grading(h), h);
        study = study.on(h, v).expect(OK);
    }
    let c = study.check(5e-3 * study.values[2]).expect(OK);
    c.gated().expect("a converged grid")
}

/// Wang et al. 2018 (Opt. Express): their 2 mm push–pull Mach–Zehnder's Vπ of 9 V, 1.8 V·cm.
fn wang_2018_vpi_l() -> Outcome {
    outcome(published(&wang_2018(ANGLE)), 1.8, 0.15)
}

/// Zhang et al. 2022: 2.3 V·cm at DC.
fn zhang_2022_vpi_l() -> Outcome {
    outcome(published(&zhang_2022(ANGLE)), 2.3, 0.15)
}

/// The overlap Γ of Wang et al.'s cross-section (10 nm grids) as the papers define it, and the
/// Vπ·L their formula λ g / (2 n_e³ r₃₃ Γ) gives with it scaled by n_eff / n_e (the power's
/// normalization, which their form leaves out), against the perturbation's: the relative
/// difference, and Γ.
pub(crate) fn overlap_gamma() -> (f64, f64) {
    let t = wang_2018(ANGLE);
    let field = t.field(Coefficients::Film, &grading(0.01), 1.0);
    let (cs, optics) = t.optics(0.01);
    let mode = te_mode(&cs);
    let (xs, ys) = (cs.x().to_vec(), cs.y().to_vec());
    let share = |i: usize, j: usize| {
        Optics::samples(xs[i], xs[i + 1], ys[j], ys[j + 1])
            .filter(|&(x, y)| optics.at(x, y).is_none())
            .count() as f64
            / (SAMPLES * SAMPLES) as f64
    };
    // the field across, from the signal on the right to the ground: along −x
    let gamma = overlap_factor(&mode, &cs, share, |x, y| -field.at(x, y)[0], t.gap, 1.0).expect(OK);
    let ne = optics.crystal[0][0].re.sqrt();
    let r33 = Coefficients::Film.pockels().voigt()[2][2] * 1e-12;
    let papers = 100.0 * LAMBDA * 1e-6 * t.gap * 1e-6 / (2.0 * ne.powi(3) * r33 * gamma);
    let scaled = papers * mode.effective_index().re / ne;
    let ours = modulation(&t, Coefficients::Film, &grading(0.01), 0.01);
    (scaled / ours - 1.0, gamma)
}

fn overlap_gamma_case() -> Outcome {
    outcome(overlap_gamma().0, 0.0, 0.02)
}

// ---------------------------------------------------------------------------------------------
// Barium titanate
// ---------------------------------------------------------------------------------------------

/// The catalogue's barium titanate tensor labelled `label`.
fn batio3_tensor(label: &str) -> Tensor {
    entry("batio3")
        .expect(OK)
        .tensors
        .into_iter()
        .find(|t| t.label == label)
        .expect("a tensor of the catalogue")
}

/// A tetragonal (4mm) tensor with r₁₃, r₃₃ and r₄₂ = r₅₁ (pm/V).
fn tetragonal(r13: f64, r33: f64, r42: f64) -> Pockels {
    let mut v = [[0.0; 3]; 6];
    v[0][2] = r13;
    v[1][2] = r13;
    v[2][2] = r33;
    v[3][1] = r42;
    v[4][0] = r42;
    Pockels::from_voigt(v).expect(OK)
}

/// The effective coefficient r_xxx (pm/V) of `r` in a film whose c axis lies in the plane at
/// `degrees` from the chip's x (the Euler angles (φ, 90°, 0): c along (sin φ, −cos φ, 0), so x
/// is at φ from the in-plane a axis), the field and the light both along x.
fn in_plane(r: &Pockels, degrees: f64) -> f64 {
    let o = Orientation::euler_deg(degrees, 90.0, 0.0).expect(OK);
    r.rotated(&o).element(0, 0, 0) * 1e12
}

/// Abel et al.'s tensor (r₁₃ taken as 0, which they don't resolve) turned in the plane from 0 to
/// 90° by 15°: r_xxx against r₃₃ sin³φ + (r₁₃ + 2r₄₂) cos²φ sin φ, the largest difference over
/// the largest value.
fn bto_angle() -> Outcome {
    let t = batio3_tensor("thin film on SOI, 1550 nm");
    let (r33, r42) = (t.value(3, 3).expect(OK), t.value(4, 2).expect(OK));
    let r = tetragonal(0.0, r33, r42);
    let (mut worst, mut largest) = (0.0f64, 0.0f64);
    for k in 0..=6 {
        let phi = (15.0 * f64::from(k)).to_radians();
        let (s, c) = phi.sin_cos();
        let expected = r33 * s.powi(3) + 2.0 * r42 * c * c * s;
        worst = worst.max((in_plane(&r, 15.0 * f64::from(k)) - expected).abs());
        largest = largest.max(expected.abs());
    }
    outcome(worst / largest, 0.0, 1e-12)
}

/// Deng et al.'s r₄₂ alone at 45° from the c axis: the effective coefficient along ⟨110⟩ they
/// measured, r₄₂/√2.
fn bto_deng() -> Outcome {
    let r42 = batio3_tensor("thin film on LSAT, 1550 nm")
        .value(5, 1)
        .expect(OK);
    let r = tetragonal(0.0, 0.0, r42);
    outcome((in_plane(&r, 45.0) * 10.0).round() / 10.0, 253.0, 1.0)
}

/// Eltes et al.'s single-arm Vπ·L (V·cm) from their own r_eff, index, gap and overlap, through
/// dn_eff/dV = ½ n³ r_eff Γ / g.
fn bto_eltes() -> Outcome {
    // Eltes et al.'s own constants in the entry
    let constant = |symbol: &str| {
        entry("batio3")
            .expect(OK)
            .constants
            .into_iter()
            .find(|c| c.symbol == symbol && c.source.reference == "eltes-2019")
            .expect("a constant of the catalogue")
            .value
    };
    let (r, n) = (constant(r"r_\mathrm{eff}"), constant("n"));
    let (gap, overlap) = (2.6e-6, 0.38);
    let per_volt = 0.5 * n.powi(3) * r * 1e-12 * overlap / gap;
    let vpi_l =
        100.0 * half_wave_voltage_length(per_volt, wavelength(), Drive::SingleArm).expect(OK);
    outcome((vpi_l * 1e4).round() / 1e4, 0.23, 0.005)
}

/// Lithium tantalate x-cut, Casson et al.'s unclamped r₁₃ and r₃₃ at 1558 nm (r₂₂ and r₅₁, which
/// they don't measure, as 0: with the field along c they don't enter) and their indices there, a
/// uniform field of 0.1 V/µm across a guide along the chip's x along c: each transverse index's
/// change from the turned tensor against −½ n³ r E, the larger relative difference.
fn litao3_x_cut() -> Outcome {
    let e = entry("litao3").expect(OK);
    let t = e
        .tensors
        .iter()
        .find(|t| t.label == "bulk, unclamped, 1558 nm")
        .expect("a tensor of the catalogue");
    let (r13, r33) = (t.value(1, 3).expect(OK), t.value(3, 3).expect(OK));
    let n = |symbol: &str| {
        e.constants
            .iter()
            .find(|c| c.symbol == symbol)
            .expect("a constant of the catalogue")
            .value
    };
    let (no, ne) = (n("n_o"), n("n_e"));
    let mut v = [[0.0; 3]; 6];
    v[0][2] = r13;
    v[1][2] = r13;
    v[2][2] = r33;
    let o = Orientation::cut(CrystalAxis::X, Some(CrystalAxis::Y)).expect(OK);
    let c = |x: f64| c64::new(x, 0.0);
    let eps = Along::X.tensor(o.rotate([c(no * no), c(no * no), c(ne * ne)]));
    let r = Pockels::from_voigt(v)
        .expect(OK)
        .rotated(&o)
        .in_section(Along::X);
    // the field along the chip's y, across the guide: the crystal's c
    let field = 1e5;
    let d = r.permittivity_change(eps, [field, 0.0, 0.0]);
    let mut worst = 0.0f64;
    for (a, coefficient) in [(0, r33), (1, r13)] {
        let n = eps[a][a].re.sqrt();
        let moved = (eps[a][a] + d[a][a]).re.sqrt() - n;
        let expected = -0.5 * n.powi(3) * coefficient * 1e-12 * field;
        worst = worst.max((moved / expected - 1.0).abs());
    }
    outcome(worst, 0.0, 1e-4)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "pockels/bulk-x-cut",
            title: r"A uniform field of 0.1 V/µm in x-cut lithium niobate (Jazbinšek & Zgonik's clamped tensor, 633 nm; Zelmon's indices at 1.55 µm), across a guide along the chip's x, the c axis across: each transverse index's change from the tensor turned by the cut, $\Delta\varepsilon = -\varepsilon\thinspace\Delta\eta\thinspace\varepsilon$, against $-\tfrac12 n^3 r E$ ($r_{33}$ for $n_e$ across, $r_{13}$ for $n_o$ up; the larger relative difference shown, second order in $E$)",
            tier: Tier::Analytic,
            source: r"the index ellipsoid to first order in $E$: $\Delta(1/n^2)_{ij} = r_{ijk} E_k$",
            run: bulk_x_cut,
        },
        Case {
            id: "pockels/bulk-z-cut",
            title: r"The same in z-cut lithium niobate, the field up along c ($r_{13}$ for $n_o$ across, $r_{33}$ for $n_e$ up)",
            tier: Tier::Analytic,
            source: r"the index ellipsoid to first order in $E$",
            run: bulk_z_cut,
        },
        Case {
            id: "pockels/bulk-y-cut",
            title: r"The same in y-cut lithium niobate, the field up along the crystal's y, its c axis along the guide: the ordinary indices across and up split by $\pm\tfrac12 n_o^3 r_{22} E$",
            tier: Tier::Analytic,
            source: r"the index ellipsoid to first order in $E$ ($r_{12} = -r_{22}$ in 3m)",
            run: bulk_y_cut,
        },
        Case {
            id: "pockels/plates-x-cut",
            title: r"A 600 nm x-cut lithium niobate film in silica, c across, a uniform field of 1 V/µm across from plates 10 µm apart (the static solver's): its TE₀ mode's $\Delta n_\mathrm{eff}$ at 1.55 µm by first-order perturbation on the full-vector solver (5 nm cells) against the slab solved again with the film's $n_e$ moved by $-\tfrac12 n_e^3 r_{33} E$ (relative difference shown)",
            tier: Tier::Analytic,
            source: r"the slab's dispersion relation; Jazbinšek & Zgonik's clamped $r_{33}$ (Appl. Phys. B 74, 407 (2002), doi:10.1007/s003400200818)",
            run: plates_x_cut,
        },
        Case {
            id: "pockels/plates-z-cut",
            title: r"The same film z-cut between plates above and below it, 1 µm of oxide ($\varepsilon$ 3.9) between each and the film at 2 V: the film's field from the static solve against $D/\varepsilon_{33}$ by $D$'s continuity, and TE₀'s $\Delta n_\mathrm{eff}$ against the slab with $n_o$ moved by $-\tfrac12 n_o^3 r_{13} E$ (the larger relative difference shown)",
            tier: Tier::Analytic,
            source: r"a layered capacitor's field and the slab's dispersion relation; Jazbinšek & Zgonik's clamped $r_{13}$ and $\varepsilon^S_{33}$",
            run: plates_z_cut,
        },
        Case {
            id: "pockels/resolve-tfln",
            title: r"Wang et al. 2018's thin-film modulator cross-section (below) at 10 V: TE₀'s $\Delta n_\mathrm{eff}$ by first-order perturbation against the mode of the perturbed cross-section solved again (20 nm optical cells; relative difference shown)",
            tier: Tier::Analytic,
            source: r"the re-solve, the elements of $\Delta\varepsilon$ coupling $z$ to $x$ and $y$ left out of both (they add nothing to first order)",
            run: resolve_tfln,
        },
        Case {
            id: "pockels/resolve-order",
            title: r"The same at 10, 20 and 40 V: the difference's part that grows with the field, second order in $\Delta\varepsilon$, doubles with it ($\log_2$ of the ratio of its changes shown)",
            tier: Tier::Analytic,
            source: r"first-order perturbation leaves an error of second order in $\Delta\varepsilon$, so of first order relative to $\Delta n_\mathrm{eff}$",
            run: resolve_order,
        },
        Case {
            id: "pockels/static-order",
            title: r"Wang et al. 2018's push–pull $V_\pi L$ (film coefficients, 20 nm optical cells) on the static grid's smallest cells of 20, 10 and 5 nm at the rib's and the electrodes' corners: its order of convergence (shown), about first at the electrodes' edges",
            tier: Tier::Analytic,
            source: r"the static solver's order at a thin electrode's $r^{-1/2}$ singularity (0.82 on a coplanar waveguide, static/coplanar-order)",
            run: static_order,
        },
        Case {
            id: "pockels/optical-convergence",
            title: r"The same $V_\pi L$ on optical cells of 40, 20 and 10 nm (the static grid's smallest 10 nm): the grid study's estimate of the 10 nm value's error, relative (shown)",
            tier: Tier::Analytic,
            source: r"the grid study (docs/methods/convergence.md)",
            run: optical_convergence,
        },
        Case {
            id: "pockels/overlap-gamma",
            title: r"Wang et al. 2018's cross-section on 10 nm grids: the papers' overlap $\Gamma = (g/V)\int_\mathrm{LN} E_x \lvert E\rvert^2 / \int \lvert E\rvert^2$ (0.51 here) in their $V_\pi L = \lambda g / (2 n_e^3 r_{33} \Gamma)$, scaled by $n_\mathrm{eff}/n_e$, against the perturbation's $V_\pi L$ (relative difference shown): their form normalizes the mode by $\int \lvert E\rvert^2$, not by its power, so unscaled it is 10% high; what is left is $E$'s other components and $r_{13}$",
            tier: Tier::Analytic,
            source: r"first-order perturbation; the overlap as C. Wang et al., Opt. Express 26, 1547 (2018), doi:10.1364/OE.26.001547, and D. Zhu et al., Adv. Opt. Photon. 13, 242 (2021), doi:10.1364/AOP.411024, use it",
            run: overlap_gamma_case,
        },
        Case {
            id: "pockels/wang-2018",
            title: r"Wang et al. 2018's x-cut thin-film modulator, a 700 nm film etched 400 nm to a rib 900 nm wide on top on 2 µm of silica, a 3.5 µm gap between electrodes of 15 nm of Ti and 300 nm of Au, 1.5 µm of silica over them: push–pull $V_\pi L$ (V·cm, a cross-section's, per unit length) at 1.55 µm from the static solve and the TE₀ mode's first-order perturbation, the thin film's $r_{33}$ = 26.9 pm/V with the bulk's other clamped elements and $\varepsilon^S$; both grids refined together to 10 nm and gated at 0.5% (shown). The sidewall, which the paper doesn't give, at 70°",
            tier: Tier::Published,
            source: r"C. Wang et al., Opt. Express 26, 1547 (2018), doi:10.1364/OE.26.001547: a 2 mm push–pull Mach–Zehnder's DC $V_\pi$ of 9 V, 1.8 V·cm. The tolerance: $r_{33}$'s ±0.9 pm/V (D. Chelladurai et al. 2025), the sidewall from 60 to 90° (±0.04 V·cm), the paper's two digits; a DC measurement, where lithium niobate's relaxation below about 1 MHz (J. Holzgrafe et al., Opt. Express 32, 3619 (2024), doi:10.1364/OE.507536) isn't modelled",
            run: wang_2018_vpi_l,
        },
        Case {
            id: "pockels/zhang-2022",
            title: r"Zhang et al. 2022's x-cut thin-film modulator, a 600 nm film etched 300 nm to a rib 1.2 µm wide on top, 2 µm of thermal oxide on silicon, a 5 µm gap beside a 20 µm signal electrode 0.83 µm thick, 0.6 µm of PECVD oxide ($\varepsilon$ 5.5, their model's) and air: push–pull $V_\pi L$ as above (V·cm, a cross-section's, per unit length; shown), the sidewall at 70° (not given)",
            tier: Tier::Published,
            source: r"Y. Zhang et al., Photon. Res. 10, 2380 (2022), doi:10.1364/PRJ.468518: 2.3 V·cm at DC (2.67 at 1 GHz, through the line's impedance). The tolerance: $r_{33}$'s ±0.9 pm/V, the sidewall from 60 to 90° (±0.03 V·cm), the paper's two digits; DC, as above",
            run: zhang_2022_vpi_l,
        },
        Case {
            id: "pockels/valdez-2023",
            title: r"Valdez et al. 2023's hybrid modulator at 784 nm: an unetched 200 nm x-cut film of 5 mol% MgO-doped lithium niobate bonded, over 40 nm of silica, onto a planarized silicon nitride strip (0.9 µm wide, 0.18 µm thick, LPCVD index, 2.03 here) in silica on silicon, gold electrodes 1 µm thick on the film 4 µm apart (the narrowest gap of their segmented electrode, taken as a plain gap), air above: push–pull $V_\pi L$ of the hybrid TE mode (V·cm, a cross-section's, per unit length; shown), from Akiyama et al.'s unclamped $r_{13}$ = 9.3 and $r_{33}$ = 30.1 pm/V of the doped crystal (633 nm) with the bulk's other unclamped elements and $\varepsilon^T$; both grids refined together to 5 nm and gated at 0.5%",
            tier: Tier::Published,
            source: r"F. Valdez, V. Mere, S. Mookherjea, Optica 10, 578 (2023), doi:10.1364/OPTICA.484549: a push–pull $V_\pi$ of 2.0 V (0.4 cm) and 1.0 V (0.8 cm), $V_\pi L$ = 0.8 V·cm at 100 kHz; the tolerance their one digit. Not in the main text, and left as taken here: the nitride's index (its Fig. 1(e) gives $n_\mathrm{eff}$ 1.90 and the power in the film and the strip 0.69 and 0.15; ours are 1.908, 0.64 and 0.20, and a nitride index lower by 0.05 gives 1.898, 0.67 and 0.16, so these hang on it), the electrodes' thickness and widths, and the field of the segmented electrode between its segments; the coefficients are those measured near 633 nm, not 784 nm",
            run: hybrid::valdez_2023_vpi_l,
        },
        Case {
            id: "pockels/bto-angle",
            title: r"Barium titanate (4mm), Abel et al.'s $r_{42}$ = 923 and $r_{33}$ = 342 pm/V ($r_{13}$ taken as 0) in a film with its c axis in the plane, turned by Euler angles $(\varphi, 90°, 0)$ from 0 to 90° by 15°: the effective coefficient for the field and the light along $x$ from the rotated tensor against $r_{33}\sin^3\varphi + (r_{13} + 2r_{42})\cos^2\varphi\sin\varphi$ (the largest difference over the largest value shown)",
            tier: Tier::Analytic,
            source: r"the projection $r_\mathrm{eff} = \sum_{ijk} u_i u_j u_k r_{ijk}$, the form of D. Chelladurai et al., Nat. Mater. 24, 868 (2025), doi:10.1038/s41563-025-02158-1, Eq. (4); S. Abel et al., Nat. Mater. 18, 42 (2019), doi:10.1038/s41563-018-0208-0, p. 44, for the tensor",
            run: bto_angle,
        },
        Case {
            id: "pockels/bto-deng",
            title: r"Deng et al.'s barium titanate on LSAT, $r_{42}$ = 358 pm/V alone, the field and the light along ⟨110⟩, 45° from the c axis: the effective coefficient from the rotated tensor (pm/V, shown)",
            tier: Tier::Published,
            source: r"C. Deng et al., Light Sci. Appl. 15, 21 (2026), doi:10.1038/s41377-025-02081-9: $r_c$ = 253 pm/V measured along ⟨110⟩, of which their 358 pm/V is the projection; the tolerance their last digit",
            run: bto_deng,
        },
        Case {
            id: "pockels/bto-eltes",
            title: r"Eltes et al.'s barium titanate modulator in a silicon photonics back end: $V_\pi L$ on one arm (V·cm, shown) from their $r_\mathrm{eff}$ = 380 pm/V, $n$ = 2.29, $\Gamma$ = 0.38 and a 2.6 µm gap through $dn_\mathrm{eff}/dV = \tfrac12 n^3 r_\mathrm{eff}\Gamma/g$ and the one-arm $V_\pi L = \lambda/(2\thinspace dn_\mathrm{eff}/dV)$",
            tier: Tier::Published,
            source: r"F. Eltes et al., J. Lightwave Technol. 37, 1456 (2019), doi:10.1109/JLT.2019.2893500, Section IV: a DC $V_\pi L$ of 0.23 V·cm from a voltage on one arm, its Eq. (1) giving $r_\mathrm{eff}$; the tolerance their two digits",
            run: bto_eltes,
        },
        Case {
            id: "pockels/litao3-x-cut",
            title: r"A uniform field of 0.1 V/µm in x-cut lithium tantalate along c, across a guide along the chip's x (Casson et al.'s unclamped $r_{13}$ = 6.92 and $r_{33}$ = 27.4 pm/V and their indices at 1558 nm): each transverse index's change from the turned tensor against $-\tfrac12 n^3 r E$ (the larger relative difference shown)",
            tier: Tier::Analytic,
            source: r"the index ellipsoid to first order in $E$; J. L. Casson et al., J. Opt. Soc. Am. B 21, 1948 (2004), doi:10.1364/JOSAB.21.001948, Table 2, for the coefficients",
            run: litao3_x_cut,
        },
    ]
}
