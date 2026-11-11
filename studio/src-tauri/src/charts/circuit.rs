//! The chart of the lesson on circuits: a compact model fitted to a component's response. An
//! add-drop ring's through and drop fields (`circuit::components::AddDropRing`) sampled over a
//! band are fitted by vector fitting with common poles (`compact::fit::vector_fit`, Gustavsen and
//! Semlyen 1999), and the model's powers drawn over the samples' as the number of poles changes.

use photonoxide::circuit::Component;
use photonoxide::circuit::components::{AddDropRing, Dispersion};
use photonoxide::compact::fit::{Options, vector_fit};
use photonoxide::compact::model::laplace;
use photonoxide::units::Wavelength;

use super::{ChartData, ChartSpec, Curve, figure, invalid, param, sig};

/// The palette's grey, for the samples.
const GREY: usize = 7;
/// The ring's guide, as the ring charts take it: n_eff 2.4 and n_g 4.2 at 1.55 µm, 3 dB/cm.
const CENTRE: f64 = 1.55;
const NEFF: f64 = 2.4;
const NG: f64 = 4.2;
const LOSS: f64 = 3.0;
/// The samples the fit sees across the band.
const SAMPLES: usize = 401;

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
        id: "circuit-fit",
        title: "A compact model of a ring, pole by pole",
        about: "An add-drop ring's through and drop fields, sampled over a band, fitted by a sum of poles shared by both responses: with a pole for each resonance in the band the model follows the ring to round-off; with fewer, it misses the resonances it has no pole for.",
        computed_by: "circuit::components::AddDropRing::s_matrix at 401 wavelengths; compact::fit::vector_fit with N common poles, a complex model with a constant term (Gustavsen and Semlyen 1999); compact::model::laplace for s = −iω",
        params: vec![
            param(
                "poles",
                "Poles $N$",
                "",
                (1.0, 24.0, 6.0),
                1.0,
                false,
                "How many poles the model has, shared by the through and drop responses.",
            ),
            param(
                "radius",
                "Radius $R$",
                "um",
                (3.0, 50.0, 10.0),
                0.1,
                false,
                "The ring's radius: a larger ring has more resonances in the band.",
            ),
            param(
                "coupling",
                "Coupling $\\kappa^2$",
                "",
                (0.01, 0.5, 0.1),
                0.01,
                true,
                "Both couplers' power coupling: weaker coupling, narrower resonances.",
            ),
            param(
                "window",
                "Band",
                "nm",
                (2.0, 60.0, 30.0),
                1.0,
                false,
                "The band the samples cover, centred on 1.55 µm.",
            ),
        ],
    }]
}

/// The lesson's chart `id`, if it is one.
pub fn evaluate(id: &str, at: impl Fn(&str) -> f64) -> Option<photonoxide::Result<ChartData>> {
    Some(match id {
        "circuit-fit" => fit(
            at("poles") as usize,
            at("radius"),
            at("coupling"),
            at("window"),
        ),
        _ => return None,
    })
}

/// The ring's through and drop fields at `wavelengths`, and the ring's FSR at the centre, µm.
fn sampled(
    radius: f64,
    coupling: f64,
    wavelengths: &[Wavelength],
) -> photonoxide::Result<(
    Vec<num_complex::Complex64>,
    Vec<num_complex::Complex64>,
    f64,
)> {
    let ring =
        AddDropRing::new(Dispersion::new(Wavelength::um(CENTRE)?, NEFF, NG).with_loss(LOSS))?;
    let values = [2.0 * std::f64::consts::PI * radius, coupling, coupling];
    let (mut through, mut drop) = (Vec::new(), Vec::new());
    for &w in wavelengths {
        let s = ring.s_matrix(w, &values)?;
        through.push(s[(1, 0)]);
        drop.push(s[(3, 0)]);
    }
    let fsr = ring.fsr(Wavelength::um(CENTRE)?, values[0])?;
    Ok((through, drop, fsr))
}

/// The ring over `window_nm` fitted with `poles` poles.
pub fn fit(
    poles: usize,
    radius: f64,
    coupling: f64,
    window_nm: f64,
) -> photonoxide::Result<ChartData> {
    if poles == 0 {
        return Err(invalid("poles", "a model needs at least one pole"));
    }
    let half = window_nm * 1e-3 / 2.0;
    let wavelengths: Vec<Wavelength> = (0..SAMPLES)
        .map(|k| Wavelength::um(CENTRE - half + 2.0 * half * k as f64 / (SAMPLES - 1) as f64))
        .collect::<photonoxide::Result<_>>()?;
    let (through, drop, fsr) = sampled(radius, coupling, &wavelengths)?;
    let s: Vec<_> = wavelengths.iter().map(|&w| laplace(w)).collect();
    let model = vector_fit(&s, &[through.clone(), drop.clone()], &Options::new(poles))?;
    let mut series = Vec::new();
    let mut largest = 0.0_f64;
    for (e, (name, data)) in [("through", &through), ("drop", &drop)]
        .into_iter()
        .enumerate()
    {
        let samples: Vec<[f64; 2]> = wavelengths
            .iter()
            .zip(data)
            .map(|(w, f)| [w.to_um(), f.norm_sqr()])
            .collect();
        let fitted: Vec<[f64; 2]> = wavelengths
            .iter()
            .zip(&s)
            .map(|(w, &x)| [w.to_um(), model.evaluate(e, x).norm_sqr()])
            .collect();
        largest = largest.max(fitted.iter().map(|p| p[1]).fold(0.0, f64::max));
        series.push(curve(&format!("{name}: the ring"), samples, true, GREY));
        series.push(curve(&format!("{name}: the model"), fitted, false, e));
    }
    let resonances = (window_nm * 1e-3 / fsr).floor() as usize
        + usize::from((window_nm * 1e-3 / fsr).fract() > 0.0);
    let stable = model.poles.iter().all(|p| p.re < 0.0);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "wavelength".to_owned(),
        x_length: true,
        y_label: "power".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.05_f64.max(1.02 * largest)]),
        series,
        marker: None,
        figures: vec![
            figure(
                "Error, rms",
                model.error.rms_relative,
                2,
                "",
                "|f − f_model| over |f|, both responses, at the samples",
            ),
            figure(
                "Error, largest",
                model.error.max_relative,
                2,
                "",
                "the largest |f − f_model| over the largest |f|",
            ),
            figure(
                "Resonances in the band",
                resonances as f64,
                2,
                "",
                &format!("the FSR is {} nm: a pole each, at least", sig(fsr * 1e3, 3)),
            ),
            figure(
                "Poles",
                poles as f64,
                2,
                "",
                if stable {
                    "every pole stable: Re s < 0"
                } else {
                    "a pole unstable"
                },
            ),
            figure(
                "Largest model power",
                largest,
                4,
                "",
                "above 1 would make light: a model for a circuit must be passive",
            ),
        ],
        note: format!(
            "Exact ring, no grid: n_eff {NEFF}, n_g {NEFF_G} and {LOSS} dB/cm at {CENTRE} µm, \
             R = {} µm, κ² = {} on both couplers; {SAMPLES} samples over {} nm, fitted by \
             {poles} common poles and a constant in {} relocations.",
            sig(radius, 3),
            sig(coupling, 3),
            sig(window_nm, 3),
            model.iterations,
            NEFF_G = NG,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fig(data: &ChartData, label: &str) -> f64 {
        data.figures
            .iter()
            .find(|f| f.label.starts_with(label))
            .unwrap_or_else(|| panic!("no figure {label}"))
            .value
            .unwrap()
    }

    /// With a pole for every resonance in the band the model follows the ring closely; with too
    /// few, it misses resonances and its error is large.
    #[test]
    fn enough_poles_fit_the_ring() {
        let enough = fit(6, 10.0, 0.1, 30.0).unwrap();
        let resonances = fig(&enough, "Resonances");
        assert!((3.0..=4.0).contains(&resonances), "{resonances}");
        assert!(
            fig(&enough, "Error, rms") < 1e-3,
            "{}",
            fig(&enough, "Error, rms")
        );
        assert!(fig(&enough, "Largest model power") <= 1.0 + 1e-3);
        let few = fit(1, 10.0, 0.1, 30.0).unwrap();
        assert!(
            fig(&few, "Error, rms") > 0.05,
            "{}",
            fig(&few, "Error, rms")
        );
        // the samples are the ring's own
        let w = Wavelength::um(CENTRE).unwrap();
        let (t, d, _) = sampled(10.0, 0.1, &[w]).unwrap();
        let mid = enough.series[0].points[SAMPLES / 2];
        assert!((mid[0] - CENTRE).abs() < 1e-12 && mid[1] == t[0].norm_sqr());
        assert_eq!(enough.series[2].points[SAMPLES / 2][1], d[0].norm_sqr());
        assert!(fit(0, 10.0, 0.1, 30.0).is_err());
    }
}
