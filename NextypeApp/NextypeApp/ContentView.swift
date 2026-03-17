//
//  ContentView.swift
//  NextypeApp
//
//  落笔 Nextype - 主界面
//

import SwiftUI

struct ContentView: View {
    @StateObject private var pairedMacManager = PairedMacManager()
    @ObservedObject private var connectionManager = ConnectionManager.shared

    @AppStorage("skipPairing") private var skipPairing = false

    var body: some View {
        Group {
            if pairedMacManager.pairedMacs.isEmpty && !skipPairing {
                EmptyStateView(onSkip: { skipPairing = true })
                    .environmentObject(pairedMacManager)
            } else {
                MainInputView(onGoToWelcome: { skipPairing = false })
                    .environmentObject(pairedMacManager)
                    .environmentObject(connectionManager)
            }
        }
        .onAppear {
            // 连接到中继服务器
            connectionManager.connect()
            // DEBUG: 注入假配对设备，方便 UI 预览各页面
            #if false
            if pairedMacManager.pairedMacs.isEmpty {
                pairedMacManager.pairedMacs = [
                    PairedMac(
                        deviceId: "debug-mac-preview",
                        deviceName: "我的 MacBook Pro",
                        ip: "relay",
                        port: 8080,
                        encryptionKey: "debug-key",
                        supportRelay: true
                    )
                ]
            }
            #endif
        }
        .onChange(of: scenePhase) { oldPhase, newPhase in
            handleScenePhaseChange(newPhase)
        }
    }

    @Environment(\.scenePhase) private var scenePhase

    private func handleScenePhaseChange(_ phase: ScenePhase) {
        switch phase {
        case .active:
            #if DEBUG
            print("📱 [生命周期] 进入前台")
            #endif
            connectionManager.checkAndReconnect()
            UIApplication.shared.isIdleTimerDisabled = UserDefaults.standard.bool(forKey: "keepScreenOn")
            ScreenDimManager.shared.handleAppWillEnterForeground()
        case .background:
            #if DEBUG
            print("📱 [生命周期] 进入后台")
            #endif
            connectionManager.disconnect()
            UIApplication.shared.isIdleTimerDisabled = false
            ScreenDimManager.shared.handleAppDidEnterBackground()
        case .inactive:
            break
        @unknown default:
            break
        }
    }
}

#Preview {
    ContentView()
}
