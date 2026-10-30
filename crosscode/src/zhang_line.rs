//! Zhang et al.'s thin-film lithium niobate line (Y. Zhang et al., Photon. Res. 10, 2380
//! (2022), doi:10.1364/PRJ.468518, Section 2), its full stack, in a perfectly conducting box,
//! drawn on the same mesh lines for openEMS's 3D FDTD and for photonoxide's quasi-TEM line from
//! its static solver.
//!
//! A ground–signal–ground line: a signal 20 µm wide, gaps of 5 µm, grounds 100 µm wide (not
//! given), the electrodes 0.83 µm thick (800 nm of copper under 30 nm of gold), perfect
//! conductors here, on an x-cut film 600 nm thick etched 300 nm to a rib 1.2 µm wide in each
//! gap (its sidewalls upright, so that the mesh lines follow them), on 2 µm of thermal oxide (ε
//! 3.9) over 500 µm of silicon (ε 11.7), under 0.6 µm of PECVD oxide (ε 5.5, the paper's) and
//! air. The film's clamped static permittivity is diagonal: 26.2 across (the crystal's c axis)
//! and 45.5 up and along the line. The box is as wide as the line, the grounds running to its
//! sides (so that they are the box's potential, and no charge is left on them), its floor
//! under the silicon and its lid 200 µm above the film. photonoxide solves half
//! of it beside the mirror plane through the signal's centre; openEMS runs a section
//! [`crate::cpw::LENGTH`] long with the coplanar line's planes, probes and pulse, driven across
//! both gaps, the voltage across one gap and the current around the signal.

use photonoxide::electro::{
    Axis, Boundary, Conductor, Electrostatic, Grading, Grid, Section, Side, Tensor,
};
use photonoxide::geometry::{Point, Shape};
use photonoxide::units::Length;

use crate::cpw::{C, FIRST, LENGTH, Mesh, SOURCE, SPACING, STEP_ALONG};
use crate::openems::{self, Cuboid, Material, Measure, Model, Probe, Source};

/// Lithium niobate's clamped static permittivity across the line (along c) and up (and along
/// the line), x-cut.
pub const LN: [f64; 2] = [26.2, 45.5];
/// The thermal oxide's, silicon's and the PECVD cladding's.
pub const OXIDE: f64 = 3.9;
/// Silicon's.
pub const SILICON: f64 = 11.7;
/// The cladding's (the paper's).
pub const CLADDING: f64 = 5.5;

/// The current loop's extent in height, µm: from inside the buried oxide to the air.
const LOOP: [f64; 2] = [-1.0, 3.0];

fn rect(x: [f64; 2], y: [f64; 2]) -> Shape {
    Shape::rect(
        Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
        Length::um(x[1] - x[0]),
        Length::um(y[1] - y[0]),
    )
    .expect("a rectangle of positive size")
}

/// Zhang et al.'s line in a box, µm: across from the signal's centre, up from the film's
/// bottom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZhangLine {
    /// The signal's half width.
    pub signal: f64,
    /// The gaps.
    pub gap: f64,
    /// The grounds' widths.
    pub ground: f64,
    /// The electrodes' thickness.
    pub metal: f64,
    /// The film's thickness.
    pub film: f64,
    /// The slab left beside the ribs.
    pub slab: f64,
    /// The ribs' width.
    pub rib: f64,
    /// The buried oxide's thickness.
    pub buried: f64,
    /// The silicon's thickness.
    pub silicon: f64,
    /// The cladding's thickness over the slab.
    pub clad: f64,
    /// The box's lid's height over the film's bottom.
    pub lid: f64,
}

impl ZhangLine {
    /// The line above.
    pub fn zhang() -> ZhangLine {
        ZhangLine {
            signal: 10.0,
            gap: 5.0,
            ground: 100.0,
            metal: 0.83,
            film: 0.6,
            slab: 0.3,
            rib: 1.2,
            buried: 2.0,
            silicon: 500.0,
            clad: 0.6,
            lid: 200.0,
        }
    }

    /// The box's half width: the grounds' outer edge.
    pub fn reach(&self) -> f64 {
        self.signal + self.gap + self.ground
    }

    fn floor(&self) -> f64 {
        -self.buried - self.silicon
    }

    /// The rib in the right gap, across.
    fn rib_span(&self) -> [f64; 2] {
        let c = self.signal + 0.5 * self.gap;
        [c - 0.5 * self.rib, c + 0.5 * self.rib]
    }

    fn top(&self) -> f64 {
        self.slab + self.metal
    }

    /// The mesh lines across (from the mirror at 0 to the box's side) and in height (from the
    /// box's floor to its lid).
    ///
    /// # Errors
    ///
    /// When `mesh` isn't a valid grading.
    pub fn axes(&self, mesh: &Mesh) -> photonoxide::Result<(Axis, Axis)> {
        let g = Grading::new(mesh.largest, mesh.smallest, mesh.growth)?;
        let [r0, r1] = self.rib_span();
        let inner = self.signal + self.gap;
        let across = [self.signal, r0, r1, inner, inner + self.ground];
        let up = [0.0, self.slab, self.film, self.slab + self.clad, self.top()];
        let mut lines = up.to_vec();
        lines.push(-self.buried);
        Ok((
            Axis::graded(0.0, self.reach(), &g, &across, &across)?,
            Axis::graded(self.floor(), self.lid, &g, &lines, &up)?,
        ))
    }

    fn electrostatic(
        &self,
        across: &Axis,
        height: &Axis,
        vacuum: bool,
    ) -> photonoxide::Result<Electrostatic> {
        let mut section = Section::new("air");
        section
            .layer(self.floor(), -self.buried, "Si")?
            .layer(-self.buried, 0.0, "box")?
            .layer(self.slab, self.slab + self.clad, "clad")?
            .layer(0.0, self.slab, "LN")?;
        section.draw(rect(self.rib_span(), [self.slab, self.film]), "LN");
        let mut problem = Electrostatic::new(section, Grid::new(across.clone(), height.clone()));
        let eps = |t: Tensor| if vacuum { Tensor::isotropic(1.0) } else { t };
        let inner = self.signal + self.gap;
        problem
            .permittivity("air", Tensor::isotropic(1.0))
            .permittivity("Si", eps(Tensor::isotropic(SILICON)))
            .permittivity("box", eps(Tensor::isotropic(OXIDE)))
            .permittivity("clad", eps(Tensor::isotropic(CLADDING)))
            .permittivity("LN", eps(Tensor::diagonal(LN[0], LN[1])))
            .boundary(Side::Right, Boundary::Fixed(0.0))
            .boundary(Side::Bottom, Boundary::Fixed(0.0))
            .boundary(Side::Top, Boundary::Fixed(0.0))
            .conductor(Conductor::shape(
                rect([-1.0, self.signal], [self.slab, self.top()]),
                1.0,
            ))
            .conductor(Conductor::shape(
                rect([inner, inner + self.ground], [self.slab, self.top()]),
                0.0,
            ));
        Ok(problem)
    }

    /// The quasi-TEM index and impedance (Ω) on the mesh lines `across` and `height`, from the
    /// signal's capacitance with the dielectrics and without them: n = √(C/C₀),
    /// Z₀ = 1/(c√(CC₀)).
    ///
    /// # Errors
    ///
    /// When the solve fails.
    pub fn static_line(&self, across: &Axis, height: &Axis) -> photonoxide::Result<(f64, f64)> {
        let c = 2.0 * self.electrostatic(across, height, false)?.capacitance()?[0][0];
        let c0 = 2.0 * self.electrostatic(across, height, true)?.capacitance()?[0][0];
        Ok(((c / c0).sqrt(), 1.0 / (C * (c * c0).sqrt())))
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
        let [r0, r1] = self.rib_span();
        let (s, inner) = (self.signal, self.signal + self.gap);
        let (slab, film, clad, top) = (self.slab, self.film, self.slab + self.clad, self.top());
        let materials = vec![
            Material {
                name: "silicon".into(),
                epsilon: [SILICON; 3],
                kappa: 0.0,
                boxes: vec![along(-r, r, self.floor(), -self.buried)],
            },
            Material {
                name: "oxide".into(),
                epsilon: [OXIDE; 3],
                kappa: 0.0,
                boxes: vec![along(-r, r, -self.buried, 0.0)],
            },
            Material {
                name: "film".into(),
                // along the line, across (c), up
                epsilon: [LN[1], LN[0], LN[1]],
                kappa: 0.0,
                boxes: vec![
                    along(-r, r, 0.0, slab),
                    along(r0, r1, slab, film),
                    along(-r1, -r0, slab, film),
                ],
            },
            Material {
                name: "cladding".into(),
                epsilon: [CLADDING; 3],
                kappa: 0.0,
                // around the ribs, which are the film's
                boxes: vec![
                    along(-r, -r1, slab, film),
                    along(-r0, r0, slab, film),
                    along(r1, r, slab, film),
                    along(-r, r, film, clad),
                ],
            },
        ];
        let mut probes = Vec::new();
        for k in 0..3 {
            // across the right gap, signal to ground, at the ribs' top
            probes.push(Probe {
                name: format!("v{}", k + 1),
                measure: Measure::Voltage,
                at: at(FIRST + k as f64 * SPACING, s, inner, film, film),
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
            pulse: [30e9, 25e9],
            max_steps: 20_000_000,
            end_energy,
            materials,
            metal: vec![
                along(-s, s, slab, top),
                along(inner, inner + self.ground, slab, top),
                along(-inner - self.ground, -inner, slab, top),
            ],
            sources: vec![
                Source {
                    name: "gap_right".into(),
                    direction: [0.0, 1.0, 0.0],
                    at: at(SOURCE, s, inner, slab, top),
                },
                Source {
                    name: "gap_left".into(),
                    direction: [0.0, -1.0, 0.0],
                    at: at(SOURCE, -inner, -s, slab, top),
                },
            ],
            probes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Near photonoxide's own Zhang line (its validation case, open sides): an RF index about
    /// 2.2 and Z₀ about 40 Ω; openEMS's mesh is photonoxide's half mirrored.
    #[test]
    fn the_static_line_and_the_model_share_the_lines() {
        let line = ZhangLine::zhang();
        let (y, z) = line
            .axes(&Mesh {
                smallest: 1.0,
                growth: 1.1,
                largest: 10.0,
            })
            .unwrap();
        let (n, ohm) = line.static_line(&y, &z).unwrap();
        assert!(
            (n - 2.2).abs() < 0.3 && (ohm - 40.0).abs() < 10.0,
            "{n} {ohm}"
        );
        let m = line.model(&y, &z, 1e-7);
        assert_eq!(m.lines[1].len(), 2 * y.nodes().len() - 1);
        assert!(m.xml().is_ok());
    }
}
