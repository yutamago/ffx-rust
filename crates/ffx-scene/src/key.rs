//! Content keys: what names a GPU object independently of the run, so a
//! captured draw and an object built from the archive can be matched
//! (`docs/formats/scene.md`, "Content keys").
//!
//! - shaders: FNV-1a 64 of the DXBC container ([`ffx_dxbc::hash`]);
//! - textures: FNV-1a 64 of the top mip level, rows packed (no pitch
//!   padding), in the bytes' own row order;
//! - static vertex and index buffers: FNV-1a 64 of the whole contents.
//!
//! These are the keys of the M4 round-1 plan; `ffx-drawdiff` defines the
//! same scheme for the draw-stream comparison.

/// FNV-1a 64.
#[must_use]
pub fn fnv(bytes: &[u8]) -> u64 {
    ffx_dxbc::hash(bytes)
}

/// The key of a shader's bytecode.
#[must_use]
pub fn shader(bytecode: &[u8]) -> u64 {
    fnv(bytecode)
}

/// The key of a static buffer's contents.
#[must_use]
pub fn buffer(bytes: &[u8]) -> u64 {
    fnv(bytes)
}

/// The key of a texture's top mip level, given tightly packed rows.
#[must_use]
pub fn texture(top_mip: &[u8]) -> u64 {
    fnv(top_mip)
}

/// (bytes per row, rows) of one mip level of a `DXGI_FORMAT`, for the
/// formats the game's scene uses; `None` for others.
#[must_use]
pub fn level_shape(format: u32, width: u32, height: u32) -> Option<(usize, usize)> {
    let blocks = |n: u32| n.div_ceil(4).max(1) as usize;
    let (w, h) = (width.max(1) as usize, height.max(1) as usize);
    Some(match format {
        // BC1
        70..=72 => (blocks(width) * 8, blocks(height)),
        // BC2, BC3
        73..=78 => (blocks(width) * 16, blocks(height)),
        // BC4
        79..=81 => (blocks(width) * 8, blocks(height)),
        // BC5
        82..=84 => (blocks(width) * 16, blocks(height)),
        // R32G32B32A32
        1..=4 => (w * 16, h),
        // R16G16B16A16
        9..=14 => (w * 8, h),
        // R10G10B10A2, R8G8B8A8, R16G16, R32, D24S8, B8G8R8A8/X8
        23..=45 | 87..=93 => (w * 4, h),
        // R8G8, R16, D16
        48..=59 => (w * 2, h),
        // R8, A8
        60..=65 => (w, h),
        _ => return None,
    })
}

/// A captured subresource's rows without the pitch padding.
#[must_use]
pub fn pack_rows(data: &[u8], row_pitch: usize, row_bytes: usize, rows: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(row_bytes * rows);
    for r in 0..rows {
        let at = r * row_pitch;
        match data.get(at..at + row_bytes) {
            Some(row) => out.extend_from_slice(row),
            None => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_matches_the_reference_values() {
        assert_eq!(fnv(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn shapes_and_packing() {
        assert_eq!(level_shape(71, 256, 128), Some((64 * 8, 32)));
        assert_eq!(level_shape(77, 2, 2), Some((16, 1)));
        assert_eq!(level_shape(28, 3, 2), Some((12, 2)));
        assert_eq!(level_shape(999, 1, 1), None);
        let data = [1, 2, 0, 0, 3, 4, 0, 0];
        assert_eq!(pack_rows(&data, 4, 2, 2), vec![1, 2, 3, 4]);
    }
}
