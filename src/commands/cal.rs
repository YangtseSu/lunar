//! `lunar cal` — month and year grids with Chinese lunar content.
//!
//! The command surface:
//!
//! * no arguments → the current civil month (or the current lunar month with `-L`)
//! * one argument → a whole year
//! * two arguments → `年 月`
//! * `-L` switches from civil months to lunar months, `-R` picks the leap month
//! * `-s` / `-m` pick the first column (Monday is the default); `-y` prints
//!   the whole year, `-3` the month named and its neighbours, `-n N` the next
//!   `N` months — the readings `cal(1)` gives them
//! * `--number`, `--no-month-name`, `--no-festival`, `--no-holiday` change cell
//!   content
//! * `--color[=auto|always|never]` / `--no-color` override the automatic SGR of
//!   the reference day, the 法定节假日 and the 调休 marks; the two override
//!   each other, so the one written last decides

use lunar_rs::{Lunar, LunarMonth};

use crate::calendar::{self, CalError};
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
    /// `-y`: the whole year.
    pub year: bool,
    /// `-3`: the month named and the one before and after it.
    pub three: bool,
    /// `-n N`: the next `N` months, starting at the month named.
    pub months: Option<i32>,
    /// `--number`.
    pub number: bool,
    /// `--no-month-name`.
    pub no_month_name: bool,
    /// `--no-festival`.
    pub no_festival: bool,
    /// `--no-holiday`.
    pub no_holiday: bool,
    /// `--color[=WHEN]` / `--no-color`, already resolved to one mode.
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

/// How many months a request prints, and where its window starts.
///
/// The readings are `cal(1)`'s: `-3` centres on the month named, `-n N` starts
/// at it, `-y` is the whole year. A month the arguments leave out is the one
/// the reference day falls in, which is how a bare `cal` reaches the current
/// month and `cal 2026` the year the reference day is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Span {
    /// The twelve months of the year named — a lunar year's thirteen included.
    WholeYear,
    /// `count` months with the month named in the middle.
    Centred(i32),
    /// `count` months from the month named forwards.
    Forward(i32),
    /// The month named, alone.
    Single,
}

impl Span {
    /// The months a span takes out of a continuous month sequence, as
    /// `(offset of the first from the month named, how many)`.
    ///
    /// `None` is the whole year: it is the named year rather than a window
    /// through it, and a lunar year holds thirteen months where a civil one
    /// holds twelve, so the two views answer it from that year's own months.
    fn window(self) -> Option<(i32, i32)> {
        match self {
            Self::WholeYear => None,
            Self::Single => Some((0, 1)),
            // Centred: the month named in the middle, an even count falling
            // one month forwards. `-3` is the only centred span the surface
            // has.
            Self::Centred(count) => Some((-(count - 1) / 2, count)),
            Self::Forward(count) => Some((0, count)),
        }
    }
}

/// The span the arguments ask for.
///
/// A span flag is the reader's own reading of the range, so it wins over the
/// shape of the positionals; `-y` names a year and nothing else, which is
/// what `cal(1)` reads it as. `-n N` is bounded below by 1 at parse time, so
/// a count reaching here is always positive.
fn span_of(args: &CalArgs) -> Span {
    if args.year {
        return Span::WholeYear;
    }
    if let Some(months) = args.months {
        return Span::Forward(months);
    }
    if args.three {
        return Span::Centred(3);
    }
    match args.positional.len() {
        // A bare year asks for the year, as `cal(1)` reads it.
        1 => Span::WholeYear,
        _ => Span::Single,
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
        1 => (positional_int(&args.positional[0])?, today.month),
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

    for (index, first) in civil_months(year, month, span_of(args))?
        .into_iter()
        .enumerate()
    {
        if index > 0 {
            out.push('\n');
        }
        let grid = Grid::civil(
            calgrid::civil_title(first.year, first.month),
            first.year,
            first.month,
            week_start,
            style,
            today,
            color,
        )?;
        grid.render(out);
    }
    Ok(())
}

/// The civil months a request prints, as the first day of each.
///
/// The months are stepped through the one sequence both views share, so a
/// window crosses a year boundary the way `cal(1)`'s does: `cal 2026 12 -3`
/// is 十一月, 十二月 and 一月 2027. A month outside the supported range is
/// reported by name rather than clipped, so a window that runs off the end of
/// the range fails instead of coming back short.
fn civil_months(year: i32, month: i32, span: Span) -> Result<Vec<CivilDate>, CalError> {
    let (first, count) = match span.window() {
        Some(window) => window,
        // A whole year is the twelve months of the year the anchor names, so
        // it is a window of the same sequence measured from the anchor.
        None => (1 - month, 12),
    };
    let anchor = CivilDate::new(year, month, 1);
    let mut months = Vec::new();
    for step in 0..count {
        let first_of_month = anchor.add_months(first + step)?;
        calendar::check_year(first_of_month.year)?;
        months.push(first_of_month);
    }
    Ok(months)
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
        0 => (current.year(), current.month()),
        1 => (positional_int(&args.positional[0])?, current.month()),
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
    // The month is user input wherever a positional carries it, and its
    // Chinese name is a table lookup on the way to an error message, so `0`
    // or `13` would index past the end of the table. Checked before anything
    // reads it — the check also rejects `i32::MIN`, whose `abs()` would
    // overflow below.
    calendar::check_lunar_month(month)?;

    let selected = select_lunar_months(year, month, args.leap, span_of(args))?;
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
/// The month named anchors every span and `-y` ignores it: `-y` names a year,
/// exactly as `cal(1)` reads it, and a lunar year walks its own months with
/// the leap month in its place.
fn select_lunar_months(
    year: i32,
    month: i32,
    leap: bool,
    span: Span,
) -> Result<Vec<LunarMonth>, CalError> {
    let months = calendar::lunar_year(year)?;
    let wanted = if leap { -month.abs() } else { month };
    let Some(anchor) = calendar::lunar_year_month(&months, wanted) else {
        return match leap {
            true => Err(CalError::NoLeapMonth { year }),
            false => Err(CalError::NoSuchLunarMonth { year, month }),
        };
    };
    match span.window() {
        None => Ok(calendar::lunar_year_months(&months)),
        Some((first, count)) => lunar_sequence(&anchor, first, count),
    }
}

/// `count` consecutive lunar months, the first of them `first` months before
/// `anchor`.
///
/// The lunar month sequence is continuous — 腊月 is followed by the next
/// year's 正月 — so a window that runs off the end of a year continues in the
/// next one instead of stopping short. Every grid is titled with its own
/// ganzhi year, so a window that crosses the new year reads without a note.
/// The months come from the years the window actually reaches, and a year
/// outside the supported range is reported: the window is what reached past
/// the end, and the year it reached is the honest thing to name.
fn lunar_sequence(
    anchor: &LunarMonth,
    first: i32,
    count: i32,
) -> Result<Vec<LunarMonth>, CalError> {
    let mut sequence = lunar_months_of(anchor.year())?;
    let Some(start) = sequence
        .iter()
        .position(|month| month.month() == anchor.month())
    else {
        return Err(CalError::NoSuchLunarMonth {
            year: anchor.year(),
            month: anchor.month(),
        });
    };
    // How many months of earlier years the window's first month needs, and
    // the year the sequence starts with: years are prepended until the window
    // starts inside what is held.
    let mut before = start as i64 + i64::from(first);
    let mut year = anchor.year();
    while before < 0 {
        year -= 1;
        let mut earlier = lunar_months_of(year)?;
        before += earlier.len() as i64;
        earlier.append(&mut sequence);
        sequence = earlier;
    }
    // And forwards, until the sequence holds every month the window wants.
    while (sequence.len() as i64) < before + i64::from(count) {
        year += 1;
        sequence.append(&mut lunar_months_of(year)?);
    }
    let (Ok(first), Ok(count)) = (usize::try_from(before), usize::try_from(count)) else {
        return Err(CalError::BadArgument {
            detail: String::new(),
        });
    };
    Ok(sequence[first..first + count].to_vec())
}

/// The months of a lunar year, in calendar order.
fn lunar_months_of(year: i32) -> Result<Vec<LunarMonth>, CalError> {
    let year = calendar::lunar_year(year)?;
    Ok(calendar::lunar_year_months(&year))
}

/// `农历 丙午年 七月`, with `闰` for a leap month; the year pillar and the
/// month name come from the engine in the Chinese it already uses.
fn lunar_month_title(month: &LunarMonth) -> String {
    let first = month.get_first_day();
    let gan_zhi = first
        .as_ref()
        .map(Lunar::year_in_gan_zhi)
        .unwrap_or_default();
    calgrid::lunar_title(&gan_zhi, &calendar::lunar_month_name(month.month()))
}

/// Parses a positional integer.
fn positional_int(text: &str) -> Result<i32, CalError> {
    text.parse().map_err(|_| CalError::BadArgument {
        detail: text.to_string(),
    })
}
