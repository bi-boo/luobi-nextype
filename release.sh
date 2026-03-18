#!/bin/bash
set -e
export PATH="/opt/homebrew/bin:$PATH"

# 上传最新打包的 Mac DMG + Android APK 到 GitHub Releases，覆盖 latest
# 用法：bash release.sh

REPO="bi-boo/luobi-nextype"
TAG="latest"
MAC_DMG_DIR="NextypeMac/src-tauri/target/release/bundle/dmg"
ANDROID_APK="NextypeAndroid/app/build/outputs/apk/release/app-release.apk"

# 找 Mac DMG
DMG=$(ls "$MAC_DMG_DIR"/*.dmg 2>/dev/null | head -1)
if [ -z "$DMG" ]; then
    echo "❌ 未找到 Mac DMG，请先执行 Mac 打包"
    exit 1
fi

# 检查 Android APK
if [ ! -f "$ANDROID_APK" ]; then
    echo "❌ 未找到 Android APK，请先执行 Android 打包"
    exit 1
fi

# 从 DMG 文件名提取版本号
VERSION=$(basename "$DMG" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')
echo "🏷️  版本：$VERSION"
echo "📦 Mac：$(basename "$DMG")"
echo "📦 Android：落笔 Nextype_${VERSION}_android.apk"

# 重命名 APK（加上版本号）
RENAMED_APK="/tmp/落笔 Nextype_${VERSION}_android.apk"
cp "$ANDROID_APK" "$RENAMED_APK"

# 删除旧 latest release
gh release delete "$TAG" --repo "$REPO" --yes 2>/dev/null && echo "🗑️  已删除旧 latest release" || true

# 创建新 release，同时上传 Mac + Android
gh release create "$TAG" "$DMG" "$RENAMED_APK" \
    --repo "$REPO" \
    --title "落笔 Nextype v$VERSION" \
    --notes "最新版本 v$VERSION" \
    --latest

echo "✅ 已上传 Mac + Android 到 GitHub Releases（latest）"
