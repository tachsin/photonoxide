//! The shared RF case: a coplanar waveguide of perfectly conducting strips of zero thickness on
//! a finite substrate, in a perfectly conducting box, drawn on the same mesh lines for openEMS's
//! 3D FDTD and for photonoxide's quasi-TEM line ([`photonoxide::electro::line::QuasiTem`]).
//!
//! The line is Ghione and Naldi's (G. Ghione, C. Naldi, Electron. Lett. 20, 179 (1984),
//! doi:10.1049/el:19840120): a signal 2a = 20 µm wide, gaps to the grounds at b = 20 µm, on a
//! substrate of ε = 10 as thick as b, air below it and above the strips, the grounds out to the
//! box's sides. The box is ±200 µm across and from −200 to 200 µm in height, its walls at the
//! grounds' potential: photonoxide's window is the box (half of it, beside its mirror through
//! the signal's centre), and openEMS's sides across are perfect conductors, the grounds
//! touching them.
//!
//! openEMS runs a section 4 mm long, 25 µm cells along it, absorbing layers (PML) of 8 cells at
//! both ends that the line runs into. A soft source of E across both gaps (the even, coplanar
//! mode), 700 µm before the first plane, launches a Gaussian pulse of 5 to 55 GHz (openEMS's
//! f₀ = 30 GHz, f_c = 25 GHz). The voltage across a gap is taken at three planes 1025 µm apart
//! (41 cells, on the primary mesh; the last 475 µm before the absorbing layer), the current on
//! the signal midway between them (on the dual mesh, a loop around the signal through the
//! gaps); [`crate::line`] turns them into k and Z₀ at each frequency, whatever the ends
//! reflect.

use num_complex::Complex64 as c64;
use photonoxide::electro::EPSILON_0;
use photonoxide::electro::line::QuasiTem;
use photonoxide::electro::{
    Axis, Boundary, Conductor, Electrostatic, Grading, Grid, Section, Side, Tensor,
};

use crate::line::{impedance, wavenumber};
use crate::openems::{self, Cuboid, Material, Measure, Model, Probe, Run, Source};

/// The speed of light, m/s.
pub const C: f64 = 299_792_458.0;

/// The frequencies the line is measured at, GHz.
pub const FREQUENCIES_GHZ: [f64; 5] = [10.0, 20.0, 30.0, 40.0, 50.0];

/// The cells along the line, µm.
pub const STEP_ALONG: f64 = 25.0;
/// The section's length, µm.
pub const LENGTH: f64 = 4000.0;
/// The source's plane, µm.
pub const SOURCE: f64 = 600.0;
/// The first voltage plane, µm.
pub const FIRST: f64 = 1300.0;
/// The voltage planes' spacing, µm: an odd number of cells, so that the current midway is on
/// the dual mesh.
pub const SPACING: f64 = 1025.0;
/// The current loop's half height about the strips, µm.
const LOOP: f64 = 5.0;

/// A coplanar waveguide in a box, µm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cpw {
    /// The signal's half width a.
    pub half_width: f64,
    /// The grounds' inner edge b.
    pub ground_edge: f64,
    /// The substrate's relative permittivity.
    pub permittivity: f64,
    /// The substrate's thickness under the strips.
    pub thickness: f64,
    /// The box's half width and half height, the strips at its middle.
    pub reach: f64,
}

/// The mesh across the line: cells from `smallest` at the strips' edges and in their plane,
/// growing by `growth`, to `largest` (µm).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mesh {
    /// The smallest cell, µm.
    pub smallest: f64,
    /// The growth from one cell to the next.
    pub growth: f64,
    /// The largest cell, µm.
    pub largest: f64,
}

/// What openEMS's run gives at one frequency.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measured {
    /// The frequency, GHz.
    pub frequency_ghz: f64,
    /// The wavenumber k = β + iα, 1/m.
    pub wavenumber: c64,
    /// Z₀ from the first two planes and from the last two, Ω.
    pub impedance: [c64; 2],
}

impl Measured {
    /// The RF index, βc/ω.
    pub fn index(&self) -> f64 {
        self.wavenumber.re * C / (2.0 * std::f64::consts::PI * self.frequency_ghz * 1e9)
    }

    /// Z₀'s real part, the two estimates' mean, Ω.
    pub fn ohm(&self) -> f64 {
        0.5 * (self.impedance[0].re + self.impedance[1].re)
    }
}

impl Cpw {
    /// Ghione and Naldi's line, k = a/b = 0.5 on ε = 10 with h = b, in a box of ±200 µm.
    pub fn ghione_naldi() -> Cpw {
        Cpw {
            half_width: 10.0,
            ground_edge: 20.0,
            permittivity: 10.0,
            thickness: 20.0,
            reach: 200.0,
        }
    }

    /// The mesh lines across (from the mirror at 0 to the box's side) and in height.
    ///
    /// # Errors
    ///
    /// When `mesh` isn't a valid grading.
    pub fn axes(&self, mesh: &Mesh) -> photonoxide::Result<(Axis, Axis)> {
        let g = Grading::new(mesh.largest, mesh.smallest, mesh.growth)?;
        let (a, b, h, r) = (
            self.half_width,
            self.ground_edge,
            self.thickness,
            self.reach,
        );
        Ok((
            Axis::graded(0.0, r, &g, &[a, b], &[a, b])?,
            Axis::graded(-r, r, &g, &[-h, 0.0], &[0.0])?,
        ))
    }

    /// photonoxide's quasi-TEM line on the mesh lines `across` (half the line, from its mirror)
    /// and `height`. It needs cells fine enough at the strips' edges for their current
    /// ([`QuasiTem::mirrored`]); [`Cpw::static_line`] gives the index and impedance on any.
    ///
    /// # Errors
    ///
    /// When the solve fails.
    pub fn quasi_tem(&self, across: &Axis, height: &Axis) -> photonoxide::Result<QuasiTem> {
        QuasiTem::mirrored(&self.electrostatic(across, height, self.permittivity)?, 0)
    }

    /// The quasi-TEM index and impedance (Ω) on the mesh lines `across` and `height`, from the
    /// signal's capacitance with the substrate and without it: n = √(C/C₀), Z₀ = 1/(c√(CC₀)),
    /// as [`QuasiTem`] has them.
    ///
    /// # Errors
    ///
    /// When the solve fails.
    pub fn static_line(&self, across: &Axis, height: &Axis) -> photonoxide::Result<(f64, f64)> {
        // the whole line's, twice the half's beside its mirror
        let c = 2.0
            * self
                .electrostatic(across, height, self.permittivity)?
                .capacitance()?[0][0];
        let c0 = 2.0 * self.electrostatic(across, height, 1.0)?.capacitance()?[0][0];
        Ok(((c / c0).sqrt(), 1.0 / (C * (c * c0).sqrt())))
    }

    /// Elmer's case on the same mesh lines and half box: StatElecSolver's capacitance of the
    /// signal (its boundary at 1 V, the grounds and the box's walls at 0, the mirror plane left
    /// free, which is its own symmetry) with the substrate's permittivity `permittivity`, the
    /// whole line's being twice the half's ([`crate::elmer::capacitance`]).
    ///
    /// # Errors
    ///
    /// When a strip or a wall isn't on the mesh lines.
    pub fn elmer(
        &self,
        across: &Axis,
        height: &Axis,
        permittivity: f64,
    ) -> Result<crate::elmer::Case, String> {
        let (a, b, h, r) = (
            self.half_width,
            self.ground_edge,
            self.thickness,
            self.reach,
        );
        let mut mesh = crate::elmer::Mesh::new(across.nodes(), height.nodes(), |_, y| {
            if y > -h && y < 0.0 { 1 } else { 2 }
        });
        mesh.along_x(1, 0.0, [0.0, a])?
            .along_x(2, 0.0, [b, r])?
            .along_x(2, -r, [0.0, r])?
            .along_x(2, r, [0.0, r])?
            .along_y(2, r, [-r, r])?;
        let sif = format!(
            "Header\n  Mesh DB \".\" \"mesh\"\nEnd\n\n\
             Simulation\n  Coordinate System = Cartesian 2D\n  Coordinate Scaling = 1.0e-6\n  \
             Simulation Type = Steady State\n  Steady State Max Iterations = 1\n  \
             Output Intervals = 0\nEnd\n\n\
             Constants\n  Permittivity Of Vacuum = {EPSILON_0:e}\nEnd\n\n\
             Body 1\n  Name = \"substrate\"\n  Equation = 1\n  Material = 1\nEnd\n\n\
             Body 2\n  Name = \"air\"\n  Equation = 1\n  Material = 2\nEnd\n\n\
             Equation 1\n  Active Solvers(1) = 1\nEnd\n\n\
             Solver 1\n  Equation = Stat Elec Solver\n  \
             Procedure = \"StatElecSolve\" \"StatElecSolver\"\n  Variable = Potential\n  \
             Variable DOFs = 1\n  Calculate Electric Energy = True\n  \
             Calculate Capacitance Matrix = True\n  \
             Capacitance Matrix Filename = \"cmatrix.dat\"\n  \
             Linear System Solver = Direct\n  Linear System Direct Method = Umfpack\nEnd\n\n\
             Material 1\n  Relative Permittivity = {permittivity:e}\nEnd\n\n\
             Material 2\n  Relative Permittivity = 1.0\nEnd\n\n\
             Boundary Condition 1\n  Name = \"signal\"\n  Target Boundaries(1) = 1\n  \
             Capacitance Body = 1\nEnd\n\n\
             Boundary Condition 2\n  Name = \"grounds and walls\"\n  Target Boundaries(1) = 2\n  \
             Potential = 0.0\nEnd\n"
        );
        Ok(crate::elmer::Case { mesh, sif })
    }

    fn electrostatic(
        &self,
        across: &Axis,
        height: &Axis,
        permittivity: f64,
    ) -> photonoxide::Result<Electrostatic> {
        let (a, b, h, r) = (
            self.half_width,
            self.ground_edge,
            self.thickness,
            self.reach,
        );
        let mut section = Section::new("air");
        section.layer(-h, 0.0, "substrate")?;
        let grid = Grid::new(across.clone(), height.clone());
        let mut problem = Electrostatic::new(section, grid);
        problem
            .permittivity("air", Tensor::isotropic(1.0))
            .permittivity("substrate", Tensor::isotropic(permittivity))
            .boundary(Side::Right, Boundary::Fixed(0.0))
            .boundary(Side::Bottom, Boundary::Fixed(0.0))
            .boundary(Side::Top, Boundary::Fixed(0.0))
            .conductor(Conductor::strip([0.0, a], 0.0, 1.0))
            .conductor(Conductor::strip([b, r], 0.0, 0.0));
        Ok(problem)
    }

    /// The openEMS model on the mesh lines `across` (half the line, mirrored for the whole)
    /// and `height`, ended at an energy of `end_energy`.
    pub fn model(&self, across: &Axis, height: &Axis, end_energy: f64) -> Model {
        let (a, b, h, r) = (
            self.half_width,
            self.ground_edge,
            self.thickness,
            self.reach,
        );
        let half = across.nodes();
        let mut y: Vec<f64> = half.iter().skip(1).rev().map(|v| -v).collect();
        y.extend_from_slice(half);
        let cells = (LENGTH / STEP_ALONG).round() as usize;
        let x: Vec<f64> = (0..=cells).map(|i| i as f64 * STEP_ALONG).collect();
        let along = |y0: f64, y1: f64, z0: f64, z1: f64| Cuboid {
            from: [0.0, y0, z0],
            to: [LENGTH, y1, z1],
        };
        let across_at = |x: f64, y0: f64, y1: f64, z0: f64, z1: f64| Cuboid {
            from: [x, y0, z0],
            to: [x, y1, z1],
        };
        let mut probes = Vec::new();
        for k in 0..3 {
            probes.push(Probe {
                name: format!("v{}", k + 1),
                measure: Measure::Voltage,
                at: across_at(FIRST + k as f64 * SPACING, a, b, 0.0, 0.0),
            });
        }
        let middle = 0.5 * (a + b);
        for k in 0..2 {
            probes.push(Probe {
                name: format!("i{}", k + 1),
                measure: Measure::Current { normal: 0 },
                at: across_at(
                    FIRST + (k as f64 + 0.5) * SPACING,
                    -middle,
                    middle,
                    -LOOP,
                    LOOP,
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
            pulse: [30e9, 25e9],
            max_steps: 5_000_000,
            end_energy,
            materials: vec![Material {
                name: "substrate".into(),
                epsilon: [self.permittivity; 3],
                kappa: 0.0,
                boxes: vec![along(-r, r, -h, 0.0)],
            }],
            metal: vec![
                along(-a, a, 0.0, 0.0),
                along(b, r, 0.0, 0.0),
                along(-r, -b, 0.0, 0.0),
            ],
            sources: vec![
                Source {
                    name: "gap_right".into(),
                    direction: [0.0, 1.0, 0.0],
                    at: across_at(SOURCE, a, b, 0.0, 0.0),
                },
                Source {
                    name: "gap_left".into(),
                    direction: [0.0, -1.0, 0.0],
                    at: across_at(SOURCE, -b, -a, 0.0, 0.0),
                },
            ],
            probes,
        }
    }
}

/// k and Z₀ at each of `frequencies_ghz` from `run`'s probes.
///
/// # Errors
///
/// When a probe's file is missing or malformed.
pub fn measured(run: &Run, frequencies_ghz: &[f64]) -> Result<Vec<Measured>, String> {
    let v = [run.probe("v1")?, run.probe("v2")?, run.probe("v3")?];
    let i = [run.probe("i1")?, run.probe("i2")?];
    let d = SPACING * 1e-6;
    Ok(frequencies_ghz
        .iter()
        .map(|&f| {
            let hz = f * 1e9;
            let vf = v.each_ref().map(|s| s.spectrum(hz));
            let jf = i.each_ref().map(|s| s.spectrum(hz));
            let k = wavenumber(vf, d);
            Measured {
                frequency_ghz: f,
                wavenumber: k,
                impedance: [
                    impedance([vf[0], vf[1]], jf[0], k, d),
                    impedance([vf[1], vf[2]], jf[1], k, d),
                ],
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The capacitances' index and impedance are the quasi-TEM line's, on a grid fine enough
    /// for its current.
    #[test]
    fn the_static_line_is_the_quasi_tem_lines() {
        let cpw = Cpw::ghione_naldi();
        let (y, z) = cpw
            .axes(&Mesh {
                smallest: 0.02,
                growth: 1.1,
                largest: 10.0,
            })
            .unwrap();
        let (n, ohm) = cpw.static_line(&y, &z).unwrap();
        let line = cpw.quasi_tem(&y, &z).unwrap();
        assert!((n - line.index()).abs() < 1e-12 * n, "{n} {}", line.index());
        assert!((ohm - line.impedance()).abs() < 1e-12 * ohm);
        // near Ghione and Naldi's open line (54.4 Ω, n 2.216), the box adding a little C
        assert!(
            (n - 2.217).abs() < 2e-3 && (ohm - 54.2).abs() < 0.3,
            "{n} {ohm}"
        );
    }

    /// openEMS's mesh is photonoxide's half mirrored; its voltage planes are mesh lines and its
    /// current loops midway between two, on the dual mesh.
    #[test]
    fn the_openems_model_shares_the_lines_and_places_the_probes() {
        let cpw = Cpw::ghione_naldi();
        let (y, z) = cpw
            .axes(&Mesh {
                smallest: 1.0,
                growth: 1.1,
                largest: 10.0,
            })
            .unwrap();
        let m = cpw.model(&y, &z, 1e-8);
        let half = y.nodes();
        assert_eq!(m.lines[1].len(), 2 * half.len() - 1);
        assert_eq!(&m.lines[1][half.len() - 1..], half);
        assert_eq!(m.lines[2], z.nodes());
        let along = &m.lines[0];
        let on_line = |x: f64| along.iter().any(|&l| (l - x).abs() < 1e-9);
        for p in &m.probes {
            let x = p.at.from[0];
            match p.measure {
                Measure::Voltage => assert!(on_line(x), "{} at {x}", p.name),
                Measure::Current { normal } => {
                    assert_eq!(normal, 0);
                    assert!(
                        !on_line(x) && on_line(x - 0.5 * STEP_ALONG),
                        "{} at {x}",
                        p.name
                    );
                }
            }
        }
        // the planes inside the line, clear of the source and of the 8-cell layers
        let last = FIRST + 2.0 * SPACING;
        assert!(FIRST > SOURCE && last < LENGTH - 8.0 * STEP_ALONG);
        assert!(m.xml().is_ok());
    }
}
