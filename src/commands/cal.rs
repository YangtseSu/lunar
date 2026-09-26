//! `lunar cal` — month and year grids with Chinese lunar content.
//!
//! The command surface:
//!
//! * no arguments → the current civil month (or the current lunar month with `-L`)
//! * one argument → a whole year
//! * two arguments → `年 月`
//! * `-L` switches from civil months to lunar months, `-R` picks the leap month
//! * `-s` / `-m` pick the first column (Monday is the default), `-y`, `-3`, `-n N`
//!   extend the range
//! * `--number`, `--no-month-name`, `--no-festival`, `--no-holiday` change cell
//!   content
//! * `--color` / `--no-color` override the automatic SGR of the reference day,
//!   the 法定节假日 and the 调休 marks

use std::sync::Arc;

use lunar_rs::{Lunar, LunarMonth, LunarYear};

use crate::calendar::{self, CalError, MAX_YEAR};
use crate::calgrid::{self, Grid};
use crate::cell::CellStyle;
use crate::civil::CivilDate;
use crate::mark::Color;

/// Everything `lunar cal` was asked to do.
#[derive(Debug, Clone)]
pub struct CalArgs {
    /// Positional `年 [月]`.
    pub positional: Vec<String>,
    /// `-L`: show lunar months instead of civil months.
    pub lunar: bool,
    /// `-R`: with `-L`, select the leap month.
    pub leap: bool,
    /// `-s`: Sunday starts the week.
    pub sunday: bool,
    /// `-y`: whole year.
    pub year: bool,
    /// `-3`: three consecutive months.
    pub three: bool,
    /// `-n N`: N consecutive months.
    pub months: Option<i32>,
    /// `--number`.
    pub number: bool,
    /// `--no-month-name`.
    pub no_month_name: bool,
    /// `--no-festival`.
    pub no_festival: bool,
    /// `--no-holiday`.
    pub no_holiday: bool,
    /// `--color` / `--no-color`.
    pub color: Color,
}

/// Runs `lunar cal`, writing to `out`.
pub fn run(args: &CalArgs, today: CivilDate, out: &mut String) -> Result<(), CalError> {
    let style = CellStyle {
        number: args.number,
        month_name: !args.no_month_name,
        festival: !args.no_festival,
        holiday: !args.no_holiday,
    };
    let color = args.color;
    // Monday is the default first column, `-s` switches to Sunday.
    let week_start = if args.sunday { 0 } else { 1 };

    if args.lunar {
        run_lunar(args, today, style, color, week_start, out)
    } else {
        run_civil(args, today, style, color, week_start, out)
    }
}

/// Civil-month view: `cal`, `cal 2026`, `cal 2026 9`, `-y`, `-3`, `-n N`.
fn run_civil(
    args: &CalArgs,
    today: CivilDate,
    style: CellStyle,
    color: Color,
    week_start: i32,
    out: &mut String,
) -> Result<(), CalError> {
    let (year, month) = match args.positional.len() {
        0 => (today.year, today.month),
        // A bare year means the whole year, so it starts in January.
        1 => (positional_int(&args.positional[0])?, 1),
        2 => (
            positional_int(&args.positional[0])?,
            positional_int(&args.positional[1])?,
        ),
        _ => {
            return Err(CalError::BadArgument {
                detail: String::new(),
            });
        }
    };
    calendar::check_year(year)?;
    calendar::check_month(month)?;

    // A single positional is a year, so it always means the whole year; `-n`,
    // `-3` and `-y` can extend a month request.
    let count = if args.positional.len() == 1 && args.months.is_none() && !args.three {
        12
    } else {
        month_count(args)
    };
    let mut cursor = CivilDate::new(year, month, 1);
    for index in 0..count {
        if index > 0 {
            out.push('\n');
        }
        if cursor.year > MAX_YEAR {
            return Err(CalError::YearOutOfRange {
                year: cursor.year,
                min: calendar::MIN_YEAR,
                max: MAX_YEAR,
            });
        }
        let grid = Grid::civil(
            calgrid::civil_title(cursor.year, cursor.month),
            cursor.year,
            cursor.month,
            week_start,
            style,
            today,
            color,
        )?;
        grid.render(out);
        cursor = cursor.add_months(1);
    }
    Ok(())
}

/// Lunar-month view: `cal -L`, `cal -L 2026`, `cal -L 2026 7`, `-L -R`.
fn run_lunar(
    args: &CalArgs,
    today: CivilDate,
    style: CellStyle,
    color: Color,
    week_start: i32,
    out: &mut String,
) -> Result<(), CalError> {
    let current = today.to_solar()?.lunar();
    let (year, month) = match args.positional.len() {
        0 | 1 => {
            let year = if args.positional.is_empty() {
                current.year()
            } else {
                positional_int(&args.positional[0])?
            };
            // A bare year shows the whole lunar year unless a month is implied.
            let month = if args.year { 1 } else { current.month() };
            (year, month)
        }
        2 => (
            positional_int(&args.positional[0])?,
            positional_int(&args.positional[1])?,
        ),
        _ => {
            return Err(CalError::BadArgument {
                detail: String::new(),
            });
        }
    };
    let months = calendar::lunar_year(year)?;
    let selected = select_lunar_months(&months, year, month, args)?;

    for (index, month) in selected.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let grid = Grid::lunar(
            lunar_month_title(month),
            *month,
            week_start,
            style,
            today,
            color,
        )?;
        grid.render(out);
    }
    Ok(())
}

/// Resolves which lunar months a `-L` invocation should print.
///
/// `cal -L 2026` walks the whole lunar year, so it picks up the leap month on
/// its own; an explicit `年 月` prints just that month, and only prints the
/// leap month when `-R` is given.
fn select_lunar_months(
    year: &Arc<LunarYear>,
    year_number: i32,
    month: i32,
    args: &CalArgs,
) -> Result<Vec<LunarMonth>, CalError> {
    let all = calendar::lunar_year_months(year);

    if args.positional.len() >= 2 {
        // The month is user-supplied here, and its Chinese name is a table
        // lookup, so `0` or `13` would index past the end of the table and
        // panic. Validate it before anything reads it — the check also
        // rejects `i32::MIN`, whose `abs()` would overflow below.
        calendar::check_lunar_month(month)?;
        let wanted = if args.leap { -month.abs() } else { month };
        let Some(anchor) = calendar::lunar_year_month(year, wanted) else {
            return match args.leap {
                true => Err(CalError::NoLeapMonth { year: year_number }),
                false => Err(CalError::NoSuchLunarMonth {
                    year: year_number,
                    month,
                }),
            };
        };
        // `-3` / `-n N` widen the request the way they do in the civil view:
        // the named month plus its neighbours, centred on it. A span of one is
        // the named month alone, which is what an explicit `年 月` always was.
        let span = month_count(args) as usize;
        if span <= 1 {
            return Ok(vec![anchor]);
        }
        let start = all
            .iter()
            .position(|candidate| candidate.month() == anchor.month())
            .unwrap_or(0)
            .saturating_sub(span / 2);
        return Ok(all.into_iter().skip(start).take(span).collect());
    }

    // A whole-year request, or an explicit span, walks every month.
    let whole_year = args.year || args.positional.len() == 1;
    if whole_year || args.three || args.months.is_some() {
        return Ok(all);
    }

    // A single month, defaulting to the one the reference day falls in.
    let anchor = all
        .iter()
        .position(|candidate| candidate.month() == month)
        .unwrap_or(0);
    let span = month_count(args) as usize;
    let start = if span > 1 {
        anchor.saturating_sub(span / 2)
    } else {
        anchor
    };
    Ok(all.into_iter().skip(start).take(span).collect())
}

/// `农历 丙午年 七月`, with `闰` for a leap month; the year pillar and the
/// month name are rendered in the chosen language.
fn lunar_month_title(month: &LunarMonth) -> String {
    let first = month.get_first_day();
    let gan_zhi = first
        .as_ref()
        .map(Lunar::year_in_gan_zhi)
        .unwrap_or_default();
    calgrid::lunar_title(&gan_zhi, &calendar::lunar_month_name(month.month()))
}

/// How many months to print: `-n N`, `-3`, or — for a whole-year request such
/// as `cal -y` or `cal 2026` — all twelve.
fn month_count(args: &CalArgs) -> i32 {
    if let Some(months) = args.months {
        return months.max(1);
    }
    if args.three {
        return 3;
    }
    if args.year {
        return 12;
    }
    1
}

/// Parses a positional integer.
fn positional_int(text: &str) -> Result<i32, CalError> {
    text.parse().map_err(|_| CalError::BadArgument {
        detail: text.to_string(),
    })
}
