//! Grid rendering for `lunar cal`.
//!
//! Two views, each with a date band over a lunar content band:
//!
//! * **civil overlay** — one civil month, each cell `solar day` over
//!   `lunar day / solar term / festival`:
//!
//!   ```text
//!         2026年9月
//!   一     二     三     四     五     六     日
//!          1      2      3      4      5      6
//!          二十   廿一   廿二   廿三   廿四   廿五
//!   ```
//!
//! * **lunar month** (`-L`) — one lunar month, each cell `M/D` over the same
//!   lunar content:
//!
//!   ```text
//!       农历 丙午年 七月
//!   一     二     三     四     五     六     日
//!                        8/13   8/14   8/15   8/16
//!                        初一   初二   初三   初四
//!   ```
//!
//! Layout rules taken from those examples:
//!
//! * the cell pitch is 7 for the civil overlay (the widest cell content is a
//!   two-character solar term) and 6 for the lunar view;
//! * the weekday header is always 6 wide, so both views share one header line;
//! * the title is indented by 6 spaces (civil) or 5 (lunar view);
//! * days of the neighbouring months are blank in the content band, and the
//!   leading padding week is dropped entirely unless the month starts in the
//!   first column.

use std::fmt::Write as _;

use lunar_rs::LunarMonth;

use crate::calendar::{self, CalError};
use crate::cell::{self, CellStyle};
use crate::civil::CivilDate;

/// Cell pitch of the civil overlay.
const CIVIL_PITCH: usize = 7;
/// Cell pitch of the lunar-month view.
const LUNAR_PITCH: usize = 6;
/// Weekday header pitch, identical in both views.
const HEADER_PITCH: usize = 6;
/// Title indent of the civil overlay.
const CIVIL_TITLE_INDENT: usize = 6;
/// Title indent of the lunar view.
const LUNAR_TITLE_INDENT: usize = 5;

/// One day slot of a grid.
struct Entry {
    /// The cell's date label: day of month, or `M/D` in the lunar view.
    label: String,
    /// Lunar-side cell content: lunar day, solar term or festival.
    content: String,
    /// `false` for the padding slots of the neighbouring months.
    in_month: bool,
}

/// A run of consecutive days forming one grid.
pub struct Grid {
    title: String,
    title_indent: usize,
    days: Vec<Entry>,
    pitch: usize,
}

impl Grid {
    /// Grid for one lunar month.
    ///
    /// The lunar view keeps weekday alignment: day 1 of the month sits in the
    /// column of its weekday, and the days before it are left blank. Cells are
    /// 6 wide, which also fits a three-character cell such as `闰四月`.
    pub fn lunar(
        title: String,
        month: LunarMonth,
        week_start: i32,
        style: CellStyle,
    ) -> Result<Self, CalError> {
        let first = calendar::lunar_month_start(&month);
        let lead = (first.weekday() - week_start).rem_euclid(7) as usize;
        let mut grid = Self::build(
            title,
            first.add_days(-(lead as i64)),
            lead,
            month.get_day_count() as usize,
            Some(month.month()),
            style,
        )?;
        for index in 0..lead {
            if let Some(entry) = grid.days.get_mut(index) {
                entry.label.clear();
                entry.content.clear();
                entry.in_month = false;
            }
        }
        Ok(grid)
    }

    /// Fills whole weeks starting at `first`; a cell belongs to the rendered
    /// month when it is one of its `days`, or when its lunar month is
    /// `lunar_month` (the first day of a lunar month).
    fn build(
        title: String,
        first: CivilDate,
        lead: usize,
        days: usize,
        lunar_month: Option<i32>,
        style: CellStyle,
    ) -> Result<Self, CalError> {
        // A lone overflow day is dropped, but a partially filled last week is
        // completed with the next month's days, as `cal` does; those cells keep
        // their date but never carry lunar content.
        let natural = (lead + days).div_ceil(7) * 7;
        let slots = match natural - lead - days {
            0 | 1 => natural,
            rest => natural - rest + 7,
        };
        let mut entries = Vec::with_capacity(slots);
        for index in 0..slots {
            let solar = first.add_days(index as i64).to_solar()?;
            let lunar = solar.lunar();
            let in_month = index < days || lunar_month == Some(lunar.month());
            let label = match lunar_month {
                Some(_) => format!("{}/{}", solar.month(), solar.day()),
                None => solar.day().to_string(),
            };
            let content = if in_month {
                cell::content(&lunar, style)
            } else {
                String::new()
            };
            entries.push(Entry {
                label,
                content,
                in_month,
            });
        }
        let (pitch, title_indent) = match lunar_month {
            Some(_) => (LUNAR_PITCH, LUNAR_TITLE_INDENT),
            None => (CIVIL_PITCH, CIVIL_TITLE_INDENT),
        };
        Ok(Self {
            title,
            title_indent,
            days: entries,
            pitch,
        })
    }
    /// Grid for one civil month whose leading neighbouring-month days are
    /// blanked out, the way `cal` does it: the days that share a week with the
    /// first of the month are not shown, so a grid never opens on a partial
    /// week. The documented samples print exactly that, e.g. 2026年9月 opens
    /// with a week that starts on 1 September.
    pub fn civil_blanked(
        title: String,
        year: i32,
        month: i32,
        week_start: i32,
        style: CellStyle,
    ) -> Result<Self, CalError> {
        let days = calendar::days_in_civil_month(year, month) as usize;
        let lead = calendar::week_offset(year, month, week_start);
        let slots = (lead + days).div_ceil(7) * 7;
        let blank = (slots - lead - days).min(lead);
        let mut grid = Self::build(
            title,
            CivilDate::new(year, month, 1).add_days(-(lead as i64)),
            lead,
            days,
            None,
            style,
        )?;
        for index in 0..blank {
            if let Some(entry) = grid.days.get_mut(index) {
                entry.label.clear();
                entry.content.clear();
                entry.in_month = false;
            }
        }
        Ok(grid)
    }

    /// Renders the grid, appending to `out`.
    pub fn render(&self, week_start: i32, out: &mut String) {
        let _ = writeln!(out, "{}{}", " ".repeat(self.title_indent), self.title);
        let _ = writeln!(out, "{}", weekday_header(week_start));

        for chunk in self.days.chunks(7) {
            if chunk.iter().all(|entry| !entry.in_month) {
                continue;
            }
            let _ = writeln!(out, "{}", self.row_line(chunk, |entry| entry.label.clone()));
            let _ = writeln!(
                out,
                "{}",
                self.row_line(chunk, |entry| entry.content.clone())
            );
        }
    }

    /// One output line; `label` supplies each cell's text.
    fn row_line(&self, chunk: &[Entry], label: impl Fn(&Entry) -> String) -> String {
        let mut line = String::with_capacity(self.pitch * 7);
        for entry in chunk {
            let text = if entry.in_month {
                label(entry)
            } else {
                String::new()
            };
            let _ = write!(line, "{:>width$}", text, width = self.pitch);
        }
        line.trim_end().to_string()
    }
}

/// `一     二     …` on the shared header pitch.
fn weekday_header(week_start: i32) -> String {
    let mut line = String::with_capacity(HEADER_PITCH * 7);
    for column in 0..7 {
        let weekday = (week_start + column).rem_euclid(7);
        let _ = write!(
            line,
            "{:>width$}",
            calendar::weekday_name(weekday),
            width = HEADER_PITCH
        );
    }
    line.trim_end().to_string()
}

/// The `2026年9月` title of the civil overlay.
pub fn civil_title(year: i32, month: i32) -> String {
    format!("{year}年{month}月")
}

/// The `农历 丙午年 七月` title of the lunar view.
pub fn lunar_title(gan_zhi: &str, month_name: &str) -> String {
    format!("农历 {gan_zhi}年 {month_name}")
}
