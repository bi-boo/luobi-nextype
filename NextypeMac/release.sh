#!/bin/bash
set -e
export PATH="/opt/homebrew/bin:$PATH"

# 上传最新打包的 DMG 到 GitHub Releases，覆盖 latest tag
# 用法：bash release.sh

REPO="bi-boo/luobi-nextype"
TAG="latest"
DMG_DIR="src-tauri/target/release/bundle/dmg"

# 找到 DMG 文件
DMG=$(ls "$DMG_DIR"/*.dmg 2>/dev/null | head -1)
if [ -z "$DMG" ]; then
    echo "❌ 未找到 DMG 文件，请先执行 npm run tauri build"
    exit 1
fi

VERSION=$(basename "$DMG" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')
echo "📦 准备上传: $(basename "$DMG")（版本 $VERSION）"

# 删除旧的 latest release（如果存在）
gh release delete "$TAG" --repo "$REPO" --yes 2>/dev/null && echo "🗑️  已删除旧 latest release" || true

# 创建新 release 并上传 DMG
gh release create "$TAG" "$DMG" \
    --repo "$REPO" \
    --title "落笔 Nextype v$VERSION" \
    --notes "最新版本 v$VERSION" \
    --latest

echo "✅ 已上传到 GitHub Releases（latest）"
