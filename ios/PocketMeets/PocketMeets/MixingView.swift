import SwiftUI

/// Landscape host. The radio is the fake device, not the phone's Bluetooth.
/// TODO(mixing-api-codes): SPEC §14.10. Command codes are not mapped.
/// TODO(mixing-url-slash): the joined URL is tried first.
/// TODO(mixing-marriage-server): the shell does not validate a marriage result.
struct MixingView: View {
    @EnvironmentObject private var model: AppModel
    @State private var showToy = false
    @State private var confirmBack = false
    @State private var log = ""
    @State private var sceneName = "title"
    @State private var devices = ""

    private let scenes = [
        "title", "register and login", "home", "town", "profile", "friends",
        "discover", "point shop", "item box", "propose", "marriage",
        "marriage date", "mini-games", "achievements", "papa/mama", "settings"
    ]

    var body: some View {
        ZStack(alignment: .topLeading) {
            VStack(alignment: .leading, spacing: 8) {
                Text(model.t("mixing_title")).font(.title2)
                Text("client \(model.engine.mixingVersion())")
                Text(devices).font(.caption2)
                Text(log).font(.caption).lineLimit(4)
                ScrollView(.horizontal) {
                    HStack {
                        ForEach(scenes, id: \.self) { name in
                            Button(name) { openScene(name) }
                        }
                    }
                }
                Text(sceneName).font(.headline)
                if sceneName == "register and login" {
                    Button("login") { login() }
                }
                if sceneName == "propose" || sceneName == "marriage" {
                    Text(model.t("propose_ok"))
                    Text(model.t("propose_ng"))
                }
            }
            .padding()
            VStack {
                HStack {
                    Button(model.t("toy")) { showToy.toggle() }
                    Spacer()
                    Button(model.t("back_to_app")) { back() }
                }
                Spacer()
                if showToy {
                    toy
                }
            }
            .padding()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(red: 0.93, green: 0.95, blue: 0.9))
        .onAppear { bootStub() }
        .confirmationDialog(model.t("back_confirm"), isPresented: mixingConfirm) {
            Button(model.t("back_to_app"), role: .destructive) {
                model.engine.linkCancel()
                model.page = .toy
            }
        }
    }

    private var toy: some View {
        VStack {
            LcdView(
                pixels: model.pixels,
                asleep: model.asleep,
                dotMatrix: false,
                side: 128,
                strength: 0
            )
            HStack {
                toyKey("A", 0)
                toyKey("B", 1)
                toyKey("C", 2)
            }
        }
        .onReceive(Timer.publish(every: 0.1, on: .main, in: .common).autoconnect()) { _ in
            model.pixels = model.engine.frame()
            let state = model.engine.linkState()
            if state == 0 || state == 5 || state > 5 {
                model.page = .toy
            }
        }
    }

    private func toyKey(_ title: String, _ index: Int) -> some View {
        Text(title)
            .frame(width: 36, height: 36)
            .background(Circle().fill(.white))
            .gesture(DragGesture(minimumDistance: 0).onChanged { _ in
                model.engine.key(index, down: true)
            }.onEnded { _ in
                model.engine.key(index, down: false)
            })
    }

    private func bootStub() {
        log = "Success"
        devices = model.engine.fakeDeviceJSON()
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
            log = "Success"
            model.engine.linkConnect()
        }
    }

    private func openScene(_ name: String) {
        sceneName = name
        if name == "settings" {
            log = model.engine.supportURL()
        }
    }

    private func login() {
        let form = model.engine.loginForm()
        let primary = model.engine.mixingURL()
        post(primary, form: form) { ok in
            if ok { return }
            let slashed = primary.replacingOccurrences(of: "tamagotchionapi_en", with: "tamagotchion/api_en")
            post(slashed, form: form) { second in
                if !second { log = model.t("network_error") }
            }
        }
    }

    private func post(_ urlString: String, form: String, done: @escaping (Bool) -> Void) {
        guard let url = URL(string: urlString) else { done(false); return }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/x-www-form-urlencoded", forHTTPHeaderField: "Content-Type")
        request.httpBody = form.data(using: .utf8)
        URLSession.shared.dataTask(with: request) { data, _, error in
            DispatchQueue.main.async {
                if error != nil || data == nil {
                    done(false)
                    return
                }
                log = String(data: data ?? Data(), encoding: .utf8) ?? model.t("network_error")
                done(true)
            }
        }.resume()
    }

    private var mixingConfirm: Binding<Bool> {
        Binding(get: { model.confirmMixingBack || confirmBack }, set: { value in
            confirmBack = value
            model.confirmMixingBack = value
        })
    }

    private func back() {
        let state = model.engine.linkState()
        if state == 0 || state == 5 || state > 5 {
            model.engine.linkCancel()
            model.page = .toy
        } else {
            confirmBack = true
            model.confirmMixingBack = true
        }
    }
}
