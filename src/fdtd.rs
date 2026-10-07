//! Finite-difference time-domain (FDTD): Maxwell's equations stepped in time on Yee's grid.
//!
//! - **The scheme** (K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966),
//!   doi:10.1109/TAP.1966.1138693): E and H staggered in space, on the same grid as FDFD's
//!   ([`Grid3d`]: E's components at the middle of the cells' edges, H's at the centres of their
//!   faces), and in time, H half a step after E (leapfrog). With c = 1, lengths in µm and times
//!   in µm/c, and H̃ = η₀H as FDFD writes it:
//!
//!   ∂H̃/∂t = −∇ × E − M,   ε ∂E/∂t + σE = ∇ × H̃ − J.
//!
//!   The current J and the magnetic current M are FDFD's: a source here that oscillates as
//!   Re(Ĵ e^(−iωt)) settles to the field FDFD finds for Ĵ, at ω̃ = (2/Δt) sin(ωΔt/2), the
//!   leapfrog's own frequency.
//! - **Stability:** Δt = C / √(Σ 1/Δ²) with the Courant number C ≤ 1 (A. Taflove, M. E. Brodwin,
//!   IEEE Trans. Microw. Theory Tech. 23, 623 (1975), doi:10.1109/TMTT.1975.1128640), the sum
//!   over the axes the field varies along (an axis one cell long and periodic doesn't count, or
//!   counts sin²(kΔ/2) for a Bloch phase k across it). The scheme's numerical dispersion, their
//!   relation
//!   sin²(ωΔt/2)/Δt² = Σ sin²(k_i Δ_i/2)/Δ_i², holds exactly.
//! - **Open boundaries:** the convolutional PML (J. A. Roden, S. D. Gedney, Microw. Opt.
//!   Technol. Lett. 27, 334 (2000), doi:10.1002/1098-2760(20001205)27:5<334::AID-MOP14>3.0.CO;2-A),
//!   the complex-frequency-shifted stretch s = κ + σ/(α − iω) (the PML of J.-P. Berenger,
//!   J. Comput. Phys. 114, 185 (1994), doi:10.1006/jcph.1994.1159, as a stretched coordinate),
//!   its convolution recursive: ψ ← bψ + a ∂F, with b = e^(−(σ/κ + α)Δt) and
//!   a = σ(b − 1)/(σκ + κ²α). σ and κ are graded as (depth/d)^m; α is constant. With κ = 1 and
//!   α = 0 it is FDFD's stretched-coordinate PML, s = 1 + iσ/ω, graded the same way. Its
//!   auxiliary fields live only in the PML slabs.
//! - **Walls** (a PML of zero cells): the field's tangential E is zero on the wall, as in FDFD.
//!   **Periodic** sides (a Bloch boundary with k = 0) wrap. **Bloch** sides with k ≠ 0 make the
//!   fields complex: the imaginary part runs alongside the real one, the two coupled where a
//!   difference reaches across the side. A 2D problem is a grid one cell thick along z,
//!   periodic there.
//! - **Media:** a real permittivity per E component, averaged over its cell as FDFD averages it
//!   (harmonically along the component, arithmetically across), and a conductivity σ, sampled at
//!   the component. [`Dispersive`] media, Drude and Lorentz terms by auxiliary differential
//!   equations (M. Okoniewski, M. Mrozowski, M. A. Stuchly, IEEE Microw. Guided Wave Lett. 7,
//!   121 (1997), doi:10.1109/75.569723), sampled at the component.
//! - **Sources:** point currents, [`Current`]s over many values and [`Dipole`]s anywhere
//!   (restricted to the grid as A. F. Oskooi et al., Comput. Phys. Commun. 181, 687 (2010),
//!   doi:10.1016/j.cpc.2009.11.008, restrict them); [`PlaneWave`]s on
//!   total-field/scattered-field boxes (K. Umashankar, A. Taflove, IEEE Trans. Electromagn.
//!   Compat. EMC-24, 397 (1982), doi:10.1109/TEMC.1982.304054), fed by an auxiliary 1D run
//!   with the grid's dispersion; one-way waveguide modes; [`GaussianBeam`]s. A run's DFT
//!   divided by [`Waveform::spectrum`] is FDFD's field at ω̃, exactly.
//!
//! Every update is a sum over a fixed stencil with no reduction, its z-planes shared among
//! rayon's threads: the fields are the same bits on any number of threads.

use num_complex::Complex64 as c64;
use rayon::prelude::*;

use crate::fdfd::{Axis, Edges, Grid3d, averaged_3d as averaged};
use crate::units::Frequency;
use crate::{Error, Result};

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("fdtd", reason)
}

/// The convolutional PML's parameters, the same on every axis that has one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cpml {
    /// The target reflection at normal incidence, R in (0, 1): σ's maximum is
    /// (m + 1)(−ln R)/(2d), d the PML's thickness in µm (FDFD's grading). Ignored if `sigma` is
    /// given.
    pub reflection: f64,
    /// σ's maximum, in units of 1/µm (with ε₀ = c = 1), instead of the one from `reflection`.
    pub sigma: Option<f64>,
    /// The grading's order m, for σ and κ.
    pub order: f64,
    /// κ's maximum, κ ≥ 1 (1: no real stretching).
    pub kappa: f64,
    /// α, the complex frequency shift, in 1/µm, constant through the PML (0: none).
    pub alpha: f64,
}

impl Default for Cpml {
    /// FDFD's PML: R = 1e-8, order 3, κ = 1, α = 0.
    fn default() -> Cpml {
        Cpml {
            reflection: 1e-8,
            sigma: None,
            order: 3.0,
            kappa: 1.0,
            alpha: 0.0,
        }
    }
}

impl Cpml {
    /// Gedney's "optimal" σ, 0.8 (m + 1)/(√ε_r Δ) in units of 1/µm with η = 1: Roden and Gedney's
    /// Eq. 15, σ_opt = (m + 1)/(150π √ε_r Δ) in S/m with Δ in metres.
    pub fn sigma_optimal(order: f64, eps: f64, step: f64) -> f64 {
        (order + 1.0) * 376.730_313_668 / (150.0 * std::f64::consts::PI) / (eps.sqrt() * step)
    }
}

/// The boundaries of an FDTD grid: what is beyond each axis's ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boundaries {
    /// Along x: a CPML of `low` and `high` cells inside the grid (0 cells: a wall), or periodic
    /// (`Bloch` with k = 0).
    pub x: Edges,
    /// Along y.
    pub y: Edges,
    /// Along z.
    pub z: Edges,
    /// The CPMLs' parameters.
    pub cpml: Cpml,
}

impl Boundaries {
    /// A CPML of `cells` on every side, with [`Cpml::default`].
    pub fn cpml(cells: usize) -> Boundaries {
        let e = Edges::Pml {
            low: cells,
            high: cells,
        };
        Boundaries {
            x: e,
            y: e,
            z: e,
            cpml: Cpml::default(),
        }
    }

    /// Walls (perfect electric conductors) on every side.
    pub fn walls() -> Boundaries {
        Boundaries::cpml(0)
    }

    /// A 2D problem: a CPML of `cells` across x and y, periodic along z (the grid one cell
    /// thick there).
    pub fn cpml_2d(cells: usize) -> Boundaries {
        Boundaries {
            z: Edges::Bloch { k: 0.0 },
            ..Boundaries::cpml(cells)
        }
    }

    fn edges(&self, axis: Axis) -> Edges {
        [self.x, self.y, self.z][axis.index()]
    }

    fn periodic(&self, axis: Axis) -> bool {
        matches!(self.edges(axis), Edges::Bloch { .. })
    }
}

/// How a source's value varies in time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Waveform {
    /// A Gaussian pulse on a carrier: exp(−((t − delay)/width)²) sin(ω(t − delay)), ω the
    /// carrier's angular frequency. Its spectrum is centred on the carrier, about 1/(π width)
    /// wide in frequency (c/µm) at e^(−1).
    Gaussian {
        /// The carrier.
        frequency: Frequency,
        /// The envelope's width, µm/c.
        width: f64,
        /// The envelope's peak, µm/c.
        delay: f64,
    },
    /// The derivative of a Gaussian, −((t − delay)/width) exp(−((t − delay)/width)²) √(2e),
    /// peaking at ±1: no DC, as Roden and Gedney's source.
    DifferentiatedGaussian {
        /// The width, µm/c.
        width: f64,
        /// The centre, µm/c.
        delay: f64,
    },
    /// A continuous wave, Re(amplitude e^(−iωt)), switched on smoothly over `ramp` (a
    /// smoothstep from 0 to 1), so that the field settles to FDFD's for that amplitude.
    Continuous {
        /// Its frequency.
        frequency: Frequency,
        /// Its complex amplitude (the phase at t = 0).
        amplitude: c64,
        /// How long it takes to switch on, µm/c (0: at once).
        ramp: f64,
    },
}

impl Waveform {
    /// A Gaussian pulse on a carrier at `frequency` whose power spectrum is `bandwidth` wide at
    /// half its peak (c/µm): width √(2 ln 2)/(π bandwidth), its peak 6 widths in, where the
    /// envelope starts at e^(−36) = 2e-16 of it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a bandwidth that isn't positive and finite.
    pub fn pulse(frequency: Frequency, bandwidth: f64) -> Result<Waveform> {
        if !(bandwidth.is_finite() && bandwidth > 0.0) {
            return Err(invalid(format!(
                "a pulse's bandwidth must be positive and finite, got {bandwidth}"
            )));
        }
        let width = (2.0 * std::f64::consts::LN_2).sqrt() / (std::f64::consts::PI * bandwidth);
        Ok(Waveform::Gaussian {
            frequency,
            width,
            delay: 6.0 * width,
        })
    }

    /// The carrier's frequency: the Gaussian's and the continuous wave's; the differentiated
    /// Gaussian has none.
    pub fn carrier(&self) -> Option<Frequency> {
        match *self {
            Waveform::Gaussian { frequency, .. } | Waveform::Continuous { frequency, .. } => {
                Some(frequency)
            }
            Waveform::DifferentiatedGaussian { .. } => None,
        }
    }

    /// Its analytic form w(t), whose real part is [`Waveform::at`]: i e^(−u²) e^(−iω(t − delay))
    /// for the Gaussian, the switched-on amplitude e^(−iωt) for the continuous wave, and the
    /// differentiated Gaussian itself (real). A current of complex amplitude a takes the
    /// values Re(a w(t)).
    pub fn complex_at(&self, t: f64) -> c64 {
        match *self {
            Waveform::Gaussian {
                frequency,
                width,
                delay,
            } => {
                let u = (t - delay) / width;
                c64::new(0.0, (-u * u).exp())
                    * c64::new(0.0, -frequency.angular() * (t - delay)).exp()
            }
            Waveform::DifferentiatedGaussian { .. } => c64::new(self.at(t), 0.0),
            Waveform::Continuous {
                frequency,
                amplitude,
                ramp,
            } => smoothstep(t, ramp) * amplitude * c64::new(0.0, -frequency.angular() * t).exp(),
        }
    }

    /// The discrete Fourier transform Σ s(tₙ) e^(iωtₙ) Δt of the values a source on `field`
    /// takes over the first `steps` steps of Δt = `dt`, at the times it takes them (a current
    /// on E, J, at (n + ½)Δt; on H̃, M, at nΔt): s = [`Waveform::at`], a point source's.
    ///
    /// It normalizes a run exactly. Each field's transform, at its own times (E at nΔt, H̃ at
    /// (n + ½)Δt), solves FDFD's equations at the leapfrog's frequency ω̃ = (2/Δt) sin(ωΔt/2)
    /// for the currents' transforms, once the fields have died away: the field per unit source
    /// spectrum is the field's transform divided by this.
    pub fn spectrum(&self, field: Field, dt: f64, steps: usize, frequency: Frequency) -> c64 {
        self.transform(field, dt, steps, frequency, |w, t| c64::new(w.at(t), 0.0))
    }

    /// The transform of the analytic form [`Waveform::complex_at`], W(ω), as
    /// [`Waveform::spectrum`] takes it. A current of complex amplitude a, whose values are
    /// Re(a w), transforms as ½(a W(ω) + ā W(−ω)*): near the carrier, the second term is the
    /// negative frequencies' image, e^(−(ω width)²) of the first for the Gaussian at its
    /// carrier.
    pub fn analytic_spectrum(
        &self,
        field: Field,
        dt: f64,
        steps: usize,
        frequency: Frequency,
    ) -> c64 {
        self.transform(field, dt, steps, frequency, |w, t| w.complex_at(t))
    }

    fn transform(
        &self,
        field: Field,
        dt: f64,
        steps: usize,
        frequency: Frequency,
        value: impl Fn(&Waveform, f64) -> c64,
    ) -> c64 {
        let omega = frequency.angular();
        let offset = if field == Field::E { 0.5 * dt } else { 0.0 };
        (0..steps)
            .map(|n| {
                let t = n as f64 * dt + offset;
                value(self, t) * c64::new(0.0, omega * t).exp() * dt
            })
            .sum()
    }

    /// Its value at time `t` (µm/c).
    pub fn at(&self, t: f64) -> f64 {
        match *self {
            Waveform::Gaussian {
                frequency,
                width,
                delay,
            } => {
                let u = (t - delay) / width;
                (-u * u).exp() * (frequency.angular() * (t - delay)).sin()
            }
            Waveform::DifferentiatedGaussian { width, delay } => {
                let u = (t - delay) / width;
                -u * (-u * u).exp() * (2.0 * std::f64::consts::E).sqrt()
            }
            Waveform::Continuous {
                frequency,
                amplitude,
                ramp,
            } => {
                smoothstep(t, ramp) * (amplitude * c64::new(0.0, -frequency.angular() * t).exp()).re
            }
        }
    }
}

/// A smoothstep from 0 at t = 0 to 1 at t = `ramp` (1 throughout for no ramp).
fn smoothstep(t: f64, ramp: f64) -> f64 {
    if ramp > 0.0 {
        let s = (t / ramp).clamp(0.0, 1.0);
        s * s * (3.0 - 2.0 * s)
    } else {
        1.0
    }
}

/// Which field a source drives or a probe records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// E: a source on it is an electric current J.
    E,
    /// H̃ = η₀H: a source on it is a magnetic current M.
    H,
}

/// A point source: a current density on one value of E (J) or of H̃ (M).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Source {
    /// E (an electric current) or H (a magnetic current).
    pub field: Field,
    /// The component.
    pub component: Axis,
    /// Its value's index (i, j, k), as [`Grid3d::index`] numbers them.
    pub at: (usize, usize, usize),
    /// Its value in time: J = η₀J (or M), as FDFD's sources.
    pub waveform: Waveform,
}

/// A recorded time series: one value of E or H̃ at every step.
#[derive(Clone, Debug, PartialEq)]
struct Probe {
    field: Field,
    component: Axis,
    index: usize,
    values: Vec<f64>,
}

/// One CPML slab's auxiliary field for one component and one axis: ψ over the slab's cells.
#[derive(Clone, Debug)]
struct Slab {
    field: Field,
    /// The component updated.
    component: Axis,
    /// The axis of the PML, and of the derivative ψ convolves.
    axis: Axis,
    /// The slab's first index along `axis`, and its thickness.
    from: usize,
    count: usize,
    /// The recursion's coefficients along `axis`, `count` of each.
    b: Vec<f64>,
    a: Vec<f64>,
    psi: Vec<f64>,
}

/// An FDTD problem: the grid, the medium, the boundaries, the fields at one time, the sources
/// and the probes. [`Simulation::step`] advances it by Δt.
#[derive(Clone, Debug)]
pub struct Simulation {
    grid: Grid3d,
    boundaries: Boundaries,
    dt: f64,
    steps: usize,
    e: [Vec<f64>; 3],
    h: [Vec<f64>; 3],
    /// E's update coefficients per value: E ← ca E + cb (∇ × H̃ − J); 0 and 0 where E is held
    /// at zero (walls, conductors).
    ca: [Vec<f64>; 3],
    cb: [Vec<f64>; 3],
    /// 1/κ along each axis, at the nodes (for E's updates) and halfway after them (for H's).
    kappa_nodes: [Vec<f64>; 3],
    kappa_halves: [Vec<f64>; 3],
    slabs: Vec<Slab>,
    sources: Vec<Source>,
    /// Distributed currents (J or M), each with its complex amplitudes and waveform.
    currents: Vec<sources::Applied>,
    /// Plane waves on total-field/scattered-field boxes, each with its auxiliary grid.
    plane_waves: Vec<sources::PlaneWaveRun>,
    probes: Vec<Probe>,
    /// The axes the field varies along: all but those one cell long and periodic.
    varies: [bool; 3],
    /// The dispersive media's polarization currents.
    media: media::Media,
    /// With a Bloch phase on a side, the fields' imaginary part.
    bloch: Option<Box<bloch::Bloch>>,
}

/// The permittivity's samples per axis in a cell, as FDFD's 3D solver averages it.
const SAMPLES: usize = 8;

impl Simulation {
    /// The problem on `grid` with relative permittivity `eps(x, y, z)` (real, µm), inside
    /// `boundaries`, stepped at the Courant number `courant` (0 < C ≤ 1).
    ///
    /// A Bloch boundary with k ≠ 0 makes the fields complex: [`Simulation::is_complex`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for an empty grid, a step that isn't positive, CPMLs that fill an
    /// axis, CPML parameters out of range (R outside (0, 1), an order below 0, κ below 1, α or σ
    /// negative), a Bloch wavenumber that isn't finite, a Courant number outside (0, 1], or a
    /// permittivity that isn't finite and positive.
    pub fn new(
        grid: Grid3d,
        eps: impl Fn(f64, f64, f64) -> f64,
        boundaries: Boundaries,
        courant: f64,
    ) -> Result<Simulation> {
        let ok = |v: f64| v.is_finite() && v > 0.0;
        if grid.cells() == 0 || !ok(grid.dx) || !ok(grid.dy) || !ok(grid.dz) {
            return Err(invalid(format!(
                "the grid needs cells and positive steps, got {grid:?}"
            )));
        }
        let c = boundaries.cpml;
        if !(c.reflection > 0.0 && c.reflection < 1.0)
            || !(c.order.is_finite() && c.order >= 0.0)
            || !(c.kappa.is_finite() && c.kappa >= 1.0)
            || !(c.alpha.is_finite() && c.alpha >= 0.0)
            || c.sigma.is_some_and(|s| !(s.is_finite() && s >= 0.0))
        {
            return Err(invalid(format!(
                "the CPML needs R in (0, 1), an order >= 0, kappa >= 1, alpha >= 0 and sigma >= \
                 0, got {c:?}"
            )));
        }
        for axis in Axis::ALL {
            let n = grid.n(axis);
            match boundaries.edges(axis) {
                Edges::Pml { low, high } if low + high >= n => {
                    return Err(invalid(format!(
                        "CPMLs of {low} and {high} cells leave nothing of {n} cells"
                    )));
                }
                Edges::Bloch { k } if !k.is_finite() => {
                    return Err(invalid(format!(
                        "the Bloch wavenumber must be finite, got {k}"
                    )));
                }
                _ => {}
            }
        }
        if !(courant > 0.0 && courant <= 1.0) {
            return Err(invalid(format!(
                "the Courant number must be in (0, 1], got {courant}"
            )));
        }
        // the axes the field varies along: not one cell long and periodic
        let varies = |axis: Axis| !(grid.n(axis) == 1 && boundaries.periodic(axis));
        let sum = bloch::curl_bound(&grid, &boundaries);
        if sum == 0.0 {
            return Err(invalid("the field varies along no axis"));
        }
        let dt = courant / sum.sqrt();
        let n = grid.cells();
        let h = [grid.dx, grid.dy, grid.dz];
        let mut eps_e: [Vec<f64>; 3] = [vec![0.0; n], vec![0.0; n], vec![0.0; n]];
        let complex = |x: f64, y: f64, z: f64| c64::new(eps(x, y, z), 0.0);
        for component in Axis::ALL {
            let values: Vec<f64> = (0..n)
                .into_iter()
                .map(|r| {
                    let (i, j, k) = (
                        r % grid.nx,
                        (r / grid.nx) % grid.ny,
                        r / (grid.nx * grid.ny),
                    );
                    let p = grid.e_position(component, (i, j, k));
                    averaged(&complex, p, h, component, SAMPLES).re
                })
                .collect();
            if let Some(bad) = values.iter().find(|v| !ok(**v)) {
                return Err(invalid(format!(
                    "the permittivity must be finite and positive, got {bad}"
                )));
            }
            eps_e[component.index()] = values;
        }
        let mut ca: [Vec<f64>; 3] = [vec![1.0; n], vec![1.0; n], vec![1.0; n]];
        let cb: [Vec<f64>; 3] =
            Axis::ALL.map(|c| eps_e[c.index()].iter().map(|e| dt / e).collect());
        let mut cb = cb;
        // walls: E tangential to a wall is zero on it, at node 0 (the wall at node n is beyond)
        for component in Axis::ALL {
            for axis in Axis::ALL {
                if axis == component || boundaries.periodic(axis) {
                    continue;
                }
                for r in 0..n {
                    let at = [
                        r % grid.nx,
                        (r / grid.nx) % grid.ny,
                        r / (grid.nx * grid.ny),
                    ];
                    if at[axis.index()] == 0 {
                        ca[component.index()][r] = 0.0;
                        cb[component.index()][r] = 0.0;
                    }
                }
            }
        }
        let mut s = Simulation {
            grid,
            boundaries,
            dt,
            steps: 0,
            e: [vec![0.0; n], vec![0.0; n], vec![0.0; n]],
            h: [vec![0.0; n], vec![0.0; n], vec![0.0; n]],
            ca,
            cb,
            kappa_nodes: Axis::ALL.map(|a| vec![1.0; grid.n(a)]),
            kappa_halves: Axis::ALL.map(|a| vec![1.0; grid.n(a)]),
            slabs: Vec::new(),
            sources: Vec::new(),
            currents: Vec::new(),
            plane_waves: Vec::new(),
            probes: Vec::new(),
            varies: Axis::ALL.map(varies),
            media: media::Media::default(),
            bloch: None,
        };
        s.build_cpml(&eps_e);
        s.bloch = bloch::Bloch::new(&s).map(Box::new);
        Ok(s)
    }

    /// The CPML's κ along each axis and its slabs' recursions.
    fn build_cpml(&mut self, eps_e: &[Vec<f64>; 3]) {
        let _ = eps_e;
        let grid = self.grid;
        let cpml = self.boundaries.cpml;
        for axis in Axis::ALL {
            let Edges::Pml { low, high } = self.boundaries.edges(axis) else {
                continue;
            };
            let n = grid.n(axis);
            let h = grid.step(axis);
            // σ, κ at a position along the axis (in cells from node 0), for a slab of `cells`
            let profile = |depth_cells: f64, cells: usize| -> (f64, f64) {
                let d = cells as f64 * h;
                let x = (depth_cells / cells as f64)
                    .clamp(0.0, 1.0)
                    .powf(cpml.order);
                let sigma_max = cpml
                    .sigma
                    .unwrap_or((cpml.order + 1.0) * (-cpml.reflection.ln()) / (2.0 * d));
                (sigma_max * x, 1.0 + (cpml.kappa - 1.0) * x)
            };
            let at = |pos: f64| -> (f64, f64) {
                if low > 0 && pos < low as f64 {
                    profile(low as f64 - pos, low)
                } else if high > 0 && pos > (n - high) as f64 {
                    profile(pos - (n - high) as f64, high)
                } else {
                    (0.0, 1.0)
                }
            };
            for m in 0..n {
                self.kappa_nodes[axis.index()][m] = 1.0 / at(m as f64).1;
                self.kappa_halves[axis.index()][m] = 1.0 / at(m as f64 + 0.5).1;
            }
            let dt = self.dt;
            let coefficients = |pos: f64| -> (f64, f64) {
                let (sigma, kappa) = at(pos);
                let b = (-(sigma / kappa + cpml.alpha) * dt).exp();
                let denominator = sigma * kappa + kappa * kappa * cpml.alpha;
                let a = if sigma > 0.0 && denominator > 0.0 {
                    sigma * (b - 1.0) / denominator
                } else {
                    0.0
                };
                (b, a)
            };
            let (b_axis, c_axis) = axis.others();
            let others = grid.cells() / n;
            for (from, count) in [(0, low), (n - high, high)] {
                if count == 0 {
                    continue;
                }
                // E's components across the axis sit at its nodes, H's halfway
                for (field, offset) in [(Field::E, 0.0), (Field::H, 0.5)] {
                    let (b, a): (Vec<f64>, Vec<f64>) = (from..from + count)
                        .map(|m| coefficients(m as f64 + offset))
                        .unzip();
                    for component in [b_axis, c_axis] {
                        self.slabs.push(Slab {
                            field,
                            component,
                            axis,
                            from,
                            count,
                            b: b.clone(),
                            a: a.clone(),
                            psi: vec![0.0; count * others],
                        });
                    }
                }
            }
        }
    }

    /// The grid.
    pub fn grid(&self) -> Grid3d {
        self.grid
    }

    /// The time step Δt, µm/c.
    pub fn dt(&self) -> f64 {
        self.dt
    }

    /// The steps taken so far.
    pub fn steps(&self) -> usize {
        self.steps
    }

    /// The time of the current E, µm/c: steps × Δt (H̃ is half a step behind).
    pub fn time(&self) -> f64 {
        self.steps as f64 * self.dt
    }

    /// E's `component`, one value per cell as [`Grid3d::index`] numbers them within a component.
    pub fn e(&self, component: Axis) -> &[f64] {
        &self.e[component.index()]
    }

    /// H̃'s `component`, half a step behind E.
    pub fn h(&self, component: Axis) -> &[f64] {
        &self.h[component.index()]
    }

    /// E's `component` to change, for initial conditions.
    pub fn e_mut(&mut self, component: Axis) -> &mut [f64] {
        &mut self.e[component.index()]
    }

    /// H̃'s `component` to change.
    pub fn h_mut(&mut self, component: Axis) -> &mut [f64] {
        &mut self.h[component.index()]
    }

    /// Whether the fields are complex: a Bloch phase on a side. [`Simulation::e`],
    /// [`Simulation::h`] and [`Simulation::probe`] are then the real parts.
    pub fn is_complex(&self) -> bool {
        self.bloch.is_some()
    }

    /// E's `component`'s imaginary part, if the fields are complex.
    pub fn e_imaginary(&self, component: Axis) -> Option<&[f64]> {
        self.bloch.as_ref().map(|b| b.twin.e(component))
    }

    /// H̃'s `component`'s imaginary part, if the fields are complex.
    pub fn h_imaginary(&self, component: Axis) -> Option<&[f64]> {
        self.bloch.as_ref().map(|b| b.twin.h(component))
    }

    /// E's `component`'s imaginary part to change, if the fields are complex.
    pub fn e_imaginary_mut(&mut self, component: Axis) -> Option<&mut [f64]> {
        self.bloch.as_mut().map(|b| b.twin.e_mut(component))
    }

    /// H̃'s `component`'s imaginary part to change, if the fields are complex.
    pub fn h_imaginary_mut(&mut self, component: Axis) -> Option<&mut [f64]> {
        self.bloch.as_mut().map(|b| b.twin.h_mut(component))
    }

    /// The conductivity σ(x, y, z) (in 1/µm with η₀ = 1: σ[S/m] × 376.73 Ω × 1e-6 m/µm), sampled
    /// at each value of E, for a lossy medium.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a conductivity that isn't finite and nonnegative, or after
    /// [`Simulation::with_medium`] (the conductivity comes first).
    pub fn with_conductivity(self, sigma: impl Fn(f64, f64, f64) -> f64) -> Result<Simulation> {
        self.conductivity(&sigma)
    }

    fn conductivity(mut self, sigma: &dyn Fn(f64, f64, f64) -> f64) -> Result<Simulation> {
        if !self.media.is_empty() {
            return Err(invalid(
                "the conductivity comes before the dispersive media: with_conductivity, then \
                 with_medium",
            ));
        }
        if let Some(mut b) = self.bloch.take() {
            b.twin = b.twin.conductivity(sigma)?;
            self.bloch = Some(b);
        }
        let grid = self.grid;
        for component in Axis::ALL {
            let c = component.index();
            for r in 0..grid.cells() {
                if self.cb[c][r] == 0.0 {
                    continue;
                }
                let (i, j, k) = (
                    r % grid.nx,
                    (r / grid.nx) % grid.ny,
                    r / (grid.nx * grid.ny),
                );
                let [x, y, z] = grid.e_position(component, (i, j, k));
                let s = sigma(x, y, z);
                if !(s.is_finite() && s >= 0.0) {
                    return Err(invalid(format!(
                        "the conductivity must be finite and >= 0, got {s}"
                    )));
                }
                // cb is Δt/ε until now
                let half = s * self.cb[c][r] / 2.0;
                self.ca[c][r] = (1.0 - half) / (1.0 + half);
                self.cb[c][r] /= 1.0 + half;
            }
        }
        Ok(self)
    }

    /// Holds E's `component` at `at` at zero: a perfect electric conductor there.
    pub fn conductor(&mut self, component: Axis, at: (usize, usize, usize)) {
        let r = self.index(at);
        let c = component.index();
        self.ca[c][r] = 0.0;
        self.cb[c][r] = 0.0;
        self.e[c][r] = 0.0;
        self.media.remove(c, r);
        if let Some(b) = &mut self.bloch {
            b.twin.conductor(component, at);
        }
    }

    /// A perfectly conducting plate normal to `normal` on its node `node`, covering the cells
    /// `first` and `second` along `normal.others()` (cell ranges, end excluded): its
    /// tangential E is held at zero on the plate's edges and inside.
    pub fn plate(
        &mut self,
        normal: Axis,
        node: usize,
        first: std::ops::Range<usize>,
        second: std::ops::Range<usize>,
    ) {
        let (a, b) = normal.others();
        let mut put = |component: Axis,
                       ra: std::ops::RangeInclusive<usize>,
                       rb: std::ops::RangeInclusive<usize>| {
            for ia in ra {
                for ib in rb.clone() {
                    let mut at = [0usize; 3];
                    at[normal.index()] = node;
                    at[a.index()] = ia;
                    at[b.index()] = ib;
                    if at[0] < self.grid.nx && at[1] < self.grid.ny && at[2] < self.grid.nz {
                        self.conductor(component, (at[0], at[1], at[2]));
                    }
                }
            }
        };
        // E along a: half positions along a (the cells), nodes along b (their edges included)
        put(
            a,
            first.start..=first.end.saturating_sub(1),
            second.start..=second.end,
        );
        put(
            b,
            first.start..=first.end,
            second.start..=second.end.saturating_sub(1),
        );
    }

    /// Adds a source.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if it is outside the grid.
    pub fn add_source(&mut self, source: Source) -> Result<()> {
        self.inside(source.at)?;
        self.sources.push(source);
        Ok(())
    }

    /// Records `field`'s `component` at `at` after every step (E at the step's end, H̃ half a
    /// step before), and returns the probe's number for [`Simulation::probe`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if `at` is outside the grid.
    pub fn add_probe(
        &mut self,
        field: Field,
        component: Axis,
        at: (usize, usize, usize),
    ) -> Result<usize> {
        self.inside(at)?;
        self.probes.push(Probe {
            field,
            component,
            index: self.index(at),
            values: Vec::new(),
        });
        if let Some(b) = &mut self.bloch {
            b.twin.add_probe(field, component, at)?;
        }
        Ok(self.probes.len() - 1)
    }

    /// Probe `n`'s values, one per step since it was added.
    ///
    /// # Panics
    ///
    /// If there is no probe `n`.
    pub fn probe(&self, n: usize) -> &[f64] {
        &self.probes[n].values
    }

    /// Probe `n`'s values as complex numbers: the real part [`Simulation::probe`], the
    /// imaginary part zero unless the fields are complex.
    ///
    /// # Panics
    ///
    /// If there is no probe `n`.
    pub fn probe_complex(&self, n: usize) -> Vec<c64> {
        let re = &self.probes[n].values;
        match &self.bloch {
            Some(b) => re
                .iter()
                .zip(b.twin.probe(n))
                .map(|(&a, &b)| c64::new(a, b))
                .collect(),
            None => re.iter().map(|&a| c64::new(a, 0.0)).collect(),
        }
    }

    fn inside(&self, (i, j, k): (usize, usize, usize)) -> Result<()> {
        if i < self.grid.nx && j < self.grid.ny && k < self.grid.nz {
            Ok(())
        } else {
            Err(invalid(format!(
                "({i}, {j}, {k}) is outside the {} x {} x {} grid",
                self.grid.nx, self.grid.ny, self.grid.nz
            )))
        }
    }

    fn index(&self, (i, j, k): (usize, usize, usize)) -> usize {
        (k * self.grid.ny + j) * self.grid.nx + i
    }

    /// Advances by `count` steps.
    pub fn run(&mut self, count: usize) {
        for _ in 0..count {
            self.step();
        }
    }

    /// Steps until the time of E reaches `time` (µm/c).
    pub fn run_until(&mut self, time: f64) {
        while self.time() < time {
            self.step();
        }
    }

    /// One step: H̃ from t − Δt/2 to t + Δt/2 with E at t, then E from t to t + Δt with H̃ at
    /// t + Δt/2.
    ///
    /// The magnetic currents M enter H̃'s update at t, the electric ones J E's at t + Δt/2. A
    /// plane wave's auxiliary grid steps alongside: its E at t gives the boxes' M, then its H̃
    /// steps to t + Δt/2 and gives their J, then its E steps to t + Δt.
    ///
    /// With a Bloch phase, the imaginary part steps alongside, each half step followed by the
    /// two parts' coupling across the Bloch sides; the dispersive media's currents step last,
    /// from E at t + Δt.
    pub fn step(&mut self) {
        let t = self.time();
        let half = t + 0.5 * self.dt;
        let mut bloch = self.bloch.take();
        self.step_h(t);
        if let Some(b) = &mut bloch {
            b.twin.step_h(t);
            b.wrap(Field::H, self);
        }
        self.step_e(half);
        if let Some(b) = &mut bloch {
            b.twin.step_e(half);
            b.wrap(Field::E, self);
            b.twin.finish_step();
        }
        self.bloch = bloch;
        self.finish_step();
    }

    /// H̃ from t − Δt/2 to t + Δt/2, with its sources at t.
    fn step_h(&mut self, t: f64) {
        self.update_h();
        for s in &self.sources {
            if s.field == Field::H {
                let r = self.index(s.at);
                self.h[s.component.index()][r] -= self.dt * s.waveform.at(t);
            }
        }
        for c in self.currents.iter().filter(|c| c.field == Field::H) {
            let w = c.waveform.complex_at(t);
            for &(component, r, a) in &c.values {
                self.h[component][r] -= self.dt * (a * w).re;
            }
        }
        for p in &mut self.plane_waves {
            p.correct(Field::H, &mut self.h, &self.cb, self.dt);
            p.step_h();
        }
    }

    /// E from t to t + Δt but for the dispersive media's currents, with its sources at
    /// `half` = t + Δt/2.
    fn step_e(&mut self, half: f64) {
        self.media.save(&self.e);
        self.update_e();
        for s in &self.sources {
            if s.field == Field::E {
                let r = self.index(s.at);
                let c = s.component.index();
                self.e[c][r] -= self.cb[c][r] * s.waveform.at(half);
            }
        }
        for c in self.currents.iter().filter(|c| c.field == Field::E) {
            let w = c.waveform.complex_at(half);
            for &(component, r, a) in &c.values {
                self.e[component][r] -= self.cb[component][r] * (a * w).re;
            }
        }
        for p in &mut self.plane_waves {
            p.correct(Field::E, &mut self.e, &self.cb, self.dt);
            p.step_e(half);
        }
    }

    /// The dispersive media's part of E's update and their currents' step, then the probes.
    fn finish_step(&mut self) {
        self.media.update(&mut self.e, self.dt);
        self.steps += 1;
        for p in &mut self.probes {
            let v = match p.field {
                Field::E => self.e[p.component.index()][p.index],
                Field::H => self.h[p.component.index()][p.index],
            };
            p.values.push(v);
        }
    }

    /// For each axis, the offset in a component's values from coordinate m to its neighbour one
    /// step forward or back: across a periodic side it wraps, beyond a wall it is `None` (the
    /// field is zero there).
    fn offsets(&self, forward: bool) -> [Vec<Option<isize>>; 3] {
        let g = self.grid;
        Axis::ALL.map(|axis| {
            let n = g.n(axis);
            let stride = [1, g.nx, g.nx * g.ny][axis.index()] as isize;
            let periodic = self.boundaries.periodic(axis);
            (0..n)
                .map(|m| {
                    if forward {
                        if m + 1 < n {
                            Some(stride)
                        } else if periodic {
                            Some(stride - n as isize * stride)
                        } else {
                            None
                        }
                    } else if m > 0 {
                        Some(-stride)
                    } else if periodic {
                        Some((n as isize - 1) * stride)
                    } else {
                        None
                    }
                })
                .collect()
        })
    }

    /// One component's update: `out = keep out + scale (D_a f_b − D_b f_a)`, D the difference
    /// along an axis (forward for H's updates, back for E's) times 1/(κ Δ) there; without
    /// `keep` and `scale`, `out −= Δt (D_a f_b − D_b f_a)` (H's update).
    #[allow(clippy::too_many_arguments)]
    fn curl_update(
        grid: Grid3d,
        out: &mut [f64],
        component: Axis,
        (fa, fb): (&[f64], &[f64]),
        offsets: &[Vec<Option<isize>>; 3],
        factors: &[Vec<f64>; 3],
        coefficients: Option<(&[f64], &[f64])>,
        dt: f64,
    ) {
        let (a, b) = component.others();
        let plane = grid.nx * grid.ny;
        let forward = coefficients.is_none();
        let difference = |f: &[f64], r: usize, offset: Option<isize>| -> f64 {
            let other = offset.map_or(0.0, |o| f[(r as isize + o) as usize]);
            if forward { other - f[r] } else { f[r] - other }
        };
        out.par_chunks_mut(plane)
            .enumerate()
            .for_each(|(k, values)| {
                for j in 0..grid.ny {
                    for i in 0..grid.nx {
                        let m = [i, j, k];
                        let r = k * plane + j * grid.nx + i;
                        let (ma, mb) = (m[a.index()], m[b.index()]);
                        let curl = difference(fb, r, offsets[a.index()][ma])
                            * factors[a.index()][ma]
                            - difference(fa, r, offsets[b.index()][mb]) * factors[b.index()][mb];
                        let v = &mut values[j * grid.nx + i];
                        match coefficients {
                            None => *v -= dt * curl,
                            Some((keep, scale)) => *v = keep[r] * *v + scale[r] * curl,
                        }
                    }
                }
            });
    }

    fn update_h(&mut self) {
        let offsets = self.offsets(true);
        let factors: [Vec<f64>; 3] = Axis::ALL.map(|a| {
            let h = self.grid.step(a);
            self.kappa_halves[a.index()].iter().map(|k| k / h).collect()
        });
        let mut h = std::mem::take(&mut self.h);
        for component in Axis::ALL {
            let (a, b) = component.others();
            Self::curl_update(
                self.grid,
                &mut h[component.index()],
                component,
                (&self.e[a.index()], &self.e[b.index()]),
                &offsets,
                &factors,
                None,
                self.dt,
            );
        }
        self.h = h;
        self.update_slabs(Field::H, &offsets);
    }

    fn update_e(&mut self) {
        let offsets = self.offsets(false);
        let factors: [Vec<f64>; 3] = Axis::ALL.map(|a| {
            let h = self.grid.step(a);
            self.kappa_nodes[a.index()].iter().map(|k| k / h).collect()
        });
        let mut e = std::mem::take(&mut self.e);
        for component in Axis::ALL {
            let (a, b) = component.others();
            let c = component.index();
            Self::curl_update(
                self.grid,
                &mut e[c],
                component,
                (&self.h[a.index()], &self.h[b.index()]),
                &offsets,
                &factors,
                Some((&self.ca[c], &self.cb[c])),
                self.dt,
            );
        }
        self.e = e;
        self.update_slabs(Field::E, &offsets);
    }

    /// The CPML's convolutions for `field`'s updates, added to the fields in their slabs.
    fn update_slabs(&mut self, field: Field, offsets: &[Vec<Option<isize>>; 3]) {
        let grid = self.grid;
        let plane = grid.nx * grid.ny;
        let forward = field == Field::H;
        let mut slabs = std::mem::take(&mut self.slabs);
        for slab in slabs.iter_mut().filter(|s| s.field == field) {
            let (w, c) = (slab.axis, slab.component);
            let (first, second) = c.others();
            // in c's curl, the derivative along `first` (of F_second) enters with +1, along
            // `second` (of F_first) with −1
            let (sign, differentiated) = if w == first {
                (1.0, second)
            } else {
                (-1.0, first)
            };
            let h = grid.step(w);
            let source = match field {
                Field::E => &self.h[differentiated.index()],
                Field::H => &self.e[differentiated.index()],
            };
            // the slab's extent along each axis, and its planes of ψ
            let mut from = [0usize; 3];
            let mut count = [grid.nx, grid.ny, grid.nz];
            from[w.index()] = slab.from;
            count[w.index()] = slab.count;
            let local_plane = count[0] * count[1];
            let (bs, as_) = (&slab.b, &slab.a);
            let start = slab.from;
            let off = &offsets[w.index()];
            slab.psi
                .par_chunks_mut(local_plane)
                .enumerate()
                .for_each(|(lk, psi)| {
                    for lj in 0..count[1] {
                        for li in 0..count[0] {
                            let m = [from[0] + li, from[1] + lj, from[2] + lk];
                            let r = m[2] * plane + m[1] * grid.nx + m[0];
                            let mw = m[w.index()];
                            let other = off[mw].map_or(0.0, |o| source[(r as isize + o) as usize]);
                            let d = if forward {
                                other - source[r]
                            } else {
                                source[r] - other
                            } / h;
                            let q = lj * count[0] + li;
                            let s = mw - start;
                            psi[q] = bs[s] * psi[q] + as_[s] * d;
                        }
                    }
                });
            let cb = &self.cb[c.index()];
            let dt = self.dt;
            let psi = &slab.psi;
            let target = match field {
                Field::E => &mut self.e[c.index()],
                Field::H => &mut self.h[c.index()],
            };
            target
                .par_chunks_mut(plane)
                .enumerate()
                .skip(from[2])
                .take(count[2])
                .for_each(|(k, values)| {
                    let lk = k - from[2];
                    for lj in 0..count[1] {
                        for li in 0..count[0] {
                            let (i, j) = (from[0] + li, from[1] + lj);
                            let u = sign * psi[lk * local_plane + lj * count[0] + li];
                            let v = &mut values[j * grid.nx + i];
                            match field {
                                Field::E => *v += cb[k * plane + j * grid.nx + i] * u,
                                Field::H => *v -= dt * u,
                            }
                        }
                    }
                });
        }
        self.slabs = slabs;
    }

    /// ½ Σ ε E·E ΔV with E now: the electric half of the energy the leapfrog conserves (with
    /// ½ Σ H̃(t − Δt/2)·H̃(t + Δt/2) ΔV, the magnetic half).
    pub(crate) fn electric_energy(&self) -> f64 {
        let volume = self.grid.dx * self.grid.dy * self.grid.dz;
        let mut sum = 0.0;
        for c in 0..3 {
            for (r, &v) in self.e[c].iter().enumerate() {
                if self.cb[c][r] != 0.0 {
                    // cb = Δt/ε where there's no conductivity
                    sum += self.dt / self.cb[c][r] * v * v;
                }
            }
        }
        0.5 * sum * volume
    }
}

mod bloch;
pub(crate) mod bloch_checks;
pub(crate) mod checks;
mod media;
pub(crate) mod media_checks;
mod sources;
pub use media::{Dispersive, Fit, Pole};
pub use sources::{BeamPolarization, Current, Dipole, GaussianBeam, PlaneWave};
#[cfg(test)]
mod media_tests;
#[cfg(test)]
mod tests;
