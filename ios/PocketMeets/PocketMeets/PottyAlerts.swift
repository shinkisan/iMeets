import AVFoundation
import Foundation
import UIKit
import UserNotifications

/// Presentation runs on main; the firmware queue supplies an episode token.
/// Foreground delivery never depends on system notification permission or Focus.
final class PottyAlerts: NSObject, UNUserNotificationCenterDelegate {
    static let identifier = "potty-urge"
    private static let episodeKey = "pottyEpisode"
    var isCurrent: ((UInt64) -> Bool)?
    var canPlaySound: ((UInt64) -> Bool)?
    var onInApp: ((String?) -> Void)?
    var onOpen: (() -> Void)?

    private var episode: UInt64?
    private var shownInApp = false
    private let chime = PottyChime()

    func begin(episode: UInt64, title: String, body: String) {
        guard isCurrent?(episode) == true else { return }
        clear()
        self.episode = episode
        if UIApplication.shared.applicationState == .active {
            presentInApp(body: body)
        } else {
            let content = UNMutableNotificationContent()
            content.title = title
            content.body = body
            content.sound = canPlaySound?(episode) == true ? .default : nil
            content.threadIdentifier = Self.identifier
            content.userInfo[Self.episodeKey] = String(episode)
            let request = UNNotificationRequest(identifier: Self.identifier, content: content, trigger: nil)
            UNUserNotificationCenter.current().add(request)
        }
    }

    func clear() {
        episode = nil
        chime.stop()
        if shownInApp { onInApp?(nil) }
        shownInApp = false
        let center = UNUserNotificationCenter.current()
        center.removeDeliveredNotifications(withIdentifiers: [Self.identifier])
        center.removePendingNotificationRequests(withIdentifiers: [Self.identifier])
    }

    func mute() { chime.stop() }

    private func presentInApp(body: String) {
        guard let episode, isCurrent?(episode) == true,
              !shownInApp, UIApplication.shared.applicationState == .active else { return }
        shownInApp = true
        onInApp?(body)
        if canPlaySound?(episode) == true { chime.play() }
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        guard notification.request.identifier == Self.identifier else {
            completionHandler([.banner, .list, .sound])
            return
        }
        // A background request can reach delivery after the app became active.
        // Convert that request to the same in-app route, with no second banner/sound.
        DispatchQueue.main.async { [weak self] in
            guard let self,
                  notification.request.identifier == Self.identifier,
                  let text = notification.request.content.userInfo[Self.episodeKey] as? String,
                  let token = UInt64(text), token == self.episode,
                  self.isCurrent?(token) == true else {
                completionHandler([])
                return
            }
            self.presentInApp(body: notification.request.content.body)
            completionHandler([])
            center.removeDeliveredNotifications(withIdentifiers: [Self.identifier])
        }
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        DispatchQueue.main.async { [weak self] in
            if response.notification.request.identifier == Self.identifier,
               response.actionIdentifier == UNNotificationDefaultActionIdentifier {
                self?.onOpen?()
            }
            completionHandler()
        }
    }
}

/// A short, one-shot foreground reminder. Restore the previous audio category
/// afterwards so normal firmware audio keeps its existing silent-switch behavior.
private final class PottyChime: NSObject, AVAudioPlayerDelegate {
    private struct SessionState {
        let category: AVAudioSession.Category
        let mode: AVAudioSession.Mode
        let options: AVAudioSession.CategoryOptions
    }
    private var previous: SessionState?
    private var player: AVAudioPlayer?
    private var observers: [NSObjectProtocol] = []

    override init() {
        super.init()
        let center = NotificationCenter.default
        for name in [UIApplication.willResignActiveNotification,
                     AVAudioSession.interruptionNotification,
                     AVAudioSession.mediaServicesWereResetNotification] {
            observers.append(center.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in
                self?.stop()
            })
        }
    }

    deinit {
        stop()
        observers.forEach(NotificationCenter.default.removeObserver)
    }

    func play() {
        stop()
        guard UIApplication.shared.applicationState == .active else { return }
        let session = AVAudioSession.sharedInstance()
        do {
            // PiP already uses playback/moviePlayback. Leave its ownership alone.
            if session.category != .playback && session.category != .playAndRecord {
                previous = SessionState(category: session.category, mode: session.mode, options: session.categoryOptions)
                try session.setCategory(.playback, mode: .default, options: [.mixWithOthers])
            }
            try session.setActive(true)
            let player = try AVAudioPlayer(data: Self.wave)
            player.delegate = self
            self.player = player
            guard player.play() else { stop(); return }
        } catch {
            stop() // The visual reminder is still delivered if audio is unavailable.
        }
    }

    func stop() {
        player?.stop()
        player = nil
        guard let previous else { return }
        self.previous = nil
        let session = AVAudioSession.sharedInstance()
        // A PiP start can take over while the chime is playing; do not undo it.
        guard session.category == .playback, session.mode == .default,
              session.categoryOptions == [.mixWithOthers] else { return }
        try? session.setCategory(previous.category, mode: previous.mode, options: previous.options)
        // Keep the firmware's existing active session; never deactivate its audio.
    }

    func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) { stop() }
    func audioPlayerDecodeErrorDidOccur(_ player: AVAudioPlayer, error: Error?) { stop() }

    /// Two soft tones with fades, generated once; no polling or silent-audio loop.
    private static let wave: Data = {
        let rate = 22_050
        let count = Int(Double(rate) * 0.55)
        var pcm = Data(capacity: count * 2)
        for i in 0..<count {
            let t = Double(i) / Double(rate)
            let local = t < 0.25 ? t : t - 0.30
            let duration = 0.22
            var sample: Int16 = 0
            if local >= 0 && local < duration {
                let fade = min(1, min(local / 0.015, (duration - local) / 0.035))
                let frequency = t < 0.25 ? 880.0 : 1174.66
                sample = Int16(sin(2 * .pi * frequency * local) * fade * 0.22 * Double(Int16.max))
            }
            var little = sample.littleEndian
            withUnsafeBytes(of: &little) { pcm.append(contentsOf: $0) }
        }
        var data = Data()
        func word(_ value: UInt32) {
            var v = value.littleEndian
            withUnsafeBytes(of: &v) { data.append(contentsOf: $0) }
        }
        func half(_ value: UInt16) {
            var v = value.littleEndian
            withUnsafeBytes(of: &v) { data.append(contentsOf: $0) }
        }
        data.append(contentsOf: "RIFF".utf8); word(UInt32(pcm.count + 36))
        data.append(contentsOf: "WAVEfmt ".utf8); word(16); half(1); half(1)
        word(UInt32(rate)); word(UInt32(rate * 2)); half(2); half(16)
        data.append(contentsOf: "data".utf8); word(UInt32(pcm.count)); data.append(pcm)
        return data
    }()
}
