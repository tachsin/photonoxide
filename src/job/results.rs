//! `results.json`: a run's numbers for scripts (#271's phase 0), written into its directory when
//! it ends, however it ends.
//!
//! The events (`events.jsonl`) were made for the studio's views: their pictures are `f32`,
//! normalized to a peak of 1, and a reader must know every event's type to find the numbers in
//! them. `results.json` holds the numbers alone, by kind, under a schema version a reader can
//! check:
//!
//! - **What ran and how it ended:** the job's name and kind, `"finished"`, `"stopped"` (by its
//!   timeout or on request), `"failed"` with the error's text, or `"incomplete"` for a record
//!   that just stops (the program was killed);
//! - **the grids** the solvers ran on, as the [`Event::Solver`] events give them;
//! - **the modes** found, each effective index and TE fraction at its wavelength;
//! - **a sweep's points**, each with its value and its modes' effective indices;
//! - **the S-parameters**, an array over wavelengths, ports out and ports in;
//! - **FDTD's spectra and resonances.**
//!
//! Every number is the one the event recorded, which serde_json writes so that it reads back to
//! the same bits. An array is an object with its `dtype` (`"float64"` or `"complex128"`), its
//! `shape` and its `unit`, in C order (the last index fastest), and either its `data` (a flat
//! list, a complex value as its real part followed by its imaginary part) or, above
//! [`INLINE`] values, the `file` beside `results.json` that holds the same values as raw
//! little-endian doubles, which `numpy.fromfile`, or any language's reader of raw doubles,
//! reads without a library.
//! `docs/runs.md` describes it all.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::Event;
use crate::run::Run;
use crate::{Error, Result};

/// The schema's version. A change that removes or changes a field raises it; one that adds a
/// field doesn't.
pub const SCHEMA: u32 = 1;

/// The most values an array keeps in `results.json` itself: larger ones go to a file beside it.
pub const INLINE: usize = 4096;

/// An array of numbers, in C order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Array {
    /// `"float64"`, or `"complex128"` (each value its real part, then its imaginary part).
    pub dtype: String,
    /// Its shape, the last index fastest.
    pub shape: Vec<usize>,
    /// What its values are in: `"um"`, `"1"` for a plain number, or as its field says.
    pub unit: String,
    /// Its values, if they are here: shape's product of them, twice that for complex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<f64>>,
    /// The file beside `results.json` that holds them, if they aren't here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

impl Array {
    fn real(shape: Vec<usize>, unit: &str, data: Vec<f64>) -> Array {
        Array {
            dtype: "float64".into(),
            shape,
            unit: unit.into(),
            data: Some(data),
            file: None,
        }
    }

    fn complex(shape: Vec<usize>, unit: &str, data: Vec<f64>) -> Array {
        Array {
            dtype: "complex128".into(),
            ..Array::real(shape, unit, data)
        }
    }

    /// Its values: those here, or those read from its file in `dir`.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the file can't be read, and [`Error::Parse`] if it holds another number
    /// of values than the shape says.
    pub fn values(&self, dir: &Path) -> Result<Vec<f64>> {
        let count =
            self.shape.iter().product::<usize>() * if self.dtype == "complex128" { 2 } else { 1 };
        let values = match (&self.data, &self.file) {
            (Some(data), _) => data.clone(),
            (None, Some(file)) => {
                let path = dir.join(file);
                let bytes = std::fs::read(&path).map_err(|e| Error::Io {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                })?;
                let (words, _) = bytes.as_chunks::<8>();
                words.iter().map(|b| f64::from_le_bytes(*b)).collect()
            }
            (None, None) => Vec::new(),
        };
        if values.len() != count {
            return Err(Error::Parse {
                what: "results.json".into(),
                reason: format!(
                    "an array of shape {:?} holds {} values",
                    self.shape,
                    values.len()
                ),
            });
        }
        Ok(values)
    }
}

/// A grid a solver ran on, as its [`Event::Solver`] says.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    /// The solver, e.g. `"mode::vector"`.
    pub module: String,
    /// Its cells along its two axes.
    pub cells: [usize; 2],
    /// Its step, µm.
    pub step_um: f64,
    /// Its unknowns.
    pub unknowns: usize,
    /// What else it says of itself, as names and values.
    pub details: Vec<[String; 2]>,
}

/// A mode found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mode {
    /// What it is, e.g. `"mode 1 of 2"`.
    pub label: String,
    /// The vacuum wavelength, µm.
    pub wavelength_um: f64,
    /// The effective index, real and imaginary parts.
    pub effective_index: [f64; 2],
    /// The share of the transverse magnetic field in H_y: near 1 for a TE-like mode.
    pub te_fraction: f64,
    /// Where the cross-section was cut along the guide, µm.
    pub cut_um: f64,
}

/// A mode's effective index with its grid's check ([`crate::convergence`]): the grid study a
/// modes job's `[task.convergence]` made, or [`crate::convergence::Verdict::Unchecked`] for a
/// mode solved on one grid.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GridCheck {
    /// The mode's label, as in [`Results::modes`].
    pub label: String,
    /// Its wavelength, µm.
    pub wavelength_um: f64,
    /// Its Re n_eff, its grid, the study and the verdict.
    pub check: crate::convergence::Convergence,
}

/// A point of a sweep.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SweepPoint {
    /// The parameter's value: µm for the wavelength, a width or a length; radians for an angle.
    pub value: f64,
    /// The vacuum wavelength, µm.
    pub wavelength_um: f64,
    /// The modes' effective indices, highest first, real and imaginary parts.
    pub effective_indices: Vec<[f64; 2]>,
    /// Their TE fractions, in the same order.
    pub te_fractions: Vec<f64>,
}

/// A sweep's points.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sweep {
    /// `"wavelength"`, `"width"` or a parameter's name.
    pub parameter: String,
    /// The points, in the order they were solved.
    pub points: Vec<SweepPoint>,
}

/// The S-parameters of a 2D FDFD run, at each of its wavelengths.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SParameters {
    /// The ports' names, in order.
    pub ports: Vec<String>,
    /// The wavelengths, µm.
    pub wavelengths_um: Vec<f64>,
    /// Each port's mode's effective index at each wavelength: shape [wavelengths, ports].
    pub effective_indices: Array,
    /// The power-normalized S-matrix at each wavelength, `s[k][q][p]` from port p into port q:
    /// shape [wavelengths, ports, ports].
    pub s: Array,
}

/// A spectrum an FDTD monitor recorded, as it stood at the run's end.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spectrum {
    /// The monitor's name.
    pub monitor: String,
    /// What kind of monitor, as its event says.
    pub kind: String,
    /// The quantity, as its event says.
    pub quantity: String,
    /// The wavelengths, µm.
    pub wavelengths_um: Vec<f64>,
    /// Each series' label, in the order of `values`' rows.
    pub labels: Vec<String>,
    /// The series: shape [series, wavelengths].
    pub values: Array,
    /// How far the run had gone when it was taken, µm of c t.
    pub time_um: f64,
}

/// The resonances an FDTD monitor's harmonic inversion found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resonances {
    /// The monitor's name.
    pub monitor: String,
    /// Where it is, µm.
    pub position_um: Vec<f64>,
    /// The field's component it records.
    pub component: String,
    /// The resonances, as its event lists them.
    pub resonances: Vec<super::FoundResonance>,
}

/// An S-parameters event's fields, as [`Results::collect`] gathers them: the wavelength, the
/// ports, their modes' indices and the S-matrix.
type SRow<'a> = (f64, &'a [String], &'a [f64], &'a Vec<Vec<[f64; 2]>>);

/// A run's numbers, as `results.json` holds them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Results {
    /// [`SCHEMA`].
    pub schema: u32,
    /// The photonoxide that ran it.
    pub photonoxide: String,
    /// The job's name.
    pub job: String,
    /// The task's kind: `"structure"`, `"modes"`, `"fdfd"`, `"fdtd"` or `"device"`.
    pub kind: String,
    /// `"finished"`, `"stopped"`, `"failed"` or `"incomplete"`.
    pub outcome: String,
    /// Why it stopped early, if it did: `"timeout"` or `"requested"`.
    pub stopped: Option<String>,
    /// The error, if it failed.
    pub error: Option<String>,
    /// How long it ran, seconds, if its end was recorded.
    pub seconds: Option<f64>,
    /// The grids its solvers ran on.
    pub grids: Vec<Grid>,
    /// The modes it found.
    pub modes: Vec<Mode>,
    /// Each mode's effective index with its grid's check, in the modes' order: converged, not
    /// converged or unchecked (grid convergence gates every reported number: ROADMAP's
    /// pitfalls).
    #[serde(default)]
    pub convergence: Vec<GridCheck>,
    /// Its sweep, if it had one.
    pub sweep: Option<Sweep>,
    /// Its S-parameters, if it computed any.
    pub s_parameters: Option<SParameters>,
    /// FDTD's spectra, each as it stood last.
    pub spectra: Vec<Spectrum>,
    /// FDTD's resonances.
    pub resonances: Vec<Resonances>,
}

impl Results {
    /// The numbers of a run's events, in their order.
    pub fn collect(events: &[Event]) -> Results {
        let mut r = Results {
            schema: SCHEMA,
            photonoxide: env!("CARGO_PKG_VERSION").into(),
            job: String::new(),
            kind: String::new(),
            outcome: "incomplete".into(),
            stopped: None,
            error: None,
            seconds: None,
            grids: Vec::new(),
            modes: Vec::new(),
            convergence: Vec::new(),
            sweep: None,
            s_parameters: None,
            spectra: Vec::new(),
            resonances: Vec::new(),
        };
        // the S-parameters' rows as they come: (wavelength, ports, indices, s)
        let mut s_rows: Vec<SRow<'_>> = Vec::new();
        // each mode's grid (the solver's before it), and the grid studies recorded
        let mut step = f64::NAN;
        let mut mode_steps = Vec::new();
        let mut checked: Vec<GridCheck> = Vec::new();
        for event in events {
            match event {
                Event::Started { job, kind } => {
                    r.job.clone_from(job);
                    r.kind.clone_from(kind);
                }
                Event::Solver {
                    module,
                    cells,
                    step_um,
                    unknowns,
                    details,
                } => {
                    step = *step_um;
                    r.grids.push(Grid {
                        module: module.clone(),
                        cells: *cells,
                        step_um: *step_um,
                        unknowns: *unknowns,
                        details: details.clone(),
                    });
                }
                Event::Mode {
                    label,
                    wavelength_um,
                    effective_index,
                    te_fraction,
                    cut_y_um,
                    ..
                } => {
                    mode_steps.push(step);
                    r.modes.push(Mode {
                        label: label.clone(),
                        wavelength_um: *wavelength_um,
                        effective_index: *effective_index,
                        te_fraction: *te_fraction,
                        cut_um: *cut_y_um,
                    });
                }
                Event::GridConvergence {
                    label,
                    wavelength_um,
                    check,
                } => checked.push(GridCheck {
                    label: label.clone(),
                    wavelength_um: *wavelength_um,
                    check: check.clone(),
                }),
                Event::Sweep { parameter, .. } => {
                    r.sweep.get_or_insert_with(|| Sweep {
                        parameter: parameter.clone(),
                        points: Vec::new(),
                    });
                }
                Event::SweepPoint {
                    parameter,
                    value,
                    wavelength_um,
                    effective_indices,
                    te_fractions,
                } => r
                    .sweep
                    .get_or_insert_with(|| Sweep {
                        parameter: parameter.clone(),
                        points: Vec::new(),
                    })
                    .points
                    .push(SweepPoint {
                        value: *value,
                        wavelength_um: *wavelength_um,
                        effective_indices: effective_indices.clone(),
                        te_fractions: te_fractions.clone(),
                    }),
                Event::SParameters {
                    wavelength_um,
                    ports,
                    effective_indices,
                    s,
                } => s_rows.push((*wavelength_um, ports, effective_indices, s)),
                Event::FdtdSpectrum {
                    monitor,
                    kind,
                    quantity,
                    wavelengths_um,
                    series,
                    time_um,
                    ..
                } => {
                    let spectrum = Spectrum {
                        monitor: monitor.clone(),
                        kind: kind.clone(),
                        quantity: quantity.clone(),
                        wavelengths_um: wavelengths_um.clone(),
                        labels: series.iter().map(|s| s.label.clone()).collect(),
                        values: Array::real(
                            vec![series.len(), wavelengths_um.len()],
                            "1",
                            series
                                .iter()
                                .flat_map(|s| s.values.iter().copied())
                                .collect(),
                        ),
                        time_um: *time_um,
                    };
                    // the latest of each monitor's: a spectrum is recorded as it grows
                    match r
                        .spectra
                        .iter_mut()
                        .find(|s| s.monitor == *monitor && s.kind == *kind)
                    {
                        Some(earlier) => *earlier = spectrum,
                        None => r.spectra.push(spectrum),
                    }
                }
                Event::FdtdResonances {
                    monitor,
                    position_um,
                    component,
                    resonances,
                } => r.resonances.push(Resonances {
                    monitor: monitor.clone(),
                    position_um: position_um.clone(),
                    component: component.clone(),
                    resonances: resonances.clone(),
                }),
                Event::Finished { stopped, seconds } => {
                    r.outcome = if stopped.is_some() {
                        "stopped"
                    } else {
                        "finished"
                    }
                    .into();
                    r.stopped.clone_from(stopped);
                    r.seconds = Some(*seconds);
                }
                Event::Failed { error, seconds } => {
                    r.outcome = "failed".into();
                    r.error = Some(error.clone());
                    r.seconds = Some(*seconds);
                }
                _ => {}
            }
        }
        // every mode's index with its check: the study recorded, or none
        r.convergence = r
            .modes
            .iter()
            .zip(&mode_steps)
            .map(|(m, &h)| {
                checked
                    .iter()
                    .find(|c| c.label == m.label && c.wavelength_um == m.wavelength_um)
                    .cloned()
                    .unwrap_or_else(|| GridCheck {
                        label: m.label.clone(),
                        wavelength_um: m.wavelength_um,
                        check: crate::convergence::Convergence::unchecked(
                            &format!("Re n_eff of {}", m.label),
                            h,
                            m.effective_index[0],
                        ),
                    })
            })
            .collect();
        if let Some((_, ports, _, _)) = s_rows.first() {
            let ports: Vec<String> = ports.to_vec();
            let p = ports.len();
            // a row of another size than the first's (it can't happen) is left out, not padded
            let rows: Vec<_> = s_rows
                .iter()
                .filter(|(_, q, i, s)| {
                    q.len() == p && i.len() == p && s.len() == p && s.iter().all(|r| r.len() == p)
                })
                .collect();
            let w = rows.len();
            r.s_parameters = Some(SParameters {
                ports,
                wavelengths_um: rows.iter().map(|(l, ..)| *l).collect(),
                effective_indices: Array::real(
                    vec![w, p],
                    "1",
                    rows.iter()
                        .flat_map(|(_, _, i, _)| i.iter().copied())
                        .collect(),
                ),
                s: Array::complex(
                    vec![w, p, p],
                    "1",
                    rows.iter()
                        .flat_map(|(_, _, _, s)| s.iter().flatten().flat_map(|z| [z[0], z[1]]))
                        .collect(),
                ),
            });
        }
        r
    }

    /// Writes `results.json` into `dir`, each array above [`INLINE`] values into a file beside
    /// it named after where it is (`s_parameters.s.c128`, `spectra.0.values.f64`).
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if a file can't be written.
    pub fn write(mut self, dir: &Path) -> Result<()> {
        let out = |array: &mut Array, name: &str| -> Result<()> {
            let Some(data) = &array.data else {
                return Ok(());
            };
            let count = array.shape.iter().product::<usize>();
            if count <= INLINE {
                return Ok(());
            }
            let file = format!(
                "{name}.{}",
                if array.dtype == "complex128" {
                    "c128"
                } else {
                    "f64"
                }
            );
            let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
            let path = dir.join(&file);
            std::fs::write(&path, bytes).map_err(|e| Error::Io {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;
            array.data = None;
            array.file = Some(file);
            Ok(())
        };
        if let Some(s) = &mut self.s_parameters {
            out(&mut s.s, "s_parameters.s")?;
            out(&mut s.effective_indices, "s_parameters.effective_indices")?;
        }
        for (k, spectrum) in self.spectra.iter_mut().enumerate() {
            out(&mut spectrum.values, &format!("spectra.{k}.values"))?;
        }
        let text = serde_json::to_string_pretty(&self).map_err(|e| Error::Parse {
            what: "results.json".into(),
            reason: e.to_string(),
        })?;
        let path = dir.join("results.json");
        std::fs::write(&path, text + "\n").map_err(|e| Error::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// The `results.json` in `dir`.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if it can't be read, [`Error::Parse`] if it isn't one, or is of a schema
    /// this photonoxide doesn't read.
    pub fn read(dir: &Path) -> Result<Results> {
        let path = dir.join("results.json");
        let text = std::fs::read_to_string(&path).map_err(|e| Error::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        let results: Results = serde_json::from_str(&text).map_err(|e| Error::Parse {
            what: "results.json".into(),
            reason: e.to_string(),
        })?;
        if results.schema != SCHEMA {
            return Err(Error::Parse {
                what: "results.json".into(),
                reason: format!(
                    "schema {}, where this photonoxide reads {SCHEMA}",
                    results.schema
                ),
            });
        }
        Ok(results)
    }
}

/// The events of the run directory `dir`'s `events.jsonl`. A line that isn't one of
/// [`Event`]'s (a line cut off mid-write) is passed over.
///
/// # Errors
///
/// [`Error::Io`] if the record can't be read.
pub(crate) fn events_in(dir: &Path) -> Result<Vec<Event>> {
    let path = dir.join("events.jsonl");
    let text = std::fs::read_to_string(&path).map_err(|e| Error::Io {
        path: path.display().to_string(),
        reason: e.to_string(),
    })?;
    // each line's arrays in `events.bin` written back into it (crate::run::arrays)
    let mut arrays = crate::run::arrays::Arrays::new(dir);
    let mut events = Vec::new();
    for line in text.lines() {
        if let Ok(event) = serde_json::from_str(&arrays.inline(line)?) {
            events.push(event);
        }
    }
    Ok(events)
}

/// Writes `run`'s `results.json` from its record, if it has a directory, and the tables its
/// job's `[task.export]` asks for ([`crate::export::tables`]): every one it has by default.
///
/// # Errors
///
/// [`Error::Io`] if the record can't be read or the results written.
pub(crate) fn write_for(run: &Run) -> Result<()> {
    let dir = run.dir();
    if dir.as_os_str().is_empty() {
        return Ok(());
    }
    let events = events_in(dir)?;
    Results::collect(&events).write(dir)?;
    if let Some(which) = super::export_in(dir).tables() {
        crate::export::tables::write_tables_of(dir, &events, which.as_deref())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("photonoxide-results-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn s_event(wavelength_um: f64, base: f64) -> Event {
        Event::SParameters {
            wavelength_um,
            ports: vec!["1".into(), "2".into()],
            effective_indices: vec![2.4 + base, 2.5 + base],
            s: vec![
                vec![[base, 0.1], [0.2, -0.3]],
                vec![[0.4, 0.5], [-0.6, base / 3.0]],
            ],
        }
    }

    #[test]
    fn a_record_without_its_end_is_incomplete_and_its_s_matrix_is_by_wavelength() {
        let events = vec![
            Event::Started {
                job: "two".into(),
                kind: "fdfd".into(),
            },
            s_event(1.5, 0.7),
            s_event(1.6, 1.0 / 3.0),
        ];
        let r = Results::collect(&events);
        assert_eq!(
            (r.outcome.as_str(), r.job.as_str(), r.kind.as_str()),
            ("incomplete", "two", "fdfd")
        );
        let s = r.s_parameters.unwrap();
        assert_eq!(s.wavelengths_um, [1.5, 1.6]);
        assert_eq!(s.s.shape, [2, 2, 2]);
        let data = s.s.data.unwrap();
        // s[1][1][0] at 1.6 um: from port 1 into port 2, its imaginary part after its real one
        assert_eq!(&data[8 + 4..8 + 6], [0.4, 0.5]);
        assert_eq!(data[15], (1.0 / 3.0) / 3.0);
        assert_eq!(
            s.effective_indices.data.unwrap(),
            [2.4 + 0.7, 2.5 + 0.7, 2.4 + 1.0 / 3.0, 2.5 + 1.0 / 3.0]
        );
    }

    #[test]
    fn a_large_array_goes_to_a_file_with_the_same_bits() {
        let dir = temp("large");
        // 1100 wavelengths × 2 × 2 complex: 4400 values, above INLINE
        let events: Vec<Event> = (0..1100)
            .map(|k| s_event(1.5 + k as f64 * 1e-4, (k as f64).sqrt() / 7.0))
            .chain([Event::Finished {
                stopped: None,
                seconds: 1.0,
            }])
            .collect();
        let collected = Results::collect(&events);
        let expected = collected
            .s_parameters
            .as_ref()
            .unwrap()
            .s
            .data
            .clone()
            .unwrap();
        collected.write(&dir).unwrap();
        let r = Results::read(&dir).unwrap();
        let s = r.s_parameters.as_ref().unwrap();
        assert_eq!(s.s.file.as_deref(), Some("s_parameters.s.c128"));
        assert!(s.s.data.is_none());
        let bytes = std::fs::metadata(dir.join("s_parameters.s.c128"))
            .unwrap()
            .len();
        assert_eq!(bytes, 1100 * 4 * 2 * 8);
        assert_eq!(s.s.values(&dir).unwrap(), expected);
        // the indices, 2200 values, stay in results.json
        assert!(s.effective_indices.data.is_some() && s.effective_indices.file.is_none());
        assert_eq!(r.outcome, "finished");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_spectrum_is_kept_as_it_last_stood_and_another_schema_is_refused() {
        let spectrum = |time_um: f64, value: f64| Event::FdtdSpectrum {
            monitor: "out".into(),
            kind: "flux".into(),
            quantity: "transmission".into(),
            wavelengths_um: vec![1.5, 1.6],
            series: vec![super::super::SpectrumSeries {
                label: "T".into(),
                values: vec![value, value / 2.0],
            }],
            time_um,
            last: false,
        };
        let r = Results::collect(&[spectrum(10.0, 0.5), spectrum(20.0, 0.9)]);
        assert_eq!(r.spectra.len(), 1);
        assert_eq!(r.spectra[0].time_um, 20.0);
        assert_eq!(r.spectra[0].values.data.as_deref(), Some(&[0.9, 0.45][..]));
        assert_eq!(r.spectra[0].labels, ["T"]);
        let dir = temp("schema");
        let mut other = r;
        other.schema = SCHEMA + 1;
        other.write(&dir).unwrap();
        let e = Results::read(&dir).unwrap_err().to_string();
        assert!(e.contains("schema"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
