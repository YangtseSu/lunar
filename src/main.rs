//! `lunar` — a Chinese lunisolar calendar command line tool.
//!
//! Two subcommands, both powered by the `lunar-rs` calendar engine:
//!
//! * `lunar date` — one day's almanac profile
//!   (公历/星期/农历/干支/生肖/节气/法定/星座), with a custom format mode and
//!   a `-a` 黄历 block
//! * `lunar cal` — month and year grids with lunar days, solar terms and
//!   festivals, over civil months or, with `-L`, lunar months
//!
//! Supported years are 1–9999, the window `lunar-rs` can serve.

mod calendar;
mod calgrid;
mod cell;
mod civil;
mod commands;
mod datestr;
mod format;
mod lang;
mod mark;
mod tz;

use std::io::Write as _;
use std::process::ExitCode;

use clap::{ArgAction, Parser, Subcommand};

use crate::commands::{cal, date};

/// Token help, printed by `lunar date --help-format`.
const FORMAT_HELP: &str = concat!(
    "格式令牌(在 -f/--format 中):\n",
    "%Y 公历年   %m 公历月   %d 公历日   %A 星期几单字(一~日)\n",
    "%G 农历年干支(丙午)  %M 农历月汉字(正月/闰六月/腊月)  %N 农历日汉字(初一)\n",
    "%n 农历日数字(23)  %H 干支月(丙申)  %D 干支日(辛巳)\n",
    "%S 生肖(马)  %Q 节气(当日无则空)  %Z 星座(处女)  %% 字面%\n",
    "%A 只给单字，前缀自理: 星期%A=星期一 / 周%A=周一 / 礼拜%A=礼拜一\n",
    concat!(
        r"\n 换行  \t 制表符  \r 回车  \% 字面%(同 %%)  \\ 字面反斜杠",
        "\n",
    ),
    "例: lunar date -f '%G年%M%N，星期%A，%Q' -d 2026-09-07",
);

/// A Chinese lunisolar calendar CLI: `date`-style day profiles and `cal`-style
/// grids.
#[derive(Debug, Parser)]
#[command(name = "lunar", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// 某一天的农历档案：公历/星期/农历/干支/生肖/节气/法定/星座
    Date {
        /// 日期，date 风格，如 '2026-09-04'（缺省为今天）
        #[arg(short = 'd', long = "date", value_name = "DATE")]
        date: Option<String>,

        /// 自定义输出格式（令牌见 `lunar date --help-format`）
        #[arg(
            short = 'f',
            long = "format",
            value_name = "FORMAT",
            conflicts_with = "almanac"
        )]
        format: Option<String>,

        /// 追加黄历：宜忌/冲煞/神煞/星宿/纳音/方位/物候
        #[arg(short = 'a', long = "almanac")]
        almanac: bool,

        /// 位置参数按农历解读（`-d` 仍为公历）
        #[arg(short = 'l', long = "lunar")]
        lunar: bool,

        /// (-l 时)选择闰月
        #[arg(short = 'R', long = "leap", requires = "lunar")]
        leap: bool,

        /// 打印令牌说明
        #[arg(long = "help-format", action = ArgAction::SetTrue)]
        help_format: bool,

        /// 位置参数：年 月 日（或只给字符串日期）
        #[arg(value_name = "年 月 日")]
        positional: Vec<String>,
    },

    /// 公历月内叠加农历日/节气/节日/法定节假日，或单独显示农历月
    Cal {
        /// 单独显示农历月
        #[arg(short = 'L', long = "lunar")]
        lunar: bool,

        /// (-L 时)选择闰月
        #[arg(short = 'R', long = "leap", requires = "lunar")]
        leap: bool,

        /// 周日作为一周第一天
        #[arg(short = 's', long = "sunday", conflicts_with = "monday")]
        sunday: bool,

        /// 周一作为一周第一天（默认）
        #[arg(short = 'm', long = "monday", conflicts_with = "sunday")]
        monday: bool,

        /// 整年(缺省当前年)
        #[arg(short = 'y', long = "year")]
        year: bool,

        /// 显示连续 3 个月
        #[arg(short = '3', long = "three")]
        three: bool,

        /// 显示连续 N 个月
        #[arg(short = 'n', long = "months", value_name = "MONTHS", value_parser = clap::value_parser!(i32).range(1..))]
        months: Option<i32>,

        /// 农历日用数字(默认汉字)
        #[arg(long = "number")]
        number: bool,

        /// 关闭“初一显示为月份名”
        #[arg(long = "no-month-name")]
        no_month_name: bool,

        /// 关闭节日覆盖
        #[arg(long = "no-festival")]
        no_festival: bool,

        /// 关闭法定节假日(放假/调休)
        #[arg(long = "no-holiday")]
        no_holiday: bool,

        /// 着色方式：auto(默认)/always/never，后写者胜（取值须用 `=`）
        #[arg(
            long = "color",
            value_name = "WHEN",
            num_args = 0..=1,
            require_equals = true,
            default_missing_value = "always",
            value_parser = ["auto", "always", "never"],
            overrides_with = "no_color",
        )]
        color: Option<String>,

        /// 不着色（`--color=never` 的简写）
        #[arg(long = "no-color", overrides_with = "color")]
        no_color: bool,

        /// 位置参数：公历年/公历月，或 -L 农历年/农历月
        #[arg(value_name = "年 [月]")]
        positional: Vec<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // The token help needs no date and no zone, so it is answered before the
    // reference day is resolved: a machine whose `TZ` cannot be read would
    // otherwise fail on the one page that explains the tokens it is about to
    // be asked to expand.
    if let Command::Date { help_format, .. } = &cli.command
        && *help_format
    {
        println!("{FORMAT_HELP}");
        return ExitCode::SUCCESS;
    }

    let today = match tz::today() {
        Ok(today) => today,
        Err(error) => {
            eprintln!("lunar: {}", error.message());
            return ExitCode::FAILURE;
        }
    };

    let mut out = String::new();
    let result = match &cli.command {
        Command::Date {
            date,
            format,
            almanac,
            lunar,
            leap,
            help_format: _,
            positional,
        } => date::run(
            &date::DateArgs {
                date: date.clone(),
                format: format.clone(),
                almanac: *almanac,
                lunar: *lunar,
                leap: *leap,
                positional: positional.clone(),
            },
            today,
            &mut out,
        ),
        Command::Cal {
            lunar,
            leap,
            sunday,
            monday,
            year,
            three,
            months,
            number,
            no_month_name,
            no_festival,
            no_holiday,
            color,
            no_color,
            positional,
        } => {
            // `--color` and `--no-color` override each other, so only the one
            // written last survives parsing: `no_color` is true exactly when
            // it is the last of the two, and an absent `color` is `auto`.
            let color = match (color.as_deref(), no_color) {
                (_, true) => mark::Color::Never,
                (Some("never"), _) => mark::Color::Never,
                (Some("always"), _) => mark::Color::Always,
                _ => mark::Color::Auto,
            };
            cal::run(
                &cal::CalArgs {
                    positional: positional.clone(),
                    lunar: *lunar,
                    leap: *leap,
                    // `-m` is the default, so it only has to *clear* `-s`;
                    // clap rejects the two together.
                    sunday: *sunday && !monday,
                    year: *year,
                    three: *three,
                    months: *months,
                    number: *number,
                    no_month_name: *no_month_name,
                    no_festival: *no_festival,
                    no_holiday: *no_holiday,
                    color,
                },
                today,
                &mut out,
            )
        }
    };

    match result {
        Ok(()) => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            let _ = handle.write_all(out.as_bytes());
            let _ = handle.flush();
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("lunar: {}", error.message());
            ExitCode::FAILURE
        }
    }
}
