//! The text a `cal` cell shows under (or over) a day number.
//!
//! Priority, as documented for `cal_nongli`:
//!
//! ```text
//! 格子优先级：节日 > 初一显示月份名 > 节气 > 农历日
//! ```
//!
//! Each level can be switched off with `--no-festival` / `--no-month-name`,
//! and `--number` replaces the Chinese day numerals with digits.

use lunar_rs::Lunar;

use crate::calendar;

/// Which optional overlays are enabled.
#[derive(Debug, Clone, Copy)]
pub struct CellStyle {
    /// `--number`: show 农历日 as digits instead of 初一/廿六.
    pub number: bool,
    /// `--no-month-name`: never replace 初一 with the month name.
    pub month_name: bool,
    /// `--no-festival`: never show festivals.
    pub festival: bool,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self { number: false, month_name: true, festival: true }
    }
}

/// Label of the lunar day itself, honouring `--number`.
pub fn day_label(lunar: &Lunar, style: CellStyle) -> String {
    if style.number { lunar.day().to_string() } else { calendar::lunar_day_name(lunar.day()).to_string() }
}

/// The full cell content: festival, else month name on 初一, else solar term,
/// else the lunar day.
pub fn content(lunar: &Lunar, style: CellStyle) -> String {
    if style.festival
        && let Some(name) = calendar::traditional_festivals(lunar) {
            return name.to_string();
        }
    // The lunar new year day keeps its festival label; every other 初一 shows
    // the month name instead of the day number.
    if style.month_name && lunar.day() == 1 && lunar.month() != 1 {
        return calendar::lunar_month_name(lunar.month());
    }
    if let Some(term) = calendar::jie_qi(lunar) {
        return term.to_string();
    }
    day_label(lunar, style)
}
