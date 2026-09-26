# 计划 03 — `cal` 跨度语义对齐 cal(1)：`-3` 居中、`-n N` 起于、`-y` 整年

Status: 待实施 · Priority: P1 · Depends: 01（先让越界可预测）、02（同文件旗标处理）

## 现象（本机 util-linux 2.42.3 `cal` 为基准，2026-09-26）

| 命令 | `cal(1)` | `lunar cal` 现状 | 差异 |
|---|---|---|---|
| `-3`，`cal -3 9 2026` | 八月 九月 十月（**居中**） | `cal 2026 9 -3` → 九月 十月 十一月（**向后**） | 语义错 |
| `-n 3`，`cal -n 3 9 2026` | 九月 十月 十一月（**起于**指定月） | 公历 ✅；`cal -L 2026 7 -n 3` → 六月 七月 八月（**居中**） | 农历视图错 |
| `-y`，`cal -y 9 2026` | 2026 全年（1–12 月） | `cal 2026 9 -y` → 2026年9月…2027年8月 | 语义错 |
| `-y` 无位置参数，`cal -y` | 当前年全年 | `cal -y` → 2026年9月…2027年8月 | 语义错 |
| 缺省月份 | 用**今天的月**补齐（`cal -3 2025` → 八月 九月 十月 2025） | `cal 2026 -3` → 1月 2月 3月（按 1 月补齐） | 锚点错 |
| `-3` 跨年 | `cal -3 12 2026` → 十一月 十二月 一月 2027 | `cal 2026 12 -3` → 十二月 2027年1月 2月（向后） | 语义错 |

农历视图另有自设的"不跨农历年、在年末回退保持长度"规则，与 cal(1) 的连续月序列
不一致；`src/commands/cal.rs` 的 `select_lunar_months` 文档注释还宣称与公历视图
"same reading"，实际两边都不是这个读法。

## 目标行为（cal(1) 语义，两个视图统一）

1. **`-3` = 指定月前后三个月**（前一月、本月、后一月），跨年自由。
2. **`-n N` = 从指定月起 N 个月**，跨年自由。
3. **`-y` = 指定日所在的整年**：公历为 1–12 月；农历为该农历年的全部月份（含闰月）。
4. **锚点规则**：给了月用给的月；只给年或缺省时，月用**今天的月**（农历视图用
   今天的农历月序号）。`cal 2026`（只给年、无跨度旗标）= 整年，与 cal(1) 相同；
   带 `-3`/`-n`/`-y` 时按上面的旗标语义执行（`cal -3 2026` = 今天所在月居中）。
5. **农历视图跨农历年**：`-3`/`-n` 的窗口沿农历月序列连续推进，需要时加载相邻
   农历年（`腊月` 之后是下一年的 `正月`）。每个网格标题带自己的干支年，所以跨年
   窗口天然可读。范围边界（农历 1 年之前、9999 年之后）报范围错误，不静默截断。

验收示例（今天是 2026-09-26，农历丙午年八月）：

```console
$ lunar cal 2026 7 -3        → 2026年6月 7月 8月
$ lunar cal 2026 12 -3       → 2026年11月 12月 2027年1月
$ lunar cal 2026 7 -n 3      → 2026年7月 8月 9月
$ lunar cal 2026 7 -y        → 2026年1月 … 2026年12月
$ lunar cal -y               → 2026年1月 … 2026年12月
$ lunar cal -3               → 2026年8月 9月 10月
$ lunar cal -L 2026 12 -n 3  → 丙午年腊月 丁未年正月 丁未年二月
$ lunar cal -L 2026 1 -3     → 乙巳年腊月 丙午年正月 丙午年二月
$ lunar cal -L 2026 12 -3    → 丙午年冬月 腊月 丁未年正月
```

## 实施步骤

1. **抽出跨度规划器**（`src/commands/cal.rs`，单一实现，两个视图共用）：

   ```rust
   enum Span { WholeYear, Centred(i32), Forward(i32), Single }
   fn span_of(args: &CalArgs) -> Span;   // -y → WholeYear; -3 → Centred(3); -n N → Forward(N); 否则 Single
   ```

   锚点解析统一为 `(year, month)`：位置参数 0 个 → 今天；1 个 → 该年 + 今天的月，
   且**无跨度旗标时为 WholeYear**；2 个 → 该年该月。
2. **公历视图**：由锚点月 `CivilDate::add_months` 步进生成月份列表——
   `Centred(3)` 取偏移 `-1..=1`，`Forward(N)` 取 `0..N-1`，`WholeYear` 取该年
   1–12 月，`Single` 取锚点月。每月仍用现有的 `Grid::civil` 渲染；越过
   `MIN_YEAR`/`MAX_YEAR` 时返回 `CalError::YearOutOfRange`（现有行为）。
3. **农历视图**：把 `select_lunar_months` 改为沿**连续农历月序列**取窗口：
   - 建立锚点所在农历年的 `months_in_year()`（含闰月，顺序即真实序列），需要时
     用 `calendar::lunar_year(y-1)` / `(y+1)` 取相邻年，拼出足够长的序列；
   - 以锚点月在该序列中的位置为中心/起点切片；
   - `WholeYear` 只取锚点农历年自己的月份（含闰月）；
   - 超出支持范围（农历年 < 1 或 > 9999）时报 `YearOutOfRange`，不截断。
   `lunar_month_title` 已按每月自己的干支年渲染，无需改动。
4. **删除**"年末回退保持长度"的实现与其文档注释（`span_from` 及其说明），改为
   连续序列切片；`-R` 与 `check_lunar_month` 的校验保持不变。
5. **测试**（`tests/documented_examples.rs`）：
   - 重写 `a_lunar_month_span_centres_on_the_named_month`（居中仅适用于 `-3`；
     `-n` 起于）；
   - 新增 `the_span_flags_match_cal_one`：上表公历各例逐条断言标题序列；
   - 新增 `the_lunar_span_crosses_the_lunar_new_year`：`-L 2026 12 -n 3`、
     `-L 2026 1 -3` 的标题序列（含干支年变化）；
   - 新增 `the_missing_month_comes_from_today`：`cal -3` / `cal 2026 -3` 的窗口
     为 3 个连续月且包含今天所在月（用二进制先问今天是哪个月，模式同既有
     reference-day 测试）；
   - 新增 `a_span_off_the_range_end_is_reported`：`cal -L 9999 12 -n 3` 报
     `年份 10000 超出支持范围`。
6. **文档同步**：
   - `README.md` 选项表 `-3` / `-n N` / `-y` 三行与「Options」后的跨度段落；
   - `docs/parity.md`：`-3`、`-n N`、`-y` 三行改为 ✅，删除对应的 🔒/🚧 说明；
   - `AGENTS.md`「Known Defects」删掉该条，并在「Command Surface Contracts」加一句
     跨度语义（`-3` 居中、`-n` 起于、`-y` 整年、缺省月取今天）。

## 验收

```bash
cargo test && cargo fmt --check && cargo clippy --all-targets
# 上表每条示例逐条手工确认；README 样例校验脚本 fails: 0
```

## 非目标

- 不实现 `cal(1)` 的 `[日] 月 年` 位置顺序与月名参数（已登记为有意保留，见
  `docs/parity.md`）；
- 不实现 `-S/--span`、`-w`、`-j`、`-v` 等展示旗标；
- 不改公历视图的纵向堆叠排版。
