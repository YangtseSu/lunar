//! `lunar date` — one day's Chinese almanac profile.
//!
//! Without `-f` it prints the standard five-line day profile:
//!
//! ```text
//! 公历：2026年9月7日 星期一
//! 农历：丙午年七月廿六
//! 干支：丙午年 丙申月 甲申日
//! 生肖：马
//! 节气：白露
//! ```
//!
//! The `节气` line disappears when the day carries no solar term.

use lunar_rs::Solar;

use crate::calendar::{self, CalError};
use crate::civil::CivilDate;
use crate::datestr;
use crate::format;

use std::fmt::Write as _;

/// Everything `lunar date` was asked to do.
#[derive(Debug, Clone)]
pub struct DateArgs {
    /// `-d`: a `date(1)` style date string.
    pub date: Option<String>,
    /// `-f`: a custom format string.
    pub format: Option<String>,
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
    }
    Ok(())
}

/// Resolves the requested day from `-d` or the positional arguments.
fn resolve(args: &DateArgs, today: CivilDate) -> Result<CivilDate, CalError> {
    if let Some(date) = &args.date {
        return datestr::parse(date, today);
    }
    match args.positional.as_slice() {
        [] => Ok(today),
        [one] => datestr::parse(one, today),
        [year, month, day, ..] => {
            let year: i32 = number(year)?;
            let month: i32 = number(month)?;
            let day: i32 = number(day)?;
            calendar::check_year(year)?;
            calendar::check_month(month)?;
            calendar::check_day(day)?;
            let date = CivilDate::new(year, month, day);
            date.to_solar()?;
            Ok(date)
        }
        _ => Err(CalError::UnparsableDate {
            input: args.positional.join(" "),
        }),
    }
}

/// Parses a positional integer.
fn number(text: &str) -> Result<i32, CalError> {
    text.parse().map_err(|_| CalError::UnparsableDate {
        input: text.to_string(),
    })
}

/// The default five-line profile.
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
    let _ = writeln!(
        out,
        "干支: {} {} {}",
        calendar::year_gan_zhi(&lunar),
        calendar::month_gan_zhi(&lunar),
        calendar::day_gan_zhi(&lunar),
    );
    let _ = writeln!(out, "生肖: {}", calendar::sheng_xiao(&lunar));
    if let Some(term) = calendar::jie_qi(&lunar) {
        let _ = writeln!(out, "节气: {term}");
    }
}
