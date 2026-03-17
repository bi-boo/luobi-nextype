//
//  UsageGuideView.swift
//  NextypeApp
//
//  使用说明页面 - 对齐 Android AboutActivity 的 4 个卡片

import SwiftUI

struct UsageGuideView: View {
    var onPairTapped: (() -> Void)? = nil

    @State private var showCopiedToast = false

    var body: some View {
        List {
            // 配对说明
            Section {
                VStack(alignment: .leading, spacing: 12) {
                    HStack(alignment: .center, spacing: 12) {
                        stepBadge("1")
                        HStack(spacing: 0) {
                            Text("访问 ")
                                .font(.subheadline)
                            Button {
                                UIPasteboard.general.string = "yuanfengai.cn"
                                showCopiedToast = true
                                DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
                                    showCopiedToast = false
                                }
                            } label: {
                                HStack(spacing: 3) {
                                    Text("yuanfengai.cn")
                                    Image(systemName: "doc.on.doc")
                                        .font(.system(size: 11))
                                }
                            }
                            .font(.subheadline)
                            .buttonStyle(.plain)
                            .foregroundStyle(Color.accentColor)
                            Text(" 下载电脑端")
                                .font(.subheadline)
                        }
                    }
                    HStack(alignment: .top, spacing: 12) {
                        stepBadge("2")
                        Text("点击电脑端菜单栏图标 → 选择配对手机")
                            .font(.subheadline)
                    }
                    HStack(alignment: .top, spacing: 12) {
                        stepBadge("3")
                        Text("在手机端输入电脑显示的 4 位配对码")
                            .font(.subheadline)
                    }
                }
                .padding(.vertical, 4)

                Button(action: { onPairTapped?() }) {
                    HStack(spacing: 6) {
                        Image(systemName: "laptopcomputer.and.iphone")
                        Text("配对电脑")
                    }
                    .frame(maxWidth: .infinity, alignment: .center)
                }
                .buttonStyle(.borderedProminent)
            } header: {
                Text("配对说明")
            } footer: {
                if showCopiedToast {
                    Text("已复制到剪贴板")
                        .foregroundStyle(.secondary)
                }
            }

            // 操作方式
            Section {
                operationRow(icon: "return", title: "发送",
                             description: "将内容同步至配对电脑，并同时按下回车键执行发送")
                operationRow(icon: "doc.on.clipboard", title: "插入",
                             description: "将内容同步至配对电脑，可以在电脑继续编辑")
operationRow(icon: "arrow.up", title: "上滑手势",
                             description: "按住发送/插入按钮上滑，可以重复上次操作\n按住清空按钮上滑，可以恢复内容")
            } header: {
                Text("操作方式")
            }

            // 故障排除
            Section {
                operationRow(icon: "checklist", title: "开启辅助功能权限",
                             description: "若内容未出现在电脑上，请先确认电脑端 App 的辅助功能权限已开启。可在电脑端偏好设置的基础设置页面找到相应开关及引导")
                operationRow(icon: "macwindow", title: "确认窗口在最前面",
                             description: "落笔会把内容输入到电脑当前焦点窗口。操作前请点击一下电脑上需要输入的窗口，确保它处于最前面")
                operationRow(icon: "doc.text.magnifyingglass", title: "查看电脑端日志",
                             description: "可在电脑端 App 的设置中查看操作日志，了解内容是否已收到，以及失败原因")
            } header: {
                Text("故障排除")
            }
        }
        .navigationTitle("使用说明")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private func operationRow(icon: String, title: String, description: String = "") -> some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: icon)
                .foregroundStyle(Color.accentColor)
                .frame(width: 22, alignment: .center)
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.subheadline).bold()
                if !description.isEmpty {
                    Text(description)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .padding(.vertical, 2)
    }

    @ViewBuilder
    private func stepBadge(_ number: String) -> some View {
        Text(number)
            .font(.caption.bold())
            .foregroundStyle(.white)
            .frame(width: 20, height: 20)
            .background(Color.accentColor)
            .clipShape(Circle())
    }
}

#Preview {
    NavigationStack {
        UsageGuideView()
    }
}
