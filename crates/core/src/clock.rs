//! Minimal UTC clock.
//!
//! Daftar only needs ISO-8601 UTC timestamps for audit rows and record
//! metadata in this phase. Pulling a full timezone database is unnecessary;
//! the conversion below is the well-known civil-from-days algorithm and is
//! covered by tests.

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current UTC time as `YYYY-MM-DDTHH:MM:SSZ`.
#[must_use]
pub fn now_iso8601() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => iso8601_from_unix(i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)),
        Err(_) => iso8601_from_unix(0),
    }
}

/// Converts Unix seconds to `YYYY-MM-DDTHH:MM:SSZ`.
#[must_use]
pub fn iso8601_from_unix(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days` for days since 1970-01-01.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    let year = if month <= 2 { year + 1 } else { year };
    (
        year,
        u32::try_from(month).unwrap_or(1),
        u32::try_from(day).unwrap_or(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_epoch() {
        assert_eq!(iso8601_from_unix(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn converts_known_dates() {
        assert_eq!(iso8601_from_unix(1_767_225_600), "2026-01-01T00:00:00Z");
        assert_eq!(iso8601_from_unix(1_769_990_399), "2026-02-01T23:59:59Z");
        assert_eq!(iso8601_from_unix(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn handles_pre_epoch() {
        assert_eq!(iso8601_from_unix(-86_400), "1969-12-31T00:00:00Z");
    }
}
