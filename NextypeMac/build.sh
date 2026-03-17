#!/bin/bash
set -e

# 加载 Rust 环境
source "$HOME/.cargo/env"

SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:-}"
TEAM_ID="${APPLE_TEAM_ID:-}"

# 检测证书是否存在于 Keychain
if [ -n "$SIGNING_IDENTITY" ] && security find-identity -v -p codesigning | grep -q "$SIGNING_IDENTITY"; then
    echo "[build] 找到签名证书，使用正式签名构建"
    npx tauri build
else
    echo "[build] 未找到签名证书，使用 ad-hoc 临时签名构建（仅供本机测试）"
    APPLE_SIGNING_IDENTITY="-" npx tauri build
fi

# 公证（仅当 APPLE_ID 和 APP_SPECIFIC_PASSWORD 环境变量已设置时执行）
if [ -n "$APPLE_ID" ] && [ -n "$APP_SPECIFIC_PASSWORD" ]; then
    echo "[notarize] 开始公证流程..."
    DMG=$(ls src-tauri/target/release/bundle/dmg/*.dmg 2>/dev/null | head -1)
    if [ -z "$DMG" ]; then
        echo "[notarize] ❌ 未找到 DMG 文件，跳过公证"
    else
        echo "[notarize] 提交 $DMG 至 Apple Notary Service..."
        xcrun notarytool submit "$DMG" \
            --apple-id "$APPLE_ID" \
            --password "$APP_SPECIFIC_PASSWORD" \
            --team-id "$TEAM_ID" \
            --wait
        echo "[notarize] 钉住公证票据..."
        xcrun stapler staple "$DMG"
        echo "[notarize] ✅ 公证完成"
    fi
else
    echo "[notarize] 跳过公证（未设置 APPLE_ID / APP_SPECIFIC_PASSWORD 环境变量）"
fi
