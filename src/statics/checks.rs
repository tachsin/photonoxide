//! The static solver's validation cases: closed forms for heat conduction, electrostatics and
//! current, and the order of the error on refined grids.

use std::f64::consts::PI;

use super::{
    Axis, Boundary, Discrete, Grading, Grid, Problem, Section, Side, Tensor, elliptic_k, spread,
};
use crate::electro::{Conductor, Current, EPSILON_0, Electrostatic};
use crate::geometry::{Point, Shape};
use crate::thermal::Heat;
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
