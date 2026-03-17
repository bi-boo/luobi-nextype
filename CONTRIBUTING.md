# 贡献指南

感谢你对落笔 Nextype 的关注！以下是参与贡献的方式。

## 提交 Issue

- **Bug 报告**：请描述复现步骤、预期行为、实际行为，并附上操作系统和应用版本
- **功能建议**：请描述使用场景和期望的行为

## 提交 Pull Request

1. Fork 本仓库
2. 创建功能分支：`git checkout -b feature/your-feature`
3. 提交更改：`git commit -m "feat: add your feature"`
4. 推送到远程：`git push origin feature/your-feature`
5. 创建 Pull Request

### PR 规范

- 标题简明扼要，说明改了什么
- 描述中包含改动原因和测试方式
- 如果涉及多端改动，请在描述中列出所有受影响的端

## 开发环境

### macOS / Windows 桌面端

- [Rust](https://rustup.rs/)（最新稳定版）
- [Node.js](https://nodejs.org/) 18+
- macOS 端额外需要：Xcode Command Line Tools（`xcode-select --install`）
- Windows 端额外需要：Visual Studio Build Tools（C++ 工作负载）
- 详见 [NextypeMac/README.md](./NextypeMac/README.md) 和 [NextypeWindows/README.md](./NextypeWindows/README.md)

### Android 端

- [Android Studio](https://developer.android.com/studio)（最新稳定版）
- JDK 17+
- 详见 [NextypeAndroid/README.md](./NextypeAndroid/README.md)

### iOS 端

- Xcode 15+
- iOS 15.0+
- 详见 [NextypeApp/README.md](./NextypeApp/README.md)

### 中继服务器

- Node.js 18+
- 详见 [relay-server/README.md](./relay-server/README.md)

## 代码规范

- **Rust**：使用 `cargo fmt` 格式化，`cargo clippy` 检查
- **Kotlin**：遵循 Android Kotlin Style Guide
- **Swift**：遵循 Swift API Design Guidelines
- **JavaScript**：使用项目现有风格（无框架，原生 JS）

## Commit 消息规范

使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

- `feat:` 新功能
- `fix:` Bug 修复
- `docs:` 文档更新
- `refactor:` 重构
- `chore:` 构建/工具变更

## 许可证

提交代码即表示你同意将代码以 [AGPL-3.0](./LICENSE) 许可证发布。
