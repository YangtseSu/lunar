# 计划 08 — 干支月 与 宜忌 所用月柱 用了两个不同基准

Status: 已实施（2026-09-28）· Priority: P1 · Depends: 无

## 现象

`lunar date -a` 打印的 `干支` 行与 `宜:` / `忌:` 行在同一天给出互相矛盾的事实。
`干支` 用 `calendar::month_gan_zhi`（`month_in_gan_zhi_exact()`，节气**瞬时**基准），
而 `day_yi()` / `day_ji()` 内部查表用的月柱是 `month_in_gan_zhi()`（节气**当日**
基准）——**这是本仓库的选择，不是引擎的缺陷**。

> **实施时的更正。** 本文原称 `month_in_gan_zhi()` 是「月序基准，正月起 `寅`」——
> 引擎里没有这一套。两个变体**都**遍历 `JIE_QI_IN_USE`（`lunar-rs/src/lunar.rs`
> 的 `compute_month`），区别只是比较粒度：前者用整日比较 `day_before(solar, end)`，
> 后者用瞬时比较 `solar.is_before(end)`。因此这是**当日 vs 瞬时**之差，不是月序 vs
> 节气；差异日确实只有每年 12 个节气当日（1–9999 共 119,988 天），这一点原文无误。

复现（2026-02-04，立春当日，立春 04:02:08 生效）：

```console
$ lunar date -a -d 2026-02-04
公历: 2026年2月4日 星期三
农历: 乙巳年腊月十七
干支: 乙巳 己丑 己酉          <-- 月柱 己丑
…
宜: 祭祀、祈福、求嗣、开光、出火、出行、拆卸、修造、动土、入宅、移徙、上梁、挂匾、开池、入殓、安葬、破土、启钻
忌: 嫁娶、作灶、安床
```

2026-02-04 的 `宜` 是按**庚寅**月查表得来的（引擎实测：`day_yi_by_sect(1)` 在该日
按 `month_in_gan_zhi()` = `庚寅` 取表），而我们印的月柱是 `己丑`。
`宜` 本身与市面黄历一致（huangli123 / 8s8s / zhuazhou 三家同日均印
`宜: 祭祀、祈福、求嗣、开光、出火、出行…忌: 嫁娶、作灶、安床`），
**错的是我们印的那行月柱**。

规模：1–9999 全量扫描，`month_in_gan_zhi()` 与 `month_in_gan_zhi_exact()`
不同的日子共 **119,988 天**（每年 12 天，全年都是节气当天）。`day_yi_by_sect(1)`
与 `_by_sect(2)` 的差异日**完全相同**的 12 天——这坐实了宜忌就是跟着月柱基准走的。

年柱原本同理但结论要改：`year_in_gan_zhi()`（春节基准）与 `year_in_gan_zhi_exact()`
（立春瞬时基准）在 1–9999 有 74,533 天不同。原文说「我们印的 `year_gan_zhi` 用春节
基准，这个是对的，无需改动」——**这条是错的**：月干由五虎遁自立春年干推出
（`compute_month` 取 `year_gan_index_by_li_chun`），所以春节年干配立春月干会印出
五虎遁不成立的柱对。1–9999 中有 **74,948 天**如此（与 74,533 不是同一个集合）。

## 归属

**本仓库的问题**，不是 `lunar-rs` 的问题。引擎两个基准都提供且都正确：

- `month_in_gan_zhi()` = 节气**当日**换月——市面黄历口径，`day_yi/ji` 用它；
- `month_in_gan_zhi_exact()` = 节气**瞬时**换月——`month_gan_index_exact` 同样走
  `JIE_QI_IN_USE` 遍历，只是比较用瞬时。

`src/calendar.rs` 的 `month_gan_zhi` 选了 `_exact()`，而它的 doc-comment 写着
「a lunar almanac numbers months from 正月 (`寅`) onwards … independent of the
solar-term instant rule (`Lunar::month_in_gan_zhi`)」——**注释指向的正是
`month_in_gan_zhi`，代码却调了 `_exact()`**，二者相反。选错的是这一行。

## 目标行为

`干支` 行的月柱改为**节气当日基准** `month_in_gan_zhi()`，与 `宜` / `忌` 同一事实，
也与市面黄历一致。

年柱一并改为**立春基准** `year_in_gan_zhi_by_li_chun()`，使 `干支:` 行三柱自洽
（五虎遁）。原文的验收值 `乙巳 庚寅 己酉` **不成立**——`庚寅` 是丙年的月，乙年正月
起 `戊寅`，且引擎的月干取自立春年干；正确值是 **`丙午 庚寅 己酉`**，与市面黄历
（wannianli123 标 `丙午年 庚寅月 己酉日`）一致。立春当日 `农历:` 行仍为 `乙巳`：
农历年按春节换，这是对的。

星座、法定与其余黄历分组不变。

## 实施步骤

1. `src/calendar.rs` 的 `month_gan_zhi` 改为 `lunar.month_in_gan_zhi()`，并重写
   doc-comment：说明**为什么是当日基准**——它是 `day_yi` / `day_ji` 查表的键，也是
   市面黄历的口径；`month_in_gan_zhi_exact()` 是瞬时基准，只在 12 个节气当日与之不同。
   `%H` 走同一个函数，自动跟随。
2. `src/calendar.rs` 新增 `li_chun_year_gan_zhi`（`year_in_gan_zhi_by_li_chun`），
   `date.rs` 的 `干支:` 行改用它；`农历:` 行、`%G`、`%S`、`cal -L` 标题**保持春节基准**
   （它们说的是农历年本身）。
3. `docs/parity.md` 登记「干支月取当日基准、干支年取立春基准」为**有意保留**，
   写明与宜忌同源这条理由。
4. `AGENTS.md`「Dependency First」的陷阱列表补一条：干支三柱必须同基准，月柱取
   `month_in_gan_zhi`、年柱取立春，`农历` 行与 `%G` 另走春节基准。

## 验收

```console
$ lunar date -a -d 2026-02-04 | sed -n '3p;7p'
干支: 丙午 庚寅 己酉
宜: 祭祀、祈福、求嗣、开光、出火、出行、拆卸、修造、动土、入宅、移徙、上梁、挂匾、开池、入殓、安葬、破土、启钻
```

测试（`tests/documented_examples.rs`，两个）：

- `the_month_pillar_is_the_basis_the_advice_is_keyed_on`：取 2026 的 12 个节气日
  `D`（`2026-01-05 02-04 03-05 04-05 05-05 06-05 07-07 08-07 09-07 10-08 11-07
  12-07`），断言各日的 `%H` 满足「节气当日已换月、次日不变」（当日基准的定义），
  且 `干支:` 行的月柱与 `%H` 相同。另 pinned 断言 2026-02-04 的 `干支:` 为
  `丙午 庚寅 己酉` 而 `农历:` 仍为 `乙巳年腊月十七`、`%G` 仍为 `乙巳`，并核对该日
  `宜` / `忌` 为庚寅月所对应者。
- `the_gan_zhi_line_is_one_self_consistent_chain`：在若干「春节与立春基准不一致」的
  日子上，按五虎遁核对 `干支:` 行年干与月干自洽。

## 非目标

- 不改 `农历:` 行、`%G`、`%S` 与 `cal -L` 标题的春节基准。
- 不引入第三种月柱基准（引擎的 `SixtyCycleMonth` 一类不在本工具使用范围内）。
- 不改 `cal`（网格不印干支）。
