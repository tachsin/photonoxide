//! The charts of the lesson on how a chip is made: thermal oxide grown on silicon by Deal and
//! Grove's law with their printed constants (`fabrication::oxidation`), and the smallest line each
//! projection lithography of the process library prints by the Rayleigh criterion, with its cited
//! wavelength and numerical apertures (`fabrication::rayleigh`). Neither has a grid: each curve is
//! a closed form evaluated.

use photonoxide::fabrication::oxidation::{self, Ambient};
use photonoxide::fabrication::rayleigh::{Optics, Reach};
use photonoxide::fabrication::{library, rules};

use super::{ChartData, Curve, figure, invalid, sig};

/// `n` points from `a` to `b`, both included.
fn span(a: f64, b: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|k| a + (b - a) * k as f64 / (n - 1) as f64)
        .collect()
}

fn curve(label: &str, points: Vec<[f64; 2]>, dashed: bool, colour: usize) -> Curve {
    Curve {
        label: label.to_owned(),
        points,
        dashed,
        colour: Some(colour),
    }
}

/// The oxide grown on bare silicon in dry and in wet oxygen at `temperature` °C, against the time
/// up to `hours`; each with its linear and parabolic limits, (B/A)(t + τ) and √(B(t + τ)).
pub fn oxidation(temperature: f64, hours: f64) -> photonoxide::Result<ChartData> {
    let ts = span(0.0, hours, 241);
    let mut series = Vec::new();
    let mut figures = Vec::new();
    for (k, (ambient, name)) in [(Ambient::Dry, "dry O₂"), (Ambient::Wet, "wet O₂")]
        .into_iter()
        .enumerate()
    {
        let r = oxidation::rates(ambient, temperature).ok_or_else(|| {
            invalid(
                "temperature",
                format!("{temperature} °C is outside Deal and Grove's tables"),
            )
        })?;
        let grown = |t: f64| oxidation::grow(ambient, temperature, t, None).map(|g| g.0);
        let mut points = Vec::with_capacity(ts.len());
        for &t in &ts {
            let x = grown(t).ok_or_else(|| invalid("hours", "no growth"))?;
            points.push([t, x]);
        }
        // the τ the curve starts from, as `grow` takes it: the printed one on a printed dry row,
        // else the initial oxide's (230 Å dry, none wet)
        let (xi, _) = oxidation::initial(ambient);
        let tau = if r.carried || ambient == Ambient::Wet {
            (xi * xi + r.a * xi) / r.b
        } else {
            r.tau
        };
        let linear: Vec<[f64; 2]> = ts.iter().map(|&t| [t, r.b / r.a * (t + tau)]).collect();
        let parabolic: Vec<[f64; 2]> = ts.iter().map(|&t| [t, (r.b * (t + tau)).sqrt()]).collect();
        series.push(curve(name, points, false, k));
        series.push(curve(&format!("{name}: (B/A)(t + τ)"), linear, true, k));
        series.push(curve(&format!("{name}: √(B(t + τ))"), parabolic, true, k));
        let (x, bound) = oxidation::grow(ambient, temperature, hours, None)
            .ok_or_else(|| invalid("hours", "no growth"))?;
        let row = if r.carried {
            format!("carried from the {} °C row by the Arrhenius law", r.row)
        } else {
            format!("the printed {} °C row", r.row)
        };
        figures.push(figure(
            &format!("{name}: $x_0$ after {} h", sig(hours, 3)),
            x,
            4,
            "µm",
            &format!(
                "± {} µm from the constants' printed precision",
                sig(bound, 2)
            ),
        ));
        figures.push(figure(
            &format!("{name}: $B/A$"),
            r.b / r.a,
            3,
            "µm/h",
            &format!("the linear rate constant; {row}"),
        ));
        figures.push(figure(
            &format!("{name}: $B$"),
            r.b,
            3,
            "µm²/h",
            "the parabolic rate constant",
        ));
        figures.push(figure(
            &format!("{name}: $A^2/4B$"),
            r.a * r.a / (4.0 * r.b),
            3,
            "h",
            "where the growth turns from linear to parabolic",
        ));
    }
    let top = series
        .iter()
        .filter(|c| !c.dashed)
        .flat_map(|c| c.points.iter().map(|p| p[1]))
        .fold(0.0, f64::max);
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "time t (h)".to_owned(),
        x_length: false,
        y_label: "oxide thickness x₀ (µm)".to_owned(),
        y_log: false,
        y_range: Some([0.0, 1.15 * top]),
        series,
        marker: None,
        figures,
        note: format!(
            "Exact, no grid: x₀² + A x₀ = B (t + τ) (Deal and Grove's Eq. 12) solved for x₀, A and B \
             from their Tables I (wet, 640 Torr of water) and II (dry, 760 Torr), dry oxidation \
             starting from the 230 Å their data extrapolate to, wet from bare silicon, at {} °C. \
             Dashed: the linear and parabolic limits.",
            sig(temperature, 4)
        ),
    })
}

/// The smallest line each projection lithography of the process library prints, W = k₁λ/NA,
/// over its cited numerical apertures, at `k1`; and the k₁ a feature `feature_nm` wide asks of
/// each at its highest NA.
pub fn resolution(k1: f64, feature_nm: f64) -> photonoxide::Result<ChartData> {
    let optics: Vec<(String, Optics)> = library()
        .iter()
        .filter_map(|p| Optics::of(p).map(|o| (p.name.clone(), o)))
        .collect();
    if optics.is_empty() {
        return Err(invalid(
            "lithography",
            "the library has no projection lithography",
        ));
    }
    let mut series = Vec::new();
    let mut figures = Vec::new();
    let (lo, hi) = optics
        .iter()
        .fold((f64::INFINITY, 0.0_f64), |(a, b), (_, o)| {
            (a.min(o.na[0]), b.max(o.na[1]))
        });
    for (k, (name, o)) in optics.iter().enumerate() {
        let points: Vec<[f64; 2]> = span(o.na[0], o.na[1], 60)
            .into_iter()
            .map(|na| [na, k1 * o.wavelength_nm / na])
            .collect();
        series.push(curve(
            &format!("{name} ({} nm)", sig(o.wavelength_nm, 4)),
            points,
            false,
            k,
        ));
        let asked = o.k1(feature_nm);
        let reach = match o.reach(feature_nm) {
            Reach::Typical => "prints as a matter of course",
            Reach::Enhanced => "needs resolution enhancement",
            _ => "beyond its limit",
        };
        figures.push(figure(
            &format!("{name}: $W$ at NA {}", sig(o.na[1], 3)),
            k1 * o.wavelength_nm / o.na[1],
            3,
            "nm",
            &format!(
                "the {} nm feature asks k₁ = {}: {reach}",
                sig(feature_nm, 3),
                sig(asked, 2)
            ),
        ));
    }
    series.push(Curve {
        label: format!("the feature, {} nm", sig(feature_nm, 3)),
        points: vec![[lo, feature_nm], [hi, feature_nm]],
        dashed: true,
        colour: Some(7),
    });
    let typical = rules::k1_typical();
    let limit = rules::k1_limit();
    Ok(ChartData {
        maps: Vec::new(),
        x_label: "numerical aperture NA".to_owned(),
        x_length: false,
        y_label: "smallest line W (nm)".to_owned(),
        y_log: true,
        y_range: None,
        series,
        marker: None,
        figures,
        note: format!(
            "Exact, no grid: W = k₁λ/NA (Campbell, Eq. 7.18) with k₁ = {}, each lithography over \
             the numerical apertures its sources cite. k₁ is typically {} down to {}, its \
             theoretical limit {} (Campbell, p. 183).",
            sig(k1, 3),
            sig(typical.high().unwrap_or(0.8), 2),
            sig(typical.low().unwrap_or(0.4), 2),
            sig(limit.low().unwrap_or(0.25), 2),
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

    /// The curves are `oxidation::grow` at every time, and at a printed row the constants are
    /// the printed ones: wet oxygen at 1000 °C, A = 0.226 µm and B = 0.287 µm²/h (Table I).
    #[test]
    fn the_oxide_is_deal_and_groves() {
        let data = oxidation(1000.0, 4.0).unwrap();
        for (name, ambient) in [("dry O₂", Ambient::Dry), ("wet O₂", Ambient::Wet)] {
            let c = data.series.iter().find(|c| c.label == name).unwrap();
            for p in c.points.iter().step_by(17) {
                let (x, _) = oxidation::grow(ambient, 1000.0, p[0], None).unwrap();
                assert_eq!(p[1], x);
            }
            // the grown thickness lies under both limits, and meets the linear one at t = 0
            let lin = &data
                .series
                .iter()
                .find(|c| c.label.starts_with(name) && c.label.contains("B/A"))
                .unwrap()
                .points;
            let par = &data
                .series
                .iter()
                .find(|c| c.label.starts_with(name) && c.label.contains('√'))
                .unwrap()
                .points;
            for ((p, l), q) in c.points.iter().zip(lin).zip(par).skip(1) {
                assert!(
                    p[1] <= l[1] + 1e-12 && p[1] <= q[1] + 1e-12,
                    "{name} at {} h",
                    p[0]
                );
            }
        }
        assert!((fig(&data, "wet O₂: $B$") - 0.287).abs() < 1e-12);
        assert!((fig(&data, "wet O₂: $B/A$") - 0.287 / 0.226).abs() < 1e-12);
        let (x, _) = oxidation::grow(Ambient::Wet, 1000.0, 4.0, None).unwrap();
        assert_eq!(fig(&data, "wet O₂: $x_0$"), x);
        // wet oxidation outgrows dry at every temperature of the tables
        for t in [920.0, 1050.0, 1200.0] {
            let d = oxidation(t, 2.0).unwrap();
            assert!(
                fig(&d, "wet O₂: $x_0$") > fig(&d, "dry O₂: $x_0$"),
                "{t} °C"
            );
        }
        assert!(oxidation(700.0, 1.0).is_err());
    }

    /// Each curve is k₁λ/NA over the lithography's cited NAs; the 500 nm strip at 193 nm and NA
    /// 0.93 asks k₁ = 2.41, as the method's write-up says (2.4).
    #[test]
    fn the_resolution_is_rayleighs() {
        let data = resolution(0.4, 500.0).unwrap();
        let mut seen = 0;
        for p in library() {
            let Some(o) = Optics::of(p) else { continue };
            seen += 1;
            let c = data
                .series
                .iter()
                .find(|c| c.label.starts_with(&p.name))
                .unwrap_or_else(|| panic!("no curve for {}", p.name));
            assert_eq!(c.points.first().unwrap()[0], o.na[0]);
            assert!((c.points.last().unwrap()[0] - o.na[1]).abs() < 1e-12);
            for q in &c.points {
                assert!((q[1] - 0.4 * o.wavelength_nm / q[0]).abs() < 1e-9);
            }
            assert!((fig(&data, &format!("{}: $W$", p.name)) - o.typical_nm).abs() < 1e-9);
        }
        assert!(seen >= 4, "{seen} projection lithographies");
        let deep = data
            .figures
            .iter()
            .find(|f| f.note.contains("k₁ = 2.4"))
            .expect("the strip at 193 nm asks k₁ = 2.4");
        assert!(deep.label.contains("0.93"), "{}", deep.label);
    }
}
