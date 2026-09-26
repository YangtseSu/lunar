//! Grid rendering for `lunar cal`.
//!
//! Two views, each with a date band over a lunar content band:
//!
//! * **civil overlay** — one civil month, each cell `solar day` over
//!   `lunar day / solar term / festival / statutory holiday`:
//!
//!   ```text
//!           2026年9月
//!       一      二      三      四      五      六      日
//!             1       2       3       4       5       6
//!             二十    廿一    廿二    廿三    廿四    廿五
//!   ```
//!
//! * **lunar month** (`-L`) — one lunar month, each cell `M/D` over the same
//!   lunar content:
//!
//!   ```text
//!       农历 丙午年 七月
//!     一      二      三      四      五      六      日
//!                     8/13    8/14    8/15    8/16
//!                     初一    初二    初三    初四
//!   ```
//!
//! Layout rules:
//! * a cell is painted, never decorated with a character: today, 放假 and
//!   调休 are attributes (see [`crate::mark`]), so the grid stays aligned
//!   whether or not stdout is a terminal;
//! * cells are padded to a common width measured in **display columns**
//!   (`lang::width`), not characters, so a two-glyph label and a five-digit
//!   date line up;
//! * the width is the widest label or content the grid holds, so a month
//!   carrying `中秋节` is wider than one that does not, and every row of that
//!   grid shares the width;
//! * cells are left-aligned within their column and every line is
//!   `trim_end`ed;
//! * only the days of the month are shown: the days that share a week with
//!   its first day, and those after its last, are blank.

use std::fmt::Write as _;

use lunar_rs::{LunarMonth, Solar};

use crate::calendar::{self, CalError};
use crate::cell::{self, CellStyle};
use crate::civil::CivilDate;
use crate::lang;
use crate::mark::{self, Color, Mark, Statutory};

/// Blank columns between two cells, so a cell that exactly fills its column
/// still reads as separate from its neighbour.
const GUTTER: usize = 2;

/// What distinguishes one month's grid from another.
struct Month {
    /// Title line of the grid.
    title: String,
    /// Day 1 of the month.
    first: CivilDate,
    /// Blank cells before day 1.
    lead: usize,
    /// Days in the month.
    count: usize,
    /// Whether cells lead with a civil `M/D` rather than a day of month.
    lunar_view: bool,
    /// 0 = Sunday, 1 = Monday.
    week_start: i32,
}

/// One day slot of a grid.
struct Entry {
    /// The cell's date label: day of month, or `M/D` in the lunar view.
    label: String,
    /// Lunar-side cell content: lunar day, solar term or festival.
    content: String,
    /// `false` for the padding slots of the neighbouring months.
    in_month: bool,
    /// What the cell is painted with.
    mark: Mark,
}

/// A run of consecutive days forming one grid.
pub struct Grid {
    title: String,
    days: Vec<Entry>,
    column: usize,
    week_start: i32,
    /// Whether SGR escapes are emitted.
    color: bool,
}

impl Grid {
    /// Grid for one lunar month.
    ///
    /// The lunar view keeps weekday alignment: day 1 of the month sits in the
    /// column of its weekday, and the days before it are left blank.
    pub fn lunar(
        title: String,
        month: LunarMonth,
        week_start: i32,
        style: CellStyle,
        today: CivilDate,
        color: Color,
    ) -> Result<Self, CalError> {
        let first = calendar::lunar_month_start(&month);
        let lead = (first.weekday() - week_start).rem_euclid(7) as usize;
        let count = month.get_day_count() as usize;
        Self::build(
            Month {
                title,
                first,
                lead,
                count,
                lunar_view: true,
                week_start,
            },
            style,
            today,
            color,
        )
    }

    /// Grid for one civil month.
    pub fn civil(
        title: String,
        year: i32,
        month: i32,
        week_start: i32,
        style: CellStyle,
        today: CivilDate,
        color: Color,
    ) -> Result<Self, CalError> {
        let count = calendar::days_in_civil_month(year, month) as usize;
        let lead = calendar::week_offset(year, month, week_start);
        Self::build(
            Month {
                title,
                first: CivilDate::new(year, month, 1),
                lead,
                count,
                lunar_view: false,
                week_start,
            },
            style,
            today,
            color,
        )
    }

    /// Fills whole weeks starting at the month's first day.
    ///
    /// `first` is day 1 of the month; slot `lead` holds it and the slots
    /// before and after the month are blank padding. Every day of the month
    /// gets exactly one slot, so the last day is never dropped.
    fn build(
        spec: Month,
        style: CellStyle,
        today: CivilDate,
        color: Color,
    ) -> Result<Self, CalError> {
        let Month {
            title,
            first,
            lead,
            count,
            lunar_view,
            week_start,
        } = spec;
        let slots = (lead + count).div_ceil(7) * 7;
        let mut days = Vec::with_capacity(slots);
        for index in 0..slots {
            let offset = index as i64 - lead as i64;
            let in_month = offset >= 0 && (offset as usize) < count;
            let (label, content, mark) = match in_month {
                true => {
                    let date = first.add_days(offset);
                    let solar = date.to_solar()?;
                    let lunar = solar.lunar();
                    let label = match lunar_view {
                        true => format!("{}/{}", solar.month(), solar.day()),
                        false => solar.day().to_string(),
                    };
                    // The mark paints the cell; it never reaches the content,
                    // so what a day reads is the calendar alone.
                    let mark = day_mark(&solar, date, today, style.holiday);
                    let content = match lunar_view {
                        true => cell::lunar_month_content(&solar, &lunar, style),
                        false => cell::content(&solar, &lunar, style),
                    };
                    (label, content, mark)
                }
                false => (String::new(), String::new(), Mark::default()),
            };
            days.push(Entry {
                label,
                content,
                in_month,
                mark,
            });
        }
        let column = column_width(&days, week_start);
        Ok(Self {
            title,
            days,
            column,
            week_start,
            color: color.enabled(),
        })
    }

    /// Renders the grid, appending to `out`.
    pub fn render(&self, out: &mut String) {
        let _ = writeln!(out, "{}", self.title);
        let _ = writeln!(out, "{}", self.header());

        for chunk in self.days.chunks(7) {
            if chunk.iter().all(|entry| !entry.in_month) {
                continue;
            }
            let _ = writeln!(out, "{}", self.row_line(chunk, |entry| &entry.label));
            let _ = writeln!(out, "{}", self.row_line(chunk, |entry| &entry.content));
        }
    }

    /// The `一      二      …` heading, on the grid's own column width.
    fn header(&self) -> String {
        let mut line = String::with_capacity((self.column + GUTTER) * 7);
        for offset in 0..7 {
            let weekday = (self.week_start + offset).rem_euclid(7);
            line.push_str(&lang::pad_right(
                calendar::weekday_name(weekday),
                self.column,
            ));
            line.push_str(&" ".repeat(GUTTER));
        }
        line.trim_end().to_string()
    }

    /// One output line; `band` supplies each cell's text.
    fn row_line<'e>(&self, chunk: &'e [Entry], band: impl Fn(&'e Entry) -> &'e str) -> String {
        let mut line = String::with_capacity((self.column + GUTTER) * 7);
        for entry in chunk {
            let cell = match entry.in_month {
                true => band(entry),
                false => "",
            };
            mark::paint(&mut line, entry.mark, cell, self.column, self.color);
            line.push_str(&" ".repeat(GUTTER));
        }
        line.trim_end().to_string()
    }
}

/// What a day is marked with.
///
/// The two marks do not exclude each other: a reference day that is also 放假
/// is red *and* inverted, and a 调休 today is bold, bright and inverted. So
/// this builds a [`Mark`] out of both facts rather than choosing between them —
/// see [`crate::mark`]. `holiday` is `--no-holiday`, and it gates the statutory
/// half only; the reference day is marked either way.
fn day_mark(solar: &Solar, date: CivilDate, today: CivilDate, holiday: bool) -> Mark {
    let statutory = match holiday {
        false => Statutory::None,
        true => match calendar::legal_holiday(solar) {
            Some(entry) if entry.is_work() => Statutory::Work,
            Some(_) => Statutory::Rest,
            None => Statutory::None,
        },
    };
    Mark {
        holiday: statutory,
        today: date == today,
    }
}

/// The grid's cell width: the widest content it must hold.
///
/// The heading counts too, so a grid is never narrower than its own column
/// labels.
fn column_width(days: &[Entry], week_start: i32) -> usize {
    let mut widest = days
        .iter()
        .filter(|entry| entry.in_month)
        .flat_map(|entry| [entry.label.as_str(), entry.content.as_str()])
        .map(lang::width)
        .max()
        .unwrap_or(0);
    for offset in 0..7 {
        let weekday = (week_start + offset).rem_euclid(7);
        widest = widest.max(lang::width(calendar::weekday_name(weekday)));
    }
    widest
}

/// The `2026年9月` title of the civil overlay.
pub fn civil_title(year: i32, month: i32) -> String {
    format!("{year}年{month}月")
}

/// The `农历 丙午年 七月` title of the lunar view.
pub fn lunar_title(gan_zhi: &str, month_name: &str) -> String {
    format!("农历 {gan_zhi}年 {month_name}")
}
