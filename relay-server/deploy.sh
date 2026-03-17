#!/bin/bash
# ============================================================
# Nextype 中继服务器安全部署脚本
# 只更新代码文件，保留数据（trust_relationships.json）
# ============================================================

set -e

SERVER_IP="${NEXTYPE_SERVER_IP:?请设置 NEXTYPE_SERVER_IP 环境变量}"
SERVER_USER="${NEXTYPE_SERVER_USER:-ubuntu}"
SSH_KEY="${NEXTYPE_SSH_KEY:-$HOME/.ssh/nextype.pem}"
REMOTE_DIR="/home/ubuntu/relay-server"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "🚀 开始部署中继服务器..."
echo "═════════════════════════════════════"

# 上传代码文件（仅 server.js）
echo "📤 上传 server.js..."
scp -i "$SSH_KEY" "$SCRIPT_DIR/server.js" "$SERVER_USER@$SERVER_IP:$REMOTE_DIR/"

# 上传 package.json（如果需要更新依赖）
echo "📤 上传 package.json..."
scp -i "$SSH_KEY" "$SCRIPT_DIR/package.json" "$SERVER_USER@$SERVER_IP:$REMOTE_DIR/"

# 重启服务
echo "🔄 重启服务..."
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no "$SERVER_USER@$SERVER_IP" "cd $REMOTE_DIR && pm2 restart nextype-relay"

echo "═════════════════════════════════════"
echo "✅ 部署完成！"
echo ""
echo "📋 数据文件 trust_relationships.json 已保留"
echo "📋 如需查看日志: pm2 logs nextype-relay"
