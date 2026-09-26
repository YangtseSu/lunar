//! The text a `cal` cell shows under (or over) a day number.
//!
//! Cell priority:
//!
//! ```text
//! 节日 > 初一显示月份名 > 节气 > 农历日
//! ```
//!
//! **The statutory calendar is not a level here.** 法定节假日 and 调休 are
//! painted onto the cell (see [`crate::mark`]) and never written into it: a
//! 放假 day reads 中秋节 and a 调休 workday reads 九月, because those are the
//! facts the calendar exists to show, and an administrative label in their
//! place would displace them. `CellStyle::holiday` therefore gates the
//! *marking*, not a level of this policy.

use lunar_rs::{Lunar, Solar};

use crate::calendar;

/// Which optional overlays are enabled.
///
#[derive(Debug, Clone, Copy)]
pub struct CellStyle {
    /// `--number`: show 农历日 as digits instead of 初一/廿六.
    pub number: bool,
    /// `--no-month-name`: never replace 初一 with the month name.
    pub month_name: bool,
    /// `--no-festival`: never show festivals.
    pub festival: bool,
    /// `--no-holiday`: never mark 法定节假日 days. It reaches the mark through
    /// [`crate::calgrid`], not through the content below.
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

/// The cell content of a day in a **lunar month** view.
///
/// The lunar grid leads with the civil date, so the month name is redundant
/// with the title and only the overlays apply: festival, else solar term, else
/// the lunar day. The civil overlay adds the month-name level through
/// [`content`].
pub fn lunar_month_content(solar: &Solar, lunar: &Lunar, style: CellStyle) -> String {
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

/// The full cell content: festival, else month name on 初一, else solar term,
/// else the lunar day.
pub fn content(solar: &Solar, lunar: &Lunar, style: CellStyle) -> String {
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
