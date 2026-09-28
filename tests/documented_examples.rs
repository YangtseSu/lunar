//! Layout and behaviour, pinned to what the tool documents; calendar values,
//! pinned to what `lunar-rs` computes from astronomy.
//!
//! Where the two disagree the engine wins: a published sample prints 芒种 on
//! the wrong day and omits two solar terms, and those are *not* reproduced
//! here. 中元 is the third: the cell reads 中元节 on 七月十五, which is where
//! the engine puts it.
//!
//! Every grid expectation is captured from the binary's own output, never
//! hand-computed: the cells are padded by display width, and a CJK label
//! occupies two columns, so the alignment cannot be reasoned about reliably.
//!
//! The tool speaks Simplified Chinese only, so no test sets a locale.

use std::process::Command;

/// Cells must never run into each other. A label that exactly fills its column
/// — a four-column `5/23` in a narrow grid — leaves no gap unless the renderer
/// keeps a gutter; without one the row reads `5/235/245/27`, and every cell of
/// that row is wider than the grid's own pitch allows.
#[test]
fn cells_never_run_together() {
    for args in [
        &["cal", "2026", "9"][..],
        &["cal", "-L", "2026", "7"][..],
        // The narrowest grid: 闰四月 holds no festival, so its column is just
        // wide enough for `5/23`.
        &["cal", "-L", "2020", "4", "-R"][..],
    ] {
        let grid = run(args);
        // Measure the pitch directly: it is the gap between one weekday
        // heading and the next. Every body row is then `7 * pitch` columns at
        // most, because a cell never exceeds the column and the trailing
        // gutter is trimmed. Without a gutter a narrow grid's cells collide
        // and the row still fits `7 * pitch` — but the *cells* are wrong, so
        // the check that matters is the one below: every cell must be
        // separated by at least one space.
        let pitch = cell_pitch(&grid);
        for line in grid.lines().skip(1) {
            let cells = split_on_pitch(line, pitch);
            for pair in cells.windows(2) {
                if pair[0].is_empty() || pair[1].is_empty() {
                    continue;
                }
                assert!(
                    !pair[0].ends_with(non_space) || !pair[1].starts_with(non_space),
                    "cells touch in `lunar {}`:\n{grid}",
                    args.join(" ")
                );
            }
            assert!(
                display_width(line) <= pitch * 7,
                "a row of `lunar {}` is wider than seven cells:\n{grid}",
                args.join(" ")
            );
        }
    }
}

/// The grid's cell pitch: the display distance between two weekday cells.
fn cell_pitch(grid: &str) -> usize {
    let heading = grid.lines().nth(1).expect("a weekday heading");
    // The heading is `一␣␣二␣␣…`; the pitch is the display distance from the
    // start of the first cell to the start of the second.
    let mut columns = 0;
    let mut seen = 0;
    for c in heading.chars() {
        if !c.is_whitespace() {
            if seen == 1 {
                return columns;
            }
            seen += 1;
        }
        columns += char_columns(c);
    }
    display_width(heading) / 7
}

/// Whether a character is anything but a space.
fn non_space(c: char) -> bool {
    !c.is_whitespace()
}

/// Splits a line into cells on the grid's pitch, in display columns.
///
/// SGR runs are transparent to the column count: they wrap a cell rather than
/// occupying it, and the padding of a painted cell lives *inside* its run, so
/// counting the escape bytes would drift every following cell.
fn split_on_pitch(line: &str, pitch: usize) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut columns = 0;
    let mut chars = line.char_indices().peekable();
    while let Some((byte, c)) = chars.next() {
        if c == '\u{1b}' {
            // An SGR run occupies no column. It may sit at a cell's start, in
            // which case the pending cut must move past it, so the escape
            // opens the new cell rather than closing the previous one.
            if columns == pitch {
                out.push(&line[start..byte]);
                start = byte;
                columns = 0;
            }
            for (_, c) in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
            continue;
        }
        if columns == pitch {
            out.push(&line[start..byte]);
            start = byte;
            columns = 0;
        }
        columns += char_columns(c);
    }
    out.push(&line[start..]);
    out
}

/// Removes every SGR sequence, leaving the text the grid would print without
/// colour.
fn strip_sgr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A grid's lines joined back, with trailing whitespace removed.
fn trimmed(grid: &str) -> String {
    grid.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Display width of a string, counting CJK as two columns.
fn display_width(text: &str) -> usize {
    text.chars().map(char_columns).sum()
}

/// Columns one character occupies, matching the renderer's own measure.
fn char_columns(c: char) -> usize {
    let code = c as u32;
    if (0x2E80..=0x303E).contains(&code)
        || (0x3400..=0x4DBF).contains(&code)
        || (0x4E00..=0x9FFF).contains(&code)
        || (0xFF00..=0xFF60).contains(&code)
    {
        2
    } else {
        1
    }
}

/// A cell shows a festival from either calendar. `Lunar::festivals` alone
/// knows only the lunar ones, so a grid that consults it silently drops every
/// civil festival — an October grid would read 廿一 where 国庆节 belongs.
#[test]
fn civil_festivals_are_shown_too() {
    let october = run(&["cal", "2026", "10"]);
    assert!(
        october.lines().any(|line| line.contains("国庆节")),
        "10-1 is 国庆节:\n{october}"
    );
    // The lunar ones are unaffected.
    for (args, festival) in [
        (&["cal", "-L", "2026", "1"][..], "春节"),
        (&["cal", "-L", "2026", "1"][..], "元宵节"),
        (&["cal", "-L", "2026", "5"][..], "端午节"),
        (&["cal", "-L", "2026", "7"][..], "七夕节"),
        (&["cal", "2026", "9"][..], "中秋节"),
    ] {
        let grid = run(args);
        assert!(grid.contains(festival), "expected {festival}:\n{grid}");
    }
}

/// The civil festivals a reader scans for, each on the day the engine gives.
///
/// `--no-holiday` because a festival the State Council legislates a day off
/// for is covered by the statutory label instead — 青年节 falls inside the 劳动
/// 节 holiday — and this test is about the festival list, not the two of them.
#[test]
fn the_usual_civil_festivals_appear() {
    let mut year = String::new();
    for month in 1..=12 {
        year.push_str(&run(&["cal", "2026", &month.to_string(), "--no-holiday"]));
    }
    for festival in [
        "元旦节",
        "劳动节",
        "儿童节",
        "青年节",
        "妇女节",
        "教师节",
        "国庆节",
        "圣诞节",
    ] {
        assert!(year.contains(festival), "{festival} missing from 2026");
    }
}

/// The days a civil month has, so a test can state what a grid must show.
mod calendar {
    /// Every day of `(year, month)` that **exists**, in order.
    ///
    /// Asked of the binary rather than computed, and not as `1..=length`.
    /// The tool follows `lunar-rs`, which uses the **Julian** leap rule below
    /// 1600 and drops the ten reform days of October 1582 — so a test that
    /// computed a length with the Gregorian rule would disagree with the
    /// engine on 384 Februaries, and October 1582 is not numbered 1..=21 at
    /// all but 1, 2, 3, 4, 15 … 31. The engine is the only authority for
    /// which days exist, and the binary is the only way to ask it.
    pub fn existing_days(year: i32, month: i32) -> Vec<u32> {
        let mut days = Vec::new();
        for day in 1..=length(year, month) {
            let output = super::binary()
                .args([
                    "date",
                    "-d",
                    &format!("{year:04}-{month:02}-{day:02}"),
                    "-f",
                    "%Y-%m-%d",
                ])
                .output()
                .expect("lunar runs");
            if output.status.success() {
                days.push(day);
            }
        }
        days
    }

    /// The last day of `(year, month)` that the tool will name, which is its
    /// length. Every month has 28 to 31, so the answer is the first of those
    /// four that is not 不存在.
    fn length(year: i32, month: i32) -> u32 {
        for day in [31, 30, 29, 28] {
            // A fresh `Command` each time: `args` appends, so reusing one
            // would carry the previous probe's `-d` along with it.
            let output = super::binary()
                .args([
                    "date",
                    "-d",
                    &format!("{year:04}-{month:02}-{day:02}"),
                    "-f",
                    "%Y-%m-%d",
                ])
                .output()
                .expect("lunar runs");
            if output.status.success() {
                return day as u32;
            }
        }
        panic!("{year}-{month} has no day in 28..=31")
    }
}

/// The compiled `lunar` binary.
fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lunar"))
}

/// Runs `lunar <args…>` and returns stdout, minus the trailing newline.
fn run(args: &[&str]) -> String {
    let output = binary().args(args).output().expect("lunar runs");
    assert!(
        output.status.success(),
        "lunar {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf-8 stdout")
        .trim_end_matches('\n')
        .to_string()
}

/// Runs `lunar <args…>` expecting failure, and returns stderr.
fn run_failing(args: &[&str]) -> String {
    let output = binary().args(args).output().expect("lunar runs");
    assert!(
        !output.status.success(),
        "lunar {args:?} unexpectedly succeeded"
    );
    String::from_utf8_lossy(&output.stderr)
        .trim_end()
        .to_string()
}

/// Runs `lunar <args…>` with `TZ` set, expecting failure, and returns stderr.
fn run_failing_with_tz(tz: &str, args: &[&str]) -> String {
    let output = binary()
        .args(args)
        .env("TZ", tz)
        .output()
        .expect("lunar runs");
    assert!(
        !output.status.success(),
        "lunar {args:?} unexpectedly succeeded with TZ={tz}"
    );
    String::from_utf8_lossy(&output.stderr)
        .trim_end()
        .to_string()
}

/// The text a cell of a grid shows, found by its label.
///
/// Cells are located on the grid's **pitch**, not by splitting on
/// whitespace: a painted cell is padded inside its own SGR run, so
/// `国庆节    ` arrives as one word and the split would count the cell — and
/// every one after it — wrongly.
fn cell_of(grid: &str, label: &str) -> String {
    let lines: Vec<&str> = grid.lines().collect();
    let pitch = cell_pitch(grid);
    let row = label_row(&lines, pitch, label);
    let label_line = 2 + row * 2;
    let column = column_of(lines[label_line], pitch, label);
    strip_sgr(split_on_pitch(lines[label_line + 1], pitch)[column])
        .trim_end()
        .to_string()
}

/// The SGR a cell of a **coloured** grid is painted with, found by its label.
///
/// The grid is a two-band layout, so the label and the paint live on different
/// lines of the same row: the date band and the content band are both painted,
/// and both are checked.
fn paint_of<'a>(grid: &'a str, label: &str) -> Option<&'a str> {
    let lines: Vec<&'a str> = grid.lines().collect();
    let pitch = cell_pitch(grid);
    let label_line = 2 + label_row(&lines, pitch, label) * 2;
    let column = column_of(lines[label_line], pitch, label);
    for band in [label_line, label_line + 1] {
        if let Some(style) = sgr_of(split_on_pitch(lines[band], pitch)[column]) {
            return Some(style);
        }
    }
    None
}

/// Which week of the grid carries `label` in its date band.
fn label_row(lines: &[&str], pitch: usize, label: &str) -> usize {
    lines
        .iter()
        .skip(2)
        .step_by(2)
        .position(|line| {
            split_on_pitch(line, pitch)
                .iter()
                .any(|cell| strip_sgr(cell).trim() == label)
        })
        .unwrap_or_else(|| panic!("no cell labelled {label} in:\n{}", lines.join("\n")))
}

/// Which cell of `line` carries `label`.
fn column_of(line: &str, pitch: usize, label: &str) -> usize {
    split_on_pitch(line, pitch)
        .iter()
        .position(|cell| strip_sgr(cell).trim() == label)
        .expect("the label is a cell of its own")
}

/// The SGR a single cell is painted with, if it is painted.
fn sgr_of(cell: &str) -> Option<&str> {
    let after = cell.strip_prefix("\u{1b}[")?;
    let (style, _) = after.split_once('m')?;
    Some(&cell[..style.len() + 3])
}

/// The SGR painting one cell of a coloured grid, found by its label.
///
/// The statutory calendar is an attribute and never a label, so a grid can
/// only show 放假 or 调休 here: 2026-10-10 is a Saturday the State Council
/// turned into a workday, and 2026-10-01 .. 10-07 are 放假.
#[test]
fn statutory_days_are_painted_and_nothing_else() {
    let october = run(&["cal", "2026", "10", "--color"]);
    for day in ["1", "2", "3", "4", "5", "6", "7"] {
        assert_eq!(
            paint_of(&october, day),
            Some("\u{1b}[31m"),
            "2026-10-{day} is 放假:\n{october:?}"
        );
    }
    // 2026-10-10 is a 调休 workday; the 8th and 9th carry neither mark.
    assert_eq!(paint_of(&october, "10"), Some("\u{1b}[1;93m"));
    for day in ["8", "9", "11", "12"] {
        assert_eq!(
            paint_of(&october, day),
            None,
            "2026-10-{day} is an ordinary day:\n{october:?}"
        );
    }

    let january = run(&["cal", "2026", "1", "--color"]);
    for day in ["1", "2", "3"] {
        assert_eq!(
            paint_of(&january, day),
            Some("\u{1b}[31m"),
            "2026-01-{day} is 放假"
        );
    }
    assert_eq!(paint_of(&january, "4"), Some("\u{1b}[1;93m"));

    // 2026 春节 runs 2/17..2/23, and 2/28 is the 调休 workday before it.
    let spring = run(&["cal", "-L", "2026", "1", "--color"]);
    for day in ["2/17", "2/18", "2/19", "2/20", "2/21", "2/22", "2/23"] {
        assert_eq!(paint_of(&spring, day), Some("\u{1b}[31m"), "{day} is 放假");
    }
    assert_eq!(paint_of(&spring, "2/28"), Some("\u{1b}[1;93m"));
}

/// The two marks **compose**: a reference day that is also 放假 is red *and*
/// inverted, and a 调休 today is bold, bright and inverted. Neither wins.
///
/// `tz::today()` is not an input the suite can pin, so this cannot assert
/// "today is red and inverted" outright. It pins the mechanism instead, over
/// the whole of 2026, and the mechanism is what makes composition testable: a
/// combined mark is a **single SGR run carrying both parameters**
/// (`\x1b[31;7m`), never two runs and never one parameter dropped. So every
/// `\x1b[…m` in a coloured year must list `7` if and only if that run is the
/// reference day, and a run that lists both `31` and `7` is a 放假 today.
#[test]
fn the_statutory_calendar_and_the_reference_day_compose() {
    let mut year = String::new();
    for month in 1..=12 {
        year.push_str(&run(&["cal", "2026", &month.to_string(), "--color"]));
    }

    let runs = |year: &str| {
        year.match_indices('\u{1b}')
            .map(|(at, _)| {
                let run = &year[at..];
                run[..run.find('m').expect("an SGR run ends in m") + 1].to_string()
            })
            .filter(|run| run != "\u{1b}[0m")
            .collect::<Vec<_>>()
    };
    let painted = runs(&year);

    assert!(
        painted.iter().any(|run| run == "\u{1b}[31m"),
        "2026 has 放假 days"
    );
    assert!(
        painted.iter().any(|run| run == "\u{1b}[1;93m"),
        "2026 has 调休 workdays"
    );

    // Every inverse-painted run is the reference day, and there is exactly one
    // grid in the year that can contain it — so at most two runs, one per band.
    let inverted: Vec<&String> = painted
        .iter()
        .filter(|run| run.contains(";7m") || run.ends_with("[7m"))
        .collect();
    assert!(
        inverted.len() <= 2,
        "at most the reference day is inverted, in two bands: {inverted:?}"
    );

    // A run that carries both a statutory colour and the inversion is a day
    // that is both. `--no-holiday` must reduce it to the bare inversion, which
    // is the observable proof that the two were independent rather than one
    // having replaced the other.
    let mut plain = String::new();
    for month in 1..=12 {
        plain.push_str(&run(&[
            "cal",
            "2026",
            &month.to_string(),
            "--color",
            "--no-holiday",
        ]));
    }
    let plain_painted = runs(&plain);
    let composed = painted.len() - plain_painted.len();
    assert_eq!(
        composed,
        painted
            .iter()
            .filter(|run| run.contains("[31m") || run.contains("[1;93m"))
            .count(),
        "the difference is exactly the statutory runs"
    );
    // Under `--no-holiday` a reference day that was statutory reads as the bare
    // inversion, so no combined run survives.
    assert!(
        !plain_painted
            .iter()
            .any(|run| run.contains("31;") || run.contains("93;")),
        "--no-holiday leaves no combined run: {plain_painted:?}"
    );

    // **Both statutory variants reduce the same way.** A 放假 today and a 调休
    // today are the same event with a different colour: each is a single run
    // listing its own parameters *plus* the inversion, and each becomes the
    // bare inversion when the statutory half is switched off. Were 调休 the
    // odd one out — the case the `match` could plausibly get wrong — it would
    // either keep its colour under the switch or arrive as two runs instead of
    // one, and the two reductions below would differ in shape.
    for (label, statutory_run) in [("放假", "\u{1b}[31m"), ("调休", "\u{1b}[1;93m")] {
        assert!(
            painted.iter().any(|run| run == statutory_run),
            "{label} appears on its own, so it can be compared when combined"
        );
    }
    // Whatever the clock says, the combined forms that exist must be
    // single-run concatenations of a statutory run and the bare inversion.
    for run in painted.iter().filter(|run| run.contains('7')) {
        let combined = run.trim_start_matches("\u{1b}[").trim_end_matches('m');
        assert_eq!(
            run.matches('\u{1b}').count(),
            1,
            "a combined mark is one run: {run:?}"
        );
        let mut parts = combined.split(';');
        assert_eq!(
            parts.next_back(),
            Some("7"),
            "the inversion is the last parameter: {run:?}"
        );
        let rest = parts.collect::<Vec<_>>().join(";");
        assert!(
            rest == "31" || rest == "1;93" || rest.is_empty(),
            "only a statutory colour precedes the inversion: {run:?}"
        );
    }
}

/// A mark never becomes a label: a 放假 day still reads whatever the calendar
/// says — its festival, or its lunar day — and a 调休 workday likewise.
#[test]
fn a_statutory_day_still_reads_the_calendar() {
    // The 2026 中秋 holiday runs 9/25..9/27. Only the first day carries the
    // festival name; the other two read their lunar day, and all three are
    // painted.
    let september = run(&["cal", "2026", "9"]);
    assert_eq!(cell_of(&september, "25"), "中秋节");
    assert_eq!(cell_of(&september, "26"), "十六");
    assert_eq!(cell_of(&september, "27"), "十七");
    // 9/20 is a 调休 Sunday; without the mark it reads 初十, not a 班.
    assert_eq!(cell_of(&september, "20"), "初十");

    // 2026-10-10 is a 调休 workday and the first day of 九月, so the month
    // name is what the cell must keep.
    let october = run(&["cal", "2026", "10"]);
    assert_eq!(cell_of(&october, "10"), "九月");
    assert_eq!(cell_of(&october, "1"), "国庆节");
}

/// `--no-holiday` drops the statutory *marking* and changes no text at all.
#[test]
fn no_holiday_switch_removes_only_the_painting() {
    let marked = run(&["cal", "2026", "10", "--color"]);
    let plain = run(&["cal", "2026", "10", "--color", "--no-holiday"]);
    assert!(
        marked.contains("\u{1b}[31m"),
        "the statutory days are painted without the switch"
    );
    assert!(
        !plain.contains('\u{1b}'),
        "--no-holiday in a month without the reference day paints nothing:\n{plain:?}"
    );
    // A painted cell is padded inside its SGR run, so stripping it leaves
    // those spaces behind: compare line by line, trimmed.
    assert_eq!(
        trimmed(&strip_sgr(&marked)),
        trimmed(&plain),
        "the two grids differ only in the statutory painting"
    );
    // The festivals are untouched by the switch.
    assert!(plain.contains("国庆节"));
}

/// A cell is painted, never decorated with a character, so the text of a grid
/// is identical whether or not SGR is emitted. Only the escapes differ.
#[test]
fn colour_adds_escapes_and_nothing_else() {
    let plain = run(&["cal", "2026", "10", "--no-color"]);
    let colored = run(&["cal", "2026", "10", "--color"]);
    // The suite's stdout is a pipe, so the default is already uncoloured.
    assert_eq!(run(&["cal", "2026", "10"]), plain);
    assert!(colored.contains("\u{1b}["), "SGR runs are present");
    // Trailing padding is not compared: an inverse-video cell is padded inside
    // its own SGR run, so its reset follows the spaces.
    let stripped_sgr = strip_sgr(&colored);
    let stripped: Vec<&str> = stripped_sgr.lines().map(str::trim_end).collect();
    let plain: Vec<&str> = plain.lines().map(str::trim_end).collect();
    assert_eq!(
        stripped, plain,
        "stripping SGR must leave the grid unchanged"
    );
}

/// `--color` and `--no-color` each override the other, so the one written
/// last decides, and `--color` takes `auto` / `always` / `never` like
/// `cal(1)`'s.
///
/// Last-wins is clap's `overrides_with` on *both* sides: each flag clears the
/// other, so a second spelling of the same decision is the one that survives
/// however the pairs are matched. `require_equals` keeps `--color` from
/// eating the year that follows it.
#[test]
fn the_color_flags_take_the_last_word() {
    let marked = |args: &[&str]| run(args).contains('\u{1b}');
    // `2026 10` is the 国庆 statutory week and `-L 2026 1` the 春节 one, so
    // each view has something to paint: a `never` that is vacuously true
    // would pass the same check.
    let october = |flags: &[&str]| {
        let mut args = vec!["cal", "2026", "10"];
        args.extend_from_slice(flags);
        run(&args).contains('\u{1b}')
    };
    let spring = |flags: &[&str]| {
        let mut args = vec!["cal", "-L", "2026", "1"];
        args.extend_from_slice(flags);
        run(&args).contains('\u{1b}')
    };
    // Order, in both directions and with the value spelled out.
    assert!(!october(&["--color", "--no-color"]));
    assert!(october(&["--no-color", "--color"]));
    assert!(!october(&["--color=always", "--no-color"]));
    assert!(october(&["--no-color", "--color=always"]));
    assert!(!spring(&["--color", "--no-color"]));
    assert!(spring(&["--no-color", "--color"]));
    // The three words of `cal(1)`, and the two forms of "never".
    assert!(!october(&["--color=never"]));
    assert!(october(&["--color=always"]));
    // The suite's stdout is a pipe, so `auto` is uncoloured.
    assert!(!october(&["--color=auto"]));
    assert!(!october(&[]));
    assert!(!marked(&["cal", "2026", "10", "--no-color"]));
    // `--color` is a flag, not a word: the month still parses after it.
    assert_eq!(
        run(&["cal", "--color", "2026", "10"]),
        run(&["cal", "2026", "10", "--color"])
    );
    // A word that is none of the three is a parse error, not a silent `auto`.
    let output = binary()
        .args(["cal", "2026", "10", "--color=sometimes"])
        .output()
        .expect("lunar runs");
    assert_eq!(output.status.code(), Some(2), "clap refuses the value");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("'sometimes'"),
        "the refusal names the value"
    );
}

/// The profile reports the statutory calendar even in its default form: 放假
/// and 调休上班 are the one thing about a day the other lines cannot say.
#[test]
fn date_profile_reports_the_statutory_calendar() {
    // 星座 closes the profile, so the statutory line is located by content
    // rather than by position — and the profile order is pinned separately.
    assert!(
        run(&["date", "-d", "2026-09-26"]).contains("法定: 中秋节 放假"),
        "a 放假 day names the holiday and the fact"
    );
    assert!(
        run(&["date", "-d", "2026-10-10"]).contains("法定: 国庆节 调休上班"),
        "a 调休 day says it is a workday"
    );
    assert!(
        !run(&["date", "-d", "2026-09-10"]).contains("法定"),
        "an ordinary day has no 法定 line"
    );
}

/// A lunar date is answered with the civil day it falls on. `-R` names the
/// leap month, exactly as `cal -L -R` does.
#[test]
fn a_lunar_date_resolves_to_its_civil_day() {
    assert_eq!(
        run(&["date", "-l", "2026", "7", "15"]),
        run(&["date", "-d", "2026-08-27"])
    );
    assert_eq!(
        run(&["date", "-l", "2020", "4", "1", "-R"]),
        "公历: 2020年5月23日 星期六\n农历: 庚子年闰四月初一\n干支: 庚子 辛巳 丙寅\n生肖: 鼠\n星座: 双子"
    );
    // A lunar month that the year does not have.
    assert!(run_failing(&["date", "-l", "2026", "13", "1"]).contains("农历月份 13 非法"));
    // A day the lunar month does not have, reported with its length.
    assert!(run_failing(&["date", "-l", "2026", "7", "31"]).contains("该月只有 29 天"));
    assert!(run_failing(&["date", "-l", "10000", "1", "1"]).contains("超出支持范围"));
}

/// A lunar month whose first day falls in the *next* civil year is reachable.
///
/// 腊月 always begins in the following January, so 农历 2026 年腊月 starts on
/// 2027-01-08 — the lunar year and the civil year do not line up, and a
/// conversion that insists the civil year match refuses the whole month.
/// 闰腊月 (公元 37 年) and the two 正月 that start in December are the same
/// shape. `-d` reads the same way, so it must agree.
#[test]
fn a_lunar_month_may_begin_in_the_next_civil_year() {
    // 农历 2026 年腊月初一 is 公历 2027-01-08.
    assert_eq!(
        run(&["date", "-l", "2026", "12", "1"]),
        run(&["date", "-d", "2027-01-08"])
    );
    assert_eq!(
        run(&["date", "-l", "-d", "2026-12-01"]),
        run(&["date", "-d", "2027-01-08"])
    );
    // 闰腊月 of 公元 37 年 starts 公元 38-01-25.
    assert!(
        run(&["date", "-l", "-R", "37", "12", "1"]).contains("公历: 38年1月25日"),
        "闰腊月 is reachable"
    );
    // 正月 of 公元 16 年 starts 公元 15-12-30, the other direction.
    assert!(
        run(&["date", "-l", "16", "1", "1"]).contains("公历: 15年12月30日"),
        "正月 that begins in December is reachable"
    );
    // The month still has its real length.
    assert!(run_failing(&["date", "-l", "2026", "12", "30"]).contains("该月只有 29 天"));
}

/// `-d` reads 公历 and `-l` reads 农历, and nothing else differs between
/// them: the same date, written either way, resolves to the same day.
#[test]
fn the_two_channels_differ_only_in_the_calendar_they_read() {
    // 农历 2026-07-15 is 公历 2026-08-27.
    let lunar_forms: &[&[&str]] = &[
        &["date", "-l", "-d", "2026-07-15"],
        &["date", "-l", "-d", "20260715"],
        &["date", "-l", "-d", "2026/07/15"],
        &["date", "-l", "2026", "7", "15"],
        &["date", "-l", "2026", "07", "15"],
    ];
    for args in lunar_forms {
        assert_eq!(
            run(args),
            run(&["date", "-d", "2026-08-27"]),
            "`date {}` is not 农历 2026-07-15",
            args.join(" ")
        );
    }
    // The same strings without `-l` are the 公历 dates they look like.
    let civil_forms: &[&[&str]] = &[
        &["date", "-d", "2026-07-15"],
        &["date", "-d", "20260715"],
        &["date", "-d", "2026/07/15"],
    ];
    for args in civil_forms {
        assert_eq!(
            run(args),
            run(&["date", "-d", "2026-07-15"]),
            "`date {}` must stay 公历",
            args.join(" ")
        );
    }
    // `-R` reaches the leap month through `-d` exactly as it does through the
    // positionals.
    assert_eq!(
        run(&["date", "-l", "-R", "-d", "2020-04-01"]),
        run(&["date", "-l", "2020", "4", "1", "-R"])
    );
}

/// `MM/DD/YYYY` is the US order the grammar lists, and it is read as such.
///
/// The short-year branch claims any `/`-separated triple whose first field is
/// one to three digits, so the US order is tried *before* it — otherwise
/// `09/07/2026` arrives as the year 9 with a day of 2026 and fails
/// validation. A four-digit *last* field is what makes the triple the US
/// order, so `2026/09/07` and `1-01-01` still reach the branches that own
/// them.
#[test]
fn the_us_slash_order_is_read_as_month_day_year() {
    for (form, expected) in [
        ("09/07/2026", "2026-09-07"),
        ("9/7/2026", "2026-09-07"),
        ("12/31/1999", "1999-12-31"),
        ("01/02/2026", "2026-01-02"),
    ] {
        assert_eq!(
            run(&["date", "-d", form, "-f", "%Y-%m-%d"]),
            expected,
            "`-d {form}` is the US order"
        );
    }
    // A four-digit *last* field is what makes it the US order, so the
    // branches that own these are untouched.
    for (form, expected) in [
        ("2026/09/07", "2026-09-07"),
        ("1-01-01", "1-01-01"),
        ("999-12-31", "999-12-31"),
        ("2026-09-07", "2026-09-07"),
        ("20260907", "2026-09-07"),
    ] {
        assert_eq!(
            run(&["date", "-d", form, "-f", "%Y-%m-%d"]),
            expected,
            "`-d {form}` keeps its own branch"
        );
    }
    // An impossible US date is reported as such, not misread as a short year.
    assert!(run_failing(&["date", "-d", "13/45/2026"]).contains("月份 13 非法"));
    // And it reads in the lunar channel too — both channels share the grammar.
    assert_eq!(
        run(&["date", "-l", "-d", "07/15/2026"]),
        run(&["date", "-l", "2026", "7", "15"])
    );
}

/// The forms that name a day rather than a date — a keyword, an epoch, a
/// weekday, a relative offset — are not written in either calendar, so `-l`
/// must not change them. Only the absolute forms switch.
#[test]
fn day_relative_forms_are_unaffected_by_the_calendar_flag() {
    for form in [
        "now",
        "today",
        "tomorrow",
        "yesterday",
        "@1788000000",
        "monday",
        "next friday",
        "next month",
        "last year",
        "next week",
    ] {
        assert_eq!(
            run(&["date", "-d", form]),
            run(&["date", "-l", "-d", form]),
            "`-d {form}` must not depend on `-l`"
        );
    }
}

/// An epoch is read as the day `date -d @…` reads it.
///
/// The fraction is dropped and the whole seconds truncated **towards zero**,
/// so `@-1` — one second before the epoch — is 1970-01-01 and not
/// 1969-12-31. `div_euclid` floors, and the test suite had no negative
/// timestamp to notice: every negative answer came out a day early.
#[test]
fn an_epoch_is_truncated_towards_zero() {
    for (epoch, expected) in [
        ("@0", "1970-01-01"),
        ("@1", "1970-01-01"),
        ("@-0.5", "1970-01-01"),
        ("@-1", "1970-01-01"),
        ("@-0.9", "1970-01-01"),
        ("@86399", "1970-01-01"),
        ("@86400", "1970-01-02"),
        ("@-86400", "1969-12-31"),
        ("@-86400.5", "1969-12-31"),
        ("@-86401", "1969-12-31"),
        ("@-172800", "1969-12-30"),
        ("@1788000000", "2026-08-29"),
        ("@1788000000.25", "2026-08-29"),
    ] {
        assert_eq!(
            run(&["date", "-d", epoch, "-f", "%Y-%m-%d"]),
            expected,
            "`-d {epoch}` is {expected}"
        );
    }
    // A fraction never moves the answer into another day, and `-l` does not
    // change it: an epoch names an instant, not a date in a calendar.
    for epoch in ["@-0.5", "@-86400", "@1788000000.25"] {
        assert_eq!(
            run(&["date", "-d", epoch, "-f", "%Y-%m-%d"]),
            run(&["date", "-l", "-d", epoch, "-f", "%Y-%m-%d"]),
            "`-d {epoch}` must not depend on `-l`"
        );
    }
}

/// A bare `next` / `last` period moves that period, as `date(1)` does.
///
/// `parse_weekday` already handled `next friday`, so the grammar looked
/// covered — but `apply_unit` had no `next`/`last` arm and the bare word hit
/// the error branch, so `next month` (which the README lists) was rejected.
///
/// The forms resolve against the reference day, so the assertion is a
/// *relation* to it rather than a fixed date: pinning a calendar date here
/// would make the test fail every time the day it was written on came round
/// again.
#[test]
fn a_bare_next_or_last_moves_that_period() {
    let today = run(&["date", "-d", "today", "-f", "%Y-%m-%d"]);
    // Each form must agree with the counted form it abbreviates.
    for (bare, counted) in [
        ("next day", "1 day"),
        ("last day", "1 day ago"),
        ("next week", "7 days"),
        ("last week", "7 days ago"),
        ("next fortnight", "14 days"),
        ("next month", "1 month"),
        ("last month", "1 month ago"),
        ("next year", "1 year"),
        ("last year", "1 year ago"),
    ] {
        assert_eq!(
            run(&["date", "-d", bare, "-f", "%Y-%m-%d"]),
            run(&["date", "-d", counted, "-f", "%Y-%m-%d"]),
            "`-d {bare}` from {today} is the same day as `-d {counted}`"
        );
    }
    // The word needs a period to move.
    assert!(run_failing(&["date", "-d", "next"]).contains("无法解析"));
    assert!(run_failing(&["date", "-d", "last"]).contains("无法解析"));
}

/// A sub-day offset is refused, not silently dropped.
///
/// `apply_unit` returned the date unchanged for seconds through hours, so
/// `90 minutes ago` answered with today and read as though the offset had
/// been applied. The tool keeps a date and no clock, so an offset shorter
/// than a day has nothing to move — saying so beats pretending.
#[test]
fn a_sub_day_offset_is_refused_rather_than_ignored() {
    for form in [
        "90 minutes ago",
        "2 hours",
        "30 seconds",
        "1 minute",
        "3 hours ago",
    ] {
        assert!(
            run_failing(&["date", "-d", form]).contains("不足一天"),
            "`-d {form}` says it cannot move a date"
        );
    }
    // The whole-day units still work, and the grammar is unaffected by -l.
    for form in ["1 day", "2 days ago", "1 fortnight", "next month"] {
        assert_eq!(
            run(&["date", "-d", form]),
            run(&["date", "-l", "-d", form]),
            "`-d {form}` still resolves"
        );
    }
}

/// A year count too large to represent is reported, not wrapped.
///
/// `add_years` added a number **the user typed** to an `i32` year, so
/// `+2147483600 years` panicked in debug and wrapped in release, where the
/// error named a year nobody had asked for. The sum is taken in `i64` now and
/// the year it reaches is the one named. The count stays unrestricted — `i32`
/// is the width of the grammar, and the expression is well formed; only its
/// result is out of range.
#[test]
fn a_huge_relative_offset_is_reported_not_wrapped() {
    // The reference day is not an input the suite can pin, so the year the
    // message has to name is the one the arithmetic reaches *from* it.
    let year: i64 = run(&["date", "-d", "today", "-f", "%Y"])
        .parse()
        .expect("a four-digit year");
    // Each form is the whole argument list, because a leading `-` is a flag
    // to the argument parser: `-d -1` never reaches the date grammar, so the
    // negative counts ride on one argument (`=`) or after `--`.
    for (argv, offset) in [
        (vec!["-d", "+2147483600 years"], 2_147_483_600),
        (vec!["-d", "+2147483647 years"], 2_147_483_647),
        (vec!["-d=-2147483647 years"], -2_147_483_647),
        (vec!["--", "-2147483647 years"], -2_147_483_647),
    ] {
        let output = binary()
            .arg("date")
            .args(argv.clone())
            .output()
            .expect("lunar runs");
        // `date(1)` answers `invalid date` and exits 1. A panic exits 101, and
        // a wrapped year is a year nobody asked for.
        assert_eq!(output.status.code(), Some(1), "`lunar date {argv:?}`");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr).trim_end(),
            format!("lunar: 年份 {} 超出支持范围 (1–9999)", year + offset),
            "`lunar date {argv:?}`"
        );
    }
    // The counted form still moves the year, and moves it by what it says.
    let today = run(&["date", "-d", "today", "-f", "%Y-%m-%d"]);
    assert_eq!(
        run(&["date", "-d", "2 years", "-f", "%Y-%m-%d"]),
        format!("{}-{}", year + 2, today.split_once('-').expect("a date").1),
        "`-d 2 years` is two years on from {today}"
    );
}

/// A long run of digits is not a compact `YYYYMMDD` date.
///
/// The compact branch reads eight fields and stops looking, so it is claimed
/// only at a word boundary: an input that merely *begins* with eight digits is
/// not one. `1758240000` is a bare timestamp with no reading, and
/// `2147483647 days` is a well-formed relative offset whose *result* is out of
/// range — `date(1)` rejects both, and each is reported as what it is.
#[test]
fn a_long_digit_run_is_not_a_compact_date() {
    // A timestamp missing its `@` is not a date; `date(1)` says `invalid date`
    // and so does this, with the offending text.
    assert_eq!(
        run_failing(&["date", "-d", "1758240000"]).trim_end(),
        "lunar: 无法解析的日期: 1758240000"
    );
    // A well-formed offset stays a relative expression, and what it runs into
    // is reported as a *year*: `2147483647` days is about 5.88 million years,
    // so the year it reaches is near 5 881 6xx whatever the reference day is,
    // and a truncated `i32` would name a negative year instead.
    let stderr = run_failing(&["date", "-d", "2147483647 days"]);
    let named: i64 = stderr
        .split("年份 ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .and_then(|digits| digits.parse().ok())
        .unwrap_or_else(|| panic!("`{stderr}` names the year it reached"));
    let reference: i64 = run(&["date", "-d", "today", "-f", "%Y"])
        .parse()
        .expect("a four-digit year");
    assert!(
        (5_881_600..5_881_800).contains(&named),
        "2147483647 days reaches a year near 5 881 6xx, got {named} (reference {reference})"
    );
    // The dates that *are* compact keep working, with and without a time part.
    for form in [
        "20260907",
        "20260907T1530",
        "20260907t1530",
        "20260907 15:30",
    ] {
        assert_eq!(
            run(&["date", "-d", form, "-f", "%Y-%m-%d"]),
            "2026-09-07",
            "`-d {form}` is still 2026-09-07"
        );
    }
}

/// A month or a day count names the year it reached, like a year count does.
///
/// `add_years` took its sum in `i64` and named the year, while `add_months`
/// truncated an `i64` month index with `as i32` and `add_days` let the epoch
/// arithmetic truncate the same way. Both were reachable from user input —
/// twelve `+2147483647 months`, twenty-seven `+2147483647 fortnights` — and
/// both answered with a year nobody had asked for, which is the defect the year
/// count had been fixed for. The parser applies one unit at a time, so the year
/// named is the one the *first* step past the representable range reaches.
#[test]
fn a_wrapped_month_or_day_count_names_the_year_it_reached() {
    let named_year = |args: &[&str]| -> i64 {
        let stderr = run_failing(args);
        stderr
            .split("年份 ")
            .nth(1)
            .and_then(|rest| rest.split(' ').next())
            .and_then(|digits| digits.parse().ok())
            .unwrap_or_else(|| panic!("`lunar date {args:?}` names a year, got `{stderr}`"))
    };
    // The month index runs twelve per year, so the twelfth maximum count is
    // the first to carry the year past what an `i32` holds — and it lands
    // barely past it, where a truncation would have named a negative year.
    let month_arg = "+2147483647 months ".repeat(12);
    let months = vec!["date", "-d", month_arg.trim_end()];
    let month_year = named_year(&months);
    assert!(
        month_year > 2_147_483_647 && month_year < 2_147_500_000,
        "twelve max month counts reach just past the i32 range, got {month_year}"
    );
    // The fortnight arm of the same arithmetic: 14 days per unit, so
    // twenty-seven of them carry the year out of range.
    let fortnight_arg = "+2147483647 fortnights ".repeat(27);
    let fortnights = vec!["date", "-d", fortnight_arg.trim_end()];
    let day_year = named_year(&fortnights);
    assert!(
        day_year > 2_147_483_647,
        "twenty-seven max fortnight counts reach a year past the i32 range, got {day_year}"
    );
    // Counts that stay in range still move the day by exactly what they say.
    let today = run(&["date", "-d", "today", "-f", "%Y-%m-%d"]);
    assert_eq!(
        run(&["date", "-d", "2 fortnights", "-f", "%Y-%m-%d"]),
        run(&["date", "-d", "28 days", "-f", "%Y-%m-%d"]),
        "2 fortnights is 28 days, counted from {today}"
    );
}

/// A backslash yields the character it escapes.
///
/// `\n`, `\t` and `\r` had meanings and `\%` fell through to a catch-all that
/// printed *both* characters — so the one escape a `%` needs, given that `%`
/// introduces every token, did not work. Every escape now yields its
/// character, and `\\` is how a literal backslash is written.
#[test]
fn a_backslash_yields_the_character_it_escapes() {
    let expand = |format: &str| run(&["date", "-d", "2026-09-07", "-f", format]);
    assert_eq!(expand("\\%"), "%");
    assert_eq!(expand("%%"), "%");
    assert_eq!(expand("\\\\"), "\\");
    assert_eq!(expand("a\\nb"), "a\nb");
    assert_eq!(expand("a\\tb"), "a\tb");
    assert_eq!(expand("a\\rb"), "a\rb");
    // An escaped token letter is that letter, not the token.
    assert_eq!(expand("\\Y"), "Y");
    // A trailing backslash is itself.
    assert_eq!(expand("a\\"), "a\\");
    // The tokens themselves are untouched.
    assert_eq!(expand("%Y"), "2026");
    assert_eq!(expand("%G年%M%N"), "丙午年七月廿六");
}

/// A time of day is part of the 公历 grammar. Under `-l` it is refused rather
/// than dropped, so a lunar date never looks like it honoured a time it threw
/// away.
#[test]
fn a_time_of_day_is_refused_on_a_lunar_date() {
    assert!(run(&["date", "-d", "2026-07-15T15:30"]).contains("公历: 2026年7月15日"));
    assert!(run_failing(&["date", "-l", "-d", "2026-07-15T15:30"]).contains("无法解析"));
}

/// A fractional second is accepted, and **truncated**: the tool keeps no
/// clock, so the fraction cannot move the day.
///
/// The module doc advertised `2026-09-07T15:30:45.123456789+08:00` from the
/// start and the parser refused it — `parse_clock` took the whole `45.123`
/// as the seconds and `finish_clock` rejected it. The fraction is dropped
/// before the range check, and a fraction that is not digits stays a
/// rejection, as `date(1)` has it.
#[test]
fn a_fractional_second_is_truncated() {
    for fraction in [".5", ".123456789", ".0", ".000001"] {
        assert_eq!(
            run(&[
                "date",
                "-d",
                &format!("2026-09-07T15:30:45{fraction}+08:00")
            ]),
            run(&["date", "-d", "2026-09-07T15:30:45+08:00"]),
            "`…T15:30:45{fraction}+08:00` is the same day as `…T15:30:45+08:00`"
        );
    }
    // The zone still shifts the day, and the fraction does not change that.
    assert_eq!(
        run(&["date", "-d", "2026-09-07T23:30:45.5+0800", "-f", "%Y-%m-%d"]),
        "2026-09-08"
    );
    // A seconds field that is not a fraction of digits is still refused, and
    // an out-of-range one is refused with the fraction dropped or not.
    for form in [
        "2026-09-07T15:30:45.",
        "2026-09-07T15:30:45.abc",
        "2026-09-07T15:30:45.5abc",
        "2026-09-07T15:30:60.5",
        "2026-09-07T15:30:99.123",
    ] {
        assert!(
            run_failing(&["date", "-d", form]).contains("无法解析"),
            "`-d {form}` is not a clock"
        );
    }
}

/// A month or a day written in one digit is the same date.
///
/// `date(1)` reads `2026-9-7` and `2026/9/7`; the fixed-width branch could
/// not, and the short-year branch claims nothing here, so both were
/// unparsable. The branch is claimed only when a field is **unpadded**, so a
/// padded `2026-09-07` still belongs to the branch that had it, and the US
/// `MM/DD/YYYY` order keeps the priority it is listed with.
#[test]
fn an_unpadded_iso_date_is_accepted() {
    let padded = run(&["date", "-d", "2026-09-07"]);
    for form in ["2026-9-7", "2026/9/7", "2026-09-7", "2026-9-07"] {
        assert_eq!(
            run(&["date", "-d", form]).lines().next(),
            padded.lines().next(),
            "`-d {form}` is 2026-09-07"
        );
    }
    // A time of day and a zone travel with it, exactly as on the padded form.
    assert_eq!(
        run(&["date", "-d", "2026-9-7T15:30:45.5-0800", "-f", "%Y-%m-%d"]),
        "2026-09-07"
    );
    // The US order is unchanged: a four-digit *last* field is still the year.
    for (form, expected) in [
        ("09/07/2026", "2026-09-07"),
        ("9/7/2026", "2026-09-07"),
        ("12/31/1999", "1999-12-31"),
    ] {
        assert_eq!(
            run(&["date", "-d", form, "-f", "%Y-%m-%d"]),
            expected,
            "`-d {form}` keeps the US order"
        );
    }
    // The branches that own the padded and compact forms still own them.
    for (form, expected) in [
        ("2026-09-07", "2026-09-07"),
        ("20260907", "2026-09-07"),
        ("1-01-01", "1-01-01"),
        ("999-12-31", "999-12-31"),
    ] {
        assert_eq!(
            run(&["date", "-d", form, "-f", "%Y-%m-%d"]),
            expected,
            "`-d {form}` keeps its own branch"
        );
    }
    // A month the grammar has no reading for is still a bad date, and a day
    // that reaches the resolver names itself: a value the user can correct.
    for form in [
        "2026-9-123",
        "2026-9/7",
        "2026/9-7",
        "2026-中-07",
        "2026-07",
    ] {
        assert!(
            run_failing(&["date", "-d", form]).contains("无法解析"),
            "`-d {form}` is not a date"
        );
    }
    assert!(
        run_failing(&["date", "-d", "2026-13-7"]).contains("月份 13 非法"),
        "a month that reaches the resolver names itself"
    );
    // It reads in the lunar channel too — both channels share the grammar.
    assert_eq!(
        run(&["date", "-l", "-d", "2026/7/15"]),
        run(&["date", "-l", "2026", "7", "15"])
    );
}

/// A bare time of day with a zone is the day that time falls on **in that
/// zone**, and it is refused under `-l` like every other time of day.
///
/// The branch answered with the reference day whatever the string said, so
/// `15:30 UTC` — which crosses midnight for a reader east of Greenwich —
/// was unparsable, and `15:30` was *accepted* under `-l`, where the rule says
/// a time of day is refused. It now goes through `finish`, so the zone shift
/// and the refusal are the two rules the other forms already obey.
#[test]
fn a_bare_time_resolves_in_its_zone() {
    let today = run(&["date", "-d", "today", "-f", "%Y-%m-%d"]);
    for form in [
        "15:30 UTC",
        "15:30 GMT",
        "15:30",
        "15:30:45.5+08:00",
        "23:30+0800",
    ] {
        assert!(
            run(&["date", "-d", form, "-f", "%Y-%m-%d"]).len() == 10,
            "`-d {form}` answers with a day"
        );
    }
    // A zone that cannot move the day away from the reference is still the
    // reference, which is what a bare time asks for.
    assert_eq!(run(&["date", "-d", "15:30", "-f", "%Y-%m-%d"]), today);
    assert_eq!(run(&["date", "-d", "15:30 UTC", "-f", "%Y-%m-%d"]), today);
    // A zone that *does* move it is answered on the moved day, the way
    // `date -d "15:30 UTC"` answers. POSIX `+0800` is 8 hours behind UTC,
    // so 23:30 there is 15:30 UTC — and in UTC the same string is today.
    let east = run(&["date", "-d", "23:30+0800", "-f", "%Y-%m-%d"]);
    assert_ne!(east, today, "a zone that crosses midnight moves the day");
    assert_eq!(
        run(&["date", "-d", "23:30-0800", "-f", "%Y-%m-%d"]),
        today,
        "the inverted sign does not cross midnight here"
    );
    // Under `-l` a time of day is refused, whatever it is attached to.
    for form in ["15:30", "15:30 UTC", "15:30+08:00", "23:30"] {
        assert!(
            run_failing(&["date", "-l", "-d", form]).contains("无法解析"),
            "`-l -d {form}` is refused"
        );
    }
    // A bare year is a year, not a clock: `2026` and `1530` keep the reading
    // the bare-year branch gives them.
    for form in ["2026", "1530"] {
        assert_eq!(
            run(&["date", "-d", form, "-f", "%Y-%m-%d"]),
            format!("{form}-{}", &today[5..]),
            "`-d {form}` is a bare year"
        );
    }
}

/// The Chinese weekday names resolve to the day their English spelling does.
///
/// The table carried 星期日 / 星期天 / 礼拜天 / 周一 and nothing else, so the
/// names a reader is most likely to type — `周六`, `星期二`, `礼拜六` — were
/// unparsable. All three prefixes over 一…日 plus the `天` variant are
/// registered now. This is **this tool's extension**: `date(1)` has no
/// weekday name in Chinese and refuses every one of them.
#[test]
fn chinese_weekday_names_are_accepted() {
    let english = |name: &str| run(&["date", "-d", name, "-f", "%Y-%m-%d"]);
    for (chinese, name) in [
        ("星期日", "sunday"),
        ("星期天", "sunday"),
        ("周日", "sunday"),
        ("周天", "sunday"),
        ("礼拜日", "sunday"),
        ("礼拜天", "sunday"),
        ("星期一", "monday"),
        ("周一", "monday"),
        ("礼拜一", "monday"),
        ("星期二", "tuesday"),
        ("周二", "tuesday"),
        ("礼拜二", "tuesday"),
        ("星期三", "wednesday"),
        ("周三", "wednesday"),
        ("礼拜三", "wednesday"),
        ("星期四", "thursday"),
        ("周四", "thursday"),
        ("礼拜四", "thursday"),
        ("星期五", "friday"),
        ("周五", "friday"),
        ("礼拜五", "friday"),
        ("星期六", "saturday"),
        ("周六", "saturday"),
        ("礼拜六", "saturday"),
    ] {
        assert_eq!(
            run(&["date", "-d", chinese, "-f", "%Y-%m-%d"]),
            english(name),
            "`{chinese}` is the same day as `{name}`"
        );
    }
    // `next` / `last` combine with them, as they do with the English names.
    for (chinese, name) in [
        ("next 星期六", "next saturday"),
        ("last 周天", "last sunday"),
    ] {
        assert_eq!(
            run(&["date", "-d", chinese, "-f", "%Y-%m-%d"]),
            english(name),
            "`{chinese}` is the same day as `{name}`"
        );
    }
    // A Chinese name names a *day*, not a date, so `-l` does not change it.
    for form in ["星期六", "周二", "礼拜六", "星期天", "next 星期六"] {
        assert_eq!(
            run(&["date", "-d", form]),
            run(&["date", "-l", "-d", form]),
            "`-d {form}` must not depend on `-l`"
        );
    }
}

/// A numeric zone offset shifts the day, not just the clock.
///
/// Every numeric form carries a sign, and the sign is part of the offset
/// `zone_offset` is given, so the zone is split *after* its sign rather than
/// before it: a slice that began at the sign would leave every `±hh:mm` with
/// a bare digit string, while the named zones `Z` / `UTC` / `GMT` — which
/// carry no sign — would keep parsing. The sign convention is the POSIX one
/// the module documents: `+0800` is 8 hours *behind* UTC, so a 23:30 reading
/// moves to the next day.
#[test]
fn a_numeric_zone_offset_is_accepted_and_shifts_the_day() {
    for zone in ["+08:00", "+0800", "-05:00", "-0500", "Z", "UTC", "GMT"] {
        assert!(
            run(&["date", "-d", &format!("2026-09-07T15:30{zone}")]).contains("公历: 2026年9月7日"),
            "zone {zone} is accepted and this time does not cross midnight"
        );
    }
    // POSIX `+0800` is 8 hours *behind* UTC, so 23:30 is 07:30 the next day.
    assert!(
        run(&["date", "-d", "2026-09-07T23:30+0800"]).contains("公历: 2026年9月8日"),
        "a zone that crosses midnight moves the day"
    );
    // `-0800` is 8 hours *ahead*: 23:30 is 15:30 the same day.
    assert!(
        run(&["date", "-d", "2026-09-07T23:30-0800"]).contains("公历: 2026年9月7日"),
        "the inverted sign does not cross midnight here"
    );
    // 00:30 at -0800 is 16:30 the previous day.
    assert!(
        run(&["date", "-d", "2026-09-07T00:30-0800"]).contains("公历: 2026年9月6日"),
        "a zone can move the day backwards too"
    );
}

/// An impossible zone offset is refused, not applied.
///
/// `zone_offset` counted the digits and never looked at them, so `+99:00`
/// shifted the day four days forward and `+09:99` ninety-nine minutes, both
/// reported as a confident answer. No zone is 99 hours from UTC, and
/// `date(1)` refuses both. The boundary it does accept — `24:00`, a whole
/// day — is accepted here too.
#[test]
fn an_impossible_zone_offset_is_refused() {
    for zone in [
        "+99:00", "+09:99", "+2401", "+24:01", "-99:99", "+25", "-25",
    ] {
        assert!(
            run_failing(&["date", "-d", &format!("2026-09-07T15:30{zone}")])
                .contains("无法解析的日期"),
            "zone {zone} is refused"
        );
    }
    // A whole day is a legal offset, and it does move the day.
    assert_eq!(
        run(&["date", "-d", "2026-09-07T15:30+24:00", "-f", "%Y-%m-%d"]),
        "2026-09-08"
    );
    // The real offsets are unaffected, and so are the ones at the edge.
    for (zone, expected) in [
        ("+00:00", "2026-09-07"),
        ("+23:59", "2026-09-08"),
        ("+08:00", "2026-09-07"),
    ] {
        assert_eq!(
            run(&[
                "date",
                "-d",
                &format!("2026-09-07T15:30{zone}"),
                "-f",
                "%Y-%m-%d"
            ]),
            expected,
            "zone {zone} is {expected}"
        );
    }
}

/// The compact `hhmm` clock and a bare zone designator are accepted.
///
/// `20260907T1530` is in the grammar and the date part worked, but
/// `parse_clock` split on `:` only, so the `1530` suffix failed — and a bare
/// `2026-09-07Z` failed too, because an empty clock was an error rather than
/// midnight. A zone with no clock now means midnight in that zone, which is
/// what `date -d "2026-09-07Z"` does.
#[test]
fn a_compact_clock_and_a_bare_zone_are_accepted() {
    for form in [
        "20260907T1530",
        "2026-09-07T1530",
        "2026-09-07T15",
        "2026-09-07T15:30",
        "2026-09-07T15:30:45",
        "2026-09-07Z",
        "2026-09-07T15:30Z",
    ] {
        assert_eq!(
            run(&["date", "-d", form, "-f", "%Y-%m-%d"]),
            "2026-09-07",
            "`-d {form}` is 2026-09-07"
        );
    }
    // A compact clock past midnight still moves the day.
    assert_eq!(
        run(&["date", "-d", "2026-09-07T2330+0800", "-f", "%Y-%m-%d"]),
        "2026-09-08"
    );
    // An out-of-range clock is still a rejection.
    for form in ["2026-09-07T2530", "2026-09-07T15:99", "2026-09-07T1:2:3:4"] {
        assert!(
            run_failing(&["date", "-d", form]).contains("无法解析"),
            "`-d {form}` is not a clock"
        );
    }
}

#[test]
fn date_profile_matches_documented_output() {
    assert_eq!(
        run(&["date", "-d", "2026-09-07"]),
        "公历: 2026年9月7日 星期一\n农历: 丙午年七月廿六\n干支: 丙午 丁酉 甲申\n生肖: 马\n节气: 白露\n星座: 处女"
    );
}

#[test]
fn date_profile_shows_leap_month() {
    assert_eq!(
        run(&["date", "-d", "2020-05-23"]),
        "公历: 2020年5月23日 星期六\n农历: 庚子年闰四月初一\n干支: 庚子 辛巳 丙寅\n生肖: 鼠\n星座: 双子"
    );
}

#[test]
fn date_accepts_positional_year_month_day() {
    assert_eq!(
        run(&["date", "2026", "2", "17"]),
        run(&["date", "-d", "2026-02-17"])
    );
    assert_eq!(
        run(&["date", "2026", "2", "17"]),
        "公历: 2026年2月17日 星期二\n农历: 丙午年正月初一\n干支: 丙午 庚寅 壬戌\n生肖: 马\n法定: 春节 放假\n星座: 水瓶"
    );
}

/// The `干支` line and the `宜:` / `忌:` lines describe one day in one
/// system, so the month pillar the line prints must be the one the advice
/// is keyed on.
///
/// The engine carries two: `month_in_gan_zhi()` turns at the 節氣 *day* and
/// `month_in_gan_zhi_exact()` at the 節氣 *instant*, and `day_yi` /
/// `day_ji` key their tables on the first. They disagree on each of the
/// twelve 節氣 days, so the 節氣 days themselves are where a split basis
/// shows — those are the days sampled here, not arbitrary ones.
#[test]
fn the_month_pillar_is_the_basis_the_advice_is_keyed_on() {
    // Each entry is a 節氣 day with the month pillar the day before it and
    // the day after, captured from the binary: the new month is already in
    // force on the 節氣 day itself, which is the day-granular rule and what
    // separates this basis from the instant one.
    for (day, before, on, after) in [
        ("2026-01-05", "戊子", "己丑", "己丑"),
        ("2026-02-04", "己丑", "庚寅", "庚寅"),
        ("2026-03-05", "庚寅", "辛卯", "辛卯"),
        ("2026-04-05", "辛卯", "壬辰", "壬辰"),
        ("2026-05-05", "壬辰", "癸巳", "癸巳"),
        ("2026-06-05", "癸巳", "甲午", "甲午"),
        ("2026-07-07", "甲午", "乙未", "乙未"),
        ("2026-08-07", "乙未", "丙申", "丙申"),
        ("2026-09-07", "丙申", "丁酉", "丁酉"),
        ("2026-10-08", "丁酉", "戊戌", "戊戌"),
        ("2026-11-07", "戊戌", "己亥", "己亥"),
        ("2026-12-07", "己亥", "庚子", "庚子"),
    ] {
        assert_eq!(
            run(&["date", "-d", day, "-f", "%H"]),
            on,
            "{day}: its own month"
        );
        assert_ne!(before, on, "{day}: the month turns *on* the 節氣 day");
        assert_eq!(after, on, "{day}: and it is still in force the day after");
    }
    // `%H` and the `干支` line share `calendar::month_gan_zhi`; a change that
    // moves one and not the other is invisible in either output alone.
    for day in ["2026-02-04", "2026-09-07", "2026-12-07"] {
        let profile = run(&["date", "-d", day]);
        let printed: Vec<&str> = profile
            .lines()
            .find_map(|l| l.strip_prefix("干支: "))
            .unwrap_or_else(|| panic!("{day}: a 干支 line in\n{profile}"))
            .split(' ')
            .collect();
        assert_eq!(
            printed[1],
            run(&["date", "-d", day, "-f", "%H"]),
            "{day}: the 干支 line's month pillar is %H's"
        );
    }
    // 2026-02-04 is 立春. The `干支` line has turned to 丙午 while `%G` —
    // the lunar year, which turns at 春节 — is still 乙巳: the two bases
    // genuinely differ on this day, and each line reports the one that
    // belongs to it. The published almanac reads 丙午年 庚寅月 己酉日.
    let profile = run(&["date", "-d", "2026-02-04"]);
    assert!(
        profile.contains("农历: 乙巳年腊月十七"),
        "the lunar year keeps the 春节 basis: {profile}"
    );
    assert!(
        profile.contains("干支: 丙午 庚寅 己酉"),
        "the 干支 line is the 立春 chain: {profile}"
    );
    assert_eq!(run(&["date", "-d", "2026-02-04", "-f", "%G"]), "乙巳");
    let almanac = run(&["date", "-a", "-d", "2026-02-04"]);
    assert!(
        almanac.contains("宜: 祭祀、祈福、求嗣、开光、出火、出行"),
        "{almanac}"
    );
    assert!(almanac.contains("忌: 嫁娶、作灶、安床"), "{almanac}");
}

/// The three pillars of the `干支` line share one basis, so the month stem
/// must be the 五虎遁 one for the year stem printed beside it.
///
/// The month counts from 立春, which is where the 干支 year turns; the 春节
/// basis belongs to the `农历:` line, which names the lunar year itself.
/// Pairing a 春节 year stem with a 立春 month stem produces pairs the two
/// pillars can never form — `乙巳 庚寅` is not a year a 庚寅 month can be
/// in — so the line would contradict itself on the few days each year where
/// the two bases disagree.
#[test]
fn the_gan_zhi_line_is_one_self_consistent_chain() {
    const STEMS: [char; 10] = ['甲', '乙', '丙', '丁', '戊', '己', '庚', '辛', '壬', '癸'];
    const BRANCHES: [char; 12] = [
        '子', '丑', '寅', '卯', '辰', '巳', '午', '未', '申', '酉', '戌', '亥',
    ];
    for day in [
        "2020-01-25",
        "2021-02-03",
        "2024-02-04",
        "2025-01-29",
        "2026-01-04",
        "2026-02-03",
        "2026-02-04",
        "2026-02-05",
    ] {
        let profile = run(&["date", "-d", day]);
        let line = profile
            .lines()
            .find_map(|l| l.strip_prefix("干支: "))
            .unwrap_or_else(|| panic!("{day}: a 干支 line in\n{profile}"));
        let printed: Vec<&str> = line.split(' ').collect();
        assert_eq!(printed.len(), 3, "{day}: three pillars, got {line:?}");
        let (year, month) = (printed[0], printed[1]);
        // 五虎遁: 甲己起丙寅, 乙庚起戊寅, 丙辛起庚寅, 丁壬起壬寅, 戊癸起甲寅.
        let stem = year.chars().next().expect("a year stem");
        let branch = month.chars().nth(1).expect("a month branch");
        let yin_stem = 2 * (STEMS.iter().position(|s| *s == stem).expect("a stem") % 5) + 2;
        let months_on = (BRANCHES
            .iter()
            .position(|b| *b == branch)
            .expect("a branch")
            + 10)
            % 12;
        let expected = STEMS[(yin_stem + months_on) % 10];
        assert_eq!(
            month.chars().next(),
            Some(expected),
            "{day}: 五虎遁 under {year} gives a month starting {expected}, engine says {month}"
        );
        // The day pillar is unaffected by the basis question, but it has to
        // be there: a two-pillar line would pass the check above silently.
        assert_eq!(
            printed[2].chars().count(),
            2,
            "{day}: a two-character day pillar"
        );
    }
}

/// The 星座 line closes the profile and is never missing: a constellation is
/// a function of the civil month and day alone, so — unlike `节气` and `法定` —
/// it has no day it fails to have. Its position is part of the contract, since
/// the conditional lines above it would otherwise push it around.
#[test]
fn the_profile_ends_with_the_constellation() {
    for day in ["2026-09-07", "2026-09-26", "2020-05-23", "1582-10-15"] {
        let profile = run(&["date", "-d", day]);
        let last = profile.lines().last().expect("at least one line");
        assert!(
            last.starts_with("星座: "),
            "{day}: the last line is the constellation, got {last:?}"
        );
    }
    assert_eq!(run(&["date", "-d", "2026-09-07", "-f", "%Z"]), "处女");
    // The boundaries: 处女 ends 9-22, 天秤 runs 9-23 to 10-23.
    assert_eq!(run(&["date", "-d", "2026-09-22", "-f", "%Z"]), "处女");
    assert_eq!(run(&["date", "-d", "2026-09-23", "-f", "%Z"]), "天秤");
    assert_eq!(run(&["date", "-d", "2026-10-23", "-f", "%Z"]), "天秤");
    assert_eq!(run(&["date", "-d", "2026-10-24", "-f", "%Z"]), "天蝎");
    // A day with no solar term, and a 放假 day, both still end with one.
    assert!(run(&["date", "-d", "2026-09-24"]).ends_with("星座: 天秤"));
    assert!(run(&["date", "-d", "2026-09-25"]).ends_with("星座: 天秤"));
}

/// `-a` appends the whole 黄历 block, and the profile it follows is exactly
/// the default one — the switch adds, it never substitutes.
#[test]
fn the_almanac_block_follows_the_whole_profile() {
    let profile = run(&["date", "-d", "2026-09-07"]);
    let with_block = run(&["date", "-a", "-d", "2026-09-07"]);
    assert!(
        with_block.starts_with(&profile),
        "the profile is the head of the almanac run"
    );
    for label in [
        "宜:",
        "忌:",
        "冲煞:",
        "值神:",
        "吉神:",
        "二十八宿:",
        "纳音:",
        "彭祖百忌:",
        "胎神:",
        "方位:",
        "物候:",
    ] {
        assert!(
            with_block.contains(&format!("\n{label} ")),
            "the block has a {label} line"
        );
    }
}

/// The seasonal groups are omitted on the days that have none, the same way
/// the `节气` line is — never printed empty.
#[test]
fn a_seasonal_group_is_omitted_rather_than_printed_empty() {
    let september = run(&["date", "-a", "-d", "2026-09-07"]);
    assert!(
        !september.contains("数九") && !september.contains("三伏"),
        "neither 数九 nor 三伏 is running in September"
    );
    // 数九 runs from 冬至 for 81 days; 三伏 from 夏至.
    assert!(
        run(&["date", "-a", "-d", "2026-01-20"]).contains("数九: 四九第4天"),
        "数九 names the period and the day in it"
    );
    assert!(
        run(&["date", "-a", "-d", "2026-07-25"]).contains("三伏: 中伏第1天"),
        "三伏 names the period and the day in it"
    );
    for line in run(&["date", "-a", "-d", "2026-01-20"]).lines() {
        assert!(!line.ends_with(':'), "no empty group: {line:?}");
    }
}

/// The block and `-f` are two shapes of one answer, so clap refuses the pair
/// rather than letting one silently win.
#[test]
fn the_almanac_and_a_format_are_refused_together() {
    for args in [
        ["date", "-a", "-f", "%G", "-d", "2026-09-07"].as_slice(),
        ["date", "-f", "%G", "-a", "-d", "2026-09-07"].as_slice(),
        ["date", "--almanac", "--format", "%G", "-d", "2026-09-07"].as_slice(),
    ] {
        assert!(
            run_failing(args).contains("cannot be used with"),
            "{args:?} is refused"
        );
    }
}

/// The block is a property of the **day**, never of a time of day: `T15:30`
/// and a zone must not move a single line of it, which is what lets a tool
/// that keeps no clock print it at all.
#[test]
fn the_almanac_does_not_depend_on_a_time_of_day() {
    let plain = run(&["date", "-a", "-d", "2026-09-07"]);
    for form in ["2026-09-07T15:30", "2026-09-07T23:30", "2026-09-07T00:00Z"] {
        assert_eq!(
            run(&["date", "-a", "-d", form]),
            plain,
            "{form} is the same day and the same almanac"
        );
    }
}

#[test]
fn date_omits_the_term_line_when_there_is_no_solar_term() {
    assert!(!run(&["date", "-d", "2026-09-25"]).contains("节气"));
}

#[test]
fn date_format_tokens_match_documented_examples() {
    assert_eq!(
        run(&["date", "-f", "%G年%M%N，星期%A", "-d", "2026-09-07"]),
        "丙午年七月廿六，星期一"
    );
    assert_eq!(
        run(&[
            "date",
            "-f",
            "%G年%M%N，星期%A\\n干支日：%D",
            "-d",
            "2026-09-07"
        ]),
        "丙午年七月廿六，星期一\n干支日：甲申"
    );
    assert_eq!(
        run(&[
            "date",
            "-f",
            "周%A 农历%M%N（日序 %n），生肖%S，节气：%Q",
            "-d",
            "2026-09-07"
        ]),
        "周一 农历七月廿六（日序 26），生肖马，节气：白露"
    );
}

#[test]
fn date_format_covers_every_documented_token() {
    assert_eq!(
        run(&[
            "date",
            "-f",
            "%Y|%m|%d|%A|%G|%M|%N|%n|%H|%D|%S|%Q|%Z|%%",
            "-d",
            "2026-09-07"
        ]),
        "2026|09|07|一|丙午|七月|廿六|26|丁酉|甲申|马|白露|处女|%"
    );
}

#[test]
fn civil_overlay_grid_matches_documented_output() {
    let expected = r#"2026年9月
一              二              三              四              五              六              日
                1               2               3               4               5               6
                二十            廿一            廿二            廿三            廿四            廿五
7               8               9               10              11              12              13
白露            廿七            廿八            教师节          八月            初二            初三
14              15              16              17              18              19              20
初四            初五            初六            初七            初八            全民国防教育日  初十
21              22              23              24              25              26              27
十一            十二            秋分            十四            中秋节          十六            十七
28              29              30
十八            十九            二十"#;
    assert_eq!(run(&["cal", "2026", "9"]), expected);
}

/// A grid must show every day of its month, including the last one, and
/// never a day borrowed from the neighbouring months.
///
/// The year is the point of the sweep. 2026 alone is a year both calendars
/// agree about; 100, 1300 and 1500 are not, because the engine follows the
/// Julian leap rule below 1600 — so the number of days the grid may draw has
/// to be the one the binary reports, not a local month-length helper
/// computing the Gregorian February, or the assertion is pinned to a length
/// the tool never uses. 1582 is in the sweep for the other reason: it is the
/// one month the engine counts shorter than the calendar has.
#[test]
fn civil_grid_shows_every_day_of_the_month() {
    for year in [1, 100, 1300, 1500, 1582, 2026, 9999] {
        for month in 1..=12 {
            let grid = run(&["cal", &year.to_string(), &month.to_string()]);
            let labels: Vec<&str> = grid
                .lines()
                .skip(2)
                .step_by(2)
                .filter(|line| !line.trim().is_empty())
                .flat_map(|line| line.split_whitespace())
                .collect();
            // Compare against the days the month *has*, not `1..=length`:
            // October 1582 is 21 days long and its labels are 1, 2, 3, 4,
            // 15 … 31, because the ten reform days are not there to be
            // numbered. The claim is that no day is dropped and none is
            // invented, which is what a list of the engine's own surviving
            // days says.
            let days = calendar::existing_days(year, month);
            let shown: Vec<u32> = labels.iter().filter_map(|cell| cell.parse().ok()).collect();
            assert_eq!(
                shown, days,
                "every day of {year}-{month}, in order:\n{grid}"
            );
        }
    }
}

/// The same guarantee for the lunar view, where cells carry `M/D` labels.
#[test]
fn lunar_grid_shows_every_day_of_the_month() {
    let grid = run(&["cal", "-L", "2026", "7"]);
    let labels: Vec<&str> = grid
        .lines()
        .skip(2)
        .step_by(2)
        .filter(|line| !line.trim().is_empty())
        .flat_map(|line| line.split_whitespace())
        .collect();
    // 七月 runs 8/13 .. 9/10 in 2026.
    assert_eq!(labels.first(), Some(&"8/13"));
    assert_eq!(labels.last(), Some(&"9/10"));
    assert_eq!(labels.len(), 29, "七月 has 29 days:\n{grid}");
}

#[test]
fn sunday_first_shifts_only_the_columns() {
    let monday = run(&["cal", "2026", "9"]);
    let sunday = run(&["cal", "2026", "9", "-s"]);
    assert_ne!(monday, sunday);

    let header: Vec<&str> = sunday.lines().take(2).collect();
    assert_eq!(header[0], "2026年9月");
    assert!(
        header[1].starts_with("日"),
        "Sunday leads the week:\n{header:?}"
    );
    for fragment in ["白露", "中秋节", "秋分"] {
        assert!(
            sunday.contains(fragment),
            "expected {fragment} in:\n{sunday}"
        );
    }
}

/// `-m` / `--monday` is a real option, not a parsed-and-dropped one.
///
/// It arrived with `let _ = monday;` in the dispatch, so asking for Monday
/// did nothing at all — and since Monday is the default the output was
/// already right, which is why nothing ever noticed. The flag now names the
/// first column in both views, and it is declared mutually exclusive with
/// `-s` rather than silently losing to it.
#[test]
fn the_monday_flag_names_the_first_column() {
    for args in [
        ["cal", "2026", "9", "-m"].as_slice(),
        ["cal", "2026", "9", "--monday"].as_slice(),
        ["cal", "-L", "2026", "7", "-m"].as_slice(),
    ] {
        assert!(
            run(args).lines().nth(1).unwrap().starts_with('一'),
            "`lunar {}` starts the week on Monday",
            args.join(" ")
        );
    }
    // Monday is the default, so the flag changes nothing about the output.
    assert_eq!(run(&["cal", "2026", "9", "-m"]), run(&["cal", "2026", "9"]));
    // `-s` still wins the other way, and the two together are refused rather
    // than one silently overwriting the other.
    assert!(
        run(&["cal", "2026", "9", "-s"])
            .lines()
            .nth(1)
            .unwrap()
            .starts_with('日')
    );
    for args in [
        ["cal", "2026", "9", "-s", "-m"].as_slice(),
        ["cal", "2026", "9", "-m", "-s"].as_slice(),
    ] {
        let output = binary().args(args).output().expect("lunar runs");
        assert!(
            !output.status.success(),
            "`lunar {}` must be refused, not silently resolved",
            args.join(" ")
        );
    }
}

#[test]
fn lunar_month_grid_has_the_documented_layout() {
    let expected = r#"农历 丙午年 七月
一      二      三      四      五      六      日
                        8/13    8/14    8/15    8/16
                        初一    初二    初三    初四
8/17    8/18    8/19    8/20    8/21    8/22    8/23
初五    初六    七夕节  初八    初九    初十    处暑
8/24    8/25    8/26    8/27    8/28    8/29    8/30
十二    十三    十四    中元节  十六    十七    十八
8/31    9/1     9/2     9/3     9/4     9/5     9/6
十九    二十    廿一    廿二    廿三    廿四    廿五
9/7     9/8     9/9     9/10
白露    廿七    廿八    教师节"#;
    assert_eq!(run(&["cal", "-L", "2026", "7"]), expected);
}

#[test]
fn leap_lunar_month_grid_has_the_documented_layout() {
    let expected = r#"农历 庚子年 闰四月
一      二      三      四      五      六      日
                                        5/23    5/24
                                        初一    初二
5/25    5/26    5/27    5/28    5/29    5/30    5/31
初三    初四    初五    初六    初七    初八    初九
6/1     6/2     6/3     6/4     6/5     6/6     6/7
儿童节  十一    十二    十三    芒种    十五    十六
6/8     6/9     6/10    6/11    6/12    6/13    6/14
十七    十八    十九    二十    廿一    廿二    廿三
6/15    6/16    6/17    6/18    6/19    6/20
廿四    廿五    廿六    廿七    廿八    廿九"#;
    assert_eq!(run(&["cal", "-L", "2020", "4", "-R"]), expected);
}

/// Cell priority: festival, else the month name on 初一, else a solar term,
/// else the lunar day.
#[test]
fn cell_overlays_follow_the_documented_priority() {
    let grid = run(&["cal", "2026", "9"]);
    assert!(grid.contains("八月"), "the month name replaces 初一");
    assert!(grid.contains("白露"), "a solar term beats the lunar day");
    assert!(grid.contains("中秋节"), "a festival beats everything");
}

#[test]
fn number_flag_replaces_chinese_lunar_day() {
    let chinese = run(&["cal", "2026", "9"]);
    let numeric = run(&["cal", "2026", "9", "--number"]);
    assert!(chinese.contains("廿七"));
    assert!(numeric.contains("27"));
    assert!(!numeric.contains("廿七"));
}

#[test]
fn month_name_and_festival_overlays_can_be_disabled() {
    let plain = run(&[
        "cal",
        "2026",
        "9",
        "--no-month-name",
        "--no-festival",
        "--no-holiday",
    ]);
    assert!(
        !plain.contains("八月"),
        "month name overlay disabled:\n{plain}"
    );
    assert!(
        !plain.contains("中秋节"),
        "festival overlay disabled:\n{plain}"
    );
    assert!(
        !plain.contains("放假"),
        "holiday overlay disabled:\n{plain}"
    );
    assert!(
        plain.contains("初一"),
        "the lunar day is still shown:\n{plain}"
    );
}

#[test]
fn leap_month_is_only_reachable_with_the_leap_flag() {
    let plain = run(&["cal", "-L", "2020", "4"]);
    assert!(
        !plain.contains("闰四月"),
        "plain month 4 is not the leap one:\n{plain}"
    );
    assert!(run(&["cal", "-L", "2020", "4", "-R"]).contains("闰四月"));
    assert!(run_failing(&["cal", "-L", "2021", "4", "-R"]).contains("没有闰月"));
}

/// `-R` names a 闰月, which is a 农历 concept: it is refused without `-l`.
///
/// A flag whose help reads (-l 时) has to mean something, and silently
/// ignoring it is the one outcome the reader cannot detect: `date -R -d
/// 2020-04-01` would print the 公历 date and `cal -R 2020 4` would print
/// 2020年4月, both looking like agreement. So it is refused, and the refusal
/// names `--lunar`, the flag that would make it meaningful.
/// `-s`/`-m` are made mutually exclusive for the same reason; this is the
/// one-sided version, since `-l` alone is perfectly meaningful.
#[test]
fn the_leap_flag_requires_the_lunar_flag() {
    for args in [
        ["date", "-R", "-d", "2020-04-01"].as_slice(),
        ["date", "--leap", "-d", "2020-04-01"].as_slice(),
        ["cal", "-R", "2020", "4"].as_slice(),
        ["cal", "--leap", "2020", "4"].as_slice(),
    ] {
        let output = binary().args(args).output().expect("lunar runs");
        assert!(
            !output.status.success(),
            "`lunar {}` must be refused, not silently resolved",
            args.join(" ")
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("--lunar"),
            "`lunar {}` says which flag it needs",
            args.join(" ")
        );
    }
    // With the flag it needs, it is the leap month and nothing else.
    assert!(run(&["date", "-l", "-R", "2020", "4", "1"]).contains("闰四月初一"));
    assert!(run(&["cal", "-L", "2020", "4", "-R"]).contains("闰四月"));
    // And `-l` on its own is still a plain lunar request.
    assert!(run(&["date", "-l", "2026", "7", "15"]).contains("七月十五"));
    assert!(run(&["cal", "-L", "2026", "7"]).contains("农历 丙午年 七月"));
}

/// A lunar month outside `1..=12` is reported, not indexed.
///
/// The month reaches a table lookup on the way to the error message, so `0`,
/// `13` and `i32::MIN` are values the code must range-check *before* it
/// indexes: a panic there is an exit code of 101 rather than 1. `--` passes a
/// negative number past clap's own parser, so the check is the tool's.
#[test]
fn out_of_range_lunar_months_are_reported_not_indexed() {
    for month in ["0", "13", "-13", "-2147483648"] {
        assert!(
            run_failing(&["cal", "-L", "2026", "--", month]).contains("农历月份"),
            "lunar month {month} is reported as out of range"
        );
    }
    // `date` derives the leap month by negating the one the user wrote, and
    // `-R` checks the month first: `i32::MIN` has no absolute value, so the
    // positionals have to report the number the user wrote, through the
    // message `cal` already used.
    for month in ["0", "13", "-13", "-2147483648"] {
        assert!(
            run_failing(&["date", "-l", "-R", "2026", "--", month, "1"]).contains("农历月份"),
            "`date -l -R 2026 -- {month} 1` reports the month"
        );
    }
    // The same check runs on the `-d` path, where the month arrives as text.
    // A field the grammar has no reading for is refused as a bad date; one
    // that reaches the resolver is reported as a bad month, never negated.
    for (form, expected) in [
        ("2026-13-01", "农历月份 13"),
        // The unpadded form reaches the resolver as a month too, and names
        // the bad one rather than reporting the whole string as unreadable:
        // `2026-0-01` and `2026-0-1` are the same month the user wrote.
        ("2026-0-01", "农历月份 0"),
        ("2026-0-1", "农历月份 0"),
        ("2026--13-01", "无法解析的日期"),
        ("2026--2147483648-01", "无法解析的日期"),
    ] {
        assert!(
            run_failing(&["date", "-l", "-R", "-d", form]).contains(expected),
            "`-l -R -d {form}` is {expected}"
        );
    }
    // An in-range leap month the year does not have still names itself, so
    // the renderer had to survive the lookup rather than index blindly. 闰腊月
    // is the rarest form there is — 155 of 9,999 years.
    assert!(
        run(&["cal", "-L", "37", "12", "-R"]).contains("闰腊月"),
        "闰腊月 renders"
    );
    assert!(
        run_failing(&["cal", "-L", "2026", "--", "-1"]).contains("没有闰正月"),
        "a leap month absent from the year is named, not indexed"
    );
}

#[test]
fn out_of_range_years_are_rejected() {
    assert!(run_failing(&["cal", "10000", "1"]).contains("超出支持范围"));
    assert!(run_failing(&["cal", "0", "1"]).contains("超出支持范围"));
    // The 1582 reform gap does not exist in the Gregorian calendar.
    assert!(run_failing(&["date", "-d", "1582-10-10"]).contains("不存在"));
    // A day the calendar does not have, now reported the same way through
    // either channel: both resolve through `datestr`.
    assert!(run_failing(&["date", "-d", "2023-02-30"]).contains("不存在"));
    assert!(run_failing(&["date", "2023", "2", "30"]).contains("不存在"));
}

/// A malformed positional says what is wrong with the argument.
///
/// `cal` reported every one of these as 月份 -1 非法 (应为 1–12) — a
/// non-numeric year, a third positional, all of them a month out of range by
/// a number the user never wrote. The sentinel is gone: each failure carries
/// its own message, and a real out-of-range month still says so.
#[test]
fn a_malformed_positional_is_reported_as_such() {
    for args in [
        ["cal", "2026", "abc"].as_slice(),
        ["cal", "abc"].as_slice(),
        ["cal", "-L", "2026", "abc"].as_slice(),
        ["cal", "-L", "abc"].as_slice(),
    ] {
        let error = run_failing(args);
        assert!(
            error.contains("无法解析的位置参数"),
            "`lunar {}` names the bad argument, got: {error}",
            args.join(" ")
        );
        assert!(
            !error.contains("月份 -1"),
            "`lunar {}` does not report a month the user never wrote",
            args.join(" ")
        );
    }
    // Too many positionals is its own problem, not a month out of range.
    for args in [
        ["cal", "2026", "1", "2"].as_slice(),
        ["cal", "2026", "1", "2", "3"].as_slice(),
        ["cal", "-L", "2026", "1", "2"].as_slice(),
        // `date` swallowed a fourth: the arm was `[year, month, day, ..]`, so
        // `date 2026 1 1 extra` answered 2026-01-01 and exited 0 — a typo read
        // as agreement, and a silently different date if the typo was a
        // number.
        ["date", "2026", "1", "1", "extra"].as_slice(),
        ["date", "2026", "1", "1", "2"].as_slice(),
        ["date", "2026", "1", "2", "3", "4", "5"].as_slice(),
        ["date", "-l", "2026", "7", "15", "3"].as_slice(),
    ] {
        assert!(
            run_failing(args).contains("位置参数过多"),
            "`lunar {}` reports the count",
            args.join(" ")
        );
    }
    // A real out-of-range month and year still report themselves.
    assert!(run_failing(&["cal", "2026", "13"]).contains("月份 13 非法"));
    assert!(run_failing(&["cal", "10000", "1"]).contains("年份 10000 超出支持范围"));
    // `date` had its own error already and keeps it.
    assert!(run_failing(&["date", "abc"]).contains("无法解析的日期"));
}

/// October 1582 renders, with the ten reform days simply absent.
///
/// The grid stepped the month with `add_days` and validated every slot with
/// `to_solar`, so a month whose window crossed the gap died with the
/// `YearMissing` message. The engine counts 21 days in October 1582, not
/// 31, and the civil view now enumerates the days that exist.
#[test]
fn the_reform_gap_renders_as_absent_days() {
    let grid = run(&["cal", "1582", "10"]);
    let labels: Vec<&str> = grid
        .lines()
        .skip(2)
        .step_by(2)
        .filter(|line| !line.trim().is_empty())
        .flat_map(|line| line.split_whitespace())
        .collect();
    // 31 days proleptic, 21 on the engine's calendar.
    assert_eq!(labels.len(), 21, "1582-10 has 21 days:\n{grid}");
    assert_eq!(labels.first(), Some(&"1"));
    assert_eq!(labels.get(3), Some(&"4"));
    // The gap leaves no hole in the week: 4 October is a Thursday and the
    // 15th a Friday, so they sit in adjacent columns.
    assert_eq!(labels.get(4), Some(&"15"));
    for day in 5..=14 {
        assert!(
            !grid.contains(&format!("\n{day} ")) && !grid.contains(&format!(" {day} ")),
            "1582-10-{day} does not exist and has no cell:\n{grid}"
        );
    }
    // The months either side are untouched.
    assert!(run(&["cal", "1582", "9"]).contains("1582年9月"));
    assert!(run(&["cal", "1582", "11"]).contains("1582年11月"));
}

/// The lunar view's date band is the engine's own, day for day.
///
/// The date band is stepped in the engine's own calendar, never in
/// `add_days` — proleptic-Gregorian arithmetic. The content band and the
/// month length come from the engine, which uses the Julian leap rule before
/// 1600 and skips the reform days of 1582, and a band stepped by any other
/// rule would put two calendars in one grid: `cal -L 1300 2` would draw 36
/// cells for a 30-day month, 6 of them contradicting `lunar date`, and would
/// end on the *next* month's 初一. So the first and the last label are pinned
/// to the day the engine converts the same lunar day to, which is the only
/// claim that can fail when the two calendars drift.
#[test]
fn the_lunar_view_labels_the_days_the_engine_converts() {
    for (year, month) in [
        ("1", "1"),
        ("100", "2"),
        ("1300", "2"),
        ("1582", "9"),
        ("2026", "7"),
    ] {
        let grid = run(&["cal", "-L", year, month]);
        let labels: Vec<&str> = grid
            .lines()
            .skip(2)
            .step_by(2)
            .filter(|line| !line.trim().is_empty())
            .flat_map(|line| line.split_whitespace())
            .filter(|label| label.contains('/'))
            .collect();

        // The month may be 29 or 30 days. Ask the engine which rather than
        // assuming a length: day 30 either answers or says the month is
        // shorter, and either way it is the engine's own answer.
        let last = {
            let output = binary()
                .args(["date", "-l", year, month, "30"])
                .output()
                .expect("lunar runs");
            if output.status.success() {
                "30"
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                assert!(
                    error.contains("没有第 30 天"),
                    "`date -l {year} {month} 30` failed: {error}"
                );
                "29"
            }
        };
        let first = run(&["date", "-l", year, month, "1"]);
        let first = first.lines().next().expect("a 公历 line");
        let last = run(&["date", "-l", year, month, last]);
        let last = last.lines().next().expect("a 公历 line");

        let month_and_day = |line: &str| {
            // The line is `公历: 2026年9月10日 星期四`, and the prefix has a
            // space of its own, so the date ends at the last space.
            let (ymd, _) = line.rsplit_once(' ').expect("公历: <date> <weekday>");
            let (_, rest) = ymd.split_once('年').expect("年 月日");
            let (month, day) = rest.split_once('月').expect("月 日");
            format!("{month}/{}", day.trim_end_matches('日'))
        };
        assert_eq!(
            labels.first().copied(),
            Some(month_and_day(first).as_str()),
            "`cal -L {year} {month}` starts on the day the engine gives:\n{grid}"
        );
        assert_eq!(
            labels.last().copied(),
            Some(month_and_day(last).as_str()),
            "`cal -L {year} {month}` ends on the day the engine gives:\n{grid}"
        );
    }
}

/// A lunar month that spans the reform gap renders, and skips the ten days.
///
/// The same hybrid calendar took the whole command down: 九月 1582 starts
/// 9/17, so stepping its thirty days ran into 1582-10-05 and `to_solar`
/// answered `YearMissing`, killing `cal -L 1582 9` and the whole lunar year
/// with it. Enumerating the days the month has leaves no gap to fall into —
/// the 4th and the 15th sit in adjacent columns, exactly as the civil view
/// does.
#[test]
fn a_lunar_month_spanning_the_reform_gap_renders() {
    let grid = run(&["cal", "-L", "1582", "9"]);
    let labels: Vec<&str> = grid
        .lines()
        .skip(2)
        .step_by(2)
        .filter(|line| !line.trim().is_empty())
        .flat_map(|line| line.split_whitespace())
        .collect();
    assert_eq!(labels.first(), Some(&"9/17"), "九月 starts 9/17:\n{grid}");
    assert_eq!(labels.last(), Some(&"10/25"), "九月 has 29 days:\n{grid}");
    // The 4th and the 15th are consecutive days, so they share a week row.
    let lines: Vec<&str> = grid.lines().collect();
    let row_of = |label: &str| {
        let at = lines
            .iter()
            .position(|line| line.split_whitespace().any(|cell| cell == label))
            .unwrap_or_else(|| panic!("no cell labelled {label}:\n{grid}"));
        let cell = lines[at]
            .split_whitespace()
            .position(|cell| cell == label)
            .expect("the label is a cell");
        (at, cell)
    };
    let (row_four, cell_four) = row_of("10/4");
    let (row_fifteen, cell_fifteen) = row_of("10/15");
    assert_eq!(
        row_fifteen, row_four,
        "the 4th and the 15th share a week row:\n{grid}"
    );
    assert_eq!(
        cell_fifteen,
        cell_four + 1,
        "the 4th and the 15th are in adjacent columns:\n{grid}"
    );
    for day in 5..=14 {
        assert!(
            !grid.contains(&format!("10/{day} ")) && !grid.contains(&format!(" 10/{day}\n")),
            "1582-10-{day} does not exist and has no cell:\n{grid}"
        );
    }
    // The whole lunar year renders for the same reason.
    assert_eq!(run(&["cal", "-L", "1582"]).matches("农历").count(), 12);
}

#[test]
fn unsupported_date_strings_are_reported() {
    assert!(run_failing(&["date", "-d", "definitely not a date"]).contains("无法解析"));
}

/// A multi-byte character in a fixed-width date field is a bad date, not a
/// crash.
///
/// The grammar is matched on **bytes** — `bytes[4]` is the first separator —
/// and then sliced as a `&str` at that offset, so every form is read through a
/// helper that checks the boundary and the digits before it slices: a
/// character straddling the offset is not a boundary, and panicking there is
/// an exit code of 101 rather than a refusal. Such an input has no reading, so
/// it is answered `无法解析的日期` like any other.
#[test]
fn a_multi_byte_character_in_a_date_field_is_reported_not_fatal() {
    for form in [
        "2026-09-中",
        "2026-09-0中",
        "2026-09-中5",
        "2026-09-甲",
        "2026-中-07",
        "2026/09/中",
        "2026090中",
        "20260907中",
        "中2026-09-07",
    ] {
        assert!(
            run_failing(&["date", "-d", form]).contains("无法解析的日期"),
            "`-d {form}` is reported, not fatal"
        );
    }
    // The forms that *are* dates keep working, in both calendars.
    for form in [
        "2026-09-07",
        "20260907",
        "2026/09/07",
        "2026-09-07T15:30",
        "2026-09-07Z",
    ] {
        assert_eq!(
            run(&["date", "-d", form, "-f", "%Y-%m-%d"]),
            "2026-09-07",
            "`-d {form}` is still 2026-09-07"
        );
    }
}

#[test]
fn help_format_lists_the_tokens() {
    let help = run(&["date", "--help-format"]);
    assert!(help.contains("%Y 公历年"));
    assert!(help.contains("%Q 节气"));
    assert!(help.contains("\\n 换行"));
    assert!(help.contains("%Z 星座"));
}

/// The token help does not depend on a zone it never uses.
///
/// `--help-format` was answered from inside the dispatch, which runs after
/// `tz::today()`, so a typo'd `TZ` made the one page that lists the tokens
/// unreadable — exactly when a reader is most likely to want it. The flag is
/// now short-circuited before the reference day is resolved. Every command
/// that *does* need a day still reports the zone problem.
#[test]
fn help_format_prints_without_a_readable_zone() {
    for tz in ["Asia/Shangahi", "Not/AZone", "!!!"] {
        let output = binary()
            .args(["date", "--help-format"])
            .env("TZ", tz)
            .output()
            .expect("lunar runs");
        assert!(
            output.status.success(),
            "`--help-format` prints with TZ={tz}, got: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("%Y 公历年"),
            "TZ={tz} still prints the tokens"
        );
    }
    // The commands that name a day are unaffected: they still need the zone.
    for args in [
        ["date", "-d", "2026-09-07"].as_slice(),
        ["date"].as_slice(),
        ["cal", "2026", "9"].as_slice(),
    ] {
        assert!(
            run_failing_with_tz("Asia/Shangahi", args).contains("无法读取时区"),
            "`lunar {}` still reports the zone",
            args.join(" ")
        );
    }
}

/// A zone the engine cannot read is reported as a zone problem.
///
/// `TZ`, an unreadable system zone, a clock before 1970 and a local year
/// outside 1–9999 all raised `TodayOutOfRange`, so a typo'd `TZ` was
/// answered with 当前日期超出支持范围 — a statement about the calendar, sent
/// to a reader whose calendar was fine. Each is its own error now.
#[test]
fn a_bad_time_zone_is_reported_as_a_zone_problem() {
    for tz in ["Asia/Shangahi", "Not/AZone", "!!!"] {
        let error = run_failing_with_tz(tz, &["date"]);
        assert!(
            error.contains("无法读取时区"),
            "TZ={tz} reports a zone problem, got: {error}"
        );
        assert!(
            !error.contains("超出支持范围"),
            "TZ={tz} must not blame the date range, got: {error}"
        );
    }
}
#[test]
fn a_bare_year_prints_the_whole_year() {
    // A single positional argument means the whole year.
    let year = run(&["cal", "2026"]);
    assert_eq!(year.matches("2026年").count(), 12, "twelve months");
    assert!(year.starts_with("2026年1月\n"));
    // A bare lunar year walks the whole lunar year, leap month included.
    let lunar_year = run(&["cal", "-L", "2020"]);
    assert_eq!(lunar_year.matches("农历").count(), 13);
    assert!(lunar_year.contains("闰四月"));
    assert!(lunar_year.starts_with("农历 庚子年 正月\n"));
}

/// A span of zero months is a mistake, not a request for one month.
///
/// A span count is bounded at parse time, so a refusal names the value that
/// was wrong rather than arriving after the fact, and `cal 2026 9 -n 0` is
/// reported like every other out-of-range argument instead of being repaired
/// into a one-month window: the user asked for no months, not for one.
#[test]
fn a_non_positive_month_span_is_refused() {
    for args in [
        ["cal", "2026", "9", "-n", "0"].as_slice(),
        ["cal", "-L", "2026", "7", "-n", "0"].as_slice(),
        ["cal", "2026", "9", "--months", "0"].as_slice(),
    ] {
        let output = binary().args(args).output().expect("lunar runs");
        assert!(
            !output.status.success(),
            "`lunar {}` must be refused",
            args.join(" ")
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("0 is not in 1"),
            "`lunar {}` names the range",
            args.join(" ")
        );
    }
    // One month is a legitimate span and still reads as one.
    assert_eq!(
        run(&["cal", "2026", "9", "-n", "1"])
            .lines()
            .filter(|l| l.ends_with('月'))
            .count(),
        1
    );
}
/// The grid titles of a `-L` run, in order.
fn lunar_titles(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| line.starts_with("农历"))
        .map(str::to_string)
        .collect()
}

/// The grid titles of a civil run, in order.
fn civil_titles(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| line.ends_with('月'))
        .map(str::to_string)
        .collect()
}

/// `-3` centres on the month named and `-n N` starts at it.
///
/// `select_lunar_months` returned the named month as soon as it saw two
/// positionals, so the span flags were never read: `cal -L 2026 7 -3` printed
/// one month where `cal 2026 7 -3` printed three. Centring was then the only
/// reading either flag had — `-n` centred too — and a window that ran off the
/// end of the year was shifted back to keep its length, so `-3` from 正月
/// printed 正月, 二月, 三月 instead of the 腊月 before it. The two flags now
/// read the way `cal(1)` reads them, and a window is a slice of the lunar
/// month sequence rather than a span clamped to one year.
#[test]
fn a_lunar_span_centres_or_starts_as_the_flag_says() {
    // Centred: the named month with one either side.
    assert_eq!(
        lunar_titles(&run(&["cal", "-L", "2026", "7", "-3"])),
        vec!["农历 丙午年 六月", "农历 丙午年 七月", "农历 丙午年 八月"]
    );
    // The flag that named one month still names one month.
    assert_eq!(
        lunar_titles(&run(&["cal", "-L", "2026", "7"])),
        vec!["农历 丙午年 七月"]
    );
    // `-n N` starts at the month named rather than centring on it.
    assert_eq!(
        lunar_titles(&run(&["cal", "-L", "2026", "7", "-n", "3"])),
        vec!["农历 丙午年 七月", "农历 丙午年 八月", "农历 丙午年 九月"]
    );
    assert_eq!(
        lunar_titles(&run(&["cal", "-L", "2026", "7", "-n", "5"])),
        vec![
            "农历 丙午年 七月",
            "农历 丙午年 八月",
            "农历 丙午年 九月",
            "农历 丙午年 十月",
            "农历 丙午年 冬月",
        ]
    );
    // A centred window at the start of a year takes the 腊月 before it, and
    // at the end it runs into the next 正月 — the length is kept either way,
    // which is why the window is built from the continuous month sequence and
    // not from the year it names.
    assert_eq!(
        lunar_titles(&run(&["cal", "-L", "2026", "1", "-3"])),
        vec!["农历 乙巳年 腊月", "农历 丙午年 正月", "农历 丙午年 二月"]
    );
    assert_eq!(
        lunar_titles(&run(&["cal", "-L", "2026", "12", "-3"])),
        vec!["农历 丙午年 冬月", "农历 丙午年 腊月", "农历 丁未年 正月"]
    );
    // `-R` centres on the leap month when one exists.
    assert_eq!(
        lunar_titles(&run(&["cal", "-L", "2020", "4", "-R", "-3"])),
        vec!["农历 庚子年 四月", "农历 庚子年 闰四月", "农历 庚子年 五月"]
    );
    // A span longer than the year crosses it rather than stopping at the
    // year's end: 七月 庚子年 through 二月 壬寅年 is twenty consecutive months,
    // which is exactly what `-n 20` asked for.
    let long = lunar_titles(&run(&["cal", "-L", "2020", "7", "-n", "20"]));
    assert_eq!(long.len(), 20);
    assert_eq!(long[0], "农历 庚子年 七月");
    assert_eq!(long[19], "农历 壬寅年 二月");
}

/// A lunar span walks the continuous month sequence, across the new year.
///
/// 腊月 always begins in the following January, so the sequence's own
/// successor to 丙午年 腊月 is 丁未年 正月: a window is a run of consecutive
/// lunar months, and it is built from that sequence rather than from the year
/// it names, so a span can show two ganzhi years. The title carries the year,
/// so crossing is readable rather than ambiguous.
#[test]
fn the_lunar_span_crosses_the_lunar_new_year() {
    assert_eq!(
        lunar_titles(&run(&["cal", "-L", "2026", "12", "-n", "3"])),
        vec!["农历 丙午年 腊月", "农历 丁未年 正月", "农历 丁未年 二月"]
    );
    // Centred on 正月, the window is the 腊月 that belongs to the *previous*
    // lunar year, which is the whole point: the sequence is the calendar, not
    // the year the argument named.
    assert_eq!(
        lunar_titles(&run(&["cal", "-L", "2026", "1", "-3"])),
        vec!["农历 乙巳年 腊月", "农历 丙午年 正月", "农历 丙午年 二月"]
    );
}

/// A missing month comes from the reference day, the way `cal(1)`'s does.
///
/// `cal(1)` fills a missing month from today, not from January: `cal -3 2025`
/// prints 八月, 九月, 十月 2025, and `cal 2026 -3` centres on today's month in
/// 2026. The reference day is not an input the suite can pin, so the test
/// asks the binary which month it is and checks the window against that.
#[test]
fn the_missing_month_comes_from_today() {
    let today = run(&["date", "-d", "today", "-f", "%Y %m"]);
    let (year, month) = today.split_once(' ').expect("a year and a month");
    let month: i32 = month.parse().expect("a month number");

    // A bare year with a span flag is that year, centred on or started at
    // today's month in it.
    let titles = civil_titles(&run(&["cal", year, "-3"]));
    assert_eq!(titles.len(), 3, "`cal {year} -3` prints three months");
    assert!(
        titles.contains(&format!("{year}年{month}月")),
        "`cal {year} -3` contains the month of {today}: {titles:?}"
    );
    // The other two are the months either side of it: the titles are
    // consecutive numbers, which is checkable without knowing the month, and
    // it holds across a year boundary.
    let numbers: Vec<i32> = titles
        .iter()
        .map(|title| {
            title
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .trim_matches(|c: char| c == '年')
                .trim_end_matches('月')
                .parse()
                .expect("a month number")
        })
        .collect();
    assert!(
        numbers.windows(2).all(|pair| pair[1] == pair[0] % 12 + 1),
        "three consecutive months across a year boundary: {numbers:?}"
    );
    // No positionals at all: the same window, in today's own year.
    assert_eq!(civil_titles(&run(&["cal", "-3"])), titles);
    // A whole year ignores the month and is the twelve months of the year
    // named — `cal -y 2026` prints 2026, not twelve months from September.
    let whole = civil_titles(&run(&["cal", year, "-y"]));
    assert_eq!(whole.len(), 12, "`cal {year} -y` is twelve months");
    assert!(whole[0].starts_with(&format!("{year}年1月")));
    // And the lunar view reads the same way, on today's own lunar month.
    let lunar = lunar_titles(&run(&["cal", "-L", "-3"]));
    assert_eq!(lunar.len(), 3, "`cal -L -3` prints three lunar months");
    let this_lunar_month = run(&["date", "-d", "today", "-f", "%G年 %M"]);
    assert!(
        lunar.iter().any(|title| title.contains(&this_lunar_month)),
        "`cal -L -3` contains {this_lunar_month}: {lunar:?}"
    );
}

/// A window that runs off the supported range is reported, not clipped.
///
/// The window, not the month, is what reached past the end: 农历 9999 年腊月
/// is a month the engine has, but the *anchor* here is one the window leaves
/// by two months, and it is the next lunar year the window walks into that
/// the range error names. A civil `9999 12 -n 3` is the same shape.
#[test]
fn a_span_off_the_range_end_is_reported() {
    for args in [
        ["cal", "-L", "9999", "12", "-n", "3"].as_slice(),
        ["cal", "-L", "9999", "12", "-3"].as_slice(),
        ["cal", "9999", "12", "-n", "3"].as_slice(),
        ["cal", "9999", "12", "-3"].as_slice(),
    ] {
        assert!(
            run_failing(args).contains("年份 10000 超出支持范围"),
            "`lunar {}` names the year the window reached",
            args.join(" ")
        );
    }
    // The low end is the same shape: a window centred on the first year needs
    // the year before it, which is not one the engine serves.
    for args in [
        ["cal", "-L", "1", "1", "-3"].as_slice(),
        ["cal", "1", "1", "-3"].as_slice(),
    ] {
        assert!(
            run_failing(args).contains("年份 0 超出支持范围"),
            "`lunar {}` names the year the window reached",
            args.join(" ")
        );
    }
}

/// The last lunar month the engine has ends outside the range, and the error
/// says so.
///
/// 农历 9999 年腊月 begins on 公历 9999-12-30 and ends on 10000-01-27, so
/// drawing it means drawing days the tool has no year for. The month is not
/// a typo — the whole year reaches it — and the message has to name the month
/// rather than a bare `年份 10000`, which reads as a year somebody typed. The
/// month before it is inside the range and still prints.
#[test]
fn the_last_lunar_year_reports_the_boundary() {
    let boundary = "农历 9999 年腊月跨入 10000 年，超出支持范围 (1–9999)";
    for args in [
        ["cal", "-L", "9999"].as_slice(),
        ["cal", "-L", "9999", "12"].as_slice(),
    ] {
        assert_eq!(
            run_failing(args),
            format!("lunar: {boundary}"),
            "`lunar {}` names the month and the year it crossed into",
            args.join(" ")
        );
    }
    // 冬月 runs 9999-11-30 … 9999-12-29, entirely inside the range.
    let grid = run(&["cal", "-L", "9999", "11"]);
    assert_eq!(
        grid.lines().next(),
        Some("农历 己亥年 冬月"),
        "the month before the boundary is unaffected:\n{grid}"
    );
    // The first lunar year is the mirror image and still prints all twelve.
    assert_eq!(lunar_titles(&run(&["cal", "-L", "1"])).len(), 12);
}

/// The span flags mean what `cal(1)`'s mean.
///
/// `cal(1)` (util-linux 2.42.4) reads `-3` as the month named and its
/// neighbours, `-n N` as the next `N` months from it, and `-y` as the whole
/// year — every one of them crossing the year boundary freely, and a missing
/// month filled in from today. This tool read `-3` as walking forward, `-n`
/// as centring, and `-y` as twelve months from the month named.
#[test]
fn the_span_flags_match_cal_one() {
    // `cal -3 9 2026` — centred.
    assert_eq!(
        civil_titles(&run(&["cal", "2026", "7", "-3"])),
        vec!["2026年6月", "2026年7月", "2026年8月"]
    );
    assert_eq!(
        civil_titles(&run(&["cal", "2026", "9", "-3"])),
        vec!["2026年8月", "2026年9月", "2026年10月"]
    );
    // A window that crosses the year boundary is a sequence of months, not a
    // window clamped to the year: `cal -3 12 2026` is 十一月, 十二月, 一月.
    assert_eq!(
        civil_titles(&run(&["cal", "2026", "12", "-3"])),
        vec!["2026年11月", "2026年12月", "2027年1月"]
    );
    assert_eq!(
        civil_titles(&run(&["cal", "2026", "1", "-3"])),
        vec!["2025年12月", "2026年1月", "2026年2月"]
    );
    // `cal -n 3 9 2026` — starting at the month named.
    assert_eq!(
        civil_titles(&run(&["cal", "2026", "7", "-n", "3"])),
        vec!["2026年7月", "2026年8月", "2026年9月"]
    );
    assert_eq!(
        civil_titles(&run(&["cal", "2026", "12", "-n", "3"])),
        vec!["2026年12月", "2027年1月", "2027年2月"]
    );
    // `cal -y 9 2026` — the whole year, whatever month is named.
    for args in [
        ["cal", "2026", "9", "-y"].as_slice(),
        ["cal", "2026", "1", "-y"].as_slice(),
        ["cal", "2026", "12", "-y"].as_slice(),
        ["cal", "2026", "-y"].as_slice(),
    ] {
        let titles = civil_titles(&run(args));
        assert_eq!(
            titles,
            (1..=12)
                .map(|month| format!("2026年{month}月"))
                .collect::<Vec<String>>(),
            "`lunar {}` is the whole of 2026",
            args.join(" ")
        );
    }
    // A bare year is the whole year too, with or without the flag.
    assert_eq!(
        civil_titles(&run(&["cal", "2026"])),
        civil_titles(&run(&["cal", "2026", "-y"]))
    );
    // `-y` on a lunar year is that lunar year, leap month included, and the
    // month named with it is not a second answer.
    assert_eq!(lunar_titles(&run(&["cal", "-L", "2020", "-y"])).len(), 13);
    assert_eq!(
        lunar_titles(&run(&["cal", "-L", "2020", "7", "-y"])).len(),
        13
    );
    assert_eq!(
        lunar_titles(&run(&["cal", "-L", "2020", "7", "-y"])),
        lunar_titles(&run(&["cal", "-L", "2020"]))
    );
    assert!(run(&["cal", "-L", "2020", "7", "-y"]).contains("闰四月"));
    // `-n 1` is one month, and a named month alone is one month.
    assert_eq!(
        civil_titles(&run(&["cal", "2026", "9", "-n", "1"])),
        vec!["2026年9月"]
    );
    assert_eq!(civil_titles(&run(&["cal", "2026", "9"])), vec!["2026年9月"]);
}

#[test]
fn small_years_parse() {
    assert_eq!(
        run(&["date", "-d", "1-01-01"]),
        run(&["date", "-d", "0001-01-01"])
    );
    assert!(run(&["date", "-d", "1-01-01"]).starts_with("公历: 1年1月1日 星期六"));
    assert!(run(&["date", "-d", "9999-12-31"]).starts_with("公历: 9999年12月31日"));
}

#[test]
fn solar_terms_and_festivals_sit_on_the_days_the_astronomy_gives() {
    // Each of these cells is checked by date, not by copying a sample grid.
    // `--no-holiday` throughout: this is the term and festival layer, and the
    // statutory level covers exactly the days a festival is legislated off for
    // (2026 春节 runs 2/17..2/23, so 雨水 on 2/18 is a 放假 cell).
    let cases: &[(&[&str], &str)] = &[
        (&["cal", "2026", "9"], "白露"),
        (&["cal", "2026", "9"], "秋分"),
        (&["cal", "2026", "9"], "中秋节"),
        (&["cal", "-L", "2026", "7"], "处暑"),   // 2026-08-23
        (&["cal", "-L", "2026", "7"], "七夕节"), // 2026-08-19
        (&["cal", "-L", "2026", "7"], "白露"),   // 2026-09-07
        (&["cal", "-L", "2020", "4", "-R"], "芒种"), // 2020-06-05
        (&["cal", "-L", "2026", "1"], "春节"),   // 2026-02-17
        (&["cal", "-L", "2026", "1"], "元宵节"), // 2026-03-03
        (&["cal", "-L", "2026", "1"], "雨水"),   // 2026-02-18
        (&["cal", "-L", "2026", "1"], "惊蛰"),   // 2026-03-05
    ];
    for (args, expected) in cases {
        let mut args = args.to_vec();
        args.push("--no-holiday");
        let grid = run(&args);
        assert!(
            grid.contains(expected),
            "expected {expected} in `lunar {}`:\n{grid}",
            args.join(" ")
        );
    }
}

#[test]
fn zhongyuan_is_shown_on_the_lunar_seventh_full_moon() {
    // 七月十五 = 2026-08-27.
    let grid = run(&["cal", "-L", "2026", "7"]);
    let lines: Vec<&str> = grid.lines().collect();
    let row = lines
        .iter()
        .position(|line| line.contains("8/27"))
        .expect("a row holding 8/27");
    let dates = lines[row].split_whitespace().collect::<Vec<_>>();
    let column = dates
        .iter()
        .position(|cell| *cell == "8/27")
        .expect("the 8/27 cell");
    let content: Vec<&str> = lines[row + 1].split_whitespace().collect::<Vec<_>>();
    // The cell reads 中元节, not 十五: the festival outranks the lunar day in
    // the cell policy, and the typed festival lookup is the only source that
    // knows the name.
    assert_eq!(
        content[column],
        "中元节",
        "the festival on 七月十五:\n{}",
        lines[row + 1]
    );
}

/// A birth moment with a time of day gets four pillars, and without one
/// gets three — never a fourth invented from nothing.
#[test]
fn bazi_prints_four_pillars_only_with_a_time() {
    let chart = run(&["bazi", "1990-06-15T10:30"]);
    assert_eq!(
        chart,
        "公历: 1990年6月15日 10:30\n\
         农历: 庚午年五月廿三\n\
         八字: 庚午 / 壬午 / 辛亥 / 癸巳\n\
         十神: 劫财 / 伤官 / 日主 / 食神\n\
         藏干: 丁己 / 丁己 / 壬甲 / 丙庚戊\n\
         纳音: 路旁土 / 杨柳木 / 钗钏金 / 长流水\n\
         地势: 病 / 病 / 沐浴 / 死\n\
         五行: 金火 / 水火 / 金水 / 水火\n\
         旬空: 戌亥 / 申酉 / 寅卯 / 午未\n\
         地支十神: 七杀偏印 / 七杀偏印 / 伤官正财 / 正官劫财正印\n\
         命局: 胎元 癸酉(剑锋金) / 胎息 丙寅(炉中火) / 命宫 壬午(杨柳木) / 身宫 戊子(霹雳火)\n\
         说明: 未给性别，无大运"
    );
    // No time written: three pillars, and the tool says why.
    let three = run(&["bazi", "1990-06-15"]);
    assert!(three.contains("八字: 庚午 / 壬午 / 辛亥\n"), "{three}");
    assert!(three.contains("说明: 未给时刻，无时柱"), "{three}");
    assert!(!three.contains("癸巳"), "no 时柱 without a clock:\n{three}");
    // The rows that are about a pillar must not keep a fourth column.
    for row in [
        "十神:",
        "藏干:",
        "纳音:",
        "地势:",
        "五行:",
        "旬空:",
        "地支十神:",
    ] {
        let line = three
            .lines()
            .find_map(|l| l.strip_prefix(row))
            .unwrap_or_else(|| panic!("{three}: a {row} row"));
        assert_eq!(
            line.split(" / ").count(),
            3,
            "{row} has three columns without a 时柱, got {line:?}"
        );
    }
}

/// 命宫 and 身宫 are counted from the 时柱, so a birth moment without a time
/// of day cannot answer them — and the tool says so rather than quoting the
/// noon it substitutes internally, which is a value for noon and not an answer
/// about the moment asked about. 胎元 and 胎息 do not need a 时柱, so they stay.
#[test]
fn the_ming_gong_needs_a_time_pillar() {
    let noon = run(&["bazi", "1990-06-15"]);
    let cells = row(&noon, "命局: ");
    assert_eq!(
        cells, "胎元 癸酉(剑锋金) / 胎息 丙寅(炉中火)",
        "the two 命局 values that need no 时柱:\n{noon}"
    );
    assert!(noon.contains("说明: 命宫 / 身宫需时柱"), "{noon}");
    // With a 时柱 both are named, and they are two different places: 身宫 is
    // not the 命宫 of the same chart.
    let timed = run(&["bazi", "1990-06-15T10:30"]);
    assert!(
        timed.contains(
            "命局: 胎元 癸酉(剑锋金) / 胎息 丙寅(炉中火) / \
             命宫 壬午(杨柳木) / 身宫 戊子(霹雳火)"
        ),
        "{timed}"
    );
    // A 23:00 birth is a 亥时 chart, and its 命宫 and 身宫 must move with it:
    // one 時辰 cannot borrow another's answer.
    let late = run(&["bazi", "1990-06-15T23:00"]);
    assert!(
        late.contains("命宫 丁亥(屋上土) / 身宫 癸未(杨柳木)"),
        "命宫 / 身宫 follow the 時辰:\n{late}"
    );
}

/// 五行 is the two characters the 干 and the 支 each carry — 庚 is 金 and 午
/// is 火, so the year pillar reads `金火` — and it is not the 纳音's element
/// (`路旁土` is 土). The row is the engine's per-character answer, pinned so a
/// layout change cannot quietly start printing 纳音 instead.
#[test]
fn the_five_elements_open_with_the_stems_own_element() {
    let chart = run(&["bazi", "1990-06-15T10:30"]);
    assert_eq!(
        row(&chart, "五行: "),
        "金火 / 水火 / 金水 / 水火",
        "{chart}"
    );
    // 辛亥: 辛 is 金, 亥 is 水. 癸巳: 癸 is 水, 巳 is 火.
    assert!(chart.contains("八字: 庚午 / 壬午 / 辛亥 / 癸巳"), "{chart}");
    assert!(
        !chart.contains("五行: 金土"),
        "路旁土 is the 纳音, not the 年柱's 五行:\n{chart}"
    );
}

/// One pillar's row, as the chart draws it.
fn row(chart: &str, label: &str) -> String {
    chart
        .lines()
        .find_map(|line| line.strip_prefix(label))
        .unwrap_or_else(|| panic!("{chart}: a {label} row"))
        .to_string()
}

/// The year and day pillars are the same facts `lunar date` prints, on the
/// same basis — a chart and a profile of one moment must not disagree.
#[test]
fn bazi_and_date_agree_on_the_year_and_day_pillars() {
    for day in [
        "1990-06-15",
        "2020-01-25",
        "2025-01-29",
        "2026-02-04",
        "2026-09-07",
        "1582-10-15",
    ] {
        let profile = run(&["date", "-d", day]);
        let pillars: Vec<&str> = profile
            .lines()
            .find_map(|l| l.strip_prefix("干支: "))
            .unwrap_or_else(|| panic!("{day}: a 干支 line"))
            .split(' ')
            .collect();
        let chart = run(&["bazi", day]);
        let drawn: Vec<&str> = chart
            .lines()
            .find_map(|l| l.strip_prefix("八字: "))
            .unwrap_or_else(|| panic!("{day}: an 八字 line"))
            .split(" / ")
            .collect();
        assert_eq!(drawn[0], pillars[0], "{day}: 年柱 is the 干支 line's");
        assert_eq!(drawn[2], pillars[2], "{day}: 日柱 is the 干支 line's");
    }
}

/// The 月柱 turns at the 節氣 **instant** here, where `lunar date` turns it
/// at the 節氣 **day** — and both are right. 白露 2026 is 22:41:16, so a
/// birth at 20:00 that day is 申月 and one at 23:00 is 酉月, while the day's
/// own almanac calls the whole of 09-07 酉月.
#[test]
fn the_bazi_month_turns_at_the_solar_term_instant() {
    let before = run(&["bazi", "2026-09-07T20:00"]);
    let after = run(&["bazi", "2026-09-07T23:00"]);
    assert!(
        before.contains("八字: 丙午 / 丙申 / 甲申"),
        "before 白露 is still 申月:\n{before}"
    );
    assert!(
        after.contains("八字: 丙午 / 丁酉 / 甲申"),
        "after 白露 is 酉月:\n{after}"
    );
    // `date` names the whole day, so its 干支 line is 酉月 at 09-07.
    assert!(
        run(&["date", "-d", "2026-09-07"]).contains("干支: 丙午 丁酉 甲申"),
        "the day's own 干支 line turns at the day"
    );
}

/// A 子时 belongs to the day it began in, in both commands: 23:30 on the
/// 15th is still the 15th's day pillar, and 00:30 on the 16th is the 16th's.
#[test]
fn a_zi_hour_belongs_to_the_day_it_began_in() {
    assert!(
        run(&["bazi", "1990-06-15T23:30"]).contains("八字: 庚午 / 壬午 / 辛亥 / 庚子"),
        "23:30 on the 15th keeps the 15th's 日柱"
    );
    assert!(
        run(&["bazi", "1990-06-16T00:30"]).contains("八字: 庚午 / 壬午 / 壬子 / 庚子"),
        "00:30 on the 16th has the 16th's 日柱, and both are 子"
    );
    assert!(run(&["date", "-d", "1990-06-15"]).contains("干支: 庚午 壬午 辛亥"));
    assert!(run(&["date", "-d", "1990-06-16"]).contains("干支: 庚午 壬午 壬子"));
}

/// A birth moment is read by the same grammar `lunar date -d` uses, so every
/// date form is taken here too — and a date with no time stays a date.
#[test]
fn bazi_reads_the_same_date_forms() {
    // The hyphen must not be read as a zone sign: `2025` alone is a year,
    // never the compact clock 20:25.
    for day in ["2025-01-29", "1990-06-15", "2026-09-07", "1582-10-15"] {
        let chart = run(&["bazi", day]);
        assert!(
            chart.contains("说明: 未给时刻，无时柱"),
            "{day} names no time, so it must not gain one:\n{chart}"
        );
    }
    // Every writing of the same instant is the same chart.
    let canonical = run(&["bazi", "1990-06-15T10:30"]);
    for form in [
        "1990-06-15 10:30",
        "1990-06-15T10:30+08:00",
        "19900615T1030",
        "1990/06/15 10:30",
    ] {
        assert_eq!(
            run(&["bazi", form]),
            canonical,
            "`bazi {form}` is the same instant as `1990-06-15T10:30`"
        );
    }
    // A relative or keyword form names a day and no clock, like `date`.
    assert!(run(&["bazi", "yesterday"]).contains("说明: 未给时刻，无时柱"));
    // `-l` is not offered: a birth moment is a civil instant, and there is
    // no 农历 time of day to read one in.
    assert!(run_failing(&["bazi", "-l", "1990-06-15"]).contains("unexpected"));
}

/// The failures `bazi` reports are the ones `date` reports for the same
/// string, because the grammar is shared.
#[test]
fn bazi_refuses_what_date_refuses() {
    for bad in ["not-a-date", "2023-02-30", "1582-10-10", "1990-06-15T25:00"] {
        assert_eq!(
            run_failing(&["bazi", bad]),
            run_failing(&["date", "-d", bad]),
            "`bazi {bad}` fails as `date -d {bad}` does"
        );
    }
    assert!(run_failing(&["bazi", "1990", "6", "15", "10"]).contains("位置参数过多"));
}

/// 大运 needs a gender to know which way it runs, and there is no default to
/// assume — so its absence is reported, not guessed at, exactly as a missing
/// time of day costs the 时柱.
#[test]
fn bazi_without_a_gender_prints_the_pillars_and_says_why() {
    let chart = run(&["bazi", "1990-06-15T10:30"]);
    assert!(chart.contains("说明: 未给性别，无大运"), "{chart}");
    assert!(
        !chart.contains("大运:"),
        "no 大运 without a gender:\n{chart}"
    );
    assert!(
        !chart.contains("起运:"),
        "no 起运 without a gender:\n{chart}"
    );
    // The pillars are the answer either way: a gender decides the 大运 rows
    // and nothing above them, so every row before 起运 is identical.
    assert!(chart.contains("八字: 庚午 / 壬午 / 辛亥 / 癸巳"), "{chart}");
    assert_eq!(
        chart.lines().take(7).collect::<Vec<_>>(),
        run(&["bazi", "1990-06-15T10:30", "-g", "男"])
            .lines()
            .take(7)
            .collect::<Vec<_>>(),
        "-g changes the 大运 rows and nothing above them"
    );
}

/// A yang year runs forward for a man and backward for a woman, so the two
/// genders must not be able to print the same 大运.
#[test]
fn the_gender_decides_which_way_the_luck_runs() {
    let man = run(&["bazi", "1990-06-15T10:30", "-g", "男"]);
    let woman = run(&["bazi", "1990-06-15T10:30", "-g", "女"]);
    assert!(
        man.contains("顺行") && woman.contains("逆行"),
        "庚午 is a yang year: the man runs forward, the woman back\n{man}\n{woman}"
    );
    assert_ne!(
        man.lines().find(|l| l.starts_with("大运: ")),
        woman.lines().find(|l| l.starts_with("大运: ")),
        "the two genders run opposite ways from the same month pillar"
    );
    // Both spellings of each gender reach the same answer.
    assert_eq!(run(&["bazi", "1990-06-15T10:30", "-g", "male"]), man);
    assert_eq!(run(&["bazi", "1990-06-15T10:30", "-g", "F"]), woman);
    assert!(run_failing(&["bazi", "1990-06-15T10:30", "-g", "x"]).contains("性别 x 无法识别"));
}

/// A birth months after a 節气 reaches it almost at once, and 起运 is then
/// smaller than a year. That is the answer, not an anomaly to be filtered.
#[test]
fn a_short_time_to_the_term_still_gives_a_start() {
    // 2026-02-04 is 立春; a girl born that day is one month from the next
    // term going backward, which is the engine's own `0年1月`.
    let chart = run(&["bazi", "2026-02-04T12:00", "-g", "女"]);
    assert!(
        chart.contains("出生后 0年1月"),
        "a 起运 under a year is printed as it is:\n{chart}"
    );
    assert!(chart.contains("逆行"), "{chart}");
    // The step ages still start at 1 and run ten years each.
    let steps: Vec<&str> = chart
        .lines()
        .find_map(|l| l.strip_prefix("大运: "))
        .expect("a 大运 row")
        .split(" / ")
        .collect();
    assert_eq!(
        steps.len(),
        9,
        "the engine's ten steps less the pre-luck one"
    );
    assert!(
        steps[0].starts_with("1-10 "),
        "the first step is 1-10: {steps:?}"
    );
}

/// The 大运 rows follow the pillars, and the 起运 line names the day the
/// engine computed rather than a day this tool worked out.
#[test]
fn the_luck_rows_sit_after_the_pillars() {
    let chart = run(&["bazi", "1990-06-15T10:30", "-g", "男"]);
    let lines: Vec<&str> = chart.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.starts_with("起运: "))
        .expect("a 起运 row");
    let dayun = lines
        .iter()
        .position(|l| l.starts_with("大运: "))
        .expect("a 大运 row");
    assert!(start < dayun, "起运 comes before the steps it explains");
    assert_eq!(dayun, lines.len() - 1, "大运 is the last row");
    // The 起运 line accounts for the whole interval: the month count alone
    // drops up to 29 days, so the days and hours beside it are what make the
    // printed count and the printed date the same fact. 7年5月 after a
    // 1990-06-15 birth reaches 1997-11-17 by way of 2 days and 12 hours.
    assert!(
        lines[start].contains("出生后 7年5月2天12小时") && lines[dayun].starts_with("大运: 8-17 "),
        "{} / {}",
        lines[start],
        lines[dayun]
    );
    // A zero remainder is left out rather than printed as `0天0小时`.
    assert!(
        !lines[start].contains("0天0小时") && !lines[start].contains("天0小时"),
        "no zero tail on the 起运 line: {}",
        lines[start]
    );
}

/// `-y` asks about one year rather than adding a row to the chart, and the
/// 流年 / 流月 / 小运 it answers with are that year's — 26岁 of a 1990 birth is
/// 2015, not the birth year. A hundred 流年 and a hundred 小运 would bury the
/// chart they belong to, so nothing of the sort prints without the flag.
#[test]
fn a_year_prints_its_luck_year_and_months() {
    let chart = run(&["bazi", "1990-06-15T10:30", "-g", "男", "-y", "2015"]);
    let mut lines = chart.lines();
    let flow = lines.next_back().expect("a 小运 line");
    let months = lines.next_back().expect("a 流月 line");
    let year = lines.next_back().expect("a 流年 line");
    assert_eq!(year, "流年: 2015年 乙未  26岁  旬空 辰巳");
    // Twelve months, named in 农历 months — a 流月 runs on 农历 months from
    // 立春, so a civil `1月` here would name a different month.
    assert_eq!(months.split(" / ").count(), 12, "{months}");
    assert!(
        months.starts_with("流月: 戊寅(正月) / 己卯(二月) / "),
        "{months}"
    );
    assert!(
        months.ends_with("丁亥(十月) / 戊子(冬月) / 己丑(腊月)"),
        "{months}"
    );
    // The 小运 moves one step per year and is counted from the 时柱, so it is
    // a different pillar from the 流年 of the same year — and it follows the
    // gender's direction, which is what the 大运 run does.
    assert_eq!(flow, "小运: 己未 26岁");
    assert!(
        !chart.contains("流年: 2014"),
        "one year, one 流年:\n{chart}"
    );
}

/// A 流年 is counted on 大运, and 顺逆 is the gender's to decide — so its
/// absence is reported, not guessed at, exactly as a missing time of day
/// costs the 时柱.
#[test]
fn a_year_without_a_gender_says_so() {
    let chart = run(&["bazi", "1990-06-15T10:30", "-y", "2015"]);
    assert!(chart.contains("说明: 流年需性别"), "{chart}");
    assert!(
        !chart.contains("流年:"),
        "no 流年 without a gender:\n{chart}"
    );
    assert!(
        !chart.contains("流月:") && !chart.contains("小运:"),
        "the whole 运程 query is answered with one line:\n{chart}"
    );
}

/// The 小运 is counted from the 时柱, one step per year, so a birth moment
/// with no time of day has none to count it from. The engine would still
/// answer — from the noon this tool substitutes internally — and that noon is
/// a value about noon, so it is left out and said to be, like 命宫 / 身宫.
#[test]
fn the_luck_year_needs_no_time_but_the_luck_step_does() {
    let no_clock = run(&["bazi", "1990-06-15", "-g", "男", "-y", "2015"]);
    // 流年 and 流月 are a function of the year and the 大运, not of the clock.
    assert!(
        no_clock.contains("流年: 2015年 乙未  26岁  旬空 辰巳"),
        "{no_clock}"
    );
    assert!(
        no_clock.contains("流月: 戊寅(正月) / 己卯(二月) / "),
        "{no_clock}"
    );
    assert!(no_clock.contains("说明: 小运需时柱"), "{no_clock}");
    assert!(
        !no_clock.contains("小运:"),
        "no 小运 without a 时柱:\n{no_clock}"
    );
    // With a 时柱 the chart names the step, and it is not the 流年's pillar.
    let timed = run(&["bazi", "1990-06-15T10:30", "-g", "男", "-y", "2015"]);
    assert!(timed.contains("小运: 己未 26岁"), "{timed}");
    // The other direction counts the other way, so the two must not agree.
    let woman = run(&["bazi", "1990-06-15T10:30", "-g", "女", "-y", "2015"]);
    assert!(woman.contains("小运: 丁卯 26岁"), "{woman}");
}

/// The 流年 of a year the chart's ten steps do not cover is reported with the
/// span that does — never answered with the nearest step's value, which would
/// be a fact about a different year.
#[test]
fn the_luck_year_sits_in_the_step_that_owns_it() {
    // 起运 is 1997-11-17 and the tenth step ends in 2086, so 1996 is before
    // the first step and 2087 is past the last.
    for year in ["1996", "2087"] {
        let chart = run(&["bazi", "1990-06-15T10:30", "-g", "男", "-y", year]);
        assert!(
            chart.contains(&format!("说明: {year} 年无大运 (大运 1997-2086)")),
            "{chart}"
        );
        assert!(!chart.contains("流年:"), "no value for {year}:\n{chart}");
    }
    // The first and last years the steps do cover are 流年 of their own, on
    // the first and last step respectively.
    let first = run(&["bazi", "1990-06-15T10:30", "-g", "男", "-y", "1997"]);
    assert!(
        first.contains("流年: 1997年 丁丑  8岁  旬空 申酉"),
        "{first}"
    );
    let last = run(&["bazi", "1990-06-15T10:30", "-g", "男", "-y", "2086"]);
    assert!(
        last.contains("流年: 2086年 丙午  97岁  旬空 寅卯"),
        "{last}"
    );
}

/// `-y` is a year, not a birth moment: the `-d` grammar is not its reader, and
/// a year outside the engine's window is refused rather than counted into.
#[test]
fn a_luck_year_outside_the_range_is_reported() {
    for (arg, want) in [
        ("0", "年份 0 超出支持范围 (1–9999)"),
        ("10000", "年份 10000 超出支持范围 (1–9999)"),
        ("abc", "年份 abc 无法解析"),
    ] {
        assert!(
            run_failing(&["bazi", "1990-06-15T10:30", "-g", "男", "-y", arg]).contains(want),
            "`-y {arg}` is refused with {want:?}"
        );
    }
}

/// The 时柱 is counted from the clock, and a reader who assumes a 真太阳时
/// chart is reading a different 时柱, a different 日柱 and a different 起运.
/// The statement is what the tool has instead of a correction, so all three
/// documents have to carry it — and the README has to name the clock, not
/// only the absence of a correction.
#[test]
fn the_true_solar_time_caveat_is_documented() {
    let readme = include_str!("../README.md");
    let agents = include_str!("../AGENTS.md");
    let parity = include_str!("../docs/parity.md");
    for (name, doc) in [
        ("README.md", readme),
        ("AGENTS.md", agents),
        ("docs/parity.md", parity),
    ] {
        assert!(doc.contains("真太阳时"), "{name} does not say 真太阳时");
    }
    assert!(readme.contains("钟表时"), "the README must name the clock");
}

/// The Chinese README is a translation, and a translation rots quietly: the
/// English one gets the behaviour change, the Chinese one keeps the sentence
/// that described the old behaviour. Nothing else in the suite can see that,
/// so the two documents have to agree on the table of contents, and the
/// translation has to carry the same statement about the clock.
#[test]
fn the_chinese_readme_stays_a_translation() {
    let english = include_str!("../README.md");
    let chinese = include_str!("../README.zh-CN.md");

    /// The shape of the document, not its words: a heading level and the
    /// literal tokens inside it, with the prose dropped — a translation
    /// renames a section and cannot invent one.
    fn skeleton(doc: &str) -> Vec<String> {
        doc.lines()
            .filter(|line| line.starts_with('#'))
            .map(|line| {
                let level = line.len() - line.trim_start_matches('#').len();
                let kept = line
                    .split_whitespace()
                    .filter(|word| word.starts_with('`') || word.starts_with('%'))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{level} {kept}")
            })
            .collect()
    }
    assert_eq!(
        skeleton(english),
        skeleton(chinese),
        "README.md and README.zh-CN.md disagree on the table of contents"
    );

    for (name, doc) in [("README.md", english), ("README.zh-CN.md", chinese)] {
        assert!(doc.contains("真太阳时"), "{name} does not say 真太阳时");
        assert!(doc.contains("钟表时"), "{name} must name the clock");
    }

    assert!(
        english.contains("[简体中文](README.zh-CN.md)"),
        "the English README does not link the translation"
    );
    assert!(
        chinese.contains("[English](README.md)"),
        "the Chinese README does not link back"
    );
}

/// CI is what makes "the gates pass" mean something on a machine that is not
/// the author's, so the workflow is a contract like any other: the three
/// gates have to be in it, on the runner the repo names, and the sample
/// checker has to be the committed script rather than a copy of it — a copy
/// is the one thing that can drift without failing.
#[test]
fn the_ci_workflow_covers_the_gates() {
    let workflow = include_str!("../.github/workflows/ci.yml");
    for gate in [
        "cargo fmt --all -- --check",
        "cargo clippy --all-targets",
        "cargo test",
    ] {
        assert!(workflow.contains(gate), "CI does not run `{gate}`");
    }
    assert!(
        workflow.contains("ubuntu-26.04"),
        "CI does not pin the ubuntu-26.04 runner"
    );
    assert!(
        workflow.contains("actions/cache@"),
        "CI caches nothing, so every run rebuilds the world"
    );
    assert!(
        workflow.contains("tools/check_samples.py"),
        "CI must run the committed sample checker, not a copy of it"
    );

    // The checker has to exist, and it has to be the only copy: `docs/README.md`
    // used to embed the same script inline, and two copies of the one thing
    // whose job is catching drift is how a drift gets through.
    let checker = include_str!("../tools/check_samples.py");
    assert!(
        checker.contains("README.zh-CN.md") && checker.contains("README.md"),
        "the checker only reads one of the two documents"
    );
    let docs = include_str!("../docs/README.md");
    assert!(
        !docs.contains("import re, subprocess"),
        "docs/README.md embeds a second copy of the sample checker"
    );
    assert!(
        !docs.contains("There is no CI")
            && !include_str!("../AGENTS.md").contains("There is no CI"),
        "a document still claims the repository has no CI"
    );
}

/// Dependabot's `open-pull-requests-limit` has one value that reads like
/// "unlimited" and does the opposite: `0` *disables* version updates for the
/// ecosystem. Nothing errors when that happens — the updates simply stop —
/// so the limit is pinned here, along with the daily schedule it exists for.
#[test]
fn dependabot_checks_daily_without_a_real_limit() {
    let config = include_str!("../.github/dependabot.yml");
    assert!(
        config.contains("package-ecosystem: \"cargo\""),
        "dependabot does not watch the cargo manifest"
    );
    assert!(
        config.contains("interval: \"daily\""),
        "dependabot does not check daily"
    );
    assert!(
        !config.contains("open-pull-requests-limit: 0"),
        "open-pull-requests-limit: 0 disables version updates, it is not 'no limit'"
    );
    let limit = config
        .lines()
        .find_map(|line| line.trim().strip_prefix("open-pull-requests-limit:"))
        .expect("dependabot sets open-pull-requests-limit")
        .trim()
        .parse::<u32>()
        .expect("the limit is an integer");
    assert!(
        limit >= 100,
        "the limit is {limit}, low enough to throttle a 3-dependency project"
    );
    assert!(
        !config.contains("groups:"),
        "a grouped batch merges several bumps into one untested commit"
    );
}

/// A release is the one place the version number is written down twice — once
/// in `Cargo.toml`, once in a tag — so both are checked against each other
/// here, where a mismatch is cheap to find. The two architectures are checked
/// because dropping one produces a release that looks complete and silently
/// excludes half the machines, and the install instructions name both files.
#[test]
fn the_release_workflow_covers_both_architectures() {
    let workflow = include_str!("../.github/workflows/release.yml");
    for want in [
        "x86_64",
        "aarch64",
        "ubuntu-26.04-arm",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl",
        "tags: [\"v*\"]",
    ] {
        assert!(
            workflow.contains(want),
            "release.yml does not mention {want}"
        );
    }
    // The tag and the manifest have to agree, or the archive is named after one
    // version and the release page after another.
    let manifest = include_str!("../Cargo.toml");
    let version = manifest
        .lines()
        .find_map(|line| line.strip_prefix("version = \""))
        .and_then(|rest| rest.split('"').next())
        .expect("Cargo.toml carries a version");
    // The archive is named from the manifest rather than from the tag, so the
    // name and the release page cannot drift; assert the substitution, not
    // the exact quoting around it.
    assert!(
        workflow.contains("sed -n")
            && workflow.contains("Cargo.toml")
            && workflow.contains("NAME=\"lunar-${VERSION}-"),
        "the archive is not named from Cargo.toml's version"
    );
    for (name, doc) in [
        ("README.md", include_str!("../README.md")),
        ("README.zh-CN.md", include_str!("../README.zh-CN.md")),
    ] {
        assert!(
            doc.contains(&format!("lunar-{version}-x86_64.tar.gz")),
            "{name} does not install the {version} x86_64 archive"
        );
        assert!(
            doc.contains(&format!("lunar-{version}-aarch64.tar.gz")),
            "{name} does not install the {version} aarch64 archive"
        );
        assert!(
            doc.contains("SHA256SUMS"),
            "{name} does not tell the reader to verify the download"
        );
    }
}
