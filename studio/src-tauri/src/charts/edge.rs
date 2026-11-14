//! The chart of the lesson on edge couplers: how much of a fibre's mode a chip's mode takes, both
//! Gaussian, against their lateral offset, at a tilt between them. Each field across the facet is
//! `eme::Beam`'s Gaussian (tilted by `Beam::tilted`), their overlap summed over the plane; the
//! vertical overlap, neither offset nor tilted that way, multiplies it. Marcuse's closed form for
//! two offset Gaussians is the check.

use photonoxide::eme::Beam;
use photonoxide::portable;
use photonoxide::units::{Length, Wavelength};

use super::{ChartData, ChartSpec, Curve, figure, invalid, param, sig};

/// The palette's grey, for reference curves.
const GREY: usize = 7;
/// The wavelength, µm.
const WAVELENGTH: f64 = 1.55;
/// The fibres compared: their name and mode-field diameter, µm.
const FIBRES: [(&str, f64); 3] = [
    ("standard single-mode, 10.4 µm", 10.4),
    ("high-NA, 4 µm", 4.0),
    ("lensed, 2.5 µm", 2.5),
];

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: usize) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: Some(colour),
    }
}

/// The lesson's chart.
pub fn specs() -> Vec<ChartSpec> {
    vec![ChartSpec {
        id: "edge-overlap",
        title: "A chip's mode against a fibre's",
        about: "The share of a fibre's power a chip's mode takes at the facet, both Gaussian, against their lateral offset: for the fibre chosen, at the tilt chosen and without it, and for three common fibres. The modes' sizes must match, and the smaller they are the less offset they forgive.",
        computed_by: "eme::Beam::gaussian and Beam::tilted for each field across the facet, their overlap |∫ f g*|²/(∫|f|² ∫|g|²) summed over 2401 points; the vertical overlap 2w₁w₂/(w₁² + w₂²); Marcuse's closed form as the check",
        params: vec![
            param(
                "chip",
                "Chip mode $2w_c$",
                "um",
                (0.5, 12.0, 3.0),
                0.1,
                false,
                "The chip mode's diameter at the facet, where its intensity falls to 1/e²: the inverse taper's tip sets it.",
            ),
            param(
                "fibre",
                "Fibre mode $2w_f$",
                "um",
                (1.0, 12.0, 10.4),
                0.1,
                false,
                "The fibre mode's diameter, 1/e² in intensity: 10.4 µm for a standard single-mode fibre at 1.55 µm.",
            ),
            param(
                "offset",
                "Offset $d$",
                "um",
                (0.0, 4.0, 0.5),
                0.05,
                false,
                "How far the fibre's axis is from the chip mode's, sideways: the figures are at it.",
            ),
            param(
                "tilt",
                "Tilt $\\theta$ (°)",
                "",
                (0.0, 10.0, 0.0),
                0.1,
                false,
                "The angle between the two axes, in the plane of the offset, in index-matching gel of index 1.45.",
            ),
        ],
    }]
}

/// The lesson's chart `id`, if it is one.
pub fn evaluate(id: &str, at: impl Fn(&str) -> f64) -> Option<photonoxide::Result<ChartData>> {
    Some(match id {
        "edge-overlap" => overlap(at("chip"), at("fibre"), at("offset"), at("tilt")),
        _ => return None,
    })
}

/// The gel between chip and fibre: the medium the tilt is in.
const GEL: f64 = 1.45;

/// The power coupling between a Gaussian of diameter `a` µm and one of `b` µm offset by `d` µm
/// and tilted by `tilt` radians, both circular: the horizontal overlap summed, times the
/// vertical one.
fn coupling(a: f64, b: f64, d: f64, tilt: f64) -> photonoxide::Result<f64> {
    let k = 2.0 * std::f64::consts::PI / WAVELENGTH;
    let chip = Beam::gaussian(Length::um(a / 2.0), Length::ZERO)?;
    let fibre = Beam::gaussian(Length::um(b / 2.0), Length::um(d))?.tilted(tilt, GEL)?;
    let reach = 6.0 * (a.max(b) / 2.0) + d;
    let n = 2400;
    let h = 2.0 * reach / n as f64;
    let mut sum = num_complex::Complex64::new(0.0, 0.0);
    for j in 0..=n {
        let x = -reach + h * j as f64;
        let weight = if j == 0 || j == n { 0.5 } else { 1.0 };
        sum += weight * chip.field(x, k) * fibre.field(x, k).conj();
    }
    let across = (sum * h).norm_sqr() / (chip.norm() * fibre.norm());
    let (wa, wb) = (a / 2.0, b / 2.0);
    let vertical = 2.0 * wa * wb / (wa * wa + wb * wb);
    Ok(across * vertical)
}

/// Marcuse's closed form for two circular Gaussians of 1/e field radii `wa` and `wb`, offset by
/// `d` and parallel: (2w₁w₂/(w₁² + w₂²))² exp(−2d²/(w₁² + w₂²)).
fn marcuse(wa: f64, wb: f64, d: f64) -> f64 {
    let s = wa * wa + wb * wb;
    portable::powi(2.0 * wa * wb / s, 2) * portable::exp(-2.0 * d * d / s)
}

fn db(x: f64) -> f64 {
    10.0 * portable::log10(x)
}

/// The coupling against the offset, from −4 to 4 µm.
pub fn overlap(
    chip: f64,
    fibre: f64,
    offset: f64,
    tilt_deg: f64,
) -> photonoxide::Result<ChartData> {
    if chip <= 0.0 || fibre <= 0.0 {
        return Err(invalid("chip", "the modes need a size"));
    }
    Wavelength::um(WAVELENGTH)?;
    let tilt = tilt_deg.to_radians();
    let offsets: Vec<f64> = (0..=160)
        .map(|k| -4.0 + 8.0 * f64::from(k) / 160.0)
        .collect();
    let mut series = Vec::new();
    let mut chosen = Vec::with_capacity(offsets.len());
    for &d in &offsets {
        chosen.push([d, db(coupling(chip, fibre, d, tilt)?)]);
    }
    series.push(curve(
        &format!("fibre of {} µm, at {}°", sig(fibre, 3), sig(tilt_deg, 2)),
        chosen,
        false,
        0,
    ));
    if tilt_deg > 0.0 {
        let mut straight = Vec::with_capacity(offsets.len());
        for &d in &offsets {
            straight.push([d, db(coupling(chip, fibre, d, 0.0)?)]);
        }
        series.push(curve("the same, not tilted", straight, true, 0));
    }
    for (k, (name, mfd)) in FIBRES.iter().enumerate() {
        let mut points = Vec::with_capacity(offsets.len());
        for &d in &offsets {
            points.push([d, db(coupling(chip, *mfd, d, 0.0)?)]);
        }
        series.push(curve(name, points, true, if k == 0 { GREY } else { k + 1 }));
    }
    let here = coupling(chip, fibre, offset, tilt)?;
    let best = coupling(chip, fibre, 0.0, 0.0)?;
    // the offset that costs 1 dB more than the best: exp(−2d²/(w₁² + w₂²)) = 10^(−0.1)
    let s = portable::powi(chip / 2.0, 2) + portable::powi(fibre / 2.0, 2);
    let one_db = (s * 0.1 * std::f64::consts::LN_10 / 2.0).sqrt();
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "offset d".to_owned(),
        x_length: true,
        y_label: "coupling (dB)".to_owned(),
        y_log: false,
        y_range: Some([-20.0, 0.5]),
        series,
        marker: Some(offset),
        figures: vec![
            figure(
                &format!("At {} µm", sig(offset, 3)),
                db(here),
                3,
                "dB",
                &format!("{} % of the power", sig(100.0 * here, 3)),
            ),
            figure(
                "Aligned",
                db(best),
                3,
                "dB",
                "the size mismatch alone: (2w_cw_f/(w_c² + w_f²))²",
            ),
            figure(
                "Offset for 1 dB more",
                one_db,
                3,
                "µm",
                "√((w_c² + w_f²) ln 10/20): the alignment the fibre needs",
            ),
            figure(
                "Marcuse, not tilted",
                db(marcuse(chip / 2.0, fibre / 2.0, offset)),
                3,
                "dB",
                "the closed form at this offset, the check of the sum",
            ),
        ],
        note: format!(
            "Gaussian fields, their overlap summed on 2401 points: no other grid. Circular modes \
             of {} and {} µm (1/e² diameters) at {WAVELENGTH} µm, the tilt in a gel of index \
             {GEL}; a real chip mode is not quite Gaussian, which the spot-size chart computes.",
            sig(chip, 3),
            sig(fibre, 3)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The summed overlap of two parallel Gaussians is Marcuse's closed form, and a tilt only
    /// lowers it.
    #[test]
    fn the_overlap_is_marcuses() {
        for (a, b, d) in [
            (3.0, 10.4, 0.0),
            (3.0, 10.4, 1.5),
            (2.5, 2.5, 0.7),
            (8.0, 4.0, -2.0),
        ] {
            let summed = coupling(a, b, d, 0.0).unwrap();
            let closed = marcuse(a / 2.0, b / 2.0, d);
            assert!(
                (summed - closed).abs() < 1e-10,
                "{a}, {b}, {d}: {summed} against {closed}"
            );
        }
        assert!((coupling(4.0, 4.0, 0.0, 0.0).unwrap() - 1.0).abs() < 1e-10);
        assert!(
            coupling(3.0, 10.4, 0.0, 2f64.to_radians()).unwrap()
                < coupling(3.0, 10.4, 0.0, 0.0).unwrap()
        );
        let data = overlap(3.0, 10.4, 0.5, 0.0).unwrap();
        let fig = |label: &str| {
            data.figures
                .iter()
                .find(|f| f.label.starts_with(label))
                .unwrap()
                .value
                .unwrap()
        };
        assert!((fig("At") - fig("Marcuse")).abs() < 1e-8);
        // 1 dB more at the offset the figure gives
        let d = fig("Offset for 1 dB");
        let lost = db(coupling(3.0, 10.4, d, 0.0).unwrap()) - fig("Aligned");
        assert!((lost + 1.0).abs() < 1e-8, "{lost}");
        // a 3 µm mode against standard fibre loses 5.47 dB from the size alone
        assert!((fig("Aligned") + 5.472).abs() < 0.001, "{}", fig("Aligned"));
    }
}
