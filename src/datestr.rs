//! Parser for `lunar date -d` strings, in `date(1)` style.
//!
//! Accepted shapes:
//!
//! * keywords: `now`, `today`, `tomorrow`, `yesterday`
//! * epoch seconds: `@1758240000`, `@-1`, `@1758240000.25`
//! * ISO 8601: `2026-09-07`, `2026-09-07T15:30`, `20260907T1530`,
//!   `2026-09-07T15:30:45.123456789+08:00`, `...Z`
//! * slashes: `2026/09/07`, `09/07/2026` (month/day/year)
//! * relative: `+3 days`, `-2 weeks`, `2 days ago`, `1 fortnight`, `90 minutes`
//! * weekday names: `monday`, `next friday`, `last friday`
//! * **lunar** (with `date -l`): `2026-07-15`, `20260715`, `2026/7/15`
//!
//! Everything is anchored to a reference day (today by default), exactly like
//! `date -d`. Only the calendar day is kept from the reference; the CLI is a
//! calendar tool and has no time-of-day display.
//!
//! **A lunar date and a civil date share one grammar.** The three absolute
//! shapes are parsed once, into a plain year / month / day triple, and the
//! calendar the triple is *resolved against* is the only difference: with
//! `-l` the triple is a 农历 date and goes through
//! [`crate::calendar::solar_from_lunar`], without it a 公历 date and goes
//! through [`crate::calendar::solar`]. That is what makes `-d 2026-07-15` and
//! `-l -d 2026-07-15` behave the same way, differing only in the answer.
//!
//! Only the absolute shapes have a lunar reading. A keyword, an `@epoch`, a
//! weekday and a relative offset are all statements about *days*, not about a
//! date written in one calendar or the other, so they resolve against the
//! reference exactly as they do without `-l`.

use crate::calendar::{self, CalError, MAX_YEAR, MIN_YEAR};
use crate::civil::{self, CivilDate};

/// Weekday names accepted in `-d` strings.
const WEEKDAYS: [(&str, i32); 21] = [
    ("sunday", 0),
    ("sun", 0),
    ("monday", 1),
    ("mon", 1),
    ("tuesday", 2),
    ("tue", 2),
    ("tues", 2),
    ("wednesday", 3),
    ("wed", 3),
    ("thursday", 4),
    ("thu", 4),
    ("thur", 4),
    ("thurs", 4),
    ("friday", 5),
    ("fri", 5),
    ("saturday", 6),
    ("sat", 6),
    ("星期日", 0),
    ("星期天", 0),
    ("礼拜天", 0),
    ("周一", 1),
];

/// Malformed-input error, carrying the offending text.
fn invalid(input: &str) -> CalError {
    CalError::UnparsableDate {
        input: input.to_string(),
    }
}

/// The ASCII digits in `text[at..until]`, or `None` if that span is not
/// entirely them.
///
/// Every absolute form is a fixed-width date, and the grammar is matched on
/// **bytes** — `bytes[4]` is the first separator, and so on. Slicing a `&str`
/// at one of those offsets panics when a multi-byte character straddles it, so
/// `2026-09-中` was a crash rather than a bad date. A byte range that is not
/// all digits has no reading in the grammar, and `None` says so; `get` makes
/// the boundary check explicit rather than a panic waiting for a user.
fn digits(text: &str, at: usize, until: usize) -> Option<&str> {
    let field = text.get(at..until)?;
    match field.bytes().all(|byte| byte.is_ascii_digit()) {
        true => Some(field),
        false => None,
    }
}

/// Which calendar the absolute shapes of a `-d` string are written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Calendar {
    /// 公历, the default.
    Civil,
    /// 农历, from `date -l`.
    Lunar {
        /// `-R`: the month is the leap one.
        leap: bool,
    },
}

/// Parses `input` relative to `reference`.
///
/// `calendar` only changes the reading of the absolute shapes; see the module
/// doc for what it deliberately leaves alone.
pub fn parse(input: &str, reference: CivilDate, calendar: Calendar) -> Result<CivilDate, CalError> {
    let text = input.trim();
    if text.is_empty() {
        return Err(invalid(input));
    }
    let lower = text.to_ascii_lowercase();

    if let Some(result) = parse_keyword(&lower, reference)? {
        return Ok(result);
    }
    if let Some(rest) = lower.strip_prefix('@') {
        return parse_epoch(rest, input);
    }
    if let Some(result) = parse_absolute(text, &lower, reference, calendar)? {
        return Ok(result);
    }
    if let Some(result) = parse_weekday(&lower, reference)? {
        return Ok(result);
    }
    parse_relative(&lower, reference, input)
}

/// Reads a year / month / day given as three separate integers — the
/// positional form of `lunar date`.
///
/// The same [`Calendar`] reading applies as for a `-d` string, so
/// `date -l 2026 7 15` and `date -l -d 2026-07-15` are the same day. The
/// components are read as numbers rather than as text, so a month or a day may
/// be written unpadded, exactly as before.
pub fn from_parts(
    year: i32,
    month: i32,
    day: i32,
    calendar: Calendar,
) -> Result<CivilDate, CalError> {
    resolve(Parts { year, month, day }, calendar)
}

/// `now`, `today`, `tomorrow`, `yesterday`.
fn parse_keyword(lower: &str, reference: CivilDate) -> Result<Option<CivilDate>, CalError> {
    let delta = match lower {
        "now" | "today" => 0,
        "tomorrow" => 1,
        "yesterday" => -1,
        _ => return Ok(None),
    };
    Ok(Some(reference.add_days(delta)))
}

/// `@seconds[.fraction]`, truncated towards the epoch day.
fn parse_epoch(rest: &str, input: &str) -> Result<CivilDate, CalError> {
    let whole = rest.split('.').next().unwrap_or(rest);
    let seconds: i64 = whole.parse().map_err(|_| invalid(input))?;
    let day = seconds.div_euclid(civil::SECS_PER_DAY);
    let date = CivilDate::from_epoch_day(day);
    validate(date, input)
}

/// A year / month / day triple, as written, before it is read in a calendar.
struct Parts {
    year: i32,
    month: i32,
    day: i32,
}

/// Absolute dates: ISO 8601, slash forms, bare year, and a trailing time of
/// day which only matters for zone shifts.
///
/// The grammar is read once and is the same in both calendars; [`resolve`]
/// decides what the triple means. A time-of-day suffix belongs to the civil
/// reading — shifting a lunar date across a time zone has no meaning here —
/// so `-l` rejects one rather than silently ignoring it.
fn parse_absolute(
    text: &str,
    lower: &str,
    reference: CivilDate,
    calendar: Calendar,
) -> Result<Option<CivilDate>, CalError> {
    let bytes = text.as_bytes();

    let input = text;
    // YYYYMMDD, optionally followed by a time part: 20260907T1530
    if let (Some(year), Some(month), Some(day)) =
        (digits(text, 0, 4), digits(text, 4, 6), digits(text, 6, 8))
    {
        let parts = Parts {
            year: number(year, input)?,
            month: number(month, input)?,
            day: number(day, input)?,
        };
        let rest = lower.get(8..).unwrap_or_default();
        return finish(parts, rest, calendar, input).map(Some);
    }

    // MM/DD/YYYY, the US order `date(1)` uses. This is tried *before* the
    // short-year branch below, which would otherwise claim `09/07/2026` as
    // the year 9 with a day of 2026. A triple whose last field is four
    // digits is the year, so the leading fields are bounded at two digits
    // each and `2026/09/07` still falls through to the YYYY/MM/DD branch.
    let slashes: Vec<&str> = text.split('/').collect();
    if slashes.len() == 3
        && slashes[2].len() == 4
        && slashes[0].len() <= 2
        && slashes[1].len() <= 2
        && slashes
            .iter()
            .all(|field| !field.is_empty() && field.bytes().all(|byte| byte.is_ascii_digit()))
    {
        let parts = Parts {
            year: number(slashes[2], input)?,
            month: number(slashes[0], input)?,
            day: number(slashes[1], input)?,
        };
        return finish(parts, "", calendar, input).map(Some);
    }

    // A date whose first field is a one- to three-digit year, e.g. `1-01-01`
    // or `999-12-31`.
    if bytes.len() >= 6
        && bytes
            .iter()
            .all(|b| b.is_ascii_digit() || *b == b'-' || *b == b'/')
    {
        let first = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
        if (1..=3).contains(&first) && (bytes[first] == b'-' || bytes[first] == b'/') {
            let sep = bytes[first];
            let fields: Vec<&str> = text.split(sep as char).collect();
            if fields.len() == 3 {
                let parts = Parts {
                    year: number(fields[0], input)?,
                    month: number(fields[1], input)?,
                    day: number(fields[2], input)?,
                };
                return finish(parts, "", calendar, input).map(Some);
            }
        }
    }

    // YYYY-MM-DD / YYYY/MM/DD, with the same separator in both positions.
    if bytes.len() >= 10 && (bytes[4] == b'-' || bytes[4] == b'/') && bytes[7] == bytes[4] {
        let parts = match (digits(text, 0, 4), digits(text, 5, 7), digits(text, 8, 10)) {
            (Some(year), Some(month), Some(day)) => Parts {
                year: number(year, input)?,
                month: number(month, input)?,
                day: number(day, input)?,
            },
            // The separators are in place but a field is not ASCII digits —
            // a multi-byte character in the day, say. The grammar has no
            // reading for that, so it is a bad date rather than a crash.
            _ => return Err(invalid(input)),
        };
        return finish(parts, lower.get(10..).unwrap_or_default(), calendar, input).map(Some);
    }

    // A bare year keeps the reference's month and day, read in the same
    // calendar: `-l -d 2026` is 农历 2026 年的同月同日.
    if bytes.len() == 4 && bytes.iter().all(u8::is_ascii_digit) {
        let year = number(text, input)?;
        let (month, day) = match calendar {
            Calendar::Civil => (reference.month, reference.day),
            Calendar::Lunar { .. } => {
                let lunar = reference.to_solar()?.lunar();
                (lunar.month(), lunar.day())
            }
        };
        return resolve(Parts { year, month, day }, calendar).map(Some);
    }

    // A bare time of day applies to today: 15:30, 15:30:45
    if let Some((_, _, _)) = parse_clock(text) {
        return Ok(Some(reference));
    }

    let _ = text;
    Ok(None)
}

/// Reads a parsed triple in the calendar `-l` selected.
fn resolve(parts: Parts, calendar: Calendar) -> Result<CivilDate, CalError> {
    let Parts { year, month, day } = parts;
    let solar = match calendar {
        Calendar::Civil => calendar::solar(year, month, day)?,
        // A leap month is named negatively, as `cal -L -R` names it.
        Calendar::Lunar { leap: true } => calendar::solar_from_lunar(year, -month.abs(), day)?,
        Calendar::Lunar { leap: false } => calendar::solar_from_lunar(year, month, day)?,
    };
    Ok(CivilDate::new(solar.year(), solar.month(), solar.day()))
}

/// Applies an optional trailing time/zone to an absolute date.
fn finish(
    parts: Parts,
    suffix: &str,
    calendar: Calendar,
    input: &str,
) -> Result<CivilDate, CalError> {
    let suffix = suffix.trim_start_matches(['t', ' ']);
    if suffix.is_empty() {
        return resolve(parts, calendar);
    }
    // A time of day belongs to the civil grammar; under `-l` it is rejected
    // rather than dropped, so `-l -d 2026-07-15T09:00` never looks like it
    // honoured the time.
    if calendar != Calendar::Civil {
        return Err(invalid(input));
    }
    let date = resolve(parts, calendar)?;
    let (clock, zone) = split_zone(suffix);
    // A zone with no clock is midnight in that zone: `2026-09-07Z` is
    // 00:00 UTC, and `2026-09-07+08:00` likewise. Only a zone *and* a clock
    // make the suffix a time of day, so this is not the `-l` case above.
    let time = match clock.is_empty() {
        true if zone.is_some() => (0, 0, 0),
        _ => parse_clock(clock).ok_or_else(|| invalid(input))?,
    };
    match zone {
        // A zone only matters when it shifts the day; keep the reference's
        // day when it does not, mirroring `date -d "… 23:00 UTC"` in UTC.
        Some(zone) => shift_by_zone(date, time, zone, input),
        None => validate(date, input),
    }
}

/// Reinterprets `date time` in `zone` and returns the local calendar day.
fn shift_by_zone(
    date: CivilDate,
    (hour, minute, _second): (i32, i32, i32),
    zone: &str,
    input: &str,
) -> Result<CivilDate, CalError> {
    let offset = zone_offset(zone).ok_or_else(|| invalid(input))?;
    let seconds =
        date.epoch_day() * civil::SECS_PER_DAY + i64::from(hour) * 3600 + i64::from(minute) * 60
            - offset;
    validate(
        CivilDate::from_epoch_day(seconds.div_euclid(civil::SECS_PER_DAY)),
        input,
    )
}

/// Splits a trailing zone designator off a time string.
fn split_zone(rest: &str) -> (&str, Option<&str>) {
    for marker in ["utc", "gmt", "z"] {
        if let Some(stripped) = rest.strip_suffix(marker) {
            return (stripped.trim_end(), Some("UTC"));
        }
    }
    let bytes = rest.as_bytes();
    // `+08:00` / `-0500`: the sign always follows a digit or a colon.
    for (index, byte) in bytes.iter().enumerate().skip(1) {
        if (*byte == b'+' || *byte == b'-')
            && (bytes[index - 1].is_ascii_digit() || bytes[index - 1] == b':')
        {
            return (rest[..index].trim_end(), Some(&rest[index..]));
        }
    }
    (rest, None)
}

/// Offset of a `UTC` or `+HH[[:]MM]` designator. POSIX signs are inverted:
/// `+0800` means 8 hours *behind* UTC.
fn zone_offset(zone: &str) -> Option<i64> {
    if zone.eq_ignore_ascii_case("utc") || zone.eq_ignore_ascii_case("gmt") {
        return Some(0);
    }
    let bytes = zone.as_bytes();
    let sign = match bytes.first()? {
        b'+' => -1,
        b'-' => 1,
        _ => return None,
    };
    let digits: String = zone[1..].chars().filter(char::is_ascii_digit).collect();
    let (hours, minutes) = match digits.len() {
        2 => (digits.parse::<i64>().ok()?, 0),
        4 => (
            digits[0..2].parse::<i64>().ok()?,
            digits[2..4].parse::<i64>().ok()?,
        ),
        _ => return None,
    };
    Some(sign * (hours * 3600 + minutes * 60))
}

/// Parses `hh`, `hh:mm`, `hh:mm:ss` and the compact `hhmm`.
///
/// The compact form is what `20260907T1530` carries, and it is four digits
/// with no separator, so it is read before the colon-splitting below — `1530`
/// is 15:30, not the hour 1530.
fn parse_clock(text: &str) -> Option<(i32, i32, i32)> {
    if text.is_empty() {
        return None;
    }
    if !text.contains(':') && text.len() == 4 && text.bytes().all(|byte| byte.is_ascii_digit()) {
        return finish_clock(text[0..2].parse().ok()?, text[2..4].parse().ok()?, 0);
    }
    let mut parts = text.split(':');
    let hour: i32 = parts.next()?.parse().ok()?;
    let minute: i32 = match parts.next() {
        Some(value) => value.parse().ok()?,
        None => 0,
    };
    let second: i32 = match parts.next() {
        Some(value) => value.parse().ok()?,
        None => 0,
    };
    if parts.next().is_some() {
        return None;
    }
    finish_clock(hour, minute, second)
}

/// Rejects an out-of-range clock, which is the only reason a parse fails here.
fn finish_clock(hour: i32, minute: i32, second: i32) -> Option<(i32, i32, i32)> {
    match (0..=23).contains(&hour) && (0..=59).contains(&minute) && (0..=59).contains(&second) {
        true => Some((hour, minute, second)),
        false => None,
    }
}

/// `monday`, `next friday`, `last sun`.
fn parse_weekday(lower: &str, reference: CivilDate) -> Result<Option<CivilDate>, CalError> {
    let (mode, name) = if let Some(rest) = lower.strip_prefix("next ") {
        (1, rest)
    } else if let Some(rest) = lower.strip_prefix("last ") {
        (-1, rest)
    } else {
        (0, lower)
    };
    let Some((_, target)) = WEEKDAYS.iter().find(|(candidate, _)| *candidate == name) else {
        return Ok(None);
    };
    let mut delta = target - reference.weekday();
    match mode {
        1 if delta <= 0 => delta += 7,
        -1 if delta >= 0 => delta -= 7,
        0 if delta < 0 => delta += 7,
        _ => {}
    }
    Ok(Some(reference.add_days(i64::from(delta))))
}

/// `+3 days`, `-2 weeks`, `2 days ago`, `1 fortnight`, `90 minutes`, `next month`.
fn parse_relative(lower: &str, reference: CivilDate, input: &str) -> Result<CivilDate, CalError> {
    let mut text = lower.trim().to_string();
    let mut sign = 1i32;
    if let Some(rest) = text.strip_prefix('+') {
        text = rest.trim().to_string();
    } else if let Some(rest) = text.strip_prefix('-') {
        sign = -1;
        text = rest.trim().to_string();
    }
    if let Some(rest) = text.strip_suffix(" ago") {
        sign = -sign;
        text = rest.trim().to_string();
    }

    // A bare unit, or a bare `next` / `last` before one, moves that period.
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(invalid(input));
    }

    let mut date = reference;
    let mut index = 0;
    let mut moved = false;
    while index < tokens.len() {
        // `next` / `last` take the unit that follows them and move one step
        // that way. `next friday` never reaches here — the weekday arm runs
        // first — so this is the bare-period form the grammar advertises:
        // `next month`, `last year`, `next week`.
        let (mut unit, count) = match tokens[index].parse::<i32>() {
            Ok(value) => {
                let unit = *tokens.get(index + 1).ok_or_else(|| invalid(input))?;
                index += 2;
                (unit, value * sign)
            }
            Err(_) => {
                let unit = tokens[index];
                index += 1;
                (unit, sign)
            }
        };
        let count = match unit {
            "next" => {
                unit = *tokens.get(index).ok_or_else(|| invalid(input))?;
                index += 1;
                1
            }
            "last" => {
                unit = *tokens.get(index).ok_or_else(|| invalid(input))?;
                index += 1;
                -1
            }
            _ => count,
        };
        date = apply_unit(date, unit, count, input)?;
        moved = true;
    }
    if !moved {
        return Err(invalid(input));
    }
    validate(date, input)
}

/// Applies one relative unit.
///
/// Sub-day units are **rejected**, not ignored. They used to return the date
/// unchanged, so `90 minutes ago` answered with today and read as though the
/// offset had been applied. This tool has no time of day — a day is the whole
/// of what it stores — so an offset shorter than a day has nothing to move,
/// and saying so is the only honest answer. `date(1)` accepts them because it
/// keeps a clock; this does not.
fn apply_unit(date: CivilDate, unit: &str, count: i32, input: &str) -> Result<CivilDate, CalError> {
    let unit = unit.trim_end_matches(['.', ',']);
    match unit {
        "fortnight" | "fortnights" => Ok(date.add_days(i64::from(count) * 14)),
        "day" | "days" => Ok(date.add_days(i64::from(count))),
        "week" | "weeks" => Ok(date.add_days(i64::from(count) * 7)),
        "month" | "months" => Ok(date.add_months(count)),
        "year" | "years" => Ok(date.add_years(count)),
        "sec" | "secs" | "second" | "seconds" | "min" | "mins" | "minute" | "minutes" | "hour"
        | "hours" => Err(CalError::SubDayUnit {
            unit: unit.to_string(),
        }),
        _ => Err(invalid(input)),
    }
}

/// Parses a decimal number.
fn number(text: &str, input: &str) -> Result<i32, CalError> {
    text.parse().map_err(|_| invalid(input))
}

/// Rejects years outside the supported window and impossible dates.
fn validate(date: CivilDate, input: &str) -> Result<CivilDate, CalError> {
    if !(MIN_YEAR..=MAX_YEAR).contains(&date.year) {
        return Err(CalError::YearOutOfRange {
            year: date.year,
            min: MIN_YEAR,
            max: MAX_YEAR,
        });
    }
    if !(1..=12).contains(&date.month) || date.day < 1 || date.day > date.days_in_month() {
        return Err(invalid(input));
    }
    Ok(date)
}
