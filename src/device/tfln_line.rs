//! The modulator's RF line's own field ([`TflnModulator::line_field`]): the travelling wave that
//! carries the drive along the electrode, at a frequency, beside the 2D model's quasi-TEM line.
//!
//! The 2D model's line is quasi-TEM ([`crate::electro::line::QuasiTem`]): its field is the static
//! field of the line's own cross-section (the substrate and the far grounds in it, unlike the
//! Pockels field's window) at every frequency, and its γ and Z₀ follow from the capacitances and
//! the strips' internal impedance. What the field of the wave does as the frequency rises (it
//! draws into the silicon, the current crowds into the metal's skin) is the full-wave RF mode's
//! ([`crate::electro::full_wave`]): the same cross-section solved as a waveguide at f, the metal a
//! lossy medium, the substrate a half-space. This shows that mode's field at 1 V beside the
//! quasi-TEM field on the same pixels, with the mode's numbers and the device's own line figures
//! at f.

use std::sync::{Arc, Mutex};

use num_complex::Complex64 as c64;

use crate::circuit::Fidelity;
use crate::electro::Side;
use crate::electro::full_wave::{FullWave, Open, RfField};
use crate::electro::line_checks::ZHANG_TANGENTS;
use crate::raster::Raster;
use crate::{Error, Result};

use super::super::{Device, Values};
use super::{KIND, PICTURE_PIXELS, TflnModulator, outlines};

/// The full-wave grid's smallest step, µm: the line's cross-section graded from it by 15% a cell
/// to 20 µm, as jobs/tfln-line.yaml takes it (the middle of `line/full-wave-grid`'s 40, 20 and 10
/// nm, whose index, Re Z₀ and loss at 10 nm are within 1.3e-3 of their limit).
const FULL_WAVE_STEP: f64 = 0.02;
/// The perfectly matched layers below the substrate and beyond the ground (µm, and their
/// cells): the silicon a half-space, as the full-wave cases and jobs/tfln-line.yaml take it.
const LAYER_UM: f64 = 200.0;
const LAYER_CELLS: usize = 15;
/// The frequencies the full-wave mode is solved at, GHz: from 1 GHz (below it β² nears the
/// rounding of the lattice's entries, as `job::line`'s full-wave line) to the design's highest
/// frequency.
pub(super) const LOWEST_GHZ: f64 = 1.0;
const HIGHEST_GHZ: f64 = 500.0;

/// The RF line's own field at one frequency ([`TflnModulator::line_field`]): the full-wave RF
/// mode's electric field at 1 V on the signal, the line's quasi-TEM (static) field on the same
/// pixels, and the numbers of both lines there. One arm's gap and what is around it: the ground
/// on the left, the signal on the right up to the line's middle, from the substrate below the
/// buried oxide to over the electrodes; x across from the rib's centre, y up from the film's
/// bottom, µm, as [`super::ElectrodeField`]'s.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct LineField {
    /// The frequency, GHz.
    pub frequency_ghz: f64,
    /// |E| of the full-wave mode (V/m, across, up and along) at 1 V on the signal, its voltage V
    /// = 2P/I* (so that Z₀ = V/I): each pixel the value of the line's grid cell it falls in.
    pub field: Raster,
    /// |E_x| of the same mode, the field across: the component the TE mode meets through r₃₃
    /// in the x-cut film.
    pub across: Raster,
    /// |E| of the line's static field at 1 V on the signal, the grounds at 0, on the same grid and
    /// pixels: the quasi-TEM line's field at every frequency, the full-wave mode's at low
    /// frequency.
    pub quasi_static: Raster,
    /// [`LineField::field`] less the full-wave mode's |E| at 1 GHz on the same pixels (V/m, both
    /// at 1 V): where the field has gone as the frequency rose, free of the difference between
    /// the lattice's field and the static solver's on the same grid, which a comparison with
    /// [`LineField::quasi_static`] holds (1.4% of |E| root mean square at the defaults, the same
    /// at 1, 10 and 100 GHz; within the static field's own grid error, 2.8%).
    pub change: Raster,
    /// The rib's outline and the electrodes' rectangles in the window, each closed (µm).
    pub shapes: Vec<Vec<[f64; 2]>>,
    /// The layers' interfaces in the window, y (µm): the substrate's top, the buried oxide's,
    /// the slab's, and the cladding's.
    pub interfaces: Vec<f64>,
    /// The full-wave mode's RF index βc/ω ([`crate::electro::full_wave::RfMode::index`]).
    pub index: f64,
    /// The full-wave mode's Z₀ = 2P/|I|², Ω, real and imaginary parts (e^(jωt)).
    pub impedance: [f64; 2],
    /// The full-wave mode's loss, dB/cm.
    pub loss: f64,
    /// The part of the loss the metal takes, dB/cm.
    pub conductor: f64,
    /// The part the dielectrics take, dB/cm.
    pub dielectric: f64,
    /// The rest: the power radiated into the substrate's half-space, dB/cm.
    pub radiated: f64,
    /// The device's figure `n_rf` at the frequency: its 2D model's quasi-TEM line's RF index.
    pub n_rf: f64,
    /// The device's figure `Z0` at the frequency: Re Z₀ of the quasi-TEM line, Ω.
    pub z0: f64,
    /// The device's figure `alpha_rf` at the frequency: the quasi-TEM line's loss, dB/cm.
    pub alpha_rf: f64,
    /// The grid the three figures name.
    pub line_grid: String,
    /// The full-wave grid, in words.
    pub grid: String,
}

impl TflnModulator {
    /// The RF line's own field at `frequency_ghz` (1 to 500 GHz) and `values`: the full-wave RF
    /// mode of the line's cross-section (the static problem of the 2D model's quasi-TEM line,
    /// graded from 20 nm, the silicon a half-space below it, the metal of the design's
    /// conductivity a lossy medium, Zhang et al.'s loss tangents, the mode nearest the quasi-TEM
    /// index) at 1 V, beside that line's static field and the device's own figures there. See
    /// [`LineField`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for a frequency outside 1 to 500 GHz, values that aren't this
    /// device's or make no cross-section, and the solvers'.
    pub fn line_field(&self, values: &Values, frequency_ghz: f64) -> Result<LineField> {
        if !(LOWEST_GHZ..=HIGHEST_GHZ).contains(&frequency_ghz) {
            return Err(Error::invalid(
                "device",
                format!(
                    "a {KIND}'s line field is solved from {LOWEST_GHZ} to {HIGHEST_GHZ} GHz (below \
                     1 GHz the full-wave mode's β² nears the lattice's rounding), got \
                     {frequency_ghz} GHz"
                ),
            ));
        }
        let s = self.section(values)?;
        // the device's own line figures at the frequency: its 2D model's
        let at = values.with(self, "frequency", frequency_ghz)?;
        let built = self.component(&at, Fidelity::TwoD)?;
        let figure = |symbol: &str| {
            built
                .figure(symbol)
                .ok_or_else(|| Error::invalid("device", format!("the 2D model gives no {symbol}")))
        };
        let (n_rf, z0, alpha_rf) = (figure("n_rf")?, figure("Z0")?, figure("alpha_rf")?);
        let step = FULL_WAVE_STEP / self.refinement;
        let problem = s.cpw.electrostatic(step);
        let layer =
            |side: Side| -> Result<Open> { Ok(Open::new(side, LAYER_UM)?.with_cells(LAYER_CELLS)) };
        let solver = FullWave::new(0, values.at("conductivity") * 1e6)?
            .mirrored()
            .with_loss_tangents(&ZHANG_TANGENTS)
            .with_count(4)
            .with_open(layer(Side::Bottom)?)
            .with_open(layer(Side::Right)?);
        let rf = solver.field(&problem, frequency_ghz * 1e9, n_rf.value)?;
        // the mode at the lowest frequency, which the change is from: kept, as the slider moves
        // the frequency alone (at that frequency, the mode itself)
        static KEPT: Mutex<Vec<(String, Arc<RfField>)>> = Mutex::new(Vec::new());
        let key = format!(
            "{}|{:?}|{}",
            self.refinement.to_bits(),
            s.cpw,
            values.at("conductivity").to_bits()
        );
        let lock = || {
            KEPT.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        };
        let found = lock()
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, f)| Arc::clone(f));
        let lowest = match found {
            _ if frequency_ghz == LOWEST_GHZ => Arc::new(rf.clone()),
            Some(f) => f,
            None => {
                let near = self
                    .component(&values.with(self, "frequency", LOWEST_GHZ)?, Fidelity::TwoD)?
                    .figure("n_rf")
                    .map_or(n_rf.value, |g| g.value);
                Arc::new(solver.field(&problem, LOWEST_GHZ * 1e9, near)?)
            }
        };
        {
            let mut kept = lock();
            kept.retain(|(k, _)| *k != key);
            kept.push((key, Arc::clone(&lowest)));
            if kept.len() > 4 {
                kept.remove(0);
            }
        }
        let phi = problem.solve()?;
        let static_field = problem.electric_field(&phi)?;
        let t = &s.tfln;
        // the window: a gap past each of the gap's edges (the signal's to the line's middle at
        // most), from a gap below the buried oxide's bottom to 1 µm over what is highest
        let x = [-1.5 * t.gap, (1.5 * t.gap).min(0.5 * t.gap + t.signal)];
        let y = [
            -t.buried - t.gap,
            t.film.max(t.slab + t.clad).max(t.slab + t.metal) + 1.0,
        ];
        let pixel = (x[1] - x[0]).max(y[1] - y[0]) / PICTURE_PIXELS;
        let (nx, ny) = (
            ((x[1] - x[0]) / pixel).round().max(2.0) as usize,
            ((y[1] - y[0]) / pixel).round().max(2.0) as usize,
        );
        // the line's coordinates: x from its middle (the signal's), the rib's centre a half
        // signal and a half gap from it, the picture's x mirrored
        let centre = s.cpw.signal + 0.5 * s.cpw.gap;
        let grid = static_field.grid();
        let n = nx * ny;
        let (mut field, mut across, mut quasi, mut change) = (
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
        );
        let strength = |e: [c64; 3]| e.iter().map(c64::norm_sqr).sum::<f64>().sqrt();
        for j in 0..ny {
            for i in 0..nx {
                let xp = x[0] + (i as f64 + 0.5) * (x[1] - x[0]) / nx as f64;
                let yp = y[0] + (j as f64 + 0.5) * (y[1] - y[0]) / ny as f64;
                let xl = centre - xp;
                let e = rf.at(xl, yp);
                field.push(strength(e) as f32);
                across.push(e[0].norm() as f32);
                change.push((strength(e) - strength(lowest.at(xl, yp))) as f32);
                let q = static_field.value(grid.x().cell_of(xl), grid.y().cell_of(yp));
                quasi.push(q[0].hypot(q[1]) as f32);
            }
        }
        let raster = |values: Vec<f32>| Raster {
            nx,
            ny,
            x0: x[0],
            x1: x[1],
            y0: y[0],
            y1: y[1],
            values,
        };
        let (shapes, interfaces) = outlines(t, x, y);
        let m = &rf.mode;
        let db = crate::electro::line::DB_PER_NEPER / 100.0;
        Ok(LineField {
            frequency_ghz,
            field: raster(field),
            across: raster(across),
            quasi_static: raster(quasi),
            change: raster(change),
            shapes,
            interfaces,
            index: m.index(),
            impedance: [m.impedance.re, m.impedance.im],
            loss: m.db_per_cm(),
            conductor: m.conductor * db,
            dielectric: m.dielectric * db,
            radiated: m.radiation() * db,
            n_rf: n_rf.value,
            z0: z0.value,
            alpha_rf: alpha_rf.value,
            line_grid: n_rf.grid.clone(),
            grid: format!(
                "the full-wave lattice of the line's cross-section, {} x {} cells graded from \
                 {:.3} nm by 1.15 to 20 um, the substrate and the ground's side open into {LAYER_UM} \
                 um of perfectly matched layer",
                grid.x().cells(),
                grid.y().cells(),
                step * 1e3
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The relative root-mean-square difference of `a` from `b`, pixel by pixel.
    fn rms(a: &[f32], b: &[f32]) -> f64 {
        let (mut d, mut n) = (0.0f64, 0.0f64);
        for (&u, &v) in a.iter().zip(b) {
            let (u, v) = (f64::from(u), f64::from(v));
            d += (u - v) * (u - v);
            n += v * v;
        }
        (d / n).sqrt()
    }

    /// The static field of the line's cross-section at 1 V, graded from `step`, on `like`'s
    /// pixels as [`TflnModulator::line_field`] samples it: |E| and |E_x|.
    fn static_on(s: &super::super::Section, step: f64, like: &Raster) -> (Vec<f32>, Vec<f32>) {
        let problem = s.cpw.electrostatic(step);
        let field = problem.electric_field(&problem.solve().unwrap()).unwrap();
        let grid = field.grid();
        let centre = s.cpw.signal + 0.5 * s.cpw.gap;
        let (mut strength, mut across) = (Vec::new(), Vec::new());
        for j in 0..like.ny {
            for i in 0..like.nx {
                let x = like.x0 + (i as f64 + 0.5) * (like.x1 - like.x0) / like.nx as f64;
                let y = like.y0 + (j as f64 + 0.5) * (like.y1 - like.y0) / like.ny as f64;
                let e = field.value(grid.x().cell_of(centre - x), grid.y().cell_of(y));
                strength.push(e[0].hypot(e[1]) as f32);
                across.push(e[0].abs() as f32);
            }
        }
        (strength, across)
    }

    #[test]
    fn the_line_field_at_1_ghz_is_the_static_field_within_its_grid_error() {
        let d = TflnModulator::new();
        let v = Values::defaults(&d);
        let l = d.line_field(&v, LOWEST_GHZ).unwrap();
        let s = d.section(&v).unwrap();
        // the static field on the full-wave grid (graded from 20 nm), and twice as fine: their
        // difference is the grid's error, 2.8% of |E| and 3.3% of |E_x| at the defaults
        // (2026-10-30)
        let (strength, across) = static_on(&s, FULL_WAVE_STEP, &l.field);
        let (finer, finer_across) = static_on(&s, 0.5 * FULL_WAVE_STEP, &l.field);
        assert!(rms(&l.quasi_static.values, &strength) < 1e-6);
        let grid_error = rms(&strength, &finer);
        let grid_error_across = rms(&across, &finer_across);
        // at 1 GHz the mode at 1 V (V = 2P/I*) is that field, within the grid's error: the
        // lattice's field and the static solver's on the same grid differ by 1.4% of |E| (1.5% of
        // |E_x|), root mean square over the pixels
        let rf = rms(&l.field.values, &strength);
        assert!(rf < grid_error, "{rf} against the grid's {grid_error}");
        let rf_across = rms(&l.across.values, &across);
        assert!(
            rf_across < grid_error_across,
            "{rf_across} against the grid's {grid_error_across}"
        );
        // and the change is from itself
        assert!(l.change.values.iter().all(|&c| c == 0.0));
        assert!(d.line_field(&v, 0.5).is_err() && d.line_field(&v, 501.0).is_err());
    }

    #[test]
    fn the_line_numbers_shown_are_the_device_s_figures_at_the_frequency() {
        let d = TflnModulator::new();
        let v = Values::defaults(&d);
        let ghz = 100.0;
        let l = d.line_field(&v, ghz).unwrap();
        let at = v.with(&d, "frequency", ghz).unwrap();
        let built = d.component(&at, Fidelity::TwoD).unwrap();
        for (symbol, shown) in [("n_rf", l.n_rf), ("Z0", l.z0), ("alpha_rf", l.alpha_rf)] {
            let g = built.figure(symbol).unwrap();
            assert!(
                (shown / g.value - 1.0).abs() < 1e-12,
                "{symbol}: {shown} against {}",
                g.value
            );
            assert_eq!(l.line_grid, g.grid);
        }
        // the mode's loss is its parts', the radiation the rest
        let parts = l.conductor + l.dielectric + l.radiated;
        assert!((parts / l.loss - 1.0).abs() < 1e-12, "{parts} {}", l.loss);
        // the change from 1 GHz is the mode's own at both frequencies
        let low = d.line_field(&v, LOWEST_GHZ).unwrap();
        for ((&c, &f), &f0) in l
            .change
            .values
            .iter()
            .zip(&l.field.values)
            .zip(&low.field.values)
        {
            assert!((c - (f - f0)).abs() <= 1e-6 * f.max(f0), "{c} {f} {f0}");
        }
        assert!(l.change.values.iter().any(|&c| c != 0.0));
        // the window: a gap past each edge of the gap, from a gap into the silicon
        let r = &l.field;
        assert_eq!([r.x0, r.x1, r.y0], [-7.5, 7.5, -7.0]);
        assert!(r.nx <= 400 && r.ny <= 400 && r.values.len() == r.nx * r.ny);
        assert_eq!(l.interfaces, vec![-2.0, 0.0, 0.3, 0.3 + 0.6]);
        assert_eq!(l.shapes.len(), 3);
    }
}
