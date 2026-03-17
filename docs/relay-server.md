# 落笔 Nextype — 中继服务器协议与架构文档

## 服务器架构

中继服务器是落笔 Nextype 跨设备通信的核心枢纽，负责在 Mac 桌面端（server 角色）和 iPhone 移动端（client 角色）之间建立连接、管理配对关系、转发消息。

### 技术栈

- **运行时**：Node.js >= 18.0.0
- **WebSocket 库**：ws ^8.18.3（唯一生产依赖）
- **传输协议**：WSS（TLS 加密），回退 WS（开发环境）
- **进程管理**：pm2（生产环境）
- **数据持久化**：JSON 文件（`trust_relationships.json`）

### 整体设计

```
┌─────────────────────────────────────────────────────────┐
│                    中继服务器 (Node.js)                    │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ HTTPS Server │  │  WSS Server  │  │  数据持久化层   │  │
│  │  (SSL/TLS)   │──│  (ws 库)     │  │  (JSON 文件)   │  │
│  └──────────────┘  └──────┬───────┘  └───────────────┘  │
│                           │                              │
│  ┌────────────────────────┼────────────────────────┐    │
│  │              内存数据结构                         │    │
│  │  connectedDevices (Map) — 在线设备连接池          │    │
│  │  pairingCodes (Map)    — 临时配对码缓存          │    │
│  │  db (Object)           — 持久化数据内存副本       │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
         │                              │
    WSS 连接                       WSS 连接
         │                              │
┌────────┴────────┐          ┌──────────┴──────────┐
│  Mac 桌面端       │          │  iPhone 移动端       │
│  role: "server"  │          │  role: "client"     │
└─────────────────┘          └─────────────────────┘
```

### 核心数据结构（内存）

| 数据结构 | 类型 | Key | Value | 用途 |
|----------|------|-----|-------|------|
| `connectedDevices` | `Map` | `deviceId` | `{ ws, role, deviceName, lastHeartbeat, idleTime }` | 维护所有在线设备的 WebSocket 连接 |
| `pairingCodes` | `Map` | `code`（6位配对码） | `{ deviceId, expiresAt }` | 临时存储配对码，1 分钟过期 |
| `db` | `Object` | — | `{ devices: {}, pairings: [] }` | 持久化数据的内存副本 |

---

## 完整消息协议表

### 客户端 → 服务器（上行消息）

| 消息类型 | 方向 | 字段 | 说明 | 示例 |
|----------|------|------|------|------|
| `register` | 客户端 → 中继 | `type`, `deviceId`, `role`, `deviceName`, `idleTime` | 设备注册。`role` 为 `"server"`（Mac）或 `"client"`（iPhone）。`idleTime` 为设备闲置时间（秒） | `{"type":"register","deviceId":"abc123","role":"server","deviceName":"My Mac","idleTime":0}` |
| `discover` | client → 中继 | `type` | iPhone 请求发现所有在线的 Mac 设备 | `{"type":"discover"}` |
| `relay` | 客户端 → 中继 | `type`, `from`, `to`, `data` | 请求转发消息给目标设备。`data` 为任意业务数据 | `{"type":"relay","from":"phone1","to":"mac1","data":{"action":"input","text":"hello"}}` |
| `heartbeat` | 客户端 → 中继 | `type`, `idleTime` | 心跳包，维持连接活跃。可附带 `idleTime` 上报闲置时间 | `{"type":"heartbeat","idleTime":120}` |
| `register_code` | server → 中继 | `type`, `code` | Mac 端注册一个 6 位配对码，有效期 60 秒 | `{"type":"register_code","code":"123456"}` |
| `verify_code` | client → 中继 | `type`, `code`, `from`, `deviceName` | iPhone 端验证配对码。`from` 为自身 deviceId | `{"type":"verify_code","code":"123456","from":"phone1","deviceName":"My iPhone"}` |
| `unpair_device` | 客户端 → 中继 | `type`, `targetDeviceId` | 解除与目标设备的配对关系（软删除） | `{"type":"unpair_device","targetDeviceId":"mac1"}` |
| `sync_trust_list` | 客户端 → 中继 | `type` | 请求同步当前设备的信任设备列表 | `{"type":"sync_trust_list"}` |
| `set_device_alias` | 客户端 → 中继 | `type`, `targetDeviceId`, `alias` | 为配对设备设置备注名。`alias` 为空则清除备注 | `{"type":"set_device_alias","targetDeviceId":"mac1","alias":"办公室电脑"}` |
| `check_online_status` | 客户端 → 中继 | `type`, `deviceIds` | 批量查询指定设备的在线状态 | `{"type":"check_online_status","deviceIds":["mac1","mac2"]}` |

### 服务器 → 客户端（下行消息）

| 消息类型 | 方向 | 字段 | 说明 | 示例 |
|----------|------|------|------|------|
| `connected` | 中继 → 客户端 | `type`, `message` | 连接建立后的欢迎消息 | `{"type":"connected","message":"已连接到中继服务器"}` |
| `registered` | 中继 → 客户端 | `type`, `deviceId`, `role` | 注册成功确认 | `{"type":"registered","deviceId":"abc123","role":"server"}` |
| `server_list` | 中继 → client | `type`, `servers[]` | 返回在线 Mac 列表。每项含 `deviceId`, `deviceName`, `online`, `idleTime` | `{"type":"server_list","servers":[{"deviceId":"mac1","deviceName":"My Mac","online":true,"idleTime":0}]}` |
| `relay` | 中继 → 客户端 | `type`, `from`, `data` | 转发的消息。`from` 为发送方 deviceId，`data` 为原始业务数据 | `{"type":"relay","from":"phone1","data":{"action":"input","text":"hello"}}` |
| `heartbeat_ack` | 中继 → 客户端 | `type` | 心跳确认 | `{"type":"heartbeat_ack"}` |
| `code_registered` | 中继 → server | `type`, `code` | 配对码注册成功 | `{"type":"code_registered","code":"123456"}` |
| `code_conflict` | 中继 → server | `type`, `code` | 配对码冲突，已被其他设备占用 | `{"type":"code_conflict","code":"123456"}` |
| `pairing_success` | 中继 → client | `type`, `server{}` | 配对成功，返回配对的 Mac 信息。含 `deviceId`, `deviceName`, `role` | `{"type":"pairing_success","server":{"deviceId":"mac1","deviceName":"My Mac","role":"server"}}` |
| `pairing_completed` | 中继 → server | `type`, `client{}` | 通知 Mac 端配对完成。含 `deviceId`, `deviceName`, `role` | `{"type":"pairing_completed","client":{"deviceId":"phone1","deviceName":"My iPhone","role":"client"}}` |
| `pairing_error` | 中继 → client | `type`, `message` | 配对失败，附带错误原因 | `{"type":"pairing_error","message":"配对码无效或已过期"}` |
| `unpair_success` | 中继 → 客户端 | `type` | 解除配对成功确认 | `{"type":"unpair_success"}` |
| `device_unpaired` | 中继 → 客户端 | `type`, `from`, `deviceName` | 通知被解除配对的对方设备 | `{"type":"device_unpaired","from":"phone1","deviceName":"My iPhone"}` |
| `trust_list` | 中继 → 客户端 | `type`, `devices[]` | 返回信任设备列表。每项含 `id`, `name`, `customName`, `role`, `pairedAt` | `{"type":"trust_list","devices":[{"id":"mac1","name":"My Mac","customName":null,"role":"server","pairedAt":"2025-01-01T00:00:00Z"}]}` |
| `alias_updated` | 中继 → 客户端 | `type` | 设备备注名更新成功 | `{"type":"alias_updated"}` |
| `online_status_result` | 中继 → 客户端 | `type`, `devices[]` | 返回设备在线状态。每项含 `deviceId`, `online` | `{"type":"online_status_result","devices":[{"deviceId":"mac1","online":true}]}` |
| `error` | 中继 → 客户端 | `type`, `message` | 通用错误消息 | `{"type":"error","message":"目标设备离线"}` |

### 广播消息（服务器主动推送）

| 消息类型 | 方向 | 字段 | 说明 | 示例 |
|----------|------|------|------|------|
| `client_online` | 中继 → 所有 server | `type`, `clientId` | iPhone 上线通知，广播给所有在线 Mac | `{"type":"client_online","clientId":"phone1"}` |
| `server_online` | 中继 → 所有 client | `type`, `serverId`, `serverName` | Mac 上线通知，广播给所有在线 iPhone | `{"type":"server_online","serverId":"mac1","serverName":"My Mac"}` |
| `client_offline` | 中继 → 所有 server | `type`, `clientId` | iPhone 离线通知（断开或心跳超时） | `{"type":"client_offline","clientId":"phone1"}` |
| `server_offline` | 中继 → 所有 client | `type`, `serverId` | Mac 离线通知（断开或心跳超时） | `{"type":"server_offline","serverId":"mac1"}` |
| `client_heartbeat` | 中继 → 所有 server | `type`, `clientId` | iPhone 心跳转发，通知 Mac 端该客户端仍活跃 | `{"type":"client_heartbeat","clientId":"phone1"}` |

---

## 配对流程时序图

```
  iPhone (client)              中继服务器                Mac (server)
       │                          │                          │
       │                          │    1. register (server)   │
       │                          │◄─────────────────────────│
       │                          │    registered             │
       │                          │─────────────────────────►│
       │                          │                          │
       │                          │    2. register_code       │
       │                          │◄─────────────────────────│
       │                          │    code_registered        │
       │                          │─────────────────────────►│
       │                          │                          │
       │  3. register (client)    │                          │
       │─────────────────────────►│                          │
       │    registered            │                          │
       │◄─────────────────────────│                          │
       │                          │    client_online          │
       │                          │─────────────────────────►│
       │                          │                          │
       │  4. verify_code          │                          │
       │─────────────────────────►│                          │
       │                          │  [验证配对码]              │
       │                          │  [写入配对关系到数据库]     │
       │                          │                          │
       │    pairing_success       │                          │
       │◄─────────────────────────│                          │
       │                          │    pairing_completed      │
       │                          │─────────────────────────►│
       │                          │                          │
       │  ✅ 配对完成，双方可互相转发消息                       │
       │                          │                          │
```

### 配对码生命周期

1. Mac 端生成 6 位配对码，通过 `register_code` 注册到中继服务器
2. 配对码有效期 60 秒，存储在内存 `pairingCodes` Map 中
3. 如果配对码已被其他设备占用，返回 `code_conflict`
4. iPhone 端输入配对码，通过 `verify_code` 验证
5. 验证成功后：写入持久化数据库、通知双方、删除配对码
6. 过期配对码由每分钟定时器自动清理

### deviceId 后缀处理

客户端可能使用带业务通道后缀的 deviceId（如 `abc123_sync`、`abc123_ctrl`）。在写入数据库时，服务器会通过 `split('_')[0]` 剥离后缀，确保数据库中存储的是基础 ID。

---

## 消息转发流程时序图

```
  iPhone (client)              中继服务器                Mac (server)
       │                          │                          │
       │  relay                   │                          │
       │  {from:"phone1",         │                          │
       │   to:"mac1",             │                          │
       │   data:{...}}            │                          │
       │─────────────────────────►│                          │
       │                          │  [查找 mac1 连接]         │
       │                          │  [检查 ws.readyState]     │
       │                          │                          │
       │                          │    relay                  │
       │                          │    {from:"phone1",        │
       │                          │     data:{...}}           │
       │                          │─────────────────────────►│
       │                          │                          │

  ── 目标设备离线的情况 ──

       │  relay                   │                          │
       │  {to:"mac1_offline"}     │                          │
       │─────────────────────────►│                          │
       │                          │  [mac1_offline 不在线]    │
       │    error                 │                          │
       │    "目标设备离线"          │                          │
       │◄─────────────────────────│                          │
       │                          │                          │
```

### 转发机制说明

- 转发不强制校验配对关系（代码注释中提到"暂时不强制检查数据库，为了性能"）
- 仅检查目标设备是否在线且 WebSocket 连接状态为 `OPEN`
- 转发时 `to` 字段被剥离，接收方只看到 `from` 和 `data`

---

## 心跳与在线状态管理

### 心跳机制

| 参数 | 值 | 说明 |
|------|-----|------|
| `HEARTBEAT_INTERVAL` | 30000ms (30秒) | 心跳检查间隔 |
| 超时阈值 | 120000ms (120秒) | `HEARTBEAT_INTERVAL * 4`，即连续 4 次未收到心跳判定超时 |
| 配对码有效期 | 60000ms (60秒) | 配对码注册后 1 分钟过期 |

### 心跳流程

```
  客户端                       中继服务器
    │                            │
    │  heartbeat {idleTime: N}   │
    │───────────────────────────►│  更新 lastHeartbeat = Date.now()
    │                            │  更新 idleTime
    │    heartbeat_ack           │  如果是 client，广播 client_heartbeat 给所有 server
    │◄───────────────────────────│
    │                            │
```

### 超时清理（每 30 秒执行）

1. 遍历 `connectedDevices`，检查 `Date.now() - lastHeartbeat > 120000`
2. 校验当前 Map 中的 ws 实例是否与待清理的一致（防止竞争条件）
3. 广播离线通知（`server_offline` 或 `client_offline`）
4. 调用 `ws.close(1000, 'Heartbeat timeout')` 优雅关闭连接
5. 从 `connectedDevices` 中删除

### 在线状态查询

客户端可通过 `check_online_status` 消息批量查询设备在线状态，服务器直接查询 `connectedDevices` Map 返回结果。

### 定时状态输出（每 60 秒）

- 输出当前在线设备数量
- 清理过期的配对码

---

## 数据持久化结构

文件路径：`relay-server/trust_relationships.json`

### 完整结构

```json
{
  "devices": {
    "<deviceId>": {
      "name": "设备名称",
      "role": "server | client",
      "platform": "平台标识",
      "firstSeen": "2025-01-01T00:00:00.000Z",
      "lastSeen": "2025-01-01T12:00:00.000Z",
      "customName": "用户设置的备注名（可选）"
    }
  },
  "pairings": [
    {
      "id": "uuid-v4",
      "deviceA": "设备A的deviceId（基础ID，不含后缀）",
      "deviceB": "设备B的deviceId（基础ID，不含后缀）",
      "status": "active | revoked",
      "createdAt": "2025-01-01T00:00:00.000Z",
      "updatedAt": "2025-01-01T00:00:00.000Z",
      "revokedBy": "发起撤销的deviceId（仅 revoked 状态存在）"
    }
  ]
}
```

### 字段说明

**devices 对象**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | string | 设备名称，注册时由客户端提供 |
| `role` | string | `"server"`（Mac）或 `"client"`（iPhone） |
| `platform` | string | 平台标识，注册时提供（实际代码中未见客户端传递此字段，默认 `"unknown"`） |
| `firstSeen` | ISO 8601 | 设备首次注册时间 |
| `lastSeen` | ISO 8601 | 设备最后一次注册/更新时间 |
| `customName` | string | 用户通过 `set_device_alias` 设置的备注名，可选字段 |

**pairings 数组**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | UUID v4 | 配对记录唯一标识 |
| `deviceA` | string | 配对设备 A 的基础 deviceId |
| `deviceB` | string | 配对设备 B 的基础 deviceId |
| `status` | string | `"active"`（有效）或 `"revoked"`（已撤销，软删除） |
| `createdAt` | ISO 8601 | 配对创建时间 |
| `updatedAt` | ISO 8601 | 最后更新时间 |
| `revokedBy` | string | 发起撤销操作的 deviceId（仅撤销时写入） |

### 持久化时机

以下操作会触发 `saveDb()` 写入文件：
- 设备注册/更新（`updateDevice`）
- 创建配对（`addPairing`）
- 撤销配对（`removePairing`）
- 设置设备备注（`set_device_alias`）

### 数据迁移

`loadDb()` 包含简单的迁移逻辑：如果加载的 JSON 不包含 `devices` 对象或 `pairings` 不是数组，则重置为空数据库。

---

## 部署配置

### 服务器信息

| 配置项 | 值 |
|--------|-----|
| 服务器 IP | `<your-server-ip>` |
| 服务器用户 | `ubuntu` |
| 远程目录 | `/home/ubuntu/relay-server` |
| 域名 | `nextypeapi.yuanfengai.cn` |
| WSS 端口 | `8443`（生产环境，TLS 加密） |
| WS 端口 | `8080`（开发环境回退，无加密） |
| 进程管理 | pm2，进程名 `nextype-relay` |
| Node.js 要求 | >= 18.0.0 |

### SSL 证书

| 配置项 | 路径 |
|--------|------|
| 证书文件 | `relay-server/ssl/fullchain.pem` |
| 私钥文件 | `relay-server/ssl/privkey.pem` |

服务器启动时尝试读取 SSL 证书创建 HTTPS + WSS 服务器。如果证书文件不存在或读取失败，自动回退到非加密 WS 模式（端口 8080），适用于本地开发。

### 部署流程（`deploy.sh`）

1. 通过 `sshpass` + `scp` 上传 `server.js` 和 `package.json` 到远程服务器
2. 通过 SSH 执行 `pm2 restart nextype-relay` 重启服务
3. 不会覆盖 `trust_relationships.json`，保留已有配对数据

### 管理工具（`manage.js`）

交互式命令行工具，通过 SSH 远程管理服务器数据：
- 查看设备列表和配对关系
- 删除配对关系（修改状态为 `revoked` 并重启服务）
- 清理已撤销的配对记录（物理删除）
- 查看 pm2 日志
- 重启服务

使用方式：`node manage.js`（需要本地安装 `sshpass`）

---

## 安全机制

### 传输层加密

- 生产环境使用 WSS（WebSocket over TLS），端口 8443
- SSL 证书存放在 `relay-server/ssl/` 目录，不纳入版本控制

### 配对码安全

- 配对码有效期仅 60 秒，过期自动失效
- 配对码存储在内存中，服务器重启即清空
- 配对码冲突检测：如果码已被其他设备注册且未过期，返回 `code_conflict`
- 仅 `role: "server"` 的设备可以注册配对码

### 连接管理

- 心跳超时机制：120 秒无心跳自动断开，防止僵尸连接
- 连接竞争保护：断开事件处理时校验 ws 实例一致性，防止新连接被误删
- 超时清理时同样校验 ws 实例，避免竞争条件

### 当前安全局限

- **消息转发未强制校验配对关系**：`relay` 消息处理中注释说明"暂时不强制检查数据库，为了性能"，意味着任何知道目标 deviceId 的设备都可以发送消息
- **无身份认证机制**：设备注册时不验证 deviceId 的合法性，任何客户端可以声称任意 deviceId
- **部署脚本明文密码**：`deploy.sh` 和 `manage.js` 中硬编码了服务器密码

---

## 废弃与未使用代码

- **[未使用字段]** `relay-server/server.js:86` — `updateDevice` 函数中 `platform` 字段始终为 `"unknown"`
  - 原因：`register` 消息中客户端未传递 `platform` 字段，该字段在 `updateDevice` 中通过 `info.platform || 'unknown'` 赋值，但调用处 `updateDevice(deviceId, { name: message.deviceName, role: role })` 从未传入 `platform`
  - 建议：要么在客户端注册时增加 `platform` 字段上报，要么从数据结构中移除该字段

- **[残留代码]** `relay-server/server.js:306-307` — `relay` 消息处理中的配对关系校验注释
  - 原因：注释提到"严格模式下应该检查 `getTrustedDevices`"，但实际未实现任何校验逻辑，属于待完成的安全功能
  - 建议：实现配对关系校验，或至少添加可配置的开关

- **[未使用依赖]** `relay-server/server.js:5` — `const crypto = require('crypto')`
  - 原因：`crypto` 模块仅在 `addPairing` 函数中通过 `crypto.randomUUID()` 使用。模块本身有使用，但 `randomUUID()` 在 Node.js >= 19 中也可通过全局 `globalThis.crypto.randomUUID()` 调用，此处引入整个 `crypto` 模块是合理的，不算废弃
  - 状态：正常使用，无需处理

- **[已修复]** `relay-server/deploy.sh` 和 `relay-server/manage.js` — 已移除硬编码密码
  - 现在使用 SSH 密钥认证（`~/.ssh/nextype.pem`）
  - 服务器 IP 和用户名保留在配置中（非敏感信息）
