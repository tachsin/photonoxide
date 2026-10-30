//! Krylov–Schur's validation cases, shared by the tests and the validation report.

use faer::c64;

use crate::mode::vector;
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

/// The strip's eigenproblem at `h` µm and 1.55 µm: (unknowns, entries, shift).
pub(crate) fn strip_problem(h: f64) -> (usize, Vec<(usize, usize, c64)>, c64) {
    let cs = vector::strip(h);
    let (entries, shift) = vector::eigenproblem(&cs, Wavelength::from_um_unchecked(1.55), None);
    (cs.unknowns(), entries, shift)
}

/// The five-point Laplacian on `nx` × `ny` interior nodes of unit spacing, zero beyond, as
/// entries, and its exact eigenvalues 4 − 2 cos(pπ/(nx+1)) − 2 cos(qπ/(ny+1)), ascending.
pub(crate) fn laplacian(nx: usize, ny: usize) -> (Vec<(usize, usize, c64)>, Vec<f64>) {
    let mut entries = Vec::new();
    for j in 0..ny {
        for i in 0..nx {
            let r = j * nx + i;
            entries.push((r, r, c64::new(4.0, 0.0)));
            if i > 0 {
                entries.push((r, r - 1, c64::new(-1.0, 0.0)));
            }
            if i + 1 < nx {
                entries.push((r, r + 1, c64::new(-1.0, 0.0)));
            }
            if j > 0 {
                entries.push((r, r - nx, c64::new(-1.0, 0.0)));
            }
            if j + 1 < ny {
                entries.push((r, r + nx, c64::new(-1.0, 0.0)));
            }
        }
    }
    let pi = std::f64::consts::PI;
    let mut exact: Vec<f64> = (1..=nx)
        .flat_map(|p| (1..=ny).map(move |q| (p, q)))
        .map(|(p, q)| {
            4.0 - 2.0 * (p as f64 * pi / (nx + 1) as f64).cos()
                - 2.0 * (q as f64 * pi / (ny + 1) as f64).cos()
        })
        .collect();
    exact.sort_by(f64::total_cmp);
    (entries, exact)
}

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// The modes asked for of the uniform box (ε = 2.25, 2 × 1.4 µm on 20 × 14 cells, 1 µm).
const BOX_MODES: usize = 50;

/// The uniform box of [`crate::mode::region::checks::uniform_box`], its [`BOX_MODES`] modes
/// nearest the highest index by Krylov–Schur (a space of 120, restarted) against its exact
/// discrete eigenvalues, each twice (H_x and H_y): the largest relative deviation of β².
pub(crate) fn uniform_box() -> f64 {
    let (nx, ny) = (20, 14);
    let (lx, ly) = (2.0, 1.4);
    let eps = 2.25;
    let cs = vector::CrossSection::uniform((0.0, lx, nx), (0.0, ly, ny), |_, _| {
        vector::Permittivity::isotropic(c64::new(eps, 0.0))
    })
    .expect("a valid box");
    let w = Wavelength::from_um_unchecked(1.0);
    let k = w.wavenumber();
    let (hx, hy) = (lx / nx as f64, ly / ny as f64);
    let term = |p: usize, h: f64, nodes: usize| {
        4.0 / (h * h)
            * (p as f64 * std::f64::consts::PI / (2.0 * (nodes as f64 + 1.0)))
                .sin()
                .powi(2)
    };
    let mut exact: Vec<f64> = (1..=nx + 1)
        .flat_map(|p| (1..=ny + 1).map(move |q| (p, q)))
        .map(|(p, q)| k * k * eps - term(p, hx, nx + 1) - term(q, hy, ny + 1))
        .flat_map(|b| [b, b])
        .collect();
    exact.sort_by(|a, b| b.total_cmp(a));
    let Ok(modes) = vector::modes(&cs, w, BOX_MODES, None) else {
        return f64::NAN;
    };
    let mut found: Vec<f64> = modes
        .iter()
        .map(|m| (m.effective_index() * k).powi(2).re)
        .collect();
    found.sort_by(|a, b| b.total_cmp(a));
    if found.len() != BOX_MODES {
        return f64::NAN;
    }
    found
        .iter()
        .zip(&exact)
        .map(|(f, e)| (f - e).abs() / e)
        .fold(0.0, f64::max)
}

/// The leaky four-layer guide at 10 nm (901 unknowns, TE): the 30 eigenvalues nearest
/// n_eff = 1.3 + 0.03i by Krylov–Schur (a space of 80, restarted) against faer's dense solve of
/// the same matrix: the largest deviation in n_eff.
pub(crate) fn dense_leaky() -> f64 {
    let profile = crate::mode::region::checks::leaky_profile(0.01);
    let w = Wavelength::from_um_unchecked(0.6328);
    let k = w.wavenumber();
    let entries = profile.entries(crate::mode::Polarization::Te, k * k);
    let n = profile.nodes().len();
    let shift = k * k * c64::new(1.3, 0.03).powi(2);
    let mut dense = faer::Mat::<c64>::zeros(n, n);
    for &(i, j, v) in &entries {
        dense[(i, j)] += v;
    }
    let Ok(mut all) = dense.eigenvalues() else {
        return f64::NAN;
    };
    all.sort_by(|a, b| (a - shift).norm().total_cmp(&(b - shift).norm()));
    let Ok(Some((pairs, _))) =
        crate::eigen::krylov_schur(n, &entries, shift, 30, 1e-10, None, &|| false)
    else {
        return f64::NAN;
    };
    pairs
        .iter()
        .zip(&all)
        .map(|(p, d)| (p.value.sqrt() - d.sqrt()).norm() / k)
        .fold(0.0, f64::max)
}

/// The book's strip at 20 nm, its 50 modes nearest the highest index by Krylov–Schur and by the
/// growing restarts it replaced: the largest relative difference in β², each matched to the
/// other's nearest (a pair of complex conjugates at the 50th may be either one).
pub(crate) fn against_growing() -> f64 {
    let (n, entries, shift) = strip_problem(0.02);
    let (Ok(ks), Ok((growing, _))) = (
        crate::eigen::nearest(n, &entries, shift, 50, 1e-9),
        crate::eigen::nearest_growing(n, &entries, shift, 50, 1e-9),
    ) else {
        return f64::NAN;
    };
    if ks.len() != growing.len() {
        return f64::NAN;
    }
    ks.iter()
        .map(|a| {
            growing
                .iter()
                .flat_map(|b| [b.value, b.value.conj()])
                .map(|b| ((a.value - b) / b).norm())
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

/// Chilwell and Hodgkinson's four-layer guide at 1 nm, 20 modes nearest its highest index by
/// Krylov–Schur, TE and TM: the first four of each against Table 3 (largest deviation).
pub(crate) fn chilwell() -> f64 {
    let profile = crate::mode::slab_fd::chilwell_profile(0.001);
    let w = Wavelength::from_um_unchecked(0.6328);
    let table = crate::mode::region::checks::TABLE_3;
    let mut worst = 0.0f64;
    for (pol, want) in [
        (crate::mode::Polarization::Te, &table[..4]),
        (crate::mode::Polarization::Tm, &table[4..]),
    ] {
        let Ok(modes) = profile.modes(pol, w, 20, None) else {
            return f64::NAN;
        };
        let mut n: Vec<f64> = modes.iter().map(|m| m.effective_index.re).collect();
        n.sort_by(|a, b| b.total_cmp(a));
        for (a, b) in n.iter().zip(want) {
            worst = worst.max((a - b).abs());
        }
    }
    worst
}

/// The 2D Laplacian on 40 × 30 nodes, its 20 eigenvalues nearest 0 in a Krylov space of 30
/// (11 growths): over the growths, each converging Ritz value's error against the exact
/// eigenvalues and its residual from the decomposition, between 1e-8 and 1e-2; the slope of
/// ln error against ln residual by least squares, to one decimal (2 for Rayleigh–Ritz's
/// quadratic accuracy on a Hermitian matrix).
pub(crate) fn convergence_slope() -> f64 {
    let (entries, exact) = laplacian(40, 30);
    let Ok(Some((_, record))) = crate::eigen::krylov_schur(
        1200,
        &entries,
        c64::new(0.0, 0.0),
        20,
        1e-12,
        Some(30),
        &|| false,
    ) else {
        return f64::NAN;
    };
    let mut points = Vec::new();
    for (values, residuals) in record.values.iter().zip(&record.residuals) {
        for (v, &r) in values.iter().zip(residuals) {
            let error = exact
                .iter()
                .map(|x| (v.re - x).abs())
                .fold(f64::INFINITY, f64::min);
            if (1e-8..1e-2).contains(&r) && error > 1e-13 {
                points.push((r.ln(), error.ln()));
            }
        }
    }
    let k = points.len() as f64;
    let mx = points.iter().map(|p| p.0).sum::<f64>() / k;
    let my = points.iter().map(|p| p.1).sum::<f64>() / k;
    let slope = points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum::<f64>()
        / points.iter().map(|p| (p.0 - mx).powi(2)).sum::<f64>();
    (10.0 * slope).round() / 10.0
}

/// The strip's 20 modes at 20 nm by Krylov–Schur on 1 and on 4 threads: the number of effective
/// indices and field entries whose bits differ.
pub(crate) fn thread_differences() -> f64 {
    let cs = vector::strip(0.02);
    let w = Wavelength::from_um_unchecked(1.55);
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .ok()
            .and_then(|pool| pool.install(|| vector::modes(&cs, w, 20, None).ok()))
    };
    let (Some(one), Some(four)) = (run(1), run(4)) else {
        return f64::NAN;
    };
    if one.len() != four.len() {
        return f64::NAN;
    }
    let mut differ = 0usize;
    for (a, b) in one.iter().zip(&four) {
        differ += usize::from(a.effective_index() != b.effective_index());
        for i in 0..cs.x().len() {
            for j in 0..cs.y().len() {
                differ += usize::from(a.hx(i, j) != b.hx(i, j));
                differ += usize::from(a.hy(i, j) != b.hy(i, j));
            }
        }
    }
    differ as f64
}

/// A 3 µm wide, 220 nm silicon strip in oxide at 1.55 µm, 20 nm grid, in a 5 × 2.2 µm window:
/// a guide of many modes.
pub(crate) fn wide_strip() -> vector::CrossSection {
    vector::CrossSection::uniform((-2.5, 2.5, 250), (-1.1, 1.1, 110), |x, y| {
        let eps = if x.abs() < 1.5 && y.abs() < 0.11 {
            3.473 * 3.473
        } else {
            1.444 * 1.444
        };
        vector::Permittivity::isotropic(c64::new(eps, 0.0))
    })
    .expect("a valid grid")
}

/// Every guided mode of [`wide_strip`], above the oxide's 1.444, with no count
/// ([`vector::modes_above`]) against Krylov–Schur's as many nearest the highest index: (the
/// largest difference in n_eff, the modes found).
pub(crate) fn every_guided_mode() -> (f64, usize) {
    let cs = wide_strip();
    let w = Wavelength::from_um_unchecked(1.55);
    let Ok(found) = vector::modes_above(&cs, w, 1.444, &crate::mode::region::Search::default())
    else {
        return (f64::NAN, 0);
    };
    let count = found.modes.len();
    if count == 0 {
        return (f64::NAN, 0);
    }
    let Ok(mut ks) = vector::modes(&cs, w, count, None) else {
        return (f64::NAN, count);
    };
    ks.sort_by(|a, b| b.effective_index().re.total_cmp(&a.effective_index().re));
    let worst = found
        .modes
        .iter()
        .zip(&ks)
        .map(|(a, b)| (a.effective_index() - b.effective_index()).norm())
        .fold(0.0, f64::max);
    (worst, count)
}

/// Chilwell and Hodgkinson's guide at 1 nm, every mode above the substrate's 1.50 with no
/// count ([`crate::mode::slab_fd::Profile::modes_above`] at 1.5005), TE then TM: the largest
/// deviation from Table 3, NaN unless 4 of each.
pub(crate) fn chilwell_above() -> f64 {
    let profile = crate::mode::slab_fd::chilwell_profile(0.001);
    let w = Wavelength::from_um_unchecked(0.6328);
    let table = crate::mode::region::checks::TABLE_3;
    let mut worst = 0.0f64;
    for (pol, want) in [
        (crate::mode::Polarization::Te, &table[..4]),
        (crate::mode::Polarization::Tm, &table[4..]),
    ] {
        match profile.modes_above(pol, w, 1.5005, &crate::mode::region::Search::default()) {
            Ok(f) if f.modes.len() == 4 => {
                for (m, t) in f.modes.iter().zip(want) {
                    worst = worst.max((m.effective_index - t).norm());
                }
            }
            _ => return f64::NAN,
        }
    }
    worst
}

/// Krylov–Schur's validation cases, and every guided mode's.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "mode/krylov-schur-box",
            title: r"Many modes by shift-and-invert with Krylov–Schur restarts: a uniform box (ε = 2.25, 2 x 1.4 µm on 20 x 14 cells, 1 µm), its 50 modes nearest the highest index against its exact discrete eigenvalues, 25 of them each twice ($H_x$ and $H_y$), in a Krylov space of 120 (largest relative deviation of $\beta^2$ shown)",
            tier: Tier::Analytic,
            source: r"the scheme's own sines: $\beta^2 = k^2\varepsilon - (4/h_x^2)\sin^2(p\pi/2(N_x+1)) - (4/h_y^2)\sin^2(q\pi/2(N_y+1))$; Krylov–Schur restarts, G. W. Stewart, SIAM J. Matrix Anal. Appl. 23, 601 (published online 14 December 2001; the 2002 volume), doi:10.1137/S0895479800371529, keep each wanted direction, so a double eigenvalue comes out twice",
            run: || outcome(uniform_box(), 0.0, 1e-12),
        },
        Case {
            id: "mode/krylov-schur-dense",
            title: r"Krylov–Schur on a non-normal matrix: Chilwell and Hodgkinson's four layers over 2 µm of PML (10 nm grid, 901 unknowns, TE), the 30 eigenvalues nearest $n_\text{eff} = 1.3 + 0.03i$, leaky waves and the PML's own, against a dense solve of the same matrix (largest deviation in $n_\text{eff}$ shown)",
            tier: Tier::CrossCode,
            source: "faer's dense eigensolver (QR algorithm) on the same matrix, the 30 nearest the same shift; the PML's own modes off the real axis are ill-conditioned (a PML makes the matrix far from normal), each found to its residual of 1e-10 times its condition, which leaves up to 1.4e-8 between the two; the leaky waves $m = 4 \\ldots 7$ among them agree to 1.1e-11",
            run: || outcome(dense_leaky(), 0.0, 3e-8),
        },
        Case {
            id: "mode/krylov-schur-growing",
            title: r"Krylov–Schur against the restarts it replaced: the book's strip (500 x 220 nm silicon in oxide, 1.55 µm, 20 nm grid), its 50 modes nearest the highest index, guided, radiation and evanescent (largest relative difference in $\beta^2$ shown)",
            tier: Tier::CrossCode,
            source: "photonoxide 0.5.1's shift-and-invert Arnoldi, restarted from the wanted Ritz vectors summed, in a Krylov space growing by 120 vectors a restart (Y. Saad, Numerical Methods for Large Eigenvalue Problems, 2nd ed., SIAM (2011), doi:10.1137/1.9781611970739, Algorithm 6.3); both accept an eigenpair at a residual of 1e-9",
            run: || outcome(against_growing(), 0.0, 1e-10),
        },
        Case {
            id: "mode/krylov-schur-chilwell",
            title: r"Chilwell and Hodgkinson's four-layer guide at 632.8 nm (1D finite differences, 1 nm grid): 20 modes nearest its highest index by Krylov–Schur, TE and TM, the four bound modes of each among them (largest deviation from the table shown)",
            tier: Tier::Published,
            source: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Table 3: 8 bound modes, effective indices to 6 decimals",
            run: || outcome(chilwell(), 0.0, 2e-6),
        },
        Case {
            id: "mode/krylov-schur-convergence",
            title: r"Convergence over the restarts: the 2D Laplacian on 40 x 30 nodes, its 20 eigenvalues nearest 0 in a Krylov space of 30, 11 growths from a residual of 0.95 to 1.4e-14; each Ritz value's error against the exact eigenvalues as its residual falls from 1e-2 to 1e-8, the slope of $\ln$ error against $\ln$ residual (shown to one decimal; 2 when the error goes as the residual squared)",
            tier: Tier::Analytic,
            source: r"exact eigenvalues $4 - 2\cos(p\pi/41) - 2\cos(q\pi/31)$; a Ritz value of a Hermitian matrix errs by about its residual squared over the gap to the rest of the spectrum (B. N. Parlett, The Symmetric Eigenvalue Problem, SIAM (1998), doi:10.1137/1.9781611971163), which a Krylov–Schur restart keeps by keeping a Krylov decomposition (Stewart 2001, Theorem 2.2)",
            run: || outcome(convergence_slope(), 2.0, 0.3),
        },
        Case {
            id: "mode/krylov-schur-threads",
            title: r"The strip's 20 modes by Krylov–Schur (20 nm grid) on 1 and on 4 threads: effective indices and field entries whose bits differ (count shown)",
            tier: Tier::Analytic,
            source: "the Arnoldi steps, the Schur form and its reordering, and the restart's products each run in a fixed order on one thread",
            run: || outcome(thread_differences(), 0.0, 0.0),
        },
        Case {
            id: "mode/every-guided-mode",
            title: r"Every guided mode above an index, no count given: a 3 µm wide, 220 nm silicon strip in oxide at 1.55 µm (20 nm grid, 5 x 2.2 µm window), every mode above the oxide's 1.444 by contour integrals, against Krylov–Schur asked for as many (largest difference in $n_\text{eff}$ shown)",
            tier: Tier::CrossCode,
            source: "photonoxide's shift-and-invert Arnoldi with Krylov–Schur restarts on the same matrix (G. W. Stewart, SIAM J. Matrix Anal. Appl. 23, 601 (published online 14 December 2001; the 2002 volume), doi:10.1137/S0895479800371529), its residual below 1e-9",
            run: || {
                let (worst, count) = every_guided_mode();
                outcome(if count > 0 { worst } else { f64::NAN }, 0.0, 1e-9)
            },
        },
        Case {
            id: "mode/every-guided-mode-chilwell",
            title: r"Every mode of Chilwell and Hodgkinson's four-layer guide above its substrate's index (1D finite differences, 1 nm grid, the threshold 1.5005), no count given: 4 TE and 4 TM found (largest deviation from the table shown)",
            tier: Tier::Published,
            source: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Table 3: 8 bound modes, effective indices to 6 decimals",
            run: || outcome(chilwell_above(), 0.0, 2e-6),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn many_modes_of_a_box_are_its_exact_eigenvalues_each_twice() {
        let e = uniform_box();
        assert!(e < 1e-12, "{e}");
    }

    #[test]
    fn the_leaky_guide_has_a_dense_solves_eigenvalues_nearest_the_shift() {
        let e = dense_leaky();
        assert!(e < 3e-8, "{e}");
    }

    #[test]
    fn chilwell_and_hodgkinsons_bound_modes_are_among_twenty_and_above_the_substrate() {
        assert!(chilwell() < 2e-6);
        assert!(chilwell_above() < 2e-6);
    }

    #[test]
    fn the_error_falls_as_the_residual_squared() {
        let s = convergence_slope();
        assert!((s - 2.0).abs() <= 0.3, "{s}");
    }

    #[test]
    fn the_space_never_holds_more_than_its_size() {
        // 20 eigenvalues of the Laplacian in a space of 30: 31 vectors at most, however many
        // restarts (the growing restarts held 30 more each)
        let (entries, exact) = laplacian(40, 30);
        let (pairs, record) = crate::eigen::krylov_schur(
            1200,
            &entries,
            c64::new(0.0, 0.0),
            20,
            1e-12,
            Some(30),
            &|| false,
        )
        .unwrap()
        .unwrap();
        assert!(record.restarts >= 5, "{record:?}");
        assert_eq!(record.held, 31);
        for (p, e) in pairs.iter().zip(&exact) {
            assert!((p.value.re - e).abs() < 1e-12);
        }
        // by default, a space of 2 count + 20 (40 at least) keeping half the unwanted
        assert_eq!(crate::eigen::sizes(1200, 20), (60, 40));
        assert_eq!(crate::eigen::sizes(1200, 2), (40, 21));
        assert_eq!(crate::eigen::sizes(30, 20), (30, 25));
    }

    /// Krylov–Schur against the growing restarts on the strip, many modes: time, vectors held,
    /// and the largest difference in β² (relative). `KS_H` the grid (µm), `KS_COUNTS` the
    /// counts, `KS_WHICH` "ks" or "growing" for one alone. Run with --release --ignored
    /// --nocapture.
    #[test]
    #[ignore = "a measurement, minutes on a fine grid"]
    fn measure_many_modes() {
        let h: f64 = std::env::var("KS_H")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.02);
        let counts: Vec<usize> = std::env::var("KS_COUNTS").ok().map_or_else(
            || vec![2, 8, 20, 50],
            |s| s.split(',').map(|c| c.parse().unwrap()).collect(),
        );
        let which = std::env::var("KS_WHICH").unwrap_or_else(|_| "both".into());
        let (n, entries, shift) = strip_problem(h);
        println!("strip at {h} um: {n} unknowns");
        for count in counts {
            let mut ks = None;
            if which != "growing" {
                let t = Instant::now();
                let (pairs, record) =
                    crate::eigen::krylov_schur(n, &entries, shift, count, 1e-9, None, &|| false)
                        .unwrap()
                        .unwrap();
                println!(
                    "Krylov-Schur {count:3}: {:8.2} s, {:4} vectors ({:7.1} MB), {:5} solves, {:3} restarts",
                    t.elapsed().as_secs_f64(),
                    record.held,
                    record.held as f64 * n as f64 * 16.0 / 1e6,
                    record.solves,
                    record.restarts
                );
                ks = Some(pairs);
            }
            if which != "ks" {
                let t = Instant::now();
                let (pairs, held) =
                    crate::eigen::nearest_growing(n, &entries, shift, count, 1e-9).unwrap();
                println!(
                    "growing      {count:3}: {:8.2} s, {:4} vectors ({:7.1} MB)",
                    t.elapsed().as_secs_f64(),
                    held,
                    held as f64 * n as f64 * 16.0 / 1e6
                );
                if let Some(ks) = &ks {
                    let worst = ks
                        .iter()
                        .map(|a| {
                            pairs
                                .iter()
                                .flat_map(|b| [b.value, b.value.conj()])
                                .map(|b| ((a.value - b) / b).norm())
                                .fold(f64::INFINITY, f64::min)
                        })
                        .fold(0.0, f64::max);
                    println!("             difference {worst:.1e}");
                }
            }
        }
    }
}
