# 落笔 Nextype - 项目结构说明

## 📁 项目目录结构

```
落笔 Nextype/
├── NextypeAndroid/          # Android 移动端
├── NextypeApp/              # iOS 移动端
├── NextypeMac/              # Mac 桌面端 (Tauri)
├── NextypeWindows/          # Windows 桌面端 (Tauri) ✨ 新增
├── relay-server/            # 中继服务器 (Node.js)
├── website/                 # 官方网站
├── electron-app/            # [已废弃] 旧版 Electron 桌面端
├── NextypeTauri/            # [空目录] 可删除
├── docs/                    # 项目文档
├── legal/                   # 法律文件
└── credentials/             # 服务器凭证
```

## 🎯 各端说明

### 移动端
- **NextypeAndroid/** - Android 端（Kotlin）
  - 状态：✅ 主力开发中
  - 功能：最完整的移动端

- **NextypeApp/** - iOS 端（Swift）
  - 状态：⚠️ 基本可用，需重构
  - 功能：核心功能可用

### 桌面端
- **NextypeMac/** - Mac 端（Tauri 2.x）
  - 状态：✅ 主力开发中
  - 功能：完整功能，从 Electron 迁移而来

- **NextypeWindows/** - Windows 端（Tauri 2.x）
  - 状态：✅ 架构完成，待 Windows 环境测试
  - 功能：90% 复用 Mac 端代码

- **electron-app/** - 旧版 Electron 端
  - 状态：❌ 已废弃，不再维护
  - 说明：已被 Tauri 版本替代

### 服务端
- **relay-server/** - 中继服务器（Node.js + ws）
  - 状态：✅ 运行中
  - 功能：设备注册、配对管理、消息转发

### 其他
- **website/** - 官方网站
- **docs/** - 项目文档（PRD、架构文档等）
- **legal/** - 法律文件（著作权、专利）
- **credentials/** - 服务器凭证

## 🔄 目录调整说明

### 调整前
```
NextypeTauri/
├── nextype-tauri/    # Mac 端
└── nextype-windows/  # Windows 端
```

### 调整后
```
NextypeMac/           # Mac 端（独立目录）
NextypeWindows/       # Windows 端（独立目录）
```

### 调整原因
- 四个端（Android、iOS、Mac、Windows）应该是平行的项目结构
- 便于独立开发和维护
- 统一命名规范（Nextype + 平台名）

## 📝 命名规范

| 目录名 | 平台 | 技术栈 | 状态 |
|--------|------|--------|------|
| NextypeAndroid | Android | Kotlin | ✅ 活跃 |
| NextypeApp | iOS | Swift | ⚠️ 需重构 |
| NextypeMac | macOS | Tauri + Rust | ✅ 活跃 |
| NextypeWindows | Windows | Tauri + Rust | ✅ 新增 |

## 🗑️ 可删除的目录

- **NextypeTauri/** - 空目录，已将内容移出
- **electron-app/** - 已废弃的 Electron 版本（可选保留作为历史参考）

## 📚 文档位置

- 全局文档：`docs/`
  - `prd.md` - 项目总览
  - `architecture.md` - 系统架构
  - `mac-prd.md` / `mac-architecture.md` - Mac 端文档
  - `windows-prd.md` / `windows-architecture.md` - Windows 端文档
  - `android-prd.md` / `android-architecture.md` - Android 端文档
  - `ios-prd.md` / `ios-architecture.md` - iOS 端文档
  - `relay-server.md` - 中继服务器文档

- 各端文档：
  - `NextypeMac/README.md`
  - `NextypeWindows/README.md`
  - 等等

## 🚀 快速导航

### 开发 Mac 端
```bash
cd NextypeMac/src-tauri
cargo tauri dev
```

### 开发 Windows 端
```bash
cd NextypeWindows/src-tauri
cargo tauri dev
```

### 开发 Android 端
```bash
cd NextypeAndroid
bash build-and-install.sh
```

### 启动中继服务器
```bash
cd relay-server
npm start
```

---

**更新日期**: 2026-02-28
**项目状态**: 四端并行开发，结构清晰
