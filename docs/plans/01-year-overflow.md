# 计划 01 — `add_years` 溢出：用户输入导致 panic / 回绕

Status: 已实施 · Priority: P0（唯一能让进程崩溃的输入） · Depends: 无

## 现象

```console
$ cargo run -- date -d "+2147483600 years"          # debug 构建
thread 'main' panicked at src/civil.rs:153:20:
attempt to add with overflow
（退出码 101）

$ cargo run --release -- date -d "+2147483600 years"
lunar: 年份 -2147481670 超出支持范围 (1–9999)       # 回绕出的假年份
```

对照 `date(1)`：同一输入给出 `invalid date`，退出码 1，不崩溃。本工具的行为在
debug 下是 panic、在 release 下是**报出一个不存在的年份**，两者都不可接受。

## 根因

`src/civil.rs` 的 `CivilDate::add_years`：

```rust
pub fn add_years(self, delta: i32) -> Self {
    let year = self.year + delta;      // i32 + i32，无检查
    ...
}
```

`delta` 来自用户输入：`datestr::parse_relative` 解析计数，`apply_unit` 的
`"year" | "years"` 分支调用它。同文件的 `add_months` 用 i64 计算、`add_days`
走 i64 epoch 运算，所以**只有 `add_years` 这一处**会溢出。

## 目标行为

任何用户输入的相对年数，只要结果超出 1–9999，都必须以范围错误退出，且错误里
的年份是**真实算出来的那个年份**：

```console
$ lunar date -d "+2147483600 years"
lunar: 年份 2147485626 超出支持范围 (1–9999)
$ echo $?
1
```

debug 与 release 行为一致；不 panic、不回绕、不静默。

## 实施步骤

1. `src/calendar.rs`：把 `CalError::YearOutOfRange { year }` 的字段类型由 `i32`
   放宽为 `i64`（`min`/`max` 不变），`message()` 的格式串不变，因此
   `tests/documented_examples.rs` 的既有断言不受影响。`check_year` 构造处加
   `i64::from(year)`。
2. `src/civil.rs`：`add_years` 改为返回 `Result<Self, CalError>`，内部用 i64 相加，
   `i32::try_from` 失败时返回 `CalError::YearOutOfRange { year: 相加后的 i64 值, min: MIN_YEAR, max: MAX_YEAR }`。
   注意 `min`/`max` 由 `calendar` 常量提供，避免 `civil` 里再写一遍字面量。
3. `src/datestr.rs`：`apply_unit` 的 `"year" | "years"` 分支改为
   `date.add_years(count)`（`?` 透传）；确认没有其它调用点（`grep -rn 'add_years' src/`）。
4. 测试（`tests/documented_examples.rs`）：新增
   `a_huge_relative_offset_is_reported_not_wrapped`：
   - `run_failing(&["date", "-d", "+2147483600 years"])` 断言 stderr 含
     `超出支持范围`；
   - 同断言 `+2147483647 years`、`-2147483647 years`（负向回绕）；
   - 断言 `run(&["date", "-d", "2 years"])` 仍等于 2028-09-26（正常路径未破）。
   该测试在修复前于 debug 下必然失败（panic → 退出码 101），是有效回归测试。
5. 文档同步：
   - `README.md`「Known divergences」删掉 overflow 那一条的前半句；
   - `AGENTS.md`「Known Defects」删掉对应条目（不要划删除线）。

## 验收

```bash
cargo test && cargo fmt --check && cargo clippy --all-targets
cargo run -- date -d "+2147483600 years"        # 退出 1，无 panic
cargo run --release -- date -d "+2147483600 years"   # 同一消息
```

## 非目标

- 不限制相对计数的取值范围（i32 是语法本身的宽度）；
- 不改 `add_days` / `add_months`（已是 i64，无溢出）；
- 不把 `+N years` 的越界变成"解析失败"——它是合法的相对表达式，只是结果越界。

## 实施后的残留（同族，已由 `docs/plans/04` 关闭）

本计划只关掉 `add_years` 一处。实施时用同一族输入复测，发现另外两处同形回绕，
均在 `-d` 相对链上**可由用户输入到达**，且报出的年份同样是无人问过的值：

- `add_months` 用 i64 运算后 `as i32` 强转：12 段
  `+2147483647 months` 相加后超出 i32 → `年份 2147485673 超出支持范围`；
- `add_days` / `civil_from_days` 的 i64 epoch 运算末端 `as i32`：27 段
  `+2147483647 fortnights` → `年份 2222494805 超出支持范围`。

单段输入触发不了（12·2³¹ 或 14·2³¹ 天 ≈ ±1.07 亿年 < i32 上限约 2.9 万年的
百万倍），所以上面给出的段数是构造下界而非精确阈值——精确阈值是**首个越界步**
（解析器逐个单位施加，故 12 / 27 是首个失败步，而非越界步数）。

两处已随 `docs/plans/04` 一并修复：`civil_from_days` 与 `add_months` 改用
`i32::try_from` 并报出真算出的年份，`add_days` / `from_epoch_day` 随之返回
`Result`。回归测试 `a_wrapped_month_or_day_count_names_the_year_it_reached`
在 `src/` 修复前失败、修复后通过。
