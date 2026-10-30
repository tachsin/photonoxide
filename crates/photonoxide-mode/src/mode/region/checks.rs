//! The contour solver's validation cases, shared by the tests and the validation report.

use num_complex::Complex64 as c64;

use super::{Region, Search};
use crate::mode::Polarization;
use crate::mode::slab_fd::{Profile, chilwell_profile};
use crate::mode::vector::{self, CrossSection, Permittivity};
use crate::units::Wavelength;
use crate::validation::{Case, Outcome, Tier};

fn outcome(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance,
        error: (measured - expected).abs(),
    }
}

/// Chilwell and Hodgkinson's Table 3, the bound modes' effective indices: TE₀ … TE₃, then TM.
pub(crate) const TABLE_3: [f64; 8] = [
    1.622729, 1.605276, 1.557136, 1.503587, 1.620031, 1.594788, 1.554981, 1.501818,
];

/// Their Table 2, the TE leaky waves m = 4 … 7.
const TABLE_2: [(f64, f64); 4] = [
    (1.46186, 0.00716),
    (1.38250, 0.01817),
    (1.28136, 0.03588),
    (1.14231, 0.05288),
];

/// The bound modes' region: above the substrate's 1.50 (its own modes in the window lie below
/// it) to above the films' 1.66.
pub(crate) fn bound_region() -> Region {
    Region::between(1.5005, 1.67).expect("a valid region")
}

/// The leaky waves' regions: a disc about m = 4, and an ellipse about m = 5 … 7 that keeps
/// below the PML's branch of modes, which rises from the substrate's index into Im n_eff.
pub(crate) fn leaky_regions() -> [Region; 2] {
    [
        Region::disc(c64::new(1.46186, 0.00715), 0.015).expect("a valid region"),
        Region::ellipse(c64::new(1.27, 0.035), 0.17, 0.03).expect("a valid region"),
    ]
}

fn chilwell_wavelength() -> Wavelength {
    Wavelength::from_um_unchecked(0.6328)
}

/// Every mode in the bound region of the four-layer guide at 1 nm, TE then TM, no count given:
/// the largest deviation from Table 3, NaN unless there are 4 of each.
pub(crate) fn chilwell_bound() -> f64 {
    let profile = chilwell_profile(0.001);
    let mut worst = 0.0f64;
    for (pol, table) in [
        (Polarization::Te, &TABLE_3[..4]),
        (Polarization::Tm, &TABLE_3[4..]),
    ] {
        let Ok(found) = profile.modes_in(
            pol,
            chilwell_wavelength(),
            &bound_region(),
            &Search::default(),
        ) else {
            return f64::NAN;
        };
        if found.modes.len() != 4 {
            return f64::NAN;
        }
        for (m, t) in found.modes.iter().zip(table) {
            worst = worst.max((m.effective_index - t).norm());
        }
    }
    worst
}

/// The leaky profile: the four-layer guide with 2 µm of PML (α = 5) in the substrate.
pub(crate) fn leaky_profile(h: f64) -> Profile {
    chilwell_profile(h)
        .with_pml(2.0, 0.0, 5.0)
        .expect("a valid PML")
}

/// The TE leaky waves in [`leaky_regions`] at 1 nm: the largest deviation of the real or
/// imaginary part from Table 2, NaN unless the regions hold 1 and 3 of them.
pub(crate) fn chilwell_leaky() -> f64 {
    let profile = leaky_profile(0.001);
    let mut found = Vec::new();
    for (region, count) in leaky_regions().iter().zip([1, 3]) {
        match profile.modes_in(
            Polarization::Te,
            chilwell_wavelength(),
            region,
            &Search::default(),
        ) {
            Ok(f) if f.modes.len() == count => {
                found.extend(f.modes.iter().map(|m| m.effective_index));
            }
            _ => return f64::NAN,
        }
    }
    found
        .iter()
        .zip(TABLE_2)
        .map(|(n, (re, im))| (n.re - re).abs().max((n.im - im).abs()))
        .fold(0.0, f64::max)
}

/// Every eigenvalue in the ellipse of m = 5 … 7 of the leaky profile at 10 nm (901 unknowns),
/// against faer's dense eigensolver on the same matrix: the largest deviation in n_eff, NaN
/// unless the counts agree.
pub(crate) fn dense_leaky() -> f64 {
    let profile = leaky_profile(0.01);
    let w = chilwell_wavelength();
    let k = w.wavenumber();
    let entries = profile.entries(Polarization::Te, k * k);
    let n = profile.nodes().len();
    let mut dense = faer::Mat::<c64>::zeros(n, n);
    for (i, j, v) in entries {
        dense[(i, j)] += v;
    }
    let region = leaky_regions()[1];
    let Ok(all) = dense.eigenvalues() else {
        return f64::NAN;
    };
    let mut want: Vec<c64> = all
        .iter()
        .map(|b| b.sqrt() / k)
        .filter(|z| region.contains(*z))
        .collect();
    let Ok(found) = profile.modes_in(Polarization::Te, w, &region, &Search::default()) else {
        return f64::NAN;
    };
    if found.modes.len() != want.len() || want.is_empty() {
        return f64::NAN;
    }
    want.sort_by(|a, b| b.re.total_cmp(&a.re));
    found
        .modes
        .iter()
        .zip(&want)
        .map(|(m, w)| (m.effective_index - w).norm())
        .fold(0.0, f64::max)
}

/// A uniform box of ε = 2.25, 2 × 1.4 µm on 20 × 14 cells, at 1 µm: its exact discrete modes
/// (each twice, H_x and H_y), β² = k²ε − (4/h_x²) sin²(pπ/(2(N_x+1))) − (4/h_y²)
/// sin²(qπ/(2(N_y+1))) over its N_x × N_y nodes, against those found in a region holding the
/// six highest: (largest relative deviation of β², modes found, modes expected).
pub(crate) fn uniform_box() -> (f64, usize, usize) {
    let (nx, ny) = (20, 14);
    let (lx, ly) = (2.0, 1.4);
    let eps = 2.25;
    let cs = CrossSection::uniform((0.0, lx, nx), (0.0, ly, ny), |_, _| {
        Permittivity::isotropic(c64::new(eps, 0.0))
    })
    .expect("a valid box");
    let w = Wavelength::from_um_unchecked(1.0);
    let k = w.wavenumber();
    let (hx, hy) = (lx / nx as f64, ly / ny as f64);
    let (nodes_x, nodes_y) = (nx + 1, ny + 1);
    let term = |p: usize, h: f64, nodes: usize| {
        4.0 / (h * h)
            * (p as f64 * std::f64::consts::PI / (2.0 * (nodes as f64 + 1.0)))
                .sin()
                .powi(2)
    };
    let mut exact: Vec<f64> = (1..=nodes_x)
        .flat_map(|p| (1..=nodes_y).map(move |q| (p, q)))
        .map(|(p, q)| k * k * eps - term(p, hx, nodes_x) - term(q, hy, nodes_y))
        .collect();
    exact.sort_by(|a, b| b.total_cmp(a));
    // the region from just below the sixth to above the first, its edge halfway to the seventh
    let index = |beta2: f64| beta2.sqrt() / k;
    let low = 0.5 * (index(exact[5]) + index(exact[6]));
    let high = index(exact[0]) + 0.5 * (index(exact[0]) - index(exact[5]));
    let Ok(found) = vector::modes_in(
        &cs,
        w,
        &Region::between(low, high).expect("a valid region"),
        &Search::default(),
    ) else {
        return (f64::NAN, 0, 12);
    };
    let want: Vec<f64> = exact[..6].iter().flat_map(|&b| [b, b]).collect();
    let worst = found
        .modes
        .iter()
        .zip(&want)
        .map(|(m, &b)| {
            let n = m.effective_index();
            ((n * n * k * k - b).norm()) / b
        })
        .fold(0.0, f64::max);
    (worst, found.modes.len(), want.len())
}

/// The book's strip at 20 nm, its two guided modes by contour integrals in n_eff from 1.5 to
/// 3.4 and by shift-and-invert Arnoldi: the largest difference in n_eff, NaN unless two.
pub(crate) fn strip_against_shift_invert() -> f64 {
    let cs = vector::strip(0.02);
    let w = Wavelength::from_um_unchecked(1.55);
    let Ok(arnoldi) = vector::modes(&cs, w, 2, None) else {
        return f64::NAN;
    };
    let Ok(found) = vector::modes_in(
        &cs,
        w,
        &Region::between(1.5, 3.4).expect("a valid region"),
        &Search::default(),
    ) else {
        return f64::NAN;
    };
    if found.modes.len() != 2 {
        return f64::NAN;
    }
    found
        .modes
        .iter()
        .zip(&arnoldi)
        .map(|(a, b)| (a.effective_index() - b.effective_index()).norm())
        .fold(0.0, f64::max)
}

/// Chilwell and Hodgkinson's guide lying flat in the full-vector solver at 2.5 nm with its PML,
/// the TE leaky waves m = 4 … 7 in [`leaky_regions`] against shift-and-invert Arnoldi's nearest
/// each: the largest difference in n_eff, NaN unless 1 and 3 of them.
pub(crate) fn leaky_against_shift_invert() -> f64 {
    let h = 0.0025;
    let cs = vector::chilwell_flat(h);
    let w = chilwell_wavelength();
    let mut found = Vec::new();
    for (region, count) in leaky_regions().iter().zip([1, 3]) {
        match vector::modes_in(&cs, w, region, &Search::default()) {
            Ok(f) if f.modes.len() == count => {
                found.extend(f.modes.iter().map(vector::VectorMode::effective_index));
            }
            _ => return f64::NAN,
        }
    }
    let arnoldi = vector::chilwell_leaky(h, &found);
    found
        .iter()
        .zip(&arnoldi)
        .map(|(a, b)| (a - b).norm())
        .fold(0.0, f64::max)
}

/// The largest error of the four-layer guide's TE bound modes at 1 nm after one filter pass of
/// a subspace of 8, at `points` quadrature points, against the converged modes.
pub(crate) fn one_pass_error(points: usize) -> f64 {
    let profile = chilwell_profile(0.001);
    let w = chilwell_wavelength();
    let Ok(exact) = profile.modes_in(Polarization::Te, w, &bound_region(), &Search::default())
    else {
        return f64::NAN;
    };
    let once = Search {
        points,
        subspace: Some(8),
        tolerance: 1.0,
        iterations: 1,
    };
    match profile.modes_in(Polarization::Te, w, &bound_region(), &once) {
        Ok(f) if f.modes.len() == exact.modes.len() => f
            .modes
            .iter()
            .zip(&exact.modes)
            .map(|(a, b)| (a.effective_index - b.effective_index).norm())
            .fold(0.0, f64::max),
        _ => f64::NAN,
    }
}

/// The trapezoidal rule's convergence: with e(N) = C qᴺ, the rate per point from 12 to 24
/// points over that from 6 to 12, ln q₂ / ln q₁: 1 when the error falls exponentially, ½ for a
/// power of N. Rounded to two decimals, so the report reads the same on every machine.
pub(crate) fn exponential_rate() -> f64 {
    let e = [6, 12, 24].map(one_pass_error);
    let q1 = (e[1] / e[0]).ln() / 6.0;
    let q2 = (e[2] / e[1]).ln() / 12.0;
    (100.0 * q2 / q1).round() / 100.0
}

/// The strip's modes at 20 nm on 1 and on 4 threads: the number of values and field entries
/// whose bits differ.
pub(crate) fn thread_differences() -> f64 {
    let cs = vector::strip(0.02);
    let w = Wavelength::from_um_unchecked(1.55);
    let region = Region::between(1.5, 3.4).expect("a valid region");
    let run = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .ok()
            .and_then(|pool| {
                pool.install(|| vector::modes_in(&cs, w, &region, &Search::default()).ok())
            })
    };
    let (Some(one), Some(four)) = (run(1), run(4)) else {
        return f64::NAN;
    };
    if one.modes.len() != four.modes.len() {
        return f64::NAN;
    }
    let mut differ = 0usize;
    for (a, b) in one.modes.iter().zip(&four.modes) {
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

/// The ratio of Sakurai and Sugiura's largest errors at 64 and 128 points in their Example 5,
/// the median over 33 draws of u and v: the error itself depends on them, its fall doesn't.
pub(crate) fn sakurai_sugiura_ratio() -> f64 {
    use crate::eigen::contour::sakurai_sugiura_error;
    let mut ratios: Vec<f64> = (0..33)
        .map(|d| sakurai_sugiura_error(64, d) / sakurai_sugiura_error(128, d))
        .collect();
    ratios.sort_by(f64::total_cmp);
    // two significant digits, as the paper prints, and the same on every machine
    (ratios[16] / 1e5).round() * 1e5
}

/// The contour solver's validation cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "mode/contour-uniform-box",
            title: r"Every mode in a region of a uniform box's effective indices (ε = 2.25, 2 x 1.4 µm on 20 x 14 cells, 1 µm), no count given: its six highest exact discrete eigenvalues, each twice ($H_x$ and $H_y$), by contour integrals (largest relative deviation of $\beta^2$ shown)",
            tier: Tier::Analytic,
            source: r"the scheme's own sines: $\beta^2 = k^2\varepsilon - (4/h_x^2)\sin^2(p\pi/2(N_x+1)) - (4/h_y^2)\sin^2(q\pi/2(N_y+1))$; FEAST, E. Polizzi, Phys. Rev. B 79, 115112 (2009), doi:10.1103/PhysRevB.79.115112, captures multiplicities whole; 12 found of 12",
            run: || {
                let (worst, found, want) = uniform_box();
                let worst = if found == want { worst } else { f64::NAN };
                outcome(worst, 0.0, 1e-12)
            },
        },
        Case {
            id: "mode/contour-dense",
            title: r"Every eigenvalue in a region of the complex $n_\text{eff}$ plane of a leaky planar guide (Chilwell and Hodgkinson's four layers over 2 µm of PML, 10 nm grid, 901 unknowns, TE): the ellipse about 1.27 + 0.035i of semi-axes 0.17 and 0.03, against a dense solve of the same matrix (largest deviation shown)",
            tier: Tier::CrossCode,
            source: "faer's dense eigensolver (QR algorithm) on the same matrix: the same 3 eigenvalues inside, none missed and none spurious; the leaky waves are sensitive (a PML makes the matrix far from normal), so the two solvers' backward errors, each near rounding, leave 6e-10 between them",
            run: || outcome(dense_leaky(), 0.0, 2e-9),
        },
        Case {
            id: "mode/contour-strip-shift-invert",
            title: r"The book's strip (500 x 220 nm silicon in oxide, 1.55 µm, 20 nm grid): every mode with $n_\text{eff}$ from 1.5 to 3.4 by contour integrals, no count given, against shift-and-invert Arnoldi's two (largest difference shown)",
            tier: Tier::CrossCode,
            source: "photonoxide's shift-and-invert Arnoldi (Y. Saad, Numerical Methods for Large Eigenvalue Problems, 2nd ed., SIAM (2011), doi:10.1137/1.9781611970739) on the same matrix, its residual below 1e-9",
            run: || outcome(strip_against_shift_invert(), 0.0, 1e-10),
        },
        Case {
            id: "mode/contour-leaky-shift-invert",
            title: r"Chilwell and Hodgkinson's guide lying flat in the full-vector solver with 2 µm of PML (2.5 nm grid, 632.8 nm): its TE leaky waves $m = 4 \ldots 7$ found in two regions of the complex $n_\text{eff}$ plane, against shift-and-invert Arnoldi near each (largest difference shown)",
            tier: Tier::CrossCode,
            source: "photonoxide's shift-and-invert Arnoldi (Saad 2011, doi:10.1137/1.9781611970739) on the same matrix; non-Hermitian FEAST, J. Kestyn, E. Polizzi, P. T. P. Tang, SIAM J. Sci. Comput. 38, S772 (2016), doi:10.1137/15M1026572",
            run: || outcome(leaky_against_shift_invert(), 0.0, 1e-9),
        },
        Case {
            id: "mode/contour-chilwell-bound",
            title: r"Every bound mode of Chilwell and Hodgkinson's four-layer guide at 632.8 nm, by contour integrals over $n_\text{eff}$ from 1.5005 to 1.67 with no count given, 1D finite differences on a 1 nm grid: 4 TE and 4 TM found (largest deviation from the table shown)",
            tier: Tier::Published,
            source: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Table 3: 8 bound modes, effective indices to 6 decimals",
            run: || outcome(chilwell_bound(), 0.0, 2e-6),
        },
        Case {
            id: "mode/contour-chilwell-leaky",
            title: r"The same guide's TE leaky waves $m = 4 \ldots 7$ with 2 µm of PML in the substrate, found as every mode in a disc about $m = 4$ (radius 0.015) and an ellipse about $m = 5 \ldots 7$ below the PML's own branch, 1 nm grid: 1 and 3 found (largest deviation of a real or imaginary part shown)",
            tier: Tier::Published,
            source: "J. Chilwell, I. Hodgkinson, J. Opt. Soc. Am. A 1, 742 (1984), doi:10.1364/JOSAA.1.000742, Table 2: complex effective indices to 5 decimals; the grid adds about 1e-5 at $m = 7$",
            run: || outcome(chilwell_leaky(), 0.0, 2e-5),
        },
        Case {
            id: "mode/contour-exponential",
            title: r"Convergence in the quadrature points $N$: the four-layer guide's TE bound modes (1 nm grid) after one filter pass of a subspace of 8, the error $e(N) = C q^N$ at $N$ = 6, 12, 24; the rate from 12 to 24 over that from 6 to 12, $\ln q_2 / \ln q_1$ (shown to two decimals; 1 when exponential, 1/2 for a power of $N$)",
            tier: Tier::Analytic,
            source: r"the trapezoidal rule on a closed curve converges exponentially for an analytic integrand (T. Sakurai, H. Sugiura, J. Comput. Appl. Math. 159, 119 (2003), doi:10.1016/S0377-0427(03)00565-X, Section 3; Kestyn et al. 2016, Section 2.3); measured 1.3e-2, 1.9e-4, 1.8e-8, and 3.1e-3, 9.2e-6, 3.3e-11 at 8, 16, 32",
            run: || outcome(exponential_rate(), 1.0, 0.2),
        },
        Case {
            id: "mode/contour-threads",
            title: r"The strip's modes by contour integrals (20 nm grid) on 1 and on 4 threads: effective indices and field entries whose bits differ (count shown)",
            tier: Tier::Analytic,
            source: "each quadrature point's factorization and solves are the same computation on any thread, and their contributions are summed in the points' order",
            run: || outcome(thread_differences(), 0.0, 0.0),
        },
        Case {
            id: "mode/contour-sakurai-sugiura",
            title: r"Sakurai and Sugiura's Example 5 by their Hankel method ($m$ = 4, the circle of centre 0.015 and radius 0.02 holding 0, 0.01, 0.02, 0.03 of a 100 x 100 pencil with singular $B$): the largest error at 64 points over that at 128 (median of 33 random $u$, $v$ shown to two digits, as the paper prints; 1.594e6)",
            tier: Tier::Published,
            source: r"T. Sakurai, H. Sugiura, J. Comput. Appl. Math. 159, 119 (2003), doi:10.1016/S0377-0427(03)00565-X, Example 5: 2.1e-6 and 1.3e-12 for their one draw, a ratio of 1.6e6 (from 2 printed digits each, 1.52e6 to 1.72e6), and their bound $\delta^{2m-N}$ gives $1.25^{64} = 1.59 \times 10^6$; our median error at 64 points is 8.4e-7",
            run: || outcome(sakurai_sugiura_ratio(), 1.615e6, 0.1e6),
        },
    ]
}
