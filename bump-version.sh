#!/bin/bash
set -e
export PATH="/opt/homebrew/bin:$PATH"

# 用法：
#   bash bump-version.sh          # patch 递进（1.0.0 → 1.0.1）
#   bash bump-version.sh minor    # minor 递进（1.0.0 → 1.1.0）
#   bash bump-version.sh major    # major 递进（1.0.0 → 2.0.0）
#   bash bump-version.sh 1.2.3    # 指定版本号

MAC_TAURI_CONF="NextypeMac/src-tauri/tauri.conf.json"
MAC_PKG_JSON="NextypeMac/package.json"
ANDROID_GRADLE="NextypeAndroid/app/build.gradle.kts"

# 读取当前版本号（从 tauri.conf.json）
CURRENT=$(grep '"version"' "$MAC_TAURI_CONF" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
MAJOR=$(echo "$CURRENT" | cut -d. -f1)
MINOR=$(echo "$CURRENT" | cut -d. -f2)
PATCH=$(echo "$CURRENT" | cut -d. -f3)

# 计算新版本号
ARG="${1:-patch}"
if [[ "$ARG" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    NEW="$ARG"
elif [ "$ARG" = "major" ]; then
    NEW="$((MAJOR+1)).0.0"
elif [ "$ARG" = "minor" ]; then
    NEW="${MAJOR}.$((MINOR+1)).0"
else
    NEW="${MAJOR}.${MINOR}.$((PATCH+1))"
fi

echo "🔢 版本号：$CURRENT → $NEW"

# 更新 Mac tauri.conf.json
sed -i '' "s/\"version\": \"$CURRENT\"/\"version\": \"$NEW\"/" "$MAC_TAURI_CONF"

# 更新 Mac package.json
sed -i '' "s/\"version\": \"$CURRENT\"/\"version\": \"$NEW\"/" "$MAC_PKG_JSON"

# 计算 Android versionCode（major*10000 + minor*100 + patch）
ANDROID_MAJOR=$(echo "$NEW" | cut -d. -f1)
ANDROID_MINOR=$(echo "$NEW" | cut -d. -f2)
ANDROID_PATCH=$(echo "$NEW" | cut -d. -f3)
NEW_CODE=$((ANDROID_MAJOR * 10000 + ANDROID_MINOR * 100 + ANDROID_PATCH))

# 更新 Android build.gradle.kts
sed -i '' "s/versionCode = [0-9]*/versionCode = $NEW_CODE/" "$ANDROID_GRADLE"
sed -i '' "s/versionName = \"[0-9.]*\"/versionName = \"$NEW\"/" "$ANDROID_GRADLE"

echo "✅ 已更新："
echo "   Mac:     $MAC_TAURI_CONF + $MAC_PKG_JSON → $NEW"
echo "   Android: $ANDROID_GRADLE → $NEW (versionCode=$NEW_CODE)"
