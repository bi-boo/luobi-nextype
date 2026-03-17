# 官网分发就绪审查报告 (website/)

**审查日期**: 2026-03-11
**网站路径**: website/
**线上地址**: https://www.yuanfengai.cn
**技术方案**: 纯 HTML + CSS, 无 JS 框架

---

## 总览评级

| 维度 | 评级 | 核心发现 |
|------|------|---------|
| 下载入口 | ❌ 阻塞 | 四个平台没有任何一个能正常下载 |
| 内容完整性 | ⚠️ 需改进 | 产品介绍完整，但缺隐私政策链接和用户协议 |
| 移动端适配 | ✅ 就绪 | 三级响应式断点，流式字体缩放 |
| SEO 基础 | ❌ 阻塞 | 缺 meta description、OG 标签、favicon、robots.txt |
| 技术实现 | ⚠️ 需改进 | 代码质量良好，但 logo.png 312KB 过大，有废弃资源 |
| 法律合规 | ❌ 阻塞 | 隐私政策未链接且线上 404，缺用户协议，缺 ICP 备案 |
| 安全性 | ⚠️ 需改进 | HTTPS 正常，但缺安全响应头，deploy.sh 硬编码服务器信息 |
| 部署配置 | ⚠️ 需改进 | 手动部署，deploy.sh 遗漏 privacy.html |

---

## 重大发现: 本地版本与线上版本不同步

| 对比维度 | 线上版本（旧） | 本地版本（新） |
|---------|---------------|---------------|
| 页面大小 | 8,675 字节 | 15,285 字节 |
| 风格 | 暗色主题 + Google Fonts | 亮色主题 + 系统字体栈 |
| 内容 | 无用户评价、无操作步骤 | 有用户评价、有"如何开始" |

本地新版本按 Linear/Vercel 亮色工具风重构，但尚未部署到线上。

---

## 1. 下载入口 -- ❌ 阻塞

| 平台 | 按钮 | 链接 | 问题 |
|------|------|------|------|
| Mac | 存在（3处） | /downloads/latest-arm64.dmg | 返回 404，/downloads/ 目录不存在 |
| Windows | 存在，置灰 | 无链接 | 标注"敬请期待"，符合预期 |
| iPhone | 存在 | href="#" | 空锚点，标注 TestFlight 但无实际链接 |
| Android | 存在 | href="#" | 空锚点，标注 APK 下载但无实际地址 |

### 版本号不一致

- version.json 声明 1.0.0，下载指向 latest-arm64.dmg
- 本地 DMG 构建文件名含 2.0.0
- version.json 的 downloadUrl 与页面下载链接不同

---

## 2. 内容完整性 -- ⚠️ 需改进

### 已有

- 产品标题/副标题: "手机输入，电脑输出"
- 核心卖点: 端对端加密、配对码一键连接、无需注册
- 使用步骤: 三步引导
- 用户评价: 三条真实场景评价
- 产品示意图: SVG 内联

### 缺失

| 缺失项 | 严重程度 |
|--------|---------|
| 隐私政策链接（privacy.html 存在但未被链接） | 高 |
| 用户协议/服务条款 | 高 |
| 联系方式（具体邮箱） | 中 |
| 功能截图/详细特性 | 中 |
| FAQ | 低 |

---

## 3. 移动端适配 -- ✅ 就绪

- viewport meta 标签正确
- 三级断点: 860px / 680px / 420px
- clamp() 流式字体缩放
- overflow-x: hidden 防水平溢出
- prefers-reduced-motion 支持
- 小瑕疵: 860px 以下视觉图排在文案上方，移动端"下载按钮优先"可能更合适

---

## 4. SEO 基础 -- ❌ 阻塞

| 检查项 | 状态 |
|--------|------|
| title | ✅ |
| meta charset / viewport | ✅ |
| html lang="zh-CN" | ✅ |
| meta description | ❌ 缺失 |
| Open Graph 标签 | ❌ 全部缺失 |
| Twitter Card | ❌ 全部缺失 |
| canonical | ❌ 缺失 |
| Favicon | ❌ 缺失 |
| robots.txt | ❌ 缺失 |
| sitemap.xml | ❌ 缺失 |
| 语义化标签 | ⚠️ 有 nav/section/footer 但缺 main |
| avatar alt 文字 | ⚠️ 与显示姓名不匹配 |

---

## 5. 技术实现 -- ⚠️ 需改进

### 代码质量

- 纯 HTML + CSS，无 JS 依赖，符合设计规范
- CSS Token 设计良好（:root 变量）
- 现代特性: clamp()、grid、backdrop-filter

### 性能

| 检查项 | 状态 |
|--------|------|
| HTML 15KB | ✅ 可接受 |
| CSS 11.7KB | ✅ 可接受 |
| logo.png 312KB | ❌ 严重过大（显示 24x24px，应压缩到 <10KB） |
| avatar 图片 15-18KB/张 | ✅ 合理 |
| CSS/JS 压缩 | ❌ 未做 |
| 图片懒加载 | ❌ 未做 |

### 废弃资源

- assets/css/style.css (9.7KB) -- 旧版暗色样式，未被引用
- assets/js/main.js (4.2KB) -- 旧版脚本，未被引用
- assets/app-screenshot.jpg (33KB) -- 未被页面引用

---

## 6. 法律合规 -- ❌ 阻塞

| 检查项 | 状态 |
|--------|------|
| 隐私政策文件 | ✅ privacy.html 存在且内容完整 |
| 隐私政策链接 | ❌ index.html 中无链接指向 privacy.html |
| 隐私政策部署 | ❌ 线上 404，deploy.sh 未包含此文件 |
| 用户协议 | ❌ 完全缺失 |
| ICP 备案 | ❌ 面向中国大陆必须展示 |
| 版权年份 | ⚠️ 2025 需更新为 2026 |

---

## 7. 安全性 -- ⚠️ 需改进

| 检查项 | 状态 |
|--------|------|
| HTTPS | ✅ |
| HTTP -> HTTPS 重定向 | ✅ |
| HSTS | ❌ 缺失 |
| CSP | ❌ 缺失 |
| X-Frame-Options | ❌ 缺失 |
| X-Content-Type-Options | ❌ 缺失 |
| Nginx 版本泄露 | ⚠️ 暴露 nginx/1.24.0 |
| deploy.sh 硬编码服务器 IP 和用户名 | ⚠️ |

---

## 8. 部署配置 -- ⚠️ 需改进

| 检查项 | 状态 |
|--------|------|
| 部署脚本 | ✅ deploy.sh |
| 文件遗漏 | ❌ 未包含 privacy.html |
| 域名 | ✅ www + 裸域均可访问 |
| www/裸域统一 | ❌ 未重定向统一 |
| CDN | ❌ 无 |
| Gzip | ⚠️ 可能未启用 |
| 缓存策略 | ❌ 无 Cache-Control |
| 404 页面 | ❌ 默认 Nginx 404 |

---

## 优先行动项

### P0（必须解决）

1. **Mac 下载链接 404**: 在服务器创建 /downloads/ 并上传 DMG
2. **iPhone/Android 下载链接为空**: 配置 TestFlight 链接和 APK 下载地址
3. **隐私政策无链接且线上 404**: 页脚加链接 + deploy.sh 加入 privacy.html
4. **本地新版未部署**: 运行更新后的 deploy.sh

### P1（发布后尽快解决）

5. 补充 meta description 和 OG 标签
6. 添加 favicon
7. 创建用户协议页面
8. 压缩 logo.png（312KB -> <10KB）
9. 更新版权年份 2025 -> 2026
10. 修复 avatar alt 文字
11. 配置安全响应头

### P2（中期改进）

12. ICP 备案号
13. deploy.sh 改用环境变量（去除硬编码服务器信息）
14. www/裸域统一重定向
15. 清理废弃资源
16. 添加 robots.txt 和 sitemap.xml
17. 自定义 404 页面
