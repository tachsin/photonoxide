//! A TFLN Mach–Zehnder modulator's page beyond its figures (`photonoxide::device::TflnModulator`):
//!
//! - [`device_electrode_field`]: the electrode-field view, the 2D model's static field at 1 V on
//!   its cross-section with the optical mode over it, and the Pockels Δn the field makes
//!   (`TflnModulator::electrode_field`, the same solves as its Vπ·L);
//! - [`device_line_field`]: the RF line's own field at a frequency, the full-wave mode of the
//!   line's cross-section at 1 V beside the quasi-TEM line's static field, with the mode's
//!   numbers and the device's own line figures there (`TflnModulator::line_field`);
//! - [`device_vpi_l_sweep`]: Vπ·L, V_π and the bandwidths against one of its parameters (the
//!   gap, the signal electrode's width, the rib's width, the length, the cladding's thickness
//!   and permittivity, ...), each point the device's own figures at that value
//!   (`Device::component`, as the page's figures are);
//! - [`device_eo_response`]: the electro-optic response 20 log₁₀ |m(f)| against frequency,
//!   referred to its value at 1 GHz as the figures' `f_3dB` and `f_6dB` are, each point the
//!   figure `m_dB` with the design's frequency at f, and the two bandwidths marked.
//!
//! Nothing here computes the physics: each number is one the device's model gives.

use photonoxide::circuit::Fidelity;
use photonoxide::device::{self, Device, Spec, TflnModulator};
use photonoxide::raster::Raster;
use serde::Serialize;

use super::fields::{Isoline, Line, contours};
use super::{SpecIn, fidelity_name, text};

/// The modulator's kind, as a design names it.
const KIND: &str = "TFLN Mach-Zehnder modulator";
/// The frequency the bandwidths are referred to, GHz: the figures' (Zhang et al.'s 1 GHz).
pub const REFERENCE_GHZ: f64 = 1.0;
/// The highest frequency the design's `frequency` takes, GHz.
const HIGHEST_GHZ: f64 = 500.0;

/// `spec` if it is a modulator's, with its device and design.
fn modulator(spec: &SpecIn) -> Result<(Spec, Box<dyn Device>, device::Design), String> {
    let s = spec.spec()?;
    if s.kind != KIND {
        return Err(format!(
            "a {} has no electrodes: the electrode field and the Vπ·L and bandwidth sweeps are a \
             {KIND}'s",
            s.kind
        ));
    }
    let (d, design) = s.build().map_err(text)?;
    Ok((s, d, design))
}

/// The fidelity a sweep computes at: the analytic or the 2D model (a measured device is at its
/// own design only).
fn sweep_fidelity(name: &str) -> Result<Fidelity, String> {
    let f = device::fidelity(name).map_err(text)?;
    match f {
        Fidelity::Analytic | Fidelity::TwoD => Ok(f),
        _ => Err(format!(
            "a sweep is computed by the analytic or the 2D model, not the {} one (a measured \
             device is at its own design)",
            fidelity_name(f)
        )),
    }
}

/// A figure's value by its symbol, NaN if the model gives none.
fn figure(figures: &[device::Figure], symbol: &str) -> f64 {
    figures
        .iter()
        .find(|g| g.symbol == symbol)
        .map_or(f64::NAN, |g| g.value)
}

/// The electrode-field view: the 2D model's field and mode, and the mode's outline.
#[derive(Debug, Serialize)]
pub struct ElectrodeView {
    /// |E| at 1 V on the signal (V/m): x across from the rib's centre, y up from the film's
    /// bottom, µm.
    pub raster: Raster,
    /// The largest |E| in the window, V/m (the electrodes' corners, where it is singular), and
    /// |E| where the mode peaks.
    pub peak: f64,
    pub at_mode: f64,
    /// The mode's |E|² at half its peak and at 1/e² (levels 0.5 and 0.135).
    pub mode: Vec<Isoline>,
    /// The Pockels Δn at 1 V for light polarized across, on the same pixels
    /// (`ElectrodeField::index`: −½ n_e³ r₃₃ E_x in the crystal, 0 outside it), and Δn where
    /// the mode peaks.
    pub index: Raster,
    pub index_at_mode: f64,
    /// The mode's Δn_eff at 1 V on one arm: the part the picture's Δn gives over the mode, and
    /// the rest (r₁₃ on the ordinary index, r₅₁), whose sum sets `vpi_l`.
    pub index_shown: f64,
    pub index_rest: f64,
    /// E at 1 V on a coarser grid, for arrows: where (µm) and E (V/m, across and up).
    pub arrows: Vec<([f64; 2], [f64; 2])>,
    /// The rib's and the electrodes' outlines, and the layers' interfaces (y, µm).
    pub shapes: Vec<Line>,
    pub interfaces: Vec<f64>,
    /// The 2D model's Vπ·L push–pull (V·cm), from this field and this mode, and the mode's
    /// effective index.
    pub vpi_l: f64,
    pub n_eff: f64,
    /// The field's and the mode's grids, in words.
    pub grid: String,
}

/// `spec`'s electrode field by its 2D model, whatever fidelity the page uses.
///
/// # Errors
///
/// A design that isn't a modulator's, or that its 2D model refuses.
pub fn electrode_field(spec: &SpecIn) -> Result<ElectrodeView, String> {
    let (_, _, design) = modulator(spec)?;
    let e = TflnModulator::new()
        .electrode_field(design.values())
        .map_err(text)?;
    let (k, _) = e
        .mode
        .values
        .iter()
        .enumerate()
        .fold((0, f32::MIN), |b, (k, &v)| if v > b.1 { (k, v) } else { b });
    let mode = [0.5, (-2.0f64).exp()]
        .into_iter()
        .map(|level| Isoline {
            level,
            lines: contours(&e.mode, level),
        })
        .collect();
    Ok(ElectrodeView {
        peak: e.field.values.iter().copied().fold(0.0f32, f32::max).into(),
        at_mode: e.field.values[k].into(),
        raster: e.field,
        mode,
        index_at_mode: e.index.values[k].into(),
        index: e.index,
        index_shown: e.index_shown,
        index_rest: e.index_rest,
        arrows: e.arrows,
        shapes: e.shapes,
        interfaces: e.interfaces,
        vpi_l: e.vpi_l,
        n_eff: e.n_eff,
        grid: e.grid,
    })
}

/// The electrode field of a modulator's design by its 2D model.
#[tauri::command(async)]
pub fn device_electrode_field(spec: SpecIn) -> Result<ElectrodeView, String> {
    electrode_field(&spec)
}

/// The RF line's own field at one frequency (`TflnModulator::line_field`): the full-wave mode's
/// field at 1 V beside the quasi-TEM line's, on one window of the cross-section, and both lines'
/// numbers there.
#[derive(Debug, Serialize)]
pub struct LineView {
    /// The frequency, GHz, and the highest the view offers: the 2D model's response's top
    /// (1.5 times its 6 dB bandwidth, from 20 to 500 GHz, as the Study tab's response).
    pub frequency_ghz: f64,
    pub top_ghz: f64,
    /// |E| and |E_x| of the full-wave mode at 1 V, and |E| of the quasi-TEM (static) line at 1
    /// V, V/m, on the same pixels: x across from the rib's centre (the ground left, the signal
    /// right), y up from the film's bottom, µm.
    pub field: Raster,
    pub across: Raster,
    pub quasi_static: Raster,
    /// The full-wave mode's |E| less its |E| at 1 GHz, V/m, on the same pixels.
    pub change: Raster,
    /// The rib's and the electrodes' outlines, and the layers' interfaces (y, µm).
    pub shapes: Vec<Line>,
    pub interfaces: Vec<f64>,
    /// The full-wave mode: its RF index, Z₀ (Ω, real and imaginary), its loss and the metal's,
    /// the dielectrics' and the radiated parts of it (dB/cm), and its grid.
    pub index: f64,
    pub impedance: [f64; 2],
    pub loss: f64,
    pub conductor: f64,
    pub dielectric: f64,
    pub radiated: f64,
    pub grid: String,
    /// The device's own figures at the frequency (its 2D model's quasi-TEM line): `n_rf`, `Z0`,
    /// `alpha_rf`, and their grid.
    pub n_rf: f64,
    pub z0: f64,
    pub alpha_rf: f64,
    pub line_grid: String,
}

/// `spec`'s RF line at `frequency_ghz` by its 2D model, whatever fidelity the page uses.
///
/// # Errors
///
/// A design that isn't a modulator's, a frequency outside 1 to 500 GHz, or what the model
/// refuses.
pub fn line_field(spec: &SpecIn, frequency_ghz: f64) -> Result<LineView, String> {
    let (_, _, design) = modulator(spec)?;
    let d = TflnModulator::new();
    let values = design.values();
    let figures = d.component(values, Fidelity::TwoD).map_err(text)?.figures;
    let l = d.line_field(values, frequency_ghz).map_err(text)?;
    Ok(LineView {
        frequency_ghz: l.frequency_ghz,
        top_ghz: response_top(figure(&figures, "f_6dB")),
        field: l.field,
        across: l.across,
        quasi_static: l.quasi_static,
        change: l.change,
        shapes: l.shapes,
        interfaces: l.interfaces,
        index: l.index,
        impedance: l.impedance,
        loss: l.loss,
        conductor: l.conductor,
        dielectric: l.dielectric,
        radiated: l.radiated,
        grid: l.grid,
        n_rf: l.n_rf,
        z0: l.z0,
        alpha_rf: l.alpha_rf,
        line_grid: l.line_grid,
    })
}

/// The RF line's own field of a modulator's design at a frequency, by its 2D model.
#[tauri::command(async)]
pub fn device_line_field(spec: SpecIn, frequency_ghz: f64) -> Result<LineView, String> {
    line_field(&spec, frequency_ghz)
}

/// One point of a sweep: the device's figures at the parameter's value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SweepPoint {
    /// The parameter's value, in its unit.
    pub x: f64,
    /// Vπ·L push–pull (V·cm), V_π at DC (V), the 3 and 6 dB bandwidths from 1 GHz (GHz): the
    /// figures `VpiL`, `Vpi`, `f_3dB` and `f_6dB`, NaN where the point has none.
    pub vpi_l: f64,
    pub vpi: f64,
    pub f_3db: f64,
    pub f_6db: f64,
    /// Why the model gives nothing here (a rib wider than the gap, say), if it doesn't.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

/// A sweep of one parameter.
#[derive(Debug, Serialize)]
pub struct VpiSweep {
    pub parameter: String,
    pub unit: String,
    pub fidelity: &'static str,
    pub points: Vec<SweepPoint>,
    /// The grid the `VpiL` figure names at the first point that has one.
    pub grid: String,
    /// The model's stated error in the bandwidths against the 2D model, %: the analytic
    /// fidelity's (`TflnModulator::ANALYTIC_BANDWIDTH_ERROR`), none for the 2D model itself.
    pub bandwidth_error: Option<f64>,
}

/// The stated error in the bandwidths of the model at `f`, % against the 2D model's.
fn bandwidth_error(f: Fidelity) -> Option<f64> {
    (f == Fidelity::Analytic).then_some(TflnModulator::ANALYTIC_BANDWIDTH_ERROR)
}

/// `spec`'s figures at `fidelity` with `parameter` at each of `values` (and what uses it
/// evaluated again): each point the device's component there, as the page's figures are.
///
/// # Errors
///
/// A design that isn't a modulator's, a fidelity other than the analytic or the 2D, or a
/// parameter the design doesn't have. A value the model refuses is a point with its problem.
pub fn vpi_l_sweep(
    spec: &SpecIn,
    fidelity: &str,
    parameter: &str,
    values: &[f64],
) -> Result<VpiSweep, String> {
    let (_, d, design) = modulator(spec)?;
    let f = sweep_fidelity(fidelity)?;
    let unit = d
        .parameters()
        .iter()
        .find(|p| p.name == parameter)
        .map(|p| p.unit.clone())
        .ok_or_else(|| format!("a {KIND} has no parameter {parameter}"))?;
    let mut grid = String::new();
    let points = values
        .iter()
        .map(|&x| {
            let built = design
                .with(parameter, x)
                .and_then(|at| d.component(at.values(), f));
            match built {
                Ok(b) => {
                    if grid.is_empty() {
                        grid = b
                            .figures
                            .iter()
                            .find(|g| g.symbol == "VpiL")
                            .map(|g| g.grid.clone())
                            .unwrap_or_default();
                    }
                    SweepPoint {
                        x,
                        vpi_l: figure(&b.figures, "VpiL"),
                        vpi: figure(&b.figures, "Vpi"),
                        f_3db: figure(&b.figures, "f_3dB"),
                        f_6db: figure(&b.figures, "f_6dB"),
                        problem: None,
                    }
                }
                Err(e) => SweepPoint {
                    x,
                    vpi_l: f64::NAN,
                    vpi: f64::NAN,
                    f_3db: f64::NAN,
                    f_6db: f64::NAN,
                    problem: Some(text(e)),
                },
            }
        })
        .collect();
    Ok(VpiSweep {
        parameter: parameter.to_owned(),
        unit,
        fidelity: fidelity_name(f),
        points,
        grid,
        bandwidth_error: bandwidth_error(f),
    })
}

/// A modulator's figures with one of its parameters at each of `values`.
#[tauri::command(async)]
pub fn device_vpi_l_sweep(
    spec: SpecIn,
    fidelity: String,
    parameter: String,
    values: Vec<f64>,
) -> Result<VpiSweep, String> {
    vpi_l_sweep(&spec, &fidelity, &parameter, &values)
}

/// The electro-optic response against frequency.
#[derive(Debug, Serialize)]
pub struct Response {
    pub fidelity: &'static str,
    /// The frequency the response is referred to, GHz, and 20 log₁₀ |m| there (dB).
    pub reference_ghz: f64,
    pub reference_db: f64,
    /// (GHz, dB): 20 log₁₀ |m(f)| less its value at the reference, the figure `m_dB` with the
    /// design's frequency at f; the bandwidths among them.
    pub points: Vec<[f64; 2]>,
    /// The figures `f_3dB` and `f_6dB` (GHz): where the response falls 3 and 6 dB below the
    /// reference's; the highest frequency sought when it doesn't (`falls`).
    pub f_3db: f64,
    pub f_6db: f64,
    pub falls_3db: bool,
    pub falls_6db: bool,
    /// The line's grid, as the figures name it.
    pub grid: String,
    /// The model's stated error in the bandwidths against the 2D model, %, as the sweep's.
    pub bandwidth_error: Option<f64>,
}

/// The highest frequency a response is drawn to when none is asked, GHz: 1.5 times the 6 dB
/// bandwidth `f_6db` (GHz), from 20 to 500 GHz.
fn response_top(f_6db: f64) -> f64 {
    if f_6db.is_finite() {
        (1.5 * f_6db).clamp(20.0, HIGHEST_GHZ)
    } else {
        HIGHEST_GHZ
    }
}

/// `spec`'s response at `fidelity` from the reference to `to_ghz` (if not given, 1.5 times the
/// 6 dB bandwidth, from 20 to 500 GHz), on `points` frequencies evenly apart, the two bandwidths
/// added.
///
/// # Errors
///
/// A design that isn't a modulator's, a fidelity other than the analytic or the 2D, and what
/// its model refuses.
pub fn eo_response(
    spec: &SpecIn,
    fidelity: &str,
    to_ghz: Option<f64>,
    points: usize,
) -> Result<Response, String> {
    let (_, d, design) = modulator(spec)?;
    let f = sweep_fidelity(fidelity)?;
    let at = |ghz: f64| -> Result<Vec<device::Figure>, String> {
        let values = design
            .values()
            .with(d.as_ref(), "frequency", ghz)
            .map_err(text)?;
        Ok(d.component(&values, f).map_err(text)?.figures)
    };
    let reference = at(REFERENCE_GHZ)?;
    let reference_db = figure(&reference, "m_dB");
    let (f_3db, f_6db) = (figure(&reference, "f_3dB"), figure(&reference, "f_6dB"));
    let grid = reference
        .iter()
        .find(|g| g.symbol == "m_dB")
        .map(|g| g.grid.clone())
        .unwrap_or_default();
    // a bandwidth is the highest frequency sought when the response doesn't fall there: it fell
    // if the response at it is at the level
    let falls = |bw: f64, level: f64| -> Result<bool, String> {
        Ok(bw.is_finite()
            && bw <= HIGHEST_GHZ
            && figure(&at(bw)?, "m_dB") - reference_db <= level + 1e-6)
    };
    let (falls_3db, falls_6db) = (falls(f_3db, -3.0)?, falls(f_6db, -6.0)?);
    let to = to_ghz
        .unwrap_or_else(|| response_top(f_6db))
        .clamp(20.0, HIGHEST_GHZ);
    let n = points.max(2);
    let mut frequencies: Vec<f64> = (0..n)
        .map(|k| REFERENCE_GHZ + (to - REFERENCE_GHZ) * k as f64 / (n - 1) as f64)
        .collect();
    for bw in [f_3db, f_6db] {
        if bw.is_finite() && bw > REFERENCE_GHZ && bw < to {
            frequencies.push(bw);
        }
    }
    frequencies.sort_by(f64::total_cmp);
    frequencies.dedup();
    let points = frequencies
        .into_iter()
        .map(|ghz| Ok([ghz, figure(&at(ghz)?, "m_dB") - reference_db]))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Response {
        fidelity: fidelity_name(f),
        reference_ghz: REFERENCE_GHZ,
        reference_db,
        points,
        f_3db,
        f_6db,
        falls_3db,
        falls_6db,
        grid,
        bandwidth_error: bandwidth_error(f),
    })
}

/// A modulator's electro-optic response against frequency, its bandwidths marked.
#[tauri::command(async)]
pub fn device_eo_response(
    spec: SpecIn,
    fidelity: String,
    to_ghz: Option<f64>,
    points: Option<usize>,
) -> Result<Response, String> {
    eo_response(&spec, &fidelity, to_ghz, points.unwrap_or(100))
}

#[cfg(test)]
mod tests {
    use photonoxide::device::Values;

    use super::*;
    use crate::devices::StackIn;

    /// The modulator at its defaults (Zhang et al.'s 5.8 mm device) with `parameters`.
    fn zhang(parameters: &[(&str, &str)]) -> SpecIn {
        SpecIn {
            kind: KIND.into(),
            fidelity: "analytic".into(),
            stack: StackIn::Preset {
                name: "soi_220".into(),
                bottom_oxide_um: 2.0,
            },
            parameters: parameters
                .iter()
                .map(|&(n, e)| (n.to_owned(), e.to_owned()))
                .collect(),
            compact: None,
        }
    }

    /// The device's own figures at its defaults with `changes`, at `fidelity`.
    fn figures(changes: &[(&str, f64)], fidelity: Fidelity) -> Vec<device::Figure> {
        let d = TflnModulator::new();
        let values = Values::new(&d, changes).unwrap();
        d.component(&values, fidelity).unwrap().figures
    }

    /// Equal to round-off: the same code gives the same bits, but say so to 1e-12.
    fn same(a: f64, b: f64) -> bool {
        a == b || (a / b - 1.0).abs() < 1e-12
    }

    #[test]
    fn a_vpi_l_sweep_s_points_are_the_device_s_figures_at_each_value() {
        for (parameter, unit, values) in [
            ("gap", "µm", vec![3.5, 5.0, 8.0]),
            ("signal", "µm", vec![10.0, 40.0]),
            ("width", "µm", vec![0.9, 2.0]),
            ("length", "µm", vec![2000.0, 5800.0, 10800.0]),
            // the bandwidth against the cladding: its thickness and its permittivity
            ("cladding", "µm", vec![0.1, 0.6, 3.0]),
            ("cladding_permittivity", "", vec![1.0, 3.9, 8.0]),
        ] {
            let s = vpi_l_sweep(&zhang(&[]), "analytic", parameter, &values).unwrap();
            assert_eq!(s.fidelity, "analytic");
            assert_eq!(
                s.bandwidth_error,
                Some(TflnModulator::ANALYTIC_BANDWIDTH_ERROR)
            );
            assert_eq!(s.unit, unit);
            assert_eq!(s.points.len(), values.len());
            for p in &s.points {
                assert_eq!(p.problem, None, "{parameter} {}", p.x);
                let g = figures(&[(parameter, p.x)], Fidelity::Analytic);
                for (symbol, v) in [
                    ("VpiL", p.vpi_l),
                    ("Vpi", p.vpi),
                    ("f_3dB", p.f_3db),
                    ("f_6dB", p.f_6db),
                ] {
                    assert!(
                        same(v, figure(&g, symbol)),
                        "{parameter} {}: {symbol} {v} against {}",
                        p.x,
                        figure(&g, symbol)
                    );
                }
            }
            assert!(!s.grid.is_empty(), "{parameter}");
        }
        // the cladding's permittivity moves the line, so the bandwidth
        let s = vpi_l_sweep(
            &zhang(&[]),
            "analytic",
            "cladding_permittivity",
            &[1.0, 8.0],
        )
        .unwrap();
        assert!(s.points[0].f_3db != s.points[1].f_3db, "{:?}", s.points);
        // Vπ·L is per length: the length moves V_π alone
        let s = vpi_l_sweep(&zhang(&[]), "analytic", "length", &[2000.0, 4000.0]).unwrap();
        assert!(same(s.points[0].vpi_l, s.points[1].vpi_l));
        assert!(same(s.points[0].vpi, 2.0 * s.points[1].vpi));
        // what uses the parameter is evaluated again: the ground as twice the gap
        let s = vpi_l_sweep(&zhang(&[("ground", "2*gap")]), "analytic", "gap", &[6.0]).unwrap();
        let g = figures(&[("gap", 6.0), ("ground", 12.0)], Fidelity::Analytic);
        assert!(same(s.points[0].f_3db, figure(&g, "f_3dB")));
        // a value the model refuses is a point with its reason; a measured device, another
        // device and an unknown parameter are refused
        let s = vpi_l_sweep(&zhang(&[]), "analytic", "gap", &[1.0]).unwrap();
        assert!(s.points[0].vpi_l.is_nan() && s.points[0].problem.is_some());
        assert!(vpi_l_sweep(&zhang(&[]), "measured", "gap", &[5.0]).is_err());
        assert!(vpi_l_sweep(&zhang(&[]), "analytic", "radius", &[5.0]).is_err());
        let mut guide = zhang(&[]);
        guide.kind = "waveguide".into();
        let e = vpi_l_sweep(&guide, "analytic", "width", &[0.5]).unwrap_err();
        assert!(e.contains("no electrodes"), "{e}");
    }

    #[test]
    fn the_response_is_the_m_db_figure_referred_to_1_ghz_its_bandwidths_on_it() {
        let r = eo_response(&zhang(&[]), "analytic", None, 40).unwrap();
        let reference = figure(&figures(&[("frequency", 1.0)], Fidelity::Analytic), "m_dB");
        assert!(same(r.reference_db, reference));
        assert_eq!(r.points[0], [1.0, 0.0]);
        for &[ghz, db] in &r.points {
            let m = figure(&figures(&[("frequency", ghz)], Fidelity::Analytic), "m_dB");
            assert!((db - (m - reference)).abs() < 1e-12, "{ghz} GHz: {db}");
        }
        // the bandwidths are the figures, on the curve where it is 3 and 6 dB down
        let g = figures(&[], Fidelity::Analytic);
        assert!(same(r.f_3db, figure(&g, "f_3dB")) && same(r.f_6db, figure(&g, "f_6dB")));
        // with the analytic model's stated error in them, which the figures' names give too
        assert_eq!(
            r.bandwidth_error,
            Some(TflnModulator::ANALYTIC_BANDWIDTH_ERROR)
        );
        for symbol in ["f_3dB", "f_6dB"] {
            let name = &g.iter().find(|f| f.symbol == symbol).unwrap().name;
            assert!(name.ends_with("stated error 80%"), "{name}");
        }
        assert!(r.falls_3db && r.falls_6db && r.f_3db < r.f_6db);
        for (bw, level) in [(r.f_3db, -3.0), (r.f_6db, -6.0)] {
            let p = r.points.iter().find(|p| p[0] == bw).unwrap();
            assert!((p[1] - level).abs() < 1e-6, "{bw} GHz: {}", p[1]);
        }
        // to 1.5 times the 6 dB bandwidth, the frequencies in order
        let last = r.points.last().unwrap()[0];
        assert!(same(last, (1.5 * r.f_6db).clamp(20.0, 500.0)), "{last}");
        assert!(r.points.windows(2).all(|w| w[0][0] < w[1][0]));
        assert!(eo_response(&zhang(&[]), "measured", None, 10).is_err());
    }

    #[test]
    fn the_2d_sweeps_and_the_electrode_field_are_the_2d_model_s() {
        // one solve at the defaults, kept by the device: the sweep's point, its Vπ·L, and the
        // electrode field's are the same numbers
        let d = TflnModulator::new();
        let values = Values::new(&d, &[]).unwrap();
        let vpi_l = d.vpi_l(&values).unwrap();
        let s = vpi_l_sweep(&zhang(&[]), "2D", "length", &[5800.0]).unwrap();
        let g = figures(&[], Fidelity::TwoD);
        assert_eq!(s.fidelity, "2D");
        assert!(same(s.points[0].vpi_l, vpi_l) && same(s.points[0].f_3db, figure(&g, "f_3dB")));
        let r = eo_response(&zhang(&[]), "2D", Some(60.0), 12).unwrap();
        assert!(same(r.f_3db, figure(&g, "f_3dB")));
        // the 2D model is what the analytic one's bandwidths are stated against
        assert_eq!((s.bandwidth_error, r.bandwidth_error), (None, None));
        assert!(r.grid.contains("line's static grid"), "{}", r.grid);
        for &[ghz, db] in &r.points {
            let m = figure(&figures(&[("frequency", ghz)], Fidelity::TwoD), "m_dB");
            assert!((db - (m - r.reference_db)).abs() < 1e-12, "{ghz} GHz");
        }

        let e = electrode_field(&zhang(&[])).unwrap();
        assert!(same(e.vpi_l, vpi_l), "{} {vpi_l}", e.vpi_l);
        assert!(same(e.n_eff, figure(&g, "n_eff")));
        assert_eq!(e.grid, g.iter().find(|f| f.symbol == "VpiL").unwrap().grid);
        // the window: the gap of 5 µm and 1.5 µm past its edges, at most 400 pixels across
        let r = &e.raster;
        assert_eq!([r.x0, r.x1, r.y0], [-4.0, 4.0, -1.5]);
        assert!(r.nx <= 400 && r.ny <= 400 && r.values.len() == r.nx * r.ny);
        assert!(r.values.iter().all(|v| v.is_finite() && *v >= 0.0));
        // the mode in the rib (1.2 µm on top, the film 0.6 µm), the field there of the order of
        // 1 V over the 5 µm gap, less in the crystal for its permittivity
        assert!(
            (0.1 / 5e-6..1.0 / 5e-6).contains(&e.at_mode),
            "{} V/m",
            e.at_mode
        );
        assert!(e.peak > e.at_mode);
        // the Pockels Δn on the same pixels: its overlap with the mode and the elements it
        // leaves out give the figure's Vπ·L, push–pull, λ / (4 |Δn_eff|)
        let i = &e.index;
        assert_eq!([i.nx, i.ny, i.values.len()], [r.nx, r.ny, r.values.len()]);
        assert_eq!([i.x0, i.x1, i.y0, i.y1], [r.x0, r.x1, r.y0, r.y1]);
        let from_index = 100.0 * 1.55e-6 / (4.0 * (e.index_shown + e.index_rest).abs());
        assert!(same(from_index, vpi_l), "{from_index} {vpi_l}");
        assert!(e.index_at_mode > 0.0 && e.index_shown > 0.0);
        assert_eq!(e.mode.len(), 2);
        for m in &e.mode {
            let longest = m.lines.iter().max_by_key(|l| l.len()).unwrap();
            assert_eq!(longest.first(), longest.last(), "closed");
            let inside = |p: &[f64; 2]| p[0].abs() < 2.5 && (-1.0..1.6).contains(&p[1]);
            assert!(longest.iter().all(inside), "{longest:?}");
        }
        // the rib and the two electrodes; the film's bottom, the slab's top and the cladding's
        assert_eq!(e.shapes.len(), 3);
        assert_eq!(e.interfaces, vec![0.0, 0.3, 0.3 + 0.6]);
        assert!(e.arrows.len() >= 32 * 8);
        let mut guide = zhang(&[]);
        guide.kind = "waveguide".into();
        assert!(electrode_field(&guide).is_err());
    }

    #[test]
    fn the_line_field_is_the_library_s_and_its_numbers_the_device_s_figures() {
        let ghz = 50.0;
        let l = line_field(&zhang(&[]), ghz).unwrap();
        // the quasi-TEM line's numbers are the device's figures at that frequency
        let g = figures(&[("frequency", ghz)], Fidelity::TwoD);
        for (symbol, shown) in [("n_rf", l.n_rf), ("Z0", l.z0), ("alpha_rf", l.alpha_rf)] {
            assert!(same(shown, figure(&g, symbol)), "{symbol}: {shown}");
        }
        let n_rf = g.iter().find(|f| f.symbol == "n_rf").unwrap();
        assert_eq!(l.line_grid, n_rf.grid);
        // the field and the mode's numbers are the library's line field (a solve of its own, so
        // to round-off)
        let d = TflnModulator::new();
        let lib = d.line_field(&Values::new(&d, &[]).unwrap(), ghz).unwrap();
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs();
        assert!(close(l.index, lib.index) && close(l.loss, lib.loss));
        assert!(close(l.impedance[0], lib.impedance[0]));
        for (a, b) in [
            (&l.field, &lib.field),
            (&l.across, &lib.across),
            (&l.quasi_static, &lib.quasi_static),
        ] {
            assert_eq!([a.nx, a.ny], [b.nx, b.ny]);
            assert_eq!([a.x0, a.x1, a.y0, a.y1], [b.x0, b.x1, b.y0, b.y1]);
            for (u, v) in a.values.iter().zip(&b.values) {
                assert!((u - v).abs() <= 1e-6 * v.abs().max(1.0), "{u} {v}");
            }
        }
        assert_eq!(l.change.values.len(), l.field.values.len());
        // the slider's top is the 2D response's
        let r = eo_response(&zhang(&[]), "2D", None, 2).unwrap();
        assert!(
            same(l.top_ghz, r.points.last().unwrap()[0]),
            "{}",
            l.top_ghz
        );
        assert_eq!(l.frequency_ghz, ghz);
        // below 1 GHz, and another device, are refused
        assert!(line_field(&zhang(&[]), 0.5).is_err());
        let mut guide = zhang(&[]);
        guide.kind = "waveguide".into();
        assert!(line_field(&guide, ghz).is_err());
    }
}
