import SwiftUI

@main
struct PocketMeetsApp: App {
    @StateObject private var model = AppModel()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(model)
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
            Group {
                switch model.page {
                case .setup: SetupView()
                case .notes: NotesView()
                case .toy: ToyView()
                case .settings: SettingsView()
                case .library: LibraryView()
                case .help: HelpView()
                case .about: AboutView()
                case .web: WebMeetsView()
                case .mixing: MixingView()
                case .shelf: ShelfView()
                }
            }
            .id(model.page)
            .transition(pageTransition)
        }
        .animation(.spring(response: 0.34, dampingFraction: 0.9), value: model.page)
        .tint(Color.pmAccent)
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

    /// A card stack: the new page slides in over the old one, and the old one
    /// shrinks back under it instead of sliding away.
    private var pageTransition: AnyTransition {
        let card = AnyTransition.scale(scale: 0.92, anchor: .leading).combined(with: .opacity)
        let cover = AnyTransition.move(edge: .trailing).combined(with: .opacity)
        return model.navForward
            ? .asymmetric(insertion: cover, removal: card)
            : .asymmetric(insertion: card, removal: cover)
    }
}
