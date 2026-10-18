//! The static solver's validation cases: closed forms for heat conduction, electrostatics and
//! current, and the order of the error on refined grids.

use std::f64::consts::PI;

use super::{
    Axis, Boundary, Discrete, Grading, Grid, Problem, Section, Side, Tensor, elliptic_k, spread,
};
use crate::electro::{Conductor, Current, EPSILON_0, Electrostatic};
use crate::geometry::{Point, Shape};
use crate::thermal::{Heat, Scheme, Stepping};
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};

fn rect(x: [f64; 2], y: [f64; 2]) -> Shape {
    Shape::rect(
        Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
        Length::um(x[1] - x[0]),
        Length::um(y[1] - y[0]),
    )
    .expect("a rectangle")
}

fn order(coarse: f64, fine: f64, ratio: f64) -> f64 {
    (coarse / fine).ln() / ratio.ln()
}

/// Two layers between a fixed temperature below and above: oxide (1.4 W/(m·K), 2 µm) under
/// silicon (148 W/(m·K), 0.22 µm), 0 K at the bottom, 1 K at the top, on a graded grid with a
/// line at the interface. Returns the largest error at the cells' centres against the
/// piecewise-linear exact temperature (K), and the relative error of the flux through each side
/// against 1/(d₁/k₁ + d₂/k₂).
pub(crate) fn two_layers() -> (f64, f64) {
    let (k1, k2, d1, d2) = (1.4, 148.0, 2.0, 0.22);
    let mut section = Section::new("oxide");
    section.layer(d1, d1 + d2, "silicon").expect("a layer");
    let grid = Grid::fitted(
        [0.0, 1.0],
        [0.0, d1 + d2],
        &Grading::new(0.25, 0.01, 1.3).expect("a grading"),
        section.features(),
    )
    .expect("a grid");
    let mut heat = Heat::new(section, grid);
    heat.conductivity("oxide", Tensor::isotropic(k1))
        .conductivity("silicon", Tensor::isotropic(k2))
        .boundary(Side::Bottom, Boundary::Fixed(0.0))
        .boundary(Side::Top, Boundary::Fixed(1.0));
    let t = heat.solve().expect("a solve");
    let resistance = d1 / k1 + d2 / k2;
    let at_interface = (d1 / k1) / resistance;
    let exact = |y: f64| {
        if y <= d1 {
            at_interface * y / d1
        } else {
            at_interface + (1.0 - at_interface) * (y - d1) / d2
        }
    };
    let g = t.grid();
    let mut worst = 0.0f64;
    for j in 0..g.y().cells() {
        for i in 0..g.x().cells() {
            worst = worst.max((t.value(i, j) - exact(g.y().centre(j))).abs());
        }
    }
    // W/m through the 1 µm wide column: (1 K / resistance in µm·m·K/W) × 1 µm
    let flux = 1.0 / resistance;
    let flux_error =
        ((t.outflow(Side::Bottom) - flux).abs()).max((t.outflow(Side::Top) + flux).abs()) / flux;
    (worst, flux_error)
}

/// A slab of oxide (1.4 W/(m·K), 2 µm) heated uniformly (1 W/m over its 1 µm wide column) with
/// both faces at 0 K, on `cells` equal rows: the largest error at the cells' centres against
/// T = q y (L − y) / 2k (K).
pub(crate) fn heated_slab(cells: usize) -> f64 {
    let (k, length, width, power) = (1.4, 2.0, 1.0, 1.0);
    let grid = Grid::new(
        Axis::uniform(0.0, width, 1).expect("an axis"),
        Axis::uniform(0.0, length, cells).expect("an axis"),
    );
    let mut heat = Heat::new(Section::new("oxide"), grid);
    heat.conductivity("oxide", Tensor::isotropic(k))
        .boundary(Side::Bottom, Boundary::Fixed(0.0))
        .boundary(Side::Top, Boundary::Fixed(0.0))
        .heater(rect([0.0, width], [0.0, length]), power);
    let t = heat.solve().expect("a solve");
    // q in W/m³: power over the area (µm² → m²)
    let q = power / (width * length * 1e-12);
    let g = t.grid();
    (0..cells)
        .map(|j| {
            let y = g.y().centre(j) * 1e-6;
            let exact = q * y * (length * 1e-6 - y) / (2.0 * k);
            (t.value(0, j) - exact).abs()
        })
        .fold(0.0, f64::max)
}

/// Oxide's heat capacity per volume, J/(m³·K): Jacques et al.'s Table 3 (doi:10.1364/OE.27.010456),
/// 2203 kg/m³ and 709 J/(kg·K) at 300 K.
const OXIDE_RHO_CP: f64 = 2203.0 * 709.0;

/// Silicon's, J/(m³·K): the same table's 2330 kg/m³ and 711 J/(kg·K).
const SILICON_RHO_CP: f64 = 2330.0 * 711.0;

/// The heated slab of [`heated_slab`] (oxide, 2 µm, 1 W/m over its 1 µm wide column, both faces
/// at 0 K) turned on at t = 0 from 0 K, on `cells` rows, `steps` steps of `scheme` to
/// `until` seconds. Returns the mean's largest error over the record against the series
/// [`slab_series_mean`], relative to its steady value, the last mean (K), and the largest error
/// of the last field's cells against [`slab_series`], relative to the steady peak.
pub(crate) fn slab_transient(cells: usize, scheme: Scheme, steps: usize, until: f64) -> [f64; 3] {
    let (k, length, width, power) = (1.4, 2.0, 1.0, 1.0);
    let grid = Grid::new(
        Axis::uniform(0.0, width, 1).expect("an axis"),
        Axis::uniform(0.0, length, cells).expect("an axis"),
    );
    let mut heat = Heat::new(Section::new("oxide"), grid);
    heat.conductivity("oxide", Tensor::isotropic(k))
        .heat_capacity("oxide", OXIDE_RHO_CP)
        .boundary(Side::Bottom, Boundary::Fixed(0.0))
        .boundary(Side::Top, Boundary::Fixed(0.0))
        .heater(rect([0.0, width], [0.0, length]), power);
    let stepping = Stepping {
        step_s: until / steps as f64,
        steps,
        scheme,
    };
    let slab = rect([0.0, width], [0.0, length]);
    let history = heat
        .transient(&stepping, 0.0, &|_| 1.0, std::slice::from_ref(&slab))
        .expect("a transient");
    let q = power / (width * length * 1e-12);
    let (d, alpha) = (length * 1e-6, k / OXIDE_RHO_CP);
    let steady = q * d * d / (12.0 * k);
    let mean_error = history
        .times_s
        .iter()
        .zip(&history.watched[0])
        .map(|(&t, &m)| (m - slab_series_mean(q, k, d, alpha, t)).abs() / steady)
        .fold(0.0, f64::max);
    let last = history.watched[0].last().copied().expect("a record");
    let peak = q * d * d / (8.0 * k);
    let g = history.last.grid();
    let field_error = (0..cells)
        .map(|j| {
            let y = g.y().centre(j) * 1e-6;
            (history.last.value(0, j) - slab_series(q, k, d, alpha, y, until)).abs() / peak
        })
        .fold(0.0, f64::max);
    [mean_error, last, field_error]
}

/// A slab 0 < y < d heated at `q` W/m³ from 0 K at t = 0, its faces held at 0 K, conductivity
/// `k` and diffusivity `alpha`: T(y, t) = q y (d − y)/2k − Σ_odd 4qd²/(k n³π³) sin(nπy/d)
/// e^{−n²π²αt/d²} (H. S. Carslaw, J. C. Jaeger, Conduction of Heat in Solids, 2nd ed., Oxford
/// 1959, Section 3.14), its odd terms to n = 4001.
fn slab_series(q: f64, k: f64, d: f64, alpha: f64, y: f64, t: f64) -> f64 {
    let mut sum = q * y * (d - y) / (2.0 * k);
    for n in (1..=4001).step_by(2) {
        let n = f64::from(n);
        sum -= 4.0 * q * d * d / (k * (n * PI).powi(3))
            * (n * PI * y / d).sin()
            * (-(n * PI / d).powi(2) * alpha * t).exp();
    }
    sum
}

/// The same slab's mean over its thickness: qd²/12k − Σ_odd 8qd²/(k n⁴π⁴) e^{−n²π²αt/d²}.
fn slab_series_mean(q: f64, k: f64, d: f64, alpha: f64, t: f64) -> f64 {
    let mut sum = q * d * d / (12.0 * k);
    for n in (1..=4001).step_by(2) {
        let n = f64::from(n);
        sum -= 8.0 * q * d * d / (k * (n * PI).powi(4)) * (-(n * PI / d).powi(2) * alpha * t).exp();
    }
    sum
}

/// The order in time of `scheme` on the slab of [`slab_transient`] (80 rows, to 0.5 µs): from
/// the last mean on 25, 50 and 100 steps, log₂ of the ratio of successive differences.
pub(crate) fn slab_time_order(scheme: Scheme) -> f64 {
    let at = |steps| slab_transient(80, scheme, steps, 0.5e-6)[1];
    let (a, b, c) = (at(25), at(50), at(100));
    order((a - b).abs(), (b - c).abs(), 2.0)
}

/// A lumped thermal RC: silicon 0.22 µm thick (148 W/(m·K), Table 3's ρc_p) heated uniformly on 2
/// µm of oxide (1.38 W/(m·K)) whose heat capacity is set to 1 J/(m³·K), so the silicon's alone
/// counts, over a sink at 0 K at the oxide's bottom, the top insulated, on a grid graded from 10
/// to 100 nm. Returns the silicon's 1/e rise time from a turn-on with BDF2 (steps of τ/400 to
/// 3τ) and its −3 dB frequency, against τ = (d_ox/k_ox)(ρc_p,Si t_Si) and 1/(2πτ), relative
/// errors, and τ (s).
pub(crate) fn lumped_rc() -> (f64, f64, f64) {
    let (k_ox, d_ox, t_si) = (1.38, 2.0, 0.22);
    let mut section = Section::new("oxide");
    section
        .layer(d_ox, d_ox + t_si, "silicon")
        .expect("a layer");
    let grid = Grid::fitted(
        [0.0, 1.0],
        [0.0, d_ox + t_si],
        &Grading::new(0.1, 0.01, 1.3).expect("a grading"),
        section.features(),
    )
    .expect("a grid");
    let silicon = rect([0.0, 1.0], [d_ox, d_ox + t_si]);
    let mut heat = Heat::new(section, grid);
    heat.conductivity("oxide", Tensor::isotropic(k_ox))
        .conductivity("silicon", Tensor::isotropic(148.0))
        .heat_capacity("oxide", 1.0)
        .heat_capacity("silicon", SILICON_RHO_CP)
        .boundary(Side::Bottom, Boundary::Fixed(0.0))
        .heater(silicon.clone(), 1.0);
    let tau = (d_ox * 1e-6 / k_ox) * (SILICON_RHO_CP * t_si * 1e-6);
    let steady = heat
        .solve()
        .expect("a solve")
        .mean(&silicon)
        .expect("the silicon");
    let stepping = Stepping {
        step_s: tau / 400.0,
        steps: 1200,
        scheme: Scheme::Bdf2,
    };
    let history = heat
        .transient(&stepping, 0.0, &|_| 1.0, std::slice::from_ref(&silicon))
        .expect("a transient");
    let rise = history
        .crossing(0, 0.0, steady, 1.0 - (-1.0f64).exp())
        .expect("a rise");
    let cutoff = heat.cutoff(&silicon).expect("a cutoff");
    let exact_cutoff = 1.0 / (2.0 * PI * tau);
    (
        (rise - tau).abs() / tau,
        (cutoff - exact_cutoff).abs() / exact_cutoff,
        tau,
    )
}

/// A parallel-plate capacitor 1 µm wide with two dielectrics between its plates (0.15 µm of
/// ε = 3.9 under 0.05 µm of ε = 7.5), insulated at the sides, on a graded grid: its capacitance
/// against ε₀ W / (d₁/ε₁ + d₂/ε₂), relative error.
pub(crate) fn parallel_plate() -> f64 {
    let (e1, e2, d1, d2) = (3.9, 7.5, 0.15, 0.05);
    let mut section = Section::new("oxide");
    section.layer(d1, d1 + d2, "nitride").expect("a layer");
    let lower = rect([-1.0, 2.0], [-0.1, 0.0]);
    let upper = rect([-1.0, 2.0], [d1 + d2, d1 + d2 + 0.1]);
    let mut features = section.features().clone();
    features.shape(&lower).shape(&upper);
    let grid = Grid::fitted(
        [0.0, 1.0],
        [-0.1, d1 + d2 + 0.1],
        &Grading::new(0.05, 0.005, 1.3).expect("a grading"),
        &features,
    )
    .expect("a grid");
    let mut problem = Electrostatic::new(section, grid);
    problem
        .permittivity("oxide", Tensor::isotropic(e1))
        .permittivity("nitride", Tensor::isotropic(e2))
        .conductor(Conductor::shape(lower, 0.0))
        .conductor(Conductor::shape(upper, 1.0));
    let c = problem.capacitance().expect("a capacitance");
    let exact = EPSILON_0 * 1.0 / (d1 / e1 + d2 / e2);
    ((c[1][1] - exact).abs() / exact).max((c[0][1] + exact).abs() / exact)
}

/// A coaxial line, inner radius 0.3 µm at 1 V, outer 1 µm at 0 V, ε = 3.9 between, on an n × n
/// grid of the 2.4 µm square around it: the capacitance's relative error against
/// 2πε₀ε / ln(b/a).
pub(crate) fn coaxial(n: usize) -> f64 {
    let (a, b, eps) = (0.3, 1.0, 3.9);
    let side = Axis::uniform(-1.2, 1.2, n).expect("an axis");
    let grid = Grid::new(side.clone(), side);
    let inner = Shape::circle(Point::um(0.0, 0.0), Length::um(a)).expect("a disk");
    let outer = Shape::Ring {
        center: Point::um(0.0, 0.0),
        inner: Length::um(b),
        outer: Length::um(5.0),
    };
    let mut problem = Electrostatic::new(Section::new("oxide"), grid);
    problem
        .permittivity("oxide", Tensor::isotropic(eps))
        .conductor(Conductor::shape(outer, 0.0))
        .conductor(Conductor::shape(inner, 1.0));
    let field = problem.solve().expect("a solve");
    let exact = 2.0 * PI * EPSILON_0 * eps / (b / a).ln();
    (field.held()[1] - exact).abs() / exact
}

/// A line source of heat (1 W/m, a disk of radius 0.1 µm) 1 µm above an isothermal plane, in a
/// medium of 1.4 W/(m·K), on the window [−2, 2] × [0, 2] µm of `n` × n/2 equal cells, the
/// window's other sides held at the exact temperature: the largest error (K) at the cells'
/// centres farther than 0.2 µm from the source, against the images' P/(2πk) ln(r′/r).
pub(crate) fn line_source(n: usize) -> f64 {
    let (k, h, a, power) = (1.4, 1.0, 0.1, 1.0);
    let exact = |x: f64, y: f64| {
        let r = x.hypot(y - h);
        let image = x.hypot(y + h);
        power / (2.0 * PI * k) * (image / r).ln()
    };
    let grid = Grid::new(
        Axis::uniform(-2.0, 2.0, n).expect("an axis"),
        Axis::uniform(0.0, 2.0, n / 2).expect("an axis"),
    );
    let profile = |side: Side, along: f64| match side {
        Side::Left => exact(-2.0, along),
        Side::Right => exact(2.0, along),
        Side::Bottom => 0.0,
        Side::Top => exact(along, 2.0),
    };
    let problem = Problem {
        grid: &grid,
        coefficients: vec![[k, k]; grid.cells()],
        sides: [Boundary::Fixed(0.0); 4],
        holds: Vec::new(),
        profile: Some(&profile),
    };
    let discrete = Discrete::new(&problem).expect("a problem");
    let mut sources = vec![0.0; grid.cells()];
    let disk = Shape::circle(Point::um(0.0, h), Length::um(a)).expect("a disk");
    spread(&grid, &disk, power, &mut sources).expect("a source");
    let t = discrete.solve(&[], &sources).expect("a solve");
    let mut worst = 0.0f64;
    for j in 0..grid.y().cells() {
        for i in 0..grid.x().cells() {
            let (x, y) = (grid.x().centre(i), grid.y().centre(j));
            if x.hypot(y - h) > 2.0 * a {
                worst = worst.max((t.value(i, j) - exact(x, y)).abs());
            }
        }
    }
    worst
}

/// A coplanar waveguide of zero-thickness strips on a substrate (ε = 11.7) filling y < 0 under
/// air: the signal from −1 to 1 µm, the grounds beyond ±1.6 µm to the window's edge, the window
/// ±`reach` µm on every side held at 0 V, graded from `smallest` µm at the strips' edges with
/// growth 1.04 to 5 µm. Its capacitance against Wen's 2ε₀(ε + 1) K(k)/K(k′), k = a/b (relative error),
/// and the number of cells.
pub(crate) fn coplanar(smallest: f64, reach: f64) -> (f64, usize) {
    coplanar_graded(smallest, reach, 1.04, 5.0)
}

/// [`coplanar`] with its grading's growth and largest step.
pub(crate) fn coplanar_graded(
    smallest: f64,
    reach: f64,
    growth: f64,
    largest: f64,
) -> (f64, usize) {
    let (a, b, eps) = (1.0, 1.6, 11.7);
    let mut section = Section::new("air");
    section
        .layer(-2.0 * reach, 0.0, "substrate")
        .expect("a layer");
    let mut features = section.features().clone();
    features
        .strip([-a, a], 0.0)
        .strip([b, reach], 0.0)
        .strip([-reach, -b], 0.0);
    let grid = Grid::fitted(
        [-reach, reach],
        [-reach, reach],
        &Grading::new(largest, smallest, growth).expect("a grading"),
        &features,
    )
    .expect("a grid");
    let cells = grid.cells();
    let mut problem = Electrostatic::new(section, grid);
    problem
        .permittivity("air", Tensor::isotropic(1.0))
        .permittivity("substrate", Tensor::isotropic(eps));
    for side in Side::ALL {
        problem.boundary(side, Boundary::Fixed(0.0));
    }
    problem
        .conductor(Conductor::strip([-a, a], 0.0, 1.0))
        .conductor(Conductor::strip([b, reach], 0.0, 0.0))
        .conductor(Conductor::strip([-reach, -b], 0.0, 0.0));
    let field = problem.solve().expect("a solve");
    let k = a / b;
    let kp = (1.0 - k * k).sqrt();
    let exact = 2.0 * EPSILON_0 * (eps + 1.0) * elliptic_k(k) / elliptic_k(kp);
    ((field.held()[0] - exact).abs() / exact, cells)
}

/// A resistor: a 2 µm long, 0.22 µm thick bar of 1e4 S/m between two contacts at 0 and 1 V,
/// in oxide (1e-12 S/m), and its Joule heat conducted to a substrate at 0 K. Returns the Joule
/// power's relative error against V² σ t / L, and the heat balance's (all of it leaves through
/// the substrate).
pub(crate) fn resistor() -> (f64, f64) {
    let (sigma, length, thickness) = (1e4, 2.0, 0.22);
    let mut section = Section::new("oxide");
    let bar = rect([0.0, length], [1.0, 1.0 + thickness]);
    section.draw(bar, "doped");
    let left = rect([-0.5, 0.0], [1.0, 1.0 + thickness]);
    let right = rect([length, length + 0.5], [1.0, 1.0 + thickness]);
    let mut features = section.features().clone();
    features.shape(&left).shape(&right);
    let grid = Grid::fitted(
        [-1.0, length + 1.0],
        [0.0, 3.0],
        &Grading::new(0.1, 0.01, 1.2).expect("a grading"),
        &features,
    )
    .expect("a grid");
    let mut current = Current::new(section.clone(), grid.clone());
    current
        .conductivity("doped", Tensor::isotropic(sigma))
        .conductivity("oxide", Tensor::isotropic(1e-12))
        .contact(Conductor::shape(left, 0.0))
        .contact(Conductor::shape(right, 1.0));
    let flow = current.solve().expect("a current");
    let exact = sigma * thickness / length;
    let power = (flow.power() - exact).abs() / exact;
    let mut heat = Heat::new(section, grid);
    heat.conductivity("doped", Tensor::isotropic(148.0))
        .conductivity("oxide", Tensor::isotropic(1.4))
        .boundary(Side::Bottom, Boundary::Fixed(0.0));
    heat.joule(&flow).expect("the same grid");
    let t = heat.solve().expect("a solve");
    let balance = (t.outflow(Side::Bottom) - flow.power()).abs() / flow.power();
    (power, balance)
}

/// A heater (10 W/m, 2 × 0.2 µm) 2 µm above the 220 nm SOI stack's silicon, the stack cut
/// across a 500 nm strip (silicon 148, oxide 1.4 W/(m·K)), the substrate's bottom 10 µm down at
/// 0 K, a convective film (10 W/(m²·K)) on top, the sides insulated: the balance's relative
/// imbalance, and the strip's mean rise (K).
pub(crate) fn soi_heater() -> (f64, f64) {
    let mut structure = crate::stack::Structure::new(crate::stack::LayerStack::soi_220());
    let strip = rect([-5.0, 5.0], [-0.25, 0.25]);
    structure.draw("Si", strip).expect("the strip");
    let mut section = Section::cut(&structure, super::Cut::X(0.0)).expect("a cut");
    let heater = rect([-1.0, 1.0], [4.22, 4.42]);
    let mut features = section.features().clone();
    features.shape(&heater);
    section.draw(heater.clone(), "SiO2");
    let grid = Grid::fitted(
        [-20.0, 20.0],
        [-12.0, 6.0],
        &Grading::new(1.0, 0.02, 1.25).expect("a grading"),
        &features,
    )
    .expect("a grid");
    let mut heat = Heat::new(section, grid);
    heat.conductivity("Si", Tensor::isotropic(148.0))
        .conductivity("SiO2", Tensor::isotropic(1.4))
        .boundary(Side::Bottom, Boundary::Fixed(0.0))
        .boundary(
            Side::Top,
            Boundary::Convective {
                coefficient: 10.0,
                ambient: 0.0,
            },
        )
        .heater(heater, 10.0);
    let t = heat.solve().expect("a solve");
    let guide = rect([-0.25, 0.25], [2.0, 2.22]);
    (t.imbalance(), t.mean(&guide).expect("the strip's cells"))
}

/// Jacques et al. 2019's TiN heater (Opt. Express 27, 10456, doi:10.1364/OE.27.010456): the
/// paper's cross-section and Table 3's conductivities, its simulated Pπ of 23.9 mW for a 320 µm
/// phase shifter at 1550 nm.
///
/// - **The stack** (Section 2.2, Fig. 1, Table 3): a 500 × 220 nm silicon strip (k = 90
///   W/(m·K), the 220 nm film's) on a 2 µm buried oxide over the silicon substrate (148), in an
///   oxide cladding (1.38); a TiN heater (28) 2.0 µm wide, 2 µm above the strip. The region is
///   40 µm wide and 18 µm high, its bottom (in the substrate) held at the ambient, a film of 10
///   W/(m²·K) to the air above the oxide.
/// - **What the paper doesn't give,** varied here: the TiN's thickness ("withheld" in Table 3),
///   taken as 100 or 200 nm, and whether its 2 µm are from the strip's top or from the buried
///   oxide's (Fig. 4's H_clad); the oxide above the heater as 1 µm; the sides insulated.
/// - **Into Pπ** as the paper does (its Eqs. 4 and 5): the strip's mean rise per watt, the
///   effective index's change per change of silicon's index (photonoxide's full-vector mode
///   solver, the strip in the paper's indices, silicon 3.476 and oxide 1.55 from Table 3),
///   and silicon's dn/dT, 1.8 × 10⁻⁴ K⁻¹ (the paper's). Pπ = λ/(2 L) ÷ (∂n_eff/∂n_Si · dn/dT ·
///   ΔT/P′) × L, with L = 320 µm.
///
/// Returns Pπ (mW) for each of the four variants, the strip's rise per W/m (K) of the first,
/// and ∂n_eff/∂n_Si.
pub(crate) fn jacques_tin_heater() -> (Vec<f64>, f64, f64) {
    jacques_tin_heater_on(&Grading::new(0.5, 0.02, 1.2).expect("a grading"), 1.55)
}

/// [`jacques_tin_heater`] on `grading`, the cladding's index `cladding` in the mode solve.
pub(crate) fn jacques_tin_heater_on(grading: &Grading, cladding: f64) -> (Vec<f64>, f64, f64) {
    let wavelength_um = 1.55;
    let length_um = 320.0;
    let dn_dt = 1.8e-4;
    let slope = strip_index_slope(0.5, 0.22, 3.476, cladding, wavelength_um);
    let mut rises = Vec::new();
    for (tin, from_strip_top) in JACQUES_VARIANTS {
        let (heat, strip) = jacques_section(tin, from_strip_top, grading);
        let t = heat.solve().expect("a solve");
        rises.push(t.mean(&strip).expect("the strip's cells"));
    }
    // a π shift: Δn_eff = λ / (2 L), from P′ (W/m) giving Δn_eff = slope · dn/dT · rise · P′
    let pi = |rise: f64| {
        let watts_per_metre = wavelength_um / (2.0 * length_um) / (slope * dn_dt * rise);
        watts_per_metre * length_um * 1e-6 * 1e3
    };
    (rises.iter().map(|&r| pi(r)).collect(), rises[0], slope)
}

/// The variants of what Jacques et al. don't state: the TiN's thickness (µm), and whether its 2
/// µm are from the strip's top (or the buried oxide's).
const JACQUES_VARIANTS: [(f64, bool); 4] = [(0.1, true), (0.2, true), (0.1, false), (0.2, false)];

/// Jacques et al.'s TiN heater cross-section of [`jacques_tin_heater`], one variant, on
/// `grading`, the heater at 1 W/m, with Table 3's heat capacities (Si 2330 kg/m³ and 711
/// J/(kg·K), TiN 5240 and 598, oxide 2203 and 709); and the strip.
fn jacques_section(tin: f64, from_strip_top: bool, grading: &Grading) -> (Heat, Shape) {
    let (strip_w, strip_h, box_h) = (0.5, 0.22, 2.0);
    let (window_w, window_h) = (40.0, 18.0);
    // y = 0 at the buried oxide's top
    let tin_bottom = if from_strip_top { strip_h + 2.0 } else { 2.0 };
    let top = tin_bottom + tin + 1.0;
    let bottom = top - window_h;
    let mut section = Section::new("SiO2");
    section
        .layer(bottom, -box_h, "substrate")
        .expect("the substrate");
    let strip = rect([-0.5 * strip_w, 0.5 * strip_w], [0.0, strip_h]);
    let heater = rect([-1.0, 1.0], [tin_bottom, tin_bottom + tin]);
    section.draw(strip.clone(), "Si");
    section.draw(heater.clone(), "TiN");
    let mut features = section.features().clone();
    features.shape(&strip).shape(&heater);
    let grid = Grid::fitted(
        [-0.5 * window_w, 0.5 * window_w],
        [bottom, top],
        grading,
        &features,
    )
    .expect("a grid");
    let mut heat = Heat::new(section, grid);
    heat.conductivity("SiO2", Tensor::isotropic(1.38))
        .conductivity("substrate", Tensor::isotropic(148.0))
        .conductivity("Si", Tensor::isotropic(90.0))
        .conductivity("TiN", Tensor::isotropic(28.0))
        .heat_capacity("SiO2", OXIDE_RHO_CP)
        .heat_capacity("substrate", SILICON_RHO_CP)
        .heat_capacity("Si", SILICON_RHO_CP)
        .heat_capacity("TiN", 5240.0 * 598.0)
        .boundary(Side::Bottom, Boundary::Fixed(0.0))
        .boundary(
            Side::Top,
            Boundary::Convective {
                coefficient: 10.0,
                ambient: 0.0,
            },
        )
        .heater(heater, 1.0);
    (heat, strip)
}

/// Jacques et al.'s TiN heater of [`jacques_tin_heater`] in time: turned on from the ambient,
/// the strip's mean temperature recorded, steps of `step_s` by BDF2 to 20 µs, on `grading`.
/// Returns each of the four variants' 1/e rise time (s), against the paper's simulated ~10.2 µs
/// (Section 2.2, Fig. 1(c)). The fall from the steady state is the same: the problem is linear,
/// so a turn-off's history is the steady state less a turn-on's.
pub(crate) fn jacques_tin_heater_times(grading: &Grading, step_s: f64) -> Vec<f64> {
    let stepping = Stepping {
        step_s,
        steps: (20e-6 / step_s).round() as usize,
        scheme: Scheme::Bdf2,
    };
    JACQUES_VARIANTS
        .iter()
        .map(|&(tin, from_strip_top)| {
            let (heat, strip) = jacques_section(tin, from_strip_top, grading);
            let steady = heat
                .solve()
                .expect("a solve")
                .mean(&strip)
                .expect("the strip");
            heat.transient(&stepping, 0.0, &|_| 1.0, std::slice::from_ref(&strip))
                .expect("a turn-on")
                .crossing(0, 0.0, steady, 1.0 - (-1.0f64).exp())
                .expect("a rise")
        })
        .collect()
}

/// ∂n_eff/∂n_core of a strip's fundamental TE-like mode at `wavelength_um`, by a central
/// difference of photonoxide's full-vector mode solver (10 nm grid, a 2.1 × 1.5 µm window, the
/// strip's edges on its nodes).
fn strip_index_slope(width: f64, height: f64, core: f64, cladding: f64, wavelength_um: f64) -> f64 {
    let wavelength = crate::units::Wavelength::um(wavelength_um).expect("a wavelength");
    let n_eff = |n: f64| {
        let (h, hw, hh) = (0.01, 1.05, 0.75);
        let cs = crate::mode::vector::CrossSection::uniform(
            (-hw, hw, (2.0 * hw / h).round() as usize),
            (-hh, hh, (2.0 * hh / h).round() as usize),
            |x, y| {
                let inside = x.abs() < 0.5 * width && y.abs() < 0.5 * height;
                let n = if inside { n } else { cladding };
                crate::mode::vector::Permittivity::isotropic(num_complex::Complex64::new(
                    n * n,
                    0.0,
                ))
            },
        )
        .expect("a cross-section");
        let modes = crate::mode::vector::modes(&cs, wavelength, 1, None).expect("a mode");
        modes[0].effective_index().re
    };
    let d = 1e-3;
    (n_eff(core + d) - n_eff(core - d)) / (2.0 * d)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "static/two-layer-temperature",
            title: r"Static solver, heat: 2 µm of oxide (1.4 W/(m·K)) under 0.22 µm of silicon (148 W/(m·K)), 0 and 1 K at the bottom and top, on a grid graded from 10 to 250 nm with a line at the interface; every cell's temperature against the piecewise-linear exact one (largest error shown, K)",
            tier: Tier::Analytic,
            source: r"$T$ linear in each layer, continuous with $k\thinspace\partial T/\partial y$ continuous at the interface; the scheme's faces take the layers' resistances in series (Patankar 1980, Section 4.2-3; doi:10.1201/9781482234213), exact on the grid's lines",
            run: || {
                let (e, _) = two_layers();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-12,
                    error: e,
                }
            },
        },
        Case {
            id: "static/two-layer-flux",
            title: r"Static solver, heat: the same two layers; the heat through the bottom and top against $\Delta T / (d_1/k_1 + d_2/k_2)$ per µm of width (largest relative error shown)",
            tier: Tier::Analytic,
            source: r"the series thermal resistance of a layered slab; flux continuity at the interface",
            run: || {
                let (_, e) = two_layers();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-12,
                    error: e,
                }
            },
        },
        Case {
            id: "static/heated-slab-order",
            title: r"Static solver, heat: a 2 µm oxide slab generating 1 W/m uniformly, both faces at 0 K, on 20, 40 and 80 equal rows; the order of the largest error against $T = q y (L - y)/2k$ from 40 to 80 rows (shown)",
            tier: Tier::Analytic,
            source: r"the slab's parabola; cell-centred finite volumes converge at second order",
            run: || {
                let (e1, e2) = (heated_slab(40), heated_slab(80));
                let p = order(e1, e2, 2.0);
                Outcome {
                    measured: p,
                    expected: 2.0,
                    tolerance: 0.05,
                    error: (p - 2.0).abs(),
                }
            },
        },
        Case {
            id: "static/heated-slab",
            title: r"Static solver, heat: the same slab on 80 rows of 25 nm; the largest error against the parabola, relative to its peak $qL^2/8k$ (shown)",
            tier: Tier::Analytic,
            source: r"$T = q y (L - y) / 2k$",
            run: || {
                let peak = 1.0 / (1.0 * 2.0 * 1e-12) * (2e-6f64).powi(2) / (8.0 * 1.4);
                let e = heated_slab(80) / peak;
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 2e-4,
                    error: e,
                }
            },
        },
        Case {
            id: "static/transient-slab",
            title: r"Static solver, heat in time: the same 2 µm oxide slab ($\rho c_p$ = 1.56 MJ/(m³·K), Jacques et al.'s Table 3) turned on from 0 K, on 80 rows of 25 nm, 400 Crank–Nicolson steps of 1.25 ns to 0.5 µs (its slowest time $d^2/\pi^2\alpha$ is 0.45 µs); the mean temperature at every step against the series solution, relative to its steady value (largest shown)",
            tier: Tier::Analytic,
            source: r"$\bar T(t) = \frac{qd^2}{12k} - \sum_{n\ \mathrm{odd}} \frac{8qd^2}{k n^4\pi^4} e^{-n^2\pi^2\alpha t/d^2}$, from H. S. Carslaw, J. C. Jaeger, Conduction of Heat in Solids, 2nd ed. (Oxford, 1959), Section 3.14; the error is the grid's, as the steady slab's",
            run: || {
                let e = slab_transient(80, Scheme::CrankNicolson, 400, 0.5e-6)[0];
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 3e-4,
                    error: e,
                }
            },
        },
        Case {
            id: "static/transient-slab-order-time-cn",
            title: r"Static solver, heat in time: the same slab on 80 rows, its mean at 0.5 µs with 25, 50 and 100 Crank–Nicolson steps, the first as two backward-Euler half steps; the order in the time step from the successive differences (shown)",
            tier: Tier::Analytic,
            source: r"Crank–Nicolson is second order; Rannacher's start (Numer. Math. 43, 309 (1984), doi:10.1007/BF01390130) damps the step's start without losing it",
            run: || {
                let p = slab_time_order(Scheme::CrankNicolson);
                Outcome {
                    measured: p,
                    expected: 2.0,
                    tolerance: 0.1,
                    error: (p - 2.0).abs(),
                }
            },
        },
        Case {
            id: "static/transient-slab-order-time-bdf2",
            title: r"Static solver, heat in time: the same, by BDF2 from a backward-Euler first step; the order in the time step (shown)",
            tier: Tier::Analytic,
            source: r"the second-order backward differentiation formula; one first-order step costs a constant, not the order",
            run: || {
                let p = slab_time_order(Scheme::Bdf2);
                Outcome {
                    measured: p,
                    expected: 2.0,
                    tolerance: 0.1,
                    error: (p - 2.0).abs(),
                }
            },
        },
        Case {
            id: "static/transient-slab-order-grid",
            title: r"Static solver, heat in time: the same slab at 0.5 µs on 40 and 80 rows, 4000 Crank–Nicolson steps of 0.125 ns; the order in the grid of every cell's largest error against the series $T(y, t)$ (shown)",
            tier: Tier::Analytic,
            source: r"$T(y,t) = \frac{q y (d-y)}{2k} - \sum_{n\ \mathrm{odd}} \frac{4qd^2}{k n^3\pi^3} \sin\frac{n\pi y}{d} e^{-n^2\pi^2\alpha t/d^2}$; cell-centred finite volumes, second order in space as in the steady state",
            run: || {
                let at = |rows| slab_transient(rows, Scheme::CrankNicolson, 4000, 0.5e-6)[2];
                let p = order(at(40), at(80), 2.0);
                Outcome {
                    measured: p,
                    expected: 2.0,
                    tolerance: 0.05,
                    error: (p - 2.0).abs(),
                }
            },
        },
        Case {
            id: "static/transient-rc-step",
            title: r"Static solver, heat in time: 0.22 µm of silicon (148 W/(m·K), $\rho c_p$ = 1.66 MJ/(m³·K)) heated on 2 µm of oxide (1.38 W/(m·K), its heat capacity set to 1 J/(m³·K)) over a sink, on a grid graded from 10 to 100 nm, BDF2 steps of τ/400; the silicon's 1/e rise time against the lumped $\tau = (d_\mathrm{ox}/k_\mathrm{ox})\,\rho c_{p,\mathrm{Si}}\, t_\mathrm{Si}$ = 0.528 µs (relative error shown)",
            tier: Tier::Analytic,
            source: r"a lumped thermal RC, exact as the silicon's own resistance (Biot number $t_\mathrm{Si}k_\mathrm{ox}/d_\mathrm{ox}k_\mathrm{Si}$ ≈ 1e-3) and the oxide's capacity vanish: $T(t) = T_\infty(1 - e^{-t/\tau})$",
            run: || {
                let (e, _, _) = lumped_rc();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-3,
                    error: e,
                }
            },
        },
        Case {
            id: "static/transient-rc-cutoff",
            title: r"Static solver, heat in time: the same RC; the frequency where the silicon's small-signal response to the heater's power falls to $1/\sqrt 2$, from $(K + i\omega M)\hat T = \hat Q$, against $1/2\pi\tau$ = 301 kHz (relative error shown)",
            tier: Tier::Analytic,
            source: r"a single pole: $\hat T/\hat T_0 = 1/(1 + i\omega\tau)$",
            run: || {
                let (_, e, _) = lumped_rc();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-3,
                    error: e,
                }
            },
        },
        Case {
            id: "static/parallel-plate",
            title: r"Static solver, electrostatics: a parallel-plate capacitor 1 µm wide with 0.15 µm of $\varepsilon$ = 3.9 under 0.05 µm of $\varepsilon$ = 7.5, the sides mirrors, on a grid graded from 5 to 50 nm; its capacitance matrix against $\varepsilon_0 W / (d_1/\varepsilon_1 + d_2/\varepsilon_2)$ (largest relative error shown)",
            tier: Tier::Analytic,
            source: r"the field is uniform in each dielectric, $D$ continuous across them: capacitors in series",
            run: || {
                let e = parallel_plate();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-12,
                    error: e,
                }
            },
        },
        Case {
            id: "static/coaxial",
            title: r"Static solver, electrostatics: a coaxial line, radii 0.3 and 1 µm, $\varepsilon$ = 3.9, on 384 × 384 cells of 6.25 nm; its capacitance against $2\pi\varepsilon_0\varepsilon / \ln(b/a)$ (relative error shown)",
            tier: Tier::Analytic,
            source: r"Gauss's law in a cylinder; the conductors' curved surfaces met where they cross the lines between centres (G. H. Shortley, R. Weller, J. Appl. Phys. 9, 334 (1938), doi:10.1063/1.1710426)",
            run: || {
                let e = coaxial(384);
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 3e-5,
                    error: e,
                }
            },
        },
        Case {
            id: "static/coaxial-order",
            title: r"Static solver, electrostatics: the same coaxial line on 96, 192 and 384 cells across; the order of the capacitance's error from 192 to 384 (shown)",
            tier: Tier::Analytic,
            source: r"Shortley and Weller's boundaries keep the scheme's second order at curved conductors, where staircasing gives first",
            run: || {
                let p = order(coaxial(192), coaxial(384), 2.0);
                Outcome {
                    measured: p,
                    expected: 2.0,
                    tolerance: 0.1,
                    error: (p - 2.0).abs(),
                }
            },
        },
        Case {
            id: "static/line-source-images",
            title: r"Static solver, heat: a line source (1 W/m, a disk of radius 0.1 µm) 1 µm above an isothermal plane, $k$ = 1.4 W/(m·K), on 400 × 200 cells of 10 nm; every temperature farther than 0.2 µm from it against the images' (largest error shown, K)",
            tier: Tier::Analytic,
            source: r"the method of images: $T = \frac{P}{2\pi k}\ln(r'/r)$, $r'$ the distance to the image below the plane; outside a uniform disk its field is the line's",
            run: || {
                let e = line_source(400);
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 2e-5,
                    error: e,
                }
            },
        },
        Case {
            id: "static/line-source-order",
            title: r"Static solver, heat: the same line source on 100 × 50, 200 × 100 and 400 × 200 cells; the order of the largest error from 200 to 400 (shown)",
            tier: Tier::Analytic,
            source: r"second order, the source spread over the cells by the disk's exact fill",
            run: || {
                let p = order(line_source(200), line_source(400), 2.0);
                Outcome {
                    measured: p,
                    expected: 2.0,
                    tolerance: 0.15,
                    error: (p - 2.0).abs(),
                }
            },
        },
        Case {
            id: "static/coplanar-wen",
            title: r"Static solver, electrostatics: a coplanar waveguide of zero-thickness strips (signal 2 µm wide, slots 0.6 µm) on a semi-infinite substrate ($\varepsilon$ = 11.7), the window ±200 µm at 0 V, graded from 2 nm at the strips' edges; the signal's capacitance against conformal mapping (relative error shown)",
            tier: Tier::Analytic,
            source: r"C. P. Wen, IEEE Trans. Microw. Theory Tech. 17, 1087 (1969), doi:10.1109/TMTT.1969.1127105: $C = 2\varepsilon_0(\varepsilon_r + 1) K(k)/K(k')$, $k = a/b$, the limit of G. Ghione, C. Naldi, Electron. Lett. 20, 179 (1984), doi:10.1049/el:19840120, Eqs. 1–2 for a thick substrate",
            run: || {
                let (e, _) = coplanar(0.001, 200.0);
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-3,
                    error: e,
                }
            },
        },
        Case {
            id: "static/coplanar-edges-order",
            title: r"Static solver, electrostatics: the same coplanar waveguide graded from 4 and 1 nm at the strips' edges; the order of the capacitance's error in the smallest step (shown)",
            tier: Tier::Analytic,
            source: r"the field is singular as $r^{-1/2}$ at a thin strip's edge; grading toward the edges is what finite differences need there (ROADMAP's decision 2: finite elements only if they don't converge)",
            run: || {
                let p = order(coplanar(0.004, 200.0).0, coplanar(0.001, 200.0).0, 4.0);
                Outcome {
                    measured: p,
                    expected: 1.0,
                    tolerance: 0.25,
                    error: (p - 1.0).abs(),
                }
            },
        },
        Case {
            id: "static/joule-resistor",
            title: r"Static solver, current: a bar 2 µm long and 0.22 µm thick of $10^4$ S/m between contacts at 0 and 1 V, on a grid graded from 10 to 100 nm; its Joule heat against $V^2 \sigma t / L$ (relative error shown)",
            tier: Tier::Analytic,
            source: r"Ohm's law: a uniform bar's conductance per unit length $\sigma t / L$",
            run: || {
                let (e, _) = resistor();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-12,
                    error: e,
                }
            },
        },
        Case {
            id: "static/joule-balance",
            title: r"Static solver, current then heat: the same bar's Joule heat conducted to a substrate at 0 K; the heat through the substrate against the Joule power (relative error shown)",
            tier: Tier::Analytic,
            source: r"energy conservation: every face's $g\Delta V^2$ heats the cells either side, and finite volumes conserve it",
            run: || {
                let (_, e) = resistor();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-12,
                    error: e,
                }
            },
        },
        Case {
            id: "static/jacques-tin-heater",
            title: r"Static solver, heat, published: Jacques et al.'s TiN heater (2.0 µm wide, 2 µm above a 500 × 220 nm silicon strip on 2 µm of buried oxide, Table 3's conductivities: Si film 90, substrate 148, TiN 28, oxide 1.38 W/(m·K); a 40 × 18 µm region, its bottom held, 10 W/(m²·K) to the air), on a grid graded from 20 nm to 500 nm (0.1 % from a grid twice as fine); the strip's mean rise per watt, into Pπ for 320 µm at 1550 nm through ∂n_eff/∂n_Si from the full-vector mode solver (10 nm grid) and the paper's dn/dT of 1.8e-4/K. The paper withholds the TiN's thickness and leaves open whether its 2 µm are from the strip's top or the oxide's: the four variants (100 or 200 nm, either reading) give 24.5 to 26.3 mW, their midpoint shown (mW)",
            tier: Tier::Published,
            source: r"M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456: the simulated Pπ of the 2.0 µm TiN design, 23.9 mW (Section 2.2, Fig. 1(e)); 10 % for what the paper doesn't state",
            run: || {
                let (pi, _, _) = jacques_tin_heater();
                let (low, high) = pi
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &p| {
                        (a.min(p), b.max(p))
                    });
                let measured = 0.5 * (low + high);
                Outcome {
                    measured,
                    expected: 23.9,
                    tolerance: 0.1 * 23.9,
                    error: (measured - 23.9).abs(),
                }
            },
        },
        Case {
            id: "static/jacques-tin-heater-time",
            title: r"Static solver, heat in time, published: the same TiN heater with Table 3's heat capacities (Si 2330 kg/m³ and 711 J/(kg·K), TiN 5240 and 598, oxide 2203 and 709), turned on from the ambient, on the same grid (0.1 % from one twice as fine), BDF2 steps of 0.2 µs (3e-5 from steps half as long); the strip's mean 1/e rise time (the fall's is the same, the problem being linear). The four variants give 8.75 to 10.17 µs, their midpoint shown (µs)",
            tier: Tier::Published,
            source: r"M. Jacques et al., Opt. Express 27, 10456 (2019), doi:10.1364/OE.27.010456: the TiN design's simulated limiting 1/e time constant, ~10.2 µs (Section 2.2, Fig. 1(c)); 10 % for what the paper doesn't state",
            run: || {
                let times = jacques_tin_heater_times(
                    &Grading::new(0.5, 0.02, 1.2).expect("a grading"),
                    0.2e-6,
                );
                let (low, high) = times
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &t| {
                        (a.min(t), b.max(t))
                    });
                let measured = 0.5 * (low + high) * 1e6;
                Outcome {
                    measured,
                    expected: 10.2,
                    tolerance: 0.1 * 10.2,
                    error: (measured - 10.2).abs(),
                }
            },
        },
        Case {
            id: "static/soi-heater-balance",
            title: r"Static solver, heat: the 220 nm SOI stack cut across a 500 nm strip, a 10 W/m heater 2 µm above it, the substrate 0 K 10 µm down, a convective top, on a grid graded from 20 nm to 1 µm; the heat in against what leaves (relative imbalance shown)",
            tier: Tier::Analytic,
            source: r"energy conservation: the free cells' balances add up to the window's",
            run: || {
                let (e, _) = soi_heater();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-12,
                    error: e,
                }
            },
        },
    ]
}

/// The order of a problem's error, for the tests: `f` at n and 2n.
#[cfg(test)]
pub(crate) fn order_of(f: impl Fn(usize) -> f64, n: usize) -> f64 {
    order(f(n), f(2 * n), 2.0)
}
