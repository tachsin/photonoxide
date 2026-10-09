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
//!   121 (1997), doi:10.1109/75.569723), sampled at the component. Or a [`Structure`] of
//!   bodies with tensor permittivities, smoothed over each cell by [`Simulation::smoothed`]
//!   (Farjadpour et al. 2006, Kottke et al. 2008, Oskooi et al. 2009; see the smoothing
//!   module): a tensor that couples E's components steps D, and E follows from it.
//! - **Sources:** point currents, [`Current`]s over many values and [`Dipole`]s anywhere
//!   (restricted to the grid as A. F. Oskooi et al., Comput. Phys. Commun. 181, 687 (2010),
//!   doi:10.1016/j.cpc.2009.11.008, restrict them); [`PlaneWave`]s on
//!   total-field/scattered-field boxes (K. Umashankar, A. Taflove, IEEE Trans. Electromagn.
//!   Compat. EMC-24, 397 (1982), doi:10.1109/TEMC.1982.304054), fed by an auxiliary 1D run
//!   with the grid's dispersion; one-way waveguide modes; [`GaussianBeam`]s. A run's DFT
//!   divided by [`Waveform::spectrum`] is FDFD's field at ω̃, exactly.
//! - **Monitors:** [`Dft`] transforms of E and H̃ over boxes, the flux through [`FluxPlane`]s and
//!   closed boxes on Yee's grid, and waveguide modes' amplitudes by FDFD's own projection;
//!   resonances by [`harmonic_inversion`] (V. A. Mandelshtam, H. S. Taylor, J. Chem. Phys. 107,
//!   6756 (1997), doi:10.1063/1.475324); runs until the fields decay.
//! - **Adjoint gradients** of an objective on the monitors with respect to every cell's density
//!   in a [`Design`] region, from the forward run and one adjoint run ([`Simulation::adjoint`],
//!   [`Simulation::gradient`]): exact for the scheme, from the transforms alone (see the adjoint
//!   module; C. M. Lalau-Keraly et al., Opt. Express 21, 21693 (2013),
//!   doi:10.1364/OE.21.021693).
//! - **Mie's series** for a plane wave on a sphere, [`Mie`] and [`Sphere`] (G. Mie, Ann. Phys. 330,
//!   377 (1908), doi:10.1002/andp.19083300302), written from his paper: FDTD's spheres are
//!   checked against it.
//!
//! Every update is a sum over a fixed stencil with no reduction, its rows shared among
//! rayon's threads (a 2D grid's too): the fields are the same bits on any number of threads.

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
    /// The maximum of an ordinary electric conductivity added inside the CPML, in 1/µm
    /// (σ\[S/m\] × η₀, as [`Simulation::with_conductivity`]'s), rising from 0 at the CPML's
    /// inner face as (depth/d)^(3m), three times as steeply as σ (0: none, the default).
    ///
    /// For long runs with a guide that runs into the CPML. There the fields can decay and
    /// then grow again, exponentially, at a rate no σ, κ or α removes (measured:
    /// docs/methods/fdtd.md, "Late growth"). It is what a PML does to a wave whose phase and
    /// group velocities point opposite ways along it: P.-R. Loh, A. F. Oskooi, M. Ibanescu,
    /// M. Skorobogatiy, S. G. Johnson, Phys. Rev. E 79, 065601 (2009),
    /// doi:10.1103/PhysRevE.79.065601. An ordinary loss damps a wave whatever its direction.
    ///
    /// Measured on a strip of ε = 6 through CPMLs of 6 cells: 5/µm stops the growth. The
    /// price is reflection, the loss being electric alone: a CPML of 16 cells then reflects
    /// 6.2e-4 of a pulse where it reflected 6.1e-6, and one of 6 cells 6.9e-4 where it
    /// reflected 1.4e-3. Not with a permittivity that couples E's components.
    pub damping: f64,
}

impl Default for Cpml {
    /// FDFD's PML: R = 1e-8, order 3, κ = 1, α = 0, no damping.
    fn default() -> Cpml {
        Cpml {
            reflection: 1e-8,
            sigma: None,
            order: 3.0,
            kappa: 1.0,
            alpha: 0.0,
            damping: 0.0,
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
    slabs: Vec<kernel::Slab<f64>>,
    /// E's (back) and H̃'s (forward) differences, tabulated once.
    stencils: [kernel::Stencil<f64>; 2],
    /// Whether to step with the plain loops the kernel replaced.
    reference: bool,
    /// How the kernel cuts the grid into tiles ([`kernel::Tiling`]).
    tiling: kernel::Tiling,
    /// Where the sources and currents add, for the tiled kernel: built when first needed.
    points: sources::Points,
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
    /// A smoothed permittivity whose tensor couples E's components: D, and E from it.
    anisotropic: Option<Box<smoothing::Anisotropic>>,
    /// Transforms, flux and mode monitors.
    monitors: monitors::Monitors,
}

/// How much more steeply a CPML's damping is graded than its σ: as depth⁹ for σ's depth³. It
/// then sits where the CPML has already absorbed what came in, and reflects little of it;
/// measured (docs/methods/fdtd.md, "Late growth"), depth³ reflects twenty times more, and
/// depth¹² no longer reaches the wave it is there to damp.
const DAMPING_GRADING: f64 = 3.0;

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
        Simulation::with_permittivity(grid, boundaries, courant, |grid| {
            let ok = |v: f64| v.is_finite() && v > 0.0;
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
            Ok(eps_e)
        })
    }

    /// [`Simulation::new`] with the permittivity's samples taken on rayon's threads, each value's
    /// average alone, so the same bits on any number of threads: for a permittivity that takes
    /// a while to evaluate at each of a value's 512 samples, as a job's layer stack does.
    ///
    /// # Errors
    ///
    /// As [`Simulation::new`].
    pub(crate) fn new_parallel(
        grid: Grid3d,
        eps: impl Fn(f64, f64, f64) -> f64 + Sync,
        boundaries: Boundaries,
        courant: f64,
    ) -> Result<Simulation> {
        Simulation::with_permittivity(grid, boundaries, courant, |grid| {
            let ok = |v: f64| v.is_finite() && v > 0.0;
            let h = [grid.dx, grid.dy, grid.dz];
            let complex = |x: f64, y: f64, z: f64| c64::new(eps(x, y, z), 0.0);
            let mut eps_e: [Vec<f64>; 3] = [Vec::new(), Vec::new(), Vec::new()];
            for component in Axis::ALL {
                let values: Vec<f64> = (0..grid.cells())
                    .into_par_iter()
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
            Ok(eps_e)
        })
    }

    /// The problem in a uniform medium of relative permittivity `eps`, with no averaging to
    /// do: for grids too large to sample, as the kernel's benchmarks step.
    ///
    /// # Errors
    ///
    /// As [`Simulation::new`].
    #[cfg_attr(not(test), allow(dead_code))] // read by the tests
    pub(crate) fn in_uniform_medium(
        grid: Grid3d,
        eps: f64,
        boundaries: Boundaries,
        courant: f64,
    ) -> Result<Simulation> {
        Simulation::with_permittivity(grid, boundaries, courant, |grid| {
            if !(eps.is_finite() && eps > 0.0) {
                return Err(invalid(format!(
                    "the permittivity must be finite and positive, got {eps}"
                )));
            }
            Ok(std::array::from_fn(|_| vec![eps; grid.cells()]))
        })
    }

    /// The problem with the permittivity each value of E sees from `eps`, one per value of each
    /// component, asked for once the grid and the boundaries are checked.
    fn with_permittivity(
        grid: Grid3d,
        boundaries: Boundaries,
        courant: f64,
        eps: impl FnOnce(&Grid3d) -> Result<[Vec<f64>; 3]>,
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
            || !(c.damping.is_finite() && c.damping >= 0.0)
            || c.sigma.is_some_and(|s| !(s.is_finite() && s >= 0.0))
        {
            return Err(invalid(format!(
                "the CPML needs R in (0, 1), an order >= 0, kappa >= 1, alpha >= 0, a damping >= \
                 0 and sigma >= 0, got {c:?}"
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
        let eps_e = eps(&grid)?;
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
            stencils: std::array::from_fn(|a| {
                kernel::Stencil::new(
                    grid,
                    a == 1,
                    [vec![], vec![], vec![]],
                    &[vec![], vec![], vec![]],
                )
            }),
            reference: false,
            tiling: kernel::Tiling::default(),
            points: sources::Points::default(),
            sources: Vec::new(),
            currents: Vec::new(),
            plane_waves: Vec::new(),
            probes: Vec::new(),
            varies: Axis::ALL.map(varies),
            media: media::Media::default(),
            bloch: None,
            anisotropic: None,
            monitors: monitors::Monitors::default(),
        };
        s.build_cpml(&eps_e);
        s.build_stencils();
        s.bloch = bloch::Bloch::new(&s).map(Box::new);
        if c.damping > 0.0 {
            // the CPML's own loss, with no other conductivity yet
            s = s.conductivity(&|_, _, _| 0.0)?;
        }
        Ok(s)
    }

    /// The CPML's damping at a point ([`Cpml::damping`]): its maximum times the depth into
    /// the deepest CPML there, as a fraction of its thickness, to [`DAMPING_GRADING`] times
    /// the grading's order; 0 outside.
    fn damping_at(&self, position: [f64; 3]) -> f64 {
        let cpml = self.boundaries.cpml;
        if cpml.damping == 0.0 {
            return 0.0;
        }
        let grid = self.grid;
        let origin = [grid.x0, grid.y0, grid.z0];
        let mut deepest: f64 = 0.0;
        for axis in Axis::ALL {
            let Edges::Pml { low, high } = self.boundaries.edges(axis) else {
                continue;
            };
            let a = axis.index();
            let n = grid.n(axis) as f64;
            // in cells from node 0, as the CPML's own profile
            let pos = (position[a] - origin[a]) / grid.step(axis);
            let depth = if low > 0 && pos < low as f64 {
                (low as f64 - pos) / low as f64
            } else if high > 0 && pos > n - high as f64 {
                (pos - (n - high as f64)) / high as f64
            } else {
                0.0
            };
            deepest = deepest.max(depth.clamp(0.0, 1.0));
        }
        cpml.damping * deepest.powf(DAMPING_GRADING * cpml.order)
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
                        self.slabs.push(kernel::Slab {
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
    /// [`Error::InvalidValue`] for a conductivity that isn't finite and nonnegative, after
    /// [`Simulation::with_medium`] (the conductivity comes first), or in a smoothed medium whose
    /// permittivity couples E's components.
    ///
    /// A CPML's damping ([`Cpml::damping`]) adds to it inside the CPML.
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
        if self.anisotropic.is_some() {
            return Err(invalid(
                "a conductivity in a medium whose permittivity couples E's components isn't \
                 supported",
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
                let s = s + self.damping_at([x, y, z]);
                // Δt/ε, whatever conductivity the value had (the CPML's damping alone, set
                // when the problem was built): with none, cb itself to the bit
                let plain = 2.0 * self.cb[c][r] / (1.0 + self.ca[c][r]);
                let half = s * plain / 2.0;
                self.ca[c][r] = (1.0 - half) / (1.0 + half);
                self.cb[c][r] = plain / (1.0 + half);
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
        if let Some(a) = &mut self.anisotropic {
            a.hold(c, r);
        }
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
    ///
    /// On a grid beyond the caches the kernel takes several steps at a time by tiles (Malas et
    /// al.'s diamonds), copying out what the probes and monitors read at each step, where it
    /// can: not with a dispersive medium, a Bloch side, a side periodic along y or z, a smoothed
    /// tensor or a plane wave. The same bits as one step at a time.
    pub fn run(&mut self, count: usize) {
        let mut left = count;
        if let Some(b) = self.tiles(true).filter(|b| b.steps > 1) {
            while left > 0 {
                let n = left.min(b.steps);
                self.step_tiled(b, n);
                left -= n;
            }
        }
        for _ in 0..left {
            self.step();
        }
    }

    /// Steps until the time of E reaches `time` (µm/c), as [`Simulation::run`] does.
    pub fn run_until(&mut self, time: f64) {
        let mut count = 0;
        while ((self.steps + count) as f64 * self.dt) < time {
            count += 1;
        }
        self.run(count);
    }

    /// The tiles the kernel steps this problem by, `temporal`ly (several steps at a time) or
    /// one step at a time, if it can: not with the plain loops, a Bloch phase, a tensor
    /// permittivity or a plane wave, whose updates read the whole grid between H̃'s and E's;
    /// not across a side that wraps along y or z; and several steps at a time only with nothing
    /// to record or update between steps (probes, monitors, dispersive media). Otherwise, and
    /// on a grid the caches hold, the whole grid every step ([`kernel::Blocking::auto`]).
    pub(crate) fn tiles(&self, temporal: bool) -> Option<kernel::Blocking> {
        if self.reference
            || self.bloch.is_some()
            || self.anisotropic.is_some()
            || !self.plane_waves.is_empty()
        {
            return None;
        }
        let temporal = temporal && self.media.is_empty();
        let b = match self.tiling {
            kernel::Tiling::Whole => None,
            kernel::Tiling::Fixed(b) if temporal => Some(b),
            kernel::Tiling::Fixed(b) => Some(kernel::Blocking { steps: 1, ..b }),
            kernel::Tiling::Auto => kernel::Blocking::auto(
                self.grid,
                std::mem::size_of::<f64>(),
                temporal,
                rayon::current_num_threads(),
            ),
        }?;
        kernel::blocked::blockable(self.grid, &self.stencils).then_some(b)
    }

    /// How the kernel cuts the grid into tiles from now on: for comparing the paths.
    pub(crate) fn set_tiling(&mut self, tiling: kernel::Tiling) {
        self.tiling = tiling;
    }

    /// `n` steps by the tiles `b`: H̃ and E plane by plane in each tile, the sources and
    /// currents added as each value is updated, then what [`Simulation::step`] does after E's
    /// update. Several steps only with no dispersive medium ([`Simulation::tiles`]): the probes'
    /// and monitors' values are copied out of each step as the tiles update them, and recorded
    /// after, step by step.
    fn step_tiled(&mut self, b: kernel::Blocking, n: usize) {
        self.media.save(&self.e);
        let mut points = std::mem::take(&mut self.points);
        points.amounts(self, self.steps, n);
        let lists: Vec<&[kernel::Injection<f64>]> = (0..n).map(|m| points.step(m)).collect();
        // the probes' values, then the monitors' boxes
        let taps: Vec<kernel::Tap> = if n > 1 {
            let g = self.grid;
            self.probes
                .iter()
                .map(|p| {
                    let (i, j, k) = (
                        p.index % g.nx,
                        (p.index / g.nx) % g.ny,
                        p.index / g.nx / g.ny,
                    );
                    kernel::Tap {
                        field: p.field,
                        component: p.component.index(),
                        ranges: [i..i + 1, j..j + 1, k..k + 1],
                    }
                })
                .chain(self.monitors.taps())
                .collect()
        } else {
            Vec::new()
        };
        let volume: usize = taps.iter().map(kernel::Tap::volume).sum();
        let mut captured = vec![0.0; n * volume];
        kernel::blocked::step_blocked(
            kernel::blocked::Fields {
                grid: self.grid,
                e: &mut self.e,
                h: &mut self.h,
                ca: &self.ca,
                cb: &self.cb,
                slabs: &mut self.slabs,
                stencils: &self.stencils,
                dt: self.dt,
            },
            b,
            n,
            kernel::blocked::Extras {
                injections: &lists,
                taps: &taps,
                captured: &mut captured,
            },
        );
        drop(lists);
        self.points = points;
        if n == 1 {
            return self.finish_step();
        }
        // as finish_step, from the copies: no dispersive medium to update
        for m in 0..n {
            let step = &captured[m * volume..(m + 1) * volume];
            self.steps += 1;
            let (probes, mut boxes) = step.split_at(self.probes.len());
            for (p, &v) in self.probes.iter_mut().zip(probes) {
                p.values.push(v);
            }
            let te = self.time();
            self.monitors
                .record_captured(&mut boxes, [te, te - 0.5 * self.dt], self.dt);
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
    ///
    /// On a grid beyond the caches the kernel steps H̃ and E together by tiles, plane by plane,
    /// where it can: the same bits.
    pub fn step(&mut self) {
        if let Some(b) = self.tiles(false) {
            return self.step_tiled(b, 1);
        }
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
        // a tensor permittivity: D is updated as E would be in vacuum, then E = ε⁻¹D
        if let Some(a) = &mut self.anisotropic {
            a.begin(&mut self.e, &mut self.cb);
        }
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
        if let Some(a) = &mut self.anisotropic {
            a.end(&mut self.e, &mut self.cb);
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
        let te = self.time();
        let imaginary = self.bloch.as_ref().map(|b| [&b.twin.e, &b.twin.h]);
        self.monitors.record(
            self.grid,
            [&self.e, &self.h],
            imaginary,
            [te, te - 0.5 * self.dt],
            self.dt,
        );
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

    /// H̃ from t − Δt/2 to t + Δt/2 by the curl of E: the kernel, or the plain loops it replaced
    /// ([`Simulation::use_reference_kernel`]).
    fn update_h(&mut self) {
        if self.reference {
            return self.update_h_reference();
        }
        let stencil = &self.stencils[1];
        kernel::curl_update(self.grid, &mut self.h, &self.e, stencil, None, self.dt);
        kernel::update_slabs(
            self.grid,
            &mut self.slabs,
            Field::H,
            &mut self.h,
            &self.e,
            &self.cb,
            stencil,
            self.dt,
        );
    }

    /// E from t to t + Δt by the curl of H̃, as [`Simulation::update_h`].
    fn update_e(&mut self) {
        if self.reference {
            return self.update_e_reference();
        }
        let stencil = &self.stencils[0];
        kernel::curl_update(
            self.grid,
            &mut self.e,
            &self.h,
            stencil,
            Some((&self.ca, &self.cb)),
            self.dt,
        );
        kernel::update_slabs(
            self.grid,
            &mut self.slabs,
            Field::E,
            &mut self.e,
            &self.h,
            &self.cb,
            stencil,
            self.dt,
        );
    }

    /// Steps with the plain loops the kernel replaced, from now on: the reference the kernel
    /// is checked against (the same bits).
    pub(crate) fn use_reference_kernel(&mut self) {
        self.reference = true;
        if let Some(b) = &mut self.bloch {
            b.twin.reference = true;
        }
    }

    /// The tables of E's (back) and H̃'s (forward) differences, built once.
    fn build_stencils(&mut self) {
        let grid = self.grid;
        let factors = |kappa: &[Vec<f64>; 3]| -> [Vec<f64>; 3] {
            std::array::from_fn(|a| {
                let h = grid.step(Axis::ALL[a]);
                kappa[a].iter().map(|k| k / h).collect()
            })
        };
        self.stencils = [
            kernel::Stencil::new(
                grid,
                false,
                self.offsets(false),
                &factors(&self.kappa_nodes),
            ),
            kernel::Stencil::new(grid, true, self.offsets(true), &factors(&self.kappa_halves)),
        ];
    }

    /// ½ Σ ε E·E ΔV with E now: the electric half of the energy the leapfrog conserves (with
    /// ½ Σ H̃(t − Δt/2)·H̃(t + Δt/2) ΔV, the magnetic half).
    pub(crate) fn electric_energy(&self) -> f64 {
        let volume = self.grid.dx * self.grid.dy * self.grid.dz;
        if let Some(a) = &self.anisotropic {
            // ½ Σ E·D, E = ε⁻¹D with ε⁻¹ symmetric
            return 0.5 * a.dot(&self.e) * volume;
        }
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

pub(crate) mod adjoint;
pub(crate) mod adjoint_checks;
#[cfg(test)]
mod adjoint_tests;
pub(crate) mod agreement_checks;
mod bloch;
pub(crate) mod bloch_checks;
pub(crate) mod checks;
mod harmonic;
#[cfg(test)]
mod harmonic_tests;
mod kernel;
#[cfg(test)]
mod kernel_rates;
#[cfg(test)]
mod kernel_tests;
mod media;
pub(crate) mod media_checks;
pub(crate) mod meep_checks;
pub(crate) mod mie;
pub(crate) mod mie_checks;
#[cfg(test)]
mod mie_tests;
mod monitors;
pub(crate) mod monitors_checks;
#[cfg(test)]
mod monitors_tests;
pub(crate) mod ring;
pub(crate) mod smoothing;
mod sources;
pub use adjoint::{Design, Term, ValueGradient};
pub use harmonic::{Resonance, harmonic_inversion};
pub(crate) use kernel::{Blocking, Real, Yee};
pub use media::{Dispersive, Fit, Pole};
pub use mie::{CrossSections, Mie, Sphere};
pub use monitors::{Dft, FluxPlane};
pub use smoothing::{Average, Body, Coupling, Permittivity, Smoothing, Structure};
pub use sources::{BeamPolarization, Current, Dipole, GaussianBeam, PlaneWave};
#[cfg(test)]
mod media_tests;
#[cfg(test)]
mod tests;
