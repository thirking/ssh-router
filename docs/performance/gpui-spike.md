# GPUI Windows 可行性验证

## 依赖

- GPUI：`0.2.2`，通过 Cargo 精确版本约束和 `Cargo.lock` 锁定。
- 包名：`ssh-router-gpui`。
- 目标：`x86_64-pc-windows-msvc`。

选择 crates.io 稳定版本，避免 MVP 开始阶段依赖浮动的 Zed 主分支 revision。

## 当前验证结果

| 项目 | 状态 | 说明 |
|---|---|---|
| macOS 主机类型检查 | 环境受限 | 当前仅安装 Command Line Tools，缺少 Metal 编译器 |
| Windows 原生构建 | 通过 | 在 `dev` 的 Windows/MSVC 环境完成 debug 构建 |
| Windows 窗口创建 | 通过 | 交互会话中窗口标题为 `SSH Router GPUI Spike`，原生窗口句柄有效 |
| 中文静态文本渲染 | 通过 | 发布候选截图中标题、状态、路由名和错误文本均正常 |
| 鼠标点击 | 通过 | 完成提示关闭、路由新增、编辑和删除 |
| Tab/Shift-Tab 焦点 | 通过 | 实测 Tab 将焦点从输入框移到刷新按钮，Shift-Tab 恢复输入框焦点边框 |
| 窗口缩放 | 通过 | Win32 实测调整为 640×520，窗口保持响应且长内容可滚动 |
| 中文输入法 | 部分通过 | 中文输入和渲染成功；自动 SendKeys 的完整拼音选词不稳定，仍需人工 IME 复核 |

通过仅在用户登录时运行的计划任务进入交互 Session 1，已取得真实窗口截图并完成鼠标、滚动、中文输入和生命周期验证。SSH Session 0 仍不能直接创建 DirectX renderer。

## Windows 验证步骤

```powershell
cargo build -q -p ssh-router-gpui --target x86_64-pc-windows-msvc
cargo run -q -p ssh-router-gpui --target x86_64-pc-windows-msvc
```

1. 确认窗口标题和中文说明正常显示。
2. 调整窗口大小，确认布局没有崩溃或明显绘制错误。
3. 点击两个按钮，确认状态文本更新。
4. 使用 Tab 和 Shift-Tab 切换焦点，确认蓝色焦点边框随之切换。
5. 关闭窗口，确认进程正常退出。

完整中文输入法验证将在基础文本输入组件实现时进行；若 GPUI 0.2.2 在 Windows 上存在输入法、焦点或渲染阻断问题，必须在继续业务迁移前重新评估版本。
