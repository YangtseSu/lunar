//! Layout and behaviour, pinned to what the tool documents; calendar values,
//! pinned to what `lunar-rs` computes from astronomy.
//!
//! Where the two disagree the engine wins: a published sample prints a couple
//! of stale cells (中元 three days early, 芒种 on the wrong day, two solar
//! terms missing), and those are *not* reproduced here.
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
fn split_on_pitch(line: &str, pitch: usize) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut columns = 0;
    for (byte, c) in line.char_indices() {
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
#[test]
fn the_usual_civil_festivals_appear() {
    let mut year = String::new();
    for month in 1..=12 {
        year.push_str(&run(&["cal", "2026", &month.to_string()]));
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
        "公历: 2026年2月17日 星期二\n农历: 丙午年正月初一\n干支: 丙午 庚寅 壬戌\n生肖: 马"
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

#[test]
fn lunar_month_grid_has_the_documented_layout() {
    let expected = r#"农历 丙午年 七月
一      二      三      四      五      六      日
                        8/13    8/14    8/15    8/16
                        初一    初二    初三    初四
8/17    8/18    8/19    8/20    8/21    8/22    8/23
初五    初六    七夕节  初八    初九    初十    处暑
8/24    8/25    8/26    8/27    8/28    8/29    8/30
十二    十三    十四    十五    十六    十七    十八
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
    let plain = run(&["cal", "2026", "9", "--no-month-name", "--no-festival"]);
    assert!(
        !plain.contains("八月"),
        "month name overlay disabled:\n{plain}"
    );
    assert!(
        !plain.contains("中秋节"),
        "festival overlay disabled:\n{plain}"
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

#[test]
fn out_of_range_years_are_rejected() {
    assert!(run_failing(&["cal", "10000", "1"]).contains("超出支持范围"));
    assert!(run_failing(&["cal", "0", "1"]).contains("超出支持范围"));
    // The 1582 reform gap does not exist in the Gregorian calendar.
    assert!(run_failing(&["date", "-d", "1582-10-10"]).contains("不存在"));
    assert!(run_failing(&["date", "-d", "2023-02-30"]).contains("无法解析"));
}

#[test]
fn unsupported_date_strings_are_reported() {
    assert!(run_failing(&["date", "-d", "definitely not a date"]).contains("无法解析"));
}

#[test]
fn help_format_lists_the_tokens() {
    let help = run(&["date", "--help-format"]);
    assert!(help.contains("%Y 公历年"));
    assert!(help.contains("%Q 节气"));
    assert!(help.contains("\\n 换行"));
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
        let grid = run(args);
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
    assert_eq!(
        content[column],
        "十五",
        "the lunar day:\n{}",
        lines[row + 1]
    );
}
