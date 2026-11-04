//! The validation cases of a mode's quantities ([`crate::mode::quantities`], #432): the
//! effective area of a Gaussian in closed form, whole and behind mirror walls; a silicon slab's
//! TE area against its exact field's, with the grid's order; the slab's confinement against the
//! exact power fractions of [`crate::mode::multilayer`]; and the orthogonality of a thick slab's
//! modes, which goes to 0 with the grid.

use num_complex::Complex64 as c64;

use crate::mode::Polarization;
use crate::mode::fields::Fields;
use crate::mode::multilayer::Multilayer;
use crate::mode::quantities::{Product, Region, orthogonality};
use crate::mode::slab::Slab;
use crate::mode::vector::{self, Boundaries, Boundary, CrossSection, Permittivity, VectorMode};
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

const CORE: f64 = 3.473;
const CLAD: f64 = 1.444;

fn lam() -> Wavelength {
    Wavelength::from_um_unchecked(1.55)
}

/// A Gaussian E_x = H_y = e^(−r²/w²) on square cells of side `h`, `n` of them along x and y
/// from `from`, with the given mirror planes: its S_z and |E_t|² are e^(−2r²/w²), and its
/// effective area π w² by either definition.
pub(crate) fn gaussian(
    w: f64,
    h: f64,
    from: [f64; 2],
    n: [usize; 2],
    mirrors: [Option<f64>; 2],
) -> Fields {
    let xc: Vec<f64> = (0..n[0]).map(|i| from[0] + (i as f64 + 0.5) * h).collect();
    let yc: Vec<f64> = (0..n[1]).map(|j| from[1] + (j as f64 + 0.5) * h).collect();
    let (mut e, mut hh) = (Vec::new(), Vec::new());
    let zero = c64::new(0.0, 0.0);
    for x in &xc {
        for y in &yc {
            let g = c64::new((-(x * x + y * y) / (w * w)).exp(), 0.0);
            e.push([g, zero, zero]);
            hh.push([zero, g, zero]);
        }
    }
    Fields {
        area: vec![h * h; xc.len() * yc.len()],
        xc,
        yc,
        e,
        h: hh,
        mirrors,
    }
}

/// The largest relative error of a Gaussian's area (w = 1 µm, 50 nm cells, ±6 µm), by Eqs. 41
/// and 44, on the whole window, its half behind a wall at x = 0, and its quarter.
pub(crate) fn gaussian_area_error() -> f64 {
    let (w, h) = (1.0, 0.05);
    let exact = std::f64::consts::PI * w * w;
    [
        gaussian(w, h, [-6.0, -6.0], [240, 240], [None, None]),
        gaussian(w, h, [0.0, -6.0], [120, 240], [Some(0.0), None]),
        gaussian(w, h, [0.0, 0.0], [120, 120], [Some(0.0), Some(0.0)]),
    ]
    .iter()
    .flat_map(|f| [f.effective_area(), f.effective_area_transverse()])
    .map(|a| (a / exact - 1.0).abs())
    .fold(0.0, f64::max)
}

/// A slab of 3.473 in 1.444, `t` µm thick, interfaces normal to x, uniform in y between walls
/// 4 cells apart (electric for TE, E along y; magnetic for TM), at 1.55 µm, on a uniform grid of
/// spacing `h` over x from −2.01 to 2.01 µm, or over its half from 0 behind a wall that keeps the
/// even modes (`half`): its `count` modes of highest index.
pub(crate) fn slab(
    t: f64,
    h: f64,
    pol: Polarization,
    count: usize,
    half: bool,
) -> (CrossSection, Vec<VectorMode>) {
    let (along, across) = match pol {
        Polarization::Te => (Boundary::ElectricWall, Boundary::MagneticWall),
        Polarization::Tm => (Boundary::MagneticWall, Boundary::ElectricWall),
    };
    let x0 = if half { 0.0 } else { -2.01 };
    let n = ((2.01 - x0) / h).round() as usize;
    let cs = CrossSection::uniform((x0, 2.01, n), (0.0, 4.0 * h, 4), |x, _| {
        let n: f64 = if x.abs() < t / 2.0 { CORE } else { CLAD };
        Permittivity::isotropic(c64::new(n * n, 0.0))
    })
    .and_then(|cs| {
        cs.with_boundaries(Boundaries {
            west: if half { across } else { Boundary::Zero },
            south: along,
            north: along,
            ..Boundaries::default()
        })
    })
    .expect("a valid slab");
    let modes = vector::modes(&cs, lam(), count, None).expect("the solver converges");
    (cs, modes)
}

/// The exact TE₀ slab's effective width, (∫E²)² / ∫E⁴, µm, and its share of power in the core,
/// ∫_core E² / ∫E², in closed form from [`Slab`]'s field: in the core a cos(hx) + b sin(hx) =
/// R cos(hx − φ), outside its exponential tails.
pub(crate) fn slab_te_exact(t: f64) -> (f64, f64) {
    let m = Slab::new(CLAD, CORE, CLAD, Length::um(t))
        .expect("a valid slab")
        .modes(Polarization::Te, lam())[0];
    let (h, q, p) = m.wavenumbers();
    let (a, b) = (1.0, q / h);
    let (r2, phi) = (a * a + b * b, b.atan2(a));
    let s = |n: f64| (n * (h * t - phi)).sin() + (n * phi).sin();
    let core2 = r2 * (t / 2.0 + s(2.0) / (4.0 * h));
    let core4 = r2 * r2 * (3.0 * t / 8.0 + s(2.0) / (4.0 * h) + s(4.0) / (32.0 * h));
    let top = m.field(Length::um(t)).0;
    let all2 = core2 + a * a / (2.0 * q) + top * top / (2.0 * p);
    let all4 = core4 + a.powi(4) / (4.0 * q) + top.powi(4) / (4.0 * p);
    (all2 * all2 / all4, core2 / all2)
}

/// The full-vector TE₀ slab's effective width at spacing `h`: its area (Eq. 41) over the
/// window's height, 4h.
pub(crate) fn slab_te_width(t: f64, h: f64) -> f64 {
    let (cs, m) = slab(t, h, Polarization::Te, 1, false);
    m[0].fields(&cs).expect("its grid").effective_area() / (4.0 * h)
}

/// The full-vector slab's fundamental mode's confinement in the silicon at spacing `h`, and the
/// exact one from [`Multilayer`]'s power fractions (the core's share).
pub(crate) fn slab_confinement(pol: Polarization, t: f64, h: f64) -> (f64, f64) {
    let (cs, m) = slab(t, h, pol, 1, false);
    let si = Permittivity::isotropic(c64::new(CORE * CORE, 0.0));
    let got = m[0]
        .confinement(&cs, &Region::Material(si))
        .unwrap_or(f64::NAN);
    let n = |v: f64| c64::new(v, 0.0);
    let exact = Multilayer::new(n(CLAD), &[(n(CORE), Length::um(t))], n(CLAD))
        .and_then(|s| s.bound_modes(pol, lam()))
        .map(|modes| modes[0].power_fractions()[1])
        .unwrap_or(f64::NAN);
    (got, exact)
}

/// The exact TM₀ slab's field-weighted n̄₂ over the core's n₂, with n₂ in the core only (Afshar
/// and Monro's Eqs. 39 and 42): from [`Slab`]'s H_y and its derivative, Ampère's law gives (in
/// Z₀'s units) E_x = n_eff H_y / ε, E_z = i H_y′ / (k ε) and S_z = n_eff H_y² / ε, so
/// |E|² = (n_eff H_y / ε)² + (H_y′ / kε)² and E·E = (n_eff H_y / ε)² − (H_y′ / kε)². The
/// integrals by Simpson's rule on 2 × 10⁵ intervals across the core and each tail (to 40 decay
/// lengths), far below the grid's error.
pub(crate) fn slab_tm_kerr_exact(t: f64) -> f64 {
    let m = Slab::new(CLAD, CORE, CLAD, Length::um(t))
        .expect("a valid slab")
        .modes(Polarization::Tm, lam())[0];
    let (_, q, p) = m.wavenumbers();
    let (neff, k) = (m.effective_index(), lam().wavenumber());
    let simpson = |a: f64, b: f64, f: &dyn Fn(f64) -> f64| {
        let n = 200_000;
        let d = (b - a) / n as f64;
        let mut s = f(a) + f(b);
        for j in 1..n {
            s += f(a + d * j as f64) * if j % 2 == 1 { 4.0 } else { 2.0 };
        }
        s * d / 3.0
    };
    let parts = |x: f64, eps: f64| {
        let (hy, dh) = m.field(Length::um(x));
        let (ex, ez) = (neff * hy / eps, dh / (k * eps));
        let norm = ex * ex + ez * ez;
        let square = ex * ex - ez * ez;
        let sz = neff * hy * hy / eps;
        (2.0 * norm * norm + square * square, sz * sz)
    };
    let (core, clad) = (CORE * CORE, CLAD * CLAD);
    let top = simpson(0.0, t, &|x| core * parts(x, core).0);
    let bottom = simpson(0.0, t, &|x| parts(x, core).1)
        + simpson(-40.0 / q, 0.0, &|x| parts(x, clad).1)
        + simpson(t, t + 40.0 / p, &|x| parts(x, clad).1);
    top / (3.0 * bottom)
}

/// The full-vector TM₀ slab's n̄₂ over the core's n₂ at spacing `h`, n₂ in the core only
/// ([`VectorMode::field_weighted_n2`]).
pub(crate) fn slab_tm_kerr(t: f64, h: f64) -> f64 {
    let (cs, m) = slab(t, h, Polarization::Tm, 1, false);
    let si = Permittivity::isotropic(c64::new(CORE * CORE, 0.0));
    m[0].field_weighted_n2(&cs, |p| if *p == si { 1.0 } else { 0.0 })
        .unwrap_or(f64::NAN)
}

/// The largest |O_ab| between the four TE modes of highest index of a 1 µm slab at spacing `h`
/// (the conjugated product), and the largest |O_aa − 1|.
pub(crate) fn slab_orthogonality(h: f64) -> (f64, f64) {
    let (cs, m) = slab(1.0, h, Polarization::Te, 4, false);
    let fields: Vec<Fields> = m.iter().map(|m| m.fields(&cs).expect("its grid")).collect();
    orthogonality(&fields, Product::Conjugated)
        .map_or((f64::NAN, f64::NAN), |o| (o.off_diagonal(), o.diagonal()))
}

/// Afshar and Monro's silicon rod (their Fig. 1c), for the study of n̄₂ at an interface oblique
/// to the grid: n = 3.45 and n₂ = 4.5e-18 m²/W, in silica (n = 1.45, n₂ = 2.6e-20 m²/W), at
/// 800 nm.
#[cfg(test)]
const ROD: [f64; 4] = [3.45, 4.5e-18, 1.45, 2.6e-20];

/// The paper's wavelength for the rod, 800 nm.
#[cfg(test)]
fn rod_lam() -> Wavelength {
    Wavelength::from_um_unchecked(0.8)
}

/// What the rod's fundamental mode gives for the Kerr effect.
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
struct RodKerr {
    /// Its effective index.
    pub neff: f64,
    /// Eq. 39's (and 42's) field-weighted n̄₂, m²/W.
    pub n2_vector: f64,
    /// Foster et al.'s, ∫ n₂ S_z² dA / ∫ S_z² dA (the paper's Eq. 48 as 2π/λ times it over
    /// Eq. 41's area), m²/W.
    pub n2_foster: f64,
    /// Eq. 41's effective area, µm².
    pub area: f64,
    /// Eq. 44's (Agrawal's) effective area from E_t alone, µm².
    pub area_transverse: f64,
}

#[cfg(test)]
impl RodKerr {
    /// γ^V / γ^F: the paper's Eq. 39 over Eq. 48, n̄₂ over Foster's, the two sharing Eq. 41's
    /// area.
    fn vector_over_foster(&self) -> f64 {
        self.n2_vector / self.n2_foster
    }

    /// γ^V / γ^S: Eq. 39 over Eq. 47 (Agrawal's, with the core's n₂ and E_t alone).
    fn vector_over_standard(&self) -> f64 {
        self.n2_vector / ROD[1] * self.area_transverse / self.area
    }
}

/// The rod of diameter `d` µm on a quarter of the window, 1.2 µm a side, behind an electric wall
/// at x = 0 and a magnetic one at y = 0 (the HE₁₁ mode with E along x), square cells of side `h`:
/// its mode's Kerr quantities. `how` 0: cells averaged where the rod's edge crosses them (the
/// area share from 64 × 64 samples), n² n₂ and n₂ there their area means, and the one-sided
/// fields; 1: the same with the central differences of [`VectorMode::fields`]; 2: staircased.
#[cfg(test)]
fn rod_kerr(d: f64, h: f64, how: u8) -> RodKerr {
    let [nc, n2c, ns, n2s] = ROD;
    let (ec, es) = (nc * nc, ns * ns);
    let a = 0.5 * d;
    let side = 1.2;
    let n = (side / h).round() as usize;
    let x: Vec<f64> = (0..=n).map(|i| side * i as f64 / n as f64).collect();
    let mut shares = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            let r = (0.5 * (x[i] + x[i + 1])).hypot(0.5 * (x[j] + x[j + 1]));
            let f = if (r - a).abs() < h {
                let k = 64;
                let inside = (0..k * k)
                    .filter(|s| {
                        let px = x[i] + ((s % k) as f64 + 0.5) / k as f64 * (x[i + 1] - x[i]);
                        let py = x[j] + ((s / k) as f64 + 0.5) / k as f64 * (x[j + 1] - x[j]);
                        px.hypot(py) < a
                    })
                    .count();
                inside as f64 / (k * k) as f64
            } else if r < a {
                1.0
            } else {
                0.0
            };
            shares.push(if how == 2 { f.round() } else { f });
        }
    }
    let cells = shares
        .iter()
        .map(|&f| {
            Permittivity::isotropic(crate::stack::mean(c64::new(ec, 0.0), c64::new(es, 0.0), f))
        })
        .collect();
    let cs = CrossSection::new(x.clone(), x, cells)
        .and_then(|cs| {
            cs.with_boundaries(Boundaries {
                west: Boundary::ElectricWall,
                south: Boundary::MagneticWall,
                ..Boundaries::default()
            })
        })
        .expect("a valid rod");
    let mode = vector::modes(&cs, rod_lam(), 1, None)
        .expect("the rod's mode")
        .remove(0);
    let fields = if how == 1 {
        mode.fields(&cs)
    } else {
        mode.fields_one_sided(&cs)
    }
    .expect("its grid");
    let weight: Vec<f64> = shares
        .iter()
        .map(|f| f * ec * n2c + (1.0 - f) * es * n2s)
        .collect();
    let (mut top, mut bottom) = (0.0, 0.0);
    for ((e, h), (a, f)) in fields
        .e
        .iter()
        .zip(&fields.h)
        .zip(fields.area.iter().zip(&shares))
    {
        let s2 = (e[0] * h[1].conj() - e[1] * h[0].conj()).norm_sqr() * a;
        top += (f * n2c + (1.0 - f) * n2s) * s2;
        bottom += s2;
    }
    RodKerr {
        neff: mode.effective_index().re,
        n2_vector: fields
            .field_weighted_n2(&weight)
            .expect("one weight a cell"),
        n2_foster: top / bottom,
        area: fields.effective_area(),
        area_transverse: fields.effective_area_transverse(),
    }
}

/// A strip symmetric about x = 0 and y = 0, 480 × 200 nm of 3.473 in 1.444, on a uniform grid of
/// spacing `h` over ±1 µm × ±0.7 µm: the whole window, or its part from x = 0 behind the wall
/// `west` and from y = 0 behind the wall `south` ([`Boundary::Zero`] for an axis left whole).
fn symmetric_strip(h: f64, west: Boundary, south: Boundary) -> CrossSection {
    let from = |wall: Boundary, half: f64| if wall == Boundary::Zero { -half } else { 0.0 };
    let (x0, y0) = (from(west, 1.0), from(south, 0.7));
    let cells = |a: f64, b: f64| ((b - a) / h).round() as usize;
    CrossSection::uniform(
        (x0, 1.0, cells(x0, 1.0)),
        (y0, 0.7, cells(y0, 0.7)),
        |x, y| {
            let n: f64 = if x.abs() < 0.24 && y.abs() < 0.1 {
                CORE
            } else {
                CLAD
            };
            Permittivity::isotropic(c64::new(n * n, 0.0))
        },
    )
    .and_then(|cs| {
        cs.with_boundaries(Boundaries {
            west,
            south,
            ..Boundaries::default()
        })
    })
    .expect("a valid strip")
}

/// What a mode's fields give beside its index: its effective area (Eq. 41, with the mirror
/// images of a folded window), its confinement in the silicon, and its index change for a
/// change of 0.01 in the silicon's permittivity by first-order perturbation.
fn field_quantities(cs: &CrossSection, mode: &VectorMode) -> [f64; 3] {
    let si = Permittivity::isotropic(c64::new(CORE * CORE, 0.0));
    let delta: Vec<Permittivity> = cs
        .cells()
        .iter()
        .map(|&e| {
            let d = if e == si { 0.01 } else { 0.0 };
            Permittivity::isotropic(c64::new(d, 0.0))
        })
        .collect();
    [
        mode.fields_one_sided(cs)
            .map_or(f64::NAN, |f| f.effective_area()),
        mode.confinement(cs, &Region::Material(si))
            .unwrap_or(f64::NAN),
        crate::mode::thermo::index_change(mode, cs, &delta).map_or(f64::NAN, |d| d.re),
    ]
}

/// The largest relative difference between the symmetric strip's quantities from its whole
/// window and from each fold of it at spacing `h` (half behind a wall at x = 0, half behind a
/// wall at y = 0, and the quarter behind both), over its TE-like mode (an electric wall at
/// x = 0, a magnetic one at y = 0) and its TM-like one (the opposite), and over the effective
/// area, the confinement and the index change of [`field_quantities`]. A fold is the same
/// discrete problem as the whole window, so the difference is the eigensolver's round-off;
/// E_z beside a wall must take its parity across it for that (odd across an electric wall, even
/// across a magnetic one), or ∂E_z there is one-sided and E's transverse part is off in the
/// cells along the wall.
pub(crate) fn fold_difference(h: f64) -> f64 {
    fold_differences(h)
        .iter()
        .flat_map(|(_, _, _, d)| *d)
        .fold(0.0, f64::max)
}

/// [`fold_difference`] by mode and fold: whether the mode is the TE-like one, the walls at
/// x = 0 and y = 0, and each quantity's relative difference.
fn fold_differences(h: f64) -> Vec<(bool, Boundary, Boundary, [f64; 3])> {
    use Boundary::{ElectricWall, MagneticWall, Zero};
    let full = symmetric_strip(h, Zero, Zero);
    let both = vector::modes(&full, lam(), 2, None).expect("the solver converges");
    let mut out = Vec::new();
    for (te, mode, across_x, across_y) in [
        (true, &both[0], ElectricWall, MagneticWall),
        (false, &both[1], MagneticWall, ElectricWall),
    ] {
        let whole = field_quantities(&full, mode);
        let n = mode.effective_index().re;
        for (west, south) in [(across_x, Zero), (Zero, across_y), (across_x, across_y)] {
            let cs = symmetric_strip(h, west, south);
            let folded = vector::modes(&cs, lam(), 1, Some(n)).expect("the solver converges");
            let part = field_quantities(&cs, &folded[0]);
            let d = [0, 1, 2].map(|k| (part[k] / whole[k] - 1.0).abs());
            out.push((te, west, south, d));
        }
    }
    out
}

/// The observed order of `error` over spacings 10, 5 and 2.5 nm: the mean of the two halvings'.
fn order(error: impl Fn(f64) -> f64) -> f64 {
    let e: Vec<f64> = [0.01, 0.005, 0.0025].into_iter().map(error).collect();
    0.5 * ((e[0] / e[1]).log2() + (e[1] / e[2]).log2())
}

/// to `digits` significant digits, so the report reads the same on every system.
fn significant(v: f64, digits: i32) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let scale = 10f64.powi(digits - 1 - v.abs().log10().floor() as i32);
    (v * scale).round() / scale
}

fn relative(measured: f64, expected: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured,
        expected,
        tolerance: tolerance * expected.abs(),
        error: (measured - expected).abs(),
    }
}

fn orders(measured: f64) -> Outcome {
    let measured = (100.0 * measured).round() / 100.0;
    Outcome {
        measured,
        expected: 2.0,
        tolerance: 0.1,
        error: (measured - 2.0).abs(),
    }
}

/// Every case of a mode's quantities, in report order.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "mode/effective-area-gaussian",
            title: r"Effective area of a Gaussian mode field $e^{-r^2/w^2}$ ($w$ = 1 µm, 50 nm cells to $\pm 6$ µm) by Afshar and Monro's Eq. 41 and Eq. 44, on the whole window, its half behind a wall and its quarter: the largest relative difference from $\pi w^2$ (shown)",
            tier: Tier::Analytic,
            source: r"$A_\text{eff} = (\int e^{-2r^2/w^2} dA)^2 / \int e^{-4r^2/w^2} dA = \pi w^2$; the midpoint sums of a Gaussian are exact far below round-off at 20 cells a width, and the window cuts off $e^{-72}$",
            run: || {
                let e = significant(gaussian_area_error(), 1);
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-10,
                    error: e,
                }
            },
        },
        Case {
            id: "mode/effective-area-slab-te",
            title: r"Effective area of a silicon slab's TE₀ mode (220 nm, 3.473 in 1.444, 1.55 µm) by Eq. 41 from the full-vector fields at 2.5 nm, over the window's height: its effective width, µm",
            tier: Tier::Analytic,
            source: r"the exact slab's (mode::slab) $(\int E_y^2 dx)^2 / \int E_y^4 dx$ in closed form, the core's $\cos^2$ and $\cos^4$ and the tails' exponentials; for TE $S_z \propto \lvert E_y \rvert^2$, so Eqs. 41 and 44 agree",
            run: || relative(slab_te_width(0.22, 0.0025), slab_te_exact(0.22).0, 3e-4),
        },
        Case {
            id: "mode/effective-area-slab-te-order",
            title: r"The same at 10, 5 and 2.5 nm: the order of convergence of the effective width's error (shown)",
            tier: Tier::Analytic,
            source: r"second order: the full-vector scheme's, with the interfaces on the grid's nodes",
            run: || {
                let exact = slab_te_exact(0.22).0;
                orders(order(|h| (slab_te_width(0.22, h) - exact).abs()))
            },
        },
        Case {
            id: "mode/confinement-slab-te",
            title: r"Confinement of the same slab's TE₀ mode in the silicon, $\int_\text{Si} \operatorname{Re} S_z dA / \int \operatorname{Re} S_z dA$, from the full-vector fields at 2.5 nm",
            tier: Tier::Analytic,
            source: r"the core's share of the exact slab's power (mode::multilayer's power fractions, $\int \lvert E_y \rvert^2$ by layer, the tails in closed form)",
            run: || {
                let (got, exact) = slab_confinement(Polarization::Te, 0.22, 0.0025);
                relative(got, exact, 8e-5)
            },
        },
        Case {
            id: "mode/confinement-slab-tm",
            title: r"The same slab's TM₀ mode in the silicon, at 2.5 nm",
            tier: Tier::Analytic,
            source: r"the core's share of the exact slab's power (mode::multilayer's power fractions, $\int \lvert H_y \rvert^2 / n^2$ by layer)",
            run: || {
                let (got, exact) = slab_confinement(Polarization::Tm, 0.22, 0.0025);
                relative(got, exact, 4e-4)
            },
        },
        Case {
            id: "mode/confinement-slab-order",
            title: r"The TE₀ slab's confinement at 10, 5 and 2.5 nm: the order of convergence of its error (shown)",
            tier: Tier::Analytic,
            source: r"second order: the full-vector scheme's, with the interfaces on the grid's nodes",
            run: || {
                orders(order(|h| {
                    let (got, exact) = slab_confinement(Polarization::Te, 0.22, h);
                    (got - exact).abs()
                }))
            },
        },
        Case {
            id: "mode/kerr-index-slab-tm",
            title: r"Afshar and Monro's field-weighted $\bar n_2$ (their Eqs. 39 and 42) of the same slab's TM₀ mode, $n_2$ in the silicon only, from the full-vector fields at 2.5 nm: $\bar n_2 / n_2$",
            tier: Tier::Analytic,
            source: r"$\bar n_2 = \int n^2 n_2 [2\lvert\mathbf E\rvert^4 + \lvert\mathbf E\cdot\mathbf E\rvert^2] dA / 3\int S_z^2 dA$ from the exact slab's (mode::slab) $H_y$: $E_x = n_\text{eff} H_y/\varepsilon$, $E_z = iH_y'/k\varepsilon$, $S_z = n_\text{eff} H_y^2/\varepsilon$, integrated by Simpson's rule on $2 \times 10^5$ intervals a region; $E_z$, a quarter period out of phase, makes $\lvert\mathbf E\cdot\mathbf E\rvert^2$ less than $\lvert\mathbf E\rvert^4$",
            run: || relative(slab_tm_kerr(0.22, 0.0025), slab_tm_kerr_exact(0.22), 5e-4),
        },
        Case {
            id: "mode/orthogonality-slab",
            title: r"Orthogonality of a 1 µm silicon slab's four TE modes of highest index (3.473 in 1.444, 1.55 µm) at 2.5 nm: the largest $\lvert O_{ab} \rvert$ between two of them, $O_{ab} = \langle a, b\rangle / (\operatorname{Re}\langle a, a\rangle \operatorname{Re}\langle b, b\rangle)^{1/2}$, $\langle a, b\rangle = \int (\mathbf E_a \times \mathbf H_b^*)\cdot\hat z \thinspace dA$ (shown)",
            tier: Tier::Analytic,
            source: r"0: a lossless guide's modes carry power independently; on the grid, the product of the fields at the cells' centres leaves the scheme's second-order error",
            run: || {
                let (off, _) = slab_orthogonality(0.0025);
                let off = significant(off, 2);
                Outcome {
                    measured: off,
                    expected: 0.0,
                    tolerance: 8e-5,
                    error: off,
                }
            },
        },
        Case {
            id: "mode/orthogonality-slab-order",
            title: r"The same at 10, 5 and 2.5 nm: the order of convergence of the largest $\lvert O_{ab} \rvert$ (shown)",
            tier: Tier::Analytic,
            source: r"second order: the cells' averages of the nodal field",
            run: || orders(order(|h| slab_orthogonality(h).0)),
        },
        Case {
            id: "mode/fields-folded-strip",
            title: r"A strip symmetric about both axes (480 × 200 nm, 3.473 in 1.444, 1.55 µm, 10 nm cells over ±1 × ±0.7 µm), its TE-like and TM-like modes on the whole window and on its halves and quarter behind mirror walls (electric and magnetic, at x = 0 and at y = 0): the largest relative difference in the effective area, the confinement in the silicon and the first-order index change for $\Delta\varepsilon$ = 0.01 in the silicon (shown)",
            tier: Tier::Analytic,
            source: r"0: a fold is the same discrete problem as the whole window, its walls the mirror images of the field; $E_z$ beside a wall takes its parity across it (odd across an electric wall, even across a magnetic one), so $\partial E_z$ there is the whole window's central difference",
            run: || {
                let e = significant(fold_difference(0.01), 1);
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-10,
                    error: e,
                }
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_closed_form_core_share_is_the_multilayers() {
        // the slab's closed-form integrals against mode::multilayer's Simpson sums
        let (_, closed) = slab_te_exact(0.22);
        let (_, multilayer) = slab_confinement(Polarization::Te, 0.22, 0.02);
        assert!(
            (closed - multilayer).abs() < 1e-12,
            "{closed} vs {multilayer}"
        );
    }

    #[test]
    #[ignore = "a grid study, run by hand (cargo test --release -- --ignored --nocapture)"]
    fn afshar_and_monros_rod_study() {
        // the silicon rod of their Fig. 1c at 800 nm: γ^V/γ^F and γ^V/γ^S by grid and diameter
        let start = std::time::Instant::now();
        let exact = slab_tm_kerr_exact(0.22);
        eprintln!("TM slab n2 ratio exact {exact:.9}");
        for h in [0.02, 0.01, 0.005, 0.0025] {
            let got = slab_tm_kerr(0.22, h);
            eprintln!(
                "TM slab h {h}: {got:.9} rel {:.3e} ({:.0} s)",
                (got / exact - 1.0).abs(),
                start.elapsed().as_secs_f64()
            );
        }
        let runs: Vec<(f64, f64, u8)> = [0u8, 1, 2]
            .into_iter()
            .flat_map(|how| [0.02, 0.01, 0.005].into_iter().map(move |h| (0.15, h, how)))
            .collect();
        for (k, (d, h, how)) in runs.iter().enumerate() {
            let r = rod_kerr(*d, *h, *how);
            eprint!("how {how} ");
            println!(
                "step {}/{} ({:.0} %, {:.0} s): D {d} h {h}: neff {:.6} n2V {:.6e} n2F {:.6e} \
                 A41 {:.6} A44 {:.6} V/F {:.5} V/S {:.5} gV {:.2} gF {:.2}",
                k + 1,
                runs.len(),
                100.0 * (k + 1) as f64 / runs.len() as f64,
                start.elapsed().as_secs_f64(),
                r.neff,
                r.n2_vector,
                r.n2_foster,
                r.area,
                r.area_transverse,
                r.vector_over_foster(),
                r.vector_over_standard(),
                crate::mode::quantities::nonlinear_coefficient(rod_lam(), r.n2_vector, r.area),
                crate::mode::quantities::nonlinear_coefficient(rod_lam(), r.n2_foster, r.area),
            );
        }
    }

    #[test]
    #[ignore = "a grid study, run by hand (cargo test --release -- --ignored --nocapture)"]
    fn folds_by_grid() {
        // the effective area, confinement and index change: whole window against each fold
        for h in [0.02, 0.01, 0.005] {
            for (te, west, south, d) in fold_differences(h) {
                println!(
                    "folds h {h} te {te} west {west:?} south {south:?}: {:.3e} {:.3e} {:.3e}",
                    d[0], d[1], d[2]
                );
            }
        }
    }

    #[test]
    #[ignore = "a grid study, run by hand (cargo test --release -- --ignored --nocapture)"]
    fn numbers_by_grid() {
        println!("gaussian {:e}", gaussian_area_error());
        let (w, g) = slab_te_exact(0.22);
        println!("exact width {w} core share {g}");
        for h in [0.02, 0.01, 0.005, 0.0025] {
            let wv = slab_te_width(0.22, h);
            let (te, te_x) = slab_confinement(Polarization::Te, 0.22, h);
            let (tm, tm_x) = slab_confinement(Polarization::Tm, 0.22, h);
            println!(
                "h {h}: width {wv} rel {:e}; conf te {te} vs {te_x} rel {:e}; tm {tm} vs {tm_x} rel {:e}",
                (wv / w - 1.0).abs(),
                (te / te_x - 1.0).abs(),
                (tm / tm_x - 1.0).abs()
            );
        }
        for h in [0.01, 0.005, 0.0025] {
            println!("orth h {h}: {:?}", slab_orthogonality(h));
        }
        for h in [0.02, 0.01, 0.005] {
            let cs = vector::strip(h);
            let m = vector::modes(&cs, lam(), 2, None).unwrap();
            for m in &m {
                let (f, g) = (m.fields_one_sided(&cs).unwrap(), m.fields(&cs).unwrap());
                let si = Permittivity::isotropic(c64::new(CORE * CORE, 0.0));
                println!(
                    "strip h {h} n {} te {:.3}: a41 {} a44 {} (central {} {}) gamma(n2=5e-18) {} conf {}",
                    m.effective_index().re,
                    m.te_fraction(),
                    f.effective_area(),
                    f.effective_area_transverse(),
                    g.effective_area(),
                    g.effective_area_transverse(),
                    f.nonlinear_coefficient(lam(), 5e-18),
                    m.confinement(&cs, &Region::Material(si)).unwrap()
                );
            }
        }
    }
}
