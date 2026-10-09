//! The adjoint gradients' setups that both the tests and the validation report run.

use num_complex::Complex64 as c64;
use rayon::prelude::*;

use super::checks::{courant_for, grid, guide};
use super::{Boundaries, Design, Field, FluxPlane, Simulation, Term, Waveform};
use crate::fdfd::{
    Axis, Boundaries3d, Direction, Edges, Formulation, IterativeSolver3d, PortMode3d, Solver3d,
};
use crate::units::Frequency;

/// How far the transforms may move over a block of 1 µm/c when the open runs stop, relative to
/// each monitor's largest: they are then the frequency-domain fields to about that. A pulse
/// whose currents have complex amplitudes (a mode source's) leaves a static field of about
/// e^(−(ωτ/2)²) of its peak, 2e-10 for the strip's, which moves a transform by about 1e-12 of
/// its largest over a block and never dies in a CPML: the tolerance stays above that.
pub(crate) const SETTLED: f64 = 1e-11;

/// The same in the lossy box, over blocks of 2 µm/c: its loss takes everything away.
const BOX_SETTLED: f64 = 1e-13;

/// The longest a run may take, µm/c: beyond what any here needs, and short of the growth from
/// round-off a CPML (α = 0) shows after some hundreds of µm/c (see docs/methods/fdtd-adjoint.md).
const LIMIT: f64 = 300.0;

/// The cells along each axis of [`Strip`].
const STRIP: [usize; 3] = [36, 20, 20];

/// A strip guide along x in 3D, a core of ε = 6, 0.3 × 0.3 µm, in vacuum, on cells of 50 nm
/// (36 × 20 × 20, CPMLs of 6 all round), at Courant number 0.99: its mode launched forward on
/// plane 10 by a pulse at 1 c/µm, 0.25 c/µm wide. The design is a box of 3 × 2 × 2 cells in
/// the core, ε from 1 to 6, between the source and the monitors.
pub(crate) struct Strip {
    pub(crate) design: Design,
    /// The mode's frequencies: the carrier and 6 % above it.
    pub(crate) frequencies: Vec<Frequency>,
    /// The guide's mode at each frequency, at the leapfrog's frequency, on its plane.
    modes: Vec<PortMode3d>,
    /// The planes the modes are measured on: behind the source and ahead of the design.
    behind: usize,
    ahead: usize,
}

/// The strip's permittivity outside the design.
pub(crate) fn strip(_: f64, y: f64, z: f64) -> f64 {
    if y.abs() < 0.15 && z.abs() < 0.15 {
        6.0
    } else {
        1.0
    }
}

/// The densities the gradients are checked at: gray and uneven, so that no two cells'
/// gradients are alike.
pub(crate) fn uneven(design: &Design) -> Vec<f64> {
    (0..design.len())
        .map(|v| 0.25 + 0.5 * ((v as f64 * 0.618_034).fract()))
        .collect()
}

impl Strip {
    pub(crate) fn new() -> Strip {
        let g = grid(STRIP, 0.05);
        let s = Strip::empty();
        let frequencies = [1.0, 1.06].map(|f| Frequency::natural(f).unwrap()).to_vec();
        // each mode on a few planes cut out of the grid, at the leapfrog's frequency
        let modes = frequencies
            .iter()
            .map(|&f| {
                let mut cut = g;
                cut.nx = 8;
                let solver = IterativeSolver3d::new(
                    cut,
                    s.fdfd_wavelength(f).unwrap(),
                    |x, y, z| c64::new(strip(x, y, z), 0.0),
                    Boundaries3d {
                        x: Edges::Pml { low: 0, high: 0 },
                        ..Boundaries3d::pml(6)
                    },
                    Formulation::CurlCurl,
                )
                .unwrap();
                solver.port_modes(Axis::X, 3, 1).unwrap().remove(0)
            })
            .collect();
        Strip {
            design: Design {
                cells: [15..18, 9..11, 9..11],
                eps: [1.0, 6.0],
            },
            frequencies,
            modes,
            behind: 8,
            ahead: 24,
        }
    }

    /// The simulation without a design or sources.
    fn empty() -> Simulation {
        Simulation::new(grid(STRIP, 0.05), strip, Boundaries::cpml(6), 0.99).unwrap()
    }

    fn mode(&self, f: usize, plane: usize) -> PortMode3d {
        self.modes[f].clone().moved_to(plane)
    }

    /// The simulation at `density` with the mode source, not yet run.
    pub(crate) fn forward_unrun(&self, density: &[f64]) -> Simulation {
        let mut s = Strip::empty()
            .with_design(&self.design, density, strip)
            .unwrap();
        s.add_mode_source(
            &self.mode(0, 10),
            Direction::Forward,
            Waveform::pulse(self.frequencies[0], 0.25).unwrap(),
        )
        .unwrap();
        s
    }

    /// The forward run at `density`, run until its fields are quiet, with its monitors: the
    /// modes behind the source and ahead of the design (one monitor each, the two frequencies'
    /// modes in it), the modes ahead with their imaginary parts dropped, a flux plane ahead
    /// (its window reaching into the CPMLs across) at both frequencies, a transform of E_y at
    /// a point ahead, and the design's.
    pub(crate) fn forward(&self, density: &[f64]) -> Run {
        self.monitored(self.forward_unrun(density))
    }

    /// [`Strip::forward`] from the unrun simulation `s`.
    pub(crate) fn monitored(&self, mut s: Simulation) -> Run {
        let behind = s
            .add_mode_monitor(&[self.mode(0, self.behind), self.mode(1, self.behind)])
            .unwrap();
        let ahead = s
            .add_mode_monitor(&[self.mode(0, self.ahead), self.mode(1, self.ahead)])
            .unwrap();
        let real = s
            .add_mode_monitor(&[
                self.mode(0, self.ahead).real_part(),
                self.mode(1, self.ahead).real_part(),
            ])
            .unwrap();
        let frequencies = s.mode_frequencies(ahead);
        let flux = s
            .add_flux(
                FluxPlane::new(Axis::X, 25).within((5, 14), (5, 14)),
                &frequencies,
            )
            .unwrap();
        let point = s.add_dft(POINT, POINT, &frequencies).unwrap();
        let design = s.add_design_monitor(&self.design, &frequencies).unwrap();
        s.run_until_converged(SETTLED, 1.0, LIMIT).unwrap();
        Run {
            s,
            behind,
            ahead,
            real,
            flux,
            point,
            design,
        }
    }
}

/// The value of E_y the strip's flux objective reads: in the core, ahead of the design.
const POINT: (usize, usize, usize) = (26, 11, 9);

/// A forward run of [`Strip`] and its monitors' numbers.
pub(crate) struct Run {
    pub(crate) s: Simulation,
    behind: usize,
    ahead: usize,
    real: usize,
    flux: usize,
    point: usize,
    design: usize,
}

/// Which objective [`Run`] is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Objective {
    /// The power into the guide's mode ahead at both frequencies, less half the power
    /// reflected into it behind at the carrier: F = |a₊(f₀)|² + |a₊(f₁)|² − ½|a₋(f₀)|², each
    /// amplitude in the transforms' units.
    Modes,
    /// The same, its adjoint's sources from the modes ahead with their imaginary parts
    /// dropped (the objective itself unchanged): a wrong gradient, for the check that shows it.
    #[cfg_attr(not(test), allow(dead_code))]
    ModesAsReal,
    /// The flux through the plane ahead at the carrier plus |E_y|² at the point ahead at the
    /// second frequency.
    Flux,
}

impl Run {
    /// The objective and its terms.
    pub(crate) fn objective(&self, objective: Objective) -> (f64, Vec<Term>) {
        match objective {
            Objective::Modes | Objective::ModesAsReal => {
                let ahead = self.s.mode_amplitudes(self.ahead);
                let behind = self.s.mode_amplitudes(self.behind);
                let value =
                    ahead[0].0.norm_sqr() + ahead[1].0.norm_sqr() - 0.5 * behind[0].1.norm_sqr();
                let projected = if objective == Objective::Modes {
                    self.ahead
                } else {
                    self.real
                };
                let terms = vec![
                    Term::Mode {
                        monitor: projected,
                        mode: 0,
                        direction: Direction::Forward,
                        derivative: ahead[0].0.conj(),
                    },
                    Term::Mode {
                        monitor: projected,
                        mode: 1,
                        direction: Direction::Forward,
                        derivative: ahead[1].0.conj(),
                    },
                    Term::Mode {
                        monitor: self.behind,
                        mode: 0,
                        direction: Direction::Backward,
                        derivative: -0.5 * behind[0].1.conj(),
                    },
                ];
                (value, terms)
            }
            Objective::Flux => {
                let flux = self.s.flux(self.flux)[0];
                let e = self
                    .s
                    .dft(self.point)
                    .value(Field::E, Axis::Y, POINT, 1)
                    .unwrap();
                let terms = vec![
                    Term::Flux {
                        monitor: self.flux,
                        frequency: 0,
                        derivative: 1.0,
                    },
                    Term::Transform {
                        monitor: self.point,
                        field: Field::E,
                        component: Axis::Y,
                        at: POINT,
                        frequency: 1,
                        derivative: e.conj(),
                    },
                ];
                (flux + e.norm_sqr(), terms)
            }
        }
    }

    /// The adjoint run of `objective`, run until its fields are quiet.
    pub(crate) fn adjoint(&self, objective: Objective) -> Simulation {
        let (_, terms) = self.objective(objective);
        let mut adjoint = self.s.adjoint(&terms, self.design, 0.25).unwrap();
        adjoint.run_until_converged(SETTLED, 1.0, LIMIT).unwrap();
        adjoint
    }

    /// The gradient of `objective` with respect to the densities, by the adjoint run.
    pub(crate) fn gradient(&self, objective: Objective) -> Vec<f64> {
        self.s
            .gradient(&self.adjoint(objective), self.design)
            .unwrap()
    }
}

/// The largest difference between `a` and `b`, relative to the largest of `b`.
fn relative(a: &[f64], b: &[f64]) -> f64 {
    let largest = b.iter().fold(0.0f64, |m, d| m.max(d.abs()));
    a.iter()
        .zip(b)
        .fold(0.0f64, |m, (x, y)| m.max((x - y).abs()))
        / largest
}

/// [`Strip`]: the gradient of `objective` with respect to every design cell's density by
/// fourth-order central differences (δ = 1e-3).
pub(crate) fn strip_differences(objective: Objective) -> Vec<f64> {
    let strip = Strip::new();
    let density = uneven(&strip.design);
    let delta = 1e-3;
    let value = |v: usize, d: f64| {
        let mut rho = density.clone();
        rho[v] += d;
        strip.forward(&rho).objective(objective).0
    };
    (0..density.len())
        .into_par_iter()
        .map(|v| {
            (8.0 * (value(v, delta) - value(v, -delta))
                - (value(v, 2.0 * delta) - value(v, -2.0 * delta)))
                / (12.0 * delta)
        })
        .collect()
}

/// [`Strip`]: the gradient of `objective` by the adjoint against [`strip_differences`], at
/// every design cell. The largest difference relative to the largest gradient, and the pairs.
pub(crate) fn strip_against_differences(objective: Objective) -> (f64, Vec<(f64, f64)>) {
    let strip = Strip::new();
    let run = strip.forward(&uneven(&strip.design));
    let adjoint = run.gradient(objective);
    let differences = strip_differences(objective);
    (
        relative(&adjoint, &differences),
        adjoint.into_iter().zip(differences).collect(),
    )
}

/// [`Strip`]: the mode objective's gradient with the adjoint's sources from the modes with
/// their imaginary parts dropped, against the finite differences: the largest difference
/// relative to the largest gradient.
#[cfg_attr(not(test), allow(dead_code))] // read by the tests
pub(crate) fn real_mode_error() -> f64 {
    let strip = Strip::new();
    let run = strip.forward(&uneven(&strip.design));
    relative(
        &run.gradient(Objective::ModesAsReal),
        &strip_differences(Objective::Modes),
    )
}

/// [`Strip`]'s gradient of the mode objective, forward and adjoint runs on `threads` threads
/// with the kernel's tiles chosen by `tiling`.
#[cfg_attr(not(test), allow(dead_code))] // read by the tests
pub(crate) fn strip_gradient_on(threads: usize, tiling: super::kernel::Tiling) -> Vec<f64> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap()
        .install(|| {
            let strip = Strip::new();
            let mut forward = strip.forward_unrun(&uneven(&strip.design));
            forward.set_tiling(tiling);
            let tiled = forward.tiles(false).is_some();
            assert_eq!(tiled, matches!(tiling, super::kernel::Tiling::Fixed(_)));
            let run = strip.monitored(forward);
            run.gradient(Objective::Modes)
        })
}

/// FDFD on the permittivities and conductivities `s` has at each value of E, at the leapfrog's
/// frequency for `frequency` (ε + iσ cos(ωΔt/2)/ω̃), inside `boundaries`: the frequency-domain
/// problem FDTD's transforms solve, but for the CPMLs, which FDFD's PMLs are only to first
/// order in Δt.
fn fdfd_of(s: &Simulation, frequency: Frequency, boundaries: Boundaries3d) -> Solver3d {
    let g = s.grid();
    let tilde = s.leapfrog_wavenumber(frequency);
    let lag = (frequency.angular() * s.dt() / 2.0).cos();
    let solver = Solver3d::new(
        g,
        s.fdfd_wavelength(frequency).unwrap(),
        |_, _, _| c64::new(1.0, 0.0),
        boundaries,
    )
    .unwrap();
    let mut eps = vec![c64::new(1.0, 0.0); g.unknowns()];
    for component in Axis::ALL {
        for k in 0..g.nz {
            for j in 0..g.ny {
                for i in 0..g.nx {
                    if let Some((e, sigma)) = s.medium_at(component, (i, j, k)) {
                        eps[g.index(component, (i, j, k))] = c64::new(e, sigma * lag / tilde);
                    }
                }
            }
        }
    }
    solver.with_values(eps).unwrap()
}

/// The guide of [`guide`] along z (ε = 12, 0.4 × 0.3 µm, in 2.1) in a box of a lossy medium
/// (σ = 4/µm, 16 × 14 × 24 cells of 50 nm between its ends, walls across), at `steps` steps a
/// period of 1.55 µm; along the guide walls (`cpml` 0) or CPMLs of `cpml` cells beyond those 24.
/// A design of 3 × 3 × 3 cells across the core's edge between the mode source (6 cells into the
/// 24) and the monitor (16), ε from 2.1 to 12 at uneven densities, lossless with a cell about it;
/// FDFD on the same values at the leapfrog's frequency (with PMLs of FDFD's grading where the
/// CPMLs are), and the guide's modes from it. The frequency is the one whose leapfrog frequency
/// is 2π/(1.55 µm), the same for every step. The fields die away in it: nothing is trapped.
struct LossyBox {
    s: Simulation,
    solver: Solver3d,
    design: Design,
    source: PortMode3d,
    ahead: PortMode3d,
    pulse: Waveform,
}

impl LossyBox {
    fn new(cpml: usize, steps: usize) -> LossyBox {
        let g = grid([16, 14, 24 + 2 * cpml], 0.05);
        let design = Design {
            cells: [9..12, 6..9, cpml + 10..cpml + 13],
            eps: [2.1, 12.0],
        };
        // lossless from a cell before the design's values to a cell after
        let pocket = |x: f64, y: f64, z: f64| {
            let inside = |axis: Axis, v: f64| {
                let r = &design.cells[axis.index()];
                v >= g.node(axis, r.start - 1) && v <= g.node(axis, r.end + 1)
            };
            inside(Axis::X, x) && inside(Axis::Y, y) && inside(Axis::Z, z)
        };
        let ends = Edges::Pml {
            low: cpml,
            high: cpml,
        };
        let boundaries = Boundaries {
            z: ends,
            ..Boundaries::walls()
        };
        let s = Simulation::new(g, guide, boundaries, courant_for(steps))
            .unwrap()
            .with_conductivity(|x, y, z| if pocket(x, y, z) { 0.0 } else { 4.0 })
            .unwrap()
            .with_design(&design, &uneven(&design), guide)
            .unwrap();
        let frequency = s.leapfrog_frequency(std::f64::consts::TAU / 1.55).unwrap();
        let solver = fdfd_of(
            &s,
            frequency,
            Boundaries3d {
                z: ends,
                ..Boundaries3d::pml(0)
            },
        );
        let source = solver.port_modes(Axis::Z, cpml + 6, 1).unwrap().remove(0);
        let ahead = solver.port_modes(Axis::Z, cpml + 16, 1).unwrap().remove(0);
        LossyBox {
            s,
            solver,
            design,
            source,
            ahead,
            pulse: Waveform::pulse(frequency, 0.1).unwrap(),
        }
    }

    /// A run of the box with `mode` launched `direction`, its mode monitor ahead and its design
    /// monitor, run until its transforms settle: the run, the two monitors' numbers, and the
    /// source's amplitude at the monitor's frequency (half the pulse's analytic spectrum).
    fn run(&self, mode: &PortMode3d, direction: Direction) -> (Simulation, usize, usize, c64) {
        let mut s = self.s.clone();
        s.add_mode_source(mode, direction, self.pulse).unwrap();
        let monitor = s
            .add_mode_monitor(std::slice::from_ref(&self.ahead))
            .unwrap();
        let frequencies = s.mode_frequencies(monitor);
        let design = s.add_design_monitor(&self.design, &frequencies).unwrap();
        s.run_until_converged(BOX_SETTLED, 2.0, LIMIT).unwrap();
        let amplitude = 0.5
            * self
                .pulse
                .analytic_spectrum(Field::E, s.dt(), s.steps(), frequencies[0]);
        (s, monitor, design, amplitude)
    }

    /// The forward run, its adjoint for F = |a₊|² ahead, both run until their transforms
    /// settle, and the design monitor's number, the forward amplitude and the source's.
    fn forward_and_adjoint(&self) -> (Simulation, Simulation, usize, c64, c64) {
        let (s, monitor, design, amplitude) = self.run(&self.source, Direction::Forward);
        let a = s.mode_amplitudes(monitor)[0].0;
        let mut adjoint = s
            .adjoint(
                &[Term::Mode {
                    monitor,
                    mode: 0,
                    direction: Direction::Forward,
                    derivative: a.conj(),
                }],
                design,
                0.1,
            )
            .unwrap();
        adjoint
            .run_until_converged(BOX_SETTLED, 2.0, LIMIT)
            .unwrap();
        (s, adjoint, design, a, amplitude)
    }

    /// The gradient of the mode's forward power ahead with respect to the permittivity at each
    /// value of E the design reaches, by FDTD's adjoint (divided by the source's amplitude
    /// squared) and by FDFD's on the same values: the largest difference relative to the
    /// largest gradient, and the number of values compared.
    fn against_fdfd(&self) -> (f64, usize) {
        let g = self.s.grid();
        let (s, adjoint, design, _, amplitude) = self.forward_and_adjoint();
        let fdtd = s.permittivity_gradient(&adjoint, design).unwrap();
        let field = self
            .solver
            .solve_system(&self.solver.mode_source(&self.source, Direction::Forward))
            .unwrap();
        let (_, fdfd) = self
            .solver
            .mode_power_gradient(&field, &self.ahead, Direction::Forward)
            .unwrap();
        let ours: Vec<f64> = fdtd
            .iter()
            .map(|v| v.derivative / amplitude.norm_sqr())
            .collect();
        let theirs: Vec<f64> = fdtd
            .iter()
            .map(|v| fdfd[g.index(v.component, v.at)])
            .collect();
        (relative(&ours, &theirs), ours.len())
    }
}

/// [`LossyBox`] closed (walls all round, 64 steps a period): FDTD's gradient against FDFD's on
/// the same values, the largest difference relative to the largest gradient, and the number of
/// values compared.
pub(crate) fn against_fdfd_closed() -> (f64, usize) {
    LossyBox::new(0, 64).against_fdfd()
}

/// [`LossyBox`] open along the guide (CPMLs of 8 cells against FDFD's PMLs) at `steps` steps a
/// period: FDTD's gradient against FDFD's, the largest difference relative to the largest
/// gradient. The CPML is FDFD's PML only to first order in Δt.
pub(crate) fn against_fdfd_open(steps: usize) -> f64 {
    LossyBox::new(8, steps).against_fdfd().0
}

/// C. M. Lalau-Keraly et al.'s Eq. 8 (Opt. Express 21, 21693 (2013), doi:10.1364/OE.21.021693):
/// the adjoint field of a mode's transmission is that mode sent backwards from the monitor into
/// the device, its amplitude the forward overlap's conjugate (their Eq. 9). [`LossyBox`] closed,
/// F = |a₊|² ahead: the adjoint run's transforms over the design, Ê_adj, against a run that
/// launches the mode backwards from the monitor's plane through the same structure, Ê_back per
/// unit source amplitude. With FDFD's Lorentz form as the projection, Ê_adj = ā (ΔV/4) Ê_back
/// (docs/methods/fdtd-adjoint.md). Returns the largest |Ê_adj/(ā Ê_back) − ΔV/4| relative to
/// ΔV/4, over the design's values where Ê_back is above a thousandth of its largest, and the
/// ratio's mean over ΔV/4.
pub(crate) fn adjoint_is_the_mode_sent_backward() -> (f64, c64) {
    let b = LossyBox::new(0, 64);
    let (_, adjoint, design, a, _) = b.forward_and_adjoint();
    let (back, _, monitor, amplitude) = b.run(&b.ahead, Direction::Backward);
    let ours = &adjoint.monitors.designs[design];
    let theirs = &back.monitors.designs[monitor];
    let pairs: Vec<(c64, c64)> = ours
        .touched
        .iter()
        .map(|t| (ours.value(t, 0), theirs.value(t, 0) / amplitude))
        .collect();
    let largest = pairs.iter().fold(0.0f64, |m, p| m.max(p.1.norm()));
    let g = back.grid();
    let expected = g.dx * g.dy * g.dz / 4.0;
    let ratios: Vec<c64> = pairs
        .iter()
        .filter(|p| p.1.norm() > 1e-3 * largest)
        .map(|(adj, e)| adj / (a.conj() * e) / expected)
        .collect();
    let worst = ratios.iter().fold(0.0f64, |m, r| m.max((r - 1.0).norm()));
    (worst, ratios.iter().sum::<c64>() / ratios.len() as f64)
}

/// A slab of ε = 4, 0.3 µm thick, in vacuum, lit at normal incidence at 1 c/µm (λ = 1 µm): its
/// transmission T = 1/(1 + ((ε − 1)²/(4ε)) sin²(√ε k₀ d)) (Airy's formula) and dT/dε.
pub(crate) fn airy() -> (f64, f64) {
    let (eps, d, k0) = (4.0f64, 0.3, std::f64::consts::TAU);
    let a = (eps - 1.0).powi(2) / (4.0 * eps);
    let phase = eps.sqrt() * k0 * d;
    let s = phase.sin();
    let t = 1.0 / (1.0 + a * s * s);
    let da = (eps * eps - 1.0) / (4.0 * eps * eps);
    let ds2 = 2.0 * s * phase.cos() * k0 * d / (2.0 * eps.sqrt());
    (t, -(da * s * s + a * ds2) * t * t)
}

/// The slab of [`airy`] on a 1D grid (one cell across x and y, periodic there; along z cells
/// of `h`, CPMLs of 40 cells, Courant number 0.5), its faces on nodes, the design its cells at
/// density 1 (ε from 1 to 4); a sheet of E_x 0.25 µm before it radiates a pulse at 1 c/µm,
/// 0.5 c/µm wide, and F = |Ê_x|² 0.25 µm after it, at 1 c/µm. Returns F per unit incident
/// |Ê_x|² (the run without the slab) and the adjoint gradient summed over the slab's cells per
/// unit incident |Ê_x|² and unit ε: the discrete T and dT/dε.
pub(crate) fn slab(h: f64) -> (f64, f64) {
    let cells = |length: f64| (length / h).round() as usize;
    let pml = 40;
    let (before, thick, after) = (cells(0.5), cells(0.3), cells(0.5));
    let nz = pml + before + thick + after + pml;
    let g = crate::fdfd::Grid3d {
        nx: 1,
        ny: 1,
        nz,
        dx: h,
        dy: h,
        dz: h,
        x0: 0.0,
        y0: 0.0,
        z0: -((pml + before) as f64) * h,
    };
    let periodic = Edges::Bloch { k: 0.0 };
    let boundaries = Boundaries {
        x: periodic,
        y: periodic,
        ..Boundaries::cpml(pml)
    };
    let design = Design {
        cells: [0..1, 0..1, pml + before..pml + before + thick],
        eps: [1.0, 4.0],
    };
    let frequency = Frequency::natural(1.0).unwrap();
    let sheet = pml + before / 2;
    let point = pml + before + thick + after / 2;
    let run = |rho: f64| {
        let mut s = Simulation::new(g, |_, _, _| 1.0, boundaries, 0.5)
            .unwrap()
            .with_design(&design, &vec![rho; design.len()], |_, _, _| 1.0)
            .unwrap();
        s.add_current(super::Current {
            field: Field::E,
            values: vec![(Axis::X, (0, 0, sheet), c64::new(1.0, 0.0))],
            waveform: Waveform::pulse(frequency, 0.5).unwrap(),
        })
        .unwrap();
        let dft = s
            .add_dft((0, 0, point), (0, 0, point), &[frequency])
            .unwrap();
        let monitor = s.add_design_monitor(&design, &[frequency]).unwrap();
        s.run_until_converged(SETTLED, 1.0, LIMIT).unwrap();
        let e = s
            .dft(dft)
            .value(Field::E, Axis::X, (0, 0, point), 0)
            .unwrap();
        (s, e, dft, monitor)
    };
    let (_, incident, _, _) = run(0.0);
    let (s, e, dft, monitor) = run(1.0);
    let mut adjoint = s
        .adjoint(
            &[Term::Transform {
                monitor: dft,
                field: Field::E,
                component: Axis::X,
                at: (0, 0, point),
                frequency: 0,
                derivative: e.conj(),
            }],
            monitor,
            0.5,
        )
        .unwrap();
    adjoint.run_until_converged(SETTLED, 1.0, LIMIT).unwrap();
    let gradient: f64 = s.gradient(&adjoint, monitor).unwrap().iter().sum();
    let unit = incident.norm_sqr();
    (e.norm_sqr() / unit, gradient / (3.0 * unit))
}

/// The slab's dT/dε by [`slab`] on cells of 1/40, 1/80 and 1/160 µm: each one's error against
/// [`airy`]'s, relative to it.
#[cfg_attr(not(test), allow(dead_code))] // read by the tests
pub(crate) fn slab_errors() -> Vec<f64> {
    let (_, exact) = airy();
    [40.0, 80.0, 160.0]
        .par_iter()
        .map(|&n| ((slab(1.0 / n).1 - exact) / exact).abs())
        .collect()
}

/// What a gradient costs against a forward run.
#[cfg_attr(not(test), allow(dead_code))] // read by the tests
#[derive(Clone, Debug)]
pub(crate) struct Cost {
    /// The grid's cells and the design's.
    pub(crate) cells: usize,
    pub(crate) design: usize,
    /// The forward run alone: seconds, steps and bytes.
    pub(crate) forward: (f64, usize, usize),
    /// The forward run with the design's transforms: seconds and bytes.
    pub(crate) recorded: (f64, usize),
    /// The adjoint run: seconds, steps and bytes; and the gradient from both, seconds.
    pub(crate) adjoint: (f64, usize, usize),
    pub(crate) product: f64,
}

/// The strip of [`Strip`] scaled up: a core of ε = 6, 0.3 × 0.3 µm, along x in vacuum on
/// `cells` cells of 50 nm (CPMLs of 8), its mode launched by a pulse at 1 c/µm, 0.25 c/µm wide,
/// F = |a₊|² 1 µm before the far end, and a design of `design` cells in the middle at uneven
/// densities: the cost of the forward run alone, with the design's transforms, and of the
/// adjoint run, each run until its transforms settle to [`SETTLED`].
#[cfg_attr(not(test), allow(dead_code))] // read by the tests
pub(crate) fn cost(cells: [usize; 3], design: [usize; 3]) -> Cost {
    let g = grid(cells, 0.05);
    let boundaries = Boundaries::cpml(8);
    let frequency = Frequency::natural(1.0).unwrap();
    let probe = Simulation::new(g, strip, boundaries, 0.99).unwrap();
    let mut cut = g;
    cut.nx = 8;
    let mode = IterativeSolver3d::new(
        cut,
        probe.fdfd_wavelength(frequency).unwrap(),
        |x, y, z| c64::new(strip(x, y, z), 0.0),
        Boundaries3d {
            x: Edges::Pml { low: 0, high: 0 },
            ..Boundaries3d::pml(8)
        },
        Formulation::CurlCurl,
    )
    .unwrap()
    .port_modes(Axis::X, 3, 1)
    .unwrap()
    .remove(0);
    let middle = |axis: usize| {
        let (n, d) = (cells[axis], design[axis]);
        (n - d) / 2..(n - d) / 2 + d
    };
    let region = Design {
        cells: [middle(0), middle(1), middle(2)],
        eps: [1.0, 6.0],
    };
    let ahead = cells[0] - 8 - 20;
    let setup = || {
        let mut s = Simulation::new(g, strip, boundaries, 0.99)
            .unwrap()
            .with_design(&region, &uneven(&region), strip)
            .unwrap();
        s.add_mode_source(
            &mode.clone().moved_to(10),
            Direction::Forward,
            Waveform::pulse(frequency, 0.25).unwrap(),
        )
        .unwrap();
        let monitor = s.add_mode_monitor(&[mode.clone().moved_to(ahead)]).unwrap();
        (s, monitor)
    };
    let seconds = |start: std::time::Instant| start.elapsed().as_secs_f64();
    // the forward run alone
    let (mut s, _) = setup();
    let start = std::time::Instant::now();
    s.run_until_converged(SETTLED, 1.0, LIMIT).unwrap();
    let forward = (seconds(start), s.steps(), s.bytes());
    drop(s);
    // with the design's transforms, then the adjoint
    let (mut s, monitor) = setup();
    let frequencies = s.mode_frequencies(monitor);
    let design_monitor = s.add_design_monitor(&region, &frequencies).unwrap();
    let start = std::time::Instant::now();
    s.run_until_converged(SETTLED, 1.0, LIMIT).unwrap();
    let recorded = (seconds(start), s.bytes());
    let a = s.mode_amplitudes(monitor)[0].0;
    let start = std::time::Instant::now();
    let mut adjoint = s
        .adjoint(
            &[Term::Mode {
                monitor,
                mode: 0,
                direction: Direction::Forward,
                derivative: a.conj(),
            }],
            design_monitor,
            0.25,
        )
        .unwrap();
    adjoint.run_until_converged(SETTLED, 1.0, LIMIT).unwrap();
    let adjoint_cost = (seconds(start), adjoint.steps(), adjoint.bytes());
    let start = std::time::Instant::now();
    let gradient = s.gradient(&adjoint, design_monitor).unwrap();
    assert!(gradient.iter().all(|g| g.is_finite()));
    Cost {
        cells: g.cells(),
        design: region.len(),
        forward,
        recorded,
        adjoint: adjoint_cost,
        product: seconds(start),
    }
}
