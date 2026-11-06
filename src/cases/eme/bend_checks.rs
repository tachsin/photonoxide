//! Bends ([`super::Slice::bent`], [`super::Section::bend`]): the mapped stack's modes against
//! the exact bent slab's ([`crate::mode::bend::SlabBend`]), the staircase's order, the joint
//! between a straight and a bend, and two bends against one.

use num_complex::Complex64 as c64;

use super::scattering::{Blocks, interface, overlaps, star, then_propagate};
use super::{Eme, Mode, Slice};
use crate::mode::Polarization;
use crate::mode::bend::SlabBend;
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

/// Silicon, the core's index (as the step's, `eme/step-one-mode`).
pub(crate) const SI: f64 = 3.473;
/// Silica, the cladding's.
pub(crate) const SIO2: f64 = 1.444;
/// The slab's thickness, µm.
pub(crate) const T: f64 = 0.22;
/// The wavelength, µm.
pub(crate) const LAMBDA: f64 = 1.55;
/// The thin layers' step the cases start from, µm: [`super::BEND_STEP`].
pub(crate) const STEP: f64 = super::BEND_STEP;

fn lam() -> Wavelength {
    Wavelength::from_um_unchecked(LAMBDA)
}

/// The window about a slab bent at `r` µm to its centre: the cladding inside it and outside it,
/// µm. Inside, half the radius up to 2.5 µm; outside 2.5 µm, to the PML.
pub(crate) fn window(r: f64) -> (f64, f64) {
    ((0.5 * r).min(2.5), 2.5)
}

/// The radius to the window's middle of a slab bent at `r` µm to its centre.
pub(crate) fn middle(r: f64) -> f64 {
    let (inner, outer) = window(r);
    r + 0.5 * (outer - inner)
}

/// The slab in its window, straight.
pub(crate) fn straight(r: f64) -> Slice {
    let (inner, outer) = window(r);
    let clad = c64::new(SIO2, 0.0);
    Slice::new(&[
        (clad, Length::um(inner)),
        (c64::new(SI, 0.0), Length::um(T)),
        (clad, Length::um(outer)),
    ])
    .expect("a slab")
}

/// The slab bent at `r` µm to its centre.
pub(crate) fn bent(r: f64) -> Slice {
    straight(r)
        .bent(Length::um(middle(r)))
        .expect("a bent slab")
}

/// The exact bend of the same slab, its reference radius the window's middle.
pub(crate) fn exact(r: f64) -> SlabBend {
    let (inner, outer) = window(r);
    let half = 0.5 * (inner + T + outer);
    SlabBend::new(
        Length::um(middle(r)),
        SIO2,
        &[(SI, Length::um(T))],
        SIO2,
        Length::um(inner - half),
    )
    .expect("a bent slab")
}

/// The exact fundamental mode's n_eff = ν/(kR) at the window's middle, its Runge–Kutta steps
/// half [`SlabBend`]'s.
pub(crate) fn exact_index(polarization: Polarization, r: f64) -> c64 {
    exact_with(polarization, r, 0.5)
}

/// [`exact_index`] with the Runge–Kutta steps `steps` times [`SlabBend`]'s.
pub(crate) fn exact_with(polarization: Polarization, r: f64, steps: f64) -> c64 {
    exact(r)
        .with_steps(steps)
        .fundamental(polarization, lam())
        .expect("the exact bend's mode")
}

/// [`exact_index`]'s own error: its Runge–Kutta steps' fourth order, |n(h) − n(h/2)|/15.
pub(crate) fn exact_error(polarization: Polarization, r: f64) -> f64 {
    (exact_with(polarization, r, 1.0) - exact_index(polarization, r)).norm() / 15.0
}

/// The expansion's settings: `modes` modes, a PML 2 µm thick of strength 0.5 at the upper wall
/// (outside the bend), the thin layers at most `step` µm thick; a PML 1/`scale` as strong.
pub(crate) fn settings(polarization: Polarization, modes: usize, step: f64, scale: f64) -> Eme {
    Eme::new(polarization, lam(), modes)
        .and_then(|e| e.with_upper_pml(Length::um(2.0), 0.5 / scale))
        .and_then(|e| e.with_bend_step(Length::um(step)))
        .expect("the bend's settings")
}

/// The mode of `modes` nearest `n`.
pub(crate) fn nearest(modes: &[Mode], n: c64) -> usize {
    (0..modes.len())
        .min_by(|&a, &b| {
            (modes[a].effective_index() - n)
                .norm()
                .total_cmp(&(modes[b].effective_index() - n).norm())
        })
        .unwrap_or(0)
}

/// The bend's fundamental n_eff at `r` µm by the mapped stack with `step` and the PML scaled by
/// `scale`: the mode nearest `near`.
pub(crate) fn index(polarization: Polarization, r: f64, step: f64, scale: f64, near: c64) -> c64 {
    let modes = settings(polarization, 6, step, scale)
        .modes(&bent(r))
        .expect("the bend's modes");
    modes[nearest(&modes, near)].effective_index()
}

/// Richardson's limit of a second-order quantity at a step and half of it.
pub(crate) fn limit(coarse: c64, fine: c64) -> c64 {
    fine + (fine - coarse) / 3.0
}

/// The fundamental n_eff at `r` µm: exact, Richardson's limit from [`STEP`] and half of it, and
/// that limit's own error: the larger of its change from the limit of the next two halvings and
/// from a PML twice as thick and half as strong.
pub(crate) fn index_limit(polarization: Polarization, r: f64) -> (c64, c64, f64) {
    let want = exact_index(polarization, r);
    let n = |step: f64, scale: f64| index(polarization, r, step, scale, want);
    let (a, b, c) = (n(STEP, 1.0), n(0.5 * STEP, 1.0), n(0.25 * STEP, 1.0));
    let got = limit(a, b);
    let finer = limit(b, c);
    let pml = limit(n(STEP, 2.0), n(0.5 * STEP, 2.0));
    (want, got, (got - finer).norm().max((got - pml).norm()))
}

/// dB per 90° of a mode of effective index `n` at the radius `r` µm its index is taken at.
pub(crate) fn db_per_quarter(n: c64, r: f64) -> f64 {
    let k = std::f64::consts::TAU / LAMBDA;
    20.0 / std::f64::consts::LN_10 * k * n.im * r * std::f64::consts::FRAC_PI_2
}

/// The staircase's order from the errors at [`STEP`] and 2, 4 and 8 times as many layers
/// against the exact n_eff, at `r` µm: the three orders.
pub(crate) fn orders(polarization: Polarization, r: f64) -> [f64; 3] {
    let want = exact_index(polarization, r);
    let e: Vec<f64> = [1.0, 0.5, 0.25, 0.125]
        .iter()
        .map(|&f| (index(polarization, r, f * STEP, 1.0, want) - want).norm())
        .collect();
    [0, 1, 2].map(|j| (e[j] / e[j + 1]).log2())
}

/// The S-matrix as blocks, of the straight slab, a bend of `angles` (rad) in turn at `r` µm
/// (each its own section, the joints between them taken by their overlaps), and the straight
/// slab, with `modes` modes: between the straight's modes at both ends.
pub(crate) fn turn(polarization: Polarization, r: f64, angles: &[f64], modes: usize) -> Blocks {
    let eme = settings(polarization, modes, STEP, 1.0);
    let s = eme.modes(&straight(r)).expect("the straight's modes");
    let b = eme.modes(&bent(r)).expect("the bend's modes");
    let k = std::f64::consts::TAU / LAMBDA;
    let advance = |angle: f64| -> Vec<c64> {
        b.iter()
            .map(|m| (c64::new(0.0, k * middle(r) * angle) * m.effective_index()).exp())
            .collect()
    };
    let mut total = interface(&overlaps(&s, &b));
    for (j, &angle) in angles.iter().enumerate() {
        if j > 0 {
            total = star(&total, &interface(&overlaps(&b, &b)));
        }
        total = then_propagate(&total, &advance(angle));
    }
    star(&total, &interface(&overlaps(&b, &s)))
}

/// The largest |a_qp − b_qp| over the four blocks.
pub(crate) fn blocks_departure(a: &Blocks, b: &Blocks) -> f64 {
    [
        (&a.s11, &b.s11),
        (&a.s12, &b.s12),
        (&a.s21, &b.s21),
        (&a.s22, &b.s22),
    ]
    .iter()
    .flat_map(|(x, y)| {
        (0..x.nrows())
            .flat_map(move |i| (0..x.ncols()).map(move |j| (x[(i, j)] - y[(i, j)]).norm()))
    })
    .fold(0.0, f64::max)
}

/// A turn of 180° at `r` µm as two bends of 90° against one of 180°: the largest |ΔS_qp|.
pub(crate) fn halves(polarization: Polarization, r: f64, modes: usize) -> f64 {
    let q = std::f64::consts::FRAC_PI_2;
    blocks_departure(
        &turn(polarization, r, &[q, q], modes),
        &turn(polarization, r, &[2.0 * q], modes),
    )
}

/// Gauss–Legendre points and weights on [a, b], 20 a panel, panels at most 0.02 µm wide.
fn quadrature(a: f64, b: f64) -> Vec<(f64, f64)> {
    let panels = ((b - a) / 0.02).ceil().max(1.0) as usize;
    let h = (b - a) / panels as f64;
    let mut out = Vec::new();
    for p in 0..panels {
        let centre = a + (p as f64 + 0.5) * h;
        for &(node, weight) in &super::scattering::GL20 {
            for x in [centre - 0.5 * h * node, centre + 0.5 * h * node] {
                out.push((x, 0.5 * h * weight));
            }
        }
    }
    out
}

/// The straight-to-bend joint's coupling between the two fundamental modes,
/// η = ⟨E_s, H_b⟩⟨E_b, H_s⟩ / (⟨E_s, H_s⟩⟨E_b, H_b⟩), at `r` µm by the expansion's overlaps,
/// its normalized modes, the bend's at `step`.
pub(crate) fn coupling(polarization: Polarization, r: f64, step: f64) -> c64 {
    let want = exact_index(polarization, r);
    let eme = settings(polarization, 6, step, 1.0);
    let s = eme.modes(&straight(r)).expect("the straight's modes");
    let b = eme.modes(&bent(r)).expect("the bend's modes");
    let g = nearest(&b, want);
    overlaps(&s, &b)[(0, g)] * overlaps(&b, &s)[(g, 0)]
}

/// The same coupling with the exact bend's field ([`SlabBend::field`], its Runge–Kutta steps
/// `steps` times its own) and the straight slab's mode, integrated across the window.
pub(crate) fn exact_coupling(polarization: Polarization, r: f64, steps: f64) -> c64 {
    let want = exact_with(polarization, r, steps);
    let straight_modes = settings(polarization, 6, STEP, 1.0)
        .modes(&straight(r))
        .expect("the straight's modes");
    let (inner, outer) = window(r);
    let w = inner + T + outer;
    let mut points = quadrature(0.0, inner);
    points.extend(quadrature(inner, inner + T));
    points.extend(quadrature(inner + T, w));
    let xs: Vec<f64> = points.iter().map(|p| p.0 - 0.5 * w).collect();
    let psi = exact(r)
        .with_steps(steps)
        .field(polarization, lam(), want, &xs);
    let rm = middle(r);
    let index_at = |x: f64| if x > inner && x < inner + T { SI } else { SIO2 };
    let zero = c64::new(0.0, 0.0);
    let (mut sb, mut bs, mut ss, mut bb) = (zero, zero, zero, zero);
    for (j, &(x, wt)) in points.iter().enumerate() {
        let (es, hs) = straight_modes[0].field(x);
        // the bend's tangential fields, up to one factor: TE (E_y, H_ρ) = (ψ, n_eff (R/ρ) ψ),
        // TM (E_ρ, H_y) = (n_eff (R/ρ) ψ / n², ψ)
        let shrink = rm / (rm + xs[j]);
        let n2 = index_at(x).powi(2);
        let (eb, hb) = match polarization {
            Polarization::Te => (psi[j], want * shrink * psi[j]),
            Polarization::Tm => (want * shrink * psi[j] / n2, psi[j]),
        };
        // the straight's H_x is −n_eff E_y: the bend's taken with the same sign
        let hs = match polarization {
            Polarization::Te => -hs,
            Polarization::Tm => hs,
        };
        sb += wt * es * hb;
        bs += wt * eb * hs;
        ss += wt * es * hs;
        bb += wt * eb * hb;
    }
    sb * bs / (ss * bb)
}
/// Both polarizations.
const BOTH: [Polarization; 2] = [Polarization::Te, Polarization::Tm];

/// The index case's radii, µm.
pub(crate) const RADII: [f64; 4] = [2.0, 5.0, 10.0, 20.0];

/// The loss case's bends: those whose loss is above the arithmetic's reach (an Im n_eff of
/// 1e-12 or more).
pub(crate) const LOSSY: [(Polarization, f64); 5] = [
    (Polarization::Te, 2.0),
    (Polarization::Te, 3.0),
    (Polarization::Tm, 2.0),
    (Polarization::Tm, 3.0),
    (Polarization::Tm, 5.0),
];

/// The joint case's bends: those whose radiation, which the exact field's integral across the
/// window leaves out, is under 1e-6 of their power per µm.
pub(crate) const JOINTS: [(Polarization, f64); 4] = [
    (Polarization::Te, 2.0),
    (Polarization::Te, 3.0),
    (Polarization::Te, 5.0),
    (Polarization::Tm, 5.0),
];

/// The index case: the largest |Δn_eff| of Richardson's limit against the exact over both
/// polarizations at [`RADII`], and the largest own error, the limit's or the exact's.
pub(crate) fn index_case() -> (f64, f64) {
    let mut worst = (0.0f64, 0.0f64);
    for pol in BOTH {
        for r in RADII {
            let (want, got, own) = index_limit(pol, r);
            worst.0 = worst.0.max((got - want).norm());
            worst.1 = worst.1.max(own).max(exact_error(pol, r));
        }
    }
    worst
}

/// The loss case: the largest relative error of the loss (Im n_eff) of Richardson's limit at
/// [`LOSSY`], and the largest own error, relative.
pub(crate) fn loss_case() -> (f64, f64) {
    let mut worst = (0.0f64, 0.0f64);
    for (pol, r) in LOSSY {
        let (want, got, own) = index_limit(pol, r);
        let im = want.im;
        worst.0 = worst.0.max((got.im / im - 1.0).abs());
        worst.1 = worst.1.max(own / im).max(exact_error(pol, r) / im);
    }
    worst
}

/// The order case: the order from [`STEP`] to 8 times as many layers, TE and TM at 2 µm, the
/// one furthest from 2.
pub(crate) fn order_case() -> f64 {
    BOTH.iter()
        .flat_map(|&pol| orders(pol, 2.0))
        .fold(2.0, |a: f64, b| if (b - 2.0).abs() > (a - 2.0).abs() { b } else { a })
}

/// The halves case: TE and TM at 2 µm with 12 modes, the largest |ΔS_qp|.
pub(crate) fn halves_case() -> f64 {
    BOTH.iter()
        .map(|&pol| halves(pol, 2.0, 12))
        .fold(0.0, f64::max)
}

/// A bend of no angle between two straights at `r` µm with `modes` modes: the departure of S
/// from the straight's (the identity), max(|S21 − 1|, |S11|) between the fundamental modes.
pub(crate) fn no_angle(polarization: Polarization, r: f64, modes: usize) -> f64 {
    let s = turn(polarization, r, &[0.0], modes);
    (s.s21[(0, 0)] - 1.0).norm().max(s.s11[(0, 0)].norm())
}

/// The no-angle case: TE and TM at 2 and 3 µm with 24 modes, the largest departure.
pub(crate) fn no_angle_case() -> f64 {
    BOTH.iter()
        .flat_map(|&pol| [2.0, 3.0].map(|r| no_angle(pol, r, 24)))
        .fold(0.0, f64::max)
}

/// The joint case: the largest |Δη| of Richardson's limit of the expansion's coupling against
/// the exact field's at [`JOINTS`], and the largest own error, the limit's or the exact's.
pub(crate) fn joint_case() -> (f64, f64) {
    let mut worst = (0.0f64, 0.0f64);
    for (pol, r) in JOINTS {
        let c = [STEP, 0.5 * STEP, 0.25 * STEP].map(|s| coupling(pol, r, s));
        let (got, finer) = (limit(c[0], c[1]), limit(c[1], c[2]));
        let (rough, want) = (exact_coupling(pol, r, 1.0), exact_coupling(pol, r, 0.5));
        worst.0 = worst.0.max((got - want).norm());
        worst.1 = worst
            .1
            .max((got - finer).norm())
            .max((rough - want).norm() / 15.0);
    }
    worst
}

fn zero(measured: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected: 0.0,
        tolerance,
        error: measured,
    }
}

/// Bends' cases, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "eme/bend-index",
            title: r"A bend by the conformal map: a 220 nm silicon slab (3.473 in 1.444, 1.55 µm) bent at 2, 5, 10 and 20 µm to its centre, TE and TM, the window 2.5 µm outside it to a PML 2 µm thick of strength 0.5 and half the radius (at most 2.5 µm) inside: the fundamental mode's $n_\text{eff} = \nu/kR$ at the window's middle by the mapped stack $n(u)\thinspace e^{u/R}$ in thin layers at most 10 and 5 nm thick, Richardson's limit of the two, against the exact bent slab's (largest $\lvert \Delta n_\text{eff} \rvert$ shown)",
            tier: Tier::Analytic,
            source: r"`mode::bend`'s exact $\nu$: radial shooting matched to the outgoing Hankel function $H^{(1)}_\nu(kn\rho)$ (D. Marcuse, Bell Syst. Tech. J. 50, 2551 (1971), doi:10.1002/j.1538-7305.1971.tb02620.x, Eq. 10, without his approximation inside), its Runge–Kutta steps halved; the map $u = R \ln(\rho/R)$ is exact for the 2D wave equation (M. Heiblum, J. H. Harris, IEEE J. Quantum Electron. 11, 75 (1975), doi:10.1109/JQE.1975.1068563)",
            run: || {
                let (measured, own) = index_case();
                crate::verdict::note_numerical(own);
                zero(measured, INDEX_TOLERANCE)
            },
        },
        Case {
            id: "eme/bend-loss",
            title: r"The same bends' radiation loss, $\operatorname{Im} n_\text{eff}$ (so the loss in dB per 90°) where it is above the arithmetic's reach: TE at 2 and 3 µm (3.1e-7 and 2.0e-11 dB per 90°), TM at 2, 3 and 5 µm (0.19, 0.017 and 8.0e-5 dB per 90°), Richardson's limit against the exact (largest relative error shown)",
            tier: Tier::Analytic,
            source: r"as `eme/bend-index`: the exact bent slab's $\operatorname{Im}\nu$, the light it radiates through the outgoing Hankel function; here through the PML, the mapped profile continued analytically into it",
            run: || {
                let (measured, own) = loss_case();
                crate::verdict::note_numerical(own);
                zero(measured, LOSS_TOLERANCE)
            },
        },
        Case {
            id: "eme/bend-order",
            title: r"The thin layers' order: the same bend at 2 µm, TE and TM, its $n_\text{eff}$ with layers at most 10 nm thick and 2, 4 and 8 times as many, each against the exact: the observed order furthest from 2 (shown)",
            tier: Tier::Analytic,
            source: r"second order: each layer the map's index at its middle, the midpoint rule's staircase; each region's count of layers a power of two, so that halving the step doubles every count and the staircases are nested",
            run: || {
                let order = (100.0 * order_case()).round() / 100.0;
                Outcome {
                    measured: order,
                    expected: 2.0,
                    tolerance: 0.1,
                    error: (order - 2.0).abs(),
                }
            },
        },
        Case {
            id: "eme/bend-halves",
            title: r"A 180° turn made of two bends: the same slab at 2 µm between two straights, TE and TM, 12 modes, a 90° bend joined to another by their overlaps against one 180° bend (largest $\lvert \Delta S_{qp} \rvert$ over the four blocks shown)",
            tier: Tier::Analytic,
            source: r"a uniform section's modes are orthonormal in the unconjugated product, so the joint between two halves of one bend is the identity and the cascade of the halves is the whole: round-off",
            run: || zero(halves_case(), 1e-12),
        },
        Case {
            id: "eme/bend-no-angle",
            title: r"A bend of no angle between two straights: the same slab at 2 and 3 µm, TE and TM, 24 modes, the straight-to-bend joint and back, its S between the straights' fundamental modes against the straight's ($\max(\lvert S_{21} - 1 \rvert, \lvert S_{11} \rvert)$ shown; with 6, 12, 16 and 24 modes at 2 µm TE: 3.2e-3, 2.5e-3, 1.3e-4, 3.5e-6)",
            tier: Tier::Analytic,
            source: r"as the angle goes to 0 the bend leaves only its two joints, whose overlaps are inverse with every mode: what remains is the truncation of two bases (`eme/grating-squaring`'s method page: a zero-length section between two others is not the identity with finitely many modes), and it falls with the modes",
            run: || zero(no_angle_case(), 1e-4),
        },
        Case {
            id: "eme/bend-joint",
            title: r"The joint between a straight and a bend: the same slab, TE at 2, 3 and 5 µm and TM at 5 µm, the fundamental modes' coupling $\eta = \langle E_s, H_b \rangle \langle E_b, H_s \rangle / (\langle E_s, H_s \rangle \langle E_b, H_b \rangle)$ by the expansion's overlaps (the bend's mode at 10 and 5 nm layers, Richardson's limit) against the exact bent field's overlap with the straight slab's mode (largest $\lvert \Delta \eta \rvert$ shown; $1 - \eta$ is 2.8e-3 at 2 µm TE)",
            tier: Tier::Analytic,
            source: r"`mode::bend`'s field $\psi(\rho)$ at its exact $\nu$, by the same radial integration (Runge–Kutta, its steps halved), its tangential H (TE) or E (TM) $n_\text{eff}(R/\rho)\psi$, against the straight slab's mode, both integrated across the window by Gauss–Legendre; the bends whose radiation is under 1e-6 of their power per µm, so that the window holds the mode",
            run: || {
                let (measured, own) = joint_case();
                crate::verdict::note_numerical(own);
                zero(measured, JOINT_TOLERANCE)
            },
        },
    ]
}

/// `eme/bend-index`'s tolerance: three times the largest own error, rounded up.
pub(crate) const INDEX_TOLERANCE: f64 = 1e-7;
/// `eme/bend-loss`'s tolerance, relative: three times the largest own error, rounded up.
pub(crate) const LOSS_TOLERANCE: f64 = 1e-3;
/// `eme/bend-joint`'s tolerance: three times the largest own error, rounded up.
pub(crate) const JOINT_TOLERANCE: f64 = 1e-6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "prints the numbers"]
    fn the_case_numbers() {
        let t = std::time::Instant::now();
        eprintln!("index {:?} {:.1} s", index_case(), t.elapsed().as_secs_f64());
        let t = std::time::Instant::now();
        eprintln!("loss {:?} {:.1} s", loss_case(), t.elapsed().as_secs_f64());
        let t = std::time::Instant::now();
        for pol in BOTH {
            eprintln!("orders {pol:?} {:?}", orders(pol, 2.0));
        }
        eprintln!("order {} {:.1} s", order_case(), t.elapsed().as_secs_f64());
        let t = std::time::Instant::now();
        eprintln!("halves {:e} {:.1} s", halves_case(), t.elapsed().as_secs_f64());
        let t = std::time::Instant::now();
        for modes in [6, 12, 16, 24] {
            eprintln!(
                "no angle {modes}: TE2 {:e} TM2 {:e}",
                no_angle(Polarization::Te, 2.0, modes),
                no_angle(Polarization::Tm, 2.0, modes)
            );
        }
        eprintln!("no angle {:e} {:.1} s", no_angle_case(), t.elapsed().as_secs_f64());
        let t = std::time::Instant::now();
        eprintln!("joint {:?} {:.1} s", joint_case(), t.elapsed().as_secs_f64());
        for (pol, r) in LOSSY {
            let n = exact_index(pol, r);
            eprintln!("{pol:?} {r}: {n:.12} {:.4e} dB/90", db_per_quarter(n, middle(r)));
        }
        for (pol, r) in JOINTS {
            eprintln!("{pol:?} {r}: 1 - eta {:.4e}", 1.0 - exact_coupling(pol, r, 0.5).norm());
        }
    }

    #[test]
    #[ignore = "prints the numbers"]
    fn the_bend_case_study() {
        for pol in [Polarization::Te, Polarization::Tm] {
            for r in [2.0, 3.0, 5.0, 10.0, 20.0] {
                let t = std::time::Instant::now();
                let (want, got, own) = index_limit(pol, r);
                let own_exact = exact_error(pol, r);
                eprintln!("{pol:?} R {r}: the exact's own error {own_exact:.2e}");
                eprintln!(
                    "{pol:?} R {r}: exact {want:.12} limit {got:.12} err {:.2e} (re {:.2e} im {:.2e}) own {own:.2e}; loss {:.6e} dB/90 vs {:.6e} rel {:.2e}; {:.1} s",
                    (got - want).norm(),
                    got.re - want.re,
                    got.im - want.im,
                    db_per_quarter(got, middle(r)),
                    db_per_quarter(want, middle(r)),
                    got.im / want.im - 1.0,
                    t.elapsed().as_secs_f64()
                );
            }
            eprintln!("{pol:?} orders at 2: {:?}", orders(pol, 2.0));
            eprintln!("{pol:?} orders at 5: {:?}", orders(pol, 5.0));
        }
    }

    #[test]
    #[ignore = "prints the numbers"]
    fn the_robustness_study() {
        for pol in [Polarization::Tm, Polarization::Te] {
            for r in [2.0, 3.0, 5.0, 10.0, 20.0] {
                for modes in [4, 6, 8, 12] {
                    let mut failed = Vec::new();
                    for f in [1.0, 0.5, 0.25, 0.125] {
                        for scale in [1.0, 2.0] {
                            let eme = settings(pol, modes, f * STEP, scale);
                            if let Err(e) = eme.modes(&bent(r)) {
                                failed.push(format!("{f}/{scale}: {e}"));
                            }
                        }
                    }
                    eprintln!("{pol:?} R {r} {modes} modes: {} failed {:?}", failed.len(), failed.first());
                }
            }
        }
    }

    #[test]
    #[ignore = "prints the numbers"]
    fn the_orthogonality_study() {
        let pol = if std::env::var("TM").is_ok() {
            Polarization::Tm
        } else {
            Polarization::Te
        };
        let r: f64 = std::env::var("R").ok().and_then(|v| v.parse().ok()).unwrap_or(2.0);
        let modes: usize = std::env::var("M").ok().and_then(|v| v.parse().ok()).unwrap_or(6);
        let eme = settings(pol, modes, STEP, 1.0);
        let b = match eme.modes(&bent(r)) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };
        let o = overlaps(&b, &b);
        for i in 0..b.len() {
            let row: Vec<String> = (0..b.len())
                .map(|j| {
                    let want = if i == j { 1.0 } else { 0.0 };
                    format!("{:.1e}", (o[(i, j)] - want).norm())
                })
                .collect();
            eprintln!("{i}: n {:.8} | {}", b[i].effective_index(), row.join(" "));
        }
        let xs: Vec<f64> = (0..=40).map(|j| j as f64 * 0.1).collect();
        let psi = exact(r).field(pol, lam(), exact_index(pol, r), &xs);
        eprintln!("exact field: {:?}", &psi[..8]);
    }

    #[test]
    #[ignore = "prints the numbers"]
    fn the_joint_study() {
        for pol in [Polarization::Te, Polarization::Tm] {
            for r in [2.0, 3.0, 5.0] {
                for modes in [6, 12, 16, 24] {
                    let t = std::time::Instant::now();
                    let h = halves(pol, r, modes);
                    let zero = turn(pol, r, &[0.0], modes);
                    let _ = &zero;
                    eprintln!(
                        "{pol:?} R {r} {modes} modes: halves {h:.2e}; theta 0: |S21-1| {:.3e} |S11| {:.3e} |S21|^2 {:.9}; {:.1} s",
                        (zero.s21[(0, 0)] - 1.0).norm(),
                        zero.s11[(0, 0)].norm(),
                        zero.s21[(0, 0)].norm_sqr(),
                        t.elapsed().as_secs_f64()
                    );
                }
                let c: Vec<c64> = [STEP, 0.5 * STEP, 0.25 * STEP]
                    .iter()
                    .map(|&s| coupling(pol, r, s))
                    .collect();
                let (l1, l2) = (limit(c[0], c[1]), limit(c[1], c[2]));
                let (e1, e2) = (exact_coupling(pol, r, 1.0), exact_coupling(pol, r, 0.5));
                eprintln!(
                    "{pol:?} R {r}: coupling {:.10} limits {l1:.10} {l2:.10} exact {e2:.10} diffs {:.2e} {:.2e} own {:.2e} exact own {:.2e}",
                    c[0],
                    (l1 - e2).norm(),
                    (l2 - e2).norm(),
                    (l1 - l2).norm(),
                    (e1 - e2).norm() / 15.0
                );
            }
        }
    }
}
