//! Cell decoration: the reference day, the statutory calendar, and colour.
//!
//! A `cal` cell is marked for two independent reasons, and they compose:
//!
//! * the **reference day** — today, or whatever the local zone says — which is
//!   the cell the reader came for;
//! * the **statutory calendar** — 法定节假日 放假, and the 调休 workdays the
//!   State Council moves onto weekends (see [`crate::calendar::legal_holiday`]).
//!
//! A mark is a foreground or background attribute, never a character, so the
//! grid is as legible piped to a file as it is on a terminal: without colour
//! the 放假 days still read 放假 and the 调休 days still read [`WORK_LABEL`].
//! That is why the colour mode is only ever resolved once, by the caller, and
//! passed down as a plain `bool` — the grid itself holds no global state.

use std::fmt::Write as _;
use std::io::IsTerminal;

use crate::lang;

/// The label a 放假 cell shows when the statutory name is covered by a
/// traditional festival: a holiday without a name of its own.
pub const REST_LABEL: &str = "放假";

/// The label a 调休 cell shows. A weekend made a workday has no festival and
/// no solar term, so [`crate::cell::content`] keeps it whatever else the day
/// carries.
pub const WORK_LABEL: &str = "班";

/// SGR for a 放假 day: red text, the conventional colour of a holiday.
const REST_STYLE: &str = "31";

/// SGR for a 调休 day: bold and bright. The `班` label is only as rare as the
/// day itself, so it is drawn like any other highlighted cell.
const WORK_STYLE: &str = "1;93";

/// SGR for the reference day: inverse video, a solid block the eye finds in a
/// wall of text before it reads a single glyph.
const TODAY_STYLE: &str = "7";

/// The SGR that clears the attributes again.
const RESET: &str = "\x1b[0m";

/// How `--color` / `--no-color` resolve.
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

/// What a cell is marked with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mark {
    /// An ordinary day, drawn in the terminal's own colours.
    #[default]
    None,
    /// 法定节假日: a day off.
    Rest,
    /// 调休: a weekend the State Council turned into a workday.
    Work,
    /// The reference day.
    Today,
}

impl Mark {
    /// Whether the cell is painted rather than left plain.
    pub const fn is_tint(self) -> bool {
        !matches!(self, Self::None)
    }

    /// The SGR painting this mark.
    const fn sgr(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Rest => REST_STYLE,
            Self::Work => WORK_STYLE,
            Self::Today => TODAY_STYLE,
        }
    }
}

/// The SGR painting a cell, or `None` to leave it plain.
///
/// A cell showing [`WORK_LABEL`] paints even when its mark lost its argument
/// to a holiday on the same day — a 调休 Sunday that is also 春节 — because
/// `班` only reads as 调休 with the attribute behind it.
fn sgr_for(mark: Mark, text: &str) -> Option<&'static str> {
    match mark {
        Mark::None if text == WORK_LABEL => Some(WORK_STYLE),
        Mark::None => None,
        mark => Some(mark.sgr()),
    }
}

/// Appends one cell to `out`: `text` padded to `column` display columns,
/// painted when `color` is set and the mark says so.
///
/// The padding is inside the SGR run on purpose, so an inverse-video cell
/// fills its whole column instead of a single glyph.
pub fn paint(out: &mut String, mark: Mark, text: &str, column: usize, color: bool) {
    match sgr_for(mark, text) {
        Some(style) if color => {
            let _ = write!(out, "\x1b[{style}m");
            out.push_str(&lang::pad_right(text, column));
            out.push_str(RESET);
        }
        _ => out.push_str(&lang::pad_right(text, column)),
    }
}
