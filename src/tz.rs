//! Local civil date resolution.
//!
//! A calendar CLI only ever needs "what day is it here", so this is the whole
//! time zone surface: the `TZ` environment variable, IANA zone names and the
//! system zone are resolved by `tz-rs`, which also handles the POSIX `TZ`
//! syntax.

use std::time::{SystemTime, UNIX_EPOCH};

use tz::{DateTime, TimeZone};

use crate::calendar::CalError;
use crate::civil::CivilDate;

/// Loads the time zone named by `TZ`, or the system zone when `TZ` is unset.
fn local_zone() -> Result<TimeZone, CalError> {
    match std::env::var("TZ") {
        Ok(value) if !value.is_empty() => {
            TimeZone::from_posix_tz(&value).map_err(|_| CalError::TodayOutOfRange)
        }
        _ => TimeZone::local().map_err(|_| CalError::TodayOutOfRange),
    }
}

/// Today in the local time zone.
///
/// `Unix time` arithmetic is proleptic Gregorian all the way down, so a clock
/// this far outside 1–9999 can only mean a broken environment.
pub fn today() -> Result<CivilDate, CalError> {
    let zone = local_zone()?;
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| CalError::TodayOutOfRange)?
        .as_secs() as i64;
    let local = DateTime::from_timespec(seconds, 0, zone.as_ref())
        .map_err(|_| CalError::TodayOutOfRange)?;
    let (year, month, day) = (
        local.year(),
        i32::from(local.month()),
        i32::from(local.month_day()),
    );
    crate::calendar::check_year(year)?;
    Ok(CivilDate::new(year, month, day))
}
