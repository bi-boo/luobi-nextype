//
//  AboutView.swift
//  NextypeApp
//
//  产品介绍页 - 配对前入口（EmptyStateView → "这个 App 是做什么的？"）

import SwiftUI

struct AboutView: View {
    var body: some View {
        List {
            // 落笔能做什么
            Section {
                scenarioRow(icon: "keyboard", title: "替代电脑键盘", description: "手机输入的内容实时出现在电脑光标位置，无需复制粘贴或切换窗口")
                scenarioRow(icon: "slider.horizontal.3", title: "搭配任意输入工具", description: "落笔只负责同步内容，不做语音识别——系统自带语音、任何输入法都可以")
            } header: {
                Text("落笔能做什么")
            }

            // 适合什么场景
            Section {
                scenarioRow(icon: "bubble.left.and.bubble.right", title: "与 AI 对话", description: "语音提问、持续追问，手机可以贴近嘴边，轻声说话也能识别，不打扰周围人")
                scenarioRow(icon: "pencil.and.outline", title: "内容创作", description: "口述初稿、记录灵感、会议要点，随时拿起手机说出来")
            } header: {
                Text("适合什么场景")
            }

            // 怎么开始
            Section {
                scenarioRow(icon: "1.circle.fill", title: "下载电脑端", description: "前往 yuanfengai.cn 下载并安装")
                scenarioRow(icon: "2.circle.fill", title: "输入配对码", description: "在电脑端获取配对码，手机输入即可完成配对")
            } header: {
                Text("怎么开始")
            }

            // 隐私与安全
            Section {
                Label("无需登录，不收集任何个人信息", systemImage: "hand.raised")
                    .font(.subheadline)
                Label("服务器不保留任何传输内容，仅存储设备配对信息", systemImage: "lock.shield")
                    .font(.subheadline)
                Label("本项目已在 GitHub 开源，欢迎审查代码", systemImage: "checkmark.seal")
                    .font(.subheadline)
            } header: {
                Text("隐私与安全")
            }
        }
        .navigationTitle("关于落笔")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private func scenarioRow(icon: String, title: String, description: String) -> some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: icon)
                .foregroundStyle(Color.accentColor)
                .frame(width: 22, alignment: .center)
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.subheadline).bold()
                Text(description)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.vertical, 2)
    }


}

#Preview {
    NavigationStack {
        AboutView()
    }
}
