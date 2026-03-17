//
//  SettingsView.swift
//  NextypeApp
//
//  设置页面 - 配置应用选项
//

import SwiftUI

struct SettingsView: View {
    var onPairTapped: (() -> Void)? = nil

    @AppStorage("pasteCopiesToClipboard") private var pasteCopiesToClipboard = true
    @AppStorage("pasteEnterCopiesToClipboard") private var pasteEnterCopiesToClipboard = true
    @AppStorage("handMode") private var handMode = "right"
    @AppStorage("inputFontSizeIndex") private var inputFontSizeIndex = 1
    @Environment(\.dismiss) private var dismiss
    @ObservedObject private var screenDimManager = ScreenDimManager.shared

    @State private var keepScreenOn: Bool = UserDefaults.standard.object(forKey: "keepScreenOn") as? Bool ?? true
    @State private var autoDimEnabled: Bool = UserDefaults.standard.object(forKey: "autoDimEnabled") as? Bool ?? true
    @State private var autoDimTimeout: Int = {
        let v = UserDefaults.standard.integer(forKey: "autoDimTimeout")
        return v > 0 ? v : 30
    }()
    private static let fontSizeValues: [CGFloat] = [16, 18, 20, 24, 28]
    private static let fontSizeLabels = ["极小", "标准", "适中", "偏大", "特大"]

    private static let dimTimeoutOptions: [(Int, String)] = [
        (30, "30 秒"),
        (60, "1 分钟"),
        (300, "5 分钟"),
        (600, "10 分钟"),
    ]

    var body: some View {
        NavigationStack {
            List {
                // 显示设置（惯用手 + 字号）
                Section {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("惯用手")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                        Picker("惯用手", selection: $handMode) {
                            Text("左手").tag("left")
                            Text("右手").tag("right")
                        }
                        .pickerStyle(.segmented)
                    }
                    .padding(.vertical, 4)

                    VStack(alignment: .leading, spacing: 12) {
                        Text("输入框字号")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                        Picker("字号", selection: $inputFontSizeIndex) {
                            ForEach(0..<5, id: \.self) { i in
                                Text(Self.fontSizeLabels[i]).tag(i)
                            }
                        }
                        .pickerStyle(.segmented)

                        Text("把这个按钮调大一点")
                            .font(.system(size: Self.fontSizeValues[inputFontSizeIndex]))
                            .frame(maxWidth: .infinity, minHeight: 44, alignment: .center)
                            .padding(.vertical, 4)
                    }
                    .padding(.vertical, 4)
                }

                // 剪贴板同步设置
                Section {
                    Toggle("插入时同步到剪贴板", isOn: $pasteCopiesToClipboard)
                    Toggle("发送时同步到剪贴板", isOn: $pasteEnterCopiesToClipboard)
                }

                // 屏幕设置
                Section {
                    Toggle("保持屏幕常亮", isOn: $keepScreenOn)
                        .onChange(of: keepScreenOn) { _, newValue in
                            screenDimManager.setKeepScreenOn(newValue)
                        }
                    if keepScreenOn {
                        Toggle("闲置自动变暗", isOn: $autoDimEnabled)
                            .onChange(of: autoDimEnabled) { _, newValue in
                                screenDimManager.setAutoDimEnabled(newValue)
                            }
                        if autoDimEnabled {
                            Picker(
                                "变暗等待时间",
                                selection: Binding(
                                    get: { autoDimTimeout },
                                    set: { v in
                                        autoDimTimeout = v
                                        UserDefaults.standard.set(v, forKey: "autoDimTimeout")
                                        screenDimManager.loadSettings()
                                        screenDimManager.resetDimTimer()
                                    }
                                )
                            ) {
                                ForEach(Self.dimTimeoutOptions, id: \.0) { sec, label in
                                    Text(label).tag(sec)
                                }
                            }
                        }
                    }
                } header: {
                    Text("屏幕")
                } footer: {
                    Text("开启后，应用将保持屏幕常亮；启用闲置变暗后，一段时间无操作自动变暗以节省电量")
                }

                // 使用说明入口
                Section {
                    NavigationLink {
                        UsageGuideView(onPairTapped: {
                            dismiss()
                            DispatchQueue.main.asyncAfter(deadline: .now() + 0.4) {
                                onPairTapped?()
                            }
                        })
                    } label: {
                        Label("使用说明", systemImage: "book")
                    }
                }
            }
            .navigationTitle("设置")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("完成") { dismiss() }
                }
            }
        }
    }
}

#Preview {
    SettingsView()
}
