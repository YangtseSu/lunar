# 计划 09 — 新增 `lunar bazi`：生辰八字（唯一读时刻的子命令）

Status: 已实施（2026-09-28，回填立案） · Priority: P2 · Depends: 无

> **本文件是回填。** 实现先于「新增能力也走计划流程」这条规则确立，规则随后按本
> 文件补写进 `docs/README.md`。本文件记录本应先写下来的内容：复现、归属、目标
> 行为与验收，与计划 01–08 同一规格。

## 现象

`lunar-rs` 早已实现八字，入口是 `Lunar::eight_char`（`src/eight_char.rs`，64 个
公开方法：四柱、十神、藏干、纳音、十二长生地势、胎元/命宫/身宫、大运/流年/流月），
本仓库**一个都没暴露**。用户在 `lunar` 里拿不到任何八字信息：

```console
$ lunar bazi 1990-06-15T10:30
error: unrecognized subcommand 'bazi'

Usage: lunar <COMMAND>

For more information, try '--help'.
$ echo $?
2
```
引擎侧实测（证明能力齐备，且本仓库未用）：

```
1990-06-15 10:30  八字=庚午 壬午 辛亥 癸巳  纳音=钗钏金
2026-02-04 12:00  八字=丙午 庚寅 己酉 庚午  纳音=大驿土
```

（该 64 个方法为 `grep -c 'pub fn ' src/eight_char.rs` 实测。）

## 为什么不能挂进 `date -a`

八字四柱缺一柱不成立，而第四柱是**时柱**——它必须有出生时刻。本仓库的非目标明写
「a clock or time-of-day」（AGENTS.md「Goal」节），黄历也因同一理由拒绝 `time_yi` /
`time_chong`。所以这不是加一个分组、也不是加一个开关，而是**打破一条既定原则**，
需要独立入口，并把该原则改写为"`date` / `cal` 无时钟"。

## 归属

**本仓库未暴露的能力**，不是缺陷、不是上游问题。引擎两个基准都提供且都正确，本
仓库要做的是选一个并说明理由（见下「取舍」）。

## 目标行为

新增子命令 `lunar bazi [生辰]`，位置参数一个 `date(1)` 风格字符串，可带时刻。

1. **给了时刻**：印年月日时**四柱**，加十神、藏干、纳音、地势四行。
2. **没给时刻**：印**三柱**并附 `说明: 未给时刻，无时柱`；十神/藏干/纳音/地势
   三列。**不补时柱**——不给时刻而造一个，是编造用户没写的事实。
3. **语法共用**：日期由 `datestr::parse_moment` 读，与 `date -d` 同一份语法，
   失败信息也一致（`bazi 2023-02-30` 与 `date -d 2023-02-30` 同错）。
4. **不接受 `-l`**：生辰是公历时刻，没有"农历时刻"可言。
5. 日柱的十神印作 `日主`（日干即自身，无十神可名）。
6. 行内以 ` / ` 分隔而非空格对齐：藏干一格宽 2–3 字（午=丁己，巳=丙庚戊），
   空格对齐会错位。

## 实施步骤

1. `src/datestr.rs`：新增 `Moment { day, seconds_of_day }` 与 `parse_moment`，
   复用 `parse` 取日、再从同一文本读回时刻；新增 `clock_of` / `clock_seconds`。
   **裸时钟必须带冒号**（见「实施时发现」）。
2. `src/calendar.rs`：新增 `CalError::BadTimeOfDay` 与其中文消息；
   `solar_at`（带时刻的 `Solar`，自带时刻范围检查）；`eight_char(&Lunar)`
   ——取 `&Lunar` 而非 `&Solar`，因为 `EightChar` 借用它所读的 `Lunar`。
3. `src/commands/bazi.rs`（新）：`BaziArgs` + `run`，签名与另两个子命令一致。
4. `src/main.rs`：注册 `Command::Bazi` 与分发臂；模块文档与 `about` 改为三个子命令。
5. 测试（`tests/documented_examples.rs`，六个）——见验收。
6. 文档：README 新增 `## lunar bazi` 整节 + 三个 console 块；AGENTS.md 改非目标
   （"a clock in `date` or `cal`"）、架构图、`datestr` 职责、加两条契约；
   `docs/parity.md` 新增八字专节；`docs/README.md` 记本规则。

## 取舍（登记于 `docs/parity.md`）

- **无时刻则三柱**，不补时柱（同上）。
- **月柱取节气瞬时基准**，与 `date` 的干支行不同：八字排的是一个**出生时刻**
  ——白露 22:41 之前出生属申月、之后属酉月；而 `date` 答**一整天**，整天都叫酉月。
  引擎 `EightChar` 全程是瞬时口径。两者在每个节气当日不同（1–9999 共 119,988 天），
  其余全同；年柱与日柱始终同源。**两者都对，不得互相"修正"。**
- **不做大运**：需要性别，且起运月数算法各家不同。

## 验收

```console
$ lunar bazi 1990-06-15T10:30
公历: 1990年6月15日 10:30
农历: 庚午年五月廿三
八字: 庚午 / 壬午 / 辛亥 / 癸巳
十神: 劫财 / 伤官 / 日主 / 食神
藏干: 丁己 / 丁己 / 壬甲 / 丙庚戊
纳音: 路旁土 / 杨柳木 / 钗钏金 / 长流水
地势: 病 / 病 / 沐浴 / 死
```

测试：

- `bazi_prints_four_pillars_only_with_a_time` —— 四柱全行；无时刻则三列且带
  `说明`，且不含 `癸巳`。
- `bazi_and_date_agree_on_the_year_and_day_pillars` —— 六个日子里八字年柱、日柱
  与 `date` 的干支行相同。
- `the_bazi_month_turns_at_the_solar_term_instant` —— 2026-09-07 20:00 属丙申、
  23:00 属丁酉（白露 22:41:16），而 `date -d 2026-09-07` 整天是丁酉。
- `a_zi_hour_belongs_to_the_day_it_began_in` —— 23:30 仍属当日日柱，次日 00:30
  属次日；两处皆子时。
- `bazi_reads_the_same_date_forms` —— 裸日期不得凭空得到时刻；四种写法同一瞬时。
- `bazi_refuses_what_date_refuses` —— 四个坏输入与 `date -d` 逐字同错；
  位置参数过多报 `位置参数过多`。

## 实施时发现

- **`split_zone` 会把日期的 `-` 当时区符号**，`2025-01-29` 因此被切成 `2025` +
  `-01-29`，而紧凑 `hhmm` 分支把 `2025` 读成 **20:25** —— 于是每个连字符日期都被
  凭空造出一个时刻和一个时柱。修法：裸时钟必须含冒号，与 `parse_absolute` 既有
  规则一致。测试 `bazi_reads_the_same_date_forms` 钉住。
- **`Terrain` 与 `hide_gan` 的表是对的**，第一版曾误判为错（把 9 个天干看成"全表"），
  实际 `午=丁己`、`亥=壬甲`、`巳=丙庚戊` 正是藏干本气/中气/余气。

## 非目标

- **大运 / 流年**（另立计划；需性别与起运算法）。
- **`-f` 令牌**：`-f` 的令牌是按"一天"设计的，八字行结构不同，不硬套。
- **不改 `date` / `cal` 的任何输出**。
