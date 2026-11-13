# iMeets

English · [简体中文](README.md)

An iOS shell for **Tamagotchi Meets / On** firmware dumps, currently optimized primarily
for the Japanese Sweets firmware. Built on the MIT-licensed
[nijiiro-egg](https://github.com/oreo-fox/nijiiro-egg) core, with its interface modelled on
[Open Meets](https://github.com/kanatallow/Open-Meets).

iMeets is not affiliated with or endorsed by Bandai. No firmware is provided here.

## Features

- **Runs an 8 MiB Meets / On dump.**
- **Face-to-face multiplayer.** Connect two devices running iMeets over a local network using a QR code or connection string, then use the firmware’s original multiplayer menus. The Japanese Sweets “みーつ → あそぶ” flow has been verified between two physical iPhones. Simulators can connect using a connection string; see the [connection guide](docs/ios-lan.md).
- **Picture in Picture.** Keep an eye on your pet while using other apps. The PiP play/pause control also controls care pausing.
- **Offline catch-up.** Reopening replays the time you were away with a progress card, and the toy's clock ends up matching your phone.
- **Pause care.** For verified Japanese Sweets v062 native characters, resuming replays only the overlap between the paused interval and the most recent sleep period. Awake time and earlier nights are skipped before syncing the phone clock.
- **Care information.** The toolbar info button shows growth time remaining, care mistakes, call deadlines and need timers. Currently limited to the verified Japanese Sweets v062 ROM; incompatible ROMs show a notice.
- **Toilet reminders.** Verified Sweets urges use an in-app message and short tone in the foreground, and system notifications in the background. Sound respects the app’s sound setting.
- **Live translation panel.** Text on the toy's screen appears in Chinese right away; the panel stays put and shows `……` when there is nothing to read.
- **Faster buttons (experimental, off by default).** Button-to-screen response drops from 63 ms to 25 ms, so taps feel immediate.
- **Looks.** Shell and minimal each have a light and a dark skin, and either can follow the system; there is also a small-screen look.
- **Customizable toolbar.** Choose and reorder frequently used buttons; hidden actions remain accessible from More.
- **Key haptics.** Every shell key gives a light tap, and it can be turned off in settings.

## Requirements

- macOS with Xcode (iOS 16+) and Rust with the iOS targets installed
- A device or the simulator
- Your own 8 MiB firmware dump (`.bin`)

## Install

**Use a prebuilt package**

Download the latest `iMeets-*-unsigned.ipa` from
[Releases](https://github.com/shinkisan/iMeets/releases) and sign it with one of:

| Tool | Notes |
|---|---|
| AltStore / Sidekick | Install AltServer on a computer first, then sideload with your Apple ID |
| Sideloadly | Connect the phone to a computer, drop the IPA in, enter your Apple ID |
| TrollStore | Installs directly on supported iOS versions, no signing needed |

**Build from source**

See the Build section below.

**First launch**

Tap "Choose ROM file" and pick your own 8 MiB dump from Files (`.zip` works too). The toy starts as soon as the import finishes.

## Build

Rust core and its tests (Linux or macOS):

```sh
cargo test --manifest-path rust/shell/Cargo.toml
```

Tests take a firmware image from `POCKET_MEETS_ROM` and load it when it is set. The menu
corpus tests need screen captures made locally; the scripts that produce them live in
`tools/`.

iOS:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo build --release --target aarch64-apple-ios \
  --manifest-path rust/shell/Cargo.toml
cargo build --release --target aarch64-apple-ios-sim \
  --manifest-path rust/shell/Cargo.toml
open ios/PocketMeets/PocketMeets.xcodeproj
```

Pick your Development Team in the target settings and run. The Xcode project links the
static library from `target/aarch64-apple-ios{,-sim}/release` and pulls in
`rust/shell/include/pocket_meets.h` through the bridging header.

An Intel simulator needs its own archive:

```sh
cargo build --release --target x86_64-apple-ios \
  --manifest-path rust/shell/Cargo.toml
```

Import the dump on first launch. The app container lives in Application Support and is
excluded from backups.

## Layout

| Path | What it is |
|---|---|
| `third_party/nijiiro-egg` | MIT emulator core (CPU, LCD, GPIO, RTC, SPI flash, sound) |
| `rust/shell` | Session pump, catch-up, key patch, text buffers, settings, FFI (`include/pocket_meets.h`) |
| `ios/PocketMeets` | SwiftUI app |
| `tools` | String extraction, translation table generation, font/menu research, coverage checks |
| `docs` | Infrared protocol notes and local verification |
| `rust/shell/examples` | Headless probes: menu traversal, latency, key scan, clock jump |

## Infrared link

The original two-toy infrared visit (`つうしん → みーつ → あそぶ`) now runs in the
emulator: the firmware itself performs link negotiation, the OBEX `TAMA` exchange and
the goodbye flow. Protocol notes and the local verification harness are in
[docs/infrared.md](docs/infrared.md).

## Related projects

- [Open Meets](https://github.com/kanatallow/Open-Meets) — the sister project on Android; this shell learned its interaction from it.
- [nijiiro-egg](https://github.com/oreo-fox/nijiiro-egg) — the MIT core bundled here.

## Legal

iMeets is an independent project. Tamagotchi, Meets, On and all related marks and
artwork belong to Bandai. This repository does not distribute firmware or its resources.

## License

The emulator core is MIT (Copyright (c) 2026 oreo-fox, see
[third_party/nijiiro-egg/LICENSE](third_party/nijiiro-egg/LICENSE)); the shell is GPL-3.0
(see [LICENSE](LICENSE)).
