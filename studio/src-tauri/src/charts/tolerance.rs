//! The chart of the lesson on tolerances: how far a ring's resonance moves when the strip comes
//! out wider or narrower, thicker or thinner, than drawn. The strip's TE mode by the effective
//! index method (`mode::eim::Ridge::strip`, silicon and silica as the strip charts take them), its group
//! index from its index at λ ± 10 nm, and the shift Δλ = λ Δn_eff/n_g of a resonance held at its
//! order; at the corners every combination of the two errors.

use photonoxide::material::{silica, silicon};
use photonoxide::mode::Polarization;
use photonoxide::mode::eim::Ridge;
use photonoxide::units::{Length, Wavelength};

use super::{ChartData, ChartSpec, Curve, figure, invalid, param, sig};

/// The wavelength, µm.
const WAVELENGTH: f64 = 1.55;
/// The ring the shift is compared with: its radius, µm.
const RING_RADIUS: f64 = 10.0;

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
        id: "tolerance-ring",
        title: "A ring's resonance under process errors",
        about: "A silicon strip in silica, its width or its thickness off by δ: how far a ring's resonance near 1.55 µm moves, Δλ = λΔn_eff/n_g, the ring's order held. Below, the four corners, both errors at once, against the ring's free spectral range.",
        computed_by: "mode::eim::Ridge::strip(..).mode (TE, the effective index method) on material::silicon and material::silica at each width and thickness; the group index from the index at λ ± 10 nm",
        params: vec![
            param(
                "width",
                "Width $w$",
                "nm",
                (300.0, 1000.0, 500.0),
                1.0,
                false,
                "The strip's drawn width.",
            ),
            param(
                "thickness",
                "Thickness $t$",
                "nm",
                (150.0, 400.0, 220.0),
                1.0,
                false,
                "The silicon film's nominal thickness.",
            ),
            param(
                "bias",
                "Width corner $\\pm\\Delta w$",
                "nm",
                (0.0, 30.0, 10.0),
                0.5,
                false,
                "How far the corners take the width: lithography and etch together.",
            ),
            param(
                "film",
                "Thickness corner $\\pm\\Delta t$",
                "nm",
                (0.0, 20.0, 5.0),
                0.5,
                false,
                "How far the corners take the film's thickness.",
            ),
        ],
    }]
}

/// The lesson's chart `id`, if it is one.
pub fn evaluate(id: &str, at: impl Fn(&str) -> f64) -> Option<photonoxide::Result<ChartData>> {
    Some(match id {
        "tolerance-ring" => ring(at("width"), at("thickness"), at("bias"), at("film")),
        _ => return None,
    })
}

/// The TE mode's effective index of a strip `width` by `thickness` nm at `l` µm.
fn index(width: f64, thickness: f64, l: f64) -> photonoxide::Result<f64> {
    let lam = Wavelength::um(l)?;
    let (si, ox) = (
        silicon().refractive_index(lam)?.re,
        silica().refractive_index(lam)?.re,
    );
    Ok(
        Ridge::strip(ox, si, ox, Length::nm(thickness), Length::nm(width))?
            .mode(Polarization::Te, lam)?
            .effective_index,
    )
}

/// Its group index, n − λ dn/dλ, from λ ± 10 nm.
fn group(width: f64, thickness: f64) -> photonoxide::Result<f64> {
    let dl = 0.01;
    let n = index(width, thickness, WAVELENGTH)?;
    let slope = (index(width, thickness, WAVELENGTH + dl)?
        - index(width, thickness, WAVELENGTH - dl)?)
        / (2.0 * dl);
    Ok(n - WAVELENGTH * slope)
}

/// The resonance's shift, nm, when the strip is `dw` wider and `dt` thicker.
fn shift(
    width: f64,
    thickness: f64,
    dw: f64,
    dt: f64,
    n0: f64,
    ng: f64,
) -> photonoxide::Result<f64> {
    Ok(WAVELENGTH * 1e3 * (index(width + dw, thickness + dt, WAVELENGTH)? - n0) / ng)
}

/// The shifts against a width or a thickness error from −20 to 20 nm, and the corners.
pub fn ring(width: f64, thickness: f64, bias: f64, film: f64) -> photonoxide::Result<ChartData> {
    if width - 20.0 <= 0.0 || thickness - 20.0 <= 0.0 {
        return Err(invalid(
            "width",
            "the strip must stay wider and thicker than the errors",
        ));
    }
    let n0 = index(width, thickness, WAVELENGTH)?;
    let ng = group(width, thickness)?;
    let errors: Vec<f64> = (0..=40).map(|k| -20.0 + f64::from(k)).collect();
    let (mut by_width, mut by_film) = (Vec::new(), Vec::new());
    for &d in &errors {
        by_width.push([d, shift(width, thickness, d, 0.0, n0, ng)?]);
        by_film.push([d, shift(width, thickness, 0.0, d, n0, ng)?]);
    }
    let per_width = (shift(width, thickness, 1.0, 0.0, n0, ng)?
        - shift(width, thickness, -1.0, 0.0, n0, ng)?)
        / 2.0;
    let per_film = (shift(width, thickness, 0.0, 1.0, n0, ng)?
        - shift(width, thickness, 0.0, -1.0, n0, ng)?)
        / 2.0;
    let mut corners = Vec::new();
    for (sw, st) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        corners.push((
            sw,
            st,
            shift(width, thickness, sw * bias, st * film, n0, ng)?,
        ));
    }
    let (lo, hi) = corners
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), c| {
            (a.min(c.2), b.max(c.2))
        });
    let fsr = WAVELENGTH * WAVELENGTH * 1e3 / (ng * 2.0 * std::f64::consts::PI * RING_RADIUS);
    let sign = |s: f64| if s > 0.0 { "+" } else { "−" };
    let mut figures = vec![
        figure(
            "$d\\lambda/dw$",
            per_width,
            3,
            "nm/nm",
            "the resonance's shift per nanometre of width",
        ),
        figure(
            "$d\\lambda/dt$",
            per_film,
            3,
            "nm/nm",
            "per nanometre of thickness: a thin film is far more sensitive to its thickness",
        ),
        figure(
            "$n_\\text{eff}$",
            n0,
            4,
            "",
            "nominal, by the effective index method",
        ),
        figure("$n_g$", ng, 4, "", "nominal, from λ ± 10 nm"),
    ];
    for (sw, st, s) in &corners {
        figures.push(figure(
            &format!(
                "Corner $w{}\\Delta w$, $t{}\\Delta t$",
                sign(*sw),
                sign(*st)
            ),
            *s,
            3,
            "nm",
            "both errors at once",
        ));
    }
    figures.push(figure(
        "Corners' spread",
        hi - lo,
        3,
        "nm",
        &format!(
            "{} of the FSR of a ring of {RING_RADIUS} µm radius ({} nm)",
            sig((hi - lo) / fsr, 2),
            sig(fsr, 3)
        ),
    ));
    let top = by_width
        .iter()
        .chain(&by_film)
        .map(|p| p[1].abs())
        .fold(0.0, f64::max);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "error δ (nm)".to_owned(),
        x_length: false,
        y_label: "resonance shift Δλ (nm)".to_owned(),
        y_log: false,
        y_range: Some([-1.1 * top, 1.1 * top]),
        series: vec![
            curve("width w + δ", by_width, false, 0),
            curve("thickness t + δ", by_film, false, 1),
        ],
        marker: None,
        figures,
        note: format!(
            "The effective index method on {} × {} nm of silicon in silica at {WAVELENGTH} µm, \
             each error solved anew, not linearized; the effective index method overstates a \
             strip's index, so its slopes are estimates, as the strip lesson compares.",
            sig(width, 3),
            sig(thickness, 3)
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

    /// A wider or thicker strip holds its mode more, so the resonance moves to longer
    /// wavelengths; the thickness of a 220 nm film counts for more than the width of a 500 nm
    /// strip; and the curves are the solver's index differences.
    #[test]
    fn the_shifts_are_the_indices() {
        let data = ring(500.0, 220.0, 10.0, 5.0).unwrap();
        let (dw, dt) = (fig(&data, "$d\\lambda/dw$"), fig(&data, "$d\\lambda/dt$"));
        assert!(dw > 0.0 && dt > 0.0, "{dw}, {dt}");
        assert!(dt > dw, "{dt} against {dw}");
        let n0 = index(500.0, 220.0, WAVELENGTH).unwrap();
        let ng = group(500.0, 220.0).unwrap();
        let p = data.series[0].points[30];
        assert_eq!(p[0], 10.0);
        let expected = WAVELENGTH * 1e3 * (index(510.0, 220.0, WAVELENGTH).unwrap() - n0) / ng;
        assert_eq!(p[1], expected);
        assert_eq!(data.series[1].points[20][1], 0.0);
        // the largest corner is wide and thick, the smallest narrow and thin
        let wide = fig(&data, "Corner $w+");
        assert!(wide > 0.0);
        assert!(ng > n0 && ng < 5.0, "{ng}");
        assert!(ring(15.0, 220.0, 10.0, 5.0).is_err());
    }
}
