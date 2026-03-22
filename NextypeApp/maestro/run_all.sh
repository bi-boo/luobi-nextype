#!/bin/bash
# 运行全部 Nextype iOS 测试流程
# 使用方式：先在 Xcode 中将 App 构建并安装到模拟器，然后运行此脚本

export PATH="$PATH":"$HOME/.maestro/bin"

FLOWS_DIR="$(dirname "$0")/flows"

echo "=============================="
echo "  落笔 Nextype - Maestro 测试"
echo "=============================="
echo ""

passed=0
failed=0

for flow in "$FLOWS_DIR"/*.yaml; do
    name=$(basename "$flow")
    echo "▶ 运行：$name"
    if maestro test "$flow"; then
        echo "✓ 通过：$name"
        ((passed++))
    else
        echo "✗ 失败：$name"
        ((failed++))
    fi
    echo ""
done

echo "=============================="
echo "  结果：通过 $passed | 失败 $failed"
echo "=============================="
