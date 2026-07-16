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
| Windows 交叉类型检查 | 环境受限 | 缺少 MSVC SDK C 头文件，`ring` 无法在 macOS 完成交叉构建 |
| 中文静态文本渲染 | 待 Windows 验证 | 验证标题和说明文本 |
| 鼠标点击 | 待 Windows 验证 | 两个按钮会更新状态文本 |
| Tab/Shift-Tab 焦点 | 待 Windows 验证 | 两个按钮显示蓝色焦点边框 |
| 窗口缩放 | 待 Windows 验证 | 最小内容尺寸为 640×480 |
| 中文输入法 | 待后续输入组件验证 | 当前 spike 不实现完整文本编辑器 |

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
