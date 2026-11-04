# Porting notes

The tree under this directory is the MIT core **nijiiro-egg** / **tg18-emu**,
Copyright (c) 2026 oreo-fox. The license in `LICENSE` is unchanged. The CPU,
instruction set, and flash interpreter are not rewritten.

## Local time (`core/src/lib.rs`)

`local_time()` on non-Windows targets used `unix_now()`, which is UTC, and
then formatted that number as a civil clock. iOS (and Linux) would therefore
show UTC on the toy’s clock.

The function now calls libc `localtime_r`. If that fails it falls back to the
old UTC breakdown. The Windows `GetLocalTime` path is untouched.

No other core behavior is intentionally changed.
