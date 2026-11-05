import AVFoundation
import SwiftUI
import UIKit

struct LanView: View {
    @EnvironmentObject private var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var scanning = false

    var body: some View {
        NavigationStack {
            VStack(spacing: 18) {
                if let qr = model.lan.qr {
                    Image(uiImage: qr)
                        .interpolation(.none)
                        .resizable()
                        .frame(width: 260, height: 260)
                } else {
                    ProgressView()
                        .frame(width: 260, height: 260)
                }

                if model.lan.connected {
                    Text("已连接")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                    Button("断开") { model.lan.stop() }
                } else {
                    Button {
                        scanning = true
                    } label: {
                        Label("扫码加入", systemImage: "camera.viewfinder")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)

                    if case .joining = model.lan.mode {
                        ProgressView()
                    }
                    if case .failed(let message) = model.lan.mode {
                        Text(message)
                            .font(.footnote)
                            .multilineTextAlignment(.center)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            .padding(24)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .navigationTitle("红外联机")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button {
                        model.lan.stop()
                        dismiss()
                    } label: {
                        Image(systemName: "chevron.backward")
                            .font(.system(size: 17, weight: .semibold))
                    }
                    .accessibilityLabel(model.t("back"))
                }
            }
            .sheet(isPresented: $scanning) {
                QrScannerView { code in
                    scanning = false
                    model.lan.join(payload: code)
                }
            }
            .onAppear {
                if !model.lan.connected && model.lan.qr == nil {
                    model.lan.host()
                }
            }
            .onChange(of: model.lan.connected) { value in
                if value {
                    model.page = .toy
                    dismiss()
                }
            }
        }
        .interactiveDismissDisabled()
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
