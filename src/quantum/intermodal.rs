//! Photon pairs by spontaneous four-wave mixing with two pumps in two modes of one guide, one
//! of them delayed.
//!
//! The method is in [`DualPump`]'s documentation.

use std::f64::consts::TAU;

use num_complex::Complex64 as c64;

use crate::nonlinear::pairs::{JointSpectrum, Pump};
use crate::units::SPEED_OF_LIGHT;
use crate::{Error, Result};

/// A propagation constant β(ν) (rad/µm) of one wave's mode, ν in 1/µm.
pub type Beta<'a> = &'a (dyn Fn(f64) -> f64 + Sync);

/// Photon pairs by spontaneous four-wave mixing with two pumps in two modes of one guide, one
/// pump delayed: inter-modal phase matching and the pump-delayed scheme of S. Paesani et al.,
/// Nat. Commun. 11, 2505 (2020), doi:10.1038/s41467-020-16187-8.
///
/// Pump photons a (in one mode) and b (in another) make a signal and an idler, each in a mode of
/// its own, ν_a + ν_b = ν_s + ν_i. With both pumps' spectral amplitude α (one pulse split in
/// two), pump a delayed by τ, and the guide's propagation constants β_j(ν) of each wave's mode,
/// the pairs' amplitude is
///
/// f(ν_s, ν_i) = ∫ α(ν_a) e^{iω_a τ} α(ν_s + ν_i − ν_a) ∫₀^L e^{iΔk z} dz dν_a,
/// Δk = β_a(ν_a) + β_b(ν_s + ν_i − ν_a) − β_s(ν_s) − β_i(ν_i),
///
/// ω = 2πcν (W. P. Grice, I. A. Walmsley, Phys. Rev. A 56, 1627 (1997),
/// doi:10.1103/PhysRevA.56.1627, Eq. (9), with the two pumps' integral written out). The delay
/// matters where the pumps' group velocities differ: the faster pump, delayed, catches up with the
/// slower one inside the guide and passes it, so the pumps overlap only for a while, which
/// switches the interaction on and off smoothly and removes the sinc's side lobes; the overlap's
/// middle is at z = τ/(1/v_b − 1/v_a). The z integral is L e^{iΔkL/2} sinc(ΔkL/2).
///
/// Two pumps in two modes of a guide: the four waves' β, its length and the delay.
#[derive(Clone, Copy)]
pub struct DualPump<'a> {
    /// β of pump a, pump b, the signal and the idler.
    pub beta: [Beta<'a>; 4],
    /// L, µm.
    pub length: f64,
    /// τ, ps: pump a's delay behind pump b.
    pub delay: f64,
    /// Both pumps' spectrum (each pump half the pulse).
    pub pump: Pump,
}

/// The joint spectrum of a [`DualPump`] guide: `points` signal and idler frequencies `half`
/// (1/µm) either side of `centers` (1/µm), the pump's integral over `pump_points` frequencies
/// `pump_half` either side of its centre ([`DualPump`]).
///
/// # Errors
///
/// [`Error::InvalidValue`] for fewer than two points, a half width or length that isn't
/// positive, or a delay that isn't finite.
pub fn dual_pump_spectrum(
    guide: &DualPump<'_>,
    centers: [f64; 2],
    half: f64,
    points: usize,
    pump_half: f64,
    pump_points: usize,
) -> Result<JointSpectrum> {
    let pump = guide.pump;
    let amplitude = move |nu: f64| c64::new(pump.amplitude(nu), 0.0);
    shaped_dual_pump_spectrum(
        guide,
        &amplitude,
        centers,
        half,
        points,
        pump_half,
        pump_points,
    )
}

/// [`dual_pump_spectrum`] with a pump of any spectral amplitude, a measured one with its spectral
/// phase for instance: `amplitude(ν)` is α(ν), complex, ν in 1/µm, the same for both pumps. The
/// pump's integral is centred on `guide.pump.center`; `guide.pump.bandwidth` isn't used. In the
/// convention of the formula in [`DualPump`] (e^{+iωτ}, e^{+iΔkz}), a spectrum written with
/// e^{−iωτ} and e^{−iΔkz} (Paesani et al.'s Supplementary Eq. 3) takes the conjugate of its
/// α: the two joint spectra are then each other's conjugates, with the same purity.
///
/// # Errors
///
/// As [`dual_pump_spectrum`].
pub fn shaped_dual_pump_spectrum(
    guide: &DualPump<'_>,
    amplitude: &(dyn Fn(f64) -> c64 + Sync),
    centers: [f64; 2],
    half: f64,
    points: usize,
    pump_half: f64,
    pump_points: usize,
) -> Result<JointSpectrum> {
    if points < 2
        || pump_points < 2
        || !(half > 0.0 && pump_half > 0.0 && guide.length > 0.0)
        || !guide.delay.is_finite()
    {
        return Err(Error::invalid(
            "dual-pump spectrum",
            "needs two or more points, positive half widths and length, and a finite delay",
        ));
    }
    let [ba, bb, bs, bi] = guide.beta;
    let c = SPEED_OF_LIGHT * 1e-6; // µm/ps
    let step = 2.0 * half / (points - 1) as f64;
    let axis = |c0: f64| -> Vec<f64> { (0..points).map(|k| c0 - half + step * k as f64).collect() };
    let (signal, idler) = (axis(centers[0]), axis(centers[1]));
    let beta_s: Vec<f64> = signal.iter().map(|&v| bs(v)).collect();
    let beta_i: Vec<f64> = idler.iter().map(|&v| bi(v)).collect();
    let p_step = 2.0 * pump_half / (pump_points - 1) as f64;
    let nu_a: Vec<f64> = (0..pump_points)
        .map(|k| guide.pump.center - pump_half + p_step * k as f64)
        .collect();
    let sums: Vec<f64> = (0..2 * points - 1)
        .map(|k| signal[0] + idler[0] + step * k as f64)
        .collect();
    // for each sum: the pumps' weight and their β, per pump frequency
    let tables: Vec<Vec<(c64, f64)>> = crate::parallel::map_in_order(&sums, |&total| {
        nu_a.iter()
            .enumerate()
            .map(|(k, &v)| {
                let other = total - v;
                let w = if k == 0 || k + 1 == pump_points {
                    0.5
                } else {
                    1.0
                };
                let weight = amplitude(v)
                    * amplitude(other)
                    * c64::from_polar(w * p_step, TAU * c * v * guide.delay);
                (weight, ba(v) + bb(other))
            })
            .collect()
    });
    let half_l = 0.5 * guide.length;
    let rows: Vec<usize> = (0..points).collect();
    let amplitude: Vec<c64> = crate::parallel::map_in_order(&rows, |&s| {
        (0..points)
            .map(|i| {
                let table = &tables[s + i];
                let mut acc = c64::new(0.0, 0.0);
                for &(weight, pumps) in table {
                    let x = (pumps - beta_s[s] - beta_i[i]) * half_l;
                    let sinc = if x.abs() < 1e-4 {
                        1.0 - x * x / 6.0
                    } else {
                        x.sin() / x
                    };
                    acc += weight * c64::from_polar(guide.length * sinc, x);
                }
                acc
            })
            .collect::<Vec<c64>>()
    })
    .into_iter()
    .flatten()
    .collect();
    Ok(JointSpectrum {
        signal,
        idler,
        amplitude,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shaped pump given the Gaussian's own amplitude is the Gaussian pump, and a pump with a
    /// constant phase only multiplies the spectrum by that phase twice.
    #[test]
    fn shaped_pump_matches_the_gaussian() {
        let (b0, b1) = (|nu: f64| 2.0 * nu + 0.1 * nu * nu, |nu: f64| 2.1 * nu);
        let guide = DualPump {
            beta: [&b0, &b1, &b1, &b0],
            length: 1000.0,
            delay: 0.5,
            pump: Pump {
                center: 0.645,
                bandwidth: 0.002,
            },
        };
        let run = |f: &(dyn Fn(f64) -> c64 + Sync)| {
            shaped_dual_pump_spectrum(&guide, f, [0.63, 0.66], 0.004, 9, 0.006, 21)
                .unwrap()
                .amplitude
        };
        let plain = dual_pump_spectrum(&guide, [0.63, 0.66], 0.004, 9, 0.006, 21)
            .unwrap()
            .amplitude;
        let pump = guide.pump;
        let same = run(&|nu| c64::new(pump.amplitude(nu), 0.0));
        let turned = run(&|nu| c64::from_polar(pump.amplitude(nu), 0.3));
        for ((a, b), c) in plain.iter().zip(&same).zip(&turned) {
            assert!((a - b).norm() <= 1e-12 * a.norm().max(1e-300), "{a} {b}");
            let expected = a * c64::from_polar(1.0, 0.6);
            assert!(
                (c - expected).norm() <= 1e-12 * a.norm().max(1e-300),
                "{c} {expected}"
            );
        }
    }
}
