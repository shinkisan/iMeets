import CoreImage
import CoreImage.CIFilterBuiltins
import Darwin
import Foundation
import Network
import UIKit

/// One TCP link between two app instances. The firmware is the IR stack;
/// this object only moves UART bytes and keeps both sides on the same 20 ms
/// tick. The user keeps the native device controls.
final class LanLink: ObservableObject {
    enum Mode: Equatable {
        case idle
        case hosting
        case joining
        case connected
        case failed(String)
    }

    @Published private(set) var mode: Mode = .idle
    @Published private(set) var qr: UIImage?
    @Published private(set) var payload = ""
    @Published private(set) var connected = false

    weak var engine: Engine?

    private var listener: NWListener?
    private var connection: NWConnection?
    private let netQueue = DispatchQueue(label: "pm.lan.net")
    private let tickQueue = DispatchQueue(label: "pm.lan.tick")
    private var timer: DispatchSourceTimer?
    private let tick = DispatchSemaphore(value: 0)
    private var rx = Data()
    private var running = false
    private var started = false
    private var hosts: [String] = []
    private var nextHost = 0
    private var port: UInt16 = 0
    private var failure = ""

    func host() {
        stop()
        mode = .hosting
        do {
            let listener = try NWListener(using: .tcp, on: .any)
            self.listener = listener
            listener.newConnectionHandler = { [weak self] conn in
                Task { @MainActor in self?.accept(conn) }
            }
            listener.stateUpdateHandler = { [weak self, weak listener] state in
                guard let self, let listener else { return }
                switch state {
                case .ready:
                    let p = listener.port?.rawValue ?? 0
                    let payload = Self.payload(port: p)
                    if let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first {
                        try? payload.write(to: dir.appendingPathComponent("lan-payload.txt"), atomically: true, encoding: .utf8)
                    }
                    let image = Self.qrImage(payload)
                    Task { @MainActor in
                        self.port = p
                        self.qr = image
                        self.payload = payload
                        self.mode = .hosting
                    }
                case .failed(let error):
                    Task { @MainActor in self.fail(error.localizedDescription) }
                default:
                    break
                }
            }
            listener.start(queue: netQueue)
        } catch {
            fail(error.localizedDescription)
        }
    }

    func join(payload: String) {
        stop()
        guard let parsed = Self.parse(payload) else {
            fail("二维码无效")
            return
        }
        hosts = parsed.hosts
        port = parsed.port
        nextHost = 0
        mode = .joining
        connectNext()
    }

    func stop() {
        running = false
        started = false
        timer?.cancel()
        timer = nil
        connection?.cancel()
        connection = nil
        listener?.cancel()
        listener = nil
        netQueue.sync { rx.removeAll() }
        while tick.wait(timeout: .now()) == .success {}
        connected = false
        mode = .idle
        qr = nil
        payload = ""
        hosts = []
        nextHost = 0
    }

    private func accept(_ conn: NWConnection) {
        connection?.cancel()
        connection = conn
        conn.stateUpdateHandler = { [weak self] state in
            guard let self else { return }
            switch state {
            case .ready:
                Task { @MainActor in self.begin() }
            case .failed(let error):
                Task { @MainActor in self.fail(error.localizedDescription) }
            default:
                break
            }
        }
        conn.start(queue: netQueue)
    }

    private func connectNext() {
        guard nextHost < hosts.count else {
            fail(failure.isEmpty ? "无法连接" : failure)
            return
        }
        let host = hosts[nextHost]
        nextHost += 1
        guard let nwPort = NWEndpoint.Port(rawValue: port) else {
            fail("端口无效")
            return
        }
        let conn = NWConnection(host: NWEndpoint.Host(host), port: nwPort, using: .tcp)
        connection = conn
        conn.stateUpdateHandler = { [weak self] state in
            guard let self else { return }
            switch state {
            case .ready:
                Task { @MainActor in self.begin() }
            case .failed(let error):
                Task { @MainActor in
                    self.failure = error.localizedDescription
                    guard self.mode == .joining, !self.started else { return }
                    self.connectNext()
                }
            default:
                break
            }
        }
        conn.start(queue: netQueue)
    }

    private func begin() {
        guard let conn = connection, !started else { return }
        started = true
        running = true
        connected = true
        mode = .connected
        receive(conn)
        let timer = DispatchSource.makeTimerSource(queue: tickQueue)
        timer.schedule(deadline: .now(), repeating: .milliseconds(20))
        timer.setEventHandler { [weak self] in self?.step() }
        timer.resume()
        self.timer = timer
    }

    private func step() {
        guard running, let conn = connection else { return }
        if let out = engine?.irRead(), !out.isEmpty {
            send(conn, type: 0, data: out)
        }
        send(conn, type: 1, data: [])
        _ = tick.wait(timeout: .now() + 2.0)
    }

    private func send(_ conn: NWConnection, type: UInt8, data: [UInt8]) {
        var frame = Data([type, UInt8(data.count & 0xff), UInt8((data.count >> 8) & 0xff)])
        frame.append(contentsOf: data)
        conn.send(content: frame, completion: .contentProcessed { _ in })
    }

    private func receive(_ conn: NWConnection) {
        conn.receive(minimumIncompleteLength: 1, maximumLength: 65536) { [weak self] data, _, complete, error in
            guard let self, self.running else { return }
            if let data, !data.isEmpty {
                self.rx.append(data)
                self.parse()
            }
            if let error {
                Task { @MainActor in self.fail(error.localizedDescription) }
                return
            }
            if complete {
                Task { @MainActor in self.fail("对方已断开") }
                return
            }
            if self.running {
                self.receive(conn)
            }
        }
    }

    private func parse() {
        guard running else { return }
        while rx.count >= 3 {
            let start = rx.startIndex
            let type = rx[start]
            let len = Int(rx[start + 1]) | (Int(rx[start + 2]) << 8)
            guard rx.count >= 3 + len else { break }
            let body = [UInt8](rx[(start + 3)..<(start + 3 + len)])
            rx.removeFirst(3 + len)
            if type == 0 {
                engine?.irWrite(body)
            } else if type == 1 {
                tick.signal()
            }
        }
    }

    private func fail(_ message: String) {
        guard running || mode == .joining || mode == .hosting else { return }
        running = false
        connected = false
        mode = .failed(message)
        timer?.cancel()
        timer = nil
        connection?.cancel()
        connection = nil
        listener?.cancel()
        listener = nil
        while tick.wait(timeout: .now()) == .success {}
    }

    private static func payload(port: UInt16) -> String {
        let token = UUID().uuidString.replacingOccurrences(of: "-", with: "").lowercased()
        let code = String(format: "%06d", Int.random(in: 0..<1_000_000))
        let hosts = localIPs().joined(separator: ",")
        let text = "tama://join?v=1&h=\(hosts)&p=\(port)&t=\(token)&c=\(code)"
        print("LAN PAYLOAD \(text)")
        return text
    }

    private static func parse(_ text: String) -> (hosts: [String], port: UInt16)? {
        guard let query = text.split(separator: "?", maxSplits: 1).last else { return nil }
        var hosts: [String] = []
        var port: UInt16?
        for part in query.split(separator: "&") {
            let kv = part.split(separator: "=", maxSplits: 1)
            guard kv.count == 2 else { continue }
            let key = String(kv[0])
            let value = String(kv[1])
            if key == "h" {
                hosts = value.split(separator: ",").map(String.init)
            } else if key == "p" {
                port = UInt16(value)
            }
        }
        guard !hosts.isEmpty, let port else { return nil }
        return (hosts, port)
    }

    private static func localIPs() -> [String] {
        var out = ["127.0.0.1"]
        var head: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&head) == 0, let first = head else { return out }
        defer { freeifaddrs(head) }
        for ptr in sequence(first: first, next: { $0.pointee.ifa_next }) {
            let ifa = ptr.pointee
            guard let addr = ifa.ifa_addr, addr.pointee.sa_family == UInt8(AF_INET) else { continue }
            if String(cString: ifa.ifa_name) == "lo0" { continue }
            var host = [CChar](repeating: 0, count: Int(NI_MAXHOST))
            let rc = host.withUnsafeMutableBufferPointer { buf in
                getnameinfo(addr, socklen_t(addr.pointee.sa_len), buf.baseAddress, socklen_t(buf.count), nil, 0, NI_NUMERICHOST)
            }
            guard rc == 0 else { continue }
            let ip = host.withUnsafeBufferPointer { String(cString: $0.baseAddress!) }
            if !out.contains(ip) { out.append(ip) }
        }
        return out
    }

    private static func qrImage(_ text: String) -> UIImage? {
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(text.utf8)
        filter.correctionLevel = "M"
        guard let output = filter.outputImage else { return nil }
        let scaled = output.transformed(by: CGAffineTransform(scaleX: 10, y: 10))
        let context = CIContext()
        guard let cg = context.createCGImage(scaled, from: scaled.extent) else { return nil }
        return UIImage(cgImage: cg)
    }
}
