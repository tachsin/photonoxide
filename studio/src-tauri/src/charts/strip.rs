//! The strip waveguide's charts: a silicon strip's modes, solved on the strip waveguide device's
//! own cross-section (`photonoxide::device::StripWaveguide`: 220 nm SOI, Li's silicon and
//! Malitson's silica at the chart's wavelength, cells fitted to the strip with 1 µm of oxide
//! beyond it), so that a lesson and the device's page can't disagree; and the approximations a
//! strip is designed with, the effective index method (`mode::eim`) and Marcatili's
//! (`mode::marcatili`), against it.
//!
//! The device solves its TE-like mode on a quarter of the cross-section: an electric wall on
//! the vertical plane through the strip and a magnetic wall on the core's mid-plane. The other
//! three pairs of walls hold the other symmetries, so the four quarters together give every mode
//! of the first vertical order: TE₀ and TE₂ (electric, magnetic), TE₁ and TE₃ (magnetic,
//! magnetic), TM₀ and TM₂ (magnetic, electric), TM₁ and TM₃ (electric, electric). A mode is
//! kept when it is guided, its effective index above the oxide's (`VectorMode::is_guided`);
//! below it the closed window holds modes of its own.
//!
//! The solves are cached by their geometry, wavelength and grid, so a chart asked again with only
//! its marker moved is drawn at once.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use photonoxide::circuit::Fidelity;
use photonoxide::circuit::components::Dispersion;
use photonoxide::device::{Mesh, Scheme, StripWaveguide, Values};
use photonoxide::material::{silica, silicon};
use photonoxide::mode::Polarization;
use photonoxide::mode::eim::Ridge;
use photonoxide::mode::marcatili::{Family, Rectangle};
use photonoxide::mode::vector::{Boundaries, Boundary, CrossSection, VectorMode, modes};
use photonoxide::raster::Raster;
use photonoxide::units::{Length, Wavelength};

use super::{ChartData, Curve, FieldMap, figure, invalid, sig};

/// The cladding beyond the strip in the solves, µm: the device's default.
const MARGIN: f64 = 1.0;
/// The cells of the sweeps, µm: on them Hadley's equations put the 500 × 220 nm strip's index
/// within 2e-4 of its value on 5 nm cells.
pub const SWEEP_STEP: f64 = 0.02;
/// The cells of one cross-section's fields, µm: the device's default.
pub const FIELD_STEP: f64 = 0.01;
/// The share of a field picture's cells below the top of its colour scale.
const BRIGHTEST: f64 = 0.995;
/// The palette's grey, for reference lines.
const GREY: usize = 7;

/// The four quarters: the walls on the vertical plane (west) and the mid-plane (south), the
/// lateral orders of the two modes each holds first, and their polarization.
const QUARTERS: [(Boundary, Boundary, [usize; 2], Polarization); 4] = [
    (
        Boundary::ElectricWall,
        Boundary::MagneticWall,
        [0, 2],
        Polarization::Te,
    ),
    (
        Boundary::MagneticWall,
        Boundary::MagneticWall,
        [1, 3],
        Polarization::Te,
    ),
    (
        Boundary::MagneticWall,
        Boundary::ElectricWall,
        [0, 2],
        Polarization::Tm,
    ),
    (
        Boundary::ElectricWall,
        Boundary::ElectricWall,
        [1, 3],
        Polarization::Tm,
    ),
];

/// The strip waveguide device on cells fitted `step` µm apart, by `scheme`.
fn device(step: f64, scheme: Scheme) -> StripWaveguide {
    StripWaveguide::new().with_mesh(Mesh::fitted(step, MARGIN, scheme))
}

/// The device's values for a strip `width` × `thickness` µm at `l` µm.
fn values(d: &StripWaveguide, width: f64, thickness: f64, l: f64) -> photonoxide::Result<Values> {
    Values::new(
        d,
        &[
            ("width", width),
            ("thickness", thickness),
            ("wavelength", l),
        ],
    )
}

/// The device's own cross-section of a strip `width` × `thickness` µm at `l` µm on cells
/// `step` µm apart: the quarter its TE-like mode is solved on.
fn section(width: f64, thickness: f64, l: f64, step: f64) -> photonoxide::Result<CrossSection> {
    let d = device(step, Scheme::Standard);
    d.cross_section(&values(&d, width, thickness, l)?, Wavelength::um(l)?)
}

/// The same quarter with `walls` (west, south).
pub fn quarter(
    width: f64,
    thickness: f64,
    l: f64,
    step: f64,
    walls: (Boundary, Boundary),
) -> photonoxide::Result<CrossSection> {
    section(width, thickness, l, step)?.with_boundaries(Boundaries {
        west: walls.0,
        south: walls.1,
        ..Boundaries::default()
    })
}

/// A guided mode of the strip.
#[derive(Clone, Debug)]
pub struct Guided {
    /// The polarization its quarter holds.
    pub polarization: Polarization,
    /// Its lateral order: the zeros of its main field across the width.
    pub order: usize,
    pub effective_index: f64,
    pub te_fraction: f64,
}

impl Guided {
    /// `TE₀`, `TM₁`, ...
    pub fn name(&self) -> String {
        name(self.polarization, self.order)
    }
}

/// `TE₀`, `TM₁`, ...
pub fn name(p: Polarization, order: usize) -> String {
    const SUB: [char; 10] = ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'];
    let p = match p {
        Polarization::Te => "TE",
        Polarization::Tm => "TM",
    };
    format!("{p}{}", SUB[order.min(9)])
}

/// The guided modes of quarter `q`, at most two, highest index first, each with its mode and the
/// cross-section it was found on.
fn quarter_modes(
    width: f64,
    thickness: f64,
    l: f64,
    step: f64,
    q: usize,
) -> photonoxide::Result<Vec<(Guided, VectorMode, CrossSection)>> {
    let (west, south, orders, polarization) = QUARTERS[q];
    let cs = quarter(width, thickness, l, step, (west, south))?;
    let lam = Wavelength::um(l)?;
    let surrounding = cs.surrounding_index(lam)?;
    let found = modes(&cs, lam, 2, None)?;
    Ok(found
        .into_iter()
        .filter(|m| m.is_guided(surrounding))
        .zip(orders)
        .map(|(m, order)| {
            (
                Guided {
                    polarization,
                    order,
                    effective_index: m.effective_index().re,
                    te_fraction: m.te_fraction(),
                },
                m,
                cs.clone(),
            )
        })
        .collect())
}

type Key = (i64, i64, i64, i64);

fn key(a: f64, b: f64, c: f64, d: f64) -> Key {
    // to a picometre: a slider's values are far coarser
    let pm = |v: f64| (v * 1e6).round() as i64;
    (pm(a), pm(b), pm(c), pm(d))
}

/// A small cache: cleared when it grows past `LIMIT` entries, so it never holds much.
struct Cache<T> {
    map: Mutex<HashMap<Key, T>>,
}

const LIMIT: usize = 600;

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

/// `f` of each of `items`, on up to 12 threads, in their order.
fn parallel<T: Sync, R: Send>(
    items: &[T],
    f: impl Fn(&T) -> photonoxide::Result<R> + Sync,
) -> photonoxide::Result<Vec<R>> {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(1, 12);
    let chunk = items.len().div_ceil(threads).max(1);
    let f = &f;
    let parts: Vec<photonoxide::Result<Vec<R>>> = std::thread::scope(|s| {
        let handles: Vec<_> = items
            .chunks(chunk)
            .map(|part| s.spawn(move || part.iter().map(f).collect()))
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err(invalid("strip chart", "a solve panicked")))
            })
            .collect()
    });
    let mut out = Vec::with_capacity(items.len());
    for p in parts {
        out.extend(p?);
    }
    Ok(out)
}

/// Every guided mode of the first vertical order of a strip `width` × `thickness` µm at `l` µm,
/// on cells `step` µm apart, highest index first: the four quarters solved side by side.
pub fn guided(width: f64, thickness: f64, l: f64, step: f64) -> photonoxide::Result<Vec<Guided>> {
    static CACHE: OnceLock<Cache<Vec<Guided>>> = OnceLock::new();
    CACHE
        .get_or_init(Cache::new)
        .get_or(key(width, thickness, l, step), || {
            let quarters: Vec<usize> = (0..QUARTERS.len()).collect();
            let found = parallel(&quarters, |&q| quarter_modes(width, thickness, l, step, q))?;
            let mut all: Vec<Guided> = found.into_iter().flatten().map(|(g, _, _)| g).collect();
            all.sort_by(|a, b| b.effective_index.total_cmp(&a.effective_index));
            Ok(all)
        })
}

/// The indices of silicon and silica at `l` µm, as the device's stack has them.
pub fn indices(l: f64) -> photonoxide::Result<(f64, f64)> {
    let lam = Wavelength::um(l)?;
    Ok((
        silicon().refractive_index(lam)?.re,
        silica().refractive_index(lam)?.re,
    ))
}

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: Option<usize>) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour,
    }
}

/// A grey dashed level `y` from `from` to `to`, in 20 steps.
fn level(label: &str, y: f64, from: f64, to: f64) -> Curve {
    let points = (0..=20)
        .map(|k| [from + (to - from) * f64::from(k) / 20.0, y])
        .collect();
    curve(label, points, true, Some(GREY))
}

/// From `from` to `to` in steps of `step`, both ends included.
fn span(from: f64, to: f64, step: f64) -> Vec<f64> {
    let n = ((to - from) / step).round() as usize;
    // to a picometre, so that 0.4 + 20 × 0.01 is 0.6 and inside a range that ends there
    (0..=n)
        .map(|k| ((from + step * k as f64) * 1e6).round() / 1e6)
        .collect()
}

/// The strip's outline, a closed line, µm.
fn outline(width: f64, thickness: f64) -> Vec<[f64; 2]> {
    let (a, b) = (width / 2.0, thickness / 2.0);
    vec![[-a, -b], [a, -b], [a, b], [-a, b], [-a, -b]]
}

// ---- the fields ----

/// One mode's fields as pictures: |E_x|, |E_y| and |E_z| over the strip and the oxide around
/// it, each brightest where only half a per cent of its cells are brighter; and their peaks.
#[derive(Clone, Debug)]
pub struct Pictures {
    pub mode: Guided,
    /// How many guided modes the strip has.
    pub count: usize,
    /// |E_x|, |E_y|, |E_z|.
    pub maps: [Raster; 3],
    /// Each component's peak |E|.
    pub peaks: [f64; 3],
    /// The grid, as the device states it.
    pub grid: String,
}

/// The `n`th guided mode (1 the highest index) of a strip, or its last when it has fewer.
pub fn pictures(width: f64, thickness: f64, l: f64, n: usize) -> photonoxide::Result<Pictures> {
    static CACHE: OnceLock<Cache<Pictures>> = OnceLock::new();
    CACHE
        .get_or_init(Cache::new)
        .get_or(key(width, thickness, l, n as f64), || {
            let quarters: Vec<usize> = (0..QUARTERS.len()).collect();
            let found = parallel(&quarters, |&q| {
                quarter_modes(width, thickness, l, FIELD_STEP, q)
            })?;
            let mut all: Vec<(Guided, VectorMode, CrossSection)> =
                found.into_iter().flatten().collect();
            all.sort_by(|a, b| b.0.effective_index.total_cmp(&a.0.effective_index));
            let count = all.len();
            let (mode, vector, cs) = all
                .into_iter()
                .nth(n.clamp(1, count.max(1)) - 1)
                .ok_or_else(|| invalid("strip chart", "the strip guides no mode"))?;
            let fields = vector.fields(&cs)?;
            // the picture: the strip and the oxide around it, cells of about half the solver's
            let (x, y) = (cs.x(), cs.y());
            let half = [
                (width / 2.0 + 0.45).min(x[x.len() - 1]),
                (thickness / 2.0 + 0.35).min(y[y.len() - 1]),
            ];
            let pixel = FIELD_STEP / 2.0;
            let (nx, ny) = (
                (2.0 * half[0] / pixel).round() as usize,
                (2.0 * half[1] / pixel).round() as usize,
            );
            // the solver's cell holding |u| along nodes `v`
            let cell = |v: &[f64], u: f64| {
                v.partition_point(|&node| node <= u.abs())
                    .saturating_sub(1)
                    .min(v.len() - 2)
            };
            let mut values = [
                vec![0.0f64; nx * ny],
                vec![0.0; nx * ny],
                vec![0.0; nx * ny],
            ];
            for j in 0..ny {
                let yc = -half[1] + (j as f64 + 0.5) * 2.0 * half[1] / ny as f64;
                let cj = cell(y, yc);
                for i in 0..nx {
                    let xc = -half[0] + (i as f64 + 0.5) * 2.0 * half[0] / nx as f64;
                    let e = fields.e(cell(x, xc), cj);
                    for c in 0..3 {
                        values[c][j * nx + i] = e[c].norm();
                    }
                }
            }
            let peaks = values
                .clone()
                .map(|v| v.iter().copied().fold(0.0, f64::max));
            // each picture's colours run to the value its brightest half per cent of cells
            // exceed: the field's peaks at the corners, a few cells, would leave the rest dark
            let scales = values.clone().map(|mut v| {
                v.sort_by(f64::total_cmp);
                v[(v.len() as f64 * BRIGHTEST) as usize].max(f64::MIN_POSITIVE)
            });
            let maps = [0, 1, 2].map(|c| Raster {
                nx,
                ny,
                x0: -half[0],
                x1: half[0],
                y0: -half[1],
                y1: half[1],
                // three decimals are all a picture shows
                values: values[c]
                    .iter()
                    .map(|v| ((v / scales[c]).min(1.0) * 1e3).round() as f32 / 1e3)
                    .collect(),
            });
            Ok(Pictures {
                mode,
                count,
                maps,
                peaks,
                grid: format!(
                    "cells of {:.1} × {:.1} nm",
                    1e3 * (x[1] - x[0]),
                    1e3 * (y[1] - y[0])
                ),
            })
        })
}

/// The chart of a strip's mode: its three electric field components across the cross-section.
pub fn fields(
    width_nm: f64,
    thickness_nm: f64,
    l: f64,
    n: usize,
) -> photonoxide::Result<ChartData> {
    let (w, t) = (width_nm * 1e-3, thickness_nm * 1e-3);
    let p = pictures(w, t, l, n)?;
    let major = if p.mode.polarization == Polarization::Te {
        0
    } else {
        1
    };
    let names = ["$|E_x|$", "$|E_y|$", "$|E_z|$"];
    let relative = |c: usize| p.peaks[c] / p.peaks[major];
    let mut maps = Vec::new();
    for c in 0..3 {
        maps.push(FieldMap {
            label: if c == major {
                format!("{}, the main field", names[c])
            } else {
                format!(
                    "{}, peak {} of {}'s",
                    names[c],
                    sig(relative(c), 2),
                    names[major]
                )
            },
            raster: p.maps[c].clone(),
            outline: vec![outline(w, t)],
        });
    }
    let shown = n.min(p.count);
    let mut figures = vec![
        figure("Mode", shown as f64, 1, "", ""),
        figure("$n_\\text{eff}$", p.mode.effective_index, 6, "", ""),
        figure(
            "TE fraction",
            p.mode.te_fraction,
            3,
            "",
            "the share of the transverse H in H_y",
        ),
        figure(
            "Guided modes",
            p.count as f64,
            1,
            "",
            "first vertical order",
        ),
    ];
    figures[0].text = p.mode.name();
    figures[0].note = match p.mode.polarization {
        Polarization::Te => "TE-like: E mostly across the width".to_owned(),
        Polarization::Tm => "TM-like: E mostly up the height".to_owned(),
    };
    let ez = 2;
    figures.push(figure(
        "Peak $|E_z|$ / main",
        relative(ez),
        3,
        "",
        "the longitudinal field, largest at the faces the main field crosses",
    ));
    Ok(ChartData {
        x_label: "across the strip, x".to_owned(),
        x_length: true,
        y_label: "up the stack, y".to_owned(),
        y_log: false,
        y_range: None,
        series: Vec::new(),
        marker: None,
        figures,
        note: format!(
            "{} of a {width_nm:.0} × {thickness_nm:.0} nm silicon strip in oxide at {l} µm{}, by the full-vector solver on the strip waveguide device's cross-section, its four quarters solved with mirror walls; {} with 1 µm of oxide beyond the strip; each picture's colours run to the value only half a per cent of its cells exceed, so that the few cells at the corners don't leave the rest dark.",
            p.mode.name(),
            if n > p.count {
                format!(" (the strip guides {} modes)", p.count)
            } else {
                String::new()
            },
            p.grid
        ),
        maps,
    })
}

// ---- the modes against the width ----

/// The widths the index chart is drawn at, µm.
fn neff_widths() -> Vec<f64> {
    span(0.2, 1.2, 0.025)
}

/// The width, µm, from which a strip `thickness` µm thick guides TE₁ at `l` µm: bisected to a
/// nanometre between `below` and `above`.
fn te1_appears(thickness: f64, l: f64, below: f64, above: f64) -> photonoxide::Result<f64> {
    static CACHE: OnceLock<Cache<f64>> = OnceLock::new();
    CACHE
        .get_or_init(Cache::new)
        .get_or(key(thickness, l, below, above), || {
            let (mut lo, mut hi) = (below, above);
            while hi - lo > 1e-3 {
                let mid = 0.5 * (lo + hi);
                if quarter_modes(mid, thickness, l, SWEEP_STEP, 1)?.is_empty() {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            Ok(0.5 * (lo + hi))
        })
}

/// Every guided mode's effective index against the strip's width.
pub fn neff(thickness_nm: f64, l: f64, width_nm: f64) -> photonoxide::Result<ChartData> {
    let t = thickness_nm * 1e-3;
    let widths = neff_widths();
    let sweep = parallel(&widths, |&w| guided(w, t, l, SWEEP_STEP))?;
    let (_, oxide) = indices(l)?;
    let mut series = Vec::new();
    for order in 0..4 {
        for pol in [Polarization::Te, Polarization::Tm] {
            let points: Vec<[f64; 2]> = widths
                .iter()
                .zip(&sweep)
                .filter_map(|(&w, modes)| {
                    modes
                        .iter()
                        .find(|g| g.polarization == pol && g.order == order)
                        .map(|g| [w, g.effective_index])
                })
                .collect();
            if !points.is_empty() {
                series.push(curve(
                    &name(pol, order),
                    points,
                    pol == Polarization::Tm,
                    Some(order),
                ));
            }
        }
    }
    series.push(level("oxide", oxide, widths[0], widths[widths.len() - 1]));
    let at = guided(width_nm * 1e-3, t, l, SWEEP_STEP)?;
    let mut figures = Vec::new();
    // where TE₁ is born: between the last width without it and the first with it
    let has_te1 = |m: &Vec<Guided>| {
        m.iter()
            .any(|g| g.polarization == Polarization::Te && g.order == 1)
    };
    if let Some(k) = sweep.iter().position(has_te1).filter(|&k| k > 0) {
        let w1 = te1_appears(t, l, widths[k - 1], widths[k])?;
        figures.push(figure(
            "One TE-like mode below",
            w1 * 1e3,
            3,
            "nm",
            "where TE₁'s index passes the oxide's",
        ));
    }
    figures.push(figure(
        "Guided modes",
        at.len() as f64,
        1,
        "",
        &format!("at {width_nm:.0} nm"),
    ));
    for g in at.iter().take(4) {
        figures.push(figure(
            &format!("$n_\\text{{eff}}$, {}", g.name()),
            g.effective_index,
            5,
            "",
            &format!("TE fraction {}", sig(g.te_fraction, 2)),
        ));
    }
    Ok(ChartData {
        x_label: "strip width w".to_owned(),
        x_length: true,
        y_label: "effective index n_eff".to_owned(),
        y_log: false,
        y_range: Some([oxide - 0.05, indices(l)?.0.min(3.0)]),
        series,
        marker: Some(width_nm * 1e-3),
        figures,
        note: format!(
            "The guided modes of a silicon strip {thickness_nm:.0} nm thick in oxide at {l} µm, TE-like solid, TM-like dashed, by the full-vector solver on the strip waveguide device's cross-section (its four quarters), cells fitted about 20 nm apart with 1 µm of oxide beyond the strip. Near its cutoff a mode reaches the window's walls, which lower its index a little: on 2 µm of oxide TE₁ of a 500 nm strip is 0.004 higher, and its cutoff about 2 nm narrower."
        ),
        maps: Vec::new(),
    })
}

// ---- the approximations against the full-vector mode ----

/// The widths the approximations are compared at, µm.
fn model_widths() -> Vec<f64> {
    span(0.25, 1.0, 0.05)
}

/// The TE-like mode's index by Hadley's equations on the device's quarter, cells `step` µm.
fn hadley_index(width: f64, thickness: f64, l: f64, step: f64) -> photonoxide::Result<f64> {
    static CACHE: OnceLock<Cache<f64>> = OnceLock::new();
    CACHE
        .get_or_init(Cache::new)
        .get_or(key(width, thickness, l, step), || {
            let lam = Wavelength::um(l)?;
            let cs = section(width, thickness, l, step)?;
            let mode = photonoxide::mode::hadley::modes(&cs, lam, 1, None)?.remove(0);
            Ok(mode.effective_index().re)
        })
}

/// The effective index method's TE-like index of a strip of silicon in oxide at `l` µm.
fn eim_index(width: f64, thickness: f64, l: f64) -> photonoxide::Result<f64> {
    let (si, ox) = indices(l)?;
    Ok(
        Ridge::strip(ox, si, ox, Length::um(thickness), Length::um(width))?
            .mode(Polarization::Te, Wavelength::um(l)?)?
            .effective_index,
    )
}

/// The device's analytic model (its fit) at its band's centre, where it holds.
fn fitted(width: f64, thickness: f64, l: f64) -> Option<Dispersion> {
    let d = StripWaveguide::new();
    let v = values(&d, width, thickness, l).ok()?;
    d.dispersion(&v, Fidelity::Analytic).ok().map(|(d, _)| d)
}

/// The TE-like mode's index by each approximation and by the full-vector solver.
pub fn models(thickness_nm: f64, l: f64, width_nm: f64) -> photonoxide::Result<ChartData> {
    let t = thickness_nm * 1e-3;
    let lam = Wavelength::um(l)?;
    let (si, ox) = indices(l)?;
    let widths = model_widths();
    let full = parallel(&widths, |&w| hadley_index(w, t, l, SWEEP_STEP))?;
    let rect = |w: f64| Rectangle::uniform(si, ox, Length::um(w), Length::um(t));
    let mut eim = Vec::new();
    let mut marcatili = Vec::new();
    let mut closed = Vec::new();
    let mut fit = Vec::new();
    for &w in &widths {
        eim.push([w, eim_index(w, t, l)?]);
        if let Some(n) = rect(w)?.mode(Family::Ex, 1, 1, lam) {
            marcatili.push([w, n]);
        }
        if let Some(n) = rect(w)?.closed_form(Family::Ex, 1, 1, lam) {
            closed.push([w, n]);
        }
    }
    for w in span(0.4, 0.6, 0.01) {
        if let Some(d) = fitted(w, t, l) {
            fit.push([w, d.effective_index_at(lam)]);
        }
    }
    let solved: Vec<[f64; 2]> = widths.iter().zip(&full).map(|(&w, &n)| [w, n]).collect();
    let w = width_nm * 1e-3;
    let reference = hadley_index(w, t, l, SWEEP_STEP)?;
    let versus = |n: f64| {
        format!(
            "{}{} ({}{} %)",
            if n >= reference { "+" } else { "−" },
            sig((n - reference).abs(), 2),
            if n >= reference { "+" } else { "−" },
            sig(100.0 * (n - reference).abs() / reference, 2)
        )
    };
    let mut figures = vec![figure(
        "Full-vector, Hadley's equations",
        reference,
        5,
        "",
        "the reference",
    )];
    let e = eim_index(w, t, l)?;
    figures.push(figure("Effective index method", e, 5, "", &versus(e)));
    if let Some(n) = rect(w)?.mode(Family::Ex, 1, 1, lam) {
        figures.push(figure("Marcatili", n, 5, "", &versus(n)));
    }
    if let Some(n) = rect(w)?.closed_form(Family::Ex, 1, 1, lam) {
        figures.push(figure("Marcatili's closed form", n, 5, "", &versus(n)));
    }
    if let Some(d) = fitted(w, t, l) {
        let n = d.effective_index_at(lam);
        figures.push(figure("The device's fit", n, 5, "", &versus(n)));
    }
    let mut series = vec![
        curve("full-vector (Hadley's equations)", solved, false, Some(0)),
        curve("effective index method", eim, true, Some(1)),
        curve("Marcatili", marcatili, true, Some(2)),
        curve("Marcatili's closed form", closed, true, Some(3)),
    ];
    if !fit.is_empty() {
        series.push(curve("the device's analytic fit", fit, false, Some(4)));
    }
    Ok(ChartData {
        x_label: "strip width w".to_owned(),
        x_length: true,
        y_label: "TE-like effective index n_eff".to_owned(),
        y_log: false,
        y_range: None,
        series,
        marker: Some(w),
        figures,
        note: format!(
            "A silicon strip {thickness_nm:.0} nm thick in oxide at {l} µm (silicon {}, oxide {}, the catalogue's), its TE-like mode: by Hadley's equations on the strip waveguide device's quarter, cells fitted about 20 nm apart with 1 µm of oxide beyond the strip, and by the approximations, which have no grid. The device's analytic fit holds from 400 to 600 nm wide and 200 to 240 nm thick, from 1.5 to 1.6 µm.",
            sig(si, 5),
            sig(ox, 5)
        ),
        maps: Vec::new(),
    })
}

// ---- the corners and the grid ----

/// The cells the convergence chart solves on, µm.
pub const CORNER_STEPS: [f64; 11] = [
    0.04, 0.036, 0.033, 0.03, 0.027, 0.024, 0.021, 0.018, 0.015, 0.0125, 0.01,
];

/// The TE-like index of the device's quarter on cells about `step` µm apart by `scheme`, and
/// the cells' size h = √(Δx Δy), µm.
fn on_grid(
    width: f64,
    thickness: f64,
    step: f64,
    scheme: Scheme,
) -> photonoxide::Result<(f64, f64)> {
    static CACHE: OnceLock<Cache<(f64, f64)>> = OnceLock::new();
    let s = if scheme == Scheme::Hadley { 1.0 } else { 0.0 };
    CACHE
        .get_or_init(Cache::new)
        .get_or(key(width, thickness, step, s), || {
            let l = 1.55;
            let lam = Wavelength::um(l)?;
            let cs = section(width, thickness, l, step)?;
            let h = ((cs.x()[1] - cs.x()[0]) * (cs.y()[1] - cs.y()[0])).sqrt();
            let mode = match scheme {
                Scheme::Standard => modes(&cs, lam, 1, None)?.remove(0),
                Scheme::Hadley => photonoxide::mode::hadley::modes(&cs, lam, 1, None)?.remove(0),
            };
            Ok((mode.effective_index().re, h))
        })
}

/// The order p of a sequence converging as h^p: from three values on cells h₁ > h₂ > h₃.
fn order(points: &[[f64; 2]]) -> f64 {
    let [a, b, c] = [points[0], points[1], points[2]];
    // e(h) = C h^p: (n₁ − n₂)/(n₂ − n₃) = (h₁^p − h₂^p)/(h₂^p − h₃^p); bisected for p
    let target = (a[1] - b[1]) / (b[1] - c[1]);
    let f = |p: f64| (a[0].powf(p) - b[0].powf(p)) / (b[0].powf(p) - c[0].powf(p)) - target;
    let (mut lo, mut hi) = (0.05, 6.0);
    if !(target.is_finite() && target > 0.0) || f(lo) * f(hi) > 0.0 {
        return f64::NAN;
    }
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if f(lo) * f(mid) <= 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    0.5 * (lo + hi)
}

/// The strip's TE-like index against the cells' size, by the standard scheme and by Hadley's
/// equations, at 1.55 µm.
pub fn corners(width_nm: f64, thickness_nm: f64) -> photonoxide::Result<ChartData> {
    let (w, t) = (width_nm * 1e-3, thickness_nm * 1e-3);
    let jobs: Vec<(f64, Scheme)> = CORNER_STEPS
        .iter()
        .flat_map(|&s| [(s, Scheme::Standard), (s, Scheme::Hadley)])
        .collect();
    let solved = parallel(&jobs, |&(s, scheme)| on_grid(w, t, s, scheme))?;
    let pick = |scheme: Scheme| -> Vec<[f64; 2]> {
        let mut v: Vec<[f64; 2]> = jobs
            .iter()
            .zip(&solved)
            .filter(|((_, s), _)| *s == scheme)
            .map(|(_, &(n, h))| [h, n])
            .collect();
        v.sort_by(|a, b| b[0].total_cmp(&a[0]));
        v
    };
    let (standard, hadley) = (pick(Scheme::Standard), pick(Scheme::Hadley));
    let k = hadley.len();
    // Richardson's extrapolation of Hadley's equations from the two finest grids at their
    // measured order
    let p_hadley = order(&hadley[k - 3..]);
    let p = if p_hadley.is_finite() { p_hadley } else { 2.0 };
    let ([h1, n1], [h2, n2]) = (hadley[k - 2], hadley[k - 1]);
    let limit = n2 + (n2 - n1) / ((h1 / h2).powf(p) - 1.0);
    let finest = standard[k - 1];
    let figures = vec![
        figure(
            "Standard scheme, finest",
            finest[1],
            6,
            "",
            &format!("on {} nm cells", sig(finest[0] * 1e3, 3)),
        ),
        figure(
            "Hadley's equations, finest",
            hadley[k - 1][1],
            6,
            "",
            &format!("on {} nm cells", sig(hadley[k - 1][0] * 1e3, 3)),
        ),
        figure(
            "Extrapolated",
            limit,
            6,
            "",
            "Hadley's two finest, at their order",
        ),
        figure(
            "Order, Hadley's",
            p_hadley,
            2,
            "",
            "from the three finest grids",
        ),
        figure(
            "Standard's error, finest",
            finest[1] - limit,
            2,
            "",
            "against the extrapolation",
        ),
        figure(
            "Hadley's error, coarsest",
            hadley[0][1] - limit,
            2,
            "",
            &format!("on {} nm cells", sig(hadley[0][0] * 1e3, 3)),
        ),
    ];
    let ends = [standard[0][0], 0.0];
    Ok(ChartData {
        x_label: "cell size h".to_owned(),
        x_length: true,
        y_label: "TE-like effective index n_eff".to_owned(),
        y_log: false,
        y_range: None,
        series: vec![
            curve("standard scheme", standard, false, Some(0)),
            curve("Hadley's equations", hadley, false, Some(1)),
            level("extrapolated", limit, ends[1], ends[0]),
        ],
        marker: None,
        figures,
        note: format!(
            "A {width_nm:.0} × {thickness_nm:.0} nm silicon strip in oxide at 1.55 µm, its TE-like mode on the strip waveguide device's quarter, cells fitted so that the strip's faces lie on nodes, from about 40 to 10 nm (drawn at h = √(Δx Δy)), 1 µm of oxide beyond the strip."
        ),
        maps: Vec::new(),
    })
}

// ---- the group index ----

/// The widths the group index is drawn at, µm.
fn group_widths() -> Vec<f64> {
    span(0.3, 0.9, 0.05)
}

/// The device's 3D model of a strip: n_eff, n_g and D at `l` µm, by Hadley's equations on cells
/// `step` µm apart at five wavelengths 25 nm apart.
fn three_d(width: f64, thickness: f64, l: f64, step: f64) -> photonoxide::Result<Dispersion> {
    static CACHE: OnceLock<Cache<Dispersion>> = OnceLock::new();
    CACHE
        .get_or_init(Cache::new)
        .get_or(key(width, thickness, l, step), || {
            let d = device(step, Scheme::Hadley);
            Ok(
                d.dispersion(&values(&d, width, thickness, l)?, Fidelity::ThreeD)?
                    .0,
            )
        })
}

/// The effective index method's group index: its index at l ± 10 nm, the materials' at each.
fn eim_group(width: f64, thickness: f64, l: f64) -> photonoxide::Result<f64> {
    let dl = 0.01;
    let (minus, plus) = (
        eim_index(width, thickness, l - dl)?,
        eim_index(width, thickness, l + dl)?,
    );
    Ok(eim_index(width, thickness, l)? - l * (plus - minus) / (2.0 * dl))
}

/// The TE-like mode's group and effective indices against the strip's width.
pub fn group_index(thickness_nm: f64, l: f64, width_nm: f64) -> photonoxide::Result<ChartData> {
    let t = thickness_nm * 1e-3;
    let lam = Wavelength::um(l)?;
    let widths = group_widths();
    let solved = parallel(&widths, |&w| three_d(w, t, l, SWEEP_STEP))?;
    let silicon_ng = silicon().group_index(lam)?;
    let (silicon_n, _) = indices(l)?;
    let pts = |f: &dyn Fn(&Dispersion) -> f64| -> Vec<[f64; 2]> {
        widths
            .iter()
            .zip(&solved)
            .map(|(&w, d)| [w, f(d)])
            .collect()
    };
    let mut eim = Vec::new();
    for &w in &widths {
        eim.push([w, eim_group(w, t, l)?]);
    }
    let fit: Vec<[f64; 2]> = span(0.4, 0.6, 0.01)
        .into_iter()
        .filter_map(|w| fitted(w, t, l).map(|d| [w, d.group_index_at(lam)]))
        .collect();
    let ends = [widths[0], widths[widths.len() - 1]];
    let mut series = vec![
        curve("n_g, full-vector", pts(&|d| d.group_index), false, Some(0)),
        curve(
            "n_eff, full-vector",
            pts(&|d| d.effective_index),
            false,
            Some(1),
        ),
        curve("n_g, effective index method", eim, true, Some(0)),
        level("n_g of bulk silicon", silicon_ng, ends[0], ends[1]),
    ];
    if !fit.is_empty() {
        series.push(curve("n_g, the device's analytic fit", fit, true, Some(4)));
    }
    let w = width_nm * 1e-3;
    let d = three_d(w, t, l, SWEEP_STEP)?;
    let e = eim_group(w, t, l)?;
    let figures = vec![
        figure("$n_\\text{eff}$", d.effective_index, 5, "", "full-vector"),
        figure("$n_g$", d.group_index, 4, "", "full-vector"),
        figure("$D$", d.dispersion, 3, "ps/(nm km)", "full-vector"),
        figure(
            "$n_g$, effective index method",
            e,
            4,
            "",
            &format!(
                "{} % from the full-vector",
                sig(100.0 * (e - d.group_index) / d.group_index, 2)
            ),
        ),
        figure(
            "Bulk silicon",
            silicon_ng,
            4,
            "",
            &format!("its $n_g$; its $n$ is {}", sig(silicon_n, 4)),
        ),
    ];
    Ok(ChartData {
        x_label: "strip width w".to_owned(),
        x_length: true,
        y_label: "index".to_owned(),
        y_log: false,
        y_range: None,
        series,
        marker: Some(w),
        figures,
        note: format!(
            "A silicon strip {thickness_nm:.0} nm thick in oxide at {l} µm, its TE-like mode by the strip waveguide device's 3D model: Hadley's equations at five wavelengths 25 nm apart, the materials' dispersion included (Li's silicon, Malitson's silica), on cells fitted about 20 nm apart with 1 µm of oxide beyond the strip. The effective index method's from its index at ±10 nm, exact slabs."
        ),
        maps: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::charts::Figure;

    fn fig<'a>(data: &'a ChartData, label: &str) -> &'a Figure {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .unwrap_or_else(|| panic!("no figure {label}"))
    }

    /// The device's own cross-section is what the charts solve: its TE-like quarter is the
    /// first, and the fundamental mode found there is the device's 3D model's at that grid.
    #[test]
    fn the_charts_solve_the_devices_cross_section() {
        let d = device(SWEEP_STEP, Scheme::Standard);
        let v = values(&d, 0.5, 0.22, 1.55).unwrap();
        let own = d.cross_section(&v, Wavelength::um(1.55).unwrap()).unwrap();
        let ours = quarter(0.5, 0.22, 1.55, SWEEP_STEP, (QUARTERS[0].0, QUARTERS[0].1)).unwrap();
        assert_eq!(own.x(), ours.x());
        assert_eq!(own.y(), ours.y());
        assert_eq!(own.boundaries(), ours.boundaries());
        let g = guided(0.5, 0.22, 1.55, SWEEP_STEP).unwrap();
        let lam = Wavelength::um(1.55).unwrap();
        let first = modes(&own, lam, 1, None).unwrap()[0].effective_index().re;
        assert_eq!(g[0].name(), "TE₀");
        assert_eq!(g[0].effective_index, first);
        // Hadley's equations on the same quarter are the device's 3D model's at its centre
        let h = hadley_index(0.5, 0.22, 1.55, SWEEP_STEP).unwrap();
        let three = three_d(0.5, 0.22, 1.55, SWEEP_STEP).unwrap();
        assert!(
            (h - three.effective_index).abs() < 1e-6,
            "{h} {}",
            three.effective_index
        );
    }

    /// The quarters' walls sort the modes by symmetry: the TE-like quarters' modes are TE-like
    /// and the TM-like quarters' TM-like, the 500 × 220 nm strip guides TE₀ and TM₀ (and TE₁
    /// near its cutoff), and TE₀'s index is the strip_waveguide example's within the grid's
    /// error, the materials' differing.
    #[test]
    fn the_quarters_hold_the_modes_of_their_symmetry() {
        let g = guided(0.5, 0.22, 1.55, SWEEP_STEP).unwrap();
        for m in &g {
            match m.polarization {
                Polarization::Te => assert!(m.te_fraction > 0.5, "{m:?}"),
                Polarization::Tm => assert!(m.te_fraction < 0.5, "{m:?}"),
            }
        }
        let names: Vec<String> = g.iter().map(Guided::name).collect();
        assert_eq!(&names[..2], ["TE₀", "TM₀"]);
        // the standard scheme: 2.447067 on 20 nm cells with 3.473 and 1.444; Li's silicon is
        // 3.4757 at 1.55 µm, which raises it by about 0.003
        assert!(
            (g[0].effective_index - 2.4499).abs() < 2e-3,
            "{}",
            g[0].effective_index
        );
        // a narrower strip has fewer modes, and each index falls with the width
        let narrow = guided(0.3, 0.22, 1.55, SWEEP_STEP).unwrap();
        assert!(narrow.len() <= g.len());
        assert!(narrow[0].effective_index < g[0].effective_index);
    }

    /// The index chart: each curve's points are the guided modes at its widths, TE₀ rising
    /// with the width towards the 220 nm slab's TE₀ index; TE₁ appears where the figure says.
    #[test]
    fn neff_draws_each_mode_from_its_cutoff() {
        let data = neff(220.0, 1.55, 500.0).unwrap();
        let te0 = data.series.iter().find(|c| c.label == "TE₀").unwrap();
        assert_eq!(te0.points.len(), neff_widths().len());
        assert!(te0.points.windows(2).all(|p| p[1][1] > p[0][1]));
        let (si, ox) = indices(1.55).unwrap();
        let slab = photonoxide::mode::slab::Slab::new(ox, si, ox, Length::um(0.22))
            .unwrap()
            .modes(Polarization::Te, Wavelength::um(1.55).unwrap())[0]
            .effective_index();
        let widest = te0.points[te0.points.len() - 1][1];
        assert!(widest < slab && slab - widest < 0.1, "{widest} {slab}");
        let w1 = fig(&data, "One TE-like mode below").value.unwrap();
        let te1 = data.series.iter().find(|c| c.label == "TE₁").unwrap();
        assert!(
            te1.points[0][0] * 1e3 >= w1 - 1.0,
            "{w1} {:?}",
            te1.points[0]
        );
        assert!((300.0..600.0).contains(&w1), "{w1}");
        assert!(data.series.iter().any(|c| c.label == "TM₀" && c.dashed));
    }

    /// The approximations are the library's: the effective index method's 500 × 220 nm strip at
    /// the book's indices is the effective_index_method example's 2.488368, and every model is
    /// within a few per cent of the full-vector index, the effective index method above it.
    #[test]
    fn models_are_the_librarys_approximations() {
        let e = Ridge::strip(1.444, 3.473, 1.444, Length::um(0.22), Length::um(0.5))
            .unwrap()
            .mode(Polarization::Te, Wavelength::um(1.55).unwrap())
            .unwrap()
            .effective_index;
        assert!((e - 2.488368).abs() < 1e-6, "{e}");
        let data = models(220.0, 1.55, 500.0).unwrap();
        let reference = fig(&data, "Full-vector").value.unwrap();
        let eim = fig(&data, "Effective index method").value.unwrap();
        assert!(
            eim > reference && (eim - reference) / reference < 0.03,
            "{eim} {reference}"
        );
        let marcatili = fig(&data, "Marcatili").value.unwrap();
        assert!(
            (marcatili - reference).abs() / reference < 0.05,
            "{marcatili}"
        );
        // the device's analytic fit is within its stated 1e-4 of the 3D model's n_eff on its
        // own grid; on these coarser cells within the grid's error too
        let fit = fig(&data, "The device's fit").value.unwrap();
        assert!((fit - reference).abs() < 1e-3, "{fit} {reference}");
        // outside the fit's thicknesses there is no fit curve
        let thick = models(250.0, 1.55, 500.0).unwrap();
        assert!(!thick.series.iter().any(|c| c.label.contains("fit")));
    }

    /// Hadley's equations converge at about second order on the strip and the standard scheme
    /// at under one, as the hadley_corners example and the vector.md write-up find; the finest
    /// standard grid stays above the limit.
    #[test]
    fn corners_converge_at_their_orders() {
        let data = corners(500.0, 220.0).unwrap();
        let ph = fig(&data, "Order, Hadley's").value.unwrap();
        assert!(ph > 1.5, "{ph}");
        // the standard scheme's finest grid is further from the limit than Hadley's coarsest
        let err = fig(&data, "Standard's error").value.unwrap();
        let coarse = fig(&data, "Hadley's error").value.unwrap();
        assert!(err > 1e-3 && coarse.abs() < err, "{err} {coarse}");
        // the extrapolation is the device's 3D index on 5 nm cells, 2.445077, within 1e-5
        let limit = fig(&data, "Extrapolated").value.unwrap();
        assert!((limit - 2.445077).abs() < 1e-5, "{limit}");
        assert!(order(&[[4.0, 16.0], [2.0, 4.0], [1.0, 1.0]]) - 2.0 < 1e-9);
    }

    /// The group index is the device's 3D model's, above the effective index and above bulk
    /// silicon's; the effective index method's is below it.
    #[test]
    fn group_index_is_the_devices_3d_model() {
        let data = group_index(220.0, 1.55, 500.0).unwrap();
        let ng = fig(&data, "$n_g$").value.unwrap();
        let n = fig(&data, "$n_\\text{eff}$").value.unwrap();
        let bulk = fig(&data, "Bulk silicon").value.unwrap();
        assert!(ng > bulk && bulk > n, "{ng} {bulk} {n}");
        // the device's 3D model at its default grid gives 4.189; these cells are coarser
        assert!((ng - 4.19).abs() < 0.02, "{ng}");
        let eim = fig(&data, "$n_g$, effective").value.unwrap();
        assert!(eim < ng, "{eim} {ng}");
    }

    /// The field chart: three pictures, the main one TE-like's E_x, scaled to 1; the others
    /// smaller; a mode past the last is the last.
    #[test]
    fn fields_draw_the_three_components() {
        let data = fields(500.0, 220.0, 1.55, 1).unwrap();
        assert_eq!(data.maps.len(), 3);
        for m in &data.maps {
            let r = &m.raster;
            assert_eq!(r.values.len(), r.nx * r.ny);
            assert!((r.range().1 - 1.0).abs() < 1e-6);
        }
        assert!(data.maps[0].label.contains("main"));
        assert_eq!(fig(&data, "Mode").text, "TE₀");
        let ez = fig(&data, "Peak $|E_z|$").value.unwrap();
        assert!(ez > 0.1 && ez < 1.0, "{ez}");
        let tm = fields(500.0, 220.0, 1.55, 2).unwrap();
        assert_eq!(fig(&tm, "Mode").text, "TM₀");
        assert!(tm.maps[1].label.contains("main"));
        let last = fields(500.0, 220.0, 1.55, 6).unwrap();
        let count = fig(&last, "Guided modes").value.unwrap() as usize;
        assert!(count < 6 && last.note.contains("guides"));
    }
}
