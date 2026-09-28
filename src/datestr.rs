//! Parser for `lunar date -d` strings, in `date(1)` style.
//!
//! Accepted shapes:
//!
//! * keywords: `now`, `today`, `tomorrow`, `yesterday`
//! * epoch seconds: `@1758240000`, `@-1`, `@1758240000.25`
//! * ISO 8601: `2026-09-07`, `2026-9-7` (a month or day may be unpadded),
//!   `2026-09-07T15:30`, `20260907T1530`, `2026-09-07T15:30:45.5+08:00` (a
//!   fractional second is truncated — no clock is kept), `...Z`
//! * slashes: `2026/09/07`, `2026/9/7`, `09/07/2026` (month/day/year)
//! * relative: `+3 days`, `-2 weeks`, `2 days ago`, `1 fortnight`, `90 minutes`
//! * weekday names: `monday`, `next friday`, `last friday`, and the Chinese
//!   `星期六`, `周二`, `礼拜六`, `星期天` (this tool's extension)
//! * a bare time of day: `15:30`, `15:30 UTC` — today, in the zone given
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
//!
//! A time of day is **civil syntax**, whichever way it is written: as a suffix
//! on an absolute date or as a bare `15:30` of its own, and with or without a
//! zone. Under `-l` it is refused rather than dropped, and a zone on a bare
//! time resolves the day that time falls on in *that* zone — which is not
//! always the reference day.
//!
//! [`parse_moment`] is the one reader that **keeps** the clock, for
//! `lunar bazi`: a 时柱 cannot be answered without one. It shares this
//! grammar rather than a second one, so every form `parse` takes it takes
//! too, with the same failure — and a form that names no clock reads as no
//! clock, which is a fact about the input rather than a default.
//!
//! The fixed-width shapes are claimed on a **word boundary**, not on a digit
//! count: the compact `YYYYMMDD` form needs its eight digits to end the token,
//! so a bare timestamp (`1758240000`) and a relative offset (`2147483647 days`)
//! fall through to the branches that can answer them rather than being read as
//! a year / month / day the user never wrote.

use crate::calendar::{self, CalError, MAX_YEAR, MIN_YEAR};
use crate::civil::{self, CivilDate};

/// Weekday names accepted in `-d` strings.
///
/// The Chinese names are **this tool's extension** — `date(1)` has no
/// weekday name in Chinese, and refuses all of them. The table carries the
/// three prefixes a reader writes (`星期X`, `周X`, `礼拜X`) over 一…日 plus
/// the `天` variant, because the same suffix has to answer whichever prefix
/// a user reaches for: `周六` and `星期二` are the same request written two
/// ways, and a table that admitted only some of them made half the forms
/// unparsable.
const WEEKDAYS: [(&str, i32); 41] = [
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
    ("周日", 0),
    ("周天", 0),
    ("礼拜日", 0),
    ("礼拜天", 0),
    ("星期一", 1),
    ("周一", 1),
    ("礼拜一", 1),
    ("星期二", 2),
    ("周二", 2),
    ("礼拜二", 2),
    ("星期三", 3),
    ("周三", 3),
    ("礼拜三", 3),
    ("星期四", 4),
    ("周四", 4),
    ("礼拜四", 4),
    ("星期五", 5),
    ("周五", 5),
    ("礼拜五", 5),
    ("星期六", 6),
    ("周六", 6),
    ("礼拜六", 6),
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

/// A four-digit year followed by a month and a day, at least one of them a
/// single digit, as in `2026-9-7` or `2026/9/7`.
///
/// The fields are **digit runs, not spans between separators**, so a trailing
/// time of day is left over rather than mistaken for a third field; the last
/// element is the offset the date ends at, and the caller hands what follows
/// to [`finish`] as every other absolute branch does.
///
/// `None` for a fully padded `2026-09-07`, which the fixed-width branch owns,
/// and for a field pair the grammar cannot read: the two separators must
/// match, and the day must be digits. `2026-中-07` — a year, an unreadable
/// month and a two-digit day — has no reading here, and falls through to be
/// reported rather than to be answered with the reference day.
///
/// The input is the lower-cased text, as [`compact_suffix`] takes it, so the
/// `T` / `t` distinction of a suffix is already gone.
fn unpadded_iso(text: &str) -> Option<(&str, &str, &str, usize)> {
    let digit = |field: &str| field.bytes().all(|byte| byte.is_ascii_digit());
    let year = text.get(..4)?;
    let after_year = text.get(4..)?;
    if !digit(year) {
        return None;
    }
    for separator in ['-', '/'] {
        let Some(after_separator) = after_year.strip_prefix(separator) else {
            continue;
        };
        let month: usize = after_separator
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        let Some(rest) = after_separator.get(month + 1..) else {
            continue;
        };
        let day: usize = rest.bytes().take_while(u8::is_ascii_digit).count();
        // The day has to follow the **same** separator: `2026-9/7` and
        // `2026-中-07` — a year, an unreadable month and a two-digit day —
        // are not dates, and `date(1)` refuses both. They fall through here
        // to be reported rather than to be answered with the reference day.
        let (month_field, after_month) = after_separator.split_at_checked(month)?;
        let (day_field, after_day) = after_month.strip_prefix(separator)?.split_at_checked(day)?;
        // What may follow the day is the same set the compact form allows: a
        // `T` / `t` / space before a time part, or a zone designator standing
        // on its own. `2026-09-0中` has a digit run and then a character the
        // grammar cannot read, and the branch must leave it alone — claiming
        // it would report a bad **day** for an input the user wrote as a bad
        // date, where `date(1)` says `invalid date` and so does every other
        // branch here.
        let suffixed =
            after_day.is_empty() || after_day.starts_with(['t', ' ']) || is_bare_zone(after_day);
        // A padded `2026-09-07` is the fixed-width branch's, and the compact
        // `20260907` has no separator here at all.
        let unpadded =
            (1..=2).contains(&month) && (1..=2).contains(&day) && (month == 1 || day == 1);
        if unpadded && suffixed {
            return Some((year, month_field, day_field, 4 + 1 + month + 1 + day));
        }
    }
    None
}

/// What follows a compact `YYYYMMDD` date, or `None` when the run of digits
/// is not a compact date at all.
///
/// `text` is the lower-cased input, so the `T` / `t` distinction is gone and
/// only the word boundary matters. `lower.get(8..)` is `None` when offset 8
/// splits a character, which is how a multi-byte character in the date field
/// falls through to the branch that reports it.
fn compact_suffix(lower: &str) -> Option<&str> {
    let rest = lower.get(8..)?;
    match rest.is_empty() || rest.starts_with(['t', ' ']) || is_bare_zone(rest) {
        true => Some(rest),
        false => None,
    }
}

/// Whether a suffix is a zone designator with no time part, as in
/// `20260907Z`, `20260907UTC` and `20260907 GMT`.
///
/// These are the word-boundary shapes the grammar reads, and they only hold
/// when nothing is attached in front of the marker — `202609071Z` is a nine
/// digit run, not a date with a zone.
fn is_bare_zone(rest: &str) -> bool {
    matches!(rest.trim(), "z" | "utc" | "gmt")
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

/// A civil date **with** a time of day, as `lunar bazi` needs it.
///
/// [`parse`] keeps only the day — a calendar tool has nothing to do with a
/// clock — but 四柱 has a 时柱, so this carries what [`finish`] already
/// reads and then discards. The grammar is the same one: `parse_moment`
/// calls [`parse`] for the day and reads the clock off the same suffix, so
/// a form one accepts the other accepts too, with the same failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moment {
    /// The day the instant falls on.
    pub day: CivilDate,
    /// `hour * 3600 + minute * 60`, or `None` when no time was written.
    ///
    /// `None` is the honest answer for `1990-06-15`: the day is named, the
    /// clock is not, and a 时柱 invented from it would be a fabrication.
    /// Seconds are dropped — no 时辰 turns on a second, and `date` already
    /// truncates them.
    pub seconds_of_day: Option<i32>,
}

/// Parses `input` keeping the time of day, for `lunar bazi`.
///
/// Only the absolute 公历 shapes carry a time, which is the same restriction
/// [`parse`] places on one: a keyword, an `@epoch`, a weekday and a relative
/// offset all name an instant the tool cannot put a clock to. A bare `15:30`
/// applies to the reference day and *is* kept, since that is a clock written
/// for itself.
pub fn parse_moment(input: &str, reference: CivilDate) -> Result<Moment, CalError> {
    // `parse` is the authority on whether the string is a date at all, and it
    // has already read the suffix as part of that. Reading the clock back off
    // the same text cannot contradict it: a shape `parse` rejects never
    // reaches the second line, and one it accepts has a clock or has none.
    let day = parse(input, reference, Calendar::Civil)?;
    Ok(Moment {
        day,
        seconds_of_day: clock_of(input),
    })
}

/// The clock a `-d` string wrote, or `None` when it named none.
///
/// The scan mirrors [`finish`]: an ISO or compact date takes a trailing `T`
/// or space-separated time, and a clock written as itself is a clock too.
fn clock_of(input: &str) -> Option<i32> {
    let lower = input.trim().to_ascii_lowercase();
    // The `T` or space that opens a trailing clock, which is the same
    // boundary `finish` splits on. Every other shape has no clock.
    let Some((split, _)) = lower.char_indices().find(|(_, c)| *c == 't' || *c == ' ') else {
        // A clock written as itself, `15:30`, applies to the reference day.
        // The **colon** is what makes it one, and it has to be required here
        // for the reason `parse_absolute` requires it: `split_zone` reads the
        // `-` in `2025-01-29` as a zone sign and leaves `2025`, which the
        // compact `hhmm` branch would then take as 20:25 — a clock nobody
        // wrote, invented for every bare hyphenated date.
        let (clock, _) = split_zone(&lower);
        return match clock.contains(':') {
            true => clock_seconds(clock),
            false => None,
        };
    };
    let (clock, _) = split_zone(&lower[split + 1..]);
    clock_seconds(clock)
}

/// A parsed clock as seconds from midnight, or `None` if there is no clock.
fn clock_seconds(clock: &str) -> Option<i32> {
    let (hour, minute, _) = parse_clock(clock)?;
    Some(hour * 3600 + minute * 60)
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
    Ok(Some(reference.add_days(delta)?))
}

/// `@seconds[.fraction]`, read as the day `date(1)` reads it.
///
/// The fraction is dropped and the whole seconds are then **truncated
/// towards zero**, which is what `date -d @…` does: `@-1` is one second
/// before the epoch and GNU date answers 1970-01-01, not 1969-12-31. The
/// rule therefore is "the day holding the timestamp", not "the day at or
/// below it", and a negative timestamp never lands a day early.
///
/// The `-l` channel and the range check are unaffected: an epoch names an
/// instant, not a date written in either calendar.
fn parse_epoch(rest: &str, input: &str) -> Result<CivilDate, CalError> {
    let whole = rest.split('.').next().unwrap_or(rest);
    let seconds: i64 = whole.parse().map_err(|_| invalid(input))?;
    let date = CivilDate::from_epoch_day(seconds / civil::SECS_PER_DAY)?;
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
    //
    // The eight leading digits are a compact date only when a **word boundary**
    // follows them. The check was absent, so the branch claimed any input
    // beginning with eight digits: `1758240000` was read as 1758-24-00 and
    // `2147483647 days` as 2147-48-36, each reporting a month the user never
    // wrote. What may follow is a `T` / `t` / space before a time part, or a
    // zone designator standing on its own (`20260907Z`) — the shapes
    // `finish` and `split_zone` actually read. Anything else leaves the
    // compact form alone and falls through to the branches below, where the
    // grammar gives the input a truer answer or a rejection.
    if let (Some(year), Some(month), Some(day), Some(rest)) = (
        digits(text, 0, 4),
        digits(text, 4, 6),
        digits(text, 6, 8),
        compact_suffix(lower),
    ) {
        let parts = Parts {
            year: number(year, input)?,
            month: number(month, input)?,
            day: number(day, input)?,
        };
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

    // YYYY-M-D / YYYY/M/D, with a month or a day written in one digit:
    // `2026-9-7`, `2026/9/7`. Claimed only when a field is unpadded, so a
    // padded `2026-09-07` keeps the fixed-width branch below and no form
    // that already parsed changes shape. `date(1)` reads both.
    if let Some((year, month, day, consumed)) = unpadded_iso(lower) {
        let parts = Parts {
            year: number(year, input)?,
            month: number(month, input)?,
            day: number(day, input)?,
        };
        return finish(
            parts,
            lower.get(consumed..).unwrap_or_default(),
            calendar,
            input,
        )
        .map(Some);
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

    // A bare time of day applies to today: 15:30, 15:30:45, and with a
    // zone, `15:30 UTC` — the day that time falls on *in that zone*, which
    // is what `date(1)` answers and is not always today. It goes through
    // [`finish`] so the zone shift and the `-l` refusal are the two rules
    // every other time of day already obeys.
    //
    // The **colon** is what makes it a time rather than a year. `2026` and
    // `1530` are bare years the branch above already answered as the
    // reference's month and day, and a clock read off a `2026` hour would
    // have turned `2026-中-07` — a year, a Chinese character and a day —
    // into `2026:00` with a `07` zone, and answered the reference day for
    // an input the grammar has no reading for.
    let (clock, _) = split_zone(lower);
    if clock.contains(':') && parse_clock(clock).is_some() {
        let parts = Parts {
            year: reference.year,
            month: reference.month,
            day: reference.day,
        };
        return finish(parts, lower, calendar, input).map(Some);
    }

    let _ = text;
    Ok(None)
}

/// Reads a parsed triple in the calendar `-l` selected.
///
/// The month is **user input**, so it is checked before the leap form is
/// derived from it: `i32::MIN` has no absolute value, and negating it
/// panicked. `check_lunar_month` accepts it — the whole point of
/// `checked_abs` there is to let the error message name the number the
/// user actually wrote — so the check belongs here, before the `-`.
fn resolve(parts: Parts, calendar: Calendar) -> Result<CivilDate, CalError> {
    let Parts { year, month, day } = parts;
    let solar = match calendar {
        Calendar::Civil => calendar::solar(year, month, day)?,
        // A leap month is named negatively, as `cal -L -R` names it.
        Calendar::Lunar { leap: true } => {
            calendar::check_lunar_month(month)?;
            calendar::solar_from_lunar(year, -month, day)?
        }
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
        CivilDate::from_epoch_day(seconds.div_euclid(civil::SECS_PER_DAY))?,
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

/// Offset of a `UTC` or `+HH[[:]MM]` designator, in seconds. POSIX signs are
/// inverted: `+0800` means 8 hours *behind* UTC.
///
/// The hours and minutes are **range-checked**, not merely counted: the digit
/// count was all that was checked, so `+99:00` and `+09:99` parsed and
/// silently moved the day by four days and by ninety-nine minutes. No zone on
/// earth is 99 hours off UTC, and `date(1)` refuses both. `24:00` is the one
/// hour `date(1)` accepts past midnight, so it is accepted here too: it is a
/// whole day, not an offset.
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
    match hours <= 24 && (hours < 24 || minutes == 0) && minutes <= 59 {
        true => Some(sign * (hours * 3600 + minutes * 60)),
        false => None,
    }
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
        // A fractional second is accepted and **truncated**: this tool keeps
        // no clock, so `…T15:30:45.5+08:00` and `…T15:30:45+08:00` are the
        // same day. A fraction that was written at all must be digits — a
        // bare `.` and a `.abc` tail are two spellings `date(1)` refuses,
        // and so are we — while a seconds field without one is unchanged.
        Some(value) => match value.split_once('.') {
            None => value.parse().ok()?,
            Some((whole, fraction)) => {
                if fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
                    return None;
                }
                whole.parse().ok()?
            }
        },
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
    Ok(Some(reference.add_days(i64::from(delta))?))
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
/// Sub-day units are **rejected**, not ignored. This tool has no time of
/// day — a day is the whole of what it stores — so an offset shorter than a
/// day has nothing to move, and refusing it is the only honest answer:
/// `90 minutes ago` returned unchanged would read as though the offset had
/// been applied. `date(1)` accepts them because it keeps a clock; this does
/// not.
fn apply_unit(date: CivilDate, unit: &str, count: i32, input: &str) -> Result<CivilDate, CalError> {
    let unit = unit.trim_end_matches(['.', ',']);
    match unit {
        "fortnight" | "fortnights" => date.add_days(i64::from(count) * 14),
        "day" | "days" => date.add_days(i64::from(count)),
        "week" | "weeks" => date.add_days(i64::from(count) * 7),
        "month" | "months" => date.add_months(count),
        "year" | "years" => date.add_years(count),
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
            year: i64::from(date.year),
            min: MIN_YEAR,
            max: MAX_YEAR,
        });
    }
    if !(1..=12).contains(&date.month) || date.day < 1 || date.day > date.days_in_month() {
        return Err(invalid(input));
    }
    Ok(date)
}
