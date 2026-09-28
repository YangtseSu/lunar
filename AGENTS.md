# Repository Guidelines

## Goal

`lunar` is a thin command-line wrapper around the
[`lunar-rs`](https://crates.io/crates/lunar-rs) 寿星天文历 engine. It exists to
display, query and convert the Chinese lunisolar calendar, through three
subcommands — two modelled on the Unix tools they are named after, and one the
tools have no counterpart for:

- **`lunar date`** — one day's almanac (公历 / 星期 / 农历 / 干支 / 生肖 / 节气 /
  法定 / 星座), with `date(1)`-style input (`-d`), a `-f` format engine and a
  `-a` 黄历 block;
- **`lunar cal`** — `cal(1)`-style month and year grids overlaid with lunar days,
  solar terms, festivals and the statutory calendar, over civil months or, with
  `-L`, lunar months;
- **`lunar bazi`** — 生辰八字: the four pillars of a birth moment, with 十神,
  藏干, 纳音, 地势, 五行, 旬空, 地支十神 and 命局.

**This repository never implements calendar arithmetic.** It parses arguments,
shapes `lunar-rs` output and lays out grids. That is the whole job. When the
engine and this tool disagree, the engine is right; when the engine lacks
something, fix it upstream, not here.

Non-goals: a library crate, a clock in `date` or `cal`, a language layer, async,
weekday-derived holiday guesses. Do not add one without being asked.

`lunar bazi` is the single exception to the no-clock rule, and it exists
because a 时柱 cannot exist without one: it reads a time of day, keeps it, and
prints four pillars, while a birth moment with no clock prints three and says
so. `date` and `cal` still answer for a day alone.

Behaviour that differs from `cal(1)` / `date(1)` is a bug with a plan, not a
feature: the open ones live in [`docs/plans`](docs/plans).

## Dependency First

The upstream checkout is at `~/projects/lunar-rs` — read it before writing code
that answers a calendar question. `Solar` exposes ~144 public methods and `Lunar`
~305 across roughly thirty calendar systems; the odds that the question is
already answered are high.

| To find | Look in |
|---|---|
| A date conversion or attribute | `src/solar.rs`, `src/lunar.rs` — `grep 'pub fn'` |
| Name tables (weekday, month, day, ganzhi, zodiac) | `src/solar_util.rs`, `src/lunar_util/tables.rs` |
| Festivals and solar terms | `src/festival.rs`, `src/lunar_util/maps.rs`, `src/solar_util.rs` |
| Other calendar systems | `src/<system>.rs`, re-exported from `src/lib.rs` |
| The astronomy | `src/shou_xing/` |
| The statutory table (放假 / 调休) | `src/holiday_util.rs`, `src/holiday.rs`, `src/holiday_data.rs` — `Solar::get_legal_holiday` |

Two consequences:

1. **Add to `src/calendar.rs`, not to the command modules.** It is the single
   boundary to the engine and every wrapper in it is a one-line forward. A second
   path to `lunar-rs` is a design regression.
2. **Adapt quirks in the wrapper that owns them.** `solar()` maps the engine's
   `GregorianGap` to a range error; `solar_from_lunar` round-trips across the
   lunar/civil year mismatch. A wrapper that stops changing anything is dead
   weight — delete it.

Traps that have already cost this project a defect, and the reason each is now
written down:

- **Festivals come from three sources, and all three are needed.**
  `Solar::festivals()` (civil: 国庆节, 劳动节, weekday-floating ones),
  `Lunar::festivals()` (春节, 中秋节) and the **typed** `LunarFestival::from_ymd`,
  which alone knows 清明节, 上巳节, 中元节 and 冬至节. `calendar::traditional_festivals`
  chains them; `festival_rank` prefers the name a reader scans for. A name in
  `PRINCIPAL` that is not spelled as the engine spells it (`清明节`, not `清明`)
  sorts last and loses.
- **A lunar year and a civil year do not line up.** 腊月 always begins in the
  following January, so 农历 2026 年腊月初一 is 公历 2027-01-08. Lunar→civil must
  go through `calendar::solar_from_lunar`, which validates by whole-triple round
  trip; comparing civil years refuses 336,947 days and makes 腊月 unreachable.
- **Leap months are negative numbers** (`-4` = 闰四月). `calendar::m_abs` renders
  them and is also fed user input by error paths, so it range-checks before
  indexing the month table.
- **A 5-digit year is not a range error**: `date -d 10000-01-01` falls out of the
  grammar as unparsable, while `cal 10000` fails the range check. Both are
  correct; do not "fix" one into the other.
- **The 干支 line is one chain, and it is not the lunar year.** Two engine
  month pillars exist and both are correct: `month_in_gan_zhi()` turns at the
  節氣 **day**, `month_in_gan_zhi_exact()` at the 節氣 **instant**; they differ
  on each of the twelve 節氣 days (119,988 in 1–9999). `Lunar::day_yi` and
  `day_ji` key their 宜 / 忌 tables on the first, so the `干支:` line prints it
  — anything else makes the line and the advice contradict each other. The
  month stem comes from 五虎遁 off the **立春** year stem, so that line's year
  pillar is `year_in_gan_zhi_by_li_chun()`, not `year_in_gan_zhi()`; mixing
  bases emits pairs 五虎遁 cannot produce (74,948 days in 1–9999). The 农历
  line, `%G`, `%S` and the `cal -L` titles keep the **春节** basis: they name
  the lunar year, not the 干支 chain, and the two legitimately differ between
  春节 and 立春.

## Architecture & Data Flow

There is no `lib.rs` — everything is binary-internal, declared in `src/main.rs`.
Each module has one job and the dependency direction is one-way:

```mermaid
graph TD
  A[main.rs<br/>clap Cli/Command<br/>ExitCode] --> B[commands/date.rs]
  A --> C[commands/cal.rs]
  A --> N[commands/bazi.rs]
  A --> D[tz.rs]
  B --> E[datestr.rs]
  N --> E
  B --> F[format.rs]
  C --> G[calgrid.rs]
  G --> H[cell.rs]
  B --> I[calendar.rs]
  C --> I
  N --> I
  I --> J[lunar-rs]
  G --> L[lang.rs<br/>width/pad_right]
  G --> M[mark.rs<br/>Mark/Color/paint]
  H --> M
  M --> L
  D --> K[civil.rs]
  I --> K
  E --> K
```

`bazi` is a peer of the other two, not a group of `date`'s: it shares
`datestr` (one grammar) and `calendar` (one engine boundary) and nothing else.

| Path | Role |
|---|---|
| `src/calendar.rs` | the only `lunar-rs` boundary: `CalError`, `MIN_YEAR`/`MAX_YEAR`, one-line wrappers |
| `src/civil.rs` | proleptic-Gregorian epoch ↔ civil bridge (Hinnant), `CivilDate` stepping; the engine models the 1582 reform and refuses 1582-10-05..=14, which the parser and grids must cross |
| `src/calgrid.rs` | grid layout: `Entry`/`Grid`, `GUTTER`, pitch, `render`; each cell carries its `Mark` |
| `src/cell.rs` | cell content priority, `CellStyle` |
| `src/datestr.rs` | the `-d` grammar; `parse` drops the clock, `parse_moment` keeps it for `bazi` |
| `src/format.rs` | the `-f` token engine |
| `src/lang.rs` | display-width measurement and `pad_right` |
| `src/mark.rs` | `Mark`, `Color`, `paint` — the only place SGR is written |
| `src/commands/` | the three subcommand implementations |
| `src/tz.rs` | local "today" from `TZ` / the system zone |

**Data flow.** `main()` parses clap, prints `--help-format` early if asked,
resolves `today` once via `tz::today()`, builds one `String`, dispatches to
`date::run` / `cal::run` / `bazi::run`, and writes the buffer once. On `Err` it
prints `lunar: {error}` to stderr, returns `ExitCode::FAILURE`, and **discards
the buffer** — a mid-loop failure in `cal` prints nothing on stdout.

All three commands share the signature:

```rust
pub fn run(args: &XArgs, today: CivilDate, out: &mut String) -> Result<(), CalError>
```

Command modules must never print. They append to `out` and return an error.

Invariants, in the order they are easiest to break:

- **`calendar.rs` is the only path to `lunar-rs` for calendar answers.** New
  wrappers go there, and they either shape output or adapt a quirk.
- **A wrapper that adapts a quirk earns its place; one that only forwards
  does not.** `calendar::eight_char` takes a `&Lunar`, not a `&Solar`,
  because `EightChar` borrows the `Lunar` it reads — a `&Solar` would have to
  build a temporary and hand back a borrow of it. `calendar::solar_at` is the
  one constructor that carries a clock, and it range-checks the clock itself
  because `lunar-rs` takes an `i32` per field and would not.
- **The two injection points are `today: CivilDate` and the `CellStyle` value
  struct.** Flags are mapped to `CellStyle` in exactly one place, `cal::run`. The
  `Color` mode is resolved in `main` and passed beside `today`; no module reads
  global state.
- **Cell priority is policy, not configuration.** `cell::content` is the single
  source of truth: `节日 > 初一显示月份名 > 节气 > 农历日`. The month-name level
  is skipped on 正月 so 春节 keeps its festival label; `--number` only affects
  the final level. Do not reorder or reimplement it elsewhere.
- **The statutory calendar is a mark, not a label.** 放假 and 调休 are painted
  (`mark::paint`), never written into the cell: a label there would displace the
  festival name or lunar day, which are the facts the tool exists to print. The
  cost is stated plainly: a piped grid shows no mark; `--color=always` is the way
  out, and `lunar date` reports the statutory calendar in words.
- **The two marks compose; neither wins.** `Mark::sgr` joins the parameters into
  one run — a 放假 reference day is `\x1b[31;7m`. Padding goes *inside* the SGR
  run, so a colourless and a coloured grid differ in trailing spaces only.
- **Grid width is display width**, measured by `lang::width` (CJK counts 2).
  Rust's `{:>width$}` counts characters and would misalign every CJK cell.
  `GUTTER` must stay: a cell that exactly fills its column would otherwise run
  into its neighbour (`5/235/24…`).
- **Only the days the month has are drawn.** Civil months enumerate
  `CivilDate::nth_existing_day`; lunar months ask the engine
  (`calendar::lunar_month_days`). Never step a month locally with `add_days` —
  the engine's calendar skips the ten reform days and uses the Julian leap rule
  before 1600, so local stepping drifts and walks into the gap.

## Conventions

- **Comments.** Module / function / struct docs are English prose; every `pub`
  item carries one. They describe what the code *is*: no history, no "used to",
  no strikethrough. All user-facing strings are Chinese, including errors and
  clap help. Do not add Chinese prose to doc-comments.
- **Errors.** One type, `CalError` (`src/calendar.rs`), with a hand-written
  Chinese `Display` using full-width punctuation, and `impl std::error::Error`.
  Every fallible path returns `Result<_, CalError>`; no `unwrap`/`expect` in
  `src/` (tests only). Helper constructors: `check_year`, `check_month`,
  `check_day`, `check_lunar_month`, `solar`, `solar_from_lunar`.
  `tests/documented_examples.rs` substring-matches the messages — **changing an
  error message is a suite change, in the same commit.**
- **Rendering.** Build into `&mut String` with `std::fmt::Write`
  (`let _ = write!(out, …)`), never `println!` — except the `--help-format`
  short-circuit. The `let _ =` is deliberate: a `String` cannot fail.
- **Formatting.** Plain rustfmt defaults; no config file. One import per line,
  no glob imports, std / external / `crate::` groups separated by blank lines.
- **No async.** Nothing to await; keep it synchronous.

## Command Surface Contracts

- **`-d` is 公历, `-l` is 农历, and the two differ in nothing else.** Both
  channels go through `datestr` — a `-d` string and a single positional through
  `parse`, the three positionals through `from_parts` — so a form one channel
  accepts the other must accept too, with the same failure. This invariant has
  broken before (`-l` re-reading only the positionals); it is pinned by
  `the_two_channels_differ_only_in_the_calendar_they_read`.
- **Adding a `-d` form:** extend `parse`'s precedence chain — keyword → `@epoch`
  → absolute → weekday → relative, first match wins. A new *absolute* shape must
  go through `Parts` + `resolve`; calling `calendar::solar` directly makes it
  civil-only. Relative units live in `apply_unit`; sub-day units are **refused**
  (`CalError::SubDayUnit`), not ignored.
- **What `-l` must not change:** keyword, `@epoch`, weekday and relative forms
  name a *day*, so they resolve against the reference exactly as before. A
  time of day is civil syntax — a suffix on an absolute date **or a bare
  `15:30` of its own** — and is refused under `-l` rather than dropped.
- **Adding a `-f` token:** extend the `match` in `format.rs` **and** all three
  tables — the module doc, `FORMAT_HELP` (`src/main.rs`) and the README table.
  `--help-format` must list every token; the suite checks it does. The Chinese
  README carries the same table, so the count is four, not three;
  `the_chinese_readme_stays_a_translation` is what keeps them in step.
- **The 黄历 is one block, and `-a` is its only switch.** `calendar::almanac`
  builds the whole set of `label: text` lines; there is no per-group flag and
  none may be added — a reader asking for 冲煞 gets 冲煞, not a fifth of the
  answer. Two groups are seasonal (`shu_jiu`, `fu`) and their lines are
  **omitted** when the engine has no value, the same way the `节气` line is. The
  block is `conflicts_with = "format"`: the profile, the block and `-f` are three
  shapes of one answer, and only one of them prints. A field that depends on a
  time of day (`time_yi`, `time_chong`) is out of scope — the block answers
  about a **day**, and `lunar bazi` is where a time of day is read at all — and
  `lunar cal` never shows the block: a cell has room for a label, not a 黄历.
  The block is never a group of `bazi`'s either; a chart and a day's almanac are
  different answers to different questions.
- **Adding a `cal` flag:** map it to `CellStyle` in `cal::run`, the single seam;
  do not grow a second configuration path.
- **Span semantics are `cal(1)`'s, and `span_of` is the one place they live.**
  `-3` centres on the month named, `-n N` starts at it, `-y` is the year it
  belongs to, and a month the arguments leave out is today's (`-L`: today's
  lunar month). Both views slice one continuous month sequence and cross year
  boundaries freely — the lunar one through `lunar_sequence`, which loads
  neighbouring lunar years. A window that leaves 1–9999 is reported, never
  clipped.
- **Two range errors, two facts.** A window reaching into 10000
  (`cal -L 9999 12 -n 3`) and a month's own days reaching out of it
  (`cal -L 9999`, whose 腊月 ends on 10000-01-27) are different things and
  read differently: the first names the year the window reached, the second
  the month that crosses. A lunar month is never clipped to fit the range —
  a clipped grid and `lunar date` would answer differently about one day.
- **`lunar bazi` reads a time of day, and the 月柱 turns at the 節氣
  *instant*.** The engine's `EightChar` is instant-based throughout, which is
  right for a birth: 白露 at 22:41 puts a 20:00 birth in 申月 and a 23:00 one
  in 酉月, while the `干支` line of `date` — which names a *day* — calls the
  whole of 09-07 酉月. The two differ on each 節氣 day and agree on the rest;
  do not "fix" one to match the other. The 年柱 and 日柱 *are* shared with
  `date`, and must stay so.
- **A birth moment with no clock has no 时柱, and no 命宫 or 身宫.** `bazi`
  prints three pillars and a `说明` line saying so; it never supplies a clock
  of its own. 命宫 and 身宫 are counted from the 時辰, so the no-clock chart
  drops them from the `命局` row and says why — the engine *would* answer,
  from a noon it was handed, and a noon is not the moment asked about. 胎元 and
  胎息 need no 時辰 and stay. The trap is
  `split_zone` reading the `-` in `2025-01-29` as a zone sign and leaving
  `2025`, which the compact `hhmm` branch of `parse_clock` would then read as
  20:25 — so a bare clock must be required to carry a colon, exactly as
  `parse_absolute` already does. A 子时 belongs to the day it began in.
- **`sect` is one engine parameter with two meanings, and this tool pins it to
  2 in both.** In `EightChar` it reaches the **day pillar only** (sect 1 moves a
  23:00 子时 to the next day, sect 2 does not); in `Yun` it selects one of **two
  起运 algorithms** (sect 1 counts 时辰, sect 2 counts minutes, differing by up
  to eight days). The engine's own defaults disagree — `eight_char()` is 2,
  `yun()` is 1 — so `calendar::yun` must pass the value explicitly. Neither is
  exposed as a flag: both are professional choices, and `date` and `bazi` have
  to agree about the day pillar.
- **A missing `-g` costs the 大运, not the chart.** 顺逆 needs a gender and there
  is none to assume, so `bazi` prints the four pillars and a `说明` line and
  exits 0 — the same choice a missing time of day makes about the 时柱.
  `calendar::yun` takes `&EightChar`, not `&Lunar`, because `EightChar` borrows
  the `Lunar` it reads.
- **`-y` is a query about one year, and a chart answers it only when it can.**
  The 流年 / 流月 / 小运 hang a layer below the ten 大运, and all hundred of
  each are unreadable by default, so they print for the year `-y` names and for
  no other. Three facts decide what a run can answer, and each is reported
  rather than defaulted: the 流年 is counted **on** 大运, so no `-g` means
  `说明: 流年需性别`; the year must be **owned by a step**, and one before
  起运 or past the tenth has none, so the year is reported with the span that
  does (`说明: … 年无大运 (大运 1997-2086)`) instead of a neighbour's value;
  and the 小运 is counted **from the 时柱** one step per year, so a no-clock
  chart prints the 流年 and 流月 and says `说明: 小运需时柱`. That is the
  命宫 / 身宫 rule, for the same reason: the engine would answer from the noon
  this tool substitutes, and a noon is not the moment asked about. The 流月
  are named in 农历 months (`正月` … `冬月` `腊月`) — a 流月 runs on 农历
  months from 立春, so a civil `1月` there would name a different month. The
  engine's index-0 step (the span up to 起运) carries a 流年 and is **skipped**:
  a chart has no 大运 pillar for those years to sit on. `-y` takes a bare
  integer, not a `-d` string — it is a year, not a moment — and is range-checked
  to 1–9999 before the 运程 arithmetic runs.
- **`bazi`'s `write_chart` is a dispatcher, not a sequence of `writeln!`
  calls, and that is load-bearing.** As one function it had lost a row twice in
  three rounds of change, caught only because the suite pins the whole chart
  byte-for-byte. It is split into `write_pillar_rows` / `write_ming_jun` /
  `write_yun` / `write_luck_year`, and the column count that trims a
  three-pillar chart to three cells lives in exactly one place, `column_count`.
  Keep it that way. When editing it, the pinned chart in
  `bazi_prints_four_pillars_only_with_a_time` is the contract — if the output
  is meant to change, that test is what must be updated first, never after the
  fact.
- **A 八字 row is about a pillar or it is about the chart; never both.** 七
  rows carry four cells aligned to the four pillars (十神 / 藏干 / 纳音 / 地势 /
  五行 / 旬空 / 地支十神) and are cut to three when there is no 时柱. The
  `命局` row carries four single values that belong to no pillar and is not
  aligned. 五行 is the 干's element then the 支's, not the 纳音's; 地支十神 is
  one 十神 per 藏干, so a cell is a phrase. Both are the engine's per-character
  answer, and this tool only lays them out.
- **The 时柱 is counted from the clock, and this tool never says otherwise.**
  `bazi` charts the instant it is handed; the 时柱 is 钟表时, uncorrected for
  longitude, daylight saving or the equation of time. Do not describe, imply
  or let a flag suggest a 真太阳时 chart — there is no such thing here, and a
  reader who assumes one is reading a different 时柱, a different 日柱 and a
  different 起运. Correcting needs the birthplace longitude, an input this
  tool has no concept of, and the schools disagree about which definition to
  apply. Were it ever added, the 均时差 has to come from the engine: `sa_lon_t`
  / `gxc_sun_lon` / `nutation_lon2` are private and `mod shou_xing` re-exports
  four items, so the upstream change is a new public surface, not a helper. A
  correction also moves the 日柱 and the 年柱 when it crosses midnight or a
  节氣 instant, so it is never a 时柱-only switch.

## Testing & QA

One file, one mechanism: `tests/documented_examples.rs` spawns the real binary
through `run` / `run_failing` and compares output. There are no unit tests in
`src/`, no mocks, no fixtures, no snapshot framework, no dev-dependencies. No
locale is set: the tool speaks Simplified Chinese unconditionally.

- **Copy expected output from a real run.** The column width is per-grid (the
  widest cell that month holds), so hand-computed padding is wrong by
  construction. A few tests pin whole grids byte-for-byte as raw strings.
- **The READMEs' `console` blocks are executable samples.** Every `$ lunar …`
  line in `README.md` *and* `README.zh-CN.md` must reproduce byte-for-byte;
  re-verify after any behaviour change. The translation is a copy, so it
  rots the moment one of them is edited and the other is not.
- **The reference day is not a testable input.** `tz::today()` is the only
  source; pin the *mechanism* instead, as
  `the_statutory_calendar_and_the_reference_day_compose` does (a combined run
  must reduce to a bare inversion under `--no-holiday`).
- **A painted cell is not a `split_whitespace` away.** Padding sits inside the
  SGR run; locate cells on the grid pitch (`split_on_pitch` / `cell_of` /
  `paint_of`), treat an SGR run as zero columns, and cut a cell boundary before a
  run that opens the next cell.
- **Ask the binary which days a month has** — never derive `1..=length`. October
  1582 has 21 days numbered 1, 2, 3, 4, 15 … 31, and the Julian rule applies
  before 1600. The suite's own `existing_days` helper (it asks the binary) is the
  shape to copy.
- **Engine wins over samples.** Where a published sample and the astronomy
  disagree, the engine is correct; do not "fix" it to match the sample.

## Development

```bash
cargo build --release        # release: lto, codegen-units=1, strip
cargo run -- cal 2026 9
cargo run -- date -d 2026-09-07
cargo test                   # the integration suite
cargo fmt                    # required: `cargo fmt --check` must be clean
cargo clippy --all-targets   # required: zero warnings
```

Rust 2024 edition, distro `rust` (no `rust-toolchain.toml`), plain `cargo`, no
workspace, `Cargo.lock` tracked, `target/` ignored. There is no CI: the gates
above are local, and nothing is "done" until they pass.

## Commit Discipline

- **One plan, one commit.** A commit is one coherent working change — never two
  features, never a half-done step. It must build, pass `cargo test`, and leave
  the tree a reader can check out and understand.
- **Documentation moves with behaviour.** A user-facing change updates the
  README, the `-f` tables and any affected section of this file in the same
  commit. A defect fixed is **deleted** from the known list in that commit — the
  list describes the tree, not its history.
- **Message.** One imperative subject under 72 characters, naming the behaviour;
  the body explains what was wrong and what changed. Prose subjects, not
  conventional-commit prefixes.
- **Never `git push`** without an explicit instruction. Committing locally is the
  end of the job.

## Implementation Plans

[`docs/plans`](docs/plans) holds one file per defect, with reproduction,
target behaviour, steps and acceptance. Every plan in it has been executed:
the queue is empty, and a new divergence starts as a new file. Execute them
one at a time; each is its own commit and its own verification.
`docs/parity.md` records the `cal(1)` / `date(1)` compatibility matrix, and
`docs/README.md` tracks which plans are done — a divergence goes there before
it is judged.


