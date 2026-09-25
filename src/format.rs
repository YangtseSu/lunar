//! Format token engine for `lunar date -f`.
//!
//! Tokens (as documented by `--help-format`):
//!
//! | token | meaning |
//! |-------|---------|
//! | `%Y`  | 公历年 (signed for years < 1) |
//! | `%m`  | 公历月, zero padded |
//! | `%d`  | 公历日, zero padded |
//! | `%A`  | 星期几单字 (一..日) |
//! | `%G`  | 农历年干支 (丙午) |
//! | `%M`  | 农历月汉字 (正月 / 闰六月 / 腊月) |
//! | `%N`  | 农历日汉字 (初一) |
//! | `%n`  | 农历日数字 (23) |
//! | `%H`  | 干支月 (丙申) |
//! | `%D`  | 干支日 (辛巳) |
//! | `%S`  | 生肖 (马) |
//! | `%Q`  | 节气, empty when the day has none |
//! | `%%`  | literal `%` |
//!
//! `\` escapes the next character, so `\n` and `\t` produce a newline and a
//! tab, and `\%` a literal percent sign.

use std::fmt::Write as _;

use lunar_rs::Solar;
use lunar_rs::solar_util;

use crate::calendar;

/// Expands `format` for `solar`, appending the result to `out`.
pub fn expand(format: &str, solar: Solar, out: &mut String) {
    let lunar = solar.lunar();
    let weekday = solar_util::WEEK[solar.week() as usize];
    let mut chars = format.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            },
            '%' => {
                let Some(token) = chars.next() else {
                    out.push('%');
                    break;
                };
                match token {
                    'Y' => {
                        if solar.year() < 0 {
                            out.push('-');
                        }
                        let _ = write!(out, "{}", solar.year().unsigned_abs());
                    }
                    'm' => {
                        let _ = write!(out, "{:02}", solar.month());
                    }
                    'd' => {
                        let _ = write!(out, "{:02}", solar.day());
                    }
                    'A' => out.push_str(weekday),
                    'G' => out.push_str(&lunar.year_in_gan_zhi()),
                    'M' => out.push_str(&calendar::lunar_month_name(lunar.month())),
                    'N' => out.push_str(calendar::lunar_day_name(lunar.day())),
                    'n' => {
                        let _ = write!(out, "{}", lunar.day());
                    }
                    'H' => out.push_str(&lunar.month_in_gan_zhi_exact()),
                    'D' => out.push_str(&lunar.day_in_gan_zhi()),
                    'S' => out.push_str(lunar.year_sheng_xiao()),
                    'Q' => {
                        if let Some(term) = calendar::jie_qi(&lunar) {
                            out.push_str(term);
                        }
                    }
                    '%' => out.push('%'),
                    other => {
                        out.push('%');
                        out.push(other);
                    }
                }
            }
            other => out.push(other),
        }
    }
}
