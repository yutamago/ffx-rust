//! The sphere grid screen's helpers (`g_Abmap`, the grid `LoadAbmap`
//! reads): zoom scale, node radius, range tests.

/// The map scale of zoom level `level` (`g_Abmap + 0x115cc`): 1, 1/2, 1/4,
/// 1/8; 1 for any other level.
#[must_use]
pub const fn zoom_scale(level: u8) -> f32 {
    match level {
        1 => 0.5,
        2 => 0.25,
        3 => 0.125,
        _ => 1.0,
    }
}

/// The drawn radius of a node whose kind has size `size`: half the size
/// (arithmetic shift) plus 3, rounded to f32 once.
#[must_use]
pub fn node_radius(size: i16) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // the original's fstp dword
    let r = (f64::from(size >> 1) + 3.0) as f32;
    r
}

/// Whether `[a, b]` and `[c, d]` overlap, as `0x00a57000` decides it:
/// when `c` is past `a` it must not be past `b`; when `c` is before `a`,
/// `d` must reach `a`; equal or unordered starts overlap. A NaN end
/// against an ordered start fails.
#[must_use]
pub fn ranges_overlap(a: f32, b: f32, c: f32, d: f32) -> bool {
    if c > a {
        b >= c
    } else if c < a {
        d >= a
    } else {
        true
    }
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `g_Abmap`: the sphere grid data.
    pub const ABMAP: Global<u32> = Global::new(0x0230_5834);

    /// `AbmapZoomScale()` (`0x00a56820`): [`super::zoom_scale`] of the
    /// current zoom level (`g_Abmap + 0x115cc`).
    pub fn zoom_scale(mem: &impl Mem) -> f32 {
        let g = ABMAP.get(mem);
        super::zoom_scale(mem.read(g.wrapping_add(0x115cc)))
    }

    /// `AbmapNodeRadius(node)` (`0x00a56f30`): [`super::node_radius`] of
    /// the size of node `node`'s kind (node `+6`, kind sizes at `+0xf834`,
    /// 0x30 each).
    pub fn node_radius(mem: &impl Mem, node: u32) -> f32 {
        let g = ABMAP.get(mem);
        let kind: u16 = mem.read(g.wrapping_add(node.wrapping_mul(0x28)).wrapping_add(0x80e));
        let size: i16 = mem.read(g.wrapping_add(u32::from(kind) * 0x30 + 0xf834));
        super::node_radius(size)
    }

    /// `AbmapRangesOverlap(a, b, c, d)` (`0x00a57000`): 1 or 0.
    #[must_use]
    pub fn ranges_overlap(a: f32, b: f32, c: f32, d: f32) -> u32 {
        u32::from(super::ranges_overlap(a, b, c, d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap() {
        assert!(ranges_overlap(0.0, 2.0, 1.0, 3.0));
        assert!(!ranges_overlap(0.0, 0.5, 1.0, 3.0));
        assert!(ranges_overlap(1.0, 2.0, 0.0, 1.0));
        assert!(!ranges_overlap(1.0, 2.0, 0.0, 0.5));
        assert!(ranges_overlap(1.0, f32::NAN, 1.0, f32::NAN));
        assert!(ranges_overlap(f32::NAN, 0.0, 1.0, 0.0));
        assert!(!ranges_overlap(0.0, f32::NAN, 1.0, 3.0));
    }

    #[test]
    fn radius() {
        assert_eq!(node_radius(-3), 1.0);
        assert_eq!(node_radius(10), 8.0);
    }
}
