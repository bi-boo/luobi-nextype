# 落笔 Nextype — 运维信息（不入仓库）

## 服务器信息

| 项目 | 值 |
|------|-----|
| 中继服务器 IP | 43.143.241.154 |
| SSH 用户名 | ubuntu |
| SSH 密钥 | ~/.ssh/nextype.pem |
| 服务目录 | /home/ubuntu/relay-server/ |
| 官网目录 | /var/www/nextype-website/ |
| 进程管理 | PM2，进程名 nextype-relay |
| 域名 | nextypeapi.yuanfengai.cn（中继）/ yuanfengai.cn（官网） |

## 部署命令

### 部署官网
```bash
NEXTYPE_SERVER_IP=43.143.241.154 bash website/deploy.sh
```

### 部署中继服务器
```bash
NEXTYPE_SERVER_IP=43.143.241.154 bash relay-server/deploy.sh
```

### SSH 连接
```bash
ssh -i ~/.ssh/nextype.pem ubuntu@43.143.241.154
```

## Gotchas

- macOS 全局 Fn 热键不要使用单独 Fn、Fn+Shift、Fn+Ctrl、Fn+Enter、Fn+Delete；这类组合会迫使应用长期处理纯修饰键或特殊键事件，容易影响系统交互流畅度。正确做法：只支持 Fn+普通按键，并保留非 Fn 备用快捷键。
