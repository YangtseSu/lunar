// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Terminal display width, and the labels this crate owns.
//!
//! The tool serves Simplified Chinese only, so there is no language selection
//! here and no translation table. What does live here is the one measurement
//! the grid layout depends on: how many terminal columns a string occupies.
//! CJK ideographs and full-width punctuation take two, everything else one.
//! Padding by *character* count, as Rust's `{:>width$}` does, would misalign
//! every cell of a Chinese grid.
//!
//! Names the engine owns — weekdays, ganzhi, solar terms, festivals, month and
//! day names — come from `lunar-rs` and are used as it supplies them.

/// Terminal columns a string occupies.
///
/// A two-glyph Chinese label and a nine-letter English one then occupy the
/// same space, which is what keeps the grid's columns aligned.
pub fn width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

/// Columns one character occupies.
fn char_width(c: char) -> usize {
    let code = c as u32;
    // CJK ideographs, kana, Hangul, full-width forms and CJK punctuation.
    const WIDE: [std::ops::RangeInclusive<u32>; 6] = [
        0x1100..=0x115F,
        0x2E80..=0x303E,
        0x3041..=0x33FF,
        0x3400..=0x4DBF,
        0x4E00..=0x9FFF,
        0xAC00..=0xD7A3,
    ];
    const FULLWIDTH: std::ops::RangeInclusive<u32> = 0xFF00..=0xFF60;
    if WIDE.iter().any(|r| r.contains(&code)) || FULLWIDTH.contains(&code) {
        2
    } else {
        1
    }
}

/// Pads `text` on the right with spaces up to `columns` display columns.
///
/// Cells are left-aligned inside their column, so a short label keeps its
/// trailing blank where a long one has none.
pub fn pad_right(text: &str, columns: usize) -> String {
    let mut out = String::with_capacity(text.len() + columns);
    out.push_str(text);
    for _ in width(text)..columns {
        out.push(' ');
    }
    out
}
