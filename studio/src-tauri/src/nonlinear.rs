//! The Materials page's nonlinear view: a `"nonlinear"` job computed in place, its events as its
//! run records them, and every catalogue material's transparency window, which its chart draws
//! with the job's wavelengths marked on top.

use photonoxide::job::{self, Event};
use photonoxide::material::catalogue::{self, Conditions, Edge};
use photonoxide::run::{Format, Job, Run, Stop};
use serde::Serialize;

/// The events a `"nonlinear"` job's run records, computed here without a folder: its start, its
/// setup, its map, its efficiency and its pairs, and its end. A run of the same job (`run_text`)
/// records the same events and writes its files beside them.
#[tauri::command(async)]
pub fn nonlinear_compute(text: String, format: Format) -> Result<Vec<Event>, String> {
    let job = Job::parse_as(&text, format).map_err(|e| e.to_string())?;
    if job.task().get("kind").and_then(|k| k.as_str()) != Some("nonlinear") {
        return Err("the job's task isn't of kind \"nonlinear\"".into());
    }
    let mut run = Run::in_memory();
    job::execute(&job, &mut run, &Stop::new(job.timeout())).map_err(|e| e.to_string())?;
    run.lines()
        .iter()
        .map(|l| serde_json::from_str(l).map_err(|e| e.to_string()))
        .collect()
}

/// A material's transparency window, as the chart draws it.
#[derive(Serialize)]
pub struct Window {
    id: String,
    name: String,
    formula: String,
    category: String,
    /// The composition it is taken at: an alloy's.
    composition: Option<f64>,
    band_edge: Edge,
    two_photon: Edge,
    multiphonon: Edge,
    /// The wavelengths its default index model covers, µm.
    index_range: (f64, f64),
}

/// Every catalogue material with an index model, its window at `temperature_k` (300 K when
/// not given), AlGaAs at the Al fraction `algaas_x` (0) and AlGaN at `algan_x` (none: its edge
/// then not specified).
#[tauri::command]
pub fn transparency_windows(
    temperature_k: Option<f64>,
    algaas_x: Option<f64>,
    algan_x: Option<f64>,
) -> Vec<Window> {
    catalogue::catalogue()
        .into_iter()
        .filter(|e| !e.index.is_empty())
        .filter_map(|e| {
            let composition = match e.id.as_str() {
                "algaas" => Some(algaas_x.unwrap_or(0.0)),
                "algan" => algan_x,
                _ => None,
            };
            let t = e.transparency(Conditions {
                temperature: temperature_k,
                composition,
            })?;
            let category = serde_json::to_value(e.category)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            Some(Window {
                id: e.id.clone(),
                name: e.name.clone(),
                formula: e.formula.clone(),
                category,
                composition,
                band_edge: t.band_edge,
                two_photon: t.two_photon,
                multiphonon: t.multiphonon,
                index_range: t.index_range,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PPLN: &str = include_str!("../../../jobs/ppln-shg.yaml");

    #[test]
    fn the_nonlinear_job_computes_as_its_run_records_it() {
        let events = nonlinear_compute(PPLN.into(), Format::Yaml).unwrap();
        let types: Vec<String> = events
            .iter()
            .map(|e| {
                serde_json::to_value(e).unwrap()["type"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(
            types,
            [
                "started",
                "nonlinear_setup",
                "phase_map",
                "nonlinear_efficiency",
                "finished"
            ]
        );
        // the same events as a run of the job, but for the time each took
        let job = Job::parse_as(PPLN, Format::Yaml).unwrap();
        let mut run = Run::in_memory();
        job::execute(&job, &mut run, &Stop::new(None)).unwrap();
        let again: Vec<Event> = run
            .lines()
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(events[..4], again[..4]);
        // another kind of job is refused
        let strip = include_str!("../../../jobs/strip-modes.yaml");
        assert!(nonlinear_compute(strip.into(), Format::Yaml).is_err());
    }

    #[test]
    fn every_material_has_a_window_with_its_edges_or_why_not() {
        let windows = transparency_windows(None, None, None);
        assert!(windows.len() > 10);
        let ln = windows.iter().find(|w| w.id == "linbo3").unwrap();
        assert_eq!(ln.band_edge.wavelength, Some(0.32));
        assert_eq!(ln.multiphonon.wavelength, Some(3.0));
        // AlGaN's edge needs its composition, AlGaAs's is GaAs's at x = 0
        let algan = windows.iter().find(|w| w.id == "algan").unwrap();
        assert!(
            algan.band_edge.wavelength.is_none() && algan.band_edge.how.contains("not specified")
        );
        let with_x = transparency_windows(Some(300.0), Some(0.3), Some(0.5));
        let algan = with_x.iter().find(|w| w.id == "algan").unwrap();
        assert!(algan.band_edge.wavelength.is_some());
        for w in &windows {
            for e in [&w.band_edge, &w.two_photon, &w.multiphonon] {
                assert!(
                    e.wavelength.is_some() == e.source.is_some()
                        || e.how.starts_with("not specified"),
                    "{}",
                    w.id
                );
            }
        }
    }
}
