//! Photon pairs from a ring resonator by spontaneous four-wave mixing: the joint spectral
//! amplitude from the rings' field enhancements.
//!
//! The method is in [`ring_spectrum`]'s documentation.

use num_complex::Complex64 as c64;

use crate::nonlinear::pairs::{JointSpectrum, Pump};
use crate::{Error, Result};

/// The grid of a ring's joint spectrum: `points` signal and idler frequencies `half` (1/µm)
/// either side of each resonance, and `pump_points` pump frequencies `pump_half` either side of
/// the pump's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingGrid {
    /// The signal's and idler's half widths, 1/µm.
    pub half: f64,
    /// Points across each.
    pub points: usize,
    /// The pump's half width for the integral over ν_p, 1/µm.
    pub pump_half: f64,
    /// Points across it (trapezoidal).
    pub pump_points: usize,
}

/// Photon pairs from a ring resonator by spontaneous four-wave mixing: the joint spectral
/// amplitude from the rings' field enhancements at the pump, signal and idler resonances.
///
/// A ring point-coupled to its bus enhances a wave's field by F(ν) = ik / (1 − r a e^{iφ(ν)})
/// ([`crate::circuit::components::AllPassRing::field_enhancement`]), a Lorentzian near
/// resonance. Two pump photons at ν_p and ν − ν_p, each enhanced, make a signal and an idler
/// at ν_s + ν_i = ν, each enhanced on its way out, so the pairs' amplitude is
///
/// f(ν_s, ν_i) = F_P(ν_s + ν_i) F_s(ν_s) F_i(ν_i),
/// F_P(ν) = ∫ α(ν_p) α(ν − ν_p) F_p(ν_p) F_p(ν − ν_p) dν_p,
///
/// α the pump pulse's spectral amplitude (L. G. Helt, Z. Yang, M. Liscidini, J. E. Sipe, Opt.
/// Lett. 35, 3006 (2010), doi:10.1364/OL.35.003006, Eq. (2); Z. Vernon et al., Opt. Lett. 42,
/// 3638 (2017), doi:10.1364/OL.42.003638, Eqs. (3)–(4)). The phase mismatch along the ring is
/// left out: on resonance the azimuthal orders conserve momentum (2N_P = N_S + N_I), and over a
/// resonance's width it changes by far less than a radian in a ring of a few hundred µm.
/// Vernon et al.'s ring coupled to a second channel at the pump alone is a pump wave whose ring
/// loses more per round trip (the second channel's coupling), so its resonance is wider.
///
/// The joint spectrum of pairs from a ring, pumped by `pump` (centred on the pump resonance):
/// `enhancement` gives F(ν) of the pump, signal and idler waves (ν in 1/µm), `centers` the
/// signal's and idler's resonances (1/µm), on `grid`.
///
/// # Errors
///
/// [`Error::InvalidValue`] for a grid of fewer than two points or a half width that isn't
/// positive, and the enhancements' own.
pub fn ring_spectrum(
    pump: Pump,
    enhancement: [&(dyn Fn(f64) -> Result<c64> + Sync); 3],
    centers: [f64; 2],
    grid: RingGrid,
) -> Result<JointSpectrum> {
    let RingGrid {
        half,
        points,
        pump_half,
        pump_points,
    } = grid;
    if points < 2 || pump_points < 2 || !(half > 0.0 && pump_half > 0.0) {
        return Err(Error::invalid(
            "ring spectrum",
            "needs two or more points and positive half widths",
        ));
    }
    let [fp, fs, fi] = enhancement;
    let step = 2.0 * half / (points - 1) as f64;
    let axis = |c: f64| -> Vec<f64> { (0..points).map(|k| c - half + step * k as f64).collect() };
    let (signal, idler) = (axis(centers[0]), axis(centers[1]));
    let ts = signal.iter().map(|&v| fs(v)).collect::<Result<Vec<_>>>()?;
    let ti = idler.iter().map(|&v| fi(v)).collect::<Result<Vec<_>>>()?;
    // the pump pair's amplitude at each sum ν_s + ν_i: on a grid of 2·points − 1
    let p_step = 2.0 * pump_half / (pump_points - 1) as f64;
    let nu_p: Vec<f64> = (0..pump_points)
        .map(|k| pump.center - pump_half + p_step * k as f64)
        .collect();
    let first: Vec<c64> = nu_p
        .iter()
        .map(|&v| Ok(fp(v)? * pump.amplitude(v)))
        .collect::<Result<_>>()?;
    let sums: Vec<f64> = (0..2 * points - 1)
        .map(|k| signal[0] + idler[0] + step * k as f64)
        .collect();
    let pair = crate::parallel::map_in_order(&sums, |&total| -> Result<c64> {
        let mut acc = c64::new(0.0, 0.0);
        for (k, (&v, &a)) in nu_p.iter().zip(&first).enumerate() {
            let other = total - v;
            let w = if k == 0 || k + 1 == pump_points {
                0.5
            } else {
                1.0
            };
            acc += a * fp(other)? * pump.amplitude(other) * w;
        }
        Ok(acc * p_step)
    })
    .into_iter()
    .collect::<Result<Vec<c64>>>()?;
    let mut amplitude = Vec::with_capacity(points * points);
    for (s, a) in ts.iter().enumerate() {
        for (i, b) in ti.iter().enumerate() {
            amplitude.push(pair[s + i] * a * b);
        }
    }
    Ok(JointSpectrum {
        signal,
        idler,
        amplitude,
    })
}

/// [`ring_spectrum`] and its derivative along a parameter θ that moves the enhancements (a
/// ring's coupling, or Vernon et al.'s second channel at the pump) or the pump's bandwidth:
/// `derivative` gives dF/dθ of the pump, signal and idler waves, and `bandwidth` dΔν/dθ (α's
/// derivative in Δν is α · 4 ln 2 (ν − ν₀)²/Δν³). The product rule on f = F_P(ν_s + ν_i) F_s(ν_s)
/// F_i(ν_i), and on F_P's integral, ∫ α α (dF_p(ν_p) F_p(ν − ν_p) + F_p(ν_p) dF_p(ν − ν_p))
/// dν_p, on the same grid and quadrature: exact for the discrete spectrum (0.7, #761's I10).
///
/// # Errors
///
/// As [`ring_spectrum`].
pub fn ring_spectrum_derivative(
    pump: Pump,
    bandwidth: f64,
    enhancement: [&(dyn Fn(f64) -> Result<c64> + Sync); 3],
    derivative: [&(dyn Fn(f64) -> Result<c64> + Sync); 3],
    centers: [f64; 2],
    grid: RingGrid,
) -> Result<(JointSpectrum, JointSpectrum)> {
    let value = ring_spectrum(pump, enhancement, centers, grid)?;
    let RingGrid {
        half,
        points,
        pump_half,
        pump_points,
    } = grid;
    let [fp, fs, fi] = enhancement;
    let [dp, ds, di] = derivative;
    let step = 2.0 * half / (points - 1) as f64;
    let axis = |c: f64| -> Vec<f64> { (0..points).map(|k| c - half + step * k as f64).collect() };
    let (signal, idler) = (axis(centers[0]), axis(centers[1]));
    let eval = |f: &(dyn Fn(f64) -> Result<c64> + Sync), at: &[f64]| -> Result<Vec<c64>> {
        at.iter().map(|&v| f(v)).collect()
    };
    let (ts, dts) = (eval(fs, &signal)?, eval(ds, &signal)?);
    let (ti, dti) = (eval(fi, &idler)?, eval(di, &idler)?);
    let p_step = 2.0 * pump_half / (pump_points - 1) as f64;
    let nu_p: Vec<f64> = (0..pump_points)
        .map(|k| pump.center - pump_half + p_step * k as f64)
        .collect();
    let sums: Vec<f64> = (0..2 * points - 1)
        .map(|k| signal[0] + idler[0] + step * k as f64)
        .collect();
    let pair = crate::parallel::map_in_order(&sums, |&total| -> Result<(c64, c64)> {
        let (mut acc, mut dacc) = (c64::new(0.0, 0.0), c64::new(0.0, 0.0));
        for (k, &v) in nu_p.iter().enumerate() {
            let other = total - v;
            let w = if k == 0 || k + 1 == pump_points {
                0.5
            } else {
                1.0
            };
            let (av, ao) = (pump.amplitude(v), pump.amplitude(other));
            // dα/dΔν over α
            let slope = |nu: f64| {
                let d = nu - pump.center;
                4.0 * std::f64::consts::LN_2 * d * d
                    / (pump.bandwidth * pump.bandwidth * pump.bandwidth)
            };
            let alpha = av * ao * w;
            let d_alpha = alpha * (slope(v) + slope(other)) * bandwidth;
            let (a, b) = (fp(v)?, fp(other)?);
            acc += a * b * alpha;
            dacc += (dp(v)? * b + a * dp(other)?) * alpha + a * b * d_alpha;
        }
        Ok((acc * p_step, dacc * p_step))
    })
    .into_iter()
    .collect::<Result<Vec<(c64, c64)>>>()?;
    let mut amplitude = Vec::with_capacity(points * points);
    for s in 0..points {
        for i in 0..points {
            let (f, df) = pair[s + i];
            amplitude.push(df * ts[s] * ti[i] + f * dts[s] * ti[i] + f * ts[s] * dti[i]);
        }
    }
    Ok((
        value,
        JointSpectrum {
            signal,
            idler,
            amplitude,
        },
    ))
}
