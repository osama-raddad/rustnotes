use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Howard Hinnant's civil_from_days: days since 1970-01-01 -> (year, month, day).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Compact human date for the note list: time today, "Mon D" this year, else
/// "Mon D, YYYY". `utc_offset_secs` is the local zone's offset east of UTC;
/// the epoch is shifted into local time before the day split, so both the
/// HH:MM and the today/date boundary are local (MED-2). Core is pure egui
/// and cannot read the zone itself — the host pushes it in via
/// [`crate::App::set_utc_offset_secs`].
pub fn format_note_date(epoch: i64, utc_offset_secs: i32) -> String {
    format_note_date_at(epoch, now_epoch(), utc_offset_secs)
}

/// Same, with an explicit "now" so the today/«Mon D» boundary around local
/// midnight is unit-testable without sleeping.
fn format_note_date_at(epoch: i64, now: i64, utc_offset_secs: i32) -> String {
    let local = epoch + i64::from(utc_offset_secs);
    let days = local.div_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let (cy, cm, cd) = civil_from_days((now + i64::from(utc_offset_secs)).div_euclid(86_400));
    if (y, m, d) == (cy, cm, cd) {
        let secs = local.rem_euclid(86_400);
        format!("{:02}:{:02}", secs / 3600, (secs % 3600) / 60)
    } else if y == cy {
        format!("{} {}", MONTHS[(m - 1) as usize], d)
    } else {
        format!("{} {}, {}", MONTHS[(m - 1) as usize], d, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400;

    #[test]
    fn zero_offset_matches_utc() {
        let now = 10_000 * DAY + 3661; // 01:01:01 UTC
        assert_eq!(format_note_date_at(now, now, 0), "01:01");
    }

    #[test]
    fn positive_offset_shifts_time_and_the_midnight_boundary() {
        let now = 10_000 * DAY; // 00:00 UTC; +1 h zone: local "now" is 01:00
                                // 23:59:59 UTC of the previous day is 00:59:59 local *today*.
        assert_eq!(format_note_date_at(now - 1, now, 3600), "00:59");
        // 01:00 UTC is 02:00 local, still today.
        assert_eq!(format_note_date_at(now + 3600, now, 3600), "02:00");
    }

    #[test]
    fn negative_offset_crosses_midnight_backwards() {
        let now = 10_000 * DAY; // 00:00 UTC; -1 h zone: local "now" is 23:00 yesterday
        assert_eq!(format_note_date_at(now, now, -3600), "23:00");
        // 01:00 UTC is local midnight of the *next* day → a date, not a time.
        // (Day 10_000 = 1997-05-19.)
        assert_eq!(format_note_date_at(now + 3600, now, -3600), "May 19");
    }
}
