//! Civil (proleptic Gregorian) date arithmetic for grid construction and
//! `-d` parsing.
//!
//! Weekday, day-of-year and month-length questions are answered by `lunar-rs`.
//! What lives here is the epoch ↔ civil-date bridge, which `lunar-rs` cannot
//! provide because it deliberately models the 1582 Gregorian reform (its
//! `Solar` rejects 1582-10-05..=1582-10-14), plus calendar stepping for the
//! `-d` parser.

use lunar_rs::solar_util;
use lunar_rs::Solar;

use crate::calendar::{self, CalError};

/// Seconds in a day.
pub const SECS_PER_DAY: i64 = 86_400;

/// Days from 1970-01-01 to the civil date `y-m-d` (Hinnant's `days_from_civil`).
///
/// Pure calendar arithmetic: no time zone, no leap seconds.
pub fn days_from_civil(y: i32, m: i32, d: i32) -> i64 {
    // Shift the year so that March is the first month; the leap day then falls
    // at the end of the 4-year era and the month lengths become uniform.
    let y = i64::from(y) - i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let m = i64::from(m);
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(d) - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Inverse of [`days_from_civil`]: the civil date for a day number relative to
/// the Unix epoch.
pub fn civil_from_days(z: i64) -> (i32, i32, i32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = mp + if mp < 10 { 3 } else { -9 }; // [1, 12]
    ((y + i64::from(m <= 2)) as i32, m as i32, d as i32)
}

/// A civil date with no time-of-day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CivilDate {
    /// Proleptic Gregorian year.
    pub year: i32,
    /// Month, `1..=12`.
    pub month: i32,
    /// Day of month, `1..=31`.
    pub day: i32,
}

impl CivilDate {
    /// Builds a date without validating it.
    pub const fn new(year: i32, month: i32, day: i32) -> Self {
        Self { year, month, day }
    }

    /// The civil date `days` after 1970-01-01.
    pub fn from_epoch_day(days: i64) -> Self {
        let (year, month, day) = civil_from_days(days);
        Self { year, month, day }
    }

    /// Day number relative to 1970-01-01.
    pub fn epoch_day(self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    /// Number of days in this date's month, per `lunar-rs`.
    pub fn days_in_month(self) -> i32 {
        solar_util::days_of_month(self.year, self.month)
    }

    /// Weekday, `0 = Sunday`, per `lunar-rs`.
    pub fn weekday(self) -> i32 {
        solar_util::week(self.year, self.month, self.day)
    }

    /// This date plus `delta` days.
    pub fn add_days(self, delta: i64) -> Self {
        Self::from_epoch_day(self.epoch_day() + delta)
    }

    /// This date plus `delta` months, clamping the day to the target month
    /// (2024-01-31 + 1 month → 2024-02-29).
    pub fn add_months(self, delta: i32) -> Self {
        let total = i64::from(self.year) * 12 + i64::from(self.month - 1) + i64::from(delta);
        let year = (total.div_euclid(12)) as i32;
        let month = (total.rem_euclid(12) as i32) + 1;
        Self { year, month, day: self.day.min(solar_util::days_of_month(year, month)) }
    }

    /// This date plus `delta` years, clamping 2/29 to 2/28 in common years.
    pub fn add_years(self, delta: i32) -> Self {
        let year = self.year + delta;
        Self { year, month: self.month, day: self.day.min(solar_util::days_of_month(year, self.month)) }
    }

    /// Converts to `lunar-rs`' [`Solar`], rejecting out-of-range years,
    /// impossible days and the 1582 Gregorian reform gap.
    pub fn to_solar(self) -> Result<Solar, CalError> {
        calendar::solar(self.year, self.month, self.day)
    }
}
