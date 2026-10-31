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
