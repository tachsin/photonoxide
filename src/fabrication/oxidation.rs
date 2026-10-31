//! Thermal oxidation of silicon by Deal and Grove's linear-parabolic law, with their rate
//! constants as they printed them: B. E. Deal, A. S. Grove, J. Appl. Phys. 36, 3770 (1965),
//! doi:10.1063/1.1713945.
//!
//! The oxide's thickness x₀ after a time t obeys x₀² + A x₀ = B (t + τ) (their Eq. 12), so
//! x₀ = (A/2)[√(1 + (t + τ)/(A²/4B)) − 1] (Eq. 13): linear in t while it is thin, B/A the linear
//! rate constant, and parabolic once it is thick, B the parabolic one. τ accounts for the oxide
//! there at t = 0.

/// The oxidizing ambient.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ambient {
    /// Wet oxygen: O₂ bubbled through water at 95 °C, a water partial pressure of about 640
    /// Torr (Table I).
    Wet,
    /// Dry oxygen at 760 Torr (Table II).
    Dry,
}

/// A row of Deal and Grove's Table I or II, as printed: the temperature (°C), A (µm), B
/// (µm²/h), B/A (µm/h) and τ (h). The 700 °C dry row prints only B/A and τ (an empty string).
pub type Row = (f64, &'static str, &'static str, &'static str, &'static str);

/// Table I, wet oxygen (PDF p. 5).
pub const WET: [Row; 4] = [
    (1200.0, "0.05", "0.720", "14.40", "0"),
    (1100.0, "0.11", "0.510", "4.64", "0"),
    (1000.0, "0.226", "0.287", "1.27", "0"),
    (920.0, "0.50", "0.203", "0.406", "0"),
];

/// Table II, dry oxygen (PDF p. 7).
pub const DRY: [Row; 6] = [
    (1200.0, "0.040", "0.045", "1.12", "0.027"),
    (1100.0, "0.090", "0.027", "0.30", "0.076"),
    (1000.0, "0.165", "0.0117", "0.071", "0.37"),
    (920.0, "0.235", "0.0049", "0.0208", "1.40"),
    (800.0, "0.370", "0.0011", "0.0030", "9.0"),
    (700.0, "", "", "0.00026", "81.0"),
];

/// A printed number's value, and half a unit of its last printed digit.
pub fn printed(text: &str) -> Option<(f64, f64)> {
    let value: f64 = text.parse().ok()?;
    let decimals = text.split_once('.').map_or(0, |(_, d)| d.len());
    Some((value, 0.5 * 10f64.powi(-(decimals as i32))))
}

/// The rate constants at `temperature` °C in `ambient`, a row of the tables: (A µm, B µm²/h, τ
/// h), `None` at a temperature the table doesn't hold or a row without A and B.
pub fn constants(ambient: Ambient, temperature: f64) -> Option<(f64, f64, f64)> {
    let rows: &[Row] = match ambient {
        Ambient::Wet => &WET,
        Ambient::Dry => &DRY,
    };
    let &(_, a, b, _, tau) = rows.iter().find(|r| r.0 == temperature)?;
    Some((printed(a)?.0, printed(b)?.0, printed(tau)?.0))
}

/// The oxide's thickness, µm, after `hours` with constants `a` (µm), `b` (µm²/h) and `tau` (h):
/// their Eq. 13.
pub fn thickness(a: f64, b: f64, tau: f64, hours: f64) -> f64 {
    let t = hours + tau;
    0.5 * a * ((1.0 + t / (a * a / (4.0 * b))).sqrt() - 1.0)
}

/// The time, h, to grow `x` µm of oxide: Eq. 12 solved for t.
pub fn hours(a: f64, b: f64, tau: f64, x: f64) -> f64 {
    (x * x + a * x) / b - tau
}

/// The activation energies of Deal and Grove's rate constants, kcal/mol, as printed with their
/// Arrhenius plots (Figs. 9 and 11, PDF p. 7): (B, B/A).
pub fn activation(ambient: Ambient) -> (f64, f64) {
    match ambient {
        Ambient::Dry => (28.5, 46.0),
        Ambient::Wet => (16.3, 45.3),
    }
}

/// The oxide Deal and Grove found already there at t = 0 in dry oxygen, µm: their data
/// extrapolate to 230 ± 30 Å from 700 to 1200 °C, and 230 Å was adopted (PDF p. 6). Wet
/// oxidation has none.
pub fn initial(ambient: Ambient) -> (f64, f64) {
    match ambient {
        Ambient::Dry => (0.023, 0.003),
        Ambient::Wet => (0.0, 0.0),
    }
}

/// The precision of the printed constants (PDF p. 5): B to ±2 %, A to ±2 % at low temperatures
/// and ±12 % at high; the larger is taken for A at every temperature.
pub const PRECISION: (f64, f64) = (0.12, 0.02);

/// The gas constant in kcal/(mol·K): 8.314462618 J/(mol·K) (exact in the SI) over 4184 J/kcal
/// (the thermochemical calorie).
const R_KCAL: f64 = 8.314_462_618 / 4184.0;

/// The rate constants at any temperature from 920 °C (wet) or 800 °C (dry) to 1200 °C.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rates {
    /// A, µm.
    pub a: f64,
    /// B, µm²/h.
    pub b: f64,
    /// τ, h: the time the initial oxide would have taken.
    pub tau: f64,
    /// The printed row's temperature they come from, °C.
    pub row: f64,
    /// Whether they are carried from that row by the Arrhenius law (`false`: printed).
    pub carried: bool,
}

/// Deal and Grove's rate constants at `temperature` °C in `ambient`: a printed row's, or, between
/// rows, the nearest row's B and B/A carried by the Arrhenius law with the printed activation
/// energies; τ then follows from the initial oxide (Eq. 12 at t = 0). `None` outside the rows
/// with A and B.
pub fn rates(ambient: Ambient, temperature: f64) -> Option<Rates> {
    let rows: &[Row] = match ambient {
        Ambient::Wet => &WET,
        Ambient::Dry => &DRY,
    };
    let with_ab: Vec<&Row> = rows.iter().filter(|r| !r.1.is_empty()).collect();
    let lo = with_ab.iter().map(|r| r.0).fold(f64::INFINITY, f64::min);
    let hi = with_ab
        .iter()
        .map(|r| r.0)
        .fold(f64::NEG_INFINITY, f64::max);
    if !(lo..=hi).contains(&temperature) {
        return None;
    }
    let row = with_ab.iter().min_by(|x, y| {
        (x.0 - temperature)
            .abs()
            .total_cmp(&(y.0 - temperature).abs())
    })?;
    let (a0, b0, tau0) = (printed(row.1)?.0, printed(row.2)?.0, printed(row.4)?.0);
    if row.0 == temperature {
        return Some(Rates {
            a: a0,
            b: b0,
            tau: tau0,
            row: row.0,
            carried: false,
        });
    }
    let (eb, eba) = activation(ambient);
    let kelvin = |c: f64| c + 273.15;
    let factor = |e: f64| (-e / R_KCAL * (1.0 / kelvin(temperature) - 1.0 / kelvin(row.0))).exp();
    let b = b0 * factor(eb);
    let ba = b0 / a0 * factor(eba);
    let a = b / ba;
    let (xi, _) = initial(ambient);
    Some(Rates {
        a,
        b,
        tau: (xi * xi + a * xi) / b,
        row: row.0,
        carried: true,
    })
}

/// The oxide grown on bare silicon, or on `start` µm of oxide, after `hours` at `temperature` °C:
/// its thickness, µm, and a bound on it from the constants' precision ([`PRECISION`]: every
/// combination of A and B at the ends of their precision, the largest change).
pub fn grow(
    ambient: Ambient,
    temperature: f64,
    hours: f64,
    start: Option<f64>,
) -> Option<(f64, f64)> {
    let r = rates(ambient, temperature)?;
    let tau = |a: f64, b: f64| match start {
        Some(x) => (x * x + a * x) / b,
        None if r.carried || ambient == Ambient::Wet => {
            let (xi, _) = initial(ambient);
            (xi * xi + a * xi) / b
        }
        None => r.tau,
    };
    let x = thickness(r.a, r.b, tau(r.a, r.b), hours);
    let (pa, pb) = PRECISION;
    let mut bound: f64 = 0.0;
    for sa in [-1.0, 1.0] {
        for sb in [-1.0, 1.0] {
            let (a, b) = (r.a * (1.0 + sa * pa), r.b * (1.0 + sb * pb));
            bound = bound.max((thickness(a, b, tau(a, b), hours) - x).abs());
        }
    }
    Some((x, bound))
}

/// For each row of both tables with A and B: how far the printed B/A is from B over A, in units
/// of what the printing's rounding allows, half a unit of the last printed digit of each of the
/// three carried through the ratio. At most 1 for every row if the tables hold together.
pub fn ratio_consistency() -> Vec<(Ambient, f64, f64)> {
    let mut out = Vec::new();
    for (ambient, rows) in [(Ambient::Wet, &WET[..]), (Ambient::Dry, &DRY[..])] {
        for &(t, a, b, ratio, _) in rows {
            let (Some((a, da)), Some((b, db)), Some((r, dr))) =
                (printed(a), printed(b), printed(ratio))
            else {
                continue;
            };
            let computed = b / a;
            let allowed = computed * (db / b + da / a) + dr;
            out.push((ambient, t, (computed - r).abs() / allowed));
        }
    }
    out
}
