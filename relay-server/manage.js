#!/usr/bin/env node
/**
 * 中继服务器管理工具
 * 用于查看和管理服务器上的配对关系
 * 
 * 使用方法：node manage.js
 */

const { execSync, exec } = require('child_process');
const readline = require('readline');

// 服务器配置
const CONFIG = {
    host: process.env.NEXTYPE_SERVER_IP || 'your-server-ip',
    user: process.env.NEXTYPE_SERVER_USER || 'ubuntu',
    keyPath: process.env.NEXTYPE_SSH_KEY || '~/.ssh/nextype.pem',
    remotePath: '/home/ubuntu/relay-server/trust_relationships.json'
};

// 颜色输出
const colors = {
    reset: '\x1b[0m',
    bright: '\x1b[1m',
    green: '\x1b[32m',
    yellow: '\x1b[33m',
    blue: '\x1b[34m',
    red: '\x1b[31m',
    cyan: '\x1b[36m'
};

function log(msg, color = 'reset') {
    console.log(`${colors[color]}${msg}${colors.reset}`);
}

// 执行远程命令
function remoteExec(command) {
    const sshCmd = `ssh -i ${CONFIG.keyPath} -o StrictHostKeyChecking=no ${CONFIG.user}@${CONFIG.host} "${command}"`;
    try {
        return execSync(sshCmd, { encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] });
    } catch (error) {
        throw new Error(`SSH 命令失败: ${error.message}`);
    }
}

// 获取服务器数据
function fetchData() {
    log('\n🔄 正在连接服务器...', 'cyan');
    const content = remoteExec(`cat ${CONFIG.remotePath}`);
    return JSON.parse(content);
}

// 保存数据到服务器
function saveData(data) {
    const jsonStr = JSON.stringify(data, null, 2).replace(/"/g, '\\"');
    remoteExec(`echo "${jsonStr}" > ${CONFIG.remotePath}`);
    log('✅ 数据已保存到服务器', 'green');
}

// 显示设备列表
function showDevices(db) {
    console.log('\n' + '═'.repeat(50));
    log('📱 设备列表', 'bright');
    console.log('═'.repeat(50));

    const devices = Object.entries(db.devices);
    if (devices.length === 0) {
        log('  (暂无设备)', 'yellow');
        return;
    }

    devices.forEach(([id, info], index) => {
        const roleIcon = info.role === 'server' ? '💻' : '📱';
        const shortId = id.length > 20 ? id.substring(0, 20) + '...' : id;
        log(`  ${index + 1}. ${roleIcon} ${info.name || '未命名'}`, 'green');
        log(`     ID: ${shortId}`, 'cyan');
        log(`     角色: ${info.role || 'unknown'} | 最后在线: ${info.lastSeen ? new Date(info.lastSeen).toLocaleString('zh-CN') : '未知'}`);
    });
}

// 显示配对关系
function showPairings(db) {
    console.log('\n' + '═'.repeat(50));
    log('🔗 配对关系', 'bright');
    console.log('═'.repeat(50));

    const activePairings = db.pairings.filter(p => p.status === 'active');
    const revokedPairings = db.pairings.filter(p => p.status === 'revoked');

    if (activePairings.length === 0) {
        log('  (暂无有效配对)', 'yellow');
    } else {
        log(`\n✅ 有效配对 (${activePairings.length}条):`, 'green');
        activePairings.forEach((p, index) => {
            const deviceA = db.devices[p.deviceA]?.name || p.deviceA.substring(0, 15) + '...';
            const deviceB = db.devices[p.deviceB]?.name || p.deviceB.substring(0, 15) + '...';
            const roleA = db.devices[p.deviceA]?.role === 'server' ? '💻' : '📱';
            const roleB = db.devices[p.deviceB]?.role === 'server' ? '💻' : '📱';

            log(`  ${index + 1}. ${roleA} ${deviceA}  ⟷  ${roleB} ${deviceB}`, 'cyan');
            log(`     配对时间: ${new Date(p.createdAt).toLocaleString('zh-CN')}`);
        });
    }

    if (revokedPairings.length > 0) {
        log(`\n❌ 已撤销 (${revokedPairings.length}条):`, 'red');
        revokedPairings.forEach((p, index) => {
            const deviceA = db.devices[p.deviceA]?.name || p.deviceA.substring(0, 15) + '...';
            const deviceB = db.devices[p.deviceB]?.name || p.deviceB.substring(0, 15) + '...';
            log(`  ${index + 1}. ${deviceA}  ⟷  ${deviceB}`, 'yellow');
        });
    }

    return activePairings;
}

// 删除配对
function deletePairing(db, index, activePairings) {
    if (index < 1 || index > activePairings.length) {
        log('❌ 无效的序号', 'red');
        return false;
    }

    const pairing = activePairings[index - 1];
    pairing.status = 'revoked';
    pairing.updatedAt = new Date().toISOString();

    saveData(db);

    // 重启服务让改动生效
    log('🔄 正在重启服务...', 'cyan');
    remoteExec('pm2 restart nextype-relay');
    log('✅ 服务已重启', 'green');

    return true;
}

// 清理已撤销的配对记录
function cleanupRevoked(db) {
    const before = db.pairings.length;
    db.pairings = db.pairings.filter(p => p.status === 'active');
    const after = db.pairings.length;

    if (before === after) {
        log('ℹ️ 没有需要清理的记录', 'yellow');
        return;
    }

    saveData(db);
    log(`✅ 已清理 ${before - after} 条已撤销的记录`, 'green');
}

// 主菜单
async function mainMenu() {
    const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout
    });

    const question = (prompt) => new Promise(resolve => rl.question(prompt, resolve));

    console.clear();
    log('╔════════════════════════════════════════════════╗', 'blue');
    log('║       🚀 Nextype 中继服务器管理工具            ║', 'blue');
    log('╚════════════════════════════════════════════════╝', 'blue');

    let db;
    try {
        db = fetchData();
    } catch (error) {
        log(`\n❌ 连接服务器失败: ${error.message}`, 'red');
        log('\n请确保:');
        log('  1. 已安装 sshpass (brew install hudochenkov/sshpass/sshpass)');
        log('  2. 网络连接正常');
        log('  3. 服务器地址和密码正确');
        rl.close();
        return;
    }

    while (true) {
        showDevices(db);
        const activePairings = showPairings(db);

        console.log('\n' + '─'.repeat(50));
        log('📋 操作菜单:', 'bright');
        log('  [1] 刷新数据');
        log('  [2] 删除配对关系');
        log('  [3] 清理已撤销记录');
        log('  [4] 查看服务器日志');
        log('  [5] 重启服务');
        log('  [0] 退出');
        console.log('─'.repeat(50));

        const choice = await question('\n请输入选项 > ');

        switch (choice.trim()) {
            case '1':
                try {
                    db = fetchData();
                    log('✅ 数据已刷新', 'green');
                } catch (error) {
                    log(`❌ 刷新失败: ${error.message}`, 'red');
                }
                break;

            case '2':
                if (activePairings.length === 0) {
                    log('ℹ️ 没有可删除的配对', 'yellow');
                } else {
                    const indexStr = await question(`请输入要删除的配对序号 (1-${activePairings.length}，输入 0 取消) > `);
                    const index = parseInt(indexStr);
                    if (index === 0) {
                        log('已取消', 'yellow');
                    } else {
                        deletePairing(db, index, activePairings);
                        db = fetchData(); // 重新加载
                    }
                }
                break;

            case '3':
                cleanupRevoked(db);
                db = fetchData();
                break;

            case '4':
                log('\n📜 最近日志:', 'cyan');
                console.log('─'.repeat(50));
                try {
                    const logs = remoteExec('pm2 logs nextype-relay --lines 20 --nostream');
                    console.log(logs);
                } catch (e) {
                    log('获取日志失败', 'red');
                }
                await question('\n按回车继续...');
                break;

            case '5':
                log('🔄 正在重启服务...', 'cyan');
                try {
                    remoteExec('pm2 restart nextype-relay');
                    log('✅ 服务重启成功', 'green');
                } catch (e) {
                    log('重启失败', 'red');
                }
                await question('\n按回车继续...');
                break;

            case '0':
            case 'q':
            case 'exit':
                log('\n👋 再见！', 'green');
                rl.close();
                return;

            default:
                log('❌ 无效选项，请重新输入', 'red');
        }

        console.clear();
        log('╔════════════════════════════════════════════════╗', 'blue');
        log('║       🚀 Nextype 中继服务器管理工具            ║', 'blue');
        log('╚════════════════════════════════════════════════╝', 'blue');
    }
}

// 启动
mainMenu().catch(console.error);
