import AVFoundation
import SwiftUI
import UIKit

struct LanView: View {
    @EnvironmentObject private var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var scanning = false

    private var isJoining: Bool { model.lan.mode == .joining }

    var body: some View {
        NavigationStack {
            VStack(spacing: 16) {
                if isJoining {
                    Spacer()
                    ProgressView()
                        .controlSize(.large)
                    Text("正在连接…")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                    Spacer()
                } else {
                    if case .failed(let message) = model.lan.mode {
                        Image(systemName: "wifi.exclamationmark")
                            .font(.system(size: 44))
                            .foregroundStyle(.secondary)
                            .frame(width: 240, height: 240)
                        Text(message)
                            .font(.footnote)
                            .multilineTextAlignment(.center)
                            .foregroundStyle(.secondary)
                        Button("重试") { model.lan.host() }
                            .buttonStyle(.bordered)
                            .controlSize(.large)
                    } else if let qr = model.lan.qr {
                        Image(uiImage: qr)
                            .interpolation(.none)
                            .resizable()
                            .scaledToFit()
                            .frame(width: 240, height: 240)
                            .padding(16)
                            .background(Color.white)
                            .clipShape(RoundedRectangle(cornerRadius: 24, style: .continuous))
                            .overlay(
                                RoundedRectangle(cornerRadius: 24, style: .continuous)
                                    .stroke(Color.primary.opacity(0.06), lineWidth: 0.5)
                            )
                            .shadow(color: .black.opacity(0.10), radius: 18, y: 6)
                            .accessibilityLabel("面对面联机二维码")

                        Text("让另一台 iPhone 扫描二维码")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    } else {
                        ProgressView()
                            .frame(width: 240, height: 240)
                        Text("正在准备二维码…")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    }

                    if model.lan.connected {
                        Text("已连接")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                        Button("断开") { model.lan.stop() }
                    } else {
                        Text("或")
                            .font(.caption)
                            .foregroundStyle(.tertiary)

                        Button {
                            scanning = true
                        } label: {
                            Label("扫码加入", systemImage: "camera.viewfinder")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.large)

                        if case .hosting = model.lan.mode {
                            HStack(spacing: 6) {
                                ProgressView().controlSize(.small)
                                Text("等待扫码")
                            }
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        }
                    }
                }
            }
            .padding(24)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .navigationTitle("面对面联机")
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
            .fullScreenCover(isPresented: $scanning) {
                ZStack(alignment: .topLeading) {
                    QrScannerView { code in
                        scanning = false
                        model.lan.join(payload: code)
                    }
                    .ignoresSafeArea()

                    Button {
                        scanning = false
                    } label: {
                        Image(systemName: "xmark")
                            .font(.system(size: 17, weight: .semibold))
                            .foregroundStyle(.white)
                            .frame(width: 44, height: 44)
                            .background(.black.opacity(0.35), in: Circle())
                    }
                    .padding(.leading, 20)
                    .padding(.top, 8)
                    .accessibilityLabel("关闭")
                }
            }
            .onAppear {
                if model.prefs.care_paused {
                    model.setPaused(false)
                }
                // If not connected and not in flight joining, automatically start hosting
                // (including when reopening after a previous disconnect or failure).
                if !model.lan.connected && model.lan.mode != .joining {
                    if model.lan.mode != .hosting || model.lan.qr == nil {
                        model.lan.host()
                    }
                }
            }
            .onChange(of: model.lan.connected) { value in
                if value {
                    model.page = .toy
                    dismiss()
                }
            }
            .onDisappear {
                if !model.lan.connected {
                    model.lan.stop()
                }
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
    private var messageLabel: UILabel?
    private var configured = false
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
        #if targetEnvironment(simulator)
        showMessage("模拟器没有相机\n请用另一台设备扫码")
        return
        #endif
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized:
            configure()
        case .notDetermined:
            AVCaptureDevice.requestAccess(for: .video) { [weak self] granted in
                DispatchQueue.main.async {
                    guard let self else { return }
                    if granted {
                        self.configure()
                    } else {
                        self.showMessage("相机权限未开启")
                    }
                }
            }
        default:
            showMessage("相机权限未开启")
        }
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        if configured { start() }
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        if configured { start() }
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        preview?.frame = view.bounds
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        if session.isRunning {
            DispatchQueue.global(qos: .userInitiated).async { [session] in
                session.stopRunning()
            }
        }
    }

    private func configure() {
        guard !configured else {
            start()
            return
        }
        guard let device = AVCaptureDevice.default(.builtInWideAngleCamera, for: .video, position: .back)
                ?? AVCaptureDevice.default(for: .video),
              let input = try? AVCaptureDeviceInput(device: device) else {
            showMessage("相机不可用")
            return
        }
        session.beginConfiguration()
        guard session.canAddInput(input) else {
            session.commitConfiguration()
            showMessage("相机不可用")
            return
        }
        session.addInput(input)
        let output = AVCaptureMetadataOutput()
        guard session.canAddOutput(output) else {
            session.commitConfiguration()
            showMessage("相机不可用")
            return
        }
        session.addOutput(output)
        output.setMetadataObjectsDelegate(self, queue: .main)
        output.metadataObjectTypes = [.qr]
        session.commitConfiguration()

        let layer = AVCaptureVideoPreviewLayer(session: session)
        layer.videoGravity = .resizeAspectFill
        layer.frame = view.bounds
        view.layer.insertSublayer(layer, at: 0)
        preview = layer
        configured = true
        start()
    }

    private func start() {
        guard configured, !session.inputs.isEmpty, !session.isRunning else { return }
        DispatchQueue.global(qos: .userInitiated).async { [session] in
            session.startRunning()
        }
    }

    private func showMessage(_ text: String) {
        guard messageLabel == nil else { return }
        let label = UILabel()
        label.text = text
        label.numberOfLines = 0
        label.textColor = .white
        label.font = .preferredFont(forTextStyle: .footnote)
        label.textAlignment = .center

        let stack = UIStackView(arrangedSubviews: [label])
        stack.axis = .vertical
        stack.spacing = 12
        stack.alignment = .center
        stack.translatesAutoresizingMaskIntoConstraints = false

        if text.contains("权限") {
            let button = UIButton(type: .system)
            button.setTitle("打开设置", for: .normal)
            button.setTitleColor(.white, for: .normal)
            button.addTarget(self, action: #selector(openSettings), for: .touchUpInside)
            stack.addArrangedSubview(button)
        }

        view.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            stack.centerYAnchor.constraint(equalTo: view.centerYAnchor),
            stack.leadingAnchor.constraint(greaterThanOrEqualTo: view.leadingAnchor, constant: 24),
            stack.trailingAnchor.constraint(lessThanOrEqualTo: view.trailingAnchor, constant: -24),
        ])
        messageLabel = label
    }

    @objc private func openSettings() {
        guard let url = URL(string: UIApplication.openSettingsURLString) else { return }
        UIApplication.shared.open(url)
    }

    func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput metadataObjects: [AVMetadataObject], from connection: AVCaptureConnection) {
        guard !handled,
              let object = metadataObjects.first as? AVMetadataMachineReadableCodeObject,
              let code = object.stringValue else { return }
        handled = true
        DispatchQueue.global(qos: .userInitiated).async { [session] in
            if session.isRunning { session.stopRunning() }
        }
        onCode(code)
    }
}
