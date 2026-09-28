//! `lunar bazi` — the four pillars of a birth moment.
//!
//! 八字 is a birth *chart*, not a day's profile: one of its four pillars is
//! the 时柱, so this is the one command in the tool that keeps a clock.
//! Everything else still answers for a day.
//!
//! ```text
//! $ lunar bazi 1990-06-15T10:30
//! 公历: 1990年6月15日 10:30
//! 农历: 庚午年五月廿三
//! 八字: 庚午 壬午 辛亥 癸巳
//! 十神: 劫财 伤官 日主 食神
//! 藏干: 丁己 / 丁己 / 壬甲 / 丙庚戊
//! 纳音: 路旁土 / 杨柳木 / 钗钏金 / 长流水
//! 地势: 病 / 病 / 沐浴 / 死
//! ```
//!
//! With no time of day the 时柱 does not exist, and the tool prints three
//! pillars and says so rather than inventing one:
//!
//! ```text
//! $ lunar bazi 1990-06-15
//! 公历: 1990年6月15日
//! 农历: 庚午年五月廿三
//! 八字: 庚午 壬午 辛亥
//! 说明: 未给时刻，无时柱
//! 十神: 劫财 伤官 日主
//! 藏干: 丁己 / 丁己 / 壬甲
//! 纳音: 路旁土 / 杨柳木 / 钗钏金
//! 地势: 病 / 病 / 沐浴
//! ```
//!
//! The 月柱 turns at the 節氣 **instant**, where the `干支` line of
//! `lunar date` turns at the 節氣 **day** — and both are right, because they
//! answer different questions. A chart is a birth, and 白露 at 22:41 puts a
//! birth at 20:00 in 申月 and one at 23:00 in 酉月; a day's almanac names the
//! whole day, so 2026-09-07 is 酉月 throughout. The two agree on every day
//! that is not a 節氣 day, which is all but twelve a year.
//!
//! The 年柱 and 日柱 do use the same basis as the `干支` line of
//! `lunar date` — the 立春 year, and a day pillar whose 子时 still belongs
//! to the day it began in — so those two pillars never disagree between the
//! two commands.
//!
//! 大运 comes with `-g`, because it runs forward or backward according to the
//! year stem's parity and the gender, and there is no default to assume:
//!
//! ```text
//! $ lunar bazi 1990-06-15T10:30 -g 男 | tail -2
//! 起运: 1997年11月17日  (出生后 7年5月2天12小时)  顺行
//! 大运: 8-17 癸未 / 18-27 甲申 / 28-37 乙酉 / 38-47 丙戌 / 48-57 丁亥 / 58-67 戊子 / 68-77 己丑 / 78-87 庚寅 / 88-97 辛卯
//! ```
//!
//! Without `-g` the four pillars still print, and a `说明` line says what is
//! missing — the same answer a birth moment with no clock gets, three pillars
//! instead of four. 流年 and 小运 are a layer below the steps and are not here.

use std::fmt::Write as _;

use lunar_rs::{EightChar, Gender, Solar};

use crate::calendar::{self, CalError};
use crate::civil::CivilDate;
use crate::datestr::{self, Moment};

/// Everything `lunar bazi` was asked to do.
#[derive(Debug, Clone)]
pub struct BaziArgs {
    /// `-g`: the gender that decides whether 大运 runs forward or back.
    pub gender: Option<String>,
    /// Positional: one `date(1)` style string, optionally with a time of day.
    pub positional: Vec<String>,
}

/// Runs `lunar bazi`, writing to `out`.
pub fn run(args: &BaziArgs, today: CivilDate, out: &mut String) -> Result<(), CalError> {
    let moment = resolve(args, today)?;
    let gender = match &args.gender {
        Some(text) => Some(gender(text)?),
        None => None,
    };
    let (hour, minute) = match moment.seconds_of_day {
        Some(seconds) => (seconds / 3600, (seconds % 3600) / 60),
        // No clock was written. Noon resolves the same three pillars the day
        // has, because the engine reads the day pillar from the day's own
        // noon and no 时辰 turns on the choice — the 时柱 is simply not
        // printed, so the substitution cannot reach the output.
        None => (12, 0),
    };
    let solar = calendar::solar_at(
        moment.day.year,
        moment.day.month,
        moment.day.day,
        hour,
        minute,
    )?;
    write_chart(&solar, moment.seconds_of_day.is_some(), gender, out);
    Ok(())
}

/// The gender names `-g` accepts, in the forms a user is likely to reach for.
///
/// 大运 runs forward for a yang year and a man and backward otherwise, so
/// there is no default to assume: a chart without it prints the four pillars
/// and says the 大运 are missing, which is the same choice a birth moment
/// with no time of day makes about its 时柱.
fn gender(text: &str) -> Result<Gender, CalError> {
    match text.trim() {
        "男" | "male" | "m" | "M" => Ok(Gender::Man),
        "女" | "female" | "f" | "F" => Ok(Gender::Woman),
        other => Err(CalError::BadGender {
            value: other.to_string(),
        }),
    }
}

/// Resolves the birth moment from the single positional argument.
///
/// A birth moment is a **civil** instant — there is no 农历 time of day, so
/// this command has no `-l`, and a lunar reading is not offered rather than
/// offered and refused. The grammar is still `datestr`'s, so every date form
/// `lunar date -d` takes is taken here too, with the same failure.
fn resolve(args: &BaziArgs, today: CivilDate) -> Result<Moment, CalError> {
    match args.positional.as_slice() {
        // Today, with whatever clock the reference day carries — and it
        // carries none, so a bare `lunar bazi` is today's three pillars.
        [] => Ok(Moment {
            day: today,
            seconds_of_day: None,
        }),
        [one] => datestr::parse_moment(one, today),
        // A fourth positional is a fourth, and `date` and `cal` both report
        // their own count rather than reading the first three and stopping.
        _ => Err(CalError::BadArgument {
            detail: String::new(),
        }),
    }
}

/// Writes the chart: the pillar rows, then 大运 when a gender was given.
fn write_chart(solar: &Solar, has_time: bool, gender: Option<Gender>, out: &mut String) {
    let (hour, minute) = (solar.hour(), solar.minute());
    let lunar = solar.lunar();
    let _ = writeln!(
        out,
        "公历: {}年{}月{}日{}",
        solar.year(),
        solar.month(),
        solar.day(),
        match has_time {
            true => format!(" {hour:02}:{minute:02}"),
            false => String::new(),
        }
    );
    let _ = writeln!(
        out,
        "农历: {}年{}{}",
        calendar::year_gan_zhi(&lunar),
        calendar::lunar_month_name(lunar.month()),
        calendar::lunar_day_name(lunar.day()),
    );

    let eight: EightChar = calendar::eight_char(&lunar);
    let mut pillars = vec![eight.year(), eight.month(), eight.day(), eight.time()];
    let mut shi_shen = vec![
        eight.year_shi_shen_gan().to_string(),
        eight.month_shi_shen_gan().to_string(),
        // The day stem is the 日主, so it has no 十神 of its own to name.
        "日主".to_string(),
        eight.time_shi_shen_gan().to_string(),
    ];
    let mut hide = vec![
        hide_gan(eight.year_hide_gan()),
        hide_gan(eight.month_hide_gan()),
        hide_gan(eight.day_hide_gan()),
        hide_gan(eight.time_hide_gan()),
    ];
    let mut na_yin = vec![
        eight.year_na_yin().to_string(),
        eight.month_na_yin().to_string(),
        eight.day_na_yin().to_string(),
        eight.time_na_yin().to_string(),
    ];
    let mut terrain = vec![
        eight.year_terrain().name().to_string(),
        eight.month_terrain().name().to_string(),
        eight.day_terrain().name().to_string(),
        eight.time_terrain().name().to_string(),
    ];

    if has_time {
        let _ = writeln!(out, "八字: {}", pillars.join(" / "));
    } else {
        pillars.truncate(3);
        shi_shen.truncate(3);
        hide.truncate(3);
        na_yin.truncate(3);
        terrain.truncate(3);
        let _ = writeln!(out, "八字: {}", pillars.join(" / "));
        let _ = writeln!(out, "说明: 未给时刻，无时柱");
    }
    let _ = writeln!(out, "十神: {}", shi_shen.join(" / "));
    let _ = writeln!(out, "藏干: {}", hide.join(" / "));
    let _ = writeln!(out, "纳音: {}", na_yin.join(" / "));
    let _ = writeln!(out, "地势: {}", terrain.join(" / "));
    match gender {
        None => {
            let _ = writeln!(out, "说明: 未给性别，无大运");
        }
        Some(gender) => {
            let yun = calendar::yun(&eight, gender);
            let start = yun.start_solar();
            // Both remainders the engine hands back, in the one field: the
            // month count alone drops up to 29 days, and `start_hour` is
            // always an even hour (0–22) standing for the hours left inside
            // the last day. Dropping either would print a number that quietly
            // disagrees with the date beside it.
            let _ = writeln!(
                out,
                "起运: {}年{}月{}日  (出生后 {}年{}月{}天{})  {}",
                start.year(),
                start.month(),
                start.day(),
                yun.start_year(),
                yun.start_month(),
                yun.start_day(),
                match yun.start_hour() {
                    0 => String::new(),
                    hours => format!("{hours}小时"),
                },
                match yun.is_forward() {
                    true => "顺行",
                    false => "逆行",
                }
            );
            // Ten steps, the engine's own span. No `-n`: how many steps to
            // show is a reading choice, not a different answer.
            let steps: Vec<String> = yun
                .da_yun()
                .iter()
                .filter(|d| d.index() >= 1)
                .map(|d| format!("{}-{} {}", d.start_age(), d.end_age(), d.gan_zhi()))
                .collect();
            let _ = writeln!(out, "大运: {}", steps.join(" / "));
        }
    }
}

/// The 藏干 of one pillar, as the engine lists them: 本气 first, then 中气
/// and 余气, which is why a pillar's cell is two or three characters wide and
/// the rows are separated by ` / ` rather than space-aligned.
fn hide_gan(gan: &[&'static str]) -> String {
    gan.iter()
        .map(|g| g.to_string())
        .collect::<Vec<String>>()
        .join("")
}
