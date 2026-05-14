import SwiftUI

struct Config {
    static let adb = ProcessInfo.processInfo.environment["HOME"]! + "/Library/Android/sdk/platform-tools/adb"
    static let scrcpy = "/opt/homebrew/bin/scrcpy"
    static let home = ProcessInfo.processInfo.environment["HOME"]!
}

func runSilent(_ cmd: String) {
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/bin/bash")
    task.arguments = ["-c", cmd]
    task.standardOutput = FileHandle.nullDevice
    task.standardError = FileHandle.nullDevice
    try? task.run()
}

func shell(_ cmd: String) -> String {
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/bin/bash")
    task.arguments = ["-c", cmd]
    let pipe = Pipe()
    task.standardOutput = pipe
    task.standardError = FileHandle.nullDevice
    try? task.run()
    task.waitUntilExit()
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    return String(data: data, encoding: .utf8) ?? ""
}

func notify(_ title: String, _ body: String) {
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
    task.arguments = ["-e", "display notification \"\(body)\" with title \"\(title)\""]
    try? task.run()
    task.waitUntilExit()
}

func hasDevice() -> Bool {
    let output = shell("\(Config.adb) devices")
    let lines = output.components(separatedBy: "\n")
    for line in lines.dropFirst() {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        if trimmed.contains("device") && !trimmed.contains("daemon") && !trimmed.isEmpty {
            return true
        }
    }
    return false
}

func checkDevice() -> String {
    let output = shell("\(Config.adb) devices")
    let lines = output.components(separatedBy: "\n")
    for line in lines.dropFirst() {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        if trimmed.contains("device") && !trimmed.contains("daemon") && !trimmed.isEmpty {
            let id = trimmed.components(separatedBy: CharacterSet.whitespaces).first ?? "unknown"
            return "📱 \(id)"
        }
    }
    return "📵 No device"
}

func isScrcpyRunning() -> Bool {
    let result = shell("pgrep -x scrcpy 2>/dev/null")
    return !result.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
}

func takeScreenshot() {
    let timestamp = shell("date +%Y%m%d_%H%M%S").trimmingCharacters(in: .whitespacesAndNewlines)
    let filename = "\(Config.home)/Pictures/phone_screenshot_\(timestamp).png"
    _ = shell("\(Config.adb) shell screencap -p /sdcard/phone_screenshot.png")
    _ = shell("\(Config.adb) pull /sdcard/phone_screenshot.png \"\(filename)\"")
    _ = shell("\(Config.adb) shell rm /sdcard/phone_screenshot.png")
    let check = shell("test -f \"\(filename)\" && echo OK || echo FAIL")
    if check.contains("OK") {
        notify("📸 Screenshot Saved", filename)
    } else {
        notify("❌ Screenshot Failed", "Is your phone connected?")
    }
}

func startRecording() {
    let timestamp = shell("date +%Y%m%d_%H%M%S").trimmingCharacters(in: .whitespacesAndNewlines)
    let filename = "\(Config.home)/Movies/phone_recording_\(timestamp).mp4"
    runSilent("\(Config.scrcpy) --record \(filename) --no-playback --no-audio --no-window 2>/dev/null &")
    notify("🎬 Recording Started", "Saving to: \(filename)")
}

func startMirror() {
    let task = Process()
    task.executableURL = URL(fileURLWithPath: Config.scrcpy)
    task.arguments = ["--shortcut-mod=lctrl"]
    task.standardOutput = FileHandle.nullDevice
    task.standardError = FileHandle.nullDevice
    try? task.run()
    notify("📱 Mirror Started", "scrcpy window opened")
}

func closeMirror() {
    runSilent("pkill -x scrcpy 2>/dev/null")
    notify("📵 Mirror Closed", "scrcpy window closed")
}

class AppState: ObservableObject {
    @Published var status: String = "Checking..."
    @Published var isRecording: Bool = false
    @Published var deviceConnected: Bool = false
    @Published var mirrorRunning: Bool = false
    private var timer: Timer?
    private var wasConnected: Bool = false
    
    init() {
        refresh()
        startPolling()
    }
    
    deinit {
        timer?.invalidate()
    }
    
    func startPolling() {
        timer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { [weak self] _ in
            self?.refresh()
        }
    }
    
    func refresh() {
        DispatchQueue.global(qos: .userInitiated).async {
            let connected = hasDevice()
            let statusText = checkDevice()
            let mirrorOn = isScrcpyRunning()
            DispatchQueue.main.async {
                // Notify when phone connects
                if connected && !self.wasConnected {
                    notify("PhoneMirror", "📱 Phone connected!")
                }
                // Notify when phone disconnects
                if !connected && self.wasConnected {
                    notify("PhoneMirror", "📵 Phone disconnected")
                }
                self.wasConnected = connected
                self.deviceConnected = connected
                self.status = statusText
                self.mirrorRunning = mirrorOn
            }
        }
    }
    
    func screenshot() {
        DispatchQueue.global(qos: .userInitiated).async {
            takeScreenshot()
        }
    }
    
    func mirror() {
        DispatchQueue.global(qos: .userInitiated).async {
            startMirror()
            DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
                self.mirrorRunning = true
            }
        }
    }
    
    func closeMirrorAction() {
        DispatchQueue.global(qos: .userInitiated).async {
            closeMirror()
            DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
                self.mirrorRunning = false
            }
        }
    }
    
    func record() {
        if isRecording {
            runSilent("pkill -f 'scrcpy.*--record' 2>/dev/null")
            isRecording = false
            notify("⏹ Recording Stopped", "Check Movies folder for the file")
        } else {
            isRecording = true
            DispatchQueue.global(qos: .userInitiated).async {
                startRecording()
            }
        }
    }
}

@main
struct PhoneMirrorApp: App {
    @StateObject private var app = AppState()
    
    var body: some Scene {
        MenuBarExtra("PhoneMirror", systemImage: app.deviceConnected ? "iphone" : "iphone.slash") {
            VStack(alignment: .leading, spacing: 6) {
                Text(app.status)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                
                if !app.deviceConnected {
                    Text("Waiting for phone...")
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
                
                Divider()
                
                if app.deviceConnected {
                    if app.mirrorRunning {
                        Button("📵 Close Mirror") {
                            app.closeMirrorAction()
                        }
                    } else {
                        Button("📱 Mirror Screen") {
                            app.mirror()
                        }
                    }
                    
                    Button("📸 Screenshot") {
                        app.screenshot()
                    }
                    
                    if app.isRecording {
                        Button("⏹ Stop Recording") {
                            app.record()
                        }
                    } else {
                        Button("🎬 Record Video") {
                            app.record()
                        }
                    }
                    
                    Divider()
                }
                
                Button("🔄 Refresh") {
                    app.refresh()
                }
                
                Divider()
                
                Button("Quit PhoneMirror") {
                    runSilent("pkill -x scrcpy 2>/dev/null")
                    NSApplication.shared.terminate(nil)
                }
            }
            .padding(8)
        }
        .menuBarExtraStyle(.menu)
    }
}