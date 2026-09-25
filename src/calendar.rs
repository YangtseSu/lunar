//! Calendar core: the range lunar-rs can serve, solar/lunar conversion,
//! ganzhi, festivals and solar terms.
//!
//! Every calendar computation is delegated to the `lunar-rs` crate (ShouXing
//! astronomical engine). This module only shapes its output for the CLI.

use std::fmt;
use std::sync::Arc;

use lunar_rs::solar_util;
use lunar_rs::{Lunar, LunarMonth, LunarYear, Solar};

use crate::civil::CivilDate;

/// Smallest civil year the CLI accepts.
///
/// `lunar-rs` is engineered and documented for 公元 1–9999: its ShouXing
/// astronomy is constrained by the `LEAP_11` / `LEAP_12` tables, so outside
/// this window its extrapolated lunar months and solar terms are not meant to
/// be relied on. This is the widest range lunar-rs can actually serve.
pub const MIN_YEAR: i32 = 1;

/// Largest civil year the CLI accepts (see [`MIN_YEAR`]).
pub const MAX_YEAR: i32 = 9999;

/// Everything that can go wrong while resolving a date or a month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalError {
    /// A year outside the supported window was requested.
    YearOutOfRange { year: i32, min: i32, max: i32 },
    /// A year that the Gregorian calendar itself skips (1582: 10-05..=10-14).
    YearMissing { year: i32 },
    /// A month number outside `1..=12` (lunar months may be negative: `-4` is
    /// the leap fourth month).
    MonthOutOfRange { month: i32 },
    /// A day number outside `1..=31`.
    DayOutOfRange { day: i32 },
    /// The requested civil date does not exist, e.g. 2023-02-30.
    NonexistentDate { year: i32, month: i32, day: i32 },
    /// A lunar year that has no such month.
    NoSuchLunarMonth { year: i32, month: i32 },
    /// The selected lunar year has no leap month while `-R` was requested.
    NoLeapMonth { year: i32 },
    /// A `-d` string could not be parsed.
    UnparsableDate { input: String },
    /// The current local date is outside the supported window.
    TodayOutOfRange,
}

impl CalError {
    /// The user-facing message, in Simplified Chinese.
    pub fn message(&self) -> String {
        match self {
            Self::YearOutOfRange { year, min, max } => {
                format!("年份 {year} 超出支持范围 ({min}–{max})")
            }
            Self::YearMissing { year } => {
                format!("{year} 年 10 月 5 日至 14 日不存在（公历改革跳过的 10 天）")
            }
            Self::MonthOutOfRange { month } => format!("月份 {month} 非法 (应为 1–12)"),
            Self::DayOutOfRange { day } => format!("日期 {day} 非法 (应为 1–31)"),
            Self::NonexistentDate { year, month, day } => {
                format!("公历 {year}-{month:02}-{day:02} 不存在")
            }
            Self::NoSuchLunarMonth { year, month } => {
                format!("农历 {year} 年没有{}月", m_abs(*month))
            }
            Self::NoLeapMonth { year } => format!("农历 {year} 年没有闰月"),
            Self::UnparsableDate { input } => format!("无法解析的日期: {input}"),
            Self::TodayOutOfRange => "当前日期超出支持范围 (1–9999 年)".to_string(),
        }
    }
}

impl fmt::Display for CalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for CalError {}

/// Renders a possibly negative (leap) lunar month number in Chinese.
fn m_abs(month: i32) -> String {
    let index = month.unsigned_abs() as usize;
    let name = lunar_rs::lunar_util::MONTH[index];
    format!("{}{name}月", if month < 0 { "闰" } else { "" })
}

/// Checks a civil year against the supported window.
pub fn check_year(year: i32) -> Result<(), CalError> {
    if (MIN_YEAR..=MAX_YEAR).contains(&year) {
        Ok(())
    } else {
        Err(CalError::YearOutOfRange {
            year,
            min: MIN_YEAR,
            max: MAX_YEAR,
        })
    }
}

/// Checks a civil month (`1..=12`).
pub fn check_month(month: i32) -> Result<(), CalError> {
    if (1..=12).contains(&month) {
        Ok(())
    } else {
        Err(CalError::MonthOutOfRange { month })
    }
}

/// Checks a civil day (`1..=31`).
pub fn check_day(day: i32) -> Result<(), CalError> {
    if (1..=31).contains(&day) {
        Ok(())
    } else {
        Err(CalError::DayOutOfRange { day })
    }
}

/// Builds a [`Solar`] after range and existence checks.
pub fn solar(year: i32, month: i32, day: i32) -> Result<Solar, CalError> {
    check_year(year)?;
    check_month(month)?;
    check_day(day)?;
    Solar::from_ymd(year, month, day).map_err(|error| match error {
        // lunar-rs models the 1582 Gregorian reform, so 1582-10-05..=14 are
        // the one civil range it refuses.
        lunar_rs::LunarError::GregorianGap { .. } => CalError::YearMissing { year },
        _ => CalError::NonexistentDate { year, month, day },
    })
}

/// Chinese lunar month name: `正月`, `闰四月`, `腊月`.
pub fn lunar_month_name(month: i32) -> String {
    m_abs(month)
}

/// Chinese lunar day name: `初一` … `三十`.
pub fn lunar_day_name(day: i32) -> &'static str {
    lunar_rs::lunar_util::DAY[day as usize]
}

/// Weekday label: `日` … `六`.
pub fn weekday_name(weekday: i32) -> &'static str {
    solar_util::WEEK[weekday as usize]
}

/// Number of days in a civil month, honouring lunar-rs's 1582 reform model.
pub fn days_in_civil_month(year: i32, month: i32) -> i32 {
    solar_util::days_of_month(year, month)
}

/// GanZhi of the lunar year: `丙午`.
pub fn year_gan_zhi(lunar: &Lunar) -> String {
    lunar.year_in_gan_zhi()
}

/// GanZhi of the lunar **month** pillar.
///
/// A lunar almanac numbers months from 正月 (`寅`) onwards, which is the
/// traditional 月柱; it is independent of the solar-term instant rule
/// (`Lunar::month_in_gan_zhi`).
pub fn month_gan_zhi(lunar: &Lunar) -> String {
    lunar.month_in_gan_zhi_exact()
}

/// GanZhi of the day pillar: `甲申`.
pub fn day_gan_zhi(lunar: &Lunar) -> String {
    lunar.day_in_gan_zhi()
}

/// Chinese zodiac animal of the lunar year.
pub fn sheng_xiao(lunar: &Lunar) -> String {
    lunar.year_sheng_xiao().to_string()
}

/// The solar term falling exactly on this day, if any.
pub fn jie_qi(lunar: &Lunar) -> Option<String> {
    match lunar.jie_qi() {
        "" => None,
        name => Some(name.to_string()),
    }
}

/// How many blank cells precede day 1 of a civil month when a week starts on
/// `week_start` (0 = Sunday).
pub fn week_offset(year: i32, month: i32, week_start: i32) -> usize {
    (solar_util::week(year, month, 1) - week_start).rem_euclid(7) as usize
}

/// The festival a cell shows, or `None` when the day carries none.
///
/// Both calendars' festivals are consulted. `Lunar::festivals` knows the
/// lunar ones (春节, 中秋节) and `Solar::festivals` the civil ones (国庆节,
/// 劳动节) plus the floating ones keyed to a weekday (母亲节, 感恩节), so
/// asking only one of them silently drops the other half — an October grid
/// would show 廿一 where 国庆节 belongs.
///
/// The name is the engine's own, `中秋节` and not a shortened `中秋`; when a
/// day carries several, the one a reader recognises wins.
pub fn traditional_festivals(solar: &Solar, lunar: &Lunar) -> Option<String> {
    solar
        .festivals()
        .iter()
        .chain(lunar.festivals().iter())
        .min_by_key(|name| festival_rank(name))
        .map(|name| name.to_string())
}

/// Sort key for [`traditional_festivals`]: lower wins.
///
/// The festivals a reader scans a calendar for come first, lunar and civil
/// alike; everything else follows, so a busy day still shows its most
/// recognisable name.
fn festival_rank(name: &str) -> usize {
    const PRINCIPAL: [&str; 16] = [
        "春节",
        "元宵节",
        "清明",
        "端午",
        "七夕节",
        "中元节",
        "中秋节",
        "重阳节",
        "除夕",
        "元旦节",
        "劳动节",
        "国庆节",
        "儿童节",
        "青年节",
        "妇女节",
        "教师节",
    ];
    PRINCIPAL
        .iter()
        .position(|principal| *principal == name)
        .unwrap_or(usize::MAX)
}

/// A whole lunar year, for `cal -L <year>`.
pub fn lunar_year(year: i32) -> Result<Arc<LunarYear>, CalError> {
    check_year(year)?;
    Ok(LunarYear::from_year(year))
}
/// First civil day of a lunar month.
///
/// `LunarMonth::get_first_day` walks the lunar-new-year window, so it can hand
/// back a day belonging to the neighbouring lunar month — for 丙午年正月 it
/// reports 2026-01-01, which is 冬月十三. The real first day is the earliest
/// day whose lunar month matches the requested one.
pub fn lunar_month_start(month: &LunarMonth) -> CivilDate {
    let first = month.first_solar_day();
    let offset = if first.lunar().month() == month.month() {
        0
    } else {
        month.get_day_count()
    };
    let date = first.next_day(offset);
    CivilDate::new(date.year(), date.month(), date.day())
}

/// Looks up one month of a lunar year by its number (`-4` is the leap fourth
/// month). `LunarYear::get_month` walks the lunar-new-year window, so the
/// result is checked to belong to `year` before being returned.
pub fn lunar_year_month(year: &LunarYear, month: i32) -> Option<LunarMonth> {
    year.get_month(month)
        .filter(|candidate| candidate.year() == year.year())
}

/// The months of a lunar year that actually belong to it, in calendar order:
/// 正月 through 冬月/腊月, with the leap month in its place.
///
/// `LunarYear::months` deliberately spans the *lunar new year*, so it also
/// yields the tail of the previous lunar year and the head of the next one.
/// Those months are not part of the requested year, so they are filtered out.
pub fn lunar_year_months(year: &LunarYear) -> Vec<LunarMonth> {
    year.months()
        .into_iter()
        .filter(|month| month.year() == year.year())
        .collect()
}
