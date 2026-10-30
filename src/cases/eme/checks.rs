//! Eigenmode expansion's validation cases, shared by the tests and the validation report.

use num_complex::Complex64 as c64;

use super::{Eme, Section, Slice, Wall};
use crate::mode::Polarization;
use crate::mode::slab::Slab;
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

/// The step's silicon slab in oxide, at 1.55 µm: core, cladding.
const CORE: f64 = 3.473;
const CLAD: f64 = 1.444;

fn lam() -> Wavelength {
    Wavelength::from_um_unchecked(1.55)
}

fn re(n: f64) -> c64 {
    c64::new(n, 0.0)
}

/// A slab `t` µm thick centred in a window `width` µm wide.
fn slab(t: f64, width: f64) -> Slice {
    Slice::slab(re(CORE), Length::um(t), re(CLAD), Length::um(width)).expect("a valid slab")
}

/// Half of a slab `t` µm thick: its upper half, the plane of symmetry at the lower wall, in a
/// half-window `width` µm wide.
pub(crate) fn half(t: f64, width: f64) -> Slice {
    Slice::new(&[
        (re(CORE), Length::um(0.5 * t)),
        (re(CLAD), Length::um(width - 0.5 * t)),
    ])
    .expect("a valid half slab")
}

/// One mode on each side of a step from a 220 nm slab to a 300 nm one, mode-matching's closed
/// form: with the even modes U = cos(hx) in the core and cos(ha) e^(−q(|x| − a)) outside (a the
/// half-thickness; Slab's exact n_eff) and C = ∫U₁U₂w / (∫U₁²w₁ ∫U₂²w₂)^½, the weight w = 1 for TE
/// and 1/n² of the first slab for TM (w₁, w₂ each slab's own), the transformer of the module's
/// third point with O = (n₂/n₁)^(±½) C gives R = (n₁ − C²n₂)/(n₁ + C²n₂) and
/// T = 2C(n₁n₂)^½/(n₁ + C²n₂) for TE, and n₁ and n₂ exchanged in both for TM. Every integral in
/// closed form. Returns (R, |T|) closed and from the expansion, its walls 3 µm from the core.
pub(crate) fn one_mode_step(polarization: Polarization) -> ((f64, f64), (f64, f64)) {
    let (a1, a2) = (0.11, 0.15);
    let k = lam().wavenumber();
    let mode = |t: f64| {
        let n = Slab::new(CLAD, CORE, CLAD, Length::um(t))
            .expect("a valid slab")
            .modes(polarization, lam())[0]
            .effective_index();
        let h = k * (CORE * CORE - n * n).sqrt();
        let q = k * (n * n - CLAD * CLAD).sqrt();
        (n, h, q)
    };
    let ((n1, h1, q1), (n2, h2, q2)) = (mode(2.0 * a1), mode(2.0 * a2));
    let (w_core, w_clad) = match polarization {
        Polarization::Te => (1.0, 1.0),
        Polarization::Tm => (1.0 / (CORE * CORE), 1.0 / (CLAD * CLAD)),
    };
    // ∫_0^a cos(h x) cos(g x) dx
    let cc = |a: f64, h: f64, g: f64| {
        let s = |w: f64| {
            if w.abs() < 1e-12 {
                a
            } else {
                (w * a).sin() / w
            }
        };
        0.5 * (s(h - g) + s(h + g))
    };
    // the even field on x > 0, x = 0 at the centre
    let (c1, c2) = ((h1 * a1).cos(), (h2 * a2).cos());
    // ∫ over x > 0 of U₁² w and U₂² w, each with its own slab's indices
    let self1 = cc(a1, h1, h1) * w_core + c1 * c1 / (2.0 * q1) * w_clad;
    let self2 = cc(a2, h2, h2) * w_core + c2 * c2 / (2.0 * q2) * w_clad;
    // ∫ U₁ U₂ w with the first slab's indices: the core to a1, U₁'s tail times U₂'s core to a2,
    // the two tails beyond
    let middle = {
        // ∫_a1^a2 e^(−q₁(x − a₁)) cos(h₂x) dx, the real part of a complex exponential's integral
        let z = c64::new(-q1, h2);
        let f = |x: f64| (z * x).exp() * (q1 * a1).exp() / z;
        (f(a2) - f(a1)).re
    };
    let cross = cc(a1, h1, h2) * w_core
        + c1 * middle * w_clad
        + c1 * c2 * (-q1 * (a2 - a1)).exp() / (q1 + q2) * w_clad;
    let c = cross / (self1 * self2).sqrt();
    let (p, s) = match polarization {
        Polarization::Te => (n1, n2),
        Polarization::Tm => (n2, n1),
    };
    let r = (p - c * c * s) / (p + c * c * s);
    let t = 2.0 * c * (n1 * n2).sqrt() / (p + c * c * s);
    let eme = Eme::new(polarization, lam(), 1).expect("one mode");
    let sections = [
        Section::new(slab(2.0 * a1, 6.0), Length::ZERO),
        Section::new(slab(2.0 * a2, 6.0), Length::ZERO),
    ];
    let got = eme.s_matrix(&sections).expect("the step's S-matrix");
    ((r, t.abs()), (got.s11[0][0].re, got.s21[0][0].norm()))
}

/// The largest relative difference of [`one_mode_step`]'s R and |T| from the closed form, TE
/// and TM.
fn one_mode_step_error() -> f64 {
    [Polarization::Te, Polarization::Tm]
        .into_iter()
        .map(|p| {
            let ((r, t), (er, et)) = one_mode_step(p);
            ((er - r) / r).abs().max(((et - t) / t).abs())
        })
        .fold(0.0, f64::max)
}

/// A step between two uniform media, 1.444 to 3.473, in a closed window 3 µm wide (electric
/// walls for TE, magnetic for TM, so that no mode is uniform across), 20 modes: each mode m
/// meets its own on the far side, sin(mπx/W) on both, with Fresnel's reflection at its angle,
/// for the tangential E: (β₁ − β₂)/(β₁ + β₂) for TE, (n₁²β₂ − n₂²β₁)/(n₁²β₂ + n₂²β₁) for TM
/// (−r_p, which is H's), β = (n²k² − (mπ/W)²)^½, complex past the critical angle. The largest
/// difference from Fresnel's on the diagonal of S₁₁, and of the rest of S₁₁ from 0.
pub(crate) fn fresnel(polarization: Polarization) -> f64 {
    let (n1, n2, w, m) = (CLAD, CORE, 3.0, 20);
    let wall = match polarization {
        Polarization::Te => Wall::Electric,
        Polarization::Tm => Wall::Magnetic,
    };
    let eme = Eme::new(polarization, lam(), m)
        .expect("modes")
        .with_walls(wall, wall);
    let uniform = |n: f64| Slice::new(&[(re(n), Length::um(w))]).expect("a uniform slice");
    let s = eme
        .s_matrix(&[
            Section::new(uniform(n1), Length::ZERO),
            Section::new(uniform(n2), Length::ZERO),
        ])
        .expect("the step's S-matrix");
    let k = lam().wavenumber();
    let mut worst: f64 = 0.0;
    for i in 0..m {
        let kx = (i + 1) as f64 * std::f64::consts::PI / w;
        let beta = |n: f64| {
            let b = re(n * n * k * k - kx * kx).sqrt();
            if b.im < 0.0 { -b } else { b }
        };
        let (b1, b2) = (beta(n1), beta(n2));
        let r = match polarization {
            Polarization::Te => (b1 - b2) / (b1 + b2),
            Polarization::Tm => (n1 * n1 * b2 - n2 * n2 * b1) / (n1 * n1 * b2 + n2 * n2 * b1),
        };
        for j in 0..m {
            let expected = if i == j { r } else { c64::new(0.0, 0.0) };
            worst = worst.max((s.s11[i][j] - expected).norm());
        }
    }
    worst
}

/// A lossless structure in a closed window (electric walls, no PML): a 220 nm slab stepping to
/// 300 nm through 240, 260 and 280 nm, each 0.1 µm long, in a window 3 µm wide, 30 modes, TE
/// and TM. Its S-matrix's largest departures from symmetry, |S − Sᵀ| over all 60 × 60 entries
/// (reciprocity), and from unitarity on the propagating modes, |S_pᴴS_p − 1| (power).
pub(crate) fn lossless(polarization: Polarization) -> (f64, f64) {
    let m = 30;
    let mut sections = vec![Section::new(slab(0.22, 3.0), Length::ZERO)];
    for t in [0.24, 0.26, 0.28] {
        sections.push(Section::new(slab(t, 3.0), Length::um(0.1)));
    }
    sections.push(Section::new(slab(0.30, 3.0), Length::ZERO));
    let s = Eme::new(polarization, lam(), m)
        .expect("modes")
        .s_matrix(&sections)
        .expect("the S-matrix");
    // the whole S, ports 1 then 2
    let at = |i: usize, j: usize| match (i < m, j < m) {
        (true, true) => s.s11[i][j],
        (true, false) => s.s12[i][j - m],
        (false, true) => s.s21[i - m][j],
        (false, false) => s.s22[i - m][j - m],
    };
    let mut symmetry: f64 = 0.0;
    for i in 0..2 * m {
        for j in 0..2 * m {
            symmetry = symmetry.max((at(i, j) - at(j, i)).norm());
        }
    }
    // the propagating modes of each port, n_eff real
    let real = |n: &[c64]| -> Vec<usize> {
        (0..m)
            .filter(|&i| n[i].im == 0.0 && n[i].re > 0.0)
            .collect()
    };
    let open: Vec<usize> = real(&s.first)
        .into_iter()
        .chain(real(&s.last).into_iter().map(|i| i + m))
        .collect();
    let mut unitarity: f64 = 0.0;
    for &a in &open {
        for &b in &open {
            let sum: c64 = open.iter().map(|&q| at(q, a).conj() * at(q, b)).sum();
            let expected = if a == b { 1.0 } else { 0.0 };
            unitarity = unitarity.max((sum - expected).norm());
        }
    }
    (symmetry, unitarity)
}

/// The expansion's settings for half a slab (TE's even modes: a magnetic wall at the plane of
/// symmetry), 1.5 µm to the upper wall, a PML 4 µm thick of strength 0.1 inside it.
fn half_te(modes: usize) -> Eme {
    Eme::new(Polarization::Te, lam(), modes)
        .expect("modes")
        .with_walls(Wall::Magnetic, Wall::Electric)
        .with_upper_pml(Length::um(4.0), 0.1)
        .expect("a PML")
}

/// The stepped taper's thickness at x (µm): 220 nm up to x = 0.8, then 240, 260 and 280 nm
/// each 0.1 µm long, then 300 nm; on 10, 5 and 2.5 nm grids alike.
fn stepped(x: f64) -> f64 {
    if x < 0.8 {
        0.22
    } else if x < 1.1 {
        0.24 + 0.02 * ((x - 0.8) / 0.1).floor()
    } else {
        0.30
    }
}

/// The stepped taper's S₁₁ and S₂₁ between the two slabs' fundamental modes by the expansion
/// with `modes` even modes (the ports at the taper's two ends: the moduli are FDFD's).
pub(crate) fn stepped_eme(modes: usize) -> (c64, c64) {
    let mut sections = vec![Section::new(half(0.22, 1.5), Length::ZERO)];
    for t in [0.24, 0.26, 0.28] {
        sections.push(Section::new(half(t, 1.5), Length::um(0.1)));
    }
    sections.push(Section::new(half(0.30, 1.5), Length::ZERO));
    let s = half_te(modes).s_matrix(&sections).expect("the S-matrix");
    (s.s11[0][0], s.s21[0][0])
}

/// The stepped taper's |S₁₁|² by 2D FDFD (E along z, the guides' fundamental port modes at 0.3
/// and 1.7 µm, PMLs of 20 cells beyond ±1.5 µm) on an `h` grid.
pub(crate) fn stepped_fdfd(h: f64) -> f64 {
    use crate::fdfd::checks::{guide, two_ports};
    let solver = guide(crate::fdfd::Polarization::Ez, h, 2.0, (CORE, CLAD), stepped);
    let ports = two_ports(&solver);
    solver.s_matrix(&ports).expect("the S-matrix")[0][0].norm_sqr()
}

/// The stepped taper's reflection by the expansion with 200 modes against FDFD's grid limit,
/// Richardson's from 10, 5 and 2.5 nm; the expansion's own error noted, its change from 160
/// modes. (The expansion's reflection with 40, 80, 120, 160 and 200 modes: 9.1385, 9.1752,
/// 9.1801, 9.1816 and 9.1819e-5; FDFD's 9.1901, 9.1838 and 9.1823e-5, second order, their
/// limit 9.1817e-5. A PML of 0.2 or 0.4 µm moves FDFD's by under 1e-12; the expansion's
/// needs one 4 µm thick: 1 µm of strength 0.025 to 0.1 leaves it 1e-3 to 6e-4 low, the
/// grazing radiation reflected.)
fn taper_fdfd() -> Outcome {
    let fdfd = [0.01, 0.005, 0.0025].map(stepped_fdfd);
    let (d1, d2) = (fdfd[1] - fdfd[0], fdfd[2] - fdfd[1]);
    let limit = fdfd[2] + d2 * d2 / (d1 - d2);
    let fine = stepped_eme(200).0.norm_sqr();
    let coarse = stepped_eme(160).0.norm_sqr();
    // to the digits that hold on every system: a PML's high modes are ill-conditioned
    crate::validation::note_numerical(significant(((fine - coarse) / fine).abs(), 2));
    let fine = significant(fine, 5);
    Outcome {
        measured: fine,
        expected: limit,
        tolerance: 2e-4 * limit,
        error: (fine - limit).abs(),
    }
}

/// A smooth taper: a slab widening linearly from 220 to 600 nm over 1 µm, between half-slabs
/// of 220 and 600 nm, cut into `steps` sections, `modes` even modes: its S₂₁.
pub(crate) fn linear_taper(steps: usize, modes: usize) -> c64 {
    let mut sections = vec![Section::new(half(0.22, 1.5), Length::ZERO)];
    sections.extend(
        Section::sampled(Length::um(1.0), steps, |z| {
            Ok(half(0.22 + 0.38 * z.to_um(), 1.5))
        })
        .expect("sections"),
    );
    sections.push(Section::new(half(0.60, 1.5), Length::ZERO));
    half_te(modes)
        .s_matrix(&sections)
        .expect("the S-matrix")
        .s21[0][0]
}

/// The smooth taper's S₂₁ on 40, 80 and 160 steps (60 modes): the order of convergence of its
/// changes, |S(80) − S(40)| / |S(160) − S(80)| in log₂.
pub(crate) fn steps_order() -> f64 {
    let s = [40, 80, 160].map(|n| linear_taper(n, 60));
    ((s[1] - s[0]).norm() / (s[2] - s[1]).norm()).log2()
}

/// The smooth taper's |S₂₁|² on 40 steps with 40, 80 and 160 modes, a grid study in 1/N
/// ([`crate::convergence`]): its estimated error at 160, relative.
pub(crate) fn modes_estimate() -> f64 {
    let mut study = crate::convergence::Study::new("the taper's |S21|^2");
    for n in [40, 80, 160] {
        study = study
            .on(1.0 / n as f64, linear_taper(40, n).norm_sqr())
            .expect("a study");
    }
    let c = study.check(1.0).expect("a check");
    c.error.unwrap_or(f64::INFINITY) / c.value
}

///  to digits significant digits.
fn significant(v: f64, digits: i32) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let scale = 10f64.powi(digits - 1 - v.abs().log10().floor() as i32);
    (v * scale).round() / scale
}

fn zero(measured: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected: 0.0,
        tolerance,
        error: measured,
    }
}

/// Every eigenmode-expansion case, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "eme/step-one-mode",
            title: r"Eigenmode expansion with one mode a side: a silicon slab (3.473 in 1.444, 1.55 µm) stepping from 220 to 300 nm, its walls 3 µm from the core, TE and TM: $R$ and $\lvert T \rvert$ (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"mode-matching's closed form for one mode a side: $R = (n_1 - C^2 n_2)/(n_1 + C^2 n_2)$, $T = 2C\sqrt{n_1 n_2}/(n_1 + C^2 n_2)$ (TE; $n_1$ and $n_2$ exchanged for TM), $C$ the slabs' normalized overlap, from the exact slab's $n_\text{eff}$ and its fields' integrals in closed form",
            run: || zero(one_mode_step_error(), 1e-10),
        },
        Case {
            id: "eme/step-fresnel-te",
            title: r"A step between two uniform media, 1.444 to 3.473, in a closed window 3 µm wide, 20 modes, TE: each mode against its own across the step (largest difference of $S_{11}$ from Fresnel's diagonal shown)",
            tier: Tier::Analytic,
            source: r"Fresnel's $r_s = (\beta_1 - \beta_2)/(\beta_1 + \beta_2)$ at each mode's angle, $\beta = \sqrt{n^2 k^2 - (m\pi/W)^2}$, complex past the critical angle; no coupling between modes",
            run: || zero(fresnel(Polarization::Te), 1e-12),
        },
        Case {
            id: "eme/step-fresnel-tm",
            title: r"The same with TM, magnetic walls (largest difference of $S_{11}$ from Fresnel's diagonal shown)",
            tier: Tier::Analytic,
            source: r"Fresnel's coefficient for the tangential $E$, $-r_p = (n_1^2\beta_2 - n_2^2\beta_1)/(n_1^2\beta_2 + n_2^2\beta_1)$, at each mode's angle",
            run: || zero(fresnel(Polarization::Tm), 1e-12),
        },
        Case {
            id: "eme/lossless-reciprocity",
            title: r"A lossless stepped taper in a closed window (220 to 300 nm in four steps 0.1 µm apart, 3 µm wide, 30 modes, TE and TM): the S-matrix's largest departure from symmetry, $\lvert S - S^T \rvert$ over all its $60 \times 60$ entries (shown)",
            tier: Tier::Analytic,
            source: r"Lorentz reciprocity: the interfaces' S-matrices are symmetric for any number of modes, with the modes normalized by the unconjugated product, and the star product keeps it",
            run: || {
                let worst = [Polarization::Te, Polarization::Tm]
                    .into_iter()
                    .map(|p| lossless(p).0)
                    .fold(0.0, f64::max);
                zero(worst, 1e-12)
            },
        },
        Case {
            id: "eme/lossless-power",
            title: r"The same taper: the propagating modes' block of $S$ against unitarity, $\lvert S_p^\dagger S_p - 1 \rvert$ (largest entry shown)",
            tier: Tier::Analytic,
            source: r"power conservation: each interface conserves the complex power exactly for any number of modes (an ideal transformer between the two sets of modes), and the evanescent modes carry none out of the ports",
            run: || {
                let worst = [Polarization::Te, Polarization::Tm]
                    .into_iter()
                    .map(|p| lossless(p).1)
                    .fold(0.0, f64::max);
                zero(worst, 1e-12)
            },
        },
        Case {
            id: "eme/taper-steps-order",
            title: r"A smooth taper: a silicon slab widening linearly from 220 to 600 nm over 1 µm (TE, even modes, 60 of them, a PML 4 µm thick), cut into 40, 80 and 160 uniform sections: the order of convergence of $S_{21}$'s changes (shown)",
            tier: Tier::Analytic,
            source: r"second order: each section is the cross-section at its middle, the midpoint rule's staircase; 2.18 from 10, 20 and 40 sections, 2.07 from 20, 40 and 80",
            run: || {
                // to two decimals, the digits that hold on every system
                let order = (100.0 * steps_order()).round() / 100.0;
                Outcome {
                    measured: order,
                    expected: 2.0,
                    tolerance: 0.1,
                    error: (order - 2.0).abs(),
                }
            },
        },
        Case {
            id: "eme/taper-modes",
            title: r"The same taper on 40 sections with 40, 80 and 160 modes, a grid study in $1/N$ of $\lvert S_{21} \rvert^2$: its estimated error at 160 modes, relative (shown)",
            tier: Tier::Analytic,
            source: r"the convergence gate's estimate (`convergence::Study`): the values 0.996513, 0.996506 and 0.996525 oscillate, so the larger of the last two changes",
            run: || zero(significant(modes_estimate(), 2), 1e-4),
        },
        Case {
            id: "eme/taper-fdfd",
            title: r"A stepped taper: a silicon slab (3.473 in 1.444, 1.55 µm) from 220 to 300 nm through 240, 260 and 280 nm, each 0.1 µm long, TE: its reflection $\lvert S_{11} \rvert^2$ by eigenmode expansion (half the slab, 200 even modes, a PML 4 µm thick of strength 0.1 beyond 1.5 µm) against 2D FDFD (shown)",
            tier: Tier::CrossCode,
            source: r"photonoxide's 2D FDFD on the same staircase (on the grid at 10, 5 and 2.5 nm), its ports' fundamental modes, Richardson's limit of the three: second order, 9.18173e-5",
            run: taper_fdfd,
        },
    ]
}
