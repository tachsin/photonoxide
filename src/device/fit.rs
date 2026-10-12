//! The analytic fidelity's fits: a silicon strip's effective and group index over its width and
//! thickness, and the coupling of two strips over their gap, width and thickness, each
//! interpolated through the full-vector solver's values on a grid of designs and anchored at
//! the studio library's 500 × 220 nm strip.
//!
//! The solver's values are photonoxide's own, on 220 nm-class SOI (Li's silicon, Malitson's
//! silica) at 1.55 µm, each on a quarter of its cross-section with cells fitted to the core
//! about 10 nm apart and 1 µm of oxide beyond it: the strip's mode by Hadley's equations
//! (doi:10.1109/JLT.2002.800361, doi:10.1109/JLT.2002.800371), n_g from five wavelengths 25 nm
//! apart; the pair's supermodes by the standard full-vector scheme (Fallahkhair et al. 2008,
//! doi:10.1109/JLT.2008.923643) with mirror walls, C = πΔn/λ (Chrostowski & Hochberg 2015,
//! Eq. 4.3, doi:10.1017/CBO9781316084168). The device module's ignored test
//! `device::tests::the_fits_tables` recomputes them.
//!
//! Each table is interpolated by Lagrange polynomials through all its nodes, one variable at a
//! time (the coupling in ln C, which falls nearly exponentially with the gap), so the fit is the
//! solver's value at every node. It is then shifted to agree, at 500 × 220 nm (and a 200 nm
//! gap), with the numbers the studio's library has always used: n_eff 2.44506 and n_g 4.172901
//! (the `group_index` example: Chrostowski & Hochberg's silicon and 1.444 oxide, a 6.25 × 5 nm
//! grid) and C = 0.04228/µm (the `directional_coupler` example, a 5 nm grid). The shifts are
//! −5e-5 in n_eff, −0.0164 in n_g (the books' silicon is less dispersive than Li's) and −0.8 %
//! in C; the validation report measures what they and the interpolation leave against the
//! solver.

use crate::{Error, Result};

/// The wavelength the fits are at, µm.
pub(super) const WAVELENGTH: f64 = 1.55;

/// The band about [`WAVELENGTH`] the fits' first-order dispersion is used over, µm.
pub(super) const BAND: (f64, f64) = (1.5, 1.6);

/// The grid the figures of the fits name.
pub(super) const GUIDE_GRID: &str =
    "a fit to the full-vector solver (Hadley) on fitted grids about 10 nm";

/// The grid the coupling's figures name.
pub(super) const COUPLING_GRID: &str =
    "a fit to the full-vector solver on fitted grids about 10 nm";

/// The widths of the tables, µm.
pub(super) const WIDTHS: [f64; 5] = [0.40, 0.45, 0.50, 0.55, 0.60];
/// The thicknesses of the strip's table, µm.
pub(super) const THICKNESSES: [f64; 5] = [0.20, 0.21, 0.22, 0.23, 0.24];
/// The thicknesses of the coupling's table, µm.
pub(super) const PAIR_THICKNESSES: [f64; 3] = [0.20, 0.22, 0.24];
/// The gaps of the coupling's table, µm.
pub(super) const GAPS: [f64; 5] = [0.1, 0.2, 0.3, 0.4, 0.5];

/// n_eff and n_g of the TE-like mode at 1.55 µm, by width then thickness.
pub(super) const GUIDE: [[(f64, f64); 5]; 5] = [
    [
        (2.145_267_622, 4.325_740_438),
        (2.187_363_135, 4.359_269_681),
        (2.226_800_256, 4.384_111_959),
        (2.263_689_727, 4.402_165_401),
        (2.298_200_410, 4.414_818_354),
    ],
    [
        (2.271_813_823, 4.248_655_145),
        (2.313_795_709, 4.269_224_615),
        (2.352_903_026, 4.283_969_065),
        (2.389_320_495, 4.294_178_952),
        (2.423_257_714, 4.300_790_479),
    ],
    [
        (2.365_125_843, 4.162_457_203),
        (2.406_588_649, 4.178_151_538),
        (2.445_109_789, 4.189_345_360),
        (2.480_907_538, 4.197_007_732),
        (2.514_207_588, 4.201_851_325),
    ],
    [
        (2.435_147_542, 4.087_662_642),
        (2.476_057_732, 4.101_507_601),
        (2.514_013_173, 4.111_503_572),
        (2.549_248_362, 4.118_447_543),
        (2.581_995_075, 4.122_937_517),
    ],
    [
        (2.488_807_394, 4.026_391_054),
        (2.529_224_323, 4.039_593_481),
        (2.566_693_450, 4.049_290_912),
        (2.601_457_294, 4.056_182_706),
        (2.633_749_724, 4.060_803_479),
    ],
];

/// The coupling C = πΔn/λ of two strips at 1.55 µm, 1/µm, by width, thickness, then gap.
pub(super) const COUPLING: [[[f64; 5]; 3]; 5] = [
    [
        [
            0.278_654_547,
            0.128_245_477,
            0.061_898_889,
            0.030_258_041,
            0.014_908_053,
        ],
        [
            0.251_648_760,
            0.110_682_757,
            0.051_231_571,
            0.024_020_994,
            0.011_349_289,
        ],
        [
            0.230_339_053,
            0.097_508_466,
            0.043_561_563,
            0.019_714_145,
            0.008_988_003,
        ],
    ],
    [
        [
            0.180_833_051,
            0.077_073_205,
            0.034_827_441,
            0.015_966_190,
            0.007_377_980,
        ],
        [
            0.162_068_159,
            0.066_165_951,
            0.028_726_793,
            0.012_652_207,
            0.005_615_293,
        ],
        [
            0.147_720_265,
            0.058_182_667,
            0.024_424_400,
            0.010_399_029,
            0.004_460_147,
        ],
    ],
    [
        [
            0.123_109_153,
            0.049_690_179,
            0.021_429_677,
            0.009_384_229,
            0.004_141_967,
        ],
        [
            0.109_992_672,
            0.042_624_162,
            0.017_689_763,
            0.007_452_291,
            0.003_163_227,
        ],
        [
            0.100_124_843,
            0.037_509_093,
            0.015_072_068,
            0.006_145_327,
            0.002_523_691,
        ],
    ],
    [
        [
            0.087_495_323,
            0.033_951_078,
            0.014_151_627,
            0.005_991_896,
            0.002_556_760,
        ],
        [
            0.078_078_458,
            0.029_142_428,
            0.011_703_579,
            0.004_772_109,
            0.001_960_223,
        ],
        [
            0.071_060_619,
            0.025_680_546,
            0.009_995_392,
            0.003_948_020,
            0.001_570_374,
        ],
    ],
    [
        [
            0.064_466_104,
            0.024_292_916,
            0.009_869_982,
            0.004_073_981,
            0.001_694_448,
        ],
        [
            0.057_505_866,
            0.020_874_465,
            0.008_178_643,
            0.003_253_576,
            0.001_303_699,
        ],
        [
            0.052_349_838,
            0.018_420_762,
            0.006_999_956,
            0.002_699_305,
            0.001_048_057,
        ],
    ],
];

/// The studio library's strip: n_eff and n_g at 500 × 220 nm.
pub(super) const ANCHOR: (f64, f64) = (2.44506, 4.172_901);

/// The studio library's coupling at a 200 nm gap between 500 × 220 nm strips, 1/µm.
pub(super) const COUPLING_ANCHOR: f64 = 0.042_28;

/// The anchor's design: width, thickness and gap, µm.
const AT: (f64, f64, f64) = (0.5, 0.22, 0.2);

/// The Lagrange weights of `nodes` at x, and their derivatives: Σ wₖ f(xₖ) interpolates f.
fn lagrange<const N: usize>(nodes: &[f64; N], x: f64) -> ([f64; N], [f64; N]) {
    let mut w = [0.0; N];
    let mut d = [0.0; N];
    for k in 0..N {
        let mut value = 1.0;
        let mut slope = 0.0;
        for m in (0..N).filter(|&m| m != k) {
            let f = (x - nodes[m]) / (nodes[k] - nodes[m]);
            slope = slope * f + value / (nodes[k] - nodes[m]);
            value *= f;
        }
        w[k] = value;
        d[k] = slope;
    }
    (w, d)
}

/// Refuses `x` outside `nodes`' span: the fit's range.
fn within(what: &str, x: f64, nodes: &[f64]) -> Result<()> {
    let (lo, hi) = (nodes[0], nodes[nodes.len() - 1]);
    if (lo..=hi).contains(&x) {
        Ok(())
    } else {
        Err(Error::invalid(
            "device",
            format!(
                "the analytic fit holds for {what} from {lo} to {hi} um, not {x} um: solve the \
                 modes (3D)"
            ),
        ))
    }
}

/// The interpolated table at (width, thickness).
fn guide_table(width: f64, thickness: f64) -> (f64, f64) {
    let (a, _) = lagrange(&WIDTHS, width);
    let (b, _) = lagrange(&THICKNESSES, thickness);
    let mut n = 0.0;
    let mut ng = 0.0;
    for (i, row) in GUIDE.iter().enumerate() {
        for (j, &(e, g)) in row.iter().enumerate() {
            n += a[i] * b[j] * e;
            ng += a[i] * b[j] * g;
        }
    }
    (n, ng)
}

/// n_eff and n_g at 1.55 µm of a `width` × `thickness` µm silicon strip in silica, TE-like:
/// exactly [`ANCHOR`] at 500 × 220 nm.
///
/// # Errors
///
/// [`Error::InvalidValue`] outside the table's widths and thicknesses.
pub(super) fn guide(width: f64, thickness: f64) -> Result<(f64, f64)> {
    within("widths", width, &WIDTHS)?;
    within("thicknesses", thickness, &THICKNESSES)?;
    let (n, ng) = guide_table(width, thickness);
    let (n0, ng0) = guide_table(AT.0, AT.1);
    Ok((ANCHOR.0 + (n - n0), ANCHOR.1 + (ng - ng0)))
}

/// ln C interpolated at (gap, width, thickness), and its derivative in the gap.
fn coupling_table(gap: f64, width: f64, thickness: f64) -> (f64, f64) {
    let (a, _) = lagrange(&WIDTHS, width);
    let (b, _) = lagrange(&PAIR_THICKNESSES, thickness);
    let (c, dc) = lagrange(&GAPS, gap);
    let mut value = 0.0;
    let mut slope = 0.0;
    for (i, plane) in COUPLING.iter().enumerate() {
        for (j, row) in plane.iter().enumerate() {
            for (k, &v) in row.iter().enumerate() {
                let ln = libm::log(v);
                value += a[i] * b[j] * c[k] * ln;
                slope += a[i] * b[j] * dc[k] * ln;
            }
        }
    }
    (value, slope)
}

/// The coupling C (1/µm) of two `width` × `thickness` µm strips `gap` µm apart at 1.55 µm,
/// and its decay γ = −d ln C/d gap (1/µm): exactly [`COUPLING_ANCHOR`] at a 200 nm gap
/// between 500 × 220 nm strips.
///
/// # Errors
///
/// [`Error::InvalidValue`] outside the table's gaps, widths and thicknesses.
pub(super) fn coupling(gap: f64, width: f64, thickness: f64) -> Result<(f64, f64)> {
    within("gaps", gap, &GAPS)?;
    within("widths", width, &WIDTHS)?;
    within("thicknesses", thickness, &PAIR_THICKNESSES)?;
    let (ln, slope) = coupling_table(gap, width, thickness);
    let (ln0, _) = coupling_table(AT.2, AT.0, AT.1);
    Ok((COUPLING_ANCHOR * libm::exp(ln - ln0), -slope))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fits_are_their_anchors_at_the_library_strip() {
        assert_eq!(guide(0.5, 0.22).unwrap(), ANCHOR);
        assert_eq!(coupling(0.2, 0.5, 0.22).unwrap().0, COUPLING_ANCHOR);
    }

    #[test]
    fn the_fits_pass_through_their_nodes_less_the_anchors_shift() {
        let (n0, ng0) = GUIDE[2][2];
        for (i, &w) in WIDTHS.iter().enumerate() {
            for (j, &t) in THICKNESSES.iter().enumerate() {
                let (n, ng) = guide(w, t).unwrap();
                let (e, g) = GUIDE[i][j];
                assert!((n - (e - n0 + ANCHOR.0)).abs() < 1e-14, "{w} {t}");
                assert!((ng - (g - ng0 + ANCHOR.1)).abs() < 1e-14, "{w} {t}");
            }
        }
        let scale = COUPLING_ANCHOR / COUPLING[2][1][1];
        for (i, &w) in WIDTHS.iter().enumerate() {
            for (j, &t) in PAIR_THICKNESSES.iter().enumerate() {
                for (k, &g) in GAPS.iter().enumerate() {
                    let (c, _) = coupling(g, w, t).unwrap();
                    let expected = COUPLING[i][j][k] * scale;
                    assert!((c / expected - 1.0).abs() < 1e-13, "{w} {t} {g}");
                }
            }
        }
    }

    #[test]
    fn the_decay_is_the_slope_of_ln_c() {
        let (g, w, t) = (0.23, 0.47, 0.215);
        let h = 1e-4;
        let ln = |g: f64| coupling(g, w, t).unwrap().0.ln();
        let numeric = -(ln(g + h) - ln(g - h)) / (2.0 * h);
        let (_, gamma) = coupling(g, w, t).unwrap();
        assert!((gamma - numeric).abs() < 1e-6 * gamma, "{gamma} {numeric}");
        // about 8.8 per µm near 200 nm: C falls e-fold every 114 nm
        assert!((8.0..10.0).contains(&coupling(0.2, 0.5, 0.22).unwrap().1));
    }

    #[test]
    fn outside_the_tables_the_fit_refuses() {
        assert!(guide(0.39, 0.22).is_err());
        assert!(guide(0.5, 0.25).is_err());
        assert!(coupling(0.05, 0.5, 0.22).is_err());
        assert!(coupling(0.2, 0.5, 0.21).is_ok());
    }
}
