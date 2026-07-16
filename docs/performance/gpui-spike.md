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
| 中文静态文本渲染 | 待 Windows 验证 | 验证标题和说明文本 |
| 鼠标点击 | 待 Windows 验证 | 两个按钮会更新状态文本 |
| Tab/Shift-Tab 焦点 | 待 Windows 验证 | 两个按钮显示蓝色焦点边框 |
| 窗口缩放 | 待 Windows 验证 | 最小内容尺寸为 640×480 |
| 中文输入法 | 待后续输入组件验证 | 当前 spike 不实现完整文本编辑器 |

远程 Windows 控制台处于锁定状态，交互会话截图只能获得锁屏背景，因此无法通过截图确认文字像素和焦点样式。窗口创建、标题、句柄和进程生命周期已通过系统信息确认。

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
