# 落笔 Nextype — 官方网站

产品介绍页，驱动用户下载各平台客户端。

## 技术栈

- 纯 HTML + CSS，无 JS 框架依赖
- 响应式设计，移动端适配

## 本地开发

直接用浏览器打开 `index.html`，或使用任意静态文件服务器：

```bash
cd website
python3 -m http.server 8080
# 访问 http://localhost:8080
```

## 部署

```bash
# 设置环境变量
export NEXTYPE_SERVER_IP="your-server-ip"

# 执行部署
bash deploy.sh
```

## 许可证

[AGPL-3.0](../LICENSE)
