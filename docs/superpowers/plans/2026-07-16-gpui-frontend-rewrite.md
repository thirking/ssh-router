# SSH Router GPUI 原生 GUI 重写实施计划

**目标：** 使用纯 Rust + GPUI 一次性替换现有 React/Vite/Tauri GUI，保留全部桌面功能和配置兼容性，并建立后续性能优化基线。

**设计依据：** `docs/superpowers/specs/2026-07-16-gpui-frontend-rewrite-design.md`

**目标平台：** Windows x64，Rust stable，MSVC 工具链。

## 全局约束

- `crates/cli` 的 SSH 路由行为不在本次重写范围内。
- `C:\ProgramData\ssh\ssh-router.json` 格式与路径保持兼容。
- GPUI 版本必须锁定到经过 Windows 验证的 revision。
- UI 线程不得执行文件、注册表、服务、进程等待或网络操作。
- 可恢复错误不得使用 `unwrap()`，异步错误不得静默丢弃。
- 旧前端只在替代能力通过验证后删除，但最终提交中不保留双 GUI。
- 每个任务独立提交，提交描述使用中文 Conventional Commits。
- Cargo 命令使用 `-q`；Windows 专用验证在 Windows 环境执行。

## 任务 1：验证 GPUI Windows MVP 能力并锁定版本

**目的：** 在大量迁移前确认 GPUI 能满足窗口、输入、焦点和基本渲染要求。

**文件：**

- 修改：`Cargo.toml`
- 修改：`Cargo.lock`
- 新建：`crates/gpui-app/Cargo.toml`
- 新建：`crates/gpui-app/src/main.rs`
- 新建：`docs/performance/gpui-spike.md`

**步骤：**

- [ ] 将 `crates/gpui-app` 加入 workspace。
- [ ] 使用明确的 Git revision 或稳定版本引入 GPUI，禁止浮动分支。
- [ ] 创建最小 Windows 窗口，验证：
  - 文本渲染和中文字体。
  - 鼠标点击、键盘输入、Tab 焦点和窗口缩放。
  - 窗口隐藏、显示、聚焦和退出。
- [ ] 在 Windows x64 执行：

```powershell
cargo build -q -p ssh-router-gpui --target x86_64-pc-windows-msvc
cargo run -q -p ssh-router-gpui
```

- [ ] 在 `docs/performance/gpui-spike.md` 记录 GPUI revision、Windows 版本、验证结果和已知限制。
- [ ] 若窗口、中文输入或焦点存在阻断问题，停止后续任务并重新评估 GPUI revision，不用临时代码绕过核心缺陷。

**提交：**

```text
build: 建立 GPUI Windows 应用骨架
```

## 任务 2：建立 `app-core` 与统一错误模型

**目的：** 创建不依赖 Tauri 和 GPUI 的桌面业务层。

**文件：**

- 修改：`Cargo.toml`
- 修改：`Cargo.lock`
- 新建：`crates/app-core/Cargo.toml`
- 新建：`crates/app-core/src/app_core.rs`
- 新建：`crates/app-core/src/error.rs`
- 新建：`crates/app-core/src/platform.rs`
- 新建：`crates/app-core/src/platform/windows.rs`

**接口：**

```rust
pub enum AppError {
    Config(ConfigError),
    Permission(PermissionError),
    Cli(CliError),
    Registry(RegistryError),
    Service(ServiceError),
    Update(UpdateError),
    Platform(PlatformError),
}

pub type AppResult<T> = Result<T, AppError>;
```

**步骤：**

- [ ] 将 `crates/app-core` 加入 workspace，并通过 `[lib] path = "src/app_core.rs"` 声明库入口。
- [ ] 定义结构化错误及可展示消息，不直接依赖 GPUI。
- [ ] 为文件、注册表、Windows 服务、提权进程、资源路径和应用版本定义窄接口。
- [ ] 使用 `cfg(windows)` 隔离 Windows 实现；非 Windows 构建仅提供明确的 Unsupported 错误。
- [ ] 添加错误分类和错误来源测试。
- [ ] 验证：

```bash
cargo test -q -p ssh-router-app-core
cargo clippy -q -p ssh-router-app-core --all-targets -- -D warnings
```

**提交：**

```text
feat: 建立桌面业务核心与错误模型
```

## 任务 3：集中配置校验并迁移配置服务

**目的：** 将 React 层的默认路由校验和 Tauri command 配置读写迁移到 Rust。

**文件：**

- 修改：`crates/config/src/lib.rs`
- 新建：`crates/app-core/src/config_service.rs`
- 修改：`crates/app-core/src/app_core.rs`
- 测试参考：`src/App.tsx`、`src/components/RouteDialog.tsx`
- 迁移来源：`src-tauri/src/commands.rs`

**接口：**

```rust
pub fn validate_config(config: &Config) -> Result<(), ConfigValidationError>;

pub trait ConfigRepository {
    fn load(&self) -> AppResult<Config>;
    fn save(&self, config: &Config) -> AppResult<()>;
    fn create_default(&self) -> AppResult<Config>;
}
```

**步骤：**

- [ ] 在 `config` crate 增加端口唯一、端口范围、必填字段、扩展名和恰好一条默认路由校验。
- [ ] 为每条校验规则先补充单元测试。
- [ ] 迁移配置加载、JSON 序列化、直接写入和提权写入流程。
- [ ] 使用临时文件和原子替换，避免保存中断造成配置损坏。
- [ ] 保持 JSON camelCase 和旧配置兼容。
- [ ] 使用 fake repository 测试加载、创建默认配置、保存失败和权限升级路径。
- [ ] 验证：

```bash
cargo test -q -p ssh-router-config
cargo test -q -p ssh-router-app-core config
```

**提交：**

```text
feat: 迁移配置校验与读写服务
```

## 任务 4：迁移状态检查、CLI 同步和提权操作

**目的：** 替代全部非更新类 Tauri commands。

**文件：**

- 新建：`crates/app-core/src/status_service.rs`
- 新建：`crates/app-core/src/cli_service.rs`
- 新建：`crates/app-core/src/elevation_service.rs`
- 修改：`crates/app-core/src/app_core.rs`
- 迁移来源：`src-tauri/src/commands.rs`
- 迁移来源：`src-tauri/src/cli_sync.rs`
- 迁移来源：`src-tauri/src/elevate.rs`

**接口：**

```rust
pub struct SystemStatus {
    pub cli_deployed: bool,
    pub cli_up_to_date: bool,
    pub default_shell_set: bool,
    pub config_exists: bool,
    pub sshd_running: bool,
    pub cli_path: String,
    pub default_shell: String,
    pub sshd_detail: String,
}

pub trait SystemService {
    fn check_status(&self) -> AppResult<SystemStatus>;
    fn install_cli(&self) -> AppResult<OperationMessage>;
    fn set_default_shell(&self) -> AppResult<OperationMessage>;
    fn restart_sshd(&self) -> AppResult<OperationMessage>;
}
```

**步骤：**

- [ ] 无行为变化地迁移 `files_match` 及现有测试。
- [ ] 迁移 UAC 脚本、唯一临时文件、结果文件解析和清理逻辑。
- [ ] 将 UAC 取消映射为独立的 `PermissionError::Cancelled`。
- [ ] 迁移 CLI 资源定位、部署、DefaultShell 注册表操作和 sshd 服务操作。
- [ ] 将状态检查拆成可独立测试的 Windows provider。
- [ ] 补充临时文件冲突、部署文件缺失、注册表读取失败和服务不存在测试。
- [ ] Windows 集成验证三项快捷操作。

**提交：**

```text
feat: 迁移系统状态与提权操作
```

## 任务 5：实现 GPUI 应用状态、主题和基础组件

**目的：** 建立后续页面使用的 GPUI 组件与状态约定。

**文件：**

- 新建：`crates/gpui-app/src/app.rs`
- 新建：`crates/gpui-app/src/app_state.rs`
- 新建：`crates/gpui-app/src/theme.rs`
- 新建：`crates/gpui-app/src/components.rs`
- 新建：`crates/gpui-app/src/components/button.rs`
- 新建：`crates/gpui-app/src/components/text_input.rs`
- 新建：`crates/gpui-app/src/components/modal.rs`
- 新建：`crates/gpui-app/src/components/toast.rs`
- 新建：`crates/gpui-app/src/components/status_badge.rs`
- 修改：`crates/gpui-app/src/main.rs`

**步骤：**

- [ ] 定义 `AppState`、通知队列和应用级 action。
- [ ] 建立颜色、排版、间距、边框、焦点和禁用状态 token。
- [ ] 实现按钮、文本输入、复选/单选、模态框、Toast 和状态徽标。
- [ ] 所有交互组件支持键盘焦点、Enter/Space 激活和清晰焦点样式。
- [ ] `render()` 不修改状态；事件通过 `cx.listener()` 更新 Entity。
- [ ] 文本使用 `SharedString`；反向引用使用 `WeakEntity`。
- [ ] 使用 `#[gpui::test]` 测试 action、焦点、禁用状态和通知生命周期。

**提交：**

```text
feat: 建立 GPUI 状态与基础组件
```

## 任务 6：实现路由配置主流程

**目的：** 完成配置加载、编辑、删除、默认路由、SFTP 和保存功能。

**文件：**

- 新建：`crates/gpui-app/src/config_state.rs`
- 新建：`crates/gpui-app/src/route_editor_state.rs`
- 新建：`crates/gpui-app/src/views.rs`
- 新建：`crates/gpui-app/src/views/main_window.rs`
- 新建：`crates/gpui-app/src/views/route_table.rs`
- 新建：`crates/gpui-app/src/views/route_editor.rs`
- 新建：`crates/gpui-app/src/views/sftp_editor.rs`
- 对照：`src/App.tsx`
- 对照：`src/components/RouteTable.tsx`
- 对照：`src/components/RouteDialog.tsx`
- 对照：`src/components/SftpField.tsx`

**步骤：**

- [ ] `ConfigState` 分离已保存配置和编辑草稿，并维护 `is_dirty`。
- [ ] 为 Route 增加仅用于 UI 的稳定 ID，不写入 JSON。
- [ ] 实现加载中、配置不存在、配置损坏和加载成功状态。
- [ ] 实现添加、编辑、删除和设置默认路由。
- [ ] 表单即时显示字段错误；最终保存仍调用共享 `validate_config`。
- [ ] 配置不存在时提供创建默认配置操作。
- [ ] 保存成功更新基线并清除脏状态；保存失败保留草稿。
- [ ] 测试恰好一条默认路由、重复端口、取消编辑、删除后脏状态和保存失败。

**提交：**

```text
feat: 实现 GPUI 路由配置界面
```

## 任务 7：实现安装状态与快捷操作界面

**目的：** 替代 `StatusPanel`、`QuickActions` 和 CLI 同步弹窗。

**文件：**

- 新建：`crates/gpui-app/src/system_status_state.rs`
- 新建：`crates/gpui-app/src/views/status_panel.rs`
- 新建：`crates/gpui-app/src/views/quick_actions.rs`
- 新建：`crates/gpui-app/src/views/cli_sync_dialog.rs`
- 修改：`crates/gpui-app/src/views.rs`
- 修改：`crates/gpui-app/src/views/main_window.rs`
- 对照：`src/components/StatusPanel.tsx`
- 对照：`src/components/QuickActions.tsx`
- 对照：`src/components/CliSyncDialog.tsx`

**步骤：**

- [ ] 窗口初始化完成后异步刷新状态，不阻塞显示或激活。
- [ ] 同一时间只允许一个状态刷新任务。
- [ ] 快捷操作执行期间禁用对应按钮，并显示明确进度。
- [ ] 操作成功后刷新系统状态。
- [ ] CLI 已部署但版本不一致时每次启动只自动提示一次。
- [ ] UAC 取消显示中性提示，系统失败显示可重试错误。
- [ ] 使用 fake service 测试成功、失败、取消、重复点击和刷新顺序。

**提交：**

```text
feat: 实现安装状态与快捷操作界面
```

## 任务 8：实现 Windows 托盘、窗口生命周期和单实例

**目的：** 替代 Tauri tray-icon 与 single-instance 插件。

**文件：**

- 新建：`crates/app-core/src/platform/windows/tray.rs`
- 新建：`crates/app-core/src/platform/windows/single_instance.rs`
- 新建：`crates/gpui-app/src/window_controller.rs`
- 修改：`crates/gpui-app/src/app.rs`
- 修改：`crates/gpui-app/src/main.rs`
- 对照：`src-tauri/src/lib.rs`

**步骤：**

- [ ] 使用 Windows 原生通知区域 API 创建图标和菜单。
- [ ] 实现打开主界面、双击激活和退出事件。
- [ ] 窗口关闭事件改为隐藏到托盘。
- [ ] 使用命名互斥量建立单实例。
- [ ] 第二实例通过命名消息窗口或等价 IPC 通知已有实例显示并聚焦。
- [ ] 确保退出时释放托盘图标、消息窗口和互斥量。
- [ ] Windows 手工验证：隐藏、恢复、重复启动、资源管理器重启后的托盘恢复和正常退出。

**提交：**

```text
feat: 实现原生托盘与单实例
```

## 任务 9：实现独立更新服务和 GPUI 更新界面

**目的：** 替代 Tauri updater 和 process 插件，同时保持更新签名安全。

**文件：**

- 新建：`crates/app-core/src/update_service.rs`
- 新建：`crates/app-core/src/update_manifest.rs`
- 新建：`crates/app-core/src/signature.rs`
- 新建：`crates/gpui-app/src/update_state.rs`
- 新建：`crates/gpui-app/src/views/update_panel.rs`
- 新建：`crates/gpui-app/src/views/update_dialog.rs`
- 修改：`crates/gpui-app/src/views.rs`
- 修改：`crates/gpui-app/src/views/main_window.rs`
- 对照：`src/lib/update-manager.ts`
- 对照：`src/lib/tauri-update-adapter.ts`
- 对照：`src-tauri/tauri.conf.json`

**步骤：**

- [ ] 定义 `idle/checking/available/downloading/installing/error` 更新状态机。
- [ ] 解析现有 Tauri v2 `latest.json` 字段并验证版本比较。
- [ ] 使用内置 minisign 公钥验证下载的 NSIS 安装包签名。
- [ ] 无有效签名时禁止启动安装程序。
- [ ] 支持启动检查、24 小时间隔检查和手动检查。
- [ ] 自动检查失败不弹阻塞错误；手动检查失败需要明确提示。
- [ ] 对下载进度事件节流，避免高频 Entity 通知。
- [ ] 安装前检测未保存配置并要求用户确认。
- [ ] 被验证的安装程序成功启动后退出应用；启动失败时保持应用运行。
- [ ] 使用本地 HTTP fake 测试无更新、有更新、无效元数据、签名失败、下载失败和重复安装。

**提交：**

```text
feat: 实现签名更新服务与界面
```

## 任务 10：建立 GPUI Windows 构建和安装包流程

**目的：** 在删除 Tauri 前确保纯 Rust 应用可打包、安装和更新。

**文件：**

- 修改：`build.sh`
- 修改：`build.cmd`
- 新建：`installer/ssh-router.nsi`
- 新建：`scripts/package-update.ps1`
- 修改：`.github/workflows/release.yml`
- 修改：`README.md`

**步骤：**

- [ ] 构建 `ssh-router-cli.exe` 和 GPUI `ssh-router.exe`。
- [ ] 将 CLI 作为安装资源打入 NSIS 安装包。
- [ ] 保持安装路径、应用标识、开始菜单和卸载升级行为兼容。
- [ ] 生成与现有客户端字段兼容的 `latest.json`。
- [ ] 使用现有加密 minisign 私钥签名安装包；私钥不得写入磁盘日志或仓库。
- [ ] 更新 release workflow，移除 Node.js、npm 和 `tauri-action`。
- [ ] 发布后校验安装包、`.sig` 和 `latest.json`。
- [ ] 在干净 Windows VM 验证首次安装、覆盖升级、卸载和自动更新。

**提交：**

```text
build: 建立 GPUI Windows 发布流程
```

## 任务 11：建立性能基线记录工具

**目的：** 为 MVP 后的持续优化提供可重复数据，不设置发布门槛。

**文件：**

- 新建：`scripts/measure-startup.ps1`
- 新建：`scripts/measure-idle.ps1`
- 新建：`docs/performance/README.md`
- 新建：`docs/performance/baseline-template.md`
- 修改：`README.md`

**步骤：**

- [ ] 启动脚本记录进程创建到应用发出“窗口可交互”标记的时间。
- [ ] 空闲脚本记录固定观察期的 Working Set、Private Bytes 和 CPU。
- [ ] 文档规定 Windows 版本、硬件、电源模式、采样次数和操作步骤。
- [ ] 使用已发布的 v0.0.8 安装包记录旧 Tauri 基线，并记录首个 GPUI MVP 数据，避免依赖已被修改的本地旧构建链。
- [ ] 数据用于发现退化，不在 MVP CI 中设置失败阈值。

**提交：**

```text
perf: 建立桌面应用性能基线工具
```

## 任务 12：删除 Tauri、React 和 Node.js 实现

**目的：** 完成一次性替换，收敛为纯 Rust 构建链。

**删除：**

- `src/`
- `src-tauri/`
- `package.json`
- `package-lock.json`
- `index.html`
- `vite.config.ts`
- `tsconfig.json`
- `tsconfig.node.json`
- `components.json`
- Tailwind 和前端测试配置

**修改：**

- `Cargo.toml`
- `Cargo.lock`
- `.gitignore`
- `README.md`
- 构建与发布脚本

**步骤：**

- [ ] 建立功能映射表，确认每个 Tauri command 和 React 组件已有 GPUI 替代。
- [ ] 确认 GPUI 安装包、托盘、单实例、提权和更新冒烟通过。
- [ ] 删除旧源码和前端工具链。
- [ ] 清理 Tauri crate、插件、capability、图标副本和无用依赖。
- [ ] 将仍需使用的图标迁移到 `crates/gpui-app/assets/`。
- [ ] 确认仓库不再包含运行时或构建期的 Tauri、React、Vite、Node.js 依赖。
- [ ] 验证：

```bash
rg -n "tauri|react|vite|node_modules|npm" Cargo.toml crates .github build.sh build.cmd README.md
cargo test -q --workspace
```

允许 README 的历史迁移说明出现旧技术名称，其余命中必须逐项确认。

**提交：**

```text
refactor: 移除 Tauri 与 Web 前端实现
```

## 任务 13：完成 Windows 全量验证与 MVP 发布准备

**目的：** 确认功能等价、数据兼容和发布链完整。

**文件：**

- 新建：`docs/testing/gpui-mvp-checklist.md`
- 修改：`README.md`
- 按验证结果修改相关代码和测试

**自动验证：**

```powershell
cargo fmt --all -- --check
cargo clippy -q --workspace --all-targets -- -D warnings
cargo test -q --workspace
cargo build -q --workspace --release --target x86_64-pc-windows-msvc
```

**MVP 冒烟清单：**

- [ ] 首次启动保持隐藏到托盘，并可打开主窗口。
- [ ] 已有 `ssh-router.json` 可直接加载，保存后 CLI 可读取。
- [ ] 配置不存在时可创建默认配置。
- [ ] 添加、编辑、删除、默认路由和 SFTP 命令行为正确。
- [ ] CLI 状态、DefaultShell、配置文件和 sshd 状态显示正确。
- [ ] 安装 CLI、设置 DefaultShell、重启 sshd 均能完成或明确报告取消/失败。
- [ ] CLI 版本不一致提示和重试行为正确。
- [ ] 关闭窗口隐藏到托盘，退出菜单真正终止应用。
- [ ] 重复启动只激活已有实例。
- [ ] 自动更新、手动检查、签名失败和安装重启行为正确。
- [ ] 冷启动、空闲内存、操作响应、安装包和空闲 CPU 已记录。
- [ ] 无明显 UI 线程阻塞、持续高 CPU 或阻断使用的严重缺陷。

**提交：**

```text
test: 完成 GPUI MVP 发布验证
```

## 推荐执行顺序

```text
任务 1 GPUI 可行性关卡
  → 任务 2 app-core 骨架
  → 任务 3 配置服务
  → 任务 4 系统服务
  → 任务 5 GPUI 基础组件
  → 任务 6 路由配置 UI
  → 任务 7 状态与快捷操作 UI
  → 任务 8 托盘与单实例
  → 任务 9 自动更新
  → 任务 10 构建与安装包
  → 任务 11 性能基线
  → 任务 12 删除旧实现
  → 任务 13 全量验证
```

任务 5 可在任务 3、4 的核心接口稳定后开始；任务 8、9 可并行开发，但在删除旧实现前必须全部通过 Windows 验证。

## 完成定义

- 13 个任务全部完成并有对应提交。
- GPUI GUI 覆盖现有 Tauri GUI 的全部功能。
- 现有配置文件和已部署 CLI 无需迁移。
- 构建、测试、安装和更新只依赖 Rust、Windows SDK、NSIS 与签名工具。
- 仓库不再依赖 React、Vite、Node.js、WebView 或 Tauri。
- 性能基线已记录，后续优化可以对比版本变化。
