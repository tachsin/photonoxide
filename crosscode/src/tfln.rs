//! A thin-film lithium niobate modulator's electrodes: Zhang et al.'s cross-section (Y. Zhang et
//! al., Photon. Res. 10, 2380 (2022), doi:10.1364/PRJ.468518, Section 2), its static field by
//! photonoxide's electrostatics ([`photonoxide::electro::Electrostatic`]) with the crystal's
//! anisotropic permittivity, and by Elmer's StatElecSolver with a tensor permittivity, on the
//! same mesh lines.
//!
//! An x-cut film 600 nm thick, etched 300 nm to a rib 1.2 µm wide (its sidewalls upright here,
//! so that the mesh lines follow them), on 2 µm of thermal oxide (ε 3.9) on silicon (ε 11.7,
//! 20 µm of it); electrodes 0.83 µm thick on the slab (800 nm of copper under 30 nm of gold), a
//! signal 20 µm wide 5 µm from a ground 15 µm wide (the ground's width isn't given), under
//! 0.6 µm of PECVD oxide (ε 5.5, the paper's) and 20 µm of air. The film's static permittivity
//! is lithium niobate's clamped one, diagonal in the section's frame: ε₃₃ = 26.2 across (the
//! crystal's c axis) and ε₁₁ = 45.5 up (photonoxide's catalogue, Jazbinšek and Zgonik). Half of
//! the push–pull line is solved, beside the mirror plane through the signal's centre, every
//! side of the window without surface charge. The figures compared are the whole line's
//! capacitance and the mean field across the guide (the rib and the slab under it) at 1 V,
//! which sets V_π·L.

use photonoxide::electro::{
    Axis, Conductor, EPSILON_0, Electrostatic, Grading, Grid, Section, Tensor,
};

use crate::elmer::{Case, Mesh as ElmerMesh, Watch, save_points};
use crate::heaters::{Mesh, rect};

/// Lithium niobate's clamped static permittivity along c (ε₃₃) and across it (ε₁₁).
pub const LN: [f64; 2] = [26.2, 45.5];
/// The thermal oxide's static permittivity.
pub const OXIDE: f64 = 3.9;
/// Silicon's.
pub const SILICON: f64 = 11.7;
/// The PECVD cladding's (the paper's).
pub const CLADDING: f64 = 5.5;

/// Zhang et al.'s cross-section, µm; y = 0 at the film's bottom, the rib centred on x = 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tfln {
    /// The film's thickness.
    pub film: f64,
    /// The slab left beside the rib.
    pub slab: f64,
    /// The rib's width.
    pub rib: f64,
    /// The electrodes' gap.
    pub gap: f64,
    /// The signal's half width (to the mirror).
    pub signal: f64,
    /// The ground's width.
    pub ground: f64,
    /// The electrodes' thickness.
    pub metal: f64,
    /// The buried oxide's thickness.
    pub buried: f64,
    /// The cladding's thickness over the slab.
    pub clad: f64,
    /// The silicon under the buried oxide, and the air over the cladding.
    pub beyond: f64,
}

impl Tfln {
    /// The case above.
    pub fn zhang() -> Tfln {
        Tfln {
            film: 0.6,
            slab: 0.3,
            rib: 1.2,
            gap: 5.0,
            signal: 10.0,
            ground: 15.0,
            metal: 0.83,
            buried: 2.0,
            clad: 0.6,
            beyond: 20.0,
        }
    }

    fn window(&self) -> ([f64; 2], [f64; 2]) {
        (
            [-0.5 * self.gap - self.ground, 0.5 * self.gap + self.signal],
            [
                -self.buried - self.beyond,
                self.slab + self.clad + self.beyond,
            ],
        )
    }

    /// The electrodes, across and in height: the ground, then the signal.
    fn electrodes(&self) -> [([f64; 2], [f64; 2]); 2] {
        let ([left, right], _) = self.window();
        let y = [self.slab, self.slab + self.metal];
        [([left, -0.5 * self.gap], y), ([0.5 * self.gap, right], y)]
    }

    /// The guide: the rib and the slab under it.
    pub fn guide(&self) -> ([f64; 2], [f64; 2]) {
        ([-0.5 * self.rib, 0.5 * self.rib], [0.0, self.film])
    }

    /// What fills the cross-section.
    pub fn section(&self) -> Section {
        let (_, [bottom, _]) = self.window();
        let mut section = Section::new("air");
        section
            .layer(bottom, -self.buried, "Si")
            .and_then(|s| s.layer(-self.buried, 0.0, "box"))
            .and_then(|s| s.layer(self.slab, self.slab + self.clad, "clad"))
            .and_then(|s| s.layer(0.0, self.slab, "LN"))
            .expect("layers inside the window");
        section.draw(
            rect([-0.5 * self.rib, 0.5 * self.rib], [self.slab, self.film]),
            "LN",
        );
        section
    }

    /// The mesh lines across and in height.
    ///
    /// # Errors
    ///
    /// When `mesh` isn't a valid grading.
    pub fn axes(&self, mesh: &Mesh) -> photonoxide::Result<(Axis, Axis)> {
        let g = Grading::new(mesh.largest, mesh.smallest, mesh.growth)?;
        let ([left, right], [bottom, top]) = self.window();
        let (g2, r2) = (0.5 * self.gap, 0.5 * self.rib);
        let across = [-g2, -r2, r2, g2];
        let up = self.slab + self.metal;
        Ok((
            Axis::graded(left, right, &g, &across, &across)?,
            Axis::graded(
                bottom,
                top,
                &g,
                &[
                    -self.buried,
                    0.0,
                    self.slab,
                    self.film,
                    self.slab + self.clad,
                    up,
                ],
                &[self.slab, self.film, up],
            )?,
        ))
    }

    /// photonoxide's capacitance of the whole line (F/m, twice the half's) and mean field
    /// across the guide at 1 V (V/m) on the mesh lines `across` and `height`: the signal at 1
    /// V, the ground at 0, the signal's charge its capacitance.
    ///
    /// # Errors
    ///
    /// When the solve fails.
    pub fn photonoxide(&self, across: &Axis, height: &Axis) -> photonoxide::Result<[f64; 2]> {
        let mut problem =
            Electrostatic::new(self.section(), Grid::new(across.clone(), height.clone()));
        problem
            .permittivity("air", Tensor::isotropic(1.0))
            .permittivity("Si", Tensor::isotropic(SILICON))
            .permittivity("box", Tensor::isotropic(OXIDE))
            .permittivity("clad", Tensor::isotropic(CLADDING))
            .permittivity("LN", Tensor::diagonal(LN[0], LN[1]));
        for ((x, y), volts) in self.electrodes().into_iter().zip([0.0, 1.0]) {
            problem.conductor(Conductor::shape(rect(x, y), volts));
        }
        let phi = problem.solve()?;
        let capacitance = 2.0 * phi.held()[1];
        let e = problem.electric_field(&phi)?;
        let ([x0, x1], [y0, y1]) = self.guide();
        let (gx, gy) = (across, height);
        let (mut sum, mut area) = (0.0, 0.0);
        for j in 0..gy.cells() {
            let yc = gy.centre(j);
            if yc < y0 || yc > y1 {
                continue;
            }
            for i in 0..gx.cells() {
                let xc = gx.centre(i);
                if xc < x0 || xc > x1 {
                    continue;
                }
                let a = gx.width(i) * gy.width(j);
                sum += a * e.value(i, j)[0];
                area += a;
            }
        }
        Ok([capacitance, sum / area])
    }

    /// Elmer's case on the same mesh lines: StatElecSolver, the film's permittivity a tensor
    /// (`Relative Permittivity(2,2)`), the ground's faces at 0 and the signal's at 1 V, the
    /// electrodes' insides meshed and held by their faces, every side of the window left
    /// without surface charge. Its capacitance (the half line's) is the one it gives from the
    /// field's energy, 2W/V² (`res: capacitance`, [`crate::elmer::result`]); SaveScalars writes
    /// the potential at the guide's sides' nodes, and the watch returned takes the mean field
    /// across from them ([`ElmerMesh::mean_across`]).
    ///
    /// # Errors
    ///
    /// When an electrode or the guide isn't on the mesh lines.
    pub fn elmer(&self, across: &Axis, height: &Axis) -> Result<(Case, Watch), String> {
        let section = self.section();
        let media = ["Si", "box", "LN", "clad", "air"];
        let electrodes = self.electrodes();
        let mut mesh = ElmerMesh::new(across.nodes(), height.nodes(), |x, y| {
            if let Some(k) = electrodes
                .iter()
                .position(|(ex, ey)| x > ex[0] && x < ex[1] && y > ey[0] && y < ey[1])
            {
                6 + k as u32
            } else {
                let m = section.medium_at(x, y);
                media
                    .iter()
                    .position(|&n| n == m)
                    .map_or(0, |k| k as u32 + 1)
            }
        });
        for (id, (x, y)) in [1u32, 2].into_iter().zip(electrodes) {
            mesh.along_x(id, y[0], x)?.along_x(id, y[1], x)?;
            let inner = if id == 1 { x[1] } else { x[0] };
            mesh.along_y(id, inner, y)?;
        }
        let (gx, gy) = self.guide();
        let field = mesh.mean_across(gx, gy)?;
        let mut bodies = String::new();
        for (k, name) in media.iter().chain(&["ground", "signal"]).enumerate() {
            bodies.push_str(&format!(
                "Body {}\n  Name = \"{name}\"\n  Equation = 1\n  Material = {}\nEnd\n\n",
                k + 1,
                (k + 1).min(6)
            ));
        }
        let sif = format!(
            "Header\n  Mesh DB \".\" \"mesh\"\nEnd\n\n\
             Simulation\n  Coordinate System = Cartesian 2D\n  Coordinate Scaling = 1.0e-6\n  \
             Simulation Type = Steady State\n  Steady State Max Iterations = 1\n  \
             Output Intervals = 0\nEnd\n\n\
             Constants\n  Permittivity Of Vacuum = {EPSILON_0:e}\nEnd\n\n\
             {bodies}\
             Equation 1\n  Active Solvers(1) = 1\nEnd\n\n\
             Solver 1\n  Equation = Stat Elec Solver\n  \
             Procedure = \"StatElecSolve\" \"StatElecSolver\"\n  Variable = Potential\n  \
             Variable DOFs = 1\n  Calculate Electric Energy = True\n  \
             Linear System Solver = Direct\n  Linear System Direct Method = Umfpack\nEnd\n\n\
             Solver 2\n  Exec Solver = After All\n  Equation = SaveScalars\n  \
             Procedure = \"SaveData\" \"SaveScalars\"\n  Filename = \"scalars.dat\"\n  \
             Variable 1 = Potential\n  Operator 1 = max\n{}End\n\n\
             Material 1\n  Relative Permittivity = {SILICON:e}\nEnd\n\n\
             Material 2\n  Relative Permittivity = {OXIDE:e}\nEnd\n\n\
             Material 3\n  Relative Permittivity(2,2) = Real {:e} 0.0 0.0 {:e}\nEnd\n\n\
             Material 4\n  Relative Permittivity = {CLADDING:e}\nEnd\n\n\
             Material 5\n  Relative Permittivity = 1.0\nEnd\n\n\
             Material 6\n  Name = \"the electrodes' insides, at their faces' potential\"\n  \
             Relative Permittivity = 1.0\nEnd\n\n\
             Boundary Condition 1\n  Name = \"ground\"\n  Target Boundaries(1) = 1\n  \
             Potential = 0.0\nEnd\n\n\
             Boundary Condition 2\n  Name = \"signal\"\n  Target Boundaries(1) = 2\n  \
             Potential = 1.0\nEnd\n",
            save_points(&[&field]),
            LN[0],
            LN[1]
        );
        Ok((Case { mesh, sif }, field))
    }
}
