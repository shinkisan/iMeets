import CoreImage
import CoreImage.CIFilterBuiltins
import Darwin
import Foundation
import Network
import UIKit

/// One TCP link between two app instances. The firmware is the IR stack;
/// this object moves UART bytes and exchanges a heartbeat every 20 ms.
/// The firmware pump runs independently; waiting here does not pause it.
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
    private var connectTimeout: DispatchWorkItem?
    private var useWifi = true
    // Each counter is owned by its corresponding queue, across trace sessions.
    private var txTicks: UInt64 = 0
    private var rxTicks: UInt64 = 0
    private var lastTransportTick: TimeInterval = 0

    func host() {
        stop()
        LanDiagnostics.shared.start(role: "host")
        engine?.traceLanState("host_start")
        mode = .hosting
        LanDiagnostics.console("LAN host start")
        do {
            let listener = try NWListener(using: .tcp, on: .any)
            self.listener = listener
            listener.newConnectionHandler = { [weak self] conn in
                Task { @MainActor in self?.accept(conn) }
            }
            listener.stateUpdateHandler = { [weak self, weak listener] state in
                guard let self, let listener else { return }
                LanDiagnostics.shared.record("listener_state", ["state": String(describing: state)])
                switch state {
                case .ready:
                    let p = listener.port?.rawValue ?? 0
                    let payload = Self.payload(port: p)
                    // Do not persist the token/code; only the routing information.
                    LanDiagnostics.shared.record("listening", ["port": p, "hosts": Self.localIPs()])
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
        LanDiagnostics.shared.start(role: "join")
        engine?.traceLanState("join_start")
        guard let parsed = Self.parse(payload) else {
            LanDiagnostics.console("LAN join invalid payload")
            fail("二维码无效")
            return
        }
        LanDiagnostics.console("LAN join hosts=\(parsed.hosts) port=\(parsed.port)")
        hosts = parsed.hosts
        LanDiagnostics.shared.record("join_targets", ["hosts": parsed.hosts, "port": parsed.port])
        port = parsed.port
        nextHost = 0
        useWifi = true
        mode = .joining
        connectNext()
    }

    func stop() {
        if LanDiagnostics.shared.active {
            engine?.traceLanState("stop_state")
            LanDiagnostics.shared.finish("stop")
        }
        cancelConnectTimeout()
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
        LanDiagnostics.shared.record("accept", ["endpoint": String(describing: conn.endpoint)])
        connection?.cancel()
        connection = conn
        conn.stateUpdateHandler = { [weak self] state in
            guard let self else { return }
            LanDiagnostics.shared.record("connection_state", ["state": String(describing: state)])
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

    private func cancelConnectTimeout() {
        connectTimeout?.cancel()
        connectTimeout = nil
    }

    private func connectNext() {
        cancelConnectTimeout()
        guard nextHost < hosts.count else {
            // A VPN can capture the default route even though the devices
            // share a Wi-Fi network. Retry every host once bound to Wi-Fi
            // before giving up; that also covers wired-only Macs.
            if useWifi {
                useWifi = false
                nextHost = 0
                LanDiagnostics.console("LAN retry without interface binding")
                connectNext()
                return
            }
            fail(failure.isEmpty ? "无法连接" : failure)
            return
        }
        let host = hosts[nextHost]
        nextHost += 1
        LanDiagnostics.console("LAN try \(host):\(port) wifi=\(useWifi)")
        LanDiagnostics.shared.record("connect_attempt", ["host": host, "port": port, "wifi": useWifi])
        guard let nwPort = NWEndpoint.Port(rawValue: port) else {
            fail("端口无效")
            return
        }
        let params = NWParameters.tcp
        if useWifi {
            // Bind to Wi-Fi so a VPN/utun default route cannot swallow the
            // connection to the other device on the same network.
            params.requiredInterfaceType = .wifi
        }
        let conn = NWConnection(host: NWEndpoint.Host(host), port: nwPort, using: params)
        connection = conn
        conn.stateUpdateHandler = { [weak self] state in
            guard let self else { return }
            LanDiagnostics.shared.record("connection_state", ["state": String(describing: state), "host": host])
            switch state {
            case .ready:
                LanDiagnostics.console("LAN connected")
                self.cancelConnectTimeout()
                Task { @MainActor in self.begin() }
            case .failed(let error):
                LanDiagnostics.console("LAN connect failed \(host): \(error.localizedDescription)")
                self.cancelConnectTimeout()
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
        // A host that black-holes (a stale address, or 127.0.0.1 on a phone)
        // can take over a minute to fail; give each host a short window.
        let work = DispatchWorkItem { [weak self] in
            guard let self, self.mode == .joining, !self.started else { return }
            LanDiagnostics.console("LAN timeout \(host)")
            LanDiagnostics.shared.record("connect_timeout", ["host": host])
            self.failure = "连接超时"
            self.connection?.cancel()
            self.connection = nil
            self.connectNext()
        }
        connectTimeout = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 3, execute: work)
    }

    private func begin() {
        cancelConnectTimeout()
        guard let conn = connection, !started else { return }
        started = true
        running = true
        connected = true
        mode = .connected
        LanDiagnostics.shared.record("connected", ["endpoint": String(describing: conn.endpoint),
            "path": String(describing: conn.currentPath), "transport_ms": 20,
            "firmware_lockstep": false])
        engine?.traceLanState("connected_state")
        _ = engine?.saveNow()
        receive(conn)
        let timer = DispatchSource.makeTimerSource(queue: tickQueue)
        timer.schedule(deadline: .now(), repeating: .milliseconds(20))
        timer.setEventHandler { [weak self] in self?.step() }
        timer.resume()
        self.timer = timer
    }

    private func step() {
        guard running, let conn = connection else { return }
        let trace = LanDiagnostics.shared.active
        let start = trace ? ProcessInfo.processInfo.systemUptime : 0
        let interval = trace && lastTransportTick != 0 ? (start - lastTransportTick) * 1000 : 0
        if trace {
            lastTransportTick = start
            txTicks &+= 1
        }
        if let out = engine?.irRead(), !out.isEmpty {
            send(conn, type: 0, data: out)
        }
        send(conn, type: 1, data: [])
        let waitStart = trace ? ProcessInfo.processInfo.systemUptime : 0
        let result = tick.wait(timeout: .now() + 2.0)
        guard trace else { return }
        let waitMs = (ProcessInfo.processInfo.systemUptime - waitStart) * 1000
        if txTicks % 50 == 0 || result == .timedOut || waitMs > 100 || interval > 100 {
            LanDiagnostics.shared.record("transport_tick", ["tx_tick": txTicks,
                "interval_ms": interval, "wait_ms": waitMs, "timeout": result == .timedOut,
                "total_ms": (ProcessInfo.processInfo.systemUptime - start) * 1000])
        }
    }

    private func send(_ conn: NWConnection, type: UInt8, data: [UInt8]) {
        var frame = Data([type, UInt8(data.count & 0xff), UInt8((data.count >> 8) & 0xff)])
        frame.append(contentsOf: data)
        conn.send(content: frame, completion: .contentProcessed { error in
            if let error {
                LanDiagnostics.shared.record("send_error", ["type": type, "count": data.count,
                    "error": error.localizedDescription])
            }
        })
    }

    private func receive(_ conn: NWConnection) {
        conn.receive(minimumIncompleteLength: 1, maximumLength: 65536) { [weak self] data, _, complete, error in
            guard let self, self.running else { return }
            if let data, !data.isEmpty {
                self.rx.append(data)
                self.parse()
            }
            if let error {
                LanDiagnostics.shared.record("receive_error", ["error": error.localizedDescription])
                Task { @MainActor in self.fail(error.localizedDescription) }
                return
            }
            if complete {
                LanDiagnostics.shared.record("peer_closed")
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
                if LanDiagnostics.shared.active {
                    rxTicks &+= 1
                    if rxTicks % 50 == 0 {
                        LanDiagnostics.shared.record("peer_tick", ["rx_tick": rxTicks, "buffered_bytes": rx.count])
                    }
                }
                tick.signal()
            } else {
                LanDiagnostics.shared.record("unknown_frame", ["type": type, "count": len])
            }
        }
    }

    private func fail(_ message: String) {
        LanDiagnostics.console("LAN fail: \(message)")
        LanDiagnostics.shared.record("failure", ["message": message])
        engine?.traceLanState("failure_state")
        LanDiagnostics.shared.finish("failure")
        cancelConnectTimeout()
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
        LanDiagnostics.console("LAN PAYLOAD \(text)")
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
        var out: [String] = []
        var head: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&head) == 0, let first = head else { return ["127.0.0.1"] }
        defer { freeifaddrs(head) }
        for ptr in sequence(first: first, next: { $0.pointee.ifa_next }) {
            let ifa = ptr.pointee
            guard let addr = ifa.ifa_addr, addr.pointee.sa_family == UInt8(AF_INET) else { continue }
            let name = String(cString: ifa.ifa_name)
            if name == "lo0" { continue }
            // VPN / peer-to-peer interfaces are not reachable from the other
            // device; advertising them only adds a timeout.
            if name.hasPrefix("utun") || name.hasPrefix("tun") || name.hasPrefix("tap")
                || name.hasPrefix("awdl") || name.hasPrefix("llw") || name.hasPrefix("bridge") {
                continue
            }
            var host = [CChar](repeating: 0, count: Int(NI_MAXHOST))
            let rc = host.withUnsafeMutableBufferPointer { buf in
                getnameinfo(addr, socklen_t(addr.pointee.sa_len), buf.baseAddress, socklen_t(buf.count), nil, 0, NI_NUMERICHOST)
            }
            guard rc == 0 else { continue }
            let ip = host.withUnsafeBufferPointer { String(cString: $0.baseAddress!) }
            // Link-local addresses are not reachable from the other device.
            if ip.hasPrefix("169.254.") { continue }
            // Clash/Surge fake-IP range.
            if ip.hasPrefix("198.18.") || ip.hasPrefix("198.19.") { continue }
            if !out.contains(ip) { out.append(ip) }
        }
        // Loopback is only a fallback for two simulators on one Mac; trying it
        // first from a phone just stalls until the TCP timeout.
        out.append("127.0.0.1")
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
