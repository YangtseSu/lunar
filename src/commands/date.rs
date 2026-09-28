//! `lunar date` — one day's Chinese almanac profile.
//!
//! Without `-f` it prints the standard day profile:
//!
//! ```text
//! 公历: 2026年9月7日 星期一
//! 农历: 丙午年七月廿六
//! 干支: 丙午 丁酉 甲申
//! 生肖: 马
//! 节气: 白露
//! 星座: 处女
//! ```
//!
//! The `节气` line disappears when the day carries no solar term, and a
//! `法定: 中秋节 放假` / `法定: 中秋节 调休上班` line appears only when the
//! day is on the statutory calendar. The `星座` line is last and never absent:
//! a constellation is a function of the civil month and day alone.
//!
//! The `农历` and `干支` lines carry two different year pillars on purpose:
//! the lunar year turns at 春节, the 干支 chain turns at 立春, and the month
//! pillar the `干支` line prints is the one the 黄历's 宜 / 忌 tables are
//! keyed on. See `calendar::li_chun_year_gan_zhi`.
//!
//! `-a` / `--almanac` appends the 黄历 block for the day — 宜忌, 冲煞, 神煞,
//! 星宿, 纳音, 方位 and 物候 — after the profile. It is refused with `-f`,
//! since the two are two shapes of the same answer.
//!
//! The day is a **civil** one by default; `-l` reads the same three positionals
//! as lunar instead, and `-R` picks the leap month, so `date -l 2020 4 1 -R`
//! is 2020-05-23.

use std::fmt::Write as _;

use lunar_rs::{Lunar, Solar};

use crate::calendar::{self, CalError};
use crate::civil::CivilDate;
use crate::datestr;
use crate::format;

/// Everything `lunar date` was asked to do.
#[derive(Debug, Clone)]
pub struct DateArgs {
    /// `-d`: a `date(1)` style date string.
    pub date: Option<String>,
    /// `-f`: a custom format string.
    pub format: Option<String>,
    /// `-a`: append the 黄历 block after the profile.
    pub almanac: bool,
    /// `-l`: read the positionals as a lunar date.
    pub lunar: bool,
    /// `-R`: with `-l`, select the leap month.
    pub leap: bool,
    /// Positional `年 月 日`, or a single date string.
    pub positional: Vec<String>,
}

/// Runs `lunar date`, writing to `out`; returns the process exit code.
pub fn run(args: &DateArgs, today: CivilDate, out: &mut String) -> Result<(), CalError> {
    let date = resolve(args, today)?;
    let solar = date.to_solar()?;

    if let Some(format) = &args.format {
        format::expand(format, solar, out);
        out.push('\n');
    } else {
        write_profile(solar, out);
        if args.almanac {
            write_almanac(&solar.lunar(), out);
        }
    }
    Ok(())
}

/// Resolves the requested day from `-d` or the positional arguments.
///
/// Both channels accept the same dates and differ only in the calendar they are
/// read in, so both go through [`datestr`]: a `-d` string and a single
/// positional through [`datestr::parse`], the three positionals through
/// [`datestr::from_parts`]. Sharing the reading is the point — a form one
/// channel accepts the other must accept too, with the same failure.
fn resolve(args: &DateArgs, today: CivilDate) -> Result<CivilDate, CalError> {
    let calendar = datestr::Calendar::Lunar { leap: args.leap };
    let calendar = match args.lunar {
        true => calendar,
        false => datestr::Calendar::Civil,
    };

    if let Some(date) = &args.date {
        return datestr::parse(date, today, calendar);
    }
    match args.positional.as_slice() {
        [] => Ok(today),
        [year, month, day] => {
            let year = number(year)?;
            let month = number(month)?;
            let day = number(day)?;
            datestr::from_parts(year, month, day, calendar)
        }
        // A single positional is a `date(1)` string, in whichever calendar
        // `-l` selected — `now`, `next friday`, `2026-07-15` all read the
        // same way in both.
        [one] => datestr::parse(one, today, calendar),
        // A fourth is a fourth, whatever the first three said. The arm used
        // to be `[year, month, day, ..]`, so `date 2026 1 1 extra` answered
        // 2026-01-01 and exited 0 — a typo read as agreement. `cal` reports
        // 位置参数过多 for the same mistake; `date` names its own count.
        _ => Err(CalError::BadArgument {
            detail: String::new(),
        }),
    }
}

/// Parses a positional integer.
fn number(text: &str) -> Result<i32, CalError> {
    text.parse().map_err(|_| CalError::UnparsableDate {
        input: text.to_string(),
    })
}

/// The default day profile.
fn write_profile(solar: Solar, out: &mut String) {
    let lunar = solar.lunar();
    let _ = writeln!(
        out,
        "公历: {}年{}月{}日 星期{}",
        solar.year(),
        solar.month(),
        solar.day(),
        calendar::weekday_name(solar.week()),
    );
    let _ = writeln!(
        out,
        "农历: {}年{}{}",
        calendar::year_gan_zhi(&lunar),
        calendar::lunar_month_name(lunar.month()),
        calendar::lunar_day_name(lunar.day()),
    );
    // The `干支` line is the 干支 chain, so all three of its pillars share
    // one basis: the year turns at 立春 here, not at 春节, because the month
    // pillar the line prints is the one the 黄历 keys 宜 / 忌 on and it
    // counts from 立春. The `农历:` line above is the lunar year itself and
    // keeps the 春节 basis.
    let _ = writeln!(
        out,
        "干支: {} {} {}",
        calendar::li_chun_year_gan_zhi(&lunar),
        calendar::month_gan_zhi(&lunar),
        calendar::day_gan_zhi(&lunar),
    );
    let _ = writeln!(out, "生肖: {}", calendar::sheng_xiao(&lunar));
    if let Some(term) = calendar::jie_qi(&lunar) {
        let _ = writeln!(out, "节气: {term}");
    }
    // The statutory calendar is the one overlay the profile reports even in
    // its default form: 放假 and 调休上班 change what the day *is* rather than
    // what it is called, and no other line of the profile says either.
    if let Some(line) = calendar::legal_holiday_line(&solar) {
        let _ = writeln!(out, "{line}");
    }
    // The constellation closes the profile: it is the only line that is
    // neither a calendar fact nor a statutory one, it is last so that nothing
    // conditional can push it around, and it is never absent — unlike `节气`
    // and `法定`, it has no day it fails to have.
    let _ = writeln!(out, "星座: {}", calendar::xing_zuo(&solar));
}

/// Appends the 黄历 block, one `label: text` line per group.
fn write_almanac(lunar: &Lunar, out: &mut String) {
    for text in calendar::almanac(lunar) {
        let _ = writeln!(out, "{text}");
    }
}
