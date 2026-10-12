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

/// Compact human date for the note list: time today, "Mon D" this year, else "Mon D, YYYY".
pub fn format_note_date(epoch: i64) -> String {
    let days = epoch.div_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let (cy, cm, cd) = civil_from_days(now_epoch().div_euclid(86_400));
    if (y, m, d) == (cy, cm, cd) {
        let secs = epoch.rem_euclid(86_400);
        format!("{:02}:{:02}", secs / 3600, (secs % 3600) / 60)
    } else if y == cy {
        format!("{} {}", MONTHS[(m - 1) as usize], d)
    } else {
        format!("{} {}, {}", MONTHS[(m - 1) as usize], d, y)
    }
}
