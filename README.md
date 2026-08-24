# DeepSeek Harness Desktop

> 一个把 DeepSeek Harness 变成 macOS 原生桌面应用的小工具。
> A tiny macOS desktop wrapper that turns DeepSeek Harness into a native app.

你在浏览器里用 DeepSeek Harness（DSH）做开发时，是不是经常要在网页、编辑器和终端之间来回切换？这个启动器把 DSH 的网页直接装进一个原生 macOS 窗口，并在对话旁边加上本地文件、Git 改动和文件预览——让你少切窗口、多看代码。

When you use DeepSeek Harness (DSH) for coding in the browser, you constantly jump between the web page, your editor, and the terminal. This launcher puts DSH into a native macOS window and adds a local file tree, Git diffs, and file previews right next to your chat—so you switch less and read more code.

## ✨ 功能亮点 / Features

**🖥️ 原生桌面体验 · Native desktop experience**

打开就是独立的 macOS 窗口，不占用浏览器标签页；点关闭只是藏到程序坞（后台 DSH 继续运行），点程序坞图标随时恢复。单实例设计，二次启动只会聚焦已有窗口。

Launch into a standalone macOS window instead of a browser tab. Closing the window only hides it to the Dock (DSH keeps running in the background); click the Dock icon to bring it back anytime. Single-instance: a second launch just focuses the existing window.

**📁 本地文件树 + 文件/图片预览 · Local file tree & file/image preview**

右侧边栏直接浏览当前项目的文件树（自动跳过 `node_modules`、`.git` 等），点开任意文件即可预览文本内容或查看图片——不用切到编辑器或终端。

Browse your project's file tree right from the sidebar (auto-skips `node_modules`, `.git`, etc.), and open any file to preview its text or view images—no need to switch to your editor or terminal.

**🔀 Git 改动一览 · Git changes at a glance**

侧边栏一键查看当前工作区的 Git 改动（diff），新增/删除行数与改动文件列表一目了然，省去切到终端跑 `git diff`。

See the current working tree's Git changes (diff) with one click from the sidebar—additions/deletions and the changed-file list at a glance, without dropping to a terminal `git diff`.

**📋 一键复制 / 在访达打开 · Copy & reveal in Finder**

文件内容一键复制到剪贴板，或一键在访达（Finder）中定位并选中——把本地文件与 AI 对话之间的搬运成本降到最低。

Copy file contents to the clipboard with one click, or reveal and select the file in Finder—minimizing the back-and-forth between your local files and the AI chat.

**🗂️ 对话批量归档 · Batch archive conversations**

一键把会话批量归档，保持工作区整洁；归档会自动跳过正在运行的会话，并采用两段式确认避免误操作。

Archive your conversations in one click to keep the workspace tidy. Archiving automatically skips sessions that are still running and uses a two-step confirmation to prevent mistakes.

**🔗 文件链接拦截弹窗 · File-link interception**

对话里 DSH 工具返回的文件路径按钮、以及「产物」文件按钮，不再跳转到系统默认程序，而是用自带弹窗展示，查看更顺手。

Path buttons returned by DSH tools in the chat—and "artifact" file buttons—no longer jump to the system default app; they open in a built-in popup for a smoother viewing experience.

## 🚀 安装与快速上手 / Install & get started

**前置条件 · Prerequisites**

本启动器运行期复用你本机已安装的 `node` 与 `dsh`，自身不打包这两者。使用前请确保：

- 已安装 Node.js
- 已安装 `dsh`（DeepSeek Harness CLI）

This launcher reuses the `node` and `dsh` already installed on your machine at runtime—it does not bundle either. Before starting, make sure you have:

- Node.js installed
- `dsh` (the DeepSeek Harness CLI) installed

**下载安装 · Download & install**

1. 前往 GitHub Releases 下载最新的 `DeepSeek Harness-x.y.z.dmg`。
2. 打开 `.dmg`，把 **DeepSeek Harness** 拖入「应用程序」。
3. 从启动台或程序坞打开即可。

1. Go to GitHub Releases and download the latest `DeepSeek Harness-x.y.z.dmg`.
2. Open the `.dmg` and drag **DeepSeek Harness** into Applications.
3. Launch it from Launchpad or the Dock.

**首次启动 · First launch**

如果启动器没有自动找到 `dsh`，首屏会让你填写 `dsh` 可执行文件的路径（例如 `/opt/homebrew/bin/dsh`）。保存后它会自动定位并拉起 DSH，之后就能正常使用了。

If the launcher can't find `dsh` automatically, the first screen asks for the path to the `dsh` executable (e.g. `/opt/homebrew/bin/dsh`). After saving, it locates and launches DSH for you, and you're ready to go.

## ❓ 常见问题 / FAQ

**打开后界面是空的？ / The window is blank?**

请确认本机已安装 `node` 与 `dsh`，并在首屏正确填写了 `dsh` 路径。启动器本身不内置这两者。

Make sure `node` and `dsh` are installed on your machine and that you entered the correct `dsh` path on the first screen. The launcher does not bundle them.

**关闭窗口后 DSH 还在跑吗？ / Is DSH still running after I close the window?**

点关闭只是把窗口藏到程序坞，后台 DSH 继续运行；只有完全退出应用才会清理 DSH 进程。

Closing only hides the window to the Dock and DSH keeps running in the background; the DSH process is cleaned up only when you fully quit the app.

**支持 Windows / Linux 吗？ / Windows / Linux support?**

当前仅支持 macOS。

Currently macOS only.
