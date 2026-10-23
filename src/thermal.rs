//! Heat conduction on a waveguide's cross-section: −∇·(k∇T) = Q, steady, and in time,
//! ρc_p ∂T/∂t = ∇·(k∇T) + Q ([`Heat::transient`], [`Heat::response`]), per unit length along
//! the guide.
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

use num_complex::Complex64 as c64;

use crate::geometry::Shape;
use crate::statics::{Discrete, Hold, Problem, spread};
use crate::{Error, Result};

pub use crate::statics::{
    Axis, Boundary, Cut, Features, Field, Grading, Grid, MAX_CELLS, Section, Side, Tensor,
};
pub use rc::{Cauer, Fitted, Foster, RcFit, RcModel};
pub use shifter::{Drive, Efficiency, Guide, Optics, PhaseShifter, StepResponses, Timing};

pub(crate) mod crosstalk_checks;
mod rc;
pub(crate) mod rc_checks;
#[cfg(test)]
mod rc_tests;
mod shifter;
pub(crate) mod shifter_checks;
#[cfg(test)]
mod shifter_tests;

/// A heat-conduction problem on a cross-section, steady or in time. See the module's docs.
#[derive(Clone, Debug)]
pub struct Heat {
    section: Section,
    grid: Grid,
    conductivity: Vec<(String, Tensor)>,
    sides: [Boundary; 4],
    sinks: Vec<(Shape, f64)>,
    heaters: Vec<(Shape, f64)>,
    extra: Option<Vec<f64>>,
    capacity: Vec<(String, f64)>,
    /// Heaters given cell by cell (W/m in each), which follow the heaters' factor as they do:
    /// a phase shifter's drive of a current's Joule heat ([`shifter`]).
    driven: Vec<Vec<f64>>,
}

/// How [`Heat::transient`] steps in time. Both are implicit, unconditionally stable and second
/// order; each factorizes its one matrix once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Scheme {
    /// The second-order backward differentiation formula, its first step by backward Euler:
    /// (3/2 M/Δt + K) Tⁿ⁺¹ = M (2Tⁿ − Tⁿ⁻¹/2)/Δt + bⁿ⁺¹. L-stable: a step's sharp start is
    /// damped, not carried along as an oscillation.
    Bdf2,
    /// Crank–Nicolson, (2M/Δt + K) Tⁿ⁺¹ = (2M/Δt − K) Tⁿ + bⁿ + bⁿ⁺¹, its first step taken as two
    /// backward-Euler half steps on the same matrix (R. Rannacher, Numer. Math. 43, 309 (1984),
    /// doi:10.1007/BF01390130), which damps the start's oscillation.
    CrankNicolson,
}

/// A transient's time steps.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Stepping {
    /// The step, seconds.
    pub step_s: f64,
    /// How many.
    pub steps: usize,
    /// The scheme.
    pub scheme: Scheme,
}

/// A step of [`Heat::transient_observed`], as its observer sees it.
pub struct Step<'a> {
    /// The step's number, from 1.
    pub index: usize,
    /// Its time, seconds.
    pub time_s: f64,
    /// Each watched shape's mean temperature.
    pub means: Vec<f64>,
    discrete: &'a Discrete,
    free: &'a [f64],
    values: &'a [f64],
    source: f64,
}

impl Step<'_> {
    /// The temperature at this step.
    pub fn field(&self) -> Field {
        self.discrete.field(self.free, self.values, self.source)
    }
}

/// A transient's record: each watched shape's mean temperature at each time, and the last
/// field.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct History {
    /// The times, seconds: 0 (the start's steady state), then each step's.
    pub times_s: Vec<f64>,
    /// Each watched shape's area-weighted mean temperature, at each time.
    pub watched: Vec<Vec<f64>>,
    /// The field at the last time.
    pub last: Field,
}

impl Stepping {
    /// `steps` steps of `step_s` seconds by `scheme` (checked when a transient takes them).
    pub fn new(step_s: f64, steps: usize, scheme: Scheme) -> Stepping {
        Stepping {
            step_s,
            steps,
            scheme,
        }
    }
}

impl History {
    /// When watched shape `watch`'s mean first gets `fraction` of the way from `from` to
    /// `to`, seconds, between steps by linear interpolation; `None` if it doesn't within the
    /// record. The 1/e time of a step from `from` to `to` is `fraction` = 1 − 1/e; its
    /// 10–90 % time the difference of 0.9's and 0.1's.
    pub fn crossing(&self, watch: usize, from: f64, to: f64, fraction: f64) -> Option<f64> {
        let values = self.watched.get(watch)?;
        let target = from + fraction * (to - from);
        let above = |v: f64| (v - target) * (to - from) >= 0.0;
        values.windows(2).enumerate().find_map(|(k, w)| {
            if above(w[0]) {
                return (k == 0).then_some(self.times_s[0]);
            }
            above(w[1]).then(|| {
                let s = (target - w[0]) / (w[1] - w[0]);
                self.times_s[k] + s * (self.times_s[k + 1] - self.times_s[k])
            })
        })
    }
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
            capacity: Vec::new(),
            driven: Vec::new(),
        }
    }

    /// The heat capacity per volume of `medium`, ρ c_p in J/(m³·K), for [`Heat::transient`]
    /// and [`Heat::response`] (a later call for the same medium wins).
    pub fn heat_capacity(&mut self, medium: &str, rho_cp: f64) -> &mut Heat {
        self.capacity.retain(|(m, _)| m != medium);
        self.capacity.push((medium.to_owned(), rho_cp));
        self
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
        self.steady_at(1.0)
    }

    /// The steady temperature with the heaters' power times `factor`: where a transient that
    /// ends driven at `factor` goes (0 for a turn-off's end).
    ///
    /// # Errors
    ///
    /// As [`Heat::solve`], and [`Error::InvalidValue`] for a factor that isn't finite.
    pub fn steady_at(&self, factor: f64) -> Result<Field> {
        if !factor.is_finite() {
            return Err(Error::invalid(
                "static problem",
                format!("the heaters' factor must be finite, got {factor}"),
            ));
        }
        let Assembled {
            discrete,
            base,
            heat,
            values,
        } = self.discretize()?;
        let sources: Vec<f64> = base
            .iter()
            .zip(&heat)
            .map(|(b, h)| b + factor * h)
            .collect();
        discrete.solve(&values, &sources)
    }

    /// The temperature from the steady state the heaters' power times `start` gives, the
    /// heaters' power then `drive(t)` times its own from t = 0 (t in seconds), every held
    /// region and side as the problem holds them: implicit steps of `stepping`, recording each
    /// shape of `watch`'s mean at every step. A turn-on is `start` 0 and `drive` 1; a turn-off
    /// `start` 1 and `drive` 0.
    ///
    /// # Errors
    ///
    /// As [`Heat::solve`], and [`Error::InvalidValue`] if a medium has no heat capacity, the
    /// step isn't positive and finite, there are no steps, `drive` gives a value that isn't
    /// finite, or a watched shape covers no cell.
    pub fn transient(
        &self,
        stepping: &Stepping,
        start: f64,
        drive: &dyn Fn(f64) -> f64,
        watch: &[Shape],
    ) -> Result<History> {
        self.transient_observed(stepping, start, drive, watch, &mut |_| Ok(true))
    }

    /// [`Heat::transient`], `observe` given each step as it is taken: its time, the watched
    /// shapes' means and its field ([`Step::field`]). The steps end early, the history with
    /// them, when `observe` returns false (a run stopped).
    ///
    /// # Errors
    ///
    /// As [`Heat::transient`], and `observe`'s own.
    pub fn transient_observed(
        &self,
        stepping: &Stepping,
        start: f64,
        drive: &dyn Fn(f64) -> f64,
        watch: &[Shape],
        observe: &mut dyn FnMut(&Step<'_>) -> Result<bool>,
    ) -> Result<History> {
        let parts = watch
            .iter()
            .map(|s| shape_parts(&self.grid, s))
            .collect::<Result<Vec<_>>>()?;
        self.transient_weighted(stepping, start, drive, &parts, observe)
    }

    /// [`Heat::transient_observed`], each watch a weighted sum of cells' temperatures,
    /// (cell, weight) in the grid's order of cells, in place of a shape's mean.
    pub(crate) fn transient_weighted(
        &self,
        stepping: &Stepping,
        start: f64,
        drive: &dyn Fn(f64) -> f64,
        watch: &[Vec<(usize, f64)>],
        observe: &mut dyn FnMut(&Step<'_>) -> Result<bool>,
    ) -> Result<History> {
        let dt = stepping.step_s;
        if !(dt.is_finite() && dt > 0.0) || stepping.steps == 0 || !start.is_finite() {
            return Err(Error::invalid(
                "static problem",
                format!(
                    "a transient needs a positive step and at least one, and a finite start: {} s, {} steps, start {start}",
                    dt, stepping.steps
                ),
            ));
        }
        let Assembled {
            discrete,
            base,
            heat,
            values,
        } = self.discretize()?;
        let mass = self.mass(&discrete)?;
        let watches = watch
            .iter()
            .map(|p| Watch::new(&self.grid, &discrete, &values, p))
            .collect::<Result<Vec<_>>>()?;
        let sources = |factor: f64| -> Vec<f64> {
            base.iter()
                .zip(&heat)
                .map(|(b, h)| b + factor * h)
                .collect()
        };
        let load = |t: f64| -> Result<(Vec<f64>, f64)> {
            let f = drive(t);
            if !f.is_finite() {
                return Err(Error::invalid(
                    "static problem",
                    format!("a transient's drive must be finite, got {f} at {t} s"),
                ));
            }
            Ok(discrete.load(&values, &sources(f), true))
        };
        // the start: the steady state at `start`
        let steady = discrete.solve(&values, &sources(start))?;
        let free = discrete.free_cells();
        let mut x: Vec<f64> = free.iter().map(|&c| steady.values()[c]).collect();
        let mut times = vec![0.0];
        let mut watched: Vec<Vec<f64>> = watches.iter().map(|w| vec![w.mean(&x)]).collect();
        let mut source = steady.source();
        let solve = |f: &crate::sparse::Multifrontal, rhs: &[f64]| -> Vec<f64> {
            let b: Vec<c64> = rhs.iter().map(|&v| c64::new(v, 0.0)).collect();
            f.solve(&b).iter().map(|v| v.re).collect()
        };
        let factor = |a: f64| -> Result<Option<crate::sparse::Multifrontal>> {
            let shift: Vec<c64> = mass.iter().map(|m| c64::new(a * m / dt, 0.0)).collect();
            discrete.shifted(&shift)
        };
        if discrete.unknowns() > 0 {
            // BDF2: backward Euler's matrix for the first step, its own after; Crank–Nicolson:
            // one matrix, its first step two backward-Euler half steps on it
            let (first, then) = match stepping.scheme {
                Scheme::Bdf2 => (factor(1.0)?, factor(1.5)?),
                Scheme::CrankNicolson => (None, factor(2.0)?),
            };
            let then = then.ok_or_else(no_free_cells)?;
            let mut previous = x.clone();
            for n in 1..=stepping.steps {
                let t = n as f64 * dt;
                let (b1, total) = load(t)?;
                source = total;
                let next = match (stepping.scheme, n) {
                    (Scheme::Bdf2, 1) => {
                        let rhs: Vec<f64> =
                            (0..x.len()).map(|u| b1[u] + mass[u] / dt * x[u]).collect();
                        solve(first.as_ref().ok_or_else(no_free_cells)?, &rhs)
                    }
                    (Scheme::Bdf2, _) => {
                        let rhs: Vec<f64> = (0..x.len())
                            .map(|u| b1[u] + mass[u] / dt * (2.0 * x[u] - 0.5 * previous[u]))
                            .collect();
                        solve(&then, &rhs)
                    }
                    (Scheme::CrankNicolson, 1) => {
                        // (2M/Δt + K) T = 2M/Δt T + b, twice
                        let mut y = x.clone();
                        for half in [0.5, 1.0] {
                            let (b, _) = load(half * dt)?;
                            let rhs: Vec<f64> = (0..y.len())
                                .map(|u| b[u] + 2.0 * mass[u] / dt * y[u])
                                .collect();
                            y = solve(&then, &rhs);
                        }
                        y
                    }
                    (Scheme::CrankNicolson, _) => {
                        let (b0, _) = load((n - 1) as f64 * dt)?;
                        let kx = discrete.apply(&x);
                        let rhs: Vec<f64> = (0..x.len())
                            .map(|u| 2.0 * mass[u] / dt * x[u] - kx[u] + b0[u] + b1[u])
                            .collect();
                        solve(&then, &rhs)
                    }
                };
                previous = std::mem::replace(&mut x, next);
                times.push(t);
                let means: Vec<f64> = watches.iter().map(|w| w.mean(&x)).collect();
                for (list, m) in watched.iter_mut().zip(&means) {
                    list.push(*m);
                }
                let step = Step {
                    index: n,
                    time_s: t,
                    means,
                    discrete: &discrete,
                    free: &x,
                    values: &values,
                    source,
                };
                if !observe(&step)? {
                    break;
                }
            }
        }
        Ok(History {
            times_s: times,
            watched,
            last: discrete.field(&x, &values, source),
        })
    }

    /// The small-signal response at `frequency_hz` of `watch`'s mean temperature to the
    /// heaters' power, against its steady response: T̂(f) / T̂(0), from (K + i2πf M) T̂ = Q̂ on the
    /// same grid (the held regions and sides holding the fluctuation at 0). Its magnitude is
    /// 1/√2 at the −3 dB frequency ([`Heat::cutoff`]).
    ///
    /// # Errors
    ///
    /// As [`Heat::transient`], and [`Error::InvalidValue`] for a frequency that isn't finite and
    /// non-negative, or no heater (nothing to respond to).
    pub fn response(&self, frequency_hz: f64, watch: &Shape) -> Result<c64> {
        self.responder(&shape_parts(&self.grid, watch)?)?(frequency_hz)
    }

    /// [`Heat::response`] at any frequency, the problem assembled once, of a weighted sum of
    /// cells' temperatures (as [`Heat::transient_weighted`]'s).
    pub(crate) fn responder(&self, watch: &[(usize, f64)]) -> Result<impl Fn(f64) -> Result<c64>> {
        let Assembled {
            discrete,
            heat,
            values,
            ..
        } = self.discretize()?;
        let mass = self.mass(&discrete)?;
        let w = Watch::new(&self.grid, &discrete, &values, watch)?;
        let (q, total) = discrete.load(&values, &heat, false);
        if total == 0.0 || discrete.unknowns() == 0 {
            return Err(Error::invalid(
                "static problem",
                "a response needs a heater, and a cell free to respond",
            ));
        }
        let b: Vec<c64> = q.iter().map(|&v| c64::new(v, 0.0)).collect();
        let at = move |f: f64| -> Result<c64> {
            let omega = 2.0 * std::f64::consts::PI * f;
            let shift: Vec<c64> = mass.iter().map(|m| c64::new(0.0, omega * m)).collect();
            let factors = discrete.shifted(&shift)?.ok_or_else(no_free_cells)?;
            Ok(w.mean_complex(&factors.solve(&b)))
        };
        let dc = at(0.0)?;
        Ok(move |f: f64| -> Result<c64> {
            if !(f.is_finite() && f >= 0.0) {
                return Err(Error::invalid(
                    "static problem",
                    format!("a frequency must be finite and non-negative, got {f}"),
                ));
            }
            Ok(at(f)? / dc)
        })
    }

    /// The −3 dB frequency of `watch`'s mean temperature's response to the heaters' power, Hz:
    /// where |[`Heat::response`]| falls to 1/√2, by bisection in log f between 1 Hz and 1 THz to
    /// 1e-6 of the frequency.
    ///
    /// # Errors
    ///
    /// As [`Heat::response`], and [`Error::InvalidValue`] if the response doesn't fall to 1/√2
    /// in that range.
    pub fn cutoff(&self, watch: &Shape) -> Result<f64> {
        self.cutoff_weighted(&shape_parts(&self.grid, watch)?)
    }

    /// [`Heat::cutoff`] of a weighted sum of cells' temperatures.
    pub(crate) fn cutoff_weighted(&self, watch: &[(usize, f64)]) -> Result<f64> {
        let half = std::f64::consts::FRAC_1_SQRT_2;
        let response = self.responder(watch)?;
        let below = |f: f64| -> Result<bool> { Ok(response(f)?.norm() < half) };
        let (mut lo, mut hi) = (1.0f64, 1e12f64);
        if below(lo)? || !below(hi)? {
            return Err(Error::invalid(
                "static problem",
                "the response doesn't fall to 1/√2 between 1 Hz and 1 THz",
            ));
        }
        while hi / lo > 1.0 + 1e-6 {
            let mid = (lo * hi).sqrt();
            if below(mid)? {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        Ok((lo * hi).sqrt())
    }

    /// The problem assembled and factorized: its discrete form, the sources that don't follow
    /// the heaters (a current's Joule heat), the heaters' own, and the sinks' temperatures.
    fn discretize(&self) -> Result<Assembled> {
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
        let base = self
            .extra
            .clone()
            .unwrap_or_else(|| vec![0.0; self.grid.cells()]);
        let mut heat = vec![0.0; self.grid.cells()];
        for (shape, power) in &self.heaters {
            if !power.is_finite() {
                return Err(Error::invalid(
                    "static problem",
                    format!("a heater's power must be finite, got {power}"),
                ));
            }
            spread(&self.grid, shape, *power, &mut heat)?;
        }
        for cells in &self.driven {
            if cells.len() != heat.len() {
                return Err(Error::invalid(
                    "static problem",
                    "a heater given cell by cell is on another grid",
                ));
            }
            for (h, q) in heat.iter_mut().zip(cells) {
                *h += q;
            }
        }
        let values: Vec<f64> = self.sinks.iter().map(|s| s.1).collect();
        Ok(Assembled {
            discrete,
            base,
            heat,
            values,
        })
    }

    /// Each free cell's heat capacity per unit length, J/(m·K): its media's ρ c_p by volume,
    /// times its area.
    fn mass(&self, discrete: &Discrete) -> Result<Vec<f64>> {
        let capacity = self.section.means(
            &self.grid,
            |m| self.capacity.iter().find(|(n, _)| n == m).map(|(_, c)| *c),
            "heat capacity",
        )?;
        let nx = self.grid.x().cells();
        Ok(discrete
            .free_cells()
            .iter()
            .map(|&c| {
                let area = self.grid.x().width(c % nx) * self.grid.y().width(c / nx) * 1e-12;
                capacity[c] * area
            })
            .collect())
    }
}

/// A heat problem assembled: its discrete form, the sources that don't follow the heaters (a
/// current's Joule heat), the heaters' own, and the sinks' temperatures.
struct Assembled {
    discrete: Discrete,
    base: Vec<f64>,
    heat: Vec<f64>,
    values: Vec<f64>,
}

/// A shifted operator asked for where no cell is free (which the callers rule out first).
fn no_free_cells() -> Error {
    Error::invalid("static problem", "no cell is free to step or respond")
}

/// A watched shape's mean, from the free cells' values: each cell's share of its area, the
/// held cells at their sinks' temperatures.
struct Watch {
    /// (free unknown, weight) and the held cells' weighted part
    free: Vec<(usize, f64)>,
    held: f64,
}

/// A shape's mean as weights: each cell it covers, its share of the shape's area.
fn shape_parts(grid: &Grid, shape: &Shape) -> Result<Vec<(usize, f64)>> {
    let parts = grid.overlap(shape);
    let area: f64 = parts.iter().map(|p| p.1).sum();
    if area <= 0.0 {
        return Err(Error::invalid(
            "static problem",
            "a watched shape covers no cell of the grid",
        ));
    }
    Ok(parts.into_iter().map(|(c, a)| (c, a / area)).collect())
}

impl Watch {
    /// The sum of `parts`' cells' values, each times its weight.
    fn new(
        grid: &Grid,
        discrete: &Discrete,
        values: &[f64],
        parts: &[(usize, f64)],
    ) -> Result<Watch> {
        if parts
            .iter()
            .any(|&(c, w)| c >= grid.cells() || !w.is_finite())
        {
            return Err(Error::invalid(
                "static problem",
                "a watch's cells must be the grid's, its weights finite",
            ));
        }
        let free_cells = discrete.free_cells();
        let mut unknown = vec![None; grid.cells()];
        for (u, &c) in free_cells.iter().enumerate() {
            unknown[c] = Some(u);
        }
        // a held cell's value is its sink's, which a field at zero sources gives
        let zero = discrete.field(&vec![0.0; free_cells.len()], values, 0.0);
        let (mut free, mut held) = (Vec::new(), 0.0);
        for &(c, w) in parts {
            match unknown[c] {
                Some(u) => free.push((u, w)),
                None => held += zero.values()[c] * w,
            }
        }
        Ok(Watch { free, held })
    }

    fn mean(&self, x: &[f64]) -> f64 {
        self.held + self.free.iter().map(|&(u, w)| w * x[u]).sum::<f64>()
    }

    /// For a fluctuation: the held cells don't fluctuate.
    fn mean_complex(&self, x: &[c64]) -> c64 {
        self.free.iter().map(|&(u, w)| x[u] * w).sum()
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
