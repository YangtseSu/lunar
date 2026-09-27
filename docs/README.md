# 实施计划总览

本目录是 `lunar` 的缺陷修复队列。每个文件是一个**独立可执行**的计划：一名实施者
AI 读完该文件即可开工，不需要本目录之外的上下文。

## 项目目标（唯一确定，不可协商）

`lunar` 是 [`lunar-rs`](https://crates.io/crates/lunar-rs) 的包装器，通过两个子命令
完成农历的**展示、查询和公历↔农历转换**：

- `lunar date` —— 某一天的农历档案与查询，输入行为向 Linux `date(1)` 看齐；
- `lunar cal` —— 月/年网格，行为向 Linux `cal(1)`（util-linux）看齐。

两个子命令**与系统 cal/date 的差异一律视为缺陷**，要么修，要么在 `docs/parity.md`
里登记为"有意保留"并给出理由。本仓库不实现任何日历数学：所有换算、节气、干支、
节日、法定节假日都来自 `lunar-rs`。

## 执行规则（每个计划都适用）

1. **一个计划一个 commit。** 不得把两个计划合进一个 commit，也不得留下半成品。
2. **先复现，后修改。** 每个计划都给了复现命令；先跑一遍确认现状，再动手。
3. **测试同 commit。** 改动落在 `tests/documented_examples.rs`（唯一测试文件，
   启动真实二进制、精确比对）。测试期望值必须**从真实输出复制**，不得手算填充。
4. **文档同 commit。** 行为变化必须同步更新 README 的对应 console 块与选项表；
   修复的缺陷要从 AGENTS.md 的 Known Defects 列表**删除**（不要划删除线保留）。
   README 样例改动后用下面的脚本重新校验。
5. **验收门（缺一不可）：**

   ```bash
   cargo test
   cargo fmt --check
   cargo clippy --all-targets
   ```

6. **commit message**：祈使句主语，<72 字符，说行为不说文件；正文解释错在哪、
   为何重要、改了什么。风格参考 `git log`。
7. **不得 `git push`**，除非用户明确指示。

### README 样例校验脚本

README 中每个 console 代码块里的 `$ lunar …` 行都必须与真实输出逐字节一致。
在仓库根目录用以下脚本校验（实现者可按需另存为本地脚本，不必提交）：

```python
import re, subprocess, shlex
B = "./target/debug/lunar"
FENCE = "`" * 3
blocks = re.findall(f"{FENCE}console\n(.*?){FENCE}", open("README.md").read(), re.S)
fails = 0
for bi, block in enumerate(blocks):
    lines = block.rstrip("\n").split("\n")
    i = 0
    while i < len(lines):
        if lines[i].startswith("$ "):
            cmd = re.split(r"\s{4,}#", lines[i][2:])[0]
            args = shlex.split(cmd)[1:]
            out = []
            j = i + 1
            while j < len(lines) and not lines[j].startswith("$ "):
                out.append(lines[j]); j += 1
            p = subprocess.run([B] + args, capture_output=True, text=True)
            got = (p.stdout + p.stderr).rstrip("\n").split("\n")
            want = list(out)
            while want and want[-1].strip() == "":
                want.pop()
            if got != want:
                fails += 1
                print(f"BLOCK {bi} MISMATCH: {cmd}")
                print("  WANT:", want)
                print("  GOT :", got)
            i = j
        else:
            i += 1
print("blocks:", len(blocks), "fails:", fails)
```

## 已完成的基线（本目录建立时的状态）

- `README.md` 与 `AGENTS.md` 已按上面的目标重写：README 的 console 样例全部取自
  真实运行（11 个块，校验 `fails: 0`），AGENTS.md 的 Known Defects 只列当前缺陷，
  旧的"已修复条目划删除线存档"已删除。
- `docs/parity.md` 建立了 `cal(1)` / `date(1)` 兼容矩阵；未登记的新差异先补矩阵，
  再决定立案。
- 计划 01–07 是当时**未修复**的全部已知缺陷；执行者不要再去旧文档里找已消失的
  历史描述。

## 计划顺序

按修复价值与依赖排序，**严格按序执行**；后面的计划建立在前面稳定下来的基线上。

| # | 计划 | 类型 | 优先级 |
|---|---|---|---|
| 01 | [add_years 溢出](plans/01-year-overflow.md) ✅ | 崩溃/静默错误 | P0 |
| 02 | [--color/--no-color 语义](plans/02-color-precedence.md) ✅ | CLI 契约 | P1 |
| 03 | [cal 跨度语义对齐 cal(1)](plans/03-span-semantics.md) ✅ | 行为对齐 | P1 |
| 04 | [YYYYMMDD 分支吞噬长数字](plans/04-greedy-digits.md) ✅ | 解析正确性 | P1 |
| 05 | [农历 9999 年末月不可渲染](plans/05-lunar-top-year.md) ✅ | 边界 | P2 |
| 06 | [date(1) 能力缺口](plans/06-date-parity-forms.md) ✅ | 能力补齐 | P2 |
| 07 | [过时文档注释清理](plans/07-doc-comment-rot.md) ✅ | 文档 | P2 |
| 08 | [干支月与宜忌用了不同月柱基准](plans/08-month-pillar-basis.md) | 自相矛盾 | **P1** |

✅ = 已实施。

计划 08 是**待实施**队列里唯一的一条，由 2026-09-28 接入黄历时发现：`date -a`
的 `干支` 行印的月柱，与同一份输出里 `宜` / `忌` 所依据的月柱，是两个不同基准
（1–9999 共 119,988 天）。归属**本仓库**——引擎两个基准都提供且都正确，错的是
`calendar::month_gan_zhi` 选了 `_exact()`。

`docs/parity.md` 是判定"差异是否为缺陷"的依据；新增差异先登记到那里，再决定是否
立案。

## 完成定义（DoD）

一个计划只有同时满足以下各条才算完成：

- 计划中的**复现命令**输出与"目标行为"一节描述一致（含退出码与 stderr）；
- `tests/documented_examples.rs` 有测试覆盖该行为（既有测试断言需随语义更新）；
- README / AGENTS.md 相关段落已同步，README console 块校验 `fails: 0`；
- 三个验收门全绿；
- AGENTS.md 的 Known Defects 中该条目已删除。
