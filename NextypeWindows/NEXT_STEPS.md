# Windows 端后续步骤指南

## 当前状态

✅ **基础架构已完成**
- 项目结构创建完成
- Rust 代码移植完成（90% 复用 Mac 端）
- Windows 特定实现完成（剪贴板、快捷键）
- 配置文件就绪
- 文档编写完成

⏳ **待在 Windows 环境中测试**

---

## 在 Windows 环境中的操作步骤

### 1. 环境准备

#### 安装 Rust
```powershell
# 下载并安装 rustup
# https://rustup.rs/
# 或使用 winget
winget install Rustlang.Rustup
```

#### 安装 Visual Studio Build Tools
```powershell
# 需要 C++ 构建工具
# 下载 Visual Studio Installer
# 选择 "Desktop development with C++"
```

#### 安装 WebView2 Runtime
```powershell
# 通常 Windows 11 已预装
# 如果没有，从微软官网下载
# https://developer.microsoft.com/microsoft-edge/webview2/
```

### 2. 编译项目

```powershell
# 进入项目目录
cd "落笔 Nextype WorkSpace\落笔 Nextype\NextypeTauri\nextype-windows\src-tauri"

# 检查 Rust 环境
rustc --version
cargo --version

# 编译（Debug 模式）
cargo build

# 编译（Release 模式）
cargo build --release
```

### 3. 运行开发模式

```powershell
# 在 src-tauri 目录下
cargo tauri dev
```

**预期结果**：
- 应用窗口打开
- 系统托盘显示图标
- 可以生成配对码

### 4. 功能测试清单

#### 基础功能
- [ ] 应用启动成功
- [ ] 系统托盘图标显示
- [ ] 托盘菜单可点击
- [ ] 偏好设置窗口打开

#### 配对功能
- [ ] 生成 4 位配对码
- [ ] 配对码倒计时正常
- [ ] 二维码显示正常
- [ ] 与 Android 手机配对成功
- [ ] 配对成功后显示设备

#### 剪贴板同步
- [ ] 手机发送文字到 PC
- [ ] PC 剪贴板写入成功
- [ ] 自动粘贴（Ctrl+V）正常
- [ ] 粘贴+回车正常
- [ ] 加密解密正常

#### 快捷键
- [ ] 注册快捷键成功
- [ ] 快捷键触发正常
- [ ] 发送指令（send）
- [ ] 插入指令（insert）
- [ ] 清空指令（clear）
- [ ] 点击指令（tap）
- [ ] 长按指令（longpress）

#### 系统集成
- [ ] 开机启动设置生效
- [ ] 单实例保护正常
- [ ] 窗口关闭不退出应用
- [ ] 日志系统正常

#### 网络功能
- [ ] 连接中继服务器成功
- [ ] 断线自动重连
- [ ] 心跳机制正常
- [ ] 设备上下线通知

### 5. 常见问题排查

#### 编译错误

**问题**: `error: linker 'link.exe' not found`
**解决**: 安装 Visual Studio Build Tools

**问题**: `windows-rs` 相关错误
**解决**: 检查 Cargo.toml 中的 windows 依赖版本

**问题**: WebView2 相关错误
**解决**: 安装 WebView2 Runtime

#### 运行时错误

**问题**: 剪贴板粘贴不工作
**解决**: 检查 SendInput API 调用，确认权限

**问题**: 快捷键不响应
**解决**: 检查 tauri-plugin-global-shortcut 是否正常注册

**问题**: 托盘图标不显示
**解决**: 检查 .ico 图标文件是否存在

### 6. 打包发布

```powershell
# 在 src-tauri 目录下
cargo tauri build
```

**生成的文件位置**：
- MSI: `src-tauri\target\release\bundle\msi\`
- NSIS: `src-tauri\target\release\bundle\nsis\`

**测试安装包**：
1. 运行 MSI 或 NSIS 安装程序
2. 检查安装路径
3. 检查开始菜单快捷方式
4. 检查开机启动项
5. 测试卸载功能

### 7. 性能测试

- [ ] 内存占用（预期 < 100MB）
- [ ] CPU 占用（空闲时 < 1%）
- [ ] 启动速度（< 3 秒）
- [ ] 网络流量（心跳 < 1KB/10s）

### 8. 优化建议

#### 图标优化
- 确保 .ico 文件包含多种尺寸（16x16, 32x32, 48x48, 256x256）
- 托盘图标使用简洁设计，适配深色/浅色主题

#### 性能优化
- 检查内存泄漏
- 优化日志输出频率
- 减少不必要的网络请求

#### 用户体验
- 添加首次运行引导
- 优化错误提示信息
- 添加更新检查功能

---

## 预期问题和解决方案

### 问题 1: Windows Defender 误报
**现象**: 安装包被 Windows Defender 拦截
**解决**: 
- 申请代码签名证书
- 或在 README 中说明如何添加信任

### 问题 2: 快捷键冲突
**现象**: 某些快捷键与系统或其他软件冲突
**解决**:
- 提供快捷键自定义功能
- 在文档中说明常见冲突

### 问题 3: 防火墙拦截
**现象**: WebSocket 连接失败
**解决**:
- 首次运行时提示用户允许网络访问
- 在文档中说明如何配置防火墙

---

## 发布前检查清单

- [ ] 所有功能测试通过
- [ ] 无明显 Bug
- [ ] 性能指标达标
- [ ] 安装包测试通过
- [ ] 卸载测试通过
- [ ] 文档完善（README、用户手册）
- [ ] 更新日志编写
- [ ] 官网更新（下载链接、系统要求）

---

## 联系方式

如果在 Windows 环境测试中遇到问题，请记录：
1. 错误信息（完整的错误日志）
2. 操作步骤（如何复现）
3. 系统环境（Windows 版本、Rust 版本）
4. 截图（如果适用）

---

## 预计时间

- 环境准备: 1-2 小时
- 编译测试: 0.5 小时
- 功能测试: 2-3 小时
- 问题修复: 1-2 天（取决于问题数量）
- 打包测试: 1 小时
- 优化完善: 1-2 天

**总计**: 3-5 天（在 Windows 环境中）

---

## 成功标准

✅ 编译无错误
✅ 所有核心功能正常工作
✅ 与 Android 手机配对成功
✅ 剪贴板同步正常
✅ 快捷键远程控制正常
✅ 安装包可正常安装和卸载
✅ 性能指标达标
