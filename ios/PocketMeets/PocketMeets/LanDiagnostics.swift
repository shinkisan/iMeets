import Foundation
import OSLog
import UIKit

/// Local, bounded JSONL traces. File IO never runs on the firmware/network queues.
/// Disabled by default. Explicitly launch with -pmLanTrace to record a session.
final class LanDiagnostics {
    static let shared = LanDiagnostics()
    static let revision = "lan-trace-v1"
    static let enabled = ProcessInfo.processInfo.arguments.contains("-pmLanTrace")

    static func console(_ message: @autoclosure () -> String) {
        guard enabled else { return }
        print(message())
    }

    private let lock = NSLock()
    private let writer = DispatchQueue(label: "pm.lan.diagnostics", qos: .utility)
    private let logger = Logger(subsystem: "app.pocketmeets.ios", category: "LAN")
    private var run: String?
    private var sequence = 0
    private var pending = 0
    private var dropped = 0
    // Only the writer queue touches these fields.
    private var handle: FileHandle?
    private var base: URL?
    private var part = 0
    private var bytesWritten = 0
    private var observers: [NSObjectProtocol] = []

    var active: Bool {
        guard Self.enabled else { return false }
        lock.lock()
        defer { lock.unlock() }
        return run != nil
    }

    private init() {
        guard Self.enabled else { return }
        for name in [UIApplication.willResignActiveNotification,
                     UIApplication.didEnterBackgroundNotification,
                     UIApplication.didBecomeActiveNotification,
                     UIApplication.willTerminateNotification] {
            observers.append(NotificationCenter.default.addObserver(forName: name, object: nil, queue: nil) { [weak self] note in
                self?.record("lifecycle", ["notification": note.name.rawValue])
            })
        }
    }

    /// Called on main by LanLink. Each device/run gets its own correlation ID.
    func start(role: String) {
        guard Self.enabled else { return }
        let defaults = UserDefaults.standard
        let installKey = "pm.lanDiagnosticInstallID"
        let install = defaults.string(forKey: installKey) ?? UUID().uuidString
        defaults.set(install, forKey: installKey)
        let id = "\(Int(Date().timeIntervalSince1970 * 1000))-\(role)-\(UUID().uuidString.prefix(8))"
        let metadata: [String: Any] = [
            "revision": Self.revision, "role": role, "install_id": install,
            "version": Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "",
            "build": Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? "",
            "os": UIDevice.current.systemVersion,
            "model": UIDevice.current.model,
            "note": "Transport tick waits do not pause the independent firmware pump."
        ]
        lock.lock()
        run = id
        sequence = 0
        dropped = 0
        writer.async { [self] in
            closeFile()
            do {
                var dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
                    .appendingPathComponent("LanDiagnostics", isDirectory: true)
                try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
                var values = URLResourceValues()
                values.isExcludedFromBackup = true
                try dir.setResourceValues(values)
                base = dir.appendingPathComponent(id)
                part = 0
                try openPart()
                logger.info("PM_LAN_LOG \(base!.path, privacy: .public) revision=\(Self.revision, privacy: .public)")
            } catch {
                logger.error("PM_LAN_LOG open failed: \(error.localizedDescription, privacy: .public)")
            }
        }
        enqueueLocked("start", metadata)
        lock.unlock()
    }

    func record(_ event: String, _ fields: @autoclosure () -> [String: Any] = [:]) {
        guard Self.enabled else { return }
        let fields = fields()
        lock.lock()
        defer { lock.unlock() }
        guard run != nil else { return }
        // Diagnostic storage must not grow an unbounded queue if IO stalls.
        guard pending < 2048 else { dropped += 1; return }
        enqueueLocked(event, fields)
    }

    func finish(_ reason: String) {
        guard Self.enabled else { return }
        lock.lock()
        defer { lock.unlock() }
        guard run != nil else { return }
        enqueueLocked("end", ["reason": reason])
        run = nil
        writer.async { [self] in closeFile() }
    }

    private func enqueueLocked(_ event: String, _ fields: [String: Any]) {
        guard let run else { return }
        sequence += 1
        var row = fields
        row["event"] = event
        row["run"] = run
        row["seq"] = sequence
        row["unix_ms"] = Date().timeIntervalSince1970 * 1000
        row["uptime_s"] = ProcessInfo.processInfo.systemUptime
        row["dropped_events"] = dropped
        dropped = 0
        pending += 1
        let record = row
        writer.async { [self] in
            defer {
                lock.lock()
                pending -= 1
                lock.unlock()
            }
            do {
                guard handle != nil else { return }
                var data = try JSONSerialization.data(withJSONObject: record, options: [.sortedKeys])
                data.append(0x0a)
                if bytesWritten + data.count > 4 * 1024 * 1024 {
                    closeFile()
                    part += 1
                    try openPart()
                }
                try handle?.write(contentsOf: data)
                bytesWritten += data.count
            } catch {
                logger.error("PM_LAN_LOG write failed: \(error.localizedDescription, privacy: .public)")
                closeFile()
            }
        }
    }

    private func openPart() throws {
        guard let base else { return }
        let file = base.appendingPathExtension("\(part).jsonl")
        guard FileManager.default.createFile(atPath: file.path, contents: nil,
                                            attributes: [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication]) else {
            throw CocoaError(.fileWriteUnknown)
        }
        handle = try FileHandle(forWritingTo: file)
        bytesWritten = 0
        // At most twelve 4 MiB parts are retained across all sessions.
        let files = try FileManager.default.contentsOfDirectory(at: file.deletingLastPathComponent(),
                                                               includingPropertiesForKeys: [.creationDateKey])
            .filter { $0.pathExtension == "jsonl" }
            .sorted { lhs, rhs in
                let a = (try? lhs.resourceValues(forKeys: [.creationDateKey]).creationDate) ?? .distantPast
                let b = (try? rhs.resourceValues(forKeys: [.creationDateKey]).creationDate) ?? .distantPast
                return a < b
            }
        for old in files.prefix(max(0, files.count - 12)) where old != file {
            try? FileManager.default.removeItem(at: old)
        }
    }

    private func closeFile() {
        try? handle?.close()
        handle = nil
    }
}
