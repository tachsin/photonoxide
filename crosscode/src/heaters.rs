//! Two heaters side by side, the thermal crosstalk: Jacques et al.'s N++ silicon heater (M.
//! Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456, Section 2.2, Fig. 1
//! and Table 3) beside a second one, the first on and the second off, by photonoxide's heat
//! solver ([`photonoxide::thermal::Heat`]) and by Elmer's HeatSolver on the same mesh lines.
//!
//! Each heater is a rib 500 nm wide and 220 nm high on a slab 90 nm thick and 2.1 µm wide, with
//! a strip of N++ silicon 1 µm wide beyond each side of the slab; y = 0 at the buried oxide's
//! top. They are 9.1 µm apart, centre to centre, so that 5 µm separate their facing N++ strips
//! (the paper's gap of 5 µm, Fig. 9). Around them, as the paper's model has it: 2 µm of buried
//! oxide on silicon, the oxide's top 3.32 µm up with 10 W/(m²·K) to air at 0, the window 18 µm
//! high with its bottom, in the silicon, held at 0, and its sides insulated, 20 µm beyond each
//! guide. The first heater dissipates the paper's P_π, 22.8 mW over 320 µm (71.25 W/m), in its
//! two N++ strips. The figure compared is the second rib's mean rise.

use photonoxide::geometry::{Point, Shape};
use photonoxide::thermal::{Axis, Boundary, Grading, Grid, Heat, Section, Side, Tensor};
use photonoxide::units::Length;

use crate::elmer::{Case, Mesh as ElmerMesh, save_points};

/// Jacques et al.'s Table 3, W/(m·K): the oxide, the substrate, the 220 nm rib, the 90 nm slab
/// and the N++ strips.
pub const K_OXIDE: f64 = 1.38;
/// The substrate's.
pub const K_SUBSTRATE: f64 = 148.0;
/// The rib's (220 nm of silicon).
pub const K_RIB: f64 = 90.0;
/// The slab's (90 nm of silicon).
pub const K_SLAB: f64 = 55.0;
/// The N++ silicon's.
pub const K_NPP: f64 = 25.0;

/// The heater's power, W/m: the paper's P_π, 22.8 mW, over its 320 µm.
pub const WATTS_PER_METRE: f64 = 22.8e-3 / 320e-6;

/// A rectangle, µm.
pub(crate) fn rect(x: [f64; 2], y: [f64; 2]) -> Shape {
    Shape::rect(
        Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
        Length::um(x[1] - x[0]),
        Length::um(y[1] - y[0]),
    )
    .expect("a rectangle of positive size")
}

/// The mesh: cells from `smallest` at the silicon's edges, growing by `growth`, to `largest`
/// (µm).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mesh {
    /// The smallest cell, µm.
    pub smallest: f64,
    /// The growth from one cell to the next.
    pub growth: f64,
    /// The largest cell, µm.
    pub largest: f64,
}

/// Two N++ heaters side by side, µm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heaters {
    /// The heaters' centres' distance.
    pub pitch: f64,
    /// The window's reach beyond each guide.
    pub reach: f64,
    /// The oxide's top over the buried oxide's.
    pub top: f64,
    /// The window's height.
    pub height: f64,
}

impl Heaters {
    /// The case above.
    pub fn jacques() -> Heaters {
        Heaters {
            pitch: 2.05 + 5.0 + 2.05,
            reach: 20.0,
            top: 3.32,
            height: 18.0,
        }
    }

    fn bottom(&self) -> f64 {
        self.top - self.height
    }

    /// The window across.
    fn across(&self) -> [f64; 2] {
        [-self.reach, self.pitch + self.reach]
    }

    /// The heater's N++ strips at centre `c`.
    fn strips(c: f64) -> [[f64; 2]; 2] {
        [[c - 2.05, c - 1.05], [c + 1.05, c + 2.05]]
    }

    /// The rib at centre `c`, across.
    fn rib(c: f64) -> [f64; 2] {
        [c - 0.25, c + 0.25]
    }

    /// What fills the cross-section.
    pub fn section(&self) -> Section {
        let mut section = Section::new("SiO2");
        section
            .layer(self.bottom(), -2.0, "substrate")
            .expect("a layer inside the window");
        for c in [0.0, self.pitch] {
            section
                .draw(rect([c - 1.05, c + 1.05], [0.0, 0.09]), "slab")
                .draw(rect(Heaters::rib(c), [0.0, 0.22]), "Si");
            for s in Heaters::strips(c) {
                section.draw(rect(s, [0.0, 0.09]), "N++");
            }
        }
        section
    }

    /// The mesh lines across and in height.
    ///
    /// # Errors
    ///
    /// When `mesh` isn't a valid grading.
    pub fn axes(&self, mesh: &Mesh) -> photonoxide::Result<(Axis, Axis)> {
        let g = Grading::new(mesh.largest, mesh.smallest, mesh.growth)?;
        let mut across = Vec::new();
        for c in [0.0, self.pitch] {
            across.extend([c - 2.05, c - 1.05, c - 0.25, c + 0.25, c + 1.05, c + 2.05]);
        }
        let [a, b] = self.across();
        Ok((
            Axis::graded(a, b, &g, &across, &across)?,
            Axis::graded(
                self.bottom(),
                self.top,
                &g,
                &[-2.0, 0.0, 0.09, 0.22],
                &[0.0, 0.09, 0.22],
            )?,
        ))
    }

    /// The guides: the heated one's rib and its neighbour's.
    pub fn guides(&self) -> [([f64; 2], [f64; 2]); 2] {
        [
            (Heaters::rib(0.0), [0.0, 0.22]),
            (Heaters::rib(self.pitch), [0.0, 0.22]),
        ]
    }

    /// photonoxide's mean rises (K) in the heated guide and in its neighbour, on the mesh lines
    /// `across` and `height`.
    ///
    /// # Errors
    ///
    /// When the solve fails.
    pub fn photonoxide(&self, across: &Axis, height: &Axis) -> photonoxide::Result<[f64; 2]> {
        let mut heat = Heat::new(self.section(), Grid::new(across.clone(), height.clone()));
        for (m, k) in [
            ("SiO2", K_OXIDE),
            ("substrate", K_SUBSTRATE),
            ("Si", K_RIB),
            ("slab", K_SLAB),
            ("N++", K_NPP),
        ] {
            heat.conductivity(m, Tensor::isotropic(k));
        }
        heat.boundary(Side::Bottom, Boundary::Fixed(0.0)).boundary(
            Side::Top,
            Boundary::Convective {
                coefficient: 10.0,
                ambient: 0.0,
            },
        );
        for s in Heaters::strips(0.0) {
            heat.heater(rect(s, [0.0, 0.09]), 0.5 * WATTS_PER_METRE);
        }
        let t = heat.solve()?;
        let [own, neighbour] = self.guides().map(|(x, y)| t.mean(&rect(x, y)));
        Ok([own?, neighbour?])
    }

    /// Elmer's case on the same mesh lines: HeatSolver, the first heater's power as a heat
    /// source per volume (Elmer's `Heat Source` per mass, with a density of 1), the window's
    /// bottom at 0, a heat transfer coefficient of 10 W/(m²·K) to 0 at its top, its sides left
    /// insulated; SaveScalars writes the temperature at the guides' nodes, and the watches
    /// returned with the case take their means from them.
    ///
    /// # Errors
    ///
    /// When a side or a guide isn't on the mesh lines.
    pub fn elmer(
        &self,
        across: &Axis,
        height: &Axis,
    ) -> Result<(Case, [crate::elmer::Watch; 2]), String> {
        let section = self.section();
        let media = ["substrate", "SiO2", "Si", "slab", "N++"];
        let mut mesh = ElmerMesh::new(across.nodes(), height.nodes(), |x, y| {
            let m = section.medium_at(x, y);
            if m == "N++" && x < 0.5 * self.pitch {
                // the heated strips, a body of their own
                6
            } else {
                media
                    .iter()
                    .position(|&n| n == m)
                    .map_or(0, |k| k as u32 + 1)
            }
        });
        let [a, b] = self.across();
        mesh.along_x(1, self.bottom(), [a, b])?
            .along_x(2, self.top, [a, b])?;
        let [own, neighbour] = self.guides().map(|(x, y)| mesh.mean(x, y));
        let watches = [own?, neighbour?];
        // W/m over both strips' cross-section, W/m³
        let source = WATTS_PER_METRE / (2.0 * 1.0 * 0.09 * 1e-12);
        let mut bodies = String::new();
        for (k, name) in media.iter().enumerate() {
            bodies.push_str(&format!(
                "Body {}\n  Name = \"{name}\"\n  Equation = 1\n  Material = {}\nEnd\n\n",
                k + 1,
                k + 1
            ));
        }
        bodies.push_str(
            "Body 6\n  Name = \"heated N++\"\n  Equation = 1\n  Material = 5\n  Body Force = 1\nEnd\n\n",
        );
        let mut materials = String::new();
        for (k, kk) in [K_SUBSTRATE, K_OXIDE, K_RIB, K_SLAB, K_NPP]
            .iter()
            .enumerate()
        {
            materials.push_str(&format!(
                "Material {}\n  Heat Conductivity = {kk:e}\n  Density = 1.0\nEnd\n\n",
                k + 1
            ));
        }
        let sif = format!(
            "Header\n  Mesh DB \".\" \"mesh\"\nEnd\n\n\
             Simulation\n  Coordinate System = Cartesian 2D\n  Coordinate Scaling = 1.0e-6\n  \
             Simulation Type = Steady State\n  Steady State Max Iterations = 1\n  \
             Output Intervals = 0\nEnd\n\n\
             {bodies}\
             Equation 1\n  Active Solvers(1) = 1\nEnd\n\n\
             Solver 1\n  Equation = Heat Equation\n  Procedure = \"HeatSolve\" \"HeatSolver\"\n  \
             Variable = Temperature\n  Linear System Solver = Direct\n  \
             Linear System Direct Method = Umfpack\nEnd\n\n\
             Solver 2\n  Exec Solver = After All\n  Equation = SaveScalars\n  \
             Procedure = \"SaveData\" \"SaveScalars\"\n  Filename = \"scalars.dat\"\n  \
             Variable 1 = Temperature\n  Operator 1 = max\n{}End\n\n\
             {materials}\
             Body Force 1\n  Heat Source = {source:e}\nEnd\n\n\
             Boundary Condition 1\n  Name = \"bottom\"\n  Target Boundaries(1) = 1\n  \
             Temperature = 0.0\nEnd\n\n\
             Boundary Condition 2\n  Name = \"to the air\"\n  Target Boundaries(1) = 2\n  \
             Heat Transfer Coefficient = 10.0\n  External Temperature = 0.0\nEnd\n",
            save_points(&[&watches[0], &watches[1]])
        );
        Ok((Case { mesh, sif }, watches))
    }
}
