#!/bin/bash
set -e

SERVER_IP="${NEXTYPE_SERVER_IP:?请设置 NEXTYPE_SERVER_IP 环境变量}"
SERVER_USER="${NEXTYPE_SERVER_USER:-ubuntu}"
SSH_KEY="${NEXTYPE_SSH_KEY:-$HOME/.ssh/nextype.pem}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "开始部署官网..."

scp -r -i "$SSH_KEY" \
  "$SCRIPT_DIR/index.html" \
  "$SCRIPT_DIR/style.css" \
  "$SCRIPT_DIR/privacy.html" \
  "$SCRIPT_DIR/terms.html" \
  "$SCRIPT_DIR/robots.txt" \
  "$SCRIPT_DIR/sitemap.xml" \
  "$SCRIPT_DIR/favicon.ico" \
  "$SCRIPT_DIR/version.json" \
  "$SCRIPT_DIR/assets" \
  "$SERVER_USER@$SERVER_IP:/home/ubuntu/website-tmp/"

ssh -i "$SSH_KEY" "$SERVER_USER@$SERVER_IP" \
  "sudo cp -r /home/ubuntu/website-tmp/* /var/www/nextype-website/ && sudo chown -R www-data:www-data /var/www/nextype-website"

echo "部署完成！"
