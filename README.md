# lunar

A Chinese lunisolar calendar command line tool:

- `lunar date` — one day's almanac profile (公历 / 星期 / 农历 / 干支 / 生肖 / 节气)
  plus a custom format mode;
- `lunar cal` — month and year grids with lunar days, solar terms and festivals,
  over civil months or, with `-L`, lunar months.

All calendar computation is delegated to [`lunar-rs`](https://crates.io/crates/lunar-rs),
a pure-Rust port of the 寿星天文历 engine: it computes the solar/lunar conversion,
the 24 solar terms, ganzhi, zodiac and the traditional festivals from astronomy
rather than from a table of leap months.

## Build and run

```bash
cargo build --release
./target/release/lunar date
./target/release/lunar cal
```

## `lunar date`

```console
$ lunar date -d 2026-09-07
公历: 2026年9月7日 星期一
农历: 丙午年七月廿六
干支: 丙午 丙申 甲申
生肖: 马
节气: 白露
```

The `节气` line is omitted when the day carries no solar term. A leap month shows
up in the month name:

```console
$ lunar date -d 2020-05-23
公历: 2020年5月23日 星期六
农历: 庚子年闰四月初一
干支: 庚子 辛巳 丙寅
生肖: 鼠
```

The day can also be given as positionals, or as a `date(1)` style string:

```console
$ lunar date 2026 2 17
$ lunar date -d '2026-09-04'
$ lunar date -d 'next friday'
$ lunar date -d '@1788000000'
```

On a day the State Council legislates, the profile gains a `法定` line — the
name of the holiday, and whether the day is off or one of the 调休 workdays
moved onto a weekend:

```console
$ lunar date -d 2026-09-26
公历: 2026年9月26日 星期六
农历: 丙午年八月十六
干支: 丙午 丁酉 癸卯
生肖: 马
法定: 中秋节 放假

$ lunar date -d 2026-10-10
公历: 2026年10月10日 星期六
农历: 丙午年九月廿一
干支: 丙午 戊戌 壬寅
生肖: 马
法定: 国庆节 调休上班
```

`-d` reads a **公历** date and `-l` a **农历** one; the two differ in nothing
else. Every form `-d` accepts, `-l` accepts too — the same string is just
resolved in the other calendar:

```console
$ lunar date -l -d 2026-07-15      # 农历 2026 年七月十五
公历: 2026年8月27日 星期四
农历: 丙午年七月十五

$ lunar date -d 2026-07-15         # 公历 2026 年 7 月 15 日
公历: 2026年7月15日 星期三
农历: 丙午年六月初二
```

`-R` names the leap month — 庚子年闰四月初一是 2020 年 5 月 23 日:

```console
$ lunar date -l -R -d 2020-04-01
公历: 2020年5月23日 星期六
农历: 庚子年闰四月初一
```

The day can also be given as positionals, which follow the same rule:

```console
$ lunar date 2026 2 17
$ lunar date -l 2026 7 15
$ lunar date -d '2026-09-04'
$ lunar date -d 'next friday'
$ lunar date -d '@1788000000'
```

Only the forms that name a *date* switch. `now`, `tomorrow`, `next friday` and
`+3 days` are statements about days, not about a date written in one calendar,
so they resolve against the reference day exactly as they do without `-l`.
A time of day (`T15:30`) belongs to the 公历 grammar and is **refused** under
`-l` rather than ignored, so a lunar date never looks like it honoured a time it
threw away.

On a day the State Council legislates, the profile gains a `法定` line — the
name of the holiday, and whether the day is off or one of the 调休 workdays
moved onto a weekend:

```console
$ lunar date -d 2026-09-26
公历: 2026年9月26日 星期六
农历: 丙午年八月十六
干支: 丙午 丁酉 癸卯
生肖: 马
法定: 中秋节 放假

$ lunar date -d 2026-10-10
公历: 2026年10月10日 星期六
农历: 丙午年九月廿一
干支: 丙午 戊戌 壬寅
生肖: 马
法定: 国庆节 调休上班
```

A date that does not exist in the calendar named is reported as such, with the
lunar month's real length:

```console
$ lunar date -l -d 2026-07-31
lunar: 农历 2026 年七月没有第 31 天 (该月只有 29 天)
```

`-d` understands `now` / `today` / `tomorrow` / `yesterday`, epoch seconds
(`@…`), ISO 8601 dates and times (with `Z` or `±hh:mm` zones, the sign
convention of POSIX: `+0800` is 8 hours *behind* UTC), `YYYY/MM/DD`,
`MM/DD/YYYY`, relative offsets (`+3 days`, `-2 weeks`, `1 fortnight`,
`next month`) and weekday names (`monday`, `next friday`, `last sun`).

Offsets shorter than a day are refused rather than ignored: this tool
answers with a date and has no clock, so `90 minutes ago` would have
nothing to move.

### Custom format

```console
$ lunar date -f '%G年%M%N，星期%A，%Q' -d 2026-09-07
丙午年七月廿六，星期一，白露

$ lunar date -f '%G年%M%N，星期%A\n干支日：%D' -d 2026-09-07
丙午年七月廿六，星期一
干支日：甲申

$ lunar date -f '周%A 农历%M%N（日序 %n），生肖%S，节气：%Q' -d 2026-09-07
周一 农历七月廿六（日序 26），生肖马，节气：白露
```

| token | meaning |
|---|---|
| `%Y` | 公历年 |
| `%m` | 公历月 |
| `%d` | 公历日 |
| `%A` | 星期几单字 (一~日) |
| `%G` | 农历年干支 (丙午) |
| `%M` | 农历月汉字 (正月 / 闰六月 / 腊月) |
| `%N` | 农历日汉字 (初一) |
| `%n` | 农历日数字 (23) |
| `%H` | 干支月 (丙申) |
| `%D` | 干支日 (辛巳) |
| `%S` | 生肖 (马) |
| `%Q` | 节气 (当日无则空) |
| `%%` | 字面 `%` |

`%A` only yields the weekday character, so any prefix works: `星期%A` = 星期一,
`周%A` = 周一, `礼拜%A` = 礼拜一. A backslash escapes the next character and
yields it: `\n` a newline, `\t` a tab, `\r` a carriage return, `\%` a literal
`%` (as `%%` does) and `\\` a literal backslash. `lunar date --help-format`
prints the same table.

## `lunar cal`

Without arguments it prints the current civil month, with the lunar day, solar
term or festival under each date. The grid's text is the calendar alone — the
statutory calendar below is a *colour*, and this example went to a pipe, so
nothing is marked:

```console
$ lunar cal 2026 9
2026年9月
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
十八            十九            二十
```

Cell content follows a fixed priority — **节日 > 初一显示月份名 > 节气 > 农历日** —
so 11 September shows 八月 instead of 初一, 7 September is covered by 白露 and
25 September by 中秋节. Both calendars' festivals count: 1 October shows 国庆节,
and a day carrying several shows the one a reader recognises, so 十月十日 is
地藏节 rather than the more obscure 天灸日.

### The statutory calendar, and the reference day

`lunar cal` also marks China's 法定节假日, which is a published table and not a
festival. Every mark is an attribute, never a character:

| mark | shown as |
|---|---|
| 法定节假日 放假 | red |
| 调休 上班 — a weekend the State Council made a workday | bold bright yellow |
| the reference day — today, or the local date the run resolved | inverse video |

**The marks compose, and neither wins.** A day can be both at once: when today
is a 放假 day the cell is red *and* inverted (`ESC[31;7m`), and a 调休 today is
bold, bright and inverted. One attribute cannot say two things, but a single SGR
run can carry both parameters, so nothing has to be given up. Run `lunar cal` on
a day inside a holiday and today is still the block the eye finds first, in the
colour the day itself carries.

The mark is never written into the cell. A 放假 day still reads 中秋节 and a 调休
workday still reads 九月, because those are the facts the calendar exists to
show and an administrative label in their place would displace them:

```console
$ lunar cal 2026 10
2026年10月
一          二          三          四          五          六          日
                                    1           2           3           4
                                    国庆节      廿二        廿三        廿四
5           6           7           8           9           10          11
世界住房日  廿六        廿七        寒露        廿九        九月        初二
```

Read that in a terminal and 1–7 October are red, and 10 October — a Saturday the
State Council made a workday — is bold bright yellow.

Colour is emitted only when stdout is a terminal; `--color` forces it on (for
`less -R` and `grep --color`) and `--no-color` forces it off.
`--no-holiday` drops the statutory half of the mark, and a reference day that
is itself statutory keeps only its own: a 放假 today becomes plain inverse video.
**A piped or redirected grid shows no mark at all** — that is the
trade this design makes, and `--color` is the way out of it. For a single day,
`lunar date` reports the statutory calendar in words:

```console
$ lunar date -d 2026-10-10
公历: 2026年10月10日 星期六
农历: 丙午年九月廿一
干支: 丙午 戊戌 壬寅
生肖: 马
法定: 国庆节 调休上班
```

`-L` switches to lunar months, where each cell leads with the civil date:

```console
$ lunar cal -L 2026 7
农历 丙午年 七月
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
白露    廿七    廿八    廿九
```

处暑 lands on 8/23 and 七夕节 on 8/19, both from the astronomical engine.

Options:

| option | meaning |
|---|---|
| *(none)* | the current civil month (the current lunar month with `-L`) |
| `年` | the whole year |
| `年 月` | that month |
| `-L`, `--lunar` | show lunar months instead of civil months |
| `-R`, `--leap` | with `-L`/`-l`, select the leap month (refused without it) |
| `-s`, `--sunday` / `-m`, `--monday` | first day of the week (Monday by default) |
| `-y`, `--year` | whole year (current year by default) |
| `-3`, `--three` / `-n N`, `--months N` | N consecutive months, centred on the month named (`N` ≥ 1) |
| `--number` | lunar day as digits instead of 初一/廿六 |
| `--no-month-name` | never replace 初一 with the month name |
| `--no-festival` | never show festivals |
| `--no-holiday` | never show 法定节假日 (放假 / 调休) |
| `--color` / `--no-color` | force SGR on or off (default: only when stdout is a terminal) |

`lunar cal -L 2026` walks the whole lunar year, so it picks up a leap month on
its own; an explicit `-L 2020 4` prints the ordinary fourth month, and
`-L 2020 4 -R` prints 闰四月. `-3` and `-n N` widen an explicit month to a span
**centred** on it, and a span that would run off either end of the year is
shifted back rather than cut short — three months centred on 十二月 are 十月,
冬月, 腊月. A span at least as long as the year is the year.

## Supported range

Years **1–9999**, the window `lunar-rs` can serve: its ShouXing astronomy is
bounded by the `LEAP_11` / `LEAP_12` tables, so outside it the engine's
extrapolated lunar months and solar terms are not meant to be relied on. Years
outside the range, impossible dates and the days the Gregorian reform skipped
(1582-10-05 … 1582-10-14) are reported as errors.

Because the cells are laid out by display width, the output is designed for a
terminal with a CJK-capable font.

## Implementation notes

- `src/calendar.rs` — the supported range, festival, ganzhi and statutory-holiday
  helpers, solar↔lunar conversion in both directions. All answers come from
  `lunar-rs`.
- `src/civil.rs` — the epoch ↔ civil-date bridge. `lunar-rs` models the 1582
  Gregorian reform, so its `Solar` refuses 1582-10-05..14; the civil arithmetic
  needed by the `-d` parser and the grids lives here.
- `src/calgrid.rs` — grid layout. Cells are padded by **display width**, not by
  character count, so a two-glyph label and a two-digit day line up. The column
  width is the widest cell the grid holds, so a month carrying `中秋节` is wider
  than one that does not; days of the neighbouring months are blank.
- `src/cell.rs` — the cell content priority.
- `src/mark.rs` — the reference-day / 放假 / 调休 marks and the SGR painting.
- `src/datestr.rs` — the `date(1)` style `-d` parser.
- `src/format.rs` — the `-f` token engine.
- `src/lang.rs` — the display-width measurement the grid layout depends on.
- `src/commands/` — the two subcommands.

Festivals come from both calendars: the lunar ones (春节, 中秋节, 端午) and the
civil ones (国庆节, 劳动节, 儿童节), including those that float to a weekday
(母亲节, 感恩节). `--no-festival` turns off both.

The 法定节假日 table is a third source and is kept apart from both: it is a
published calendar, not a festival, and a 调休 Saturday is exactly the day the
weekday and the festival list both get wrong. `lunar-rs` ships it only for the
years it was given (2001-2026 at this release), so a grid outside that window
shows festivals and no statutory marks rather than guessing.

`lunar-rs` numbers months from 正月 (`寅`) for the month pillar, which is the
traditional almanac convention; `tz-rs` resolves `TZ` for the local "today".

## Tests

```bash
cargo test
```

`tests/documented_examples.rs` pins the documented layout, the token table, the
overlay switches and the error messages, and separately pins the calendar values
by date (芒种 on 2020-06-05, and so on), and a check that **every** month of the
year shows every one of its days — the invariant the civil overlay used to
break.

Where a published sample disagrees with the astronomical engine, the engine
wins: a few sample cells are stale (中元 shown three days early, 芒种 on the
wrong day, 雨水 and 惊蛰 missing), and this tool prints the computed dates.

## License

MIT
