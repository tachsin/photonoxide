//! Versions compared by their numbers, and days since 1970 as dates: what the libraries'
//! table and the benchmarks' records compare and date by.

use std::cmp::Ordering;

/// A version's numbers, up to the first part that isn't one: `v0.3.34` is 0, 3, 34;
/// `2020.4.304-2ubuntu3` 2020, 4, 304.
pub fn numbers(version: &str) -> Vec<u64> {
    let v = version.trim().trim_start_matches(['v', 'V']);
    let v = v.split(['-', '+', ' ', '_']).next().unwrap_or("");
    v.split('.').map_while(|p| p.parse().ok()).collect()
}

/// Two versions by their numbers, a missing number as 0: `26` is `26.0`.
pub fn compare(a: &str, b: &str) -> Ordering {
    let (a, b) = (numbers(a), numbers(b));
    for i in 0..a.len().max(b.len()) {
        let o = a.get(i).unwrap_or(&0).cmp(b.get(i).unwrap_or(&0));
        if o != Ordering::Equal {
            return o;
        }
    }
    Ordering::Equal
}

/// Days since 1970 as a day of the Gregorian calendar (Hinnant's `civil_from_days`).
pub fn date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_their_numbers() {
        assert_eq!(numbers("v0.3.34"), [0, 3, 34]);
        assert_eq!(numbers("2020.4.304-2ubuntu3"), [2020, 4, 304]);
        assert_eq!(numbers("0.8.0.10"), [0, 8, 0, 10]);
        assert_eq!(numbers("7.0.0rc1"), [7, 0]);
        assert_eq!(compare("5.10.0", "5.9.1"), Ordering::Greater);
        assert_eq!(compare("26", "26.0"), Ordering::Equal);
        assert_eq!(compare("27.0.1", "26"), Ordering::Greater);
        assert_eq!(compare("13.4.92", "13.4.92"), Ordering::Equal);
        assert_eq!(compare("12.8.6.49", "12.8.6.72"), Ordering::Less);
    }

    #[test]
    fn days_are_dates() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(11_016), "2000-02-29");
        assert_eq!(date(20_736), "2026-10-10");
    }
}
