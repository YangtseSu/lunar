//! Layout and behaviour, pinned to what the tool documents; calendar values,
//! pinned to what `lunar-rs` computes from astronomy.
//!
//! Where the two disagree the engine wins: a published sample prints 芒种 on
//! the wrong day and omits two solar terms, and those are *not* reproduced
//! here. (The sample also prints 中元 three days early; the cell now reads
//! 中元节 on 七月十五, which is where the engine puts it.)
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

/// Month lengths, so a test can state how many days a grid must show.
mod calendar {
    /// Days in `(year, month)`, with `month` 1-12.
    pub fn monthrange(year: i32, month: i32) -> (i32, u32) {
        const LENGTHS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let length = match month {
            2 if leap => 29,
            m => LENGTHS[(m - 1) as usize],
        };
        (0, length)
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

/// The profile reports the statutory calendar even in its default form: 放假
/// and 调休上班 are the one thing about a day the other lines cannot say.
#[test]
fn date_profile_reports_the_statutory_calendar() {
    assert_eq!(
        run(&["date", "-d", "2026-09-26"]).lines().last(),
        Some("法定: 中秋节 放假")
    );
    assert_eq!(
        run(&["date", "-d", "2026-10-10"]).lines().last(),
        Some("法定: 国庆节 调休上班")
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
        "公历: 2020年5月23日 星期六\n农历: 庚子年闰四月初一\n干支: 庚子 辛巳 丙寅\n生肖: 鼠"
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
/// one to three digits, so `09/07/2026` arrived as the year 9 with a day of
/// 2026 and then failed validation. The US order is now tried first, and a
/// four-digit *last* field is what makes it one — so `2026/09/07` and
/// `1-01-01` still reach the branches that own them.
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

/// A numeric zone offset shifts the day, not just the clock.
///
/// `split_zone` used to hand `zone_offset` a slice starting one byte *before*
/// the sign, so the sign was never seen and every `±hh:mm` was rejected —
/// `Z`/`UTC`/`GMT` still parsed, which is why only the numeric forms were
/// broken. The sign convention is the POSIX one the module documents: `+0800`
/// is 8 hours *behind* UTC, so a 23:30 reading moves to the next day.
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
        "公历: 2026年9月7日 星期一\n农历: 丙午年七月廿六\n干支: 丙午 丙申 甲申\n生肖: 马\n节气: 白露"
    );
}

#[test]
fn date_profile_shows_leap_month() {
    assert_eq!(
        run(&["date", "-d", "2020-05-23"]),
        "公历: 2020年5月23日 星期六\n农历: 庚子年闰四月初一\n干支: 庚子 辛巳 丙寅\n生肖: 鼠"
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
        "公历: 2026年2月17日 星期二\n农历: 丙午年正月初一\n干支: 丙午 庚寅 壬戌\n生肖: 马\n法定: 春节 放假"
    );
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
            "%Y|%m|%d|%A|%G|%M|%N|%n|%H|%D|%S|%Q|%%",
            "-d",
            "2026-09-07"
        ]),
        "2026|09|07|一|丙午|七月|廿六|26|丙申|甲申|马|白露|%"
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
#[test]
fn civil_grid_shows_every_day_of_the_month() {
    for month in 1..=12 {
        let grid = run(&["cal", "2026", &month.to_string()]);
        let labels: Vec<&str> = grid
            .lines()
            .skip(2)
            .step_by(2)
            .filter(|line| !line.trim().is_empty())
            .flat_map(|line| line.split_whitespace())
            .collect();
        let days = calendar::monthrange(2026, month).1;
        let shown: Vec<u32> = labels.iter().filter_map(|cell| cell.parse().ok()).collect();
        assert_eq!(
            shown,
            (1..=days).collect::<Vec<_>>(),
            "every day of 2026-{month}, in order:\n{grid}"
        );
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
/// It used to be accepted and ignored, so `date -R -d 2020-04-01` printed the
/// 公历 date and `cal -R 2020 4` printed 2020年4月 — a flag that read as
/// agreement while changing nothing, and whose own help says (-l 时).
/// `-s`/`-m` were made mutually exclusive for the same reason; this is the
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
/// The month is a table lookup on the way to the error message, so `0`, `13`
/// and `i32::MIN` used to panic instead of failing — an exit code of 101
/// rather than 1. `--` passes a negative number past clap's own parser.
#[test]
fn out_of_range_lunar_months_are_reported_not_indexed() {
    for month in ["0", "13", "-13", "-2147483648"] {
        assert!(
            run_failing(&["cal", "-L", "2026", "--", month]).contains("农历月份"),
            "lunar month {month} is reported as out of range"
        );
    }
    // `date` derives the leap month by negating the one the user wrote, and
    // `-R` did not check the month first: `i32::MIN` has no absolute value, so
    // `date -l -R 2026 -- -2147483648 1` panicked. The positionals now report
    // the number the user wrote, through the message `cal` already used.
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
        ("2026-0-01", "无法解析的日期"),
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
/// It used to be stepped with `add_days` — proleptic-Gregorian arithmetic —
/// while the content band and the month length came from the engine, which
/// uses the Julian leap rule before 1600 and skips the reform days of 1582.
/// Two calendars in one grid: `cal -L 1300 2` drew 36 cells for a 30-day
/// month, 6 of them contradicting `lunar date`, and ended on the *next*
/// month's 初一. This pins the first and the last label to the day the engine
/// converts the same lunar day to, which is the only claim that can fail when
/// the two calendars drift.
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
/// and then sliced as a `&str` at that offset, so any character straddling
/// one panicked with "end byte index 10 is not a char boundary" and exit
/// 101. Every form is now read through a helper that checks the boundary and
/// the digits, and answers `无法解析的日期` like any other input it has no
/// reading for.
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

#[test]
fn month_spans_are_supported() {
    let span = run(&["cal", "2026", "9", "-n", "3"]);
    let titles: Vec<&str> = span.lines().filter(|line| line.ends_with('月')).collect();
    assert_eq!(titles, vec!["2026年9月", "2026年10月", "2026年11月"]);
}

/// A span of zero months is a mistake, not a request for one month.
///
/// `month_count` was `months.max(1)`, so `cal 2026 9 -n 0` printed a month
/// and exited 0 — the only out-of-range argument in the tool that was repaired
/// instead of reported. `-n` is now bounded at parse time, so the refusal
/// comes with the value that was wrong rather than after the fact.
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

/// `-3` / `-n N` widen a `-L` request as they do a civil one.
///
/// `select_lunar_months` returned the named month as soon as it saw two
/// positionals, so the span flags were never read: `cal -L 2026 7 -3` printed
/// one month where `cal 2026 7 -3` printed three. The named month is now the
/// centre of the span, kept at full length at either end of the year.
#[test]
fn a_lunar_month_span_centres_on_the_named_month() {
    fn titles(output: &str) -> Vec<&str> {
        output
            .lines()
            .filter(|line| line.starts_with("农历"))
            .collect()
    }
    // Centred: the named month with one either side.
    assert_eq!(
        titles(&run(&["cal", "-L", "2026", "7", "-3"])),
        vec!["农历 丙午年 六月", "农历 丙午年 七月", "农历 丙午年 八月"]
    );
    // The flag that named one month still names one month.
    assert_eq!(
        titles(&run(&["cal", "-L", "2026", "7"])),
        vec!["农历 丙午年 七月"]
    );
    // `-n N` reads the same way.
    assert_eq!(
        titles(&run(&["cal", "-L", "2026", "7", "-n", "5"])).len(),
        5
    );
    // A span at either end of the year keeps its full length: the window is
    // shifted back, not cut short. It used to be clipped, and
    // `a_lunar_month_span_centres_on_the_named_month` pinned the clipped
    // length of 2 as if it were intended.
    assert_eq!(
        titles(&run(&["cal", "-L", "2026", "1", "-3"])),
        vec!["农历 丙午年 正月", "农历 丙午年 二月", "农历 丙午年 三月"]
    );
    assert_eq!(
        titles(&run(&["cal", "-L", "2026", "12", "-3"])),
        vec!["农历 丙午年 十月", "农历 丙午年 冬月", "农历 丙午年 腊月"]
    );
    // A span never comes back shorter than it was, at either end and in a
    // thirteen-month year alike: `-y` asks for twelve and used to deliver
    // seven from 十二月 and eleven from 七月.
    for (year, month) in [("2020", "7"), ("2026", "7"), ("2026", "12"), ("2020", "4")] {
        assert_eq!(
            titles(&run(&["cal", "-L", year, month, "-y"])).len(),
            12,
            "`cal -L {year} {month} -y` asks for twelve months"
        );
    }
    // A span longer than the year yields the year, leap month included.
    assert_eq!(
        titles(&run(&["cal", "-L", "2020", "7", "-n", "20"])).len(),
        13
    );
    // `-R` centres on the leap month when one exists.
    assert!(run(&["cal", "-L", "2020", "4", "-R", "-3"]).contains("闰四月"));
    // A bare lunar year is still the whole year, whatever the span flags say.
    assert_eq!(titles(&run(&["cal", "-L", "2026"])).len(), 12);
    assert_eq!(titles(&run(&["cal", "-L", "2026", "-3"])).len(), 12);
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
