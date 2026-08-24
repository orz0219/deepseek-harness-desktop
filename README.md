# DeepSeek Harness Desktop（macOS 薄启动器）

一个原生 macOS 窗口（Tauri 2 / WKWebView），打开即加载你本机运行的 `dsh web`。
启动器**不打包 node / dsh / node_modules**，运行期完全复用你环境里已装的 node 与 dsh。

> 完整设计见 [`PLAN.md`](./PLAN.md)；对本机 dsh 行为的实测与修正见
> [`FEASIBILITY.md`](./FEASIBILITY.md)；自有界面的配色规范见 [`THEME.md`](./THEME.md)。

## 工作原理

```
locate dsh → spawn `dsh web` → 轮询就绪 → webview 导航 → 看守进程 → 退出时清理子树
```

1. **定位 dsh**（Tier 0-3，详见 PLAN 门禁 A）：
   Tier 0 用户指定路径 > Tier 1 扫描常见 bin 目录并按版本排序 >
   Tier 2 解析 `/etc/paths`、`/etc/paths.d/*`、`~/.zprofile` 的 PATH 导出 >
   Tier 3 `zsh -lic 'command -v dsh'` 兜底。GUI 启动不继承 shell PATH，
   因此还会把 Homebrew / nvm / pnpm 常见位置并入 spawn 的 `PATH`。
2. **拉起 dsh**：`<node> <dsh> web --host 127.0.0.1 --port <port> --no-open`
   （默认端口 3080）。必须 `--no-open`，否则 dsh 会自己拉起系统浏览器；
   cwd 设为用户 Home，传入 env 快照（HOME / PATH / SHELL / LANG /
   NODE_PATH / NVM_DIR，见 PLAN 验收点 2）；子进程放入独立进程组便于整树清理。
3. **就绪信号**：dsh 没有真实 `/health` 端点（任意路径都是 SPA 兜底页），
   就绪判定为 **`GET /` 与 `/manifest.webmanifest` 双 200**，30 秒超时——与 dsh
   官方桌面插件 `@linxin666/dsh-desktop-launcher` 的行为一致。
4. **导航 + 注入**：就绪后 webview 导航到 `http://127.0.0.1:<port>`，并向页面
   注入三类增强脚本（锚定 dsh 现有 DOM，不改动 dsh 自身前端）：
   - **归档所有对话**侧边栏按钮（锚定 dsh 搜索按钮的 aria-label；两段式确认，
     经 dsh 自身 RPC `/api/*` 执行，跳过正在运行中的会话）。
   - **右侧侧边栏**：Git 改动（调用 `get_git_diff`）、文件树（调用 `get_file_tree`）、
     文件/图片预览（文本走 `read_file_content`、图片走 `read_file_base64`，
     复制到剪贴板走 `copy_to_clipboard`、在访达中打开走 `reveal_in_finder`）。
   - **文件链接拦截**（`file_link`）：拦截 DSH 工具结果路径按钮与对话「产物」文件按钮，
     改用自有弹窗展示，而非跳转到系统默认处理。
5. **看守与清理**：dsh 以独立进程组运行；App 退出时对整个进程组发 SIGTERM，
   宽限 5 秒后 SIGKILL 幸存者，并用 `pgrep -g` 校验子树已清（PLAN 验收点 4）。
6. **单实例与窗口行为**：二次启动只聚焦已有窗口；点关闭按钮隐藏到程序坞
   （后台 dsh 继续运行），点程序坞图标恢复。

## 已实测的关键事实（详见 FEASIBILITY.md）

- **启动命令必须加 `--no-open`**：否则 dsh 会自己拉起系统浏览器。
- **没有真实 `/health` 端点**：`/health` 只是 SPA 兜底页（任意路径都返回 200 HTML），
  就绪信号改用 `GET /` + `/manifest.webmanifest` 双 200。
- **关机桥接 dsh 自带**：dsh web 已注册 loopback-only 的
  `/api/dsh-desktop-launcher/shutdown`，启动器无需实现原生关机桥。
- **默认端口 3080 ✅**、**回环信任栅栏 ✅** 与 dsh 桌面插件一致。
- 若配置端口上已有 dsh 在跑（如终端里手动起的），启动器直接连接该实例，
  不再重复拉起。

## 构建与运行

需要：Rust 工具链、Xcode Command Line Tools；运行期需已装 `node` 与 `dsh`。

```sh
# 安装 Tauri CLI（一次性）
cargo install tauri-cli --version "^2"

# 开发运行
cd src-tauri
cargo tauri dev

# 单测（纯逻辑层：定位 / 启动计划 / 设置，不依赖 Tauri）
cargo test --lib
```

首次启动若未发现 dsh，初始页会让你填入 dsh 可执行文件路径（例如
`/opt/homebrew/bin/dsh`），保存后自动重新定位并拉起。

## 配置

设置持久化于
`~/Library/Application Support/com.deepseek.harness.desktop/settings.json`：

| 字段 | 说明 | 默认 |
|---|---|---|
| `dsh_path` | Tier 0 用户指定的 dsh 可执行文件路径（唯一完全可靠的来源） | 无 |
| `node_path` | 显式指定 node 运行时；缺省时从 dsh 的 shebang 解析 | 自动解析 |
| `port` | dsh web 监听端口（loopback） | `3080` |
| `profile` | 透传给 `dsh web` 的 `--profile` 参数 | 无 |

> 注意：当前设置页只支持填写 dsh 路径；修改端口需手动编辑上述 JSON 文件后重启。

## 打包 / 签名 / 公证（PLAN §7）

安全模型：**App Sandbox OFF**（spawn 外部用户 binary 所必需）+
**Hardened Runtime ON**（由 codesign 标志启用）。

```sh
cargo tauri build   # 生成 .app / .dmg

APP_PATH=target/release/bundle/macos/"DeepSeek Harness.app" \
APPLE_KEY_ID=... APPLE_ISSUER=... APPLE_KEY_PATH=... TEAM_ID=... \
./sign.sh          # codesign (runtime) + notarytool 公证 + stapler
```

## 仓库布局

```
deepseek-harness-desktop/
 ├─ PLAN.md                      设计实施方案（v4.1）
 ├─ FEASIBILITY.md               门禁实测与对 PLAN 的修正
 ├─ THEME.md                     自有界面配色规范
 ├─ ui/index.html                初始状态/引导页（无构建步骤的静态页）
 └─ src-tauri/
    ├─ Cargo.toml                tauri v2 + Tauri-free 纯逻辑 lib（可单测）
    ├─ tauri.conf.json           window / csp / identifier
    ├─ entitlements.plist        Sandbox OFF
    ├─ resources/settings.default.json
    ├─ icons/                    应用图标（icon-source.svg 为源图）
    └─ src/
       ├─ lib.rs                 公共类型 + 模块声明（纯逻辑，无 Tauri 依赖）
       ├─ locate_dsh.rs          定位（Tier 0-3）+ 单测
       ├─ dsh_launch.rs          启动计划构造 + env 快照 + 设置 + 单测
       └─ main.rs                Tauri 粘合：定位→拉起→就绪轮询→导航→supervisor→清理
```

## 已知问题与限制

按「最小可用、随 dsh 演进」原则推进，以下为当前已知边界：

- **注入按钮依赖 DOM 锚点**（搜索按钮 aria-label / 侧边栏文案），dsh 改版可能
  失效；批量归档范围含 blank 会话，与侧边栏可见集合略有差异（属已自知的取舍）。
- 就绪探测为「`/` + `/manifest.webmanifest` 双 200 且 `/api` 404」签名，
  极端情况下仍可能把端口上另一个 dsh 形态的服务误认（残leave歧义已接受）。
- 跨机访问（非回环）不在第一版范围（dsh 设计上锁死回环栅栏）；
  Windows/Linux 不在第一版范围。
- 归档管理（撤销归档 / 物理删除）由启动器直接编辑 `~/.dsh` 存储，**操作后需重启
  dsh 生效**（dsh 在内存缓存中）；若 dsh 由本启动器托管则自动重启一次，否则
  页面会提示用户手动重启。

### 已修复的已知缺陷（历史记录）

早期版本存在的以下问题均已修复：意外退出后「重启一次」不可达、导航后无错误页
（现出错时导航回内置页）、stderr 不排空导致 dsh 长跑假死、无文件日志、改端口需
手编 JSON、以及 `alert()` 在 WKWebView 下不显示（现改用页面内联提示并新增「重新
启动」按钮）。
