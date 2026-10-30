//! Adjoint gradients from FDTD runs: how an objective on the monitors changes with the density
//! of every cell of a design region, from two runs, the forward and the adjoint.
//!
//! - **The frequency-domain system a run solves.** Once its fields have died away, a run's
//!   transforms at ω (E at nΔt, H̃ at (n + ½)Δt, the currents at the times they enter) solve,
//!   exactly,
//!
//!   iω̃ Ĥ − C_E Ê = M̂,   −iω̃ ε Ê − C_H Ĥ = −Ĵ,   ω̃ = (2/Δt) sin(ωΔt/2),
//!
//!   K(ε) x = b for x = (Ê, Ĥ), with C_E and C_H the grid's curls, the CPML's convolutions
//!   turning each difference along an axis w into one over s̃_w(ω), 1/s̃ = 1/κ + a/(1 − b e^(iωΔt))
//!   (the recursion ψ ← bψ + a∂F transformed). Conductivity, dispersive media and smoothed
//!   tensors stay on K's diagonal blocks, symmetric.
//! - **Its transpose is a run.** Yee's curls are each other's transposes, C_H = C_Eᵀ, and each
//!   stretched curl factorizes as Λ C S, S the stretch along each component's own axis and Λ the
//!   other two's. So with D the product of the three stretches at each value of E and H
//!   (s̃_c at E_c's half step along c times s̃ at the nodes across; at H_c the other way round),
//!   Kᵀ D = D K, and outside the CPMLs D = 1: the adjoint system Kᵀ μ = g is solved by a run
//!   whose sources are D⁻¹g, μ = D y.
//! - **The gradient.** For a real objective F of the transforms, dF = 2 Re(gᵀ dx) with g = ∂F/∂x
//!   (the Wirtinger derivative, x̄ held), and K's only entry with ε_r in it is −iω̃ε_r, so
//!
//!   ∂F/∂ε_r = −2 Re(μᵀ (∂K/∂ε_r) x) = 2 Re(iω̃ Ê_adj,r Ê_r),
//!
//!   summed over the objective's frequencies: G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29,
//!   2288 (2004), doi:10.1364/OL.29.002288, Eqs. 2–4, and C. M. Lalau-Keraly, S. Bhargava,
//!   O. D. Miller, E. Yablonovitch, Opt. Express 21, 21693 (2013), doi:10.1364/OE.21.021693,
//!   Eq. 5, Re(E_adj · E_old), from the transforms of both runs over the design region. The adjoint
//!   run's sources are J = −g_E and M = g_H at the monitors (T. W. Hughes, M. Minkov,
//!   I. A. D. Williamson, S. Fan, ACS Photonics 5, 4781 (2018), doi:10.1021/acsphotonics.8b01522,
//!   Eqs. 9–13, give the same structure for FDFD).
//! - **The objective's derivative.** A mode's amplitude is linear in Ê, a = Σ ωᵣ Êᵣ by FDFD's
//!   Lorentz form, so g = (∂F/∂a) ω with the mode's whole complex profile and the form's phase:
//!   nothing of the mode's imaginary part is dropped. A flux ½ Re(E × H̃*) gives g_E = ½κH̄ and
//!   g_H = ½κĒ at the values its sum pairs. A transform's value gives itself.
//! - **Exact transforms from a real run.** The adjoint's sources must have exactly the transforms
//!   g at each frequency of the objective, complex. Each value takes Σₘ βₘ wₘ(t) with real
//!   amplitudes, two Gaussian pulses e^(−u²) sin ω(t − d) on each frequency a quarter period
//!   apart: with Wₘ their transforms at the source's own times, the real system
//!   Σₘ βₘ Wₘ(ωⱼ) = target_j gives the β, the negative frequencies' image included. Each pulse
//!   is odd about its peak, so its mean is zero and it leaves no charge, whose static field
//!   would never die away.
//! - **Memory.** Nothing of a run's history is kept or recomputed: no checkpointing. Each run
//!   keeps the transforms of E over the design's values at the objective's frequencies, and the
//!   gradient is their product. The cost is the adjoint run, the forward's twin: about one
//!   more forward run's time and memory (docs/methods/fdtd-adjoint.md).
//!
//! **The design.** A box of cells, each with a density ρ and the permittivity
//! ε(ρ) = ε₀ + ρ(ε₁ − ε₀). Each value of E sees it averaged over its cell as
//! [`Simulation::new`] averages any permittivity (FDFD's average, harmonic along the component
//! and arithmetic across it): along its component its cell lies in one design cell, so its
//! permittivity is linear in the densities, each design cell's weight its share of the cell
//! across. The gradient goes through that average, the forward run's own.

use std::collections::BTreeMap;
use std::ops::Range;

use num_complex::Complex64 as c64;

use super::monitors::{Dft, Series};
use super::{Field, SAMPLES, Simulation, Waveform, averaged, invalid, kernel};
use crate::Result;
use crate::fdfd::{Axis, Direction, Edges, Grid3d};
use crate::units::Frequency;

/// A design region: a box of the grid's cells, each with a density ρ, whose permittivity
/// ε(ρ) = ε₀ + ρ (ε₁ − ε₀) is what an optimization varies.
#[derive(Clone, Debug, PartialEq)]
pub struct Design {
    /// The cells along x, y and z: cell i spans nodes i to i + 1.
    pub cells: [Range<usize>; 3],
    /// The relative permittivity at density 0 and at density 1.
    pub eps: [f64; 2],
}

/// A value of E whose cell reaches a design's cells, and each design cell's share of it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Touched {
    pub(super) component: Axis,
    pub(super) at: [usize; 3],
    /// Its index within the component.
    pub(super) r: usize,
    /// (design cell, share): ∂ε/∂ε(cell).
    pub(super) shares: Vec<(usize, f64)>,
}

impl Design {
    /// The number of design cells: one density each, i fastest, then j, then k.
    pub fn len(&self) -> usize {
        self.cells.iter().map(|r| r.len()).product()
    }

    /// Whether the region has no cells.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The density's index of cell (i, j, k), if it is in the region.
    pub fn index(&self, (i, j, k): (usize, usize, usize)) -> Option<usize> {
        let [x, y, z] = &self.cells;
        if x.contains(&i) && y.contains(&j) && z.contains(&k) {
            Some(((k - z.start) * y.len() + (j - y.start)) * x.len() + (i - x.start))
        } else {
            None
        }
    }

    /// The permittivity at density `rho`, ε₀ + ρ (ε₁ − ε₀).
    pub fn permittivity(&self, rho: f64) -> f64 {
        self.eps[0] + rho * (self.eps[1] - self.eps[0])
    }

    /// Checks the region against `s`'s grid and boundaries: on the grid, clear of the CPMLs
    /// with the values of E its cells reach, and its permittivities finite and positive.
    fn check(&self, s: &Simulation) -> Result<()> {
        let g = s.grid;
        if !self.eps.iter().all(|e| e.is_finite() && *e > 0.0) {
            return Err(invalid(format!(
                "a design's permittivities must be finite and positive, got {:?}",
                self.eps
            )));
        }
        for axis in Axis::ALL {
            let range = &self.cells[axis.index()];
            let n = g.n(axis);
            if range.is_empty() || range.end > n {
                return Err(invalid(format!(
                    "a design's cells along {axis:?} must be a nonempty range within the grid's \
                     {n}, got {range:?}"
                )));
            }
            if let Edges::Pml { low, high } = s.boundaries.edges(axis)
                && (low > 0 || high > 0)
                && (range.start < low || range.end + high >= n)
            {
                return Err(invalid(format!(
                    "a design's cells along {axis:?}, {range:?}, and the values of E they reach \
                     must be clear of the CPMLs ({low} and {high} of {n} cells)"
                )));
            }
        }
        Ok(())
    }

    /// Checks `density`: one finite value per cell, each giving a positive permittivity.
    fn check_density(&self, density: &[f64]) -> Result<()> {
        if density.len() != self.len() {
            return Err(invalid(format!(
                "a design of {} cells needs as many densities, got {}",
                self.len(),
                density.len()
            )));
        }
        if let Some(bad) = density
            .iter()
            .find(|&&rho| !(rho.is_finite() && self.permittivity(rho) > 0.0))
        {
            return Err(invalid(format!(
                "a density must be finite and give a positive permittivity, got {bad}"
            )));
        }
        Ok(())
    }

    /// The design cell holding the point `p`, across a periodic side wrapped onto the grid.
    fn cell_at(&self, grid: &Grid3d, periodic: [bool; 3], p: [f64; 3]) -> Option<usize> {
        let mut at = [0usize; 3];
        for axis in Axis::ALL {
            let a = axis.index();
            let n = grid.n(axis) as i64;
            let m = ((p[a] - grid.node(axis, 0)) / grid.step(axis)).floor() as i64;
            let m = if periodic[a] {
                m.rem_euclid(n)
            } else if (0..n).contains(&m) {
                m
            } else {
                return None;
            };
            at[a] = m as usize;
        }
        self.index((at[0], at[1], at[2]))
    }

    /// The relative permittivity with `density` in the design's cells and `background`
    /// elsewhere.
    fn composed<'a>(
        &'a self,
        grid: &'a Grid3d,
        periodic: [bool; 3],
        density: &'a [f64],
        background: &'a dyn Fn(f64, f64, f64) -> f64,
    ) -> impl Fn(f64, f64, f64) -> f64 + 'a {
        move |x, y, z| match self.cell_at(grid, periodic, [x, y, z]) {
            Some(v) => self.permittivity(density[v]),
            None => background(x, y, z),
        }
    }

    /// Every value of E whose cell (as [`Simulation::new`] averages over it) reaches the
    /// design's cells, with each design cell's share of the cell: along its component the cell
    /// is within one design cell, so the permittivity it sees is the shares' sum of the design
    /// cells' and the background's average over the rest.
    pub(super) fn values(&self, grid: &Grid3d, periodic: [bool; 3]) -> Vec<Touched> {
        let n = SAMPLES as f64;
        let offset = |s: usize| (s as f64 + 0.5) / n - 0.5;
        let h = [grid.dx, grid.dy, grid.dz];
        let mut out = Vec::new();
        for component in Axis::ALL {
            let c = component.index();
            // the indices along each axis whose cells can reach the design's
            let candidates: [Vec<usize>; 3] = std::array::from_fn(|a| {
                let range = &self.cells[a];
                let len = grid.n(Axis::ALL[a]);
                let mut set = std::collections::BTreeSet::new();
                let last = if a == c { range.end - 1 } else { range.end };
                for m in range.start..=last {
                    if m < len {
                        set.insert(m);
                    } else if periodic[a] {
                        set.insert(m % len);
                    }
                }
                set.into_iter().collect()
            });
            let (b_axis, c_axis) = component.others();
            let (b, cc) = (b_axis.index(), c_axis.index());
            for &k in &candidates[2] {
                for &j in &candidates[1] {
                    for &i in &candidates[0] {
                        let at = [i, j, k];
                        let centre = grid.e_position(component, (i, j, k));
                        let mut shares: BTreeMap<usize, f64> = BTreeMap::new();
                        let mut p = centre;
                        for sb in 0..SAMPLES {
                            p[b] = centre[b] + offset(sb) * h[b];
                            for sc in 0..SAMPLES {
                                p[cc] = centre[cc] + offset(sc) * h[cc];
                                // along the component the cell is within one design cell
                                p[c] = centre[c];
                                if let Some(v) = self.cell_at(grid, periodic, p) {
                                    *shares.entry(v).or_default() += 1.0 / (n * n);
                                }
                            }
                        }
                        if !shares.is_empty() {
                            out.push(Touched {
                                component,
                                at,
                                r: (k * grid.ny + j) * grid.nx + i,
                                shares: shares.into_iter().collect(),
                            });
                        }
                    }
                }
            }
        }
        out
    }
}

/// A design region's transforms of E over the values its cells reach, for a gradient.
#[derive(Clone, Debug)]
pub(super) struct DesignMonitor {
    pub(super) design: Design,
    pub(super) touched: Vec<Touched>,
    pub(super) dft: Dft,
}

impl DesignMonitor {
    /// The same monitor with its transforms at zero.
    fn at_rest(&self) -> DesignMonitor {
        let mut d = self.clone();
        d.dft.clear();
        d
    }

    /// The transform at frequency `f` of the value `t` touches.
    pub(super) fn value(&self, t: &Touched, f: usize) -> c64 {
        self.dft
            .value(Field::E, t.component, (t.at[0], t.at[1], t.at[2]), f)
            .expect("the design monitor's transforms")
    }
}

/// An objective's derivative with respect to the permittivity at one value of E.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueGradient {
    /// The value's component.
    pub component: Axis,
    /// Its index (i, j, k).
    pub at: (usize, usize, usize),
    /// ∂F/∂ε there.
    pub derivative: f64,
}

/// What an objective's derivative is with respect to: one measurement of a monitor, and the
/// objective's derivative with respect to it. For a real objective F of a complex measurement
/// a, the derivative is the Wirtinger one, ∂F/∂a with ā held: dF = 2 Re((∂F/∂a) da), so for
/// F = |a|² it is ā.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Term {
    /// A mode monitor's amplitude ([`Simulation::mode_amplitudes`]): monitor `monitor`'s mode
    /// number `mode`, going `direction`, at the monitor's frequency for it.
    Mode {
        /// The mode monitor.
        monitor: usize,
        /// The mode, among the monitor's.
        mode: usize,
        /// Forward or backward.
        direction: Direction,
        /// ∂F/∂a.
        derivative: c64,
    },
    /// A flux monitor's flux ([`Simulation::flux`]) at its frequency number `frequency`.
    Flux {
        /// The flux monitor.
        monitor: usize,
        /// The frequency's number among the monitor's.
        frequency: usize,
        /// dF/dP, real.
        derivative: f64,
    },
    /// A transform monitor's value ([`super::Dft::value`]) of `field`'s `component` at `at`,
    /// at its frequency number `frequency`.
    Transform {
        /// The transform monitor ([`Simulation::add_dft`]).
        monitor: usize,
        /// E or H.
        field: Field,
        /// The component.
        component: Axis,
        /// The value's index (i, j, k).
        at: (usize, usize, usize),
        /// The frequency's number among the monitor's.
        frequency: usize,
        /// ∂F/∂(the value).
        derivative: c64,
    },
}

/// An objective's derivative with respect to each transform a term reads: (field, component,
/// index within the component) and ∂F/∂(value).
type Derivatives = Vec<((Field, usize, usize), c64)>;

/// An adjoint source's transform at one value: field, component, index within the component.
type Key = (u8, usize, usize);

fn key(field: Field, component: usize, r: usize) -> Key {
    (u8::from(field == Field::H), component, r)
}

impl Simulation {
    /// This problem with `design`'s cells at `density` (one value per cell, as
    /// [`Design::index`] numbers them), the permittivity of every value of E whose cell reaches
    /// them averaged as [`Simulation::new`] averages, over `background` outside the design and
    /// ε(ρ) inside. Values whose cells don't reach the design keep the permittivity they have,
    /// smoothed or not. Before the run starts.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] for a design off the grid or reaching into a CPML, a
    /// density of the wrong length or one that gives a permittivity that isn't positive, after
    /// the run has started, with a Bloch phase, or where a value the design reaches has a
    /// conductivity, a dispersive medium or a smoothed tensor that couples it to the others.
    pub fn with_design(
        mut self,
        design: &Design,
        density: &[f64],
        background: impl Fn(f64, f64, f64) -> f64,
    ) -> Result<Simulation> {
        design.check(&self)?;
        design.check_density(density)?;
        if self.steps > 0 {
            return Err(invalid("a design is set before the run starts"));
        }
        if self.bloch.is_some() {
            return Err(invalid(
                "a design's gradient needs a reciprocal problem: no Bloch phase",
            ));
        }
        let grid = self.grid;
        let periodic = Axis::ALL.map(|a| self.boundaries.periodic(a));
        let eps = design.composed(&grid, periodic, density, &background);
        let h = [grid.dx, grid.dy, grid.dz];
        for t in design.values(&grid, periodic) {
            let c = t.component.index();
            if self.cb[c][t.r] == 0.0 {
                // held at zero: a wall or a conductor
                continue;
            }
            if self.ca[c][t.r] != 1.0 || self.media.contains(c, t.r) {
                return Err(invalid(
                    "a design's values must be free of conductivity and dispersive media",
                ));
            }
            if self.anisotropic.as_ref().is_some_and(|a| a.couples(c, t.r)) {
                return Err(invalid(
                    "a design's values must be clear of smoothed tensors that couple E's \
                     components",
                ));
            }
            let complex = |x: f64, y: f64, z: f64| c64::new(eps(x, y, z), 0.0);
            let value = averaged(
                &complex,
                grid.e_position(t.component, (t.at[0], t.at[1], t.at[2])),
                h,
                t.component,
                SAMPLES,
            )
            .re;
            if !(value.is_finite() && value > 0.0) {
                return Err(invalid(format!(
                    "the permittivity must be finite and positive, got {value}"
                )));
            }
            self.cb[c][t.r] = self.dt / value;
            if let Some(a) = &mut self.anisotropic {
                a.set_diagonal(c, t.r, 1.0 / value);
            }
        }
        Ok(self)
    }

    /// Transforms E over the values of E `design`'s cells reach, at `frequencies`, from the next
    /// step on: what a gradient with respect to the design's densities needs, from both runs.
    /// Returns the monitor's number for [`Simulation::adjoint`] and [`Simulation::gradient`].
    /// The frequencies must be the objective's, as its monitors take them
    /// ([`Simulation::mode_frequencies`], [`Simulation::flux_frequencies`]).
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] as [`Simulation::with_design`] for the design, and for no
    /// frequency or one at or beyond the Nyquist frequency.
    pub fn add_design_monitor(
        &mut self,
        design: &Design,
        frequencies: &[Frequency],
    ) -> Result<usize> {
        self.check_frequencies(frequencies)?;
        design.check(self)?;
        let grid = self.grid;
        let periodic = Axis::ALL.map(|a| self.boundaries.periodic(a));
        let touched = design.values(&grid, periodic);
        // each component's box of values
        let series = Axis::ALL
            .into_iter()
            .filter_map(|component| {
                let mine: Vec<&Touched> = touched
                    .iter()
                    .filter(|t| t.component == component)
                    .collect();
                if mine.is_empty() {
                    return None;
                }
                let ranges: [Range<usize>; 3] = std::array::from_fn(|a| {
                    let low = mine.iter().map(|t| t.at[a]).min().unwrap_or(0);
                    let high = mine.iter().map(|t| t.at[a]).max().unwrap_or(0);
                    low..high + 1
                });
                Some(Series::new(Field::E, component, ranges, frequencies.len()))
            })
            .collect();
        self.monitors.designs.push(DesignMonitor {
            design: design.clone(),
            touched,
            dft: Dft::new(frequencies, series),
        });
        Ok(self.monitors.designs.len() - 1)
    }

    /// Design monitor `n`'s frequencies.
    ///
    /// # Panics
    ///
    /// If there is no design monitor `n`.
    pub fn design_frequencies(&self, n: usize) -> &[Frequency] {
        self.monitors.designs[n].dft.frequencies()
    }

    /// The adjoint run for the objective whose derivatives with respect to this run's monitors
    /// are `terms`, after this run (the forward one) has run until its fields died away: the
    /// same grid, medium and boundaries at rest, without this run's sources and monitors but
    /// design monitor `monitor`, and with the adjoint's sources, currents on the values the
    /// terms read whose transforms at each of the design monitor's frequencies are the
    /// objective's derivative there (see the module's docs). Each frequency's currents follow a
    /// Gaussian pulse on it, `bandwidth` wide (c/µm, as [`Waveform::pulse`]); pulses narrower
    /// than the frequencies' spacing keep their system well conditioned. Run it until its
    /// fields die away too ([`Simulation::run_until_converged`]), then [`Simulation::gradient`].
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] for no design monitor `monitor`, a term whose monitor,
    /// mode, frequency or value doesn't exist, a term at a frequency the design monitor doesn't
    /// transform at, a bandwidth that isn't positive and finite, or with a Bloch phase.
    pub fn adjoint(&self, terms: &[Term], monitor: usize, bandwidth: f64) -> Result<Simulation> {
        if self.bloch.is_some() {
            return Err(invalid(
                "an adjoint run needs a reciprocal problem: no Bloch phase",
            ));
        }
        let design = self
            .monitors
            .designs
            .get(monitor)
            .ok_or_else(|| invalid(format!("there is no design monitor {monitor}")))?;
        let frequencies = design.dft.frequencies().to_vec();
        let count = frequencies.len();
        // each value's transform targets, frequency by frequency
        let mut targets: BTreeMap<Key, Vec<c64>> = BTreeMap::new();
        for term in terms {
            let (frequency, derivatives) = self.term_derivatives(term)?;
            let f = frequencies
                .iter()
                .position(|&x| x == frequency)
                .ok_or_else(|| {
                    invalid(format!(
                        "the objective's frequency {frequency} isn't among design monitor \
                         {monitor}'s"
                    ))
                })?;
            let omega = frequency.angular();
            for ((field, component, r), g) in derivatives {
                let at = [
                    r % self.grid.nx,
                    (r / self.grid.nx) % self.grid.ny,
                    r / (self.grid.nx * self.grid.ny),
                ];
                let d = self.symmetrizer(field, Axis::ALL[component], at, omega);
                // the adjoint's sources: J = −g_E, M = g_H, over D
                let value = match field {
                    Field::E => -g / d,
                    Field::H => g / d,
                };
                targets
                    .entry(key(field, component, r))
                    .or_insert_with(|| vec![c64::new(0.0, 0.0); count])[f] += value;
            }
        }
        let waveforms = basis(&frequencies, bandwidth)?;
        let mut adjoint = self.at_rest(monitor);
        for field in [Field::E, Field::H] {
            let mine: Vec<(Key, &Vec<c64>)> = targets
                .iter()
                .filter(|(k, _)| k.0 == u8::from(field == Field::H))
                .map(|(k, v)| (*k, v))
                .collect();
            if mine.is_empty() {
                continue;
            }
            let amplitudes = self.amplitudes(field, &waveforms, &frequencies, &mine)?;
            for (k, waveform) in waveforms.iter().enumerate() {
                let values: Vec<(usize, usize, c64)> = mine
                    .iter()
                    .zip(&amplitudes)
                    .map(|(((_, c, r), _), a)| (*c, *r, c64::new(a[k], 0.0)))
                    .filter(|v| v.2.re != 0.0)
                    .collect();
                if !values.is_empty() {
                    adjoint.currents.push(super::sources::Applied {
                        field,
                        values,
                        waveform: *waveform,
                    });
                }
            }
        }
        Ok(adjoint)
    }

    /// The real amplitudes βₘ, one per waveform of [`basis`], of each value's current
    /// Σₘ βₘ wₘ(t) whose transform at each frequency ωⱼ is the value's target there: the real
    /// system Σₘ βₘ Wₘ(ωⱼ) = target_j, its real and imaginary parts, Wₘ the transform of wₘ at
    /// the times a current on `field` enters.
    fn amplitudes(
        &self,
        field: Field,
        waveforms: &[Waveform],
        frequencies: &[Frequency],
        targets: &[(Key, &Vec<c64>)],
    ) -> Result<Vec<Vec<f64>>> {
        use faer::linalg::solvers::Solve;
        let m = waveforms.len();
        // the transforms are summed until each pulse has died, to e^(−64) of its peak
        let steps = waveforms
            .iter()
            .map(|w| match *w {
                Waveform::Gaussian { width, delay, .. } => {
                    ((delay + 8.0 * width) / self.dt).ceil() as usize + 1
                }
                _ => unreachable!("the adjoint's waveforms are pulses"),
            })
            .max()
            .unwrap_or(0);
        let mut matrix = faer::Mat::<f64>::zeros(m, m);
        for (q, w) in waveforms.iter().enumerate() {
            for (j, &f) in frequencies.iter().enumerate() {
                let t = w.spectrum(field, self.dt, steps, f);
                matrix[(2 * j, q)] = t.re;
                matrix[(2 * j + 1, q)] = t.im;
            }
        }
        let mut rhs = faer::Mat::<f64>::zeros(m, targets.len());
        for (col, (_, target)) in targets.iter().enumerate() {
            for (j, t) in target.iter().enumerate() {
                rhs[(2 * j, col)] = t.re;
                rhs[(2 * j + 1, col)] = t.im;
            }
        }
        let solution = matrix.partial_piv_lu().solve(&rhs);
        let out: Vec<Vec<f64>> = (0..targets.len())
            .map(|col| (0..m).map(|q| solution[(q, col)]).collect())
            .collect();
        if out.iter().flatten().any(|a| !a.is_finite()) {
            return Err(invalid(
                "the adjoint's pulses can't give its sources' transforms: frequencies too close \
                 for the bandwidth",
            ));
        }
        Ok(out)
    }

    /// A term's frequency, and the objective's derivative with respect to each transform it
    /// reads: (field, component, index within the component) and ∂F/∂(value).
    fn term_derivatives(&self, term: &Term) -> Result<(Frequency, Derivatives)> {
        let g = self.grid;
        let cells = g.cells();
        let missing = || {
            invalid(format!(
                "the objective's term {term:?} reads no monitor here"
            ))
        };
        match *term {
            Term::Mode {
                monitor,
                mode,
                direction,
                derivative,
            } => {
                let m = self
                    .monitors
                    .modes
                    .get(monitor)
                    .and_then(|m| m.get(mode))
                    .ok_or_else(missing)?;
                let weights = crate::fdfd::mode_amplitude_weights_of(
                    g,
                    self.fdfd_boundaries(),
                    &m.mode,
                    direction,
                );
                let derivatives = weights
                    .into_iter()
                    .map(|(r, w)| ((Field::E, r / cells, r % cells), derivative * w))
                    .collect();
                Ok((m.dft.frequencies()[0], derivatives))
            }
            Term::Flux {
                monitor,
                frequency,
                derivative,
            } => {
                let flux = self.monitors.fluxes.get(monitor).ok_or_else(missing)?;
                let f = flux.faces[0]
                    .2
                    .frequencies()
                    .get(frequency)
                    .ok_or_else(missing)?;
                let mut derivatives = Vec::new();
                for (plane, sign, dft) in &flux.faces {
                    derivatives.extend(flux_derivatives(
                        g,
                        plane,
                        dft,
                        frequency,
                        sign * derivative,
                    ));
                }
                Ok((*f, derivatives))
            }
            Term::Transform {
                monitor,
                field,
                component,
                at,
                frequency,
                derivative,
            } => {
                let dft = self.monitors.dfts.get(monitor).ok_or_else(missing)?;
                let f = *dft.frequencies().get(frequency).ok_or_else(missing)?;
                dft.value(field, component, at, frequency)
                    .ok_or_else(missing)?;
                Ok((
                    f,
                    vec![((field, component.index(), self.index(at)), derivative)],
                ))
            }
        }
    }

    /// 1/s̃ of the CPML along `axis` at index `m`, on a node (where E's convolutions are) or
    /// halfway after it (H̃'s), at angular frequency `omega`: 1/κ + a/(1 − b e^(iωΔt)), the
    /// transform of ψ ← bψ + a∂F; 1 outside the CPMLs.
    fn inverse_stretch(&self, axis: Axis, m: usize, half: bool, omega: f64) -> c64 {
        let a = axis.index();
        let (kappa, field) = if half {
            (self.kappa_halves[a][m], Field::H)
        } else {
            (self.kappa_nodes[a][m], Field::E)
        };
        let mut v = c64::new(kappa, 0.0);
        if let Some(s) = self
            .slabs
            .iter()
            .find(|s| s.field == field && s.axis == axis && (s.from..s.from + s.count).contains(&m))
        {
            let q = m - s.from;
            v += s.a[q] / (1.0 - s.b[q] * c64::from_polar(1.0, omega * self.dt));
        }
        v
    }

    /// D at `field`'s `component` at `at`: the product of the CPMLs' stretches along the three
    /// axes there, which makes the run's frequency-domain system symmetric (see the module's
    /// docs); 1 outside the CPMLs.
    fn symmetrizer(&self, field: Field, component: Axis, at: [usize; 3], omega: f64) -> c64 {
        Axis::ALL.into_iter().fold(c64::new(1.0, 0.0), |d, axis| {
            let half = match field {
                Field::E => axis == component,
                Field::H => axis != component,
            };
            d / self.inverse_stretch(axis, at[axis.index()], half, omega)
        })
    }

    /// The same problem at rest, from the start: the grid, the medium and the boundaries, no
    /// sources, probes or monitors but design monitor `keep`, its transforms at zero.
    fn at_rest(&self, keep: usize) -> Simulation {
        let n = self.grid.cells();
        let zeros = || [vec![0.0; n], vec![0.0; n], vec![0.0; n]];
        let monitors = super::monitors::Monitors {
            designs: self
                .monitors
                .designs
                .iter()
                .enumerate()
                .map(|(m, d)| {
                    if m == keep {
                        d.at_rest()
                    } else {
                        DesignMonitor {
                            design: d.design.clone(),
                            touched: Vec::new(),
                            dft: Dft::new(d.dft.frequencies(), Vec::new()),
                        }
                    }
                })
                .collect(),
            ..Default::default()
        };
        Simulation {
            grid: self.grid,
            boundaries: self.boundaries,
            dt: self.dt,
            steps: 0,
            e: zeros(),
            h: zeros(),
            ca: self.ca.clone(),
            cb: self.cb.clone(),
            kappa_nodes: self.kappa_nodes.clone(),
            kappa_halves: self.kappa_halves.clone(),
            slabs: self
                .slabs
                .iter()
                .map(|s| kernel::Slab {
                    field: s.field,
                    component: s.component,
                    axis: s.axis,
                    from: s.from,
                    count: s.count,
                    b: s.b.clone(),
                    a: s.a.clone(),
                    psi: vec![0.0; s.psi.len()],
                })
                .collect(),
            stencils: self.stencils.clone(),
            reference: self.reference,
            tiling: self.tiling,
            points: Default::default(),
            sources: Vec::new(),
            currents: Vec::new(),
            plane_waves: Vec::new(),
            probes: Vec::new(),
            varies: self.varies,
            media: self.media.at_rest(),
            bloch: None,
            anisotropic: self.anisotropic.as_ref().map(|a| Box::new(a.at_rest())),
            monitors,
            #[cfg(feature = "gpu")]
            gpu: self.gpu.clone(),
        }
    }

    /// The relative permittivity and the conductivity E's `component` at `at` has, from its
    /// update's coefficients: ca = (1 − x)/(1 + x) and cb = Δt/(ε(1 + x)), x = σΔt/(2ε).
    /// `None` where E is held at zero.
    pub(crate) fn medium_at(
        &self,
        component: Axis,
        at: (usize, usize, usize),
    ) -> Option<(f64, f64)> {
        let (c, r) = (component.index(), self.index(at));
        let (ca, cb) = (self.ca[c][r], self.cb[c][r]);
        if cb == 0.0 {
            return None;
        }
        let x = (1.0 - ca) / (1.0 + ca);
        let eps = self.dt / (cb * (1.0 + x));
        Some((eps, 2.0 * eps * x / self.dt))
    }

    /// The objective's gradient with respect to the permittivity at each value of E design
    /// monitor `monitor` records, from this run (the forward one) and `adjoint`
    /// ([`Simulation::adjoint`]), both run until their fields died away: each value's component,
    /// index (i, j, k) and ∂F/∂ε, Σ 2 Re(iω̃ Ê_adj Ê) over the frequencies.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] if either run has no design monitor `monitor`, or if
    /// theirs differ (another design, other frequencies).
    pub fn permittivity_gradient(
        &self,
        adjoint: &Simulation,
        monitor: usize,
    ) -> Result<Vec<ValueGradient>> {
        let (forward, backward) = self.design_pair(adjoint, monitor)?;
        let tilde: Vec<f64> = forward
            .dft
            .frequencies()
            .iter()
            .map(|&f| self.leapfrog_wavenumber(f))
            .collect();
        Ok(forward
            .touched
            .iter()
            .map(|t| {
                let sum = tilde
                    .iter()
                    .enumerate()
                    .map(|(f, &w)| {
                        2.0 * (c64::new(0.0, w) * backward.value(t, f) * forward.value(t, f)).re
                    })
                    .sum();
                ValueGradient {
                    component: t.component,
                    at: (t.at[0], t.at[1], t.at[2]),
                    derivative: sum,
                }
            })
            .collect())
    }

    /// The objective's gradient with respect to the densities of design monitor `monitor`'s
    /// design, one per cell as [`Design::index`] numbers them, from this run (the forward one)
    /// and `adjoint`: [`Simulation::permittivity_gradient`] through the average each value of E
    /// takes of the design's cells, ∂ε/∂ρ = (ε₁ − ε₀) times each cell's share.
    ///
    /// # Errors
    ///
    /// As [`Simulation::permittivity_gradient`].
    pub fn gradient(&self, adjoint: &Simulation, monitor: usize) -> Result<Vec<f64>> {
        let values = self.permittivity_gradient(adjoint, monitor)?;
        let design = &self.monitors.designs[monitor];
        let scale = design.design.eps[1] - design.design.eps[0];
        let mut out = vec![0.0; design.design.len()];
        for (t, value) in design.touched.iter().zip(&values) {
            let g = value.derivative;
            for &(v, share) in &t.shares {
                out[v] += g * share * scale;
            }
        }
        Ok(out)
    }

    /// Design monitor `monitor` of this run and of `adjoint`, checked to be the same.
    fn design_pair<'a>(
        &'a self,
        adjoint: &'a Simulation,
        monitor: usize,
    ) -> Result<(&'a DesignMonitor, &'a DesignMonitor)> {
        let forward = self.monitors.designs.get(monitor);
        let backward = adjoint.monitors.designs.get(monitor);
        match (forward, backward) {
            (Some(f), Some(b))
                if f.design == b.design
                    && f.dft.frequencies() == b.dft.frequencies()
                    && f.touched == b.touched =>
            {
                Ok((f, b))
            }
            _ => Err(invalid(format!(
                "design monitor {monitor} must be the same in the forward and the adjoint runs"
            ))),
        }
    }

    /// Runs in blocks of `interval` (µm/c) until, after every source has finished, no
    /// monitor's transform has changed over a whole block by more than `tolerance` of the
    /// largest transform in it, or until the time reaches `limit`: whether they had settled.
    /// The transforms are then the frequency-domain fields to about `tolerance`. A field that
    /// is left standing (a static field, or one growing from round-off in the CPMLs long after)
    /// moves the transforms by its own size over ω̃ at most, so the rule stops where the waves
    /// have gone, whatever stays.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] for no monitor, a tolerance that isn't positive and
    /// finite, an interval that isn't positive and finite, or a limit that isn't finite.
    pub fn run_until_converged(
        &mut self,
        tolerance: f64,
        interval: f64,
        limit: f64,
    ) -> Result<bool> {
        if !(tolerance.is_finite() && tolerance > 0.0)
            || !(interval.is_finite() && interval > 0.0)
            || !limit.is_finite()
        {
            return Err(invalid(format!(
                "running until the transforms settle needs a positive tolerance and interval and \
                 a finite limit, got {tolerance}, {interval} and {limit}"
            )));
        }
        if self.monitors.transforms().next().is_none() {
            return Err(invalid(
                "running until the transforms settle needs a monitor",
            ));
        }
        let finished = self.sources_finish();
        let snapshot = |s: &Simulation| -> Vec<Vec<c64>> {
            s.monitors.transforms().map(<[c64]>::to_vec).collect()
        };
        let mut before = snapshot(self);
        while self.time() < limit {
            self.run_until((self.time() + interval).min(limit));
            let now = snapshot(self);
            // each monitor against its own largest transform
            let settled = now.iter().zip(&before).all(|(now, before)| {
                let largest = now.iter().fold(0.0f64, |m, v| m.max(v.norm()));
                let change = now
                    .iter()
                    .zip(before)
                    .fold(0.0f64, |m, (a, b)| m.max((a - b).norm()));
                change <= tolerance * largest
            });
            if self.time() >= finished && settled {
                return Ok(true);
            }
            before = now;
        }
        Ok(false)
    }

    /// The bytes of what a run keeps, for measuring a gradient's memory against a forward
    /// run's: the fields and their update coefficients, the CPMLs' auxiliary fields, the
    /// distributed currents and every monitor's transforms (the stencils' tables, a few values
    /// per index along each axis, and a smoothed tensor's or a medium's own arrays left out).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn bytes(&self) -> usize {
        let real = std::mem::size_of::<f64>();
        let n = self.grid.cells();
        let fields = 4 * 3 * n * real;
        let slabs: usize = self
            .slabs
            .iter()
            .map(|s| (s.psi.len() + s.a.len() + s.b.len()) * real)
            .sum();
        let currents: usize = self
            .currents
            .iter()
            .map(|c| c.values.len() * std::mem::size_of::<(usize, usize, c64)>())
            .sum();
        let transforms: usize = self.monitors.transforms().map(std::mem::size_of_val).sum();
        fields + slabs + currents + transforms
    }

    /// When every source has finished: a pulse 6 widths after its peak, a continuous wave
    /// never.
    fn sources_finish(&self) -> f64 {
        let end = |w: &Waveform| match *w {
            Waveform::Gaussian { width, delay, .. }
            | Waveform::DifferentiatedGaussian { width, delay } => delay + 6.0 * width,
            Waveform::Continuous { .. } => f64::INFINITY,
        };
        self.sources
            .iter()
            .map(|s| end(&s.waveform))
            .chain(self.currents.iter().map(|c| end(&c.waveform)))
            .chain(self.plane_waves.iter().map(|p| end(&p.waveform())))
            .fold(0.0, f64::max)
    }
}

/// The adjoint's waveforms: for each frequency, two Gaussian pulses on it `bandwidth` wide
/// ([`Waveform::pulse`]), e^(−u²) sin(ω(t − d)), the second a quarter period after the first.
/// Each is odd about its peak, so its mean is zero and no charge is left behind (a pulse whose
/// cosine part has a mean leaves a static field that never dies); together, real amplitudes
/// on them give any complex transform at each frequency.
fn basis(frequencies: &[Frequency], bandwidth: f64) -> Result<Vec<Waveform>> {
    let mut out = Vec::with_capacity(2 * frequencies.len());
    for &f in frequencies {
        let Waveform::Gaussian {
            frequency,
            width,
            delay,
        } = Waveform::pulse(f, bandwidth)?
        else {
            unreachable!("a pulse is a Gaussian")
        };
        for shift in [0.0, 0.25 / f.to_natural()] {
            out.push(Waveform::Gaussian {
                frequency,
                width,
                delay: delay + shift,
            });
        }
    }
    Ok(out)
}

/// The derivative of a flux plane's flux at frequency `f` (from its transforms `dft`) with
/// respect to each transform it reads, times `scale`: the flux is Σ κ Re(E H̄) over the pairs
/// [`super::FluxPlane`] sums, so ∂/∂E = ½κH̄ and ∂/∂H = ½κĒ.
fn flux_derivatives(
    grid: Grid3d,
    plane: &super::FluxPlane,
    dft: &Dft,
    f: usize,
    scale: f64,
) -> Vec<((Field, usize, usize), c64)> {
    let (b, c) = plane.axis.others();
    let index = |at: [usize; 3]| (at[2] * grid.ny + at[1]) * grid.nx + at[0];
    let value = |field, component, at: [usize; 3]| {
        dft.value(field, component, (at[0], at[1], at[2]), f)
            .expect("the flux plane's transforms")
    };
    let area = grid.step(b) * grid.step(c);
    let mut out = Vec::new();
    for (component, h, sign) in [(b, c, 0.5), (c, b, -0.5)] {
        for v in plane.range(grid, c) {
            for u in plane.range(grid, b) {
                let weight =
                    plane.weight(b, u, component == b) * plane.weight(c, v, component == c);
                if weight == 0.0 {
                    continue;
                }
                // flux += weight sign Re(½(low + high) H̄) area
                let kappa = scale * weight * sign * 0.5 * area;
                let mut at = [0; 3];
                at[plane.axis.index()] = plane.plane;
                at[b.index()] = u;
                at[c.index()] = v;
                let hv = value(Field::H, h, at);
                let low = value(Field::E, component, at);
                let h_at = index(at);
                at[plane.axis.index()] = plane.plane + 1;
                let high = value(Field::E, component, at);
                out.push(((Field::E, component.index(), h_at), 0.5 * kappa * hv.conj()));
                out.push((
                    (Field::E, component.index(), index(at)),
                    0.5 * kappa * hv.conj(),
                ));
                out.push((
                    (Field::H, h.index(), h_at),
                    0.5 * kappa * (low + high).conj(),
                ));
            }
        }
    }
    out
}
