//! The crossing and taper charts of the lesson on crossings and mode converters, on the plane of
//! the effective index method: a strip of 220 nm of silicon in silica seen from above, its core
//! the slab's TE₀ index, the 150 nm slab of gdsfactory's crossing its own slab's, the silica
//! around (Hocker and Burns 1977's method, as the MMI charts take it). The TE-like mode's E lies
//! in this plane, across the guide: TM in the plane's own terms.
//!
//! - **The crossing's light** (`crossing-field`): photonoxide's 2D FDFD (`fdfd::Solver2d`, H
//!   along z) of a plain crossing of two 500 nm guides, of gdsfactory's crossing without and
//!   with its ellipses (Liu and Poon 2025, Section 3.2), or of arms widened to a width of the
//!   reader's choice: |H_z| from above, and where the power goes.
//! - **The crossing's arms** (`crossing-width`): eigenmode expansion (`eme::Eme`, Bienstman and
//!   Baets 2001) of the same crossings, its sections cut across the through arm: the side arm
//!   fills the window where it crosses, so the expansion holds the crossing exactly. The
//!   through port's TE₀ against the width the arms widen to, beside the Gaussian beam's
//!   diffraction across the open intersection.
//! - **The taper** (`taper-length`): eigenmode expansion of a guide widened from 500 nm, its
//!   TE₀ through against the taper's length for a linear, a parabolic and an exponential
//!   profile, and the adiabatic parameter the coupled local modes give.

use photonoxide::portable;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::{Mutex, OnceLock};

use num_complex::Complex64 as c64;
use photonoxide::eme::{Eme, Section, Slice, Wall};
use photonoxide::fdfd::{Boundaries, Direction, Grid, Polarization as Plane, PortMode, Solver2d};
use photonoxide::material::{silica, silicon};
use photonoxide::mode::Polarization;
use photonoxide::mode::slab::{Slab, SlabMode};
use photonoxide::raster::Raster;
use photonoxide::units::{Length, Wavelength};

use super::{ChartData, ChartSpec, Curve, FieldMap, Figure, figure, invalid, param};

/// The wavelength of every chart here, µm.
pub const LAMBDA: f64 = 1.55;
/// The guides' width, µm, and the crossing's arms' half-width beyond their tapers.
pub const GUIDE: f64 = 0.5;
/// gdsfactory's crossing: the square at the centre, µm, and the tapers to it (Liu and Poon
/// 2025, Section 3.2 and Fig. 5).
pub const GDS_CENTRE: f64 = 1.2;
pub const GDS_TAPER: f64 = 3.4;
/// The ellipses in the 150 nm slab: semi-axes along and across their arm, µm.
pub const ELLIPSE: [f64; 2] = [3.0, 1.1];

fn wavelength() -> photonoxide::Result<Wavelength> {
    Wavelength::um(LAMBDA)
}

/// The plane's indices: the 220 nm slab's TE₀ (the core), the 150 nm slab's TE₀ and silica's,
/// at 1.55 µm (Li's silicon, Malitson's silica).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Indices {
    pub core: f64,
    pub slab: f64,
    pub clad: f64,
}

pub fn indices() -> photonoxide::Result<Indices> {
    let l = wavelength()?;
    let si = silicon().refractive_index(l)?.re;
    let ox = silica().refractive_index(l)?.re;
    let te0 = |t: f64| -> photonoxide::Result<f64> {
        Slab::new(ox, si, ox, Length::um(t))?
            .modes(Polarization::Te, l)
            .first()
            .map(|m| m.effective_index())
            .ok_or_else(|| invalid("crossing chart", "the slab has no TE mode"))
    };
    Ok(Indices {
        core: te0(0.22)?,
        slab: te0(0.15)?,
        clad: ox,
    })
}

/// A crossing of two equal arms: each 500 nm guide widened to `centre` µm over `taper` µm on
/// either side of a `centre` square, with gdsfactory's two ellipses of 150 nm slab if `ellipses`.
/// A plain crossing has `centre` 0.5 and no taper.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crossing {
    pub centre: f64,
    pub taper: f64,
    pub ellipses: bool,
}

impl Crossing {
    pub const PLAIN: Crossing = Crossing {
        centre: GUIDE,
        taper: 0.0,
        ellipses: false,
    };

    #[cfg(test)]
    pub fn gdsfactory(ellipses: bool) -> Crossing {
        Crossing {
            centre: GDS_CENTRE,
            taper: GDS_TAPER,
            ellipses,
        }
    }

    /// The arm's half-width at a distance `s` µm from the centre along it.
    pub fn half(&self, s: f64) -> f64 {
        let (c, g) = (self.centre / 2.0, GUIDE / 2.0);
        let s = s.abs();
        if s <= c {
            c
        } else if s < c + self.taper {
            c - (c - g) * (s - c) / self.taper
        } else {
            g
        }
    }

    /// How far the arms reach: the centre, the tapers and a micrometre of guide.
    pub fn reach(&self) -> f64 {
        self.centre / 2.0 + self.taper + 1.0
    }

    /// The material at (x, z), z along the through arm and x across it: 2 for silicon's full
    /// height, 1 for the 150 nm slab, 0 for silica.
    pub fn material(&self, x: f64, z: f64) -> u8 {
        if x.abs() <= self.half(z) || z.abs() <= self.half(x) {
            2
        } else if self.ellipses && self.in_ellipses(x, z) {
            1
        } else {
            0
        }
    }

    fn in_ellipses(&self, x: f64, z: f64) -> bool {
        let [a, b] = ELLIPSE;
        portable::powi(z / a, 2) + portable::powi(x / b, 2) <= 1.0
            || portable::powi(x / a, 2) + portable::powi(z / b, 2) <= 1.0
    }

    /// The through arm's cross-section at z, from the axis out: the extent of silicon (with the
    /// side arm's, where it crosses), then of the slab.
    fn extents(&self, z: f64) -> (f64, f64) {
        let g = GUIDE / 2.0;
        let c = self.centre / 2.0;
        let side = if z.abs() <= g {
            f64::INFINITY
        } else if z.abs() <= c && self.taper > 0.0 {
            c + self.taper * (c - z.abs()) / (c - g)
        } else {
            0.0
        };
        let core = self.half(z).max(side);
        let slab = if self.ellipses {
            let [a, b] = ELLIPSE;
            let along = if z.abs() < a {
                b * (1.0 - portable::powi(z / a, 2)).sqrt()
            } else {
                0.0
            };
            let across = if z.abs() < b {
                a * (1.0 - portable::powi(z / b, 2)).sqrt()
            } else {
                0.0
            };
            along.max(across)
        } else {
            0.0
        };
        (core, slab)
    }
}

// ---------------------------------------------------------------------------------------------
// eigenmode expansion

/// How the expansion is set: its half window inside the PML, µm, the modes, and the longest
/// step along z, µm (each the cross-section at its middle).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub window: f64,
    pub modes: usize,
    pub step: f64,
}

/// The charts' settings, and the PML (Bienstman and Baets's, as the `eme-taper` job takes it).
pub const SETTINGS: Settings = Settings {
    window: 2.0,
    modes: 40,
    step: 0.1,
};
const EME_PML: (f64, f64) = (4.0, 0.1);

fn eme(modes: usize) -> photonoxide::Result<Eme> {
    // TM in the plane (E across the guide), even across the axis: an electric wall there
    Eme::new(Polarization::Tm, wavelength()?, modes)?
        .with_walls(Wall::Electric, Wall::Electric)
        .with_upper_pml(Length::um(EME_PML.0), EME_PML.1)
}

/// The half slice from the axis out: silicon to `core`, slab to `slab`, silica to `window`.
fn slice(n: &Indices, core: f64, slab: f64, window: f64) -> photonoxide::Result<Slice> {
    let w = window;
    let core = core.min(w);
    let mut layers = vec![(c64::new(n.core, 0.0), Length::um(core))];
    let slab = slab.min(w);
    if slab > core + 1e-9 {
        layers.push((c64::new(n.slab, 0.0), Length::um(slab - core)));
    }
    let last = slab.max(core);
    if w > last + 1e-9 {
        layers.push((c64::new(n.clad, 0.0), Length::um(w - last)));
    }
    Slice::new(&layers)
}

/// The expansion's result: the through TE₀'s and reflected TE₀'s shares of the power.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Through {
    pub through: f64,
    pub reflected: f64,
}

fn fundamental(first: &[c64]) -> usize {
    first
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.re.total_cmp(&b.1.re))
        .map_or(0, |(k, _)| k)
}

/// The crossing by eigenmode expansion, the input and output guides 0.5 µm long. The steps
/// break where the cross-section has a kink or a jump: the ends of the centre, of the tapers and
/// of the ellipses, and where the side arm's guide meets the through arm's.
pub fn eme_crossing(c: &Crossing, set: Settings) -> photonoxide::Result<Through> {
    let n = indices()?;
    let reach = c.centre / 2.0 + c.taper;
    let mut breaks = vec![reach, c.centre / 2.0, GUIDE / 2.0, 0.0];
    if c.ellipses {
        breaks.extend(ELLIPSE);
    }
    let mut cuts: Vec<f64> = breaks
        .iter()
        .flat_map(|&b| [b, -b])
        .filter(|b| b.abs() <= reach)
        .collect();
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    let lead = slice(&n, GUIDE / 2.0, c.extents(reach + 1.0).1, set.window)?;
    let mut sections = vec![Section::new(lead.clone(), Length::um(0.5))];
    for pair in cuts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let steps = ((b - a) / set.step).ceil().max(1.0) as usize;
        let h = (b - a) / steps as f64;
        for k in 0..steps {
            let (core, slab) = c.extents(a + (k as f64 + 0.5) * h);
            sections.push(Section::new(
                slice(&n, core, slab, set.window)?,
                Length::um(h),
            ));
        }
    }
    sections.push(Section::new(lead, Length::um(0.5)));
    let s = eme(set.modes)?.s_matrix(&sections)?;
    let (p, q) = (fundamental(&s.first), fundamental(&s.last));
    let r = Through {
        through: s.s21[q][p].norm_sqr(),
        reflected: s.s11[p][p].norm_sqr(),
    };
    // a passive crossing returns no more than it was sent: an expansion that does has not
    // converged in its window (wide centres with short tapers, whose light reaches the PML)
    if r.through + r.reflected > 1.0 + 1e-3 {
        return Err(invalid(
            "crossing chart",
            format!(
                "the expansion returns {:.3} of the input: not converged in its window",
                r.through + r.reflected
            ),
        ));
    }
    Ok(r)
}

/// The Gaussian beam's estimate of the through power: a beam of waist w₀ (the mode's, fitted)
/// crossing an open gap of length `d` in the core's index n keeps
/// [1 + (d/2z_R)²]^(−1/2) of its power in the waist's mode, z_R = π n w₀²/λ.
pub fn gaussian_through(w0: f64, n: f64, d: f64) -> f64 {
    let zr = PI * n * w0 * w0 / LAMBDA;
    1.0 / (1.0 + portable::powi(d / (2.0 * zr), 2)).sqrt()
}

/// Points across a guide `w` µm wide in the plane, from 3 µm beyond one face to 3 beyond the
/// other, and their spacing, for the sums below.
fn across(w: f64, points: usize) -> (Vec<f64>, f64) {
    let span = w / 2.0 + 3.0;
    let dx = 2.0 * span / points as f64;
    (
        (0..points).map(|k| -span + (k as f64 + 0.5) * dx).collect(),
        dx,
    )
}

/// The `order`-th TE (E along the uniform direction: the scalar wave) or TM mode of a guide `w`
/// µm wide in the plane.
fn plane_mode(
    n: &Indices,
    w: f64,
    polarization: Polarization,
    order: usize,
) -> photonoxide::Result<Option<SlabMode>> {
    Ok(Slab::new(n.clad, n.core, n.clad, Length::um(w))?
        .modes(polarization, wavelength()?)
        .into_iter()
        .nth(order))
}

/// The field of a plane mode at x, x = 0 on the guide's axis.
fn on_axis(m: &SlabMode, w: f64, x: f64) -> f64 {
    m.field(Length::um(x + w / 2.0)).0
}

/// The best Gaussian for a guide `w` µm wide in the plane: the waist w₀ whose beam its TE₀ (the
/// scalar field, summed on 4000 points) overlaps most, and that overlap.
pub fn fitted_waist(n: &Indices, w: f64) -> photonoxide::Result<(f64, f64)> {
    let mode = plane_mode(n, w, Polarization::Te, 0)?
        .ok_or_else(|| invalid("crossing chart", "the guide has no mode"))?;
    let (xs, dx) = across(w, 4000);
    let field: Vec<f64> = xs.iter().map(|&x| on_axis(&mode, w, x)).collect();
    let own: f64 = field.iter().map(|f| f * f).sum::<f64>() * dx;
    let overlap = |w0: f64| -> f64 {
        let (mut fg, mut gg) = (0.0, 0.0);
        for (x, f) in xs.iter().zip(&field) {
            let g = portable::exp(-portable::powi(x / w0, 2));
            fg += f * g;
            gg += g * g;
        }
        portable::powi(fg * dx, 2) / (own * gg * dx)
    };
    // golden section on the waist
    let (mut a, mut b) = (0.05, w + 2.0);
    let r = 0.5 * (5f64.sqrt() - 1.0);
    for _ in 0..80 {
        let (c, d) = (b - r * (b - a), a + r * (b - a));
        if overlap(c) > overlap(d) {
            b = d;
        } else {
            a = c;
        }
    }
    let w0 = 0.5 * (a + b);
    Ok((w0, overlap(w0)))
}

// ---------------------------------------------------------------------------------------------
// 2D FDFD

/// The FDFD grid's PML, in cells, and its cells, µm.
const FDFD_PML: usize = 20;
pub const FDFD_STEP: f64 = 0.025;

/// Where the light of a crossing goes, by 2D FDFD: the through and reflected TE₀, the power
/// crossing the lines y = ±(R − 0.5) µm into the side arms' ends (and whatever radiates that
/// way), as shares of the input; and |H_z| over the crossing.
#[derive(Clone, Debug)]
pub struct Fdfd {
    pub through: f64,
    pub reflected: f64,
    pub up: f64,
    pub down: f64,
    pub picture: Raster,
}

pub fn fdfd_crossing(c: &Crossing, h: f64) -> photonoxide::Result<Fdfd> {
    let n = indices()?;
    let reach = c.reach();
    let cells = (2.0 * reach / h).round() as usize;
    let grid = Grid {
        nx: cells + 2 * FDFD_PML,
        ny: cells + 2 * FDFD_PML,
        dx: h,
        dy: h,
        x0: -reach - FDFD_PML as f64 * h,
        y0: -reach - FDFD_PML as f64 * h,
    };
    let eps = [n.clad * n.clad, n.slab * n.slab, n.core * n.core];
    let crossing = *c;
    // the through arm along x, the side arm along y
    let solver = Solver2d::new(
        grid,
        Plane::Hz,
        wavelength()?,
        move |x, y| c64::new(eps[crossing.material(y, x) as usize], 0.0),
        Boundaries::pml(FDFD_PML),
    )?;
    let column = |x: f64| ((x - grid.x0) / h).floor() as usize;
    let row = |y: f64| ((y - grid.y0) / h).floor() as usize;
    let mode = |x: f64| -> photonoxide::Result<PortMode> {
        solver
            .port_modes(column(x), 1)?
            .into_iter()
            .next()
            .ok_or_else(|| invalid("crossing-field", "no port mode"))
    };
    let (left, right) = (mode(-reach + 0.3)?, mode(reach - 0.5)?);
    let field = solver.solve_system(&solver.mode_source(&left, Direction::Forward))?;
    let input = left.power(h);
    let (_, back) = field.mode_amplitudes(&left);
    let (out, _) = field.mode_amplitudes(&right);
    // the face between rows j and j + 1, and its mirror image across y = 0
    let j = row(reach - 0.5);
    let up = field.flux_y(j) / input;
    let down = -field.flux_y(grid.ny - 2 - j) / input;
    let mut values = Vec::with_capacity(cells * cells);
    for j in FDFD_PML..FDFD_PML + cells {
        for i in FDFD_PML..FDFD_PML + cells {
            values.push(portable::complex::abs(field.at(i, j)));
        }
    }
    let top = values.iter().fold(0.0f64, |m, v| m.max(*v)).max(1e-300);
    Ok(Fdfd {
        through: out.norm_sqr() * right.power(h) / input,
        reflected: back.norm_sqr(),
        up,
        down,
        picture: Raster {
            nx: cells,
            ny: cells,
            x0: -reach,
            x1: reach,
            y0: -reach,
            y1: reach,
            values: values.iter().map(|v| (v / top) as f32).collect(),
        },
    })
}

/// The power a linear taper from 500 nm to `to` µm over `length` µm delivers from one guide's
/// TE₀ into the other's, by 2D FDFD on cells `h` µm (H along z), a micrometre of guide either side.
#[cfg(test)]
pub fn fdfd_taper(to: f64, length: f64, h: f64) -> photonoxide::Result<f64> {
    let n = indices()?;
    let (x0, x1) = (-1.0, length + 1.0);
    let half = to / 2.0 + 1.5;
    let (nx, ny) = (
        ((x1 - x0) / h).round() as usize,
        (2.0 * half / h).round() as usize,
    );
    let grid = Grid {
        nx: nx + 2 * FDFD_PML,
        ny: ny + 2 * FDFD_PML,
        dx: h,
        dy: h,
        x0: x0 - FDFD_PML as f64 * h,
        y0: -half - FDFD_PML as f64 * h,
    };
    let (core, clad) = (n.core * n.core, n.clad * n.clad);
    let width = move |x: f64| Profile::Linear.width(GUIDE, to, (x / length).clamp(0.0, 1.0));
    let solver = Solver2d::new(
        grid,
        Plane::Hz,
        wavelength()?,
        move |x, y| c64::new(if y.abs() < width(x) / 2.0 { core } else { clad }, 0.0),
        Boundaries::pml(FDFD_PML),
    )?;
    let column = |x: f64| ((x - grid.x0) / h).floor() as usize;
    let mode = |x: f64| -> photonoxide::Result<PortMode> {
        solver
            .port_modes(column(x), 1)?
            .into_iter()
            .next()
            .ok_or_else(|| invalid("taper-length", "no port mode"))
    };
    let (left, right) = (mode(-0.7)?, mode(length + 0.5)?);
    let field = solver.solve_system(&solver.mode_source(&left, Direction::Forward))?;
    let (out, _) = field.mode_amplitudes(&right);
    Ok(out.norm_sqr() * right.power(h) / left.power(h))
}

/// The crossing's outline from above, µm: the arms' silicon, then the ellipses' slab.
fn outlines(c: &Crossing) -> Vec<Vec<[f64; 2]>> {
    let r = c.reach();
    let (g, w, t) = (GUIDE / 2.0, c.centre / 2.0, c.centre / 2.0 + c.taper);
    let mut edge = Vec::new();
    for (s, co) in [(0.0, 1.0), (1.0, 0.0), (0.0, -1.0), (-1.0, 0.0)] {
        let turn = |[x, y]: [f64; 2]| [co * x - s * y, s * x + co * y];
        edge.push(turn([w, -w]));
        edge.push(turn([t, -g]));
        edge.push(turn([r, -g]));
        edge.push(turn([r, g]));
        edge.push(turn([t, g]));
    }
    edge.push(edge[0]);
    let mut out = vec![edge];
    if c.ellipses {
        let [a, b] = ELLIPSE;
        for (ax, by) in [(a, b), (b, a)] {
            out.push(
                (0..=96)
                    .map(|k| {
                        let t = 2.0 * PI * k as f64 / 96.0;
                        [ax * portable::cos(t), by * portable::sin(t)]
                    })
                    .collect(),
            );
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// the taper

/// A taper's profile: how its width goes from w₁ to w₂ over its length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Profile {
    Linear,
    /// The width squared grows linearly.
    Parabolic,
    /// The width grows by the same factor each micrometre.
    Exponential,
}

impl Profile {
    pub const ALL: [Profile; 3] = [Profile::Linear, Profile::Parabolic, Profile::Exponential];

    /// The width at `t`, from 0 at the narrow end to 1 at the wide.
    pub fn width(self, from: f64, to: f64, t: f64) -> f64 {
        match self {
            Profile::Linear => from + (to - from) * t,
            Profile::Parabolic => (from * from + (to * to - from * from) * t).sqrt(),
            Profile::Exponential => from * portable::powf(to / from, t),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Profile::Linear => "linear",
            Profile::Parabolic => "parabolic",
            Profile::Exponential => "exponential",
        }
    }
}

/// The steps of a taper's staircase.
pub const TAPER_STEPS: usize = 40;

fn guide_slice(n: &Indices, w: f64, window: f64) -> photonoxide::Result<Slice> {
    Slice::new(&[
        (c64::new(n.core, 0.0), Length::um(w / 2.0)),
        (c64::new(n.clad, 0.0), Length::um(window - w / 2.0)),
    ])
}

/// The power a 500 nm guide's TE₀ delivers into the TE₀ of a guide `to` µm wide through a taper
/// `length` µm long of `profile`, in `steps` equal steps (a length of 0: the abrupt junction),
/// with `modes` modes in a half window of `to`/2 + 1.5 µm.
pub fn taper_through(
    profile: Profile,
    to: f64,
    length: f64,
    steps: usize,
    modes: usize,
) -> photonoxide::Result<f64> {
    let n = indices()?;
    let window = to / 2.0 + 1.5;
    let eme = eme(modes)?;
    let mut sections = vec![Section::new(
        guide_slice(&n, GUIDE, window)?,
        Length::um(0.5),
    )];
    if length > 0.0 {
        sections.extend(Section::sampled(Length::um(length), steps, |z| {
            guide_slice(&n, profile.width(GUIDE, to, z.to_um() / length), window)
        })?);
    }
    sections.push(Section::new(guide_slice(&n, to, window)?, Length::um(0.5)));
    let s = eme.s_matrix(&sections)?;
    let (p, q) = (fundamental(&s.first), fundamental(&s.last));
    Ok(s.s21[q][p].norm_sqr())
}

/// The abrupt junction's closed form: the share of power the wider guide's TE₀ takes from the
/// narrow one's when nothing is reflected, Re(⟨1, 2⟩⟨2, 1⟩)/(⟨1, 1⟩⟨2, 2⟩) with
/// ⟨a, b⟩ = ∫ E_a H_b dx, E_x = n_eff H_y/n² (TM in the plane), the exact slab modes summed on
/// 20 000 points.
pub fn junction_overlap(to: f64) -> photonoxide::Result<f64> {
    let n = indices()?;
    let mode = |w: f64| -> photonoxide::Result<SlabMode> {
        plane_mode(&n, w, Polarization::Tm, 0)?.ok_or_else(|| invalid("taper-length", "no TM mode"))
    };
    let (a, b) = (mode(GUIDE)?, mode(to)?);
    let (xs, dx) = across(to, 20000);
    let eps = |w: f64, x: f64| portable::powi(if x.abs() < w / 2.0 { n.core } else { n.clad }, 2);
    let (mut ab, mut ba, mut aa, mut bb) = (0.0, 0.0, 0.0, 0.0);
    for &x in &xs {
        let (ha, hb) = (on_axis(&a, GUIDE, x), on_axis(&b, to, x));
        let ea = a.effective_index() * ha / eps(GUIDE, x);
        let eb = b.effective_index() * hb / eps(to, x);
        ab += ea * hb;
        ba += eb * ha;
        aa += ea * ha;
        bb += eb * hb;
    }
    let _ = dx;
    Ok(ab * ba / (aa * bb))
}

/// The coupled local modes' adiabatic parameter of a taper, in the scalar form (the plane's TE):
/// at each point the coupling of TE₀ to TE₂,
/// C = k²(n₁² − n₂²) w′ ψ₀ψ₂ (at the edge, ∫ψ² = 1) / (β₀² − β₂²), over their beat β₀ − β₂; the
/// largest along the taper where TE₂ is guided (0 if it never is).
pub fn adiabatic_parameter(profile: Profile, to: f64, length: f64) -> photonoxide::Result<f64> {
    let n = indices()?;
    let k = 2.0 * PI / LAMBDA;
    let mut worst = 0.0f64;
    let points = 100;
    for s in 0..=points {
        let t = s as f64 / points as f64;
        let w = profile.width(GUIDE, to, t);
        let (t0, t1) = ((t - 1e-4).max(0.0), (t + 1e-4).min(1.0));
        let slope =
            (profile.width(GUIDE, to, t1) - profile.width(GUIDE, to, t0)) / ((t1 - t0) * length);
        let (Some(m0), Some(m2)) = (
            plane_mode(&n, w, Polarization::Te, 0)?,
            plane_mode(&n, w, Polarization::Te, 2)?,
        ) else {
            continue;
        };
        let edge = |m: &SlabMode| -> f64 {
            let (xs, dx) = across(w, 2000);
            let norm: f64 = xs
                .iter()
                .map(|&x| portable::powi(on_axis(m, w, x), 2))
                .sum::<f64>()
                * dx;
            on_axis(m, w, w / 2.0) / norm.sqrt()
        };
        let (b0, b2) = (k * m0.effective_index(), k * m2.effective_index());
        let c = k * k * (n.core * n.core - n.clad * n.clad) * slope * edge(&m0) * edge(&m2)
            / (b0 * b0 - b2 * b2);
        worst = worst.max(c.abs() / (b0 - b2));
    }
    Ok(worst)
}

// ---------------------------------------------------------------------------------------------
// the charts

/// The chart specs.
pub fn specs() -> Vec<ChartSpec> {
    vec![
        ChartSpec {
            id: "crossing-field",
            title: "Light through a crossing, from above",
            about: "A crossing of two 500 nm silicon guides on the plane of the effective index method at 1.55 µm, TE₀ sent in from the left: a plain cross, arms widened to a width W over tapers of length L_t, or the same with gdsfactory's two ellipses of 150 nm slab. |H_z| from above (E lies in the plane), the TE₀ that leaves through the far arm, what is reflected, and the power that crosses into the side arms' ends.",
            computed_by: "fdfd::Solver2d::new (H along z, 25 nm cells, 8 × 8 samples per cell), port_modes, mode_source, solve_system, Field2d::mode_amplitudes and flux_y; mode::slab::Slab::modes for the plane's indices; eme::Eme::s_matrix on the same crossing for the cross-check",
            params: vec![
                param(
                    "kind",
                    "Crossing",
                    "",
                    (1.0, 3.0, 3.0),
                    1.0,
                    false,
                    "1: a plain cross of 500 nm guides; 2: arms widened to W over L_t; 3: the same with gdsfactory's ellipses of 150 nm slab.",
                ),
                width_param((0.5, 2.0, GDS_CENTRE)),
                taper_param((0.5, 5.0, GDS_TAPER)),
            ],
        },
        ChartSpec {
            id: "crossing-width",
            title: "A crossing's through port against its arms' width",
            about: "The same crossings by eigenmode expansion: the TE₀ through the far arm against the width W the arms widen to at the centre, over tapers of length L_t, without and with gdsfactory's ellipses, beside a Gaussian beam's estimate for the open intersection, [1 + (W/2z_R)²]^(−1/2) with the waist fitted to the W-wide guide's mode.",
            computed_by: "eme::Eme::s_matrix (TM in the plane, 40 modes, a half window of 2 µm inside a PML 4 µm thick, steps of at most 0.1 µm broken at every kink) on 16 widths; mode::slab::Slab::modes for the fitted waist",
            params: vec![
                taper_param((3.0, 6.0, GDS_TAPER)),
                width_param((0.5, 2.0, GDS_CENTRE)),
            ],
        },
        ChartSpec {
            id: "taper-length",
            title: "A taper's transmission against its length",
            about: "A 500 nm guide widened to w₂ on the plane of the effective index method at 1.55 µm, linearly, parabolically (the width squared growing linearly) or exponentially: the power its TE₀ delivers into the wide guide's TE₀ against the taper's length, from the abrupt junction to an adiabatic taper, and the coupled local modes' adiabatic parameter at the marked length.",
            computed_by: "eme::Eme::s_matrix (TM in the plane, 40 modes, a half window of w₂/2 + 1.5 µm inside a PML 4 µm thick, steps of at most 0.25 µm and at least 40) at 15 lengths; mode::slab::Slab::modes for the abrupt junction's overlap and the adiabatic parameter",
            params: vec![
                param(
                    "to",
                    "Wide end $w_2$",
                    "um",
                    (1.0, 6.0, 3.0),
                    0.05,
                    false,
                    "The width the taper ends at, µm.",
                ),
                param(
                    "length",
                    "Length $L$",
                    "um",
                    (0.5, 40.0, 10.0),
                    0.1,
                    true,
                    "The length marked, µm, and the adiabatic parameter's.",
                ),
            ],
        },
    ]
}

fn width_param(range: (f64, f64, f64)) -> super::Param {
    param(
        "width",
        "Centre width $W$",
        "um",
        range,
        0.05,
        false,
        "The width the arms widen to at the crossing, µm.",
    )
}

fn taper_param(range: (f64, f64, f64)) -> super::Param {
    param(
        "taper",
        "Taper $L_t$",
        "um",
        range,
        0.1,
        false,
        "The length of each arm's taper from 500 nm to W, µm.",
    )
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

/// `f` of each of `items` on up to 12 threads, in their order.
pub fn parallel<T: Sync, R: Send>(
    items: &[T],
    f: impl Fn(&T) -> photonoxide::Result<R> + Sync,
) -> photonoxide::Result<Vec<R>> {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(1, 12);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let slots: Vec<Mutex<Option<photonoxide::Result<R>>>> =
        items.iter().map(|_| Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..threads.min(items.len()) {
            s.spawn(|| {
                loop {
                    let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if k >= items.len() {
                        break;
                    }
                    let r = f(&items[k]);
                    if let Ok(mut slot) = slots[k].lock() {
                        *slot = Some(r);
                    }
                }
            });
        }
    });
    slots
        .into_iter()
        .map(|m| {
            m.into_inner()
                .ok()
                .flatten()
                .unwrap_or_else(|| Err(invalid("crossing chart", "a solve didn't finish")))
        })
        .collect()
}

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: usize) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: Some(colour),
    }
}

fn db(t: f64) -> f64 {
    10.0 * portable::log10(t)
}

/// The crossing a chart's `kind`, `width` and `taper` name.
pub fn chosen(kind: f64, width: f64, taper: f64) -> Crossing {
    if kind < 1.5 {
        Crossing::PLAIN
    } else {
        Crossing {
            centre: width,
            taper,
            ellipses: kind > 2.5,
        }
    }
}

/// The crossing's FDFD solve, cached.
pub fn fdfd_cached(c: &Crossing) -> photonoxide::Result<Fdfd> {
    static CACHE: OnceLock<Cache<Fdfd>> = OnceLock::new();
    let tag = if c.ellipses { 1.0 } else { 0.0 };
    CACHE
        .get_or_init(Cache::new)
        .get_or(key(c.centre, c.taper, tag), || fdfd_crossing(c, FDFD_STEP))
}

pub fn field(kind: f64, width: f64, taper: f64) -> photonoxide::Result<ChartData> {
    let c = chosen(kind, width, taper);
    let f = fdfd_cached(&c)?;
    let e = eme_crossing(&c, SETTINGS).map_or(f64::NAN, |e| e.through);
    let rest = 1.0 - f.through - f.reflected - f.up - f.down;
    let name = if kind < 1.5 {
        "a plain cross of 500 nm guides".to_owned()
    } else {
        format!(
            "arms widened to {:.2} µm over {:.1} µm{}",
            c.centre,
            c.taper,
            if c.ellipses {
                ", with gdsfactory's ellipses"
            } else {
                ""
            }
        )
    };
    let figures = vec![
        figure("Through, TE₀", f.through, 4, "", "2D FDFD"),
        figure("Through, in dB", db(f.through), 3, "dB", ""),
        Figure {
            label: "Through, TE₀, by eigenmode expansion".to_owned(),
            value: e.is_finite().then_some(e),
            text: if e.is_finite() {
                super::sig(e, 4)
            } else {
                "not converged".to_owned()
            },
            unit: String::new(),
            note: "the cross-check: 40 modes in a half window of 2 µm".to_owned(),
        },
        figure("Reflected, TE₀", f.reflected, 2, "", ""),
        figure(
            "Into each side arm's end",
            0.5 * (f.up + f.down),
            3,
            "",
            "the power crossing y = ±(R − 0.5) µm, and whatever radiates that way",
        ),
        figure(
            "Elsewhere",
            rest,
            2,
            "",
            "radiated forward and back, and the through arm's other modes",
        ),
    ];
    Ok(ChartData {
        maps: vec![FieldMap {
            label: format!("$\\lvert H_z\\rvert$ from above: {name}"),
            raster: f.picture,
            outline: outlines(&c),
        }],
        x_label: "x".to_owned(),
        x_length: true,
        y_label: "y".to_owned(),
        y_log: false,
        y_range: None,
        series: Vec::new(),
        marker: None,
        figures,
        note: format!(
            "2D FDFD on the plane of the effective index method at {LAMBDA} µm: the core {:.4} (the TE₀ of 220 nm of silicon in silica), the 150 nm slab {:.4}, silica {:.4}; H along z on 25 nm cells with PMLs of 20, TE₀ launched by its port mode from the left.",
            indices()?.core,
            indices()?.slab,
            indices()?.clad
        ),
    })
}

/// The widths of the crossing-width chart, µm.
fn widths() -> Vec<f64> {
    (0..16).map(|k| 0.5 + 0.1 * f64::from(k)).collect()
}

pub fn width_chart(taper: f64, width: f64) -> photonoxide::Result<ChartData> {
    static CACHE: OnceLock<Cache<Vec<[f64; 3]>>> = OnceLock::new();
    let sweep = CACHE
        .get_or_init(Cache::new)
        .get_or(key(taper, 0.0, 0.0), || {
            let cases: Vec<(f64, bool)> = widths()
                .into_iter()
                .flat_map(|w| [(w, false), (w, true)])
                .collect();
            let found = parallel(&cases, |&(w, ellipses)| {
                let c = Crossing {
                    centre: w,
                    taper: if w > GUIDE + 1e-9 { taper } else { 0.0 },
                    ellipses,
                };
                Ok(eme_crossing(&c, SETTINGS).map_or(f64::NAN, |e| e.through))
            })?;
            Ok(widths()
                .iter()
                .enumerate()
                .map(|(k, &w)| [w, found[2 * k], found[2 * k + 1]])
                .collect())
        })?;
    let n = indices()?;
    let gauss: Vec<[f64; 2]> = widths()
        .iter()
        .map(|&w| -> photonoxide::Result<[f64; 2]> {
            let (w0, _) = fitted_waist(&n, w)?;
            Ok([w, gaussian_through(w0, n.core, w)])
        })
        .collect::<photonoxide::Result<_>>()?;
    let at = |col: usize| -> f64 {
        // the swept value at the marked width, linearly between samples
        let k = sweep
            .windows(2)
            .position(|p| width >= p[0][0] && width <= p[1][0])
            .unwrap_or(0);
        let (a, b) = (sweep[k], sweep[(k + 1).min(sweep.len() - 1)]);
        if (b[0] - a[0]).abs() < 1e-12 {
            a[col]
        } else {
            a[col] + (b[col] - a[col]) * (width - a[0]) / (b[0] - a[0])
        }
    };
    let (w0, fit) = fitted_waist(&n, width)?;
    let plain = eme_crossing(&Crossing::PLAIN, SETTINGS)?.through;
    let figures = vec![
        figure("Plain cross, through", plain, 4, "", "no widening"),
        figure("At W, the arms alone", at(1), 4, "", "between the samples"),
        figure(
            "At W, with the ellipses",
            at(2),
            4,
            "",
            "between the samples",
        ),
        figure(
            "Gaussian estimate at W",
            gaussian_through(w0, n.core, width),
            4,
            "",
            "",
        ),
        figure(
            "Fitted waist $w_0$",
            w0,
            3,
            "µm",
            &format!("its overlap with the guide's mode {fit:.4}"),
        ),
        figure(
            "Rayleigh length $z_R = \\pi n w_0^2/\\lambda$",
            PI * n.core * w0 * w0 / LAMBDA,
            3,
            "µm",
            "",
        ),
    ];
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "centre width W".to_owned(),
        x_length: true,
        y_label: "through, TE₀ (share of input)".to_owned(),
        y_log: false,
        y_range: Some([0.7, 1.0]),
        series: vec![
            curve(
                "the arms alone",
                sweep
                    .iter()
                    .filter(|p| p[1].is_finite())
                    .map(|p| [p[0], p[1]])
                    .collect(),
                false,
                0,
            ),
            curve(
                "with gdsfactory's ellipses",
                sweep
                    .iter()
                    .filter(|p| p[2].is_finite())
                    .map(|p| [p[0], p[2]])
                    .collect(),
                false,
                1,
            ),
            curve("Gaussian beam across the open centre", gauss, true, 2),
        ],
        marker: Some(width),
        figures,
        note: format!(
            "Eigenmode expansion on the plane of the effective index method at {LAMBDA} µm, TM in the plane (the strip's TE), 40 modes in a half window of 2 µm inside a PML 4 µm thick, steps of at most 0.1 µm; tapers of {taper:.1} µm. At W = 0.5 µm the crossing is the plain cross."
        ),
    })
}

/// The lengths of the taper chart, µm: 15 from 0.5 to 40, evenly on a logarithmic scale.
fn lengths() -> Vec<f64> {
    (0..15)
        .map(|k| 0.5 * portable::powf(80f64, f64::from(k) / 14.0))
        .collect()
}

/// A taper's staircase: steps of at most 0.25 µm, at least 40.
pub fn taper_steps(length: f64) -> usize {
    ((length / 0.25).ceil() as usize).max(TAPER_STEPS)
}

/// The shortest length at which `points` (length, transmission) reach `level`, by linear
/// interpolation in the logarithm of the length; `None` if they never do.
fn reaches(points: &[[f64; 2]], level: f64) -> Option<f64> {
    if points.first()?[1] >= level {
        return Some(points[0][0]);
    }
    points.windows(2).find_map(|p| {
        let ([l0, t0], [l1, t1]) = (p[0], p[1]);
        (t0 < level && t1 >= level).then(|| {
            let f = (level - t0) / (t1 - t0);
            portable::exp(portable::ln(l0) + f * (portable::ln(l1) - portable::ln(l0)))
        })
    })
}

pub fn taper_chart(to: f64, length: f64) -> photonoxide::Result<ChartData> {
    static CACHE: OnceLock<Cache<Vec<Vec<[f64; 2]>>>> = OnceLock::new();
    let curves = CACHE
        .get_or_init(Cache::new)
        .get_or(key(to, 0.0, 0.0), || {
            let cases: Vec<(Profile, f64)> = Profile::ALL
                .iter()
                .flat_map(|&p| lengths().into_iter().map(move |l| (p, l)))
                .collect();
            let found = parallel(&cases, |&(p, l)| {
                taper_through(p, to, l, taper_steps(l), SETTINGS.modes)
            })?;
            let n = lengths().len();
            Ok((0..Profile::ALL.len())
                .map(|k| {
                    lengths()
                        .iter()
                        .zip(&found[k * n..(k + 1) * n])
                        .map(|(&l, &t)| [l, t])
                        .collect()
                })
                .collect())
        })?;
    let abrupt = taper_through(Profile::Linear, to, 0.0, 0, SETTINGS.modes)?;
    let mut figures = vec![
        figure("Abrupt junction", abrupt, 4, "", "eigenmode expansion"),
        figure(
            "Abrupt junction, the modes' overlap",
            junction_overlap(to)?,
            4,
            "",
            "nothing reflected",
        ),
    ];
    for (k, p) in Profile::ALL.iter().enumerate() {
        let shortest = reaches(&curves[k], 0.99);
        figures.push(Figure {
            label: format!("{}: 99 % from", p.name()),
            value: shortest,
            text: shortest.map_or("beyond 40".to_owned(), |l| super::sig(l, 3)),
            unit: "µm".to_owned(),
            note: "between the samples".to_owned(),
        });
        figures.push(figure(
            &format!("{}: adiabatic parameter at L", p.name()),
            adiabatic_parameter(*p, to, length)?,
            3,
            "",
            "largest C₀₂/(β₀ − β₂) along it",
        ));
    }
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "taper length L".to_owned(),
        x_length: true,
        y_label: "TE₀ to TE₀ (share of input)".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.02]),
        series: Profile::ALL
            .iter()
            .enumerate()
            .map(|(k, p)| curve(p.name(), curves[k].clone(), false, k))
            .collect(),
        marker: Some(length),
        figures,
        note: format!(
            "Eigenmode expansion on the plane of the effective index method at {LAMBDA} µm: 500 nm to {to:.2} µm, TM in the plane, 40 modes in a half window of w₂/2 + 1.5 µm inside a PML 4 µm thick, the staircase's steps at most 0.25 µm long; the length on a logarithmic scale."
        ),
    })
}

// ---------------------------------------------------------------------------------------------
// tests

#[cfg(test)]
mod tests {
    use super::*;

    fn fig(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .and_then(|f| f.value)
            .unwrap_or_else(|| panic!("no figure {label}"))
    }

    /// The expansion against 2D FDFD on the same plane, for the plain cross and gdsfactory's
    /// crossing with its ellipses. FDFD's limit is Richardson's from 50 and 25 nm (second order),
    /// its own error the change that makes; the expansion's own error its change from 40 to 50
    /// modes and from steps of 0.1 to 0.05 µm. The two agree within three times the larger.
    #[test]
    fn the_expansion_agrees_with_fdfd() {
        for c in [Crossing::PLAIN, Crossing::gdsfactory(true)] {
            let (coarse, fine) = (
                fdfd_crossing(&c, 0.05).unwrap().through,
                fdfd_crossing(&c, 0.025).unwrap().through,
            );
            let limit = fine + (fine - coarse) / 3.0;
            let fdfd_error = (fine - coarse).abs() / 3.0;
            let e = eme_crossing(&c, SETTINGS).unwrap().through;
            let more = eme_crossing(
                &c,
                Settings {
                    modes: 50,
                    ..SETTINGS
                },
            )
            .unwrap()
            .through;
            let finer = eme_crossing(
                &c,
                Settings {
                    step: 0.05,
                    ..SETTINGS
                },
            )
            .unwrap()
            .through;
            let eme_error = (more - e).abs().max((finer - e).abs());
            let tol = 3.0 * fdfd_error.max(eme_error);
            assert!(
                (e - limit).abs() < tol,
                "{c:?}: expansion {e} against FDFD's {limit} (tolerance {tol})"
            );
        }
    }

    /// The crossing is mirror-symmetric across the through arm, so the two side arms take the
    /// same power; and the shares the field chart reports add up to what was sent in.
    #[test]
    fn the_side_arms_take_the_same() {
        let f = fdfd_crossing(&Crossing::PLAIN, 0.05).unwrap();
        assert!((f.up - f.down).abs() < 1e-8 * f.up, "{} {}", f.up, f.down);
        let data = field(3.0, GDS_CENTRE, GDS_TAPER).unwrap();
        let total = fig(&data, "Through, TE₀")
            + fig(&data, "Reflected")
            + 2.0 * fig(&data, "Into each side")
            + fig(&data, "Elsewhere");
        assert!((total - 1.0).abs() < 1e-12, "{total}");
    }

    /// The Gaussian beam's estimate is its closed form: no gap keeps everything, a gap of twice
    /// the Rayleigh length keeps 1/√2.
    #[test]
    fn the_gaussian_estimate_is_its_closed_form() {
        let n = indices().unwrap();
        let (w0, fit) = fitted_waist(&n, 1.2).unwrap();
        assert!(fit > 0.99, "{fit}");
        let zr = PI * n.core * w0 * w0 / LAMBDA;
        assert!((gaussian_through(w0, n.core, 0.0) - 1.0).abs() < 1e-15);
        assert!((gaussian_through(w0, n.core, 2.0 * zr) - 0.5f64.sqrt()).abs() < 1e-12);
    }

    /// The taper by the expansion against 2D FDFD of the same taper, at one length; a long
    /// parabolic taper delivers nearly everything, and the abrupt junction is near its overlap.
    #[test]
    fn the_taper_agrees_with_fdfd() {
        let (to, length) = (3.0, 5.0);
        let e = taper_through(Profile::Linear, to, length, taper_steps(length), 40).unwrap();
        let finer =
            taper_through(Profile::Linear, to, length, 2 * taper_steps(length), 50).unwrap();
        let (coarse, fine) = (
            fdfd_taper(to, length, 0.05).unwrap(),
            fdfd_taper(to, length, 0.025).unwrap(),
        );
        let limit = fine + (fine - coarse) / 3.0;
        let tol = 3.0 * ((fine - coarse).abs() / 3.0).max((finer - e).abs());
        assert!((e - limit).abs() < tol, "{e} against {limit} ({tol})");
        let long = taper_through(Profile::Parabolic, to, 30.0, taper_steps(30.0), 40).unwrap();
        assert!(long > 0.99, "{long}");
    }
}
