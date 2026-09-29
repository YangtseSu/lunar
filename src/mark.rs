// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cell decoration: the reference day, the statutory calendar, and colour.
//!
//! A `cal` cell carries two independent marks, and **they compose**:
//!
//! * the **statutory calendar** — 法定节假日 放假 in red, and the 调休 workdays
//!   the State Council moves onto weekends in bold bright (see
//!   [`crate::calendar::legal_holiday`]);
//! * the **reference day** — today, or whatever the local zone says — in
//!   inverse video, the block the eye finds first.
//!
//! A day can be both. When today is a 放假 day the cell is red *and* inverted,
//! so neither fact is lost: the two SGR parameters are written in one run
//! (`\x1b[31;7m`) and the terminal applies both. A 调休 today reads bold, bright
//! and inverted. `Today` is therefore a flag alongside the statutory variant,
//! not a rival to it.
//!
//! **A mark is an attribute and nothing else.** The cell's text is the calendar
//! and festival layer alone, unmodified: a 调休 Saturday still reads 九月, a
//! 放假 day still reads 中秋节. Spelling the statutory entry out in the cell
//! would buy nothing the colour does not already say, and it would cost the
//! lunar day or the festival name — the two things a calendar is for.
//!
//! The consequence is deliberate: **a redirected or piped grid shows no mark
//! at all.** `cal --color > october.txt` keeps it, `cal > october.txt` does
//! not. For a *day*, `lunar date` still reports the statutory calendar in
//! words, because a line of text is the only surface it has.

use std::fmt::Write as _;
use std::io::IsTerminal;

use crate::lang;

/// SGR for a 放假 day: red text, the conventional colour of a holiday.
const REST_STYLE: &str = "31";

/// SGR for a 调休 day: bold and bright. A 调休 is a working day the State
/// Council moved onto a weekend, not a rare kind of day, so it takes the same
/// kind of highlight as any other marked cell.
const WORK_STYLE: &str = "1;93";

/// SGR for the reference day: inverse video, a solid block the eye finds in a
/// wall of text before it reads a single glyph.
const TODAY_STYLE: &str = "7";

/// The SGR that clears the attributes again.
const RESET: &str = "\x1b[0m";

/// How `--color[=WHEN]` / `--no-color` resolve, the `cal(1)` tri-state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    /// Colour when stdout is a terminal. The default.
    #[default]
    Auto,
    /// Colour even when stdout is not a terminal.
    Always,
    /// Never colour.
    Never,
}

impl Color {
    /// Whether SGR escapes are emitted at all.
    pub fn enabled(self) -> bool {
        match self {
            Self::Auto => std::io::stdout().is_terminal(),
            Self::Always => true,
            Self::Never => false,
        }
    }
}

/// What a cell is marked with: a statutory variant, plus the reference day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mark {
    /// The statutory calendar, when the day has an entry.
    pub holiday: Statutory,
    /// Whether this is the reference day.
    pub today: bool,
}

/// The statutory calendar's own answer for a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Statutory {
    /// An ordinary day: neither 放假 nor 调休.
    #[default]
    None,
    /// 法定节假日: a day off.
    Rest,
    /// 调休: a weekend the State Council turned into a workday.
    Work,
}

impl Mark {
    /// The SGR painting this mark, or `None` to leave the cell plain.
    ///
    /// The parameters are **joined, not chosen between**: a 放假 reference day
    /// is `\x1b[31;7m` — red and inverted at once. Nothing here is a text
    /// counterpart.
    pub fn sgr(self) -> Option<String> {
        let mut parameters = String::new();
        match self.holiday {
            Statutory::None => {}
            Statutory::Rest => parameters.push_str(REST_STYLE),
            Statutory::Work => parameters.push_str(WORK_STYLE),
        }
        if self.today {
            if !parameters.is_empty() {
                parameters.push(';');
            }
            parameters.push_str(TODAY_STYLE);
        }
        match parameters.is_empty() {
            true => None,
            false => Some(format!("\x1b[{parameters}m")),
        }
    }
}

/// Appends one cell to `out`: `text` padded to `column` display columns,
/// painted when `color` is set and the cell is marked.
///
/// The padding is inside the SGR run on purpose, so an inverse-video cell
/// fills its whole column instead of a single glyph.
pub fn paint(out: &mut String, mark: Mark, text: &str, column: usize, color: bool) {
    match color.then(|| mark.sgr()).flatten() {
        Some(style) => {
            let _ = write!(out, "{style}");
            out.push_str(&lang::pad_right(text, column));
            out.push_str(RESET);
        }
        None => out.push_str(&lang::pad_right(text, column)),
    }
}
