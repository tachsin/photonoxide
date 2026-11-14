//! The Chip page's quantum view's Gaussian part: squeezed, displaced vacuum on the chip's
//! external ports, through the chip's own S-matrix (its loss as vacuum noise), as
//! `photonoxide::quantum::Gaussian` computes it.
//!
//! It returns the output state's covariance, each port's squeezing in dB before and after the
//! chip, its mean photon number, one port's photon-number distribution (by hafnians) and its
//! Wigner function on a grid ([`chip_gaussian`]).

use photonoxide::portable;
use std::f64::consts::PI;
use std::path::Path;
use std::time::Instant;

use num_complex::Complex64 as c64;
use photonoxide::circuit::Component;
use photonoxide::quantum::{Gaussian, Transfer};
use serde::{Deserialize, Serialize};

use super::quantum::{MOST_PORTS, in_sweep};
use super::{Chip, complete};
use crate::studio::Studio;

/// The most squeezing a port takes, dB: 20 dB is r = 2.3, beyond any squeezer on a chip.
pub const MOST_SQUEEZING_DB: f64 = 20.0;
/// The largest displacement |α|.
pub const MOST_DISPLACEMENT: f64 = 5.0;
/// The most photons a port's distribution is given for: the library's hafnians take up to 30.
pub const MOST_PHOTONS: usize = 30;
/// The most points across the Wigner function's grid.
pub const MOST_WIGNER_POINTS: usize = 201;

/// What one port is given: squeezed vacuum (its squeezing in dB, at the angle φ of ξ = r e^{iφ}),
/// then displaced by α.
#[derive(Clone, Debug, Deserialize)]
pub struct SqueezedInput {
    pub port: String,
    /// 10 log₁₀ e^{2r}: the squeezed quadrature's variance below the vacuum's, dB (≥ 0).
    pub squeezing_db: f64,
    /// φ, degrees: x is squeezed at 0, p at 180.
    pub angle_deg: f64,
    /// α, its real and imaginary parts.
    pub displacement: [f64; 2],
}

/// What to show of the output: the port whose distribution and Wigner function are drawn.
#[derive(Clone, Debug, Deserialize)]
pub struct GaussianRequest {
    pub inputs: Vec<SqueezedInput>,
    /// The port shown: its photon numbers and its Wigner function.
    pub shown: String,
    /// P(0) … P(`photons`).
    pub photons: usize,
    /// The Wigner function's grid: ±`span` in x and p (vacuum σ = 1), `points` across.
    pub span: f64,
    pub points: usize,
}

/// A port of the output state.
#[derive(Debug, Serialize)]
pub struct PortState {
    pub(crate) port: String,
    /// The squeezing given it, dB (0 for none).
    pub(crate) given_db: f64,
    /// Its smallest and largest quadrature variances after the chip, in dB from the vacuum's:
    /// negative when squeezed.
    pub(crate) squeezed_db: f64,
    pub(crate) anti_squeezed_db: f64,
    /// ⟨n⟩ after the chip.
    pub(crate) mean_photons: f64,
    /// The phase of its most squeezed quadrature x cos θ + p sin θ, degrees.
    pub(crate) squeezed_at_deg: f64,
}

/// The state after the chip, as the view draws it.
#[derive(Debug, Serialize)]
pub struct GaussianView {
    pub(crate) wavelength_um: f64,
    pub(crate) ports: Vec<String>,
    /// V, 2n × 2n, row by row, (x₁ … x_n, p₁ … p_n), the vacuum's 1 on the diagonal.
    pub(crate) covariance: Vec<f64>,
    /// The output's means, (x₁ … x_n, p₁ … p_n).
    pub(crate) means: Vec<f64>,
    pub(crate) states: Vec<PortState>,
    /// The whole state's purity, 1/√det V: below 1 when the chip loses light.
    pub(crate) purity: f64,
    pub(crate) loss_modes: usize,
    pub(crate) shown: String,
    /// The shown port's P(0) … P(photons), and what lies beyond.
    pub(crate) distribution: Vec<f64>,
    pub(crate) beyond: f64,
    /// The Wigner function of the shown port on the grid: x across, p up, row by row (p rows).
    pub(crate) axis: Vec<f64>,
    pub(crate) wigner: Vec<f64>,
    pub(crate) notes: Vec<String>,
    pub(crate) seconds: f64,
}

/// Squeezed light on `chip`'s external ports at `wavelength_um`, through the chip.
#[tauri::command(async)]
pub fn chip_gaussian(
    state: tauri::State<'_, Studio>,
    chip: Chip,
    wavelength_um: f64,
    request: GaussianRequest,
) -> Result<GaussianView, String> {
    gaussian(&chip, &state.workspace(), wavelength_um, &request)
}

/// The amount of squeezing r of a variance `db` dB below the vacuum's: e^{−2r} = 10^{−dB/10}.
pub fn r_of_db(db: f64) -> f64 {
    db * std::f64::consts::LN_10 / 20.0
}

pub(crate) fn gaussian(
    chip: &Chip,
    root: &Path,
    wavelength_um: f64,
    request: &GaussianRequest,
) -> Result<GaussianView, String> {
    if request.inputs.is_empty() {
        return Err("give a port squeezed light first".into());
    }
    if request.photons > MOST_PHOTONS {
        return Err(format!(
            "a port's distribution is given up to {MOST_PHOTONS} photons (each a hafnian of twice \
             as many rows), not {}",
            request.photons
        ));
    }
    if !(request.span.is_finite() && request.span > 0.0 && request.span <= 50.0) {
        return Err(format!(
            "the Wigner function's span must be above 0 and at most 50 (the vacuum's σ is 1), not {}",
            request.span
        ));
    }
    if !(11..=MOST_WIGNER_POINTS).contains(&request.points) {
        return Err(format!(
            "the Wigner function takes 11 to {MOST_WIGNER_POINTS} points across, not {}",
            request.points
        ));
    }
    let wavelength = in_sweep(chip, wavelength_um)?;
    let circuit = complete(chip, root)?.compile().map_err(|e| e.to_string())?;
    let ports: Vec<String> = circuit.ports().iter().map(|p| p.name.clone()).collect();
    if ports.len() > MOST_PORTS {
        return Err(format!(
            "{} external ports: the view takes up to {MOST_PORTS}",
            ports.len()
        ));
    }
    let started = Instant::now();
    let transfer = Transfer::of(&circuit, wavelength).map_err(|e| e.to_string())?;
    let n = ports.len();
    let index = |name: &str| -> Result<usize, String> {
        transfer.port(name).ok_or_else(|| {
            format!(
                "no port {name}: the chip's ports are {}",
                transfer.ports().join(", ")
            )
        })
    };
    let mut given = vec![0.0; n];
    let mut seen = vec![false; n];
    let mut input = Gaussian::vacuum(n);
    for i in &request.inputs {
        let m = index(&i.port)?;
        if seen[m] {
            return Err(format!("port {} is given squeezed light twice", i.port));
        }
        if !(i.squeezing_db.is_finite() && (0.0..=MOST_SQUEEZING_DB).contains(&i.squeezing_db)) {
            return Err(format!(
                "a port's squeezing must be from 0 to {MOST_SQUEEZING_DB} dB, not {} dB at {}",
                i.squeezing_db, i.port
            ));
        }
        if !i.angle_deg.is_finite() {
            return Err(format!("the angle at {} isn't a number", i.port));
        }
        let alpha = c64::new(i.displacement[0], i.displacement[1]);
        if !portable::complex::abs(alpha).is_finite()
            || portable::complex::abs(alpha) > MOST_DISPLACEMENT
        {
            return Err(format!(
                "a displacement |α| is at most {MOST_DISPLACEMENT}, not {:.3} at {}",
                portable::complex::abs(alpha),
                i.port
            ));
        }
        seen[m] = true;
        given[m] = i.squeezing_db;
        input = input
            .squeezed(m, r_of_db(i.squeezing_db), i.angle_deg.to_radians())
            .and_then(|g| g.displaced(m, alpha))
            .map_err(|e| e.to_string())?;
    }
    let out = input.through(&transfer).map_err(|e| e.to_string())?;
    let db = |v: f64| 10.0 * portable::log10(v);
    let mut states = Vec::with_capacity(n);
    for (m, port) in ports.iter().enumerate() {
        let (low, high) = out.quadrature_extremes(m).map_err(|e| e.to_string())?;
        // the angle of the smallest variance: the 2 × 2 covariance's eigenvector
        let v = out.covariance();
        let (a, b, c) = (
            v[m * 2 * n + m],
            v[(m + n) * 2 * n + m + n],
            v[m * 2 * n + m + n],
        );
        let theta = 0.5 * portable::atan2(2.0 * c, a - b) + PI / 2.0;
        states.push(PortState {
            port: port.clone(),
            given_db: given[m],
            squeezed_db: db(low),
            anti_squeezed_db: db(high),
            mean_photons: out.mean_photons(m).map_err(|e| e.to_string())?,
            squeezed_at_deg: (theta.to_degrees() + 180.0).rem_euclid(180.0),
        });
    }
    let m = index(&request.shown)?;
    let single = out.reduced(&[m]).map_err(|e| e.to_string())?;
    let distribution = (0..=request.photons)
        .map(|k| single.probability(&[k]))
        .collect::<photonoxide::Result<Vec<f64>>>()
        .map_err(|e| e.to_string())?;
    let beyond = (1.0 - distribution.iter().sum::<f64>()).max(0.0);
    let mut notes = Vec::new();
    if beyond > 1e-3 {
        notes.push(format!(
            "{:.1}% of {}'s photon numbers lie beyond {}: the bars don't hold them all",
            beyond * 100.0,
            request.shown,
            request.photons
        ));
    }
    let k = request.points;
    let axis: Vec<f64> = (0..k)
        .map(|i| -request.span + 2.0 * request.span * i as f64 / (k - 1) as f64)
        .collect();
    let mut wigner = Vec::with_capacity(k * k);
    for &p in &axis {
        for &x in &axis {
            wigner.push(single.wigner(0, x, p).map_err(|e| e.to_string())?);
        }
    }
    Ok(GaussianView {
        wavelength_um: wavelength.to_um(),
        covariance: out.covariance().to_vec(),
        means: out.means().to_vec(),
        purity: out.purity(),
        loss_modes: transfer.loss_modes(),
        shown: request.shown.clone(),
        ports,
        states,
        distribution,
        beyond,
        axis,
        wigner,
        notes,
        seconds: started.elapsed().as_secs_f64(),
    })
}
