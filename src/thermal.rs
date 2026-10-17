//! Heat conduction on a waveguide's cross-section: −∇·(k∇T) = Q, steady, per unit length
//! along the guide.
//!
//! A [`Heat`] problem is a [`Section`] (what fills the cross-section: shapes drawn with media,
//! or a [`Structure`](crate::stack::Structure)'s cut with its stack, etches and walls) on a
//! [`Grid`], each medium's thermal conductivity k in W/(m·K) ([`Tensor`]: isotropic, or
//! diagonal in the section's frame), the window's sides ([`Boundary`]: a fixed temperature, a
//! flux, or a convective film), heat sinks held at a temperature, and heaters that give a
//! power per unit length (W/m), or the Joule heat of a current ([`crate::electro::Current`]).
//! Its solution is a [`Field`] of temperatures at the cells' centres, with the heat each sink
//! takes and what leaves through each side; a 2D number, per metre of a straight section.
//!
//! Temperatures are in whatever unit the problem's are (K or °C); a rise above a sink at 0 is
//! a temperature difference.
//!
//! The method, shared with [`crate::electro`], is the static solver of docs/methods/static.md:
//! cell-centred finite volumes on a graded grid, exact for layered media on the grid's lines,
//! second order elsewhere, solved by photonoxide's multifrontal solver. Its validation cases
//! (`static/` in docs/validation.md) check it against closed forms: a slab and a two-layer
//! stack, a line source above an isothermal substrate by images, and the order of the error.
//!
//! ```
//! use photonoxide::geometry::{Point, Shape};
//! use photonoxide::thermal::{Boundary, Features, Grading, Grid, Heat, Section, Side, Tensor};
//! use photonoxide::units::Length;
//!
//! // 2 µm of oxide on an isothermal substrate, a 1 W/m heater 1 µm up
//! let mut section = Section::new("oxide");
//! let heater = Shape::rect(Point::um(0.0, 1.0), Length::um(0.5), Length::um(0.1))?;
//! let mut features = section.features().clone();
//! features.shape(&heater);
//! let grid = Grid::fitted([-20.0, 20.0], [0.0, 2.0], &Grading::new(0.5, 0.02, 1.2)?, &features)?;
//! let mut heat = Heat::new(section, grid);
//! heat.conductivity("oxide", Tensor::isotropic(1.4))
//!     .boundary(Side::Bottom, Boundary::Fixed(0.0))
//!     .heater(heater, 1.0);
//! let t = heat.solve()?;
//! // all the heat leaves through the substrate
//! assert!((t.outflow(Side::Bottom) - 1.0).abs() < 1e-9);
//! assert!(t.max().0 > 0.0);
//! # Ok::<(), photonoxide::Error>(())
//! ```

use crate::geometry::Shape;
use crate::statics::{Discrete, Hold, Problem, spread};
use crate::{Error, Result};

pub use crate::statics::{
    Axis, Boundary, Cut, Features, Field, Grading, Grid, MAX_CELLS, Section, Side, Tensor,
};

/// A steady heat-conduction problem on a cross-section. See the module's docs.
#[derive(Clone, Debug)]
pub struct Heat {
    section: Section,
    grid: Grid,
    conductivity: Vec<(String, Tensor)>,
    sides: [Boundary; 4],
    sinks: Vec<(Shape, f64)>,
    heaters: Vec<(Shape, f64)>,
    extra: Option<Vec<f64>>,
}

impl Heat {
    /// `section` on `grid`, every side insulated, nothing held, nothing heating.
    pub fn new(section: Section, grid: Grid) -> Heat {
        Heat {
            section,
            grid,
            conductivity: Vec::new(),
            sides: [Boundary::INSULATED; 4],
            sinks: Vec::new(),
            heaters: Vec::new(),
            extra: None,
        }
    }

    /// The thermal conductivity of `medium`, W/(m·K) (a later call for the same medium wins).
    pub fn conductivity(&mut self, medium: &str, k: Tensor) -> &mut Heat {
        self.conductivity.retain(|(m, _)| m != medium);
        self.conductivity.push((medium.to_owned(), k));
        self
    }

    /// What holds `side`.
    pub fn boundary(&mut self, side: Side, boundary: Boundary) -> &mut Heat {
        self.sides[side_index(side)] = boundary;
        self
    }

    /// A heat sink: the cells whose centres `shape` holds, at `temperature`. Returns the
    /// problem; [`Field::held`] lists the heat each sink gives, in the order added.
    pub fn sink(&mut self, shape: Shape, temperature: f64) -> &mut Heat {
        self.sinks.push((shape, temperature));
        self
    }

    /// A heater: `watts_per_metre` spread over `shape` by area.
    pub fn heater(&mut self, shape: Shape, watts_per_metre: f64) -> &mut Heat {
        self.heaters.push((shape, watts_per_metre));
        self
    }

    /// The Joule heat of `flow` (a current solved on the same grid) as a source.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if `flow` was solved on another grid.
    pub fn joule(&mut self, flow: &crate::electro::Flow) -> Result<&mut Heat> {
        if flow.field().grid() != &self.grid {
            return Err(Error::invalid(
                "static problem",
                "a current's Joule heat heats a problem on its own grid",
            ));
        }
        self.extra = Some(flow.joule().to_vec());
        Ok(self)
    }

    /// The section.
    pub fn section(&self) -> &Section {
        &self.section
    }

    /// The grid.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// The steady temperature.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if a medium has no conductivity, a coefficient or boundary isn't
    /// finite (or positive), a heater covers no cell, or nothing holds the temperature (every
    /// side insulated and no sink).
    pub fn solve(&self) -> Result<Field> {
        let coefficients = self.section.coefficients(
            &self.grid,
            |m| {
                self.conductivity
                    .iter()
                    .find(|(n, _)| n == m)
                    .map(|(_, t)| *t)
            },
            "thermal conductivity",
        )?;
        let problem = Problem {
            grid: &self.grid,
            coefficients,
            sides: self.sides,
            holds: self
                .sinks
                .iter()
                .map(|(s, _)| Hold::Shape(s.clone()))
                .collect(),
            profile: None,
        };
        let discrete = Discrete::new(&problem)?;
        let mut sources = self
            .extra
            .clone()
            .unwrap_or_else(|| vec![0.0; self.grid.cells()]);
        for (shape, power) in &self.heaters {
            if !power.is_finite() {
                return Err(Error::invalid(
                    "static problem",
                    format!("a heater's power must be finite, got {power}"),
                ));
            }
            spread(&self.grid, shape, *power, &mut sources)?;
        }
        let values: Vec<f64> = self.sinks.iter().map(|s| s.1).collect();
        discrete.solve(&values, &sources)
    }
}

pub(crate) fn side_index(side: Side) -> usize {
    match side {
        Side::Left => 0,
        Side::Right => 1,
        Side::Bottom => 2,
        Side::Top => 3,
    }
}
