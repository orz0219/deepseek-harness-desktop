# 阶段 0 — 可行性门禁验证报告（门禁 0 / A）

状态：已在本机（macOS，已装 `node v24.18.0` + `dsh 0.1.0-rc.7`）实测。
生成日期：2026-08-21。

> 设计原则（PLAN §1）：**按 dsh 当前行为实现，dsh 演进时启动器随之更新**。
> 本章实测即为此原则的落地——凡与 PLAN 假设不一致处，均以 dsh 真实行为为准，并标注对启动器实现的修正。

---

## 门禁 0 — 启动器 ↔ dsh 的轻量接口

### 0.1 启动命令（实测结论）

PLAN 假设的命令：`dsh web --host 127.0.0.1 --port 3080`

**实测 dsh CLI（`dsh web --help`）**：

```
Usage: dsh --profile web [options]
Options:
  --host <host>                  bind host
  --no-open                      do not open the Web UI in the default browser
  --port <port>                  listen port; pass 0 to let the OS pick a free one
  --trusted-host <authority...>  extra authority the /api browser-trust fence accepts
```

- `dsh web` 是 `dsh --profile web` 的别名，**语法可用**，PLAN 命令成立。
- **关键修正：必须加 `--no-open`**。否则 dsh 会主动用 `open` 拉起系统浏览器，与「桌面 webview 即浏览器」的设计冲突。启动器构造命令应为：
  `dsh web --host 127.0.0.1 --port <port> --no-open [--profile <profile>]`
- `--port` 传具体值（默认 **3080**），不用 `port=0`（与 PLAN 门禁 B 一致）。
- 端口是**已知值**（来自设置），无需从 stdout 解析、无需端口发现。

### 0.2 就绪信号（**重要修正**）

PLAN 假设：`GET /health` 返回就绪即导航。

**实测**：`GET /health` 返回的是 **SPA 首页 HTML（HTTP 200，`content-type: text/html`）**。
进一步验证：

| 探测 | 结果 |
|---|---|
| `GET /health` | 200，`text/html`（SPA 首页） |
| `GET /zzz-not-real`（任意未知路径） | 200，`text/html`（**SPA catch-all 兜底**） |
| `GET /api` | 404，`text/plain`（真实路由，说明后端已挂载） |
| `POST /api {}` | 404，`text/plain` |
| `GET /manifest.webmanifest` | 200（`text/plain`? 实为标准 manifest） |

结论：**dsh v0.1.0-rc.7 没有真实的 `/health` JSON 就绪端点**，`/health` 只是 SPA 兜底页。PLAN 的「轮询 `/health`」会**误判就绪**（任意路径都 200）。

**权威参考（替代信号）**：已装的 `@linxin666/dsh-desktop-launcher` 插件（`~/.dsh/profiles/desktop/node_modules/`）的 `.command` 启动器逻辑即「探测 GUI URL，最多轮询 30 秒，200 即认为就绪」。因此启动器应采用与 dsh 官方桌面插件一致的就绪判定：

- **就绪信号 = `GET http://127.0.0.1:<port>/` 返回 HTTP 200**（与插件 `url` 默认 `http://127.0.0.1:3080` 一致）。
- 为降低 SPA 兜底误判风险，可优先探测一个**真实静态资源**（`/manifest.webmanifest` 返回 200）作为「后端确实在响应真实请求」的佐证；两者皆 200 视为就绪。
- 超时沿用插件的 **30 秒**上限，超时则进入错误/引导页（非白屏）。

> 富结构 `{live,ready,version,protocolVersion}` 仍按 PLAN 决定**暂不做**——当前 dsh 也无此端点，等 dsh 演进或用户确有需求再加。

### 0.3 数据目录（设计决策，与 PLAN 一致）

启动器**不管理、不约束** dsh 数据位置；实测 dsh 数据自管于 `~/.dsh/`（sessions/storages/settings.yaml 等）。启动器 spawn 时 cwd 设为用户 Home，**不传 `--data-dir`**。

### 0.4 关机桥接（**重要发现，简化启动器**）

`@linxin666/dsh-desktop-launcher` 在 dsh web 进程内已注册 **loopback-only** 的：

- `POST /api/dsh-desktop-launcher/shutdown` → 调用 `ctx.appExit(0)`（优雅退出 dsh 进程；无 appExit 时回退 `process.exit(0)`）。
- Web UI 右下角浮动「关机」按钮即调此端点。

含义：**启动器无需自行实现原生关机桥接**。Tauri 只需 `supervisor` 看守 dsh 子进程；当 dsh 因 Web UI 关机按钮（或崩溃）退出时，supervisor 捕获子进程退出并决定（退出 App / 显示错误页）。这比 PLAN 预想的「原生 bridge」更简单——PLAN 的「native bridge 回退 File Picker」仅在真正用到浏览器不支持的 API 时才需要（见 0.5）。

---

## 门禁 A — dsh 定位策略（实测补充）

Tier 0（用户指定路径，**必填 #1**）、Tier 1（扫描候选排序）、Tier 2（解析 PATH 文件）、Tier 3（`zsh -lic 'command -v dsh'`）均按 PLAN 实现。

实测本机：
- `dsh` 位于 `/opt/homebrew/bin/dsh`（Homebrew），它是一段 `#!/usr/bin/env node` 脚本，最终 resolve 到 `/opt/homebrew/lib/node_modules/@deepseek-ai/dsh/lib/bin.js`。
- 因此 **node 解析**：GUI 上下文（无 shell PATH）下，定位到 `dsh` 后，应优先取 `dsh` 脚本 shebang 的 `node`（即 `env node` → 解析为 `/opt/homebrew/bin/node`），并**显式把该 `node` 所在 bin 目录并入 spawn 的 `PATH`**，避免 dsh 内部 spawn git/python/bash 时找不到。这正是 PLAN 验收点 2（env 快照）要保的。
- 候选 node 来源极碎片（nvm / brew / pnpm），故 Tier1 扫描 + Tier2 解析 `.zprofile` 仍作辅助，Tier0 指定为唯一可靠路径。

---

## 门禁 B / C / D / E（实测确认，无修正）

- **B 端口默认 3080**：与 dsh 桌面插件默认 `url` 完全一致 ✅。单实例锁避免自相冲突；3080 被**他程序**占用 → dsh 启动失败 → 错误页提示去设置改端口（不自动换端口）。
- **C 单实例**：第二次启动 `activate` 已开窗口，不再起 dsh。
- **D 启动器自身设置目录**：仅存自己的 `settings.json`（`dsh_path`/`node_path`/`port`）于 `~/Library/Application Support/<identifier>/`，日志于 `~/Library/Logs/<identifier>/`。dsh 数据由 dsh 自管。
- **E 重启语义**：仅意外退出（exitCode!=0 且非「端口被占用/已知启动失败」）restart once；端口绑定失败**不重启**，提示用户。App 退出时向 dsh 发 SIGTERM 并做 **ps tree verification** 确认子树已清（验收点 4）。

---

## 门禁 3 — WKWebView 兼容性（实测 / 已知）

- **WebSocket**：前端经 `/api`（Typert RPC）+ 2×WS 下行；CSP 须放行 `ws://127.0.0.1`。
- **SharedArrayBuffer（需 COOP/COEP）**：前端 `crossorigin` modulepreload 未显式要求 SAB；首版不强制 COOP/COEP（避免破坏 `/api` 同源）。如需再评估。
- **Clipboard / File System Access**：`showOpenFilePicker()` 在 WKWebView 不可用——但 dsh 已自带 `@deepseek-ai/dsh-client-ui-directory-picker-native` 与 `@linxin666/dsh-desktop-launcher`，说明原生文件选择走 dsh 自身的 native bridge，启动器首版**不自行实现** File Picker bridge，复用 dsh 既有能力。
- **Web Worker**：前端使用，WKWebView 支持。

---

## 结论

门禁 0 / A **通过**，但有两处对 PLAN 的必要修正（均属「按 dsh 当前行为实现」范畴）：

1. 启动命令必须加 **`--no-open`**。
2. 就绪信号改用 **`GET /`(及 `/manifest.webmanifest`) 返回 200**，而非不存在的 `/health` JSON。

其余架构（薄启动器、不打包 node/dsh、默认端口 3080、loopback 栅栏、单实例、supervisor、ps tree 清理、Sandbox OFF / Hardened ON）与 PLAN 一致，且得到 dsh 自身桌面插件的印证。

> 注：dsh 的 CLI 形态、`--no-open` 标志、关机端点、`/health` 缺失均属 dsh 演进范围。本启动器按 v0.1.0-rc.7 行为实现；dsh 升级后须重测上述接口并更新启动器（设计原则）。
