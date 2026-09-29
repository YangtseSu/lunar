# 实施计划总览

本目录是 `lunar` 的缺陷修复队列。每个文件是一个**独立可执行**的计划：一名实施者
AI 读完该文件即可开工，不需要本目录之外的上下文。

## 项目目标（唯一确定，不可协商）

`lunar` 是 [`lunar-rs`](https://crates.io/crates/lunar-rs) 的包装器，通过三个子命令
完成农历的**展示、查询和公历↔农历转换**：

- `lunar date` —— 某一天的农历档案与查询，输入行为向 Linux `date(1)` 看齐；
- `lunar cal` —— 月/年网格，行为向 Linux `cal(1)`（util-linux）看齐；
- `lunar bazi` —— 生辰八字（四柱十神），唯一读时刻的子命令，因为时柱需要它。

`date` / `cal` **与系统 cal/date 的差异一律视为缺陷**，要么修，要么在
`docs/parity.md` 里登记为"有意保留"并给出理由。`bazi` 在两个工具里都没有对应物，
它的取舍单独登记在 `parity.md`。本仓库不实现任何日历数学：所有换算、节气、干支、
节日、法定节假日、八字都来自 `lunar-rs`。

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
   reuse lint
   ```

   （`reuse lint` 只在新增或删除文件时需要跑；声明写在 `REUSE.toml`，日常改
   代码不触碰它。理由见该文件注释。）

6. **commit message**：祈使句主语，<72 字符，说行为不说文件；正文解释错在哪、
   为何重要、改了什么。风格参考 `git log`。
7. **不得 `git push`**，除非用户明确指示。

新增能力（引擎已实现、本仓库未暴露的子命令或输出面）**同样走本流程**：立案为
`docs/plans/NN-*.md`，一个 commit 完成，同样受上面那些验收门约束。区别只在「归属」
与「现象」两节怎么写——新增能力没有 `date(1)` / `cal(1)` 作参照，因此：

- **现象**写"能力缺失"而非"行为对不上"：引擎的哪个入口没被暴露、用户现在拿不到
  什么，附上引擎实测值作为依据；
- **归属**写"本仓库未暴露"，不是缺陷、不是上游问题；
- **取舍**（新增能力必然自带的对齐判断）登记到 `docs/parity.md`，格式同"有意保留"，
  并写明与既有子命令的关系。

计划 09 是这条规则下的第一个实例，文件为回填：实现先于规则确立，但按此规则本应
先立案。

### README 样例校验

`README.md` 与 `README.zh-CN.md` 中每个 console 代码块里的 `$ lunar …` 行都必须与真实
输出逐字节一致——译文与原文的输出是同一份，抄错一处就是错。校验脚本是仓库里的
**唯一一份** `tools/check_samples.py`，纯标准库、无依赖、发现任何一处不符即以非零码
退出：

```bash
cargo build && python3 tools/check_samples.py
python3 tools/check_samples.py README.md   # 只校验一个文件
```

脚本**不**在此处内嵌：内嵌一份就会有两份，两份迟早不一致——而这个脚本的全部作用就是
抓不一致。CI（`.github/workflows/ci.yml` 的 `samples` job）执行的就是仓库里这一个文件。

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
| 08 | [干支月与宜忌用了不同月柱基准](plans/08-month-pillar-basis.md) ✅ | 自相矛盾 | **P1** |
| 09 | [新增 `lunar bazi` 生辰八字](plans/09-bazi-chart.md) ✅ | 新增能力 | P2 |
| 10 | [新增 `lunar bazi -g` 大运](plans/10-bazi-yun.md) ✅ | 新增能力 | P2 |
| 11 | [八字补全：五行/旬空/胎元/命宫](plans/11-bazi-fill-in.md) ✅ | 新增能力 | P2 |
| 12 | [流年 / 流月 / 小运](plans/12-bazi-liu-nian.md) ✅ | 新增能力 | P2 |
| 13 | [真太阳时：只做方法论声明](plans/13-true-solar-time.md) ✅ | 文档声明 | P2 |
| 14 | [GitHub Actions CI](plans/14-github-actions-ci.md) ✅ | 工程设施 | P1 |
| 15 | [发布 0.1.0：Linux amd64 / arm64 二进制](plans/15-release-0.1.0.md) ✅ | 分发 | P2 |

✅ = 已实施。

计划 14 交付 `.github/workflows/ci.yml`（`ubuntu-26.04` + `actions/cache`，
三个验收门 + 样例校验两个 job）与 `tools/check_samples.py`。实施时把本文件里
那段内嵌脚本提到 `tools/`，因为「照抄进 workflow」会产生第二份，而抓不一致的脚本
自己有第二份就是漏洞；`the_ci_workflow_covers_the_gates` 现在断言 `docs/README.md`
不再内嵌它。套件由 96 增至 97。runner 选 26.04 而非 `latest`，是因为
`ubuntu-latest` 到 2026-11-19 仍指向 24.04。

CI 落地前实测过一件事：套件唯一的外部输入是 `tz::today()`，而 runner 是 UTC。实测
`TZ=UTC` / `America/Los_Angeles` / `Asia/Shanghai` / `Pacific/Kiritimati` 下均
96 passed，故 CI 不设 `TZ` 也不会有隐藏的时区依赖。

计划 08 由 2026-09-28 接入黄历时发现并已实施：`date -a` 的 `干支` 行印的月柱，
与同一份输出里 `宜` / `忌` 所依据的月柱，是两个不同基准（1–9999 共 119,988 天）。
归属**本仓库**——引擎两个基准都提供且都正确，错的是 `calendar::month_gan_zhi` 选了
`_exact()`。实施时另发现原计划两处事实错误，已在计划文件中更正：引擎并无「月序
基准」（两个变体都遍历 `JIE_QI_IN_USE`），且原验收值 `乙巳 庚寅 己酉` 不满足五虎遁
（庚寅是丙年的月），正确值为 `丙午 庚寅 己酉`，与市面黄历一致。


计划 09 是**新增能力**（引擎 `Lunar::eight_char` 已实现、本仓库未暴露），不是缺陷
修复，因此没有 `cal(1)` / `date(1)` 作参照——它的「现象」写能力缺失，「归属」写
本仓库未暴露。文件为回填：实现先于该规则确立，规则随后按它补写（见「执行规则」）。
它带来的取舍——无时刻只印三柱、月柱取节气瞬时基准、不做大运——已登记在
`docs/parity.md`。

`docs/parity.md` 是判定"差异是否为缺陷"的依据；新增差异先登记到那里，再决定是否
立案。

## 完成定义（DoD）

一个计划只有同时满足以下各条才算完成：

- 计划中的**复现命令**输出与"目标行为"一节描述一致（含退出码与 stderr）；
- `tests/documented_examples.rs` 有测试覆盖该行为（既有测试断言需随语义更新）；
- README / AGENTS.md 相关段落已同步，README console 块校验 `fails: 0`；
- 验收门全绿（新增/删除文件时另跑 `reuse lint`）；
- AGENTS.md 的 Known Defects 中该条目已删除。

**新增能力计划**额外要求：「现象」引用引擎实测值以证明能力齐备；「取舍」一节
非空，且每条在 `docs/parity.md` 都有对应登记。

计划 12 是**新增能力**（引擎 `DaYun::liu_nian` / `LiuNian::liu_yue` /
`DaYun::xiao_yun` 已实现、本仓库未暴露），`bazi` 新增 `-y/--year <公历年>`。
实施时有三处依据计划文件之外的事实更正：

1. **小运需时柱。** 计划写「小运跟随 `-y` 同行印出」，未提时刻依赖；实测小运
    **从时柱起算**（同一 2015 年：10:30 得 `己未`、23:30 得 `丙寅`、05:00 得
    `丁巳`），而流年 / 流月 **与时刻无关**（四个时刻完全相同）。故小运在无
    时柱时省略并印说明，与 命宫 / 身宫 同一理由（引擎会从本工具代入的正午
    作答）。
2. **起运前的年份不是「印说明」而是同一答案的一种。** 引擎 index 0 步（起运前
    那段）也带 流年，但那一步没有大运柱；本工具跳过它，改为印
    `说明: <年> 年无大运 (大运 1997-2086)`，把「何时开始 / 何时结束」一起印出。
3. **`-y` 收整数而非 `-d` 日期串**：它是一个公历年，不是一个时刻，故经
    `CalError::BadYear` 单独报错，1–9999 之外按 `YearOutOfRange` 报。

取舍已从「待实施」转正登记到 `docs/parity.md`。实施后复验：套件由 89 增至 94
（新增 5 个测试），README console 校验 22 块 `fails: 0`。

计划队列已空。计划 13 交付 README 的「The 时柱, and 真太阳时」小节与一条契约，
不改一行代码；其取舍已转正登记到 `docs/parity.md`。

计划 11 交付五行 / 旬空 / 地支十神三行与 `命局` 行。它带来的取舍——命宫 /
身宫依赖时柱故无时刻时省略、只用引擎的「旬空」名不印别名、命局不按柱对齐——
已登记在 `docs/parity.md`。实施时把步骤 0 的拆分与新行放在同一遍改动里，
故没有"拆分前后各跑一次套件"的独立计数：pinned 测试
`bazi_prints_four_pillars_only_with_a_time` 失败时打印的 `left`（真实输出）逐行
仍是拆分前那七行、顺序未变，这是"只搬移、没丢行"的证据；套件由 87 增至 89
（新增两个测试），README console 校验 19 块 `fails: 0`。

## 版权与许可：REUSE 兼容

许可声明集中在 `REUSE.toml` 一处，`LICENSES/GPL-3.0-or-later.txt` 是指向根目录
`LICENSE` 的符号链接（根 `LICENSE` 保留不动：README、发布包与发行版都按这个名字
找）。`reuse lint` 是 CI 的第四个 job，装的是 runner universe 源里的 `reuse`。

这里不用 41 个文件头是有原因的，不是图省事：`reuse` 靠**前 2048 字节**猜编码来
判断文件是不是文本，本仓库中文密集的 Markdown 猜不出结果，于是被当作二进制、
整个头被跳过，`reuse lint` 会报"未授权"——无论头里写了什么。所以文件头在这里
不构成证据，声明写在文件外才作数。实测 5.0.2（Ubuntu 26.04 源）与 6.2.0 均
`42 / 42` 通过。
