# DeepSeek Harness 桌面版（macOS）实施方案（修订版 v4.1 — 薄启动器，不打包）

状态：草案 v4.1（在 v4 经 Round 004 review 后修订）。v4 已定为「**薄启动器、不打包 node/dsh**」（用户全是有环境的 harness 用户，运行期复用其已装的 node/dsh）。v4.1 落实 Round 004 的 locator 优先级重做 + macOS 安全模型修正 + 五个验收点；并按 Human 决策做了收敛：**不约束 dsh 的数据目录（dsh 自管）**、**不冻结集成契约 / `/health` 先做最小就绪信号**、**端口默认 3080（用户可告知/设置覆盖，不用 port=0 自动分配）**。

> 设计原则（贯穿全文）：**最小可用、随 dsh 演进**。我们是服务于 dsh 的启动器，不限制 dsh 的行为（数据目录自管），也不把 dsh 的 CLI/健康端点冻结成「永久担保」的硬契约——按 dsh 当前行为实现，dsh 变了就更新启动器。富契约（`protocolVersion`、状态细分、版本门槛）等用户真正长时间使用后暴露需求时再加。

## 1. 目标与范围

**做：**
- 一个原生 macOS 窗口（WKWebView），打开即加载用户本机运行的 `dsh web`。
- 启动时由 Rust 侧**定位用户已安装的 `dsh`**（及其 node），按 dsh 当前行为拉起 `dsh web --host 127.0.0.1 --port 3080`（端口默认 3080，设置可覆盖）。
- 同机浏览器访问 `http://127.0.0.1:3080` 时，看到的是**同一后端、同一前端**——桌面与浏览器共用 `dsh web`。
- **不打包 node 运行时、不打包 dsh、不打包 node_modules**；运行期完全复用用户环境里已装的东西。
- 已签名、已公证、可分发的薄 `.app` / `.dmg`。

**不做：**
- 不打包/内置 node、dsh、`@deepseek-ai/*`、原生 addon。
- **不约束 dsh 的数据目录 / 运行时行为**——dsh 怎么存数据、存哪，是 dsh 自己的事；启动器只负责把 dsh 拉起来并看守进程（我们服务于 dsh，而非限制它）。
- **不冻结 dsh 的 CLI/健康端点为永久契约**——按 dsh 当前行为实现，dsh 演进时启动器随之更新（Human 负责跟进 dsh 变更）。
- 不重写 agent 内核、工具、前端（全在 `dsh web` 里，原样复用）。
- 不在第一版支持跨机浏览器访问（`dsh web` 设计上拒绝 `--host 0.0.0.0`，回环信任栅栏默认锁死）。
- 不在第一版做 Windows/Linux 对称包（见 §11 待定）。

**心智模型**：这是「桌面启动器 + 外部服务管理器」，不是「运行时打包器」。生命周期 `Tauri → supervisor → 用户 dsh 进程 → agent workers`。app 与 dsh 之间只走 dsh 当前支持的轻量接口（CLI 参数 + HTTP `/api` + `/health` 就绪 + WS），绝不 import dsh 内部模块；**dsh 的数据位置由其自管，启动器不指定、不限制**。**当前最大工程风险已从「Node 运行时分发」转为「GUI 上下文里可靠定位用户 dsh」+「随 dsh 演进而跟进」**（Round 004 结论 + Human 决策）。

## 2. 架构

```
┌──────────────────────────────────────────────────────────────┐
│  Tauri (Rust 薄壳)  src-tauri/main.rs                          │
│   · locate dsh（用户指定 > 扫描候选排序 > 解析 PATH 文件 > zsh -lic）│
│   · spawn: <node> <dsh> web --host 127.0.0.1 --port 3080       │
│   · polling http://127.0.0.1:3080/health 就绪，再 navigate      │
│   · single-instance + supervisor(ProcessState) + 崩溃重启        │
└───────────────┬───────────────────────────┬──────────────────┘
                │ spawn（用户环境里的 node/dsh）│ http://127.0.0.1:3080 (导航)
┌───────────────▼───────────────────────────▼──────────────────┐
│  用户已装的 dsh web（node + @deepseek-ai/* + 原生 addon）        │
│   = web profile: 托管 dist + /api (Typert RPC) + 2×WS 下行      │
│   + /health（就绪信号；富结构见 §5 门禁 0，暂不做）              │
│   （数据目录由 dsh 自管，启动器不干预）                          │
└───────────────┬───────────────────────────────────────────────┘
                │ 同机浏览器也连这里（同 origin，见 §6 设计决策）
        ┌───────▼────────┐        ┌──────────────┐
        │ 桌面 webview   │   +    │ 浏览器(同机) │
        └────────────────┘        └──────────────┘
```

> 端口：默认 **3080**，存于启动器设置（可被用户覆盖）。不用 `port=0` 自动分配——端口是已知值，直接轮询 `/health` 后导航，无需端口发现机制。若 3080 被其他程序占用，错误页提示用户在设置里改端口（用户直接告知/配置）。

### 2.1 为什么是启动器而非打包器（v4 决策依据，沿用）
- 目标用户全是有环境的 harness 用户，已装 node + dsh；打包一份是重复且带来维护/签名/体积灾难。
- 启动器把风险从「Node 运行时分发」转移到「可靠定位用户已装的 dsh」——可控。
- 若未来要发给无环境的普通用户，再回头做 v3 式打包（supervisor/port 模型已预留）。

## 3. 仓库布局（精简）

```
deepseek-harness-desktop/
 ├─ PLAN.md
 ├─ README.md                      # 构建/运行/「需先装 dsh（支持 --port 与 /health 就绪）」说明
 ├─ src-tauri/
 │   ├─ Cargo.toml                 # tauri v2 + tauri-plugin-shell + serde
 │   ├─ tauri.conf.json            # 窗口(about:blank 起手) + csp + identifier
 │   ├─ entitlements.plist         # Hardened Runtime ON / App Sandbox OFF
 │   ├─ src/
 │   │   ├─ main.rs                # 启动 + /health 探活 + 单例 + 状态机 + ps tree 校验
 │   │   ├─ locate_dsh.rs          # 定位（候选发现 + 排序 + 用户确认 + 缓存）
 │   │   └─ dsh_launch.rs          # 按 dsh 当前行为构造启动命令（端口默认 3080，随 dsh 演进而更新）
 │   └─ resources/settings.default.json
 └─ ui/                            # 极简设置页（选择 dsh 路径、端口、显示版本检测）
```

`locate_dsh` 产出结构化候选（`DshCandidate`，见 §5 门禁 A），便于 debug 与用户确认。

## 4. 前置条件

- Rust toolchain + `cargo`、Tauri CLI（钉死版本）。
- **用户侧（运行期必备，写进 README + 首屏检测）**：已装 `node`（≥ 项目要求版本）与 `dsh`（当前版本须支持 `--port`（默认 3080）与 `/health` 就绪信号；数据位置由 dsh 自管，启动器不要求 `--data-dir`）。
- macOS：Xcode CLT（`xcrun`、`codesign`、`notarytool`）；薄壳签名用，不需要 `lipo`/`file` 扫 addon。
- Apple Developer Program 账号（签名/公证薄壳）。

## 5. 阶段 0 — 可行性门禁（先验证，再写骨架）

开工前必须确认。v4.1 的最大风险是「GUI 上下文里可靠定位并启动用户 dsh」。

**门禁 0 — 启动器↔dsh 的轻量接口（不冻结、随 dsh 演进）**：
- **启动命令**：按 dsh **当前行为** 用 `dsh web --host 127.0.0.1 --port 3080`（默认 3080，可在启动器设置里覆盖）。端口是**已知值**（来自设置），无需从 stdout 解析、无需 OS 自动分配、无需端口发现机制。
- **就绪信号（最小即可）**：`GET http://127.0.0.1:3080/health` 返回就绪即导航；Slice 1 **只要求「好了没」这一最小信号**，不要求特定 JSON 字段。
  - **富结构 `{live,ready,version,protocolVersion}` 暂不做**（Round 004 曾建议）。等用户真正长时间使用、出现「分不清 STARTING/READY」「要拦过旧 dsh」等需求时再加；那时再决定是否引入 `protocolVersion` 之类的兼容信号。
- **数据目录（设计决策，非接口）**：启动器**不管理、不约束** dsh 的数据位置；dsh 在自己的用户环境里自管数据，是 dsh 的行为，启动器服务于 dsh 而非限制它。启动器只拥有自己的少量设置目录（`settings.json`，见门禁 D）。因此**不要求 dsh 支持 `--data-dir`**。

1. **在干净 Mac（有 node+dsh，但非本仓库 dev 环境）能否 `dsh web --port 3080` 起来 + `/health` 返回就绪**：验证 `@deepseek-ai/*` 解析、spawn 子进程（bash/git/python）正常。
2. **从 GUI 上下文（无 shell PATH）能否找到并启动 dsh（头号门禁）**：Finder 双击的 `.app` 不继承 shell PATH。按门禁 A 的优先级验证（尤其「非终端启动」也能起）。
3. **WKWebView 兼容性**：grep 前端是否用到 WebSocket、**SharedArrayBuffer（需 COOP/COEP）**、Clipboard、**File System Access API（`showOpenFilePicker()` 基本不可用，需 native bridge）**、Web Worker。逐项回退路径。

**门禁 A — dsh 定位策略（优先级重做，Round 004）**：
- **Tier 0 用户指定路径（★★★★★，唯一完全可靠，#1 且非 optional）**：首次启动若未配置 dsh → 弹「选择 dsh 可执行文件」→ 存 `~/Library/Application Support/<identifier>/settings.json`（`{ dsh_path, node_path?, port?, source }`）；以后直接校验（inode+版本），不盲信缓存。
- **Tier 1 扫描候选并排序（★★★，辅助）**：扫描 `/usr/local/bin`、`/opt/homebrew/bin`、`~/.local/share/pnpm`、`~/.nvm/versions/node/*/bin`、`~/.cargo/bin`、`~/Library/pnpm` 等，但**不是 first-match-wins**：产出候选列表 `{executable, node, dsh_version, source}`，按 dsh 版本从高到低排序选最优（node 安装方式极碎片，扫不全是预期，故仅作辅助）。
- **Tier 2 解析 PATH 文件（不执行 shell）**：读取 `/etc/paths`、`/etc/paths.d/*`、`~/.zprofile`（解析 `export PATH=...`），拼出 PATH 后在其中 `command -v dsh`。很多 nvm 用户把 PATH 放 `.zshrc`，故**解析 `.zprofile` 而非执行 shell**。
- **Tier 3 `zsh -lic 'command -v dsh'`（末位回退）**：用 **interactive login**（`-l -i -c`）而非 `-lc`（`.zshrc` 多在 interactive 才初始化）。仍脆弱，仅作最后兜底。
- **备选 onboarding**：让用户首次从终端跑 `dsh setup-desktop`，由 CLI 注册 `~/Library/.../dsh-path.json`（开发者工具常见做法，代价是多一步）。
- 定位产出统一为 `DshCandidate { executable, node: Option<PathBuf>, version, source }`，写日志便于排查。

**门禁 B — 端口模型（默认 3080）**：端口固定为默认 **3080**（设置可覆盖），是已知值，直接轮询 `/health` 后导航。**single-instance 锁避免同一 app 多次启动自相冲突**（第二次启动激活已开窗口，不再起 dsh）。若 3080 被**其他程序**占用，dsh 启动会失败 → 错误页提示用户在设置里改端口（用户直接告知/配置），而非自动换端口。
**门禁 C — 多实例**：single instance lock；第二次启动 `activate` 已开窗口，而非再起 dsh backend。
**门禁 D — 启动器自身设置目录（仅限 app 自己的配置）**：启动器只把**自己的**设置（选中的 dsh 路径、node 路径、端口）存于 `~/Library/Application Support/<identifier>/settings.json`，并在 `~/Library/Logs/<identifier>/` 写日志。**dsh 的运行时数据由 dsh 自管，启动器不干预、不传 `--data-dir`**。webview 缓存由 Tauri 管理，与 dsh 数据无关。启动器 spawn dsh 时显式设置 cwd 为用户 Home（避免继承 app 包目录），但不对 dsh 的数据位置做任何假定或约束。
**门禁 E — supervisor 重启语义**：仅 `SIGSEGV`/意外退出（`exitCode!=0` 且非已知「启动失败」）才 restart once；**不**在启动依赖缺失/配置错误/端口绑定失败（端口被占用属此类，不重启，提示用户）时重启。Node 给 Rust `exitCode`/`signal`/`stderr tail` 用于判断。

**Slice 1 前必补验收点（Round 004 五条，写入 §6 验收）**：
1. `DshCandidate` 结构化输出 + 日志。
2. **spawn 环境快照**：定义并保存传入的环境（至少 `HOME`、`PATH`、`SHELL`、`LANG`、`NODE_PATH`、`NVM_DIR`）；dsh 可能靠 `PATH` 找 git/python/bash；写 log。
3. **每次启动重验 inode/version**，不盲信缓存（`nvm use` / `brew upgrade` 可能改路径）。
4. **退出语义**：SIGTERM 给 node 不一定传到 dsh/worker 树；确认 dsh 转发信号 / node 清理 worker，否则 app 退出后残留后台进程；加 **ps tree verification**。
5. **WebView origin（设计决策，见 §6）**：navigate `http://127.0.0.1:3080` 与同端口 Safari **同 origin** → 共享 cookie/localStorage/auth token，须作为有意设计记录。

## 6. 阶段 1（Slice 1）— 启动器骨架（~1.5~2 周）

目标：`tauri dev` 打开即见用户本机 `dsh web`，跑通一个 agent 任务，且从第一天验证最终架构（定位 dsh + 默认端口 3080 + /health 就绪 + supervisor + 单例 + 退出清理）。

**Slice 1A — 定位 dsh + 外部进程 + 最终生命周期**
- `locate_dsh.rs`：按 Tier0→3 定位，产出 `DshCandidate`；找不到 → 显示**选择/安装引导页**（非白屏）。首启弹「选择 dsh」并存 settings（含端口，默认 3080）。
- `dsh_launch.rs`：按 dsh 当前行为构造 `Command::new(<node>).args([<dsh>, "web", "--host","127.0.0.1","--port",<settings.port 默认 3080>])`；传入 **env 快照**（门禁 E 验收点 2）；cwd 设为用户 Home。
- 启动后 Rust 轮询 `http://127.0.0.1:<port>/health` 直到就绪；随后 `webview.navigate("http://127.0.0.1:<port>")`。窗口**以 `about:blank` 起手**（不要让 Tauri 自己访问 port 的 devUrl），探活完成后再 navigate。
- **supervisor 状态机**：Rust 只维护 `ProcessState`：`STARTING → READY → EXITED → RESTARTING → STOPPING`。业务错误走 dsh 自己的 error page，Rust 不吸收（避免三层状态机）。崩溃 restart once，再失败显示 error page；App 退出时向 dsh 发 SIGTERM 并做 **ps tree verification** 确认子树已清（验收点 4）。
- single-instance lock（门禁 C）。

**Slice 1B — /health 就绪 + 错误/升级页（关键 Slice）**
- 实现 `/health` **最小就绪探测**（Slice 1 只判「好了没」；富 JSON 结构暂不做，见门禁 0）。
- 错误页覆盖：dsh 未找到（引导选择/安装）、启动失败（展示 stderr tail 引导排查；若失败原因是端口被占用，提示去设置改端口）、配置错误——每种明确引导。
- **数据目录（设计决策）**：dsh 数据由其自管，启动器不指定位置、不传 `--data-dir`（见门禁 0/D）。
- **设计决策（验收点 5，有意）**：桌面 webview 与同端口浏览器**共享同一 origin/cookie/localStorage/auth token**——因为二者本就是同一个本地 `dsh web` 的两个客户端，单后端多端是预期行为；此点在设计与隐私说明中显式记录（用户若在浏览器登出，桌面端也会登出，反之亦然）。

**验收（完整链路，含 R004 五验收点）**：
`cargo tauri dev`
→ locate dsh（**GUI 上下文，非终端启动**；输出 `DshCandidate` 日志）
→ spawn dsh web `--port 3080`（env 快照已保存 log，cwd=用户 Home）
→ `http://127.0.0.1:3080/health` 就绪 → navigate
→ agent task 成功返回
→ **手动 kill dsh** → `RESTARTING` 一次 → 二次 kill → `EXITED`/`FAILED` + error page
→ 再启 App B → `activate` 已开窗口（无 dsh process #2）
→ 退出 App → **ps tree verification** 确认 dsh/worker 子树已清，无孤儿进程
→ 升级 dsh 后重开 App → 重验 inode/version 命中新路径（非缓存）

## 7. 阶段 2（Slice 2）— 生产构建、薄壳签名与公证

目标：`.app` 已签名、已公证、可发 `.dmg`，Gatekeeper 不拦截。**只签薄壳**（Rust 二进制 + 极薄 helper），不签用户环境的 node/dsh（app 包外，不由本 app 公证）。

- **macOS 安全模型（Round 004 修正）**：
  - **App Sandbox = OFF**（`com.apple.security.app-sandbox=false`）：开 Sandbox 后 spawn 外部用户 binary（`/opt/homebrew/bin/dsh`、nvm 路径）会极受限，开发者工具类（终端/IDE/本地服务管理器）通常关 Sandbox。
  - **Hardened Runtime = ON**：保护 code injection / unsigned executable memory / dylib loading；spawn 外部 dsh 不受影响。
  - 最小 entitlement：`allow-jit=false`、`disable-library-validation=false`；Tauri/WKWebView 不报错就不加，**不提前加 `allow-unsigned-executable-memory`**。
- **签名策略（fail fast）**：`sign.sh` 强制 `set -euo pipefail`；薄壳结构简单，逐文件签后签 `.app`。
- **notarization 前检查**：`codesign --verify --deep --strict` + `spctl --assess`（verify 可用 `--deep`，仅不要用 `codesign --deep --sign`）。`xcrun notarytool submit` + `xcrun stapler staple`。
- **验收**：干净 Mac 双击安装，`spctl --assess -v` 通过。

## 8. 阶段 3 — 分发与 OS 集成（可选增强）

- Tauri 自动更新插件（更新**启动器**，dsh 由用户自行升级）。
- Dock 行为、托盘/菜单栏、文件关联（`.dsh` / `cordis.yml`）、快捷键。
- 设置页（指定 dsh 路径、端口、显示版本检测）。
- 跨机浏览器访问：若后续需要，单独加认证层（不在第一版）。

## 9. 风险与缓解（v4.1）

| 风险 | 影响 | 缓解 |
|---|---|---|
| **GUI 上下文找不到 dsh / PATH（头号）** | app 双击打不开 backend | 门禁 A：Tier0 用户指定(#1 必填) + 候选排序 + 解析 PATH 文件 + `zsh -lic` 末位；找不到显式引导 |
| 用户 dsh 版本过旧（无 `--port` / `/health`） | 启动失败 | 错误页展示 stderr tail 引导；版本门槛作为**后续增强**（见 §11） |
| 端口 3080 被其他程序占用 | dsh 起不来 | 错误页提示去设置改端口（用户直接告知/配置）；单实例锁避免自相冲突 |
| 多 node/dsh 匹配排序错误 | 选到旧/不兼容 | 候选列表按 dsh 版本排序，非 first-match |
| `zsh -lc` 回退读不到 nvm PATH | 末位回退失效 | 改 `-lic` 且仅末位；主靠 Tier0/1/2 |
| spawn env 缺失（git/python/bash 找不到） | dsh 子功能崩 | 验收点 2：保存并 log env 快照（HOME/PATH/SHELL/LANG/NODE_PATH/NVM_DIR） |
| 缓存路径失效（nvm use/brew upgrade） | 启动旧/错 dsh | 验收点 3：每次启动重验 inode/version |
| 退出残留孤儿进程 | 后台泄漏 | 验收点 4：SIGTERM + ps tree verification |
| 多实例起多个 dsh backend | 数据竞争 | 门禁 C：single instance，二次激活已开窗口 |
| backend 崩溃后白屏 | 用户见死窗口 | 状态机 + restart once + error page；Rust 只管 ProcessState |
| **App Sandbox 阻 spawn 外部 dsh** | 起不来 | 门禁：Sandbox OFF / Hardened ON（Round 004 修正） |
| Tauri CSP 挡 `/api` 的 WS | 前端连不上 | csp 放行 `ws://127.0.0.1` |
| WKWebView 不支持的 API（SAB/Clipboard/File Picker） | 功能缺失 | 门禁 3 grep + native bridge 回退 |
| **webview 与浏览器同 origin 共享登录态** | 隐私/UX 意外 | §6 设计决策：有意共享（单后端多端），显式记录 |
| 启动器设置丢失（重装/多用户） | 需重选 dsh | 门禁 D：settings.json 存于 `~/Library/Application Support/<id>/` |
| **dsh 演进导致启动器失配** | 某次 dsh 升级后桌面端起不来 | 设计原则：按 dsh 当前行为实现，dsh 变了即更新启动器（Human 负责跟进） |

> 注：dsh 自身的数据目录位置、CLI 形态、健康端点结构是 dsh 的职责与演进范围，启动器按当前行为集成并随之更新，不将其冻结为永久契约（设计决策）。

## 10. 里程碑与工时估算

- 阶段 0（门禁：定位 dsh + 默认端口 3080 + /health 就绪 + WKWebView + 单例）：数天（含「非终端启动」实机验证）。
- Slice 1A（定位 + 外部进程 + env 快照 + 默认端口 + /health 就绪 + supervisor + 单例 + ps tree）：约 1 周。
- Slice 1B（`/health` 就绪 + 错误页 + 共享 origin 决策）：约 0.5~1 周。
- Slice 2（薄壳签名/公证：Sandbox OFF / Hardened ON）：数天~1 周（远比 v3 轻）。
- 阶段 3（分发/集成/设置页）：按需。

总评：v4.1 把真正难点收敛为「GUI 里可靠定位并看守用户 dsh」，工程量与风险比 v3 低一个量级。最大剩余风险 = **macOS 环境发现**；契约层面遵循「最小可用、随 dsh 演进」，不提前冻结。

## 11. 待定问题（确认）

- **默认端口 3080**：当前定为 3080（设置可覆盖）。若 Human 想换默认值，改 `settings.default.json` 即可。
- **富 `/health` 结构与版本门槛（后续增强，非现在做）**：`{live,ready,version,protocolVersion}` 及老版本拦截，等用户长时间使用后确有需求再加。当前 Slice 1 只做最小就绪信号。
- **共享 origin 是否有意**：v4.1 按「有意共享（单后端多端）」记录（§6 设计决策）；若 Human 认为应隔离，需改架构（如随机 token / 独立 storage 分区）。
- **locator 最终策略**：是否采用「`dsh setup-desktop` 注册路径」的 onboarding（多一步但最稳）？还是纯首启弹窗选择？
- **Windows/Linux 对称包（推迟）**：先跑通 macOS；跨平台引入 PATH/进程差异，后续评估。
- **未来无环境用户分发**：若以后发给没装 dsh 的普通用户，再回头做 v3 式打包；当前架构已预留切换成本。
- **Electron 备选**：当前启动器场景下 Electron 优势更小（不嵌 node），仍以 Tauri 推进。
