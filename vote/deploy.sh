#!/bin/bash
set -e

SERVER_IP="${NEXTYPE_SERVER_IP:-43.143.241.154}"
SSH_KEY="$HOME/.ssh/nextype.pem"
SSH_USER="ubuntu"
REMOTE_DIR="/home/ubuntu/team-vote"
PM2_NAME="team-vote"

echo ">>> Deploying to $SERVER_IP ..."

ssh -i "$SSH_KEY" "$SSH_USER@$SERVER_IP" "mkdir -p $REMOTE_DIR/public/thumbnails $REMOTE_DIR/public/projects"

scp -i "$SSH_KEY" -r \
  server.js package.json public/ \
  "$SSH_USER@$SERVER_IP:$REMOTE_DIR/"

ssh -i "$SSH_KEY" "$SSH_USER@$SERVER_IP" "
  cd $REMOTE_DIR
  npm install --production
  pm2 describe $PM2_NAME > /dev/null 2>&1 && pm2 restart $PM2_NAME || pm2 start server.js --name $PM2_NAME
  pm2 save
"

echo ">>> Done! http://$SERVER_IP:3210"
