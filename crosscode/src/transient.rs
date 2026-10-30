//! Transient heat: Jacques et al.'s N++ silicon heater (M. Jacques et al., Opt. Express 27,
//! 10456 (2019), doi:10.1364/OE.27.010456, Section 2.2, Figs. 1(c) and Table 3) turned on, its
//! guide's rise against time, by photonoxide's transient heat solver
//! ([`photonoxide::thermal::Heat::transient`], BDF2) and by Elmer's HeatSolver in time (BDF of
//! order 2), with the same time steps on the same mesh lines.
//!
//! The heater of [`crate::heaters`] alone, in the paper's 40 × 18 µm window, its sides
//! insulated; half of it is solved, beside the mirror plane through the rib's centre. It is cold
//! (0) until t = 0, then dissipates the paper's P_π, 71.25 W/m, in its N++ strips. The heat
//! capacities per volume are Table 3's densities times specific heats. The figures compared
//! are the rib's mean rise at 1, 5 and 20 µs and its 1/e time, when it first reaches
//! 1 − 1/e of its steady rise (the paper's simulated limiting 1/e time is about 5.8 µs, of the
//! phase, Fig. 1(c)).

use photonoxide::thermal::{
    Axis, Boundary, Grading, Grid, Heat, Scheme, Section, Side, Stepping, Tensor,
};

use crate::elmer::{Case, Mesh as ElmerMesh, Watch, save_points};
use crate::heaters::{K_NPP, K_OXIDE, K_RIB, K_SLAB, K_SUBSTRATE, Mesh, WATTS_PER_METRE, rect};

/// Jacques et al.'s Table 3, J/(m³·K): silicon's density times its specific heat, and the
/// oxide's.
pub const C_SILICON: f64 = 2330.0 * 711.0;
/// The oxide's.
pub const C_OXIDE: f64 = 2203.0 * 709.0;

/// The times the rise is compared at, µs.
pub const TIMES_US: [f64; 3] = [1.0, 5.0, 20.0];

/// The steps' end, µs: the last of [`TIMES_US`].
pub const END_US: f64 = 20.0;

/// The heater in its window, µm, half of it beside the mirror at x = 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transient {
    /// The window's half width.
    pub reach: f64,
    /// The oxide's top over the buried oxide's.
    pub top: f64,
    /// The window's height.
    pub height: f64,
}

/// The guide, the rib's half beside the mirror, µm.
const RIB: ([f64; 2], [f64; 2]) = ([0.0, 0.25], [0.0, 0.22]);

/// The heated N++ strip, the half's, µm.
const STRIP: ([f64; 2], [f64; 2]) = ([1.05, 2.05], [0.0, 0.09]);

impl Transient {
    /// The case above.
    pub fn jacques() -> Transient {
        Transient {
            reach: 20.0,
            top: 3.32,
            height: 18.0,
        }
    }

    fn bottom(&self) -> f64 {
        self.top - self.height
    }

    /// What fills the cross-section.
    pub fn section(&self) -> Section {
        let mut section = Section::new("SiO2");
        section
            .layer(self.bottom(), -2.0, "substrate")
            .expect("a layer inside the window");
        section
            .draw(rect([-1.05, 1.05], [0.0, 0.09]), "slab")
            .draw(rect([-0.25, 0.25], [0.0, 0.22]), "Si")
            .draw(rect([-2.05, -1.05], [0.0, 0.09]), "N++")
            .draw(rect(STRIP.0, STRIP.1), "N++");
        section
    }

    /// The mesh lines across (from the mirror) and in height.
    ///
    /// # Errors
    ///
    /// When `mesh` isn't a valid grading.
    pub fn axes(&self, mesh: &Mesh) -> photonoxide::Result<(Axis, Axis)> {
        let g = Grading::new(mesh.largest, mesh.smallest, mesh.growth)?;
        let across = [0.25, 1.05, 2.05];
        Ok((
            Axis::graded(0.0, self.reach, &g, &across, &across)?,
            Axis::graded(
                self.bottom(),
                self.top,
                &g,
                &[-2.0, 0.0, 0.09, 0.22],
                &[0.0, 0.09, 0.22],
            )?,
        ))
    }

    fn heat(&self, across: &Axis, height: &Axis) -> Heat {
        let mut heat = Heat::new(self.section(), Grid::new(across.clone(), height.clone()));
        for (m, k, c) in [
            ("SiO2", K_OXIDE, C_OXIDE),
            ("substrate", K_SUBSTRATE, C_SILICON),
            ("Si", K_RIB, C_SILICON),
            ("slab", K_SLAB, C_SILICON),
            ("N++", K_NPP, C_SILICON),
        ] {
            heat.conductivity(m, Tensor::isotropic(k))
                .heat_capacity(m, c);
        }
        heat.boundary(Side::Bottom, Boundary::Fixed(0.0))
            .boundary(
                Side::Top,
                Boundary::Convective {
                    coefficient: 10.0,
                    ambient: 0.0,
                },
            )
            .heater(rect(STRIP.0, STRIP.1), 0.5 * WATTS_PER_METRE);
        heat
    }

    /// photonoxide's steady mean rise of the rib (K) on the mesh lines `across` and `height`.
    ///
    /// # Errors
    ///
    /// When the solve fails.
    pub fn steady(&self, across: &Axis, height: &Axis) -> photonoxide::Result<f64> {
        self.heat(across, height).solve()?.mean(&rect(RIB.0, RIB.1))
    }

    /// photonoxide's rib's mean rise from cold, the heater on from t = 0: (t in s, K) at 0 and
    /// after each of `steps` BDF2 steps of `step_s`.
    ///
    /// # Errors
    ///
    /// When the solve fails.
    pub fn photonoxide(
        &self,
        across: &Axis,
        height: &Axis,
        step_s: f64,
        steps: usize,
    ) -> photonoxide::Result<Vec<(f64, f64)>> {
        let history = self.heat(across, height).transient(
            &Stepping::new(step_s, steps, Scheme::Bdf2),
            0.0,
            &|_| 1.0,
            &[rect(RIB.0, RIB.1)],
        )?;
        Ok(history
            .times_s
            .iter()
            .copied()
            .zip(history.watched[0].iter().copied())
            .collect())
    }

    /// Elmer's cases on the same mesh lines: HeatSolver steady, and in time from 0 by BDF of
    /// order 2 with `steps` steps of `step_s`, the heater's power a heat source per volume
    /// (Elmer's `Heat Source` per mass and `Heat Capacity` per mass, with a density of 1), the
    /// window's bottom at 0, 10 W/(m²·K) to 0 at its top, its sides left insulated;
    /// SaveScalars writes the temperature at the rib's nodes after each step, and the watch
    /// returned takes the rib's mean from them.
    ///
    /// # Errors
    ///
    /// When a side or the rib isn't on the mesh lines.
    pub fn elmer(
        &self,
        across: &Axis,
        height: &Axis,
        step_s: f64,
        steps: usize,
    ) -> Result<(Case, Case, Watch), String> {
        let section = self.section();
        let media = ["substrate", "SiO2", "Si", "slab", "N++"];
        let mut mesh = ElmerMesh::new(across.nodes(), height.nodes(), |x, y| {
            let m = section.medium_at(x, y);
            if m == "N++" && x > STRIP.0[0] {
                6
            } else {
                media
                    .iter()
                    .position(|&n| n == m)
                    .map_or(0, |k| k as u32 + 1)
            }
        });
        mesh.along_x(1, self.bottom(), [0.0, self.reach])?.along_x(
            2,
            self.top,
            [0.0, self.reach],
        )?;
        let rib = mesh.mean(RIB.0, RIB.1)?;
        // the half heater's W/m over its strip's cross-section, W/m³
        let source = 0.5 * WATTS_PER_METRE / ((STRIP.0[1] - STRIP.0[0]) * 0.09 * 1e-12);
        let mut bodies = String::new();
        for (k, name) in media.iter().enumerate() {
            bodies.push_str(&format!(
                "Body {}\n  Name = \"{name}\"\n  Equation = 1\n  Material = {}\n  \
                 Initial Condition = 1\nEnd\n\n",
                k + 1,
                k + 1
            ));
        }
        bodies.push_str(
            "Body 6\n  Name = \"heated N++\"\n  Equation = 1\n  Material = 5\n  Body Force = 1\n  \
             Initial Condition = 1\nEnd\n\n",
        );
        let mut materials = String::new();
        for (k, (kk, c)) in [
            (K_SUBSTRATE, C_SILICON),
            (K_OXIDE, C_OXIDE),
            (K_RIB, C_SILICON),
            (K_SLAB, C_SILICON),
            (K_NPP, C_SILICON),
        ]
        .iter()
        .enumerate()
        {
            materials.push_str(&format!(
                "Material {}\n  Heat Conductivity = {kk:e}\n  Heat Capacity = {c:e}\n  \
                 Density = 1.0\nEnd\n\n",
                k + 1
            ));
        }
        let sif = |simulation: &str, exec: &str| {
            format!(
                "Header\n  Mesh DB \".\" \"mesh\"\nEnd\n\n\
                 Simulation\n  Coordinate System = Cartesian 2D\n  Coordinate Scaling = 1.0e-6\n\
                 {simulation}  Steady State Max Iterations = 1\n  Output Intervals = 0\nEnd\n\n\
                 {bodies}\
                 Initial Condition 1\n  Temperature = 0.0\nEnd\n\n\
                 Equation 1\n  Active Solvers(1) = 1\nEnd\n\n\
                 Solver 1\n  Equation = Heat Equation\n  Procedure = \"HeatSolve\" \"HeatSolver\"\n  \
                 Variable = Temperature\n  Linear System Solver = Direct\n  \
                 Linear System Direct Method = Umfpack\nEnd\n\n\
                 Solver 2\n  Exec Solver = {exec}\n  Equation = SaveScalars\n  \
                 Procedure = \"SaveData\" \"SaveScalars\"\n  Filename = \"scalars.dat\"\n  \
                 Variable 1 = Temperature\n  Operator 1 = max\n  Variable 2 = Time\n{}End\n\n\
                 {materials}\
                 Body Force 1\n  Heat Source = {source:e}\nEnd\n\n\
                 Boundary Condition 1\n  Name = \"bottom\"\n  Target Boundaries(1) = 1\n  \
                 Temperature = 0.0\nEnd\n\n\
                 Boundary Condition 2\n  Name = \"to the air\"\n  Target Boundaries(1) = 2\n  \
                 Heat Transfer Coefficient = 10.0\n  External Temperature = 0.0\nEnd\n",
                save_points(&[&rib])
            )
        };
        let steady = sif("  Simulation Type = Steady State\n", "After All");
        let transient = sif(
            &format!(
                "  Simulation Type = Transient\n  Timestepping Method = BDF\n  BDF Order = 2\n  \
                 Timestep Sizes(1) = {step_s:e}\n  Timestep Intervals(1) = {steps}\n"
            ),
            "After Timestep",
        );
        Ok((
            Case {
                mesh: mesh.clone(),
                sif: steady,
            },
            Case {
                mesh,
                sif: transient,
            },
            rib,
        ))
    }
}

/// The figures of a rise `history` ((t in s, K), from t = 0) whose steady rise is `steady`:
/// the rise at each of [`TIMES_US`], and the 1/e time (µs), when it first reaches 1 − 1/e of
/// `steady`, both linear between steps.
pub fn figures(history: &[(f64, f64)], steady: f64) -> Vec<f64> {
    let at = |t: f64| -> f64 {
        history
            .windows(2)
            .find(|w| w[1].0 >= t * (1.0 - 1e-9))
            .map_or(f64::NAN, |w| {
                let s = ((t - w[0].0) / (w[1].0 - w[0].0)).clamp(0.0, 1.0);
                w[0].1 + s * (w[1].1 - w[0].1)
            })
    };
    let target = (1.0 - (-1.0f64).exp()) * steady;
    let tau = history
        .windows(2)
        .find(|w| w[1].1 >= target)
        .map_or(f64::NAN, |w| {
            w[0].0 + (target - w[0].1) / (w[1].1 - w[0].1) * (w[1].0 - w[0].0)
        });
    let mut out: Vec<f64> = TIMES_US.iter().map(|&t| at(t * 1e-6)).collect();
    out.push(tau * 1e6);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rises_figures_are_read_between_its_steps() {
        // T = 1 − exp(−t/τ) at τ = 5 µs, every 0.5 µs to 20 µs
        let history: Vec<(f64, f64)> = (0..=40)
            .map(|k| {
                let t = k as f64 * 0.5e-6;
                (t, 1.0 - (-t / 5e-6).exp())
            })
            .collect();
        let f = figures(&history, 1.0);
        assert!((f[0] - (1.0 - (-0.2f64).exp())).abs() < 1e-12);
        assert!((f[2] - (1.0 - (-4.0f64).exp())).abs() < 1e-12);
        assert!((f[3] - 5.0).abs() < 0.01, "{}", f[3]);
    }
}
