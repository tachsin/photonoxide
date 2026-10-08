//! FDTD against FDFD over a band (issue #169): one pulsed run's S-parameters at several
//! frequencies against `Solver3d`'s at each, on the same grid at the leapfrog's frequency,
//! with the run's own source currents.

use num_complex::Complex64 as c64;

use super::checks::grid;
use super::*;
use crate::fdfd::{Boundaries3d, Direction, PortMode3d, Solver3d};
use crate::geometry::{Point, Shape};
use crate::units::Length;

/// A port's window: the cells along its plane's two axes.
type Window = (std::ops::Range<usize>, std::ops::Range<usize>);

/// A guide's two S-parameters at each frequency of a band, by FDTD and by FDFD.
#[derive(Clone, Debug)]
pub(crate) struct Band {
    /// The frequencies, c/µm (the tests print them).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) frequencies: Vec<f64>,
    /// FDTD's transmission S₂₁ and reflection S₁₁ at each.
    pub(crate) fdtd: Vec<[c64; 2]>,
    /// FDFD's.
    pub(crate) fdfd: Vec<[c64; 2]>,
}

impl Band {
    /// The largest |ΔS| over the band.
    pub(crate) fn largest_difference(&self) -> f64 {
        self.fdtd
            .iter()
            .zip(&self.fdfd)
            .flat_map(|(a, b)| [(a[0] - b[0]).norm(), (a[1] - b[1]).norm()])
            .fold(0.0, f64::max)
    }
}

/// A guided problem: its grid and medium, the input's mode source and reference plane, the
/// output's plane, a probe for the stopping rule and the band.
pub(crate) struct Guided<'a> {
    pub(crate) grid: Grid3d,
    /// The permittivity, FDFD's and (unless `structure`) FDTD's, averaged over each value's
    /// cell as both average it.
    pub(crate) eps: &'a dyn Fn(f64, f64, f64) -> f64,
    /// For FDTD, this structure smoothed by Werner, Bauer and Cary's triplets instead.
    pub(crate) structure: Option<&'a Structure>,
    pub(crate) boundaries: Boundaries,
    pub(crate) courant: f64,
    /// The input guide's axis, its source's plane and the reference plane after it.
    pub(crate) input: (Axis, usize, usize),
    /// The output guide's axis and plane.
    pub(crate) output: (Axis, usize),
    /// E's component and value the stopping rule watches.
    pub(crate) probe: (Axis, (usize, usize, usize)),
    /// The pulse's carrier and its bandwidth, c/µm.
    pub(crate) carrier: f64,
    pub(crate) bandwidth: f64,
    /// The band, c/µm.
    pub(crate) frequencies: Vec<f64>,
    /// The input's and the output's windows, if the ports don't span their planes.
    pub(crate) windows: [Option<Window>; 2],
}

impl Guided<'_> {
    fn fdfd_boundaries(&self) -> Boundaries3d {
        Boundaries3d {
            x: self.boundaries.x,
            y: self.boundaries.y,
            z: self.boundaries.z,
            reflection: self.boundaries.cpml.reflection,
            order: self.boundaries.cpml.order,
            real_stretch: 0.0,
        }
    }
}

/// The input's fundamental mode at the carrier launched forward by a pulse, and at each
/// frequency the fundamental modes' amplitudes on the reference plane (forward and backward)
/// and on the output plane (forward): S₂₁ = a⁺_out/a⁺_in and S₁₁ = a⁻_in/a⁺_in, by FDTD's mode
/// monitors and by FDFD at the leapfrog's frequency ω̃ for the run's own currents, each J and
/// M times its transform at the times it is applied. The mode source launches the carrier's
/// mode at every frequency, as the run does; the ratios make S independent of the pulse.
pub(crate) fn band(problem: &Guided) -> Band {
    let g = problem.grid;
    let eps = problem.eps;
    let mut s = match problem.structure {
        Some(structure) => Simulation::smoothed(
            g,
            structure,
            Smoothing::default(),
            problem.boundaries,
            problem.courant,
        ),
        None => Simulation::new(g, eps, problem.boundaries, problem.courant),
    }
    .unwrap();
    let complex = move |x: f64, y: f64, z: f64| c64::new(eps(x, y, z), 0.0);
    let carrier = Frequency::natural(problem.carrier).unwrap();
    let boundaries = problem.fdfd_boundaries();
    let (axis, source, reference) = problem.input;
    // the guide's fundamental mode, or in 2D the fundamental with E in the plane (E_z zero)
    let mode =
        |solver: &Solver3d, axis: Axis, plane: usize, window: &Option<Window>| -> PortMode3d {
            let modes = match window {
                Some(w) => solver.port_modes_within(axis, plane, w.clone(), 2),
                None => solver.port_modes(axis, plane, 2),
            }
            .unwrap();
            if g.nz > 1 {
                return modes.into_iter().next().unwrap();
            }
            let (b, c) = axis.others();
            let across = if b == Axis::Z { c } else { b };
            let largest = |m: &PortMode3d, component: Axis| {
                let mut w = 0.0f64;
                for u in 0..g.n(b) {
                    for v in 0..g.n(c) {
                        w = w.max(m.e(component, (u, v)).norm());
                    }
                }
                w
            };
            let in_plane = |m: &PortMode3d| largest(m, Axis::Z) < 1e-9 * largest(m, across);
            modes
                .into_iter()
                .find(in_plane)
                .expect("a mode with E in the plane")
        };
    // FDFD at each frequency's ω̃, the matrix's analysis shared; the carrier is one of them
    let mut solvers: Vec<Solver3d> = Vec::new();
    for &f in &problem.frequencies {
        let wavelength = s.fdfd_wavelength(Frequency::natural(f).unwrap()).unwrap();
        let solver = match solvers.first() {
            Some(first) => first.reuse(wavelength, complex),
            None => Solver3d::new(g, wavelength, complex, boundaries),
        };
        solvers.push(solver.unwrap());
    }
    let at_carrier = problem
        .frequencies
        .iter()
        .position(|&f| f == problem.carrier)
        .expect("the carrier among the frequencies");
    let launched = mode(&solvers[at_carrier], axis, source, &problem.windows[0]);
    let pulse = Waveform::pulse(carrier, problem.bandwidth).unwrap();
    let before = s.currents.len();
    s.add_mode_source(&launched, Direction::Forward, pulse)
        .unwrap();
    let applied: Vec<sources::Applied> = s.currents[before..].to_vec();
    let mut ports = Vec::new();
    let mut monitors = Vec::new();
    for solver in &solvers {
        let input = mode(solver, axis, reference, &problem.windows[0]);
        let output = mode(
            solver,
            problem.output.0,
            problem.output.1,
            &problem.windows[1],
        );
        monitors.push(
            s.add_mode_monitor(&[input.clone(), output.clone()])
                .unwrap(),
        );
        ports.push((input, output));
    }
    let (component, at) = problem.probe;
    let probe = s.add_probe(Field::E, component, at).unwrap();
    let decayed = s.run_until_decayed(&[probe], 1e-24, 10.0, 2000.0).unwrap();
    assert!(decayed, "the guide's field didn't decay");
    let (dt, steps, cells) = (s.dt(), s.steps(), g.cells());
    let mut fdtd = Vec::new();
    let mut fdfd = Vec::new();
    for (((&f, solver), (input, output)), &n) in problem
        .frequencies
        .iter()
        .zip(&solvers)
        .zip(&ports)
        .zip(&monitors)
    {
        let a = s.mode_amplitudes(n);
        fdtd.push([a[1].0 / a[0].0, a[0].1 / a[0].0]);
        let frequency = s.mode_frequencies(n)[0];
        debug_assert!((frequency.to_natural() - f).abs() < 1e-9 * f);
        let transform = |field: Field| pulse.analytic_spectrum(field, dt, steps, frequency);
        let (se, sh) = (transform(Field::E), transform(Field::H));
        let mut j = vec![c64::new(0.0, 0.0); g.unknowns()];
        let mut m = vec![c64::new(0.0, 0.0); g.unknowns()];
        for current in &applied {
            for &(c, r, value) in &current.values {
                match current.field {
                    Field::E => j[c * cells + r] += value * se,
                    Field::H => m[c * cells + r] += value * sh,
                }
            }
        }
        let electric = solver.solve(&j).unwrap();
        let magnetic = solver.solve_magnetic(&m).unwrap();
        let amplitudes = |mode| {
            let (a, b) = electric.mode_amplitudes(mode);
            let (c, d) = magnetic.mode_amplitudes(mode);
            (a + c, b + d)
        };
        let (into, back) = amplitudes(input);
        let (out, _) = amplitudes(output);
        fdfd.push([out / into, back / into]);
    }
    Band {
        frequencies: problem.frequencies.clone(),
        fdtd,
        fdfd,
    }
}

/// Five frequencies across ±5 % of the carrier 1/1.55 c/µm.
fn band_of_five() -> Vec<f64> {
    let carrier = 1.0 / 1.55;
    (0..5)
        .map(|k| carrier * (1.0 + 0.025 * (k as f64 - 2.0)))
        .collect()
}

/// A 2D guide of ε = 12 and 0.3 µm in air on cells of `h` (CPMLs of 0.5 µm; a 3.5 µm square,
/// one cell along z and periodic there; TE, E in the plane), straight along x, or bent by
/// 90° into +y around a quarter circle of 1 µm (`bend`), its fundamental mode launched at
/// 1.55 µm: S over the band, with ε averaged as FDFD averages it, or smoothed for FDTD
/// (`smoothed`). The guides' faces are halfway between nodes, where both averages agree.
pub(crate) fn guide_2d(h: f64, bend: bool, smoothed: bool) -> Band {
    use crate::fdfd::Edges;
    let (side, width, radius, core) = (3.5, 0.3, 1.0, 12.0);
    let n = (side / h).round() as usize;
    let cpml = (0.5 / h).round() as usize;
    let g = Grid3d {
        nz: 1,
        ..grid([n, n, 1], h)
    };
    // the input along y = y0 (a node row's midpoint), the bend's centre at (x_c, y0 + R)
    let y0 = g.y0 + h * (cpml as f64 + (0.75 / h).round() + 0.5);
    let xc = g.x0 + h * (n as f64 - cpml as f64 - (0.75 / h).round() - 0.5) - radius;
    let yc = y0 + radius;
    let eps = move |x: f64, y: f64, _: f64| {
        let inside = if bend {
            let r = (x - xc).hypot(y - yc);
            (x <= xc && (y - y0).abs() < width / 2.0)
                || (y >= yc && (x - xc - radius).abs() < width / 2.0)
                || (x > xc && y < yc && (r - radius).abs() < width / 2.0)
        } else {
            (y - y0).abs() < width / 2.0
        };
        if inside { core } else { 1.0 }
    };
    let um = Length::um;
    let structure = {
        let air = Permittivity::isotropic(1.0).unwrap();
        let si = Permittivity::isotropic(core).unwrap();
        let far = 10.0;
        let s = Structure::new(air);
        if bend {
            s.with(
                Body::extruded(
                    Shape::rect(Point::um(xc - far / 2.0, y0), um(far), um(width)).unwrap(),
                ),
                si,
            )
            .with(
                Body::extruded(
                    Shape::sector(
                        Point::um(xc, yc),
                        um(radius - width / 2.0),
                        um(radius + width / 2.0),
                        -std::f64::consts::FRAC_PI_2,
                        std::f64::consts::FRAC_PI_2,
                    )
                    .unwrap(),
                ),
                si,
            )
            .with(
                Body::extruded(
                    Shape::rect(Point::um(xc + radius, yc + far / 2.0), um(width), um(far))
                        .unwrap(),
                ),
                si,
            )
        } else {
            s.with(
                Body::extruded(Shape::rect(Point::um(0.0, y0), um(far), um(width)).unwrap()),
                si,
            )
        }
    };
    let pml = Edges::Pml {
        low: cpml,
        high: cpml,
    };
    let boundaries = Boundaries {
        x: pml,
        y: pml,
        z: Edges::Bloch { k: 0.0 },
        cpml: Cpml::default(),
    };
    let index = |x: f64, origin: f64| ((x - origin) / h).round() as usize;
    // a port's window: 0.6 µm either side of the guide's centre
    let around = |x: f64, origin: f64| {
        let reach = (0.6 / h).round() as usize;
        let centre = index(x, origin);
        centre - reach..centre + reach + 1
    };
    let source = cpml + (0.2 / h).round() as usize;
    let reference = source + (0.3 / h).round() as usize;
    let (output, probe) = if bend {
        let plane = n - cpml - 1 - (0.3 / h).round() as usize;
        (
            (Axis::Y, plane),
            (Axis::X, (index(xc + radius, g.x0), plane + 2, 0)),
        )
    } else {
        let plane = n - cpml - 1 - (0.3 / h).round() as usize;
        ((Axis::X, plane), (Axis::Y, (plane + 2, index(y0, g.y0), 0)))
    };
    band(&Guided {
        grid: g,
        eps: &eps,
        structure: smoothed.then_some(&structure),
        boundaries,
        courant: 0.9,
        input: (Axis::X, source, reference),
        output,
        probe,
        carrier: 1.0 / 1.55,
        bandwidth: 0.1,
        frequencies: band_of_five(),
        windows: [
            Some((around(y0, g.y0), 0..1)),
            Some(if bend {
                (0..1, around(xc + radius, g.x0))
            } else {
                (around(y0, g.y0), 0..1)
            }),
        ],
    })
}

/// The carrier 1/1.55 c/µm and 5 % either side.
fn band_of_three() -> Vec<f64> {
    let carrier = 1.0 / 1.55;
    vec![carrier * 0.95, carrier, carrier * 1.05]
}

/// A 3D strip along z (a core of ε = 12, 0.4 × 0.25 µm, in ε = 2.1) on cells of 50 nm (24 × 20
/// × 28 inside CPMLs of `cpml` cells), its fundamental mode launched at 1.55 µm: S over the
/// band, from 4 cells after the source to 3 before the far CPML.
pub(crate) fn strip_3d(cpml: usize) -> Band {
    let h = 0.05;
    let n = [24 + 2 * cpml, 20 + 2 * cpml, 28 + 2 * cpml];
    let g = grid(n, h);
    let eps = |x: f64, y: f64, _: f64| {
        if x.abs() < 0.2 && y.abs() < 0.125 {
            12.0
        } else {
            2.1
        }
    };
    band(&Guided {
        grid: g,
        eps: &eps,
        structure: None,
        boundaries: Boundaries::cpml(cpml),
        courant: 0.9,
        input: (Axis::Z, cpml + 2, cpml + 6),
        output: (Axis::Z, n[2] - cpml - 3),
        probe: (Axis::X, (n[0] / 2, n[1] / 2, n[2] - cpml - 1)),
        carrier: 1.0 / 1.55,
        bandwidth: 0.1,
        frequencies: band_of_three(),
        windows: [None, None],
    })
}

/// The strip of [`strip_3d`] along x bent by 90° into +y around a quarter circle of 0.8 µm
/// (its centre line), on cells of 50 nm (40 × 40 × 18 inside CPMLs of `cpml` cells): S over
/// the band, from the input guide into the output's.
pub(crate) fn bend_3d(cpml: usize) -> Band {
    let h = 0.05;
    let n = [34 + 2 * cpml, 34 + 2 * cpml, 16 + 2 * cpml];
    let g = grid(n, h);
    let (width, thick, radius) = (0.4, 0.25, 0.6);
    let y0 = g.y0 + h * (cpml as f64 + 10.5);
    let xc = g.x0 + h * (n[0] as f64 - cpml as f64 - 10.5) - radius;
    let yc = y0 + radius;
    let eps = move |x: f64, y: f64, z: f64| {
        let r = (x - xc).hypot(y - yc);
        let inside = z.abs() < thick / 2.0
            && ((x <= xc && (y - y0).abs() < width / 2.0)
                || (y >= yc && (x - xc - radius).abs() < width / 2.0)
                || (x > xc && y < yc && (r - radius).abs() < width / 2.0));
        if inside { 12.0 } else { 2.1 }
    };
    let index = |x: f64, origin: f64| ((x - origin) / h).round() as usize;
    let out = n[1] - cpml - 4;
    band(&Guided {
        grid: g,
        eps: &eps,
        structure: None,
        boundaries: Boundaries::cpml(cpml),
        courant: 0.9,
        input: (Axis::X, cpml + 2, cpml + 5),
        output: (Axis::Y, out),
        probe: (Axis::X, (index(xc + radius, g.x0), out + 2, n[2] / 2)),
        carrier: 1.0 / 1.55,
        bandwidth: 0.1,
        frequencies: band_of_three(),
        windows: [None, None],
    })
}

/// The largest |ΔS| over the band of the straight and the bent 2D guides of [`guide_2d`] on
/// cells of `h`, FDTD's ε averaged as FDFD's.
pub(crate) fn guides_2d(h: f64) -> f64 {
    [false, true]
        .into_iter()
        .map(|bend| guide_2d(h, bend, false).largest_difference())
        .fold(0.0, f64::max)
}

/// The bent 2D guide of [`guide_2d`] on cells of `h`, smoothed for FDTD by Werner, Bauer and
/// Cary's triplets and averaged as FDFD averages it there: the largest |ΔS| over the band.
pub(crate) fn smoothed_bend_2d(h: f64) -> f64 {
    guide_2d(h, true, true).largest_difference()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(name: &str, b: &Band) {
        println!("{name}: largest |ΔS| {:.3e}", b.largest_difference());
        for k in 0..b.frequencies.len() {
            println!(
                "  {:.5} c/µm: S21 {:.7} against {:.7}, |S11| {:.3e} against {:.3e}",
                b.frequencies[k],
                b.fdtd[k][0],
                b.fdfd[k][0],
                b.fdtd[k][1].norm(),
                b.fdfd[k][1].norm()
            );
        }
    }

    /// A straight 2D guide's S over the band, one pulse against FDFD at each frequency: the
    /// CPML's difference from FDFD's PML, 3.2e-5 on 50 nm cells.
    #[test]
    fn a_straight_guides_band_is_fdfds() {
        let b = guide_2d(0.05, false, false);
        show("straight", &b);
        assert!(b.largest_difference() < 1e-4, "{b:?}");
        // the guide is lossless and nearly matched
        assert!(
            b.fdtd
                .iter()
                .all(|s| (s[0].norm() - 1.0).abs() < 2e-3 && s[1].norm() < 1e-3)
        );
    }

    /// The 3D strip's agreement as the CPMLs thicken (8, 10 and 12 cells: 2.2e-4, 6.5e-5 and
    /// 7.9e-6), its S₁₁ the CPML's own reflection; about three minutes, so on demand:
    /// `cargo test --release --lib fdtd::agreement_checks -- --ignored`.
    #[test]
    #[ignore]
    fn the_strips_agreement_converges_with_the_cpml() {
        let differences: Vec<f64> = [8, 10, 12]
            .into_iter()
            .map(|cpml| {
                let b = strip_3d(cpml);
                show(&format!("strip, CPMLs of {cpml}"), &b);
                b.largest_difference()
            })
            .collect();
        assert!(
            differences.windows(2).all(|w| w[1] < 0.5 * w[0]) && differences[2] < 2e-5,
            "{differences:?}"
        );
    }

    /// The smoothed bend against FDFD on 50, 25 and 12.5 nm cells: 0.56, 0.31 and 0.16, first
    /// order; about a minute, so on demand.
    #[test]
    #[ignore]
    fn the_smoothed_bend_converges_to_fdfds() {
        let differences: Vec<f64> = [0.05, 0.025, 0.0125]
            .into_iter()
            .map(|h| {
                let b = guide_2d(h, true, true);
                show(&format!("smoothed bend, {h} µm cells"), &b);
                b.largest_difference()
            })
            .collect();
        assert!(
            differences.windows(2).all(|w| w[1] < 0.6 * w[0]),
            "{differences:?}"
        );
    }
}
