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
use crate::portable;
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
            let g = c64::new(portable::exp(-(x * x + y * y) / (w * w)), 0.0);
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
    let (r2, phi) = (a * a + b * b, portable::atan2(b, a));
    let s = |n: f64| portable::sin(n * (h * t - phi)) + portable::sin(n * phi);
    let core2 = r2 * (t / 2.0 + s(2.0) / (4.0 * h));
    let core4 = r2 * r2 * (3.0 * t / 8.0 + s(2.0) / (4.0 * h) + s(4.0) / (32.0 * h));
    let top = m.field(Length::um(t)).0;
    let all2 = core2 + a * a / (2.0 * q) + top * top / (2.0 * p);
    let all4 = core4 + portable::powi(a, 4) / (4.0 * q) + portable::powi(top, 4) / (4.0 * p);
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
/// fields; 1: the same with the central differences of [`VectorMode::fields`]; 2: staircased
/// (one-sided fields); 3, 4, 5 and 6: E_t from D ([`rod_kerr_from_d`] with `split` 1, 0, 2 and
/// 3). One mode serves every `how` but 2's; with `exact_h` it is the exact mode's H at the nodes
/// ([`RodExact`]), no eigensolve.
#[cfg(test)]
fn rod_kerr(d: f64, h: f64, hows: &[u8], exact_h: bool) -> Vec<(u8, RodKerr)> {
    let mut out = Vec::new();
    for stair in [false, true] {
        let hows: Vec<u8> = hows
            .iter()
            .copied()
            .filter(|&w| (w == 2) == stair)
            .collect();
        if !hows.is_empty() {
            out.extend(rod_kerr_grid(d, h, stair, &hows, exact_h));
        }
    }
    out
}

/// [`rod_kerr`] on one grid, staircased or not.
#[cfg(test)]
fn rod_kerr_grid(d: f64, h: f64, stair: bool, hows: &[u8], exact_h: bool) -> Vec<(u8, RodKerr)> {
    let [nc, _, ns, _] = ROD;
    let (ec, es) = (nc * nc, ns * ns);
    let a = 0.5 * d;
    let side = 1.2;
    let n = (side / h).round() as usize;
    let x: Vec<f64> = (0..=n).map(|i| side * i as f64 / n as f64).collect();
    let mut shares = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            let r = portable::hypot(0.5 * (x[i] + x[i + 1]), 0.5 * (x[j] + x[j + 1]));
            let f = if (r - a).abs() < h {
                let k = 64;
                let inside = (0..k * k)
                    .filter(|s| {
                        let px = x[i] + ((s % k) as f64 + 0.5) / k as f64 * (x[i + 1] - x[i]);
                        let py = x[j] + ((s / k) as f64 + 0.5) / k as f64 * (x[j + 1] - x[j]);
                        portable::hypot(px, py) < a
                    })
                    .count();
                inside as f64 / (k * k) as f64
            } else if r < a {
                1.0
            } else {
                0.0
            };
            shares.push(if stair { f.round() } else { f });
        }
    }
    let cells = shares
        .iter()
        .map(|&f| {
            Permittivity::isotropic(crate::stack::mean(c64::new(ec, 0.0), c64::new(es, 0.0), f))
        })
        .collect();
    let nodes = x.clone();
    let cs = CrossSection::new(x.clone(), x, cells)
        .and_then(|cs| {
            cs.with_boundaries(Boundaries {
                west: Boundary::ElectricWall,
                south: Boundary::MagneticWall,
                ..Boundaries::default()
            })
        })
        .expect("a valid rod");
    let mode = if exact_h {
        // the exact mode's H at the nodes, with its β: no eigensolve
        let rod = RodExact::new(d);
        let (mut hx, mut hy) = (Vec::new(), Vec::new());
        for &px in &nodes {
            for &py in &nodes {
                let [a, b, _] = rod.h_at(px, py);
                hx.push(a);
                hy.push(b);
            }
        }
        VectorMode::from_nodes(rod.beta, rod.k, nodes.clone(), nodes, hx, hy)
    } else {
        vector::modes(&cs, rod_lam(), 1, None)
            .expect("the rod's mode")
            .remove(0)
    };
    hows.iter()
        .map(|&how| (how, rod_kerr_eval(&cs, &mode, &shares, how)))
        .collect()
}

/// [`rod_kerr`]'s quantities of one mode by one `how`.
#[cfg(test)]
fn rod_kerr_eval(cs: &CrossSection, mode: &VectorMode, shares: &[f64], how: u8) -> RodKerr {
    let [nc, n2c, ns, n2s] = ROD;
    let (ec, es) = (nc * nc, ns * ns);
    let fields = if how == 1 || how >= 3 {
        mode.fields(cs)
    } else {
        mode.fields_one_sided(cs)
    }
    .expect("its grid");
    if how >= 3 {
        return rod_kerr_from_d(
            mode,
            &fields,
            shares,
            match how {
                3 => 1,
                4 => 0,
                5 => 2,
                _ => 3,
            },
        );
    }
    let weight: Vec<f64> = shares
        .iter()
        .map(|f| f * ec * n2c + (1.0 - f) * es * n2s)
        .collect();
    let (mut top, mut bottom) = (0.0, 0.0);
    for ((e, h), (a, f)) in fields
        .e
        .iter()
        .zip(&fields.h)
        .zip(fields.area.iter().zip(shares))
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

/// The rod's Kerr quantities with E's transverse part from D: D_t = (i/k)(∇×H)_t from H, which
/// is continuous everywhere (D_x = (β/k)H_y + (i/k)∂_y H_z, D_y = −(β/k)H_x − (i/k)∂_x H_z,
/// ∂H_z by central differences of the cells' H_z, H_z even across the electric wall at x = 0 and
/// odd across the magnetic one at y = 0), and E_z as [`VectorMode::fields`] has it. A cell of
/// one material takes E_t = D_t/ε. A cell the rod's edge crosses (share `f` of the core):
///
/// - `split` 0: E_t = D_t/⟨ε⟩, ⟨ε⟩ the cell's mean, and the area mean of n² n₂;
/// - 1: taken as its two sides, each with its share of the area: along the edge (t = ẑ × n, n
///   the rod's outward normal at the cell's centre) E_t·t = D·t/⟨ε⟩ on both (E continuous
///   there), across it E·n = D·n/ε on each side (D continuous there), and each side's |E|⁴,
///   |E·E|² and S_z² weighed by its own n² n₂;
/// - 2: as 1, with E·t and D·n the mean over the cells of one material within two cells of it
///   (each E = D_t/ε), not the cell's own D_t;
/// - 3: as 2, with H_z and E_z in such a cell (and so ∂H_z beside it) the mean over the same
///   cells: both are continuous across the edge, but a cell's ∇·H and ∇×H, from which they come,
///   mix the two sides' derivatives there, and are O(1) wrong at any grid
///   (`tests::rod_exact_h_diagnostic`).
#[cfg(test)]
fn rod_kerr_from_d(mode: &VectorMode, fields: &Fields, shares: &[f64], split: u8) -> RodKerr {
    let [nc, n2c, ns, n2s] = ROD;
    let (ec, es) = (nc * nc, ns * ns);
    let n = fields.xc.len();
    let h = fields.xc[1] - fields.xc[0];
    let k = rod_lam().wavenumber();
    let beta = mode.effective_index().re * k;
    let i = c64::new(0.0, 1.0);
    // H_z and E_z, continuous across the edge: with split 3, in a cell the edge crosses (where
    // ∇·H and ∇×H over the cell mix the two sides' derivatives) the mean of the cells of one
    // material within two cells of it
    let interface = |c: usize| shares[c] > 0.0 && shares[c] < 1.0;
    let smooth = |q: usize| -> Vec<c64> {
        (0..n * n)
            .map(|c| {
                let own = if q == 2 {
                    fields.h[c][2]
                } else {
                    fields.e[c][2]
                };
                if split != 3 || !interface(c) {
                    return own;
                }
                let (a, b) = (c / n, c % n);
                let (mut sum, mut count) = (c64::new(0.0, 0.0), 0.0);
                for p in a.saturating_sub(2)..(a + 3).min(n) {
                    for r in b.saturating_sub(2)..(b + 3).min(n) {
                        if !interface(p * n + r) {
                            sum += if q == 2 {
                                fields.h[p * n + r][2]
                            } else {
                                fields.e[p * n + r][2]
                            };
                            count += 1.0;
                        }
                    }
                }
                if count > 0.0 { sum / count } else { own }
            })
            .collect()
    };
    let (hzc, ezc) = (smooth(2), smooth(3));
    let hz = |a: isize, b: isize| -> c64 {
        // the images across the walls at x = 0 (electric: H_z even) and y = 0 (magnetic: odd)
        let (a, sa) = if a < 0 { (-a - 1, 1.0) } else { (a, 1.0) };
        let (b, sb) = if b < 0 { (-b - 1, -1.0) } else { (b, 1.0) };
        hzc[a as usize * n + b as usize] * (sa * sb)
    };
    let d = |a: usize, b: usize, along_x: bool| -> c64 {
        let (a, b) = (a as isize, b as isize);
        let last = n as isize - 1;
        if along_x {
            let (lo, hi) = ((a - 1).max(-1), (a + 1).min(last));
            (hz(hi, b) - hz(lo, b)) / (h * (hi - lo) as f64)
        } else {
            let (lo, hi) = ((b - 1).max(-1), (b + 1).min(last));
            (hz(a, hi) - hz(a, lo)) / (h * (hi - lo) as f64)
        }
    };
    let (mut top, mut s2, mut fos, mut sz, mut t2, mut t4) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    // what a side of a cell adds: its E, H, n² n₂, n₂ and area
    let mut add = |e: [c64; 3], hh: [c64; 3], w: f64, n2: f64, area: f64| {
        let norm = e.iter().map(|c| c.norm_sqr()).sum::<f64>();
        let square: c64 = e.iter().map(|c| c * c).sum();
        let s = (e[0] * hh[1].conj() - e[1] * hh[0].conj()).re;
        let t = e[0].norm_sqr() + e[1].norm_sqr();
        top += w * (2.0 * norm * norm + square.norm_sqr()) * area;
        s2 += s * s * area;
        fos += n2 * s * s * area;
        sz += s * area;
        t2 += t * area;
        t4 += t * t * area;
    };
    let dt: Vec<[c64; 2]> = (0..n * n)
        .map(|c| {
            let (a, b) = (c / n, c % n);
            let hh = fields.h[c];
            [
                (beta * hh[1] + i * d(a, b, false)) / k,
                (-beta * hh[0] - i * d(a, b, true)) / k,
            ]
        })
        .collect();
    for a in 0..n {
        for b in 0..n {
            let c = a * n + b;
            let hh = fields.h[c];
            let [dx, dy] = dt[c];
            let ez = ezc[c];
            let f = shares[c];
            let area = fields.area[c];
            let mean = f * ec + (1.0 - f) * es;
            if split >= 2 && f > 0.0 && f < 1.0 {
                // E along the edge and D across it, both continuous, the mean over the cells
                // of one material within two cells of this one
                let r = portable::hypot(fields.xc[a], fields.yc[b]);
                let (nx, ny) = (fields.xc[a] / r, fields.yc[b] / r);
                let (mut along, mut across, mut count) =
                    (c64::new(0.0, 0.0), c64::new(0.0, 0.0), 0.0);
                for p in a.saturating_sub(2)..(a + 3).min(n) {
                    for q in b.saturating_sub(2)..(b + 3).min(n) {
                        let g = shares[p * n + q];
                        if g > 0.0 && g < 1.0 {
                            continue;
                        }
                        let eps = if g >= 1.0 { ec } else { es };
                        let [px, py] = dt[p * n + q];
                        along += (-ny * px + nx * py) / eps;
                        across += nx * px + ny * py;
                        count += 1.0;
                    }
                }
                if count > 0.0 {
                    let (along, across) = (along / count, across / count);
                    for (eps, n2, part) in [(ec, n2c, f), (es, n2s, 1.0 - f)] {
                        let en = across / eps;
                        let e = [nx * en - ny * along, ny * en + nx * along, ez];
                        add(e, hh, eps * n2, n2, area * part);
                    }
                    continue;
                }
            }
            if f <= 0.0 || f >= 1.0 || split == 0 {
                let (w, n2) = (
                    f * ec * n2c + (1.0 - f) * es * n2s,
                    f * n2c + (1.0 - f) * n2s,
                );
                add([dx / mean, dy / mean, ez], hh, w, n2, area);
                continue;
            }
            let r = portable::hypot(fields.xc[a], fields.yc[b]);
            let (nx, ny) = (fields.xc[a] / r, fields.yc[b] / r);
            let along = (-ny * dx + nx * dy) / mean;
            let across = nx * dx + ny * dy;
            for (eps, n2, part) in [(ec, n2c, f), (es, n2s, 1.0 - f)] {
                let en = across / eps;
                let e = [nx * en - ny * along, ny * en + nx * along, ez];
                add(e, hh, eps * n2, n2, area * part);
            }
        }
    }
    let copies = 4.0;
    RodKerr {
        neff: mode.effective_index().re,
        n2_vector: top / (3.0 * s2),
        n2_foster: fos / s2,
        area: copies * sz * sz / s2,
        area_transverse: copies * t2 * t2 / t4,
    }
}

/// The rod's exact HE₁₁ Kerr quantities at diameter `d` µm, from the step-index fibre's fields:
/// E_z = J₁(ur) cos φ and H_z = B J₁(ur) sin φ in the core, the same with K₁(wr) J₁(ua)/K₁(wa)
/// outside (u² = k²n₁² − β², w² = β² − k²n₂²), β from [`crate::stack::checks::he11`], B from E_φ's
/// continuity at r = a, and the transverse fields from E_z and H_z,
/// E_t = (i/κ²)(β∇E_z − k ẑ × ∇H_z), H_t = (i/κ²)(β∇H_z + kε ẑ × ∇E_z), κ² = k²ε − β² (in Z₀'s
/// units). The integrals by Simpson's rule in r (2000 intervals in the core, 20000 outside to
/// 40/w beyond the edge) and the trapezoidal rule in φ (exact for the trigonometric polynomial
/// the integrands are). Also returns |H_φ|'s relative jump at the edge, 0 for the right β.
#[cfg(test)]
fn rod_kerr_exact(d: f64) -> (RodKerr, f64) {
    use std::f64::consts::PI;
    let [_, n2c, _, n2s] = ROD;
    let rod = RodExact::new(d);
    let (a, w) = (rod.a, rod.w);
    let nphi = 64;
    // per radius: ∫ dφ of [n² n₂ (2|E|⁴ + |E·E|²), S_z², n₂ S_z², S_z, |E_t|², |E_t|⁴]
    let ring = |r: f64| -> [f64; 6] {
        if r < 1e-9 {
            // the centre: each integrand carries a factor r
            return [0.0; 6];
        }
        let n2 = if r < a { n2c } else { n2s };
        let mut out = [0.0; 6];
        for p in 0..nphi {
            let phi = 2.0 * PI * p as f64 / nphi as f64;
            let (eps, [er, ep, ez], [hr, hp, _]) = rod.polar(r, phi);
            let sz = (er * hp.conj() - ep * hr.conj()).re;
            let norm = er.norm_sqr() + ep.norm_sqr() + ez.norm_sqr();
            let square = er * er + ep * ep + ez * ez;
            let t = er.norm_sqr() + ep.norm_sqr();
            let dphi = 2.0 * PI / nphi as f64;
            out[0] += eps * n2 * (2.0 * norm * norm + square.norm_sqr()) * dphi;
            out[1] += sz * sz * dphi;
            out[2] += n2 * sz * sz * dphi;
            out[3] += sz * dphi;
            out[4] += t * dphi;
            out[5] += t * t * dphi;
        }
        out.map(|v| v * r)
    };
    let simpson = |lo: f64, hi: f64, n: usize| -> [f64; 6] {
        let h = (hi - lo) / n as f64;
        let mut s = [0.0; 6];
        for j in 0..=n {
            // the edge's side taken from the region integrated
            let r = (lo + h * j as f64).clamp(lo, hi);
            let r = if lo > 0.0 && j == 0 {
                lo * (1.0 + 1e-14)
            } else if j == n {
                hi * (1.0 - 1e-14)
            } else {
                r
            };
            let wgt = if j == 0 || j == n {
                1.0
            } else if j % 2 == 1 {
                4.0
            } else {
                2.0
            };
            for (s, v) in s.iter_mut().zip(ring(r)) {
                *s += wgt * v;
            }
        }
        s.map(|v| v * h / 3.0)
    };
    let (inner, outer) = (simpson(0.0, a, 2000), simpson(a, a + 40.0 / w, 20000));
    let t: Vec<f64> = (0..6).map(|q| inner[q] + outer[q]).collect();
    (
        RodKerr {
            neff: rod.beta / rod.k,
            n2_vector: t[0] / (3.0 * t[1]),
            n2_foster: t[2] / t[1],
            area: t[3] * t[3] / t[1],
            area_transverse: t[4] * t[4] / t[5],
        },
        rod.jump(),
    )
}

/// The step-index fibre's HE₁₁ mode for the rod of [`rod_kerr_exact`], its fields at any point.
#[cfg(test)]
struct RodExact {
    a: f64,
    k: f64,
    beta: f64,
    u: f64,
    w: f64,
    /// J₁(ua)/K₁(wa): the outside's radial function's scale.
    c: f64,
    /// H_z's amplitude over E_z's.
    b: f64,
}

#[cfg(test)]
impl RodExact {
    fn new(d: f64) -> RodExact {
        let [nc, _, ns, _] = ROD;
        let a = 0.5 * d;
        let lam = rod_lam();
        let k = lam.wavenumber();
        let beta = crate::stack::checks::he11(nc, ns, a, lam) * k;
        let (u, w) = (
            (k * k * nc * nc - beta * beta).sqrt(),
            (beta * beta - k * k * ns * ns).sqrt(),
        );
        let c = Self::jn(1.0, u * a) / Self::kn(1.0, w * a);
        let mut rod = RodExact {
            a,
            k,
            beta,
            u,
            w,
            c,
            b: 0.0,
        };
        // E_φ continuous at r = a
        let (f, d1, d2) = rod.edge();
        rod.b =
            -beta * f * (1.0 / (u * u) + 1.0 / (w * w)) / (a * k * (d1 / (u * u) + d2 / (w * w)));
        rod
    }

    /// J_n(x) by its integral (the midpoint rule, spectrally accurate), its series near 0.
    fn jn(n: f64, x: f64) -> f64 {
        if x.abs() < 1e-3 {
            return if n == 0.0 {
                1.0 - x * x / 4.0
            } else {
                x / 2.0 - x * x * x / 16.0
            };
        }
        let m = 2000;
        (0..m)
            .map(|i| {
                let t = (i as f64 + 0.5) / m as f64 * std::f64::consts::PI;
                portable::cos(n * t - x * portable::sin(t))
            })
            .sum::<f64>()
            / m as f64
    }

    /// K_n(x) by its integral.
    fn kn(n: f64, x: f64) -> f64 {
        let (m, top) = (4000, 8.0);
        (0..m)
            .map(|i| {
                let t = (i as f64 + 0.5) / m as f64 * top;
                portable::exp(-x * portable::cosh(t)) * portable::cosh(n * t)
            })
            .sum::<f64>()
            * top
            / m as f64
    }

    /// The radial function at the edge, and its derivative inside and outside.
    fn edge(&self) -> (f64, f64, f64) {
        let (u, w, a) = (self.u, self.w, self.a);
        let f = Self::jn(1.0, u * a);
        let d1 = u * (Self::jn(0.0, u * a) - f / (u * a));
        let d2 = self.c * w * (-Self::kn(0.0, w * a) - Self::kn(1.0, w * a) / (w * a));
        (f, d1, d2)
    }

    /// H_φ's relative jump at the edge: 0 for the right β.
    fn jump(&self) -> f64 {
        let [nc, _, ns, _] = ROD;
        let (f, d1, d2) = self.edge();
        let (u, w) = (self.u, self.w);
        let inside = (self.beta * self.b * f / self.a + self.k * nc * nc * d1) / (u * u);
        let outside = (self.beta * self.b * f / self.a + self.k * ns * ns * d2) / (-w * w);
        (inside - outside).abs() / inside.abs()
    }

    /// ε and the fields' polar components (r, φ, z) at (r, φ), r > 0: E_z = F(r) cos φ,
    /// H_z = B F(r) sin φ.
    fn polar(&self, r: f64, phi: f64) -> (f64, [c64; 3], [c64; 3]) {
        let [nc, _, ns, _] = ROD;
        let (f, df, kappa2, eps) = if r < self.a {
            let x = self.u * r;
            let (j0, j1) = (Self::jn(0.0, x), Self::jn(1.0, x));
            // J₁′ = J₀ − J₁/x, ½ at 0
            let dj = if x > 1e-3 {
                j0 - j1 / x
            } else {
                0.5 - 3.0 * x * x / 16.0
            };
            (j1, self.u * dj, self.u * self.u, nc * nc)
        } else {
            let x = self.w * r;
            let (k0, k1) = (Self::kn(0.0, x), Self::kn(1.0, x));
            (
                self.c * k1,
                self.c * self.w * (-k0 - k1 / x),
                -self.w * self.w,
                ns * ns,
            )
        };
        // F/r, its limit u/2 at the centre
        let fr = if r < self.a && self.u * r < 1e-3 {
            self.u * (0.5 - portable::powi(self.u * r, 2) / 16.0)
        } else {
            f / r
        };
        let (beta, k, b) = (self.beta, self.k, self.b);
        let i = c64::new(0.0, 1.0);
        let (cs, sn) = (portable::cos(phi), portable::sin(phi));
        let e = [
            i / kappa2 * (beta * df + k * b * fr) * cs,
            i / kappa2 * (-beta * fr - k * b * df) * sn,
            c64::new(f * cs, 0.0),
        ];
        let h = [
            i / kappa2 * (beta * b * df + k * eps * fr) * sn,
            i / kappa2 * (beta * b * fr + k * eps * df) * cs,
            c64::new(b * f * sn, 0.0),
        ];
        (eps, e, h)
    }

    /// H's Cartesian components at (x, y).
    fn h_at(&self, x: f64, y: f64) -> [c64; 3] {
        let r = portable::hypot(x, y);
        let phi = portable::atan2(y, x);
        let (_, _, [hr, hp, hz]) = self.polar(r.max(1e-12), phi);
        let (cs, sn) = (portable::cos(phi), portable::sin(phi));
        [hr * cs - hp * sn, hr * sn + hp * cs, hz]
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

/// The symmetric strip's two guided modes on 20 nm cells through the façade
/// ([`crate::facade::vector_modes`], as Python has them) against the library's own calls on the
/// same cross-section: the largest relative gap of the effective areas (Eq. 41 and 44), of the
/// confinement in the silicon (the façade's power shares summed over its cells against
/// [`VectorMode::confinement`]), and the largest gap of the orthogonality matrix; and the
/// façade's fundamental's area, µm².
pub(crate) fn facade_quantities() -> (f64, f64) {
    let cs = symmetric_strip(0.02, Boundary::Zero, Boundary::Zero);
    let library = vector::guided_modes(&cs, lam(), 2, None).expect("the solver converges");
    let (x, y) = (cs.x().to_vec(), cs.y().to_vec());
    let (nx, ny) = (x.len() - 1, y.len() - 1);
    let cells = cs.cells();
    // the library's (i, j) at i·ny + j; the façade's (ny, nx) in C order
    let eps: Vec<c64> = (0..ny)
        .flat_map(|j| (0..nx).map(move |i| (i, j)))
        .map(|(i, j)| cells[i * ny + j].xx)
        .collect();
    let si = Permittivity::isotropic(c64::new(CORE * CORE, 0.0));
    let facade = crate::facade::vector_modes(
        &crate::facade::CrossSection::new(x, y, eps.clone()),
        1.55,
        2,
        None,
        &crate::run::Stop::new(None),
    )
    .expect("the façade solves")
    .expect("not stopped");
    let sided: Vec<Fields> = library
        .iter()
        .map(|m| m.fields_one_sided(&cs).expect("one-sided fields"))
        .collect();
    let products = orthogonality(&sided, Product::Conjugated).expect("one grid");
    let rel = |a: f64, b: f64| (a / b - 1.0).abs();
    let mut gap = 0.0f64;
    for (a, (f, m)) in facade.iter().zip(&library).enumerate() {
        let core: f64 = f
            .power_shares
            .iter()
            .zip(&eps)
            .filter(|(_, e)| **e == si.xx)
            .map(|(s, _)| s)
            .sum();
        gap = gap
            .max(rel(f.effective_area_um2, sided[a].effective_area()))
            .max(rel(
                f.effective_area_transverse_um2,
                sided[a].effective_area_transverse(),
            ))
            .max(rel(
                core,
                m.confinement(&cs, &Region::Material(si))
                    .expect("a confinement"),
            ));
        for (b, &overlap) in f.orthogonality.iter().enumerate().take(library.len()) {
            gap = gap.max(portable::complex::abs(overlap - products.get(a, b)));
        }
    }
    (gap, facade[0].effective_area_um2)
}

#[test]
fn the_facades_quantities_are_the_librarys() {
    let (gap, area) = facade_quantities();
    assert!(gap < 1e-12, "{gap:e}");
    assert!(area > 0.05 && area < 0.5, "{area}");
}

/// The observed order of `error` over spacings 10, 5 and 2.5 nm: the mean of the two halvings'.
fn order(error: impl Fn(f64) -> f64) -> f64 {
    let e: Vec<f64> = [0.01, 0.005, 0.0025].into_iter().map(error).collect();
    0.5 * (portable::log2(e[0] / e[1]) + portable::log2(e[1] / e[2]))
}

/// to `digits` significant digits, so the report reads the same on every system.
fn significant(v: f64, digits: i32) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let scale = portable::powi(10f64, digits - 1 - portable::log10(v.abs()).floor() as i32);
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
            id: "mode/quantities-facade-strip",
            title: r"A 480 × 200 nm silicon strip (3.473) in oxide (1.444) on 20 nm cells, its two guided modes at 1.55 µm through the façade's `vector_modes` (Python's `VectorMode`) against the library's own calls on the same cross-section: the effective areas (Afshar and Monro's Eq. 41 and 44), the confinement in the silicon from the façade's power shares, and the orthogonality matrix, the largest gap (the fundamental's area in µm² shown)",
            tier: Tier::Analytic,
            source: r"`Fields::effective_area`, `effective_area_transverse`, `VectorMode::confinement` and `quantities::orthogonality` of the same modes (themselves checked by `mode/effective-area-slab-te`, `mode/confinement-slab-te` and `mode/orthogonality-slab`): one set of sums reached two ways",
            run: || {
                let (gap, area) = facade_quantities();
                Outcome {
                    measured: area,
                    expected: area,
                    tolerance: 1e-12 * area,
                    error: gap * area,
                }
            },
        },
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
            title: r"Confinement of the same slab's TE₀ mode in the silicon, $\int_\text{Si} \mathop{\mathrm{Re}} S_z dA / \int \mathop{\mathrm{Re}} S_z dA$, from the full-vector fields at 2.5 nm",
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
            title: r"Orthogonality of a 1 µm silicon slab's four TE modes of highest index (3.473 in 1.444, 1.55 µm) at 2.5 nm: the largest $\lvert O_{ab} \rvert$ between two of them, $O_{ab} = \langle a, b\rangle / (\mathop{\mathrm{Re}}\langle a, a\rangle \mathop{\mathrm{Re}}\langle b, b\rangle)^{1/2}$, $\langle a, b\rangle = \int (\mathbf E_a \times \mathbf H_b^*)\cdot\hat z \thinspace dA$ (shown)",
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
        // the silicon rod of their Fig. 1c at 800 nm, D = 0.15 µm: γ^V/γ^F and γ^V/γ^S by grid and\n        // by how the fields are taken (rod_kerr), against the exact fields (rod_kerr_exact)
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
        // ROD_GRIDS="0.02,0.01" and ROD_HOWS="0,3,4" choose the runs
        let list = |var: &str, default: &str| -> Vec<f64> {
            std::env::var(var)
                .unwrap_or_else(|_| default.to_owned())
                .split(',')
                .map(|s| s.trim().parse().expect("a number"))
                .collect()
        };
        let grids = list("ROD_GRIDS", "0.02,0.01,0.005");
        let hows: Vec<u8> = list("ROD_HOWS", "0,1,2,3,4,5")
            .into_iter()
            .map(|v| v as u8)
            .collect();
        // ROD_EXACT_H=1: the exact mode's H at the nodes instead of the eigensolver's
        let exact_h = std::env::var("ROD_EXACT_H").is_ok_and(|v| v == "1");
        let d = 0.15;
        let (exact, jump) = rod_kerr_exact(d);
        println!(
            "exact D {d}: neff {:.6} (H_phi jump {jump:.1e}) n2V {:.6e} n2F {:.6e} A41 {:.6} \
             A44 {:.6} V/F {:.5} V/S {:.5}",
            exact.neff,
            exact.n2_vector,
            exact.n2_foster,
            exact.area,
            exact.area_transverse,
            exact.vector_over_foster(),
            exact.vector_over_standard(),
        );
        for (k, h) in grids.iter().enumerate() {
            for (how, r) in rod_kerr(d, *h, &hows, exact_h) {
                println!(
                    "step {}/{} ({:.0} %, {:.0} s): how {how} D {d} h {h}: neff {:.6} \
                     n2V {:.6e} n2F {:.6e} A41 {:.6} A44 {:.6} V/F {:.5} (rel {:.2e}) V/S {:.5}",
                    k + 1,
                    grids.len(),
                    100.0 * (k + 1) as f64 / grids.len() as f64,
                    start.elapsed().as_secs_f64(),
                    r.neff,
                    r.n2_vector,
                    r.n2_foster,
                    r.area,
                    r.area_transverse,
                    r.vector_over_foster(),
                    r.vector_over_foster() / exact.vector_over_foster() - 1.0,
                    r.vector_over_standard(),
                );
            }
        }
    }

    #[test]
    #[ignore = "a study, run by hand (cargo test --release -- --ignored --nocapture)"]
    fn rod_exact_h_diagnostic() {
        // the exact H at the nodes: the cells' H_z (from ∇·H) and H_t against the exact ones
        let rod = RodExact::new(0.15);
        for h in [0.01, 0.005, 0.0025] {
            let n = (1.2f64 / h).round() as usize;
            let x: Vec<f64> = (0..=n).map(|i| 1.2 * i as f64 / n as f64).collect();
            let (mut hx, mut hy) = (Vec::new(), Vec::new());
            for &px in &x {
                for &py in &x {
                    let [a, b, _] = rod.h_at(px, py);
                    hx.push(a);
                    hy.push(b);
                }
            }
            let cs = CrossSection::uniform((0.0, 1.2, n), (0.0, 1.2, n), |_, _| {
                Permittivity::isotropic(c64::new(1.0, 0.0))
            })
            .unwrap();
            let mode = VectorMode::from_nodes(rod.beta, rod.k, x.clone(), x.clone(), hx, hy);
            let f = mode.fields(&cs).unwrap();
            let mut worst = [(0.0f64, 0usize); 3];
            for (c, hh) in f.h.iter().enumerate() {
                let (px, py) = (f.xc[c / n], f.yc[c % n]);
                let ex = rod.h_at(px, py);
                for q in 0..3 {
                    let e = portable::complex::abs(hh[q] - ex[q]);
                    if e > worst[q].0 {
                        worst[q] = (e, c);
                    }
                }
            }
            for (q, (e, c)) in worst.iter().enumerate() {
                let (px, py) = (f.xc[c / n], f.yc[c % n]);
                println!(
                    "h {h}: H[{q}] largest error {e:.3e} at r {:.5} (x {px:.5}, y {py:.5}), exact {:.4}",
                    portable::hypot(px, py),
                    rod.h_at(px, py)[q]
                );
            }
        }
    }

    #[test]
    #[ignore = "a study, run by hand (cargo test --release -- --ignored --nocapture)"]
    fn afshar_and_monros_rod_exact() {
        // the step-index fibre's exact fields: γ^V, γ^F and γ^S by diameter, as their Fig. 1c
        let ds = [
            0.1, 0.12, 0.14, 0.15, 0.16, 0.18, 0.19, 0.2, 0.22, 0.25, 0.3, 0.4, 0.6,
        ];
        for (k, d) in ds.iter().enumerate() {
            let (r, jump) = rod_kerr_exact(*d);
            let g = |n2: f64, area: f64| {
                crate::mode::quantities::nonlinear_coefficient(rod_lam(), n2, area)
            };
            println!(
                "exact {}/{} D {d}: neff {:.6} jump {jump:.1e} n2V {:.6e} n2F {:.6e} A41 {:.6} \
                 A44 {:.6} V/F {:.5} V/S {:.5} gV {:.1} gF {:.1} gS {:.1}",
                k + 1,
                ds.len(),
                r.neff,
                r.n2_vector,
                r.n2_foster,
                r.area,
                r.area_transverse,
                r.vector_over_foster(),
                r.vector_over_standard(),
                g(r.n2_vector, r.area),
                g(r.n2_foster, r.area),
                g(ROD[1], r.area_transverse),
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
