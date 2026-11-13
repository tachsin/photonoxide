//! Khan et al.'s second device: a converter between a subwavelength-grating (SWG) waveguide and a
//! strip, its gratings' widths and lengths and its bridge's edge optimized on adjoint gradients,
//! once on the drawn design and once on what a lithography proxy prints from it.
//!
//! S. Raza (Khan), M. Hammood, N. A. F. Jaeger, L. Chrostowski, "Fabrication-aware inverse design
//! with shape optimization for photonic integrated circuits", Opt. Lett. 50, 117 (2024),
//! [doi:10.1364/OL.543961](https://doi.org/10.1364/OL.543961), read in the manuscript
//! (arXiv:2410.07353): Fig. 2's parametrization (each grating's width and length, the bridge's
//! boundary as a spline from a fixed point at the strip's width to the SWG's end) and their test
//! of a simulation, a design made with the lithography model printing better than the standard
//! one (their steps 1 and 2; Table 1, Fig. 6).
//!
//! **What the paper doesn't give, and what is here instead.** Their converter's dimensions, its
//! wavelength band's design (Table 1 is at 1320 nm), their DUV model (UBC's, in Mentor Graphics
//! Calibre) and their EBL model (PreFab's network) are not published. The SWG here is R. Halir et
//! al.'s, "Ultra-broadband nanophotonic beamsplitter using an anisotropic sub-wavelength
//! metamaterial", Laser Photonics Rev. 10, 1039 (2016),
//! [doi:10.1002/lpor.201600213](https://doi.org/10.1002/lpor.201600213), Table 1: 220 nm silicon,
//! a 190 nm period at a 50 % duty cycle, 0.5 µm wide, and a taper of 30 periods; at 1.55 µm, their
//! design wavelength. The converter is drawn as Khan et al.'s Fig. 2: 30 gratings over a bridge
//! that narrows from the strip's 0.5 µm to nothing. Two of them back to back, with 10 periods of
//! SWG between, from a strip to a strip, so that both ports are a strip's mode; each converter's
//! loss is half the pair's. The lithography proxy is `topology::Lithography`, a Gaussian blur and a
//! threshold at ½, σ the widest that prints a 150 nm line within 1 % of its width (58.16 nm):
//! S. K. Selvaraja et al., J. Lightwave Technol. 27, 4076 (2009),
//! [doi:10.1109/JLT.2009.2022282](https://doi.org/10.1109/JLT.2009.2022282), Section II-B and
//! Fig. 6, print lines from 150 nm up on target by 193 nm lithography and ask 1 % of a line's
//! width (as `optimize_ybranch`).
//!
//! **The model.** The device's plane by the effective index method, as `optimize_ybranch`'s: the
//! 220 nm slab's TE index inside the silicon, the oxide's outside, $H$ along $z$, 2D FDFD on 20 nm
//! cells, each cell's share of silicon its exact area (the gratings are rectangles and the bridge
//! piecewise linear, all centred on the axis). The variables: the 30 gratings' widths (0.3 to
//! 0.8 µm) and lengths (40 to 150 nm) and the bridge's half-width at 6 points (0 to 0.25 µm), the
//! second converter the first's mirror image. Each evaluation is a solve and its adjoint, the
//! derivatives through the cells' exact areas (central differences of the areas, no solve) and,
//! for the printed model, through the proxy (Khan et al.'s Eq. 2); checked against fourth-order
//! differences of the power along each variable's change of the cells' shares, within what they
//! resolve (central differences in the variable itself are off by their truncation, 1e-3 at a
//! grating's length for 1e-4 µm, and cross the areas' kinks where an edge crosses a row). Genoxide's L-BFGS-B from the
//! plain converter (every grating 0.5 µm by 95 nm, the bridge straight), 40 evaluations, once on
//! the drawn model (the standard design) and once on the printed model (designed printed). Every
//! design is then solved on 10 nm cells too, printed by the proxy on 10 nm pixels: our grid's
//! change, printed beside.
//!
//! **What it found** (each converter's loss; 20 nm cells, 10 nm beside). The plain converter
//! loses 0.0005 dB drawn (0.0004) and 0.0579 dB printed (0.0640): a taper of 30 periods is
//! nearly lossless as drawn, and the proxy's rounding of its narrow bridge and gratings costs
//! 0.06 dB. The standard design reaches 0.0001 dB drawn (0.0015) and 0.0403 dB printed (0.0437);
//! the one designed printed 0.0017 dB printed (0.0029), drawn wider and longer where the bridge
//! thins (0.0480 dB drawn: it is drawn to print right). Khan et al.'s pattern for DUV (Table 1:
//! 0.034 dB as designed, 0.836 printed, 0.111 designed printed) and their test, the design made
//! with the model printing better than the standard one, which is checked. Their numbers are
//! their own converter at 1320 nm under their models, whose standard design loses 0.836 dB
//! printed where ours loses 0.0403 dB: printed beside ours as context, not compared.
//!
//! ```sh
//! cargo run --release --example optimize_swg_converter
//! ```

mod common;

use photonoxide::portable;
use std::f64::consts::LN_10;
use std::process::ExitCode;

use genoxide::math;
use genoxide::prelude::*;
use photonoxide::Complex64 as c64;
use photonoxide::backend::Choice;
use photonoxide::fdfd::{Boundaries, Direction, Grid, Polarization, PortMode, Solver2d};
use photonoxide::material;
use photonoxide::mode::slab::Slab;
use photonoxide::topology::{Edge, Lithography};
use photonoxide::units::{Length, Wavelength};
use rayon::prelude::*;

/// The wavelength, µm.
const WAVELENGTH: f64 = 1.55;
/// Halir et al.'s SWG (Table 1): the period and duty cycle, the width (the strip's too), µm,
/// and the taper's periods.
const PERIOD: f64 = 0.19;
const DUTY: f64 = 0.5;
const WIDTH: f64 = 0.5;
const PERIODS: usize = 30;
/// The SWG's periods between the two converters.
const MIDDLE: usize = 10;
/// The bridge's points: its half-width at `KNOTS` − 1 interior points of the converter, its ends
/// fixed (the strip's half-width and nothing).
const KNOTS: usize = 7;
/// The variables' bounds: a grating's width and length, the bridge's half-width, µm.
const W_BOUNDS: (f64, f64) = (0.3, 0.8);
const L_BOUNDS: (f64, f64) = (0.04, 0.15);
const B_BOUNDS: (f64, f64) = (0.0, 0.25);
/// The searches' evaluations, and the shapes' step for the cells' ∂m/∂x, µm.
const EVALUATIONS: u64 = 40;
const SHAPE_STEP: f64 = 1e-4;
/// The grid's cell (and the grid study's), µm, the PMLs' cells, the strip before and after and
/// the cladding beside, µm.
const STEP: f64 = 0.02;
const FINE: f64 = 0.01;
const PML: usize = 12;
const LEAD: f64 = 1.0;
const MARGIN: f64 = 1.0;
/// The lithography proxy's basis (Selvaraja et al. 2009): lines from 150 nm up print on target,
/// within 1 % of their width; the threshold.
const LINEAR_FROM: f64 = 0.15;
const ACCURACY: f64 = 0.01;
const THRESHOLD: f64 = 0.5;
/// Khan et al.'s Table 1, their SWG-to-strip converter at 1320 nm, dB: as designed, printed by
/// each model, and designed with it.
const KHAN_SWG: [(&str, f64, f64, f64); 2] =
    [("EBL", 0.034, 0.072, 0.036), ("DUV", 0.034, 0.836, 0.111)];

/// The converter's length, µm, and the pair's.
fn converter() -> f64 {
    PERIODS as f64 * PERIOD
}

fn total() -> f64 {
    (2 * PERIODS + MIDDLE) as f64 * PERIOD
}

/// The plain converter: every grating at the SWG's width and duty cycle, the bridge straight.
fn plain() -> Vec<f64> {
    let mut x = vec![WIDTH; PERIODS];
    x.extend(vec![DUTY * PERIOD; PERIODS]);
    x.extend((1..KNOTS).map(|j| 0.5 * WIDTH * (1.0 - j as f64 / KNOTS as f64)));
    x
}

/// The drawing of the variables `x`: every grating (its centre, length and half-width), and the
/// bridge's half-width at any x.
struct Drawing {
    gratings: Vec<(f64, f64, f64)>,
    knots: Vec<f64>,
}

impl Drawing {
    fn new(x: &[f64]) -> Drawing {
        let (c, t) = (converter(), total());
        let mut gratings = Vec::new();
        for k in 0..PERIODS {
            let centre = (k as f64 + 0.5) * PERIOD;
            gratings.push((centre, x[PERIODS + k], 0.5 * x[k]));
            gratings.push((t - centre, x[PERIODS + k], 0.5 * x[k]));
        }
        for m in 0..MIDDLE {
            let centre = c + (m as f64 + 0.5) * PERIOD;
            gratings.push((centre, DUTY * PERIOD, 0.5 * WIDTH));
        }
        let mut knots = vec![0.5 * WIDTH];
        knots.extend_from_slice(&x[2 * PERIODS..]);
        knots.push(0.0);
        Drawing { gratings, knots }
    }

    /// The bridge's half-width at `x`: the strip before and after, linear between the knots in
    /// each converter, nothing between them.
    fn bridge(&self, x: f64) -> f64 {
        let (c, t) = (converter(), total());
        let along = if x <= 0.0 || x >= t {
            return 0.5 * WIDTH;
        } else if x < c {
            x
        } else if x > t - c {
            t - x
        } else {
            return 0.0;
        };
        let h = c / KNOTS as f64;
        let j = ((along / h).floor() as usize).min(KNOTS - 1);
        let f = along / h - j as f64;
        self.knots[j] * (1.0 - f) + self.knots[j + 1] * f
    }

    /// The points along x where the outline's pieces change: the gratings' ends and the
    /// bridge's knots.
    fn breaks(&self) -> Vec<f64> {
        let (c, t) = (converter(), total());
        let h = c / KNOTS as f64;
        let mut b: Vec<f64> = (0..=KNOTS)
            .flat_map(|j| [j as f64 * h, t - j as f64 * h])
            .collect();
        for &(centre, length, _) in &self.gratings {
            b.push(centre - 0.5 * length);
            b.push(centre + 0.5 * length);
        }
        b
    }

    /// The largest grating half-width over `x`, 0 if none.
    fn grating(&self, x: f64) -> f64 {
        self.gratings
            .iter()
            .filter(|(centre, length, _)| (x - centre).abs() < 0.5 * length)
            .map(|g| g.2)
            .fold(0.0, f64::max)
    }
}

/// The length of [−h, h] within [a, b].
fn overlap(h: f64, a: f64, b: f64) -> f64 {
    (h.min(b) - (-h).max(a)).max(0.0)
}

/// Each cell's share of silicon for the variables `x`, its exact area: along each piece where
/// the outline's half-width h is linear, the overlap of [−h, h] with each row integrated exactly
/// (it is linear in h between |y| of the row's edges).
fn mask(g: &Grid, x: &[f64]) -> Vec<f64> {
    let d = Drawing::new(x);
    let breaks = d.breaks();
    let columns: Vec<Vec<f64>> = (0..g.nx)
        .into_par_iter()
        .map(|i| {
            let (xa, xb) = (g.x(i) - 0.5 * g.dx, g.x(i) + 0.5 * g.dx);
            let mut points: Vec<f64> = breaks
                .iter()
                .copied()
                .filter(|&p| p > xa && p < xb)
                .collect();
            points.push(xa);
            points.push(xb);
            points.sort_by(f64::total_cmp);
            // the pieces where h is linear: [p, q] with its h at each end
            let mut pieces: Vec<(f64, f64, f64, f64)> = Vec::new();
            for w in points.windows(2) {
                let (p, q) = (w[0], w[1]);
                if q <= p {
                    continue;
                }
                let c = d.grating(0.5 * (p + q));
                let (bp, bq) = (d.bridge(p), d.bridge(q));
                if (bp - c) * (bq - c) < 0.0 {
                    let s = p + (c - bp) / (bq - bp) * (q - p);
                    pieces.push((p, s, bp.max(c), c));
                    pieces.push((s, q, c, bq.max(c)));
                } else {
                    pieces.push((p, q, bp.max(c), bq.max(c)));
                }
            }
            (0..g.ny)
                .map(|j| {
                    let (ya, yb) = (g.y(j) - 0.5 * g.dy, g.y(j) + 0.5 * g.dy);
                    let mut area = 0.0;
                    for &(p, q, h0, h1) in &pieces {
                        // split where h crosses |ya| or |yb|
                        let mut cuts = vec![0.0, 1.0];
                        for level in [ya.abs(), yb.abs()] {
                            if (h0 - level) * (h1 - level) < 0.0 {
                                cuts.push((level - h0) / (h1 - h0));
                            }
                        }
                        cuts.sort_by(f64::total_cmp);
                        for c in cuts.windows(2) {
                            let at = |f: f64| overlap(h0 + f * (h1 - h0), ya, yb);
                            area += 0.5 * (at(c[0]) + at(c[1])) * (c[1] - c[0]) * (q - p);
                        }
                    }
                    area / (g.dx * g.dy)
                })
                .collect()
        })
        .collect();
    (0..g.nx * g.ny)
        .map(|k| columns[k % g.nx][k / g.nx])
        .collect()
}

/// The plane's 2D model on cells of `step`, drawn or printed by the proxy of σ `sigma` (µm).
struct Model {
    grid: Grid,
    core: f64,
    clad: f64,
    litho: Option<Lithography>,
    template: Solver2d,
    input: PortMode,
    output: PortMode,
}

impl Model {
    fn new(step: f64, sigma: Option<f64>) -> photonoxide::Result<Model> {
        let lam = Wavelength::um(WAVELENGTH)?;
        let n = |m: &material::Material| -> photonoxide::Result<f64> {
            Ok(m.refractive_index(lam)?.re)
        };
        let (film, oxide) = (n(&material::silicon())?, n(&material::silica())?);
        let index = Slab::new(oxide, film, oxide, Length::nm(220.0))?
            .modes(photonoxide::mode::Polarization::Te, lam)[0]
            .effective_index();
        let pml = (PML as f64 * STEP / step).round() as usize;
        let length = total() + 2.0 * LEAD;
        let nx = (length / step).round() as usize + 2 * pml;
        let ny = 2 * (((0.5 * WIDTH + MARGIN) / step).ceil() as usize + pml);
        let grid = Grid {
            nx,
            ny,
            dx: step,
            dy: step,
            x0: -LEAD - pml as f64 * step,
            y0: -(ny as f64) * step / 2.0,
        };
        let litho = sigma
            .map(|s| Lithography::new([nx, ny, 1], step, s, THRESHOLD, Edge::Renormalized))
            .transpose()?;
        let (core, clad) = (index * index, oxide * oxide);
        let shares = shares(&grid, litho.as_ref(), &plain())?;
        let template = Solver2d::from_cells_on(
            grid,
            Polarization::Hz,
            lam,
            &permittivities(&shares, (core, clad)),
            Boundaries::pml(pml),
            &Choice::Photonoxide,
        )?;
        let input = template.port_modes(pml + 3, 1)?.remove(0);
        let output = template.port_modes(nx - pml - 4, 1)?.remove(0);
        Ok(Model {
            grid,
            core,
            clad,
            litho,
            template,
            input,
            output,
        })
    }

    fn cells(&self, x: &[f64]) -> photonoxide::Result<Vec<c64>> {
        Ok(permittivities(
            &shares(&self.grid, self.litho.as_ref(), x)?,
            (self.core, self.clad),
        ))
    }

    /// The power into the output strip's mode for the cells' shares `drawn`, as drawn (printed
    /// first if the model prints).
    fn power_of(&self, drawn: &[f64]) -> photonoxide::Result<f64> {
        let shares = match &self.litho {
            Some(l) => l.print(drawn)?,
            None => drawn.to_vec(),
        };
        let lam = Wavelength::um(WAVELENGTH)?;
        let s = self
            .template
            .reuse_cells(lam, &permittivities(&shares, (self.core, self.clad)))?;
        let field = s.solve_system(&s.mode_source(&self.input, Direction::Forward))?;
        let dy = self.grid.dy;
        let (forward, _) = field.mode_amplitudes(&self.output);
        Ok(forward.norm_sqr() * self.output.power(dy) / self.input.power(dy))
    }

    /// The power into the output strip's mode, and with `gradient` its derivatives in the
    /// variables: one solve and its adjoint, through the proxy if the model prints.
    fn power(&self, x: &[f64], gradient: bool) -> photonoxide::Result<(f64, Vec<f64>)> {
        let lam = Wavelength::um(WAVELENGTH)?;
        let s = self.template.reuse_cells(lam, &self.cells(x)?)?;
        let field = s.solve_system(&s.mode_source(&self.input, Direction::Forward))?;
        let dy = self.grid.dy;
        let scale = self.output.power(dy) / self.input.power(dy);
        if !gradient {
            let (forward, _) = field.mode_amplitudes(&self.output);
            return Ok((forward.norm_sqr() * scale, Vec::new()));
        }
        let (p, per_cell) = s.mode_power_gradient(&field, &self.output, Direction::Forward)?;
        let by_share: Vec<f64> = per_cell
            .iter()
            .map(|g| g * (self.core - self.clad))
            .collect();
        let by_mask = match &self.litho {
            Some(l) => l.pullback(&mask(&self.grid, x), &by_share)?,
            None => by_share,
        };
        let gradient = (0..x.len())
            .into_par_iter()
            .map(|k| {
                let at = |d: f64| {
                    let mut v = x.to_vec();
                    v[k] += d;
                    mask(&self.grid, &v)
                };
                let (up, down) = (at(SHAPE_STEP), at(-SHAPE_STEP));
                let d: f64 = by_mask
                    .iter()
                    .zip(up.iter().zip(&down))
                    .map(|(g, (a, b))| g * (a - b))
                    .sum();
                d * scale / (2.0 * SHAPE_STEP)
            })
            .collect();
        Ok((p * scale, gradient))
    }
}

/// Each cell's share of silicon, drawn or printed by `litho`.
fn shares(g: &Grid, litho: Option<&Lithography>, x: &[f64]) -> photonoxide::Result<Vec<f64>> {
    let drawn = mask(g, x);
    match litho {
        Some(l) => l.print(&drawn),
        None => Ok(drawn),
    }
}

fn permittivities(shares: &[f64], (core, clad): (f64, f64)) -> Vec<c64> {
    shares
        .iter()
        .map(|f| c64::new(clad + (core - clad) * f, 0.0))
        .collect()
}

/// Each converter's loss for the pair's transmission `t`, dB.
fn loss(t: f64) -> f64 {
    -5.0 * math::ln(t) / LN_10
}

/// The variables' bounds.
fn bounds() -> Vec<std::ops::RangeInclusive<f64>> {
    let mut b = vec![W_BOUNDS.0..=W_BOUNDS.1; PERIODS];
    b.extend(vec![L_BOUNDS.0..=L_BOUNDS.1; PERIODS]);
    b.extend(vec![B_BOUNDS.0..=B_BOUNDS.1; KNOTS - 1]);
    b
}

/// Genoxide's L-BFGS-B on `model`'s transmission from the plain converter, EVALUATIONS of them:
/// the best variables and the evaluations.
fn search(model: &Model) -> (Vec<f64>, u64) {
    let fitness = |x: &Reals, gradient: &mut [f64]| -> f64 {
        match model.power(x, true) {
            Ok((p, g)) => {
                for (a, b) in gradient.iter_mut().zip(&g) {
                    *a = -(b * 1e9).round() / 1e9;
                }
                -(p * 1e9).round() / 1e9
            }
            Err(_) => f64::NAN,
        }
    };
    let lbfgsb = Lbfgsb::builder(Real::new(bounds()).expect("bounds"))
        .initial_genome(Reals::from(plain()))
        .gradient_tolerance(0.0)
        .function_tolerance(0.0)
        .minimize()
        .build()
        .expect("a valid L-BFGS-B");
    let outcome = Engine::new(lbfgsb, Differentiable(&fitness))
        .stop_when(Stop::evaluations(EVALUATIONS))
        .run()
        .expect("a run");
    (outcome.best_genome().to_vec(), outcome.evaluations())
}

/// The forward solve's relative residual for the variables `x`.
fn residual(model: &Model, x: &[f64]) -> photonoxide::Result<f64> {
    let lam = Wavelength::um(WAVELENGTH)?;
    let s = model.template.reuse_cells(lam, &model.cells(x)?)?;
    let source = s.mode_source(&model.input, Direction::Forward);
    let field = s.solve_system(&source)?;
    s.residual(&field, &source)
}

/// The adjoint's derivatives at the plain converter, at a grating's width and length and a bridge
/// point, against fourth-order differences of the power along the same change of the cells'
/// shares, the derivative ∂m/∂x the adjoint is dotted with (δ = 1e-5 µm): each within what the
/// differences resolve, the powers' precision u (twice the solve's residual, rounded up to a
/// power of ten) through the formula's weights, (18/12) u T/(δ |g|). How many are within it, of
/// three: the check is the count, not the relative differences, whose last digits vary by system.
/// The adjoint's and the differences' values are printed beside it to 6 significant digits (the
/// designs to 1 nm, the losses to 4 decimals); Linux and Windows print the committed output, and
/// the Weekly run's examples job compares macOS.
fn check_derivatives(model: &Model, label: &str) -> photonoxide::Result<usize> {
    let x = plain();
    let (t, exact) = model.power(&x, true)?;
    let u = portable::powf(10f64, portable::log10(2.0 * residual(model, &x)?).ceil());
    let delta = 1e-5;
    let drawn = mask(&model.grid, &x);
    let mut within = 0;
    for (k, name) in [
        (4usize, "w5"),
        (PERIODS + 12, "l13"),
        (2 * PERIODS + 2, "b3"),
    ] {
        let shifted = |d: f64| {
            let mut v = x.clone();
            v[k] += d;
            mask(&model.grid, &v)
        };
        let (up, down) = (shifted(SHAPE_STEP), shifted(-SHAPE_STEP));
        let at = |s: f64| -> photonoxide::Result<f64> {
            let m: Vec<f64> = drawn
                .iter()
                .zip(up.iter().zip(&down))
                .map(|(m, (a, b))| m + s * (a - b) / (2.0 * SHAPE_STEP))
                .collect();
            model.power_of(&m)
        };
        let differences = (-at(2.0 * delta)? + 8.0 * at(delta)? - 8.0 * at(-delta)?
            + at(-2.0 * delta)?)
            / (12.0 * delta);
        let relative = (exact[k] - differences).abs() / exact[k].abs();
        let resolution = 18.0 / 12.0 * u * t / (delta * exact[k].abs());
        println!(
            "  {label}: dT/d{name}, adjoint {:.5e}, fourth-order differences {differences:.5e} (resolved to {resolution:.0e})",
            exact[k]
        );
        within += usize::from(relative <= resolution);
    }
    Ok(within)
}

pub fn main() -> photonoxide::Result<ExitCode> {
    let mut checks = common::Checks::default();
    let sigma = Lithography::widest_sigma(LINEAR_FROM, ACCURACY, THRESHOLD);
    let drawn = Model::new(STEP, None)?;
    let printing = Model::new(STEP, Some(sigma))?;
    let mut setup = common::Setup::new();
    setup.wavelength(Wavelength::um(WAVELENGTH)?);
    setup.item(
        "SWG (Halir et al.'s Table 1)",
        &format!(
            "220 nm silicon in oxide, period {}, duty cycle {} %, {} wide",
            common::length(PERIOD),
            100.0 * DUTY,
            common::length(WIDTH)
        ),
    );
    setup.item(
        "converters",
        &format!(
            "{PERIODS} gratings over a bridge narrowing from {} to nothing (Khan et al.'s Fig. 2), two back to back with {MIDDLE} periods of SWG between, from a strip to a strip",
            common::length(WIDTH)
        ),
    );
    setup.item(
        "variables",
        &format!(
            "the gratings' widths ({} to {}) and lengths ({} to {}), the bridge's half-width at {} points ({} to {}); the second converter the first's mirror image",
            common::length(W_BOUNDS.0),
            common::length(W_BOUNDS.1),
            common::length(L_BOUNDS.0),
            common::length(L_BOUNDS.1),
            KNOTS - 1,
            common::length(B_BOUNDS.0),
            common::length(B_BOUNDS.1)
        ),
    );
    setup.item(
        "2D plane",
        &format!(
            "the effective index method's: the 220 nm slab's TE index, {}, in the silicon, the oxide's, {}, outside; H along z",
            common::n(drawn.core.sqrt()),
            common::n(drawn.clad.sqrt())
        ),
    );
    setup.item(
        "grid",
        &format!(
            "{} × {} cells of {}, PMLs {PML} cells, each cell's share of silicon its exact area; the designs also on {} cells",
            drawn.grid.nx,
            drawn.grid.ny,
            common::length(STEP),
            common::length(FINE)
        ),
    );
    setup.item(
        "lithography proxy",
        &format!(
            "a Gaussian blur of σ {} and a threshold at {THRESHOLD}, sharp: the widest blur that prints a {} line within {} % (Selvaraja et al.)",
            common::length(sigma),
            common::length(LINEAR_FROM),
            100.0 * ACCURACY
        ),
    );
    setup.item(
        "solver",
        &format!(
            "genoxide's L-BFGS-B from the plain converter, {EVALUATIONS} evaluations, each a solve and its adjoint"
        ),
    );
    setup.print();
    println!(
        "Khan et al.'s SWG-to-strip converter, designed drawn and designed printed by a lithography proxy (σ = {:.2} nm, threshold {THRESHOLD}), at {WAVELENGTH} µm",
        1000.0 * sigma
    );

    let within = check_derivatives(&drawn, "drawn")? + check_derivatives(&printing, "printed")?;
    checks.count("derivatives within the differences' resolution", within, 6);

    let (standard, n_standard) = search(&drawn);
    println!("  the standard design: L-BFGS-B on the drawn model, {n_standard} evaluations");
    let (aware, n_aware) = search(&printing);
    println!(
        "  designed printed (Khan et al.'s FAID): on the printed model, {n_aware} evaluations"
    );
    for (label, x) in [("standard", &standard), ("designed printed", &aware)] {
        let w: Vec<String> = x[..PERIODS]
            .iter()
            .map(|v| format!("{:.0}", 1000.0 * v))
            .collect();
        let l: Vec<String> = x[PERIODS..2 * PERIODS]
            .iter()
            .map(|v| format!("{:.0}", 1000.0 * v))
            .collect();
        let b: Vec<String> = x[2 * PERIODS..]
            .iter()
            .map(|v| format!("{:.0}", 1000.0 * v))
            .collect();
        println!("  {label}: the gratings' widths (nm) {}", w.join(" "));
        println!("  {label}: their lengths (nm) {}", l.join(" "));
        println!("  {label}: the bridge's half-width (nm) {}", b.join(" "));
    }

    let fine_drawn = Model::new(FINE, None)?;
    let fine_printing = Model::new(FINE, Some(sigma))?;
    let each =
        |m: &Model, x: &[f64]| -> photonoxide::Result<f64> { Ok(loss(m.power(x, false)?.0)) };
    println!("  each converter's loss (dB)   drawn, 20 nm   10 nm    printed, 20 nm   10 nm");
    let mut rows = Vec::new();
    for (label, x) in [
        ("the plain converter", plain()),
        ("the standard design", standard.clone()),
        ("designed printed", aware.clone()),
    ] {
        let row = [
            each(&drawn, &x)?,
            each(&fine_drawn, &x)?,
            each(&printing, &x)?,
            each(&fine_printing, &x)?,
        ];
        println!(
            "  {label:<26} {:.4}         {:.4}   {:.4}           {:.4}",
            row[0], row[1], row[2], row[3]
        );
        rows.push(row);
    }
    for (process, ideal, standard, aware) in KHAN_SWG {
        println!(
            "  Khan et al.'s Table 1, their converter at 1320 nm, {process}: {ideal} as designed, {standard} printed, {aware} designed printed (their device, wavelength and models: context, not compared)"
        );
    }
    // their test of a simulation: the design made with the model prints better than the standard one
    checks.at_most(
        "printed loss, designed printed less the standard's (dB)",
        rows[2][2] - rows[1][2],
        0.0,
    );
    Ok(checks.finish())
}
