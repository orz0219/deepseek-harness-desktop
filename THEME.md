# THEME.md — DeepSeek Harness 桌面启动器配色规范

本仓库「自己绘制」的界面只有两处（真正的 `dsh web` 是外部进程，不在此仓库内）：

- `ui/index.html` — 启动器的加载 / 引导 / 错误页（无构建步骤的静态页）
- `src-tauri/src/main.rs` 中注入到 DeepSeek Harness 侧边栏的「归档所有对话」按钮

这两处统一采用下方浅色冷调方案，与 DeepSeek Harness 网页视觉一致。**新增自有页面 / 按钮时，请直接复用下方 token，不要另起一套颜色。**

---

## 设计原则

- **极简留白** — 大面积白底，减少视觉干扰。
- **冷色调为主** — 蓝 / 紫系传达科技、专业调性。
- **渐变光晕** — 主区域柔和蓝紫渐变，增加层次而不喧宾夺主。
- **高对比度** — 深色文字配白色背景，保证可读性。
- **克制强调色** — 彩色仅用于关键元素（品牌标记、主按钮、状态徽章）。

---

## 设计 Token

### 颜色（CSS 变量，详见 `ui/index.html` `:root`）

| Token | 值 | 规范来源 | 用途 |
|---|---|---|---|
| `--bg` | `#ffffff` | 主背景 白 | 页面底色 |
| `--text` | `#1f2937` | 主文字 深灰 | 标题、正文、输入文字 |
| `--text-secondary` | `#6b7280` | 次要文字 中灰 | 副标题、label、占位符说明 |
| `--border` | `#e5e7eb` | 浅边框 | 卡片、信息框描边 |
| `--card` | `#ffffff` | 侧边栏 纯白 | 卡片背景 |
| `--accent` | `#3b6cf6` | 品牌蓝 | 输入框聚焦描边 / 焦点环 |
| `--accent-soft` | `#e5edff` | 蓝色标签 | 状态徽章浅蓝底（如 `starting`） |
| `--accent-gradient` | `linear-gradient(135deg, #4f7cff 0%, #8b5cf6 100%)` | 强调色 蓝/紫渐变 | 主按钮、品牌标记 |

### 渐变光晕（页面背景）

```css
background:
  radial-gradient(1100px 560px at 50% -10%, #eef2ff 0%, rgba(238, 242, 255, 0) 60%),
  radial-gradient(900px 520px at 82% 18%, #f5f3ff 0%, rgba(245, 243, 255, 0) 55%),
  #ffffff;
```

顶部中心柔和的蓝（`#eef2ff`）与紫（`#f5f3ff`）光晕，叠在白底上。

### 阴影 / 圆角

| 用途 | 值 |
|---|---|
| 卡片阴影 | `0 12px 40px rgba(31, 41, 55, 0.1)`（浅、柔） |
| 卡片 / 输入框 / 按钮圆角 | `16px` / `9px` / `9px` |
| 状态徽章圆角 | `9px` |

### 字体

```css
font-family: -apple-system, system-ui, "Segoe UI", Roboto,
             "PingFang SC", "Microsoft YaHei", sans-serif;
```

---

## 组件规范

### 主按钮（CTA）

- 背景：`var(--accent-gradient)`（蓝→紫渐变）
- 文字：`#ffffff`，`font-weight: 600`
- 圆角 `9px`，加柔光阴影 `0 4px 14px rgba(79, 124, 255, 0.25)`
- `:hover` 微微提亮（`filter: brightness(1.05)`）
- `:disabled` → 背景 `#e5e7eb`，文字 `#9ca3af`，无阴影

```css
button {
  background: var(--accent-gradient);
  color: #fff;
  border: none;
  border-radius: 9px;
  font-weight: 600;
  box-shadow: 0 4px 14px rgba(79, 124, 255, 0.25);
}
```

### 幽灵按钮 (Ghost Button) — 如「新会话」

用细边框而非填充定义边界，保持界面通透（参考 DeepSeek Harness 侧边栏「新会话」按钮）。注意：注入到 dsh 侧边栏的「归档所有对话」按钮**不是**幽灵按钮，而是与 dsh 原生图标按钮同款的纯图标按钮（见下文）。

**外观**

| 属性 | 值 |
|---|---|
| 形状 | 圆角矩形，圆角 `8px`（独立页面/「新会话」实测 ≈ `12px`，内联侧边栏按钮用 `8px` 贴合相邻 dsh 按钮） |
| 背景 | 纯白 `#ffffff` |
| 边框 | `1px solid #e5e7eb`（浅灰，对应 `--border`） |
| 文字 | `#1f2937`，常规字重，`14–15px` |
| 内边距 | 垂直 `12–14px`、水平宽松（内联侧边栏按钮用 `9px 12px` 以适配行高） |
| 阴影 | 无 |

**内容组成（`[⊕ 新会话]` 这种带图标按钮）**

- 图标：圆圈内嵌加号 `⊕`，线条细腻，`16–18px`
- 图标与文字间距：`8px`

**交互状态**

| 状态 | 效果 |
|---|---|
| 默认 | 白底 + 浅灰边框 `#e5e7eb` |
| `:hover` | 边框变深（`#374151`）+ 背景变浅灰（`#f9fafb`） |
| `:active` | 轻微下沉（`transform: translateY(1px)`）+ 背景 `#f3f4f6` |

```css
.ghost-btn {
  background: #ffffff;
  color: #1f2937;
  border: 1px solid #e5e7eb;   /* --border */
  border-radius: 8px;
  padding: 9px 12px;
  font-weight: 400;
  cursor: pointer;
  transition: background .15s ease, border-color .15s ease;
}
.ghost-btn:hover  { background: #f9fafb; border-color: #374151; }
.ghost-btn:active { background: #f3f4f6; transform: translateY(1px); }
```

> 说明：「新会话」按钮实测边框为浅灰 `#e5e7eb`；早前提纲里的「深灰按钮边框」以本处浅灰为准。

### 注入的「归档所有对话」图标按钮（纯图标，非幽灵）

侧边栏里由 `src-tauri/src/main.rs` 的 `INJECT_JS` 注入，位于「搜索」图标按钮**左侧**。为避免之前带 1px 浅灰边框的幽灵样式过于突兀，改为与 dsh 原生图标按钮（搜索 / 添加）完全一致的样式：**28×28 圆形、无边框、透明底**，颜色用 dsh 的 `--dsw-alias-label-secondary`，hover / active 用 dsh 自带的 `--dsw-alias-interactive-bg-hover` / `-active` 变量（浅色 / 深色主题下都和原生按钮一致）。图标为归档盒（outline，`stroke=currentColor` 继承按钮颜色），明确表达「归档」语义而非「删除」。因是纯图标按钮，状态反馈改用 `title` 悬浮提示（含「可从归档区恢复」）+ 图标变红（`.armed` 类）表达「二次确认」，不再用文字。

### 状态徽章（语义、浅色底 + 高对比文字）

| 状态 | 背景 | 文字 |
|---|---|---|
| `starting` | `#e5edff` | `#1e40af` |
| `ready` | `#dcfce7` | `#166534` |
| `error` | `#fee2e2` | `#b91c1c` |
| `missing-dsh` | `#fef3c7` | `#92400e` |
| `restarting` | `#f3e8ff` | `#6d28d9` |

> 注意：状态徽章沿用冷色调家族（蓝 / 紫），仅 `ready`/`error` 用绿 / 红表达语义，避免引入额外色相。

### 输入框

- 背景 `#ffffff`，边框 `1px solid #d1d5db`，文字 `#1f2937`
- 占位符 `#9ca3af`
- `:focus` → 边框 `var(--accent)` + 焦点环 `0 0 0 3px rgba(59,108,246,0.15)`

---

## 已落地位置

| 文件 | 应用 |
|---|---|
| `ui/index.html` | `:root` 定义全部 token；页面背景渐变光晕；卡片、状态徽章、输入框、主按钮 |
| `src-tauri/src/main.rs` (`INJECT_JS`) | 注入的侧边栏「归档所有对话」按钮为**纯图标按钮（归档盒）**，样式与 dsh 原生图标按钮一致（28×28 圆形、无边框、透明底，颜色 / hover 复用 dsh 的 `--dsw-alias-*` 变量）；锚定到「搜索会话」图标按钮（`aria-label`），插入在其**左侧**（宽模式插到 searchSlot 前、折叠 rail 插到搜索框前） |

> Tauri 窗口未设 `backgroundColor`，webview 默认白底，与浅色主题一致，无需改动。

---

## 反模式（不要做）

- ❌ 深色背景（`#0f1115` / `#171a21` 之类）——本启动器自有界面一律浅色。
- ❌ 扁平纯蓝按钮（`#3b82f6`）——主按钮用蓝紫渐变，次级按钮用浅灰幽灵边框（`#e5e7eb`）。
- ❌ 在自有界面引入 DeepSeek Harness 没有的强调色（如橙、青）。
- ❌ 硬阴影 / 重边框——保持柔和、克制。
