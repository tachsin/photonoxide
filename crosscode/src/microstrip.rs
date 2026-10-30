//! A microstrip line: a perfectly conducting strip of zero thickness on a substrate over a
//! ground plane, in a perfectly conducting box, drawn on the same mesh lines for openEMS's 3D
//! FDTD and for photonoxide's quasi-TEM line from its static solver.
//!
//! A strip 20 µm wide on 20 µm of a substrate of ε = 10 (w/h = 1), the ground plane the box's
//! floor, the box ±200 µm across and 200 µm high above the strip, its walls at the ground's
//! potential. photonoxide solves half of it, beside the mirror plane through the strip's
//! centre; openEMS runs a section of it [`crate::cpw::LENGTH`] long with the coplanar line's
//! planes, probes and pulse ([`crate::cpw`]): a soft source of E between the strip and the
//! ground, the voltage from the strip down to the ground at three planes, the current around
//! the strip midway between them (a loop through the substrate and the air, clear of the
//! ground), so that [`crate::cpw::measured`] gives k and Z₀.

use photonoxide::electro::{
    Axis, Boundary, Conductor, Electrostatic, Grading, Grid, Section, Side, Tensor,
};

use crate::cpw::{C, FIRST, LENGTH, Mesh, SOURCE, SPACING, STEP_ALONG};
use crate::openems::{self, Cuboid, Material, Measure, Model, Probe, Source};

/// The current loop's half height about the strip, µm: inside the substrate below.
const LOOP: f64 = 5.0;

/// A microstrip line in a box, µm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Microstrip {
    /// The strip's half width.
    pub half_width: f64,
    /// The substrate's thickness, the ground plane under it.
    pub height: f64,
    /// The substrate's relative permittivity.
    pub permittivity: f64,
    /// The box's half width, and its height above the strip.
    pub reach: f64,
}

impl Microstrip {
    /// w/h = 1 on ε = 10: a strip 20 µm wide on 20 µm, in a box ±200 µm across and 200 µm high.
    pub fn alumina_like() -> Microstrip {
        Microstrip {
            half_width: 10.0,
            height: 20.0,
            permittivity: 10.0,
            reach: 200.0,
        }
    }

    /// The mesh lines across (from the mirror at 0 to the box's side) and in height (from the
    /// ground to the lid).
    ///
    /// # Errors
    ///
    /// When `mesh` isn't a valid grading.
    pub fn axes(&self, mesh: &Mesh) -> photonoxide::Result<(Axis, Axis)> {
        let g = Grading::new(mesh.largest, mesh.smallest, mesh.growth)?;
        let (a, h, r) = (self.half_width, self.height, self.reach);
        Ok((
            Axis::graded(0.0, r, &g, &[a], &[a])?,
            Axis::graded(-h, r, &g, &[0.0], &[0.0])?,
        ))
    }

    fn electrostatic(
        &self,
        across: &Axis,
        height: &Axis,
        permittivity: f64,
    ) -> photonoxide::Result<Electrostatic> {
        let mut section = Section::new("air");
        section.layer(-self.height, 0.0, "substrate")?;
        let mut problem = Electrostatic::new(section, Grid::new(across.clone(), height.clone()));
        problem
            .permittivity("air", Tensor::isotropic(1.0))
            .permittivity("substrate", Tensor::isotropic(permittivity))
            .boundary(Side::Right, Boundary::Fixed(0.0))
            .boundary(Side::Bottom, Boundary::Fixed(0.0))
            .boundary(Side::Top, Boundary::Fixed(0.0))
            .conductor(Conductor::strip([0.0, self.half_width], 0.0, 1.0));
        Ok(problem)
    }

    /// The quasi-TEM index and impedance (Ω) on the mesh lines `across` and `height`, from the
    /// strip's capacitance with the substrate and without it: n = √(C/C₀), Z₀ = 1/(c√(CC₀)).
    ///
    /// # Errors
    ///
    /// When the solve fails.
    pub fn static_line(&self, across: &Axis, height: &Axis) -> photonoxide::Result<(f64, f64)> {
        let c = 2.0
            * self
                .electrostatic(across, height, self.permittivity)?
                .capacitance()?[0][0];
        let c0 = 2.0 * self.electrostatic(across, height, 1.0)?.capacitance()?[0][0];
        Ok(((c / c0).sqrt(), 1.0 / (C * (c * c0).sqrt())))
    }

    /// The openEMS model on the mesh lines `across` (half the line, mirrored for the whole)
    /// and `height`, ended at an energy of `end_energy`.
    pub fn model(&self, across: &Axis, height: &Axis, end_energy: f64) -> Model {
        let (a, h, r) = (self.half_width, self.height, self.reach);
        let half = across.nodes();
        let mut y: Vec<f64> = half.iter().skip(1).rev().map(|v| -v).collect();
        y.extend_from_slice(half);
        let cells = (LENGTH / STEP_ALONG).round() as usize;
        let x: Vec<f64> = (0..=cells).map(|i| i as f64 * STEP_ALONG).collect();
        let at = |x: f64, y0: f64, y1: f64, z0: f64, z1: f64| Cuboid {
            from: [x, y0, z0],
            to: [x, y1, z1],
        };
        let mut probes = Vec::new();
        for k in 0..3 {
            // from the strip down to the ground: the strip's potential
            probes.push(Probe {
                name: format!("v{}", k + 1),
                measure: Measure::Voltage,
                at: at(FIRST + k as f64 * SPACING, 0.0, 0.0, 0.0, -h),
            });
        }
        for k in 0..2 {
            probes.push(Probe {
                name: format!("i{}", k + 1),
                measure: Measure::Current { normal: 0 },
                at: at(
                    FIRST + (k as f64 + 0.5) * SPACING,
                    -2.0 * a,
                    2.0 * a,
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
                boxes: vec![Cuboid {
                    from: [0.0, -r, -h],
                    to: [LENGTH, r, 0.0],
                }],
            }],
            metal: vec![Cuboid {
                from: [0.0, -a, 0.0],
                to: [LENGTH, a, 0.0],
            }],
            sources: vec![Source {
                name: "under_strip".into(),
                direction: [0.0, 0.0, -1.0],
                at: at(SOURCE, -a, a, -h, 0.0),
            }],
            probes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// w/h = 1 on ε = 10: near the closed forms' n ≈ 2.6 and Z₀ ≈ 48 Ω, the box adding a
    /// little capacitance; openEMS's mesh is photonoxide's half mirrored, its probes on it.
    #[test]
    fn the_static_line_and_the_model_share_the_lines() {
        let line = Microstrip::alumina_like();
        let (y, z) = line
            .axes(&Mesh {
                smallest: 0.5,
                growth: 1.1,
                largest: 10.0,
            })
            .unwrap();
        let (n, ohm) = line.static_line(&y, &z).unwrap();
        assert!(
            (n - 2.6).abs() < 0.1 && (ohm - 48.0).abs() < 3.0,
            "{n} {ohm}"
        );
        let m = line.model(&y, &z, 1e-7);
        assert_eq!(m.lines[1].len(), 2 * y.nodes().len() - 1);
        assert_eq!(m.lines[2], z.nodes());
        assert!(m.xml().is_ok());
    }
}
