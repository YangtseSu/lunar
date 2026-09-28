# 计划 14 — GitHub Actions CI

Status: 已实施（2026-09-28） · Priority: P1 · Depends: —

> **实施时两处偏离本文件，理由如下。**
>
> 1. **`tools/check_samples.py` 取代了内嵌脚本。** 原步骤 2 说把 `docs/README.md`
>    里的脚本"原文照抄"进 workflow。照抄就会有两份，而两份迟早不一致——这个脚本的
>    全部作用就是抓不一致。改为把脚本提到 `tools/check_samples.py`（纯标准库），
>    CI 执行仓库里那**一份**，并在测试里断言 `docs/README.md` 不再内嵌第二份。
> 2. **徽章改用 workflow status 端点。** 计划正文已说明不用 shields 的 github
>    代理（对本仓库渲染成 "repo not found" 灰章），CI 徽章改用
>    `github.com/.../actions/workflows/ci.yml/badge.svg`，它指向 workflow 自身。

## 现象

本仓库**没有 CI**。AGENTS.md「Development」明写：

> There is no CI: the gates above are local, and nothing is "done" until they
> pass.

三个验收门（`cargo test` / `cargo fmt --check` / `cargo clippy --all-targets`）只在
作者本机跑过。这带来三个具体问题，都不是假设：

1. **"本地绿" 不等于 "干净检出能绿"。** 集成套件 `tests/documented_examples.rs`
   启动真实二进制（`env!("CARGO_BIN_EXE_lunar")`），依赖 `cargo test` 自己构建出
   的产物；`Cargo.lock` 已跟踪，但依赖能否解析、edition 2024 是否被工具链支持、
   二进制路径是否正确，全都没有在别的机器上验证过。
2. **文档里钉死的字节没有回归保护。** 两份 README 的 22 个 console 块（各 22 块，
   校验 `fails: 0`）只由 `docs/README.md` 的**本地脚本**守着，而那个脚本从未被
   自动执行。`docs/plans/01`–`13` 每次都要求"README console 块校验 fails: 0"，
   十三次都靠人记得手动跑。
3. **没有任何信号告诉外部读者这份代码是否健康。** 仓库是公开的（GitHub 页面 404
   说明尚未公开或已改名），但即便公开了，README 上也没有 CI 徽章能回答"它现在
   绿不绿"。

## 归属

**本仓库未配置。** 不是 `date(1)` / `cal(1)` 的行为差异，也无上游可依赖：CI 是本
仓库自己的工程设施，属「新增能力」类（见 `docs/README.md` 执行规则第 7 条后段）。

## 目标行为

新增 `.github/workflows/ci.yml`，在 `push`（master）与 `pull_request` 上跑完三个
验收门，并额外自动执行 README 样例校验。

### 选型依据（均为实测，非推断）

| 决定 | 取值 | 依据 |
|---|---|---|
| runner | `ubuntu-26.04` | `actions/runner-images` 的 Available Images 表列出 26.04(x64) 为最新；**没有** 25.10 或更新版本。`ubuntu-latest` 目前仍指向 24.04（2026-10-19 ~ 11-19 渐进迁移），故不能用 `-latest` |
| checkout | `actions/checkout@v7` | `git ls-remote --tags` 显示 v4/v5/v6/v7 均存在，v7 为最大 major |
| cache | `actions/cache@v6` | 同上，v3…v6，v6 为最大 major |
| 工具链 | `dtolnay/rust-toolchain@stable` | 该仓库只有 `master` / `stable` 两个分支；`@master` 装 nightly，本项目用 stable |
| Rust 版本 | `1.88.0`（最低要求） | edition 2024 的 MSRV；`Cargo.toml` 未设 `rust-version`，故 CI 显式装这一版以**验证** MSRV，而不是假装有 |

**缓存**：按仓库既有风格（`Cargo.lock` 已跟踪、依赖全在 lock 里）用
`actions/cache`，`key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}`，
`path: | ~/.cargo/registry ~/.cargo/git target`。用 `actions/cache` 而非
`Swatinem/rust-cache@v2` 的理由：后者会自动 `cargo fetch`，掩盖"lock 文件能否解析"
这一真实失败模式；我们要的正是让 `cargo build --locked` 的失败可见。

### 时区不是问题（已实测）

套件唯一的外部输入是 `tz::today()`，而 GitHub runner 是 UTC。实测套件在
`TZ=UTC` / `America/Los_Angeles` / `Asia/Shanghai` / `Pacific/Kiritimati` 下
**均 96 passed / 0 failed**——它按 `AGENTS.md` 记载的"参考日不可测"原则把断言写成
关系而非固定日期。故 CI 不需要设 `TZ`。

## 实施步骤

1. 新建 `.github/workflows/ci.yml`：单个 `gates` job，`ubuntu-26.04`，
   `dtolnay/rust-toolchain@stable` 装 `rustfmt` + `clippy` 组件，`actions/cache@v6`
   缓存 `~/.cargo/registry`、`~/.cargo/git`、`target`，然后依次跑
   `cargo build --locked`、`cargo fmt --all -- --check`、`cargo clippy --all-targets --locked`、
   `cargo test --locked`。
2. 加 README 样例校验 job：`ubuntu-26.04`，装 `uv`，跑
   `docs/README.md` 里那段脚本（**原文照抄**，不重新发明），断言两文件 `fails: 0`。
3. 校验 job 必须**因不一致而失败**，不能只打印：脚本退出码须反映 `fails`。
4. `AGENTS.md`：删掉 "There is no CI" 那句，改为如实描述 CI；「Development」的
   三个门仍标为本地门。
5. `AGENTS.md` 的 Testing 约定加一条：README 样例现在由 CI 逐次校验。
6. `docs/README.md`：把校验脚本的用法写清（两个文件、退出码）；把本计划加入
   计划表并标注 ✅。
7. 测试：加一个断言，锁定"CI 配置存在且引用了三个验收门"——这是**配置契约**，
   不是日历行为，用 `contains` 即可（与 `13-true-solar-time` 同规格）。
8. README（两份）：加 CI 徽章。**必须**用 GitHub Actions 的 workflow status
   badge（`github/actions/workflow/status/...`），因为它指向 workflow 本身而不需要
   shields 代理仓库可见性；另加一个静态 license badge（shields 静态端点实测 200）。

### 关于徽章（前一轮已核实的坑）

`img.shields.io/github/**` 对本仓库会渲染成写着 "repo not found" 的灰章——
shields 对不存在的仓库**不报错**，而是给一个看似无害的假徽章。所以：

- **不用** `img.shields.io/github/actions/workflow/status/...`（经 shields 代理，同样受
  仓库可见性影响），改用 `https://github.com/YangtseSu/lunar/actions/workflows/ci.yml/badge.svg?branch=master`。
- **不用** `img.shields.io/crates/v/lunar`——crates.io 上 `lunar` 这个名字被**他人**
  的包占用（`cargo search lunar` 实测指向一个陌生 crate），会把别人的 v0.1.0
  印成本项目的版本。
- **不用** `img.shields.io/github/license/...`——受同一可见性影响。

## 取舍（登记于 `docs/parity.md`）

- **用 `actions/cache` 而非 `Swatinem/rust-cache`。** 换取"依赖解析失败要可见"，
  代价是缓存 key 要自己管。
- **MSRV 用 2024 edition 的 1.88.0 显式装一次**，而不是跑 `stable` 就完事：
  多一次约 1 分钟的 job，换来"哪天 edition 2024 / 依赖真的不再兼容旧工具链"时的
  明确信号。本项目没有 `rust-toolchain.toml`，所以这条只能写在 workflow 里。
- **不加 publish job。** crates.io 上 `lunar` 名字已被他人占用，本项目当前也不应
  以该名发布；发布是独立决定，另立计划。
- **不跑 `--all-features`。** 本项目无 feature（`Cargo.toml` 无 `[features]`），
  该开关无意义；用 `--all-targets` 覆盖测试与示例。

## 验收

```bash
# CI 能发现文档漂移：临时改坏中文 README 的一行输出，校验 job 必须失败
cargo test && cargo fmt --check && cargo clippy --all-targets
# 两份 README 样例各 fails: 0（用 docs/README.md 的脚本）
```

测试：

- `the_ci_workflow_is_documented` —— `.github/workflows/ci.yml` 存在，且同时包含
  `cargo fmt`、`cargo clippy`、`cargo test` 三个门与 `ubuntu-26.04`。

## 非目标

- 不做 `cargo publish`（名字冲突，另议）。
- 不做 `cargo audit` / `cargo deny` / `codecov` / `msrv` action——本项目零额外依赖
  风险面，且 `Cargo.lock` 里只有 3 个直接依赖。
- 不加 `rust-clippy` SARIF 上传（那是 upstream 用的 CodeQL 集成，本项目无共享分支
  策略需要）。
- 不改任何 `src/` 行为。
