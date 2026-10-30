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

/// The largest |O_ab| between the four TE modes of highest index of a 1 µm slab at spacing `h`
/// (the conjugated product), and the largest |O_aa − 1|.
pub(crate) fn slab_orthogonality(h: f64) -> (f64, f64) {
    let (cs, m) = slab(1.0, h, Polarization::Te, 4, false);
    let fields: Vec<Fields> = m.iter().map(|m| m.fields(&cs).expect("its grid")).collect();
    orthogonality(&fields, Product::Conjugated)
        .map_or((f64::NAN, f64::NAN), |o| (o.off_diagonal(), o.diagonal()))
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
