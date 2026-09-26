# 计划 04 — 紧凑 `YYYYMMDD` 分支吞噬长数字串

Status: 待实施 · Priority: P1 · Depends: 01（越界要报错而不是回绕）

## 现象

```console
$ lunar date -d "2147483647 days"
lunar: 月份 48 非法 (应为 1–12)        # 前 8 位被读成 2147-48-36

$ lunar date -d "1758240000"
lunar: 月份 24 非法 (应为 1–12)        # 漏写 @ 的时间戳被读成 1758-24-00
```

对照 `date(1)`：两者都是 `invalid date`。我们的错误信息在说一个用户从未写过的
月份，属于答非所问；`2147483647 days` 本是一个合法的相对表达式，只是结果越界。

## 根因

`src/datestr.rs` 的 `parse_absolute` 第一个分支：

```rust
if let (Some(year), Some(month), Some(day)) =
    (digits(text, 0, 4), digits(text, 4, 6), digits(text, 6, 8))
```

只看**前 8 位是不是数字**，不看其后是什么，于是任何以 8 位以上数字开头的输入都
被它认领为紧凑日期。`digits` 本身做了越界与多字节防护，问题只在这个分支的贪婪。

## 目标行为

紧凑形式只在**词边界成立**时认领：

- 恰好 8 位数字（`20260907`），或
- 8 位数字后紧跟时间/时区后缀：`T`/`t`/空格（`20260907T1530`、`20260907 15:30`）。

其余输入按既有优先级继续走（slash → 短年 → ISO → 裸年 → 时刻 → 星期 → 相对）：

| 输入 | 目标输出 |
|---|---|
| `1758240000` | `无法解析的日期: 1758240000`（与 date(1) 同为拒绝） |
| `2147483647 days` | `年份 5881637 超出支持范围 (1–9999)`（合法相对表达式，结果越界） |
| `20260907` | 2026-09-07，行为不变 |
| `20260907T1530` | 2026-09-07，行为不变 |
| `2026-09-中` | `无法解析的日期`（既有测试 `a_multi_byte_character_in_a_date_field_is_reported_not_fatal` 必须继续通过） |

## 实施步骤

1. `src/datestr.rs`：在构造 `Parts` 之前加词边界判断：

   ```rust
   let compact_ok = matches!(text.get(8..), Some(rest)
       if rest.is_empty() || rest.starts_with(['T', 't', ' ']));
   ```

   `text.get(8..)` 在 8 不是字符边界时返回 `None`，多字节输入自然落空、交给后面的
   分支去诊断。仅当 `compact_ok` 时才认领紧凑形式；`rest` 的后续处理沿用现有
   `finish(parts, rest, …)`。
2. 确认落空路径的两种结局（无需新代码，靠计划 01 修好的 `add_years`/i64 epoch
   运算与既有 `validate`）：
   - 10 位数字 → `parse_relative` 里"有计数无单位"→ `invalid(input)`；
   - 大数 + 单位 → i64 运算 → `validate` 报 `YearOutOfRange`。
3. 测试（`tests/documented_examples.rs`）：新增
   `a_long_digit_run_is_not_a_compact_date`，覆盖上表五行；其中越界断言用
   `run_failing` 检查 stderr 子串，`20260907` / `20260907T1530` 用 `run` 比对首行。
4. 文档同步：
   - `README.md`「Known divergences」删掉"长数字串被紧凑形式认领"一句；
   - `docs/parity.md` 中 `1758240000`、`2147483647 days` 两行改为 ✅（同为拒绝，
     措辞不同）；
   - `AGENTS.md`「Known Defects」删掉该条。

## 验收

```bash
cargo test && cargo fmt --check && cargo clippy --all-targets
lunar date -d "1758240000"        # 无法解析的日期，退出 1
lunar date -d "2147483647 days"   # 年份 … 超出支持范围，退出 1
lunar date -d "20260907"          # 2026年9月7日
lunar date -d "20260907T1530"     # 2026年9月7日
```

## 非目标

- 不接受不带 `@` 的裸时间戳（date(1) 也不接受，`@` 是唯一入口）；
- 不引入 14 位 `YYYYMMDDhhmmss` 形式（本工具不保留时刻）；
- 不改 `digits` 的字节级防护。
