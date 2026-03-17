//
//  EmptyStateView.swift
//  NextypeApp
//
//  空状态视图 - 无配对设备时显示
//

import SwiftUI

struct EmptyStateView: View {
    var onSkip: (() -> Void)? = nil
    @EnvironmentObject var pairedMacManager: PairedMacManager
    @State private var showPairingView = false
    @State private var showUsageGuide = false
    @State private var showToast = false
    @State private var toastMessage = ""
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            VStack(spacing: 32) {
                Spacer()

                // 图标
                Image(systemName: "laptopcomputer")
                    .font(.system(size: 80))
                    .foregroundColor(.secondary)

                // 提示文字
                VStack(spacing: 12) {
                    Text("欢迎使用落笔 Nextype")
                        .font(.title2)
                        .fontWeight(.semibold)

                    Text("请先添加一台电脑设备")
                        .font(.body)
                        .foregroundColor(.secondary)
                }

                // 添加设备按钮
                Button(action: {
                    showPairingView = true
                }) {
                    HStack {
                        Image(systemName: "plus.circle.fill")
                        Text("配对电脑")
                    }
                    .font(.headline)
                    .foregroundColor(.white)
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.accentColor)
                    .cornerRadius(12)
                }
                .padding(.horizontal, 32)

                // 下载引导
                VStack(spacing: 16) {
                    Text("还没有安装电脑端？")
                        .font(.subheadline)
                        .foregroundColor(.secondary)

                    Button(action: {
                        copyUrlToClipboard()
                    }) {
                        HStack {
                            Text("下载地址：")
                                .foregroundColor(.primary)
                            Text("yuanfengai.cn")
                                .foregroundColor(.accentColor)
                                .underline()
                            Image(systemName: "doc.on.doc")
                                .font(.caption)
                                .foregroundColor(.accentColor)
                        }
                        .padding(.vertical, 8)
                        .padding(.horizontal, 16)
                        .background(Color(.systemGray6))
                        .cornerRadius(8)
                    }
                }
                .padding(.top, 16)


                Spacer()

                // 底部入口
                HStack(spacing: 24) {
                    Button(action: { showUsageGuide = true }) {
                        HStack(spacing: 3) {
                            Text("使用场景")
                            Image(systemName: "chevron.right")
                                .font(.system(size: 11, weight: .light))
                        }
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                    }
                    if let onSkip {
                        Button(action: onSkip) {
                            HStack(spacing: 3) {
                                Text("稍后配对")
                                Image(systemName: "chevron.right")
                                    .font(.system(size: 11, weight: .light))
                            }
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                        }
                    }
                }
                .padding(.bottom, 32)
            }
            .overlay(alignment: .bottom) {
                if showToast {
                    Text(toastMessage)
                        .font(.subheadline)
                        .foregroundColor(.white)
                        .padding(.vertical, 10)
                        .padding(.horizontal, 20)
                        .background(Color.black.opacity(0.8))
                        .cornerRadius(25)
                        .padding(.bottom, 50)
                        .transition(.move(edge: .bottom).combined(with: .opacity))
                }
            }
            .onAppear {
                tryRecoverPairingFromServer()
            }
            .navigationTitle("开始使用")
            .navigationBarTitleDisplayMode(.inline)
            .sheet(isPresented: $showPairingView) {
                PairingCodeView()
                    .environmentObject(pairedMacManager)
            }
            .sheet(isPresented: $showUsageGuide) {
                NavigationStack {
                    AboutView()
                        .toolbar {
                            ToolbarItem(placement: .confirmationAction) {
                                Button("完成") { showUsageGuide = false }
                            }
                        }
                }
            }
        }
    }

    /// 复制 URL 到剪贴板
    private func copyUrlToClipboard() {
        UIPasteboard.general.string = "https://yuanfengai.cn"

        // 触觉反馈
        let impact = UIImpactFeedbackGenerator(style: .medium)
        impact.impactOccurred()

        // 显示 Toast
        toastMessage = "链接已复制"
        withAnimation {
            showToast = true
        }

        // 自动隐藏
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
            withAnimation {
                showToast = false
            }
        }
    }

    /// 尝试从服务器恢复配对信息
    private func tryRecoverPairingFromServer() {
        guard pairedMacManager.pairedMacs.isEmpty else { return }

        #if DEBUG
        print("🔄 EmptyState: 尝试从服务器恢复配对信息...")
        #endif

        if !ConnectionManager.shared.isConnected {
            ConnectionManager.shared.connect()
        }
    }
}

#Preview {
    EmptyStateView()
        .environmentObject(PairedMacManager())
}
