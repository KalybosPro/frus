//! Today's date, from the system clock, with no date library.

use crate::models::profile::Date;

/// Today, in UTC — or `None` where the standard library has no clock to read (the Web:
/// `SystemTime::now()` is not available on `wasm32`, and a real application would ask the
/// browser for the date). The rules that need today skip themselves when it is unknown.
pub fn today() -> Option<Date> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let since_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        let days = i64::try_from(since_epoch.as_secs() / 86_400).ok()?;
        Some(civil_from_days(days))
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

/// The calendar date `days` days after 1970-01-01 (Howard Hinnant's `civil_from_days`).
pub fn civil_from_days(days: i64) -> Date {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    Date::new(year, month, day)
}
