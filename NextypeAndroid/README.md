# 落笔 Nextype — Android 端

Kotlin 原生 Android 客户端。

## 技术栈

- 语言: Kotlin
- 构建: Gradle
- 最低版本: Android 8.0 (API 26)

## 前置条件

- [Android Studio](https://developer.android.com/studio) (最新稳定版)
- JDK 17+
- Android SDK (API 34+)

## 开发

1. 用 Android Studio 打开 `NextypeAndroid/` 目录
2. 同步 Gradle 依赖
3. 连接设备或启动模拟器
4. 运行应用

## 构建

```bash
cd NextypeAndroid

# 编译 Debug APK
./gradlew assembleDebug

# 编译并安装到已连接的设备
bash build-and-install.sh
```

## 权限说明

- 网络访问: 连接中继服务器
- 辅助功能服务: 模拟屏幕点击（可选，用于远程遥控）

## 许可证

[AGPL-3.0](../LICENSE)
