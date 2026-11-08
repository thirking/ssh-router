# gpuix 功能与调用边界

| 功能 | 实现 |
| --- | --- |
| 窗口、按钮、原生输入和弹窗 | `apps/gpuix/src/ui.tsx`，`@gpuix/react` / `@gpuix/native` 0.11.0 |
| 路由搜索、草稿、操作互斥与通知 | `apps/gpuix/src/model.ts` |
| 本地管道、请求关联与业务进程退出处理 | `apps/gpuix/src/backend.ts` |
| 配置加载、校验、保存和默认配置 | `config.*` → `ConfigService` / `validate_config` |
| 状态和三项提权操作 | `system.*` → `DesktopSystemService` |
| 更新检查、下载进度、验签与安装 | `update.*` → `UpdateService` |
| 单实例、托盘、关闭隐藏和 Explorer 恢复 | Node-API 模块 → Win32 `single_instance` / `tray` |
| 发布构建 | Bun 独立程序 + Rust 后端 + Node-API 模块 + NSIS |

界面只通过结构化请求调用列出的业务操作。配置校验、原子写入、UAC、更新源和 minisign 公钥由 Rust 核心管理。

`config.save` 保存请求发起时的快照；保存期间产生的新修改仍保留为未保存草稿。重新加载未保存草稿、删除路由和安装更新需要界面确认。

Windows GUI 与业务后端位于同一安装目录，CLI 和 Node-API 模块位于 `resources/`。后台业务进程随 GUI 退出。
