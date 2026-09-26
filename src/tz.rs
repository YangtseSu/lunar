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
            TimeZone::from_posix_tz(&value).map_err(|_| CalError::BadTimeZone { source: "TZ" })
        }
        _ => TimeZone::local().map_err(|_| CalError::BadTimeZone {
            source: "系统时区"
        }),
    }
}

/// Today in the local time zone.
///
/// Each failure gets its own error: a zone `tz-rs` cannot read, a clock
/// before 1970, and a local year outside 1–9999 are different problems, and
/// reporting the last of them for the first two sent the reader looking at
/// the calendar instead of at `TZ`.
pub fn today() -> Result<CivilDate, CalError> {
    let zone = local_zone()?;
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| CalError::ClockBeforeEpoch)?
        .as_secs() as i64;
    let local = DateTime::from_timespec(seconds, 0, zone.as_ref())
        .map_err(|_| CalError::BadTimeZone { source: "TZ" })?;
    let (year, month, day) = (
        local.year(),
        i32::from(local.month()),
        i32::from(local.month_day()),
    );
    crate::calendar::check_year(year)?;
    Ok(CivilDate::new(year, month, day))
}
