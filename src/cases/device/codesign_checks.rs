//! The validation cases of device and circuit co-design (0.7, #761's I11): the circuit
//! adjoint's sensitivity to one instance's S-matrix, 2D FDFD's gradient of a function of its
//! S-matrix, and the two composed: a ring whose coupler is a 2D FDFD problem, the through
//! port's power differentiated in the coupler's cells, each against differences.

use std::sync::Arc;

use num_complex::Complex64 as c64;

use crate::Result;
use crate::circuit::components::PhaseShifter;
use crate::circuit::{Circuit, Component, Fixed, Netlist, SMatrix, objective};
use crate::fdfd::{Boundaries, Grid, Polarization, Port, Side, Solver2d};
use crate::portable;
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier, note_numerical};

const OK: &str = "a valid co-design problem";

/// A value along a direction, by the step taken.
type Along<'a> = Box<dyn Fn(f64) -> f64 + 'a>;

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "circuit/instance-sensitivity",
            title: r"An all-pass ring (a coupler given as a fixed 4 × 4 S-matrix with loss and reflection, a loop of amplitude 0.95 and a phase shifter): the through port's power's sensitivity to the coupler's own S-matrix from the circuit adjoint, $H_{ab} = \partial F/\partial S_{ab}$, dotted with two complex directions of that matrix, against fourth-order differences of the circuit solve ($h = 10^{-4}$) (largest difference relative to the largest derivative shown)",
            tier: Tier::Analytic,
            source: r"the circuit adjoint (docs/methods/circuit-adjoint.md): with $\lambda$ the transposed system's solution and $A$ the waves going into the instance's ports, $dF = 2\mathop{\mathrm{Re}}\sum_{ab}\lambda_a\thinspace dS_{ab}\thinspace A_b$, so $H_{ab} = \lambda_a A_b$ summed over the inputs $F$ depends on",
            run: instance_sensitivity_case,
        },
        Case {
            id: "fdfd/s-matrix-gradient",
            title: r"2D FDFD, $E$ along $z$: two 0.3 µm guides (2.85 in 1.444) brought to 0.6 µm apart by S-bends, a region of varied permittivity in the gap, four ports; the gradient of $F = \lvert S_{30}\rvert^2 + \frac12\lvert S_{21}\rvert^2$ with respect to the region's cells from $F$'s sensitivity to $S$ by one adjoint solve per port, against fourth-order differences ($\delta = 10^{-3}$) at a corner, an edge and the middle (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"$SA = B$ with $A$ and $B$ the incoming and outgoing amplitudes, linear functionals of the fields: $dS = (dB - S\thinspace dA)A^{-1}$, so $\sum h\thinspace dS = \sum_p w_p^{\mathsf T} du_p$ and each $du_p$ by the adjoint, $-\lambda_p^{\mathsf T}(\partial A/\partial\varepsilon)u_p$ with $A^{\mathsf T}\lambda_p = w_p$ (G. Veronis, R. W. Dutton, S. Fan, Opt. Lett. 29, 2288 (2004), doi:10.1364/OL.29.002288)",
            run: s_matrix_gradient_case,
        },
        Case {
            id: "device/codesign-gradient",
            title: r"A ring whose coupler is that 2D FDFD problem: the through port's power at 1.55 µm differentiated in the coupler's cells (the circuit adjoint's sensitivity to the coupler's $S$ carried by FDFD's adjoint) and in the loop's phase (the circuit adjoint), against fourth-order differences of the whole chain ($\delta = 10^{-3}$) at three cells and the phase (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"the chain rule across two solvers, $dF/d\varepsilon_k = 2\mathop{\mathrm{Re}}\sum_{ab} H_{ab}\thinspace\partial S_{ab}/\partial\varepsilon_k$; S. Mason et al., arXiv:2509.07233 (2025), co-optimized inverse design, as the plan's target",
            run: codesign_gradient_case,
        },
    ]
}

fn zero(error: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: error,
        expected: 0.0,
        tolerance,
        error,
    }
}

/// The fourth-order difference of `f` at 0 with step `h`.
fn differences(f: &dyn Fn(f64) -> f64, h: f64) -> f64 {
    (8.0 * (f(h) - f(-h)) - (f(2.0 * h) - f(-2.0 * h))) / (12.0 * h)
}

/// The largest |exact − D(h)| and |D(h) − D(2h)|, relative to the largest exact derivative.
fn agreement(checks: &[(f64, &dyn Fn(f64) -> f64)], h: f64) -> (f64, f64) {
    let top = checks.iter().map(|c| c.0.abs()).fold(0.0, f64::max);
    let (mut error, mut resolution) = (0.0f64, 0.0f64);
    for (exact, f) in checks {
        let (one, two) = (differences(*f, h), differences(*f, 2.0 * h));
        error = error.max((one - exact).abs() / top);
        resolution = resolution.max((one - two).abs() / top);
    }
    (error, resolution)
}

/// An all-pass ring around a coupler of S-matrix `coupler` (ports lb, lt, rt, rb): the loop
/// from rt through a loss of amplitude `a` and a phase shifter back to lt, the chip's in at lb
/// and out at rb. The circuit, its in and out ports, and the phase's slot.
pub(crate) fn ring(coupler: &[Vec<c64>], a: f64) -> Result<(Circuit, usize, usize, usize)> {
    let c = SMatrix::from_fn(4, |q, p| coupler[q][p]);
    let loss = SMatrix::from_fn(2, |q, p| {
        if q == p {
            c64::new(0.0, 0.0)
        } else {
            c64::new(a, 0.0)
        }
    });
    let mut net = Netlist::new();
    net.add(
        "c",
        Arc::new(Fixed::new("coupler", &["lb", "lt", "rt", "rb"], c)?),
    )?;
    net.add("loss", Arc::new(Fixed::new("loss", &["o1", "o2"], loss)?))?;
    net.add("phi", Arc::new(PhaseShifter::new()))?;
    net.connect("c.rt", "loss.o1")?;
    net.connect("loss.o2", "phi.o1")?;
    net.connect("phi.o2", "c.lt")?;
    net.expose("in", "c.lb")?;
    net.expose("out", "c.rb")?;
    let circuit = net.compile()?;
    let port = |name: &str| {
        circuit
            .ports()
            .iter()
            .position(|p| p.name == name)
            .expect("a port")
    };
    let slot = circuit.parameter("phi.phase").expect("the phase");
    let (i, o) = (port("in"), port("out"));
    Ok((circuit, i, o, slot))
}

/// The through power of `ring`'s circuit at `phase`.
fn through(coupler: &[Vec<c64>], a: f64, phase: f64) -> f64 {
    let (circuit, i, o, slot) = ring(coupler, a).expect(OK);
    let mut values = circuit.values().to_vec();
    values[slot] = phase;
    let s = circuit
        .s_matrix_with(Wavelength::um(1.55).expect(OK), &values)
        .expect(OK);
    s[(o, i)].norm_sqr()
}

/// A coupler with loss and reflection, as a fixed matrix.
fn lossy_coupler() -> Vec<Vec<c64>> {
    let (t, k) = (0.9, 0.38);
    let r = c64::new(0.02, 0.01);
    // lb → rb and lt → rt through, lb → rt and lt → rb across
    let mut s = vec![vec![r; 4]; 4];
    let entries = [
        (3, 0, c64::new(t, 0.05)),
        (2, 1, c64::new(t, -0.03)),
        (2, 0, c64::new(0.01, k)),
        (3, 1, c64::new(-0.02, k)),
    ];
    for (q, p, v) in entries {
        s[q][p] = v;
        s[p][q] = v;
    }
    s
}

/// The instance sensitivity against differences: (error, resolution).
pub(crate) fn instance_agreement() -> (f64, f64) {
    let coupler = lossy_coupler();
    let (a, phase) = (0.95, 0.4);
    let (circuit, i, o, slot) = ring(&coupler, a).expect(OK);
    let mut values = circuit.values().to_vec();
    values[slot] = phase;
    let lambda = Wavelength::um(1.55).expect(OK);
    let (_, h) = circuit
        .instance_sensitivity(&[lambda], &values, "c", |s| objective::power(s, o, i))
        .expect(OK);
    let directions: Vec<Vec<Vec<c64>>> = [0.3, 1.7]
        .iter()
        .map(|&seed| {
            (0..4)
                .map(|q| {
                    (0..4)
                        .map(|p| {
                            let t = seed + 1.3 * q as f64 + 0.7 * p as f64;
                            c64::new(portable::sin(2.1 * t), portable::cos(1.3 * t))
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    let mut sets: Vec<(f64, Along<'static>)> = Vec::new();
    for d in directions {
        let mut exact = 0.0;
        for q in 0..4 {
            for p in 0..4 {
                exact += 2.0 * (h[0][(q, p)] * d[q][p]).re;
            }
        }
        let coupler = coupler.clone();
        sets.push((
            exact,
            Box::new(move |step| {
                let moved: Vec<Vec<c64>> = (0..4)
                    .map(|q| (0..4).map(|p| coupler[q][p] + d[q][p] * step).collect())
                    .collect();
                through(&moved, a, phase)
            }),
        ));
    }
    let checks: Vec<(f64, &dyn Fn(f64) -> f64)> =
        sets.iter().map(|(e, f)| (*e, f.as_ref())).collect();
    agreement(&checks, 1e-4)
}

fn instance_sensitivity_case() -> Outcome {
    let (error, resolution) = instance_agreement();
    note_numerical(resolution);
    zero(error, 2e-11)
}

/// The co-designed coupler: two 0.3 µm guides of 2.85 (a 220 nm silicon slab's effective index)
/// in 1.444, E along z, at cell `h`: 1.5 µm apart at the ports, brought to 0.3 µm over 1 µm by
/// cosine S-bends 3 µm long, a design region filling the gap there; its cells, region and four
/// ports (lb, lt, rt, rb).
pub(crate) struct Coupler {
    pub(crate) base: Vec<c64>,
    pub(crate) region: Vec<usize>,
    pub(crate) ports: Vec<Port>,
    solver: Solver2d,
}

impl Coupler {
    pub(crate) fn new(h: f64) -> Result<Coupler> {
        let pml = 15;
        let (length, half) = (9.0, 1.6);
        let grid = Grid {
            nx: (length / h).round() as usize + 2 * pml,
            ny: (2.0 * half / h).round() as usize + 2 * pml,
            dx: h,
            dy: h,
            x0: -(pml as f64) * h,
            y0: -half - pml as f64 * h,
        };
        let (core, clad) = (2.85f64 * 2.85, 1.444f64 * 1.444);
        let mut base = vec![c64::new(clad, 0.0); grid.nx * grid.ny];
        let mut region = Vec::new();
        for j in 0..grid.ny {
            for i in 0..grid.nx {
                let (x, y) = (grid.x(i), grid.y(j));
                let k = j * grid.nx + i;
                let centre = guide_centre(x);
                if (y.abs() - centre).abs() < 0.15 {
                    base[k] = c64::new(core, 0.0);
                } else if y.abs() < centre - 0.15 && (4.0..5.0).contains(&x) {
                    region.push(k);
                    // a start that breaks the guides' symmetry and couples them a little
                    base[k] = c64::new(
                        clad + 0.3 * (core - clad) * (0.5 + 0.5 * portable::sin(4.0 * x + 3.0 * y)),
                        0.0,
                    );
                }
            }
        }
        let wavelength = Wavelength::um(1.55)?;
        let solver = Solver2d::from_cells(
            grid,
            Polarization::Ez,
            wavelength,
            &base,
            Boundaries::pml(pml),
        )?;
        let column = |x: f64| ((x - grid.x0) / h) as usize;
        let mid = grid.ny / 2;
        let mut ports = Vec::new();
        for (x, top, side) in [
            (0.4, false, Side::Left),
            (0.4, true, Side::Left),
            (length - 0.4, true, Side::Right),
            (length - 0.4, false, Side::Right),
        ] {
            let rows = if top { mid..grid.ny - pml } else { pml..mid };
            let mode = solver.port_modes_within(column(x), rows, 1)?.remove(0);
            ports.push(Port { mode, side });
        }
        Ok(Coupler {
            base,
            region,
            ports,
            solver,
        })
    }

    /// The solver with the region's cells changed by `delta` (their permittivities).
    pub(crate) fn solver(&self, delta: &[(usize, f64)]) -> Result<Solver2d> {
        let mut eps = self.base.clone();
        for &(k, d) in delta {
            eps[k] += d;
        }
        self.solver.reuse_cells(Wavelength::um(1.55)?, &eps)
    }
}

/// The region's three cells the cases probe: its first, its middle and its last.
fn probes(c: &Coupler) -> [usize; 3] {
    let r = &c.region;
    [r[0], r[r.len() / 2 + 7], r[r.len() - 1]]
}

/// FDFD's S-matrix gradient against differences: (error, resolution).
pub(crate) fn s_matrix_agreement() -> (f64, f64) {
    let coupler = Coupler::new(0.04).expect(OK);
    let c = &coupler;
    let f = |s: &[Vec<c64>]| s[3][0].norm_sqr() + 0.5 * s[2][1].norm_sqr();
    let base = c.solver(&[]).expect(OK);
    let s0 = base.s_matrix(&c.ports).expect(OK);
    let mut h = vec![vec![c64::new(0.0, 0.0); 4]; 4];
    h[3][0] = s0[3][0].conj();
    h[2][1] = 0.5 * s0[2][1].conj();
    let (_, gradient) = base.s_matrix_gradient(&c.ports, &h).expect(OK);
    let sets: Vec<(f64, Along<'_>)> = probes(c)
        .into_iter()
        .map(|k| {
            let g: Along<'_> =
                Box::new(move |d| f(&c.solver(&[(k, d)]).expect(OK).s_matrix(&c.ports).expect(OK)));
            (gradient[k], g)
        })
        .collect();
    let checks: Vec<(f64, &dyn Fn(f64) -> f64)> =
        sets.iter().map(|(e, f)| (*e, f.as_ref())).collect();
    agreement(&checks, 1e-3)
}

fn s_matrix_gradient_case() -> Outcome {
    let (error, resolution) = s_matrix_agreement();
    note_numerical(resolution);
    zero(error, 5e-8)
}

/// The ring's through power with the coupler's region changed by `delta` and the loop at
/// `phase`, and its gradient in the region's cells (by cell index) and the phase.
pub(crate) fn codesign(
    c: &Coupler,
    delta: &[(usize, f64)],
    phase: f64,
    a: f64,
) -> Result<(f64, Vec<f64>, f64)> {
    let solver = c.solver(delta)?;
    let s = solver.s_matrix(&c.ports)?;
    let (circuit, i, o, slot) = ring(&s, a)?;
    let mut values = circuit.values().to_vec();
    values[slot] = phase;
    let lambda = Wavelength::um(1.55)?;
    let (f, h) =
        circuit.instance_sensitivity(&[lambda], &values, "c", |m| objective::power(m, o, i))?;
    let h: Vec<Vec<c64>> = (0..4)
        .map(|q| (0..4).map(|p| h[0][(q, p)]).collect())
        .collect();
    let (_, by_cell) = solver.s_matrix_gradient(&c.ports, &h)?;
    let (_, by_parameter) = circuit.gradient(&[lambda], &values, |m| objective::power(m, o, i))?;
    Ok((f, by_cell, by_parameter[slot]))
}

/// The composed gradient against differences: (error, resolution).
pub(crate) fn codesign_agreement() -> (f64, f64) {
    let coupler = Coupler::new(0.04).expect(OK);
    let c = &coupler;
    let (a, phase) = (0.95, 0.7);
    let (_, by_cell, by_phase) = codesign(c, &[], phase, a).expect(OK);
    let mut sets: Vec<(f64, Along<'_>)> = probes(c)
        .into_iter()
        .map(|k| {
            let g: Along<'_> = Box::new(move |d| codesign(c, &[(k, d)], phase, a).expect(OK).0);
            (by_cell[k], g)
        })
        .collect();
    sets.push((
        by_phase,
        Box::new(|d| codesign(c, &[], phase + d, a).expect(OK).0),
    ));
    let checks: Vec<(f64, &dyn Fn(f64) -> f64)> =
        sets.iter().map(|(e, f)| (*e, f.as_ref())).collect();
    agreement(&checks, 1e-3)
}

fn codesign_gradient_case() -> Outcome {
    let (error, resolution) = codesign_agreement();
    note_numerical(resolution);
    zero(error, 1e-7)
}

/// The guides' centres' distance from the axis at x (µm): 0.75 at the ports, 0.3 along the
/// coupling length from 4 to 5 µm, and cosine S-bends 3 µm long between.
fn guide_centre(x: f64) -> f64 {
    let bend = |t: f64| 0.5 * (1.0 + portable::cos(std::f64::consts::PI * t.clamp(0.0, 1.0)));
    let away = if x < 4.0 {
        bend((x - 1.0) / 3.0)
    } else if x > 5.0 {
        1.0 - bend((x - 5.0) / 3.0)
    } else {
        0.0
    };
    0.3 + 0.45 * away
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The numbers the rules quote.
    #[test]
    #[ignore = "a study, printed: cargo test --release --lib the_codesign_study -- --ignored --nocapture"]
    fn the_codesign_study() {
        println!("instance (error, resolution) {:?}", instance_agreement());
        println!("s-matrix (error, resolution) {:?}", s_matrix_agreement());
        println!("codesign (error, resolution) {:?}", codesign_agreement());
    }
}
