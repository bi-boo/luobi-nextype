//
//  PairingCodeView.swift
//  NextypeApp
//
//  配对码输入界面
//

import SwiftUI

struct PairingCodeView: View {
    @Environment(\.dismiss) var dismiss
    @EnvironmentObject var pairedMacManager: PairedMacManager

    @State private var code1 = ""
    @State private var code2 = ""
    @State private var code3 = ""
    @State private var code4 = ""

    @State private var isPairing = false
    @State private var errorMessage: String?
    @State private var showSuccess = false

    @FocusState private var focusedField: Int?

    var body: some View {
        NavigationStack {
            VStack(spacing: 30) {
                Image(systemName: "laptopcomputer.and.iphone")
                    .font(.system(size: 60))
                    .foregroundColor(.accentColor)
                    .padding(.top, 40)

                VStack(spacing: 8) {
                    Text("配对新设备")
                        .font(.title.bold())
                    Text("请输入电脑上显示的4位配对码")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }

                HStack(spacing: 15) {
                    CodeTextField(text: $code1, focusedField: _focusedField, index: 0, nextIndex: 1)
                    CodeTextField(text: $code2, focusedField: _focusedField, index: 1, nextIndex: 2)
                    CodeTextField(text: $code3, focusedField: _focusedField, index: 2, nextIndex: 3)
                    CodeTextField(text: $code4, focusedField: _focusedField, index: 3, nextIndex: nil)
                }
                .padding(.horizontal, 40)
                .onChange(of: code4) { oldValue, newValue in
                    if !newValue.isEmpty && canPair {
                        UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)
                        startPairing()
                    }
                }

                if isPairing {
                    HStack(spacing: 12) {
                        ProgressView()
                        Text("正在配对...")
                            .foregroundColor(.accentColor)
                            .font(.callout)
                    }
                    .padding(.vertical, 8)
                }

                VStack(alignment: .leading, spacing: 8) {
                    HelpText(number: "1", text: "在电脑上打开「落笔 Nextype」")
                    HelpText(number: "2", text: "点击菜单栏图标 → 配对手机")
                    HelpText(number: "3", text: "输入电脑上显示的 4 位配对码")
                }
                .padding(.horizontal, 20)

                Spacer()
            }
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("取消") { dismiss() }
                }
            }
            .onAppear {
                DispatchQueue.main.async { focusedField = 0 }
                if !ConnectionManager.shared.isConnected {
                    ConnectionManager.shared.connect()
                }
            }
            .alert("配对成功", isPresented: $showSuccess) {
                Button("确定") { dismiss() }
            } message: {
                Text("已成功配对电脑，现在可以自动连接了")
            }
            .overlay(alignment: .top) {
                if let error = errorMessage {
                    Text(error)
                        .font(.system(size: 14, weight: .medium))
                        .foregroundColor(.white)
                        .multilineTextAlignment(.center)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 10)
                        .background(Color.red.opacity(0.88))
                        .cornerRadius(10)
                        .padding(.top, 8)
                        .transition(.move(edge: .top).combined(with: .opacity))
                        .animation(.spring(response: 0.3), value: error)
                }
            }
            .onChange(of: errorMessage) { _, newError in
                guard newError != nil else { return }
                DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                    withAnimation { errorMessage = nil }
                }
            }
        }
    }
    
    var canPair: Bool {
        return !code1.isEmpty && !code2.isEmpty && !code3.isEmpty && !code4.isEmpty
    }
    
    var pairingCode: String {
        return code1 + code2 + code3 + code4
    }
    
    func startPairing() {
        guard canPair else { return }

        #if targetEnvironment(simulator)
        if pairingCode == "9999" {
            let fakeMac = PairedMac(
                deviceId: "debug-mac-simulator-9999",
                deviceName: "测试 Mac（模拟器）",
                ip: "relay",
                port: 8080,
                encryptionKey: "debug-key",
                supportRelay: true
            )
            pairedMacManager.addPairedMac(fakeMac)
            showSuccess = true
            return
        }
        #endif

        isPairing = true
        errorMessage = nil

        // 统一使用中继配对（对齐 Android 端）
        ConnectionManager.shared.verifyPairingCode(pairingCode) { result in
            DispatchQueue.main.async {
                isPairing = false

                switch result {
                case .success(let remoteMac):
                    let mac = PairedMac(
                        deviceId: remoteMac.deviceId,
                        deviceName: remoteMac.deviceName,
                        ip: "relay",
                        port: 8080,
                        encryptionKey: remoteMac.encryptionKey ?? remoteMac.deviceId,
                        supportRelay: true
                    )
                    pairedMacManager.addPairedMac(mac)
                    showSuccess = true

                case .failure:
                    errorMessage = "配对失败，请确认电脑端 App 已打开，或检查网络连接"
                }
            }
        }
    }
}

// 配对码输入框组件
struct CodeTextField: View {
    @Binding var text: String
    @FocusState var focusedField: Int?
    let index: Int
    let nextIndex: Int?
    
    var body: some View {
        TextField("", text: $text)
            .keyboardType(.numberPad)
            .textContentType(.oneTimeCode) // 提示系统这是验证码输入
            .autocorrectionDisabled(true) // 禁用自动更正
            .textInputAutocapitalization(.never) // 禁用自动大写
            .multilineTextAlignment(.center)
            .font(.system(size: 36, weight: .bold))
            .frame(width: 60, height: 70)
            .background(Color.gray.opacity(0.1))
            .cornerRadius(12)
            .focused($focusedField, equals: index)
            .onChange(of: text) { oldValue, newValue in
                // 只允许输入一个数字
                if newValue.count > 1 {
                    text = String(newValue.suffix(1))
                }
                
                // 输入后自动跳转到下一个框
                if !newValue.isEmpty, let next = nextIndex {
                    focusedField = next
                }
                
                // 删除时返回上一个框
                if newValue.isEmpty && index > 0 {
                    focusedField = index - 1
                }
            }
    }
}

// 帮助文本组件
struct HelpText: View {
    let number: String
    let text: String

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            Text(number)
                .font(.caption)
                .fontWeight(.bold)
                .foregroundColor(.white)
                .frame(width: 20, height: 20)
                .background(Color.accentColor)
                .clipShape(Circle())

            Text(text)
                .font(.callout)
                .foregroundColor(.secondary)
        }
    }
}

#Preview {
    return PairingCodeView()
        .environmentObject(PairedMacManager())
}
