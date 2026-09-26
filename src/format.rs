//! Format token engine for `lunar date -f`.
//!
//! Tokens (as documented by `--help-format`):
//!
//! | token | meaning |
//! |-------|---------|
//! | `%Y`  | 公历年 (signed for years < 1) |
//! | `%m`  | 公历月, zero padded |
//! | `%d`  | 公历日, zero padded |
//! | `%A`  | 星期几 (一..日 / Mon..Sun) |
//! | `%G`  | 农历年干支 (丙午 / Bing Wu) |
//! | `%M`  | 农历月 (正月 / 闰六月 / 腊月) |
//! | `%N`  | 农历日 (初一) |
//! | `%n`  | 农历日数字 (23) |
//! | `%H`  | 干支月 (丙申) |
//! | `%D`  | 干支日 (辛巳) |
//! | `%S`  | 生肖 (马 / Horse) |
//! | `%Q`  | 节气, empty when the day has none |
//! | `%%`  | literal `%` |
//!
//! `\` escapes the next character and yields it: `\n` a newline, `\t` a tab,
//! `\r` a carriage return, and `\%` (like `%%`) a literal `%`. `\\` is a
//! literal backslash. Every name the engine owns is rendered in the chosen
//! language.

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
            // `\%` used to fall through to the catch-all and print both
            // characters, so the one escape a `%` needs — the token is
            // introduced by `%`, so a literal one cannot be written without
            // it — did not work. `%%` does the same job and always did.
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
