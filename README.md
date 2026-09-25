# lunar

A Chinese lunisolar calendar command line tool, in the shape of
[`cal_nongli`](https://crates.io/crates/cal_nongli):

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
公历：2026年9月7日 星期一
农历：丙午年七月廿六
干支：丙午年 丙申月 甲申日
生肖：马
节气：白露
```

The `节气` line is omitted when the day carries no solar term. A leap month shows
up in the month name:

```console
$ lunar date -d 2020-05-23
公历：2020年5月23日 星期六
农历：庚子年闰四月初一
干支：庚子年 辛巳月 丙寅日
生肖：鼠
```

The day can also be given as positionals, or as a `date(1)` style string:

```console
$ lunar date 2026 2 17
$ lunar date -d '2026-09-04'
$ lunar date -d 'next friday'
$ lunar date -d '@1788000000'
```

`-d` understands `now` / `today` / `tomorrow` / `yesterday`, epoch seconds
(`@…`), ISO 8601 dates and times (with `Z` or `±hh:mm` zones), `YYYY/MM/DD`,
`MM/DD/YYYY`, relative offsets (`+3 days`, `-2 weeks`, `90 minutes ago`,
`1 fortnight`, `next month`) and weekday names (`monday`, `next friday`,
`last sun`).

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
`周%A` = 周一, `礼拜%A` = 礼拜一. A backslash escapes the next character, so
`\n` and `\t` produce a newline and a tab. `lunar date --help-format` prints the
same table.

## `lunar cal`

Without arguments it prints the current civil month, with the lunar day, solar
term or festival under each date:

```console
$ lunar cal 2026 9
      2026年9月
     一     二     三     四     五     六     日
             1      2      3      4      5      6
            二十     廿一     廿二     廿三     廿四     廿五
      7      8      9     10     11     12     13
     白露   廿七   廿八   廿九   八月   初二   初三
     14     15     16     17     18     19     20
     初四   初五   初六   初七   初八   初九   初十
     21     22     23     24     25     26     27
     十一   十二   秋分   十四   中秋   十六   十七
     28     29
     十八   十九
```

Cell content follows a fixed priority — **节日 > 初一显示月份名 > 节气 > 农历日** —
so 11 September shows 八月 instead of 初一, 7 September is covered by 白露 and
25 September by 中秋.

`-L` switches to lunar months, where each cell leads with the civil date:

```console
$ lunar cal -L 2026 7
     农历 丙午年 七月
     一     二     三     四     五     六     日
                    8/13  8/14  8/15  8/16
                      七月    初二    初三    初四
  8/17  8/18  8/19  8/20  8/21  8/22  8/23
    初五    初六    七夕    初八    初九    初十    处暑
  8/24  8/25  8/26  8/27  8/28  8/29  8/30
    十二    十三    十四    中元    十六    十七    十八
  …
```

处暑 lands on 8/23 and 中元 — the fifteenth of the seventh lunar month — on
8/27, both from the astronomical engine.

Options:

| option | meaning |
|---|---|
| *(none)* | the current civil month (the current lunar month with `-L`) |
| `年` | the whole year |
| `年 月` | that month |
| `-L`, `--lunar` | show lunar months instead of civil months |
| `-R`, `--leap` | with `-L`, select the leap month |
| `-s`, `--sunday` / `-m`, `--monday` | first day of the week (Monday by default) |
| `-y`, `--year` | whole year (current year by default) |
| `-3`, `--three` / `-n N`, `--months N` | N consecutive months |
| `--number` | lunar day as digits instead of 初一/廿六 |
| `--no-month-name` | never replace 初一 with the month name |
| `--no-festival` | never show festivals |

`lunar cal -L 2026` walks the whole lunar year, so it picks up a leap month on
its own; an explicit `-L 2020 4` prints the ordinary fourth month, and
`-L 2020 4 -R` prints 闰四月.

## Supported range

Years **1–9999**, the window `lunar-rs` can serve: its ShouXing astronomy is
bounded by the `LEAP_11` / `LEAP_12` tables, so outside it the engine's
extrapolated lunar months and solar terms are not meant to be relied on. Years
outside the range, impossible dates and the days the Gregorian reform skipped
(1582-10-05 … 1582-10-14) are reported as errors.

Because the cells are laid out by character count and every label is Chinese, the
output is designed for a terminal with a CJK-capable font.

## Implementation notes

- `src/calendar.rs` — the supported range, festival and ganzhi helpers, lunar
  month lookups. All answers come from `lunar-rs`.
- `src/civil.rs` — the epoch ↔ civil-date bridge. `lunar-rs` models the 1582
  Gregorian reform, so its `Solar` refuses 1582-10-05..14; the civil arithmetic
  needed by the `-d` parser and the grids lives here.
- `src/calgrid.rs` — grid layout. The cell pitch is 7 for the civil overlay and 6
  for the lunar view, while the weekday header is always 6 wide. Neighbouring
  month days are blanked the way `cal` does it: a grid never opens on a partial
  week, and a lone overflow day at the end is kept.
- `src/cell.rs` — the cell content priority.
- `src/datestr.rs` — the `date(1)` style `-d` parser.
- `src/format.rs` — the `-f` token engine.
- `src/commands/` — the two subcommands.

`lunar-rs` numbers months from 正月 (`寅`) for the month pillar, which is the
traditional almanac convention; `tz-rs` resolves `TZ` for the local "today".

## Tests

```bash
cargo test
```

`tests/documented_examples.rs` pins the documented layout, the token table, the
overlay switches and the error messages, and separately pins the calendar values
by date (中元 on the fifteenth of the seventh lunar month, 芒种 on 2020-06-05,
and so on).

Where a published sample disagrees with the astronomical engine, the engine
wins: a few sample cells are stale (中元 shown three days early, 芒种 on the
wrong day, 雨水 and 惊蛰 missing), and this tool prints the computed dates.

## License

MIT
