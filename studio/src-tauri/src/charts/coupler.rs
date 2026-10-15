//! The directional coupler's charts: two silicon strips' supermodes by photonoxide's full-vector
//! mode solver, and what they make. The beat between the guides and its length, the length
//! against the gap, the split against wavelength from `DirectionalCoupler`, and an unequal
//! pair's incomplete transfer, from the pair's modes and each strip's own.
//!
//! The cross-section is Chrostowski and Hochberg's (Silicon Photonics Design, 2015,
//! doi:10.1017/CBO9781316084168, Section 4.1.6, as the `directional_coupler` example builds it):
//! 500 × 220 nm strips of silicon (Li's index, `material::silicon`) in silica (Malitson's,
//! `material::silica`), in their 5.5 × 1.22 µm window with metal walls. A symmetric pair is
//! solved on the quarter x, y ≥ 0, a wall on each symmetry plane (an electric wall between the
//! strips keeps the even TE-like supermode, a magnetic one the odd); an unequal pair on the half
//! y ≥ 0. The grid is graded: 10 nm over the strips, the gap and 100 nm around them, growing by
//! 0.2 nm per nm beyond to at most 100 nm, every edge of silicon on a node. On it the
//! cross-over length at a 200 nm gap is 1.3 % shorter than on a 5 nm grid graded the same way
//! (36.85 against 37.32 µm at 1.55 µm); the corners converge slowly, as in the example.
//!
//! The solves are cached by their geometry and wavelength, so a chart asked again with only a
//! length or a window changed is drawn from the cache.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::{Mutex, OnceLock};

use num_complex::Complex64 as c64;
use photonoxide::circuit::Component;
use photonoxide::circuit::components::{DirectionalCoupler, Dispersion, Supermodes};
use photonoxide::material::{silica, silicon};
use photonoxide::mode::fields::Fields;
use photonoxide::mode::vector::{
    Boundaries, Boundary, CrossSection, Permittivity, VectorMode, graded_nodes, modes,
};
use photonoxide::units::Wavelength;

use super::{ChartData, Curve, Figure, figure, invalid};

/// The strips' width and thickness, µm (Chrostowski and Hochberg's).
pub const WIDTH: f64 = 0.5;
const THICKNESS: f64 = 0.22;
/// Half the window, across and up, µm: the book's 5.5 × 1.22 µm.
const HALF_ACROSS: f64 = 2.75;
const HALF_UP: f64 = 0.61;
/// The grid: `FINE` µm over the strips, growing by `GROWTH` per µm away, to at most `COARSE`.
pub const FINE: f64 = 0.01;
const GROWTH: f64 = 0.2;
const COARSE: f64 = 0.1;
/// The grid, as the charts' notes give it.
pub const GRID: &str =
    "10 nm over the strips and the gap, graded to 100 nm at the window's metal walls";

/// Chrostowski and Hochberg's fit to their cross-over lengths against the gap at 1550 nm, their
/// Eq. 4.14: L_x = 10^(0.0037645 g[nm] + 0.799434) µm.
pub fn book_fit(gap_nm: f64) -> f64 {
    10f64.powf(0.003_764_5 * gap_nm + 0.799_434)
}

fn wavelength(um: f64) -> photonoxide::Result<Wavelength> {
    Wavelength::um(um)
}

/// Nodes from `a` to `b`, fine over `fine` and with `edges` on nodes.
fn nodes(a: f64, b: f64, fine: (f64, f64), edges: &[f64]) -> Vec<f64> {
    graded_nodes(a, b, fine, FINE, GROWTH, COARSE, edges)
}

/// The nodes up from the strips' mid-height (a magnetic wall there) to the metal wall.
fn nodes_up() -> Vec<f64> {
    let top = THICKNESS / 2.0;
    nodes(0.0, HALF_UP, (0.0, top + 0.05), &[top])
}

/// A cross-section on nodes `x`, `y`: silicon in the strips `guides` (each from, to along x, µm)
/// up to half the thickness, silica elsewhere, at `l`; `west` the wall at its west side.
fn section(
    x: Vec<f64>,
    y: Vec<f64>,
    guides: &[(f64, f64)],
    l: Wavelength,
    west: Boundary,
) -> photonoxide::Result<CrossSection> {
    let (si, ox) = (silicon().permittivity(l)?, silica().permittivity(l)?);
    let top = THICKNESS / 2.0;
    let mut cells = Vec::with_capacity((x.len() - 1) * (y.len() - 1));
    for i in 0..x.len() - 1 {
        let xc = 0.5 * (x[i] + x[i + 1]);
        let inside = guides.iter().any(|&(a, b)| xc > a && xc < b);
        for j in 0..y.len() - 1 {
            let yc = 0.5 * (y[j] + y[j + 1]);
            let core = inside && yc < top;
            cells.push(Permittivity::isotropic(if core { si } else { ox }));
        }
    }
    CrossSection::new(x, y, cells)?.with_boundaries(Boundaries {
        west,
        south: Boundary::MagneticWall,
        ..Boundaries::default()
    })
}

/// The quarter x, y ≥ 0 of two equal strips `gap` µm apart, x = 0 midway: the even supermode with
/// an electric wall at x = 0, the odd with a magnetic one.
pub fn quarter(gap: f64, l: Wavelength, even: bool) -> photonoxide::Result<CrossSection> {
    let (a, b) = (gap / 2.0, gap / 2.0 + WIDTH);
    let x = nodes(0.0, HALF_ACROSS, (0.0, b + 0.1), &[a, b]);
    let wall = if even {
        Boundary::ElectricWall
    } else {
        Boundary::MagneticWall
    };
    section(x, nodes_up(), &[(a, b)], l, wall)
}

/// The fundamental mode of `cs` at `l`, refused unless it is TE-like.
fn fundamental(cs: &CrossSection, l: Wavelength) -> photonoxide::Result<VectorMode> {
    let mode = modes(cs, l, 1, None)?.remove(0);
    if mode.te_fraction() < 0.5 {
        return Err(invalid(
            "coupler chart",
            "the fundamental mode found isn't TE-like",
        ));
    }
    Ok(mode)
}

/// A symmetric pair's two supermodes: their effective indices, and H_y across the pair at the
/// strips' mid-height, each scaled to 1 at its largest, as (x µm, H_y) over the whole width.
/// For these TE-like modes H_y is the large magnetic field, with E_x's shape and symmetry but
/// continuous where E_x jumps, at the strips' sides.
#[derive(Clone, Debug)]
pub struct Pair {
    pub even: f64,
    pub odd: f64,
    pub even_field: Vec<[f64; 2]>,
    pub odd_field: Vec<[f64; 2]>,
}

impl Pair {
    /// L_c = λ/(2Δn), µm.
    pub fn cross_over(&self, l: f64) -> f64 {
        l / (2.0 * (self.even - self.odd))
    }
}

/// H_y along the nodes at mid-height, scaled so its largest is 1 (its phase taken out), then
/// mirrored across x = 0: evenly for the even supermode, with a sign for the odd.
fn profile(mode: &VectorMode, cs: &CrossSection, even: bool) -> Vec<[f64; 2]> {
    let x = cs.x();
    let values: Vec<c64> = (0..x.len()).map(|i| mode.hy(i, 0)).collect();
    let peak = values
        .iter()
        .copied()
        .max_by(|a, b| a.norm().total_cmp(&b.norm()))
        .unwrap_or(c64::new(1.0, 0.0));
    let unit = peak.conj() / (peak.norm() * peak.norm());
    let right: Vec<[f64; 2]> = x
        .iter()
        .zip(&values)
        .map(|(&x, &v)| [x, (v * unit).re])
        .collect();
    let sign = if even { 1.0 } else { -1.0 };
    let mut out: Vec<[f64; 2]> = right
        .iter()
        .skip(1)
        .rev()
        .map(|&[x, v]| [-x, sign * v])
        .collect();
    out.extend(right);
    out
}

type Key = (i64, i64, i64);

fn key(a: f64, b: f64, c: f64) -> Key {
    // to a picometre: a slider's values are far coarser
    let pm = |v: f64| (v * 1e6).round() as i64;
    (pm(a), pm(b), pm(c))
}

/// A small cache: cleared when it grows past `LIMIT` entries, so it never holds much.
struct Cache<T> {
    map: Mutex<HashMap<Key, T>>,
}

const LIMIT: usize = 400;

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

fn pairs() -> &'static Cache<Pair> {
    static CACHE: OnceLock<Cache<Pair>> = OnceLock::new();
    CACHE.get_or_init(Cache::new)
}

/// The supermodes of two equal strips `gap` µm apart at `l` µm, the even and odd solved side by
/// side.
pub fn pair(gap: f64, l: f64) -> photonoxide::Result<Pair> {
    pairs().get_or(key(gap, l, 0.0), || {
        let lam = wavelength(l)?;
        let solve = |even: bool| -> photonoxide::Result<(f64, Vec<[f64; 2]>)> {
            let cs = quarter(gap, lam, even)?;
            let mode = fundamental(&cs, lam)?;
            Ok((mode.effective_index().re, profile(&mode, &cs, even)))
        };
        let (even, odd) = std::thread::scope(|s| {
            let odd = s.spawn(|| solve(false));
            let even = solve(true);
            (
                even,
                odd.join()
                    .unwrap_or_else(|_| Err(invalid("coupler chart", "a solve panicked"))),
            )
        });
        let ((even, even_field), (odd, odd_field)) = (even?, odd?);
        Ok(Pair {
            even,
            odd,
            even_field,
            odd_field,
        })
    })
}

/// A coupler of the pair's supermodes at `l` µm, by `DirectionalCoupler`: its dispersion isn't
/// needed at the one wavelength it is asked at.
fn coupler_at(p: &Pair, l: f64) -> photonoxide::Result<DirectionalCoupler> {
    let lam = wavelength(l)?;
    DirectionalCoupler::new(Supermodes::new(
        Dispersion::new(lam, p.even, p.even),
        Dispersion::new(lam, p.odd, p.odd),
    ))
}

/// The share of the power that crosses (port o1 to o3) and goes through (o1 to o4) a coupler
/// `length` µm long, at `l` µm.
pub fn split(c: &DirectionalCoupler, l: f64, length: f64) -> photonoxide::Result<(f64, f64)> {
    let s = c.s_matrix(wavelength(l)?, &[length])?;
    Ok((s.power(2, 0), s.power(3, 0)))
}

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: None,
    }
}

/// The strips drawn as a curve: 1 over the silicon, 0 elsewhere, across the window.
fn outline(gap: f64) -> Vec<[f64; 2]> {
    let (a, b) = (gap / 2.0, gap / 2.0 + WIDTH);
    let edge = 1.4;
    let mut out = vec![[-edge, 0.0]];
    for &(from, to) in &[(-b, -a), (a, b)] {
        out.extend([[from, 0.0], [from, 1.0], [to, 1.0], [to, 0.0]]);
    }
    out.push([edge, 0.0]);
    // more points along the flats, so the curve is a curve to the window's tests
    let mut dense = Vec::new();
    for w in out.windows(2) {
        let ([x0, y0], [x1, y1]) = (w[0], w[1]);
        let n = if y0 == y1 { 4 } else { 1 };
        for k in 0..n {
            let t = f64::from(k) / f64::from(n);
            dense.push([x0 + t * (x1 - x0), y0 + t * (y1 - y0)]);
        }
    }
    dense.push(out[out.len() - 1]);
    dense
}

/// The pair's supermodes across it.
pub fn supermodes(gap: f64, l: f64) -> photonoxide::Result<ChartData> {
    let p = pair(gap, l)?;
    let within = |v: &Vec<[f64; 2]>| -> Vec<[f64; 2]> {
        v.iter().copied().filter(|q| q[0].abs() <= 1.4).collect()
    };
    let (even, odd) = (within(&p.even_field), within(&p.odd_field));
    // light in the right strip: the two supermodes in step, (even + odd)/2 with each at 1
    let sum: Vec<[f64; 2]> = even
        .iter()
        .zip(&odd)
        .map(|(e, o)| [e[0], 0.5 * (e[1] + o[1])])
        .collect();
    let dn = p.even - p.odd;
    let figures = vec![
        figure("$n_\\text{even}$", p.even, 6, "", "the symmetric supermode"),
        figure(
            "$n_\\text{odd}$",
            p.odd,
            6,
            "",
            "the antisymmetric supermode",
        ),
        figure(
            "$\\Delta n = n_\\text{even} - n_\\text{odd}$",
            dn,
            4,
            "",
            "",
        ),
        figure(
            "$L_c = \\lambda / 2\\Delta n$",
            p.cross_over(l),
            4,
            "µm",
            "the light crosses over",
        ),
        figure(
            "$\\kappa = \\pi\\Delta n / \\lambda$",
            PI * dn / l,
            4,
            "1/µm",
            "the coupling coefficient",
        ),
    ];
    Ok(ChartData {
        x_label: "across the pair, x".to_owned(),
        x_length: true,
        y_label: "H_y at mid-height (largest 1)".to_owned(),
        y_log: false,
        y_range: Some([-1.15, 1.15]),
        series: vec![
            curve("even supermode", even, false),
            curve("odd supermode", odd, false),
            curve("(even + odd)/2: light in one strip", sum, true),
            curve("the strips", outline(gap), true),
        ],
        marker: None,
        figures,
        note: format!(
            "Two {:.0} × {:.0} nm silicon strips in silica, {:.0} nm apart, at {l} µm, by the full-vector mode solver on the quarter of Chrostowski and Hochberg's window, a mirror wall on each symmetry plane; the grid {GRID}.",
            WIDTH * 1e3,
            THICKNESS * 1e3,
            gap * 1e3
        ),
    })
}

/// The power in each strip along the coupler.
pub fn power(gap: f64, l: f64, length: f64) -> photonoxide::Result<ChartData> {
    let p = pair(gap, l)?;
    let c = coupler_at(&p, l)?;
    let lc = p.cross_over(l);
    let end = (2.0 * lc).max(1.25 * length);
    let steps = 600;
    let (mut cross, mut through) = (Vec::new(), Vec::new());
    for k in 0..=steps {
        let z = end * f64::from(k) / f64::from(steps);
        let (x, t) = split(&c, l, z)?;
        cross.push([z, x]);
        through.push([z, t]);
    }
    let (x, t) = split(&c, l, length)?;
    let figures = vec![
        figure("$L_c$", lc, 4, "µm", "complete cross-over"),
        figure("3 dB length $L_c/2$", lc / 2.0, 4, "µm", ""),
        figure(
            "$\\kappa = \\pi\\Delta n / \\lambda$",
            PI * (p.even - p.odd) / l,
            4,
            "1/µm",
            "",
        ),
        figure("Crossed at $L$", x, 4, "", "$\\sin^2 \\kappa L$"),
        figure("Through at $L$", t, 4, "", "$\\cos^2 \\kappa L$"),
    ];
    Ok(ChartData {
        x_label: "along the coupler, z".to_owned(),
        x_length: true,
        y_label: "power (share of input)".to_owned(),
        y_log: false,
        y_range: Some([-0.02, 1.05]),
        series: vec![
            curve("through: the input strip", through, false),
            curve("crossed: the other strip", cross, false),
        ],
        marker: Some(length),
        figures,
        note: format!(
            "photonoxide's DirectionalCoupler from the pair's two supermodes (n_even {:.5}, n_odd {:.5}), {:.0} nm apart at {l} µm, by the full-vector mode solver; the grid {GRID}. The coupled section only: no bends.",
            p.even,
            p.odd,
            gap * 1e3
        ),
    })
}

/// The gaps the cross-over length is drawn at, nm.
fn gaps() -> Vec<f64> {
    (0..=18).map(|k| 100.0 + 50.0 * f64::from(k)).collect()
}

/// One strip's fundamental mode, on the quarter of the window.
fn single(l: f64) -> photonoxide::Result<f64> {
    let lam = wavelength(l)?;
    let x = nodes(0.0, HALF_ACROSS, (0.0, WIDTH / 2.0 + 0.1), &[WIDTH / 2.0]);
    let cs = section(
        x,
        nodes_up(),
        &[(0.0, WIDTH / 2.0)],
        lam,
        Boundary::ElectricWall,
    )?;
    Ok(fundamental(&cs, lam)?.effective_index().re)
}

/// The slope of ln y against x by least squares.
fn log_slope(points: &[[f64; 2]]) -> f64 {
    let n = points.len() as f64;
    let (sx, sy) = points
        .iter()
        .fold((0.0, 0.0), |(a, b), p| (a + p[0], b + p[1].ln()));
    let (mx, my) = (sx / n, sy / n);
    let (num, den) = points.iter().fold((0.0, 0.0), |(a, b), p| {
        (a + (p[0] - mx) * (p[1].ln() - my), b + (p[0] - mx).powi(2))
    });
    num / den
}

/// The cross-over length against the gap.
pub fn gap_chart(l: f64) -> photonoxide::Result<ChartData> {
    let gs = gaps();
    let (results, strip) = std::thread::scope(|s| {
        let handles: Vec<_> = gs
            .iter()
            .map(|&g| s.spawn(move || pair(g * 1e-3, l)))
            .collect();
        let strip = single(l);
        let results: Vec<photonoxide::Result<Pair>> = handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err(invalid("coupler chart", "a solve panicked")))
            })
            .collect();
        (results, strip)
    });
    let mut lengths = Vec::with_capacity(gs.len());
    for (g, r) in gs.iter().zip(results) {
        lengths.push([*g, r?.cross_over(l)]);
    }
    let n = strip?;
    let ox = silica().refractive_index(wavelength(l)?)?.re;
    // the strip's field falls as e^(−γx) into the oxide beside it
    let gamma = 2.0 * PI / l * (n * n - ox * ox).sqrt();
    let wide: Vec<[f64; 2]> = lengths.iter().copied().filter(|p| p[0] >= 200.0).collect();
    let efold = 1.0 / log_slope(&wide);
    let book: Vec<[f64; 2]> = lengths.iter().map(|p| [p[0], book_fit(p[0])]).collect();
    let at200 = lengths
        .iter()
        .find(|p| p[0] == 200.0)
        .map_or(f64::NAN, |p| p[1]);
    let figures = vec![
        figure("$L_c$ at a 200 nm gap", at200, 4, "µm", ""),
        figure(
            "$L_c$ grows e-fold every",
            efold,
            3,
            "nm",
            "fitted from 200 to 1000 nm",
        ),
        figure(
            "The field's decay length $1/\\gamma$",
            1e3 / gamma,
            3,
            "nm",
            "one strip's, into the oxide",
        ),
        figure(
            "The book's e-fold length",
            1.0 / (0.003_764_5 * 10f64.ln()),
            3,
            "nm",
            "Eq. 4.14, at 1550 nm",
        ),
    ];
    Ok(ChartData {
        x_label: "gap g (nm)".to_owned(),
        x_length: false,
        y_label: "cross-over length L_c (µm)".to_owned(),
        y_log: true,
        y_range: None,
        series: vec![
            curve("photonoxide's supermodes", lengths, false),
            curve("Chrostowski and Hochberg, Eq. 4.14", book, true),
        ],
        marker: None,
        figures,
        note: format!(
            "The cross-over length λ/(2Δn) of two {:.0} × {:.0} nm silicon strips in silica at {l} µm, every 50 nm of gap, each pair by the full-vector mode solver on the grid {GRID}; one strip's effective index, {n:.4}, gives the decay γ = 2π √(n_eff² − n_ox²)/λ.",
            WIDTH * 1e3,
            THICKNESS * 1e3
        ),
    })
}

fn couplers() -> &'static Cache<DirectionalCoupler> {
    static CACHE: OnceLock<Cache<DirectionalCoupler>> = OnceLock::new();
    CACHE.get_or_init(Cache::new)
}

/// The centre wavelength and step of the spectrum's dispersion, µm: supermodes solved at
/// 1.50, 1.525, 1.55, 1.575 and 1.60 µm, valid over that band.
pub const CENTRE: f64 = 1.55;
const STEP: f64 = 0.025;

/// The coupler `gap` µm apart, its supermodes' dispersion over 1.50 to 1.60 µm, as the
/// `directional_coupler` example builds it.
pub fn dispersive(gap: f64) -> photonoxide::Result<DirectionalCoupler> {
    couplers().get_or(key(gap, CENTRE, STEP), || {
        DirectionalCoupler::from_modes(
            |l| quarter(gap, l, true),
            |l| quarter(gap, l, false),
            wavelength(CENTRE)?,
            STEP,
        )
    })
}

/// A coupler's split against wavelength, its length a share of L_c at 1.55 µm.
pub fn spectrum(gap: f64, share: f64) -> photonoxide::Result<ChartData> {
    let c = dispersive(gap)?;
    let lc = c.supermodes().cross_over_length(wavelength(CENTRE)?);
    let length = share * lc;
    let (lo, hi) = (CENTRE - 2.0 * STEP, CENTRE + 2.0 * STEP);
    let steps: usize = 400;
    let (mut cross, mut through) = (Vec::new(), Vec::new());
    for k in 0..=steps {
        let l = lo + (hi - lo) * k as f64 / steps as f64;
        let (x, t) = split(&c, l, length)?;
        cross.push([l, x]);
        through.push([l, t]);
    }
    let (at, _) = split(&c, CENTRE, length)?;
    // the band about 1.55 µm where the crossed share stays within 0.05 of its value there
    let centre = steps / 2;
    let inside = |k: usize| (cross[k][1] - at).abs() <= 0.05;
    let (mut a, mut b) = (centre, centre);
    while a > 0 && inside(a - 1) {
        a -= 1;
    }
    while b < steps && inside(b + 1) {
        b += 1;
    }
    let edge = a == 0 || b == steps;
    let band = (cross[b][0] - cross[a][0]) * 1e3;
    let at_edges = |l: f64| -> photonoxide::Result<f64> {
        Ok(c.supermodes().cross_over_length(wavelength(l)?))
    };
    let figures = vec![
        figure("$L$", length, 4, "µm", "the coupler's length"),
        figure("Crossed at 1.55 µm", at, 4, "", ""),
        Figure {
            label: "Within ±0.05 of it".to_owned(),
            value: Some(band),
            text: format!("{}{}", if edge { "> " } else { "" }, super::sig(band, 3)),
            unit: "nm".to_owned(),
            note: if edge {
                "beyond the band drawn".to_owned()
            } else {
                format!("{:.4} to {:.4} µm", cross[a][0], cross[b][0])
            },
        },
        figure("$L_c$ at 1.50 µm", at_edges(lo)?, 4, "µm", ""),
        figure("$L_c$ at 1.55 µm", lc, 4, "µm", ""),
        figure("$L_c$ at 1.60 µm", at_edges(hi)?, 4, "µm", ""),
    ];
    Ok(ChartData {
        x_label: "wavelength".to_owned(),
        x_length: true,
        y_label: "power (share of input)".to_owned(),
        y_log: false,
        y_range: Some([-0.02, 1.05]),
        series: vec![
            curve("crossed", cross, false),
            curve("through", through, false),
        ],
        marker: Some(CENTRE),
        figures,
        note: format!(
            "photonoxide's DirectionalCoupler::from_modes, as the directional_coupler example builds it: the supermodes of two {:.0} × {:.0} nm strips {:.0} nm apart solved at 1.50, 1.525, 1.55, 1.575 and 1.60 µm on the grid {GRID}, and their dispersion fitted through them; the materials' dispersion included. The coupled section only.",
            WIDTH * 1e3,
            THICKNESS * 1e3,
            gap * 1e3
        ),
    })
}

/// An unequal pair at 1.55 µm: its two modes and each strip's own, on one grid.
#[derive(Clone, Debug)]
pub struct Unequal {
    /// The 500 nm strip's index alone, and the other's.
    pub n1: f64,
    pub n2: f64,
    /// The pair's two modes' indices, higher first.
    pub modes: [f64; 2],
    /// Light launched in strip 1's mode: each pair mode's amplitude, and its overlaps with each
    /// strip's mode, ⟨φ_m, ψ_j⟩ and ⟨ψ_j, φ_m⟩, and the strips' Re⟨ψ_j, ψ_j⟩.
    pub launch: [c64; 2],
    pub into: [[c64; 2]; 2],
    pub back: [[c64; 2]; 2],
    pub norms: [f64; 2],
}

/// The wavelength of the unequal pair, µm.
pub const UNEQUAL_AT: f64 = 1.55;

fn unequals() -> &'static Cache<Unequal> {
    static CACHE: OnceLock<Cache<Unequal>> = OnceLock::new();
    CACHE.get_or_init(Cache::new)
}

/// A 500 nm strip and one `other` µm wide, `gap` µm apart, at 1.55 µm: the half y ≥ 0 of the
/// window, the pair's two TE-like modes and each strip's mode alone on the same grid.
pub fn unequal(other: f64, gap: f64) -> photonoxide::Result<Unequal> {
    unequals().get_or(key(other, gap, UNEQUAL_AT), || {
        let lam = wavelength(UNEQUAL_AT)?;
        let first = (-gap / 2.0 - WIDTH, -gap / 2.0);
        let second = (gap / 2.0, gap / 2.0 + other);
        let x = nodes(
            -HALF_ACROSS,
            HALF_ACROSS,
            (first.0 - 0.1, second.1 + 0.1),
            &[first.0, first.1, second.0, second.1],
        );
        let y = nodes_up();
        let build =
            |guides: &[(f64, f64)]| section(x.clone(), y.clone(), guides, lam, Boundary::Zero);
        let pair_cs = build(&[first, second])?;
        let one_cs = build(&[first])?;
        let two_cs = build(&[second])?;
        let (pair_modes, one, two) = std::thread::scope(|s| {
            let one = s.spawn(|| fundamental(&one_cs, lam));
            let two = s.spawn(|| fundamental(&two_cs, lam));
            let pair_modes = modes(&pair_cs, lam, 2, None);
            let join = |h: std::thread::ScopedJoinHandle<'_, photonoxide::Result<VectorMode>>| {
                h.join()
                    .unwrap_or_else(|_| Err(invalid("coupler chart", "a solve panicked")))
            };
            (pair_modes, join(one), join(two))
        });
        let (pair_modes, one, two) = (pair_modes?, one?, two?);
        if pair_modes.iter().any(|m| m.te_fraction() < 0.5) {
            return Err(invalid(
                "coupler chart",
                "the unequal pair's two highest modes aren't both TE-like",
            ));
        }
        let fields: Vec<Fields> = pair_modes
            .iter()
            .map(|m| m.fields(&pair_cs))
            .collect::<photonoxide::Result<_>>()?;
        let strips = [one.fields(&one_cs)?, two.fields(&two_cs)?];
        let cross = |a: &Fields, b: &Fields| {
            a.cross(b)
                .ok_or_else(|| invalid("coupler chart", "the modes aren't on one grid"))
        };
        let mut launch = [c64::new(0.0, 0.0); 2];
        let mut into = [[c64::new(0.0, 0.0); 2]; 2];
        let mut back = [[c64::new(0.0, 0.0); 2]; 2];
        for (m, phi) in fields.iter().enumerate() {
            let own = cross(phi, phi)?.re;
            // the amplitude of pair mode m in strip 1's field, ⟨ψ₁, φ_m⟩/⟨φ_m, φ_m⟩, averaged
            // with its reciprocal form so that it doesn't lean on E or on H alone
            launch[m] = 0.5 * (cross(&strips[0], phi)? + cross(phi, &strips[0])?.conj()) / own;
            for (j, psi) in strips.iter().enumerate() {
                into[j][m] = cross(phi, psi)?;
                back[j][m] = cross(psi, phi)?;
            }
        }
        let norms = [
            cross(&strips[0], &strips[0])?.re,
            cross(&strips[1], &strips[1])?.re,
        ];
        Ok(Unequal {
            n1: one.effective_index().re,
            n2: two.effective_index().re,
            modes: [
                pair_modes[0].effective_index().re,
                pair_modes[1].effective_index().re,
            ],
            launch,
            into,
            back,
            norms,
        })
    })
}

impl Unequal {
    /// The share of the power launched in strip 1's mode that is in strip `j`'s mode at `z` µm:
    /// the field, the pair's two modes each at its own phase, projected on that strip's mode.
    pub fn power(&self, j: usize, z: f64) -> f64 {
        let k = 2.0 * PI / UNEQUAL_AT;
        let (mut a, mut b) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0));
        for m in 0..2 {
            let amp = self.launch[m] * c64::from_polar(1.0, k * self.modes[m] * z);
            a += amp * self.into[j][m];
            b += amp.conj() * self.back[j][m];
        }
        (a * b).re / (self.norms[0] * self.norms[j])
    }

    /// Coupled-mode theory's mismatch δ = (β₁ − β₂)/2 and half the pair's splitting
    /// S = (β_a − β_b)/2 = √(κ² + δ²), 1/µm.
    pub fn delta_and_s(&self) -> (f64, f64) {
        let k = 2.0 * PI / UNEQUAL_AT;
        (
            k * (self.n1 - self.n2) / 2.0,
            k * (self.modes[0] - self.modes[1]) / 2.0,
        )
    }

    /// The largest share coupled-mode theory lets cross, κ²/(κ² + δ²) = 1 − (δ/S)².
    pub fn largest(&self) -> f64 {
        let (d, s) = self.delta_and_s();
        (1.0 - (d / s).powi(2)).max(0.0)
    }
}

/// The unequal pair's power along z.
pub fn mismatch(other_nm: f64, gap_nm: f64) -> photonoxide::Result<ChartData> {
    let u = unequal(other_nm * 1e-3, gap_nm * 1e-3)?;
    let (d, s) = u.delta_and_s();
    let f = u.largest();
    let lc = PI / (2.0 * s);
    let end = 2.5 * lc;
    let steps = 600;
    let (mut one, mut two, mut cmt) = (Vec::new(), Vec::new(), Vec::new());
    for k in 0..=steps {
        let z = end * f64::from(k) / f64::from(steps);
        one.push([z, u.power(0, z)]);
        two.push([z, u.power(1, z)]);
        cmt.push([z, f * (s * z).sin().powi(2)]);
    }
    let most = two.iter().map(|p| p[1]).fold(0.0, f64::max);
    let kappa = (s * s - d * d).max(0.0).sqrt();
    let figures = vec![
        figure("$n_1$, the 500 nm strip alone", u.n1, 5, "", ""),
        figure("$n_2$, the other alone", u.n2, 5, "", ""),
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
        figure(
            "Largest crossed, the modes' overlaps",
            most,
            4,
            "",
            "the solid curve's peak",
        ),
        figure("At $z = \\pi / 2S$", lc, 4, "µm", "where it peaks"),
    ];
    Ok(ChartData {
        x_label: "along the coupler, z".to_owned(),
        x_length: true,
        y_label: "power (share of input)".to_owned(),
        y_log: false,
        y_range: Some([-0.02, 1.05]),
        series: vec![
            curve("in the 500 nm strip", one, false),
            curve("in the other strip", two, false),
            curve("coupled-mode theory, Yariv's Eq. 6", cmt, true),
        ],
        marker: Some(lc),
        figures,
        note: format!(
            "Light launched in the 500 nm strip's own mode, expanded on the pair's two modes, each carried at its own phase, and projected on each strip's own mode: all five modes by the full-vector solver at {UNEQUAL_AT} µm on one grid, {GRID}; a strip {other_nm:.0} nm wide {gap_nm:.0} nm away. The dashed curve is κ²/(κ² + δ²) sin² Sz from the same indices."
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol * b.abs(), "{a} against {b}");
    }

    /// The supermodes against the directional_coupler example's: the cross-over length at a
    /// 200 nm gap within 1.5 % of its 5 nm grid's 37.152 µm (the book's Lorentz silicon and
    /// 1.444 oxide there, Li's silicon and Malitson's silica here), and the even supermode
    /// above the odd, each a TE-like mode with H_y even and odd about the gap's middle.
    #[test]
    fn the_supermodes_match_the_example() {
        let p = pair(0.2, 1.55).unwrap();
        assert!(p.even > p.odd && p.odd > 1.444);
        close(p.cross_over(1.55), 37.152, 0.015);
        let at = |v: &[[f64; 2]], x: f64| {
            v.iter()
                .min_by(|a, b| (a[0] - x).abs().total_cmp(&(b[0] - x).abs()))
                .unwrap()[1]
        };
        // in the strips' middles, ±350 nm: even alike, odd opposite, both large
        let (e1, e2) = (at(&p.even_field, -0.35), at(&p.even_field, 0.35));
        let (o1, o2) = (at(&p.odd_field, -0.35), at(&p.odd_field, 0.35));
        assert!(e1 > 0.5 && (e1 - e2).abs() < 1e-12);
        assert!(o2 > 0.5 && (o1 + o2).abs() < 1e-12);
        // the chart reports them
        let data = supermodes(0.2, 1.55).unwrap();
        assert_eq!(data.figures[0].value, Some(p.even));
        assert_eq!(data.figures[1].value, Some(p.odd));
    }

    /// The power chart is DirectionalCoupler's sin² and cos² of πΔn z/λ (Chrostowski and
    /// Hochberg's Eqs. 4.1–4.3): half crosses at L_c/2, all at L_c.
    #[test]
    fn the_power_is_the_couplers() {
        let p = pair(0.2, 1.55).unwrap();
        let data = power(0.2, 1.55, 20.0).unwrap();
        let lc = p.cross_over(1.55);
        let kappa = PI * (p.even - p.odd) / 1.55;
        for pt in &data.series[1].points {
            assert!(
                (pt[1] - (kappa * pt[0]).sin().powi(2)).abs() < 1e-9,
                "{pt:?}"
            );
        }
        let c = coupler_at(&p, 1.55).unwrap();
        let (x, t) = split(&c, 1.55, lc / 2.0).unwrap();
        close(x, 0.5, 1e-12);
        close(t, 0.5, 1e-12);
        close(split(&c, 1.55, lc).unwrap().0, 1.0, 1e-12);
    }

    /// The cross-over length grows about exponentially with the gap: its e-fold length within a
    /// fifth of the strip's field's decay length into the oxide and of the book's 115 nm, and the
    /// curve within 10 % of the book's fit from 200 to 600 nm.
    #[test]
    fn the_cross_over_length_grows_exponentially() {
        let data = gap_chart(1.55).unwrap();
        assert!(data.y_log);
        let ours = &data.series[0].points;
        assert_eq!(ours.len(), 19);
        assert!(ours.windows(2).all(|w| w[1][1] > w[0][1]));
        let value = |label: &str| {
            data.figures
                .iter()
                .find(|f| f.label.starts_with(label))
                .and_then(|f| f.value)
                .unwrap()
        };
        let efold = value("$L_c$ grows");
        close(efold, value("The field's"), 0.2);
        close(efold, 115.4, 0.2);
        for p in ours.iter().filter(|p| (200.0..=600.0).contains(&p[0])) {
            close(p[1], book_fit(p[0]), 0.1);
        }
        close(
            value("$L_c$ at a 200"),
            pair(0.2, 1.55).unwrap().cross_over(1.55),
            1e-12,
        );
    }

    /// The spectrum is DirectionalCoupler::from_modes's: L_c at 1.53 and 1.57 µm within 2 % of
    /// the example's (40.059 and 33.746 µm, its 10 nm uniform grid), a 3 dB coupler crossing half
    /// at 1.55 µm, and a coupler three times as long (1.5 L_c) keeping its split over a narrower
    /// band.
    #[test]
    fn the_spectrum_is_the_examples_coupler() {
        let c = dispersive(0.2).unwrap();
        let lc = |l: f64| c.supermodes().cross_over_length(wavelength(l).unwrap());
        close(lc(1.53), 40.059, 0.02);
        close(lc(1.57), 33.746, 0.02);
        let half = spectrum(0.2, 0.5).unwrap();
        let crossed = half
            .figures
            .iter()
            .find(|f| f.label == "Crossed at 1.55 µm")
            .unwrap();
        close(crossed.value.unwrap(), 0.5, 1e-9);
        let band = |d: &ChartData| {
            d.figures
                .iter()
                .find(|f| f.label.starts_with("Within"))
                .unwrap()
                .value
                .unwrap()
        };
        let long = spectrum(0.2, 1.5).unwrap();
        assert!(
            band(&long) < 0.6 * band(&half),
            "{} {}",
            band(&long),
            band(&half)
        );
    }

    /// Two equal strips: the overlaps' transfer is the supermodes' sin² within 2 % of the input,
    /// all but a percent crosses, and coupled-mode theory's largest share is 1 to round-off. An
    /// unequal pair: incomplete transfer, the overlaps' peak within 0.03 of coupled-mode theory's.
    #[test]
    fn an_unequal_pair_transfers_less() {
        let equal = unequal(0.5, 0.2).unwrap();
        // the graded grid isn't mirror symmetric: the two strips alone differ in the sixth decimal
        close(equal.n1, equal.n2, 1e-5);
        close(equal.largest(), 1.0, 1e-6);
        let (_, s) = equal.delta_and_s();
        let p = pair(0.2, 1.55).unwrap();
        close(s, PI * (p.even - p.odd) / 1.55, 0.01);
        for k in 0..=40 {
            let z = 2.0 * p.cross_over(1.55) * f64::from(k) / 40.0;
            let want = (s * z).sin().powi(2);
            assert!(
                (equal.power(1, z) - want).abs() < 0.02,
                "{z}: {}",
                equal.power(1, z)
            );
        }
        assert!(equal.power(1, PI / (2.0 * s)) > 0.98);
        let data = mismatch(480.0, 200.0).unwrap();
        let value = |label: &str| {
            data.figures
                .iter()
                .find(|f| f.label.starts_with(label))
                .and_then(|f| f.value)
                .unwrap()
        };
        let theory = value("Largest crossed, $\\kappa");
        let overlaps = value("Largest crossed, the modes");
        assert!(theory < 0.9 && theory > 0.1, "{theory}");
        assert!(
            (overlaps - theory).abs() < 0.03,
            "{overlaps} against {theory}"
        );
    }
}
