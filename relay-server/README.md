# 落笔 Nextype — 中继服务器

WebSocket 中继服务器，负责设备间的消息转发和配对管理。

## 技术栈

- 运行时: Node.js 18+
- WebSocket: ws 库
- 进程管理: pm2 (生产环境)

## 快速启动

```bash
cd relay-server
npm install
npm start
```

服务默认监听 WebSocket 连接。

## 自建部署

### 1. 服务器要求

- Node.js 18+
- 公网 IP 或域名
- SSL 证书（推荐，用于 wss:// 连接）

### 2. 部署步骤

```bash
# 上传代码到服务器
scp -r relay-server/ user@your-server:/path/to/

# 在服务器上安装依赖并启动
ssh user@your-server
cd /path/to/relay-server
npm install
pm2 start server.js --name nextype-relay
```

### 3. 配置客户端

在各端应用的设置中，将中继服务器地址修改为你的服务器地址。

## 环境变量

部署脚本支持以下环境变量：

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `NEXTYPE_SERVER_IP` | 服务器 IP | 无（必须设置） |
| `NEXTYPE_SERVER_USER` | SSH 用户名 | ubuntu |
| `NEXTYPE_SSH_KEY` | SSH 密钥路径 | ~/.ssh/nextype.pem |

## 许可证

[AGPL-3.0](../LICENSE)
