//! The shared heat case: a heater in oxide over silicon, its steady rise by photonoxide's heat
//! solver ([`photonoxide::thermal::Heat`]) and by Elmer's HeatSolver on the same mesh lines.
//!
//! A heater 2 µm wide and 0.2 µm thick (its centre 2 µm up) dissipating 1 W per metre of length
//! in oxide (k = 1.38 W/(m·K)) 3 µm thick, on 10 µm of silicon (k = 148 W/(m·K)) whose bottom
//! is held at 0; the sides and the top are insulated. Half of it is solved, beside the mirror
//! plane through the heater's centre: the half heater, 1 µm wide, dissipating 0.5 W/m, in a
//! window 30 µm wide. The figure compared is the peak rise, at the heater.

use photonoxide::geometry::{Point, Shape};
use photonoxide::thermal::{Axis, Boundary, Grading, Grid, Heat, Section, Side, Tensor};
use photonoxide::units::Length;

use crate::elmer::{Case, Mesh as ElmerMesh};

/// A heater in oxide over silicon, µm and W/(m·K).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heater {
    /// The heater's half width.
    pub half_width: f64,
    /// Its thickness.
    pub thickness: f64,
    /// Its centre's height above the silicon.
    pub height: f64,
    /// The oxide's thickness.
    pub oxide: f64,
    /// The silicon's thickness.
    pub silicon: f64,
    /// The window's half width.
    pub reach: f64,
    /// The power, W per metre of the whole heater's length.
    pub watts_per_metre: f64,
    /// The oxide's and the silicon's conductivities.
    pub conductivity: [f64; 2],
}

/// The mesh: cells from `smallest` at the heater's edges and the oxide's interface, growing by
/// `growth`, to `largest` (µm).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mesh {
    /// The smallest cell, µm.
    pub smallest: f64,
    /// The growth from one cell to the next.
    pub growth: f64,
    /// The largest cell, µm.
    pub largest: f64,
}

impl Heater {
    /// The case above.
    pub fn oxide_on_silicon() -> Heater {
        Heater {
            half_width: 1.0,
            thickness: 0.2,
            height: 2.0,
            oxide: 3.0,
            silicon: 10.0,
            reach: 30.0,
            watts_per_metre: 1.0,
            conductivity: [1.38, 148.0],
        }
    }

    fn edges(&self) -> [f64; 2] {
        [
            self.height - 0.5 * self.thickness,
            self.height + 0.5 * self.thickness,
        ]
    }

    /// The mesh lines across (from the mirror) and in height.
    ///
    /// # Errors
    ///
    /// When `mesh` isn't a valid grading.
    pub fn axes(&self, mesh: &Mesh) -> photonoxide::Result<(Axis, Axis)> {
        let g = Grading::new(mesh.largest, mesh.smallest, mesh.growth)?;
        let [lo, hi] = self.edges();
        Ok((
            Axis::graded(0.0, self.reach, &g, &[self.half_width], &[self.half_width])?,
            Axis::graded(
                -self.silicon,
                self.oxide,
                &g,
                &[0.0, lo, hi],
                &[0.0, lo, hi],
            )?,
        ))
    }

    /// photonoxide's peak rise (K) on the mesh lines `across` and `height`.
    ///
    /// # Errors
    ///
    /// When the solve fails.
    pub fn photonoxide(&self, across: &Axis, height: &Axis) -> photonoxide::Result<f64> {
        let mut section = Section::new("oxide");
        section.layer(-self.silicon, 0.0, "silicon")?;
        let heater = Shape::rect(
            Point::um(0.5 * self.half_width, self.height),
            Length::um(self.half_width),
            Length::um(self.thickness),
        )?;
        let mut heat = Heat::new(section, Grid::new(across.clone(), height.clone()));
        heat.conductivity("oxide", Tensor::isotropic(self.conductivity[0]))
            .conductivity("silicon", Tensor::isotropic(self.conductivity[1]))
            .boundary(Side::Left, Boundary::Flux(0.0))
            .boundary(Side::Right, Boundary::Flux(0.0))
            .boundary(Side::Top, Boundary::Flux(0.0))
            .boundary(Side::Bottom, Boundary::Fixed(0.0))
            .heater(heater, 0.5 * self.watts_per_metre);
        Ok(heat.solve()?.max().0)
    }

    /// Elmer's case on the same mesh lines: HeatSolver with the heater's power as a heat source
    /// per volume (Elmer's `Heat Source` per mass, with a density of 1), the silicon's bottom at
    /// 0 and the other sides left insulated, and SaveScalars writing the peak temperature to
    /// `scalars.dat`.
    ///
    /// # Errors
    ///
    /// When the silicon's bottom isn't on the mesh lines.
    pub fn elmer(&self, across: &Axis, height: &Axis) -> Result<Case, String> {
        let [lo, hi] = self.edges();
        let w = self.half_width;
        let mut mesh = ElmerMesh::new(across.nodes(), height.nodes(), |x, y| {
            if y < 0.0 {
                1
            } else if x < w && y > lo && y < hi {
                3
            } else {
                2
            }
        });
        mesh.along_x(1, -self.silicon, [0.0, self.reach])?;
        // W/m over the whole heater's cross-section, W/m³
        let source = self.watts_per_metre / (2.0 * self.half_width * self.thickness * 1e-12);
        let [k_oxide, k_silicon] = self.conductivity;
        let sif = format!(
            "Header\n  Mesh DB \".\" \"mesh\"\nEnd\n\n\
             Simulation\n  Coordinate System = Cartesian 2D\n  Coordinate Scaling = 1.0e-6\n  \
             Simulation Type = Steady State\n  Steady State Max Iterations = 1\n  \
             Output Intervals = 0\nEnd\n\n\
             Body 1\n  Name = \"silicon\"\n  Equation = 1\n  Material = 1\nEnd\n\n\
             Body 2\n  Name = \"oxide\"\n  Equation = 1\n  Material = 2\nEnd\n\n\
             Body 3\n  Name = \"heater\"\n  Equation = 1\n  Material = 2\n  Body Force = 1\nEnd\n\n\
             Equation 1\n  Active Solvers(1) = 1\nEnd\n\n\
             Solver 1\n  Equation = Heat Equation\n  Procedure = \"HeatSolve\" \"HeatSolver\"\n  \
             Variable = Temperature\n  Linear System Solver = Direct\n  \
             Linear System Direct Method = Umfpack\nEnd\n\n\
             Solver 2\n  Exec Solver = After All\n  Equation = SaveScalars\n  \
             Procedure = \"SaveData\" \"SaveScalars\"\n  Filename = \"scalars.dat\"\n  \
             Variable 1 = Temperature\n  Operator 1 = max\nEnd\n\n\
             Material 1\n  Heat Conductivity = {k_silicon:e}\n  Density = 1.0\nEnd\n\n\
             Material 2\n  Heat Conductivity = {k_oxide:e}\n  Density = 1.0\nEnd\n\n\
             Body Force 1\n  Heat Source = {source:e}\nEnd\n\n\
             Boundary Condition 1\n  Name = \"sink\"\n  Target Boundaries(1) = 1\n  \
             Temperature = 0.0\nEnd\n"
        );
        Ok(Case { mesh, sif })
    }
}
