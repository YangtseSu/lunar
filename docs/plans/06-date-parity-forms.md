# 计划 06 — 补齐 `-d` 已宣称/应对齐 date(1) 的语法形态

Status: 待实施 · Priority: P2 · Depends: 04（先修好紧凑分支的边界）

四个子项同属 `src/datestr.rs` 的同一份语法表，作为**一个** commit 完成。

## 现象

| 输入 | date(1) | 现状 |
|---|---|---|
| `2026-09-07T15:30:45.123456789+08:00` | ✅ | `无法解析的日期`（`datestr` 模块文档第 7–8 行明确宣称支持） |
| `2026-9-7` / `2026/9/7` | ✅ | `无法解析的日期` |
| `15:30 UTC` / `15:30+08:00` | ✅ | `无法解析的日期` |
| `15:30`（裸时刻）配 `-l` | — | **静默返回今天**，违反"时刻属公历语法、`-l` 下拒绝"的既定规则 |
| `星期六` / `周二` / `礼拜六` | 不支持 | 不支持；但表里已有 `星期日`/`星期天`/`礼拜天`/`周一`，是残表 |

## 目标行为

1. **小数秒**：秒字段允许 `.` 后跟数字，小数部分**截断**（本工具不保留时刻），
   `2026-09-07T15:30:45.5+08:00` 与 `…:45+08:00` 同解。
2. **不补零的 ISO**：`2026-9-7`、`2026/9/7` 与 `2026-09-07` 同解；`09/07/2026`
   的 US 顺序优先级不变。
3. **裸时刻带时区**：`15:30 UTC` 解析为**今天在该时区的日期**（跨零点时日期改变），
   与 `date -d "15:30 UTC"` 一致；`-l` 下任何时刻形态（含裸时刻）一律拒绝。
4. **中文星期名补全**：`星期X` / `周X` / `礼拜X`，X ∈ 一…日，另接受 `星期天` /
   `周天` / `礼拜天`；`next 星期六` / `last 周天` 组合随之可用。
   这是**本工具的扩展**（date(1) 不支持中文星期名），在 README 与 parity 登记。

## 实施步骤

1. **小数秒**（`parse_clock`）：秒字段先取整数部分再解析——
   `let second: i32 = value.split('.').next().unwrap_or(value).parse().ok()?;`
   紧凑 `hhmm` 分支不受影响；`finish_clock` 的范围检查不变。
2. **不补零 ISO**（`parse_absolute`）：在 US slash 分支之后、短年分支之前加一个
   分支，仅当**月或日为 1 位**时认领（补零输入继续走既有定长分支，避免回归）：

   ```rust
   // 形如 2026-9-7 / 2026/9/7：首段恰好 4 位数字，共 3 段，全为数字。
   ```

   认领后用 `finish(parts, text.get(consumed..).unwrap_or_default(), calendar, input)`
   处理时间后缀，与定长分支同路。
3. **裸时刻带时区**（`parse_absolute` 末尾的裸时刻分支）：改为走 `finish`：

   ```rust
   if parse_clock(split_zone(text).0).is_some() {
       let parts = Parts { year: reference.year, month: reference.month, day: reference.day };
       return finish(parts, text, calendar, input).map(Some);
   }
   ```

   `finish` 已实现"`-l` 拒绝时刻""有时区走 `shift_by_zone`、无时区取原日期"两条
   规则，复用它即同时修好第 3、4 两条现象。
4. **中文星期表**（`WEEKDAYS`）：补齐三套前缀 × 一…日，另加 `天` 变体；`lower` 是
   ASCII 小写化，不影响中文匹配。
5. **测试**（`tests/documented_examples.rs`）：
   - `a_fractional_second_is_truncated`：`…T15:30:45.5+08:00` 与 `…T15:30:45+08:00`
     输出一致；
   - `an_unpadded_iso_date_is_accepted`：`2026-9-7`、`2026/9/7` 与 `2026-09-07`
     首行一致；`09/07/2026` 仍按 US 顺序；
   - `a_bare_time_resolves_in_its_zone`：`15:30 UTC` 得到今天（用 `date -d today`
     对比同一天）；`run_failing(&["date","-l","-d","15:30"])` 断言 `无法解析`；
   - `chinese_weekday_names_are_accepted`：`星期六`、`周二`、`礼拜六`、`星期天`
     与对应的英文星期名解析到同一天。
6. **文档同步**：
   - `README.md` 的 `-d` 形态列表补三条（不补零 ISO、小数秒截断、裸时刻带时区），
     并注明中文星期名是扩展；
   - `README.md`「Known divergences」删掉对应条目；
   - `docs/parity.md`：前四行改为 ✅，中文星期名改为 🔒（扩展，理由写明）；
   - `AGENTS.md`「Known Defects」删掉该条；「Command Surface Contracts」补一句：
     裸时刻属公历语法，`-l` 下拒绝。

## 验收

```bash
cargo test && cargo fmt --check && cargo clippy --all-targets
lunar date -d "2026-09-07T15:30:45.123456789+08:00"   # 2026年9月7日
lunar date -d "2026-9-7"                               # 2026年9月7日
lunar date -d "15:30 UTC"                              # 今天
lunar date -l -d "15:30"                               # 无法解析，退出 1
lunar date -d "星期六"                                  # 最近的星期六
```

## 非目标

- `TZ="UTC" 2026-09-07` 前缀式时区（登记为有意保留）；
- `next friday -3 days` 之类的组合相对表达式（同上）；
- `YYYYMMDDhhmmss` 14 位形式（本工具不保留时刻）。
