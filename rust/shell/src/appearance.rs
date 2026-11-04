//! Three appearances, one save. Dot-matrix is a shader, not a framebuffer write.

pub const ASLEEP_RGB: u32 = 0x0029_2B2A;
pub const ASLEEP_ARGB: u32 = 0xFF29_2B2A;
pub const SHELL_BG: u32 = 0x00FC_F8F6;
pub const MINIMAL_BG: u32 = 0x00FF_F3F5;
pub const FLASH_MS: u32 = 420;
pub const FLASH_BAND: f32 = 0.28;
pub const FLASH_FROM: f32 = -0.3;
pub const FLASH_TO: f32 = 1.3;
pub const GAP_V: f32 = 0.21;
pub const GAP_H: f32 = 0.18;
pub const DOT_CELL: f32 = 64.0;
pub const GUIDE_COMPLETE: i32 = 2;
pub const EXIT_HOLD_MS: u32 = 1_000;
pub const SCALE_MIN: i32 = 50;
pub const SCALE_MAX: i32 = 150;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    A = 1,
    B = 2,
    C = 3,
    Shutter = 4,
    Exit = 5,
}

pub fn resolve_appearance(appearance: Option<i32>, minimal_ui: Option<bool>) -> i32 {
    if let Some(a) = appearance {
        return if (0..=2).contains(&a) { a } else { 0 };
    }
    if minimal_ui == Some(true) { 1 } else { 0 }
}

pub fn clamp_scale(v: i32) -> i32 {
    v.clamp(SCALE_MIN, SCALE_MAX)
}

pub fn zone(x: f32, y: f32, w: f32, h: f32, screenshots: bool) -> Option<Zone> {
    if !(w > 0.0 && h > 0.0) || x < 0.0 || y < 0.0 || x >= w || y >= h {
        return None;
    }
    let col = if x < w / 3.0 { 0 } else if x < 2.0 * w / 3.0 { 1 } else { 2 };
    let upper = y < h / 3.0;
    Some(match col {
        0 => Zone::A,
        1 if screenshots && upper => Zone::Shutter,
        1 => Zone::B,
        _ if upper => Zone::Exit,
        _ => Zone::C,
    })
}

pub fn guide_visible(
    appearance: i32,
    ready: bool,
    focused: bool,
    shelf_editing: bool,
    guide_version: i32,
    screenshots: bool,
    capture_seen: bool,
) -> bool {
    if appearance != 2 || !ready || !focused || shelf_editing {
        return false;
    }
    guide_version < GUIDE_COMPLETE || (screenshots && !capture_seen)
}

pub fn exit_allowed(ready: bool, shelf_editing: bool, guide: bool) -> bool {
    ready && !shelf_editing && !guide
}

/// Monotonic fade. The weights and the 64-cell are specified; the exact
/// curve is not. TODO(dot-fade-curve): SPEC §7.5 names the fade but not
/// the breakpoints. Strength is 0 at or below 1 point and 1 at or above 4.
pub fn dot_strength(pixel_points: f32) -> f32 {
    if !pixel_points.is_finite() {
        return 0.0;
    }
    ((pixel_points - 1.0) / 3.0).clamp(0.0, 1.0)
}

pub fn png_name(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32, millis: u32, hex8: &str) -> String {
    let hex8 = if hex8.len() >= 8 { &hex8[..8] } else { "00000000" };
    format!("Open-Meets-{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}-{millis:03}-{hex8}.png")
}

pub fn export_name(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> String {
    format!("OpenMeets-{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}.t18s")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appearance_zones_guide_dots_names() {
        assert_eq!(resolve_appearance(None, None), 0);
        assert_eq!(resolve_appearance(None, Some(true)), 1);
        assert_eq!(resolve_appearance(Some(2), Some(true)), 2);
        assert_eq!(resolve_appearance(Some(9), None), 0);
        assert_eq!(clamp_scale(10), 50);
        assert_eq!(clamp_scale(100), 100);
        assert_eq!(clamp_scale(400), 150);

        let w = 90.0;
        let h = 90.0;
        assert_eq!(zone(1.0, 10.0, w, h, true), Some(Zone::A));
        assert_eq!(zone(45.0, 80.0, w, h, true), Some(Zone::B));
        assert_eq!(zone(45.0, 5.0, w, h, true), Some(Zone::Shutter));
        assert_eq!(zone(45.0, 5.0, w, h, false), Some(Zone::B));
        assert_eq!(zone(80.0, 80.0, w, h, true), Some(Zone::C));
        assert_eq!(zone(80.0, 5.0, w, h, true), Some(Zone::Exit));

        assert!(guide_visible(2, true, true, false, 0, false, false));
        assert!(!guide_visible(2, true, true, false, 2, false, false));
        assert!(guide_visible(2, true, true, false, 2, true, false));
        assert!(!guide_visible(2, true, true, true, 0, false, false));
        assert!(!guide_visible(0, true, true, false, 0, false, false));
        assert!(!exit_allowed(true, false, true));
        assert!(exit_allowed(true, false, false));

        assert_eq!(GAP_V, 0.21);
        assert_eq!(GAP_H, 0.18);
        assert_eq!(DOT_CELL, 64.0);
        assert_eq!(dot_strength(0.5), 0.0);
        assert_eq!(dot_strength(4.0), 1.0);
        assert!(dot_strength(2.0) > dot_strength(1.2));
        assert_eq!(ASLEEP_RGB, 0x292B2A);
        assert_eq!(FLASH_MS, 420);
        assert!((FLASH_BAND - 0.28).abs() < 1e-6);
        assert_eq!(png_name(2026, 10, 31, 1, 2, 3, 4, "ab"), "Open-Meets-20261031-010203-004-00000000.png");
        assert_eq!(png_name(2026, 10, 31, 1, 2, 3, 45, "deadbeef00"), "Open-Meets-20261031-010203-045-deadbeef.png");
        assert_eq!(export_name(2026, 1, 2, 3, 4, 5), "OpenMeets-20260102-030405.t18s");
        assert_eq!(GUIDE_COMPLETE, 2);
        assert_eq!(EXIT_HOLD_MS, 1000);
    }
}
