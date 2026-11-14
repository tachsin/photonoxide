//! The charts of the lesson on photonic crystals and band gaps.
//!
//! - **A 1D crystal** (`crystal-stack`, `crystal-group-index`): an endless stack of two layers,
//!   its Bloch wavenumber from one period's transfer matrices, cos KΛ = Re(1/t)
//!   (`Multilayer::reflection`): exact, with no grid. The group index is dK/dω of the same
//!   function, its derivative by a central difference of 10⁻⁶ of the frequency.
//! - **A 2D crystal** (`crystal-bands`): a square or triangular lattice of rods or holes, its
//!   bands along the edge of the irreducible Brillouin zone by photonoxide's 2D FDTD between
//!   Bloch-periodic sides, each k-point one complex run from point currents, the frequencies
//!   from its probes by harmonic inversion (`fdtd::harmonic_inversion`). In 2D the field with
//!   E along the rods (TM) and the field with H along them (TE) don't couple, so one run serves
//!   both: E_z probes see the TM bands and H_z probes the TE bands. The triangular lattice is
//!   solved on its rectangular cell of two lattice sites, a × √3a; the bands that cell folds in
//!   are removed by adding each probe to its copy one lattice vector a₂ away, times
//!   e^(−ik·a₂), which cancels every field that isn't Bloch-periodic over a₂ with k.

use photonoxide::portable;
use std::collections::HashMap;
use std::f64::consts::{PI, TAU};
use std::sync::{Mutex, OnceLock};

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{Axis, Edges, Grid3d};
use photonoxide::fdtd::{
    Average, Body, Boundaries, Coupling, Cpml, Current, Field, Permittivity, Simulation, Smoothing,
    Structure, Waveform, harmonic_inversion,
};
use photonoxide::mode::multilayer::Multilayer;
use photonoxide::units::{Frequency, Length};
use rayon::prelude::*;

use super::{ChartData, ChartSpec, Curve, Figure, bloch_cos, figure, invalid, param, sig};

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: usize) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: Some(colour),
    }
}

/// The chart specs.
pub fn specs() -> Vec<ChartSpec> {
    let high = || {
        param(
            "high",
            "$\\varepsilon_1$",
            "",
            (1.0, 16.0, 13.0),
            0.1,
            false,
            "The permittivity of the first layer: 13 for Joannopoulos et al.'s examples (about silicon's or GaAs's 12 to 13 in the infrared).",
        )
    };
    let low = || {
        param(
            "low",
            "$\\varepsilon_2$",
            "",
            (1.0, 16.0, 1.0),
            0.1,
            false,
            "The permittivity of the second layer: 1 for air.",
        )
    };
    let fill = || {
        param(
            "fill",
            "Fill $d_1/a$",
            "",
            (0.05, 0.95, 0.2),
            0.005,
            false,
            "The first layer's share of the period a.",
        )
    };
    vec![
        ChartSpec {
            id: "crystal-stack",
            title: "The bands of a 1D photonic crystal",
            about: "An endless stack of two layers, ε₁ for d₁ and ε₂ for the rest of the period a, at normal incidence: the frequencies a Bloch wave of wavenumber K travels at, in the reduced zone, and the gaps between them where no wave travels.",
            computed_by: "mode::multilayer::Multilayer::reflection: one period's transmission t, and cos Ka = Re(1/t), exact",
            params: vec![high(), low(), fill()],
        },
        ChartSpec {
            id: "crystal-group-index",
            title: "Slow light at a band edge",
            about: "The group index n_g = c dK/dω of the same 1D crystal's first two bands: near the long-wavelength limit it is the stack's mean index, and towards a band edge it grows without bound. The line marks the chart's frequency.",
            computed_by: "mode::multilayer::Multilayer::reflection: cos Ka = Re(1/t), and dK/dω by a central difference of 10⁻⁶ of the frequency",
            params: vec![
                high(),
                low(),
                fill(),
                param(
                    "frequency",
                    "Frequency $\\omega a/2\\pi c$",
                    "",
                    (0.01, 1.0, 0.15),
                    0.0005,
                    false,
                    "The frequency, in units of c/a, at which to read the group index.",
                ),
            ],
        },
        ChartSpec {
            id: "crystal-bands",
            title: "The bands of a 2D photonic crystal",
            about: "A square or triangular lattice of dielectric rods in air, or of air holes in a dielectric, light travelling in its plane: the TM bands (E along the rods) and the TE bands (H along them) along the edge of the irreducible Brillouin zone, and their gaps.",
            computed_by: "fdtd::Simulation::smoothed between Bloch-periodic sides (2D, 24 cells a period, subpixel smoothing), one complex run a k-point; fdtd::harmonic_inversion on its probes",
            params: vec![
                param(
                    "lattice",
                    "Lattice",
                    "",
                    (1.0, 2.0, 1.0),
                    1.0,
                    false,
                    "1 a square lattice, Γ–X–M–Γ; 2 a triangular lattice, Γ–M–K–Γ.",
                ),
                param(
                    "kind",
                    "Rods or holes",
                    "",
                    (1.0, 2.0, 1.0),
                    1.0,
                    false,
                    "1 dielectric rods in air; 2 air holes in the dielectric.",
                ),
                param(
                    "eps",
                    "$\\varepsilon$",
                    "",
                    (2.0, 16.0, 8.9),
                    0.1,
                    false,
                    "The dielectric's permittivity: 8.9 for alumina, 12 to 13 for silicon or GaAs in the infrared.",
                ),
                param(
                    "radius",
                    "Radius $r/a$",
                    "",
                    (0.05, 0.49, 0.2),
                    0.005,
                    false,
                    "The rods' or holes' radius, in units of the lattice constant a.",
                ),
            ],
        },
    ]
}

// ---------------------------------------------------------------------------------------------
// 1D

/// The period of a 1D crystal, a = 1000 nm: ε₂ for half its share, ε₁ for `fill`, ε₂ again, in
/// ε₂: symmetric, so that cos Ka = Re(1/t).
fn period(high: f64, low: f64, fill: f64) -> photonoxide::Result<Multilayer> {
    let n = |e: f64| c64::new(e.sqrt(), 0.0);
    let d2 = 1000.0 * (1.0 - fill) / 2.0;
    Multilayer::new(
        n(low),
        &[
            (n(low), Length::nm(d2)),
            (n(high), Length::nm(1000.0 * fill)),
            (n(low), Length::nm(d2)),
        ],
        n(low),
    )
}

/// cos Ka at the frequency `u` = ωa/2πc = a/λ.
fn cos_ka(cell: &Multilayer, u: f64) -> photonoxide::Result<f64> {
    bloch_cos(cell, 1000.0 / u)
}

/// The frequencies sampled, ωa/2πc, from just above 0 to 1.
const SAMPLES: usize = 2000;

/// A band edge between `a` (inside a band, |cos Ka| ≤ 1) and `b` (inside a gap), by bisection.
fn edge(cell: &Multilayer, mut a: f64, mut b: f64) -> photonoxide::Result<f64> {
    for _ in 0..60 {
        let m = 0.5 * (a + b);
        if cos_ka(cell, m)?.abs() <= 1.0 {
            a = m;
        } else {
            b = m;
        }
    }
    Ok(0.5 * (a + b))
}

/// The bands of a 1D crystal below ωa/2πc = 1: each a list of (u, cos Ka), its ends at the
/// band edges; and the gaps, (lower edge, upper edge).
type Bands1d = (Vec<Vec<(f64, f64)>>, Vec<(f64, f64)>);

fn bands_1d(cell: &Multilayer) -> photonoxide::Result<Bands1d> {
    let us: Vec<f64> = (1..=SAMPLES).map(|i| i as f64 / SAMPLES as f64).collect();
    let cs: Vec<f64> = us
        .iter()
        .map(|&u| cos_ka(cell, u))
        .collect::<photonoxide::Result<_>>()?;
    let mut bands: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut gaps = Vec::new();
    let mut current: Vec<(f64, f64)> = vec![(0.0, 1.0)];
    let mut gap_start = None;
    for i in 0..us.len() {
        let inside = cs[i].abs() <= 1.0;
        let was = i == 0 || cs[i - 1].abs() <= 1.0;
        if inside && !was {
            let e = edge(cell, us[i], us[i - 1])?;
            gaps.push((gap_start.take().unwrap_or(e), e));
            current = vec![(e, cos_ka(cell, e)?.clamp(-1.0, 1.0))];
        }
        if !inside && was && i > 0 {
            let e = edge(cell, us[i - 1], us[i])?;
            current.push((e, cos_ka(cell, e)?.clamp(-1.0, 1.0)));
            bands.push(std::mem::take(&mut current));
            gap_start = Some(e);
        }
        if inside {
            current.push((us[i], cs[i]));
        }
    }
    if !current.is_empty() {
        bands.push(current);
    }
    Ok((bands, gaps))
}

/// The gap–midgap ratio of the gap (lower, upper), percent.
fn ratio((lo, hi): (f64, f64)) -> f64 {
    200.0 * (hi - lo) / (hi + lo)
}

/// The quarter-wave stack's gap–midgap ratio, percent: (4/π) arcsin(|n₁ − n₂|/(n₁ + n₂))
/// (Joannopoulos et al. 2008, p. 52, Eq. 5).
pub fn quarter_wave(high: f64, low: f64) -> f64 {
    let (n1, n2) = (high.sqrt(), low.sqrt());
    400.0 / PI * portable::asin((n1 - n2).abs() / (n1 + n2))
}

/// The bands of the 1D crystal of ε₁ = `high` for `fill` of the period and ε₂ = `low`.
pub fn stack(high: f64, low: f64, fill: f64) -> photonoxide::Result<ChartData> {
    let cell = period(high, low, fill)?;
    let (bands, gaps) = bands_1d(&cell)?;
    let mut series = Vec::new();
    for (n, b) in bands.iter().enumerate() {
        let points = b
            .iter()
            .map(|&(u, c)| [portable::acos(c.clamp(-1.0, 1.0)) / TAU, u])
            .collect();
        series.push(curve(&format!("band {}", n + 1), points, false, 0));
    }
    let mut figures = Vec::new();
    for (n, &g) in gaps.iter().enumerate() {
        series.push(curve(
            &format!("gap {}", n + 1),
            vec![[0.0, g.0], [0.5, g.0], [0.5, g.1], [0.0, g.1], [0.0, g.0]],
            true,
            1,
        ));
        if n < 2 {
            figures.push(figure(
                &format!("gap {} (bands {} and {})", n + 1, n + 1, n + 2),
                ratio(g),
                4,
                "%",
                &format!(
                    "gap–midgap ratio, from ωa/2πc = {} to {}",
                    sig(g.0, 5),
                    sig(g.1, 5)
                ),
            ));
        }
    }
    if gaps.is_empty() {
        figures.push(figure(
            "gap 1 (bands 1 and 2)",
            0.0,
            3,
            "%",
            "equal permittivities: no gap",
        ));
    }
    let (n1, n2) = (high.sqrt(), low.sqrt());
    figures.push(figure(
        "quarter-wave stack's gap",
        quarter_wave(high, low),
        4,
        "%",
        &format!(
            "(4/π) arcsin(|n₁ − n₂|/(n₁ + n₂)), at d₁/a = n₂/(n₁ + n₂) = {}",
            sig(n2 / (n1 + n2), 4)
        ),
    ));
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "Bloch wavenumber Ka/2π".to_owned(),
        x_length: false,
        y_label: "frequency ωa/2πc".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.0]),
        series,
        marker: None,
        figures,
        note: format!(
            "Transfer matrices, exact: no grid. ε₁ = {} for {} of the period, ε₂ = {}; normal \
             incidence. Dashed: the gaps.",
            sig(high, 3),
            sig(fill, 3),
            sig(low, 3)
        ),
    })
}

/// The group index n_g = c dK/dω = d(Ka/2π)/d(ωa/2πc) at `u` inside a band: −c′/(2π √(1 − c²))
/// in magnitude, c = cos Ka, c′ by a central difference.
fn group_index_at(cell: &Multilayer, u: f64) -> photonoxide::Result<Option<f64>> {
    let c = cos_ka(cell, u)?;
    if c.abs() >= 1.0 {
        return Ok(None);
    }
    let h = 1e-6 * u;
    let d = (cos_ka(cell, u + h)? - cos_ka(cell, u - h)?) / (2.0 * h);
    Ok(Some(d.abs() / (TAU * (1.0 - c * c).sqrt())))
}

/// The group index of the first two bands of the 1D crystal, and its value at `frequency`.
pub fn group_index(
    high: f64,
    low: f64,
    fill: f64,
    frequency: f64,
) -> photonoxide::Result<ChartData> {
    let cell = period(high, low, fill)?;
    let (bands, gaps) = bands_1d(&cell)?;
    let mut series = Vec::new();
    for (n, b) in bands.iter().take(2).enumerate() {
        let (lo, hi) = (b[0].0.max(1e-3), b[b.len() - 1].0);
        // denser towards the edges, where n_g changes fastest: u = lo + (hi − lo)(1 − cos πs)/2
        let mut points = Vec::new();
        for i in 1..800 {
            let s = i as f64 / 800.0;
            let u = lo + (hi - lo) * (1.0 - portable::cos(PI * s)) / 2.0;
            if let Some(ng) = group_index_at(&cell, u)? {
                points.push([u, ng]);
            }
        }
        series.push(curve(&format!("band {}", n + 1), points, false, n));
    }
    let mean = (fill * high + (1.0 - fill) * low).sqrt();
    series.push(curve(
        "√⟨ε⟩, the long-wavelength limit",
        vec![[0.0, mean], [1.0, mean]],
        true,
        7,
    ));
    let mut figures = Vec::new();
    match group_index_at(&cell, frequency)? {
        Some(ng) => figures.push(figure(
            "$n_g$ at the line",
            ng,
            4,
            "",
            &format!("at ωa/2πc = {}", sig(frequency, 4)),
        )),
        None => figures.push(Figure {
            label: "$n_g$ at the line".to_owned(),
            value: None,
            text: "in a gap".to_owned(),
            unit: String::new(),
            note: format!(
                "at ωa/2πc = {} no wave travels: K is complex",
                sig(frequency, 4)
            ),
        }),
    }
    figures.push(figure(
        "$\\sqrt{\\langle\\varepsilon\\rangle}$",
        mean,
        5,
        "",
        "the mean permittivity's root: the stack's index for wavelengths much longer than a",
    ));
    if let Some(&g) = gaps.first() {
        // at 1 % and 0.1 % of the band edge's frequency below it
        for share in [1e-2, 1e-3] {
            let u = g.0 * (1.0 - share);
            if let Some(ng) = group_index_at(&cell, u)? {
                figures.push(figure(
                    &format!(
                        "$n_g$ {} below the first gap",
                        if share > 5e-3 { "1 %" } else { "0.1 %" }
                    ),
                    ng,
                    4,
                    "",
                    &format!("at ωa/2πc = {}", sig(u, 5)),
                ));
            }
        }
    }
    let top = series
        .iter()
        .flat_map(|c| c.points.iter().map(|p| p[1]))
        .fold(1.0, f64::max);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "frequency ωa/2πc".to_owned(),
        x_length: false,
        y_label: "group index n_g".to_owned(),
        y_log: true,
        y_range: Some([1.0, top.min(1000.0)]),
        series,
        marker: Some(frequency),
        figures,
        note: format!(
            "Transfer matrices, exact: no grid; dK/dω by a central difference of 10⁻⁶ of the \
             frequency. ε₁ = {} for {} of the period, ε₂ = {}.",
            sig(high, 3),
            sig(fill, 3),
            sig(low, 3)
        ),
    })
}

// ---------------------------------------------------------------------------------------------
// 2D

/// The 2D lattice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lattice {
    Square,
    Triangular,
}

/// Cells a lattice constant along x.
pub const RESOLUTION: usize = 24;

/// The highest frequency the bands are drawn to, and the harmonic inversion's window's top,
/// ωa/2πc.
const TOP: f64 = 0.8;
const WINDOW: f64 = 1.0;

/// A path through the Brillouin zone: its k-points (2π/a), their distance along it, and its
/// corners' names and distances.
type Path = (Vec<[f64; 2]>, Vec<f64>, Vec<(&'static str, f64)>);

/// A 2D crystal, a = 1 µm.
#[derive(Clone, Copy, Debug)]
pub struct Crystal {
    pub lattice: Lattice,
    /// Rods (true) of `eps` in air, or holes of air in `eps`.
    pub rods: bool,
    pub eps: f64,
    pub radius: f64,
}

impl Crystal {
    /// The computational cell's size along x and y, µm, and its cells along each.
    fn cell(&self, n: usize) -> ([f64; 2], [usize; 2]) {
        match self.lattice {
            Lattice::Square => ([1.0, 1.0], [n, n]),
            Lattice::Triangular => {
                // an even number of cells along y, so a₂ = (½, √3/2) is a whole number of
                // cells along both axes (n is even)
                let m = 2 * ((3f64.sqrt() * n as f64 / 2.0).round() as usize);
                ([1.0, 3f64.sqrt()], [n, m])
            }
        }
    }

    /// The structure: the sites of the cell and their images in the eight neighbouring cells,
    /// so that the smoothing sees the bodies that cross the cell's sides.
    fn structure(&self, size: [f64; 2]) -> photonoxide::Result<Structure> {
        let air = Permittivity::isotropic(1.0)?;
        let dielectric = Permittivity::isotropic(self.eps)?;
        let (background, body) = if self.rods {
            (air, dielectric)
        } else {
            (dielectric, air)
        };
        let sites: Vec<[f64; 2]> = match self.lattice {
            Lattice::Square => vec![[0.5, 0.5]],
            Lattice::Triangular => {
                let h = 3f64.sqrt();
                vec![[0.25, 0.25 * h], [0.75, 0.75 * h]]
            }
        };
        let mut s = Structure::new(background);
        for site in sites {
            for i in -1..=1 {
                for j in -1..=1 {
                    let centre = [site[0] + i as f64 * size[0], site[1] + j as f64 * size[1]];
                    let disk = Body::ellipse(centre, [self.radius; 2], 0.0)?;
                    s = s.with(disk, body);
                }
            }
        }
        Ok(s)
    }

    /// The path along the edge of the irreducible Brillouin zone: its k-points (2π/a), their
    /// distance along the path, and the corners' names and distances.
    fn path(&self) -> Path {
        let corners: Vec<(&'static str, [f64; 2])> = match self.lattice {
            Lattice::Square => vec![
                ("Γ", [0.0, 0.0]),
                ("X", [0.5, 0.0]),
                ("M", [0.5, 0.5]),
                ("Γ", [0.0, 0.0]),
            ],
            Lattice::Triangular => {
                let h = 3f64.sqrt();
                // a₁ = (1, 0), a₂ = (½, √3/2): M = (½, 1/(2√3)), K = (2/3, 0)
                vec![
                    ("Γ", [0.0, 0.0]),
                    ("M", [0.5, 0.5 / h]),
                    ("K", [2.0 / 3.0, 0.0]),
                    ("Γ", [0.0, 0.0]),
                ]
            }
        };
        let (mut ks, mut along, mut marks) = (Vec::new(), Vec::new(), Vec::new());
        let mut s = 0.0;
        marks.push((corners[0].0, 0.0));
        ks.push(corners[0].1);
        along.push(0.0);
        for w in corners.windows(2) {
            let (a, b) = (w[0].1, w[1].1);
            let length = (portable::powi(b[0] - a[0], 2) + portable::powi(b[1] - a[1], 2)).sqrt();
            let steps = (length / 0.04).ceil().max(2.0) as usize;
            for i in 1..=steps {
                let t = i as f64 / steps as f64;
                ks.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
                along.push(s + t * length);
            }
            s += length;
            marks.push((w[1].0, s));
        }
        (ks, along, marks)
    }

    /// The TM and TE frequencies (ωa/2πc) at the Bloch wavevector `k` (2π/a) on `n` cells a
    /// lattice constant, each list sorted; ω = 0 at Γ included.
    pub fn modes(&self, k: [f64; 2], n: usize) -> photonoxide::Result<[Vec<f64>; 2]> {
        let (size, [nx, ny]) = self.cell(n);
        let h = [size[0] / nx as f64, size[1] / ny as f64];
        let grid = Grid3d {
            nx,
            ny,
            nz: 1,
            dx: h[0],
            dy: h[1],
            dz: h[0],
            x0: 0.0,
            y0: 0.0,
            z0: 0.0,
        };
        // Γ as a complex run too, at a wavevector too small to move any band but the first, whose
        // ω = 0 is added below: a real run's signal holds each frequency at ±ω, and its static
        // part, which the complex run's analytic source leaves out
        let gamma = k == [0.0, 0.0];
        let k = if gamma { [1e-6, 0.0] } else { k };
        let (kx, ky) = (TAU * k[0], TAU * k[1]);
        let boundaries = Boundaries {
            x: Edges::Bloch { k: kx },
            y: Edges::Bloch { k: ky },
            z: Edges::Bloch { k: 0.0 },
            cpml: Cpml::default(),
        };
        let structure = self.structure(size)?;
        // each value of E its own cell's subpixel average, the tensor's diagonal: a Bloch phase
        // can't carry the off-diagonal coupling across the sides. E_z (TM) lies along every
        // interface and takes the exact mean ε
        let smoothing = Smoothing {
            average: Average::Subpixel,
            diameter: 1.0,
            coupling: Coupling::Diagonal,
        };
        let mut s = Simulation::smoothed(grid, &structure, smoothing, boundaries, 0.95)?;
        let pulse = Waveform::pulse(Frequency::natural(0.45)?, 0.7)?;
        let at = |x: f64, y: f64| {
            (
                ((x * nx as f64) as usize).min(nx - 1),
                ((y * ny as f64) as usize).min(ny - 1),
                0,
            )
        };
        // sources and probes off every symmetry line of the lattices, so that no band has a
        // node at all of them
        let sources = [(0.137, 0.283), (0.611, 0.779), (0.853, 0.419)];
        let probes = [(0.291, 0.107), (0.743, 0.561), (0.419, 0.887)];
        for field in [Field::E, Field::H] {
            s.add_current(Current {
                field,
                values: sources
                    .iter()
                    .enumerate()
                    .map(|(i, &(x, y))| {
                        (
                            Axis::Z,
                            at(x, y),
                            portable::complex::from_polar(1.0, 1.3 * i as f64),
                        )
                    })
                    .collect(),
                waveform: pulse,
            })?;
        }
        // each probe, and for the triangular lattice its copy a₂ = (½, √3/2) away (half the
        // cell along both axes), wrapped into the cell
        let mut pairs = Vec::new();
        for field in [Field::E, Field::H] {
            for &(x, y) in &probes {
                let p = at(x, y);
                let first = s.add_probe(field, Axis::Z, p)?;
                let second = match self.lattice {
                    Lattice::Square => None,
                    Lattice::Triangular => {
                        let q = ((p.0 + nx / 2) % nx, (p.1 + ny / 2) % ny, 0);
                        // the copy's field is f(r + a₂) = e^(ik·L) f(q) for each side crossed
                        let mut phase = 0.0;
                        if p.0 + nx / 2 >= nx {
                            phase += kx * size[0];
                        }
                        if p.1 + ny / 2 >= ny {
                            phase += ky * size[1];
                        }
                        // the projection's weight e^(−ik·a₂), times the wrap's phase
                        let a2 = [0.5, 0.5 * 3f64.sqrt()];
                        let weight =
                            portable::complex::from_polar(1.0, phase - (kx * a2[0] + ky * a2[1]));
                        Some((s.add_probe(field, Axis::Z, q)?, weight))
                    }
                };
                pairs.push((field, first, second));
            }
        }
        let Waveform::Gaussian { delay, .. } = pulse else {
            unreachable!("a pulse is a Gaussian")
        };
        let after = 2.0 * delay;
        let record = 160.0;
        s.run_until(after + record);
        let dt = s.dt();
        let start = (after / dt).ceil() as usize;
        // a sample every 0.4 a/c: Nyquist at 1.25 c/a, above the window
        let every = ((0.4 / dt).floor() as usize).max(1);
        // the terms a signal holds: an error near round-off (Mandelshtam and Taylor's step 5),
        // no decay over the record (the crystal is lossless), and an amplitude above 10⁻⁴ of
        // the largest such term's; (frequency, amplitude)
        let terms = |signal: &[c64]| -> photonoxide::Result<Vec<(f64, f64)>> {
            let samples: Vec<c64> = signal[start..].iter().step_by(every).copied().collect();
            let found = harmonic_inversion(
                &samples,
                every as f64 * dt,
                Frequency::natural(0.005)?,
                Frequency::natural(WINDOW)?,
            )?;
            let good: Vec<(f64, f64)> = found
                .iter()
                .filter(|r| r.error < 1e-3 && r.decay.abs() * record < 1.0)
                .map(|r| (r.frequency, portable::complex::abs(r.amplitude)))
                .collect();
            let largest = good.iter().map(|t| t.1).fold(0.0, f64::max);
            Ok(good.into_iter().filter(|t| t.1 > 1e-4 * largest).collect())
        };
        let mut out = [Vec::new(), Vec::new()];
        for (field, first, second) in pairs {
            let pol = usize::from(field == Field::H);
            let a = s.probe_complex(first);
            let kept: Vec<f64> = match second {
                None => terms(&a)?.into_iter().map(|t| t.0).collect(),
                Some((q, w)) => {
                    // the projection onto k, and its opposite, which holds only the folded
                    // bands: a folded band is far stronger in the second (by 10³ and more),
                    // and at M and K, where the folded wavevector is the same point of
                    // another zone, a band is in both alike
                    let b = s.probe_complex(q);
                    let plus: Vec<c64> = a.iter().zip(&b).map(|(x, y)| x + w * y).collect();
                    let minus: Vec<c64> = a.iter().zip(&b).map(|(x, y)| x - w * y).collect();
                    let (plus, minus) = (terms(&plus)?, terms(&minus)?);
                    plus.iter()
                        .filter(|&&(f, amp)| {
                            !minus
                                .iter()
                                .any(|&(g, other)| (f - g).abs() < 1e-3 && other > 10.0 * amp)
                        })
                        .map(|t| t.0)
                        .collect()
                }
            };
            out[pol].extend(kept);
        }
        for list in &mut out {
            if gamma {
                list.push(0.0);
            }
            list.sort_by(f64::total_cmp);
            // one frequency for the probes that see the same mode
            list.dedup_by(|a, b| (*a - *b).abs() < 2e-4);
        }
        Ok(out)
    }
}

/// The bands, both polarizations, along the path.
#[derive(Clone, Debug)]
pub struct Bands2d {
    pub along: Vec<f64>,
    pub marks: Vec<(&'static str, f64)>,
    /// The frequencies found at each k-point: [TM, TE].
    pub modes: Vec<[Vec<f64>; 2]>,
}

fn cache() -> &'static Mutex<HashMap<[u64; 5], Bands2d>> {
    static CACHE: OnceLock<Mutex<HashMap<[u64; 5], Bands2d>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The bands of `crystal` on `n` cells a lattice constant, the k-points in parallel; cached.
pub fn bands_2d(crystal: &Crystal, n: usize) -> photonoxide::Result<Bands2d> {
    let key = [
        u64::from(crystal.lattice == Lattice::Triangular),
        u64::from(crystal.rods),
        crystal.eps.to_bits(),
        crystal.radius.to_bits(),
        n as u64,
    ];
    if let Some(b) = cache().lock().ok().and_then(|c| c.get(&key).cloned()) {
        return Ok(b);
    }
    let (ks, along, marks) = crystal.path();
    let modes = ks
        .par_iter()
        .map(|&k| crystal.modes(k, n))
        .collect::<photonoxide::Result<Vec<_>>>()?;
    let b = Bands2d {
        along,
        marks,
        modes,
    };
    if let Ok(mut c) = cache().lock() {
        if c.len() > 32 {
            c.clear();
        }
        c.insert(key, b.clone());
    }
    Ok(b)
}

/// The bands tracked from one k-point to the next: `count` bands, each expected near its last
/// point, matched in order to the modes found, the sum of the distances least: a band may take
/// the mode the band below it takes, since harmonic inversion returns a degenerate pair once,
/// and modes may be left over.
pub fn track(modes: &[Vec<f64>], count: usize) -> Vec<Vec<f64>> {
    let mut bands: Vec<Vec<f64>> = vec![Vec::new(); count];
    for found in modes {
        let m = found.len();
        if m == 0 {
            for band in &mut bands {
                band.push(f64::NAN);
            }
            continue;
        }
        let predicted: Vec<f64> = bands
            .iter()
            .enumerate()
            .map(|(b, v)| {
                v.last()
                    .copied()
                    .filter(|f| f.is_finite())
                    .unwrap_or_else(|| found.get(b).copied().unwrap_or(WINDOW))
            })
            .collect();
        // cost[a][j]: bands 0..=a matched, band a to mode j, each band's mode at or above the
        // one below's; from[a][j] the mode band a − 1 took
        let mut cost = vec![vec![f64::INFINITY; m]; count];
        let mut from = vec![vec![0; m]; count];
        for j in 0..m {
            cost[0][j] = (predicted[0] - found[j]).abs();
        }
        for a in 1..count {
            let (mut best, mut at) = (f64::INFINITY, 0);
            for j in 0..m {
                if cost[a - 1][j] < best {
                    (best, at) = (cost[a - 1][j], j);
                }
                cost[a][j] = best + (predicted[a] - found[j]).abs();
                from[a][j] = at;
            }
        }
        let mut j = (0..m)
            .min_by(|&x, &y| cost[count - 1][x].total_cmp(&cost[count - 1][y]))
            .unwrap_or(0);
        let mut chosen = vec![0; count];
        for a in (0..count).rev() {
            chosen[a] = j;
            j = from[a][j];
        }
        for (band, &mode) in bands.iter_mut().zip(&chosen) {
            band.push(found[mode]);
        }
    }
    bands
}

/// The gaps below `top` of one polarization's modes along the path: what no band covers, each
/// band covering, between neighbouring k-points, the interval from each mode to the nearest
/// mode at the other point. (lower, upper) pairs, the narrowest kept 10⁻³ wide.
pub fn gaps(modes: &[Vec<f64>], top: f64) -> Vec<(f64, f64)> {
    let mut covered: Vec<(f64, f64)> = Vec::new();
    for w in modes.windows(2) {
        for (a, b) in [(&w[0], &w[1]), (&w[1], &w[0])] {
            for &f in a {
                if let Some(&g) = b
                    .iter()
                    .min_by(|x, y| (*x - f).abs().total_cmp(&(*y - f).abs()))
                {
                    covered.push((f.min(g), f.max(g)));
                }
            }
        }
    }
    covered.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    let mut reach = 0.0;
    for (lo, hi) in covered {
        if lo > reach + 1e-3 && lo < top {
            out.push((reach, lo));
        }
        reach = f64::max(reach, hi);
    }
    out
}

/// Where two lists of gaps overlap.
pub fn overlap(a: &[(f64, f64)], b: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for &(a0, a1) in a {
        for &(b0, b1) in b {
            let (lo, hi) = (a0.max(b0), a1.min(b1));
            if hi > lo + 1e-3 {
                out.push((lo, hi));
            }
        }
    }
    out
}

/// The largest gap by its gap–midgap ratio.
pub fn largest(gaps: &[(f64, f64)]) -> Option<(f64, f64)> {
    gaps.iter()
        .copied()
        .max_by(|a, b| ratio(*a).total_cmp(&ratio(*b)))
}

/// The 2D chart: the TM and TE bands of `crystal` along the zone's edge.
pub fn bands(lattice: usize, kind: usize, eps: f64, radius: f64) -> photonoxide::Result<ChartData> {
    let crystal = Crystal {
        lattice: match lattice {
            1 => Lattice::Square,
            2 => Lattice::Triangular,
            _ => return Err(invalid("lattice", "1 square or 2 triangular")),
        },
        rods: match kind {
            1 => true,
            2 => false,
            _ => return Err(invalid("kind", "1 rods or 2 holes")),
        },
        eps,
        radius,
    };
    if !(0.05..=0.49).contains(&radius) {
        return Err(invalid("radius", "r/a from 0.05 to 0.49"));
    }
    let b = bands_2d(&crystal, RESOLUTION)?;
    let mut series = Vec::new();
    let mut figures = Vec::new();
    let mut found = Vec::new();
    for (pol, name) in [(0, "TM"), (1, "TE")] {
        let modes: Vec<Vec<f64>> = b.modes.iter().map(|m| m[pol].clone()).collect();
        // as many bands as come below the top anywhere, and no more than every k-point has
        // within the window
        let below = modes
            .iter()
            .map(|m| m.iter().filter(|&&f| f < TOP).count())
            .max()
            .unwrap_or(1);
        let within = modes.iter().map(Vec::len).min().unwrap_or(1);
        let count = below.min(within).max(1);
        for (n, band) in track(&modes, count).into_iter().enumerate() {
            let points = b
                .along
                .iter()
                .zip(band)
                .filter(|(_, f)| f.is_finite())
                .map(|(&x, f)| [x, f])
                .collect();
            series.push(Curve {
                label: format!("{name} {}", n + 1),
                points,
                dashed: pol == 1,
                colour: Some(pol),
            });
        }
        let g = gaps(&modes, TOP);
        match largest(&g) {
            Some(gap) => figures.push(figure(
                &format!("{name} gap"),
                ratio(gap),
                3,
                "%",
                &format!(
                    "gap–midgap ratio, ωa/2πc from {} to {}; the largest below {TOP}",
                    sig(gap.0, 4),
                    sig(gap.1, 4)
                ),
            )),
            None => figures.push(figure(
                &format!("{name} gap"),
                0.0,
                3,
                "%",
                &format!("no gap below ωa/2πc = {TOP}"),
            )),
        }
        found.push(g);
    }
    let complete = overlap(&found[0], &found[1]);
    match largest(&complete) {
        Some(gap) => {
            figures.push(figure(
                "complete gap",
                ratio(gap),
                3,
                "%",
                &format!(
                    "both polarizations, ωa/2πc from {} to {}",
                    sig(gap.0, 4),
                    sig(gap.1, 4)
                ),
            ));
        }
        None => figures.push(figure(
            "complete gap",
            0.0,
            3,
            "%",
            "no frequency is in a TM and a TE gap at once",
        )),
    }
    for &(name, x) in b.marks.iter().skip(1).take(b.marks.len() - 2) {
        series.push(curve(name, vec![[x, 0.0], [x, TOP]], true, 7));
    }
    // the narrowest dielectric between neighbouring holes, a − 2r, in cells: below two cells the
    // TE bands, whose field follows those veins, aren't converged
    let vein = (1.0 - 2.0 * radius) * RESOLUTION as f64;
    let thin = if !crystal.rods && vein < 2.0 {
        format!(
            " The dielectric between neighbouring holes is {} cells wide: the TE bands, whose field \
             runs along it, move by several percent on finer grids.",
            sig(vein, 2)
        )
    } else {
        String::new()
    };
    let end = b.along.last().copied().unwrap_or(1.0);
    let corners: Vec<&str> = b.marks.iter().map(|m| m.0).collect();
    Ok(ChartData {
        maps: Vec::new(),
        x_label: format!("wavevector along {} (2π/a)", corners.join("–")),
        x_length: false,
        y_label: "frequency ωa/2πc".to_owned(),
        y_log: false,
        y_range: Some([0.0, TOP]),
        series: series
            .into_iter()
            .map(|mut c| {
                c.points.retain(|p| p[0] <= end);
                c
            })
            .collect(),
        marker: None,
        figures,
        note: format!(
            "2D FDTD between Bloch-periodic sides, {RESOLUTION} cells a lattice constant, subpixel \
             smoothing; {} k-points, each one complex run, its frequencies by harmonic inversion. \
             Solid: TM (E along the {}), dashed: TE.{thin}",
            b.along.len(),
            if crystal.rods { "rods" } else { "holes" }
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use photonoxide::portable;

    fn value(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f| f.label == label)
            .and_then(|f| f.value)
            .unwrap_or_else(|| panic!("no figure {label}"))
    }

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} against {b} (± {tol})");
    }

    /// Joannopoulos et al. 2008, pp. 51–52: a 13:1 stack's gap between bands 1 and 2 is 76.3 %
    /// at d₁ = 0.2a and 51.9 % at 0.5a, the quarter-wave stack's 76.6 % (Eq. 5); 13:12 at 0.5a
    /// gives 2.55 %. Our transfer matrices are exact, and the book prints three digits: the
    /// tolerance is half its last digit.
    #[test]
    fn the_stacks_gaps_are_joannopoulos_et_al_s() {
        let gap = |h, l, f| value(&stack(h, l, f).unwrap(), "gap 1 (bands 1 and 2)");
        close(gap(13.0, 1.0, 0.2), 76.3, 0.05);
        close(gap(13.0, 1.0, 0.5), 51.9, 0.05);
        close(gap(13.0, 12.0, 0.5), 2.55, 0.005);
        close(quarter_wave(13.0, 1.0), 76.6, 0.05);
        // the quarter-wave fill: the stack's gap is Eq. 5's, to the bisection's precision
        let qw = 1.0 / (1.0 + 13f64.sqrt());
        close(gap(13.0, 1.0, qw), quarter_wave(13.0, 1.0), 1e-6);
    }

    /// At long wavelengths the stack is a uniform medium of ε = ⟨ε⟩ (E along the layers), and
    /// n_g → √⟨ε⟩; towards the band edge n_g grows as 1/√(ω_edge − ω).
    #[test]
    fn the_group_index_is_the_mean_index_at_long_wavelengths_and_diverges_at_the_edge() {
        let data = group_index(13.0, 1.0, 0.2, 0.005).unwrap();
        let mean = (0.2f64 * 13.0 + 0.8).sqrt();
        close(value(&data, "$n_g$ at the line"), mean, 1e-3 * mean);
        let one = value(&data, "$n_g$ 1 % below the first gap");
        let tenth = value(&data, "$n_g$ 0.1 % below the first gap");
        // ten times closer: √10 times slower, to the curvature's first order
        close(tenth / one, 10f64.sqrt(), 0.05 * 10f64.sqrt());
    }

    /// The gap's ratio (0 TM, 1 TE, 2 both) of `crystal` on `n` cells a lattice constant,
    /// percent, and its edges.
    fn gap_on(crystal: &Crystal, n: usize, which: usize) -> (f64, (f64, f64)) {
        let b = bands_2d(crystal, n).unwrap();
        let of = |pol: usize| -> Vec<Vec<f64>> { b.modes.iter().map(|m| m[pol].clone()).collect() };
        let found = match which {
            0 | 1 => gaps(&of(which), TOP),
            _ => overlap(&gaps(&of(0), TOP), &gaps(&of(1), TOP)),
        };
        let g = largest(&found).expect("a gap");
        (ratio(g), g)
    }

    /// A gap against Joannopoulos et al.'s by one rule: Richardson's extrapolation of the gap
    /// from 24 and 32 cells a lattice constant, its error taken as second order in the cell, is
    /// within half the book's last printed digit of the book's number, plus the extrapolation's
    /// own step from the 32-cell gap; and the chart's 24-cell gap within half that digit plus
    /// its step from 24 cells.
    fn against_the_book(crystal: &Crystal, which: usize, book: f64) {
        let ((coarse, edges), (fine, _)) = (gap_on(crystal, 24, which), gap_on(crystal, 32, which));
        let extrapolated = (portable::powi(32f64, 2) * fine - portable::powi(24f64, 2) * coarse)
            / (portable::powi(32f64, 2) - portable::powi(24f64, 2));
        eprintln!(
            "{crystal:?}: {coarse:.3} % at 24 cells (from {:.4} to {:.4}), {fine:.3} % at 32, \
             extrapolated {extrapolated:.3} %, the book {book} %",
            edges.0, edges.1
        );
        close(extrapolated, book, 0.05 + (extrapolated - fine).abs());
        close(coarse, book, 0.05 + (extrapolated - coarse).abs());
    }

    /// Joannopoulos et al. 2008, p. 69: a square lattice of ε = 8.9 rods, r = 0.2a, has a TM
    /// gap of 31.4 % between bands 1 and 2, about 0.32 to 0.44 (p. 91), and no TE gap.
    #[test]
    fn the_square_lattice_of_rods_has_joannopoulos_et_al_s_tm_gap() {
        let rods = Crystal {
            lattice: Lattice::Square,
            rods: true,
            eps: 8.9,
            radius: 0.2,
        };
        against_the_book(&rods, 0, 31.4);
        let data = bands(1, 1, 8.9, 0.2).unwrap();
        assert_eq!(value(&data, "TE gap"), 0.0);
        assert_eq!(value(&data, "complete gap"), 0.0);
    }

    /// Joannopoulos et al. 2008, p. 75: a triangular lattice of air holes in ε = 13, r = 0.48a,
    /// has an 18.6 % complete gap, where its TE and TM gaps overlap. Ours converges to 18.9 %
    /// (18.94, 18.89, 18.91 and 18.92 % at 32, 40, 48 and 64 cells: `triangular_convergence`),
    /// 0.3 points above the book's: not reproduced, and said so in the lesson. The test holds
    /// our gap to its convergence, 32 against 48 cells within 0.1 point, and to the book within
    /// that 0.3-point difference and the grid's spread.
    #[test]
    fn the_triangular_lattice_of_holes_has_a_complete_gap_near_joannopoulos_et_al_s() {
        let holes = Crystal {
            lattice: Lattice::Triangular,
            rods: false,
            eps: 13.0,
            radius: 0.48,
        };
        let (fine, _) = gap_on(&holes, 32, 2);
        let (finer, _) = gap_on(&holes, 48, 2);
        eprintln!("complete gap {fine:.3} % at 32 cells, {finer:.3} % at 48, the book 18.6 %");
        close(finer, fine, 0.1);
        close(finer, 18.6, 0.05 + 0.31 + (finer - fine).abs());
    }

    /// At M the square lattice's second and third TM bands meet (0.5444 and 0.5480 one step
    /// before, 0.5469 at M, returned once): both bands take it, and the fourth band the next
    /// mode, though more modes than bands were found.
    #[test]
    fn a_degenerate_pair_is_shared_by_its_two_bands() {
        let modes = vec![
            vec![0.3213, 0.5444, 0.5480, 0.6949, 0.9127],
            vec![0.3221, 0.5469, 0.6928, 0.9172, 0.9671],
        ];
        let bands = track(&modes, 4);
        let at_m: Vec<f64> = bands.iter().map(|b| b[1]).collect();
        assert_eq!(at_m, vec![0.3221, 0.5469, 0.5469, 0.6928]);
    }

    /// The triangular lattice's complete gap and TE gap from 16 to 64 cells a lattice constant:
    /// `cargo test -p photonoxide-studio triangular_convergence -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn triangular_convergence() {
        let holes = Crystal {
            lattice: Lattice::Triangular,
            rods: false,
            eps: 13.0,
            radius: 0.48,
        };
        for n in [16, 24, 32, 40, 48, 64] {
            let (g, e) = gap_on(&holes, n, 2);
            let (te, _) = gap_on(&holes, n, 1);
            eprintln!(
                "triangular holes {n}: complete {g:.3} % ({:.4} to {:.4}), TE {te:.3} %",
                e.0, e.1
            );
        }
    }

    /// The numbers the lesson quotes, printed:
    /// `cargo test -p photonoxide-studio lesson_numbers -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn lesson_numbers() {
        for (h, l, f) in [(13.0, 1.0, 0.2), (13.0, 1.0, 0.5), (13.0, 12.0, 0.5)] {
            for g in &stack(h, l, f).unwrap().figures {
                eprintln!(
                    "stack {h}:{l} at {f}: {} = {} ({})",
                    g.label, g.text, g.note
                );
            }
        }
        for g in &group_index(13.0, 1.0, 0.2, 0.2).unwrap().figures {
            eprintln!("group index: {} = {} ({})", g.label, g.text, g.note);
        }
        for (lattice, kind, eps, r) in [
            (1, 1, 8.9, 0.2),
            (2, 2, 13.0, 0.48),
            (2, 2, 13.0, 0.4),
            (2, 2, 13.0, 0.3),
        ] {
            for g in &bands(lattice, kind, eps, r).unwrap().figures {
                eprintln!(
                    "bands {lattice} {kind} {eps} {r}: {} = {} ({})",
                    g.label, g.text, g.note
                );
            }
        }
    }
}
