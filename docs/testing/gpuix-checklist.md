# gpuix 验证清单

## 自动验证

macOS arm64 上的 Rust 工作区测试、前端类型与状态测试、独立程序构建、620×760 和 520×440 原生交互验证均通过。Windows MSVC 目标的 Rust 构建及工作区 Clippy 检查通过；当前界面的 Windows 桌面实测仍待验收。

- `cargo test --workspace`：共享核心、CLI、更新工具和管道协议。
- `cargo clippy -p ssh-router-gpuix --all-targets -- -D warnings`：新增 Rust 代码。
- `cd apps/gpuix && bun run typecheck && bun test`：类型检查及草稿、互斥、取消和校验行为。
- `bun run smoke`：在原生窗口验证配置创建、搜索、重复端口提示、中文文本、草稿放弃确认、保存、删除取消和系统页面。
- `bun run build`：Rust Release 与 Bun 独立程序。

桌面测试使用临时配置。它不执行 CLI 部署、修改 DefaultShell、重启 sshd 或安装更新。通过 `SSH_ROUTER_SCREENSHOT_DIR` 可保存主窗口、编辑弹窗和系统页截图。

## Windows 实机验收

- [ ] 启动默认隐藏；托盘可恢复主窗口。
- [ ] 关闭只隐藏；二次启动激活已有窗口；托盘退出后 GUI 和业务进程均结束。
- [ ] 重启 Explorer 后托盘恢复。
- [ ] Tab、Shift+Tab、Enter、Space、Escape、Ctrl+S、Ctrl+N、Ctrl+F 正常，弹窗焦点不逸出。
- [ ] 中文 IME 完成组合、选词和提交，不重复、不丢字。
- [ ] 520×440 窗口和高 DPI 下表单、底部按钮与列表可滚动访问。
- [ ] CLI 部署、DefaultShell、重启 sshd 的成功、取消、失败及重试状态正确。
- [ ] 运行 NSIS 全新安装与覆盖安装，GUI、后端、CLI、Node-API 模块均存在；配置保持兼容。
- [ ] 手动及自动更新、网络失败、签名失败、安装替换与重启正常。
- [ ] 卸载移除应用文件，保留 `C:\ProgramData\ssh` 数据。

安装、提权、服务和更新验收应在可销毁的 Windows 交互环境执行。交叉编译和 macOS 桌面验证不能替代这些验收。
