//! The crosstalk correction's cases: Gurses et al. 2022's lumped networks (their Figs. 18 and
//! 19) against the numbers they extract from their measurements, their array corrected by their
//! Eq. 13, and the objective's gradient against differences.

use super::shifter_checks::{OK, outcome};
use super::{Correction, Network};
use crate::validation::{Case, Tier};

/// Gurses et al.'s numbers (doi:10.1109/JSTQE.2022.3189965, Sections II.C and IV): the
/// shifter's conductance G and the waveguide's heat capacity C_W (W/K, J/K) they extract from
/// its P_π (W) and −3 dB frequency (Hz); the first neighbour's coupling G_SS (W/K) from its own
/// P_π^(1), and the coupling they tune it to for the array.
pub(crate) mod gurses {
    pub const G: f64 = 62.8e-6;
    pub const C_W: f64 = 440e-12;
    pub const P_PI: f64 = 10.6e-3;
    pub const F_3DB: f64 = 56.8e3;
    pub const G_SS: f64 = 4.57e-6;
    pub const P_PI_NEIGHBOUR: f64 = 62.6e-3;
    pub const G_SS_ARRAY: f64 = 13.2e-6;
}

/// Gurses et al.'s reduced shifter (their Fig. 18) at heater node `h` and waveguide node `w`:
/// the heater R/2 to the ambient and 2R to the waveguide, the waveguide R/2 to the ambient with
/// C_W, R = 1/G.
fn shifter(n: &mut Network, h: usize, w: usize) {
    use gurses::{C_W, G};
    n.to_ambient(h, 2.0 * G).expect(OK);
    n.conductance(h, w, 0.5 * G).expect(OK);
    n.to_ambient(w, 2.0 * G).expect(OK);
    n.capacitance(w, C_W).expect(OK);
}

/// `count` of Gurses et al.'s shifters in a row, shifter i's heater node 2i and waveguide node
/// 2i + 1, each pair of neighbours joined as their Fig. 19 joins them: R_X = 2R + 1/`g_ss` from
/// each of one's two nodes to each of the other's.
pub(crate) fn row(count: usize, g_ss: f64) -> Network {
    let mut n = Network::new(2 * count);
    for i in 0..count {
        shifter(&mut n, 2 * i, 2 * i + 1);
    }
    let x = 1.0 / (2.0 / gurses::G + 1.0 / g_ss);
    for i in 1..count {
        for a in [2 * i - 2, 2 * i - 1] {
            for b in [2 * i, 2 * i + 1] {
                n.conductance(a, b, x).expect(OK);
            }
        }
    }
    n
}

/// The waveguide's rise at π, ΔT_π: P_π through one shifter (their Eq. 31, P_π R/12).
pub(crate) fn rise_at_pi() -> f64 {
    row(1, gurses::G_SS).steady(&[gurses::P_PI, 0.0]).expect(OK)[1]
}

/// The neighbour's power for π (W) through Fig. 19's pair coupled by `g_ss`: what heats the
/// second shifter's waveguide by ΔT_π from the first's heater.
pub(crate) fn neighbour_p_pi(g_ss: f64) -> f64 {
    let per_watt = row(2, g_ss).steady(&[1.0, 0.0, 0.0, 0.0]).expect(OK)[3];
    rise_at_pi() / per_watt
}

/// Gurses et al.'s array as their Eq. 13 takes it: 9 shifters in a row, first-order crosstalk
/// only, the neighbour's P_π from their tuned coupling, each shifter wanted at π.
pub(crate) fn array() -> Correction {
    Correction::row(
        9,
        gurses::P_PI,
        &[neighbour_p_pi(gurses::G_SS_ARRAY)],
        vec![std::f64::consts::PI; 9],
    )
    .expect(OK)
}

/// The largest relative difference of the objective's gradient from its central differences, at
/// the array's uncorrected powers.
fn gradient_error() -> f64 {
    let c = array();
    let p = c.uncorrected().expect(OK);
    let (_, g) = c.objective(&p).expect(OK);
    let scale = g.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    (0..p.len())
        .map(|j| {
            let h = 1e-6 * p[j];
            let at = |d: f64| {
                let mut q = p.clone();
                q[j] += d;
                c.objective(&q).expect(OK).0
            };
            ((at(h) - at(-h)) / (2.0 * h) - g[j]).abs() / scale
        })
        .fold(0.0, f64::max)
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "correction/gurses-cutoff",
            title: r"Crosstalk correction, published: Gurses et al.'s reduced shifter (Fig. 18: the heater $R/2$ to the ambient and $2R$ to the waveguide, the waveguide $R/2$ with $C_W$) at their extracted $G = 1/R$ = 62.8 µW/K and $C_W$ = 440 pJ/K, its compact model from the heater to the waveguide by its modes; the −3 dB frequency (kHz)",
            tier: Tier::Published,
            source: r"V. Gurses et al., IEEE J. Sel. Top. Quantum Electron. 28(6), 1 (2022), doi:10.1109/JSTQE.2022.3189965, Section II.C: 56.8 kHz measured, from which they extract $C_W$; 5% for their Eq. (33), which leaves out the heater node's own divider ($C_W = 2.5\thinspace G\tau$ where the network gives $2.4\thinspace G\tau$)",
            run: || {
                let m = row(1, gurses::G_SS).model(&[0], &[1]).expect(OK);
                outcome(
                    m.cutoff(0, 0).expect(OK) * 1e-3,
                    gurses::F_3DB * 1e-3,
                    0.05 * 56.8,
                )
            },
        },
        Case {
            id: "correction/gurses-neighbour",
            title: r"Crosstalk correction, published: two of Gurses et al.'s shifters coupled as their Fig. 19 couples them ($R_X = 2R + 1/G_{SS}$ from each node of one to each of the other's) at $G_{SS}$ = 4.57 µW/K, solved as a network; the first shifter's power that heats the second's waveguide by $\Delta T_\pi$ = $P_\pi R/12$ (Eq. 31, $P_\pi$ = 10.6 mW) (mW)",
            tier: Tier::Published,
            source: r"V. Gurses et al., doi:10.1109/JSTQE.2022.3189965, Section II.C: the neighbour's $P_\pi^{(1)}$ = 62.6 mW measured, from which their Eq. (36) gives $G_{SS}$ = 4.57 µW/K; 1% for the three digits of $G$ and $G_{SS}$",
            run: || {
                outcome(
                    neighbour_p_pi(gurses::G_SS) * 1e3,
                    gurses::P_PI_NEIGHBOUR * 1e3,
                    0.01 * 62.6,
                )
            },
        },
        Case {
            id: "correction/gurses-array",
            title: r"Crosstalk correction, published: Gurses et al.'s array as their Eq. 13 takes it, 9 shifters in a row with first-order crosstalk, the neighbour's $P_\pi^{(1)}$ from Fig. 19's pair at their tuned $G_{SS}$ = 13.2 µW/K, each wanted at π; set by $P = C^{-1}\Phi$, the largest phase error (rad, shown; uncorrected, each heater at its own $P_\pi$, the inner shifters are 2.15 rad over)",
            tier: Tier::Published,
            source: r"V. Gurses et al., doi:10.1109/JSTQE.2022.3189965, Section IV.A and Fig. 15: the correction cancels the crosstalk",
            run: || {
                let c = array();
                let p = c.inverse().expect(OK);
                let worst = c
                    .errors(&p)
                    .expect(OK)
                    .iter()
                    .fold(0.0f64, |m, e| m.max(e.abs()));
                outcome(worst, 0.0, 1e-12)
            },
        },
        Case {
            id: "correction/gradient",
            title: r"Crosstalk correction, analytic: the objective $\sum_i (1 - \cos(\varphi_i - \varphi_i^*))$ on Gurses et al.'s array, its exact gradient $\sum_i \sin(\varphi_i - \varphi_i^*) C_{ij}$ at the uncorrected powers against central differences of 1e-6 of each power; the largest difference over the largest component (shown)",
            tier: Tier::Analytic,
            source: r"central differences, their error of order 1e-12 of the step's square",
            run: || outcome(gradient_error(), 0.0, 1e-6),
        },
    ]
}
