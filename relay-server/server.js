const WebSocket = require('ws');
const https = require('https');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

// ============================================================
// SSL 证书配置
// ============================================================
const SSL_CERT_PATH = path.join(__dirname, 'ssl', 'fullchain.pem');
const SSL_KEY_PATH = path.join(__dirname, 'ssl', 'privkey.pem');

// 配置
const PORT = 8443; // WSS 使用 8443 端口（普通用户可用）
const HEARTBEAT_INTERVAL = 30000; // 30秒心跳
const DATA_FILE = path.join(__dirname, 'trust_relationships.json');

// 存储已连接的设备 (内存缓存)
const connectedDevices = new Map(); // deviceId -> { ws, role, deviceName, lastHeartbeat }

// 存储配对码
const pairingCodes = new Map(); // code -> { deviceId, expiresAt }

// 数据库 (持久化)
// 结构:
// {
//   devices: { [deviceId]: { name, role, platform, lastSeen } },
//   pairings: [ { id, deviceA, deviceB, status, createdAt, updatedAt } ]
// }
let db = {
    devices: {},
    pairings: []
};

// ============================================================
// 持久化存储函数
// ============================================================

function loadDb() {
    try {
        if (fs.existsSync(DATA_FILE)) {
            const data = fs.readFileSync(DATA_FILE, 'utf8');
            const loaded = JSON.parse(data);

            // 简单的迁移逻辑：如果发现旧结构，重置为新结构
            if (!loaded.devices || !Array.isArray(loaded.pairings)) {
                console.log('⚠️ 检测到旧数据结构，重置数据库...');
                db = { devices: {}, pairings: [] };
                saveDb();
            } else {
                db = loaded;
                console.log(`📂 加载数据库: ${Object.keys(db.devices).length}个设备, ${db.pairings.length}条配对记录`);
            }
        } else {
            db = { devices: {}, pairings: [] };
            console.log('📂 未找到数据文件, 创建新数据库');
            saveDb();
        }
    } catch (error) {
        console.error('❌ 加载数据失败:', error);
        db = { devices: {}, pairings: [] };
    }
}

function saveDb() {
    fs.writeFile(DATA_FILE, JSON.stringify(db, null, 2), 'utf8', (error) => {
        if (error) {
            console.error('❌ 保存数据失败:', error);
        }
    });
}

// ============================================================
// 数据库操作函数
// ============================================================

// 更新设备信息
function updateDevice(deviceId, info) {
    if (!deviceId) return;

    if (!db.devices[deviceId]) {
        db.devices[deviceId] = {
            name: info.name || 'Unknown',
            role: info.role || 'unknown',
            platform: info.platform || 'unknown',
            firstSeen: new Date().toISOString()
        };
    }

    // 更新最后在线时间
    db.devices[deviceId].lastSeen = new Date().toISOString();
    // 更新名称（如果提供了且不为空）
    if (info.name) db.devices[deviceId].name = info.name;
    if (info.role) db.devices[deviceId].role = info.role;

    saveDb();
}

// 添加配对关系
function addPairing(deviceA, deviceB) {
    // 检查是否已存在记录
    let pairing = db.pairings.find(p =>
        (p.deviceA === deviceA && p.deviceB === deviceB) ||
        (p.deviceA === deviceB && p.deviceB === deviceA)
    );

    const now = new Date().toISOString();

    if (pairing) {
        // 已存在，更新状态为 active
        if (pairing.status !== 'active') {
            pairing.status = 'active';
            pairing.updatedAt = now;
            console.log(`♻️ 恢复配对关系: ${deviceA} <-> ${deviceB}`);
        } else {
            console.log(`ℹ️ 配对关系已存在: ${deviceA} <-> ${deviceB}`);
        }
    } else {
        // 不存在，创建新记录
        pairing = {
            id: crypto.randomUUID(),
            deviceA: deviceA,
            deviceB: deviceB,
            status: 'active',
            createdAt: now,
            updatedAt: now
        };
        db.pairings.push(pairing);
        console.log(`🆕 创建配对关系: ${deviceA} <-> ${deviceB}`);
    }

    saveDb();
}

// 移除配对关系 (软删除)
function removePairing(deviceA, deviceB) {
    const pairing = db.pairings.find(p =>
        (p.deviceA === deviceA && p.deviceB === deviceB) ||
        (p.deviceA === deviceB && p.deviceB === deviceA)
    );

    if (pairing && pairing.status === 'active') {
        pairing.status = 'revoked';
        pairing.updatedAt = new Date().toISOString();
        pairing.revokedBy = deviceA; // 记录是谁发起的删除（可选）
        saveDb();
        console.log(`🚫 撤销配对关系: ${deviceA} <-> ${deviceB}`);
        return true;
    }
    return false;
}

// 获取信任设备列表
function getTrustedDevices(deviceId) {
    // 查找所有包含该设备且状态为 active 的配对
    const activePairings = db.pairings.filter(p =>
        p.status === 'active' && (p.deviceA === deviceId || p.deviceB === deviceId)
    );

    return activePairings.map(p => {
        // 确定对方的 ID
        const targetId = p.deviceA === deviceId ? p.deviceB : p.deviceA;
        const targetInfo = db.devices[targetId] || { name: 'Unknown', role: 'unknown' };

        return {
            id: targetId,
            name: targetInfo.name,
            customName: targetInfo.customName || null,
            role: targetInfo.role,
            pairedAt: p.createdAt
        };
    });
}

// 启动时加载数据
loadDb();

// ============================================================
// 创建 HTTPS + WSS 服务器
// ============================================================
let server;
let wss;

try {
    // 读取 SSL 证书
    const sslOptions = {
        cert: fs.readFileSync(SSL_CERT_PATH),
        key: fs.readFileSync(SSL_KEY_PATH)
    };

    // 创建 HTTPS 服务器
    server = https.createServer(sslOptions);

    // 将 WebSocket 服务器附加到 HTTPS 服务器
    wss = new WebSocket.Server({ server });

    // 启动服务器
    server.listen(PORT, () => {
        console.log(`🔒 中继服务器启动 (WSS 加密) 端口 ${PORT}`);
        console.log(`📍 wss://nextypeapi.yuanfengai.cn`);
    });
} catch (error) {
    console.error('❌ 启动 WSS 服务器失败:', error.message);
    console.log('⚠️ 回退到非加密模式 (WS)...');

    // 回退到普通 WebSocket (用于开发环境)
    wss = new WebSocket.Server({ port: 8080 });
    console.log(`🚀 中继服务器启动 (WS 非加密) 端口 8080`);
}

wss.on('connection', (ws) => {
    console.log('📱 新连接建立');

    let deviceId = null;
    let role = null;

    // 发送欢迎消息
    ws.send(JSON.stringify({
        type: 'connected',
        message: '已连接到中继服务器'
    }));

    ws.on('message', (data) => {
        try {
            const message = JSON.parse(data);
            // 过滤心跳日志，避免刷屏
            if (message.type !== 'heartbeat') {
                console.log(`📥 收到消息:`, message.type, `from ${deviceId || 'unknown'}`);
            }

            switch (message.type) {
                case 'register':
                    // 设备注册
                    deviceId = message.deviceId;
                    role = message.role; // 'server'(Mac) 或 'client'(iPhone)

                    if (!deviceId) {
                        console.error('❌ 注册失败: 缺少deviceId');
                        return;
                    }

                    // 更新内存中的连接信息
                    connectedDevices.set(deviceId, {
                        ws: ws,
                        role: role,
                        deviceName: message.deviceName || deviceId,
                        lastHeartbeat: Date.now(),
                        idleTime: typeof message.idleTime === 'number' ? message.idleTime : 999999
                    });

                    // 更新数据库中的设备信息
                    updateDevice(deviceId, {
                        name: message.deviceName,
                        role: role
                    });

                    console.log(`✅ 设备注册: ${deviceId} (${role}) - ${message.deviceName}`);

                    ws.send(JSON.stringify({
                        type: 'registered',
                        deviceId: deviceId,
                        role: role
                    }));

                    // 如果是phone注册，只通知与之有配对关系的Mac
                    if (role === 'client') {
                        notifyPairedServers(deviceId, {
                            type: 'client_online',
                            clientId: deviceId,
                            deviceName: message.deviceName || deviceId
                        });
                    } else if (role === 'server') {
                        notifyPairedClients(deviceId, {
                            type: 'server_online',
                            serverId: deviceId,
                            serverName: message.deviceName
                        });

                        // Mac 重连后，告知其哪些配对手机已在线
                        const baseServerId = deviceId.split('_')[0];
                        const pairedDevices = getTrustedDevices(baseServerId);
                        for (const paired of pairedDevices) {
                            const clientSession = connectedDevices.get(paired.id);
                            if (clientSession && clientSession.role === 'client' && clientSession.ws.readyState === WebSocket.OPEN) {
                                ws.send(JSON.stringify({
                                    type: 'client_online',
                                    clientId: paired.id,
                                    deviceName: clientSession.deviceName
                                }));
                                console.log(`📤 通知 Mac 已在线的手机: ${paired.id} (${clientSession.deviceName})`);
                            }
                        }
                    }
                    break;

                case 'discover':
                    // iPhone请求发现在线的Mac
                    const onlineServers = [];
                    for (const [id, device] of connectedDevices) {
                        if (device.role === 'server') {
                            onlineServers.push({
                                deviceId: id,
                                deviceName: device.deviceName,
                                online: true,
                                idleTime: typeof device.idleTime === 'number' ? device.idleTime : 999999
                            });
                        }
                    }

                    ws.send(JSON.stringify({
                        type: 'server_list',
                        servers: onlineServers
                    }));

                    console.log(`📋 返回在线服务器列表: ${onlineServers.length}个`);
                    break;

                case 'relay':
                    // 转发消息
                    // 验证是否允许转发：必须是配对设备
                    // 这里暂时不强制检查数据库，为了性能，或者可以加缓存。
                    // 严格模式下应该检查 getTrustedDevices(message.from).includes(message.to)

                    const targetDevice = connectedDevices.get(message.to);
                    if (targetDevice && targetDevice.ws.readyState === WebSocket.OPEN) {
                        targetDevice.ws.send(JSON.stringify({
                            type: 'relay',
                            from: message.from,
                            data: message.data
                        }));
                        console.log(`📤 转发消息: ${message.from} -> ${message.to}`);
                    } else {
                        ws.send(JSON.stringify({
                            type: 'error',
                            message: '目标设备离线'
                        }));
                        console.log(`❌ 目标设备离线: ${message.to}`);
                    }
                    break;

                case 'heartbeat':
                    // 心跳响应
                    if (deviceId && connectedDevices.has(deviceId)) {
                        const device = connectedDevices.get(deviceId);
                        device.lastHeartbeat = Date.now();
                        // 记录上报的闲置时间
                        if (typeof message.idleTime === 'number') {
                            device.idleTime = message.idleTime;
                        }
                        // 通知 PC 端：该客户端仍然活跃
                        if (device.role === 'client') {
                            broadcastToServers({ type: 'client_heartbeat', clientId: deviceId });
                        }
                    }
                    ws.send(JSON.stringify({ type: 'heartbeat_ack' }));
                    break;

                case 'register_code':
                    // Mac注册配对码
                    if (role !== 'server') {
                        return;
                    }
                    const code = message.code;
                    if (code) {
                        // 检查配对码是否已被其他设备注册
                        const existingRecord = pairingCodes.get(code);
                        if (existingRecord && existingRecord.expiresAt > Date.now() && existingRecord.deviceId !== deviceId) {
                            // 配对码已被其他设备占用
                            console.log(`⚠️ 配对码冲突: ${code} 已被 ${existingRecord.deviceId} 占用`);
                            ws.send(JSON.stringify({ type: 'code_conflict', code: code }));
                        } else {
                            // 注册成功（新配对码或替换自己之前的配对码）
                            pairingCodes.set(code, {
                                deviceId: deviceId,
                                expiresAt: Date.now() + 60000, // 1分钟过期
                                encryptionKey: message.encryptionKey
                            });
                            console.log(`🔢 注册配对码: ${code} -> ${deviceId}`);
                            ws.send(JSON.stringify({ type: 'code_registered', code: code }));
                        }
                    }
                    break;

                case 'verify_code':
                    // 手机验证配对码
                    const verifyCode = message.code;
                    const record = pairingCodes.get(verifyCode);

                    if (record && record.expiresAt > Date.now()) {
                        const targetServer = connectedDevices.get(record.deviceId);
                        if (targetServer) {
                            console.log(`✅ 配对码验证成功: ${verifyCode} -> ${targetServer.deviceName}`);

                            // 💡 修复：确保存入数据库的是基础 ID (不带后缀)
                            const baseFromId = message.from.split('_')[0];
                            const baseToId = record.deviceId.split('_')[0];

                            // 1. 记录设备信息
                            updateDevice(baseFromId, { name: message.deviceName, role: 'client' });

                            // 2. 建立配对关系
                            addPairing(baseFromId, baseToId);

                            // 3. 通知手机端配对成功（透传 encryptionKey）
                            ws.send(JSON.stringify({
                                type: 'pairing_success',
                                server: {
                                    deviceId: baseToId,
                                    deviceName: targetServer.deviceName,
                                    role: 'server',
                                    encryptionKey: record.encryptionKey
                                }
                            }));

                            // 4. 通知PC端配对完成
                            if (targetServer.ws.readyState === WebSocket.OPEN) {
                                targetServer.ws.send(JSON.stringify({
                                    type: 'pairing_completed',
                                    client: {
                                        deviceId: baseFromId,
                                        deviceName: message.deviceName || 'Unknown Device',
                                        role: 'client'
                                    }
                                }));
                                console.log(`📤 通知PC端配对完成: ${baseToId}`);
                            }

                            // 清除配对码
                            pairingCodes.delete(verifyCode);

                        } else {
                            ws.send(JSON.stringify({ type: 'pairing_error', message: '目标设备已离线' }));
                        }
                    } else {
                        console.log(`❌ 配对码无效或过期: ${verifyCode}`);
                        ws.send(JSON.stringify({ type: 'pairing_error', message: '配对码无效或已过期' }));
                    }
                    break;

                case 'unpair_device':
                    // 设备解除配对
                    const unpairTargetId = message.targetDeviceId.split('_')[0];
                    const initiatorId = deviceId.split('_')[0];
                    console.log(`💔 收到解除配对请求: Initiator=${initiatorId}, Target=${unpairTargetId}`);

                    if (initiatorId && unpairTargetId) {
                        // 执行解除 (软删除)
                        removePairing(initiatorId, unpairTargetId);

                        // 通知对方设备(如果在线)
                        // 注意：这里可能需要通知多个通道，或者通知主通道
                        const targetDevice = connectedDevices.get(unpairTargetId);
                        if (targetDevice && targetDevice.ws.readyState === WebSocket.OPEN) {
                            targetDevice.ws.send(JSON.stringify({
                                type: 'device_unpaired',
                                from: initiatorId,
                                deviceName: connectedDevices.get(deviceId)?.deviceName || initiatorId
                            }));
                            console.log(`📤 已通知设备解除: ${unpairTargetId}`);
                        } else {
                            console.log(`⚠️ 目标设备不在线，无法发送实时通知: ${unpairTargetId}`);
                        }

                        ws.send(JSON.stringify({ type: 'unpair_success' }));
                    } else {
                        console.error('❌ 解除配对失败: 缺少设备ID', { initiatorId, unpairTargetId });
                    }
                    break;

                case 'sync_trust_list':
                    // 同步信任列表
                    if (deviceId) {
                        // 💡 修复：如果 deviceId 带有业务通道后缀（_sync, _ctrl），剥离后缀后再查询数据库
                        // 因为数据库里存的是原始 deviceId
                        const baseDeviceId = deviceId.split('_')[0];
                        const trustedList = getTrustedDevices(baseDeviceId);
                        ws.send(JSON.stringify({
                            type: 'trust_list',
                            devices: trustedList
                        }));
                        console.log(`📋 返回信任列表: ${deviceId} (${trustedList.length}个设备)`);
                    }
                    break;

                case 'set_device_alias':
                    // 设置设备备注名
                    const aliasTargetId = message.targetDeviceId;
                    const alias = message.alias;

                    if (deviceId && aliasTargetId) {
                        // 验证发送者与目标存在活跃配对关系
                        const hasPairing = db.pairings.some(p =>
                            p.status === 'active' &&
                            ((p.deviceA === deviceId && p.deviceB === aliasTargetId) ||
                             (p.deviceA === aliasTargetId && p.deviceB === deviceId))
                        );

                        if (hasPairing && db.devices[aliasTargetId]) {
                            if (alias && alias.trim() !== '') {
                                db.devices[aliasTargetId].customName = alias.trim();
                            } else {
                                delete db.devices[aliasTargetId].customName;
                            }
                            saveDb();
                            ws.send(JSON.stringify({ type: 'alias_updated' }));
                            console.log(`✏️ 设备备注已更新: ${aliasTargetId} -> ${alias || '(cleared)'}`);
                        } else {
                            ws.send(JSON.stringify({ type: 'error', message: '无权修改该设备备注' }));
                        }
                    }
                    break;

                case 'check_online_status':
                    // 客户端请求检查指定设备列表的在线状态
                    const checkDeviceIds = message.deviceIds || [];

                    // 打印详细信息便于调试
                    console.log(`📋 查询设备ID: ${JSON.stringify(checkDeviceIds)}`);
                    console.log(`📋 当前在线设备: ${Array.from(connectedDevices.keys()).join(', ')}`);

                    const statusList = checkDeviceIds.map(id => ({
                        deviceId: id,
                        online: connectedDevices.has(id)
                    }));

                    ws.send(JSON.stringify({
                        type: 'online_status_result',
                        devices: statusList
                    }));

                    console.log(`📋 返回设备在线状态: ${statusList.filter(d => d.online).length}/${checkDeviceIds.length} 在线`);
                    break;

                default:
                    console.log(`⚠️ 未知消息类型: ${message.type}`);
            }
        } catch (error) {
            console.error('❌ 处理消息出错:', error);
            ws.send(JSON.stringify({
                type: 'error',
                message: '消息处理失败'
            }));
        }
    });

    ws.on('close', () => {
        if (deviceId) {
            // 💡 优化：只有当断开的 ws 确实是当前 map 中存储的那个 ws 时，才执行清理
            // 防止“新连接已建立并替换了 map，随后旧连接触发 close 导致误删新连接”的竞争问题
            const currentSession = connectedDevices.get(deviceId);
            if (currentSession && currentSession.ws === ws) {
                console.log(`👋 设备断开: ${deviceId}`);
                connectedDevices.delete(deviceId);

                // 通知有配对关系的设备
                if (role === 'server') {
                    notifyPairedClients(deviceId, {
                        type: 'server_offline',
                        serverId: deviceId
                    });
                } else if (role === 'client') {
                    notifyPairedServers(deviceId, {
                        type: 'client_offline',
                        clientId: deviceId
                    });
                }
            } else {
                console.log(`ℹ️ 忽略陈旧连接的断开消息: ${deviceId} (已有新连接覆盖)`);
            }
        }
    });

    ws.on('error', (error) => {
        console.error('❌ WebSocket错误:', error.message);
    });
});

// 广播到所有Mac服务端
function broadcastToServers(message) {
    for (const [id, device] of connectedDevices) {
        if (device.role === 'server' && device.ws.readyState === WebSocket.OPEN) {
            device.ws.send(JSON.stringify(message));
        }
    }
}

// 广播到所有iPhone客户端
function broadcastToClients(message) {
    for (const [id, device] of connectedDevices) {
        if (device.role === 'client' && device.ws.readyState === WebSocket.OPEN) {
            device.ws.send(JSON.stringify(message));
        }
    }
}

// 只通知与指定手机有配对关系的Mac（精准通知，避免陌生设备被推送）
function notifyPairedServers(clientId, message) {
    const baseId = clientId.split('_')[0];
    const paired = getTrustedDevices(baseId);
    const pairedIds = new Set(paired.map(d => d.id));
    for (const [id, device] of connectedDevices) {
        if (device.role === 'server' && pairedIds.has(id) && device.ws.readyState === WebSocket.OPEN) {
            device.ws.send(JSON.stringify(message));
        }
    }
}

// 只通知与指定Mac有配对关系的手机（精准通知，避免陌生设备被推送）
function notifyPairedClients(serverId, message) {
    const baseId = serverId.split('_')[0];
    const paired = getTrustedDevices(baseId);
    const pairedIds = new Set(paired.map(d => d.id));
    for (const [id, device] of connectedDevices) {
        if (device.role === 'client' && pairedIds.has(id) && device.ws.readyState === WebSocket.OPEN) {
            device.ws.send(JSON.stringify(message));
        }
    }
}

// 心跳检查 - 清理超时设备
setInterval(() => {
    const now = Date.now();
    for (const [deviceId, device] of connectedDevices) {
        if (now - device.lastHeartbeat > HEARTBEAT_INTERVAL * 4) {  // 120秒超时（30秒 * 4）
            // 💡 再次校验，确保执行关闭前没有被新连接替换（虽然异步场景下概率低，但加个保险）
            const currentSession = connectedDevices.get(deviceId);
            if (currentSession && currentSession.ws === device.ws) {
                console.log(`⏰ 设备超时，移除: ${deviceId}`);
                // 只通知有配对关系的设备
                if (device.role === 'server') {
                    notifyPairedClients(deviceId, { type: 'server_offline', serverId: deviceId });
                } else if (device.role === 'client') {
                    notifyPairedServers(deviceId, { type: 'client_offline', clientId: deviceId });
                }
                device.ws.close(1000, 'Heartbeat timeout');  // 优雅关闭，触发客户端重连
                connectedDevices.delete(deviceId);
            }
        }
    }
}, HEARTBEAT_INTERVAL);

// 定期输出状态
setInterval(() => {
    console.log(`📊 在线设备: ${connectedDevices.size}个`);

    // 清理过期配对码
    const now = Date.now();
    for (const [code, record] of pairingCodes) {
        if (now > record.expiresAt) {
            pairingCodes.delete(code);
        }
    }
}, 60000); // 每分钟输出一次

// ============================================================
// 每日自动备份
// ============================================================
const BACKUP_DIR = path.join(__dirname, 'backups');

function backupDb() {
    try {
        if (!fs.existsSync(BACKUP_DIR)) {
            fs.mkdirSync(BACKUP_DIR);
        }
        const date = new Date().toISOString().slice(0, 10); // YYYY-MM-DD
        const backupFile = path.join(BACKUP_DIR, `trust_relationships_${date}.json`);
        fs.copyFileSync(DATA_FILE, backupFile);
        console.log(`📦 数据库已备份: ${backupFile}`);

        // 只保留最近 7 天的备份
        const files = fs.readdirSync(BACKUP_DIR)
            .filter(f => f.startsWith('trust_relationships_') && f.endsWith('.json'))
            .sort();
        while (files.length > 7) {
            const old = files.shift();
            fs.unlinkSync(path.join(BACKUP_DIR, old));
            console.log(`🗑️ 删除过期备份: ${old}`);
        }
    } catch (error) {
        console.error('❌ 备份失败:', error);
    }
}

// 启动时立即备份一次
backupDb();
// 每 24 小时备份一次
setInterval(backupDb, 24 * 60 * 60 * 1000);

// 错误处理
process.on('uncaughtException', (error) => {
    console.error('💥 未捕获的异常:', error);
});

process.on('unhandledRejection', (reason, promise) => {
    console.error('💥 未处理的Promise拒绝:', reason);
});
