> [!NOTE]
> **本仓库为个人 Fork，仅供学习与个人使用。**
> 原始项目：[jlcodes99/cockpit-tools](https://github.com/jlcodes99/cockpit-tools)
> 本 Fork 在原版基础上针对个人使用习惯进行了若干定制修改，详见下方"个人修改记录"。

---

# Cockpit Tools

[English](README.en.md) · 简体中文

[![GitHub stars](https://img.shields.io/github/stars/jlcodes99/cockpit-tools?style=flat&color=gold)](https://github.com/jlcodes99/cockpit-tools)
[![GitHub downloads](https://img.shields.io/github/downloads/jlcodes99/cockpit-tools/total?style=flat&color=blue)](https://github.com/jlcodes99/cockpit-tools/releases)
[![GitHub release](https://img.shields.io/github/v/release/jlcodes99/cockpit-tools?style=flat)](https://github.com/jlcodes99/cockpit-tools/releases)
[![GitHub issues](https://img.shields.io/github/issues/jlcodes99/cockpit-tools)](https://github.com/jlcodes99/cockpit-tools/issues)

一款**通用的 AI IDE 账号管理工具**，目前支持 **Antigravity**、**Codex**、**GitHub Copilot**、**Windsurf** 和 **Kiro**，并支持多账号多实例并行运行。

> 本工具旨在帮助用户高效管理多个 AI IDE 账号，支持一键切换、配额监控、自动唤醒与多开实例并行运行，助您充分利用不同账号的资源。

**功能**：一键切号 · 多账号管理 · 多开实例 · 配额监控 · 唤醒任务 · 设备指纹 · 插件联动 · GitHub Copilot 管理 · Windsurf 管理 · Kiro 管理

**语言**：支持 16 种语言

🇺🇸 English · 🇨🇳 简体中文 · 繁體中文 · 🇯🇵 日本語 · 🇩🇪 Deutsch · 🇪🇸 Español · 🇫🇷 Français · 🇮🇹 Italiano · 🇰🇷 한국어 · 🇧🇷 Português · 🇷🇺 Русский · 🇹🇷 Türkçe · 🇵🇱 Polski · 🇨🇿 Čeština · 🇸🇦 العربية · 🇻🇳 Tiếng Việt

---

## 功能概览

### 1. 仪表盘 (Dashboard)

全新的可视化仪表盘，为您提供一站式的状态概览：

- **五平台支持**：同时展示 Antigravity、Codex、GitHub Copilot、Windsurf 与 Kiro 的账号状态
- **配额监控**：实时查看各模型剩余配额、重置时间
- **快捷操作**：一键刷新、一键唤醒
- **可视化进度**：直观的进度条展示配额消耗情况

> ![Dashboard Overview](docs/images/dashboard_overview.png)

### 2. Antigravity 账号管理

- **一键切号**：一键切换当前使用的账号，无需手动登录登出
- **多种导入**：支持 OAuth 授权、Refresh Token、插件同步
- **唤醒任务**：定时唤醒 AI 模型，提前触发配额重置周期
- **设备指纹**：生成、管理、绑定设备指纹，降低风控风险

> ![Antigravity Accounts](docs/images/antigravity_list.png)
>
> *(唤醒任务与设备指纹管理)*
> ![Wakeup Tasks](docs/images/wakeup_detail.png)
> ![Device Fingerprints](docs/images/fingerprint_detail.png)

#### 2.1 Antigravity 多开实例

支持同一平台多账号多实例并行运行。比如同时打开两个 Antigravity，分别绑定不同账号，分别处理不同项目，互不影响。

- **独立账号**：每个实例绑定不同账号并独立运行
- **并行项目**：多实例同时处理不同任务/项目
- **参数隔离**：支持自定义实例目录与启动参数

> ![Antigravity Instances](docs/images/antigravity_instances.png)

### 3. Codex 账号管理

- **专属支持**：专为 Codex 优化的账号管理体验
- **配额展示**：清晰展示 Hourly 和 Weekly 配额状态
- **计划识别**：自动识别账号 Plan 类型 (Basic, Plus, Team 等)

> ![Codex Accounts](docs/images/codex_list.png)

#### 3.1 Codex 多开实例

Codex 同样支持多账号多实例并行运行。比如同时打开两个 Codex，分别绑定不同账号，分别处理不同项目，互不影响。

- **独立账号**：每个实例绑定不同账号并独立运行
- **并行项目**：多实例同时处理不同任务/项目
- **参数隔离**：支持自定义实例目录与启动参数

> ![Codex Instances](docs/images/codex_instances.png)

### 4. GitHub Copilot 账号管理

- **账号导入**：支持 OAuth 授权、Token/JSON 导入
- **配额视图**：展示 Inline Suggestions / Chat messages 使用情况与重置时间
- **订阅识别**：自动识别 Free / Individual / Pro / Business / Enterprise 等计划类型
- **批量管理**：支持标签与批量操作

#### 4.1 GitHub Copilot 多开实例

基于 VS Code 的 Copilot 多实例管理，支持独立配置与生命周期控制。

- **独立配置**：每个实例拥有独立的用户目录
- **快速启停**：一键启动/停止/强制关闭实例
- **窗口管理**：支持打开实例窗口与批量关闭

### 5. Windsurf 账号管理

- **账号导入**：支持 OAuth 授权、Token/JSON 导入与本地导入
- **配额视图**：展示 Plan、User Prompt credits、Add-on prompt credits 与周期信息
- **批量管理**：支持标签与批量操作
- **切号注入**：支持切号后注入并启动 Windsurf

#### 5.1 Windsurf 多开实例

支持 Windsurf 多实例管理，支持独立配置与生命周期控制。

- **独立配置**：每个实例拥有独立的用户目录
- **快速启停**：一键启动/停止/强制关闭实例
- **窗口管理**：支持打开实例窗口与批量关闭

### 6. Kiro 账号管理

- **账号导入**：支持 OAuth 授权、Token/JSON 导入与本地导入
- **配额视图**：展示 Plan、User Prompt credits、Add-on prompt credits 与周期信息
- **批量管理**：支持标签与批量操作
- **切号注入**：支持切号后注入并启动 Kiro

#### 6.1 Kiro 多开实例

支持 Kiro 多实例管理，支持独立配置与生命周期控制。

- **独立配置**：每个实例拥有独立的用户目录
- **快速启停**：一键启动/停止/强制关闭实例
- **窗口管理**：支持打开实例窗口与批量关闭

### 7. 通用设置

- **个性化设置**：主题切换、语言设置、自动刷新间隔

> ![Settings](docs/images/settings_page.png)

---

## 安全性与隐私（简明版）

下面是最关心的几个问题，尽量用直白语言说明：

- **这是本地桌面工具**：不需要单独注册平台账号，也不依赖项目自建云端来存你的账号列表。
- **数据主要保存在本机**：
  - `~/.antigravity_cockpit`：Antigravity 账号、配置、WebSocket 状态等
  - `~/.codex`：Codex 官方当前登录 `auth.json`
  - 系统本地应用数据目录下 `com.antigravity.cockpit-tools`：Codex / GitHub Copilot / Windsurf / Kiro 多账号索引等
- **WebSocket 默认仅本机访问**：监听 `127.0.0.1`，默认端口 `19528`，可在设置中关闭或改端口。
- **什么时候会联网**：OAuth 登录、Token 刷新、配额查询、版本更新检查等官方接口请求。
- **实用安全建议**：
  1. 不使用插件联动时，可关闭 WebSocket 服务。
  2. 不要把用户目录直接打包分享；备份前注意脱敏 token 文件。
  3. 在公共或共用电脑上，使用后删除账号并退出应用。

## 设置项说明（小白版）

如果你只想“能用、稳定、不折腾”，优先按“推荐值”设置即可。

### 通用设置

| 设置项 | 这是做什么的（通俗） | 推荐值 | 什么时候改 |
| --- | --- | --- | --- |
| 显示语言 | 改界面文字语言 | 你最熟悉的语言 | 只在看不懂时改 |
| 应用主题 | 改亮色/暗色外观 | 跟随系统 | 长时间夜间使用可改深色 |
| 窗口关闭行为 | 点关闭按钮后的动作 | 每次询问 | 想后台常驻选“最小化到托盘” |
| Antigravity 自动刷新配额 | 后台定时更新 Antigravity 配额 | 5~10 分钟 | 账号多、想更实时可改 2 分钟 |
| Codex 自动刷新配额 | 后台定时更新 Codex 配额 | 5~10 分钟 | 同上 |
| GitHub Copilot 自动刷新配额 | 后台定时更新 GitHub Copilot 配额 | 5~10 分钟 | 同上 |
| Windsurf 自动刷新配额 | 后台定时更新 Windsurf 配额 | 5~10 分钟 | 同上 |
| Kiro 自动刷新配额 | 后台定时更新 Kiro 配额 | 5~10 分钟 | 同上 |
| 数据目录 | 存账号与配置文件的位置 | 默认即可 | 仅用于排查、备份 |
| Antigravity/Codex/VS Code/Windsurf/Kiro/OpenCode 启动路径 | 指定应用可执行文件位置 | 留空（自动检测） | 自动检测失败、或你装在自定义路径时 |
| 切换 Codex 时自动重启 OpenCode | 切换 Codex 后自动同步 OpenCode 账号信息 | 使用 OpenCode 就开启；不用就关闭 | 频繁切号且需要 OpenCode 同步时开启 |

补充说明：
- 自动刷新间隔越小，请求越频繁；若你更关注稳定，间隔可适当拉大。
- 当启用“配额重置唤醒”相关任务时，部分刷新间隔会有最小值限制（界面会提示）。

### 网络服务设置

| 设置项 | 这是做什么的（通俗） | 推荐值 | 风险/注意点 |
| --- | --- | --- | --- |
| WebSocket 服务 | 给本机插件/客户端实时通信用 | 不用插件联动就关闭 | 开启后仍是本机 `127.0.0.1` 访问 |
| 首选端口 | WebSocket 监听端口 | 默认 `19528` | 若端口冲突可改，保存后需重启应用 |
| 当前运行端口 | 实际已使用端口 | 只读查看 | 配置端口被占用时会自动回退到其它端口 |

### 三套推荐配置（直接抄）

1. **稳定省心**：自动刷新 10 分钟 + WebSocket 关闭（不用插件时）+ 路径保持默认。  
2. **高频切号**：自动刷新 2~5 分钟 + 需要联动时开启 WebSocket + OpenCode 联动开启。  
3. **安全优先**：WebSocket 关闭 + 不共享用户目录 + 定期清理不再使用的账号。  

---

---

## 安装指南 (Installation)

### 选项 A: 手动下载 (推荐)

前往 [GitHub Releases](https://github.com/jlcodes99/cockpit-tools/releases) 下载对应系统的安装包：

*   **macOS**: `.dmg` (Apple Silicon & Intel)
*   **Windows**: `.msi` (推荐) 或 `.exe`
*   **Linux**: `.deb` (Debian/Ubuntu) 或 `.AppImage` (通用)

### 选项 B: Homebrew 安装 (macOS)

> 需要先安装 Homebrew。

```bash
brew tap jlcodes99/cockpit-tools https://github.com/jlcodes99/cockpit-tools
brew install --cask cockpit-tools
```

如果遇到 macOS “应用已损坏”或无法打开，也可以使用 `--no-quarantine` 安装：

```bash
brew install --cask --no-quarantine cockpit-tools
```

如果提示已存在应用（例如：`already an App at '/Applications/Cockpit Tools.app'`），请先删除旧版本再安装：

```bash
rm -rf "/Applications/Cockpit Tools.app"
brew install --cask cockpit-tools
```

或者直接强制覆盖安装：

```bash
brew install --cask --force cockpit-tools
```

### 🛠️ 常见问题排查 (Troubleshooting)

#### macOS 提示“应用已损坏，无法打开”？
由于 macOS 的安全机制，非 App Store 下载的应用可能会触发此提示。您可以按照以下步骤快速修复：

1.  **命令行修复** (推荐):
    打开终端，执行以下命令：
    ```bash
    sudo xattr -rd com.apple.quarantine "/Applications/Cockpit Tools.app"
    ```
    > **注意**: 如果您修改了应用名称，请在命令中相应调整路径。

2.  **或者**: 在“系统设置” -> “隐私与安全性”中点击“仍要打开”。

---

## 开发与构建

### 前置要求

- Node.js v18+
- npm v9+
- Rust（Tauri 运行时）

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
npm run tauri dev
```

### 构建产物

```bash
npm run tauri build
```

---

## ☕ 赞助项目

如果不介意，请 [☕ 赞赏支持一下](docs/DONATE.md)

您的每一份支持都是对开源项目最大的鼓励！无论金额大小，都代表着您对这个项目的认可。

---

## 致谢

- Antigravity 账号切号逻辑参考：[Antigravity-Manager](https://github.com/lbjlaq/Antigravity-Manager)

感谢项目作者的开源贡献！如果这些项目对你有帮助，也请给他们点个 ⭐ Star 支持一下！

---

## 🛠️ 个人修改记录（基于 Fork 的定制内容）

以下为本 Fork 相对于原始仓库所做的全部定制改动，按功能模块归类。
部分功能经历了多次迭代或撤销，已在对应条目中标注。

---

### 🔄 配额自动刷新

- **新增**配额自动刷新可选模式：`当前账号` / `全部账号`（后重构为全局配置，对所有工具生效，并在 QuickSettings 中展示）
- **新增** AGY 智能刷新模式：优先刷新当前账号 + 候选账号，减少无效请求
- **重构**刷新模式为"额外刷新账号数"（`extra_refresh_count`）配置项，替代原 `auto_refresh_mode` 枚举
- **新增** 窗口不可见时跳过刷新
- **新增** 用户空闲超过 10 分钟时自动跳过定时刷新（Win32 `GetLastInputInfo`）
- **新增** 定时刷新对齐时钟边界（`setTimeout` 自调度替代 `setInterval`）
- **修复** `handleConfigUpdate` 添加 500ms 防抖，防止 `config-updated` 级联触发定时器多次重建
- **新增** 新增托盘区刷新配置项（默认不刷新），文案优化为「最小化到托盘时继续刷新」
- **修复** 托盘区刷新设置逻辑反转 bug
- **修复** 解绑定时器重建与唤醒任务配置修正逻辑，解决防抖失效导致的自动刷新定时器级联多次重建问题
- **新增** 切换账号时先刷新当前（退出）账号的配额
- **新增** 账号页全部刷新时仅刷新选中账号

---

### 🔀 智能切号与排序

- **新增** `Ctrl+F1` 全局热键智能切号（**后调整为 `Alt+F1`**，同时新增 `Ctrl+Z` 应用内热键复用相同逻辑）
- **新增** 左侧导航栏小灯按钮点击触发智能切号（与 `Alt+F1` 逻辑一致）
- **新增** `Alt+S` 应用内热键支持
- **新增** 全局热键支持窗口三态切换（最小化 / 还原 / 置顶）
- **修复** 热键切号后 UI 不更新当前账号的问题
- **修复** 热键切号（`Ctrl+F1` 阶段）时补充 `usage_count` 维护逻辑
- **修复** `Ctrl+F1` 切号额度充足时切当前账号重启；筛选候选及额外刷新时排除 UNKNOWN 等级账号
- **提取** 统一选号逻辑 `sort_candidates_by_best` 公共函数，新增 `find_suggested_account` 后端接口，前端改为调用后端统一维护
- **优化** 智能切号选号策略：改为按余额从高到低排序选候选，不再要求必须满额
- **新增** 点击当前账号切换按钮时自动选择建议账号（目标 == 当前账号时，自动选最优可用账号）
- **新增** 自动切号弹窗确认功能，支持通过配置开关控制（**后 revert 撤销**：移除自动选择建议账号和批量刷新跳过已重置账号的功能）
- **改进** 选号策略：按 Claude 重置时间排序，取第一个 Claude 配额 ≥ 50% 的账号
- **优化** 自动切号选号策略：并列时优先选最快重置的账号，`reset_time` 为空时回退到 `last_used`
- **新增** 切号额度优先配置（最大优先 / 最小优先）
- **新增** Antigravity 切号改为优雅关闭：先发送 `WM_CLOSE` 等待 10 秒保存会话，超时后再强杀
- **修复** 多窗口场景优雅关闭问题，等待期内周期重发 `WM_CLOSE`
- **新增** AG 账号按余额排序时，当前账号配额等效 100% 且次排序优先

---

### 📋 排序规则配置

- **重构** 排序逻辑：去除 `last_updated` 为第一排序键，统一 `created_at` 排序方向由配置控制
- **增加** 额外刷新账号排序模式选项（新账号优先 / 旧账号优先）并修复 `last_updated` 容差单位不匹配 Bug
- **新增** `last_updated` 60 秒内视为相同优先级，按创建时间降序排序
- **重构** 清理 `sort_candidates_by_best` 及其 3 个调用点的弃用代码
- **重构** 切换账号设置 UI：新增重置时间排序选项及额度过滤
- **新增** 切号排序条件优先级列表，替换单一下拉框为可启用/排序的 4 条件列表
- **重构** 刷新排序复用 `switch_sort_rules` 中 `created_at` 方向值，删除自动切号描述
- **重构** 排序规则箭头改为水平并列的 `ChevronUp/ChevronDown` 图标按钮，SettingsPage 配置控件右对齐宽度 50%
- **新增** AG 账号按余额排序时，配额相同则按 `created_at` 次排序，方向由次排序优先配置控制

---

### 📊 配额监控与账号状态

- **新增** AG 账号卡片 footer 增加 `last_updated` 显示
- **新增** 已重置模型显示后附加已重置时长（如 `3.5H`）
- **修复** Antigravity 账号卡已重置模型显示后附加已重置时长
- **新增** AGY 过滤可疑重置时间功能，默认开启，可在设置中关闭
- **移除** 过滤可疑重置时间功能（`filter_suspicious_reset_time`）（后期重构时移除此功能）
- **修复** `update_account_quota` 写入前过滤可疑 `reset_time`（当前时间 +5H 内），保留旧值避免污染存储
- **迁移** 过滤可疑重置时间开关从 `localStorage` 迁移到后端 `UserConfig`
- **新增** 账号使用计数功能，切换时自动记录配额低于 60% 的消耗次数
- **填补** 修复 `switch_account` 中未定义变量 `updated_account` 的编译错误
- **删除** `last_used` 字段

---

### 🏷️ 分组、标签与隐私

- **修复** Group 分组状态持久化
- **新增** 标签筛选勾选/取消勾选分组时联动账号卡片选中状态
- **新增** 标签过滤忽略末尾数字（`xx1/xx2` 归并为 `xx`）
- **新增** 分组管理新增隐藏分组功能，账号卡片过滤已隐藏分组的频道信息
- **调整** 隐私模式下的邮箱码规则，按用户需求修改
- **新增** 隐私模式超限账号改为独立分组显示而非直接隐藏（**后 refactor 撤销**：移除隐私模式超限分组功能，仅保留邮箱脱敏）
- **新增** 账号列表显示用户名时只显示 `@` 之前的部分
- **修复** 四处代码质量缺陷

---

### 🎨 UI 界面优化

- **新增** 禁用/启用账号按钮常驻显示于卡片工具栏，卡片 footer 时间与工具栏分行显示
- **修复** 卡片工具栏的禁用/启用按钮直接操作当前账号，不再依赖 selected 选中集合
- **重构** 简化使用消耗次数标签样式，改为普通标签并靠右显示
- **调整** 账号列表进度条颜色方案及打包配置
- **删除** 配额刷新模式和批量刷新跳过选项（AGY 自动刷新改为仅更新本地缓存时）
- **移除** 所有页面 CSS 中的 `fadeUp/fadeIn` 动画效果及 keyframes 定义
- **移除** 打砖块游戏彩蛋，保留灯按钮和点击计数器；清理 App.tsx 中所有游戏相关状态和回调；移除 SideNav 的 `hasBreakoutSession` prop；清理所有语言文件的 breakout 翻译；移除 components.css 的游戏专属样式

---

### ⏰ 唤醒任务

- **新增** 唤醒词支持 `|` 分隔多个选词，随机选取
- **新增** 配额重置唤醒增加间隔时长/唤醒数量/触发阈值配置，改进调度粒度与日志记录
- **新增** 唤醒成功后自动刷新账号配额状态

---

### 🌐 国际化

- **精简** 本地化仅保留简体中文和英文，中文为默认语言（移除其余 14 种语言）

---

### 🔧 后端与日志

- **简化** 日志前缀格式：由复杂模块路径改为 `YYYY-MM-DD HH:mm:ss [LEVEL]`

---

### 🔩 其他杂项

- **移除** 侧边栏最多只能勾选两个平台的限制
- **修复** `auto_refresh_mode` 字段缺失，更新依赖及 `.gitattributes`

---

## 许可证

[MIT](LICENSE)

---

## 免责声明

本项目仅供个人学习和研究使用。使用本项目即表示您同意：

- 不将本项目用于任何商业用途
- 承担使用本项目的所有风险和责任
- 遵守相关服务条款和法律法规

项目作者对因使用本项目而产生的任何直接或间接损失不承担责任。
