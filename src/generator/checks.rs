//! What the validation report and the tests check the generators on: every port against its
//! closed form and on its guide's end, lengths and areas against closed forms, gaps and bus
//! positions, the couplers' and crossings' symmetries, the devices' geometry against the
//! shapes they drew before they used the generators, and the polygons' convergence.

use std::f64::consts::PI;

use super::*;
use crate::device::{Device, StripBend, StripCoupler, StripRing, StripWaveguide, Values};
use crate::geometry::{Polygons, Tolerance, partial_euler};
use crate::validation::{Case, Outcome, Tier};

const L: &str = "Si";

/// `g` drawn on the layer `"Si"`.
pub(crate) fn drawn(g: &dyn Generator) -> Drawing {
    g.draw(L)
        .unwrap_or_else(|e| panic!("a {} draws: {e}", g.kind()))
}

fn p(x: f64, y: f64) -> Point {
    Point::um(x, y)
}

/// A port as (x, y, direction, width).
type Want = (f64, f64, f64, f64);

/// The generators the cases draw, with their ports' closed forms, in order.
pub(crate) fn examples() -> Vec<(Box<dyn Generator>, Vec<Want>)> {
    let (w, r) = (0.5, 5.0);
    let deg = |a: f64| a.to_radians();
    let theta: f64 = 37.0;
    let (s, c) = deg(theta).sin_cos();
    // an Euler S-bend's ends: (L, h)
    let (ls, hs): (f64, f64) = (12.0, 3.0);
    // a ring's buses
    let (gap, gd, lc) = (0.2, 0.3, 4.0);
    let reach = r + w + 1.0;
    let reach_rt = lc / 2.0 + r + w + 1.0;
    let top = w + gap + 2.0 * r;
    // a directional coupler with arms
    let (cg, cl, la, ha) = (0.25, 8.0, 6.0, 1.5);
    let cc = (cg + w) / 2.0;
    // a bent coupler
    let (br, bt) = (10.0, 30.0);
    let (bs, bcos) = deg(bt / 2.0).sin_cos();
    let outer = br + w + cg;
    // an MMI, 2 × 2, with tapers
    let (mw, ml, wa, wt, lt) = (3.0, 20.0, 0.5, 1.0, 5.0);
    // a Y-branch
    let (yl, yla, ys) = (2.0, 10.0, 3.0);
    // a spiral
    let spiral = Spiral {
        straight: Length::um(4.0),
        lead: Length::um(2.0),
        ..Spiral::new(um(w), um(2.0), um(6.0), 4)
    };
    let (end, dir) = spiral.outer_end();
    let (ex, ey) = (end.x.to_um(), end.y.to_um());
    vec![
        (
            Box::new(Straight::new(um(w), um(10.0))),
            vec![(0.0, 0.0, 180.0, w), (10.0, 0.0, 0.0, w)],
        ),
        (
            Box::new(Taper::new(um(w), um(2.0), um(15.0))),
            vec![(0.0, 0.0, 180.0, w), (15.0, 0.0, 0.0, 2.0)],
        ),
        (
            Box::new(Taper::new(um(w), um(2.0), um(15.0)).with_profile(TaperProfile::Exponential)),
            vec![(0.0, 0.0, 180.0, w), (15.0, 0.0, 0.0, 2.0)],
        ),
        (
            Box::new(Taper::new(um(2.0), um(w), um(15.0)).with_profile(TaperProfile::Parabolic)),
            vec![(0.0, 0.0, 180.0, 2.0), (15.0, 0.0, 0.0, w)],
        ),
        (
            Box::new(Bend::arc(um(w), um(r), theta)),
            vec![(0.0, 0.0, 180.0, w), (r * s, r - r * c, theta, w)],
        ),
        (
            Box::new(Bend::euler(um(w), um(r), -90.0, 0.5)),
            vec![(0.0, 0.0, 180.0, w), (r, -r, 270.0, w)],
        ),
        (
            Box::new(SBend::new(
                um(w),
                um(ls),
                um(hs),
                SBendKind::Euler { p: 1.0 },
            )),
            vec![(0.0, 0.0, 180.0, w), (ls, hs, 0.0, w)],
        ),
        (
            Box::new(SBend::new(um(w), um(ls), um(-hs), SBendKind::Cosine)),
            vec![(0.0, 0.0, 180.0, w), (ls, -hs, 0.0, w)],
        ),
        (
            Box::new(SBend::new(um(w), um(ls), um(hs), SBendKind::Sine)),
            vec![(0.0, 0.0, 180.0, w), (ls, hs, 0.0, w)],
        ),
        (
            Box::new(SBend::new(um(w), um(ls), um(hs), SBendKind::Bezier)),
            vec![(0.0, 0.0, 180.0, w), (ls, hs, 0.0, w)],
        ),
        (
            Box::new(spiral),
            vec![(-ex, -ey, degrees(dir + 180.0), w), (ex, ey, dir, w)],
        ),
        (
            Box::new(Ring::new(um(w), um(r), um(gap)).with_drop(um(gd))),
            vec![
                (-reach, 0.0, 180.0, w),
                (reach, 0.0, 0.0, w),
                (reach, top + w + gd, 0.0, w),
                (-reach, top + w + gd, 180.0, w),
            ],
        ),
        (
            Box::new(Ring::racetrack(um(w), um(r), um(gap), um(lc))),
            vec![(-reach_rt, 0.0, 180.0, w), (reach_rt, 0.0, 0.0, w)],
        ),
        (
            Box::new(Disk::new(um(w), um(r), um(gap)).with_drop(um(gd))),
            vec![
                (-reach, 0.0, 180.0, w),
                (reach, 0.0, 0.0, w),
                (reach, w + 2.0 * r + gap + gd, 0.0, w),
                (-reach, w + 2.0 * r + gap + gd, 180.0, w),
            ],
        ),
        (
            Box::new(DirectionalCoupler::new(um(w), um(cg), um(cl))),
            vec![
                (0.0, -cc, 180.0, w),
                (0.0, cc, 180.0, w),
                (cl, cc, 0.0, w),
                (cl, -cc, 0.0, w),
            ],
        ),
        (
            Box::new(DirectionalCoupler::new(um(w), um(cg), um(cl)).with_arms(
                um(la),
                um(ha),
                SBendKind::Cosine,
            )),
            vec![
                (-la, -cc - ha, 180.0, w),
                (-la, cc + ha, 180.0, w),
                (cl + la, cc + ha, 0.0, w),
                (cl + la, -cc - ha, 0.0, w),
            ],
        ),
        (
            Box::new(BentCoupler::new(um(w), um(cg), um(br), bt)),
            vec![
                (-br * bs, -br + br * bcos, 180.0 + bt / 2.0, w),
                (-outer * bs, -br + outer * bcos, 180.0 + bt / 2.0, w),
                (outer * bs, -br + outer * bcos, 360.0 - bt / 2.0, w),
                (br * bs, -br + br * bcos, 360.0 - bt / 2.0, w),
            ],
        ),
        (
            Box::new(Mmi::new(um(mw), um(ml), 2, 2, um(wa)).with_tapers(um(wt), um(lt))),
            vec![
                (-lt, -mw / 4.0, 180.0, wa),
                (-lt, mw / 4.0, 180.0, wa),
                (ml + lt, mw / 4.0, 0.0, wa),
                (ml + lt, -mw / 4.0, 0.0, wa),
            ],
        ),
        (
            Box::new(Mmi::new(um(mw), um(ml), 1, 3, um(wa))),
            vec![
                (0.0, 0.0, 180.0, wa),
                (ml, mw / 3.0, 0.0, wa),
                (ml, 0.0, 0.0, wa),
                (ml, -mw / 3.0, 0.0, wa),
            ],
        ),
        (
            Box::new(YBranch::new(um(w), um(w), um(yl), um(yla), um(ys))),
            vec![
                (0.0, 0.0, 180.0, w),
                (yl + yla, ys / 2.0, 0.0, w),
                (yl + yla, -ys / 2.0, 0.0, w),
            ],
        ),
        (
            Box::new(Crossing::new(um(w), um(3.0)).with_centre_width(um(1.2))),
            vec![
                (-3.0, 0.0, 180.0, w),
                (0.0, 3.0, 90.0, w),
                (3.0, 0.0, 0.0, w),
                (0.0, -3.0, 270.0, w),
            ],
        ),
        (
            Box::new(BraggGrating::new(um(w), um(0.02), um(0.31), 20)),
            vec![(0.0, 0.0, 180.0, 0.46), (6.2, 0.0, 0.0, 0.46)],
        ),
        (
            Box::new(SwgWaveguide {
                width_end: um(0.4),
                ..SwgWaveguide::new(um(w), um(0.25), 0.6, 12)
            }),
            vec![(0.0, 0.0, 180.0, w), (11.0 * 0.25 + 0.15, 0.0, 0.0, 0.4)],
        ),
        (
            Box::new(GratingCoupler::new(
                um(w),
                um(10.0),
                um(15.0),
                um(0.63),
                0.5,
                20,
                um(0.07),
            )),
            vec![(0.0, 0.0, 180.0, w)],
        ),
        (
            Box::new(PhotonicCrystal::hexagonal(
                um(0.42),
                um(0.12),
                [7, 4],
                Defect::W1,
            )),
            vec![
                (-7.0 * 0.42 - 0.21, 0.0, 180.0, 3f64.sqrt() * 0.42 - 0.24),
                (7.0 * 0.42 + 0.42, 0.0, 0.0, 3f64.sqrt() * 0.42 - 0.24),
            ],
        ),
        (
            Box::new(EdgeCoupler {
                straight: um(5.0),
                ..EdgeCoupler::new(um(0.15), um(w), um(30.0))
            }),
            vec![(0.0, 0.0, 180.0, 0.15), (35.0, 0.0, 0.0, w)],
        ),
    ]
}

/// Every example's ports against their closed forms, and on their guides' ends: the port's
/// position, direction and width as written; its guide's two edges, ±w/2 across it, on the
/// boundary of the shapes; a point 1 nm inside it inside them and one 1 nm out outside. The
/// largest difference, µm or degrees (infinite for a port missing or a point on the wrong side).
pub(crate) fn port_error() -> f64 {
    let mut worst = 0.0f64;
    for (g, want) in examples() {
        let d = drawn(g.as_ref());
        if d.ports().len() != want.len() {
            return f64::INFINITY;
        }
        for (port, &(x, y, dir, w)) in d.ports().iter().zip(&want) {
            worst = worst
                .max((port.position.x.to_um() - x).abs())
                .max((port.position.y.to_um() - y).abs())
                .max((port.direction - dir).abs())
                .max((port.width.to_um() - w).abs());
            let [ox, oy] = port.outward();
            let (px, py) = (port.position.x.to_um(), port.position.y.to_um());
            for side in [-0.5, 0.5] {
                let e = p(px - side * w * oy, py + side * w * ox);
                let dist = d.distance(L, e).map_or(f64::INFINITY, |l| l.to_um());
                worst = worst.max(dist.abs());
            }
            if !d.contains(L, p(px - 1e-3 * ox, py - 1e-3 * oy))
                || d.contains(L, p(px + 1e-3 * ox, py + 1e-3 * oy))
            {
                return f64::INFINITY;
            }
        }
    }
    worst
}

/// ∫₀¹ f by Simpson's rule on `n` (even) panels.
fn simpson(f: impl Fn(f64) -> f64, n: usize) -> f64 {
    let h = 1.0 / n as f64;
    let mut s = f(0.0) + f(1.0);
    for k in 1..n {
        s += f(k as f64 * h) * if k % 2 == 1 { 4.0 } else { 2.0 };
    }
    s * h / 3.0
}

/// The relative difference of `got` from `want`.
fn rel(got: f64, want: f64) -> f64 {
    ((got - want) / want).abs()
}

/// Lengths and areas against closed forms: a straight's w L; the tapers' (w₁ + w₂) L/2,
/// w₁ L (r − 1)/ln r and 2L (w₂³ − w₁³)/(3 (w₂² − w₁²)); an arc's R θ w; an Euler bend's and
/// an Euler S-bend's w times the partial Euler bends' lengths (Vogelbacher et al.'s Eqs.
/// (5)–(11)); a cosine S-bend's w ∫ √(L² + (hπ/2 sin πt)²) dt (Simpson, 4096 panels); the
/// spiral's w times its closed-form length; a ring's 2πRw, a racetrack's 2πRw + 2 L_c w and a
/// disk's πR²; an MMI's W L and its tapers' (w_a + w_t) L_t/2; a crossing's bars' a (w + w_c);
/// a bent coupler's θ R w. The largest relative difference.
pub(crate) fn area_error() -> f64 {
    let worst = std::cell::Cell::new(0.0f64);
    let w = 0.5;
    let check = |d: &Drawing, want: f64| {
        let got: f64 = d.items().iter().map(|i| i.shape.area()).sum();
        worst.set(worst.get().max(rel(got, want)));
    };
    check(&drawn(&Straight::new(um(w), um(10.0))), w * 10.0);
    let (w1, w2, l) = (0.5, 2.0, 15.0);
    let taper = Taper::new(um(w1), um(w2), um(l));
    check(&drawn(&taper), (w1 + w2) * l / 2.0);
    let ratio: f64 = w2 / w1;
    check(
        &drawn(&taper.clone().with_profile(TaperProfile::Exponential)),
        w1 * l * (ratio - 1.0) / ratio.ln(),
    );
    check(
        &drawn(&taper.with_profile(TaperProfile::Parabolic)),
        2.0 * l * (w2.powi(3) - w1.powi(3)) / (3.0 * (w2 * w2 - w1 * w1)),
    );
    let (r, theta) = (5.0, 37.0f64);
    check(
        &drawn(&Bend::arc(um(w), um(r), theta)),
        r * theta.to_radians() * w,
    );
    for pe in [0.2, 0.5, 1.0] {
        let Ok(e) = partial_euler(um(r), PI / 2.0, pe) else {
            return f64::INFINITY;
        };
        check(
            &drawn(&Bend::euler(um(w), um(r), 90.0, pe)),
            w * e.length.to_um(),
        );
    }
    let (ls, hs): (f64, f64) = (12.0, 3.0);
    let th = 2.0 * (hs / ls).atan();
    let Ok(e) = partial_euler(um(ls / (2.0 * th.sin())), th, 0.6) else {
        return f64::INFINITY;
    };
    check(
        &drawn(&SBend::new(
            um(w),
            um(ls),
            um(hs),
            SBendKind::Euler { p: 0.6 },
        )),
        2.0 * w * e.length.to_um(),
    );
    let cosine = simpson(|t| ls.hypot(hs * PI / 2.0 * (PI * t).sin()), 4096);
    check(
        &drawn(&SBend::new(um(w), um(ls), um(hs), SBendKind::Cosine)),
        w * cosine,
    );
    let spiral = Spiral {
        straight: Length::um(4.0),
        ..Spiral::new(um(w), um(2.0), um(6.0), 5)
    };
    // the closed form, written out: 2c + 2 (Σ π r_k + (n − 1) 2c + 2c + lead)
    let (two_c, lead, r0, pitch) = (4.0, 1.0, 6.0, 2.0);
    let radii: f64 = (1..=5)
        .map(|k| {
            if k == 1 {
                r0
            } else {
                2.0 * r0 + (k as f64 - 1.5) * pitch
            }
        })
        .sum();
    let length = two_c + 2.0 * (PI * radii + 4.0 * two_c + two_c + lead);
    check(&drawn(&spiral), w * length);
    worst.set(worst.get().max(rel(spiral.length().to_um(), length)));
    // a ring, a racetrack and a disk, less their buses (2 reach w)
    let (gap, lc) = (0.2, 4.0);
    let less_bus = |d: &Drawing| -> f64 { d.items()[1..].iter().map(|i| i.shape.area()).sum() };
    let ring = drawn(&Ring::new(um(w), um(r), um(gap)));
    worst.set(worst.get().max(rel(less_bus(&ring), 2.0 * PI * r * w)));
    worst.set(
        worst
            .get()
            .max(rel(ring.items()[0].shape.area(), 2.0 * (r + w + 1.0) * w)),
    );
    let racetrack = drawn(&Ring::racetrack(um(w), um(r), um(gap), um(lc)));
    worst.set(
        worst
            .get()
            .max(rel(less_bus(&racetrack), 2.0 * PI * r * w + 2.0 * lc * w)),
    );
    let disk = drawn(&Disk::new(um(w), um(r), um(gap)));
    worst.set(worst.get().max(rel(less_bus(&disk), PI * r * r)));
    let (mw, ml, wa, wt, lt) = (3.0, 20.0, 0.5, 1.0, 5.0);
    check(
        &drawn(&Mmi::new(um(mw), um(ml), 2, 3, um(wa)).with_tapers(um(wt), um(lt))),
        mw * ml + 5.0 * (wa + wt) * lt / 2.0,
    );
    let (a, wc) = (3.0, 1.2);
    let crossing = drawn(&Crossing::new(um(w), um(a)).with_centre_width(um(wc)));
    for i in crossing.items() {
        worst.set(worst.get().max(rel(i.shape.area(), a * (w + wc))));
    }
    // a Bragg grating: its core L (w − 2Δw) and its teeth 2 N Δw d Λ, half of them
    // apodized away
    let (dw, a, n) = (0.02, 0.31, 40);
    check(
        &drawn(&BraggGrating::new(um(w), um(dw), um(a), n)),
        n as f64 * a * (w - 2.0 * dw) + 2.0 * n as f64 * dw * 0.5 * a,
    );
    check(
        &drawn(&BraggGrating::new(um(w), um(dw), um(a), n).sampled(5, 10)),
        n as f64 * a * (w - 2.0 * dw) + 2.0 * (n / 2) as f64 * dw * 0.5 * a,
    );
    // a subwavelength grating: N (w₀ + w₁)/2 d Λ
    check(
        &drawn(&SwgWaveguide {
            width_end: um(0.4),
            ..SwgWaveguide::new(um(w), um(0.25), 0.6, 12)
        }),
        12.0 * (w + 0.4) / 2.0 * 0.6 * 0.25,
    );
    // a grating coupler: its taper and N teeth
    check(
        &drawn(&GratingCoupler::new(
            um(w),
            um(10.0),
            um(15.0),
            um(0.63),
            0.5,
            20,
            um(0.07),
        )),
        (w + 10.0) / 2.0 * 15.0 + 20.0 * 10.0 * 0.5 * 0.63,
    );
    // a photonic crystal: its slab less N π r²
    let crystal =
        PhotonicCrystal::hexagonal(um(0.42), um(0.12), [7, 4], Defect::L3 { shift: um(0.063) });
    let holes = 15.0 * 9.0 - 3.0;
    check(
        &drawn(&crystal),
        (15.5 * 0.42) * (8.0 * 0.42 * 3f64.sqrt() / 2.0 + 0.42) - holes * PI * 0.12 * 0.12,
    );
    // an inverse taper: (w_tip + w) L_t/2 + w L_s
    check(
        &drawn(&EdgeCoupler {
            straight: um(5.0),
            ..EdgeCoupler::new(um(0.15), um(w), um(30.0))
        }),
        (0.15 + w) / 2.0 * 30.0 + w * 5.0,
    );
    let (br, bt, bg) = (10.0, 30.0f64, 0.25);
    let bent = drawn(&BentCoupler::new(um(w), um(bg), um(br), bt));
    worst.set(
        worst
            .get()
            .max(rel(bent.items()[0].shape.area(), bt.to_radians() * br * w)),
    );
    check(
        &Drawing {
            items: bent.items()[1..].to_vec(),
            ..bent.clone()
        },
        bt.to_radians() * (br + w + bg) * w,
    );
    worst.get()
}

/// Gaps and buses: a ring's, a racetrack's (along its straights) and a disk's gap to its bus
/// and to its drop bus, a directional coupler's along its length, a bent coupler's along its
/// arcs, and a Y-branch's arms' where they leave its stem, each as the signed distance from the
/// edge of one guide to the other's shapes. The largest difference from the gap, µm.
pub(crate) fn gap_error() -> f64 {
    let mut worst = 0.0f64;
    let w = 0.5;
    // the distance from `from` to the shapes `items` of `d`
    let dist = |d: &Drawing, items: &[usize], from: Point| -> f64 {
        items
            .iter()
            .map(|&k| d.items()[k].shape.distance(from).signed.to_um())
            .fold(f64::INFINITY, f64::min)
    };
    let (r, gap, gd, lc) = (5.0, 0.2, 0.3, 4.0);
    let ring = drawn(&Ring::new(um(w), um(r), um(gap)).with_drop(um(gd)));
    let yd = w + gap + 2.0 * r + w + gd;
    worst = worst.max((dist(&ring, &[1], p(0.0, w / 2.0)) - gap).abs());
    worst = worst.max((dist(&ring, &[1], p(0.0, yd - w / 2.0)) - gd).abs());
    // and the drop bus's centre where the ports say
    worst = worst.max(
        (ring
            .port("drop")
            .map_or(f64::INFINITY, |q| q.position.y.to_um())
            - yd)
            .abs(),
    );
    let rt = drawn(&Ring::racetrack(um(w), um(r), um(gap), um(lc)));
    for x in [-lc / 2.0, -0.7, 0.0, 1.3, lc / 2.0] {
        worst = worst.max((dist(&rt, &[1, 2, 3, 4], p(x, w / 2.0)) - gap).abs());
    }
    let disk = drawn(&Disk::new(um(w), um(r), um(gap)).with_drop(um(gd)));
    worst = worst.max((dist(&disk, &[1], p(0.0, w / 2.0)) - gap).abs());
    let ydd = w / 2.0 + gap + 2.0 * r + gd + w / 2.0;
    worst = worst.max((dist(&disk, &[1], p(0.0, ydd - w / 2.0)) - gd).abs());
    let (cg, cl) = (0.25, 8.0);
    let dc = drawn(&DirectionalCoupler::new(um(w), um(cg), um(cl)));
    let c = (cg + w) / 2.0;
    for x in [0.0, 0.3, 4.0, 7.9] {
        worst = worst.max((dist(&dc, &[1], p(x, -c + w / 2.0)) - cg).abs());
    }
    let (br, bt) = (10.0, 30.0f64);
    let bent = drawn(&BentCoupler::new(um(w), um(cg), um(br), bt));
    for a in [-bt / 2.0, -5.0, 0.0, 11.0, bt / 2.0] {
        let (s, co) = (90.0 + a).to_radians().sin_cos();
        let e = p((br + w / 2.0) * co, -br + (br + w / 2.0) * s);
        worst = worst.max((dist(&bent, &[1], e) - cg).abs());
    }
    let (yl, yg) = (2.0, 0.15);
    let y = drawn(&YBranch {
        gap: Length::um(yg),
        ..YBranch::new(um(w), um(w), um(yl), um(10.0), um(3.0))
    });
    worst = worst.max((dist(&y, &[1], p(yl, -yg / 2.0)) - yg).abs());
    worst
}

/// Points spread over `b`, a fixed sequence (an additive recurrence by the plastic number), so
/// none falls on a boundary by construction.
fn spread(b: Bounds, n: usize) -> Vec<Point> {
    let g = 1.324_717_957_244_746_f64;
    let (a1, a2) = (1.0 / g, 1.0 / (g * g));
    let (x0, y0) = (b.min.x.to_um() - 0.5, b.min.y.to_um() - 0.5);
    let (dx, dy) = (
        b.max.x.to_um() - b.min.x.to_um() + 1.0,
        b.max.y.to_um() - b.min.y.to_um() + 1.0,
    );
    (1..=n)
        .map(|k| {
            let (u, v) = ((0.5 + a1 * k as f64).fract(), (0.5 + a2 * k as f64).fract());
            p(x0 + u * dx, y0 + v * dy)
        })
        .collect()
}

/// The couplers' and the crossing's symmetries: at 4000 points each, whether a point is inside
/// against its image's, for a directional coupler with cosine arms mirrored in y = 0 and in
/// x = L/2, a bent coupler in x = 0, a 2 × 2 MMI in y = 0 and x = L/2, a Y-branch in y = 0, a
/// ring with its buses in x = 0, and a tapered crossing turned a quarter. The points that
/// differ.
pub(crate) fn symmetry_errors() -> usize {
    let w = 0.5;
    let mut wrong = 0;
    let mut check = |d: &Drawing, image: &dyn Fn(Point) -> Point| {
        let Some(b) = d.bounds() else {
            wrong += 1;
            return;
        };
        for q in spread(b, 4000) {
            if d.contains(L, q) != d.contains(L, image(q)) {
                wrong += 1;
            }
        }
    };
    let flip_y = |q: Point| p(q.x.to_um(), -q.y.to_um());
    let (cl, la) = (8.0, 6.0);
    let dc = drawn(&DirectionalCoupler::new(um(w), um(0.25), um(cl)).with_arms(
        um(la),
        um(1.5),
        SBendKind::Cosine,
    ));
    check(&dc, &flip_y);
    check(&dc, &|q: Point| p(cl - q.x.to_um(), q.y.to_um()));
    let bent = drawn(&BentCoupler::new(um(w), um(0.25), um(10.0), 30.0));
    check(&bent, &|q: Point| p(-q.x.to_um(), q.y.to_um()));
    let ml = 20.0;
    let mmi = drawn(&Mmi::new(um(3.0), um(ml), 2, 2, um(0.5)).with_tapers(um(1.0), um(5.0)));
    check(&mmi, &flip_y);
    check(&mmi, &|q: Point| p(ml - q.x.to_um(), q.y.to_um()));
    let y = drawn(&YBranch::new(um(w), um(w), um(2.0), um(10.0), um(3.0)));
    check(&y, &flip_y);
    let ring = drawn(&Ring::racetrack(um(w), um(5.0), um(0.2), um(3.0)).with_drop(um(0.3)));
    check(&ring, &|q: Point| p(-q.x.to_um(), q.y.to_um()));
    let crossing = drawn(&Crossing::new(um(w), um(3.0)).with_centre_width(um(1.2)));
    check(&crossing, &|q: Point| p(-q.y.to_um(), q.x.to_um()));
    let bragg = drawn(&BraggGrating::new(um(w), um(0.02), um(0.31), 30));
    check(&bragg, &flip_y);
    check(&bragg, &|q: Point| {
        p(30.0 * 0.31 - q.x.to_um(), q.y.to_um())
    });
    let l3 = drawn(&PhotonicCrystal::hexagonal(
        um(0.42),
        um(0.12),
        [7, 4],
        Defect::L3 { shift: um(0.063) },
    ));
    check(&l3, &flip_y);
    let h1 = drawn(&PhotonicCrystal {
        lattice: CrystalLattice::Square,
        ..PhotonicCrystal::hexagonal(um(0.4), um(0.1), [5, 5], Defect::H1)
    });
    check(&h1, &|q: Point| p(-q.y.to_um(), q.x.to_um()));
    wrong
}

/// Wang et al. 2011's uniform gratings drawn (w = 500 nm, Λ = 310 nm, 50 % duty, 620 µm, the
/// designed corrugations of 10 to 30 nm on each side, recessed), and their sampled ones (bursts
/// of Z₁ = 20 periods, 6.2 µm, every Z₀ = 62 µm, ten of them, at 20 and 40 nm): every tooth's
/// place and size, the length, the guide's 500 nm and 500 − 2Δw by inside tests, the area
/// L(w − Δw), and the bursts' Z₁, Z₀, count and Z₀/Z₁ (their Eq. 4's ten peaks). The largest
/// difference, µm (or relative, the area); infinite for a count that is wrong.
pub(crate) fn wang_error() -> f64 {
    let (w, a, n) = (0.5, 0.31, 2000usize);
    let mut worst = 0.0f64;
    for dw in [0.010, 0.015, 0.020, 0.025, 0.030] {
        let d = drawn(&BraggGrating::new(um(w), um(dw), um(a), n));
        let Some(Shape::Array(teeth)) = d.items().get(1).map(|i| &i.shape) else {
            return f64::INFINITY;
        };
        if teeth.len() != 2 * n {
            return f64::INFINITY;
        }
        for e in teeth.elements() {
            let b = e.shape.bounds();
            let i = e.index[0] as f64;
            let (x0, x1) = (b.min.x.to_um(), b.max.x.to_um());
            let (y0, y1) = (b.min.y.to_um().abs(), b.max.y.to_um().abs());
            let (inner, outer) = (y0.min(y1), y0.max(y1));
            worst = worst
                .max((x0 - (i + 0.25) * a).abs())
                .max((x1 - (i + 0.75) * a).abs())
                .max((inner - (w / 2.0 - dw)).abs())
                .max((outer - w / 2.0).abs());
        }
        let length = d.port("o2").map_or(f64::INFINITY, |q| q.position.x.to_um());
        worst = worst.max((length - 620.0).abs());
        let area: f64 = d.items().iter().map(|i| i.shape.area()).sum();
        worst = worst.max(rel(area, 620.0 * (w - dw)));
        // a tooth reaches w/2, the guide between them w/2 − Δw
        for k in [0usize, 999, 1999] {
            let (tooth, gap) = ((k as f64 + 0.5) * a, k as f64 * a);
            for side in [-1.0, 1.0] {
                let inside = [
                    (tooth, w / 2.0 - 1e-4, true),
                    (tooth, w / 2.0 + 1e-4, false),
                    (gap, w / 2.0 - dw - 1e-4, true),
                    (gap, w / 2.0 - dw + 1e-4, false),
                ];
                for (x, y, want) in inside {
                    if d.contains(L, p(x, side * y)) != want {
                        return f64::INFINITY;
                    }
                }
            }
        }
    }
    for dw in [0.020, 0.040] {
        let d = drawn(&BraggGrating::new(um(w), um(dw), um(a), n).sampled(20, 200));
        let Some(Shape::Array(teeth)) = d.items().get(1).map(|i| &i.shape) else {
            return f64::INFINITY;
        };
        // the bursts: their first teeth's starts, and their teeth
        let mut starts: Vec<f64> = Vec::new();
        let mut per: Vec<usize> = Vec::new();
        let mut last = f64::NEG_INFINITY;
        for e in teeth.elements().iter().filter(|e| e.index[1] == 1) {
            let x0 = e.shape.bounds().min.x.to_um();
            if x0 - last > a * 1.5 {
                starts.push(x0);
                per.push(0);
            }
            *per.last_mut().expect("a burst") += 1;
            last = x0;
        }
        if starts.len() != 10 || per.iter().any(|&c| c != 20) {
            return f64::INFINITY;
        }
        let z1 = per[0] as f64 * a;
        let z0 = (starts[9] - starts[0]) / 9.0;
        worst = worst
            .max((z1 - 6.2).abs())
            .max((z0 - 62.0).abs())
            .max((z0 / z1 - 10.0).abs() * z1);
    }
    worst
}

/// A port as the devices drew it: name, position, direction and width.
type OldPort = (String, Point, f64, Length);

/// A device, whether it has a drop bus, and the values changed from its defaults.
type Design = (Box<dyn Device>, bool, Vec<(&'static str, f64)>);

/// The geometry the devices drew before they drew it with the generators (as #360 wrote it),
/// for a device and its values: the shapes on the core, and the ports.
fn p2_geometry(kind: &str, drop: bool, v: &Values) -> (Vec<Shape>, Vec<OldPort>) {
    use crate::geometry::Path;
    let at = |n: &str| v.get(n).expect("the device's parameter");
    let straight = |w: f64, y: f64, from: f64, to: f64| {
        Shape::rect(
            Point::um((from + to) / 2.0, y),
            Length::um(to - from),
            Length::um(w),
        )
        .expect("a strip")
    };
    let w = at("width");
    let mut shapes = Vec::new();
    let mut ports = Vec::new();
    let mut port = |name: &str, x: Point, d: f64| ports.push((name.to_owned(), x, d, um(w)));
    match kind {
        "waveguide" => {
            let l = at("length");
            if l > 0.0 {
                shapes.push(straight(w, 0.0, 0.0, l));
            }
            port("o1", Point::um(0.0, 0.0), 180.0);
            port("o2", Point::um(l, 0.0), 0.0);
        }
        "bend" => {
            let (r, angle) = (at("radius"), at("angle"));
            let sweep = angle.to_radians();
            let mut end = Point::um(0.0, 0.0);
            if sweep > 0.0 {
                let path = Path::builder(Point::um(0.0, 0.0), 0.0, um(w))
                    .arc(um(r), sweep)
                    .build()
                    .expect("a bend");
                end = path.end().position;
                shapes.push(path.shape());
            }
            port("o1", Point::um(0.0, 0.0), 180.0);
            port("o2", end, angle.rem_euclid(360.0));
        }
        "directional coupler" => {
            let (g, l) = (at("gap"), at("length"));
            let c = (g + w) / 2.0;
            if l > 0.0 {
                for y in [-c, c] {
                    shapes.push(straight(w, y, 0.0, l));
                }
            }
            port("o1", Point::um(0.0, -c), 180.0);
            port("o2", Point::um(0.0, c), 180.0);
            port("o3", Point::um(l, c), 0.0);
            port("o4", Point::um(l, -c), 0.0);
        }
        _ => {
            let (r, lc) = (at("radius"), at("coupling_length"));
            let low = w + at("gap");
            let top = low + 2.0 * r;
            let centre = low + r;
            let reach = lc / 2.0 + r + w + 1.0;
            shapes.push(straight(w, 0.0, -reach, reach));
            if lc > 0.0 {
                for (x, start) in [(-lc / 2.0, PI / 2.0), (lc / 2.0, -PI / 2.0)] {
                    shapes.push(
                        Shape::sector(
                            Point::um(x, centre),
                            um(r - w / 2.0),
                            um(r + w / 2.0),
                            start,
                            PI,
                        )
                        .expect("a half ring"),
                    );
                }
                for y in [low, top] {
                    shapes.push(straight(w, y, -lc / 2.0, lc / 2.0));
                }
            } else {
                shapes.push(Shape::ring(Point::um(0.0, centre), um(r), um(w)).expect("a ring"));
            }
            port("in", Point::um(-reach, 0.0), 180.0);
            port("through", Point::um(reach, 0.0), 0.0);
            if drop {
                let d = top + w + at("gap_drop");
                shapes.push(straight(w, d, -reach, reach));
                port("add", Point::um(reach, d), 0.0);
                port("drop", Point::um(-reach, d), 180.0);
            }
        }
    }
    (shapes, ports)
}

/// The devices' geometry, drawn by the generators, against what they drew before (P2, #360):
/// the waveguide (10 µm, and 0), the bend (90°, 37°, 360° and 0), the directional coupler (its
/// default, and 0 long) and the all-pass and add-drop rings (rings, and racetracks 3 µm long),
/// shape by shape and port by port. The shapes and ports that differ in any bit.
pub(crate) fn device_errors() -> usize {
    let mut wrong = 0;
    let cases: Vec<Design> = vec![
        (Box::new(StripWaveguide::new()), false, vec![]),
        (
            Box::new(StripWaveguide::new()),
            false,
            vec![("length", 0.0)],
        ),
        (Box::new(StripBend::new()), false, vec![]),
        (
            Box::new(StripBend::new()),
            false,
            vec![("angle", 37.0), ("radius", 3.3)],
        ),
        (Box::new(StripBend::new()), false, vec![("angle", 360.0)]),
        (Box::new(StripBend::new()), false, vec![("angle", 0.0)]),
        (Box::new(StripCoupler::new()), false, vec![]),
        (
            Box::new(StripCoupler::new()),
            false,
            vec![("length", 0.0), ("gap", 0.31)],
        ),
        (Box::new(StripRing::all_pass()), false, vec![]),
        (
            Box::new(StripRing::all_pass()),
            false,
            vec![("coupling_length", 3.0)],
        ),
        (
            Box::new(StripRing::add_drop()),
            true,
            vec![("gap_drop", 0.27)],
        ),
        (
            Box::new(StripRing::add_drop()),
            true,
            vec![("coupling_length", 3.0), ("width", 0.45)],
        ),
    ];
    for (device, drop, changes) in cases {
        let Ok(values) = Values::new(device.as_ref(), &changes) else {
            wrong += 1;
            continue;
        };
        let Ok(g) = device.geometry(&values) else {
            wrong += 1;
            continue;
        };
        let (shapes, ports) = p2_geometry(device.kind(), drop, &values);
        let drawn = g.structure.shapes(L);
        wrong += shapes.len().abs_diff(drawn.len()) + ports.len().abs_diff(g.ports.len());
        wrong += shapes.iter().zip(drawn).filter(|(a, b)| a != b).count();
        wrong += ports
            .iter()
            .zip(&g.ports)
            .filter(|((n, x, d, w), q)| {
                *n != q.name
                    || x.x.to_um().to_bits() != q.position.x.to_um().to_bits()
                    || x.y.to_um().to_bits() != q.position.y.to_um().to_bits()
                    || d.to_bits() != q.direction.to_bits()
                    || w.to_um().to_bits() != q.width.to_um().to_bits()
                    || q.layer != L
            })
            .count();
    }
    wrong
}

/// The largest distance from the exact boundary of the polygons' vertices and edges' middles,
/// and their number of vertices.
fn deviation(shape: &Shape, polygons: &Polygons) -> (f64, usize) {
    let mut worst = 0.0f64;
    let mut count = 0;
    for c in &polygons.contours {
        for ring in std::iter::once(&c.outer).chain(&c.holes) {
            let n = ring.len();
            count += n;
            for i in 0..n {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                let mid = Point {
                    x: (a.x + b.x) / 2.0,
                    y: (a.y + b.y) / 2.0,
                };
                for q in [a, mid] {
                    worst = worst.max(shape.distance(q).signed.to_um().abs());
                }
            }
        }
    }
    (worst, count)
}

/// The order of the polygons' distance from the boundary in their number of vertices, at
/// unsnapped tolerances from 1 to 0.001 nm, for a ring and a racetrack's half ring with their
/// bus, an Euler bend, a cosine S-bend and an exponential taper: the least slope of −log
/// distance against log vertices among them.
pub(crate) fn order() -> f64 {
    let w = 0.5;
    let shapes: Vec<Shape> = [
        drawn(&Ring::new(um(w), um(5.0), um(0.2))).items()[1]
            .shape
            .clone(),
        drawn(&Ring::racetrack(um(w), um(5.0), um(0.2), um(3.0))).items()[1]
            .shape
            .clone(),
        drawn(&Bend::euler(um(w), um(5.0), 90.0, 0.5)).items()[0]
            .shape
            .clone(),
        drawn(&SBend::new(um(w), um(12.0), um(3.0), SBendKind::Cosine)).items()[0]
            .shape
            .clone(),
        drawn(&Taper::new(um(w), um(2.0), um(15.0)).with_profile(TaperProfile::Exponential))
            .items()[0]
            .shape
            .clone(),
        drawn(&PhotonicCrystal::hexagonal(
            um(0.42),
            um(0.12),
            [4, 3],
            Defect::L3 { shift: um(0.063) },
        ))
        .items()[0]
            .shape
            .clone(),
    ]
    .into();
    let mut least = f64::INFINITY;
    for shape in &shapes {
        let mut points = Vec::new();
        for nm in [1.0, 0.1, 0.01, 0.001] {
            let Ok(t) = Tolerance::unsnapped(Length::nm(nm)) else {
                return f64::NAN;
            };
            let (d, n) = deviation(shape, &shape.polygons(t));
            points.push(((n as f64).ln(), d.ln()));
        }
        let m = points.len() as f64;
        let (sx, sy) = points
            .iter()
            .fold((0.0, 0.0), |(a, b), q| (a + q.0, b + q.1));
        let (mx, my) = (sx / m, sy / m);
        let (num, den) = points.iter().fold((0.0, 0.0), |(a, b), q| {
            (a + (q.0 - mx) * (q.1 - my), b + (q.0 - mx) * (q.0 - mx))
        });
        least = least.min(-num / den);
    }
    least
}

fn error(e: f64, tolerance: f64) -> Outcome {
    Outcome {
        measured: e,
        expected: 0.0,
        tolerance,
        error: e,
    }
}

fn count(n: usize) -> Outcome {
    error(n as f64, 0.0)
}

/// The generators' validation cases.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "generator/ports",
            title: r"The ports of 26 components (a straight, three tapers, an arc and a partial Euler bend, four S-bends, a double spiral, a ring, a racetrack and a disk with their buses, two directional couplers, a bent one, two MMIs, a Y-branch, a crossing, a Bragg grating, a subwavelength grating, a grating coupler, a W1 photonic-crystal guide and an inverse-taper edge coupler) against their closed forms, and on their guides' ends: each guide's two edges $\pm w/2$ across the port on its shapes' boundary, 1 nm in inside and 1 nm out outside (largest difference shown, µm or degrees)",
            tier: Tier::Analytic,
            source: r"each generator's definition: e.g. an arc's or a partial Euler bend's end $(R\sin\theta, R(1 - \cos\theta))$ facing $\theta$, a ring's buses at $y = 0$ and $2(w + R) + g + g_d$, an MMI's guides at $W(k + \frac12)/N - W/2$; the shapes' signed distances; measured 1.5e-15",
            run: || error(port_error(), 1e-12),
        },
        Case {
            id: "generator/lengths-and-areas",
            title: r"Areas against closed forms: a straight, linear, exponential and parabolic tapers, an arc, partial Euler bends ($p$ = 0.2, 0.5, 1) and an Euler S-bend ($w$ times their lengths), a cosine S-bend, a double spiral of five semicircles per arm, a ring, a racetrack and a disk less their buses, a 2 × 3 MMI with tapers, a tapered crossing's bars, a bent coupler's arcs, a uniform and a sampled Bragg grating, a tapered subwavelength grating, a grating coupler, an L3 photonic-crystal slab and an inverse taper (largest relative difference shown)",
            tier: Tier::Analytic,
            source: r"$w L$; $(w_1 + w_2)L/2$, $w_1 L (r - 1)/\ln r$, $2L(w_2^3 - w_1^3)/3(w_2^2 - w_1^2)$; $R\theta w$; F. Vogelbacher et al., Opt. Express 27, 31394 (2019), doi:10.1364/OE.27.031394, Eqs. (5)–(11); $w\int\sqrt{L^2 + (h\pi/2 \sin\pi t)^2}\thinspace dt$ (Simpson); $2c + 2(\sum\pi r_k + 2nc + \ell)$; $2\pi R w$, $+ 2L_c w$, $\pi R^2$; $WL + (w_a + w_t)L_t/2$ per guide; $a(w + w_c)$; $L(w - 2\Delta w) + 2N\Delta w\thinspace d\Lambda$; $N(w_0 + w_1)d\Lambda/2$; the slab less $N\pi r^2$; measured 1.6e-15",
            run: || error(area_error(), 1e-9),
        },
        Case {
            id: "generator/gaps",
            title: r"Gaps between guides: a ring's, a racetrack's (along its straights) and a disk's to their buses and drop buses, a directional coupler's along its length, a bent coupler's along its arcs and a Y-branch's arms' where they leave the stem, each the signed distance from one guide's edge to the other's shapes (largest difference from the gap shown, µm)",
            tier: Tier::Analytic,
            source: r"the bus at $y = 0$, the ring's centre at $w + g + R$; the guides at $\pm(g + w)/2$; concentric arcs of radii $R$ and $R + w + g$; measured 1.2e-15",
            run: || error(gap_error(), 1e-12),
        },
        Case {
            id: "generator/symmetry",
            title: r"Symmetries at 4000 points each: a directional coupler with cosine S-bend arms mirrored in $y = 0$ and in $x = L/2$, a bent coupler in $x = 0$, a 2 × 2 MMI with tapers in $y = 0$ and $x = L/2$, a Y-branch in $y = 0$, an add-drop racetrack in $x = 0$, a tapered crossing turned a quarter, a Bragg grating in $y = 0$ and $x = L/2$, an L3 photonic-crystal slab in $y = 0$ and an H1 one on a square lattice turned a quarter (points inside one and outside the other shown)",
            tier: Tier::Analytic,
            source: r"the components' definitions are symmetric; measured 0",
            run: || count(symmetry_errors()),
        },
        Case {
            id: "generator/devices",
            title: r"The devices draw their geometry with the generators (the waveguide with the straight, the bend with the arc, the coupler with the directional coupler, the rings with the ring): at twelve designs, their shapes and ports against those they drew before (shapes or ports differing in any bit shown)",
            tier: Tier::Analytic,
            source: r"the geometry of photonoxide 0.5's devices (#360), written out; measured 0",
            run: || count(device_errors()),
        },
        Case {
            id: "generator/convergence",
            title: r"A ring, a racetrack's half ring, a partial Euler bend, a cosine S-bend, an exponential taper and an L3 photonic-crystal slab as polygons at unsnapped tolerances from 1 to 0.001 nm: the least slope among them of $-\log$ of the distance they reach from the exact boundary against $\log$ of their number of vertices",
            tier: Tier::Analytic,
            source: r"chords of length $h$ stray by $\kappa h^2/8$: second order (the slope shown to two decimals)",
            run: || {
                let o = (order() * 100.0).round() / 100.0;
                Outcome {
                    measured: o,
                    expected: 2.0,
                    tolerance: 0.15,
                    error: (o - 2.0).abs(),
                }
            },
        },
        Case {
            id: "generator/wang-bragg",
            title: r"Wang et al.'s Bragg gratings drawn: SOI strips 500 nm wide with sidewall corrugations recessed $\Delta w$ = 10, 15, 20, 25 and 30 nm on each side, period 310 nm, 50 % duty, 620 µm long (2000 periods), and sampled ones ($\Delta w$ = 20 and 40 nm) with bursts of 20 periods every 62 µm; every tooth's place and size, the length, the guide's 500 nm and $500 - 2\Delta w$ nm by inside tests, the area $L(w - \Delta w)$, and the bursts' $Z_1$ = 6.2 µm, $Z_0$ = 62 µm, their ten and $Z_0/Z_1$ = 10 (largest difference shown, µm); geometry only",
            tier: Tier::Published,
            source: r"X. Wang, W. Shi, R. Vafaei, N. A. F. Jaeger, L. Chrostowski, IEEE Photon. Technol. Lett. 23, 290 (2011), doi:10.1109/LPT.2010.2103305, Section II, Fig. 1 and Fig. 5: $w$ = 500 nm, $\Lambda$ = 310 nm, duty 50 %, $L$ = 620 µm, corrugations on both sidewalls; Section III: $Z_1$ = 6.2 µm (20 periods), $Z_0$ = 62 µm, ten sampling periods, $N_{3\text{dB}} \approx Z_0/Z_1$ (Eq. 4); measured 1.1e-13",
            run: || error(wang_error(), 1e-9),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "writes the cases' rows of the report to PHOTONOXIDE_GENERATOR_ROWS"]
    fn rows() {
        let rows: String = cases()
            .into_iter()
            .chain(crate::job::component_cases())
            .map(|c| crate::validation::row(&c).0)
            .collect();
        if let Some(out) = std::env::var_os("PHOTONOXIDE_GENERATOR_ROWS") {
            std::fs::write(out, rows).expect("the rows are written");
        }
    }

    #[test]
    #[ignore = "prints the cases' values; cargo test --release --lib generator::checks -- --ignored --nocapture"]
    fn print() {
        for case in cases() {
            let start = std::time::Instant::now();
            let o = (case.run)();
            println!(
                "{}: measured {:.6e} expected {} error {:.3e} tolerance {} {} ({:.2} s)",
                case.id,
                o.measured,
                o.expected,
                o.error,
                o.tolerance,
                o.passed(),
                start.elapsed().as_secs_f64()
            );
        }
    }
}
