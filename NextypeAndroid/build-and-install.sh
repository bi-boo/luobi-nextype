#!/bin/bash

# Android 自动编译安装脚本
# 用途:修改代码后自动编译并安装到连接的 Android 设备

set -e  # 遇到错误立即退出

echo "🚀 开始自动编译安装流程..."

# 设置颜色输出
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 1. 设置 Java 环境
echo -e "${YELLOW}📦 设置 Java 环境...${NC}"

# 直接列出几个常见的安装路径进行探测
POSSIBLE_JAVA_HOMES=(
    "/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home"
    "/opt/homebrew/opt/openjdk/libexec/openjdk.jdk/Contents/Home"
    "/Applications/Android Studio.app/Contents/jbr/Contents/Home"
    "/Applications/Android Studio.app/Contents/jre/Contents/Home"
    "$(/usr/libexec/java_home -v 17 2>/dev/null || echo "")"
)

for path in "${POSSIBLE_JAVA_HOMES[@]}"; do
    if [ -n "$path" ] && [ -d "$path" ] && [ -x "$path/bin/java" ]; then
        export JAVA_HOME="$path"
        break
    fi
done

if [ -z "$JAVA_HOME" ]; then
    echo -e "${RED}❌ 错误: 找不到 Java 运行时环境。请确保已安装 JDK 17。${NC}"
    exit 1
fi

export PATH="$JAVA_HOME/bin:$PATH"
echo -e "${GREEN}✓ JAVA_HOME: $JAVA_HOME${NC}"

# 2. 检查设备连接
echo -e "${YELLOW}📱 检查 Android 设备连接...${NC}"
DEVICE_COUNT=$(adb devices | grep -v "List" | grep "device" | wc -l | tr -d ' ')
if [ "$DEVICE_COUNT" -eq 0 ]; then
    echo -e "${RED}❌ 错误:没有检测到 Android 设备${NC}"
    echo "请确保:"
    echo "  1. 手机已通过 USB 连接到电脑"
    echo "  2. 已在手机上开启「开发者选项」和「USB 调试」"
    echo "  3. 已在手机上授权此电脑进行调试"
    exit 1
fi
echo -e "${GREEN}✓ 检测到 $DEVICE_COUNT 台设备${NC}"
adb devices

# 3. 清理并编译
echo -e "${YELLOW}🔨 开始编译 APK...${NC}"
./gradlew clean assembleDebug --console=plain

# 4. 检查编译结果
APK_PATH="app/build/outputs/apk/debug/app-debug.apk"
if [ ! -f "$APK_PATH" ]; then
    echo -e "${RED}❌ 编译失败:找不到 APK 文件${NC}"
    exit 1
fi
echo -e "${GREEN}✓ 编译成功:$APK_PATH${NC}"

# 5. 覆盖安装（保留 App 数据，避免清除配对密钥）
echo -e "${YELLOW}📲 安装新版本到设备...${NC}"
adb install -r "$APK_PATH"

# 6. 启动应用
echo -e "${YELLOW}🚀 启动应用...${NC}"
adb shell am start -n com.nextype.app/com.nextype.android.EmptyStateActivity

echo -e "${GREEN}✅ 完成!应用已成功安装并启动${NC}"
