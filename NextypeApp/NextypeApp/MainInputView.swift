//
//  MainInputView.swift
//  NextypeApp
//
//  主输入界面 - 输入并发送内容到PC端
//

import SwiftUI

struct MainInputView: View {
    var onGoToWelcome: (() -> Void)? = nil
    @EnvironmentObject var pairedMacManager: PairedMacManager
    @EnvironmentObject var connectionManager: ConnectionManager
    @ObservedObject private var screenDimManager = ScreenDimManager.shared

    @State private var inputText: String = ""

    enum ActiveSheet: Identifiable {
        case settings
        case deviceEdit(PairedMac)
        var id: String {
            switch self {
            case .settings: return "settings"
            case .deviceEdit(let device): return "edit-\(device.deviceId)"
            }
        }
    }

    @State private var activeSheet: ActiveSheet? = nil
    @State private var showDeviceSelector = false
    @State private var showRePairSheet = false
    @State private var connectionCheckTimer: Timer?
    @AppStorage("pasteCopiesToClipboard") private var pasteCopiesToClipboard = true
    @AppStorage("pasteEnterCopiesToClipboard") private var pasteEnterCopiesToClipboard = true
    @AppStorage("handMode") private var handMode = "right"
    @AppStorage("inputFontSizeIndex") private var inputFontSizeIndex = 1
    private var inputFontSize: CGFloat {
        let sizes: [CGFloat] = [16, 18, 20, 24, 28]
        return sizes[min(inputFontSizeIndex, sizes.count - 1)]
    }
    @AppStorage("hasShownSwipeHint") private var hasShownSwipeHint = false

    // 上滑引导提示
    @State private var showSwipeHint = false

    // 上滑手势
    @State private var lastSentContent: String = ""
    @State private var lastClearedContent: String = ""
    @State private var showResendPopup = false
    @State private var showRestorePopup = false
    @State private var activeButton: String? = nil
    @State private var isSlideUp = false
    @State private var previewText: String = ""
    @State private var isKeyboardVisible = false
    @FocusState private var isInputFocused: Bool

    // 远程控制防抖
    @State private var lastRemoteCommandTime: TimeInterval = 0

    // 自动切换：每次前台仅切换一次，防止重复触发
    @State private var hasAutoSwitchedThisResume = false

    private var currentDevice: PairedMac? {
        // 始终从 pairedMacManager 取最新数据，确保编辑名称后立即反映
        if let deviceId = connectionManager.currentDevice?.deviceId {
            return pairedMacManager.getPairedMac(deviceId: deviceId)
        }
        return pairedMacManager.getLastConnectedMac()
    }

    private var placeholderText: String {
        switch connectionManager.connectionState {
        case .connected:
            return "已连接 · 开始输入"
        case .connecting:
            return "连接中 · 请稍候"
        case .disconnected:
            return "未连接 · 请检查网络"
        }
    }

    private var isConnected: Bool {
        connectionManager.connectionState == .connected
    }

    // MARK: - 上滑弹窗

    @ViewBuilder
    private func swipePopupView(title: String, isSelected: Bool) -> some View {
        HStack(spacing: 8) {
            Image(systemName: "arrow.up.circle.fill")
                .font(.system(size: 16, weight: .semibold))
            Text(title)
                .font(.system(size: 14, weight: .semibold))
        }
        .foregroundColor(isSelected ? .white : .primary)
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .background(Capsule().fill(isSelected ? Color.accentColor : Color(.systemGray5)))
        .shadow(color: Color.black.opacity(0.15), radius: 8, x: 0, y: 4)
    }

    // MARK: - 上滑引导条

    @ViewBuilder
    private var swipeHintBar: some View {
        HStack(spacing: 8) {
            Image(systemName: "arrow.up")
                .font(.system(size: 11, weight: .semibold))
                .foregroundColor(.secondary)
            Text("按住按钮向上滑动，可重复上次操作")
                .font(.system(size: 13))
                .foregroundColor(.secondary)
            Spacer()
            Button {
                withAnimation(.easeOut(duration: 0.2)) { showSwipeHint = false }
            } label: {
                Image(systemName: "xmark")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundColor(Color(.tertiaryLabel))
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .background(Color(.systemGray6))
        .transition(.move(edge: .bottom).combined(with: .opacity))
    }


    // MARK: - 通用上滑按钮

    @ViewBuilder
    private func actionButton(
        icon: String, title: String, style: Material = .thinMaterial,
        hasContent: Bool, lastContent: String,
        showPopup: Binding<Bool>, buttonId: String,
        useAccentBackground: Bool = false,
        dimLabel: Bool = false,
        iconTrailing: Bool = false,
        onTap: @escaping () -> Void, onSwipe: @escaping () -> Void
    ) -> some View {
        HStack(spacing: 6) {
            if iconTrailing {
                Text(title).font(.body).fontWeight(.semibold)
                Image(systemName: icon).font(.body)
            } else {
                Image(systemName: icon).font(.body)
                Text(title).font(.body).fontWeight(.semibold)
            }
        }
        .foregroundColor(useAccentBackground ? .white : (dimLabel ? .secondary : .primary))
        .frame(maxWidth: .infinity)
        .frame(height: 50)
        .background {
            if useAccentBackground {
                Capsule().fill(Color.accentColor)
            } else {
                Capsule().fill(Color(.systemBackground))
            }
        }
        .overlay {
            if !useAccentBackground {
                Capsule().stroke(Color(.systemGray4), lineWidth: 0.5)
            }
        }
        .opacity(!hasContent && lastContent.isEmpty ? 0.5 : 1.0)
        .gesture(
            DragGesture(minimumDistance: 0)
                .onChanged { value in
                    screenDimManager.onUserInteraction()
                    if activeButton == nil {
                        activeButton = buttonId
                        UIImpactFeedbackGenerator(style: .light).impactOccurred()
                    }
                    let threshold: CGFloat = -40
                    let wasSlideUp = isSlideUp
                    isSlideUp = value.translation.height < threshold

                    if !lastContent.isEmpty {
                        showPopup.wrappedValue = isSlideUp
                        if isSlideUp && !wasSlideUp {
                            previewText = lastContent
                            UIImpactFeedbackGenerator(style: .medium).impactOccurred()
                        } else if !isSlideUp && wasSlideUp {
                            previewText = ""
                        }
                    }
                }
                .onEnded { _ in
                    if isSlideUp && !lastContent.isEmpty {
                        onSwipe()
                        showFirstTimeSwipeHint()
                    } else if activeButton == buttonId {
                        onTap()
                    }
                    activeButton = nil
                    isSlideUp = false
                    showPopup.wrappedValue = false
                    previewText = ""
                }
        )
    }

    // MARK: - Body

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                // 主输入区域
                ZStack(alignment: .topLeading) {
                    TextEditor(text: $inputText)
                        .font(.system(size: inputFontSize))
                        .scrollContentBackground(.hidden)
                        .background(Color(.systemBackground))
                        .padding(.horizontal, 16)
                        .padding(.vertical, 12)
                        .focused($isInputFocused)
                        .onChange(of: inputText) { _, _ in
                            screenDimManager.onUserInteraction()
                        }


                    if !previewText.isEmpty {
                        Text(previewText)
                            .foregroundColor(Color(uiColor: .placeholderText))
                            .font(.system(size: inputFontSize))
                            .padding(.horizontal, 21)
                            .padding(.vertical, 20)
                            .allowsHitTesting(false)
                    }
                }
                .background(Color(.systemBackground))

                // 底部区域
                VStack(spacing: 0) {
                    // 上滑引导条（仅首次点击后显示）
                    if showSwipeHint {
                        swipeHintBar
                    }

                    // 按钮行
                    ZStack(alignment: .top) {
                        if showResendPopup {
                            swipePopupView(title: "重发", isSelected: isSlideUp)
                                .offset(y: -50)
                                .transition(.opacity.combined(with: .scale))
                        }
                        if showRestorePopup {
                            swipePopupView(title: "恢复", isSelected: isSlideUp)
                                .offset(y: -50)
                                .transition(.opacity.combined(with: .scale))
                        }

                        HStack(spacing: 8) {
                            if handMode == "left" {
                                sendButton; syncButton; clearButton
                            } else {
                                clearButton; syncButton; sendButton
                            }
                        }
                    }
                    .padding(.horizontal, 16)
                    .padding(.top, 12)
                    .padding(.bottom, 12)
                }
                .background(Color(.systemBackground))
                .animation(.easeOut(duration: 0.2), value: showSwipeHint)
                .animation(.easeOut(duration: 0.15), value: showResendPopup)
                .animation(.easeOut(duration: 0.15), value: showRestorePopup)
            }
            .navigationBarTitleDisplayMode(.inline)
            .safeAreaInset(edge: .top) { topBarView }
            .sheet(item: $activeSheet) { sheet in
                switch sheet {
                case .settings:
                    SettingsView(onPairTapped: { showRePairSheet = true })
                        .environmentObject(pairedMacManager)
                case .deviceEdit(let device):
                    DeviceEditView(device: device)
                        .environmentObject(pairedMacManager)
                }
            }
            .sheet(isPresented: $showRePairSheet) {
                PairingCodeView()
            }
            .alert("需要重新配对", isPresented: $connectionManager.showRePairAlert) {
                Button("立即重新配对") { showRePairSheet = true }
                Button("稍后处理", role: .cancel) { }
            } message: {
                Text("手机和电脑的连接凭证已过期。重新配对即可恢复正常使用。")
            }
            .onAppear {
                connectToDevice()
                startConnectionMonitoring()
                setupRemoteCommandHandler()
                setupAutoSwitchHandler()
                screenDimManager.applyInitialSettings()
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                    isInputFocused = true
                }
            }
            .onReceive(NotificationCenter.default.publisher(for: UIApplication.didBecomeActiveNotification)) { _ in
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                    isInputFocused = true
                }
                // 每次回到前台重置切换标志，允许本次前台做一次自动切换
                hasAutoSwitchedThisResume = false
                // 5 秒兜底：WebSocket 可能还在重连，等连上后再触发一次 discover
                DispatchQueue.main.asyncAfter(deadline: .now() + 5) {
                    guard !hasAutoSwitchedThisResume,
                          pairedMacManager.pairedMacs.count > 1,
                          connectionManager.isConnected
                    else { return }
                    #if DEBUG
                    print("⏱️ [自动切换] 5 秒兜底触发 discover")
                    #endif
                    connectionManager.discoverOnlineDevices()
                }
            }
            .onDisappear {
                stopConnectionMonitoring()
            }
            .onReceive(NotificationCenter.default.publisher(for: UIResponder.keyboardWillShowNotification)) { _ in
                withAnimation(.easeOut(duration: 0.2)) { isKeyboardVisible = true }
                screenDimManager.onUserInteraction()
            }
            .onReceive(NotificationCenter.default.publisher(for: UIResponder.keyboardWillHideNotification)) { _ in
                withAnimation(.easeOut(duration: 0.2)) { isKeyboardVisible = false }
            }
            .onChange(of: connectionManager.connectionState) { _, newState in
                if case .connected = newState, let device = currentDevice {
                    pairedMacManager.lastConnectedDeviceId = device.deviceId
                    // 连接成功后上报屏幕参数
                    connectionManager.sendScreenInfo()
                    // 多台配对设备时触发自动切换（每次前台仅执行一次）
                    if pairedMacManager.pairedMacs.count > 1, !hasAutoSwitchedThisResume {
                        connectionManager.discoverOnlineDevices()
                    }
                }
            }
            .onChange(of: connectionManager.lastSendError) { _, errorMsg in
                guard errorMsg != nil else { return }
                // 3 秒后自动清除
                DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                    connectionManager.lastSendError = nil
                }
            }
            .overlay(alignment: .top) {
                if let errorMsg = connectionManager.lastSendError {
                    Text(errorMsg)
                        .font(.system(size: 14, weight: .medium))
                        .foregroundColor(.white)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 10)
                        .background(Color.red.opacity(0.88))
                        .cornerRadius(10)
                        .padding(.top, 8)
                        .transition(.move(edge: .top).combined(with: .opacity))
                        .animation(.spring(response: 0.3), value: errorMsg)
                }
            }
        }
    }

    // MARK: - 按钮视图

    private var clearButton: some View {
        actionButton(
            icon: "xmark", title: "清空", style: .ultraThinMaterial,
            hasContent: !inputText.isEmpty, lastContent: lastClearedContent,
            showPopup: $showRestorePopup, buttonId: "clear",
            onTap: {
                if !inputText.isEmpty {
                    lastClearedContent = inputText
                    inputText = ""
                }
            },
            onSwipe: {
                inputText = lastClearedContent
                lastClearedContent = ""
            }
        )
    }

    private var syncButton: some View {
        actionButton(
            icon: "doc.on.clipboard", title: "插入",
            hasContent: !inputText.isEmpty, lastContent: lastSentContent,
            showPopup: $showResendPopup, buttonId: "sync",
            onTap: { if !inputText.isEmpty { syncContent() } },
            onSwipe: { inputText = lastSentContent; syncContent() }
        )
    }

    private var sendButton: some View {
        actionButton(
            icon: "return", title: "发送",
            hasContent: !inputText.isEmpty, lastContent: lastSentContent,
            showPopup: $showResendPopup, buttonId: "send",
            useAccentBackground: true,
            iconTrailing: true,
            onTap: { if !inputText.isEmpty { sendContent() } },
            onSwipe: { inputText = lastSentContent; sendContent() }
        )
    }

    // MARK: - 顶部栏

    private var topBarView: some View {
        HStack(spacing: 0) {
            HStack(spacing: 6) {
                Text("落笔").font(.system(size: 22, weight: .semibold)).foregroundColor(.primary)
                Text("Nextype").font(.system(size: 22, weight: .regular)).foregroundColor(.secondary)
            }
            .frame(height: 44)
            .padding(.leading, 16)

            Spacer()

            HStack(spacing: 12) {
                if let device = currentDevice {
                    deviceMenu(for: device)
                        .fixedSize()
                } else {
                    Button(action: { onGoToWelcome?() }) {
                        HStack(spacing: 5) {
                            Image(systemName: "laptopcomputer.badge.plus")
                                .font(.system(size: 14, weight: .medium))
                            Text("配对电脑")
                                .font(.system(size: 17, weight: .medium))
                        }
                        .foregroundColor(.white)
                        .padding(.vertical, 6).padding(.horizontal, 14)
                        .background(Color.accentColor, in: Capsule())
                    }
                }
                Button(action: { activeSheet = .settings }) {
                    Image(systemName: "gearshape").font(.body).foregroundColor(.primary)
                }
            }
            .frame(height: 44)
            .padding(.trailing, 16)
        }
        .frame(height: 44)
        .background(Color(.systemBackground))
        .id(currentDevice?.deviceId ?? "none")
    }

    @ViewBuilder
    private func deviceMenu(for device: PairedMac) -> some View {
        Menu {
            ForEach(pairedMacManager.pairedMacs) { mac in
                Menu {
                    if mac.deviceId != device.deviceId {
                        Button(action: { switchToDevice(mac) }) {
                            Label("切换到此设备", systemImage: "arrow.right.circle")
                        }
                    }
                    Button(action: { activeSheet = .deviceEdit(mac) }) {
                        Label("编辑设备", systemImage: "pencil")
                    }
                    Button(role: .destructive, action: { pairedMacManager.removePairedMac(deviceId: mac.deviceId) }) {
                        Label("解除配对", systemImage: "trash")
                    }
                } label: {
                    Label(
                        mac.getDisplayName(),
                        systemImage: mac.deviceId == device.deviceId ? "checkmark.circle.fill" : (mac.customIcon ?? "desktopcomputer")
                    )
                }
            }
            Divider()
            Button(action: { showRePairSheet = true }) {
                Label("配对新电脑", systemImage: "laptopcomputer.badge.plus")
            }
        } label: {
            HStack(spacing: 5) {
                if isConnected {
                    Circle()
                        .fill(Color.green)
                        .frame(width: 8, height: 8)
                }
                Image(systemName: device.customIcon ?? "laptopcomputer")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundColor(Color.primary.opacity(0.65))
                Text(String(device.getDisplayName().prefix(4)))
                    .font(.system(size: 17, weight: .medium))
                    .foregroundColor(Color.primary.opacity(0.65))
                Image(systemName: "chevron.down")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundColor(.secondary)
            }
            .padding(.vertical, 6).padding(.horizontal, 14)
            .background(Color(.systemGray6).opacity(0.6), in: Capsule())
        }
    }

    // MARK: - 操作方法

    private func syncContent() {
        sendMessage(action: "paste", copyToClipboard: pasteCopiesToClipboard)
    }

    private func sendContent() {
        sendMessage(action: "paste-enter", copyToClipboard: pasteEnterCopiesToClipboard)
    }

    private func sendMessage(action: String, copyToClipboard: Bool) {
        guard let device = currentDevice else { return }
        let content = inputText

        if copyToClipboard { UIPasteboard.general.string = content }

        do {
            // 每次发送前从 pairedMacManager 实时取最新密钥，避免重配对后 connectionManager.currentDevice 缓存旧 key
            let latestKey = pairedMacManager.getPairedMac(deviceId: device.deviceId)?.encryptionKey ?? device.encryptionKey
            let encryptionKey = latestKey.isEmpty
                ? DeviceIDManager.shared.getDeviceId()
                : latestKey
            let encrypted = try EncryptionManager.shared.encrypt(content, using: encryptionKey)
            connectionManager.sendClipboard(content: encrypted, action: action, to: device.deviceId)
            lastSentContent = content
            inputText = ""
        } catch {
            #if DEBUG
            print("❌ 加密失败: \(error.localizedDescription)")
            #endif
        }
    }

    // MARK: - 连接

    private func connectToDevice() {
        guard let device = currentDevice else { return }
        connectionManager.connectToDevice(device)
    }

    private func switchToDevice(_ device: PairedMac) {
        connectionManager.switchToDevice(device)
    }

    private func setupAutoSwitchHandler() {
        connectionManager.onServerList = { servers in
            guard !hasAutoSwitchedThisResume else { return }

            // 过滤：必须在配对列表中，且 idleTime < 120 秒
            let pairedIds = Set(pairedMacManager.pairedMacs.map { $0.deviceId })
            let candidates = servers.filter { pairedIds.contains($0.deviceId) && $0.idleTime < 120 }

            guard let best = candidates.min(by: { $0.idleTime < $1.idleTime }) else {
                #if DEBUG
                print("🔍 [自动切换] 无符合条件设备（idleTime < 120s）")
                #endif
                return
            }

            hasAutoSwitchedThisResume = true

            if best.deviceId != connectionManager.currentDevice?.deviceId,
               let targetMac = pairedMacManager.getPairedMac(deviceId: best.deviceId)
            {
                #if DEBUG
                print("🔄 [自动切换] 切换到最近活跃设备: \(best.deviceName)（idleTime=\(best.idleTime)s）")
                #endif
                switchToDevice(targetMac)
            } else {
                #if DEBUG
                print("✅ [自动切换] 当前设备已是最近活跃，无需切换（idleTime=\(best.idleTime)s）")
                #endif
            }
        }
    }

    private func startConnectionMonitoring() {
        stopConnectionMonitoring()
        connectionCheckTimer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { _ in
            connectionManager.checkAndReconnect()
        }
    }

    private func stopConnectionMonitoring() {
        connectionCheckTimer?.invalidate()
        connectionCheckTimer = nil
    }

    // MARK: - 远程控制

    private func setupRemoteCommandHandler() {
        connectionManager.onRemoteCommand = { action, data in
            handleRemoteCommand(action: action, data: data)
        }
    }

    private func handleRemoteCommand(action: String, data: [String: Any]) {
        let now = Date().timeIntervalSince1970 * 1000
        guard now - lastRemoteCommandTime > 500 else { return }
        lastRemoteCommandTime = now

        screenDimManager.onUserInteraction()
        #if DEBUG
        print("🎮 [远程] 执行指令: \(action)")
        #endif

        switch action {
        case "send":
            if !inputText.isEmpty { sendContent() }
        case "insert":
            if !inputText.isEmpty { syncContent() }
        case "clear":
            if !inputText.isEmpty {
                lastClearedContent = inputText
                inputText = ""
            }
        default:
            #if DEBUG
            print("🎮 [远程] 未知指令: \(action)")
            #endif
        }
    }

    // MARK: - 提示

    /// 首次点击发送或插入后，展示上滑操作引导条（全局只显示一次）
    private func showFirstTimeSwipeHint() {
        guard !hasShownSwipeHint else { return }
        hasShownSwipeHint = true
        withAnimation(.easeOut(duration: 0.2)) { showSwipeHint = true }
        // 5 秒后自动消失
        DispatchQueue.main.asyncAfter(deadline: .now() + 5) {
            withAnimation(.easeOut(duration: 0.2)) { showSwipeHint = false }
        }
    }
}

#Preview {
    MainInputView()
        .environmentObject(PairedMacManager())
        .environmentObject(ConnectionManager.shared)
}