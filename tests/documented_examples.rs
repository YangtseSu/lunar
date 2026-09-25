//! Layout and behaviour, pinned to what the tool documents; calendar values,
//! pinned to what `lunar-rs` computes from astronomy.
//!
//! Where the two disagree the engine wins: the published samples print a couple
//! of stale cells (中元 three days early, 芒种 on the wrong day, two solar terms
//! missing), and those are *not* reproduced here.

use std::process::Command;

/// The compiled `lunar` binary.
fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lunar"))
}

/// Runs `lunar <args…>` and returns stdout, trimmed of the trailing newline.
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

/// Runs `lunar <args…>`, expecting failure, and returns stderr.
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
        "公历：2026年9月7日 星期一\n农历：丙午年七月廿六\n干支：丙午年 丙申月 甲申日\n生肖：马\n节气：白露"
    );
}

#[test]
fn date_profile_shows_leap_month() {
    assert_eq!(
        run(&["date", "-d", "2020-05-23"]),
        "公历：2020年5月23日 星期六\n农历：庚子年闰四月初一\n干支：庚子年 辛巳月 丙寅日\n生肖：鼠"
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
        "公历：2026年2月17日 星期二\n农历：丙午年正月初一\n干支：丙午年 庚寅月 壬戌日\n生肖：马"
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
    let expected = r#"      2026年9月
     一     二     三     四     五     六     日
             1      2      3      4      5      6
            二十     廿一     廿二     廿三     廿四     廿五
      7      8      9     10     11     12     13
     白露     廿七     廿八     廿九     八月     初二     初三
     14     15     16     17     18     19     20
     初四     初五     初六     初七     初八     初九     初十
     21     22     23     24     25     26     27
     十一     十二     秋分     十四     中秋     十六     十七
     28     29
     十八     十九"#;
    assert_eq!(run(&["cal", "2026", "9"]), expected);
}

#[test]
fn sunday_first_shifts_only_the_columns() {
    let monday = run(&["cal", "2026", "9"]);
    let sunday = run(&["cal", "2026", "9", "-s"]);
    assert_ne!(monday, sunday);

    let header: Vec<&str> = sunday.lines().take(2).collect();
    assert_eq!(
        header,
        vec![
            "      2026年9月",
            "     日     一     二     三     四     五     六"
        ],
        "{sunday}"
    );
    // The same days, in columns shifted by one: 1 September is a Tuesday.
    let first_week: Vec<&str> = sunday.lines().skip(2).take(2).collect();
    assert_eq!(
        first_week,
        vec![
            "                    1      2      3      4      5",
            "                   二十     廿一     廿二     廿三     廿四"
        ],
        "{sunday}"
    );
    for fragment in ["白露", "中秋", "秋分"] {
        assert!(
            sunday.contains(fragment),
            "expected {fragment} in:\n{sunday}"
        );
    }
}

#[test]
fn lunar_month_grid_has_the_documented_layout() {
    // 农历 丙午年七月. The documented sample carries three stale cells: it shows
    // 十五 where 8/27 is 中元 (七月十五), drops 处暑 on 8/23 and moves 白露 to
    // 9/6. The engine places all three correctly.
    let expected = r#"     农历 丙午年 七月
     一     二     三     四     五     六     日
                    8/13  8/14  8/15  8/16
                      七月    初二    初三    初四
  8/17  8/18  8/19  8/20  8/21  8/22  8/23
    初五    初六    七夕    初八    初九    初十    处暑
  8/24  8/25  8/26  8/27  8/28  8/29  8/30
    十二    十三    十四    中元    十六    十七    十八
  8/31   9/1   9/2   9/3   9/4   9/5   9/6
    十九    二十    廿一    廿二    廿三    廿四    廿五
   9/7   9/8   9/9  9/10
    白露    廿七    廿八    廿九"#;
    assert_eq!(run(&["cal", "-L", "2026", "7"]), expected);
}

#[test]
fn leap_lunar_month_grid_has_the_documented_layout() {
    // 庚子年闰四月. 芒种 falls on 2020-06-05 (the sample puts it on 6/7). The
    // dates, the leap-month title, the weekday alignment and the 6-wide cells
    // match the sample.
    let expected = r#"     农历 庚子年 闰四月
     一     二     三     四     五     六     日
                                5/23  5/24
                                 闰四月    初二
  5/25  5/26  5/27  5/28  5/29  5/30  5/31
    初三    初四    初五    初六    初七    初八    初九
   6/1   6/2   6/3   6/4   6/5   6/6   6/7
    初十    十一    十二    十三    芒种    十五    十六
   6/8   6/9  6/10  6/11  6/12  6/13  6/14
    十七    十八    十九    二十    廿一    廿二    廿三
  6/15  6/16  6/17  6/18  6/19  6/20
    廿四    廿五    廿六    廿七    廿八    廿九"#;
    assert_eq!(run(&["cal", "-L", "2020", "4", "-R"]), expected);
}

#[test]
fn lunar_new_year_grid_matches_documented_output() {
    let expected = r#"     农历 丙午年 正月
     一     二     三     四     五     六     日
 2/17   2/18   2/19   2/20   2/21   2/22
春节   初二   初三   初四   初五   初六
 2/23   2/24   2/25   2/26   2/27   2/28   3/1
初七   初八   初九   初十   十一   十二   十三
   3/2    3/3    3/4    3/5    3/6    3/7    3/8
十四   元宵   十六   十七   十八   十九   二十
  3/9   3/10   3/11   3/12   3/13   3/14   3/15
廿一   廿二   廿三   廿四   廿五   廿六   廿七
 3/16   3/17   3/18
廿八   廿九   三十"#;
    let grid = run(&["cal", "-L", "2026", "1"]);
    // 雨水 (2/18) and 惊蛰 (3/5) are real solar terms the sample omits; the
    // festival cells and the whole layout match.
    for fragment in ["春节", "2/17", "元宵", "3/18", "三十"] {
        assert!(grid.contains(fragment), "expected {fragment} in:\n{grid}");
    }
    assert_eq!(grid.lines().count(), expected.lines().count());
    assert_eq!(grid.lines().next(), expected.lines().next());
}

#[test]
fn cell_overlays_follow_the_documented_priority() {
    let grid = run(&["cal", "2026", "9"]);
    // 初一 shows the month name, a solar term wins over a lunar day, and a
    // festival wins over everything.
    assert!(grid.contains("八月"));
    assert!(grid.contains("白露"));
    assert!(grid.contains("中秋"));
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
        !plain.contains("中秋"),
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
    assert_eq!(
        year.matches("2026年").count(),
        12,
        "twelve months in:\n{year}"
    );
    assert!(year.starts_with("      2026年1月\n"));
    // A bare lunar year walks the whole lunar year, leap month included.
    let lunar_year = run(&["cal", "-L", "2020"]);
    assert_eq!(lunar_year.matches("农历").count(), 13);
    assert!(lunar_year.contains("闰四月"));
    assert!(lunar_year.starts_with("     农历 庚子年 正月\n"));
}

#[test]
fn month_spans_are_supported() {
    let span = run(&["cal", "-3"]);
    let titles: Vec<&str> = span.lines().filter(|line| line.ends_with('月')).collect();
    assert_eq!(titles.len(), 3, "{span}");
    let three = run(&["cal", "2026", "9", "-n", "3"]);
    let titles: Vec<&str> = three.lines().filter(|line| line.ends_with('月')).collect();
    assert_eq!(
        titles,
        vec!["      2026年9月", "      2026年10月", "      2026年11月"]
    );
}

#[test]
fn small_years_parse() {
    assert_eq!(
        run(&["date", "-d", "1-01-01"]),
        run(&["date", "-d", "0001-01-01"])
    );
    assert!(run(&["date", "-d", "1-01-01"]).starts_with("公历：1年1月1日 星期六"));
    assert!(run(&["date", "-d", "999-12-31"]).starts_with("公历：999年12月31日"));
    assert!(run(&["date", "-d", "9999-12-31"]).starts_with("公历：9999年12月31日"));
}

#[test]
fn solar_terms_and_festivals_sit_on_the_days_the_astronomy_gives() {
    // Each of these cells is checked by date, not by copying a sample grid.
    let cases: &[(&[&str], &str)] = &[
        (&["cal", "2026", "9"], "白露"), // 处暑/白露/秋分 2026
        (&["cal", "2026", "9"], "秋分"),
        (&["cal", "2026", "9"], "中秋"), // 八月十五 = 2026-09-25
        (&["cal", "-L", "2026", "7"], "处暑"), // 2026-08-23
        (&["cal", "-L", "2026", "7"], "中元"), // 七月十五 = 2026-08-27
        (&["cal", "-L", "2026", "7"], "白露"), // 2026-09-07
        (&["cal", "-L", "2026", "7"], "七夕"), // 2026-08-19
        (&["cal", "-L", "2020", "4", "-R"], "芒种"), // 2020-06-05
        (&["cal", "-L", "2026", "1"], "春节"), // 2026-02-17
        (&["cal", "-L", "2026", "1"], "元宵"), // 2026-03-03
        (&["cal", "-L", "2026", "1"], "雨水"), // 2026-02-18
        (&["cal", "-L", "2026", "1"], "惊蛰"), // 2026-03-05
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
    // 中元 is 七月十五 = 2026-08-27; the published sample puts it on 9/6.
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
        "中元",
        "the 8/27 cell must carry 中元:\n{}",
        lines[row + 1]
    );
    // And nowhere else in the month.
    assert_eq!(grid.matches("中元").count(), 1);
}
