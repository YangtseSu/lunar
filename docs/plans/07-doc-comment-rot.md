# 计划 07 — 过时文档注释清理：删掉语言层残骸与历史叙事

Status: 已实施 · Priority: P2 · Depends: 03、06（它们重写的注释不再属于本计划）

本计划**只改注释**，不改任何行为、不改用户可见字符串。若发现某条注释描述的行为
其实是错的，不要在本计划里改行为——另立计划。

## 现象（`grep -n` 实测，HEAD 行号）

**A. 已删除的语言层残骸**（本工具只讲简体中文，没有语言选择）：

- `src/format.rs:10,11,17` — 令牌表写着 `(一..日 / Mon..Sun)`、`(丙午 / Bing Wu)`、
  `(马 / Horse)`；
- `src/format.rs:23-24` — "Every name the engine owns is rendered in the chosen
  language."；
- `src/commands/cal.rs:282-283` — "the year pillar and the month name are rendered
  in the chosen language."

**B. 已不存在的 `班` 标签**（profile 打印的是 `法定: … 放假` / `… 调休上班`，
网格从不写字符）：

- `src/commands/date.rs:14` — "`放假` / `班` line appears only when…"；
- `src/commands/date.rs:133-135` — "放假 and 班 change what the day *is*…"；
- `src/mark.rs:19` — "Writing 放假 or 班 into the cell would buy nothing…"；
- `src/mark.rs:36-37` — "the day is only as rare as the `班` that used to name it".

**C. 历史叙事**（注释应陈述现状，历史属于 commit message）：

- `src/datestr.rs:148-155`（epoch 截断，`used to div_euclid`）
- `src/datestr.rs:530-537`（sub-day 拒绝，`They used to return the date unchanged`）
- `src/format.rs:40-45`（`\%` 逃逸，`used to fall through`）
- `src/main.rs:146-150`（`--help-format` 提前应答，`It used to be printed from inside`）
- `src/commands/cal.rs:251-264`、`293-299`（跨度与 `-n` 边界；若计划 03 已重写则跳过）

**D. 测试文件里的历史叙事**（`tests/documented_examples.rs`，8 处）：

- `:930`（`split_zone` 的字节切片）、`:1138`（公历月长助手）、`:1346`（`-R`
  曾被忽略）、`:1382`（`i32::MIN` 曾 panic）、`:1530`（`add_days` 步进）、
  `:1856`、`:1868`（跨度曾被裁剪/曾被 `-y` 误读）等。

改写成"这条断言为何是这样"的现在时理由（例如：本地 `add_days` 步进会漂移，所以
扫描向二进制询问月份天数），**断言与期望值一律不动**。

## 目标行为

1. 每条注释只描述**当前**契约与理由，删除"过去如何错"的叙述；
2. 语言层残骸全部消失，令牌表只列中文形态；
3. `班` 只出现在用户可见的 `调休上班` 字符串里；
4. 行为零变化，`cargo test` 的 58 个测试**不加不改**全部通过。

## 实施步骤

1. 按 A/B/C 清单逐条改写。示例：

   - `format.rs` 令牌表改为 `| %A | 星期几单字 (一~日) |`、`| %G | 农历年干支 (丙午) |`、
     `| %S | 生肖 (马) |`；文档结尾改为"所有名字都用引擎提供的中文形态"。
   - `date.rs` 模块文档改为：profile 打印 `公历/农历/干支/生肖`，有节气时加 `节气`，
     在法定日历上有记录时加 `法定: 名称 放假|调休上班`。
   - `mark.rs:36-37` 改为陈述规则：调休日以 `1;93` 高亮，与其它高亮单元格同级。
   - `datestr.rs` epoch 段改为：小数丢弃、整秒**向零截断**（与 `date -d @…` 一致），
     并保留"负时间戳因此不会早一天"这一**当前**性质的必要说明。
   - `main.rs` 改为陈述规则：`--help-format` 不依赖日期与时区，因此在解析参考日
     **之前**应答——TZ 不可读的机器也能读到令牌表。
2. 全仓扫描确认没有漏网（这三条是验收门）：

   ```bash
   grep -rn 'chosen language\|Mon\.\.\|Bing Wu\|Horse' src/            # 空
   grep -rn '班' src/ | grep -v '调休上班'                              # 空
   grep -rn 'used to\|The old code\|earlier version\|previously\|no longer' src/   # 空
   ```
3. 改写 D 组的 8 处测试注释，只动注释文字；断言、期望字符串、测试名都不变。
4. 不改 `src/` 的任何行为与用户可见字符串；跑三个验收门确认零行为变化。

## 验收

```bash
cargo test && cargo fmt --check && cargo clippy --all-targets
# 上面三条 grep 全部为空
# README 样例校验脚本 fails: 0（README 未被本计划改动）
```

## 非目标

- 不借机改行为、改错误消息、改 README 文案；
- 不删除解释**当前**设计取舍的注释（如 `mark.rs` 的"属性不是标签"、
  `calendar.rs` 的三节日来源、`civil.rs` 的 1582 改革桥接）——它们是防回归的
  契约说明，只删其中的历史句；
- 不新增文档文件。
