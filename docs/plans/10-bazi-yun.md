# 计划 10 — 新增大运：`lunar bazi -g` 的起运与大运

Status: 已实施（2026-09-28） · Priority: P2 · Depends: 09

## 现象

计划 09 交付四柱后，`EightChar::yun` / `Yun::da_yun` 仍不可达：排八字的人拿到
年、月、日、时四柱，却拿不到大运——而大运是八字里被引用最多的部分。

```console
$ lunar bazi 1990-06-15T10:30 -g 男
error: unexpected argument '-g' found
```

引擎侧实测（`Yun` 已完整，`src/yun/mod.rs` + `src/yun/da_yun.rs`）：

```
1990-06-15 10:30  男 顺行  8-17 癸未 / 18-27 甲申 / 28-37 乙酉 / … / 88-97 辛卯
2025-01-29 00:30  女 逆行  8-17 丙子 / 18-27 乙亥 / 28-37 甲戌 / … / 88-97 戊辰
```

## 归属

**本仓库未暴露的能力**（引擎 `EightChar::yun_by_sect` 已实现）。大运不需要时刻，
但需要**性别**：顺逆由「年干阴阳 × 性别」决定，没有可假设的默认值。

## 实施中发现：`sect` 是两个不同性质的开关

`EightChar` 与 `Yun` 共用一个 `sect` 参数，但它在两处含义不同，查证如下。

**在 `EightChar` 里，`sect` 只影响日柱。** 逐处核对 `impl EightChar` 中全部
`self.sect` 使用点：只有 `day` / `day_gan` / `day_zhi` / `day_pillar` /
`day_xun` / `day_xun_kong` / `di_shi` / `terrain` 八处；年柱、月柱、时柱**无条件**
用 `_exact()`，不读 sect。差异来自 `compute_day`：

```rust
self.day_gan_index_exact2 = day_gan_exact;   // sect 2：先存"当天"的
if self.hour == 23 { day_gan_exact += 1; … } // sect 1：23 点后推到次日
self.day_gan_index_exact = day_gan_exact;
```

实测 1990-06-15：00:30 / 22:30 两 sect 同为 `辛亥`，23:30 则 sect 1 = `壬子`、
sect 2 = `辛亥`。**即 sect 1 = 晚子时换日，sect 2 = 不换日**，影响面是每天
23:00–23:59 一小时。

**在 `Yun` 里，`sect` 是两套起运算法**（`Yun::compute_start`）：

| sect | 算法 |
|---|---|
| 1 | 数**时辰**差：时辰差 ÷ 30 折月 |
| 2 | 数**分钟**差：4320 分钟折 1 年、360 折 1 月 |

实测同日同生两者差 3–8 天：

| 生辰（男） | sect 1 | sect 2 |
|---|---|---|
| 1990-06-15 10:30 | 7年5月10日 | 7年5月2日12时 |
| 1990-06-15 23:30 | 7年3月10日 | 7年2月27日12时 |
| 2000-01-01 00:05 | 8年0月10日 | 8年0月11日12时 |

**且引擎两个默认值不一致**：`eight_char()` 默认 sect 2，`yun()` 默认 sect 1。
因此 `calendar::yun` **必须显式传 sect**，不能用 `yun()` 的默认。

## 目标行为

`lunar bazi <生辰> -g <性别>` 在四柱之后追加两行：

```
起运: 1997年11月17日  (出生后 7年5月)  顺行
大运: 8-17 癸未 / 18-27 甲申 / 28-37 乙酉 / … / 88-97 辛卯
```

1. **不给 `-g`**：印四柱 + `说明: 未给性别，无大运`，**不报错**。四柱本身是完整
   答案，缺性别只是缺大运——与「无时刻缺时柱」同一取向。
2. **sect 2**（日柱不换日、起运数分钟），不暴露旗标。
3. **十步**，即引擎 `da_yun()` 的跨度；不加 `-n`。
4. **起运只印年月日 + 起运月数，不印时刻**——sect 2 的分钟折算会产出「12时」这类
   换算产物，印成时刻很奇怪，而各家算法真正对不齐的是月数。
5. **起运可以小于一年**（如 `0年1月`），照实印，不过滤。

## 实施步骤

1. `src/calendar.rs`：`yun(&EightChar, Gender) -> Yun` 包装 + 私有常量
   `YUN_SECT: u8 = 2`；`CalError::BadGender` 变体与中文消息。
2. `src/commands/bazi.rs`：`BaziArgs.gender`；`gender()` 接受
   `男/male/m/M` 与 `女/female/f/F`；`write_chart` 增 `gender: Option<Gender>`。
3. `src/main.rs`：`Command::Bazi` 增 `-g/--gender`。
4. 测试四个（见验收）。
5. 文档：README、AGENTS.md 契约、`docs/parity.md` 登记、计划表加 09→10。

## 取舍（登记于 `docs/parity.md`）

- **sect 固定 2，不暴露旗标。** 日柱不换日，与 `lunar date` 的干支行同源；起运数
  分钟。两处都是无对错的专业取舍，暴露成旗标等于让用户替我们选，而两个子命令的
  口径必须自洽。
- **不给性别不报错**，印四柱 + 说明行。
- **十步固定**，不加 `-n`。
- **不做流年 / 小运**（`liu_nian` / `xiao_yun` 引擎亦有），留待后续计划。

## 验收

```console
$ lunar bazi 1990-06-15T10:30 -g 男 | tail -2
起运: 1997年11月17日  (出生后 7年5月)  顺行
大运: 8-17 癸未 / 18-27 甲申 / 28-37 乙酉 / 38-47 丙戌 / 48-57 丁亥 / 58-67 戊子 / 68-77 己丑 / 78-87 庚寅 / 88-97 辛卯

$ lunar bazi 1990-06-15T10:30 -g 女 | tail -2
起运: 1993年7月4日  (出生后 3年0月)  逆行
```

测试：

- `bazi_without_a_gender_prints_the_pillars_and_says_why` —— 无 `-g` 时有说明行、
  无大运行，且四柱与给 `-g` 时逐行相同。
- `the_gender_decides_which_way_the_luck_runs` —— 庚午阳年男顺女逆；两种拼写同解；
  非法值报错。
- `a_short_time_to_the_term_still_gives_a_start` —— `0年1月` 照印；步数 9、首步 1-10。
- `the_luck_rows_sit_after_the_pillars` —— 行序；起运月数与首步岁数自洽（7年5月 → 8岁起运）。

## 非目标

- **流年 / 小运**（另立计划）。
- **暴露 `--sect`**。
- **`-n` 步数旗标**。
