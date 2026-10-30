//! A lossy line to 300 GHz: a coplanar line of copper electrodes a few skin depths thick on
//! fused silica in a perfectly conducting box, where the quasi-TEM line no longer holds, by
//! openEMS's 3D FDTD with the metal a conductor whose skin the mesh resolves, and by
//! photonoxide's full-wave RF mode ([`photonoxide::electro::full_wave::FullWave`]), on the same
//! mesh lines.
//!
//! The signal 10 µm wide, gaps of 5 µm, grounds 20 µm wide, the copper (5.8 × 10⁷ S/m) 0.5 µm
//! thick, about four skin depths at 300 GHz (0.12 µm), on 10 µm of fused silica (ε 3.8) whose
//! underside is the box's floor, the grounds running to the box's sides (±30 µm) and its lid
//! 50 µm above the substrate, its walls perfect conductors: small enough that its own modes start
//! above 300 GHz. openEMS
//! runs a section 1.2 mm long in 10 µm cells (25 to a guided wavelength at 300 GHz), 8-cell
//! PMLs at both ends, driven across both gaps by a Gaussian pulse of 25 to 325 GHz; the voltage
//! across a gap at three planes 150 µm apart (a quarter of a guided wavelength at 300 GHz, so
//! that the three-plane wavenumber's principal value is the right one) and the current around
//! the signal midway give k = β + iα and Z₀ ([`crate::line`]) at each frequency. photonoxide
//! solves half of the line beside the mirror plane through the signal, its full-wave mode the
//! line's even one, as openEMS's drive excites.

use num_complex::Complex64 as c64;
use photonoxide::electro::full_wave::{FullWave, RfMode};
use photonoxide::electro::{
    Axis, Boundary, Conductor, Electrostatic, Grading, Grid, Section, Side, Tensor,
};
use photonoxide::geometry::{Point, Shape};
use photonoxide::units::Length;

use crate::cpw::Mesh;
use crate::line::{impedance, wavenumber};
use crate::openems::{self, Cuboid, Material, Measure, Model, Probe, Run, Source};

/// Copper's conductivity, S/m.
pub const COPPER: f64 = 5.8e7;
/// Fused silica's static relative permittivity.
pub const SILICA: f64 = 3.8;
/// The frequencies compared, GHz.
pub const FREQUENCIES_GHZ: [f64; 3] = [100.0, 200.0, 300.0];

/// The cells along the line, µm.
const STEP_ALONG: f64 = 10.0;
/// The section's length, µm.
const LENGTH: f64 = 1200.0;
/// The source's plane, µm.
const SOURCE: f64 = 150.0;
/// The first voltage plane, µm.
const FIRST: f64 = 600.0;
/// The voltage planes' spacing, µm: an odd number of cells, the current midway on the dual
/// mesh.
const SPACING: f64 = 150.0;
/// The current loop's extent in height, µm: from inside the substrate to the air.
const LOOP: [f64; 2] = [-2.0, 3.0];

fn rect(x: [f64; 2], y: [f64; 2]) -> Shape {
    Shape::rect(
        Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
        Length::um(x[1] - x[0]),
        Length::um(y[1] - y[0]),
    )
    .expect("a rectangle of positive size")
}

/// The copper coplanar line in its box, µm: across from the signal's centre, up from the
/// substrate's top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LossyLine {
    /// The signal's half width.
    pub signal: f64,
    /// The gaps.
    pub gap: f64,
    /// The grounds' widths.
    pub ground: f64,
    /// The copper's thickness.
    pub metal: f64,
    /// The substrate's thickness.
    pub substrate: f64,
    /// The lid's height over the substrate.
    pub lid: f64,
}

/// What a line's mode gives at one frequency: the index βc/ω, Re Z₀ (Ω) and the attenuation α
/// (Np/m).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Figures {
    /// The frequency, GHz.
    pub frequency_ghz: f64,
    /// The index.
    pub index: f64,
    /// Z₀'s real part, Ω.
    pub ohm: f64,
    /// The attenuation, Np/m.
    pub alpha: f64,
}

impl Figures {
    /// The attenuation in dB/cm.
    pub fn db_per_cm(&self) -> f64 {
        self.alpha * 20.0 / std::f64::consts::LN_10 / 100.0
    }
}

impl LossyLine {
    /// The line above.
    pub fn copper_on_silica() -> LossyLine {
        LossyLine {
            signal: 5.0,
            gap: 5.0,
            ground: 20.0,
            metal: 0.5,
            substrate: 10.0,
            lid: 50.0,
        }
    }

    /// The box's half width: the grounds' outer edge.
    pub fn reach(&self) -> f64 {
        self.inner() + self.ground
    }

    fn inner(&self) -> f64 {
        self.signal + self.gap
    }

    /// The mesh lines across (from the mirror at 0 to the box's side) and in height (from the
    /// box's floor to its lid), fine at the copper's faces.
    ///
    /// # Errors
    ///
    /// When `mesh` isn't a valid grading.
    pub fn axes(&self, mesh: &Mesh) -> photonoxide::Result<(Axis, Axis)> {
        let g = Grading::new(mesh.largest, mesh.smallest, mesh.growth)?;
        let across = [self.signal, self.inner(), self.inner() + self.ground];
        let up = [0.0, self.metal];
        Ok((
            Axis::graded(0.0, self.reach(), &g, &across, &across)?,
            Axis::graded(-self.substrate, self.lid, &g, &up, &up)?,
        ))
    }

    /// The electrostatic problem the full-wave mode is solved on: the signal (conductor 0) and
    /// the ground, the box's walls at the ground's potential, the mirror insulated.
    fn problem(&self, across: &Axis, height: &Axis) -> photonoxide::Result<Electrostatic> {
        let mut section = Section::new("air");
        section.layer(-self.substrate, 0.0, "silica")?;
        let mut problem = Electrostatic::new(section, Grid::new(across.clone(), height.clone()));
        problem
            .permittivity("air", Tensor::isotropic(1.0))
            .permittivity("silica", Tensor::isotropic(SILICA))
            .boundary(Side::Right, Boundary::Fixed(0.0))
            .boundary(Side::Bottom, Boundary::Fixed(0.0))
            .boundary(Side::Top, Boundary::Fixed(0.0))
            .conductor(Conductor::shape(
                rect([-1.0, self.signal], [0.0, self.metal]),
                1.0,
            ))
            .conductor(Conductor::shape(
                rect(
                    [self.inner(), self.inner() + self.ground],
                    [0.0, self.metal],
                ),
                0.0,
            ));
        Ok(problem)
    }

    /// photonoxide's full-wave figures at each of [`FREQUENCIES_GHZ`] on the mesh lines
    /// `across` and `height`, the metal copper.
    ///
    /// # Errors
    ///
    /// When a solve fails.
    pub fn photonoxide(&self, across: &Axis, height: &Axis) -> photonoxide::Result<Vec<Figures>> {
        let problem = self.problem(across, height)?;
        let solver = FullWave::new(0, COPPER)?.mirrored();
        // the quasi-TEM index of a line on silica, about √((1 + 3.8)/2)
        let near = ((1.0 + SILICA) / 2.0).sqrt();
        FREQUENCIES_GHZ
            .iter()
            .map(|&f| {
                let m: RfMode = solver.solve(&problem, f * 1e9, near)?;
                Ok(Figures {
                    frequency_ghz: f,
                    index: m.index(),
                    ohm: m.impedance.re,
                    alpha: m.attenuation(),
                })
            })
            .collect()
    }

    /// The openEMS model on the mesh lines `across` (half the line, mirrored for the whole)
    /// and `height`, ended at an energy of `end_energy`.
    pub fn model(&self, across: &Axis, height: &Axis, end_energy: f64) -> Model {
        let r = self.reach();
        let half = across.nodes();
        let mut y: Vec<f64> = half.iter().skip(1).rev().map(|v| -v).collect();
        y.extend_from_slice(half);
        let cells = (LENGTH / STEP_ALONG).round() as usize;
        let x: Vec<f64> = (0..=cells).map(|i| i as f64 * STEP_ALONG).collect();
        let along = |y0: f64, y1: f64, z0: f64, z1: f64| Cuboid {
            from: [0.0, y0, z0],
            to: [LENGTH, y1, z1],
        };
        let at = |x: f64, y0: f64, y1: f64, z0: f64, z1: f64| Cuboid {
            from: [x, y0, z0],
            to: [x, y1, z1],
        };
        let (s, inner, t) = (self.signal, self.inner(), self.metal);
        let outer = inner + self.ground;
        let mut probes = Vec::new();
        for k in 0..3 {
            probes.push(Probe {
                name: format!("v{}", k + 1),
                measure: Measure::Voltage,
                at: at(FIRST + k as f64 * SPACING, s, inner, 0.5 * t, 0.5 * t),
            });
        }
        let middle = s + 0.5 * self.gap;
        for k in 0..2 {
            probes.push(Probe {
                name: format!("i{}", k + 1),
                measure: Measure::Current { normal: 0 },
                at: at(
                    FIRST + (k as f64 + 0.5) * SPACING,
                    -middle,
                    middle,
                    LOOP[0],
                    LOOP[1],
                ),
            });
        }
        Model {
            unit_m: 1e-6,
            lines: [x, y, height.nodes().to_vec()],
            boundaries: [
                openems::Boundary::Pml(8),
                openems::Boundary::Pml(8),
                openems::Boundary::Pec,
                openems::Boundary::Pec,
                openems::Boundary::Pec,
                openems::Boundary::Pec,
            ],
            pulse: [175e9, 150e9],
            max_steps: 20_000_000,
            end_energy,
            materials: vec![
                Material {
                    name: "silica".into(),
                    epsilon: [SILICA; 3],
                    kappa: 0.0,
                    boxes: vec![along(-r, r, -self.substrate, 0.0)],
                },
                Material {
                    name: "copper".into(),
                    epsilon: [1.0; 3],
                    kappa: COPPER,
                    boxes: vec![
                        along(-s, s, 0.0, t),
                        along(inner, outer, 0.0, t),
                        along(-outer, -inner, 0.0, t),
                    ],
                },
            ],
            metal: Vec::new(),
            sources: vec![
                Source {
                    name: "gap_right".into(),
                    direction: [0.0, 1.0, 0.0],
                    at: at(SOURCE, s, inner, 0.0, t),
                },
                Source {
                    name: "gap_left".into(),
                    direction: [0.0, -1.0, 0.0],
                    at: at(SOURCE, -inner, -s, 0.0, t),
                },
            ],
            probes,
        }
    }
}

/// openEMS's figures at each of [`FREQUENCIES_GHZ`] from `run`'s probes.
///
/// # Errors
///
/// When a probe's file is missing or malformed.
pub fn measured(run: &Run) -> Result<Vec<Figures>, String> {
    let v = [run.probe("v1")?, run.probe("v2")?, run.probe("v3")?];
    let i = [run.probe("i1")?, run.probe("i2")?];
    let d = SPACING * 1e-6;
    Ok(FREQUENCIES_GHZ
        .iter()
        .map(|&f| {
            let hz = f * 1e9;
            let vf = v.each_ref().map(|s| s.spectrum(hz));
            let jf = i.each_ref().map(|s| s.spectrum(hz));
            let k = wavenumber(vf, d);
            let z: [c64; 2] = [
                impedance([vf[0], vf[1]], jf[0], k, d),
                impedance([vf[1], vf[2]], jf[1], k, d),
            ];
            Figures {
                frequency_ghz: f,
                index: k.re * crate::cpw::C / (2.0 * std::f64::consts::PI * hz),
                ohm: 0.5 * (z[0].re + z[1].re),
                alpha: k.im,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The full-wave mode on a coarse mesh: an index near √((1 + 3.8)/2), a loss of some dB/cm
    /// growing with frequency; openEMS's planes on its mesh lines, the current on the dual mesh.
    #[test]
    fn the_full_wave_mode_and_the_model_share_the_lines() {
        let line = LossyLine::copper_on_silica();
        let (y, z) = line
            .axes(&Mesh {
                smallest: 0.12,
                growth: 1.2,
                largest: 5.0,
            })
            .unwrap();
        let f = line.photonoxide(&y, &z).unwrap();
        eprintln!("{f:?}");
        assert!((f[2].index - 1.6).abs() < 0.3, "{f:?}");
        assert!(f[2].alpha > f[0].alpha && f[0].alpha > 0.0, "{f:?}");
        let m = line.model(&y, &z, 1e-7);
        let on = |x: f64| m.lines[0].iter().any(|&l| (l - x).abs() < 1e-9);
        assert!(on(FIRST) && on(FIRST + 2.0 * SPACING) && !on(FIRST + 0.5 * SPACING));
        assert!(
            m.xml()
                .unwrap()
                .contains("Kappa=\"58000000,58000000,58000000\"")
        );
    }
}
