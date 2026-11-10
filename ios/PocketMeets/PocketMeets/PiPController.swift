import AVFoundation
import AVKit
import CoreMedia
import SwiftUI
import UIKit

/// Hosts the layer that feeds Picture in Picture. It also sits behind the app
/// content so iOS sees a layer in the window while the video is "playing".
final class PiPDisplayView: UIView {
    override class var layerClass: AnyClass { AVSampleBufferDisplayLayer.self }
    var displayLayer: AVSampleBufferDisplayLayer { layer as! AVSampleBufferDisplayLayer }
}

struct PiPLayerHost: UIViewRepresentable {
    let view: PiPDisplayView

    func makeUIView(context: Context) -> PiPDisplayView { view }
    func updateUIView(_ uiView: PiPDisplayView, context: Context) {}
}

/// Shows the LCD in a Picture in Picture window while the app is in the
/// background. The frames are rendered from the same pixels as the on-screen
/// LCD, so no separate capture path is needed.
final class PiPController: NSObject {
    let hostView = PiPDisplayView()

    /// Reads the current LCD frame. Called on the main thread.
    var frameProvider: (() -> (pixels: [UInt32], asleep: Bool))?

    private var controller: AVPictureInPictureController?
    private var feed: DispatchSourceTimer?
    private let audio = SilentAudio()
    private let cell = 3
    private let lcd = 128
    private let frameScale: CMTimeScale = 30
    private var timebase: CMTimebase?
    private var lastPts = CMTime.zero
    private var pool: CVPixelBufferPool?
    private var cachedFrame: CVPixelBuffer?
    private var frameDirty = true
    private var inBackground = false
    private var reportedFailure = false
    private(set) var isFeeding = false
    private(set) var returningFromPiP = false
    var onPiPStopped: (() -> Void)?

    /// Whether Picture in Picture is currently active or in the middle of
    /// returning the user to the app.
    var isActiveOrReturning: Bool {
        returningFromPiP || (controller?.isPictureInPictureActive == true)
    }

    override init() {
        super.init()
        hostView.isUserInteractionEnabled = false
        hostView.frame = CGRect(x: 0, y: 0, width: 384, height: 384)
        hostView.displayLayer.videoGravity = .resizeAspect
        hostView.displayLayer.frame = hostView.bounds
        makeTimebase()
        makeController()
    }

    /// A running timebase makes the layer count as live playback, which PiP
    /// needs before the app resigns active.
    private func makeTimebase() {
        var created: CMTimebase?
        CMTimebaseCreateWithSourceClock(
            allocator: kCFAllocatorDefault,
            sourceClock: CMClockGetHostTimeClock(),
            timebaseOut: &created
        )
        guard let created else { return }
        let now = CMClockGetTime(CMClockGetHostTimeClock())
        CMTimebaseSetTime(created, time: now)
        CMTimebaseSetRate(created, rate: 1)
        lastPts = now
        timebase = created
        hostView.displayLayer.controlTimebase = created
    }

    /// Whether iOS is allowed to automatically transition to PiP when the app backgrounds.
    var isAutomaticAllowed: Bool = true {
        didSet {
            controller?.canStartPictureInPictureAutomaticallyFromInline = isAutomaticAllowed
        }
    }

    private func makeController() {
        guard AVPictureInPictureController.isPictureInPictureSupported() else {
            return
        }
        let source = AVPictureInPictureController.ContentSource(
            sampleBufferDisplayLayer: hostView.displayLayer,
            playbackDelegate: self
        )
        let controller = AVPictureInPictureController(contentSource: source)
        controller.delegate = self
        controller.requiresLinearPlayback = true
        controller.canStartPictureInPictureAutomaticallyFromInline = isAutomaticAllowed
        self.controller = controller
    }

    /// Keeps the layer supplied with the current LCD frame. Runs whenever the
    /// feature is on, so PiP always has a fresh frame to move into.
    func startFeeding() {
        // PiP is unavailable on the Simulator; skip the whole pipeline there.
        guard controller != nil, !isFeeding else { return }
        isFeeding = true
        // Keep the audio session active while the feature is on: PiP only
        // reports itself as possible for a playing app.
        audio.start()
        controller?.canStartPictureInPictureAutomaticallyFromInline = isAutomaticAllowed
        render()
        // Heartbeat for a still LCD, and for a renderer that failed after
        // backgrounding. Picture changes are pushed as soon as they happen.
        let feed = DispatchSource.makeTimerSource(queue: .main)
        feed.schedule(deadline: .now() + .milliseconds(100), repeating: .milliseconds(100), leeway: .milliseconds(20))
        feed.setEventHandler { [weak self] in self?.render() }
        feed.resume()
        self.feed = feed
    }

    /// Resigning active: start the keep-alive audio and let iOS move the layer
    /// into PiP as the app backgrounds.
    func backgrounding() {
        guard isAutomaticAllowed, controller != nil else { return }
        inBackground = true
        startFeeding()
        ensureTimebaseRunning()
        controller?.canStartPictureInPictureAutomaticallyFromInline = isAutomaticAllowed
        render()
        // Starting while the app is still in the foreground half of the
        // transition is the only reliable manual call.
        if controller?.isPictureInPicturePossible == true,
           controller?.isPictureInPictureActive != true {
            controller?.startPictureInPicture()
        }
    }

    /// Starts Picture in Picture immediately from foreground.
    func startPiP() {
        guard controller != nil else { return }
        inBackground = true
        startFeeding()
        ensureTimebaseRunning()
        controller?.canStartPictureInPictureAutomaticallyFromInline = true
        render()
        if controller?.isPictureInPicturePossible == true,
           controller?.isPictureInPictureActive != true {
            controller?.startPictureInPicture()
        }
    }

    func backgrounded() {
        guard isAutomaticAllowed else { return }
        backgrounding()
        guard let controller else { return }
        if controller.isPictureInPicturePossible && !controller.isPictureInPictureActive {
            controller.startPictureInPicture()
        }
    }

    @discardableResult
    func foregrounded() -> Bool {
        let wasPiP = isActiveOrReturning
        inBackground = false
        if controller?.isPictureInPictureActive == true {
            controller?.stopPictureInPicture()
        }
        returningFromPiP = false
        return wasPiP
    }

    func stopPiP() {
        if controller?.isPictureInPictureActive == true {
            controller?.stopPictureInPicture()
        }
    }

    /// A new LCD frame arrived. In the background this is what moves the
    /// floating window; the timer only covers a still screen and a stalled layer.
    func noteFrame() {
        frameDirty = true
        guard isFeeding, inBackground else { return }
        render()
    }

    /// The setting was turned off: stop PiP, the feed and the audio session.
    func shutdown() {
        foregrounded()
        audio.stop()
        controller?.canStartPictureInPictureAutomaticallyFromInline = false
        guard isFeeding else { return }
        isFeeding = false
        feed?.cancel()
        feed = nil
        cachedFrame = nil
        frameDirty = true
    }

    private func render() {
        guard isFeeding else { return }
        prepareRenderer()
        guard let frame = frameProvider?() else { return }
        guard let sample = makeSample(pixels: frame.pixels, asleep: frame.asleep) else { return }
        push(sample)
    }

    /// PiP follows the control timebase. `DisplayImmediately` fights that
    /// clock: the system window keeps the first scheduled image and drops
    /// every later buffer. Each frame is one step ahead of the clock instead.
    private func presentationTime() -> CMTime {
        let step = CMTime(value: 1, timescale: frameScale)
        let now = timebase.map { CMTimebaseGetTime($0) } ?? CMClockGetTime(CMClockGetHostTimeClock())
        var pts = CMTimeAdd(now, step)
        if CMTimeCompare(pts, lastPts) <= 0 {
            pts = CMTimeAdd(lastPts, CMTime(value: 1, timescale: 600))
        }
        lastPts = pts
        return pts
    }

    private func ensureTimebaseRunning() {
        guard let timebase else { return }
        if CMTimebaseGetRate(timebase) == 0 {
            CMTimebaseSetRate(timebase, rate: 1)
        }
    }

    private func anchorTimebase() {
        guard let timebase else { return }
        let now = CMClockGetTime(CMClockGetHostTimeClock())
        CMTimebaseSetTime(timebase, time: now)
        CMTimebaseSetRate(timebase, rate: 1)
        lastPts = now
    }

    private func prepareRenderer() {
        if #available(iOS 17.0, *) {
            let renderer = hostView.displayLayer.sampleBufferRenderer
            recover(failed: renderer.status == .failed, needsFlush: renderer.requiresFlushToResumeDecoding, error: renderer.error) {
                renderer.flush()
            }
        } else {
            let layer = hostView.displayLayer
            recover(failed: layer.status == .failed, needsFlush: layer.requiresFlushToResumeDecoding, error: layer.error) {
                layer.flush()
            }
        }
        ensureTimebaseRunning()
    }

    private func push(_ sample: CMSampleBuffer) {
        if #available(iOS 17.0, *) {
            let renderer = hostView.displayLayer.sampleBufferRenderer
            guard renderer.isReadyForMoreMediaData else { return }
            renderer.enqueue(sample)
        } else {
            let layer = hostView.displayLayer
            guard layer.isReadyForMoreMediaData else { return }
            layer.enqueue(sample)
        }
    }

    /// Backgrounding marks the renderer failed until it is flushed. Further
    /// enqueues are ignored, so the PiP window stays on the frame it opened with.
    private func recover(failed: Bool, needsFlush: Bool, error: Error?, flush: () -> Void) {
        guard failed || needsFlush else {
            reportedFailure = false
            return
        }
        if !reportedFailure, let error {
            print("PM-PIP layer failed \(error)")
            reportedFailure = true
        }
        flush()
        anchorTimebase()
    }

    private func makeSample(pixels: [UInt32], asleep: Bool) -> CMSampleBuffer? {
        let side = lcd * cell
        guard let buffer = (!frameDirty ? cachedFrame : nil) ?? pixelBuffer() else { return nil }

        if frameDirty || cachedFrame == nil {
            CVPixelBufferLockBaseAddress(buffer, [])
            if let base = CVPixelBufferGetBaseAddress(buffer) {
                let stride = CVPixelBufferGetBytesPerRow(buffer)
                let bytes = base.assumingMemoryBound(to: UInt8.self)
                if asleep {
                    for y in 0..<side {
                        let row = bytes + y * stride
                        for x in 0..<side {
                            let p = row + x * 4
                            p[0] = 0x2A; p[1] = 0x2B; p[2] = 0x29; p[3] = 0xFF
                        }
                    }
                } else {
                    for y in 0..<lcd {
                        for x in 0..<lcd {
                            let index = y * lcd + x
                            let argb = index < pixels.count ? pixels[index] : 0xFF00_0000
                            let b = UInt8(argb & 0xFF)
                            let g = UInt8((argb >> 8) & 0xFF)
                            let r = UInt8((argb >> 16) & 0xFF)
                            for sy in 0..<cell {
                                let row = bytes + (y * cell + sy) * stride
                                for sx in 0..<cell {
                                    let p = row + (x * cell + sx) * 4
                                    p[0] = b; p[1] = g; p[2] = r; p[3] = 0xFF
                                }
                            }
                        }
                    }
                }
            }
            CVPixelBufferUnlockBaseAddress(buffer, [])
            cachedFrame = buffer
            frameDirty = false
        }

        // Explicit color tags: the PiP renderer is stricter than the inline
        // layer and can show an empty frame without them.
        CVBufferSetAttachment(
            buffer, kCVImageBufferCGColorSpaceKey,
            CGColorSpaceCreateDeviceRGB(), .shouldPropagate
        )
        CVBufferSetAttachment(
            buffer, kCVImageBufferColorPrimariesKey,
            kCVImageBufferColorPrimaries_ITU_R_709_2, .shouldPropagate
        )
        CVBufferSetAttachment(
            buffer, kCVImageBufferTransferFunctionKey,
            kCVImageBufferTransferFunction_ITU_R_709_2, .shouldPropagate
        )

        var format: CMVideoFormatDescription?
        CMVideoFormatDescriptionCreateForImageBuffer(
            allocator: kCFAllocatorDefault,
            imageBuffer: buffer,
            formatDescriptionOut: &format
        )
        guard let format else { return nil }

        var timing = CMSampleTimingInfo(
            duration: CMTime(value: 1, timescale: frameScale),
            presentationTimeStamp: presentationTime(),
            decodeTimeStamp: .invalid
        )
        var sample: CMSampleBuffer?
        CMSampleBufferCreateReadyWithImageBuffer(
            allocator: kCFAllocatorDefault,
            imageBuffer: buffer,
            formatDescription: format,
            sampleTiming: &timing,
            sampleBufferOut: &sample
        )
        return sample
    }

    /// IOSurface-backed buffers are what the PiP process can read. A pool
    /// keeps a buffer the window still holds from being overwritten.
    private func pixelBuffer() -> CVPixelBuffer? {
        let side = lcd * cell
        if pool == nil {
            let attrs: [CFString: Any] = [
                kCVPixelBufferPixelFormatTypeKey: kCVPixelFormatType_32BGRA,
                kCVPixelBufferWidthKey: side,
                kCVPixelBufferHeightKey: side,
                kCVPixelBufferIOSurfacePropertiesKey: [:] as [String: Any],
                kCVPixelBufferMetalCompatibilityKey: true,
                kCVPixelBufferCGImageCompatibilityKey: true,
                kCVPixelBufferCGBitmapContextCompatibilityKey: true,
            ]
            let poolAttrs: [CFString: Any] = [
                kCVPixelBufferPoolMinimumBufferCountKey: 4,
            ]
            var created: CVPixelBufferPool?
            CVPixelBufferPoolCreate(
                kCFAllocatorDefault,
                poolAttrs as CFDictionary,
                attrs as CFDictionary,
                &created
            )
            pool = created
        }
        guard let pool else { return nil }
        var buffer: CVPixelBuffer?
        let status = CVPixelBufferPoolCreatePixelBuffer(kCFAllocatorDefault, pool, &buffer)
        guard status == kCVReturnSuccess else { return nil }
        return buffer
    }
}

extension PiPController: AVPictureInPictureControllerDelegate {
    func pictureInPictureControllerWillStartPictureInPicture(_ controller: AVPictureInPictureController) {
        ensureTimebaseRunning()
        render()
    }

    func pictureInPictureControllerDidStartPictureInPicture(_ controller: AVPictureInPictureController) {
        ensureTimebaseRunning()
        render()
    }

    func pictureInPictureController(
        _ controller: AVPictureInPictureController,
        restoreUserInterfaceForPictureInPictureStopWithCompletionHandler completionHandler: @escaping (Bool) -> Void
    ) {
        returningFromPiP = true
        completionHandler(true)
    }

    func pictureInPictureControllerDidStopPictureInPicture(_ controller: AVPictureInPictureController) {
        onPiPStopped?()
    }
}

extension PiPController: AVPictureInPictureSampleBufferPlaybackDelegate {
    func pictureInPictureController(
        _ controller: AVPictureInPictureController,
        setPlaying playing: Bool
    ) {
        // A rate of 0 leaves the window on the frame it opened with.
        if playing { ensureTimebaseRunning() }
    }

    func pictureInPictureControllerTimeRangeForPlayback(
        _ controller: AVPictureInPictureController
    ) -> CMTimeRange {
        CMTimeRange(start: .negativeInfinity, duration: .positiveInfinity)
    }

    func pictureInPictureControllerIsPlaybackPaused(
        _ controller: AVPictureInPictureController
    ) -> Bool {
        false
    }

    func pictureInPictureController(
        _ controller: AVPictureInPictureController,
        didTransitionToRenderSize newRenderSize: CMVideoDimensions
    ) {}

    func pictureInPictureController(
        _ controller: AVPictureInPictureController,
        skipByInterval skipInterval: CMTime,
        completion completionHandler: @escaping () -> Void
    ) {
        completionHandler()
    }
}

/// A silent loop keeps the audio session alive, which is what tells iOS the
/// app is still producing media while the PiP window is up.
private final class SilentAudio {
    private let engine = AVAudioEngine()
    private let player = AVAudioPlayerNode()
    private var started = false

    func start() {
        guard !started else { return }
        started = true
        let session = AVAudioSession.sharedInstance()
        do {
            try session.setCategory(.playback, mode: .moviePlayback, options: [.mixWithOthers])
            try session.setActive(true)
        } catch {
            print("PM-PIP session failed \(error)")
        }

        guard let format = AVAudioFormat(standardFormatWithSampleRate: 44_100, channels: 1) else {
            return
        }
        engine.attach(player)
        engine.connect(player, to: engine.mainMixerNode, format: format)
        do {
            try engine.start()
        } catch {
            print("PM-PIP audio failed \(error)")
            return
        }
        guard let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 44_100) else {
            return
        }
        buffer.frameLength = 44_100
        player.scheduleBuffer(buffer, at: nil, options: .loops)
        player.play()
    }

    func stop() {
        guard started else { return }
        started = false
        player.stop()
        engine.stop()
        try? AVAudioSession.sharedInstance().setActive(false, options: [.notifyOthersOnDeactivation])
    }
}
