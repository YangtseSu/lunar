# lunar

A command-line wrapper around [`lunar-rs`](https://crates.io/crates/lunar-rs), the
pure-Rust 寿星天文历 engine. It exposes two subcommands modelled on the Unix tools
they are named after:

- **`lunar date`** — one day's almanac: 公历 / 星期 / 农历 / 干支 / 生肖 / 节气 /
  法定, queryable from either calendar, with a custom `-f` format engine.
- **`lunar cal`** — `cal(1)`-style month and year grids overlaid with lunar days,
  solar terms, festivals and the statutory calendar (放假 / 调休), over civil
  months or, with `-L`, lunar months.

Every calendar answer — the solar↔lunar conversion, the 24 solar terms, ganzhi,
the festival tables and the State Council holiday table — is delegated to
`lunar-rs`. This repository owns only argument parsing, output shaping and grid
layout; it never implements calendar arithmetic of its own.

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

Input comes from either channel, and both read the same grammar:

| input | meaning |
|---|---|
| *(none)* | today, in the local zone |
| `年 月 日` | three positional integers |
| a single positional | a `date(1)`-style string |
| `-d, --date <DATE>` | the same `date(1)`-style string |
| `-l, --lunar` | read the positional / `-d` date as **农历** instead of 公历 |
| `-R, --leap` | with `-l`, name the leap month (refused without `-l`) |
| `-f, --format <FORMAT>` | expand a custom format instead of the profile |
| `--help-format` | print the token table |

`-d` is always 公历; the positionals follow `-l`. The two differ in nothing else,
so every string `-d` accepts, `-l` accepts too — the same string is just resolved
in the other calendar:

```console
$ lunar date -l -d 2026-07-15      # 农历 2026 年七月十五
公历: 2026年8月27日 星期四
农历: 丙午年七月十五
干支: 丙午 丙申 癸酉
生肖: 马

$ lunar date -d 2026-07-15         # 公历 2026 年 7 月 15 日
公历: 2026年7月15日 星期三
农历: 丙午年六月初二
干支: 丙午 乙未 庚寅
生肖: 马
```

`-R` names the leap month, so 庚子年闰四月初一 resolves to 2020-05-23:

```console
$ lunar date -l -R -d 2020-04-01
公历: 2020年5月23日 星期六
农历: 庚子年闰四月初一
干支: 庚子 辛巳 丙寅
生肖: 鼠
```

Only forms that name a *date* switch calendars. `now`, `tomorrow`, `next friday`
and `+3 days` are statements about days, not about a date written in one calendar
or the other, so they resolve against the reference day exactly as they do
without `-l`. A time of day (`T15:30`) belongs to the 公历 grammar and is
**refused** under `-l` rather than ignored, so a lunar date never looks like it
honoured a time it threw away.

`-d` understands:

- keywords — `now`, `today`, `tomorrow`, `yesterday`;
- epoch seconds — `@1758240000`, `@-1` (truncated towards zero, like `date -d @…`);
- ISO 8601 — `2026-09-07`, `2026-09-07T15:30`, `20260907T1530`,
  `2026-09-07T15:30:45+08:00`, `2026-09-07Z`; a zone with no clock means
  midnight in that zone;
- slashes — `2026/09/07`, `09/07/2026` (month/day/year);
- relative offsets — `+3 days`, `-2 weeks`, `2 days ago`, `1 fortnight`,
  `next month`, `last year`;
- weekday names — `monday`, `sat`, `next friday`, `last friday`;
- a bare time of day — `15:30`, applied to today.

A numeric zone offset uses the **POSIX** sign convention: `+0800` is 8 hours
*behind* UTC. Offsets shorter than a day are refused rather than ignored — this
tool answers with a date and keeps no clock, so `90 minutes ago` would have
nothing to move.

On a day the State Council legislates, the profile gains a `法定` line — the name
of the holiday, and whether the day is off or one of the 调休 workdays moved onto
a weekend:

```console
$ lunar date -d 2026-09-26
公历: 2026年9月26日 星期六
农历: 丙午年八月十六
干支: 丙午 丁酉 癸卯
生肖: 马
法定: 中秋节 放假

$ lunar date -d 2026-10-10
公历: 2026年10月10日 星期六
农历: 丙午年九月初一
干支: 丙午 戊戌 丁巳
生肖: 马
法定: 国庆节 调休上班
```

A date that does not exist in the calendar named is reported as such, with the
lunar month's real length:

```console
$ lunar date -l -d 2026-07-31
lunar: 农历 2026 年七月没有第 31 天 (该月只有 29 天)

$ lunar date -d 2023-02-30
lunar: 公历 2023-02-30 不存在
```

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

`%A` yields only the weekday character, so any prefix works: `星期%A` = 星期一,
`周%A` = 周一, `礼拜%A` = 礼拜一. A backslash escapes the next character and
yields it: `\n` a newline, `\t` a tab, `\r` a carriage return, `\%` a literal `%`
(as `%%` does) and `\\` a literal backslash. `lunar date --help-format` prints
the same table.

## `lunar cal`

Without arguments it prints the current civil month, with lunar day, solar term or
festival under each date:

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
25 September by 中秋节. Both calendars' festivals count: 1 October shows 国庆节.

### The statutory calendar, and the reference day

`lunar cal` marks China's 法定节假日, which is a published table and not a
festival. Every mark is an attribute, never a character:

| mark | shown as |
|---|---|
| 法定节假日 放假 | red |
| 调休 — a weekend the State Council made a workday | bold bright |
| the reference day — today in the local zone | inverse video |

The marks compose, and neither wins: a day that is both a 放假 day and the
reference day is red *and* inverted at once (`ESC[31;7m`), and a 调休 today is
bold, bright and inverted. The mark is never written into the cell — a 放假 day
still reads 中秋节, a 调休 workday still reads 九月, because those are the facts
the calendar exists to show:

```console
$ lunar cal 2026 10
2026年10月
一          二          三          四          五          六          日
                                    1           2           3           4
                                    国庆节      廿二        廿三        廿四
5           6           7           8           9           10          11
世界住房日  廿六        廿七        寒露        廿九        九月        初二
12          13          14          15          16          17          18
初三        初四        初五        初六        初七        初八        重阳节
19          20          21          22          23          24          25
初十        十一        十二        十三        霜降        十五        十六
26          27          28          29          30          31
十七        十八        十九        二十        廿一        万圣节前夜
```

In a terminal, 1–7 October are red and 10 October — a Saturday the State Council
turned into a workday — is bold bright. **A piped or redirected grid shows no
mark at all**, since the marks are attributes; `--color` forces the escapes on
(for `less -R`, `grep --color`) and `--no-color` forces them off. `--no-holiday`
drops the statutory half of the mark and keeps the reference day's own.
For a single day, `lunar date` reports the statutory calendar in words.

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
十二    十三    十四    中元节  十六    十七    十八
8/31    9/1     9/2     9/3     9/4     9/5     9/6
十九    二十    廿一    廿二    廿三    廿四    廿五
9/7     9/8     9/9     9/10
白露    廿七    廿八    教师节
```

### Options

| option | meaning |
|---|---|
| *(none)* | the current civil month (the current lunar month with `-L`) |
| `年` | the whole year: 12 grids, January first (`-L`: the whole lunar year) |
| `年 月` | that month |
| `-L`, `--lunar` | show lunar months instead of civil months |
| `-R`, `--leap` | with `-L`, select the leap month (refused without `-L`) |
| `-s`, `--sunday` / `-m`, `--monday` | first day of the week (Monday by default) |
| `-y`, `--year` | 12 months starting at the month named |
| `-3`, `--three` / `-n N`, `--months N` | `N` consecutive months (`N` ≥ 1), civil view starting at the month named, lunar view centred on it |
| `--number` | lunar day as digits instead of 初一/廿六 |
| `--no-month-name` | never replace 初一 with the month name |
| `--no-festival` | never show festivals |
| `--no-holiday` | never mark the statutory calendar (放假 / 调休) |
| `--color` / `--no-color` | force SGR on or off (default: only when stdout is a terminal) |

The civil view walks forward from the month named, across year boundaries
(`cal 2026 12 -n 3` prints 2026年12月, 2027年1月, 2027年2月). The lunar view is
bounded by the lunar year it names: a span is centred on the named month and
shifted back to keep its length at either end, so three months centred on 十二月
are 十月, 冬月, 腊月. A span at least as long as the year is the year. These span
semantics differ from `cal(1)`; aligning them is tracked in `docs/plans`.

## Supported range

Years **1–9999**, the window `lunar-rs` can serve: its astronomy is bounded by
the `LEAP_11` / `LEAP_12` tables, and outside that range its extrapolated lunar
months and solar terms are not meant to be relied on. Years outside the window,
impossible dates and the ten days the Gregorian reform skipped
(1582-10-05 … 1582-10-14) are reported as errors, never rendered:

```console
$ lunar cal 10000
lunar: 年份 10000 超出支持范围 (1–9999)

$ lunar date -d 2023-02-30
lunar: 公历 2023-02-30 不存在
```

The statutory calendar is a separate data window: `lunar-rs` ships the years it
was given (2001–2026 at this release), so a grid outside that window shows
festivals and no 放假 / 调休 marks. The window is a property of the published
table, not something this tool can compute.

## Known divergences

Behaviour differences from the tools the subcommands are modelled on are bugs,
not features, and are tracked as implementation plans in [`docs/plans`](docs/plans):

- `lunar date -d` — no unpadded ISO (`2026-9-7`), no fractional seconds (which
  the module docs advertise), and no bare time with a zone (`15:30 UTC`); a bare
  time is also silently accepted under `-l` where the rule says refuse
  (`docs/plans/06`);
- arithmetic on a huge relative count can overflow instead of reporting a range
  error (`date -d "+2147483600 years"`, `docs/plans/01`), and a long run of
  digits is claimed by the compact `YYYYMMDD` form (`date -d "2147483647 days"`
  reports a month, `docs/plans/04`);
- `lunar cal` span flags do not match `cal(1)`: `-3` walks forward instead of
  centring, `-n N` centres instead of starting at the named month, and `-y`
  shows 12 months from the named month instead of the named year
  (`docs/plans/03`);
- `--color --no-color` silently means "always colour", and the code comments
  claim a last-flag-wins rule that is not implemented (`docs/plans/02`).

## Implementation notes

| path | role |
|---|---|
| `src/calendar.rs` | the `lunar-rs` boundary: range checks, solar↔lunar conversion, ganzhi, festivals, solar terms, statutory holidays |
| `src/civil.rs` | proleptic-Gregorian epoch math and date stepping, which `lunar-rs` cannot provide because its `Solar` models the 1582 reform |
| `src/calgrid.rs` | grid layout; cells are padded by **display width** (`src/lang.rs`), not character count, so CJK cells align |
| `src/cell.rs` | the cell content priority (节日 > 初一显示月份名 > 节气 > 农历日) |
| `src/mark.rs` | the reference-day / 放假 / 调休 marks and the one place SGR is written |
| `src/datestr.rs` | the `date(1)`-style `-d` grammar |
| `src/format.rs` | the `-f` token engine |
| `src/tz.rs` | local "today" from `TZ` / the system zone |
| `src/commands/` | the two subcommand surfaces |

Command modules never print: they append to a `String` that `main` writes to
stdout once, so a failure anywhere prints nothing and exits non-zero.

## Tests

```bash
cargo test        # the integration suite
cargo fmt --check # formatting gate
cargo clippy --all-targets
```

`tests/documented_examples.rs` spawns the real binary and pins the documented
layout, the token table, the span and overlay switches, and the error messages,
plus calendar values by date. Where a published sample disagrees with the
astronomical engine, the engine wins.

## License

MIT
