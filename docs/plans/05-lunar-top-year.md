# 计划 05 — 农历 9999 年末月跨出范围：报错要说出原因

Status: 待实施 · Priority: P2 · Depends: 01（错误类型改动）、03（农历月序列实现）

## 现象

```console
$ lunar cal -L 9999
lunar: 年份 10000 超出支持范围 (1–9999)      # 为什么？没说
$ lunar cal -L 9999 12
lunar: 年份 10000 超出支持范围 (1–9999)
$ lunar cal -L 9999 11                        # 只有冬月可看
农历 己亥年 冬月
```

原因：农历 9999 年腊月始于公历 9999-12-30，末日落在 10000-01-27。网格要把该月
**所有**天画出来，第 30 天起就出了 1–9999 的范围，于是 `Grid::lunar` 在
`to_solar()` 处拿到一个 `YearOutOfRange { year: 10000 }`——消息正确但没说是哪个月
跨的界，读起来像用户把年份写错了。

## 目标行为

范围保持 1–9999（引擎的 `LEAP_11`/`LEAP_12` 约束，不可扩），但错误必须指名道姓：

```console
$ lunar cal -L 9999
lunar: 农历 9999 年腊月跨入 10000 年，超出支持范围 (1–9999)
$ echo $?
1
```

`cal -L 9999 11` 继续正常输出；`cal -L 9999 12` 与 `cal -L 9999` 报同一条消息；
`cal -L 9999 12 -n 3`（计划 03 的跨年窗口）报 `年份 10000 超出支持范围`（窗口本身
越界，与月份自身越界是两种情形，消息各自准确）。

## 实施步骤

1. `src/calendar.rs`：新增错误变体

   ```rust
   /// A lunar month whose days cross the supported civil range.
   LunarMonthBeyondRange { lunar_year: i32, month: i32, civil_year: i32 },
   ```

   `message()` 用 `m_abs(*month)` 渲染月名：
   `农历 {lunar_year} 年{月名}跨入 {civil_year} 年，超出支持范围 ({MIN_YEAR}–{MAX_YEAR})`。
2. `src/calendar.rs`：`lunar_month_days` 改为
   `Result<Vec<CivilDate>, CalError>`。转换每一天时若 `solar.year()` 超出
   `MIN_YEAR..=MAX_YEAR`，返回上述错误，`lunar_year` 取该月首日的农历年
   （`month.get_first_day()`），`month` 取 `month.month()`。
3. `src/calgrid.rs`：`Grid::lunar` 里 `calendar::lunar_month_days(&month)?`（`?` 透传）。
   确认 `calgrid` 是唯一调用点（`grep -rn lunar_month_days src/`）。
4. 测试（`tests/documented_examples.rs`）：新增
   `the_last_lunar_year_reports_the_boundary`：
   - `run_failing(&["cal","-L","9999"])` 与 `run_failing(&["cal","-L","9999","12"])`
     断言 stderr 恰为 `lunar: 农历 9999 年腊月跨入 10000 年，超出支持范围 (1–9999)`；
   - `run(&["cal","-L","9999","11"])` 断言首行 `农历 己亥年 冬月`。
5. 文档同步：
   - `README.md`「Supported range」补一句：农历 9999 年腊月跨入 10000 年，
     `cal -L 9999` 因此报错，`cal -L 9999 11` 可用；
   - `AGENTS.md`「Known Defects」删掉该条。

## 验收

```bash
cargo test && cargo fmt --check && cargo clippy --all-targets
lunar cal -L 9999        # 新消息，退出 1
lunar cal -L 9999 11     # 正常
lunar cal -L 1           # 仍正常（12 个月）
```

## 非目标

- 不把支持范围扩到 10000（引擎约束）；
- 不裁剪该月的单元格（网格必须画出该月的全部天，裁剪会让网格与 `lunar date`
  对同一天给出不同答案）；
- 不改 `MIN_YEAR`/`MAX_YEAR` 的取值与公历侧的范围检查。
