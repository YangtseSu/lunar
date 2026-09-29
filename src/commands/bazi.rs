// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

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
//! 八字: 庚午 / 壬午 / 辛亥 / 癸巳
//! 十神: 劫财 / 伤官 / 日主 / 食神
//! 藏干: 丁己 / 丁己 / 壬甲 / 丙庚戊
//! 纳音: 路旁土 / 杨柳木 / 钗钏金 / 长流水
//! 地势: 病 / 病 / 沐浴 / 死
//! 五行: 金火 / 水火 / 金水 / 水火
//! 旬空: 戌亥 / 申酉 / 寅卯 / 午未
//! 地支十神: 七杀偏印 / 七杀偏印 / 伤官正财 / 正官劫财正印
//! 命局: 胎元 癸酉(剑锋金) / 胎息 丙寅(炉中火) / 命宫 壬午(杨柳木) / 身宫 戊子(霹雳火)
//! 说明: 未给性别，无大运
//! ```
//!
//! With no time of day the 时柱 does not exist, and the tool prints three
//! pillars and says so rather than inventing one:
//!
//! ```text
//! $ lunar bazi 1990-06-15
//! 公历: 1990年6月15日
//! 农历: 庚午年五月廿三
//! 八字: 庚午 / 壬午 / 辛亥
//! 说明: 未给时刻，无时柱
//! 十神: 劫财 / 伤官 / 日主
//! 藏干: 丁己 / 丁己 / 壬甲
//! 纳音: 路旁土 / 杨柳木 / 钗钏金
//! 地势: 病 / 病 / 沐浴
//! 五行: 金火 / 水火 / 金水
//! 旬空: 戌亥 / 申酉 / 寅卯
//! 地支十神: 七杀偏印 / 七杀偏印 / 伤官正财
//! 命局: 胎元 癸酉(剑锋金) / 胎息 丙寅(炉中火)
//! 说明: 命宫 / 身宫需时柱
//! ```
//!
//! 命宫 and 身宫 are the second thing a chart cannot answer without a clock,
//! after the 时柱 itself: both are counted from the 時辰. The engine would
//! still return a pair for a birth moment given no time — it is handed a noon
//! to work from — but that is an answer about noon, and a chart that quietly
//! assumed noon would put a 05:00 birth's 命宫 on someone else's 時辰.
//!
//! 五行 is the two characters the 干 and the 支 each carry (`庚` 金, `午` 火,
//! so `金火`) and is not the 纳音's element, and 地支十神 is one 十神 per
//! 藏干 of the branch — a phrase, not a word.
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
//! instead of four.
//!
//! The years under those steps are a **query**, not more chart: `-y` names
//! one of them, because ten steps hold a hundred 流年 and a hundred 小运 and
//! printing all of them would bury the chart they belong to.
//!
//! ```text
//! $ lunar bazi 1990-06-15T10:30 -g 男 -y 2015 | tail -3
//! 流年: 2015年 乙未  26岁  旬空 辰巳
//! 流月: 戊寅(正月) / 己卯(二月) / 庚辰(三月) / 辛巳(四月) / 壬午(五月) / 癸未(六月) / 甲申(七月) / 乙酉(八月) / 丙戌(九月) / 丁亥(十月) / 戊子(冬月) / 己丑(腊月)
//! 小运: 己未 26岁
//! ```
//!
//! Two of the three need something the chart may not have, and both are
//! reported the way 命宫 / 身宫 are. The 流年 is counted on 大运, so no gender
//! means no 流年; and the 小运 is counted from the 时柱, one step per year, so
//! a birth moment with no time of day prints the 流年 and says the 小运 is
//! missing. The 流月 are named in 农历 months, because that is what they are.

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
    /// `-y`: the civil year whose 流年 / 流月 / 小运 are asked for.
    pub year: Option<String>,
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
    // `-y` is a **query**, not part of the chart: it names a year rather than
    // adding a row to the pillars, and without it a hundred 流年 and a hundred
    // 小运 would bury the chart they belong to.
    let year = match &args.year {
        Some(text) => Some(luck_year_arg(text)?),
        None => None,
    };
    write_chart(&solar, moment.seconds_of_day.is_some(), gender, year, out)
}

/// The civil year `-y` asks about, range-checked.
///
/// It is a year and not a birth moment, so the `-d` grammar is not its
/// reader: it is one integer, and a year the engine's 运程 arithmetic could
/// count forward into is refused rather than answered.
fn luck_year_arg(text: &str) -> Result<i32, CalError> {
    let year = text.trim().parse::<i32>().map_err(|_| CalError::BadYear {
        value: text.to_string(),
    })?;
    calendar::check_year(year)?;
    Ok(year)
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

/// Writes the chart: the pillar rows, then 命局, then 大运 when a gender was
/// given, then the 流年 of a year `-y` asked about.
///
/// The four sections are separate functions because they answer four
/// separate questions, and this function has lost a row twice while changing
/// one of them; `bazi_prints_four_pillars_only_with_a_time` pins the whole
/// chart, and it is the first thing to read when one goes missing.
fn write_chart(
    solar: &Solar,
    has_time: bool,
    gender: Option<Gender>,
    year: Option<i32>,
    out: &mut String,
) -> Result<(), CalError> {
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
    write_pillar_rows(&eight, has_time, out);
    write_ming_jun(&eight, has_time, out);
    write_yun(&eight, gender, out);
    write_luck_year(&eight, has_time, gender, year, out)?;
    Ok(())
}

/// How many columns a row about a pillar has: four with a 时柱, three without.
///
/// Every such row is cut here rather than row by row, so no row can keep a
/// fourth cell after the 时柱 is gone.
fn column_count(has_time: bool) -> usize {
    match has_time {
        true => 4,
        false => 3,
    }
}

/// Writes the rows that are about a pillar, one row per attribute, aligned to
/// the pillars they describe.
fn write_pillar_rows(eight: &EightChar, has_time: bool, out: &mut String) {
    let columns = column_count(has_time);
    let _ = writeln!(
        out,
        "八字: {}",
        [eight.year(), eight.month(), eight.day(), eight.time()][..columns].join(" / ")
    );
    if !has_time {
        let _ = writeln!(out, "说明: 未给时刻，无时柱");
    }
    for (label, cells) in [
        (
            "十神",
            [
                eight.year_shi_shen_gan().to_string(),
                eight.month_shi_shen_gan().to_string(),
                // The day stem is the 日主, so it has no 十神 of its own to name.
                "日主".to_string(),
                eight.time_shi_shen_gan().to_string(),
            ],
        ),
        (
            "藏干",
            [
                join_names(eight.year_hide_gan()),
                join_names(eight.month_hide_gan()),
                join_names(eight.day_hide_gan()),
                join_names(eight.time_hide_gan()),
            ],
        ),
        (
            "纳音",
            [
                eight.year_na_yin().to_string(),
                eight.month_na_yin().to_string(),
                eight.day_na_yin().to_string(),
                eight.time_na_yin().to_string(),
            ],
        ),
        (
            "地势",
            [
                eight.year_terrain().name().to_string(),
                eight.month_terrain().name().to_string(),
                eight.day_terrain().name().to_string(),
                eight.time_terrain().name().to_string(),
            ],
        ),
        (
            "五行",
            [
                eight.year_wu_xing(),
                eight.month_wu_xing(),
                eight.day_wu_xing(),
                eight.time_wu_xing(),
            ],
        ),
        (
            "旬空",
            [
                eight.year_xun_kong().to_string(),
                eight.month_xun_kong().to_string(),
                eight.day_xun_kong().to_string(),
                eight.time_xun_kong().to_string(),
            ],
        ),
        (
            "地支十神",
            [
                join_names(&eight.year_shi_shen_zhi()),
                join_names(&eight.month_shi_shen_zhi()),
                join_names(&eight.day_shi_shen_zhi()),
                join_names(&eight.time_shi_shen_zhi()),
            ],
        ),
    ] {
        let _ = writeln!(out, "{label}: {}", cells[..columns].join(" / "));
    }
}

/// Writes the 命局 row: the four values a traditional chart sets beside the
/// pillars, none of which belongs to one pillar.
///
/// 命宫 and 身宫 are counted from the 时柱, and a birth moment with no time of
/// day has none. The engine would still answer — from a noon it was handed —
/// and that noon is a value for noon, not an answer about the moment asked
/// about, so those two are left out and said to be, the same choice a missing
/// 时柱 makes about its own row.
fn write_ming_jun(eight: &EightChar, has_time: bool, out: &mut String) {
    let mut cells = vec![
        format!("胎元 {}({})", eight.tai_yuan(), eight.tai_yuan_na_yin()),
        format!("胎息 {}({})", eight.tai_xi(), eight.tai_xi_na_yin()),
    ];
    if has_time {
        cells.push(format!(
            "命宫 {}({})",
            eight.ming_gong(),
            eight.ming_gong_na_yin()
        ));
        cells.push(format!(
            "身宫 {}({})",
            eight.shen_gong(),
            eight.shen_gong_na_yin()
        ));
    }
    let _ = writeln!(out, "命局: {}", cells.join(" / "));
    if !has_time {
        let _ = writeln!(out, "说明: 命宫 / 身宫需时柱");
    }
}

/// Writes 起运 and 大运, or the line saying why there are none.
///
/// 顺逆 runs on the year stem's parity and the gender, and there is no default
/// to assume, so a chart without a gender prints the pillars and says what is
/// missing. A gender changes these two rows and nothing above them.
fn write_yun(eight: &EightChar, gender: Option<Gender>, out: &mut String) {
    match gender {
        None => {
            let _ = writeln!(out, "说明: 未给性别，无大运");
        }
        Some(gender) => {
            let yun = calendar::yun(eight, gender);
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

/// Writes the 流年 / 流月 / 小运 of the year `-y` asked about, or the line
/// saying why there is none.
///
/// Three things can be missing and each is reported rather than defaulted:
/// `-y` was not given (nothing is asked, so nothing prints), no gender (the
/// 流年 is counted on 大运, and 顺逆 has no default to assume), and a year no
/// step of 大运 covers — before 起运 or past the tenth step, which is as far
/// as a chart is排. The 小运 is the fourth case: it is counted from the 时柱,
/// one step per year, so a birth moment with no time of day has none, the
/// same choice 命宫 / 身宫 make.
///
/// The 流月 are named in 农历 months (`正月` … `冬月` `腊月`), because that is
/// what they are: a 流月 runs on 农历 months from 立春, so a civil `1月` here
/// would name a different month. The same words the 农历 line of a day
/// profile uses.
fn write_luck_year(
    eight: &EightChar,
    has_time: bool,
    gender: Option<Gender>,
    year: Option<i32>,
    out: &mut String,
) -> Result<(), CalError> {
    let Some(year) = year else {
        return Ok(());
    };
    let Some(gender) = gender else {
        // The 流年 is counted on 大运, and 顺逆 is the gender's to decide.
        let _ = writeln!(out, "说明: 流年需性别");
        return Ok(());
    };
    let yun = calendar::yun(eight, gender);
    let luck = calendar::luck_year(&yun, year, has_time)?;
    let Some(luck) = luck else {
        // Naming the span is the answer: "before 起运" and "past the tenth
        // step" are different facts, and the two numbers between them are
        // the whole of what the chart排 covers.
        let span = calendar::luck_span(&yun);
        let _ = writeln!(
            out,
            "说明: {year} 年无大运 (大运 {}-{})",
            span.start(),
            span.end()
        );
        return Ok(());
    };
    let _ = writeln!(
        out,
        "流年: {}年 {}  {}岁  旬空 {}",
        luck.year(),
        luck.gan_zhi(),
        luck.age(),
        luck.xun_kong()
    );
    let months: Vec<String> = luck
        .months()
        .iter()
        .map(|month| format!("{}({})", month.gan_zhi(), month.month()))
        .collect();
    let _ = writeln!(out, "流月: {}", months.join(" / "));
    match luck.xiao_yun() {
        Some(xiao) => {
            let _ = writeln!(out, "小运: {} {}岁", xiao.gan_zhi(), xiao.age());
        }
        None => {
            let _ = writeln!(out, "说明: 小运需时柱");
        }
    }
    Ok(())
}

/// One pillar's cell, for an attribute the engine lists as several
/// one-character names: the 藏干 of a branch (本气 first, then 中气 and 余气)
/// and the 十神 those 藏干 form. A cell is therefore two or three characters
/// wide, and the rows are separated by ` / ` rather than space-aligned.
fn join_names(names: &[&'static str]) -> String {
    names.iter().map(|name| name.to_string()).collect()
}
