//! The validation cases of sidewall-roughness loss ([`super::roughness`]): Payne and Lacey's
//! slab against the volume current on the exact slab mode, their Eq. 9's factor of 2, and the
//! 3D analysis's convergence on the grid.

use num_complex::Complex64 as c64;

use super::roughness::{self, Roughness};
use super::vector::{Boundaries, Boundary, CrossSection, Permittivity};
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

/// The slabs of the slab cases: a weakly guiding one (SiON-like, 1.50 in 1.45, 2 µm), silicon
/// nitride (2.00 in 1.45, 0.4 µm) and silicon (3.50 in 1.45, 0.2 µm), all single-mode at
/// 1.55 µm: Barwicz and Haus's three cores.
pub(crate) const SLABS: [(f64, f64, f64); 3] =
    [(1.5, 1.45, 2.0), (2.0, 1.45, 0.4), (3.5, 1.45, 0.2)];

/// The correlation lengths of the slab cases, nm: Barwicz and Haus's 50, and a decade each side.
const CORRELATIONS: [f64; 3] = [5.0, 50.0, 500.0];

fn lam() -> Wavelength {
    Wavelength::from_um_unchecked(1.55)
}

/// A rectangular core `w` × `h` µm of index `n_core` in `n_clad` on a uniform grid of `step`
/// µm with `margin` µm of cladding, as Hadley's equations take it: its quarter x, y ≥ 0 behind
/// mirror walls for the TE-like modes (`te`, an electric wall on x = 0 and a magnetic one on
/// y = 0) or the TM-like ones, or the whole of it (`full`).
pub(crate) fn rectangle(
    (w, h): (f64, f64),
    (n_core, n_clad): (f64, f64),
    step: f64,
    margin: f64,
    te: bool,
    full: bool,
) -> CrossSection {
    let nx = ((w / 2.0 + margin) / step).round() as usize;
    let ny = ((h / 2.0 + margin) / step).round() as usize;
    let (xe, ye) = (nx as f64 * step, ny as f64 * step);
    let eps = |x: f64, y: f64| {
        let n = if x.abs() < w / 2.0 && y.abs() < h / 2.0 {
            n_core
        } else {
            n_clad
        };
        Permittivity::isotropic(c64::new(n * n, 0.0))
    };
    let cs = if full {
        CrossSection::uniform((-xe, xe, 2 * nx), (-ye, ye, 2 * ny), eps)
    } else {
        CrossSection::uniform((0.0, xe, nx), (0.0, ye, ny), eps)
    }
    .expect("a cross-section");
    if full {
        return cs;
    }
    let (west, south) = if te {
        (Boundary::ElectricWall, Boundary::MagneticWall)
    } else {
        (Boundary::MagneticWall, Boundary::ElectricWall)
    };
    cs.with_boundaries(Boundaries {
        west,
        south,
        ..Boundaries::default()
    })
    .expect("mirror walls")
}

/// Payne and Lacey's Eq. 9 over our exact slab's φ²(d), normalized to ∫ φ² = 1, the one
/// furthest from 2 of the three slabs.
pub(crate) fn eq9_ratio() -> f64 {
    let r = Roughness::exponential(Length::nm(1.0), Length::nm(50.0)).expect("roughness");
    SLABS
        .iter()
        .map(|&(n1, n2, t)| {
            let t = Length::um(t);
            let ours = roughness::slab_loss(n1, n2, t, lam(), &r).expect("a slab");
            let pl = roughness::payne_lacey(n1, n2, t, lam(), &r).expect("a slab");
            pl.eq9 / ours.field_at_wall
        })
        .fold(2.0, |worst: f64, v| {
            if (v - 2.0).abs() > (worst - 2.0).abs() {
                v
            } else {
                worst
            }
        })
}

/// The largest relative difference of the volume current on our exact slab mode from Payne
/// and Lacey's Eq. 10, its 1/n₁ read as k₀/β, over the three slabs and three correlation lengths.
pub(crate) fn slab_error() -> f64 {
    let mut worst = 0.0f64;
    for lc in CORRELATIONS {
        let r = Roughness::exponential(Length::nm(1.0), Length::nm(lc)).expect("roughness");
        for (n1, n2, t) in SLABS {
            let t = Length::um(t);
            let ours = roughness::slab_loss(n1, n2, t, lam(), &r).expect("a slab");
            let pl = roughness::payne_lacey(n1, n2, t, lam(), &r).expect("a slab");
            let closed = pl.eq10 * n1 / ours.effective_index;
            worst = worst.max((ours.alpha - closed).abs() / closed);
        }
    }
    worst
}

/// The grids of the convergence case, µm: Hadley's equations' uniform grids, halving.
pub(crate) const STEPS: [f64; 3] = [0.01, 0.005, 0.0025];

/// Barwicz and Haus's silicon guide (200 nm high, 400 nm wide, 3.50 in 1.45, 1.55 µm), its
/// TE-like mode by Hadley's equations on the quarter with 0.5 µm of cladding, σ = 1 nm and
/// L_c = 50 nm: its loss, dB/cm, on each of [`STEPS`], as a grid study.
pub(crate) fn silicon_study() -> crate::convergence::Study {
    let r = Roughness::exponential(Length::nm(1.0), Length::nm(50.0)).expect("roughness");
    let mut study = crate::convergence::Study::new("the silicon guide's roughness loss, dB/cm");
    for step in STEPS {
        let cs = rectangle((0.4, 0.2), (3.5, 1.45), step, 0.5, true, false);
        let mode = crate::mode::hadley::modes(&cs, lam(), 1, None).expect("a mode");
        let loss = roughness::sidewall_loss(&cs, &mode[0], lam(), &r).expect("a loss");
        study = study.on(step, loss.db_per_cm()).expect("a grid");
    }
    study
}

/// The corner's exponent's 2τ − 1 for silicon in silica at Barwicz and Haus's indices: the order
/// of the singular part of the wall's ∫ |E|² over the last cells.
pub(crate) fn corner_order() -> f64 {
    2.0 * roughness::corner_exponent(3.5 * 3.5, 1.45 * 1.45) - 1.0
}

fn zero(measured: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected: 0.0,
        tolerance,
        error: measured,
    }
}

/// Every sidewall-roughness case, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "roughness/payne-lacey-eq9",
            title: r"Payne and Lacey's Eq. 9 for a symmetric slab's TE₀ field at its wall, $\varphi^2(d) = 2V(1 - b)/(d(V + 1/\sqrt b))$, over our exact slab's field normalized to $\int \varphi^2 = 1$, for SiON-like (1.50), Si₃N₄ (2.00) and Si (3.50) slabs in 1.45 at 1.55 µm (the ratio furthest from 2 shown): Eq. 9 carries a factor of 2, as Barwicz and Haus say",
            tier: Tier::Analytic,
            source: r"photonoxide's exact slab mode, normalized by Gauss–Legendre sums; F. P. Payne, J. P. R. Lacey, Opt. Quantum Electron. 26, 977 (1994), doi:10.1007/BF00708339, Eq. 9; T. Barwicz, H. A. Haus, doi:10.1109/JLT.2005.850816, p. 2729",
            run: || {
                let ratio = eq9_ratio();
                Outcome {
                    measured: ratio,
                    expected: 2.0,
                    tolerance: 2e-10,
                    error: (ratio - 2.0).abs(),
                }
            },
        },
        Case {
            id: "roughness/payne-lacey-slab",
            title: r"Sidewall-roughness loss of a symmetric slab's TE₀ mode, both walls rough, by the volume current on the exact mode (each wall a sheet radiating by the 2D Green's function, the far field and the mode's power by quadrature), against Payne and Lacey's closed form, Eq. 10, its $1/n_1$ read as $k_0/\beta$: the three slabs of roughness/payne-lacey-eq9 with $L_c$ = 5, 50 and 500 nm (largest relative difference shown); Eq. 1 with the exact $\varphi^2(d)$ is half of it",
            tier: Tier::Analytic,
            source: r"F. P. Payne, J. P. R. Lacey, Opt. Quantum Electron. 26, 977 (1994), doi:10.1007/BF00708339, Eqs. 8–12: $\alpha = \sigma^2 g(V) f_e(x, \gamma)/(\sqrt 2 k_0 d^4 n_1)$, Eq. 1's $1/n_1$ the weakly guiding $\beta \approx n_1 k_0$",
            run: || zero(slab_error(), 1e-10),
        },
        Case {
            id: "roughness/silicon-grid-order",
            title: r"Sidewall-roughness loss by Barwicz and Haus's 3D analysis of their silicon guide (200 × 400 nm, 3.50 in 1.45, 1.55 µm, TE-like, $L_c$ = 50 nm), its mode by Hadley's equations on 10, 5 and 2.5 nm grids, 0.4039, 0.4144 and 0.4213 (dB/cm)/nm² (Richardson's limit 0.434): the order of convergence of the changes (shown)",
            tier: Tier::Analytic,
            source: r"$2\tau - 1$ = 0.54, the order of the singular part the corners' rows hold, $\lvert E \rvert^2 \propto r^{2\tau - 2}$ at a right-angled dielectric corner (J. Meixner, doi:10.1109/TAP.1972.1140243): $\tau$ = 0.770 for 3.50 in 1.45; the order falls from 0.66 (20, 10 and 5 nm) towards it",
            run: || {
                let s = silicon_study();
                let v = &s.values;
                let order = ((v[1] - v[0]) / (v[2] - v[1])).log2();
                // two decimals: the digits that hold on every system
                let order = (100.0 * order).round() / 100.0;
                let expected = corner_order();
                Outcome {
                    measured: order,
                    expected,
                    tolerance: 0.1,
                    error: (order - expected).abs(),
                }
            },
        },
    ]
}
