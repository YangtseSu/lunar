# 计划 08 — 干支月 与 宜忌 所用月柱 用了两个不同基准

Status: 待实施 · Priority: P1 · Depends: 无

## 现象

`lunar date -a` 打印的 `干支` 行与 `宜:` / `忌:` 行在同一天给出互相矛盾的事实。
`干支` 用 `calendar::month_gan_zhi`（`month_in_gan_zhi_exact()`，节气瞬时基准），
而 `day_yi()` / `day_ji()` 内部查表用的月柱是 `month_in_gan_zhi()`（月序基准，
正月起 `寅`）——**这是本仓库的选择，不是引擎的缺陷**。

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

2026-02-04 的 `宜` 是按**庚寅**月查表得来的（引擎实测
`day_yi_by_sect(1)` 在该日等于月序基准的结果），而我们印的月柱是 `己丑`。
`宜` 本身与市面黄历一致（huangli123 / 8s8s / zhuazhou 三家同日均印
`宜: 祭祀、祈福、求嗣、开光、出火、出行…忌: 嫁娶、作灶、安床`），
**错的是我们印的那行月柱**。

规模：1–9999 全量扫描，`month_in_gan_zhi()` 与 `month_in_gan_zhi_exact()`
不同的日子共 **119,988 天**（每年 12 天，全年都是节气当天）。`day_yi_by_sect(1)`
与 `_by_sect(2)` 的差异日**完全相同**的 12 天——这坐实了宜忌就是跟着月柱基准走的。

年柱同理但更少：`year_in_gan_zhi()`（春节基准）与 `year_in_gan_zhi_exact()`（立春
瞬时基准）在 1–9999 有 74,533 天不同。我们印的 `year_gan_zhi` 用春节基准，
这个是对的（农历年干支按春节换年），无需改动。

## 归属

**本仓库的问题**，不是 `lunar-rs` 的问题。引擎两个基准都提供且都正确：

- `month_in_gan_zhi()` = 月序基准（正月 `寅`）——市面黄历口径，`day_yi/ji` 用它；
- `month_in_gan_zhi_exact()` = 节气瞬时基准——`month_gan_index_exact` 走
  `JIE_QI_IN_USE` 遍历。

`src/calendar.rs` 的 `month_gan_zhi` 选了 `_exact()`，而它的 doc-comment 写着
「a lunar almanac numbers months from 正月 (`寅`) onwards, which is the traditional
月柱」——**注释描述的是月序基准，代码用的是节气基准**，二者相反。选错的是这一行。

## 目标行为

`干支` 行的月柱改为**月序基准** `month_in_gan_zhi()`，与 `宜` / `忌` 同一事实，
也与市面黄历一致。2026-02-04 的 `干支` 变为 `乙巳 庚寅 己酉`。

年柱、星座、其余黄历分组不变。

## 实施步骤

1. `src/calendar.rs` 的 `month_gan_zhi` 改为 `lunar.month_in_gan_zhi()`，
   并重写 doc-comment：说明**为什么是月序基准**——它是 `day_yi` / `day_ji`
   查表的键，也是市面黄历的口径；`month_in_gan_zhi_exact()` 是节气瞬时基准，
   只在 12 个节气日与之不同，本工具不选它，因为黄历里月柱和宜忌必须是同一个
   事实。`%H` 走同一个函数，自动跟随。
2. `docs/parity.md` 登记「月柱取月序基准」为**有意保留**（对 date(1)/cal(1) 无
   对应，此处是本工具的既定选择），写明与宜忌同源这条理由。
3. `AGENTS.md`「Dependency First」的陷阱列表补一条：干支月与宜忌必须同基准；
   两者都取月序（`month_in_gan_zhi`），不要用 `month_in_gan_zhi_exact()`。

## 验收

```console
$ lunar date -a -d 2026-02-04 | sed -n '3p;7p'
干支: 乙巳 庚寅 己酉
宜: 祭祀、祈福、求嗣、开光、出火、出行、拆卸、修造、动土、入宅、移徙、上梁、挂匾、开池、入殓、安葬、破土、启钻
```

测试（`tests/documented_examples.rs`，一个）：

- `the_month_pillar_is_the_basis_the_advice_is_keyed_on`：取 2026 的 12 个节气日
  `D`（`2026-01-05 02-04 03-05 04-05 05-05 06-05 07-07 08-07 09-07 10-08 11-07
  12-07`），对每个断言 `date -a -d D` 的 `干支` 行与 `date -d D -f '%H'` 给出
  **同一个**月柱。`%H` 与 `干支` 行走同一个 `calendar::month_gan_zhi`，两处一致
  才能保证将来不会只改一处。`2026-02-04` 另有一条 pinned 断言：月柱 `庚寅`
  （该日 suite 尚无 pinned 干支行）。

## 非目标

- 不改年柱（春节基准，已正确）。
- 不引入 `month_in_gan_zhi_by_li_chun` 之类第三种基准。
- 不改 `cal`（网格不印干支）。
