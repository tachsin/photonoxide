//! The mode-converter charts of the lesson on crossings and mode converters, on photonoxide's
//! full-vector mode solver (`mode::vector`):
//!
//! - **Phase matching** (`converter-match`): a 220 nm strip's TE or TM modes against its width,
//!   and the widths where each higher mode's index meets the fundamental index of a narrow
//!   access guide: the condition of an asymmetric directional coupler (Dai et al. 2013).
//! - **The asymmetric coupler** (`converter-coupler`): a 500 nm strip beside a wider one, the
//!   pair's two modes that the narrow strip's TE₀ and the wide strip's TE₁ make, and the power
//!   that crosses from one to the other along the coupler.
//! - **The rotator's hybrid modes** (`rotator-hybrid`): a strip on silica under silica or under
//!   silicon nitride, the two modes of TE₁'s and TM₀'s lateral symmetry against the width: with
//!   the vertical mirror broken they hybridize where their indices would cross (gdsfactory's
//!   splitter-rotator, Liu and Poon 2025, Section 3.5).
//! - **The spot-size converter** (`spot-size`): an inverse taper's tip in oxide, its TE₀ or TM₀
//!   and how much of a Gaussian beam's power it takes (Chrostowski and Hochberg 2015,
//!   Section 5.3.1).
//!
//! The cross-sections are drawn here on graded grids (10 nm over the strips and 100 nm around
//! them, growing to at most 100 nm), every edge of silicon on a node, with mirror walls on the
//! planes of symmetry each problem keeps.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::{Mutex, OnceLock};

use num_complex::Complex64 as c64;
use photonoxide::material::{silica, silicon};
use photonoxide::mode::fields::Fields;
use photonoxide::mode::vector::{
    Boundaries, Boundary, CrossSection, Permittivity, VectorMode, graded_nodes, modes,
};
use photonoxide::units::Wavelength;

use super::{ChartData, ChartSpec, Curve, figure, invalid, param};

/// The film's thickness, µm.
pub const THICKNESS: f64 = 0.22;
/// The grid: `FINE` µm over the strips, growing by `GROWTH` per µm away, to at most `COARSE`.
const FINE: f64 = 0.01;
const GROWTH: f64 = 0.2;
const COARSE: f64 = 0.1;

/// The indices of a strip's cross-section: the core, the cladding below and above it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stack {
    pub core: f64,
    pub below: f64,
    pub above: f64,
}

/// Silicon in silica at `l` µm (Li's, Malitson's), or under `above` if given.
pub fn stack(l: f64, above: Option<f64>) -> photonoxide::Result<Stack> {
    let lam = Wavelength::um(l)?;
    let ox = silica().refractive_index(lam)?.re;
    Ok(Stack {
        core: silicon().refractive_index(lam)?.re,
        below: ox,
        above: above.unwrap_or(ox),
    })
}

/// Strips `guides` (each from, to along x, µm) of the film on the stack: on the nodes `x` and
/// from `y0` to `y1` around the film (y = 0 its middle), the half y ≥ 0 if `y0` is 0 (a wall
/// `south` there), a wall `west` at the window's left edge.
fn section(
    s: &Stack,
    fine: f64,
    x: Vec<f64>,
    (y0, y1): (f64, f64),
    guides: &[(f64, f64)],
    (west, south): (Boundary, Boundary),
) -> photonoxide::Result<CrossSection> {
    let top = THICKNESS / 2.0;
    let fine_y = (y0.max(-top - 0.1), top + 0.1);
    let y = graded_nodes(y0, y1, fine_y, fine, GROWTH, COARSE, &[-top, top]);
    let mut cells = Vec::with_capacity((x.len() - 1) * (y.len() - 1));
    for i in 0..x.len() - 1 {
        let xc = 0.5 * (x[i] + x[i + 1]);
        let inside = guides.iter().any(|&(a, b)| xc > a && xc < b);
        for j in 0..y.len() - 1 {
            let yc = 0.5 * (y[j] + y[j + 1]);
            let n = if inside && yc.abs() < top {
                s.core
            } else if yc < 0.0 {
                s.below
            } else {
                s.above
            };
            cells.push(Permittivity::isotropic(c64::new(n * n, 0.0)));
        }
    }
    CrossSection::new(x, y, cells)?.with_boundaries(Boundaries {
        west,
        south,
        ..Boundaries::default()
    })
}

/// Nodes from `a` to `b` along x, fine over `fine`, with `edges` on nodes.
fn nodes(a: f64, b: f64, around: (f64, f64), fine: f64, edges: &[f64]) -> Vec<f64> {
    graded_nodes(a, b, around, fine, GROWTH, COARSE, edges)
}

/// The half x ≥ 0 of one strip `width` µm wide on cells `fine` µm over it, `margin` µm of
/// cladding beyond it each way: the quarter y ≥ 0 too, under a wall `vertical`, when the
/// cladding above is the one below and `vertical` isn't `Zero`.
pub fn strip(
    s: &Stack,
    width: f64,
    (margin, fine): (f64, f64),
    west: Boundary,
    vertical: Boundary,
) -> photonoxide::Result<CrossSection> {
    let half = width / 2.0;
    let x = nodes(0.0, half + margin, (0.0, half + 0.1), fine, &[half]);
    let top = THICKNESS / 2.0 + margin;
    if s.above == s.below && vertical != Boundary::Zero {
        section(s, fine, x, (0.0, top), &[(-1.0, half)], (west, vertical))
    } else {
        section(
            s,
            fine,
            x,
            (-top, top),
            &[(-1.0, half)],
            (west, Boundary::Zero),
        )
    }
}

/// The walls that keep each family: (west, south) for TE- or TM-like modes of even or odd
/// lateral order (TE₀ and TE₂ under an electric west wall and a magnetic south one, and so on,
/// as the strip charts take them).
pub fn walls(te: bool, even: bool) -> (Boundary, Boundary) {
    use Boundary::{ElectricWall as E, MagneticWall as M};
    match (te, even) {
        (true, true) => (E, M),
        (true, false) => (M, M),
        (false, true) => (M, E),
        (false, false) => (E, E),
    }
}

/// The guided modes' effective indices of one family of a symmetric strip, highest first, at
/// most `count`.
pub fn family(
    s: &Stack,
    width: f64,
    l: f64,
    te: bool,
    even: bool,
    count: usize,
) -> photonoxide::Result<Vec<f64>> {
    let (west, south) = walls(te, even);
    let cs = strip(s, width, (1.0, FINE), west, south)?;
    let lam = Wavelength::um(l)?;
    let surrounding = cs.surrounding_index(lam)?;
    Ok(modes(&cs, lam, count, None)?
        .into_iter()
        .filter(|m| m.is_guided(surrounding))
        .map(|m| m.effective_index().re)
        .collect())
}

/// The index of the lateral order `order` (0, 1, 2, ...) of a symmetric strip, if guided.
pub fn order_index(
    s: &Stack,
    width: f64,
    l: f64,
    te: bool,
    order: usize,
) -> photonoxide::Result<Option<f64>> {
    let found = family(s, width, l, te, order.is_multiple_of(2), order / 2 + 1)?;
    Ok(found.get(order / 2).copied())
}

/// The width at which mode `order` of the strip has the index `target`, bisected between `lo`
/// and `hi` µm to `tol` µm (its index rises with the width).
pub fn matched_width(
    s: &Stack,
    l: f64,
    te: bool,
    order: usize,
    target: f64,
    (lo, hi): (f64, f64),
    tol: f64,
) -> photonoxide::Result<Option<f64>> {
    let at = |w: f64| -> photonoxide::Result<f64> {
        Ok(order_index(s, w, l, te, order)?.unwrap_or(0.0))
    };
    let (mut a, mut b) = (lo, hi);
    if at(b)? < target {
        return Ok(None);
    }
    while b - a > tol {
        let m = 0.5 * (a + b);
        if at(m)? < target {
            a = m;
        } else {
            b = m;
        }
    }
    Ok(Some(0.5 * (a + b)))
}

// ---------------------------------------------------------------------------------------------
// the charts

/// gdsfactory's mode converter (Liu and Poon 2025, Section 3.4): a 500 nm guide beside a 1 µm
/// one, 150 nm apart, for 20 µm.
pub const NARROW: f64 = 0.5;
pub const GDS_WIDE: f64 = 1.0;
pub const GDS_GAP: f64 = 0.15;
pub const GDS_LENGTH: f64 = 20.0;
/// gdsfactory's splitter-rotator's slow taper: 690 to 830 nm over 44 µm, under silicon nitride
/// of index 2.0 (Liu and Poon 2025, Section 3.5 and Fig. 18).
pub const PSR_TAPER: (f64, f64, f64) = (0.69, 0.83, 44.0);
pub const NITRIDE: f64 = 2.0;
/// Chrostowski and Hochberg's nano-taper tip, µm (Section 5.3.1).
pub const TIP: f64 = 0.18;

/// The chart specs.
pub fn specs() -> Vec<ChartSpec> {
    vec![
        ChartSpec {
            id: "converter-match",
            title: "Phase matching a narrow guide to a wide one's higher modes",
            about: "A 220 nm silicon strip in silica: the effective indices of its first four TE or TM modes against its width, and the fundamental index of an access guide of width w_a, dashed. Where a higher mode's curve meets the dashed line, a strip of that width beside the access guide is phase matched to it: an asymmetric directional coupler that turns the fundamental mode into that one.",
            computed_by: "mode::vector::modes on the quarters of the strip that hold each family (mirror walls), 10 nm cells graded to 100 nm, at 23 widths; each matched width bisected to 2 nm",
            params: vec![
                polarization_param(),
                param(
                    "access",
                    "Access width $w_a$",
                    "nm",
                    (300.0, 600.0, 500.0),
                    5.0,
                    false,
                    "The narrow access guide's width, nm.",
                ),
                param(
                    "wavelength",
                    "Wavelength $\\lambda$",
                    "um",
                    (1.45, 1.65, 1.55),
                    0.01,
                    false,
                    "The wavelength, µm; Li's silicon and Malitson's silica at it.",
                ),
            ],
        },
        ChartSpec {
            id: "converter-coupler",
            title: "TE₀ into TE₁ along an asymmetric coupler",
            about: "A 500 nm strip a gap g from a strip W wide (220 nm of silicon in silica, 1.55 µm): the narrow strip's TE₀ sent in, expanded on the pair's two modes that it and the wide strip's TE₁ make, carried along z, and projected on each strip's own mode. Beside it coupled-mode theory's κ²/(κ² + δ²) sin² Sz from the same indices. gdsfactory's mode converter is W = 1 µm, g = 150 nm and 20 µm long.",
            computed_by: "mode::vector::modes on the half y ≥ 0 of the pair and of each strip alone on one grid (10 nm cells graded to 100 nm), Fields::cross and coupling for the overlaps",
            params: vec![
                param(
                    "width",
                    "Wide strip $W$",
                    "um",
                    (0.9, 1.15, GDS_WIDE),
                    0.005,
                    false,
                    "The wide strip's width, µm.",
                ),
                param(
                    "gap",
                    "Gap $g$",
                    "nm",
                    (100.0, 300.0, GDS_GAP * 1e3),
                    5.0,
                    false,
                    "The gap between the strips, nm.",
                ),
            ],
        },
        ChartSpec {
            id: "rotator-hybrid",
            title: "TM₀ and TE₁ against the width, under silica or silicon nitride",
            about: "A 220 nm silicon strip on silica, under silica or under silicon nitride (2.0): the two highest modes of TE₁'s and TM₀'s lateral symmetry against the width, their effective indices or the share of their transverse magnetic field that is TE's. Under silica the strip is mirror-symmetric up and down, the two can't couple, and their curves cross; under nitride they hybridize, and their curves turn away from each other.",
            computed_by: "mode::vector::modes on the half x ≥ 0 under a magnetic wall (both families' symmetry), the quarters of each family under silica, 10 nm cells graded to 100 nm, at 26 widths; VectorMode::te_fraction",
            params: vec![
                param(
                    "cover",
                    "Cladding above",
                    "",
                    (1.0, 2.0, 2.0),
                    1.0,
                    false,
                    "1: silica, the strip mirror-symmetric; 2: silicon nitride of index 2.0, as gdsfactory's splitter-rotator.",
                ),
                param(
                    "show",
                    "Show",
                    "",
                    (1.0, 2.0, 1.0),
                    1.0,
                    false,
                    "1: the effective indices; 2: the TE fractions.",
                ),
                param(
                    "width",
                    "Width $w$",
                    "um",
                    (0.5, 1.0, 0.75),
                    0.01,
                    false,
                    "The width marked, µm.",
                ),
            ],
        },
        ChartSpec {
            id: "spot-size",
            title: "An inverse taper's tip against a Gaussian beam",
            about: "The tip of an inverse taper, a silicon strip w_t wide and 220 nm thick in silica at 1.55 µm: its fundamental TE or TM mode, barely guided, and the share of its power a Gaussian beam focused at the facet takes, against the beam's numerical aperture, w₀ = λ/(π NA).",
            computed_by: "mode::vector::modes on the quarter of the strip with 5 µm of silica beyond it (10 nm cells graded to 100 nm), VectorMode::fields; the Gaussian's overlap Re(⟨m, g⟩⟨g, m⟩)/(⟨m, m⟩⟨g, g⟩) summed over the cells",
            params: vec![
                param(
                    "tip",
                    "Tip width $w_t$",
                    "nm",
                    (100.0, 300.0, TIP * 1e3),
                    5.0,
                    false,
                    "The tip's width, nm.",
                ),
                polarization_param(),
            ],
        },
    ]
}

fn polarization_param() -> super::Param {
    param(
        "polarization",
        "Polarization",
        "",
        (1.0, 2.0, 1.0),
        1.0,
        false,
        "1: TE (E mostly across the strip); 2: TM (E mostly up).",
    )
}

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: usize) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: Some(colour),
    }
}

type Key = (i64, i64, i64);

fn key(a: f64, b: f64, c: f64) -> Key {
    let pm = |v: f64| (v * 1e6).round() as i64;
    (pm(a), pm(b), pm(c))
}

/// A small cache: cleared when it grows past `LIMIT` entries, so it never holds much.
struct Cache<T> {
    map: Mutex<HashMap<Key, T>>,
}

const LIMIT: usize = 200;

impl<T: Clone> Cache<T> {
    fn new() -> Cache<T> {
        Cache {
            map: Mutex::new(HashMap::new()),
        }
    }

    fn get_or(
        &self,
        k: Key,
        make: impl FnOnce() -> photonoxide::Result<T>,
    ) -> photonoxide::Result<T> {
        if let Some(v) = self.map.lock().ok().and_then(|m| m.get(&k).cloned()) {
            return Ok(v);
        }
        let v = make()?;
        if let Ok(mut m) = self.map.lock() {
            if m.len() >= LIMIT {
                m.clear();
            }
            m.insert(k, v.clone());
        }
        Ok(v)
    }
}

const SUB: [char; 4] = ['₀', '₁', '₂', '₃'];

// ---- phase matching

/// The widths of the phase-matching chart, µm.
fn match_widths() -> Vec<f64> {
    (0..23).map(|k| 0.3 + 0.1 * f64::from(k)).collect()
}

/// Each of the four lateral orders' indices at each width of `widths` (NaN where not guided).
fn order_curves(s: &Stack, l: f64, te: bool, widths: &[f64]) -> photonoxide::Result<Vec<[f64; 4]>> {
    let cases: Vec<(f64, bool)> = widths
        .iter()
        .flat_map(|&w| [(w, true), (w, false)])
        .collect();
    let found = super::crossing::parallel(&cases, |&(w, even)| family(s, w, l, te, even, 2))?;
    Ok((0..widths.len())
        .map(|k| {
            let (even, odd) = (&found[2 * k], &found[2 * k + 1]);
            let at = |v: &Vec<f64>, i: usize| v.get(i).copied().unwrap_or(f64::NAN);
            [at(even, 0), at(odd, 0), at(even, 1), at(odd, 1)]
        })
        .collect())
}

/// The widths phase matched to an access guide `access` µm wide, for orders 1 to 3: bracketed on
/// the sweep `curves` at `widths`, then bisected to 2 nm.
pub fn matched(
    s: &Stack,
    l: f64,
    te: bool,
    target: f64,
    widths: &[f64],
    curves: &[[f64; 4]],
) -> photonoxide::Result<Vec<Option<f64>>> {
    let orders = [1usize, 2, 3];
    super::crossing::parallel(&orders, |&m| {
        let Some(k) = (0..widths.len() - 1).find(|&k| {
            let (a, b) = (curves[k][m], curves[k + 1][m]);
            (a.is_nan() || a < target) && b >= target
        }) else {
            return Ok(None);
        };
        matched_width(s, l, te, m, target, (widths[k], widths[k + 1]), 2e-3)
    })
}

/// A sweep's four orders at each width, the access guide's index and the matched widths.
type Matched = (Vec<[f64; 4]>, f64, Vec<Option<f64>>);

/// The two branches at one width: (n, TE fraction) of each.
type Branches = [(f64, f64); 2];

pub fn match_chart(polarization: f64, access: f64, l: f64) -> photonoxide::Result<ChartData> {
    let te = polarization < 1.5;
    let s = stack(l, None)?;
    static CACHE: OnceLock<Cache<Matched>> = OnceLock::new();
    let (curves, target, widths) =
        CACHE
            .get_or_init(Cache::new)
            .get_or(key(if te { 1.0 } else { 2.0 }, access, l), || {
                let curves = order_curves(&s, l, te, &match_widths())?;
                let target = order_index(&s, access, l, te, 0)?
                    .ok_or_else(|| invalid("converter-match", "the access guide has no mode"))?;
                let widths = matched(&s, l, te, target, &match_widths(), &curves)?;
                Ok((curves, target, widths))
            })?;
    let p = if te { "TE" } else { "TM" };
    let mut series: Vec<Curve> = (0..4)
        .map(|m| {
            let points: Vec<[f64; 2]> = match_widths()
                .iter()
                .zip(&curves)
                .filter(|(_, c)| c[m].is_finite())
                .map(|(&w, c)| [w, c[m]])
                .collect();
            curve(&format!("{p}{}", SUB[m]), points, false, m)
        })
        .collect();
    series.push(curve(
        &format!("{p}₀ of the {:.0} nm access guide", access * 1e3),
        vec![[0.3, target], [2.5, target]],
        true,
        4,
    ));
    let mut figures = vec![figure(
        &format!("{p}₀ of the access guide"),
        target,
        5,
        "",
        "",
    )];
    for (m, w) in widths.iter().enumerate() {
        figures.push(super::Figure {
            label: format!("Matched to {p}{}", SUB[m + 1]),
            value: *w,
            text: w.map_or("beyond 2.5 µm".to_owned(), |w| super::sig(w, 4)),
            unit: "µm".to_owned(),
            note: "bisected to 2 nm".to_owned(),
        });
    }
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "strip width".to_owned(),
        x_length: true,
        y_label: "effective index".to_owned(),
        y_log: false,
        y_range: None,
        series,
        marker: Some(access),
        figures,
        note: format!(
            "Full-vector modes of 220 nm of silicon ({:.4}) in silica ({:.4}) at {l} µm, each family on the quarter that holds it, 10 nm cells graded to 100 nm, a micrometre of silica beyond the strip; guided modes only.",
            s.core, s.below
        ),
    })
}

// ---- the asymmetric coupler

/// The pair of an asymmetric coupler and its parts: the narrow strip's TE₀ and the wide strip's
/// TE₀ and TE₁ alone, the pair's two modes they make, and the overlaps that carry the narrow
/// strip's TE₀ along it.
#[derive(Clone, Debug)]
pub struct Converter {
    /// The narrow strip's TE₀ and the wide strip's TE₁ and TE₀ alone.
    pub n1: f64,
    pub n2: f64,
    pub n_wide0: f64,
    /// The pair's two modes.
    pub modes: [f64; 2],
    launch: [c64; 2],
    /// ⟨φ_m, ψ_j⟩ and ⟨ψ_j, φ_m⟩ for ψ the narrow TE₀, the wide TE₁ and the wide TE₀.
    into: [[c64; 2]; 3],
    back: [[c64; 2]; 3],
    norms: [f64; 3],
}

const AT: f64 = 1.55;

pub fn converter(width: f64, gap: f64) -> photonoxide::Result<Converter> {
    static CACHE: OnceLock<Cache<Converter>> = OnceLock::new();
    CACHE
        .get_or_init(Cache::new)
        .get_or(key(width, gap, AT), || make_converter(width, gap))
}

fn make_converter(width: f64, gap: f64) -> photonoxide::Result<Converter> {
    let s = stack(AT, None)?;
    let lam = Wavelength::um(AT)?;
    let first = (-gap / 2.0 - NARROW, -gap / 2.0);
    let second = (gap / 2.0, gap / 2.0 + width);
    let x = nodes(
        first.0 - 1.5,
        second.1 + 1.5,
        (first.0 - 0.1, second.1 + 0.1),
        FINE,
        &[first.0, first.1, second.0, second.1],
    );
    let build = |guides: &[(f64, f64)]| {
        section(
            &s,
            FINE,
            x.clone(),
            (0.0, THICKNESS / 2.0 + 1.0),
            guides,
            (Boundary::Zero, Boundary::MagneticWall),
        )
    };
    let (pair_cs, one_cs, two_cs) = (
        build(&[first, second])?,
        build(&[first])?,
        build(&[second])?,
    );
    let (pair, one, two) = std::thread::scope(|sc| {
        let one = sc.spawn(|| modes(&one_cs, lam, 1, None));
        let two = sc.spawn(|| modes(&two_cs, lam, 3, None));
        let pair = modes(&pair_cs, lam, 4, None);
        let join = |h: std::thread::ScopedJoinHandle<'_, photonoxide::Result<Vec<VectorMode>>>| {
            h.join()
                .unwrap_or_else(|_| Err(invalid("converter-coupler", "a solve panicked")))
        };
        (pair, join(one), join(two))
    });
    let (pair, one, two) = (pair?, one?, two?);
    let te: Vec<&VectorMode> = two.iter().filter(|m| m.te_fraction() > 0.5).collect();
    if te.len() < 2 {
        return Err(invalid(
            "converter-coupler",
            "the wide strip alone hasn't two TE-like modes",
        ));
    }
    let strips = [
        one[0].fields(&one_cs)?,
        te[1].fields(&two_cs)?,
        te[0].fields(&two_cs)?,
    ];
    let pair_fields: Vec<Fields> = pair
        .iter()
        .map(|m| m.fields(&pair_cs))
        .collect::<photonoxide::Result<_>>()?;
    let cross = |a: &Fields, b: &Fields| {
        a.cross(b)
            .ok_or_else(|| invalid("converter-coupler", "the modes aren't on one grid"))
    };
    // the pair's two modes that the narrow strip's TE₀ launches most
    let mut shares: Vec<(usize, f64)> = pair_fields
        .iter()
        .enumerate()
        .map(|(k, f)| Ok((k, strips[0].coupling(f).unwrap_or(0.0))))
        .collect::<photonoxide::Result<_>>()?;
    shares.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut chosen = [shares[0].0, shares[1].0];
    chosen.sort_unstable();
    let mut launch = [c64::new(0.0, 0.0); 2];
    let mut into = [[c64::new(0.0, 0.0); 2]; 3];
    let mut back = [[c64::new(0.0, 0.0); 2]; 3];
    for (m, &k) in chosen.iter().enumerate() {
        let phi = &pair_fields[k];
        let own = cross(phi, phi)?.re;
        launch[m] = 0.5 * (cross(&strips[0], phi)? + cross(phi, &strips[0])?.conj()) / own;
        for (j, psi) in strips.iter().enumerate() {
            into[j][m] = cross(phi, psi)?;
            back[j][m] = cross(psi, phi)?;
        }
    }
    let norms = [
        cross(&strips[0], &strips[0])?.re,
        cross(&strips[1], &strips[1])?.re,
        cross(&strips[2], &strips[2])?.re,
    ];
    Ok(Converter {
        n1: one[0].effective_index().re,
        n2: te[1].effective_index().re,
        n_wide0: te[0].effective_index().re,
        modes: [
            pair[chosen[0]].effective_index().re,
            pair[chosen[1]].effective_index().re,
        ],
        launch,
        into,
        back,
        norms,
    })
}

impl Converter {
    /// The share of the power launched in the narrow strip's TE₀ that is at `z` µm in mode `j`:
    /// 0 the narrow TE₀, 1 the wide TE₁, 2 the wide TE₀.
    pub fn power(&self, j: usize, z: f64) -> f64 {
        let k = 2.0 * PI / AT;
        let (mut a, mut b) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0));
        for m in 0..2 {
            let amp = self.launch[m] * c64::from_polar(1.0, k * self.modes[m] * z);
            a += amp * self.into[j][m];
            b += amp.conj() * self.back[j][m];
        }
        (a * b).re / (self.norms[0] * self.norms[j])
    }

    /// δ = π(n₁ − n₂)/λ and S = π(n_a − n_b)/λ, 1/µm.
    pub fn delta_and_s(&self) -> (f64, f64) {
        let k = PI / AT;
        (
            k * (self.n1 - self.n2),
            k * (self.modes[0] - self.modes[1]).abs(),
        )
    }

    /// Coupled-mode theory's largest share crossed, κ²/(κ² + δ²) = 1 − (δ/S)².
    pub fn largest(&self) -> f64 {
        let (d, s) = self.delta_and_s();
        (1.0 - (d / s).powi(2)).max(0.0)
    }
}

pub fn coupler_chart(width: f64, gap: f64) -> photonoxide::Result<ChartData> {
    let c = converter(width, gap)?;
    let (d, s) = c.delta_and_s();
    let f = c.largest();
    let lc = PI / (2.0 * s);
    let end = (2.5 * lc).max(1.2 * GDS_LENGTH);
    let steps = 600;
    let (mut narrow, mut te1, mut te0, mut cmt) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for k in 0..=steps {
        let z = end * f64::from(k) / f64::from(steps);
        narrow.push([z, c.power(0, z)]);
        te1.push([z, c.power(1, z)]);
        te0.push([z, c.power(2, z)]);
        cmt.push([z, f * (s * z).sin().powi(2)]);
    }
    let kappa = (s * s - d * d).max(0.0).sqrt();
    let figures = vec![
        figure("$n_1$, the 500 nm strip's TE₀ alone", c.n1, 5, "", ""),
        figure("$n_2$, the wide strip's TE₁ alone", c.n2, 5, "", ""),
        figure(
            "The wide strip's TE₀ alone",
            c.n_wide0,
            5,
            "",
            "far from both",
        ),
        figure(
            "Mismatch $\\delta = \\pi(n_1 - n_2)/\\lambda$",
            d,
            3,
            "1/µm",
            "",
        ),
        figure(
            "Coupling $\\kappa = \\sqrt{S^2 - \\delta^2}$",
            kappa,
            3,
            "1/µm",
            "$S = \\pi(n_a - n_b)/\\lambda$, the pair's modes",
        ),
        figure(
            "Largest crossed, $\\kappa^2/(\\kappa^2 + \\delta^2)$",
            f,
            4,
            "",
            "coupled-mode theory",
        ),
        figure("At $z = \\pi/2S$", lc, 4, "µm", "where it peaks"),
        figure(
            "TE₁ at 20 µm",
            c.power(1, GDS_LENGTH),
            3,
            "",
            "gdsfactory's straight section, the overlaps",
        ),
        figure(
            "Wide TE₀ at 20 µm",
            c.power(2, GDS_LENGTH),
            2,
            "",
            "the crosstalk the two modes carry",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "along the coupler, z".to_owned(),
        x_length: true,
        y_label: "power (share of input)".to_owned(),
        y_log: false,
        y_range: Some([-0.02, 1.05]),
        series: vec![
            curve("in the 500 nm strip's TE₀", narrow, false, 0),
            curve("in the wide strip's TE₁", te1, false, 1),
            curve("in the wide strip's TE₀", te0, false, 2),
            curve("coupled-mode theory", cmt, true, 1),
        ],
        marker: Some(GDS_LENGTH),
        figures,
        note: format!(
            "The 500 nm strip's TE₀ expanded on the pair's two modes it launches most, each carried at its own phase, and projected on each strip's own modes: full-vector modes at {AT} µm on one grid, the half y ≥ 0 with 10 nm cells graded to 100 nm; a strip {:.0} nm wide {:.0} nm away. The dashed curve is κ²/(κ² + δ²) sin² Sz from the same indices; the line marks gdsfactory's 20 µm.",
            width * 1e3,
            gap * 1e3
        ),
    })
}

// ---- the rotator's hybrid modes

/// The widths of the hybrid chart, µm.
fn hybrid_widths() -> Vec<f64> {
    (0..26).map(|k| 0.5 + 0.02 * f64::from(k)).collect()
}

/// At each width, the two modes of TE₁'s and TM₀'s symmetry: (n, TE fraction) of each, the
/// higher index first. Under silica each from its own quarter (TE₁ first); under nitride the
/// half's two highest.
pub fn hybrids(nitride: bool) -> photonoxide::Result<Vec<[(f64, f64); 2]>> {
    static CACHE: OnceLock<Cache<Vec<Branches>>> = OnceLock::new();
    CACHE
        .get_or_init(Cache::new)
        .get_or(key(if nitride { 2.0 } else { 1.0 }, 0.0, 0.0), || {
            let s = stack(1.55, nitride.then_some(NITRIDE))?;
            let lam = Wavelength::um(1.55)?;
            super::crossing::parallel(&hybrid_widths(), |&w| {
                if nitride {
                    let cs = strip(&s, w, (1.0, FINE), Boundary::MagneticWall, Boundary::Zero)?;
                    let found = modes(&cs, lam, 2, None)?;
                    let at = |m: &VectorMode| (m.effective_index().re, m.te_fraction());
                    Ok([at(&found[0]), at(&found[1])])
                } else {
                    let one = |te: bool| -> photonoxide::Result<(f64, f64)> {
                        let (west, south) = walls(te, !te);
                        let cs = strip(&s, w, (1.0, FINE), west, south)?;
                        let m = modes(&cs, lam, 1, None)?.remove(0);
                        Ok((m.effective_index().re, m.te_fraction()))
                    };
                    Ok([one(true)?, one(false)?])
                }
            })
        })
}

/// Where the two branches come closest: the width and the gap in index, the gap's parabola
/// through its smallest sample and their neighbours. Under silica the branches are each
/// family's, so the gap is |n_TE₁ − n_TM₀| and its least is where they cross.
pub fn closest(found: &[[(f64, f64); 2]], nitride: bool) -> (f64, f64) {
    let w = hybrid_widths();
    let gap: Vec<f64> = found
        .iter()
        .map(|p| {
            let d = p[0].0 - p[1].0;
            if nitride { d.abs() } else { d }
        })
        .collect();
    if !nitride {
        // the signed difference changes sign where they cross: linearly between samples
        if let Some(k) = (0..gap.len() - 1).find(|&k| gap[k] * gap[k + 1] <= 0.0) {
            let f = gap[k] / (gap[k] - gap[k + 1]);
            return (w[k] + f * (w[k + 1] - w[k]), 0.0);
        }
        return (f64::NAN, f64::NAN);
    }
    let k = (1..gap.len() - 1)
        .min_by(|&a, &b| gap[a].total_cmp(&gap[b]))
        .unwrap_or(1);
    let (a, b, c) = (gap[k - 1], gap[k], gap[k + 1]);
    let h = w[1] - w[0];
    let denom = a - 2.0 * b + c;
    let off = if denom.abs() > 1e-15 {
        0.5 * (a - c) / denom
    } else {
        0.0
    };
    (w[k] + off * h, b - 0.25 * (a - c) * off)
}

/// The two modes through a taper that crosses the anticrossing, written on the diabatic pair
/// (TM₀ and TE₁ as they would be without the coupling): da₁/dz = i(Δβ_d/2)a₁ + iκa₂,
/// da₂/dz = iκa₁ − i(Δβ_d/2)a₂, with κ = πΔn_min/λ (half the least splitting of β) and the
/// mismatch Δβ_d(w) = ±√(S(w)² − 4κ²) from the branches' splitting S at each width (negative
/// below the least gap's width, positive above), the width growing linearly from `from` to `to`
/// µm over `length` µm. The light starts in the upper branch's local mode, the input's TM₀; the
/// share that ends in the upper branch's local mode at the far end, which the taper delivers as
/// TE₁.
pub fn two_mode_conversion(found: &[[(f64, f64); 2]], (from, to, length): (f64, f64, f64)) -> f64 {
    let (wc, gap) = closest(found, true);
    let w = hybrid_widths();
    let k0 = 2.0 * PI / 1.55;
    let kappa = k0 * gap / 2.0;
    let diabatic: Vec<f64> = found
        .iter()
        .zip(&w)
        .map(|(p, &x)| {
            let split = k0 * (p[0].0 - p[1].0).abs();
            let d = (split * split - 4.0 * kappa * kappa).max(0.0).sqrt();
            if x < wc { -d } else { d }
        })
        .collect();
    let at = |x: f64| -> f64 {
        let h = w[1] - w[0];
        let k = (((x - w[0]) / h).floor().max(0.0) as usize).min(w.len() - 2);
        let f = (x - w[k]) / h;
        diabatic[k] + f * (diabatic[k + 1] - diabatic[k])
    };
    two_mode(|z| at(from + (to - from) * z / length), kappa, length)
}

/// The local mode of the upper branch of [[d/2, κ], [κ, −d/2]]: its eigenvector of the larger
/// eigenvalue, √(d²/4 + κ²).
fn upper(d: f64, kappa: f64) -> [f64; 2] {
    let e = (d * d / 4.0 + kappa * kappa).sqrt();
    // (d/2 − e) x + κ y = 0
    let (x, y) = (kappa, e - d / 2.0);
    let n = (x * x + y * y).sqrt();
    [x / n, y / n]
}

/// The two-mode equations above with the mismatch `delta(z)` and a constant κ over `length` µm,
/// by fourth-order Runge–Kutta in steps of 10 nm, from the upper branch's local mode at z = 0:
/// the share in the upper branch's local mode at the end.
pub fn two_mode(delta: impl Fn(f64) -> f64, kappa: f64, length: f64) -> f64 {
    let i = c64::new(0.0, 1.0);
    let rate = |z: f64, a: [c64; 2]| -> [c64; 2] {
        let d = delta(z) / 2.0;
        [i * (d * a[0] + kappa * a[1]), i * (kappa * a[0] - d * a[1])]
    };
    let steps = (length / 0.01).ceil() as usize;
    let h = length / steps as f64;
    let start = upper(delta(0.0), kappa);
    let mut a = [c64::new(start[0], 0.0), c64::new(start[1], 0.0)];
    let add = |a: [c64; 2], k: [c64; 2], s: f64| [a[0] + k[0] * s, a[1] + k[1] * s];
    for n in 0..steps {
        let z = n as f64 * h;
        let k1 = rate(z, a);
        let k2 = rate(z + h / 2.0, add(a, k1, h / 2.0));
        let k3 = rate(z + h / 2.0, add(a, k2, h / 2.0));
        let k4 = rate(z + h, add(a, k3, h));
        a = [
            a[0] + (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]) * (h / 6.0),
            a[1] + (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]) * (h / 6.0),
        ];
    }
    let end = upper(delta(length), kappa);
    (a[0] * end[0] + a[1] * end[1]).norm_sqr()
}

pub fn hybrid_chart(cover: f64, show: f64, width: f64) -> photonoxide::Result<ChartData> {
    let nitride = cover > 1.5;
    let fractions = show > 1.5;
    let found = hybrids(nitride)?;
    let w = hybrid_widths();
    let names = if nitride {
        ["upper branch", "lower branch"]
    } else {
        ["TE₁", "TM₀"]
    };
    let series: Vec<Curve> = (0..2)
        .map(|b| {
            curve(
                names[b],
                w.iter()
                    .zip(&found)
                    .map(|(&x, p)| [x, if fractions { p[b].1 } else { p[b].0 }])
                    .collect(),
                false,
                b,
            )
        })
        .collect();
    let (wc, gap) = closest(&found, nitride);
    let near = w
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - width).abs().total_cmp(&(b.1 - width).abs()))
        .map_or(0, |(k, _)| k);
    let mut figures = vec![
        figure(
            if nitride {
                "Closest at"
            } else {
                "TE₁ and TM₀ cross at"
            },
            wc,
            3,
            "µm",
            if nitride {
                "the least index gap, by a parabola"
            } else {
                "Chrostowski and Hochberg: about 680 nm"
            },
        ),
        figure(
            &format!("{} at {:.2} µm: TE fraction", names[0], w[near]),
            found[near][0].1,
            3,
            "",
            "",
        ),
        figure(
            &format!("{} at {:.2} µm: TE fraction", names[1], w[near]),
            found[near][1].1,
            3,
            "",
            "",
        ),
    ];
    if nitride {
        let converted = two_mode_conversion(&found, PSR_TAPER);
        figures.push(figure(
            "Least index gap $\\Delta n_\\text{min}$",
            gap,
            3,
            "",
            "",
        ));
        figures.push(figure(
            "Converted by the 44 µm taper",
            converted,
            4,
            "",
            "the two-mode equations, 690 to 830 nm",
        ));
    }
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "strip width".to_owned(),
        x_length: true,
        y_label: if fractions {
            "TE fraction".to_owned()
        } else {
            "effective index".to_owned()
        },
        y_log: false,
        y_range: if fractions { Some([0.0, 1.0]) } else { None },
        series,
        marker: Some(width),
        figures,
        note: format!(
            "Full-vector modes of 220 nm of silicon on silica under {} at 1.55 µm, 10 nm cells graded to 100 nm, a micrometre of cladding beyond the strip. {}",
            if nitride {
                "silicon nitride (2.0)"
            } else {
                "silica"
            },
            if nitride {
                "Both on the half x ≥ 0 under a magnetic wall, which keeps TE₁'s and TM₀'s symmetry; where the lower branch's index is below 2.0 it isn't guided: it leaks into the nitride."
            } else {
                "Each on its own quarter: TE₁ under magnetic walls, TM₀ under a magnetic wall across and an electric one along the film."
            }
        ),
    })
}

// ---- the spot-size converter

/// The tip's mode and its quarter, at 1.55 µm.
fn tip_mode(tip: f64, te: bool) -> photonoxide::Result<(VectorMode, CrossSection)> {
    let s = stack(1.55, None)?;
    let (west, south) = walls(te, true);
    let cs = strip(&s, tip, (5.0, FINE), west, south)?;
    let m = modes(&cs, Wavelength::um(1.55)?, 1, None)?.remove(0);
    Ok((m, cs))
}

/// The share of the tip mode's power a Gaussian beam of waist `w0` µm, polarized as the mode
/// and centred on the strip, takes: Re(⟨m, g⟩⟨g, m⟩)/(Re⟨m, m⟩ Re⟨g, g⟩), ⟨a, b⟩ = ∫(E_a × H_b*)·ẑ
/// over the quarter (the mode and the beam are both even across its two walls), the beam a plane
/// wave's E = G, H = n ẑ × E in silica.
pub fn gaussian_coupling(m: &Fields, cs: &CrossSection, n: f64, w0: f64, te: bool) -> f64 {
    let (x, y) = (cs.x(), cs.y());
    let (mut mg, mut gm, mut gg, mut mm) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0), 0.0, 0.0);
    for i in 0..x.len() - 1 {
        for j in 0..y.len() - 1 {
            let area = (x[i + 1] - x[i]) * (y[j + 1] - y[j]);
            let (xc, yc) = (m.x()[i], m.y()[j]);
            let g = (-(xc * xc + yc * yc) / (w0 * w0)).exp();
            let (e, h) = (m.e(i, j), m.h(i, j));
            // the beam: E = (g, 0) and H = (0, n g) for TE; E = (0, g) and H = (−n g, 0) for TM
            let (egx, egy, hgx, hgy) = if te {
                (g, 0.0, 0.0, n * g)
            } else {
                (0.0, g, -n * g, 0.0)
            };
            mg += (e[0] * hgy - e[1] * hgx) * area;
            gm += (egx * h[1].conj() - egy * h[0].conj()) * area;
            gg += n * g * g * area;
            mm += (e[0] * h[1].conj() - e[1] * h[0].conj()).re * area;
        }
    }
    (mg * gm).re / (mm * gg)
}

/// The numerical apertures of the spot-size chart.
fn apertures() -> Vec<f64> {
    (0..36).map(|k| 0.1 + 0.02 * f64::from(k)).collect()
}

pub fn spot_chart(tip: f64, polarization: f64) -> photonoxide::Result<ChartData> {
    let te = polarization < 1.5;
    let (m, cs) = tip_mode(tip, te)?;
    let fields = m.fields(&cs)?;
    let n = stack(1.55, None)?.below;
    let coupling: Vec<[f64; 2]> = apertures()
        .iter()
        .map(|&na| {
            let w0 = 1.55 / (PI * na);
            [
                na,
                10.0 * gaussian_coupling(&fields, &cs, n, w0, te).log10(),
            ]
        })
        .collect();
    let best = coupling
        .iter()
        .copied()
        .max_by(|a, b| a[1].total_cmp(&b[1]))
        .unwrap_or([f64::NAN, f64::NAN]);
    let p = if te { "TE₀" } else { "TM₀" };
    let figures = vec![
        figure(
            &format!("{p}'s effective index"),
            m.effective_index().re,
            4,
            "",
            "Chrostowski and Hochberg, 180 nm, TE: 1.46",
        ),
        figure("Best NA", best[0], 2, "", "on steps of 0.02"),
        figure("Coupled there", best[1], 3, "dB", ""),
        figure(
            "Its waist $w_0 = \\lambda/\\pi\\mathrm{NA}$",
            1.55 / (PI * best[0]),
            3,
            "µm",
            "",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "the beam's numerical aperture".to_owned(),
        x_length: false,
        y_label: "coupled (dB)".to_owned(),
        y_log: false,
        y_range: Some([-6.0, 0.0]),
        series: vec![curve(
            &format!("{p} of a {:.0} nm tip", tip * 1e3),
            coupling,
            false,
            if te { 0 } else { 1 },
        )],
        marker: Some(best[0]),
        figures,
        note: format!(
            "The tip's full-vector mode at 1.55 µm on its quarter with 5 µm of silica ({n:.4}) beyond it, 10 nm cells graded to 100 nm; a Gaussian beam centred on the strip, waist λ/(π NA) at the facet, its overlap summed over the cells."
        ),
    })
}

// ---------------------------------------------------------------------------------------------
// tests

#[cfg(test)]
mod tests {
    use super::*;

    /// The fundamental mode is phase matched to itself at the access guide's own width, the
    /// closed form the bisection must find within its 2 nm; and Dai et al.'s TM design (3.455 in
    /// 1.445 at 1.5 µm, a 0.4 µm access guide) gives the widths the lesson quotes against their
    /// 1.02, 1.68 and 2.35 µm.
    #[test]
    fn the_matched_widths_are_the_lessons() {
        let s = stack(1.55, None).unwrap();
        let n0 = order_index(&s, 0.5, 1.55, true, 0).unwrap().unwrap();
        let w = matched_width(&s, 1.55, true, 0, n0, (0.3, 0.8), 2e-3)
            .unwrap()
            .unwrap();
        assert!((w - 0.5).abs() <= 2e-3, "{w}");
        let dai = Stack {
            core: 3.455,
            below: 1.445,
            above: 1.445,
        };
        let target = order_index(&dai, 0.4, 1.5, false, 0).unwrap().unwrap();
        let curves = order_curves(&dai, 1.5, false, &match_widths()).unwrap();
        let widths = matched(&dai, 1.5, false, target, &match_widths(), &curves).unwrap();
        for (got, want) in widths.iter().zip([1.047, 1.695, 2.343]) {
            let got = got.unwrap();
            assert!((got - want).abs() <= 2e-3, "{got} against {want}");
        }
    }

    /// The asymmetric coupler phase matched (the wide strip's width bisected so that its TE₁'s
    /// index is the narrow strip's TE₀'s): coupled-mode theory's κ²/(κ² + δ²) is 1 there, and the
    /// overlaps' transfer reaches it within three times the share of the launch the pair's two
    /// modes don't hold.
    #[test]
    fn the_converter_transfers_all_when_phase_matched() {
        let s = stack(AT, None).unwrap();
        let narrow = order_index(&s, NARROW, AT, true, 0).unwrap().unwrap();
        let w = matched_width(&s, AT, true, 1, narrow, (0.9, 1.2), 2e-3)
            .unwrap()
            .unwrap();
        let c = converter(w, GDS_GAP).unwrap();
        let (_, s_half) = c.delta_and_s();
        let left_out = 1.0 - c.power(0, 0.0);
        let peak = (0..=400)
            .map(|k| c.power(1, f64::from(k) * PI / (200.0 * s_half)))
            .fold(0.0, f64::max);
        assert!(c.largest() > 0.99, "{}", c.largest());
        assert!(
            (peak - c.largest()).abs() <= 3.0 * left_out,
            "{peak} against {} ({left_out} left out)",
            c.largest()
        );
    }

    /// Under silica the strip is mirror-symmetric up and down, so TE₁ and TM₀ can't couple: each
    /// keeps its polarization and their curves cross. Under nitride the two hybridize: they never
    /// meet, and their polarizations change places across the least gap.
    #[test]
    fn the_hybrids_cross_only_when_the_mirror_is_broken() {
        let silica = hybrids(false).unwrap();
        let (wc, _) = closest(&silica, false);
        assert!(wc > 0.5 && wc < 1.0, "{wc}");
        assert!(silica.iter().all(|p| p[0].1 > 0.5 && p[1].1 < 0.1));
        let nitride = hybrids(true).unwrap();
        let (_, gap) = closest(&nitride, true);
        assert!(gap > 0.01, "{gap}");
        let first = nitride.first().unwrap()[0].1;
        let last = nitride.last().unwrap()[0].1;
        assert!(first < 0.1 && last > 0.9, "{first} {last}");
    }

    /// With a constant mismatch the local modes don't change, so the light stays in the one it
    /// starts in, to the integrator's fourth order (steps of 10 nm: far below 1e-9); and the
    /// splitter-rotator's slow taper keeps nearly all of its light on the upper branch, more the
    /// slower it is.
    #[test]
    fn the_two_mode_equations_follow_the_local_mode() {
        let stays = two_mode(|_| 0.07, 0.1, 23.0);
        assert!((stays - 1.0).abs() < 1e-9, "{stays}");
        let found = hybrids(true).unwrap();
        let (a, b, length) = PSR_TAPER;
        let quick = two_mode_conversion(&found, (a, b, length));
        let slow = two_mode_conversion(&found, (a, b, 4.0 * length));
        assert!(quick > 0.9 && slow > quick, "{quick} {slow}");
    }

    /// The tip's effective index against Chrostowski and Hochberg's 1.46 for 180 × 220 nm in
    /// oxide (Fig. 5.22), within half its last digit.
    #[test]
    fn the_tip_is_the_books() {
        let (m, _) = tip_mode(TIP, true).unwrap();
        assert!(
            (m.effective_index().re - 1.46).abs() <= 0.005,
            "{}",
            m.effective_index()
        );
    }
}
