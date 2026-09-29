// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Format token engine for `lunar date -f`.
//!
//! Tokens (as documented by `--help-format`):
//!
//! | token | meaning |
//! |-------|---------|
//! | `%Y`  | 公历年 (signed for years < 1) |
//! | `%m`  | 公历月, zero padded |
//! | `%d`  | 公历日, zero padded |
//! | `%A`  | 星期几 (一..日) |
//! | `%G`  | 农历年干支, 春节换年 (丙午) |
//! | `%M`  | 农历月 (正月 / 闰六月 / 腊月) |
//! | `%N`  | 农历日 (初一) |
//! | `%n`  | 农历日数字 (23) |
//! | `%H`  | 干支月, 节气当日换月 (丁酉) |
//! | `%D`  | 干支日 (辛巳) |
//! | `%S`  | 生肖 (马) |
//! | `%Q`  | 节气, empty when the day has none |
//! | `%Z`  | 星座 (处女) |
//! | `%%`  | literal `%` |
//!
//! `\` escapes the next character and yields it: `\n` a newline, `\t` a tab,
//! `\r` a carriage return, and `\%` (like `%%`) a literal `%`. `\\` is a
//! literal backslash. Every name the engine owns is rendered in the
//! Simplified Chinese the tool speaks.

use std::fmt::Write as _;

use lunar_rs::Solar;

use crate::calendar;

/// Expands `format` for `solar`, appending the result to `out`.
pub fn expand(format: &str, solar: Solar, out: &mut String) {
    let lunar = solar.lunar();
    let weekday = calendar::weekday_name(solar.week());
    let mut chars = format.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // A backslash escapes the next character: it yields that
            // character, and only `n`, `t` and `r` have a second meaning.
            // `\%` is one of those, and is why a literal `%` needs no `%%`:
            // the token is introduced by `%`, so a format that has to spell
            // the percent sign out can escape it here as well.
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some(other) => out.push(other),
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
                    'G' => out.push_str(&calendar::year_gan_zhi(&lunar)),
                    'M' => out.push_str(&calendar::lunar_month_name(lunar.month())),
                    'N' => out.push_str(calendar::lunar_day_name(lunar.day())),
                    'n' => {
                        let _ = write!(out, "{}", lunar.day());
                    }
                    'H' => out.push_str(&calendar::month_gan_zhi(&lunar)),
                    'D' => out.push_str(&calendar::day_gan_zhi(&lunar)),
                    'S' => out.push_str(&calendar::sheng_xiao(&lunar)),
                    'Q' => {
                        if let Some(term) = calendar::jie_qi(&lunar) {
                            out.push_str(&term);
                        }
                    }
                    'Z' => out.push_str(calendar::xing_zuo(&solar)),
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
