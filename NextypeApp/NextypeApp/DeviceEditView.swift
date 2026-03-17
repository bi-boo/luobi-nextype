//
//  DeviceEditView.swift
//  NextypeApp
//
//  设备编辑页面 - 修改设备名称和图标
//

import SwiftUI

struct DeviceEditView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var pairedMacManager: PairedMacManager

    let device: PairedMac

    @State private var name: String
    @State private var selectedIcon: String

    private let iconOptions = [
        ("笔记本", "laptopcomputer"),
        ("台式机", "desktopcomputer"),
        ("工作站", "display.2"),
    ]

    init(device: PairedMac) {
        self.device = device
        _name = State(initialValue: device.customName ?? device.deviceName)
        _selectedIcon = State(initialValue: device.customIcon ?? "desktopcomputer")
    }

    var body: some View {
        NavigationStack {
            Form {
                Section(header: Text("设备名称")) {
                    TextField("输入设备名称", text: $name)
                        .submitLabel(.done)
                }

                Section(header: Text("设备图标")) {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 80))], spacing: 16) {
                        ForEach(iconOptions, id: \.1) { option in
                            VStack(spacing: 8) {
                                ZStack {
                                    Circle()
                                        .fill(
                                            selectedIcon == option.1
                                                ? Color.accentColor.opacity(0.15) : Color(.systemGray6)
                                        )
                                        .frame(width: 50, height: 50)

                                    Image(systemName: option.1)
                                        .font(.title3)
                                        .foregroundColor(
                                            selectedIcon == option.1 ? .accentColor : .primary)
                                }

                                Text(option.0)
                                    .font(.caption)
                                    .foregroundColor(selectedIcon == option.1 ? .accentColor : .secondary)
                            }
                            .onTapGesture {
                                let impact = UIImpactFeedbackGenerator(style: .light)
                                impact.impactOccurred()
                                selectedIcon = option.1
                            }
                        }
                    }
                    .padding(.vertical, 8)
                }
            }
            .navigationTitle("编辑设备")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("取消") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("保存") {
                        saveChanges()
                    }
                    .fontWeight(.semibold)
                }
            }
        }
    }

    private func saveChanges() {
        // 如果名字为空，使用原名
        let finalName =
            name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? device.deviceName : name
        pairedMacManager.updateMacCustomInfo(
            deviceId: device.deviceId, name: finalName, icon: selectedIcon)
        dismiss()
    }
}

#Preview {
    DeviceEditView(
        device: PairedMac(
            deviceId: "test", deviceName: "My Mac", ip: "127.0.0.1", port: 1234,
            encryptionKey: "key")
    )
    .environmentObject(PairedMacManager())
}
