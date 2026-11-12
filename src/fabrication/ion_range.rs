//! An implanted ion's projected range Rp (its mean depth) and projected straggle ΔRp (the
//! standard deviation of its depth), by two routes from the sources in hand, for SIMOX's oxygen
//! in silicon (#622). Neither sets the SIMOX route's film, which stays the published pair of
//! energy and film ([`super::simox`]); these are the range model that route's comparison needs.
//!
//! 1. **Smith's tables** ([`smith`]): B. J. Smith's 1971 normalised projected ranges and
//!    straggles, printed in G. Dearnaley, J. H. Freeman, R. S. Nelson, J. Stephen, *Ion
//!    Implantation* (North-Holland, 1973), Appendix 3, pp. 766–777, ISBN 9780444104885: Rp and
//!    ΔRp in mg/cm² of the substrate for ions of Z₁ = 5, 10, 20, … 80 in substrates of Z₂ = 10,
//!    20, … 80, at 10 to 100 keV in steps of 10 and 100 to 1000 keV in steps of 100, computed with
//!    "his modified form of the program by Johnson and Gibbons (1969)", the substrate taken at
//!    unit density with the masses of p. 766 (M = 21 for Z = 10, 44 for 20, 67 for 30). The
//!    method the appendix gives for an ion and a substrate between the tabulated ones (p. 767):
//!    plot Rp and ΔRp against Z₁ on double logarithmic paper, then against Z₂, and read off the
//!    value; then divide by the substrate's density. [`SMITH`] holds the part of Tables A3.1–A3.3
//!    (pp. 769–771) that oxygen (Z₁ = 8) in silicon (Z₂ = 14) needs, transcribed from the page
//!    images: Z₁ = 5, 10 and 20 in Z₂ = 10, 20 and 30. The appendix interpolates in neither the
//!    energy (150 keV isn't tabulated) nor says how to draw the curves, so both are ours, done
//!    two ways ([`Interpolation`]): straight lines between the two neighbouring points on the
//!    double logarithmic paper, or the parabola through the three nearest; the spread between
//!    them is the interpolation's own uncertainty.
//! 2. **PRAL with the universal nuclear stopping and Lindhard–Scharff electronic stopping**
//!    ([`pral`]): J. P. Biersack's projected range algorithm as J. F. Ziegler, J. P. Biersack,
//!    M. D. Ziegler, *SRIM: The Stopping and Range of Ions in Matter* (SRIM Co., 2008), ISBN
//!    978-0-9654207-1-6, gives it in Chapter 10, to first order (Eqs. 10-21, 10-26 and 10-27,
//!    pp. 10-7 and 10-12, as the differential equations 10-20, 10-23 and 10-24 they step):
//!    dx̄/dE = 1/S_t − μS_n x̄/(2S_t E), d⟨x²⟩/dE = (⟨z²⟩ − ⟨x²⟩)μS_n/(E S_t) + 2x̄/S_t,
//!    d⟨z²⟩/dE = (⟨x²⟩ − ⟨z²⟩)μS_n/(E S_t), μ = M₂/M₁, from zero at E = 0, and
//!    ΔRp² = ⟨x²⟩ − x̄² (Eq. 10-22). The nuclear stopping S_n is the book's universal one
//!    (Eqs. 10-43 to 10-46, pp. 10-17 and 10-18, from the universal (ZBL) screening length
//!    a_U = 0.8853 a₀/(Z₁^0.23 + Z₂^0.23), Eq. 10-39); the electronic stopping S_e is
//!    Lindhard and Scharff's, S_e = ξₑ·8πe²a₀·(Z₁Z₂/Z)·v/v₀, Z = (Z₁^⅔ + Z₂^⅔)^{3/2}, ξₑ =
//!    Z₁^{1/6} (J. Lindhard, M. Scharff, H. E. Schiøtt, Mat. Fys. Medd. Dan. Vid. Selsk. 33,
//!    no. 14 (1963), Eq. 2.5, p. 6), not SRIM's own, whose coefficients the book doesn't print
//!    for this pair. The higher-order form (Eqs. 10-28 to 10-30) needs the nuclear straggling
//!    Q_n, whose fit (Eqs. 10-47 and 10-48, p. 10-18) the book prints without its units, so it
//!    isn't taken; the book says the first order reproduces LSS's ranges and that Q_n mainly
//!    raises ΔRp (p. 10-13). Electronic straggling is neglected, as in LSS.
//!
//! Neither route knows the silicon's crystal: both are for an amorphous target, as de Souza et
//! al.'s measurements were made in silicon amorphized to 280 nm (p. 47).

use crate::{Error, Result};

fn invalid(message: String) -> Error {
    Error::invalid("ion range", message)
}

/// An ion and the substrate it is implanted into: their atomic numbers and masses (u), and the
/// substrate's density (g/cm³).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Pair {
    /// The ion's atomic number.
    pub z1: f64,
    /// The ion's mass, u.
    pub m1: f64,
    /// The substrate's atomic number.
    pub z2: f64,
    /// The substrate's atomic mass, u.
    pub m2: f64,
    /// The substrate's density, g/cm³.
    pub density: f64,
}

/// ¹⁶O⁺ in silicon: oxygen-16's mass (15.995 u) and silicon's mean atomic mass (28.086 u), and
/// silicon's density as Dearnaley et al.'s list gives it, 2.33 g/cm³ (p. 767).
pub const OXYGEN_IN_SILICON: Pair = Pair {
    z1: 8.0,
    m1: 15.995,
    z2: 14.0,
    m2: 28.086,
    density: 2.33,
};

impl Pair {
    /// The pair with the ion's mass `m1` u instead: ¹⁸O (17.999 u), de Souza et al.'s ion.
    pub fn with_ion_mass(self, m1: f64) -> Pair {
        Pair { m1, ..self }
    }

    /// The substrate's atoms per cm³.
    pub fn atoms(&self) -> f64 {
        self.density * AVOGADRO / self.m2
    }
}

/// Avogadro's number, per mole.
const AVOGADRO: f64 = 6.022_140_76e23;

/// A projected range and straggle.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct Range {
    /// The projected range Rp, nm.
    pub rp_nm: f64,
    /// The projected straggle ΔRp, nm.
    pub straggle_nm: f64,
}

/// How [`smith`] reads between Smith's tabulated ions, substrates and energies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Interpolation {
    /// Straight lines on double logarithmic paper between the two neighbouring tabulated points.
    Lines,
    /// The parabola on double logarithmic paper through the three nearest tabulated points
    /// (nearest in the logarithm).
    Curves,
}

/// The energies of Smith's tables, keV (Dearnaley et al., pp. 769–777).
pub const SMITH_ENERGIES: [f64; 19] = [
    10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 200.0, 300.0, 400.0, 500.0, 600.0,
    700.0, 800.0, 900.0, 1000.0,
];

/// The ions' atomic numbers of [`SMITH`].
pub const SMITH_IONS: [f64; 3] = [5.0, 10.0, 20.0];

/// The substrates' atomic numbers of [`SMITH`].
pub const SMITH_SUBSTRATES: [f64; 3] = [10.0, 20.0, 30.0];

/// The page images' blank entries: Smith's program "is unstable when computing ΔRp for the
/// higher values of Z₂ and where this has occurred the value has not been included" (p. 767).
const BLANK: f64 = f64::NAN;

/// Smith's normalised projected range Rp and straggle ΔRp, mg/cm², as Dearnaley et al. print
/// them: `SMITH[ion][substrate][energy] = (Rp, ΔRp)`, the ions [`SMITH_IONS`] (Z₁ = 5, Table
/// A3.1, p. 769; Z₁ = 10, Table A3.2, p. 770; Z₁ = 20, Table A3.3, p. 771), the substrates
/// [`SMITH_SUBSTRATES`] (the tables' first three columns, Z₂ = 10, 20, 30), the energies
/// [`SMITH_ENERGIES`]. Transcribed from the page images (PDF pp. 789–791 of the Internet
/// Archive's scan, whose text layer garbles them); a blank ΔRp is NaN.
#[rustfmt::skip]
#[allow(clippy::approx_constant)] // 0.3010 mg/cm² is a printed entry, not log₁₀ 2
pub const SMITH: [[[(f64, f64); 19]; 3]; 3] = [
    // Z1 = 5, Table A3.1, p. 769
    [
        [(0.0084, 0.0038), (0.0175, 0.0063), (0.0267, 0.0083), (0.0360, 0.0101), (0.0453, 0.0118),
         (0.0541, 0.0131), (0.0625, 0.0142), (0.0707, 0.0152), (0.0787, 0.0161), (0.0865, 0.0170),
         (0.1542, 0.0220), (0.2098, 0.0246), (0.2579, 0.0262), (0.3010, 0.0274), (0.3404, 0.0282),
         (0.3769, 0.0289), (0.4110, 0.0294), (0.4431, 0.0298), (0.4737, 0.0302)],
        [(0.0111, 0.0071), (0.0224, 0.0114), (0.0341, 0.0154), (0.0459, 0.0190), (0.0577, 0.0222),
         (0.0694, 0.0250), (0.0812, 0.0276), (0.0929, 0.0299), (0.1044, 0.0321), (0.1158, 0.0340),
         (0.2162, 0.0466), (0.3017, 0.0537), (0.3779, 0.0583), (0.4468, 0.0616), (0.5098, 0.0641),
         (0.5685, 0.0660), (0.6236, 0.0676), (0.6758, 0.0690), (0.7255, 0.0701)],
        [(0.0130, BLANK), (0.0252, 0.0165), (0.0383, 0.0221), (0.0515, 0.0272), (0.0649, 0.0319),
         (0.0784, 0.0361), (0.0918, 0.0400), (0.1050, 0.0435), (0.1182, 0.0468), (0.1314, 0.0499),
         (0.2562, 0.0711), (0.3654, 0.0837), (0.4637, 0.0922), (0.5534, 0.0984), (0.6372, 0.1031),
         (0.7156, 0.1068), (0.7891, 0.1099), (0.8587, 0.1125), (0.9251, 0.1147)],
    ],
    // Z1 = 10, Table A3.2, p. 770
    [
        [(0.0042, 0.0020), (0.0081, 0.0036), (0.0123, 0.0050), (0.0165, 0.0064), (0.0209, 0.0076),
         (0.0254, 0.0088), (0.0298, 0.0099), (0.0343, 0.0108), (0.0388, 0.0117), (0.0433, 0.0126),
         (0.0870, 0.0187), (0.1263, 0.0220), (0.1618, 0.0241), (0.1945, 0.0257), (0.2251, 0.0268),
         (0.2538, 0.0277), (0.2807, 0.0284), (0.3062, 0.0290), (0.3306, 0.0296)],
        [(0.0060, 0.0034), (0.0112, 0.0058), (0.0164, 0.0080), (0.0219, 0.0100), (0.0275, 0.0118),
         (0.0332, 0.0135), (0.0388, 0.0151), (0.0446, 0.0165), (0.0503, 0.0180), (0.0561, 0.0195),
         (0.1134, 0.0314), (0.1681, 0.0398), (0.2178, 0.0458), (0.2649, 0.0505), (0.3090, 0.0543),
         (0.3503, 0.0573), (0.3898, 0.0599), (0.4276, 0.0622), (0.4639, 0.0641)],
        [(0.0072, BLANK), (0.0133, 0.0081), (0.0193, 0.0107), (0.0253, 0.0133), (0.0317, 0.0160),
         (0.0381, 0.0184), (0.0445, 0.0208), (0.0510, 0.0231), (0.0575, 0.0252), (0.0640, 0.0273),
         (0.1293, 0.0447), (0.1935, 0.0576), (0.2552, 0.0674), (0.3119, 0.0750), (0.3661, 0.0812),
         (0.4182, 0.0865), (0.4681, 0.0910), (0.5155, 0.0949), (0.5608, 0.0983)],
    ],
    // Z1 = 20, Table A3.3, p. 771
    [
        [(0.0026, 0.0011), (0.0045, 0.0018), (0.0064, 0.0025), (0.0083, 0.0032), (0.0101, 0.0039),
         (0.0120, 0.0045), (0.0140, 0.0052), (0.0160, 0.0058), (0.0180, 0.0065), (0.0201, 0.0071),
         (0.0413, 0.0129), (0.0634, 0.0176), (0.0860, 0.0217), (0.1085, 0.0252), (0.1305, 0.0281),
         (0.1513, 0.0304), (0.1716, 0.0324), (0.1916, 0.0342), (0.2111, 0.0358)],
        [(0.0036, 0.0019), (0.0063, 0.0031), (0.0088, 0.0043), (0.0113, 0.0054), (0.0138, 0.0065),
         (0.0163, 0.0075), (0.0187, 0.0085), (0.0213, 0.0094), (0.0238, 0.0104), (0.0264, 0.0114),
         (0.0534, 0.0202), (0.0813, 0.0276), (0.1094, 0.0337), (0.1377, 0.0389), (0.1659, 0.0436),
         (0.1940, 0.0477), (0.2213, 0.0512), (0.2478, 0.0542), (0.2731, 0.0567)],
        [(0.0045, 0.0026), (0.0077, 0.0043), (0.0107, 0.0059), (0.0137, 0.0073), (0.0167, 0.0087),
         (0.0196, 0.0100), (0.0225, 0.0113), (0.0254, 0.0125), (0.0283, 0.0137), (0.0313, 0.0149),
         (0.0621, 0.0258), (0.0938, 0.0350), (0.1261, 0.0428), (0.1582, 0.0493), (0.1901, 0.0549),
         (0.2220, 0.0600), (0.2539, 0.0645), (0.2854, 0.0687), (0.3163, 0.0732)],
    ],
];

/// The indices of `grid` (increasing) to interpolate `at` from: the two neighbours, or the three
/// nearest in the logarithm.
fn nodes(grid: &[f64], at: f64, how: Interpolation) -> Vec<usize> {
    // a tabulated point is read as printed
    if let Some(k) = grid.iter().position(|g| *g == at) {
        return vec![k];
    }
    let n = grid.len();
    // the interval holding `at`
    let i = grid
        .windows(2)
        .position(|w| at >= w[0] && at <= w[1])
        .unwrap_or(if at < grid[0] { 0 } else { n - 2 });
    match how {
        Interpolation::Lines => vec![i, i + 1],
        Interpolation::Curves if n < 3 => vec![i, i + 1],
        Interpolation::Curves => {
            // the third point: the nearer in the logarithm of the next ones out, on either side
            let below = (i > 0).then(|| (at / grid[i - 1]).ln().abs());
            let above = (i + 2 < n).then(|| (grid[i + 2] / at).ln().abs());
            match (below, above) {
                (Some(b), Some(a)) if b <= a => vec![i - 1, i, i + 1],
                (Some(_), None) => vec![i - 1, i, i + 1],
                _ => vec![i, i + 1, i + 2],
            }
        }
    }
}

/// The polynomial through `(xs, ys)` (logarithms) at `x`: Lagrange's form.
fn lagrange(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let mut sum = 0.0;
    for (j, (&xj, &yj)) in xs.iter().zip(ys).enumerate() {
        let mut l = 1.0;
        for (m, &xm) in xs.iter().enumerate() {
            if m != j {
                l *= (x - xm) / (xj - xm);
            }
        }
        sum += l * yj;
    }
    sum
}

/// Smith's projected range and straggle of `pair` at `energy` keV, read from [`SMITH`] by the
/// appendix's method: at each tabulated ion and substrate, the energy read between the tabulated
/// ones; then against Z₁, then against Z₂, each on double logarithmic paper, `how` says how;
/// the mass thickness divided by the substrate's density.
///
/// # Errors
///
/// [`Error::InvalidValue`] for an energy outside 10–1000 keV, an ion outside Z₁ = 5–20 or a
/// substrate outside Z₂ = 10–30 (the part of the tables transcribed), and a straggle the
/// tables leave blank where the interpolation needs it.
pub fn smith(pair: &Pair, energy: f64, how: Interpolation) -> Result<Range> {
    let within = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
    if !within(energy, 10.0, 1000.0) {
        return Err(invalid(format!(
            "Smith's tables run from 10 to 1000 keV, not {energy} keV"
        )));
    }
    if !within(pair.z1, 5.0, 20.0) || !within(pair.z2, 10.0, 30.0) {
        return Err(invalid(format!(
            "the transcribed part of Smith's tables holds ions of Z₁ = 5 to 20 in substrates of \
             Z₂ = 10 to 30, not Z₁ = {} in Z₂ = {}",
            pair.z1, pair.z2
        )));
    }
    if !(pair.density.is_finite() && pair.density > 0.0) {
        return Err(invalid(format!(
            "a density must be positive, not {} g/cm³",
            pair.density
        )));
    }
    let e = nodes(&SMITH_ENERGIES, energy, how);
    let i1 = nodes(&SMITH_IONS, pair.z1, how);
    let i2 = nodes(&SMITH_SUBSTRATES, pair.z2, how);
    let mut out = [0.0; 2];
    for (q, value) in out.iter_mut().enumerate() {
        // the energy at each (ion, substrate) the next steps need, then Z₁, then Z₂
        let mut by_substrate = Vec::with_capacity(i2.len());
        for &b in &i2 {
            let mut by_ion = Vec::with_capacity(i1.len());
            for &a in &i1 {
                let ys: Vec<f64> = e
                    .iter()
                    .map(|&k| {
                        let (rp, d) = SMITH[a][b][k];
                        if q == 0 { rp } else { d }
                    })
                    .collect();
                if ys.iter().any(|y| !y.is_finite()) {
                    return Err(invalid(format!(
                        "Smith's table for Z₁ = {} leaves ΔRp blank in Z₂ = {} near {energy} keV \
                         (the program was unstable there, p. 767)",
                        SMITH_IONS[a], SMITH_SUBSTRATES[b]
                    )));
                }
                let es: Vec<f64> = e.iter().map(|&k| SMITH_ENERGIES[k]).collect();
                by_ion.push(lagrange(
                    &es.iter().map(|x| x.ln()).collect::<Vec<_>>(),
                    &ys.iter().map(|y| y.ln()).collect::<Vec<_>>(),
                    energy.ln(),
                ));
            }
            let zs: Vec<f64> = i1.iter().map(|&a| SMITH_IONS[a].ln()).collect();
            by_substrate.push(lagrange(&zs, &by_ion, pair.z1.ln()));
        }
        let zs: Vec<f64> = i2.iter().map(|&b| SMITH_SUBSTRATES[b].ln()).collect();
        let mg_per_cm2 = lagrange(&zs, &by_substrate, pair.z2.ln()).exp();
        // mg/cm² to g/cm², over g/cm³, cm to nm
        *value = mg_per_cm2 * 1e-3 / pair.density * 1e7;
    }
    Ok(Range {
        rp_nm: out[0],
        straggle_nm: out[1],
    })
}

/// The Bohr radius, Å.
const BOHR_A: f64 = 0.529_177_210_9;

/// e²/(4πε₀), eV·Å.
const E2_EV_A: f64 = 14.399_645;

/// The fine-structure constant: v₀/c, Bohr's velocity over light's.
const ALPHA: f64 = 1.0 / 137.035_999;

/// The atomic mass unit's energy, keV.
const U_KEV: f64 = 931_494.102_42;

/// The universal (ZBL) nuclear stopping of `pair` at `energy` keV, eV·cm² per atom: Ziegler et
/// al.'s Eqs. 10-43 to 10-46 (pp. 10-17 and 10-18).
pub fn nuclear_stopping(pair: &Pair, energy: f64) -> f64 {
    let Pair { z1, m1, z2, m2, .. } = *pair;
    let screen = z1.powf(0.23) + z2.powf(0.23);
    // Eq. 10-44, E in keV
    let eps = 32.53 * m2 * energy / (z1 * z2 * (m1 + m2) * screen);
    // Eqs. 10-45 and 10-46
    let reduced = if eps <= 30.0 {
        (1.0 + 1.1383 * eps).ln()
            / (2.0 * (eps + 0.01321 * eps.powf(0.21226) + 0.19593 * eps.sqrt()))
    } else {
        eps.ln() / (2.0 * eps)
    };
    // Eq. 10-43
    8.462e-15 * z1 * z2 * m1 * reduced / ((m1 + m2) * screen)
}

/// Lindhard and Scharff's electronic stopping of `pair` at `energy` keV, eV·cm² per atom:
/// ξₑ·8πe²a₀·(Z₁Z₂/Z)·v/v₀, Z = (Z₁^⅔ + Z₂^⅔)^{3/2}, ξₑ = Z₁^{1/6} (Lindhard, Scharff and
/// Schiøtt 1963, Eq. 2.5, p. 6; "ξₑ ≃ Z₁^{1/6}", p. 8). It holds below v₁ = v₀Z₁^⅔ (p. 6):
/// for oxygen below about 3.2 MeV.
pub fn electronic_stopping(pair: &Pair, energy: f64) -> f64 {
    let Pair { z1, m1, z2, .. } = *pair;
    let z = (z1.powf(2.0 / 3.0) + z2.powf(2.0 / 3.0)).powf(1.5);
    let v_over_v0 = (2.0 * energy / (m1 * U_KEV)).sqrt() / ALPHA;
    // eV·Å² to eV·cm²
    z1.powf(1.0 / 6.0) * 8.0 * std::f64::consts::PI * E2_EV_A * BOHR_A * z1 * z2 / z
        * v_over_v0
        * 1e-16
}

/// The steps per e-fold of energy PRAL's equations are integrated with.
const STEPS_PER_E_FOLD: f64 = 400.0;

/// The energy PRAL's equations start from, keV, all moments zero there: what that leaves out of
/// the moments at the implant's energy falls as a power of E_start/E, about 1e-8 of ⟨x²⟩ at
/// 150 keV.
const START_KEV: f64 = 1e-6;

/// The moments of PRAL's first order (Ziegler et al., Eqs. 10-20, 10-23 and 10-24), from
/// `START_KEV` to `energy` keV, for nuclear and electronic stopping `nuclear(E)` and
/// `electronic(E)` (eV·cm² per atom, E in keV), the mass ratio `mu` = M₂/M₁ and `atoms` per
/// cm³: (x̄, ⟨x²⟩), cm and cm². Fourth-order Runge–Kutta in ln E.
pub(crate) fn pral_moments(
    energy: f64,
    mu: f64,
    atoms: f64,
    start: f64,
    nuclear: impl Fn(f64) -> f64,
    electronic: impl Fn(f64) -> f64,
) -> (f64, f64) {
    // y = (x̄, ⟨x²⟩ + ⟨z²⟩, ⟨z²⟩), u = ln(E/keV); E dy/dE, energies in eV in the ratio E/S_t
    let rate = |u: f64, y: [f64; 3]| -> [f64; 3] {
        let e = u.exp();
        let (sn, se) = (nuclear(e), electronic(e));
        let st = atoms * (sn + se); // eV/cm
        let e_ev = e * 1e3;
        let ratio = mu * sn / (sn + se); // μS_n/S_t, per atom over per atom
        let (x, xi, z2) = (y[0], y[1], y[2]);
        let x2 = xi - z2;
        [
            e_ev / st - 0.5 * ratio * x,
            2.0 * x * e_ev / st,
            (x2 - z2) * ratio,
        ]
    };
    let (u0, u1) = (start.ln(), energy.ln());
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = (((u1 - u0) * STEPS_PER_E_FOLD).ceil() as usize).max(1);
    #[allow(clippy::cast_precision_loss)]
    let h = (u1 - u0) / n as f64;
    let mut y = [0.0; 3];
    let add =
        |y: [f64; 3], k: [f64; 3], s: f64| [y[0] + s * k[0], y[1] + s * k[1], y[2] + s * k[2]];
    for i in 0..n {
        #[allow(clippy::cast_precision_loss)]
        let u = u0 + h * i as f64;
        let k1 = rate(u, y);
        let k2 = rate(u + 0.5 * h, add(y, k1, 0.5 * h));
        let k3 = rate(u + 0.5 * h, add(y, k2, 0.5 * h));
        let k4 = rate(u + h, add(y, k3, h));
        for j in 0..3 {
            y[j] += h / 6.0 * (k1[j] + 2.0 * k2[j] + 2.0 * k3[j] + k4[j]);
        }
    }
    (y[0], y[1] - y[2])
}

/// PRAL's projected range and straggle of `pair` at `energy` keV, to first order, with the
/// universal nuclear stopping ([`nuclear_stopping`]) and Lindhard and Scharff's electronic
/// stopping ([`electronic_stopping`]).
///
/// # Errors
///
/// [`Error::InvalidValue`] for an energy outside 1 keV to 1 MeV (the electronic stopping's
/// velocity proportionality holds below v₀Z₁^⅔, 3.2 MeV for oxygen; the range is checked to
/// 1 MeV), and a pair whose numbers aren't positive.
pub fn pral(pair: &Pair, energy: f64) -> Result<Range> {
    if !(energy.is_finite() && (1.0..=1000.0).contains(&energy)) {
        return Err(invalid(format!(
            "the range model runs from 1 keV to 1 MeV, not {energy} keV"
        )));
    }
    let Pair {
        z1,
        m1,
        z2,
        m2,
        density,
    } = *pair;
    if ![z1, m1, z2, m2, density]
        .iter()
        .all(|v| v.is_finite() && *v > 0.0)
    {
        return Err(invalid(format!(
            "an ion and substrate need positive numbers: {pair:?}"
        )));
    }
    let (x, x2) = pral_moments(
        energy,
        m2 / m1,
        pair.atoms(),
        START_KEV,
        |e| nuclear_stopping(pair, e),
        |e| electronic_stopping(pair, e),
    );
    Ok(Range {
        rp_nm: x * 1e7,
        straggle_nm: (x2 - x * x).max(0.0).sqrt() * 1e7,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smiths_tables_rise_with_the_energy_and_the_substrate_and_fall_with_the_ion() {
        // a check of the transcription: each column rises with the energy, each row with Z₂,
        // and a heavier ion goes less deep
        for (a, ion) in SMITH.iter().enumerate() {
            for (b, col) in ion.iter().enumerate() {
                for w in col.windows(2) {
                    assert!(w[1].0 > w[0].0, "Z1 {a} Z2 {b}: {w:?}");
                    // a blank straggle is NaN
                    assert!(w[0].1.is_nan() || w[1].1 > w[0].1, "Z1 {a} Z2 {b}: {w:?}");
                }
                for (k, here) in col.iter().enumerate() {
                    if b > 0 {
                        assert!(here.0 > ion[b - 1][k].0, "Z1 {a} Z2 {b} E {k}");
                    }
                    if a > 0 {
                        assert!(here.0 < SMITH[a - 1][b][k].0, "Z1 {a} Z2 {b} E {k}");
                    }
                }
            }
        }
    }

    #[test]
    fn smiths_tables_give_back_their_own_entries() {
        // at a tabulated ion, substrate and energy, both readings are the printed number
        for how in [Interpolation::Lines, Interpolation::Curves] {
            for (a, &z1) in SMITH_IONS.iter().enumerate() {
                for (b, &z2) in SMITH_SUBSTRATES.iter().enumerate() {
                    for (k, &e) in SMITH_ENERGIES.iter().enumerate().skip(1) {
                        let pair = Pair {
                            z1,
                            z2,
                            density: 1.0,
                            ..OXYGEN_IN_SILICON
                        };
                        let got = smith(&pair, e, how).unwrap();
                        let (rp, d) = SMITH[a][b][k];
                        assert!((got.rp_nm - rp * 1e4).abs() < 1e-9 * rp * 1e4, "{got:?}");
                        assert!(
                            (got.straggle_nm - d * 1e4).abs() < 1e-9 * d * 1e4,
                            "{got:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn smiths_tables_put_oxygen_at_150_kev_near_izumis_reading() {
        // Izumi et al. 1978 took 3700 Å from "the table given by Smith" (p. 593); how they read
        // between its points isn't said, so the two readings bracket ours
        let lines = smith(&OXYGEN_IN_SILICON, 150.0, Interpolation::Lines).unwrap();
        let curves = smith(&OXYGEN_IN_SILICON, 150.0, Interpolation::Curves).unwrap();
        for r in [lines, curves] {
            assert!(r.rp_nm > 340.0 && r.rp_nm < 420.0, "{r:?}");
            assert!(r.straggle_nm > 50.0 && r.straggle_nm < 150.0, "{r:?}");
        }
        assert!(smith(&OXYGEN_IN_SILICON, 5.0, Interpolation::Lines).is_err());
        let gold = Pair {
            z1: 79.0,
            ..OXYGEN_IN_SILICON
        };
        assert!(smith(&gold, 150.0, Interpolation::Lines).is_err());
    }

    #[test]
    fn pral_gives_its_closed_forms_for_power_law_stopping() {
        // nuclear stopping alone, S = A E^(1−B): x̄ = s/(1 + μ/2B) and ⟨x²⟩ = s²/(1 + μ/B)
        // (Ziegler et al., Eqs. 10-14 and 10-15), s = E^B/(N A B) the path length
        let (mu, atoms, b, a) = (1.7555, 5e22, 0.5, 3e-15);
        let s_of = |e: f64| (e * 1e3).powf(b) / (atoms * a * b);
        let e = 150.0;
        let (x, x2) = pral_moments(
            e,
            mu,
            atoms,
            1e-10 * e,
            |e| a * (e * 1e3).powf(1.0 - b),
            |_| 0.0,
        );
        let s = s_of(e);
        let rx = x / (s / (1.0 + mu / (2.0 * b))) - 1.0;
        let rx2 = x2 / (s * s / (1.0 + mu / b)) - 1.0;
        assert!(rx.abs() < 1e-7, "x̄ off by {rx:e}");
        assert!(rx2.abs() < 1e-7, "⟨x²⟩ off by {rx2:e}");
    }

    #[test]
    fn pral_puts_oxygen_in_silicon_where_the_calculations_do() {
        // the stopping in physical units: Lindhard and Scharff's k, in reduced units 0.185 for
        // O in Si (our arithmetic from Eq. 2.10, docs/methods/fabrication.md), is 49.5e-15
        // eV·cm² at 100 keV
        let se = electronic_stopping(&OXYGEN_IN_SILICON, 100.0);
        assert!((se / 49.5e-15 - 1.0).abs() < 0.01, "{se:e}");
        // the ranges rise with the energy, and stay between the measurement and the
        // calculations' spread (de Souza et al.'s 291.8 nm and SRIM's 367 nm at 150 keV)
        let mut last = 0.0;
        for e in [25.0, 50.0, 75.0, 100.0, 150.0, 200.0, 300.0] {
            let r = pral(&OXYGEN_IN_SILICON, e).unwrap();
            assert!(
                r.rp_nm > last && r.straggle_nm > 0.0 && r.straggle_nm < r.rp_nm,
                "{r:?}"
            );
            last = r.rp_nm;
        }
        let r = pral(&OXYGEN_IN_SILICON, 150.0).unwrap();
        assert!(r.rp_nm > 300.0 && r.rp_nm < 420.0, "{r:?}");
        assert!(pral(&OXYGEN_IN_SILICON, 0.5).is_err());
    }

    #[test]
    fn the_methods_page_prints_the_models_numbers() {
        // docs/methods/fabrication.md's table of O⁺ in Si, nm, to its printed 0.1 nm
        let close = |got: f64, printed: f64| (got - printed).abs() <= 0.05 + 1e-9;
        let energies = [25.0, 50.0, 75.0, 100.0, 150.0, 200.0];
        let pral_rp = [63.3, 126.8, 190.4, 252.7, 371.8, 483.6];
        let pral_d = [25.7, 42.3, 55.4, 66.0, 82.2, 94.0];
        let pral18 = [63.0, 126.6, 190.9, 254.3, 376.5, 491.9];
        let lines_rp = [64.4, 130.8, 197.9, 264.7, 391.3, 516.4];
        let lines_d = [28.2, 47.9, 63.5, 76.6, 96.1, 112.8];
        let curves_rp = [63.7, 130.9, 199.6, 268.2, 401.6, 528.3];
        let curves_d = [28.2, 47.9, 63.2, 76.3, 97.6, 113.1];
        let o18 = OXYGEN_IN_SILICON.with_ion_mass(17.999);
        for (k, &e) in energies.iter().enumerate() {
            let p = pral(&OXYGEN_IN_SILICON, e).unwrap();
            assert!(
                close(p.rp_nm, pral_rp[k]) && close(p.straggle_nm, pral_d[k]),
                "{e}: {p:?}"
            );
            assert!(close(pral(&o18, e).unwrap().rp_nm, pral18[k]), "{e}");
            let l = smith(&OXYGEN_IN_SILICON, e, Interpolation::Lines).unwrap();
            assert!(
                close(l.rp_nm, lines_rp[k]) && close(l.straggle_nm, lines_d[k]),
                "{e}: {l:?}"
            );
            let c = smith(&OXYGEN_IN_SILICON, e, Interpolation::Curves).unwrap();
            assert!(
                close(c.rp_nm, curves_rp[k]) && close(c.straggle_nm, curves_d[k]),
                "{e}: {c:?}"
            );
        }
        for (e, rp, d) in [(250.0, 588.6, 103.0), (300.0, 687.7, 110.1)] {
            let p = pral(&OXYGEN_IN_SILICON, e).unwrap();
            assert!(close(p.rp_nm, rp) && close(p.straggle_nm, d), "{e}: {p:?}");
        }
    }
}
