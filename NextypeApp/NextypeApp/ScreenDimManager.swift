//
//  ScreenDimManager.swift
//  NextypeApp
//
//  屏幕管理器 - 屏幕常亮 + 闲置自动变暗
//

import Foundation
import UIKit
import Combine

class ScreenDimManager: ObservableObject {
    static let shared = ScreenDimManager()

    @Published var isDimmed = false

    private var isKeepScreenOn = false
    private var isAutoDimEnabled = true
    private var autoDimTimeoutSeconds: TimeInterval = 30
    private var dimTimer: Timer?
    private var savedBrightness: CGFloat = 0.5

    private init() {
        loadSettings()
    }

    // MARK: - 设置

    func loadSettings() {
        isKeepScreenOn = UserDefaults.standard.object(forKey: "keepScreenOn") as? Bool ?? true
        isAutoDimEnabled = UserDefaults.standard.object(forKey: "autoDimEnabled") as? Bool ?? true
        let timeout = UserDefaults.standard.integer(forKey: "autoDimTimeout")
        autoDimTimeoutSeconds = TimeInterval(timeout > 0 ? timeout : 30)
    }

    func setKeepScreenOn(_ enabled: Bool) {
        isKeepScreenOn = enabled
        UserDefaults.standard.set(enabled, forKey: "keepScreenOn")
        UIApplication.shared.isIdleTimerDisabled = enabled

        if enabled {
            resetDimTimer()
        } else {
            stopDimTimer()
            if isDimmed { wakeUpScreen() }
        }
    }

    func setAutoDimEnabled(_ enabled: Bool) {
        isAutoDimEnabled = enabled
        UserDefaults.standard.set(enabled, forKey: "autoDimEnabled")

        if enabled && isKeepScreenOn {
            resetDimTimer()
        } else {
            stopDimTimer()
            if isDimmed { wakeUpScreen() }
        }
    }

    // MARK: - 倒计时

    func resetDimTimer() {
        stopDimTimer()
        guard isKeepScreenOn, isAutoDimEnabled else { return }

        dimTimer = Timer.scheduledTimer(withTimeInterval: autoDimTimeoutSeconds, repeats: false) { [weak self] _ in
            self?.dimScreen()
        }
    }

    func stopDimTimer() {
        dimTimer?.invalidate()
        dimTimer = nil
    }

    // MARK: - 变暗/唤醒

    private func dimScreen() {
        guard !isDimmed else { return }

        savedBrightness = UIScreen.main.brightness
        isDimmed = true

        // 平滑变暗动画（300ms）
        let steps = 15
        let interval = 0.3 / Double(steps)
        let targetBrightness: CGFloat = 0.01
        let brightnessStep = (savedBrightness - targetBrightness) / CGFloat(steps)

        for i in 1...steps {
            DispatchQueue.main.asyncAfter(deadline: .now() + interval * Double(i)) {
                guard self.isDimmed else { return }
                UIScreen.main.brightness = self.savedBrightness - brightnessStep * CGFloat(i)
            }
        }

        #if DEBUG
        print("🌙 [屏幕] 开始变暗，起始亮度: \(String(format: "%.2f", savedBrightness))")
        #endif
    }

    func wakeUpScreen() {
        guard isDimmed else {
            resetDimTimer()
            return
        }

        isDimmed = false
        UIScreen.main.brightness = savedBrightness
        resetDimTimer()
        #if DEBUG
        print("🔆 [屏幕] 已唤醒")
        #endif
    }

    /// 用户交互时调用（触摸、输入等）
    func onUserInteraction() {
        if isDimmed {
            wakeUpScreen()
        } else {
            resetDimTimer()
        }
    }

    // MARK: - 生命周期

    func handleAppDidEnterBackground() {
        stopDimTimer()
    }

    func handleAppWillEnterForeground() {
        if isDimmed { wakeUpScreen() }
        if isKeepScreenOn {
            UIApplication.shared.isIdleTimerDisabled = true
            resetDimTimer()
        }
    }

    func applyInitialSettings() {
        if isKeepScreenOn {
            UIApplication.shared.isIdleTimerDisabled = true
            resetDimTimer()
        }
        setupWindowTouchDetection()
    }

    // 在窗口层监听任意触碰，确保屏幕变暗后任何操作都能唤醒
    private func setupWindowTouchDetection() {
        DispatchQueue.main.async {
            guard let scene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
                  let window = scene.windows.first else { return }
            guard !(window.gestureRecognizers?.contains(where: { $0 is AnyTouchRecognizer }) ?? false) else { return }
            let recognizer = AnyTouchRecognizer()
            recognizer.cancelsTouchesInView = false
            recognizer.delaysTouchesBegan = false
            window.addGestureRecognizer(recognizer)
        }
    }
}

// 窗口级触碰检测器：state 立即置为 .failed，不消费事件，其他手势正常工作
private class AnyTouchRecognizer: UIGestureRecognizer {
    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent) {
        state = .failed
        ScreenDimManager.shared.onUserInteraction()
    }
    override func canPrevent(_ preventedGestureRecognizer: UIGestureRecognizer) -> Bool { false }
    override func canBePrevented(by preventingGestureRecognizer: UIGestureRecognizer) -> Bool { false }
}
