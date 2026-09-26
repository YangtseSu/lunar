# 计划 02 — `--color` / `--no-color`：注释宣称的"后写者胜"并未实现

Status: 已实施 · Priority: P1 · Depends: 无

## 现象

```console
$ lunar cal 2026 10 --color --no-color | cat -v | grep -c '\^\['
6
$ lunar cal 2026 10 --no-color --color | cat -v | grep -c '\^\['
6
```

两种顺序都**着色**。`src/main.rs` 的注释却写着：

```rust
// The last flag on the command line wins, so `--color --no-color`
// is never, and `--no-color --color` always.
```

实际 match 是 `(true, _) => Always` 在 `(false, true) => Never` 之前，所以只要
出现 `--color` 就恒为 Always；两个布尔量在解析后不再携带顺序信息。

## 目标行为

向 `cal(1)` 看齐，并让"后写者胜"成为真的：

| 命令行 | 结果 |
|---|---|
| 无旗标 | `auto`：stdout 是终端才着色 |
| `--color` | `always` |
| `--color=always` / `--color=auto` / `--color=never` | 对应模式 |
| `--no-color` | `never`（`--color=never` 的简写） |
| `--color --no-color` | `never`（后者胜） |
| `--no-color --color` | `always`（后者胜） |
| `--color=sometimes` | clap 参数错误，退出码 2 |

## 实施步骤

1. `src/main.rs` 的 `Command::Cal` 定义改为：

   ```rust
   #[arg(
       long = "color",
       value_name = "WHEN",
       num_args = 0..=1,
       default_missing_value = "always",
       value_parser = ["auto", "always", "never"],
       overrides_with = "no_color",
   )]
   color: Option<String>,

   #[arg(long = "no-color", overrides_with = "color")]
   no_color: bool,
   ```

   `overrides_with` 是 clap 的 POSIX 语义：两者互相覆盖，**最后出现的胜**。两侧都
   要写（clap 的 overrides 不是传递关系）。实现后必须用验收命令逐条确认顺序行为，
   若 clap 版本对 `num_args=0..=1` 的覆盖行为不符预期，退路是把两个旗标声明为
   `conflicts_with`（矛盾组合直接拒绝，与 `-s/-m` 一致），并在 README 说明。
2. 解析处（`main.rs` 的 `Command::Cal` 分支）改为：

   ```rust
   let color = match (color.as_deref(), no_color) {
       (_, true) => mark::Color::Never,
       (Some("never"), _) => mark::Color::Never,
       (Some("always"), _) => mark::Color::Always,
       _ => mark::Color::Auto,
   };
   ```

   并把那段假注释改写成对实际规则的陈述（无历史叙事）。
3. 测试（`tests/documented_examples.rs`）：新增
   `the_color_flags_take_the_last_word`，断言：
   - `--color --no-color` 的输出不含 `\x1b[`；`--no-color --color` 含；
   - `--color=never` 不含、`--color=always` 含、`--color=auto` 在管道下不含；
   - `--color=sometimes` 退出码 2。
   既有 `--color` / `--no-color` 两个测试保持通过（语义未变）。
4. 文档同步：
   - `README.md` 选项表把 `--color / --no-color` 行改为
     `--color[=WHEN]`（auto/always/never）与 `--no-color` 简写，并说明后写者胜；
   - `README.md`「Known divergences」删掉该条；
   - `docs/parity.md` 的对应行从 🚧 02 改为 ✅；
   - `AGENTS.md`「Known Defects」删掉该条。

## 验收

```bash
cargo test && cargo fmt --check && cargo clippy --all-targets
lunar cal 2026 10 --color --no-color | cat -v | grep -c '\^\['   # 0
lunar cal 2026 10 --no-color --color | cat -v | grep -c '\^['    # >0
lunar cal 2026 10 --color=never | cat -v | grep -c '\^['         # 0
lunar cal 2026 10 --color=auto | cat -v | grep -c '\^['          # 0（管道）
```

## 非目标

- 不实现 `NO_COLOR` / `CLICOLOR` 环境变量（新特性，另议）；
- 不改 `mark.rs` 的着色规则与 `Color` 类型本身，只改旗标到 `Color` 的映射。

## 实施记录

`--color` 改为 `Option<String>`，两侧写 `overrides_with`；解析处按
`(_, true) => Never` / `Some("never")` / `Some("always")` / `_ => Auto` 映射。
`--color=sometimes` 退出码 2，`--color` 与 `--color=always` 同义。

一处计划外的修正：`num_args = 0..=1` 会把紧跟其后的位置参数当成 `WHEN`，
`cal --color 2026 10` 因此报 `invalid value '2026'`。补 `require_equals = true`
后，取值只能写成 `--color=always`，`--color` 恢复为纯旗标。

验收脚本里的 `cat -v | grep -c '\^\['` 数到的 3 行不是 SGR：`cat -v` 把 UTF-8
的 `0x9B` 显示成 `M-[`，网格里的「五」「九」都含该字节。改用
`grep -c $'\x1b\['` 才是真的在数转义序列。
