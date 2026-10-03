//! The Materials page's commands: the catalogue, and a model's indices over its range or at
//! one wavelength.

use photonoxide::material::catalogue::{self, Axis, Conditions, Entry, IndexModel};
use photonoxide::units::Wavelength;
use serde::Serialize;

/// One index of a model over its range of wavelengths.
#[derive(Serialize)]
pub struct Curve {
    axis: Axis,
    /// The shortest and longest wavelengths (µm) the material is valid for.
    range: (f64, f64),
    /// Wavelengths in µm, spaced evenly in frequency's logarithm.
    wavelength: Vec<f64>,
    n: Vec<f64>,
    k: Vec<f64>,
    group: Vec<f64>,
}

/// One index of a model at one wavelength.
#[derive(Serialize)]
pub struct Point {
    axis: Axis,
    n: f64,
    k: f64,
    eps_re: f64,
    eps_im: f64,
    group: f64,
}

fn model(id: &str, model: &str) -> Result<IndexModel, String> {
    let entry = catalogue::entry(id).ok_or_else(|| format!("no material {id}"))?;
    entry
        .index
        .into_iter()
        .find(|m| m.id == model)
        .ok_or_else(|| format!("{id} has no model {model}"))
}

fn conditions(temperature: Option<f64>, composition: Option<f64>) -> Conditions {
    Conditions {
        temperature,
        composition,
    }
}

/// Every material of the catalogue.
#[tauri::command]
pub fn materials() -> Vec<Entry> {
    catalogue::catalogue()
}

/// A model's indices at `points` wavelengths across each material's range.
#[tauri::command]
pub fn material_curves(
    id: String,
    model_id: String,
    temperature: Option<f64>,
    composition: Option<f64>,
    points: usize,
) -> Result<Vec<Curve>, String> {
    let m = model(&id, &model_id)?;
    let materials = m
        .materials(conditions(temperature, composition))
        .map_err(|e| e.to_string())?;
    let points = points.clamp(2, 4000);
    m.axes
        .iter()
        .zip(&materials)
        .map(|(&axis, material)| {
            let (lo, hi) = material.range();
            let (lo, hi) = (lo.to_um(), hi.to_um());
            let wavelength: Vec<f64> = (0..points)
                .map(|i| lo * (hi / lo).powf(i as f64 / (points - 1) as f64))
                .map(|w: f64| w.clamp(lo, hi))
                .collect();
            let mut curve = Curve {
                axis,
                range: (lo, hi),
                wavelength: Vec::with_capacity(points),
                n: Vec::with_capacity(points),
                k: Vec::with_capacity(points),
                group: Vec::with_capacity(points),
            };
            for w in wavelength {
                let lam = Wavelength::um(w).map_err(|e| e.to_string())?;
                let index = material.refractive_index(lam).map_err(|e| e.to_string())?;
                curve.wavelength.push(w);
                curve.n.push(index.re);
                curve.k.push(index.im);
                curve
                    .group
                    .push(material.group_index(lam).unwrap_or(f64::NAN));
            }
            Ok(curve)
        })
        .collect()
}

/// A model's indices at one wavelength (µm).
#[tauri::command]
pub fn material_at(
    id: String,
    model_id: String,
    temperature: Option<f64>,
    composition: Option<f64>,
    wavelength: f64,
) -> Result<Vec<Point>, String> {
    let m = model(&id, &model_id)?;
    let materials = m
        .materials(conditions(temperature, composition))
        .map_err(|e| e.to_string())?;
    let lam = Wavelength::um(wavelength).map_err(|e| e.to_string())?;
    m.axes
        .iter()
        .zip(&materials)
        .map(|(&axis, material)| {
            let index = material.refractive_index(lam).map_err(|e| e.to_string())?;
            let eps = material.permittivity(lam).map_err(|e| e.to_string())?;
            Ok(Point {
                axis,
                n: index.re,
                k: index.im,
                eps_re: eps.re,
                eps_im: eps.im,
                group: material.group_index(lam).map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_model_gives_curves_and_a_point() {
        for e in materials() {
            for m in &e.index {
                let curves = material_curves(e.id.clone(), m.id.clone(), None, None, 50).unwrap();
                assert_eq!(curves.len(), m.axes.len());
                let c = &curves[0];
                assert_eq!(c.wavelength.len(), 50);
                assert!(c.n.iter().all(|n| n.is_finite() && *n > 1.0), "{}", m.id);
                let mid = (c.range.0 * c.range.1).sqrt();
                let p = material_at(e.id.clone(), m.id.clone(), None, None, mid).unwrap();
                assert!((p[0].eps_re - p[0].n * p[0].n).abs() < 1e-9);
            }
        }
        assert!(material_at("si".into(), "li-1980".into(), None, None, 0.5).is_err());
        assert!(material_curves("nothing".into(), "x".into(), None, None, 10).is_err());
    }
}
