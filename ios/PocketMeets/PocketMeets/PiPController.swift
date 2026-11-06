import AVFoundation
import AVKit
import CoreMedia
import SwiftUI

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
    private var timer: Timer?
    private var timebase: CMTimebase?
    private let audio = SilentAudio()
    private let cell = 3
    private let lcd = 128
    private(set) var isFeeding = false

    override init() {
        super.init()
        hostView.isUserInteractionEnabled = false
        hostView.displayLayer.videoGravity = .resizeAspect
        makeTimebase()
        makeController()
    }

    /// Sample-buffer PiP renders on the layer's clock, so give the layer a
    /// running timebase and stamp frames with the host clock.
    private func makeTimebase() {
        var created: CMTimebase?
        CMTimebaseCreateWithSourceClock(
            allocator: kCFAllocatorDefault,
            sourceClock: CMClockGetHostTimeClock(),
            timebaseOut: &created
        )
        guard let created else { return }
        CMTimebaseSetTime(created, time: CMClockGetTime(CMClockGetHostTimeClock()))
        CMTimebaseSetRate(created, rate: 1)
        hostView.displayLayer.controlTimebase = created
        timebase = created
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
        controller.canStartPictureInPictureAutomaticallyFromInline = true
        self.controller = controller
    }

    /// Keeps the layer supplied with the current LCD frame. Runs whenever the
    /// feature is on, so PiP always has a fresh frame to move into.
    func startFeeding() {
        // PiP is unavailable on the Simulator; skip the whole pipeline there.
        guard controller != nil, !isFeeding else { return }
        isFeeding = true
        // Auto-start must already be enabled before the app backgrounds.
        controller?.canStartPictureInPictureAutomaticallyFromInline = true
        render()
        let timer = Timer(timeInterval: 0.1, repeats: true) { [weak self] _ in
            self?.render()
        }
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    /// Resigning active: start the keep-alive audio and let iOS move the layer
    /// into PiP as the app backgrounds.
    func backgrounding() {
        guard controller != nil else { return }
        startFeeding()
        audio.start()
        controller?.canStartPictureInPictureAutomaticallyFromInline = true
        // Starting while the app is still in the foreground half of the
        // transition is the only reliable manual call.
        if controller?.isPictureInPicturePossible == true,
           controller?.isPictureInPictureActive != true {
            controller?.startPictureInPicture()
        }
    }

    func backgrounded() {
        backgrounding()
        guard let controller else { return }
        if controller.isPictureInPicturePossible && !controller.isPictureInPictureActive {
            controller.startPictureInPicture()
        }
    }

    func foregrounded() {
        if controller?.isPictureInPictureActive == true {
            controller?.stopPictureInPicture()
        }
        audio.stop()
    }

    /// The setting was turned off: stop PiP, the feed and the audio session.
    func shutdown() {
        foregrounded()
        controller?.canStartPictureInPictureAutomaticallyFromInline = false
        guard isFeeding else { return }
        isFeeding = false
        timer?.invalidate()
        timer = nil
    }

    private func render() {
        guard let frame = frameProvider?() else { return }
        guard let sample = makeSample(pixels: frame.pixels, asleep: frame.asleep) else { return }
        hostView.displayLayer.enqueue(sample)
    }

    private func makeSample(pixels: [UInt32], asleep: Bool) -> CMSampleBuffer? {
        let side = lcd * cell
        let attributes: [CFString: Any] = [
            kCVPixelBufferIOSurfacePropertiesKey: [:],
            kCVPixelBufferPixelFormatTypeKey: kCVPixelFormatType_32BGRA,
        ]
        var buffer: CVPixelBuffer?
        CVPixelBufferCreate(
            kCFAllocatorDefault, side, side,
            kCVPixelFormatType_32BGRA,
            attributes as CFDictionary,
            &buffer
        )
        guard let buffer else { return nil }

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

        var format: CMVideoFormatDescription?
        CMVideoFormatDescriptionCreateForImageBuffer(
            allocator: kCFAllocatorDefault,
            imageBuffer: buffer,
            formatDescriptionOut: &format
        )
        guard let format else { return nil }

        var timing = CMSampleTimingInfo(
            duration: CMTime(value: 1, timescale: 10),
            presentationTimeStamp: CMClockGetTime(CMClockGetHostTimeClock()),
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
        guard let sample else { return nil }
        if let attachments = CMSampleBufferGetSampleAttachmentsArray(sample, createIfNecessary: true) {
            let dictionary = unsafeBitCast(
                CFArrayGetValueAtIndex(attachments, 0),
                to: CFMutableDictionary.self
            )
            CFDictionarySetValue(
                dictionary,
                Unmanaged.passUnretained(kCMSampleAttachmentKey_DisplayImmediately).toOpaque(),
                Unmanaged.passUnretained(kCFBooleanTrue).toOpaque()
            )
        }
        return sample
    }
}

extension PiPController: AVPictureInPictureControllerDelegate {
    func pictureInPictureControllerDidStartPictureInPicture(_ controller: AVPictureInPictureController) {
    }

    func pictureInPictureControllerDidStopPictureInPicture(_ controller: AVPictureInPictureController) {
    }
}

extension PiPController: AVPictureInPictureSampleBufferPlaybackDelegate {
    func pictureInPictureController(
        _ controller: AVPictureInPictureController,
        setPlaying playing: Bool
    ) {}

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
        try? session.setCategory(.playback, mode: .moviePlayback, options: [.mixWithOthers])
        try? session.setActive(true)

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
