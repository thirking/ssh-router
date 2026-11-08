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
| Windows 自定义标题栏、拖动和窗口控制 | gpuix 透明标题栏 + Node-API `WM_NCHITTEST` / `WM_SYSCOMMAND` / `ShowWindow` |
| 发布构建 | Bun 独立程序 + Rust 后端 + Node-API 模块 + NSIS |

默认窗口为 620×760，最小窗口为 520×440。界面参考 [Zed One Dark](https://github.com/zed-industries/zed/blob/main/assets/themes/one/one.json)，使用灰黑配色、11–12px 正文、紧凑控件和行式路由列表；弹窗内容独立滚动，操作按钮固定可见。

Windows 使用 32px 自定义标题栏，提供最小化、最大化/还原和关闭到托盘。标题文字与空白区域支持系统拖动和双击最大化/还原；窗口按钮和工具栏不属于拖动区域，弹窗打开时暂停标题栏拖动。命中测试按当前窗口 DPI 换算，并保留系统边缘缩放处理。

标题栏拖动范围由布局事件推送，不在首帧绘制前同步查询元素位置。Windows 后台启动先完成原生窗口初始化，托盘就绪后隐藏窗口；初始化过程不抢占焦点。

界面只通过结构化请求调用列出的业务操作。配置校验、原子写入、UAC、更新源和 minisign 公钥由 Rust 核心管理。

`config.save` 保存请求发起时的快照；保存期间产生的新修改仍保留为未保存草稿。重新加载未保存草稿、删除路由和安装更新需要界面确认。

Windows GUI 与业务后端位于同一安装目录，CLI 和 Node-API 模块位于 `resources/`。后台业务进程随 GUI 退出。
