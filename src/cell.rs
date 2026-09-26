//! The text a `cal` cell shows under (or over) a day number.
//!
//! Cell priority:
//!
//! ```text
//! 格子优先级：法定节假日 > 节日 > 初一显示月份名 > 节气 > 农历日
//! ```
//!
//! Each overlay can be switched off — `--no-holiday` / `--no-festival` /
//! `--no-month-name` — and `--number` replaces the lunar day with its digits.
//!
//! The statutory level is the one the traditional festivals cannot answer:
//! 清明 is a festival, but a 清明 holiday runs three days and the days either
//! side of it are days off too. The level is fed a [`Mark`] rather than a
//! `Holiday`, because a 调休 workday is a mark and not an entry of its own —
//! see [`holiday_label`].

use lunar_rs::{Lunar, Solar};

use crate::calendar;
use crate::mark::{self, Mark};

/// Which optional overlays are enabled.
#[derive(Debug, Clone, Copy)]
pub struct CellStyle {
    /// `--number`: show 农历日 as digits instead of 初一/廿六.
    pub number: bool,
    /// `--no-month-name`: never replace 初一 with the month name.
    pub month_name: bool,
    /// `--no-festival`: never show festivals.
    pub festival: bool,
    /// `--no-holiday`: never show the statutory calendar.
    pub holiday: bool,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            number: false,
            month_name: true,
            festival: true,
            holiday: true,
        }
    }
}

/// Label of the lunar day itself, honouring `--number`.
pub fn day_label(lunar: &Lunar, style: CellStyle) -> String {
    match style.number {
        true => lunar.day().to_string(),
        false => calendar::lunar_day_name(lunar.day()).to_string(),
    }
}

/// The statutory label of a day, or `None` when it is an ordinary day.
///
/// A 放假 day shows the name the State Council gave it — `国庆节` — and falls
/// back to [`mark::REST_LABEL`] when a traditional festival of the same name
/// already fills the cell, so `中秋节` is not printed twice. A 调休 workday
/// has no name of its own: it shows [`mark::WORK_LABEL`], which survives
/// every level below, because a working Saturday reading 七夕 is a lie.
pub fn holiday_label(solar: &Solar, lunar: &Lunar, style: CellStyle, mark: Mark) -> Option<String> {
    if !style.holiday || !mark.is_tint() {
        return None;
    }
    let holiday = calendar::legal_holiday(solar)?;
    Some(match holiday.is_work() {
        false => match calendar::traditional_festivals(solar, lunar).as_deref() {
            Some(name) if name == holiday.get_name() => mark::REST_LABEL.to_string(),
            _ => holiday.get_name(),
        },
        true => mark::WORK_LABEL.to_string(),
    })
}

/// The cell content of a day in a **lunar month** view.
///
/// The lunar grid leads with the civil date, so the month name is redundant
/// with the title and only the overlays apply: statutory holiday, else
/// festival, else solar term, else the lunar day. The civil overlay adds the
/// month-name level through [`content`].
pub fn lunar_month_content(solar: &Solar, lunar: &Lunar, style: CellStyle, mark: Mark) -> String {
    if let Some(label) = holiday_label(solar, lunar, style, mark) {
        return label;
    }
    if style.festival
        && let Some(name) = calendar::traditional_festivals(solar, lunar)
    {
        return name;
    }
    if let Some(term) = calendar::jie_qi(lunar) {
        return term;
    }
    day_label(lunar, style)
}

/// The full cell content: statutory holiday, else festival, else month name
/// on 初一, else solar term, else the lunar day.
pub fn content(solar: &Solar, lunar: &Lunar, style: CellStyle, mark: Mark) -> String {
    if let Some(label) = holiday_label(solar, lunar, style, mark) {
        return label;
    }
    // A 调休 workday outranks every level: its cell is about the calendar
    // being moved, not about what else the day happens to carry.
    if style.holiday
        && let Mark::Work = mark
    {
        return mark::WORK_LABEL.to_string();
    }
    if style.festival
        && let Some(name) = calendar::traditional_festivals(solar, lunar)
    {
        return name;
    }
    // The lunar new year day keeps its festival label; every other 初一 shows
    // the month name instead of the day number.
    if style.month_name && lunar.day() == 1 && lunar.month() != 1 {
        return calendar::lunar_month_name(lunar.month());
    }
    if let Some(term) = calendar::jie_qi(lunar) {
        return term;
    }
    day_label(lunar, style)
}
