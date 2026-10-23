//! Electrostatics and current continuity on a waveguide's cross-section: −∇·(ε∇φ) = ρ and
//! −∇·(σ∇V) = 0, per unit length along the guide.
//!
//! An [`Electrostatic`] problem is a [`Section`] on a [`Grid`], each medium's relative static
//! permittivity ([`Tensor`]: isotropic, or diagonal in the section's frame, as x-cut and z-cut
//! lithium niobate are), the window's sides ([`Boundary`]), conductors held at potentials
//! ([`Conductor`]: a shape, or a strip of zero thickness on a grid line, as a coplanar line's
//! electrodes are drawn by conformal mapping), and charges. Its solution is the potential at the
//! cells' centres ([`Field`]) with each conductor's charge (C/m); [`Electrostatic::capacitance`]
//! gives the capacitance matrix (F/m).
//!
//! A [`Current`] problem is the same with electrical conductivities (S/m) and contacts at
//! potentials; its [`Flow`] gives each contact's current (A/m) and each cell's Joule heat, which
//! a [`crate::thermal::Heat`] problem on the same grid takes as its source.
//!
//! The method, shared with [`crate::thermal`], is the static solver of docs/methods/static.md.
//! Its validation cases (`static/` in docs/validation.md) check it against closed forms: the
//! parallel-plate capacitor, a coaxial line's 2πε/ln(b/a), a coplanar waveguide by conformal
//! mapping, and a resistor's Joule heat.
//!
//! ```
//! use photonoxide::electro::{Boundary, Conductor, Electrostatic, Grid, Axis, Section, Side, Tensor};
//! use photonoxide::geometry::{Point, Shape};
//! use photonoxide::units::Length;
//!
//! // a parallel-plate capacitor 1 µm wide, 0.2 µm apart, insulated at the sides
//! let grid = Grid::new(Axis::uniform(0.0, 1.0, 10)?, Axis::uniform(-0.1, 0.3, 40)?);
//! let plate = |y: f64| Shape::rect(Point::um(0.5, y), Length::um(2.0), Length::um(0.1));
//! let mut problem = Electrostatic::new(Section::new("oxide"), grid);
//! problem
//!     .permittivity("oxide", Tensor::isotropic(3.9))
//!     .conductor(Conductor::shape(plate(-0.05)?, 0.0))
//!     .conductor(Conductor::shape(plate(0.25)?, 1.0));
//! let c = problem.capacitance()?;
//! let exact = 3.9 * photonoxide::electro::EPSILON_0 * 1e-6 / 0.2e-6; // ε W / d, F/m
//! assert!((c[1][1] - exact).abs() < 1e-12 * exact);
//! # Ok::<(), photonoxide::Error>(())
//! ```

use crate::geometry::Shape;
use crate::statics::{Discrete, Hold, Problem, spread};
use crate::thermal::side_index;
use crate::{Error, Result};

pub use crate::statics::{
    Axis, Boundary, Cut, Features, Field, Grading, Grid, MAX_CELLS, Section, Side, Tensor,
};

/// The vacuum permittivity ε₀, F/m (CODATA 2022: 8.8541878188(14) × 10⁻¹² F/m).
pub const EPSILON_0: f64 = 8.854_187_818_8e-12;

/// A conductor held at a potential (V): the cells whose centres a shape holds, or the faces a
/// strip of zero thickness covers on a horizontal line of the grid (fit the grid to it with
/// [`Features::strip`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Conductor {
    hold: Hold,
    potential: f64,
}

impl Conductor {
    /// The cells whose centres `shape` holds, at `volts`.
    pub fn shape(shape: Shape, volts: f64) -> Conductor {
        Conductor {
            hold: Hold::Shape(shape),
            potential: volts,
        }
    }

    /// A strip of zero thickness from `x[0]` to `x[1]` at height `y` (µm), at `volts`.
    pub fn strip(x: [f64; 2], y: f64, volts: f64) -> Conductor {
        Conductor {
            hold: Hold::Strip { x, y },
            potential: volts,
        }
    }

    /// Its potential, V.
    pub fn potential(&self) -> f64 {
        self.potential
    }

    fn check(&self) -> Result<()> {
        let ok = self.potential.is_finite()
            && match &self.hold {
                Hold::Shape(_) => true,
                Hold::Strip { x, y } => {
                    x[0].is_finite() && x[1].is_finite() && x[0] < x[1] && y.is_finite()
                }
            };
        if ok {
            Ok(())
        } else {
            Err(Error::invalid(
                "static problem",
                format!(
                    "a conductor needs a finite potential (a strip a finite, ordered span), got {self:?}"
                ),
            ))
        }
    }
}

fn table(entries: &[(String, Tensor)], m: &str) -> Option<Tensor> {
    entries.iter().find(|(n, _)| n == m).map(|(_, t)| *t)
}

fn set(entries: &mut Vec<(String, Tensor)>, medium: &str, value: Tensor) {
    entries.retain(|(m, _)| m != medium);
    entries.push((medium.to_owned(), value));
}

/// An electrostatic problem on a cross-section. See the module's docs.
#[derive(Clone, Debug)]
pub struct Electrostatic {
    section: Section,
    grid: Grid,
    permittivity: Vec<(String, Tensor)>,
    sides: [Boundary; 4],
    conductors: Vec<Conductor>,
    charges: Vec<(Shape, f64)>,
}

impl Electrostatic {
    /// `section` on `grid`, every side without surface charge (a mirror plane, D·n = 0),
    /// no conductors.
    pub fn new(section: Section, grid: Grid) -> Electrostatic {
        Electrostatic {
            section,
            grid,
            permittivity: Vec::new(),
            sides: [Boundary::INSULATED; 4],
            conductors: Vec::new(),
            charges: Vec::new(),
        }
    }

    /// The relative static permittivity of `medium` (a later call for the same medium wins).
    pub fn permittivity(&mut self, medium: &str, relative: Tensor) -> &mut Electrostatic {
        set(&mut self.permittivity, medium, relative);
        self
    }

    /// What holds `side`: a fixed potential (V), or a surface charge (C/m², zero for a mirror).
    pub fn boundary(&mut self, side: Side, boundary: Boundary) -> &mut Electrostatic {
        self.sides[side_index(side)] = boundary;
        self
    }

    /// A conductor; [`Field::held`] lists each conductor's charge (C/m), in the order added.
    pub fn conductor(&mut self, conductor: Conductor) -> &mut Electrostatic {
        self.conductors.push(conductor);
        self
    }

    /// A charge of `coulombs_per_metre` spread over `shape` by area.
    pub fn charge(&mut self, shape: Shape, coulombs_per_metre: f64) -> &mut Electrostatic {
        self.charges.push((shape, coulombs_per_metre));
        self
    }

    /// The conductors, in the order added.
    pub fn conductors(&self) -> &[Conductor] {
        &self.conductors
    }

    /// The grid.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    fn discrete(&self) -> Result<Discrete> {
        self.problem(Discrete::new)
    }

    fn problem(&self, build: impl Fn(&Problem<'_>) -> Result<Discrete>) -> Result<Discrete> {
        for c in &self.conductors {
            c.check()?;
        }
        let coefficients = self.section.coefficients(
            &self.grid,
            |m| {
                table(&self.permittivity, m)
                    .map(|t| Tensor::diagonal(t.xx * EPSILON_0, t.yy * EPSILON_0))
            },
            "permittivity",
        )?;
        build(&Problem {
            grid: &self.grid,
            coefficients,
            sides: self.sides,
            holds: self.conductors.iter().map(|c| c.hold.clone()).collect(),
            profile: None,
        })
    }

    /// The electric field E = −∇φ of `field` (this problem's solution) at each cell's centre,
    /// V/m: the displacement through each face, ε₀ε_r (φ_P − φ_N) over the distance with the
    /// media in series as the scheme has it, averaged over the cell's two faces along each axis
    /// and divided by the cell's own ε₀ε_r. D·n is continuous across an interface on the grid's
    /// lines, so E on each side of it is that medium's own (in a high-permittivity crystal, the
    /// smaller): a central difference of φ across it ([`Field::gradient`]) would mix the two.
    /// E is zero inside a conductor.
    ///
    /// # Errors
    ///
    /// As [`Electrostatic::solve`], and [`Error::InvalidValue`] for a field on another grid.
    pub fn electric_field(&self, field: &Field) -> Result<ElectricField> {
        if field.grid() != &self.grid {
            return Err(Error::invalid(
                "field",
                "the potential was solved on another grid",
            ));
        }
        let discrete = self.problem(Discrete::assembled)?;
        let d = discrete.flux_density(field);
        let coefficients = discrete.coefficients().to_vec();
        let values = d
            .iter()
            .zip(&coefficients)
            .map(|(d, c)| [d[0] / c[0], d[1] / c[1]])
            .collect();
        let held = (0..d.len()).map(|c| discrete.is_held(c)).collect();
        Ok(ElectricField {
            grid: self.grid.clone(),
            values,
            coefficients,
            held,
        })
    }

    /// The potential, V, and each conductor's charge, C/m.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if a medium has no permittivity, a value isn't finite, a strip
    /// isn't on a grid line, a charge covers no cell, or nothing holds the potential.
    pub fn solve(&self) -> Result<Field> {
        let discrete = self.discrete()?;
        let mut sources = vec![0.0; self.grid.cells()];
        for (shape, q) in &self.charges {
            if !q.is_finite() {
                return Err(Error::invalid(
                    "static problem",
                    format!("a charge must be finite, got {q}"),
                ));
            }
            spread(&self.grid, shape, *q, &mut sources)?;
        }
        let values: Vec<f64> = self.conductors.iter().map(|c| c.potential).collect();
        discrete.solve(&values, &sources)
    }

    /// The capacitance matrix, F/m: entry (i, j) is conductor i's charge with conductor j at
    /// 1 V and every other conductor, and every fixed side, at 0 V (the charges given and the
    /// conductors' own potentials don't enter). The matrix is factorized once.
    ///
    /// # Errors
    ///
    /// As [`Electrostatic::solve`].
    pub fn capacitance(&self) -> Result<Vec<Vec<f64>>> {
        let mut grounded = self.clone();
        for b in &mut grounded.sides {
            if let Boundary::Fixed(v) = b {
                *v = 0.0;
            }
        }
        let discrete = grounded.discrete()?;
        let n = self.conductors.len();
        let mut columns = Vec::with_capacity(n);
        for j in 0..n {
            let mut values = vec![0.0; n];
            values[j] = 1.0;
            columns.push(discrete.solve(&values, &[])?.held().to_vec());
        }
        Ok((0..n)
            .map(|i| (0..n).map(|j| columns[j][i]).collect())
            .collect())
    }
}

/// An electrostatic solution's field E at the cells' centres, V/m ([`Electrostatic::electric_field`]).
#[derive(Clone, Debug, PartialEq)]
pub struct ElectricField {
    grid: Grid,
    values: Vec<[f64; 2]>,
    coefficients: Vec<[f64; 2]>,
    held: Vec<bool>,
}

impl ElectricField {
    /// The grid.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Cell (i, j)'s field, V/m, across and up.
    pub fn value(&self, i: usize, j: usize) -> [f64; 2] {
        self.values[j * self.grid.x().cells() + i]
    }

    /// The field at (x, y), µm, V/m: bilinear between the four nearest cells' centres when they
    /// are all of the medium of the cell holding the point (the same coefficients, none inside
    /// a conductor), and that cell's own value otherwise, so an interface's two sides are never
    /// mixed. Beyond the outermost centres, the edge cells'.
    pub fn at(&self, x: f64, y: f64) -> [f64; 2] {
        let (gx, gy) = (self.grid.x(), self.grid.y());
        let nx = gx.cells();
        let own = gy.cell_of(y) * nx + gx.cell_of(x);
        let locate = |a: &Axis, v: f64| -> (usize, usize, f64) {
            let n = a.cells();
            if n == 1 || v <= a.centre(0) {
                return (0, 0, 0.0);
            }
            if v >= a.centre(n - 1) {
                return (n - 1, n - 1, 0.0);
            }
            let mut i = a.cell_of(v);
            if v < a.centre(i) {
                i -= 1;
            }
            (
                i,
                i + 1,
                (v - a.centre(i)) / (a.centre(i + 1) - a.centre(i)),
            )
        };
        let (i0, i1, s) = locate(gx, x);
        let (j0, j1, t) = locate(gy, y);
        let corners = [j0 * nx + i0, j0 * nx + i1, j1 * nx + i0, j1 * nx + i1];
        let alike = corners
            .iter()
            .all(|&c| !self.held[c] && self.coefficients[c] == self.coefficients[own]);
        if !alike {
            return self.values[own];
        }
        let v = |c: usize, k: usize| self.values[c][k];
        std::array::from_fn(|k| {
            (1.0 - t) * ((1.0 - s) * v(corners[0], k) + s * v(corners[1], k))
                + t * ((1.0 - s) * v(corners[2], k) + s * v(corners[3], k))
        })
    }
}

/// A current-continuity problem: contacts at potentials driving a current through conducting
/// media, and its Joule heat. See the module's docs.
#[derive(Clone, Debug)]
pub struct Current {
    section: Section,
    grid: Grid,
    conductivity: Vec<(String, Tensor)>,
    sides: [Boundary; 4],
    contacts: Vec<Conductor>,
}

impl Current {
    /// `section` on `grid`, every side insulated (no current through it), no contacts.
    pub fn new(section: Section, grid: Grid) -> Current {
        Current {
            section,
            grid,
            conductivity: Vec::new(),
            sides: [Boundary::INSULATED; 4],
            contacts: Vec::new(),
        }
    }

    /// The electrical conductivity of `medium`, S/m (a later call for the same medium wins).
    /// An insulator takes a conductivity many orders below the conductors'.
    pub fn conductivity(&mut self, medium: &str, sigma: Tensor) -> &mut Current {
        set(&mut self.conductivity, medium, sigma);
        self
    }

    /// What holds `side`.
    pub fn boundary(&mut self, side: Side, boundary: Boundary) -> &mut Current {
        self.sides[side_index(side)] = boundary;
        self
    }

    /// A contact at a potential; [`Flow::currents`] lists each contact's current, in the order
    /// added.
    pub fn contact(&mut self, contact: Conductor) -> &mut Current {
        self.contacts.push(contact);
        self
    }

    /// The grid.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// The potential, the contacts' currents and the Joule heat.
    ///
    /// # Errors
    ///
    /// As [`Electrostatic::solve`], for conductivities.
    pub fn solve(&self) -> Result<Flow> {
        for c in &self.contacts {
            c.check()?;
        }
        let coefficients = self.section.coefficients(
            &self.grid,
            |m| table(&self.conductivity, m),
            "electrical conductivity",
        )?;
        let discrete = Discrete::new(&Problem {
            grid: &self.grid,
            coefficients,
            sides: self.sides,
            holds: self.contacts.iter().map(|c| c.hold.clone()).collect(),
            profile: None,
        })?;
        let values: Vec<f64> = self.contacts.iter().map(|c| c.potential).collect();
        let field = discrete.solve(&values, &[])?;
        let joule = discrete.dissipation(&field);
        Ok(Flow { field, joule })
    }
}

/// A current's solution: the potential, each contact's current and each cell's Joule heat.
#[derive(Clone, Debug, PartialEq)]
pub struct Flow {
    field: Field,
    joule: Vec<f64>,
}

impl Flow {
    /// The potential, V.
    pub fn field(&self) -> &Field {
        &self.field
    }

    /// The current each contact drives into the window, A/m.
    pub fn currents(&self) -> &[f64] {
        self.field.held()
    }

    /// Each cell's Joule heat, W/m: every face's g ΔV², shared between the cells either side
    /// (all of it to the free cell at a contact, which dissipates nothing).
    pub fn joule(&self) -> &[f64] {
        &self.joule
    }

    /// The Joule heat in all, W/m: Σ V I over the contacts, to round-off.
    pub fn power(&self) -> f64 {
        self.joule.iter().sum()
    }
}
