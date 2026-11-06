//! The metasurface lesson's charts.
//!
//! - **The orders of a phase-gradient grating** (`meta-orders`): a supercell of N pillars,
//!   period p each, carrying the phases 2πk/N, is a grating of period Γ = Np; its orders leave at
//!   n_t sin θ_m = n_i sin θ_i + mλ/Γ, and a thin element whose transmission is e^(iφ) with φ that
//!   staircase sends sinc²(m/N) of the power into order m when m − 1 is a multiple of N and none
//!   elsewhere (its Fourier series). Exact, with no grid: the generalized law of refraction is
//!   order +1.
//! - **A pillar's phase** (`meta-pillar`): silicon ridges (n = 3.473) on silica (1.444) in air,
//!   one a period, lit at normal incidence from the silica, E along the ridges; 2D FDTD between
//!   periodic sides (`fdtd::Simulation`, a sheet of current in the silica, the transmitted E
//!   averaged over a period, which is the zeroth order), divided by the same run with no ridge.
//!   Beside it the waveguide picture: the ridge a slab guide of its width in air
//!   (`mode::slab::Slab::modes`, TE₀, exact), its phase (n_eff − 1)kH.
//! - **A metalens's focus** (`meta-focus`) and **its colour** (`meta-axis`): a cylindrical lens
//!   (2D), its hyperbolic phase −k₀(√(x² + f²) − f) wrapped into [0, 2π) at the design wavelength,
//!   held constant over each pillar's cell and rounded to N levels, the field after it propagated
//!   by the 2D Rayleigh–Sommerfeld integral in its far-zone form (kρ ≫ 1), summed on steps of
//!   λ/20. At another wavelength each pillar's phase is scaled by λ₀/λ, a delay with no
//!   dispersion of its own. Scalar; summed in the studio, with no library function behind it.
//! - **The achromatic bound** (`meta-bandwidth`): Presutti and Monticone 2020's Eqs. 10–12
//!   (doi:10.1364/OPTICA.389404), the largest band Δω over which a lens of focal length F,
//!   numerical aperture NA and thickness L in air can keep its focus, for resonant meta-atoms,
//!   for waveguides (Tucker's delay-bandwidth product) and for any structure (Miller's).

use std::collections::HashMap;
use std::f64::consts::{PI, TAU};
use std::sync::{Mutex, OnceLock};

use num_complex::Complex64 as c64;
use photonoxide::fdfd::{Axis, Edges, Grid3d};
use photonoxide::fdtd::{Boundaries, Cpml, Current, Field, Simulation, Waveform};
use photonoxide::mode::Polarization;
use photonoxide::mode::slab::Slab;
use photonoxide::units::{Frequency, Length, Wavelength};
use rayon::prelude::*;

use super::slab::{OXIDE, SILICON};
use super::{ChartData, ChartSpec, Curve, Figure, figure, invalid, param, sig};

/// The palette's grey, for reference lines.
const GREY: usize = 7;
/// The lens charts' design wavelength, µm.
pub const DESIGN: f64 = 1.55;
/// Light's speed, µm/ps, for the bounds in rad/ps.
const C_UM_PS: f64 = 299.792_458;

/// sin(πx)/(πx).
pub fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

/// The share of the power an N-level staircase of phases 2πk/N sends into order `m`: sinc²(m/N)
/// when m − 1 is a multiple of N, 0 otherwise.
pub fn staircase_efficiency(levels: usize, m: i64) -> f64 {
    let n = levels as i64;
    if (m - 1).rem_euclid(n) == 0 {
        sinc(m as f64 / levels as f64).powi(2)
    } else {
        0.0
    }
}

/// The angle (degrees) at which order `m` of a grating of period `gamma` µm leaves into an index
/// `n_out`, lit from `n_in` at `theta_in` degrees, or `None` where it is evanescent.
pub fn order_angle(
    wavelength: f64,
    gamma: f64,
    n_in: f64,
    n_out: f64,
    theta_in: f64,
    m: i64,
) -> Option<f64> {
    let s = (n_in * theta_in.to_radians().sin() + m as f64 * wavelength / gamma) / n_out;
    (s.abs() <= 1.0).then(|| s.asin().to_degrees())
}

pub fn orders(
    wavelength: f64,
    period_nm: f64,
    pillars: usize,
    angle: f64,
    n_in: f64,
    n_out: f64,
) -> photonoxide::Result<ChartData> {
    if pillars < 2 {
        return Err(invalid("pillars", "a supercell needs at least two pillars"));
    }
    let p = period_nm * 1e-3;
    let gamma = pillars as f64 * p;
    let colours = [5usize, 3, 1, 0, 2, 4, 6];
    let mut series = Vec::new();
    for (c, m) in (-3i64..=3).enumerate() {
        // each order as runs of angles where it propagates, split where it turns evanescent
        let mut run: Vec<[f64; 2]> = Vec::new();
        let mut runs = Vec::new();
        for k in 0..=480 {
            let t = -60.0 + 120.0 * f64::from(k) / 480.0;
            match order_angle(wavelength, gamma, n_in, n_out, t, m) {
                Some(a) => run.push([t, a]),
                None => {
                    if !run.is_empty() {
                        runs.push(std::mem::take(&mut run));
                    }
                }
            }
        }
        if !run.is_empty() {
            runs.push(run);
        }
        let eta = staircase_efficiency(pillars, m);
        for (j, points) in runs.into_iter().enumerate() {
            series.push(Curve {
                label: if j == 0 {
                    format!("order {m:+}: {} %", sig(100.0 * eta, 3))
                } else {
                    String::new()
                },
                points,
                dashed: eta == 0.0,
                colour: Some(colours[c]),
            });
        }
    }
    let lattice_ok = p < wavelength / (n_out + n_in * angle.to_radians().sin().abs());
    let plus = order_angle(wavelength, gamma, n_in, n_out, angle, 1);
    let mut figures = vec![
        figure("supercell $\\Gamma = Np$", gamma, 4, "µm", ""),
        figure(
            "order +1's share",
            100.0 * staircase_efficiency(pillars, 1),
            4,
            "%",
            "sinc²(1/N), the staircase's",
        ),
        figure(
            "order 1 − N's share",
            100.0 * staircase_efficiency(pillars, 1 - pillars as i64),
            3,
            "%",
            "the next order the staircase feeds",
        ),
    ];
    figures.push(match plus {
        Some(a) => figure("order +1 leaves at", a, 4, "°", "at the chart's incidence"),
        None => Figure {
            label: "order +1 leaves at".to_owned(),
            value: None,
            text: "evanescent".to_owned(),
            unit: String::new(),
            note: "the gradient is too steep for this incidence".to_owned(),
        },
    });
    figures.push(Figure {
        label: "the pillars' own lattice".to_owned(),
        value: None,
        text: if lattice_ok {
            "diffracts nothing".to_owned()
        } else {
            "diffracts".to_owned()
        },
        unit: String::new(),
        note: format!(
            "p < λ/(n_t + n_i|sin θ_i|) = {} nm",
            sig(
                1e3 * wavelength / (n_out + n_in * angle.to_radians().sin().abs()),
                4
            )
        ),
    });
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "incidence θ_i (°)".to_owned(),
        x_length: false,
        y_label: "order's angle θ_m (°)".to_owned(),
        y_log: false,
        y_range: Some([-90.0, 90.0]),
        series,
        marker: Some(angle),
        figures,
        note: format!(
            "n_t sin θ_m = n_i sin θ_i + mλ/Γ for a supercell of {pillars} pillars, Γ = {} µm; each order's share of the power from the thin staircase's Fourier series, the orders it doesn't feed dashed. Exact, no grid.",
            sig(gamma, 4)
        ),
    })
}

/// A row of silicon ridges on silica, one a period, in air, 2D: ridges along z, x across the
/// period, y up.
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    /// The period, µm.
    pub period: f64,
    /// The ridges' height H, µm.
    pub height: f64,
    /// The wavelength, µm.
    pub wavelength: f64,
    /// Grid cells per µm.
    pub per_um: f64,
    /// How long the run records after its pulse, µm/c.
    pub record: f64,
}

/// The overlap of [a0, a1] with [b0, b1].
fn overlap(a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    (a1.min(b1) - a0.max(b0)).max(0.0)
}

impl Cell {
    /// The E_z a ridge `w` µm wide transmits into the air, averaged over a period at a line
    /// 0.6 µm above it (the zeroth order: every other order averages to nothing over a period),
    /// as the DFT at the wavelength of a pulse from a sheet of current in the silica. `w` = 0 is
    /// no ridge; `w` ≥ the period a film. The grid holds a whole number of cells a period, the
    /// nearest to `per_um`.
    pub fn transmitted(&self, w: f64) -> photonoxide::Result<c64> {
        let nx = (self.period * self.per_um).round().max(4.0) as usize;
        let h = self.period / nx as f64;
        let cells = |x: f64| (x / h).round() as usize;
        let pml = cells(0.5);
        let (below, above) = (1.0, 1.0);
        let ny = 2 * pml + cells(below + self.height + above);
        let grid = Grid3d {
            nx,
            ny,
            nz: 1,
            dx: h,
            dy: h,
            dz: h,
            x0: 0.0,
            y0: 0.0,
            z0: 0.0,
        };
        let base = pml as f64 * h + below;
        let top = base + self.height;
        let (lo, hi) = if w >= self.period {
            (-1.0, self.period + 1.0)
        } else {
            (0.5 * (self.period - w), 0.5 * (self.period + w))
        };
        let (es, ef) = (OXIDE * OXIDE, SILICON * SILICON);
        // each value's ε the mean over the cell around it, exact for E_z, which lies along every
        // interface
        let eps = move |x: f64, y: f64, _: f64| {
            let (y0, y1) = (y - 0.5 * h, y + 0.5 * h);
            let sub = overlap(y0, y1, f64::NEG_INFINITY, base) / h;
            let layer = overlap(y0, y1, base, top) / h;
            let fill = overlap(x - 0.5 * h, x + 0.5 * h, lo, hi) / h;
            sub * es + layer * (fill * ef + (1.0 - fill)) + (1.0 - sub - layer)
        };
        let boundaries = Boundaries {
            x: Edges::Bloch { k: 0.0 },
            y: Edges::Pml {
                low: pml,
                high: pml,
            },
            z: Edges::Bloch { k: 0.0 },
            cpml: Cpml::default(),
        };
        let mut s = Simulation::new(grid, eps, boundaries, 0.9)?;
        let f0 = 1.0 / self.wavelength;
        let pulse = Waveform::pulse(Frequency::natural(f0)?, 0.5 * f0)?;
        let source = pml + cells(0.3);
        s.add_current(Current {
            field: Field::E,
            values: (0..nx)
                .map(|i| (Axis::Z, (i, source, 0), c64::new(1.0, 0.0)))
                .collect(),
            waveform: pulse,
        })?;
        let monitor = cells(top + 0.6);
        let dft = s.add_dft(
            (0, monitor, 0),
            (nx - 1, monitor, 0),
            &[Frequency::natural(f0)?],
        )?;
        let Waveform::Gaussian { delay, .. } = pulse else {
            unreachable!("a pulse is a Gaussian")
        };
        s.run_until(2.0 * delay + self.record);
        let d = s.dft(dft);
        let sum: c64 = (0..nx)
            .filter_map(|i| d.value(Field::E, Axis::Z, (i, monitor, 0), 0))
            .sum();
        Ok(sum / nx as f64)
    }

    /// The ridges' complex transmission against no ridge's, at each width in `widths`, the runs
    /// in parallel.
    pub fn relative(&self, widths: &[f64]) -> photonoxide::Result<Vec<c64>> {
        let reference = self.transmitted(0.0)?;
        widths
            .par_iter()
            .map(|&w| Ok(self.transmitted(w)? / reference))
            .collect()
    }
}

/// How long the unit cell's run records after its pulse, µm/c: the ridges ring down in tens of
/// µm/c (checked in the tests against a run twice as long).
pub const RECORD: f64 = 60.0;

/// The waveguide picture's phase of a ridge `w` µm wide and `height` µm tall: its TE₀ in air,
/// (n_eff − 1)·2πH/λ, radians; `None` for no guided mode.
pub fn waveguide_phase(w: f64, height: f64, wavelength: f64) -> photonoxide::Result<Option<f64>> {
    let modes = Slab::new(1.0, SILICON, 1.0, Length::um(w))?
        .modes(Polarization::Te, Wavelength::um(wavelength)?);
    Ok(modes
        .first()
        .map(|m| (m.effective_index() - 1.0) * TAU * height / wavelength))
}

/// The power transmittance of the bare silica–air face at normal incidence.
pub fn bare_face() -> f64 {
    4.0 * OXIDE / (1.0 + OXIDE).powi(2)
}

/// Each element's phase, unwrapped along the list.
fn unwrap(values: &[c64]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::with_capacity(values.len());
    for v in values {
        let mut a = v.arg();
        if let Some(&last) = out.last() {
            while a - last > PI {
                a -= TAU;
            }
            while a - last < -PI {
                a += TAU;
            }
        }
        out.push(a);
    }
    out
}

type PillarKey = [u64; 5];
type PillarRun = (Vec<f64>, Vec<c64>);

fn pillar_cache() -> &'static Mutex<HashMap<PillarKey, PillarRun>> {
    static CACHE: OnceLock<Mutex<HashMap<PillarKey, PillarRun>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The widths and the relative transmissions of `cell`'s sweep, cached.
pub fn pillar_sweep(cell: &Cell) -> photonoxide::Result<PillarRun> {
    let key = [
        cell.period.to_bits(),
        cell.height.to_bits(),
        cell.wavelength.to_bits(),
        cell.per_um.to_bits(),
        cell.record.to_bits(),
    ];
    if let Some(r) = pillar_cache()
        .lock()
        .ok()
        .and_then(|c| c.get(&key).cloned())
    {
        return Ok(r);
    }
    // 30 widths from a twentieth of the period to 0.9 of it
    let widths: Vec<f64> = (0..30)
        .map(|k| cell.period * (0.05 + 0.85 * f64::from(k) / 29.0))
        .collect();
    let t = cell.relative(&widths)?;
    let r = (widths, t);
    if let Ok(mut c) = pillar_cache().lock() {
        if c.len() > 32 {
            c.clear();
        }
        c.insert(key, r.clone());
    }
    Ok(r)
}

/// Cells per µm of the pillar chart: 50, 20 nm cells, about 22 a wavelength in silicon at 1.55 µm.
pub const PILLAR_GRID: f64 = 50.0;

pub fn pillar(period_nm: f64, height_nm: f64, wavelength: f64) -> photonoxide::Result<ChartData> {
    let cell = Cell {
        period: period_nm * 1e-3,
        height: height_nm * 1e-3,
        wavelength,
        per_um: PILLAR_GRID,
        record: RECORD,
    };
    if cell.period >= wavelength / OXIDE {
        return Err(invalid(
            "period",
            format!(
                "must be below λ/n_silica = {} nm, or the lattice diffracts in the silica",
                sig(1e3 * wavelength / OXIDE, 4)
            ),
        ));
    }
    let (widths, t) = pillar_sweep(&cell)?;
    let phases = unwrap(&t);
    let t_bare = bare_face();
    let phase: Vec<[f64; 2]> = widths
        .iter()
        .zip(&phases)
        .map(|(&w, &p)| [w, p / TAU])
        .collect();
    let power: Vec<[f64; 2]> = widths
        .iter()
        .zip(&t)
        .map(|(&w, v)| [w, v.norm_sqr() * t_bare])
        .collect();
    let mut guide = Vec::new();
    let mut worst: f64 = 0.0;
    for (&w, &p) in widths.iter().zip(&phases) {
        if let Some(g) = waveguide_phase(w, cell.height, wavelength)? {
            guide.push([w, g / TAU]);
            worst = worst.max((g - p).abs());
        }
    }
    let span = phases.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - phases.iter().copied().fold(f64::INFINITY, f64::min);
    let lowest = power.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
    let series = vec![
        Curve {
            label: "phase/2π, 2D FDTD".to_owned(),
            points: phase,
            dashed: false,
            colour: Some(0),
        },
        Curve {
            label: "phase/2π, waveguide picture".to_owned(),
            points: guide,
            dashed: true,
            colour: Some(0),
        },
        Curve {
            label: "transmittance, 2D FDTD".to_owned(),
            points: power,
            dashed: false,
            colour: Some(1),
        },
    ];
    let figures = vec![
        figure(
            "phase range",
            span / TAU,
            3,
            "× 2π",
            "over the chart's widths",
        ),
        figure(
            "lowest transmittance",
            100.0 * lowest,
            3,
            "%",
            "into the zeroth order",
        ),
        figure(
            "waveguide picture's largest error",
            worst / TAU,
            3,
            "× 2π",
            "against FDTD",
        ),
        figure(
            "no ridge's transmittance",
            100.0 * t_bare,
            4,
            "%",
            "the bare silica face, exact",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "ridge width w".to_owned(),
        x_length: true,
        y_label: "phase/2π and transmittance".to_owned(),
        y_log: false,
        y_range: None,
        series,
        marker: None,
        figures,
        note: format!(
            "Silicon ridges (n = {SILICON}) {} nm tall on silica ({OXIDE}), one every {} nm, lit from the silica at normal incidence, E along the ridges: 2D FDTD between periodic sides on {} nm cells, the phase against no ridge's; the waveguide picture (n_eff − 1)·2πH/λ from each ridge's exact TE₀ as a slab in air.",
            sig(height_nm, 4),
            sig(period_nm, 4),
            sig(1e3 / PILLAR_GRID, 3)
        ),
    })
}

/// A cylindrical metalens: its focal length and numerical aperture, the pillars' period and the
/// phase levels they round to (0: the exact phase, on steps of λ/20).
#[derive(Clone, Copy, Debug)]
pub struct Lens {
    /// The focal length f at the design wavelength, µm.
    pub focal: f64,
    /// NA = sin θ at the lens's rim, in air.
    pub na: f64,
    /// The pillars' period, µm (0: the exact profile).
    pub period: f64,
    /// Phase levels (0: not rounded).
    pub levels: usize,
}

impl Lens {
    /// The half-aperture R = f tan(arcsin NA), µm.
    pub fn radius(&self) -> f64 {
        self.focal * self.na.asin().tan()
    }

    /// The lens's samples: (centre x µm, width µm, phase at the design wavelength in [0, 2π)).
    fn samples(&self) -> Vec<(f64, f64, f64)> {
        let r = self.radius();
        let k0 = TAU / DESIGN;
        let step = if self.period > 0.0 {
            self.period
        } else {
            DESIGN / 20.0
        };
        let n = (r / step).ceil() as i64;
        (-n..n)
            .map(|k| {
                let x = (k as f64 + 0.5) * step;
                let mut phi =
                    (-k0 * ((x * x + self.focal * self.focal).sqrt() - self.focal)).rem_euclid(TAU);
                if self.levels > 0 {
                    let q = TAU / self.levels as f64;
                    phi = ((phi / q).round() * q).rem_euclid(TAU);
                }
                (x, step, phi)
            })
            .filter(|(x, _, _)| x.abs() <= r)
            .collect()
    }

    /// The field at (x, z), z beyond the lens, at `wavelength` µm: the 2D Rayleigh–Sommerfeld
    /// integral in its far-zone form, (z/ρ)√(k/2πρ) e^(i(kρ − π/4)), over each sample's cell on
    /// sub-steps of at most λ/20, each pillar's phase scaled by λ₀/λ.
    pub fn field(&self, samples: &[(f64, f64, f64)], wavelength: f64, x: f64, z: f64) -> c64 {
        let k = TAU / wavelength;
        let scale = DESIGN / wavelength;
        let mut u = c64::new(0.0, 0.0);
        for &(xc, width, phi) in samples {
            let sub = (width / (wavelength / 20.0)).ceil().max(1.0) as usize;
            let dx = width / sub as f64;
            let a = c64::from_polar(1.0, phi * scale);
            let mut cell = c64::new(0.0, 0.0);
            for j in 0..sub {
                let xs = xc - 0.5 * width + (j as f64 + 0.5) * dx;
                let rho = ((x - xs).powi(2) + z * z).sqrt();
                cell += c64::from_polar((z / rho) * (k / (TAU * rho)).sqrt(), k * rho - PI / 4.0);
            }
            u += a * cell * dx;
        }
        u
    }
}

/// The full width at half maximum of `points` (x, y) around its peak, interpolated linearly.
pub fn fwhm(points: &[[f64; 2]]) -> f64 {
    let (i, peak) = points
        .iter()
        .enumerate()
        .fold((0, f64::NEG_INFINITY), |a, (i, p)| {
            if p[1] > a.1 { (i, p[1]) } else { a }
        });
    let half = 0.5 * peak;
    let cross = |range: Box<dyn Iterator<Item = usize>>, step: i64| -> f64 {
        for j in range {
            let k = (j as i64 + step) as usize;
            if points[k][1] < half {
                let (a, b) = (points[j], points[k]);
                return a[0] + (half - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            }
        }
        f64::NAN
    };
    let right = cross(Box::new(i..points.len() - 1), 1);
    let left = cross(Box::new((1..=i).rev()), -1);
    right - left
}

pub fn focus(na: f64, focal: f64, period_nm: f64, levels: usize) -> photonoxide::Result<ChartData> {
    let ideal = Lens {
        focal,
        na,
        period: 0.0,
        levels: 0,
    };
    let lens = Lens {
        focal,
        na,
        period: period_nm * 1e-3,
        levels,
    };
    let (si, sl) = (ideal.samples(), lens.samples());
    // the spot is λ/(2NA) or so: draw ±4 spots
    let half = 4.0 * DESIGN / (2.0 * na);
    let xs: Vec<f64> = (0..=400)
        .map(|k| -half + 2.0 * half * f64::from(k) / 400.0)
        .collect();
    let ii: Vec<f64> = xs
        .par_iter()
        .map(|&x| ideal.field(&si, DESIGN, x, focal).norm_sqr())
        .collect();
    let il: Vec<f64> = xs
        .par_iter()
        .map(|&x| lens.field(&sl, DESIGN, x, focal).norm_sqr())
        .collect();
    let peak = ii.iter().copied().fold(0.0, f64::max);
    let ideal_pts: Vec<[f64; 2]> = xs.iter().zip(&ii).map(|(&x, &v)| [x, v / peak]).collect();
    let lens_pts: Vec<[f64; 2]> = xs.iter().zip(&il).map(|(&x, &v)| [x, v / peak]).collect();
    let ratio = il.iter().copied().fold(0.0, f64::max) / peak;
    let rim = DESIGN / (na * lens.period);
    let figures = vec![
        figure(
            "pillars' peak against the exact profile's",
            ratio,
            3,
            "",
            "at the focus",
        ),
        figure(
            "sinc²(1/N)",
            if levels > 0 {
                sinc(1.0 / levels as f64).powi(2)
            } else {
                1.0
            },
            3,
            "",
            "the N-level staircase's share",
        ),
        figure(
            "spot's FWHM",
            fwhm(&lens_pts) / DESIGN,
            3,
            "λ",
            "the pillars'",
        ),
        figure(
            "exact profile's FWHM",
            fwhm(&ideal_pts) / DESIGN,
            3,
            "λ",
            "",
        ),
        figure("0.443/NA", 0.443 / na, 3, "λ", "a slit's, paraxial"),
        figure(
            "pillars a zone at the rim",
            rim,
            3,
            "",
            "λ/(NA p); 2 at p = λ/(2NA)",
        ),
        figure("aperture 2R", 2.0 * lens.radius(), 4, "µm", ""),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "across the focal plane x".to_owned(),
        x_length: true,
        y_label: "intensity (exact profile's peak = 1)".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.05]),
        series: vec![
            Curve {
                label: "exact phase".to_owned(),
                points: ideal_pts,
                dashed: true,
                colour: Some(GREY),
            },
            Curve {
                label: format!(
                    "pillars every {} nm, {}",
                    sig(period_nm, 4),
                    if levels > 0 {
                        format!("{levels} levels")
                    } else {
                        "exact phases".to_owned()
                    }
                ),
                points: lens_pts,
                dashed: false,
                colour: Some(0),
            },
        ],
        marker: None,
        figures,
        note: format!(
            "A cylindrical lens (2D), f = {} µm at λ = {DESIGN} µm, its phase −k(√(x² + f²) − f) wrapped and held over each pillar's cell; scalar, the far-zone form valid for kρ ≫ 1.",
            sig(focal, 4)
        ),
    })
}

pub fn axis(na: f64, focal: f64, wavelength: f64) -> photonoxide::Result<ChartData> {
    let lens = Lens {
        focal,
        na,
        period: 0.0,
        levels: 0,
    };
    let s = lens.samples();
    let zs: Vec<f64> = (0..=300)
        .map(|k| focal * (0.6 + 0.8 * f64::from(k) / 300.0))
        .collect();
    let design: Vec<f64> = zs
        .par_iter()
        .map(|&z| lens.field(&s, DESIGN, 0.0, z).norm_sqr())
        .collect();
    let other: Vec<f64> = zs
        .par_iter()
        .map(|&z| lens.field(&s, wavelength, 0.0, z).norm_sqr())
        .collect();
    let peak = design.iter().copied().fold(0.0, f64::max);
    let argmax = |v: &[f64]| {
        v.iter().enumerate().fold(
            (0, f64::NEG_INFINITY),
            |a, (i, &y)| if y > a.1 { (i, y) } else { a },
        )
    };
    let (i_d, _) = argmax(&design);
    let (i_o, top) = argmax(&other);
    let to_pts =
        |v: &[f64]| -> Vec<[f64; 2]> { zs.iter().zip(v).map(|(&z, &y)| [z, y / peak]).collect() };
    let predicted = focal * DESIGN / wavelength;
    let figures = vec![
        figure("focus at the design λ", zs[i_d], 4, "µm", ""),
        figure(
            "focus at λ",
            zs[i_o],
            4,
            "µm",
            "the chart's sampling: 0.27 % of f",
        ),
        figure(
            "f λ₀/λ",
            predicted,
            4,
            "µm",
            "a diffractive lens's, paraxial",
        ),
        figure("peak at λ against the design's", top / peak, 3, "", ""),
        figure(
            "sinc²(1 − λ₀/λ)",
            sinc(1.0 - DESIGN / wavelength).powi(2),
            3,
            "",
            "the main focus's share, scalar",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "along the axis z".to_owned(),
        x_length: true,
        y_label: "intensity on the axis (design peak = 1)".to_owned(),
        y_log: false,
        y_range: None,
        series: vec![
            Curve {
                label: format!("design λ₀ = {DESIGN} µm"),
                points: to_pts(&design),
                dashed: true,
                colour: Some(GREY),
            },
            Curve {
                label: format!("λ = {} µm", sig(wavelength, 4)),
                points: to_pts(&other),
                dashed: false,
                colour: Some(0),
            },
        ],
        marker: Some(predicted),
        figures,
        note: format!(
            "A cylindrical lens (2D) designed at {DESIGN} µm, f = {} µm, NA = {}, each step's phase a delay with no dispersion of its own; scalar. The line marks f λ₀/λ.",
            sig(focal, 4),
            sig(na, 3)
        ),
    })
}

/// Presutti and Monticone's Θ(NA/n_b) in air: √(1 − NA²)/(1 − √(1 − NA²)).
pub fn theta(na: f64) -> f64 {
    let c = (1.0 - na * na).sqrt();
    c / (1.0 - c)
}

/// Presutti and Monticone's bounds on Δω, rad/ps, for a lens in air of focal length `focal` µm,
/// thickness `thickness` µm, its highest index `index`, at the band's centre `centre` µm:
/// resonant meta-atoms (Eq. 10), waveguides (Eq. 11, Tucker's) and any structure (Eq. 12,
/// Miller's).
pub fn bounds(na: f64, focal: f64, thickness: f64, index: f64, centre: f64) -> [f64; 3] {
    let w = TAU * C_UM_PS / centre;
    let t = theta(na);
    let resonant = 2.0 * C_UM_PS / focal * t;
    let tucker = w * thickness * (index - 1.0) / focal * t;
    let miller = w * thickness * (index * index - 1.0) / (2.0 * 3f64.sqrt() * focal) * t;
    [resonant, tucker, miller]
}

pub fn bandwidth(
    na: f64,
    focal: f64,
    thickness: f64,
    index: f64,
    centre: f64,
) -> photonoxide::Result<ChartData> {
    let w = TAU * C_UM_PS / centre;
    let labels = [
        "resonant meta-atoms (Eq. 10)",
        "waveguides, Tucker's (Eq. 11)",
        "any structure, Miller's (Eq. 12)",
    ];
    let nas: Vec<f64> = (0..=180)
        .map(|k| 0.05 + 0.9 * f64::from(k) / 180.0)
        .collect();
    let series = (0..3)
        .map(|j| Curve {
            label: labels[j].to_owned(),
            points: nas
                .iter()
                .map(|&a| [a, bounds(a, focal, thickness, index, centre)[j] / w])
                .collect(),
            dashed: j == 0,
            colour: Some(j),
        })
        .collect();
    let b = bounds(na, focal, thickness, index, centre);
    // a band Δω around ω_c in wavelength: λ_c·Δω/ω_c to first order
    let nm = |dw: f64| 1e3 * centre * dw / w;
    let figures = vec![
        figure(
            "resonant (Eq. 10)",
            b[0],
            4,
            "rad/ps",
            &format!("{} nm", sig(nm(b[0]), 3)),
        ),
        figure(
            "Tucker's (Eq. 11)",
            b[1],
            4,
            "rad/ps",
            &format!("{} nm", sig(nm(b[1]), 3)),
        ),
        figure(
            "Miller's (Eq. 12)",
            b[2],
            4,
            "rad/ps",
            &format!("{} nm", sig(nm(b[2]), 3)),
        ),
        figure(
            "delay the centre needs",
            focal * ((1.0 / (1.0 - na * na)).sqrt() - 1.0) / C_UM_PS * 1e3,
            4,
            "fs",
            "Eq. 4: (F/c)(√(1 + (R/F)²) − 1)",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "numerical aperture NA".to_owned(),
        x_length: false,
        y_label: "largest band Δω/ω_c".to_owned(),
        y_log: true,
        y_range: None,
        series,
        marker: Some(na),
        figures,
        note: format!(
            "Presutti and Monticone's bounds for a lens in air, F = {} µm, L = {} µm, its highest index {} (Δn = n − 1, η_max = n² − 1), centred at {} µm. Exact closed forms.",
            sig(focal, 4),
            sig(thickness, 4),
            sig(index, 4),
            sig(centre, 4)
        ),
    })
}

pub fn specs() -> Vec<ChartSpec> {
    let na = |default: f64, max: f64| {
        param(
            "na",
            "Numerical aperture NA",
            "",
            (0.05, max, default),
            0.01,
            false,
            "sin θ of the ray from the lens's rim to its focus, in air.",
        )
    };
    let focal = |default: f64| {
        param(
            "focal",
            "Focal length $f$",
            "um",
            (5.0, 200.0, default),
            0.5,
            true,
            "The focal length at the design wavelength, 1.55 µm.",
        )
    };
    vec![
        ChartSpec {
            id: "meta-orders",
            title: "The orders of a phase-gradient grating",
            about: "A supercell of N pillars, each a period p, their phases stepping by 2π/N: the angle at which each diffraction order leaves against the incidence, and each order's share for a thin staircase of phases. Order +1 is the generalized law of refraction.",
            computed_by: "the grating equation n_t sin θ_m = n_i sin θ_i + mλ/Γ, and the staircase's Fourier series, sinc²(m/N) where m − 1 is a multiple of N, exact closed forms",
            params: vec![
                param(
                    "wavelength",
                    "Wavelength $\\lambda$",
                    "um",
                    (0.4, 2.0, 0.6),
                    0.001,
                    false,
                    "The vacuum wavelength.",
                ),
                param(
                    "period",
                    "Pillar period $p$",
                    "nm",
                    (100.0, 1000.0, 375.0),
                    1.0,
                    false,
                    "The spacing of the pillars along the gradient.",
                ),
                param(
                    "pillars",
                    "Pillars a supercell $N$",
                    "",
                    (2.0, 16.0, 4.0),
                    1.0,
                    false,
                    "How many pillars share one 2π of phase.",
                ),
                param(
                    "angle",
                    "Incidence $\\theta_i$ (°)",
                    "",
                    (-60.0, 60.0, 0.0),
                    0.5,
                    false,
                    "The incident wave's angle from the normal, in the incident medium.",
                ),
                param(
                    "n_in",
                    "Incident index $n_i$",
                    "",
                    (1.0, 2.0, 1.0),
                    0.001,
                    false,
                    "The index the light comes from.",
                ),
                param(
                    "n_out",
                    "Exit index $n_t$",
                    "",
                    (1.0, 2.0, 1.0),
                    0.001,
                    false,
                    "The index the light leaves into.",
                ),
            ],
        },
        ChartSpec {
            id: "meta-pillar",
            title: "The phase a silicon ridge sets, and what it transmits",
            about: "Silicon ridges on silica, one a period, lit from the silica at normal incidence with E along them: the phase and the power of the zeroth order they transmit against their width, from 2D FDTD of one period, and the phase the waveguide picture predicts.",
            computed_by: "fdtd::Simulation::new between periodic sides (Edges::Bloch with k = 0), a sheet of Current and add_dft, divided by the run with no ridge; mode::slab::Slab::modes for the waveguide picture",
            params: vec![
                param(
                    "period",
                    "Period $p$",
                    "nm",
                    (300.0, 1000.0, 600.0),
                    10.0,
                    false,
                    "The ridges' period; below λ/n_silica so the lattice diffracts nothing.",
                ),
                param(
                    "height",
                    "Height $H$",
                    "nm",
                    (100.0, 1600.0, 1000.0),
                    10.0,
                    false,
                    "The ridges' height.",
                ),
                param(
                    "wavelength",
                    "Wavelength $\\lambda$",
                    "um",
                    (1.2, 2.0, 1.55),
                    0.01,
                    false,
                    "The vacuum wavelength.",
                ),
            ],
        },
        ChartSpec {
            id: "meta-focus",
            title: "A metalens's focal spot",
            about: "A cylindrical metalens's intensity across its focal plane: its hyperbolic phase held over each pillar's cell and rounded to N levels, against the exact phase.",
            computed_by: "the 2D Rayleigh–Sommerfeld integral in its far-zone form over the lens's phase, summed on steps of λ/20, with no library function behind it",
            params: vec![
                na(0.6, 0.95),
                focal(20.0),
                param(
                    "period",
                    "Pillar period $p$",
                    "nm",
                    (100.0, 2000.0, 600.0),
                    10.0,
                    false,
                    "The pillars' spacing; each holds one phase across its cell.",
                ),
                param(
                    "levels",
                    "Phase levels $N$",
                    "",
                    (0.0, 32.0, 8.0),
                    1.0,
                    false,
                    "How many phases the pillars' library offers between 0 and 2π; 0 for every phase.",
                ),
            ],
        },
        ChartSpec {
            id: "meta-axis",
            title: "Where a metalens focuses another colour",
            about: "A cylindrical metalens designed at 1.55 µm, its intensity along its axis at the design wavelength and at another, each pillar's phase a delay with no dispersion of its own.",
            computed_by: "the 2D Rayleigh–Sommerfeld integral in its far-zone form over the lens's wrapped phase, scaled by λ₀/λ, summed on steps of λ₀/20, with no library function behind it",
            params: vec![
                na(0.3, 0.9),
                focal(150.0),
                param(
                    "wavelength",
                    "Wavelength $\\lambda$",
                    "um",
                    (1.2, 2.0, 1.45),
                    0.005,
                    false,
                    "The wavelength lighting the lens.",
                ),
            ],
        },
        ChartSpec {
            id: "meta-bandwidth",
            title: "How wide an achromatic metalens's band can be",
            about: "Presutti and Monticone's bounds on the band over which a metalens keeps one focal length, against its numerical aperture: for resonant meta-atoms, for waveguide pillars, and for any structure of its thickness and contrast.",
            computed_by: "Presutti and Monticone 2020's Eqs. 10, 11 and 12, exact closed forms",
            params: vec![
                na(0.24, 0.95),
                param(
                    "focal",
                    "Focal length $F$",
                    "um",
                    (1.0, 2000.0, 200.0),
                    1.0,
                    true,
                    "The lens's focal length.",
                ),
                param(
                    "thickness",
                    "Thickness $L$",
                    "um",
                    (0.1, 5.0, 0.8),
                    0.01,
                    false,
                    "The pillars' height.",
                ),
                param(
                    "index",
                    "Pillar index $n$",
                    "",
                    (1.2, 4.0, 3.674),
                    0.001,
                    false,
                    "The highest index in the lens, in air: Δn = n − 1 for Tucker's bound, η_max = n² − 1 for Miller's.",
                ),
                param(
                    "centre",
                    "Band's centre $\\lambda_c$",
                    "um",
                    (0.4, 5.0, 1.454),
                    0.001,
                    false,
                    "The wavelength at the band's centre frequency.",
                ),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use photonoxide::mode::multilayer::Multilayer;

    fn value(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f| f.label == label)
            .and_then(|f| f.value)
            .unwrap_or_else(|| panic!("no figure {label}"))
    }

    #[test]
    fn staircase_shares_are_the_fourier_series() {
        // four levels: 81.06 % into +1, the 81 % Lalanne and Chavel quote from Swanson
        assert!((staircase_efficiency(4, 1) - 0.810_569).abs() < 1e-6);
        // sixteen: 98.72 %, their "99 %"
        assert!((staircase_efficiency(16, 1) - 0.987_215).abs() < 1e-6);
        // the shares the staircase feeds add to one (orders 1 + jN)
        let total: f64 = (-4000..4000).map(|m| staircase_efficiency(4, m)).sum();
        assert!((total - 1.0).abs() < 1e-3, "{total}");
        assert_eq!(staircase_efficiency(4, 2), 0.0);
        assert_eq!(staircase_efficiency(4, 0), 0.0);
    }

    #[test]
    fn order_plus_one_is_the_generalized_law() {
        // Elsawy et al.'s supercell: 1.5 µm at 600 nm, normal incidence into air: sin θ = 0.4
        let a = order_angle(0.6, 1.5, 1.0, 1.0, 0.0, 1).unwrap();
        assert!((a - 0.4f64.asin().to_degrees()).abs() < 1e-12);
        let data = orders(0.6, 375.0, 4, 0.0, 1.0, 1.0).unwrap();
        assert!((value(&data, "order +1 leaves at") - 23.578).abs() < 1e-3);
        // too steep a gradient: λ/Γ > 2 from glancing incidence
        assert!(order_angle(1.55, 0.6, 1.0, 1.0, 0.0, 1).is_none());
    }

    #[test]
    fn the_unit_cell_matches_the_transfer_matrices_for_a_film() {
        // a ridge filling the period is a film: its transmission against the bare face's must be
        // the transfer matrices' t_film e^(−ik₀H)/t_bare, light from the silica
        let cell = Cell {
            period: 0.6,
            height: 0.3,
            wavelength: 1.55,
            per_um: PILLAR_GRID,
            record: RECORD,
        };
        let fdtd = cell.relative(&[0.6]).unwrap()[0];
        let one = |n: f64| c64::new(n, 0.0);
        let l = Wavelength::um(1.55).unwrap();
        let film = Multilayer::new(one(OXIDE), &[(one(SILICON), Length::um(0.3))], one(1.0))
            .unwrap()
            .reflection(Polarization::Te, l, 0.0)
            .unwrap()
            .t;
        let d = 1e-3;
        let bare = Multilayer::new(one(OXIDE), &[(one(1.0), Length::um(d))], one(1.0))
            .unwrap()
            .reflection(Polarization::Te, l, 0.0)
            .unwrap()
            .t;
        let k0 = TAU / 1.55;
        let expected =
            film * c64::from_polar(1.0, -k0 * 0.3) / (bare * c64::from_polar(1.0, -k0 * d));
        let err = (fdtd - expected).norm() / expected.norm();
        assert!(
            err < 0.02,
            "FDTD {fdtd}, transfer matrices {expected}, {err}"
        );
    }

    #[test]
    fn the_unit_cell_converges_and_has_rung_down() {
        let coarse = Cell {
            period: 0.6,
            height: 1.0,
            wavelength: 1.55,
            per_um: PILLAR_GRID,
            record: RECORD,
        };
        let fine = Cell {
            per_um: 2.0 * PILLAR_GRID,
            ..coarse
        };
        let a = coarse.relative(&[0.3]).unwrap()[0];
        let b = fine.relative(&[0.3]).unwrap()[0];
        let dphase = (a / b).arg().abs();
        assert!(
            dphase < 0.05 * TAU,
            "20 nm against 10 nm cells: {dphase} rad"
        );
        // recording twice as long changes the transmission by less than 0.2 % of it, far below
        // what the chart draws
        let longer = Cell {
            record: 2.0 * RECORD,
            ..coarse
        };
        let c = longer.relative(&[0.3]).unwrap()[0];
        assert!((a - c).norm() < 2e-3 * c.norm(), "{a} against {c}");
    }

    #[test]
    fn the_waveguide_picture_and_fdtd_agree_roughly_and_cover_two_pi() {
        let data = pillar(600.0, 1000.0, 1.55).unwrap();
        assert!(value(&data, "phase range") > 1.0);
        assert!(value(&data, "waveguide picture's largest error") < 0.25);
        assert!((value(&data, "no ridge's transmittance") - 96.70).abs() < 0.01);
    }

    #[test]
    fn a_metalens_focuses_to_the_slits_spot_and_staircase_share() {
        // NA 0.3: the paraxial 0.443/NA within a few percent
        let d = focus(0.3, 40.0, 100.0, 0).unwrap();
        let w = value(&d, "exact profile's FWHM");
        assert!((w - 0.443 / 0.3).abs() / w < 0.03, "{w}");
        // four levels on fine pillars: the peak falls to the staircase's 81 %
        let d4 = focus(0.3, 40.0, 100.0, 4).unwrap();
        let r = value(&d4, "pillars' peak against the exact profile's");
        assert!((r - 0.8106).abs() < 0.03, "{r}");
    }

    #[test]
    fn another_colour_focuses_at_f_lambda0_over_lambda() {
        // a lens many Fresnel zones wide (R²/λf = 14), where the paraxial focus holds: a lens of
        // one or two zones focuses short of f
        let d = axis(0.3, 200.0, 1.45).unwrap();
        let z = value(&d, "focus at λ");
        let f = value(&d, "f λ₀/λ");
        assert!((z - f).abs() / f < 0.01, "{z} against {f}");
        let peak = value(&d, "focus at the design λ");
        assert!((peak - 200.0).abs() / 200.0 < 0.01, "{peak}");
    }

    #[test]
    fn the_bounds_are_presutti_and_monticones_table_1() {
        // their design d₁, Shrestha et al.'s lens: NA 0.24, F = 200 µm, 1300 to 1650 nm,
        // L = 800 nm, ε_max = 13.5; Table 1: 99.6 (single resonator), 621 (Miller) and 460
        // (Tucker) rad/ps; the band's centre is its centre frequency's wavelength
        let (fmin, fmax) = (C_UM_PS / 1.65, C_UM_PS / 1.3);
        let centre = C_UM_PS / (0.5 * (fmin + fmax));
        let b = bounds(0.24, 200.0, 0.8, 13.5f64.sqrt(), centre);
        assert!((b[0] - 99.6).abs() < 0.1, "{}", b[0]);
        assert!((b[1] - 460.0).abs() < 1.0, "{}", b[1]);
        assert!((b[2] - 621.0).abs() < 1.0, "{}", b[2]);
    }
}
