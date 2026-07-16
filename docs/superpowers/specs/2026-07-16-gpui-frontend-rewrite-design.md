# SSH Router GPUI 原生 GUI 重写设计

## 背景

SSH Router 当前使用 React、Vite 和 Tauri v2 实现 Windows 托盘 GUI。路由 CLI、共享配置模型和多数系统操作已经使用 Rust 实现，但桌面应用仍依赖 WebView、Node.js 前端工具链和 Tauri 运行时。

本次重写以纯 Rust + GPUI 替换整个 Tauri GUI 应用，不仅替换 React 组件。现有 `ssh-router-cli` 的 SSH 路由行为保持不变。

## 目标

- 使用 GPUI 实现 Windows 原生桌面界面。
- 移除 React、Vite、Node.js、WebView 和 Tauri 运行时依赖。
- 完整保留现有 GUI 功能与用户数据兼容性。
- 将桌面业务逻辑与 GPUI 展示层分离，便于测试和持续优化。
- 首先交付功能完整的 MVP，再根据持续测量结果优化性能。

## 非目标

- 不重写 `ssh-router-cli` 的路由、临时文件和 Win32 进程管理逻辑。
- 不修改 `ssh-router.json` 的格式、存储路径或生效方式。
- 不增加 macOS 或 Linux 桌面支持。
- MVP 不设立硬性性能发布门槛。
- 不在 MVP 中扩展新的路由功能或配置字段。
- 不长期维护 Tauri 和 GPUI 两套 GUI。

## 已确认决策

- 完全移除 Tauri、React 和 Node.js，使用纯 Rust + GPUI。
- 采用一次性替换，不保留可运行的旧版 GUI。
- 新建 GPUI 应用 crate，并抽取独立的桌面业务核心 crate。
- 旧实现仅通过 Git 历史回退。
- MVP 以功能等价为发布条件；性能数据只记录，不阻塞发布。

## 总体架构

```text
ssh-router
├── crates/config          共享 Config、Route 和配置校验
├── crates/cli             sshd 启动的轻量路由程序
├── crates/app-core        无 UI 的桌面业务与 Windows 系统服务
│   ├── config_service     配置加载、保存和默认配置
│   ├── status_service     CLI、DefaultShell、配置和 sshd 状态
│   ├── elevation_service  UAC 提权操作
│   ├── update_service     应用更新和 CLI 版本同步
│   └── platform           Windows API 与进程边界
└── crates/gpui-app        GPUI 原生桌面应用
    ├── app_state          顶层状态和任务协调
    ├── views              主窗口和功能区域
    ├── components         表格、表单、弹窗和通知
    ├── actions            键盘、菜单和托盘事件
    └── theme              颜色、间距、字体和样式
```

`gpui-app` 不直接访问文件系统、注册表、Windows 服务或更新网络端点。所有外部操作通过 `app-core` 提供的接口完成。

## Crate 职责

### `crates/config`

- 继续提供 `Config` 和 `Route`。
- 集中实现与 UI 无关的配置校验：端口范围、端口唯一性、字段必填和恰好一条默认路由。
- 保持现有 JSON camelCase 格式兼容。

### `crates/cli`

- 保持当前职责和运行行为。
- 不依赖 GPUI 或 `app-core`。
- 继续与 GUI 通过 `C:\ProgramData\ssh\ssh-router.json` 解耦。

### `crates/app-core`

- 承接当前 `src-tauri` 中可复用的配置、状态检查、CLI 部署、提权和更新逻辑。
- 将 Windows 特定调用封装在 `platform` 模块中。
- 返回结构化结果和错误，不包含通知、弹窗或组件状态。
- 为可测试的业务流程提供接口边界；测试中可替换文件、注册表、服务、进程和更新实现。

### `crates/gpui-app`

- 管理 GPUI 应用生命周期、主窗口、托盘和单实例行为。
- 将用户事件转换为 `app-core` 调用。
- 管理加载、成功、失败、取消和进度状态。
- 不复制 `app-core` 或 `config` 中的业务规则。

## GPUI 状态模型

顶层使用 `Entity<AppState>`，具体功能拆分为独立 Entity：

- `AppState`：窗口可见性、全局通知、子 Entity 引用和应用级 action。
- `ConfigState`：已保存配置、编辑草稿、脏状态和保存任务。
- `SystemStatusState`：安装状态、刷新时间和刷新任务。
- `UpdateState`：检查结果、下载进度和安装阶段。
- `RouteEditorState`：表单字段、校验错误和当前编辑目标。

状态更新遵守以下约束：

- 同一业务操作的相关字段在一次 Entity update 中完成。
- 状态变化后只通知受影响的 Entity。
- `render()` 只根据当前状态构造元素，不执行 I/O 或修改状态。
- Entity 反向引用使用 `WeakEntity`，避免引用环。
- UI 文本优先使用 `SharedString`，避免不必要的字符串复制。
- 订阅和可取消任务保存在对应状态对象中。

## 界面组成

MVP 保持现有信息架构和功能范围：

- `StatusPanel`：显示 CLI 部署、CLI 版本、DefaultShell、配置文件和 sshd 状态。
- `QuickActions`：安装或更新 CLI、设置 DefaultShell、重启 sshd。
- `RouteTable`：展示路由、默认标记以及编辑和删除入口。
- `RouteEditor`：添加或编辑路由并显示字段级校验错误。
- `SftpEditor`：编辑全局 SFTP 命令。
- `UpdatePanel`：检查、下载、安装应用更新。
- `CliSyncDialog`：GUI 与已部署 CLI 版本不一致时提示同步。
- `ToastHost`：统一展示成功、警告和非阻塞错误。

不要求像素级复刻现有 Web UI，但必须保持功能位置清晰、Windows 桌面交互一致，并支持键盘焦点和常用操作。

## 数据流

```text
托盘、窗口或组件事件
    → AppState action
    → app-core service
    → 后台执行文件、注册表、服务、进程或网络操作
    → 返回结果、错误或进度
    → 更新对应 Entity
    → 局部 cx.notify()
    → GPUI 重绘受影响区域
```

- 文件、网络和 Windows 系统操作不得在 UI 线程执行。
- CPU 密集或阻塞调用通过后台执行器运行。
- 更新下载进度需要节流，避免每个数据块都触发重绘。
- 重复操作需要复用或取消现有任务，避免并发执行相同系统操作。

## 关键交互行为

- 启动时先完成托盘和可激活窗口的最小初始化，保持现有默认隐藏行为；配置加载和系统状态刷新在后台执行，用户打开窗口时不得等待这些操作完成。
- 路由编辑和删除只修改内存草稿；“保存配置”才写入磁盘。
- 保存前统一执行配置校验。
- 关闭主窗口时隐藏到托盘；托盘“退出”才结束应用。
- 第二个实例启动时激活已有窗口，不创建第二套状态。
- 检测到已部署 CLI 版本落后时显示同步提示；取消后可从快捷操作重试。
- 更新安装前检查是否存在未保存配置，防止编辑内容丢失。
- 提权取消视为正常用户取消，不显示为程序故障。

## 托盘与单实例

GPUI 不负责的 Windows 壳层能力由 `app-core::platform` 或应用启动层实现：

- 使用 Windows 原生托盘 API 创建图标和菜单。
- 托盘菜单提供“打开主界面”和“退出”。
- 双击托盘图标激活主窗口。
- 使用命名互斥量或等价机制保证单实例。
- 第二实例通过 Windows 消息或 IPC 通知已有实例显示并聚焦窗口。

该实现不得依赖 Tauri 插件。

## 自动更新

Tauri updater 将被 `app-core::update_service` 替代：

- MVP 继续读取现有 GitHub Releases `latest.json` 更新源，并保持与当前 Tauri v2 updater 元数据字段兼容。
- 继续使用现有 minisign 公钥验证 NSIS 安装包签名；CI 使用现有加密私钥生成签名和 `latest.json`，客户端不接受只有摘要而没有有效签名的安装包。
- 支持启动后检查、每日检查和手动检查。
- 支持展示版本号、更新说明和下载进度。
- 下载产物必须验证签名或可信摘要后才能安装。
- 安装程序启动成功后再退出当前应用。
- 网络失败不影响配置和托盘功能。

若未来更换元数据或签名协议，必须通过单独设计迁移，不能作为本次 GPUI 重写的隐含变更。

## 错误处理

`app-core` 使用结构化错误类型，至少区分：

- 配置读取、解析、校验和保存错误。
- 权限不足和 UAC 取消。
- CLI 部署或版本读取错误。
- 注册表与 Windows 服务错误。
- 网络、更新元数据、下载和签名验证错误。
- 单实例、托盘或窗口初始化错误。

UI 将错误映射为可执行的提示，例如重试、以管理员权限执行或检查配置。禁止使用 `unwrap()` 处理可恢复错误，也不得静默丢弃异步错误。

## MVP 范围

首个 GPUI 版本必须完成：

1. GPUI 主窗口、托盘和单实例。
2. 配置加载、默认配置创建、路由编辑、校验和保存。
3. 安装状态检查和快捷操作。
4. CLI 安装、升级与版本同步。
5. 自动更新检查、提示、安装和重启。
6. 基础通知、错误提示和加载状态。
7. 移除 React、Vite、Node.js、Tauri 源码、依赖和构建流程。
8. Windows 构建、安装和基本冒烟测试。
9. 添加可重复的性能记录工具。

## 性能策略

MVP 不设置绝对或相对性能门槛，但必须记录以下指标：

- 冷启动到窗口可交互时间。
- 空闲常驻内存。
- 常用操作响应时间。
- 安装包体积。
- 托盘空闲时 CPU 占用。

性能测量需使用固定 Windows 测试环境、固定操作步骤和多次采样，并保存旧 Tauri 版本的基线。首个版本只要求没有明显 UI 线程阻塞、持续高 CPU 或严重交互卡顿。

后续迭代根据测量结果依次处理：

1. 启动关键路径和延迟初始化。
2. Entity 粒度和无效重绘。
3. 字符串、图标和列表分配。
4. 更新与状态检查任务调度。
5. 空闲后台工作和托盘生命周期。
6. 二进制与安装包体积。

## 测试策略

### 单元测试

- `config`：序列化兼容、端口规则、字段校验和默认路由约束。
- `app-core`：使用替代实现测试配置、状态、提权、CLI 同步和更新状态机。
- `gpui-app`：使用 GPUI 测试工具验证状态转换、表单校验、弹窗和 action。

### Windows 集成测试

- 配置文件读取和提权保存。
- CLI 部署和版本判断。
- DefaultShell 注册表检查与设置。
- sshd 服务状态与重启。
- 托盘、窗口隐藏恢复和单实例激活。
- 更新元数据、签名验证和安装程序启动。

### 发布冒烟测试

- 首次启动并创建默认配置。
- 添加、编辑、删除和保存路由。
- 执行三项快捷操作。
- 同步 CLI。
- 检查并安装测试更新。
- 关闭到托盘、恢复窗口并退出。

## 构建与发布

- Cargo workspace 加入 `app-core` 和 `gpui-app`。
- Windows 构建产出 GUI exe、CLI exe 和安装包。
- 安装包继续部署 GUI、内置 CLI 资源和更新所需元数据。
- 删除 npm 安装和 Vite 构建步骤。
- CI 必须在 Windows 上执行构建、测试和安装包生成。
- 版本号继续在相关 Cargo package 和发布流程中保持一致。

## 移除旧实现的条件

由于采用一次性替换，旧实现会在同一开发分支中删除，但删除前必须完成以下迁移核对：

- 每个 Tauri command 都有对应的 `app-core` 能力。
- 每个 React 页面功能都有对应 GPUI 组件或交互。
- 托盘、单实例、提权和更新均有替代实现。
- Windows 冒烟测试通过。
- 现有配置文件可直接加载和保存。

删除范围包括 `src/`、前端测试、`package.json`、Vite/TypeScript/Tailwind 配置、`src-tauri` 的 Tauri 壳层代码以及不再使用的 Tauri 依赖。可复用 Rust 逻辑迁移到新 crate 后再删除原文件。

## 主要风险与应对

### GPUI Windows 能力与版本变化

在实施开始时锁定经过验证的 GPUI revision，并先验证窗口、输入、焦点、文本渲染和打包。避免在 MVP 中依赖非必要的实验 API。

### 托盘和单实例缺少框架插件

使用窄接口封装 Windows 原生实现，并为消息处理和生命周期增加集成测试。

### 更新机制安全退化

迁移前明确现有发布产物、签名和安装流程。没有真实性验证的更新实现不得进入发布版本。

### 一次性替换导致回归集中

按能力逐项迁移并维护功能核对表；每项能力在删除旧代码前通过测试。Git 历史作为整体回退路径。

### UI 与系统操作互相阻塞

所有外部操作从设计上进入后台执行器，状态结果再回到对应 Entity；代码审查检查 render 纯度和阻塞调用位置。

## 完成标准

- GPUI 应用覆盖当前 Tauri GUI 的全部用户功能。
- 现有配置和已部署 CLI 无需迁移即可继续使用。
- React、Node.js、WebView 和 Tauri 不再出现在运行时或构建链中。
- Windows 自动化测试和发布冒烟测试通过。
- 性能基线可以重复采集并保存结果。
- 应用不存在已知的 UI 线程阻塞、持续高 CPU 或阻断使用的严重缺陷。
