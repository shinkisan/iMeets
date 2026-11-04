import CoreBluetooth
import SwiftUI

/// GATT peripheral for WebMeets. Payloads are forwarded, not interpreted.
/// TODO(att-layout): SPEC §14.4. Opcode bytes are not invented.
/// TODO(ble-adapter-name): iOS cannot rename the system adapter. The local name is advertised as TMGC_meets.
struct WebMeetsView: View {
    @EnvironmentObject private var model: AppModel
    @StateObject private var radio = WebRadio()

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(model.t("ble_waiting")).font(.headline)
            Text(radio.status)
            Text(model.t("help_web")).font(.footnote)
            Button(model.t("end")) { radio.stop(model: model, committed: radio.committed) }
        }
        .padding()
        .onAppear { radio.start(model: model) }
        .onDisappear { radio.stop(model: model, committed: radio.committed) }
        .onChange(of: model.endWeb) { _ in
            radio.stop(model: model, committed: radio.committed)
        }
    }
}

/// `queue: nil` delivers callbacks on the main thread. `respond` has to
/// run before those methods return, so the delegate hops with
/// `MainActor.assumeIsolated` instead of an async task.
@MainActor
final class WebRadio: NSObject, ObservableObject, CBPeripheralManagerDelegate {
    @Published var status = ""
    @Published var committed = false
    private var manager: CBPeripheralManager?
    private var tx: CBMutableCharacteristic?
    private var owner: CBCentral?
    private var subscribed = false
    private var rxQueued = 0
    private var txQueued: [[UInt8]] = []
    private var model: AppModel?
    private var idle: Timer?
    private var grace: Timer?
    private var sawActivity = false
    private var finishing = false
    private var stopped = false

    func start(model: AppModel) {
        stopped = false
        self.model = model
        status = model.t("ble_waiting")
        manager = CBPeripheralManager(delegate: self, queue: nil)
        armIdle()
    }

    func stop(model: AppModel, committed: Bool) {
        if stopped { return }
        stopped = true
        idle?.invalidate()
        grace?.invalidate()
        manager?.stopAdvertising()
        manager = nil
        if committed {
            model.notice = model.t("gift_saved")
            _ = model.engine.saveNow()
        } else if !finishing {
            model.engine.linkCancel()
            model.notice = model.t("conn_interrupted")
        }
        if model.page == .web { model.page = .toy }
    }

    nonisolated func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
        MainActor.assumeIsolated {
            guard let model = self.model else { return }
            switch peripheral.state {
            case .poweredOn:
                self.addService(peripheral)
            case .poweredOff:
                model.notice = model.t("ble_off")
                self.stop(model: model, committed: false)
            case .unauthorized:
                model.notice = model.t("ble_permission")
                self.stop(model: model, committed: false)
            case .unsupported:
                model.notice = model.t("ble_no_radio")
                self.stop(model: model, committed: false)
            default:
                break
            }
        }
    }

    private func addService(_ peripheral: CBPeripheralManager) {
        let txChar = CBMutableCharacteristic(
            type: CBUUID(string: PM_TX_UUID),
            properties: [.notify, .read],
            value: nil,
            permissions: [.readable]
        )
        let rxChar = CBMutableCharacteristic(
            type: CBUUID(string: PM_RX_UUID),
            properties: [.write, .writeWithoutResponse],
            value: nil,
            permissions: [.writeable]
        )
        let service = CBMutableService(type: CBUUID(string: PM_SERVICE_UUID), primary: true)
        service.characteristics = [txChar, rxChar]
        tx = txChar
        peripheral.add(service)
    }

    nonisolated func peripheralManager(_ peripheral: CBPeripheralManager, didAdd service: CBService, error: Error?) {
        MainActor.assumeIsolated {
            guard error == nil else {
                self.status = error?.localizedDescription ?? ""
                return
            }
            peripheral.startAdvertising([
                CBAdvertisementDataServiceUUIDsKey: [CBUUID(string: PM_SERVICE_UUID)],
                CBAdvertisementDataLocalNameKey: PM_BLE_NAME
            ])
        }
    }

    nonisolated func peripheralManager(_ peripheral: CBPeripheralManager, central: CBCentral, didSubscribeTo characteristic: CBCharacteristic) {
        MainActor.assumeIsolated {
            if self.owner == nil {
                self.owner = central
                self.subscribed = true
                peripheral.stopAdvertising()
                self.model?.engine.linkConnect()
                self.touch()
                self.flush()
                self.status = self.model?.t("ble_waiting") ?? ""
            } else if self.owner?.identifier != central.identifier {
                peripheral.stopAdvertising()
            }
        }
    }

    nonisolated func peripheralManager(_ peripheral: CBPeripheralManager, central: CBCentral, didUnsubscribeFrom characteristic: CBCharacteristic) {
        MainActor.assumeIsolated {
            self.subscribed = false
            self.beginGrace()
        }
    }

    nonisolated func peripheralManager(_ peripheral: CBPeripheralManager, didReceiveRead request: CBATTRequest) {
        MainActor.assumeIsolated {
            guard self.model != nil, request.central.identifier == self.owner?.identifier, request.offset == 0 else {
                peripheral.respond(to: request, withResult: .readNotPermitted)
                return
            }
            request.value = Data()
            peripheral.respond(to: request, withResult: .success)
        }
    }

    nonisolated func peripheralManager(_ peripheral: CBPeripheralManager, didReceiveWrite requests: [CBATTRequest]) {
        MainActor.assumeIsolated {
            guard let model = self.model else { return }
            for request in requests {
                let prepared = false
                let bytes = [UInt8](request.value ?? Data())
                let ownerOK = request.central.identifier == self.owner?.identifier
                let accept = model.engine.gattAccept(
                    owner: ownerOK, subscribed: self.subscribed, prepared: prepared,
                    offset: Int(request.offset), length: bytes.count, queued: self.rxQueued
                )
                if !accept || bytes.count > 20 {
                    peripheral.respond(to: request, withResult: .writeNotPermitted)
                    if self.rxQueued >= 128 {
                        self.fail(model.t("ble_failed"))
                    }
                    continue
                }
                self.rxQueued += 1
                if model.engine.linkWrite(bytes) {
                    self.rxQueued = max(0, self.rxQueued - 1)
                }
                self.touch()
                peripheral.respond(to: request, withResult: .success)
                self.pollNative(model)
            }
        }
    }

    nonisolated func peripheralManagerIsReady(toUpdateSubscribers peripheral: CBPeripheralManager) {
        MainActor.assumeIsolated {
            self.flush()
        }
    }

    private func pollNative(_ model: AppModel) {
        let state = model.engine.linkState()
        if state == 3 { status = "receiving" }
        if state == 5 {
            committed = true
            Task { @MainActor in
                try? await Task.sleep(nanoseconds: 300_000_000)
                guard let model = self.model else { return }
                self.finishing = true
                self.manager?.stopAdvertising()
                model.notice = model.t("gift_saved")
                _ = model.engine.saveNow()
                model.page = .toy
            }
        } else if state < 0 && !committed {
            fail(model.t("ble_failed"))
        }
        flush()
    }

    private func flush() {
        guard let model, let tx, subscribed else { return }
        while txQueued.count < 512 {
            let bytes = model.engine.linkRead()
            if bytes.isEmpty { break }
            if bytes.count > 20 {
                fail(model.t("ble_failed"))
                return
            }
            txQueued.append(bytes)
        }
        while let next = txQueued.first {
            let sent = manager?.updateValue(Data(next), for: tx, onSubscribedCentrals: nil) ?? false
            if !sent { break }
            txQueued.removeFirst()
        }
    }

    private func touch() {
        sawActivity = true
        armIdle()
    }

    private func armIdle() {
        idle?.invalidate()
        let seconds = sawActivity ? 120.0 : 300.0
        idle = Timer.scheduledTimer(withTimeInterval: seconds, repeats: false) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let self, let model = self.model else { return }
                self.fail(model.t("ble_timeout"))
            }
        }
    }

    private func beginGrace() {
        grace?.invalidate()
        grace = Timer.scheduledTimer(withTimeInterval: 2.5, repeats: false) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let self, let model = self.model else { return }
                if self.committed {
                    model.notice = model.t("gift_saved")
                    _ = model.engine.saveNow()
                    model.page = .toy
                } else {
                    model.engine.linkCancel()
                    model.notice = model.t("conn_interrupted")
                    model.page = .toy
                }
            }
        }
    }

    private func fail(_ text: String) {
        finishing = true
        stopped = true
        idle?.invalidate()
        grace?.invalidate()
        model?.engine.linkCancel()
        model?.notice = text
        manager?.stopAdvertising()
        manager = nil
        if model?.page == .web { model?.page = .toy }
    }
}
