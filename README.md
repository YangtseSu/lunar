# lunar

A command-line wrapper around [`lunar-rs`](https://crates.io/crates/lunar-rs), the
pure-Rust 寿星天文历 engine. It exposes two subcommands modelled on the Unix tools
they are named after:

- **`lunar date`** — one day's almanac: 公历 / 星期 / 农历 / 干支 / 生肖 / 节气 /
  法定 / 星座, queryable from either calendar, with a custom `-f` format engine
  and a `-a` 黄历 block.
- **`lunar cal`** — `cal(1)`-style month and year grids overlaid with lunar days,
  solar terms, festivals and the statutory calendar (放假 / 调休), over civil
  months or, with `-L`, lunar months.

Every calendar answer — the solar↔lunar conversion, the 24 solar terms, ganzhi,
the festival tables, the 黄历 (宜忌 / 冲煞 / 神煞 / 星宿 / 纳音 / 方位) and the
State Council holiday table — is delegated to `lunar-rs`. This repository owns
only argument parsing, output shaping and grid layout; it never implements
calendar arithmetic of its own.

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
星座: 处女
```

The `节气` line is omitted when the day carries no solar term, and the
`星座` line — last, and never omitted — is a function of the civil month and
day alone. A leap month shows up in the month name:

```console
$ lunar date -d 2020-05-23
公历: 2020年5月23日 星期六
农历: 庚子年闰四月初一
干支: 庚子 辛巳 丙寅
生肖: 鼠
星座: 双子
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
| `-a, --almanac` | append the 黄历 block after the profile (refused with `-f`) |
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
星座: 处女

$ lunar date -d 2026-07-15         # 公历 2026 年 7 月 15 日
公历: 2026年7月15日 星期三
农历: 丙午年六月初二
干支: 丙午 乙未 庚寅
生肖: 马
星座: 巨蟹
```

`-R` names the leap month, so 庚子年闰四月初一 resolves to 2020-05-23:

```console
$ lunar date -l -R -d 2020-04-01
公历: 2020年5月23日 星期六
农历: 庚子年闰四月初一
干支: 庚子 辛巳 丙寅
生肖: 鼠
星座: 双子
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
- ISO 8601 — `2026-09-07`, `2026-9-7`, `2026-09-07T15:30`, `20260907T1530`,
  `2026-09-07T15:30:45+08:00`, `2026-09-07T15:30:45.5+08:00`, `2026-09-07Z`;
  a month or a day may be written unpadded, a zone with no clock means
  midnight in that zone, and a fractional second is accepted and truncated —
  this tool keeps no clock, so `…T15:30:45.5+08:00` and `…T15:30:45+08:00` are
  the same day;
- slashes — `2026/09/07`, `2026/9/7`, `09/07/2026` (month/day/year);
- relative offsets — `+3 days`, `-2 weeks`, `2 days ago`, `1 fortnight`,
  `next month`, `last year`;
- weekday names — `monday`, `sat`, `next friday`, `last friday`, and the
  Chinese `星期六`, `周二`, `礼拜六`, `星期天` / `周天` / `礼拜天` (our
  extension: `date(1)` has no Chinese weekday names and refuses them);
- a bare time of day — `15:30`, `15:30 UTC`, applied to today; a zone moves
  the day when it crosses midnight, so `23:30+0800` is not always today.

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
星座: 天秤

$ lunar date -d 2026-10-10
公历: 2026年10月10日 星期六
农历: 丙午年九月初一
干支: 丙午 戊戌 丁巳
生肖: 马
法定: 国庆节 调休上班
星座: 天秤
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

$ lunar date -f '星座%Z，%Q' -d 2026-09-07
星座处女，白露
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
| `%Z` | 星座 (处女) |
| `%%` | 字面 `%` |

`%A` yields only the weekday character, so any prefix works: `星期%A` = 星期一,
`周%A` = 周一, `礼拜%A` = 礼拜一. A backslash escapes the next character and
yields it: `\n` a newline, `\t` a tab, `\r` a carriage return, `\%` a literal `%`
(as `%%` does) and `\\` a literal backslash. `lunar date --help-format` prints
the same table.

### The 黄历 block

`-a` / `--almanac` appends the day's almanac after the profile — 宜忌, 冲煞,
神煞, 星宿, 纳音, 方位 and 物候, all of them the engine's own tables:

```console
$ lunar date -a -d 2026-09-07
公历: 2026年9月7日 星期一
农历: 丙午年七月廿六
干支: 丙午 丙申 甲申
生肖: 马
节气: 白露
星座: 处女
宜: 嫁娶、出行、伐木、拆卸、修造、动土、移徙、安葬、破土、修坟、立碑
忌: 掘井、祈福、安床、开市、入宅、挂匾、开光
冲煞: (戊寅)虎 煞南
值神: 闭  十二神煞: 天牢(黑道)
吉神: 月空、王日、天马、五富、不将、圣心、除神、鸣吠  凶煞: 游祸、血支、五离、白虎
二十八宿: 毕(吉) 值月 禽乌 门西白虎
纳音: 泉中水  旬空: 午未  六曜: 友引  小六壬: 留连  九星: 一白水天枢  月相: 蛾眉残
彭祖百忌: 甲不开仓财物耗散 / 申不安床鬼祟入房
胎神: 占门炉 外西北  胎元: 离
方位: 喜神艮(东北)、阳贵坤(西南)、阴贵艮(东北)、福神坎(正北)、财神艮(东北)
物候: 鸿雁来 (白露 初候)
```

Two of the groups are seasonal and only exist on part of the year — 数九 runs
from 冬至 for 81 days, 三伏 from 夏至 — so their lines are **omitted** on the
days that have none, exactly as `节气` is. 2026-07-25 is in the 中伏:

```console
$ lunar date -a -d 2026-07-25
公历: 2026年7月25日 星期六
农历: 丙午年六月十二
干支: 丙午 乙未 庚子
生肖: 马
星座: 狮子
宜: 祭祀、祈福、解除、整手足甲、安床、沐浴、入殓、移柩、破土、启钻、安葬、谢土
忌: 嫁娶、斋醮、开市、出火、入宅、移徙、出行、作灶、安门、伐木
冲煞: (甲午)马 煞南
值神: 执  十二神煞: 天刑(黑道)
吉神: 月空、金堂、解神、鸣吠对  凶煞: 月害、大时、大败、咸池、小耗、五虚、九坎、九焦、归忌、天刑
二十八宿: 氐(凶) 值土 禽貉 门东青龙
纳音: 壁上土  旬空: 辰巳  六曜: 大安  小六壬: 小吉  九星: 九紫火隐元  月相: 宵
彭祖百忌: 庚不经络织机虚张 / 子不问卜自惹祸殃
胎神: 占碓磨 房内南  胎元: 兑
方位: 喜神乾(西北)、阳贵离(正南)、阴贵艮(东北)、福神坤(西南)、财神震(正东)
物候: 腐草为萤 (大暑 初候)
三伏: 中伏第1天
```

Every field is a function of the day alone, never of a time of day — the ones
that would be (`time_yi`, `time_chong`) are deliberately absent, since this tool
keeps no clock. `-a` is refused together with `-f`: the profile and the block are
two shapes of the same answer, and `-f` is already the one that takes over.

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
mark at all**, since the marks are attributes; `--color=always` forces the
escapes on (for `less -R`, `grep --color`) and `--no-color` forces them off.
`--no-holiday` drops the statutory half of the mark and keeps the reference
day's own. For a single day, `lunar date` reports the statutory calendar in
words.

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
| `-y`, `--year` | the whole year: 12 civil months, or the whole lunar year with `-L` |
| `-3`, `--three` | the month named and the one before and after it |
| `-n N`, `--months N` | the next `N` months (`N` ≥ 1), starting at the month named |
| `--number` | lunar day as digits instead of 初一/廿六 |
| `--no-month-name` | never replace 初一 with the month name |
| `--no-festival` | never show festivals |
| `--no-holiday` | never mark the statutory calendar (放假 / 调休) |
| `--color[=WHEN]` / `--no-color` | `auto` (only when stdout is a terminal) / `always` / `never`; `--no-color` is `--color=never`. Each overrides the other, so the one written last decides. The value needs `=`, as in `--color=always` |

The span flags read as `cal(1)` reads them, and every window crosses a year
boundary freely: `-3` centres (`cal 2026 12 -3` prints 2026年11月, 2026年12月 and
2027年1月), `-n N` starts at the month named, and `-y` is the year the month
belongs to — `cal 2026 9 -y` prints the twelve months of 2026, not twelve months
from September. A month the arguments leave out is the month the reference day
falls in, so `cal -3` is centred on today and `cal 2026 -3` is centred on today's
month in 2026. A month without a flag is the whole year, and with none at all it
is the current month. The lunar view walks the continuous lunar month sequence,
so a window crosses the lunar new year too: `cal -L 2026 12 -n 3` prints
丙午年 腊月, 丁未年 正月, 丁未年 二月, each grid titled with its own ganzhi year.
A window that reaches past 1–9999 is reported, never clipped.

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

农历 9999 年腊月 is the one month of the last lunar year a grid cannot draw:
it begins on 9999-12-30 and ends on 10000-01-27, so its days leave the range.
The month is reported by name instead of being clipped — a clipped grid would
answer differently from `lunar date` about the same day. The months around it
are unaffected: `cal -L 9999 11` still prints 农历 己亥年 冬月.

```console
$ lunar cal -L 9999
lunar: 农历 9999 年腊月跨入 10000 年，超出支持范围 (1–9999)
```

The statutory calendar is a separate data window: `lunar-rs` ships the years it
was given (2001–2026 at this release), so a grid outside that window shows
festivals and no 放假 / 调休 marks. The window is a property of the published
table, not something this tool can compute.

## Known divergences

Behaviour differences from the tools the subcommands are modelled on are bugs,
not features, and are tracked as implementation plans in [`docs/plans`](docs/plans).
The compatibility matrix in [`docs/parity.md`](docs/parity.md) lists every form
`date(1)` / `cal(1)` accepts, which ones are still open, and the ones kept on
purpose with the reason for each.

## Implementation notes

| path | role |
|---|---|
| `src/calendar.rs` | the `lunar-rs` boundary: range checks, solar↔lunar conversion, ganzhi, festivals, solar terms, 星座, the 黄历 block, statutory holidays |
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

GPL-3.0-or-later. See [LICENSE](LICENSE).
