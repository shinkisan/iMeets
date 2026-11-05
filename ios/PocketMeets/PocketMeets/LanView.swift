import AVFoundation
import SwiftUI
import UIKit

struct LanView: View {
    @EnvironmentObject private var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var scanning = false
    @State private var manual = ""

    var body: some View {
        NavigationStack {
            content
                .padding(24)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .navigationTitle("联机")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("关闭") {
                            model.lan.stop()
                            dismiss()
                        }
                    }
                }
                .sheet(isPresented: $scanning) {
                    QrScannerView { code in
                        scanning = false
                        model.lan.join(payload: code)
                    }
                }
                .onChange(of: model.lan.connected) { value in
                    if value {
                        model.page = .toy
                        dismiss()
                    }
                }
        }
    }

    @ViewBuilder
    private var content: some View {
        switch model.lan.mode {
        case .idle:
            VStack(spacing: 16) {
                Button {
                    model.lan.host()
                } label: {
                    Label("展示二维码", systemImage: "qrcode")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)

                Button {
                    scanning = true
                } label: {
                    Label("扫码加入", systemImage: "camera.viewfinder")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)

                Button("粘贴连接串") {
                    if let text = UIPasteboard.general.string {
                        model.lan.join(payload: text)
                    }
                }
                .font(.footnote)

                DisclosureGroup("手动连接") {
                    TextField("连接串", text: $manual)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .font(.caption.monospaced())
                    Button("连接") { model.lan.join(payload: manual) }
                        .disabled(manual.isEmpty)
                }
                .font(.footnote)
            }
        case .hosting:
            VStack(spacing: 14) {
                if let qr = model.lan.qr {
                    Image(uiImage: qr)
                        .interpolation(.none)
                        .resizable()
                        .frame(width: 260, height: 260)
                }
                Text("等待扫码")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Button("复制连接串") {
                    UIPasteboard.general.string = model.lan.payload
                }
                .font(.footnote)
                Button("取消") { model.lan.stop() }
            }
        case .joining:
            VStack(spacing: 14) {
                ProgressView()
                Text("连接中")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        case .connected:
            VStack(spacing: 14) {
                Text("已连接")
                    .font(.headline)
                Button("进入设备") {
                    model.page = .toy
                    dismiss()
                }
                .buttonStyle(.borderedProminent)
            }
        case .failed(let message):
            VStack(spacing: 14) {
                Text(message)
                    .font(.footnote)
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.secondary)
                Button("重试") { model.lan.stop() }
            }
        }
    }
}

struct QrScannerView: UIViewControllerRepresentable {
    let onCode: (String) -> Void

    func makeUIViewController(context: Context) -> ScannerController {
        ScannerController(onCode: onCode)
    }

    func updateUIViewController(_ controller: ScannerController, context: Context) {}
}

final class ScannerController: UIViewController, AVCaptureMetadataOutputObjectsDelegate {
    private let onCode: (String) -> Void
    private let session = AVCaptureSession()
    private var preview: AVCaptureVideoPreviewLayer?
    private var handled = false

    init(onCode: @escaping (String) -> Void) {
        self.onCode = onCode
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError()
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized:
            setup()
        case .notDetermined:
            AVCaptureDevice.requestAccess(for: .video) { [weak self] granted in
                DispatchQueue.main.async {
                    guard let self else { return }
                    granted ? self.setup() : self.message("相机权限未开启")
                }
            }
        default:
            message("相机权限未开启")
        }
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        preview?.frame = view.bounds
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        if !session.inputs.isEmpty {
            DispatchQueue.global(qos: .userInitiated).async { [session] in
                session.startRunning()
            }
        }
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        if session.isRunning {
            session.stopRunning()
        }
    }

    private func setup() {
        guard let device = AVCaptureDevice.default(for: .video),
              let input = try? AVCaptureDeviceInput(device: device),
              session.canAddInput(input) else {
            message("相机不可用")
            return
        }
        session.addInput(input)
        let output = AVCaptureMetadataOutput()
        guard session.canAddOutput(output) else {
            message("相机不可用")
            return
        }
        session.addOutput(output)
        output.setMetadataObjectsDelegate(self, queue: .main)
        output.metadataObjectTypes = [.qr]
        let layer = AVCaptureVideoPreviewLayer(session: session)
        layer.videoGravity = .resizeAspectFill
        layer.frame = view.bounds
        view.layer.addSublayer(layer)
        preview = layer
    }

    private func message(_ text: String) {
        let label = UILabel()
        label.text = text
        label.textColor = .white
        label.font = .preferredFont(forTextStyle: .footnote)
        label.textAlignment = .center
        label.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(label)
        NSLayoutConstraint.activate([
            label.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            label.centerYAnchor.constraint(equalTo: view.centerYAnchor),
        ])
    }

    func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput metadataObjects: [AVMetadataObject], from connection: AVCaptureConnection) {
        guard !handled,
              let object = metadataObjects.first as? AVMetadataMachineReadableCodeObject,
              let code = object.stringValue else { return }
        handled = true
        if session.isRunning { session.stopRunning() }
        onCode(code)
    }
}
