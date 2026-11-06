import SwiftUI

@main
struct PocketMeetsApp: App {
    @StateObject private var model = AppModel()
    @StateObject private var toolbar = ToolbarConfig()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(model)
                .environmentObject(toolbar)
                .frame(minWidth: 280, minHeight: 240)
                .onChange(of: scenePhase) { phase in
                    model.scene(phase)
                }
        }
    }
}

struct RootView: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        NavigationStack {
            ZStack {
                switch model.page {
                case .setup: SetupView().transition(.opacity)
                case .notes: NotesView().transition(.opacity)
                case .toy: ToyView().transition(.opacity)
                case .settings: SettingsView().transition(.opacity)
                case .library: LibraryView().transition(.opacity)
                case .help: HelpView().transition(.opacity)
                case .about: AboutView().transition(.opacity)
                case .web: WebMeetsView().transition(.opacity)
                case .mixing: MixingView().transition(.opacity)
                case .shelf: ShelfView().transition(.opacity)
                }
            }
            .animation(.easeInOut(duration: 0.22), value: model.page)
        }
        .tint(Color.pmAccent)
        // The layer must live in the window while it is playing, even though
        // the app's own content covers it.
        .background(
            PiPLayerHost(view: model.pip.hostView)
                .frame(width: 128, height: 128)
                .allowsHitTesting(false)
                .accessibilityHidden(true)
        )
        .gesture(
            DragGesture(minimumDistance: 24, coordinateSpace: .local).onEnded { value in
                guard value.startLocation.x < 28, value.translation.width > 70 else { return }
                model.edgeBack()
            }
        )
        .sheet(isPresented: $model.showLanguage) {
            LanguageView()
        }
        .overlay(alignment: .bottom) {
            if !model.notice.isEmpty {
                Text(model.notice)
                    .font(.subheadline)
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.primary)
                    .padding(.horizontal, 18)
                    .padding(.vertical, 12)
                    .frame(maxWidth: 360)
                    .pmGlassCard(22, fallback: .thickMaterial)
                    .padding(.horizontal, 24)
                    .padding(.bottom, 20)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
                    .onTapGesture { model.notice = "" }
            }
        }
        .animation(.spring(response: 0.32, dampingFraction: 0.86), value: model.notice)
    }

}
