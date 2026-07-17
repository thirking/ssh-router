# GPUI MVP 发布验证

验证环境：Windows 11 Pro Build 26100，MSVC x86_64，NSIS 3.12。源码位于隔离纯 Rust 工作树 `ssh-router-gpui-clean-3`；已安装旧版 PID 19488 和安装目录始终未修改或终止。

发布候选：`SSH-Router_0.0.8_x64-setup.exe`，6,665,387 bytes，SHA-256 `67749EC11B5C6EBBDB25E691464F11EC5F6C24EFF8BB57D552661DF19708E444`。

## 自动验证

- [x] `cargo fmt --all -- --check`
- [x] `cargo test -q --workspace --target x86_64-pc-windows-msvc`
- [x] `cargo clippy -q --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings`
- [x] Release 构建 `ssh-router.exe` 与 `ssh-router-cli.exe`
- [x] NSIS 3.12 生成发布候选安装包
- [x] 仓库依赖扫描不含旧 Web 构建链；兼容保留的历史 GitHub Secret 名称已逐项确认

## 功能与生命周期

- [x] 生产启动默认隐藏，托盘消息可恢复主窗口
- [ ] 已有 `ssh-router.json` 可加载，保存后 CLI 可读取：加载和候选 CLI 实测通过；为保护现有环境，尚未执行实际覆盖保存
- [ ] 配置不存在时可创建默认配置：fake repository 与临时文件测试通过，尚未在隔离用户实装验证
- [x] 添加、编辑、删除、默认路由和 SFTP 命令正确；UI 操作仅修改内存草稿，系统配置哈希未变
- [x] CLI、DefaultShell、配置文件和 sshd 状态显示与系统一致
- [ ] 安装 CLI、设置 DefaultShell、重启 sshd：成功、取消与失败分支测试通过，尚未在受保护远程机执行真实提权操作
- [x] CLI 版本不一致提示、稍后处理和状态重试正确
- [x] 关闭窗口隐藏到托盘，托盘菜单退出真正终止候选进程
- [x] 重复启动只激活已有候选实例并退出二次实例

## 安装与更新

- [ ] 新用户首次安装路径、资源、快捷方式和卸载键：NSIS 产物和脚本验证通过；临时用户非交互启动受 Windows 桌面权限拒绝，尚缺真实隔离桌面实装
- [ ] v0.0.8 覆盖升级到 GPUI 保留配置与 CLI：现有旧版安装受保护，未覆盖
- [ ] 运行中自动更新等待父进程退出后替换文件并重启：参数、等待和测试通过，尚缺隔离桌面端到端实装
- [ ] 手动检查、无更新、网络失败和签名失败：状态机与签名测试通过；远程机 `github.com:443` 连接失败并恢复按钮，尚缺可联网的无更新实测
- [ ] 卸载清理 GUI、快捷方式和卸载键，不删除 `C:\ProgramData\ssh` 数据：脚本验证通过，尚缺隔离桌面实装

## 性能与稳定性

- [x] 已发布 v0.0.8 完整进程树空闲基线已记录
- [ ] 已发布 v0.0.8 冷启动：原 CSV 为二次实例退出时间，已标记无效；需在无旧实例的独立 Session 或 VM 重测
- [x] GPUI 10 次首帧启动和 10 秒预热 + 60 秒空闲资源已记录
- [x] 二次实例激活、关闭隐藏和托盘恢复响应时间已记录
- [x] 未发现明显 UI 线程阻塞、持续高 CPU 或阻断使用的严重缺陷

## 实测证据

- [性能基线](../performance/baselines/2026-07-17-gpui-mvp.md)
- [主窗口](evidence/main-window.png)
- [滚动后的路由编辑器](evidence/scrolled-route-editor.png)
- [中文路由表单](evidence/route-form-filled.png)
- [新增路由](evidence/route-added.png)
- [删除路由](evidence/route-deleted.png)
- [Tab 焦点](evidence/focus-tab.png) · [Shift-Tab 焦点](evidence/focus-shift-tab.png)
- [生命周期原始数据](evidence/lifecycle.json)
- [640×520 缩放结果](evidence/resize.json)

## 剩余环境条件

完成未勾选项需要可销毁的交互 Windows 用户或 VM，并且不能与受保护旧版实例共享单实例命名空间。当前 Windows Sandbox 未启用、Hyper-V 未配置 VM；临时用户从非交互桌面启动 NSIS 返回桌面访问错误。不得把这些项目伪造为通过。
