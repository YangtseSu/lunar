# 计划 15 — 发布 0.1.0：Linux amd64 / arm64 二进制

Status: 已实施（2026-09-28） · Priority: P2 · Depends: 14

> **实施记录。** arm64 用原生 `ubuntu-26.04-arm` runner，不用 cross（用户明确）。
> 版本号单一来源：`Cargo.toml` 的 `version` 被 `#[command(version)]` 读取，实测
> `lunar --version` → `lunar 0.1.0`；产物名从 manifest 取而非从 tag 取，避免两者漂移。
> 打包、校验和、`sha256sum -c` 已在本地干跑验证。`cargo install --git` 一并写进
> README，因为源码安装对有工具链的人仍是最短路径。
>
> **一处诚实说明**：smoke test 里带 `TZ=Asia/Shanghai lunar date` 是有意的——`tz-rs`
> 的 `TimeZone::local()` 读系统 tzdb（已查源码确认），静态二进制带不走时区库，所以
> 这一步验证的是"目标机需要 `/usr/share/zoneinfo`"这一事实，而不是装饰。
>
> **未能在本机验证的部分**：本机是发行版 `rust`，无 `rustup`，因此**没有**在本地
> 试过交叉或指定 target 构建；产物由 CI 在真实 runner 上构建并 smoke test。

## 现象

本项目**只能从源码构建**：`cargo install --git …` 或者 clone 后 `cargo build`。而
它是一个 CLI 工具——工具的使用门槛应该是一个二进制文件，不是一套构建环境。`AGENTS.md`
的「Dependency First」把本仓库定位成 `lunar-rs` 的包装器，包装器的分发方式不该比被
包装的库更麻烦。

具体地：README 顶部给出的用法是

```bash
cargo build --release
./target/release/lunar date
```

对已有 Rust 工具链的人没问题，对其余的人是一次安装成本。

## 归属

**本仓库未配置。** 与 `date(1)` / `cal(1)` 行为无关，属「新增能力」（工程设施），
按 `docs/README.md` 执行规则走计划流程。

## 目标行为

tag `v0.1.0` 触发发布：构建 Linux x86_64 与 aarch64 两个 release 二进制，
打成 `.tar.gz`，附 `SHA256SUMS`，创建 GitHub Release。

### 选型依据（实测）

| 决定 | 取值 | 依据 |
|---|---|---|
| arm64 构建方式 | **原生 `ubuntu-26.04-arm` runner** | 用户明确要求「用不着 cross，有单独的 arm64 ubuntu」。runner-images 表中该 label 存在；两镜像均预装 Rust 1.98.1 + rustup 1.29.1 |
| release action | `softprops/action-gh-release@v3` | `git ls-remote --tags` 显示 v1/v2/v3，v3 为最大 major |
| 版本号来源 | `Cargo.toml` 的 `version` | `#[command(version)]` 走 clap，已实测 `./target/release/lunar --version` → `lunar 0.1.0`，**单一来源，无需在 workflow 里再写一遍** |
| 产物格式 | `.tar.gz` | Linux 惯例 |
| 链接目标 | **`x86_64-unknown-linux-gnu` / `aarch64-unknown-linux-gnu`** | 用户明确指定。宿主 target，即工具链默认产物，跑在用户自己的 glibc 上 |

### 为什么原生 arm 而不是 cross

cross 需要装 `cross`、拉 20+ 个 docker 镜像、每架构多一层失败面；原生 runner 是官方
托管的、直接可用。代价是两个 job 重复，但重复的 5 行 YAML 换掉的是整条 cross 工具链的
维护成本。用户已明确此偏好。

### 为什么是 gnu 而不是 musl

用户指定 `x86_64-unknown-linux-gnu`。初版计划曾选 musl 静态链接并写了论证，**那是我的
取舍，不是用户的要求**；链接方式是用户可见的分发决定，不该替用户定。客观差别记在这里：

- gnu 动态链接，在**更老的 glibc** 上可能报 `GLIBC_2.xx not found`；
- musl 静态链接，跨发行版最稳，但分配器不同，且需要 `musl-tools`；
- 用户装二进制的机器本来就跑 glibc，gnu 即「默认、所见即所得」。

结论：用 gnu。若将来要覆盖老 glibc 机器，那是新增一个 `-musl` 产物，不是替换。


## 实施步骤

1. `.github/workflows/release.yml`：触发 `push: tags: ["v*"]`；`permissions: contents: write`；
   两个 build job（矩阵 `ubuntu-26.04` / `ubuntu-26.04-arm`），各自
   `dtolnay/rust-toolchain@stable` + 宿主 target、`actions/cache@v6` 缓存 target、
   `cargo build --release --locked`、`tar` 打包为 `lunar-<version>-<arch>.tar.gz`。
2. `release` job 依赖两者，`softprops/action-gh-release@v3` 上传两个 tar.gz +
   `SHA256SUMS`，`generate_release_notes: true`。
3. `SHA256SUMS` 在 Linux 上用 `sha256sum` 生成，命名 `<arch>.tar.gz` 前缀对齐。
4. 校验：CI 的 smoke test 在真实 runner 上确认二进制可运行（`file` + 三种 `TZ` 调用）。
5. tag `v0.1.0` 并 push，验证 Release 与两个产物可下载、校验和可验。
6. README 两份加「安装」一节，给出下载命令；这属于用户可见行为变更，按
   `AGENTS.md` 与行为同 commit 更新。
7. 测试：加断言锁定「README 的安装路径存在」+「release workflow 存在且含两个架构」。

## 取舍（登记于 `docs/parity.md`）

- **只发 Linux 两个架构**，不发 macOS / Windows：这是一个 Unix 工具，模仿的是
  `date(1)` / `cal(1)`。要扩再说。
- **不发布到 crates.io**：该名字已被他人占用（`cargo search lunar` 实测指向陌生
  crate），且本项目是二进制工具，crates.io 分发的是库。
- **gnu 而非 musl**：用户指定。宿主 target 即工具链默认产物，代价是在**很老**的 glibc
  上可能报 `GLIBC_2.xx not found`；若要覆盖那类机器，应另发一个 `-musl` 产物而非替换。
- **不自动递增版本**：版本由 `Cargo.toml` 单一来源决定，tag 必须与之一致，由人打。

## 验收

```bash
cargo test && cargo fmt --check && cargo clippy --all-targets
python3 tools/check_samples.py
# 发布后：
gh release view v0.1.0 --repo YangtseSu/lunar      # 3 个资产
gh run list --repo YangtseSu/lunar --workflow release
```

测试：

- `the_release_workflow_covers_both_architectures` —— `release.yml` 存在，包含
  `ubuntu-26.04` 与 `ubuntu-26.04-arm`，且含 `v*` tag 触发。
- `the_readme_documents_installing` —— 两份 README 都给出下载安装路径。

## 非目标

- 不发 crates.io。
- 不做 macOS / Windows 产物。
- 不做自动发布（tag 推送即发布，不做 nightly / 自动版本号）。
- 不改任何 `src/` 行为。
