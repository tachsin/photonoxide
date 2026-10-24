//! The validation cases of the travelling-wave electrodes ([`super::line`]): the coplanar
//! waveguide's current against Holloway and Kuester's closed form, its effective permittivity on
//! a finite substrate against Ghione and Naldi's, a filling factor against two dielectrics in
//! series, the response's two limits, and the published lines (Kharel et al. 2021's regular
//! electrodes, Zhang et al. 2022's line and its bandwidth).
//!
//! The thin-film lines ([`Cpw`]): half of a ground–signal–ground line, a mirror through the
//! signal's centre (x = 0), the ground to its outer edge and the window beyond it; x-cut lithium
//! niobate (c across) on a buried oxide over the silicon substrate, a rib in the gap, the metal
//! on the slab, an oxide cladding and air above, every side of the window insulated.

use super::line::{DB_PER_NEPER, Drive, Metal, QuasiTem, Wave, falls_to, response};
use super::{Boundary, Conductor, Electrostatic, Features, Grading, Grid, Section, Side, Tensor};
use crate::geometry::{Point, Polygon, Shape};
use crate::statics::elliptic_k;
use crate::units::Length;
use crate::validation::{Case, Outcome, Tier};
use num_complex::Complex64 as c64;

const OK: &str = "a valid model";

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

fn rect(x: [f64; 2], y: [f64; 2]) -> Shape {
    Shape::rect(
        Point::um(0.5 * (x[0] + x[1]), 0.5 * (y[0] + y[1])),
        Length::um(x[1] - x[0]),
        Length::um(y[1] - y[0]),
    )
    .expect(OK)
}

// ---------------------------------------------------------------------------------------------
// The coplanar waveguide of zero-thickness strips
// ---------------------------------------------------------------------------------------------

/// Half a coplanar waveguide of zero-thickness strips at y = 0: the signal from the mirror at
/// x = 0 to `a`, the ground from `b` to the window's edge `reach` (µm); a substrate of `eps`
/// `h` thick under the strips (air below it and above them), the window's far sides at 0 V,
/// graded from `smallest` at the strips' edges by `growth` to 5 µm.
pub(crate) fn coplanar(
    a: f64,
    b: f64,
    eps: f64,
    h: f64,
    reach: f64,
    smallest: f64,
    growth: f64,
) -> (QuasiTem, usize) {
    let mut section = Section::new("air");
    section.layer(-h, 0.0, "substrate").expect(OK);
    let mut features = section.features().clone();
    features.strip([0.0, a], 0.0).strip([b, reach], 0.0);
    let grid = Grid::fitted(
        [0.0, reach],
        [-reach, reach],
        &Grading::new(5.0, smallest, growth).expect(OK),
        &features,
    )
    .expect(OK);
    let cells = grid.cells();
    let mut problem = Electrostatic::new(section, grid);
    problem
        .permittivity("air", Tensor::isotropic(1.0))
        .permittivity("substrate", Tensor::isotropic(eps))
        .boundary(Side::Right, Boundary::Fixed(0.0))
        .boundary(Side::Bottom, Boundary::Fixed(0.0))
        .boundary(Side::Top, Boundary::Fixed(0.0))
        .conductor(Conductor::strip([0.0, a], 0.0, 1.0))
        .conductor(Conductor::strip([b, reach], 0.0, 0.0));
    (QuasiTem::mirrored(&problem, 0).expect(OK), cells)
}

/// Holloway and Kuester's Eq. 12 for the conformal-mapping current of a coplanar waveguide (a
/// the signal's half width, b the ground's inner edge, Δ the stopping distance, all in m):
/// ∫ (J/I)² dl = b²/(4K²(k)(b² − a²)) {(1/a) ln[(2a/Δ − 1)(b − a + Δ)/(b + a + Δ)]
/// + (1/b) ln[(2b/Δ + 1)(b − a + Δ)/(b + a − Δ)]}, 1/m.
pub(crate) fn holloway_kuester_integral(a: f64, b: f64, delta: f64) -> f64 {
    let k = elliptic_k(a / b);
    b * b / (4.0 * k * k * (b * b - a * a))
        * ((((2.0 * a / delta - 1.0) * (b - a + delta) / (b + a + delta)).ln()) / a
            + (((2.0 * b / delta + 1.0) * (b - a + delta) / (b + a - delta)).ln()) / b)
}

/// The coplanar waveguide of Holloway and Kuester's Fig. 3 (a = 5, b = 25 µm) with Δ = 4 nm:
/// the static solver's current integral against their Eq. 12 (relative error), graded from
/// `smallest` µm.
pub(crate) fn current_against_closed_form(smallest: f64) -> f64 {
    let (a, b, delta) = (5.0, 25.0, 0.004);
    let (line, _) = coplanar(a, b, 12.9, 2000.0, 800.0, smallest, 1.08);
    let numeric = line.current_integral(delta * 1e-6).expect(OK);
    let exact = holloway_kuester_integral(a * 1e-6, b * 1e-6, delta * 1e-6);
    (numeric - exact) / exact
}

/// Ghione and Naldi's Eq. 2: a coplanar waveguide's effective permittivity on a substrate of
/// `eps`, `h` thick, a the signal's half width and b the ground's inner edge.
pub(crate) fn ghione_naldi_permittivity(a: f64, b: f64, eps: f64, h: f64) -> f64 {
    let k = a / b;
    let kp = (1.0 - k * k).sqrt();
    let k1 = (std::f64::consts::PI * a / (2.0 * h)).sinh()
        / (std::f64::consts::PI * b / (2.0 * h)).sinh();
    let k1p = (1.0 - k1 * k1).sqrt();
    1.0 + 0.5 * (eps - 1.0) * elliptic_k(kp) * elliptic_k(k1) / (elliptic_k(k) * elliptic_k(k1p))
}

/// The effective permittivity and impedance of a coplanar waveguide (a = 10, b = 20 µm, k =
/// 0.5) on a substrate of ε = 10 (their Figs. 1–2) `h` thick, against Ghione and Naldi's Eqs.
/// 2 and 1: the relative errors.
pub(crate) fn ghione_naldi(h: f64, smallest: f64) -> (f64, f64) {
    let (a, b, eps) = (10.0, 20.0, 10.0);
    let (line, _) = coplanar(a, b, eps, h, 1000.0, smallest, 1.1);
    let e = ghione_naldi_permittivity(a, b, eps, h);
    let k = a / b;
    let kp = (1.0 - k * k).sqrt();
    let z = 30.0 * std::f64::consts::PI / e.sqrt() * elliptic_k(kp) / elliptic_k(k);
    (
        (line.effective_permittivity() - e) / e,
        (line.impedance() - z) / z,
    )
}

// ---------------------------------------------------------------------------------------------
// Filling factors
// ---------------------------------------------------------------------------------------------

/// Two dielectrics in series between plates (ε 3.9 and 11.7, 0.3 and 0.5 µm), the sides a
/// mirror: the lower one's share of the energy against (d₁/ε₁)/(d₁/ε₁ + d₂/ε₂), the share of
/// the series capacitance's voltage drop it holds (relative error).
pub(crate) fn plates_share() -> f64 {
    let (d1, e1, d2, e2) = (0.3, 3.9, 0.5, 11.7);
    let mut section = Section::new("lower");
    section.layer(d1, d1 + d2, "upper").expect(OK);
    section.layer(-0.1, 0.0, "lower").expect(OK);
    section.layer(d1 + d2, d1 + d2 + 0.1, "upper").expect(OK);
    let mut features = section.features().clone();
    features.line_y(d1);
    let grid = Grid::fitted(
        [0.0, 1.0],
        [-0.1, d1 + d2 + 0.1],
        &Grading::new(0.05, 0.01, 1.2).expect(OK),
        &features,
    )
    .expect(OK);
    let mut problem = Electrostatic::new(section, grid);
    problem
        .permittivity("lower", Tensor::isotropic(e1))
        .permittivity("upper", Tensor::isotropic(e2))
        .conductor(Conductor::shape(rect([-1.0, 2.0], [-0.1, 0.0]), 0.0))
        .conductor(Conductor::shape(
            rect([-1.0, 2.0], [d1 + d2, d1 + d2 + 0.1]),
            1.0,
        ));
    let field = problem.solve().expect(OK);
    let shares = problem.energy_shares(&field).expect(OK);
    let lower = shares.iter().find(|(m, _)| m == "lower").expect(OK).1;
    let exact = (d1 / e1) / (d1 / e1 + d2 / e2);
    (lower - exact) / exact
}

// ---------------------------------------------------------------------------------------------
// The response's limits
// ---------------------------------------------------------------------------------------------

/// A lossless matched line 1 cm long whose RF index exceeds the light's group index by 0.1:
/// the response's −3 dB point as Ghione's U = ω(n_m − n_o)L/(2c), where |sin U / U| = 1/√2.
pub(crate) fn walk_off_point() -> f64 {
    let (nm, no, l) = (2.3, 2.2, 0.01);
    let drive = Drive::new(50.0, no, l).expect(OK);
    let db = |f: f64| -> crate::Result<f64> {
        let beta = 2.0 * std::f64::consts::PI * f * nm / crate::units::SPEED_OF_LIGHT;
        Ok(20.0 * response(c64::new(0.0, beta), c64::new(50.0, 0.0), f, &drive).log10())
    };
    let f = falls_to(
        db,
        20.0 * std::f64::consts::FRAC_1_SQRT_2.log10(),
        1e8,
        1e12,
    )
    .expect(OK)
    .expect("a fall");
    std::f64::consts::PI * f * (nm - no) * l / crate::units::SPEED_OF_LIGHT
}

/// A velocity- and impedance-matched line whose loss grows as √f: the line's loss (dB) at the
/// response's −3 dB (and −6 dB) point, where (1 − e^(−αL))/(αL) = 1/√2 (1/2).
pub(crate) fn loss_point(level_db: f64) -> f64 {
    let (n, l) = (2.2, 0.01);
    let drive = Drive::new(50.0, n, l).expect(OK);
    // 1 dB/cm at 1 GHz, growing as √f
    let alpha = |f: f64| (f / 1e9).sqrt() * 100.0 / DB_PER_NEPER;
    let db = |f: f64| -> crate::Result<f64> {
        let beta = 2.0 * std::f64::consts::PI * f * n / crate::units::SPEED_OF_LIGHT;
        Ok(20.0 * response(c64::new(alpha(f), beta), c64::new(50.0, 0.0), f, &drive).log10())
    };
    let f = falls_to(db, level_db, 1e7, 1e13)
        .expect(OK)
        .expect("a fall");
    alpha(f) * l * DB_PER_NEPER
}

// ---------------------------------------------------------------------------------------------
// Thin-film lines
// ---------------------------------------------------------------------------------------------

/// Half a ground–signal–ground line on thin-film lithium niobate (µm). See the module's docs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cpw {
    /// Half the signal's width.
    pub signal: f64,
    /// The gap between the signal and the ground.
    pub gap: f64,
    /// The ground's width.
    pub ground: f64,
    /// The metal's thickness.
    pub metal: f64,
    /// The film's thickness.
    pub film: f64,
    /// The slab left beside the rib (the film's thickness where there is no rib).
    pub slab: f64,
    /// The rib's top width (0: no rib).
    pub top: f64,
    /// The rib's sidewall from the plane, degrees.
    pub angle_deg: f64,
    /// The buried oxide.
    pub buried: f64,
    /// The silicon (or quartz) substrate.
    pub substrate: f64,
    /// The substrate's relative permittivity.
    pub substrate_eps: f64,
    /// The cladding over the slab.
    pub clad: f64,
    /// The cladding's relative permittivity.
    pub clad_eps: f64,
}

/// Thermal oxide's relative permittivity.
const OXIDE_EPS: f64 = 3.9;
/// Silicon's.
pub(crate) const SILICON_EPS: f64 = 11.7;

impl Cpw {
    fn half_width(&self, y: f64) -> f64 {
        0.5 * self.top + (self.film - y) / self.angle_deg.to_radians().tan()
    }

    /// The electrostatic problem on a grid graded from `smallest` by 15% a cell to 20 µm.
    pub(crate) fn electrostatic(&self, smallest: f64) -> Electrostatic {
        let (perpendicular, along_c) =
            crate::mode::pockels::checks::Coefficients::Clamped.static_permittivity();
        let outer = self.signal + self.gap + self.ground;
        let right = outer + 300.0;
        let (bottom, top) = (
            -self.buried - self.substrate,
            self.slab + self.clad.max(self.metal) + 300.0,
        );
        let mut section = Section::new("air");
        section
            .layer(bottom, -self.buried, "substrate")
            .and_then(|s| s.layer(-self.buried, 0.0, "box"))
            .expect(OK);
        if self.clad > 0.0 {
            section
                .layer(self.slab, self.slab + self.clad, "clad")
                .expect(OK);
        }
        section.layer(0.0, self.slab, "LN").expect(OK);
        let centre = self.signal + 0.5 * self.gap;
        if self.top > 0.0 && self.film > self.slab {
            let (b, t) = (self.half_width(self.slab), 0.5 * self.top);
            section.draw(
                Shape::Polygon(
                    Polygon::new(vec![
                        Point::um(centre - b, self.slab),
                        Point::um(centre + b, self.slab),
                        Point::um(centre + t, self.film),
                        Point::um(centre - t, self.film),
                    ])
                    .expect(OK),
                ),
                "LN",
            );
        }
        let y = [self.slab, self.slab + self.metal];
        let signal = rect([-1.0, self.signal], y);
        let ground = rect([self.signal + self.gap, outer], y);
        let mut features: Features = section.features().clone();
        features.shape(&signal).shape(&ground).line_x(0.0);
        let grid = Grid::fitted(
            [0.0, right],
            [bottom, top],
            &Grading::new(20.0, smallest, 1.15).expect(OK),
            &features,
        )
        .expect(OK);
        let mut problem = Electrostatic::new(section, grid);
        problem
            .permittivity("air", Tensor::isotropic(1.0))
            .permittivity("substrate", Tensor::isotropic(self.substrate_eps))
            .permittivity("box", Tensor::isotropic(OXIDE_EPS))
            .permittivity("clad", Tensor::isotropic(self.clad_eps))
            // x-cut, c across: across ε₃₃, up ε₁₁
            .permittivity("LN", Tensor::diagonal(along_c, perpendicular))
            .conductor(Conductor::shape(signal, 1.0))
            .conductor(Conductor::shape(ground, 0.0));
        problem
    }

    /// The line on a grid graded from `smallest`.
    pub(crate) fn line(&self, smallest: f64) -> QuasiTem {
        QuasiTem::mirrored(&self.electrostatic(smallest), 0).expect(OK)
    }
}

/// Kharel et al. 2021's regular electrodes on silicon (their Table 1): a 30 µm signal, a gap of
/// `gap`, 800 nm of gold, on their fabricated film (600 nm, 350 nm etched to a rib 2 µm wide,
/// the sidewall at 70°: not given) on 2 µm of oxide over 500 µm of silicon, 1 µm of oxide over
/// the slab; the ground 100 µm wide (not given).
pub(crate) fn kharel_regular(gap: f64) -> Cpw {
    Cpw {
        signal: 15.0,
        gap,
        ground: 100.0,
        metal: 0.8,
        film: 0.6,
        slab: 0.25,
        top: 2.0,
        angle_deg: 70.0,
        buried: 2.0,
        substrate: 500.0,
        substrate_eps: SILICON_EPS,
        clad: 1.0,
        clad_eps: OXIDE_EPS,
    }
}

/// Zhang et al. 2022's line: a 20 µm signal, a 5 µm gap, 0.83 µm of metal (800 nm of copper
/// under 30 nm of gold), on their film (600 nm, 300 nm etched to a rib 1.2 µm wide on top, the
/// sidewall at 70°: not given) on 2 µm of thermal oxide over 500 µm of silicon, 0.6 µm of PECVD
/// oxide (ε 5.5, their model's); the ground 100 µm wide (not given).
pub(crate) fn zhang_2022() -> Cpw {
    Cpw {
        signal: 10.0,
        gap: 5.0,
        ground: 100.0,
        metal: 0.83,
        film: 0.6,
        slab: 0.3,
        top: 1.2,
        angle_deg: 70.0,
        buried: 2.0,
        substrate: 500.0,
        substrate_eps: SILICON_EPS,
        clad: 0.6,
        clad_eps: 5.5,
    }
}

/// Gold's conductivity, S/m: bulk, at room temperature (the papers don't give theirs).
pub(crate) const GOLD: f64 = 4.1e7;
/// Copper's, S/m.
pub(crate) const COPPER: f64 = 5.8e7;

/// Zhang et al.'s loss tangents: silicon 0.013, lithium niobate 0.008, the deposited oxide
/// 0.006 (their Section 3.B; the thermal oxide's isn't given, taken as the deposited one's).
pub(crate) const ZHANG_TANGENTS: [(&str, f64); 4] = [
    ("substrate", 0.013),
    ("LN", 0.008),
    ("clad", 0.006),
    ("box", 0.006),
];

/// The wave on `line` at `ghz` on `metal`, with `tangents`.
pub(crate) fn wave(line: &QuasiTem, ghz: f64, metal: &Metal, tangents: &[(&str, f64)]) -> Wave {
    line.wave(ghz * 1e9, metal, tangents).expect(OK)
}

/// The static grids of the published lines: graded from 8, 4 and 2 nm.
const STEPS: [f64; 3] = [0.008, 0.004, 0.002];

/// `quantity` of `cpw`'s line on [`STEPS`], gated by the grid study at `tolerance` (relative):
/// the finest grid's values.
fn gated(cpw: &Cpw, quantity: impl Fn(&QuasiTem) -> Vec<f64>, tolerance: f64) -> Vec<f64> {
    let values: Vec<Vec<f64>> = STEPS.iter().map(|&h| quantity(&cpw.line(h))).collect();
    (0..values[0].len())
        .map(|k| {
            let mut study = crate::convergence::Study::new("a line's number");
            for (h, v) in STEPS.iter().zip(&values) {
                study = study.on(*h, v[k]).expect(OK);
            }
            let v = values[2][k];
            study
                .check(tolerance * v.abs())
                .expect(OK)
                .gated()
                .expect("a converged grid")
        })
        .collect()
}

/// Kharel et al.'s Table 1 for their regular electrodes on silicon at gaps of 5, 7 and 10 µm:
/// Z (Ω), the RF index at 50 GHz, and the conductor loss (dB cm⁻¹ GHz⁻½).
pub(crate) const KHAREL_TABLE: [(f64, f64, f64, f64); 3] = [
    (5.0, 39.0, 2.32, 0.75),
    (7.0, 45.0, 2.25, 0.64),
    (10.0, 48.0, 2.24, 0.57),
];

/// photonoxide's numbers for [`KHAREL_TABLE`]'s lines at 50 GHz, each gated at 0.5%: Re Z₀,
/// the RF index, and the conductor loss over √(50 GHz).
pub(crate) fn kharel_numbers() -> &'static [[f64; 3]; 3] {
    static NUMBERS: std::sync::OnceLock<[[f64; 3]; 3]> = std::sync::OnceLock::new();
    NUMBERS.get_or_init(|| {
        let gold = Metal::new(GOLD, 0.8).expect(OK);
        KHAREL_TABLE.map(|(gap, ..)| {
            let v = gated(
                &kharel_regular(gap),
                |line| {
                    let w = wave(line, 50.0, &gold, &[]);
                    vec![
                        w.impedance.re,
                        w.index(),
                        w.conductor * DB_PER_NEPER / 100.0 / 50f64.sqrt(),
                    ]
                },
                5e-3,
            );
            [v[0], v[1], v[2]]
        })
    })
}

/// The largest relative difference from Kharel et al.'s Table 1 of column `k` (0: Z, 1: the
/// index, 2: the loss), with its sign.
fn kharel_worst(k: usize) -> f64 {
    KHAREL_TABLE
        .iter()
        .zip(kharel_numbers())
        .map(|(row, v)| {
            let theirs = [row.1, row.2, row.3][k];
            (v[k] - theirs) / theirs
        })
        .fold(0.0f64, |m, e| if e.abs() > m.abs() { e } else { m })
}

/// Two plates 10 µm wide and 1 µm apart across the whole window, its sides a mirror (plates
/// without end): Wheeler's surface integral against 2/w, the current uniform on the plates'
/// inner faces (relative error).
pub(crate) fn wheeler_plates() -> f64 {
    let w = 10.0;
    let section = Section::new("air");
    let lower = rect([-1.0, w + 1.0], [-0.5, 0.0]);
    let upper = rect([-1.0, w + 1.0], [1.0, 1.5]);
    let mut features = section.features().clone();
    features.shape(&lower).shape(&upper);
    let grid = Grid::fitted(
        [0.0, w],
        [-0.5, 1.5],
        &Grading::new(0.1, 0.02, 1.2).expect(OK),
        &features,
    )
    .expect(OK);
    let mut problem = Electrostatic::new(section, grid);
    problem
        .permittivity("air", Tensor::isotropic(1.0))
        .conductor(Conductor::shape(upper, 1.0))
        .conductor(Conductor::shape(lower, 0.0));
    let line = QuasiTem::new(&problem, 0).expect(OK);
    let exact = 2.0 / (w * 1e-6);
    (line.surface_integral().expect("thick plates") - exact) / exact
}

/// Kharel et al.'s regular electrode with a 5 µm gap at `ghz`: the conductor loss by
/// Wheeler's rule on the thick metal against Holloway and Kuester's (relative difference), on
/// the grid graded from 2 nm.
pub(crate) fn wheeler_against_holloway_kuester(ghz: f64) -> f64 {
    let gold = Metal::new(GOLD, 0.8).expect(OK);
    let line = kharel_regular(5.0).line(0.002);
    let wheeler = line.wheeler_loss(ghz * 1e9, &gold).expect(OK);
    let hk = wave(&line, ghz, &gold, &[]).conductor;
    (wheeler - hk) / hk
}

/// Zhang et al.'s optical group index (their Lumerical simulation, Fig. 2(a)).
pub(crate) const ZHANG_GROUP_INDEX: f64 = 2.26;

/// Zhang et al.'s Vπ·L at 1 GHz, V·cm (Section 2).
const ZHANG_VPI_L_1GHZ: f64 = 2.67;

/// photonoxide's numbers for Zhang et al.'s line, each gated at 0.5%: the RF index at 100 GHz,
/// Re Z₀ and the loss (dB/cm) at 250 GHz, and the electro-optic response at 250 GHz against
/// 1 GHz (m(250)/m(1)) for 5.8 and 10.8 mm.
pub(crate) fn zhang_numbers() -> &'static [f64; 5] {
    static NUMBERS: std::sync::OnceLock<[f64; 5]> = std::sync::OnceLock::new();
    NUMBERS.get_or_init(|| {
        let copper = Metal::new(COPPER, 0.83).expect(OK);
        let v = gated(
            &zhang_2022(),
            |line| {
                let w1 = wave(line, 1.0, &copper, &ZHANG_TANGENTS);
                let w100 = wave(line, 100.0, &copper, &ZHANG_TANGENTS);
                let w250 = wave(line, 250.0, &copper, &ZHANG_TANGENTS);
                let ratio = |mm: f64| {
                    let d = Drive::new(50.0, ZHANG_GROUP_INDEX, mm * 1e-3).expect(OK);
                    response(w250.gamma, w250.impedance, 250e9, &d)
                        / response(w1.gamma, w1.impedance, 1e9, &d)
                };
                vec![
                    w100.index(),
                    w250.impedance.re,
                    w250.db_per_cm(),
                    ratio(5.8),
                    ratio(10.8),
                ]
            },
            5e-3,
        );
        [v[0], v[1], v[2], v[3], v[4]]
    })
}

/// The RF Vπ at 250 GHz of Zhang et al.'s modulator `mm` long: their Vπ·L at 1 GHz over the
/// length, divided by the response at 250 GHz against 1 GHz (`ratio`), V.
pub(crate) fn zhang_rf_vpi(mm: f64, ratio: f64) -> f64 {
    ZHANG_VPI_L_1GHZ / (0.1 * mm) / ratio
}

/// The 3 dB bandwidth (GHz, referred to 1 GHz) of Zhang et al.'s modulator `mm` long on the
/// quasi-TEM line graded from `smallest`.
#[cfg(test)]
pub(crate) fn zhang_bandwidth(mm: f64, smallest: f64) -> f64 {
    let copper = Metal::new(COPPER, 0.83).expect(OK);
    let line = zhang_2022().line(smallest);
    let d = Drive::new(50.0, ZHANG_GROUP_INDEX, mm * 1e-3).expect(OK);
    let m = |f: f64| {
        let w = line.wave(f, &copper, &ZHANG_TANGENTS).expect(OK);
        response(w.gamma, w.impedance, f, &d)
    };
    let r = m(1e9);
    falls_to(|f| Ok(20.0 * (m(f) / r).log10()), -3.0, 1e9, 1e12)
        .expect(OK)
        .map_or(f64::INFINITY, |f| f / 1e9)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "line/holloway-kuester-current",
            title: r"Travelling-wave electrodes, conductor loss: the current integral $\int (J/I)^2 dl$ of a coplanar waveguide of zero-thickness strips ($a$ = 5, $b$ = 25 µm, Holloway and Kuester's Fig. 3), stopped $\Delta$ = 4 nm short of each edge, from the static solver's vacuum charge graded from 1 nm (each edge's $r^{-1/2}$ fitted within a twentieth of the gap and integrated from $\Delta$) against the conformal-mapping current's closed form (relative error shown)",
            tier: Tier::Analytic,
            source: r"C. L. Holloway, E. F. Kuester, IEEE Trans. Microw. Theory Tech. 43, 2695 (1995), doi:10.1109/22.477846, Eqs. 9, 10 and 12",
            run: || outcome(current_against_closed_form(0.001), 0.0, 2e-3),
        },
        Case {
            id: "line/holloway-kuester-order",
            title: r"The same graded from 4, 2 and 1 nm: the order of the integral's error in the smallest step, from the last two (shown)",
            tier: Tier::Analytic,
            source: r"the strips' charge is singular as $r^{-1/2}$ at their edges, which the static solver meets at about first order in the edge's step (static/coplanar-edges-order)",
            run: || {
                let e = [0.002, 0.001].map(current_against_closed_form);
                outcome((e[0] / e[1]).abs().log2(), 1.0, 0.35)
            },
        },
        Case {
            id: "line/ghione-naldi-thick",
            title: r"Travelling-wave electrodes, quasi-TEM: a coplanar waveguide ($a$ = 10, $b$ = 20 µm) on a substrate of $\varepsilon_r$ = 10 as thick as $b$ (air below), the window ±1 mm at 0 V, graded from 4 nm: $\varepsilon_\mathrm{eff} = C/C_0$ against Ghione and Naldi's Eq. 2 (relative difference shown)",
            tier: Tier::Analytic,
            source: r"G. Ghione, C. Naldi, Electron. Lett. 20, 179 (1984), doi:10.1049/el:19840120, Eqs. 2–3: conformal mapping's partial capacitances, which their Fig. 2 checks against a spectral-domain variational analysis, its bounds close together at $h/b$ = 1: 1%",
            run: || outcome(ghione_naldi(20.0, 0.004).0, 0.0, 1e-2),
        },
        Case {
            id: "line/ghione-naldi-thin",
            title: r"The same on a substrate half as thick ($h/b$ = 0.5): $\varepsilon_\mathrm{eff}$ against Eq. 2 (relative difference shown)",
            tier: Tier::Analytic,
            source: r"Ghione and Naldi's Eq. 2, an approximation their Fig. 2 places near the upper of the variational bounds at $h/b$ = 0.5, the bounds there about 8% apart: 2.5%",
            run: || outcome(ghione_naldi(10.0, 0.004).0, 0.0, 2.5e-2),
        },
        Case {
            id: "line/ghione-naldi-impedance",
            title: r"The thick-substrate line's $Z_0 = 1/(c\sqrt{C C_0})$ against Eq. 1 with Eq. 2's $\varepsilon_\mathrm{eff}$ (relative difference shown)",
            tier: Tier::Analytic,
            source: r"Ghione and Naldi's Eq. 1, $Z_0 = 30\pi K(k')/(\sqrt{\varepsilon_\mathrm{eff}} K(k))$: $C_0$ is exact, so the difference is half Eq. 2's",
            run: || outcome(ghione_naldi(20.0, 0.004).1, 0.0, 5e-3),
        },
        Case {
            id: "line/wheeler-plates",
            title: r"Travelling-wave electrodes, conductor loss by Wheeler's incremental-inductance rule: two plates 10 µm wide, 1 µm apart, across a window whose sides are mirrors (plates without ends); the surface integral $\oint (J/I)^2 dl$ from the static solver's vacuum charge against $2/w$, the current uniform on the two inner faces (relative error shown)",
            tier: Tier::Analytic,
            source: r"H. A. Wheeler, Proc. IRE 30, 412 (1942), doi:10.1109/JRPROC.1942.232015, Eq. (18): $R = (R_s/\mu_0) \partial L/\partial n$ summed over the surfaces, which is $R_s \oint (J/I)^2 dl$ for a TEM line; $R = 2R_s/w$ for wide plates",
            run: || outcome(wheeler_plates(), 0.0, 1e-12),
        },
        Case {
            id: "line/wheeler-holloway-kuester",
            title: r"Conductor loss two ways: Kharel et al.'s regular electrode (5 µm gap, 800 nm of gold) at 300 GHz, where the metal is 5.6 skin depths thick, by Wheeler's rule on the thick metal against Holloway and Kuester's method on its strips, the static grid graded from 2 nm (relative difference shown)",
            tier: Tier::Analytic,
            source: r"Wheeler 1942, Eq. (18) (valid where the metal is at least twice its skin depth thick); C. L. Holloway, E. F. Kuester, IEEE Trans. Microw. Theory Tech. 43, 2695 (1995), doi:10.1109/22.477846, Eq. 7 and Table I. Their Sections I and III: the Leontovich surface impedance Wheeler's rule takes fails within a skin depth of an edge, where their stopping distance accounts for it; the two within 10%",
            run: || outcome(wheeler_against_holloway_kuester(300.0), 0.0, 0.1),
        },
        Case {
            id: "line/filling-plates",
            title: r"Travelling-wave electrodes, dielectric loss: two dielectrics in series between plates ($\varepsilon$ 3.9 and 11.7, 0.3 and 0.5 µm): the lower one's share of the electric energy, the filling factor its loss tangent takes, against $(d_1/\varepsilon_1)/(d_1/\varepsilon_1 + d_2/\varepsilon_2)$ (relative error shown)",
            tier: Tier::Analytic,
            source: r"the energy of capacitors in series divides as their elastances $d_i/\varepsilon_i$; the scheme is exact for interfaces on the grid's lines",
            run: || outcome(plates_share(), 0.0, 1e-12),
        },
        Case {
            id: "line/response-walk-off",
            title: r"Travelling-wave electrodes, the electro-optic response: a lossless matched line 1 cm long, its RF index 0.1 above the light's group index; the response's $-3$ dB point (20 log₁₀ $m$) as $U = \omega(n_m - n_o)L/(2c)$ (shown)",
            tier: Tier::Analytic,
            source: r"G. Ghione, Semiconductor Devices for High-Speed Optoelectronics, Cambridge University Press (2009), doi:10.1017/CBO9780511635595, Section 6.5.2: $m_1 = \lvert\sin U/U\rvert$, $U_\mathrm{3dB,el}$ = 1.39",
            run: || outcome(walk_off_point(), 1.39, 5e-3),
        },
        Case {
            id: "line/response-loss-3db",
            title: r"Travelling-wave electrodes, the electro-optic response: a velocity- and impedance-matched line whose loss grows as $\sqrt f$; the line's loss (dB) where the response has fallen by 3 dB (shown)",
            tier: Tier::Analytic,
            source: r"Ghione 2009, Section 6.5.2: $m_2 = (1 - e^{-\alpha L})/(\alpha L)$, $\alpha L$ = 0.736 Np, 6.393 dB, at the electrical 3 dB bandwidth (Y. Zhang et al., Photon. Res. 10, 2380 (2022), doi:10.1364/PRJ.468518: 6.41 dB)",
            run: || outcome(loss_point(-3.0), 6.393, 1e-2),
        },
        Case {
            id: "line/response-loss-6db",
            title: r"The same where the response has fallen by 6 dB (the line's loss in dB shown)",
            tier: Tier::Analytic,
            source: r"$(1 - e^{-\alpha L})/(\alpha L) = 1/2$; Zhang et al. 2022, Section 2: 13.8 dB",
            run: || outcome(loss_point(-6.0), 13.8, 5e-2),
        },
        Case {
            id: "line/kharel-2021-impedance",
            title: r"Published: Kharel et al. 2021's regular electrodes on silicon (a 30 µm signal; gaps of 5, 7 and 10 µm; 800 nm of gold on their fabricated film, a rib 2 µm wide etched 350 nm into 600 nm of x-cut lithium niobate under 1 µm of oxide, on 2 µm of oxide over 500 µm of silicon; the ground 100 µm wide and the sidewall at 70°, not given), quasi-TEM at 50 GHz, the static grids graded from 8, 4 and 2 nm and gated at 0.5%: Re $Z_0$ against their Table 1's 39, 45 and 48 Ω (the largest relative difference shown)",
            tier: Tier::Published,
            source: r"P. Kharel et al., Optica 8, 357 (2021), doi:10.1364/OPTICA.416155, Table 1 (Ansys HFSS, 3D; its erratum, Optica 8, 1218 (2021), doi:10.1364/OPTICA.440484, leaves it as it is). The tolerance: what the paper doesn't give (whether their simulation has the etch and the cladding, ±3%; the ground's width, ±1.5%) and their 7 µm row, 4–5% above the trend of the other two that conformal mapping gives (Ghione and Naldi's Eq. 1): 10%",
            run: || outcome(kharel_worst(0), 0.0, 0.10),
        },
        Case {
            id: "line/kharel-2021-index",
            title: r"The same lines' RF index at 50 GHz against their Table 1's 2.32, 2.25 and 2.24 (the largest relative difference shown)",
            tier: Tier::Published,
            source: r"Kharel et al. 2021, Table 1. The tolerance: the unstated etch and cladding (±2.5%) and their 5 µm row, 3% above the other two where the quasi-TEM index barely moves with the gap: 4%",
            run: || outcome(kharel_worst(1), 0.0, 0.04),
        },
        Case {
            id: "line/kharel-2021-loss",
            title: r"The same lines' conductor loss at 50 GHz by Holloway and Kuester's method (gold at 4.1 × 10⁷ S/m, not given; square edges) over $\sqrt{50\ \mathrm{GHz}}$, against their Table 1's 0.75, 0.64 and 0.57 dB cm⁻¹ GHz⁻½ (the largest relative difference shown)",
            tier: Tier::Published,
            source: r"Kharel et al. 2021, Table 1; their measured regular electrode (5 µm gap): 0.69 dB cm⁻¹ GHz⁻½. The tolerance: the gold's conductivity (±5%), the unstated geometry (±3%), and Holloway and Kuester's own 5–10% where gaps are narrow (their Section V): 12%",
            run: || outcome(kharel_worst(2), 0.0, 0.12),
        },
        Case {
            id: "line/zhang-2022-index",
            title: r"Published: Zhang et al. 2022's line (a 20 µm signal, 5 µm gaps, 0.83 µm of copper on their film, 0.6 µm of PECVD oxide of $\varepsilon$ 5.5 and air above, 2 µm of thermal oxide on 500 µm of silicon; the ground 100 µm wide and the sidewall at 70°, not given), quasi-TEM, the static grids graded from 8, 4 and 2 nm and gated at 0.5%: the RF index at 100 GHz (shown) against their simulated 2.27",
            tier: Tier::Published,
            source: r"Y. Zhang et al., Photon. Res. 10, 2380 (2022), doi:10.1364/PRJ.468518, Fig. 2(a) (Ansys HFSS, the 0.6 µm cladding); their measured index lies 0.02 and 0.04 above the design at 150 and 250 GHz: 0.04",
            run: || outcome(zhang_numbers()[0], 2.27, 0.04),
        },
        Case {
            id: "line/zhang-2022-impedance",
            title: r"The same line's Re $Z_0$ at 250 GHz (Ω, shown) against their measured 41.5 Ω",
            tier: Tier::Published,
            source: r"Zhang et al. 2022, Section 3.B and Fig. 2(b): 41.5 Ω extracted from the measured s-parameters at 250 GHz, 40.5 Ω simulated: 1.5 Ω",
            run: || outcome(zhang_numbers()[1], 41.5, 1.5),
        },
        Case {
            id: "line/zhang-2022-loss",
            title: r"The same line's loss at 250 GHz (dB/cm, shown; copper at 5.8 × 10⁷ S/m, not given; their loss tangents: silicon 0.013, lithium niobate 0.008, the deposited oxide 0.006, taken for the thermal oxide too) against their measured 1.3 dB/mm",
            tier: Tier::Published,
            source: r"Zhang et al. 2022, Section 3.B: 1.3 dB/mm at 250 GHz; their Fig. 3(b)'s loss extracted from the two lines reads about 16 dB/cm there, and their fit $0.42\sqrt f + 0.025 f$ gives 12.9: 3 dB/cm",
            run: || outcome(zhang_numbers()[2], 13.0, 3.0),
        },
        Case {
            id: "line/zhang-2022-rf-vpi-short",
            title: r"Their 5.8 mm modulator's RF $V_\pi$ at 250 GHz (V, shown): their $V_\pi L$ at 1 GHz, 2.67 V·cm, over the length, divided by the electro-optic response at 250 GHz against 1 GHz from the quasi-TEM line's $\gamma$ and $Z_0$ (Ghione's Eq. 6.31, 50 Ω at both ends, the light's group index 2.26)",
            tier: Tier::Published,
            source: r"Zhang et al. 2022, Fig. 5(b) and Section 3.C: 8.3 V measured at 250 GHz. The tolerance: their raw points' spread there (Fig. 5(b), about ±15%) and the line's loss (above): 20%",
            run: || outcome(zhang_rf_vpi(5.8, zhang_numbers()[3]), 8.3, 0.2 * 8.3),
        },
        Case {
            id: "line/zhang-2022-rf-vpi-long",
            title: r"The same for their 10.8 mm modulator (V, shown)",
            tier: Tier::Published,
            source: r"Zhang et al. 2022, Fig. 5(a): 7.3 V measured at 250 GHz; the same tolerance, 20%",
            run: || outcome(zhang_rf_vpi(10.8, zhang_numbers()[4]), 7.3, 0.2 * 7.3),
        },
    ]
}
