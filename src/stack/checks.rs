//! The validation cases of etches and sidewalls (docs/methods/stacks.md): a vertical wall's
//! cross-section to the bit, a sloped wall's exact share of its cells, a wide rib's limit the
//! slab, Hadley's standard rib (2002, Table I) and its convergence, a sloped wall against an
//! exact oblique interface (a round rod), and a sidewall's effect on n_eff converging.

use faer::c64;

use super::{Etch, Layer, LayerStack, Structure};
use crate::geometry::{Point, Shape};
use crate::material::{Material, Model, Provenance};
use crate::mode::Polarization;
use crate::mode::vector::{self, Boundaries, Boundary, CrossSection, Permittivity};
use crate::units::{Length, Wavelength};
use crate::validation::{Case, Outcome, Tier};

const OK: &str = "a valid case";

/// A material of constant index `n`, valid from 0.2 to 20 µm.
pub(crate) fn constant(name: &str, n: f64) -> Material {
    Material::new(
        name,
        Model::Constant(c64::new(n, 0.0)),
        Wavelength::um(0.2).expect(OK),
        Wavelength::um(20.0).expect(OK),
        Provenance {
            reference: "a constant index, for a check".into(),
            doi: String::new(),
            data: String::new(),
            temperature: None,
            notes: String::new(),
        },
    )
    .expect(OK)
}

/// A stack of one layer `core` µm thick of `n_core` (its background `n_above`) on a
/// substrate of `n_below` under a cladding of `n_above`.
pub(crate) fn one_layer(n_below: f64, n_core: f64, n_above: f64, core: f64) -> LayerStack {
    let above = constant("above", n_above);
    LayerStack::new(
        constant("below", n_below),
        vec![Layer {
            name: "core".into(),
            thickness: Length::um(core),
            material: constant("core", n_core),
            background: above.clone(),
        }],
        above,
    )
    .expect(OK)
}

/// A guide along x, `width` µm wide at its top, centred on y = 0, `etch`ed, on the layer
/// `core` of `stack`.
pub(crate) fn guide(stack: LayerStack, width: f64, etch: Etch) -> Structure {
    let mut s = Structure::new(stack);
    let shape = Shape::rect(Point::um(0.0, 0.0), Length::um(100.0), Length::um(width)).expect(OK);
    s.draw_etched("core", shape, etch).expect(OK);
    s
}

/// The cross-section of `s` cut at x = 0, across y on the nodes `y` and up z on `z` (µm), each
/// cell from [`Structure::cut_cell`] at `wavelength`, with an electric wall on its west edge
/// (the guide's vertical mirror plane, for its TE-like modes) when `y` starts at 0.
pub(crate) fn section(
    s: &Structure,
    y: Vec<f64>,
    z: Vec<f64>,
    wavelength: Wavelength,
) -> CrossSection {
    let mut cells = Vec::with_capacity((y.len() - 1) * (z.len() - 1));
    for i in 0..y.len() - 1 {
        for j in 0..z.len() - 1 {
            let fill = s.cut_cell(
                Point::um(0.0, 0.5 * (y[i] + y[i + 1])),
                0.5 * (z[j] + z[j + 1]),
                [0.0, 1.0],
                [y[i + 1] - y[i], z[j + 1] - z[j]],
            );
            cells.push(Permittivity::isotropic(
                fill.permittivity(wavelength).expect(OK),
            ));
        }
    }
    let mirrored = y[0] == 0.0;
    let cs = CrossSection::new(y, z, cells).expect(OK);
    if mirrored {
        cs.with_boundaries(Boundaries {
            west: Boundary::ElectricWall,
            ..Boundaries::default()
        })
        .expect(OK)
    } else {
        cs
    }
}

/// Nodes from `a` to `b`, `n` uniform cells.
pub(crate) fn line(a: f64, b: f64, n: usize) -> Vec<f64> {
    (0..=n).map(|i| a + (b - a) * i as f64 / n as f64).collect()
}

/// The fundamental's n_eff of `cs` by the standard scheme.
fn fundamental(cs: &CrossSection, wavelength: Wavelength) -> f64 {
    vector::modes(cs, wavelength, 1, None).expect(OK)[0]
        .effective_index()
        .re
}

/// 1.55 µm.
fn telecom() -> Wavelength {
    Wavelength::um(1.55).expect(OK)
}

/// A 220 nm silicon (3.476) strip in silica (1.444), 500 nm wide at its top, its walls at
/// `sidewall_deg` from the plane.
fn soi_strip(sidewall_deg: f64) -> Structure {
    guide(
        one_layer(1.444, 3.476, 1.444, 0.22),
        0.5,
        Etch::FULL.with_sidewall_deg(sidewall_deg),
    )
}

/// Graded nodes for the strip's half, `h` µm in the box around its core: y from its mirror
/// plane, z through its film, its top corner on a node.
fn strip_nodes(h: f64) -> (Vec<f64>, Vec<f64>) {
    (
        vector::graded_nodes(0.0, 1.6, (0.0, 0.45), h, 0.1, 0.04, &[0.25]),
        vector::graded_nodes(-1.0, 1.2, (-0.05, 0.27), h, 0.1, 0.04, &[0.0, 0.22]),
    )
}

/// The largest difference, permittivity and n_eff, between the strip drawn as it always was and
/// the same strip drawn with a 90° sidewall and through an etched layer (a sloped shape far
/// beyond the window): zero, to the bit.
pub(crate) fn vertical_bits() -> f64 {
    let lam = telecom();
    let stack = || one_layer(1.444, 3.476, 1.444, 0.22);
    let rect = || Shape::rect(Point::um(0.0, 0.0), Length::um(100.0), Length::um(0.5)).expect(OK);
    let mut plain = Structure::new(stack());
    plain.draw("core", rect()).expect(OK);
    let mut ninety = Structure::new(stack());
    ninety
        .draw_etched("core", rect(), Etch::FULL.with_sidewall_deg(90.0))
        .expect(OK);
    let mut etched = Structure::new(stack());
    etched
        .draw_etched("core", rect(), Etch::FULL.with_sidewall_deg(90.0))
        .expect(OK);
    let far = Shape::rect(Point::um(0.0, 50.0), Length::um(1.0), Length::um(1.0)).expect(OK);
    etched
        .draw_etched("core", far, Etch::FULL.with_sidewall_deg(70.0))
        .expect(OK);
    assert!(!etched.is_plain("core") && ninety.is_plain("core"));
    let (y, z) = strip_nodes(0.02);
    let reference = section(&plain, y.clone(), z.clone(), lam);
    let n0 = fundamental(&reference, lam);
    let mut worst = 0.0f64;
    for s in [&ninety, &etched] {
        let cs = section(s, y.clone(), z.clone(), lam);
        for i in 0..y.len() - 1 {
            for j in 0..z.len() - 1 {
                let (a, b) = (
                    cs.cell(i as isize, j as isize),
                    reference.cell(i as isize, j as isize),
                );
                worst = worst.max((a.xx - b.xx).norm());
            }
        }
        worst = worst.max((fundamental(&cs, lam) - n0).abs());
    }
    worst
}

/// The largest relative difference between the area a trapezoid's cells hold by their shares
/// and the trapezoid's own, (w + d cot θ) d, for walls at 60, 75 and 85 degrees on a grid whose
/// nodes miss them, and for a rib's partial etch (its slab across the window too).
pub(crate) fn sloped_fill() -> f64 {
    let mut worst = 0.0f64;
    let y = vector::graded_nodes(-1.0, 1.0, (-0.4, 0.4), 0.0137, 0.3, 0.05, &[]);
    // the film's faces and the rib's floor on nodes, as a cross-section puts them
    let z = line(-0.1, 0.4, 50);
    let cells = |s: &Structure| -> f64 {
        let mut area = 0.0;
        for i in 0..y.len() - 1 {
            for j in 0..z.len() - 1 {
                let size = [y[i + 1] - y[i], z[j + 1] - z[j]];
                let f = match s.cut_cell(
                    Point::um(0.0, 0.5 * (y[i] + y[i + 1])),
                    0.5 * (z[j] + z[j + 1]),
                    [0.0, 1.0],
                    size,
                ) {
                    super::Fill::Uniform(m) => f64::from(u8::from(m.name() == "core")),
                    super::Fill::Wall { fraction, .. } => fraction,
                };
                area += f * size[0] * size[1];
            }
        }
        area
    };
    let (film, w) = (0.22, 0.5);
    for angle in [60.0f64, 75.0, 85.0] {
        let cot = 1.0 / angle.to_radians().tan();
        // its top face w wide, widening by cot θ per µm down
        let s = guide(
            one_layer(1.444, 3.476, 1.444, film),
            w,
            Etch::FULL.with_sidewall_deg(angle),
        );
        let exact = film * (w + film * cot);
        worst = worst.max(((cells(&s) - exact) / exact).abs());
        // a rib: etched 0.12 µm deep, its slab 0.10 thick across the 2 µm window
        let rib = guide(
            one_layer(1.444, 3.476, 1.444, film),
            w,
            Etch::partial(Length::um(0.12)).with_sidewall_deg(angle),
        );
        let exact = 0.12 * (w + 0.12 * cot) + 0.10 * 2.0;
        worst = worst.max(((cells(&rib) - exact) / exact).abs());
    }
    worst
}

/// The slab's TE index, and the rib's quasi-TE at `widths` (µm) on Hadley's structure with a
/// 0.5 µm slab, on a 50 nm grid by Hadley's equations; the slab's from the widths' trend in
/// 1/W².
pub(crate) fn wide_rib() -> (f64, f64) {
    let lam = Wavelength::um(1.15).expect(OK);
    let slab = crate::mode::slab::Slab::new(3.40, 3.44, 1.0, Length::um(1.0)).expect(OK);
    let exact = slab.modes(Polarization::Te, lam)[0].effective_index();
    let widths = [8.0, 16.0];
    let n = widths.map(|w| rib_te(w, 0.5, 0.05, w / 2.0 + 3.0, 4.0, 1.0));
    // n(W) = n∞ − C/W²: from two widths
    let r = (widths[1] / widths[0]).powi(2);
    let limit = (r * n[1] - n[0]) / (r - 1.0);
    (limit, exact)
}

/// A rib `width` µm wide on Hadley's structure (2002, Fig. 13): a 1 µm film of 3.44 on a
/// substrate of 3.40 under air, etched to leave a slab `slab` µm thick (0: etched through).
pub(crate) fn rib(width: f64, slab: f64) -> Structure {
    let etch = if slab > 0.0 {
        Etch::partial(Length::um(1.0 - slab))
    } else {
        Etch::FULL
    };
    guide(one_layer(3.40, 3.44, 1.0, 1.0), width, etch)
}

/// The rib's quasi-TE n_eff at 1.15 µm by Hadley's equations ([`crate::mode::hadley`]) on a
/// uniform grid of `h` µm, its half from the vertical mirror plane out to `side` µm, from
/// `below` µm under the film to `above` µm over it.
pub(crate) fn rib_te(width: f64, slab: f64, h: f64, side: f64, below: f64, above: f64) -> f64 {
    let lam = Wavelength::um(1.15).expect(OK);
    let s = rib(width, slab);
    let ny = (side / h).round() as usize;
    let nz = ((below + 1.0 + above) / h).round() as usize;
    let cs = section(&s, line(0.0, side, ny), line(-below, 1.0 + above, nz), lam);
    crate::mode::hadley::modes(&cs, lam, 1, None).expect(OK)[0]
        .effective_index()
        .re
}

/// Hadley's Table I: the standard rib's TE index at slabs of 0 to 0.7 µm.
pub(crate) const HADLEY_TABLE: [f64; 8] = [
    3.412022, 3.412126, 3.412279, 3.412492, 3.412774, 3.413132, 3.413571, 3.414100,
];

/// The largest difference from Hadley's Table I, the rib drawn with a partial etch, on a
/// 50 nm grid, 3.5 µm of slab beside the rib and 4 µm of substrate under it.
pub(crate) fn hadley_table() -> f64 {
    HADLEY_TABLE
        .iter()
        .enumerate()
        .map(|(k, &expected)| (rib_te(3.0, k as f64 / 10.0, 0.05, 5.0, 4.0, 1.0) - expected).abs())
        .fold(0.0, f64::max)
}

/// The rib's (0.5 µm slab) order of convergence by Hadley's equations from 100 to 25 nm.
pub(crate) fn rib_order() -> f64 {
    let n = [0.1, 0.05, 0.025].map(|h| rib_te(3.0, 0.5, h, 5.0, 4.0, 1.0));
    ((n[0] - n[1]) / (n[1] - n[2])).abs().log2()
}

/// The exact HE₁₁ effective index of a round rod of index `n1` and radius `a` µm in `n2` at
/// `wavelength`: the root of the step-index fibre's eigenvalue equation for hybrid modes of
/// order ν = 1 (E. Snitzer, J. Opt. Soc. Am. 51, 491 (1961), doi:10.1364/JOSA.51.000491,
/// its characteristic equation), J and K by their integrals (each spectrally accurate by the midpoint rule).
pub(crate) fn he11(n1: f64, n2: f64, a: f64, wavelength: Wavelength) -> f64 {
    let j = |n: f64, x: f64| {
        let k = 2000;
        (0..k)
            .map(|i| {
                let t = (i as f64 + 0.5) / k as f64 * std::f64::consts::PI;
                (n * t - x * t.sin()).cos()
            })
            .sum::<f64>()
            / k as f64
    };
    let kb = |n: f64, x: f64| {
        let (k, top) = (4000, 8.0);
        (0..k)
            .map(|i| {
                let t = (i as f64 + 0.5) / k as f64 * top;
                (-x * t.cosh()).exp() * (n * t).cosh()
            })
            .sum::<f64>()
            * top
            / k as f64
    };
    let k0 = wavelength.wavenumber();
    let f = |ne: f64| {
        let u = a * k0 * (n1 * n1 - ne * ne).sqrt();
        let w = a * k0 * (ne * ne - n2 * n2).sqrt();
        let jp = (j(0.0, u) - j(1.0, u) / u) / (u * j(1.0, u));
        let kp = (-kb(0.0, w) - kb(1.0, w) / w) / (w * kb(1.0, w));
        (jp + kp) * (n1 * n1 * jp + n2 * n2 * kp)
            - (1.0 / (u * u) + 1.0 / (w * w)) * (n1 * n1 / (u * u) + n2 * n2 / (w * w))
    };
    // the fundamental: the root nearest the top, bracketed by a scan down from the core
    let steps = 400;
    let at = |s: usize| n2 + (n1 - n2) * (steps - s) as f64 / steps as f64;
    let (mut a, mut b) = (at(1), at(2));
    for s in 1..steps {
        (a, b) = (at(s + 1), at(s));
        if f(a) * f(b) <= 0.0 && f(a).is_finite() && f(b).is_finite() {
            break;
        }
    }
    for _ in 0..80 {
        let m = 0.5 * (a + b);
        if f(a) * f(m) <= 0.0 {
            b = m;
        } else {
            a = m;
        }
    }
    0.5 * (a + b)
}

/// A silicon rod 0.3 µm in radius in silica at 1.55 µm, every interface oblique to the grid,
/// its cells averaged as a sloped wall's ([`super::mean`]) on a quarter of `h` µm cells: its
/// HE₁₁ index, and the exact one.
pub(crate) fn rod(h: f64) -> (f64, f64) {
    let lam = telecom();
    let (n1, n2, a) = (3.476, 1.444, 0.3);
    let n = (1.2 / h).round() as usize;
    let x = line(0.0, 1.2, n);
    let (core, clad) = (c64::new(n1 * n1, 0.0), c64::new(n2 * n2, 0.0));
    let mut cells = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            let (x0, y0) = (x[i], x[j]);
            // the disc's share of the cell, from 64 × 64 samples where its edge crosses it
            let r = (0.5 * (x[i] + x[i + 1])).hypot(0.5 * (x[j] + x[j + 1]));
            let f = if (r - a).abs() < h {
                let k = 64;
                let inside = (0..k * k)
                    .filter(|s| {
                        let px = x0 + ((s % k) as f64 + 0.5) / k as f64 * (x[i + 1] - x0);
                        let py = y0 + ((s / k) as f64 + 0.5) / k as f64 * (x[j + 1] - y0);
                        px.hypot(py) < a
                    })
                    .count();
                inside as f64 / (k * k) as f64
            } else if r < a {
                1.0
            } else {
                0.0
            };
            cells.push(Permittivity::isotropic(super::mean(core, clad, f)));
        }
    }
    let cs = CrossSection::new(x.clone(), x, cells)
        .expect(OK)
        .with_boundaries(Boundaries {
            west: Boundary::ElectricWall,
            south: Boundary::MagneticWall,
            ..Boundaries::default()
        })
        .expect(OK);
    (fundamental(&cs, lam), he11(n1, n2, a, lam))
}

/// The sidewall's effect on the strip's n_eff, n(80°) − n(90°), at 10, 5 and 2.5 nm.
pub(crate) fn sidewall_effect() -> [f64; 3] {
    let lam = telecom();
    [0.01, 0.005, 0.0025].map(|h| {
        let (y, z) = strip_nodes(h);
        let n = |deg: f64| fundamental(&section(&soi_strip(deg), y.clone(), z.clone(), lam), lam);
        n(80.0) - n(90.0)
    })
}

/// The cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "stack/vertical-sidewall-bits",
            title: r"A 500 × 220 nm strip drawn with a 90° sidewall, and on a layer that another shape's sloped etch makes an etched one, has the cross-section and $n_\mathrm{eff}$ of the strip drawn as before, on a 20 nm grid (largest difference shown)",
            tier: Tier::Analytic,
            source: "a vertical wall is where the grid's nodes put it, every cell the material at its centre, by either path: the same bits",
            run: || {
                let e = vertical_bits();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 0.0,
                    error: e,
                }
            },
        },
        Case {
            id: "stack/sloped-wall-fill",
            title: r"The shares of a sloped wall's cells add up to the trapezoid's area, $(w + d \cot\theta)\thinspace d$, for walls at 60, 75 and 85° between a graded grid's nodes, and to a rib's mesa and slab (largest relative difference shown)",
            tier: Tier::Analytic,
            source: "a plane wall cuts each cell along a line: the share is the clipped polygon's area, exactly",
            run: || {
                let e = sloped_fill();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 1e-12,
                    error: e,
                }
            },
        },
        Case {
            id: "stack/wide-rib-slab",
            title: r"A wide rib's quasi-TE index tends to its film's slab index: Hadley's rib (3.44 on 3.40 under air, 1 µm film, 0.5 µm slab) 8 and 16 µm wide on a 50 nm grid, extrapolated in $1/W^2$ (shown), against the slab's TE index",
            tier: Tier::Analytic,
            source: r"the three-layer slab's exact TE index (mode::slab, A. Yariv, P. Yeh, Photonics (2007), Section 3.2), $n_\mathrm{eff}(W) = n_\infty - C/W^2$ for the lateral confinement",
            run: || {
                let (limit, exact) = wide_rib();
                Outcome {
                    measured: limit,
                    expected: exact,
                    tolerance: 2e-5,
                    error: (limit - exact).abs(),
                }
            },
        },
        Case {
            id: "stack/rib-hadley-table",
            title: r"Hadley's standard rib (3 µm wide, 1 µm film of 3.44 on 3.40 under air, 1.15 µm, TE), drawn as a partial etch leaving slabs of 0 to 0.7 µm, by his equations on a 50 nm grid (largest difference from his Table I shown)",
            tier: Tier::Published,
            source: r"G. R. Hadley, J. Lightwave Technol. 20, 1219 (2002), doi:10.1109/JLT.2002.800371, Table I: 3.412022 to 3.414100, $\pm 2$–$3 \times 10^{-6}$",
            run: || {
                let e = hadley_table();
                Outcome {
                    measured: e,
                    expected: 0.0,
                    tolerance: 3e-6,
                    error: e,
                }
            },
        },
        Case {
            id: "stack/rib-order",
            title: r"Hadley's rib with a 0.5 µm slab, drawn as a partial etch, by his equations: the order of convergence of its index from 100 to 25 nm (shown to two decimals)",
            tier: Tier::Published,
            source: "G. R. Hadley (2002), part II, Section IV: from second to third order with the corners' equations",
            run: || {
                let order = rib_order();
                Outcome {
                    measured: (order * 100.0).round() / 100.0,
                    expected: 2.0,
                    tolerance: 1.0,
                    error: (2.0 - order).max(0.0),
                }
            },
        },
        Case {
            id: "stack/oblique-rod",
            title: r"A sloped wall's cells as the mode solver averages them, on an interface oblique everywhere: a silicon rod 0.3 µm in radius in silica at 1.55 µm on a 5 nm grid, $\mathrm{HE}_{11}$ (index shown)",
            tier: Tier::Analytic,
            source: r"the step-index fibre's exact eigenvalue equation (E. Snitzer, J. Opt. Soc. Am. 51, 491 (1961), doi:10.1364/JOSA.51.000491): 3.001088; the scheme assumes interfaces on its grid lines, so its error at an oblique one stays near 2e-3 (staircased: 6.1e-3)",
            run: || {
                let (found, exact) = rod(0.005);
                Outcome {
                    measured: found,
                    expected: exact,
                    tolerance: 2.5e-3,
                    error: (found - exact).abs(),
                }
            },
        },
        Case {
            id: "stack/sidewall-convergence",
            title: r"A sidewall's effect on $n_\mathrm{eff}$ converges: a 500 × 220 nm silicon strip in silica at 1.55 µm, $n(80°) - n(90°)$ at 10, 5 and 2.5 nm (the change from 5 to 2.5 nm over that from 10 to 5 shown)",
            tier: Tier::Analytic,
            source: "a convergent discretization's changes shrink as its grid does; first order halves them",
            run: || {
                let d = sidewall_effect();
                let ratio = ((d[2] - d[1]) / (d[1] - d[0])).abs();
                Outcome {
                    measured: (ratio * 100.0).round() / 100.0,
                    expected: 0.5,
                    tolerance: 0.25,
                    error: (ratio - 0.5).max(0.0),
                }
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vertical_wall_keeps_the_bits() {
        assert_eq!(vertical_bits(), 0.0);
    }

    #[test]
    fn a_sloped_walls_shares_add_up_to_the_trapezoid() {
        let e = sloped_fill();
        assert!(e < 1e-12, "{e}");
    }

    /// The cases' numbers, for the report's sources:
    /// `cargo test --release --lib stack::checks::tests::numbers -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn numbers() {
        let (limit, exact) = wide_rib();
        println!(
            "wide rib {limit:.7} slab {exact:.7} ({:+.1e})",
            limit - exact
        );
        println!("Hadley table {:.2e}", hadley_table());
        println!("rib order {:.3}", rib_order());
        let (found, exact) = rod(0.005);
        println!("rod {found:.7} exact {exact:.7} ({:+.1e})", found - exact);
        let d = sidewall_effect();
        println!("sidewall {d:?} ratio {}", (d[2] - d[1]) / (d[1] - d[0]));
    }
}
