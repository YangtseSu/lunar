//! Civil (proleptic Gregorian) date arithmetic for grid construction and
//! `-d` parsing.
//!
//! Weekday, day-of-year and month-length questions are answered by `lunar-rs`.
//! What lives here is the epoch ↔ civil-date bridge, which `lunar-rs` cannot
//! provide because it deliberately models the 1582 Gregorian reform (its
//! `Solar` rejects 1582-10-05..=1582-10-14), plus calendar stepping for the
//! `-d` parser.

use lunar_rs::Solar;
use lunar_rs::solar_util;

use crate::calendar::{self, CalError};

/// Seconds in a day.
pub const SECS_PER_DAY: i64 = 86_400;

/// Proleptic-Gregorian length of a month, leap years included.
///
/// The engine's `days_of_month` reports the days that *exist*, which for
/// October 1582 is 21 rather than 31. Walking a month needs the proleptic
/// length to know where to stop.
fn proleptic_month_days(year: i32, month: i32) -> i32 {
    const LENGTHS: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    match month {
        2 if solar_util::is_leap_year(year) => 29,
        1..=12 => LENGTHS[month as usize - 1],
        _ => 0,
    }
}

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
///
/// The year is computed in `i64` and named when it outgrows the `i32` it is
/// stored in: a day count the user typed (`+2147483647 fortnights`, the
/// twenty-seventh of which carries the year out of range) reaches a year past
/// the representable one, and a wrapped year is a year nobody asked for.
pub fn civil_from_days(z: i64) -> Result<(i32, i32, i32), CalError> {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = mp + if mp < 10 { 3 } else { -9 }; // [1, 12]
    let year = y + i64::from(m <= 2);
    Ok((
        i32::try_from(year).map_err(|_| out_of_range(year))?,
        m as i32,
        d as i32,
    ))
}

/// The error for a year the arithmetic reached but `i32` cannot hold.
fn out_of_range(year: i64) -> CalError {
    CalError::YearOutOfRange {
        year,
        min: calendar::MIN_YEAR,
        max: calendar::MAX_YEAR,
    }
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
    pub fn from_epoch_day(days: i64) -> Result<Self, CalError> {
        let (year, month, day) = civil_from_days(days)?;
        Ok(Self { year, month, day })
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

    /// Whether this date exists in the calendar `lunar-rs` serves.
    ///
    /// Proleptic-Gregorian arithmetic says every date does; the engine models
    /// the 1582 reform and refuses 1582-10-05..=14. A grid asking for a day has
    /// to know which they are.
    pub fn exists(self) -> bool {
        !(self.year == 1582 && self.month == 10 && (5..=14).contains(&self.day))
    }

    /// The `index`-th day (0-based) of this date's month that **exists**.
    ///
    /// October 1582 has 31 days on a proleptic calendar and 21 on the
    /// engine's, and `days_of_month` reports the *existing* 21 — so the walk
    /// covers the proleptic length and counts the days that survive. Stepping
    /// the month by `add_days` cannot work here: the ten reform days are
    /// counted by the arithmetic and have no cell to draw.
    pub fn nth_existing_day(self, index: usize) -> Option<Self> {
        let mut seen = 0usize;
        for day in 1..=proleptic_month_days(self.year, self.month) {
            let candidate = Self::new(self.year, self.month, day);
            match candidate.exists() {
                false => continue,
                true => {
                    if seen == index {
                        return Some(candidate);
                    }
                    seen += 1;
                }
            }
        }
        None
    }

    /// This date plus `delta` days.
    ///
    /// The day count is added in `i64` and the year it reaches is named when
    /// that year outgrows an `i32`; see [`civil_from_days`].
    pub fn add_days(self, delta: i64) -> Result<Self, CalError> {
        Self::from_epoch_day(self.epoch_day() + delta)
    }

    /// This date plus `delta` months, clamping the day to the target month
    /// (2024-01-31 + 1 month → 2024-02-29).
    ///
    /// The month index is a number the user typed, so it is taken in `i64` and
    /// the year it reaches is named when that year outgrows an `i32`: thirteen
    /// `+2147483647 months` reach a year past the representable one, and a
    /// truncated `as i32` would report a year nobody asked for.
    pub fn add_months(self, delta: i32) -> Result<Self, CalError> {
        let total = i64::from(self.year) * 12 + i64::from(self.month - 1) + i64::from(delta);
        let year = total.div_euclid(12);
        let year = i32::try_from(year).map_err(|_| out_of_range(year))?;
        let month = (total.rem_euclid(12) as i32) + 1;
        Ok(Self {
            year,
            month,
            day: self.day.min(solar_util::days_of_month(year, month)),
        })
    }

    /// This date plus `delta` years, clamping 2/29 to 2/28 in common years.
    ///
    /// The sum is taken in `i64` and the year it reaches is named when that
    /// year outgrows an `i32`. `delta` is a number the user typed, and
    /// `+2147483600 years` from 2026 lands past the representable one; a
    /// wrapped year would be a year nobody asked for, so it is reported
    /// instead. Whether the year is *servable* is the caller's check — this
    /// only answers whether it exists.
    pub fn add_years(self, delta: i32) -> Result<Self, CalError> {
        let year = i64::from(self.year) + i64::from(delta);
        let year = i32::try_from(year).map_err(|_| out_of_range(year))?;

        Ok(Self {
            year,
            month: self.month,
            day: self.day.min(solar_util::days_of_month(year, self.month)),
        })
    }

    /// Converts to `lunar-rs`' [`Solar`], rejecting out-of-range years,
    /// impossible days and the 1582 Gregorian reform gap.
    pub fn to_solar(self) -> Result<Solar, CalError> {
        calendar::solar(self.year, self.month, self.day)
    }
}
