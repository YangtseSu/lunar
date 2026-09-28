# lunar

English | [简体中文](README.zh-CN.md)

[![Vibe Coded](https://img.shields.io/badge/vibe--coded-%F0%9F%A4%96-8A2BE2)](#how-this-was-built)
[![Release](https://img.shields.io/github/v/release/YangtseSu/lunar?sort=semver&label=release)](https://github.com/YangtseSu/lunar/releases/latest)
[![CI](https://github.com/YangtseSu/lunar/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/YangtseSu/lunar/actions/workflows/ci.yml)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)](LICENSE)
[![Rust 2024](https://img.shields.io/badge/Rust-2024-2b866d?logo=rust)](https://www.rust-lang.org)

A command-line wrapper around [`lunar-rs`](https://crates.io/crates/lunar-rs), the
pure-Rust 寿星天文历 engine. It exposes three subcommands — two modelled on the
Unix tools they are named after, and one the tools have no counterpart for:

- **`lunar date`** — one day's almanac: 公历 / 星期 / 农历 / 干支 / 生肖 / 节气 /
  法定 / 星座, queryable from either calendar, with a custom `-f` format engine
  and a `-a` 黄历 block.
- **`lunar cal`** — `cal(1)`-style month and year grids overlaid with lunar days,
  solar terms, festivals and the statutory calendar (放假 / 调休), over civil
  months or, with `-L`, lunar months.
- **`lunar bazi`** — 生辰八字: the four pillars of a birth moment, with 十神,
  藏干, 纳音, 地势, 五行, 旬空, 地支十神 and 命局. The only command that keeps
  a clock, because the 时柱 needs one.

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

## Install

Binaries for Linux x86_64 and aarch64, built with the standard glibc
toolchain on the target architecture:

```bash
# x86_64
curl -LO https://github.com/YangtseSu/lunar/releases/download/v0.1.0/lunar-0.1.0-x86_64.tar.gz
tar -xzf lunar-0.1.0-x86_64.tar.gz && sudo install -m755 lunar-0.1.0-x86_64/lunar /usr/local/bin/

# aarch64
curl -LO https://github.com/YangtseSu/lunar/releases/download/v0.1.0/lunar-0.1.0-aarch64.tar.gz
tar -xzf lunar-0.1.0-aarch64.tar.gz && sudo install -m755 lunar-0.1.0-aarch64/lunar /usr/local/bin/
```

Each release carries a `SHA256SUMS`; verify it before installing:

```bash
curl -LO https://github.com/YangtseSu/lunar/releases/download/v0.1.0/SHA256SUMS
sha256sum -c SHA256SUMS
```

Or build from source:

```bash
cargo install --git https://github.com/YangtseSu/lunar
```

## `lunar date`

```console
$ lunar date -d 2026-09-07
公历: 2026年9月7日 星期一
农历: 丙午年七月廿六
干支: 丙午 丁酉 甲申
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
| `%G` | 农历年干支, 春节换年 (丙午) |
| `%M` | 农历月汉字 (正月 / 闰六月 / 腊月) |
| `%N` | 农历日汉字 (初一) |
| `%n` | 农历日数字 (23) |
| `%H` | 干支月, 节气当日换月 (丁酉) |
| `%D` | 干支日 (辛巳) |
| `%S` | 生肖 (马) |
| `%Q` | 节气 (当日无则空) |
| `%Z` | 星座 (处女) |
| `%%` | 字面 `%` |


`%G` and `%H` are the two bases the tool keeps apart on purpose. `%G` is the
lunar year, which turns at 春节 — the same year the `农历:` line names, and the
year `cal -L` titles its months with. `%H` is the 干支 month, which turns on the
節氣 day, because that is the month pillar the 黄历's 宜 / 忌 tables are keyed
on. The `干支:` profile line therefore prints the 立春 year (`%G` is *not* what
it shows), so that its three pillars are one self-consistent chain; the two
bases disagree between 春节 and 立春 and on each 節氣 day, which is why the
`农历:` line and the `干支:` line can name different years on the same day.

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
干支: 丙午 丁酉 甲申
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

## `lunar bazi`

生辰八字 — the four pillars of a birth moment, with 十神, 藏干, 纳音, 地势,
五行, 旬空, 地支十神 and the 命局. It is the one command that keeps a clock,
because one of the four pillars is the 时柱 and a chart without it is not a
chart:

```console
$ lunar bazi 1990-06-15T10:30
公历: 1990年6月15日 10:30
农历: 庚午年五月廿三
八字: 庚午 / 壬午 / 辛亥 / 癸巳
十神: 劫财 / 伤官 / 日主 / 食神
藏干: 丁己 / 丁己 / 壬甲 / 丙庚戊
纳音: 路旁土 / 杨柳木 / 钗钏金 / 长流水
地势: 病 / 病 / 沐浴 / 死
五行: 金火 / 水火 / 金水 / 水火
旬空: 戌亥 / 申酉 / 寅卯 / 午未
地支十神: 七杀偏印 / 七杀偏印 / 伤官正财 / 正官劫财正印
命局: 胎元 癸酉(剑锋金) / 胎息 丙寅(炉中火) / 命宫 壬午(杨柳木) / 身宫 戊子(霹雳火)
说明: 未给性别，无大运
```

五行 is the element of the 干 and the element of the 支, two characters per
pillar — `庚` is 金 and `午` is 火, so the year pillar reads `金火`, which is
not the 纳音's element (`路旁土` is 土). 地支十神 is one 十神 per 藏干 of the
branch, so a cell is a phrase: `午` hides 丁 and 己, which against a 辛 day
stem are 七杀 and 偏印.

命局 is the four values a traditional chart sets beside the pillars rather
than in them, each with its 纳音. Two of them are counted from the 時辰, so
they need the same clock the 时柱 does and are the second thing a birth moment
without one cannot answer:

```console
$ lunar bazi 1990-06-15
公历: 1990年6月15日
农历: 庚午年五月廿三
八字: 庚午 / 壬午 / 辛亥
说明: 未给时刻，无时柱
十神: 劫财 / 伤官 / 日主
藏干: 丁己 / 丁己 / 壬甲
纳音: 路旁土 / 杨柳木 / 钗钏金
地势: 病 / 病 / 沐浴
五行: 金火 / 水火 / 金水
旬空: 戌亥 / 申酉 / 寅卯
地支十神: 七杀偏印 / 七杀偏印 / 伤官正财
命局: 胎元 癸酉(剑锋金) / 胎息 丙寅(炉中火)
说明: 命宫 / 身宫需时柱
说明: 未给性别，无大运
```

胎元 and 胎息 need no 時辰 and stay; 命宫 and 身宫 are left out and said to
be, rather than filled in from a noon the tool would have had to invent. Every
row that belongs to a pillar is cut to three columns with the 时柱, so no row
keeps a fourth cell the chart no longer has.

The `说明` lines are the tool naming what the input could not answer: the
时柱, then 命宫 / 身宫, then 大运. Each is reported rather than defaulted.

The date is read by the same grammar `lunar date -d` uses, so every form is
accepted here too — `1990-06-15 10:30`, `19900615T1030`, `1990/06/15 10:30` and
`1990-06-15T10:30+08:00` are one instant. A keyword (`yesterday`) and a
relative offset (`2 days ago`) name a day and no clock, exactly as in `date`.
There is no `-l`: a birth moment is a civil instant, and there is no 农历 time of
day to read one in.

**The 月柱 turns at the 節氣 instant here, where `lunar date` turns it at the
節氣 day — and both are right, because they answer different questions.** 白露
2026 falls at 22:41:16, so a birth at 20:00 that day is 申月 and one at 23:00 is
酉月, while the day's own almanac calls the whole of 09-07 酉月:

```console
$ lunar bazi 2026-09-07T20:00
公历: 2026年9月7日 20:00
农历: 丙午年七月廿六
八字: 丙午 / 丙申 / 甲申 / 甲戌
十神: 食神 / 食神 / 日主 / 比肩
藏干: 丁己 / 庚壬戊 / 庚壬戊 / 戊辛丁
纳音: 天河水 / 山下火 / 泉中水 / 山头火
地势: 死 / 绝 / 绝 / 养
五行: 火火 / 火金 / 木金 / 木土
旬空: 寅卯 / 辰巳 / 午未 / 申酉
地支十神: 伤官正财 / 七杀偏印偏财 / 七杀偏印偏财 / 偏财正官伤官
命局: 胎元 丁亥(屋上土) / 胎息 己巳(大林木) / 命宫 己亥(平地木) / 身宫 乙未(沙中金)
说明: 未给性别，无大运
```

The 年柱 and 日柱 are the same facts `lunar date` prints, on the same basis, so
the two commands never disagree about them — including the 子时 that belongs to
the day it began in rather than the day it ended.

### The 时柱, and 真太阳时

The 时柱 is counted from the **clock time** you typed, and this tool does not
correct it to 真太阳时 — the local time at which the sun is actually on the
meridian. Traditional 八字 practice uses the latter, so a chart here is a
钟表时 chart and should be read as one.

The difference is four minutes per degree of longitude from the standard
meridian, plus a daylight-saving hour where one was in force, plus the
equation of time (−14 to +16 minutes, never enough on its own). For Beijing
(东经 116.4°) that is about 14 minutes — usually not enough to move a pillar.
For 乌鲁木齐 (东经 87.6°) it is about 128 minutes, which **is** enough to move
the 时柱 by one 时辰, and it moves 起运 with it. China runs one zone on 东经
120° for the whole country, so the further west the birth, the wider the gap.

Daylight saving is the other half: China observed it from 1986 to 1991, and
between 1986-05-04 and 1986-09-14 a clock was an hour fast, which moves any
birth just before 23:00 into the next day.

None of this is corrected, and none of it is a bug in the engine — it charts
the instant it is handed. Correcting would need the birthplace longitude,
which is an input this tool has never taken, and the schools disagree about
whether to correct at all and which of the three definitions to use. The
engine carries no true-solar-time implementation, so the correction would have
to be built there first. Until it is, this paragraph is the whole truth about
the 时柱.

### 大运

大运 needs a **gender**: it runs forward for a man born in a yang year and
backward otherwise, and there is no default to assume. `-g` takes `男` / `女`
(or `male` / `female`). Without it the four pillars still print and the chart
says why the rest is missing:

```console
$ lunar bazi 1990-06-15T10:30 -g 男
公历: 1990年6月15日 10:30
农历: 庚午年五月廿三
八字: 庚午 / 壬午 / 辛亥 / 癸巳
十神: 劫财 / 伤官 / 日主 / 食神
藏干: 丁己 / 丁己 / 壬甲 / 丙庚戊
纳音: 路旁土 / 杨柳木 / 钗钏金 / 长流水
地势: 病 / 病 / 沐浴 / 死
五行: 金火 / 水火 / 金水 / 水火
旬空: 戌亥 / 申酉 / 寅卯 / 午未
地支十神: 七杀偏印 / 七杀偏印 / 伤官正财 / 正官劫财正印
命局: 胎元 癸酉(剑锋金) / 胎息 丙寅(炉中火) / 命宫 壬午(杨柳木) / 身宫 戊子(霹雳火)
起运: 1997年11月17日  (出生后 7年5月2天12小时)  顺行
大运: 8-17 癸未 / 18-27 甲申 / 28-37 乙酉 / 38-47 丙戌 / 48-57 丁亥 / 58-67 戊子 / 68-77 己丑 / 78-87 庚寅 / 88-97 辛卯
```

The same birth with 女 runs the other way, from a different 起运:

```console
$ lunar bazi 1990-06-15T10:30 -g 女
公历: 1990年6月15日 10:30
农历: 庚午年五月廿三
八字: 庚午 / 壬午 / 辛亥 / 癸巳
十神: 劫财 / 伤官 / 日主 / 食神
藏干: 丁己 / 丁己 / 壬甲 / 丙庚戊
纳音: 路旁土 / 杨柳木 / 钗钏金 / 长流水
地势: 病 / 病 / 沐浴 / 死
五行: 金火 / 水火 / 金水 / 水火
旬空: 戌亥 / 申酉 / 寅卯 / 午未
地支十神: 七杀偏印 / 七杀偏印 / 伤官正财 / 正官劫财正印
命局: 胎元 癸酉(剑锋金) / 胎息 丙寅(炉中火) / 命宫 壬午(杨柳木) / 身宫 戊子(霹雳火)
起运: 1993年7月4日  (出生后 3年0月18天16小时)  逆行
大运: 4-13 辛巳 / 14-23 庚辰 / 24-33 己卯 / 34-43 戊寅 / 44-53 丁丑 / 54-63 丙子 / 64-73 乙亥 / 74-83 甲戌 / 84-93 癸酉
```

Ten steps. The 起运 is printed as a date **and** the whole interval it took —
`7年5月2天12小时`, not `7年5月` — because the month count alone drops up to
29 days, and those two numbers then name the same moment instead of quietly
differing. A 起运 shorter than a year is printed as it is, not treated as an
anomaly, and a zero tail is left out rather than written `0天0小时`.

### 流年 / 流月 / 小运

`大运` is the chart; the years under it are a **query**, so `-y` / `--year`
names one of them instead of printing all hundred. Ten steps × ten years plus
ten 小运 per step is unreadable by default, and "which year" is a different
question from "which chart":

```console
$ lunar bazi 1990-06-15T10:30 -g 男 -y 2015
公历: 1990年6月15日 10:30
农历: 庚午年五月廿三
八字: 庚午 / 壬午 / 辛亥 / 癸巳
十神: 劫财 / 伤官 / 日主 / 食神
藏干: 丁己 / 丁己 / 壬甲 / 丙庚戊
纳音: 路旁土 / 杨柳木 / 钗钏金 / 长流水
地势: 病 / 病 / 沐浴 / 死
五行: 金火 / 水火 / 金水 / 水火
旬空: 戌亥 / 申酉 / 寅卯 / 午未
地支十神: 七杀偏印 / 七杀偏印 / 伤官正财 / 正官劫财正印
命局: 胎元 癸酉(剑锋金) / 胎息 丙寅(炉中火) / 命宫 壬午(杨柳木) / 身宫 戊子(霹雳火)
起运: 1997年11月17日  (出生后 7年5月2天12小时)  顺行
大运: 8-17 癸未 / 18-27 甲申 / 28-37 乙酉 / 38-47 丙戌 / 48-57 丁亥 / 58-67 戊子 / 68-77 己丑 / 78-87 庚寅 / 88-97 辛卯
流年: 2015年 乙未  26岁  旬空 辰巳
流月: 戊寅(正月) / 己卯(二月) / 庚辰(三月) / 辛巳(四月) / 壬午(五月) / 癸未(六月) / 甲申(七月) / 乙酉(八月) / 丙戌(九月) / 丁亥(十月) / 戊子(冬月) / 己丑(腊月)
小运: 己未 26岁
```

Three rules shape what prints, and each is reported rather than defaulted:
**`-y` needs `-g`**, because the 流年 is counted on 大运 and 顺逆 is the
gender's to decide; without one the chart says `说明: 流年需性别` and the
pillars are the whole answer. **A year no step covers is named, not
approximated** — the ten steps run from the 起运 year to a century on, and
before it or after it there is no 大运 to sit on, so the year is reported
with the span that does:

```console
$ lunar bazi 1990-06-15T10:30 -g 男 -y 1996
公历: 1990年6月15日 10:30
农历: 庚午年五月廿三
八字: 庚午 / 壬午 / 辛亥 / 癸巳
十神: 劫财 / 伤官 / 日主 / 食神
藏干: 丁己 / 丁己 / 壬甲 / 丙庚戊
纳音: 路旁土 / 杨柳木 / 钗钏金 / 长流水
地势: 病 / 病 / 沐浴 / 死
五行: 金火 / 水火 / 金水 / 水火
旬空: 戌亥 / 申酉 / 寅卯 / 午未
地支十神: 七杀偏印 / 七杀偏印 / 伤官正财 / 正官劫财正印
命局: 胎元 癸酉(剑锋金) / 胎息 丙寅(炉中火) / 命宫 壬午(杨柳木) / 身宫 戊子(霹雳火)
起运: 1997年11月17日  (出生后 7年5月2天12小时)  顺行
大运: 8-17 癸未 / 18-27 甲申 / 28-37 乙酉 / 38-47 丙戌 / 48-57 丁亥 / 58-67 戊子 / 68-77 己丑 / 78-87 庚寅 / 88-97 辛卯
说明: 1996 年无大运 (大运 1997-2086)
```

**The 小运 needs a 时柱.** It is counted from the 时柱, one step per year, and
a birth moment with no time of day has none to count it from — the same
choice 命宫 / 身宫 make, and for the same reason. The 流年 and 流月 are a
function of the year and the 大运, so they still print, and the chart says
what is missing:

```console
$ lunar bazi 1990-06-15 -g 男 -y 2015
公历: 1990年6月15日
农历: 庚午年五月廿三
八字: 庚午 / 壬午 / 辛亥
说明: 未给时刻，无时柱
十神: 劫财 / 伤官 / 日主
藏干: 丁己 / 丁己 / 壬甲
纳音: 路旁土 / 杨柳木 / 钗钏金
地势: 病 / 病 / 沐浴
五行: 金火 / 水火 / 金水
旬空: 戌亥 / 申酉 / 寅卯
地支十神: 七杀偏印 / 七杀偏印 / 伤官正财
命局: 胎元 癸酉(剑锋金) / 胎息 丙寅(炉中火)
说明: 命宫 / 身宫需时柱
起运: 1997年11月9日  (出生后 7年4月25天)  顺行
大运: 8-17 癸未 / 18-27 甲申 / 28-37 乙酉 / 38-47 丙戌 / 48-57 丁亥 / 58-67 戊子 / 68-77 己丑 / 78-87 庚寅 / 88-97 辛卯
流年: 2015年 乙未  26岁  旬空 辰巳
流月: 戊寅(正月) / 己卯(二月) / 庚辰(三月) / 辛巳(四月) / 壬午(五月) / 癸未(六月) / 甲申(七月) / 乙酉(八月) / 丙戌(九月) / 丁亥(十月) / 戊子(冬月) / 己丑(腊月)
说明: 小运需时柱
```

The 流月 are named in 农历 months (`正月` … `冬月` `腊月`), the same words the
`农历:` line uses, because that is what they are: a 流月 runs on 农历 months
from 立春, so a civil `1月` there would name a different month. The 小运 is
one step per year and runs the gender's direction, so it is a different pillar
from the 流年 beside it — `己未` against `乙未` above, and `丁卯` for the same
year had the same birth been read as 女.

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
| `src/commands/` | the three subcommand surfaces |

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

## How this was built

This repository was developed with an AI coding agent. A human set the
requirements, reviewed each change, and verified every claim against a real
run of the tool; the code was written largely by the agent. Nothing in the
calendar arithmetic is hand-rolled here — that is `lunar-rs`, and the
division of labour is described in `AGENTS.md`.

Two things keep that honest. Every documented sample in the READMEs is
copied from a run of the binary and checked byte-for-byte on every push, and
the whole of `AGENTS.md` is a set of contracts — the 干支 basis, the leap
month sign, the year pillar of the 干支 line — written down so a later change
has to argue with them rather than quietly break them.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
