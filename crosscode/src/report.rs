//! What the cross-code reports print: errors in percent and times, the same in each.

/// `value`'s error against `reference` in percent, signed, with enough decimals for two
/// significant digits (and at least two): `+0.00090 %`, `−0.20 %`, `+5.57 %`.
pub fn percent(value: f64, reference: f64) -> String {
    let p = 100.0 * (value - reference) / reference;
    if p == 0.0 {
        return "0.00 %".to_owned();
    }
    let decimals = (1.0 - p.abs().log10().floor()).clamp(2.0, 8.0) as usize;
    let text = format!("{:.decimals$}", p.abs());
    format!("{}{text} %", if p < 0.0 { '−' } else { '+' })
}

/// The computer a report was run on, for its times: `PHOTONOXIDE_MACHINE` where it names one
/// (`Intel Core Ultra 7 265K, 20 threads, 64 GB, Windows 11`), else its threads and system.
pub fn machine() -> String {
    std::env::var("PHOTONOXIDE_MACHINE")
        .ok()
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| {
            let threads = std::thread::available_parallelism().map_or(0, usize::from);
            format!(
                "a computer with {threads} threads ({})",
                std::env::consts::OS
            )
        })
}

/// A time: `0.31 s`, `14 s`, `6 min 32 s`, `1 h 5 min`.
pub fn duration(seconds: f64) -> String {
    if seconds < 10.0 {
        format!("{seconds:.2} s")
    } else if seconds < 60.0 {
        format!("{seconds:.0} s")
    } else if seconds < 3600.0 {
        let s = seconds.round() as u64;
        format!("{} min {} s", s / 60, s % 60)
    } else {
        let m = (seconds / 60.0).round() as u64;
        format!("{} h {} min", m / 60, m % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_show_two_significant_digits_in_percent() {
        assert_eq!(percent(54.039, 54.148), "−0.20 %");
        assert_eq!(percent(2.217176, 2.217156), "+0.00090 %");
        assert_eq!(percent(57.1665, 54.148), "+5.57 %");
        assert_eq!(percent(1.0, 1.0), "0.00 %");
    }

    #[test]
    fn times_read_in_their_own_units() {
        assert_eq!(duration(0.314), "0.31 s");
        assert_eq!(duration(14.2), "14 s");
        assert_eq!(duration(2408.0), "40 min 8 s");
        assert_eq!(duration(3900.0), "1 h 5 min");
    }
}
