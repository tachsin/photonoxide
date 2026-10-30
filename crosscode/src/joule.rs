//! Joule heating from a current: Harris et al.'s doped silicon heater (N. C. Harris et al., Opt.
//! Express 22, 10487 (2014), doi:10.1364/OE.22.010487, Section 2 and Fig. 1), the current
//! across the guide by photonoxide's current continuity ([`photonoxide::electro::Current`]),
//! its Joule heat the source of its heat solver ([`photonoxide::thermal::Heat::joule`]), and by
//! Elmer's StatCurrentSolver and HeatSolver, its Joule heating the heat's source, on the same
//! mesh lines.
//!
//! The section through one of the heater's channels, where the doped slab runs to the
//! contacts: a ridge 500 nm wide and 220 nm high on a 90 nm slab, the slab p-doped 1 µm wide
//! across the guide and p++ beyond it to ±3 µm, contacts on the slab's last 100 nm each side
//! at 1 V and 0 V. The conductivities come from the paper's sheet resistances, σ = 1/(R_s t):
//! 136 Ω (p++, 90 nm), 13.6 kΩ (p, 90 nm) and 3.87 kΩ (the ridge, 220 nm). Around it, as the
//! paper's model has it, 2 µm of buried oxide with its bottom held at 0 and 10 µm of oxide over
//! the silicon with its top held at 0, the window ±20 µm wide with its sides insulated. The
//! thermal conductivities aren't in the paper: Jacques et al.'s Table 3 (doi:10.1364/OE.27.010456)
//! for 220 nm and 90 nm of silicon and the oxide. The figures compared are the current per
//! metre of guide and the peak rise.

use photonoxide::electro::{Conductor, Current};
use photonoxide::thermal::{Axis, Boundary, Grading, Grid, Heat, Section, Side, Tensor};

use crate::elmer::{Case, Mesh as ElmerMesh};
use crate::heaters::{K_OXIDE, K_RIB, K_SLAB, Mesh, rect};

/// The ridge's conductivity, S/m: 1/(3.87 kΩ × 220 nm).
pub const SIGMA_RIDGE: f64 = 1.0 / (3.87e3 * 0.22e-6);
/// The p slab's: 1/(13.6 kΩ × 90 nm).
pub const SIGMA_P: f64 = 1.0 / (13.6e3 * 0.09e-6);
/// The p++ slab's: 1/(136 Ω × 90 nm).
pub const SIGMA_PPP: f64 = 1.0 / (136.0 * 0.09e-6);
/// The oxide's in photonoxide, which solves the current everywhere: many orders below the
/// silicon's (Elmer solves it in the silicon only).
const SIGMA_OXIDE: f64 = 1e-12;

/// The contacts, µm: the slab's last 100 nm each side.
const CONTACTS: [[f64; 2]; 2] = [[-3.0, -2.9], [2.9, 3.0]];

/// Harris et al.'s heater, µm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Joule {
    /// The window's half width.
    pub reach: f64,
    /// The oxide over the silicon's top.
    pub cladding: f64,
    /// The contacts' potentials, V.
    pub volts: [f64; 2],
}

impl Joule {
    /// The case above, 1 V across.
    pub fn harris() -> Joule {
        Joule {
            reach: 20.0,
            cladding: 10.0,
            volts: [1.0, 0.0],
        }
    }

    fn top(&self) -> f64 {
        0.22 + self.cladding
    }

    /// What fills the cross-section.
    pub fn section(&self) -> Section {
        let mut section = Section::new("SiO2");
        section
            .draw(rect([-3.0, 3.0], [0.0, 0.09]), "p++")
            .draw(rect([-0.5, 0.5], [0.0, 0.09]), "p")
            .draw(rect([-0.25, 0.25], [0.0, 0.22]), "ridge");
        section
    }

    /// The mesh lines across and in height.
    ///
    /// # Errors
    ///
    /// When `mesh` isn't a valid grading.
    pub fn axes(&self, mesh: &Mesh) -> photonoxide::Result<(Axis, Axis)> {
        let g = Grading::new(mesh.largest, mesh.smallest, mesh.growth)?;
        let across = [-3.0, -2.9, -0.5, -0.25, 0.25, 0.5, 2.9, 3.0];
        Ok((
            Axis::graded(-self.reach, self.reach, &g, &across, &across)?,
            Axis::graded(-2.0, self.top(), &g, &[0.0, 0.09, 0.22], &[0.0, 0.09, 0.22])?,
        ))
    }

    /// photonoxide's current per metre (A/m) and peak rise (K) on the mesh lines `across` and
    /// `height`.
    ///
    /// # Errors
    ///
    /// When a solve fails.
    pub fn photonoxide(&self, across: &Axis, height: &Axis) -> photonoxide::Result<[f64; 2]> {
        let grid = Grid::new(across.clone(), height.clone());
        let mut current = Current::new(self.section(), grid.clone());
        for (m, s) in [
            ("SiO2", SIGMA_OXIDE),
            ("ridge", SIGMA_RIDGE),
            ("p", SIGMA_P),
            ("p++", SIGMA_PPP),
        ] {
            current.conductivity(m, Tensor::isotropic(s));
        }
        for (c, v) in CONTACTS.iter().zip(self.volts) {
            current.contact(Conductor::shape(rect(*c, [0.0, 0.09]), v));
        }
        let flow = current.solve()?;
        let mut heat = Heat::new(self.section(), grid);
        for (m, k) in [
            ("SiO2", K_OXIDE),
            ("ridge", K_RIB),
            ("p", K_SLAB),
            ("p++", K_SLAB),
        ] {
            heat.conductivity(m, Tensor::isotropic(k));
        }
        heat.boundary(Side::Bottom, Boundary::Fixed(0.0))
            .boundary(Side::Top, Boundary::Fixed(0.0))
            .joule(&flow)?;
        Ok([flow.currents()[0], heat.solve()?.max().0])
    }

    /// Elmer's case on the same mesh lines: StatCurrentSolver in the silicon (its Joule
    /// heating computed) and HeatSolver everywhere, the Joule heating its source (`Joule Heat`,
    /// with a density of 1); the contacts' faces toward the slab held at their potentials, the
    /// contacts themselves conducting heat only; the window's bottom and top at 0, its sides
    /// left insulated. SaveScalars writes the peak temperature and the first contact's current,
    /// the sum of its nodes' reactions (`Potential Loads`), to `scalars.dat`.
    ///
    /// # Errors
    ///
    /// When a side or a contact isn't on the mesh lines.
    pub fn elmer(&self, across: &Axis, height: &Axis) -> Result<Case, String> {
        let section = self.section();
        let media = ["SiO2", "ridge", "p", "p++"];
        let mut mesh = ElmerMesh::new(across.nodes(), height.nodes(), |x, y| {
            if CONTACTS.iter().any(|c| x > c[0] && x < c[1]) && y > 0.0 && y < 0.09 {
                5
            } else {
                let m = section.medium_at(x, y);
                media
                    .iter()
                    .position(|&n| n == m)
                    .map_or(0, |k| k as u32 + 1)
            }
        });
        mesh.along_y(1, CONTACTS[0][1], [0.0, 0.09])?
            .along_y(2, CONTACTS[1][0], [0.0, 0.09])?
            .along_x(3, -2.0, [-self.reach, self.reach])?
            .along_x(3, self.top(), [-self.reach, self.reach])?;
        let [v1, v2] = self.volts;
        let sif = format!(
            "Header\n  Mesh DB \".\" \"mesh\"\nEnd\n\n\
             Simulation\n  Coordinate System = Cartesian 2D\n  Coordinate Scaling = 1.0e-6\n  \
             Simulation Type = Steady State\n  Steady State Max Iterations = 1\n  \
             Output Intervals = 0\nEnd\n\n\
             Body 1\n  Name = \"oxide\"\n  Equation = 2\n  Material = 1\nEnd\n\n\
             Body 2\n  Name = \"ridge\"\n  Equation = 1\n  Material = 2\n  Body Force = 1\nEnd\n\n\
             Body 3\n  Name = \"p\"\n  Equation = 1\n  Material = 3\n  Body Force = 1\nEnd\n\n\
             Body 4\n  Name = \"p++\"\n  Equation = 1\n  Material = 4\n  Body Force = 1\nEnd\n\n\
             Body 5\n  Name = \"contacts\"\n  Equation = 2\n  Material = 5\nEnd\n\n\
             Equation 1\n  Active Solvers(2) = 1 2\nEnd\n\n\
             Equation 2\n  Active Solvers(1) = 2\nEnd\n\n\
             Solver 1\n  Equation = Stat Current Solver\n  \
             Procedure = \"StatCurrentSolve\" \"StatCurrentSolver\"\n  Variable = Potential\n  \
             Variable DOFs = 1\n  Calculate Joule Heating = True\n  Calculate Loads = True\n  \
             Linear System Solver = Direct\n  Linear System Direct Method = Umfpack\nEnd\n\n\
             Solver 2\n  Equation = Heat Equation\n  Procedure = \"HeatSolve\" \"HeatSolver\"\n  \
             Variable = Temperature\n  Linear System Solver = Direct\n  \
             Linear System Direct Method = Umfpack\nEnd\n\n\
             Solver 3\n  Exec Solver = After All\n  Equation = SaveScalars\n  \
             Procedure = \"SaveData\" \"SaveScalars\"\n  Filename = \"scalars.dat\"\n  \
             Variable 1 = Temperature\n  Operator 1 = max\n  \
             Variable 2 = Potential Loads\n  Operator 2 = boundary sum\n  \
             Mask Name 2 = String Contact\nEnd\n\n\
             Material 1\n  Heat Conductivity = {K_OXIDE:e}\n  Density = 1.0\nEnd\n\n\
             Material 2\n  Heat Conductivity = {K_RIB:e}\n  Electric Conductivity = {SIGMA_RIDGE:e}\n  \
             Density = 1.0\nEnd\n\n\
             Material 3\n  Heat Conductivity = {K_SLAB:e}\n  Electric Conductivity = {SIGMA_P:e}\n  \
             Density = 1.0\nEnd\n\n\
             Material 4\n  Heat Conductivity = {K_SLAB:e}\n  Electric Conductivity = {SIGMA_PPP:e}\n  \
             Density = 1.0\nEnd\n\n\
             Material 5\n  Heat Conductivity = {K_SLAB:e}\n  Density = 1.0\nEnd\n\n\
             Body Force 1\n  Joule Heat = Logical True\nEnd\n\n\
             Boundary Condition 1\n  Name = \"first contact\"\n  Target Boundaries(1) = 1\n  \
             Potential = {v1:e}\n  Contact = Logical True\nEnd\n\n\
             Boundary Condition 2\n  Name = \"second contact\"\n  Target Boundaries(1) = 2\n  \
             Potential = {v2:e}\nEnd\n\n\
             Boundary Condition 3\n  Name = \"bottom and top\"\n  Target Boundaries(1) = 3\n  \
             Temperature = 0.0\nEnd\n"
        );
        Ok(Case { mesh, sif })
    }
}
