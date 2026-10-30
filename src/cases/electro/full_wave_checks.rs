//! The validation cases of a travelling-wave electrode's full-wave RF mode
//! ([`super::full_wave`]): a parallel-plate line of lossy plates against its exact dispersion
//! relation, its convergence order and its power balance; the full-wave line against the
//! quasi-TEM one where both hold; its grid study; and Zhang et al. 2022's line to 300 GHz.

use num_complex::Complex64 as c64;

use super::full_wave::{FullWave, Open, RfMode};
use super::line::{DB_PER_NEPER, Metal, QuasiTem};
use super::line_checks::{COPPER, ZHANG_TANGENTS, gated, wave, zhang_2022};
use super::{Axis, Conductor, Electrostatic, Grading, Grid, Section, Side, Tensor};
use crate::geometry::{Point, Shape};
use crate::units::{Length, SPEED_OF_LIGHT};
use crate::validation::{Case, Outcome, Tier};

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
// Parallel plates
// ---------------------------------------------------------------------------------------------

/// The parallel-plate line of the analytic cases (µm): copper plates 1 µm thick, 2 µm apart,
/// the gap filled with ε = 4, tan δ = 0.01, at 100 GHz (the copper 4.8 skin depths thick).
pub(crate) const PLATES: Plates = Plates {
    t: 1.0,
    d: 2.0,
    eps: 4.0,
    tan: 0.01,
    ghz: 100.0,
};

/// A parallel-plate line: plates `t` thick a gap `d` apart (µm), filled with a dielectric of
/// relative permittivity `eps` and loss tangent `tan`, at `ghz`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Plates {
    pub t: f64,
    pub d: f64,
    pub eps: f64,
    pub tan: f64,
    pub ghz: f64,
}

impl Plates {
    /// The line across a window 10 µm wide whose sides are mirrors (plates without ends), the
    /// window's top and bottom the plates' outer faces (insulated: magnetic walls), two cells
    /// across and up graded from `smallest` at the plates' faces by `growth` (1: uniform) to
    /// 0.2 µm.
    pub(crate) fn problem(&self, smallest: f64, growth: f64) -> Electrostatic {
        let mut section = Section::new("gap");
        section.layer(-self.t, 0.0, "gap").expect(OK);
        let lower = rect([-1.0, 11.0], [-self.t, 0.0]);
        let upper = rect([-1.0, 11.0], [self.d, self.d + self.t]);
        let faces = [-self.t, 0.0, self.d, self.d + self.t];
        let largest = if growth > 1.0 { 0.2 } else { smallest };
        let grid = Grid::new(
            Axis::uniform(0.0, 10.0, 2).expect(OK),
            Axis::graded(
                -self.t,
                self.d + self.t,
                &Grading::new(largest, smallest, growth.max(1.0)).expect(OK),
                &faces,
                &faces,
            )
            .expect(OK),
        );
        let mut problem = Electrostatic::new(section, grid);
        problem
            .permittivity("gap", Tensor::isotropic(self.eps))
            .conductor(Conductor::shape(upper, 1.0))
            .conductor(Conductor::shape(lower, 0.0));
        problem
    }

    /// The lattice's mode on a grid graded from `smallest` by `growth`.
    pub(crate) fn mode(&self, smallest: f64, growth: f64) -> RfMode {
        FullWave::new(0, COPPER)
            .expect(OK)
            .with_loss_tangents(&[("gap", self.tan)])
            .solve(
                &self.problem(smallest, growth),
                self.ghz * 1e9,
                self.eps.sqrt(),
            )
            .expect(OK)
    }

    /// The exact propagation constant β (1/µm, e^(i(βz − ωt))) of the line's TEM-like mode:
    /// the even TM mode of the metal–dielectric–metal stack whose outer faces are magnetic
    /// walls. H_x is A cos(k_d (y − d/2)) in the gap, B sinh(κ (d/2 + t − |y − d/2|)) in the
    /// metal, k_d² = k₀²ε_d − β², κ² = β² − k₀²ε_m; H and E_z ∝ ε⁻¹ ∂_y H_x continuous at the
    /// faces give
    ///
    /// (k_d/ε_d) tan(k_d d/2) = (κ/ε_m) coth(κ t),
    ///
    /// both sides functions of β² alone, solved by Newton's method from k₀²ε_d.
    pub(crate) fn exact(&self) -> c64 {
        let w = 2.0 * std::f64::consts::PI * self.ghz * 1e9;
        let k = w / SPEED_OF_LIGHT * 1e-6;
        let ed = c64::new(self.eps, self.eps * self.tan);
        let em = c64::new(1.0, COPPER / (w * super::EPSILON_0));
        let f = |u: c64| {
            let kd = (k * k * ed - u).sqrt();
            let kappa = (u - k * k * em).sqrt();
            kd / ed * (kd * self.d / 2.0).tan() - kappa / em / (kappa * self.t).tanh()
        };
        let mut u = k * k * ed;
        for _ in 0..100 {
            let h = 1e-7 * u.norm();
            let slope = (f(u + h) - f(u - h)) / (2.0 * h);
            let step = f(u) / slope;
            u -= step;
            if step.norm() < 1e-15 * u.norm() {
                break;
            }
        }
        let beta = u.sqrt();
        if beta.re < 0.0 { -beta } else { beta }
    }
}

/// The lattice's complex β (1/µm) from a mode's γ = α + jβ (1/m).
fn beta_of(m: &RfMode) -> c64 {
    c64::new(m.gamma.im, m.gamma.re) * 1e-6
}

/// The lattice's β against the exact one (relative error) on a uniform grid of `h` up.
pub(crate) fn plates_error(h: f64) -> f64 {
    let exact = PLATES.exact();
    (beta_of(&PLATES.mode(h, 1.0)) - exact).norm() / exact.norm()
}

/// The plates' loss that the power absorbed in the metal and the gap doesn't account for, over
/// the loss: the eigenvalue's α against Poynting's theorem on the lattice (relative).
pub(crate) fn plates_balance() -> f64 {
    let m = PLATES.mode(0.01, 1.0);
    m.radiation() / m.attenuation()
}

// ---------------------------------------------------------------------------------------------
// Zhang et al. 2022's line, full-wave
// ---------------------------------------------------------------------------------------------

/// The full-wave grids of the published line: graded from 40, 20 and 10 nm.
pub(crate) const FULL_STEPS: [f64; 3] = [0.04, 0.02, 0.01];

/// Zhang et al.'s line as a full-wave problem: copper at 5.8 × 10⁷ S/m, their loss tangents,
/// the silicon a half-space (the window open below, and to the right, into 200 µm of PML).
pub(crate) fn zhang_full_wave() -> FullWave {
    FullWave::new(0, COPPER)
        .expect(OK)
        .mirrored()
        .with_loss_tangents(&ZHANG_TANGENTS)
        .with_count(4)
        .with_open(Open::new(Side::Bottom, 200.0).expect(OK).with_cells(15))
        .with_open(Open::new(Side::Right, 200.0).expect(OK).with_cells(15))
}

/// The frequencies of [`zhang_full_wave_numbers`], GHz.
pub(crate) const ZHANG_GHZ: [f64; 3] = [50.0, 250.0, 300.0];

/// photonoxide's full-wave numbers for Zhang et al.'s line at [`ZHANG_GHZ`], each gated at
/// 0.5% on [`FULL_STEPS`]: per frequency the RF index, Re Z₀ (Ω) and the loss (dB/cm), and
/// the radiation (dB/cm, not gated: a small difference of two converged numbers); and the
/// largest estimated relative error of the gated ones.
#[allow(clippy::type_complexity)]
pub(crate) fn zhang_full_wave_numbers() -> &'static ([[f64; 4]; 3], f64) {
    static NUMBERS: std::sync::OnceLock<([[f64; 4]; 3], f64)> = std::sync::OnceLock::new();
    NUMBERS.get_or_init(|| {
        let fw = zhang_full_wave();
        let on: Vec<Vec<RfMode>> = FULL_STEPS
            .iter()
            .map(|&h| {
                let problem = zhang_2022().electrostatic(h);
                ZHANG_GHZ
                    .iter()
                    .map(|&f| fw.solve(&problem, f * 1e9, 2.26).expect(OK))
                    .collect()
            })
            .collect();
        let mut worst = 0.0f64;
        let mut out = [[0.0; 4]; 3];
        for (k, row) in out.iter_mut().enumerate() {
            let pick = |m: &RfMode, q: usize| match q {
                0 => m.index(),
                1 => m.impedance.re,
                _ => m.db_per_cm(),
            };
            for (q, slot) in row.iter_mut().take(3).enumerate() {
                // the index changes by 1e-4 a grid, which on graded grids that aren't nested
                // needn't shrink steadily: its study takes the two finest, at least first order
                let skip = usize::from(q == 0);
                let mut study = crate::convergence::Study::new("a full-wave line's number");
                for (h, modes) in FULL_STEPS.iter().zip(&on).skip(skip) {
                    study = study.on(*h, pick(&modes[k], q)).expect(OK);
                }
                let v = pick(&on[2][k], q);
                let check = study.check(5e-3 * v.abs()).expect(OK);
                worst = worst.max(check.error.unwrap_or(f64::INFINITY) / v.abs());
                *slot = check.gated().expect("a converged grid");
            }
            row[3] = on[2][k].radiation() * DB_PER_NEPER / 100.0;
        }
        (out, worst)
    })
}

/// The quasi-TEM line's numbers at 50 GHz on Zhang et al.'s line, gated at 0.5% on the static
/// grids graded from 8, 4 and 2 nm: the RF index, Re Z₀ and the loss (dB/cm).
pub(crate) fn zhang_quasi_tem_50() -> &'static [f64; 3] {
    static NUMBERS: std::sync::OnceLock<[f64; 3]> = std::sync::OnceLock::new();
    NUMBERS.get_or_init(|| {
        let copper = Metal::new(COPPER, 0.83).expect(OK);
        let v = gated(
            &zhang_2022(),
            |line: &QuasiTem| {
                let w = wave(line, 50.0, &copper, &ZHANG_TANGENTS);
                vec![w.index(), w.impedance.re, w.db_per_cm()]
            },
            5e-3,
        );
        [v[0], v[1], v[2]]
    })
}

/// The full-wave number `q` (0 the index, 1 Re Z₀, 2 the loss) at 50 GHz against the
/// quasi-TEM one, relative.
fn against_quasi_tem(q: usize) -> f64 {
    let full = zhang_full_wave_numbers().0[0][q];
    let quasi = zhang_quasi_tem_50()[q];
    (full - quasi) / quasi
}

pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "line/full-wave-plates",
            title: r"Travelling-wave electrodes, the full-wave RF mode: a parallel-plate line of copper plates 1 µm thick, 2 µm apart, the gap of $\varepsilon$ = 4 and tan δ = 0.01, at 100 GHz (the copper 4.8 skin depths thick), the plates' outer faces magnetic walls; the lattice's complex $\beta$ on a uniform grid of 5 nm up against the exact one (relative error shown)",
            tier: Tier::Analytic,
            source: r"the even TM mode of the metal–dielectric–metal stack: $(k_d/\varepsilon_d)\tan(k_d d/2) = (\kappa/\varepsilon_m)\coth(\kappa t)$, $k_d^2 = k_0^2\varepsilon_d - \beta^2$, $\kappa^2 = \beta^2 - k_0^2\varepsilon_m$, $\varepsilon_m = 1 + i\sigma/(\omega\varepsilon_0)$, solved by Newton's method; the lattice is second order: 3e-5",
            run: || outcome(plates_error(0.005), 0.0, 3e-5),
        },
        Case {
            id: "line/full-wave-plates-order",
            title: r"The same on uniform grids of 10 and 5 nm: the order of the error in $\beta$ (shown)",
            tier: Tier::Analytic,
            source: r"the Yee lattice's differences are centred and the interfaces on its lines (K. S. Yee, IEEE Trans. Antennas Propag. 14, 302 (1966), doi:10.1109/TAP.1966.1138693): second order",
            run: || {
                let e = [0.01, 0.005].map(plates_error);
                outcome((e[0] / e[1]).log2(), 2.0, 0.1)
            },
        },
        Case {
            id: "line/full-wave-balance",
            title: r"The same line on a uniform grid of 10 nm: the eigenvalue's loss $\alpha$ against the power the metal and the gap absorb over twice the power carried, $\tfrac12\int\omega\varepsilon_0\varepsilon''\lvert E\rvert^2 dA / (2 \operatorname{Re} P)$, in a window that radiates nothing (relative difference shown)",
            tier: Tier::Analytic,
            source: r"Poynting's theorem for a mode decaying as $e^{-\alpha z}$: $2\alpha P$ is the power absorbed per unit length; on the lattice the discrete fields keep it to the eigensolver's tolerance, 1e-10 on β², over its conditioning: 1e-7",
            run: || outcome(plates_balance(), 0.0, 1e-7),
        },
        Case {
            id: "line/full-wave-quasi-tem-index",
            title: r"The full-wave and the quasi-TEM line where both hold: Zhang et al.'s line (a 20 µm signal, 5 µm gaps, 0.83 µm of copper on thin-film lithium niobate, see line/zhang-2022-index) at 50 GHz, the full-wave grids graded from 40, 20 and 10 nm, the static from 8, 4 and 2 nm, each gated at 0.5%: the full-wave RF index against the quasi-TEM one (relative difference shown)",
            tier: Tier::Analytic,
            source: r"at 50 GHz the line's cross-section (30 µm across the gaps) is a two-hundredth of the RF wavelength and the copper 2.8 skin depths thick, where quasi-TEM with Holloway and Kuester's internal impedance holds (W. Heinrich, IEEE Trans. Microw. Theory Tech. 41, 45 (1993), doi:10.1109/22.210228, Section VI: $\lvert\Delta\beta/\beta\rvert$ below 1.5% for their quasi-TEM model): 0.5%",
            run: || outcome(against_quasi_tem(0), 0.0, 5e-3),
        },
        Case {
            id: "line/full-wave-quasi-tem-impedance",
            title: r"The same line's full-wave Re $Z_0$ (power–current) at 50 GHz against the quasi-TEM one (relative difference shown)",
            tier: Tier::Analytic,
            source: r"quasi-TEM's $Z_0 = \sqrt{(R + j\omega L)/(G + j\omega C)}$ and the full-wave's power–current $2P/\lvert I\rvert^2$ are one for a TEM line ($P = \tfrac12 V I^*$); Heinrich 1993's quasi-TEM model within 3% of full-wave: 1%",
            run: || outcome(against_quasi_tem(1), 0.0, 1e-2),
        },
        Case {
            id: "line/full-wave-quasi-tem-loss",
            title: r"The same line's full-wave loss at 50 GHz (the volume current in the copper, the dielectrics' loss tangents) against the quasi-TEM loss (Holloway and Kuester's conductor loss, the energy shares' dielectric loss; relative difference shown)",
            tier: Tier::Analytic,
            source: r"C. L. Holloway, E. F. Kuester, IEEE Trans. Microw. Theory Tech. 43, 2695 (1995), doi:10.1109/22.477846, Section V: their conductor loss within 5–10% of full-wave methods where gaps are narrow: 5%",
            run: || outcome(against_quasi_tem(2), 0.0, 5e-2),
        },
        Case {
            id: "line/full-wave-grid",
            title: r"Convergence: Zhang et al.'s line full-wave at 50, 250 and 300 GHz, the silicon a half-space (the window open below and to the right into 200 µm of PML), on grids graded from 40, 20 and 10 nm by 15% a cell: the grid study's largest estimated relative error of the RF index, Re $Z_0$ and the loss at 10 nm (shown)",
            tier: Tier::Analytic,
            source: r"the grid study of docs/methods/convergence.md (Richardson's estimate from the three grids), the conductors' edges and faces graded; the gate the published numbers pass: 0.5%",
            run: || outcome(zhang_full_wave_numbers().1, 0.0, 5e-3),
        },
        Case {
            id: "line/zhang-2022-full-wave-index",
            title: r"Published: Zhang et al. 2022's line full-wave (as line/full-wave-grid, gated at 0.5%): the RF index at 250 GHz (shown) against their simulation's, about 2.26 from 100 to 300 GHz",
            tier: Tier::Published,
            source: r"Y. Zhang et al., Photon. Res. 10, 2380 (2022), doi:10.1364/PRJ.468518, Fig. 2(a) (Ansys HFSS, the 0.6 µm cladding, read from the figure) and Section 3.B: their measured index 0.04 above the design at 250 GHz: 0.04",
            run: || outcome(zhang_full_wave_numbers().0[1][0], 2.26, 0.04),
        },
        Case {
            id: "line/zhang-2022-full-wave-impedance",
            title: r"The same line's full-wave Re $Z_0$ at 250 GHz (Ω, shown) against their measured 41.5 Ω",
            tier: Tier::Published,
            source: r"Zhang et al. 2022, Section 3.B and Fig. 2(b): 41.5 Ω extracted from the measured s-parameters at 250 GHz, 40.5 Ω simulated: 1.5 Ω",
            run: || outcome(zhang_full_wave_numbers().0[1][1], 41.5, 1.5),
        },
        Case {
            id: "line/zhang-2022-full-wave-loss",
            title: r"The same line's full-wave loss at 250 GHz (dB/cm, shown; copper at 5.8 × 10⁷ S/m, not given; their loss tangents, the thermal oxide's taken as the deposited one's) against the loss their Fig. 3(b) extracts from the measured lines, about 16 dB/cm",
            tier: Tier::Published,
            source: r"Zhang et al. 2022, Fig. 3(b) (read from the figure; their Section 3.B: the measured loss above their model's above 200 GHz) and Section 3.B: 1.3 dB/mm from the transmission, their fit $0.42\sqrt f + 0.025 f$ 12.9 dB/cm: 3 dB/cm",
            run: || outcome(zhang_full_wave_numbers().0[1][2], 16.0, 3.0),
        },
    ]
}
