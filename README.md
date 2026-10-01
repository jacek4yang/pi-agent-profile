# Pi Profile

独立的 Rust 交互式配置程序：为 Linux、macOS、Windows 上的 Pi Agent 生成配置，安装或更新所选插件，并管理后续修改、诊断与配置回滚。

**不是 `~/.pi/agent` 的备份，不同步认证和会话。** 安装器内嵌规则模板，下载一个适合本机的 Release 即可运行；不需要克隆仓库、不需要 Rust，也不需要 Python。Pi 本身以及 npm 插件的安装/更新仍要求 **Node.js ≥ 22.19 和 npm**。

[下载 Release](https://github.com/jacek4yang/pi-agent-profile/releases/latest) · [构建状态](https://github.com/jacek4yang/pi-agent-profile/actions) · [安全边界](docs/SECURITY.md) · [配置设计](docs/DESIGN.md)

## 1. 下载与运行

| 系统 | Release 文件名后缀 | 包内程序 |
|---|---|---|
| Linux x86_64 | `x86_64-unknown-linux-musl.tar.gz` | `pi-profile` |
| Linux ARM64 | `aarch64-unknown-linux-musl.tar.gz` | `pi-profile` |
| macOS Apple Silicon | `aarch64-apple-darwin.tar.gz` | `pi-profile` |
| macOS Intel | `x86_64-apple-darwin.tar.gz` | `pi-profile` |
| Windows x64 | `x86_64-pc-windows-msvc.zip` | `pi-profile.exe` |

文件完整名称为 `pi-profile-v<版本>-<目标>.tar.gz` 或 `.zip`。从同一次 Release 下载 `SHA256SUMS`，在解压和运行前核对目标压缩包的 SHA-256。校验和检测传输损坏，不等于发行者签名。

Linux/macOS，解压后进入目录：

```bash
./pi-profile
```

Windows，在 PowerShell 或 Git Bash 进入解压目录：

```powershell
.\pi-profile.exe
```

Windows 会检测 Git for Windows 的常见路径和 `git.exe` 对应目录，优先使用 **Git Bash** 执行 npm/Pi 命令，并在选择启用时把检测出的真实 Bash 路径写入 Pi 的 `shellPath`。不会将 Linux 的 `/bin/bash` 写到 Windows，也不会把 WSL 的 `bash.exe` 当作 Git Bash。没有 Git Bash 时，可使用标准 npm 安装的 Node CLI 入口；无法安全解析的自定义包装器会报告问题，不拼接到 `cmd /c` 猜测执行。

下载器/浏览器走哪条线路由它自身决定；安装器不能追溯改变下载路径。该程序没有 Authenticode/Apple notarization 签名。请核对来源与校验和并按系统的单应用许可流程操作，不要全局关闭安全检查。

## 2. 首次运行：问答生成，而非覆盖整份配置

按回车采用显示的默认值，输入 `y` / `n` 作出选择。向导依次询问：

1. 是否管理全局工程规则和运行时补充。
2. 是否启用 Code Mode only。
3. 六个插件分别启用、保持现状或解除全局加载。
4. 是否解除所选插件已有的版本锁定。
5. 是否应用建议的超时、重试和基础压缩设置。
6. 是否修改默认 provider/model/thinking；摘要模型单独填写。
7. 可选高级设置：steeringMode、Windows Git Bash。
8. 下载网络模式及代理地址；Pi 模型请求代理另行选择。
9. 是否允许安装/更新 Pi 核心。
10. 查看变更文件和插件清单，**最后确认才写入**。

没有任何问题要求输入 API key、OAuth token、Cookie 或登录密码。安装完成后按 Pi 支持的方式自行登录。模型 ID 是配置值，不代表账户实际拥有该模型；在 Pi 中用 `/model` 核实可用模型。示例中的 `gpt-6-astra`、`gpt-6-luna` 是该个人配置的选择，不是对所有账户的可用性承诺。

### 插件目录

| ID | 包 | 作用 |
|---|---|---|
| `fff` | `@ff-labs/pi-fff` | 文件搜索 |
| `lsp` | `@narumitw/pi-lsp` | Language Server 集成 |
| `usage` | `pi-context-usage` | 上下文用量显示 |
| `prune` | `pi-context-prune` | Agent 主动整理上下文 |
| `web` | `pi-web-search` | Web 搜索；服务可能另需认证 |
| `compact` | `@lll9p/pi-better-compaction` | 压缩与文本 fallback |

这些第三方包在 Pi 中执行代码；选择安装意味着信任它们。Pruner 和 Better Compaction 可能产生额外模型调用。安装器不运行模型请求，也不声称安装后已验证 provider-native compaction。

已有插件的版本 pin、对象形式的资源过滤参数和不相关插件都会保留。选择跟随最新版本时，仅解除已选插件的 pin。`disable` **移除对应全局 package 声明，不删除插件目录**；Pruner/Better Compaction 同时写 `enabled:false`。其他项目中的局部安装或规则不会被修改，因此项目仍可能单独加载同名插件。

## 3. 下次运行：修改、更新、诊断、恢复

检测到本机保存的选择后，无参数运行会显示菜单：

```text
1 修改功能/配置
2 更新已选择的 Pi/插件
3 查看配置差异
4 诊断
5 备份/恢复
6 退出
```

常用命令（Windows 对应 `pi-profile.exe`）：

```bash
./pi-profile configure                  # 重新问答；以保存选择为默认
./pi-profile update                     # 复用选择，确认后更新
./pi-profile update --no-self-update    # 仅配置/选中的插件，不更新 Pi 核心
./pi-profile configure --offline        # 只写本地配置，不下载
./pi-profile plan                       # 不写文件，不发网络请求
./pi-profile update --dry-run           # 预览本次操作
./pi-profile doctor                     # 非破坏性诊断
./pi-profile doctor --strict            # 缺少必要条件/配置漂移返回非零
./pi-profile backups                    # 列出备份 ID
./pi-profile restore --backup <ID>       # 预览并确认恢复
./pi-profile export --output my.local.json
```

`--no-self-update` 指 **Pi 核心**，不是 Rust 安装器。更新安装器本身时重新下载新的 Release；它会读取相同本机状态，不会重新要求填写所有信息。

**修改后退出并重新启动 Pi。** `/reload` 对已打开会话的资源增删并不等价于完整进程重启，尤其不要仅凭 reload 认定旧插件已经卸载。正在运行的 Pi 和配置程序不应同时修改配置。

## 4. 目标目录与无交互部署

默认目标：`~/.pi/agent`（Windows 为用户目录下 `.pi/agent`）。优先级：

```text
--agent-dir > PI_CODING_AGENT_DIR > 默认用户目录
```

```bash
./pi-profile --agent-dir /path/to/agent configure
./pi-profile --agent-dir /path/to/agent doctor
```

不需要 `sudo`。本机选择存放在目标目录的 `.pi-profile/state.json`，不保存在公开仓库。

自动化使用显式答案文件，不通过向导无限接受默认值：

```bash
./pi-profile configure --answers examples/answers.json --yes --offline
./pi-profile update --yes
```

`configure --yes` 没有 `--answers` 会拒绝执行，避免误配置新机器。没有交互终端时，也不会偷偷假定批准。答案文件仅存非秘密设置；导出不会复制 `settings.json` 中未知字段、认证或会话。

答案字段见 [examples/answers.json](examples/answers.json)。插件的 `enable` / `disable` / `keep` 含义不同：`keep` 不修改也不更新；向导对已安装而选择 n 的插件会进一步询问是解除加载还是保持现状。

### “不修改”与“关闭”

- `tuning:false`：保留现有调优字段，不重置成出厂值。
- `model:null`：保留现有默认模型。
- `provider_proxy:null`：保留现有 Pi 代理；`""`：移除 `httpProxy`；URL：设置代理。
- `steering_all:false`：不写此字段，不主动重置已有值。
- `code_mode:false`：移除本工具的启用项并加入 `-codemode`；不使用不存在的 `codemode.mode="off"`。
- `rules:false`：删除由本程序管理的规则块，保留块外个人内容。

需要撤销一次配置变更时，使用该次备份；不要把所有 n 理解为“恢复之前的一切”。

## 5. 配置保护与幂等性

JSON 仅合并选中功能的字段，不替换全部 `settings.json`，不重置不相关 `extensions`、`skills`、trust 或认证设置。数组按具体用途处理：package 身份匹配、资源过滤保留、Code Mode 仅处理自己的条目。

两份全局规则使用以下边界：

```markdown
<!-- pi-profile:begin -->
由选择生成的规则
<!-- pi-profile:end -->
```

块外内容保留；块内人工改动会报告冲突，不静默覆盖。确认要重新生成时才使用 `--overwrite-managed`。旧版完整模板在内容匹配时可直接纳入管理；不认识的既有内容不会被删除。

`AGENTS.override.md` 会影响全局规则生效，所以存在它时必须先处理或在向导中取消管理规则。项目 `AGENTS.md` 的审批门槛仍然有效；项目 `.pi/APPEND_SYSTEM.md` 也可能优先于全局补充。本工具不会修改项目授权边界。

相同选择且配置没有漂移时不重写、不生成多余配置备份。正常在线 `update` 仍会检查所选插件更新，这与“本地写入幂等”不是同一个概念。

## 6. 网络策略

默认 `split`：

| 类型 | 路由 |
|---|---|
| 国外匿名公开资源和本程序的公开 npm/Pi 更新 | `http://127.0.0.1:10809` |
| 后续 Agent 的个人认证请求，包括 Google API/OAuth、已认证 GitHub | `http://127.0.0.1:10808` |
| 国内已知服务、localhost、私网/Tailscale | 直连 |
| Pi 模型请求 | 独立保留或配置，可选 `http://127.0.0.1:10810` |

安装器公开更新路径隔离用户 npmrc 和项目 cwd，设置进程级代理/registry，避免无意携带用户 npm token。只使用 HTTP/HTTPS proxy，因为 npm 并不统一支持 SOCKS 环境变量；你的混合代理端口应填写 `http://...`。选择 `direct` 明确直连；选择 `inherit` 才继承进程网络设置。

`10809` 不可用时，不尝试 `10808`，不偷偷直连。已确认的本地配置可保存，但在线更新返回非零并说明未完成。代理检查仅确认 TCP listener，不证明上游一定可达。端口属于执行命令的本机，不代表 SSH 目标、容器或 MCP 后端有同样监听。

自定义 `npmCommand` 可能覆盖路由，所以保留但拒绝自动更新，要求明确处理；不会为了联网修改用户的全局 npm/Git 配置。现有下载任务不需要个人认证；`10808` 写入动态 Agent 规则，不拿它盲目重试失败的公开安装。

## 7. 备份、失败与回滚

备份：`~/.pi/agent/.pi-profile/backups/<ID>/`。写入前保存变化文件的原始字节和校验和；原先不存在的文件也被记录。每个文件用同目录临时文件替换，多个文件的整体操作是 **可恢复事务，不是全局原子事务**。

```bash
./pi-profile backups
./pi-profile restore --backup <ID>
```

遇到进程中断或磁盘写入失败，`.pi-profile/pending.json` 会阻止下次继续配置。使用 `restore`，默认优先恢复这个中断事务。备份中损坏、路径越界、符号链接或恢复后的意外覆盖均会被拒绝。目标文件在备份后有额外改动时先检查，必要时明确 `restore --force`；恢复本身也保存当前状态作为新的备份。

**配置回滚不等于降级 Pi/插件二进制。** npm 安装、扩展脚本和缓存副作用不属于配置事务。在线更新在本地配置之后进行，部分包失败不会伪装成全部完成，也不会抹掉独立成功结果。

日志不输出全量配置或凭据，但备份可能包含原有 `settings.json` 内的敏感字段。Unix 本机状态目录使用 0700、写入文件 0600；Windows 依赖用户目录 ACL，不自动修改整棵目录权限。备份不是加密保险箱，不要提交它们。

## 8. Doctor 与真实验收

Doctor 检查保存选择、管理块、配置漂移、Pi 版本、所选插件 `package.json` 是否实际存在、可用工具与本机代理 listener。它不会把“settings 里有声明”当作“插件已经安装”。`auth.json` 仅检查是否存在，不读取内容。

Doctor 不是模型调用测试、LSP 启动测试或真实长程任务验收。部署后建议在一个小项目中确认 `/model`、搜索工具、Code Mode、LSP 和原生测试命令，再开展长任务。

Python 的 `ty`/`ruff`、Rust 的 `rust-analyzer` 工具由现有项目环境提供。安装器不任意安装语言工具链。Rust 项目 pin 的 toolchain 与全局 toolchain 不一定相同；在项目内用 `rustup show active-toolchain` 判断，必要时为那个 toolchain 添加 `rust-analyzer`/`rust-src`。

## 9. 源码、CI 和发布

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

Rust 版本由 `rust-toolchain.toml` 固定；依赖由 `Cargo.lock` 固定。初始仓库第一次 CI 会生成锁文件并格式化源代码，提交到 main；后续构建要求已有锁文件并使用 `--locked`。测试不要求真实账号，不调用模型，也不依赖用户本机代理。

每次 main 更新运行五个平台原生测试和目标构建。只有全部成功才可发布新 Cargo 版本的 Release：先建立 draft、上传五个压缩包及总校验和，最后公开；已有正式版本不会被覆盖。维护者发布下一版时修改 `Cargo.toml` 版本并合并到 main。每个包的 `BUILD-INFO.json` 记录源码 SHA、目标、Rust 版本；Release tag 指向实际测试构建的提交。

CI 和打包脚本会使用构建机的 Python，但分发的安装程序为 Rust 原生二进制，不依赖 Python/Node 启动。项目不承诺消除所有模型提前结束或网络中断；全局规则解决工作策略，运行故障仍须按证据诊断。

## 10. 仓库边界

公开的是源码、规则模板、非秘密示例、文档和构建流程。不上传 `auth.json`、API key、OAuth token、session、模型原始 payload、npm 缓存或真实机器答案。

程序没有后台守护、遥测或定时更新。所有更改发生在用户运行命令并确认后。不要把密码或 token 填进任何模型 ID 或代理 URL 字段。

MIT，见 [LICENSE](LICENSE)。
