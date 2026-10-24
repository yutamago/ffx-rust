//! Stable content keys for the objects a draw uses: the same object made
//! by another run, or by the Rust side, gets the same key, whatever its
//! address (the capture's ids are the game's object addresses).
//!
//! | Object | Key |
//! |---|---|
//! | shader | [`shader`]: FNV-1a 64 of the bytecode ([`ffx_dxbc::hash`], the graphics layer's shader names) |
//! | texture | [`texture`]: its description and the top mip of each slice, row padding left out |
//! | static buffer | [`buffer`]: its description and contents |
//! | dynamic ring | [`ring`]: the bytes the draw uses (never the offset) |
//! | input layout | [`layout`]: its elements |
//! | render or depth target | [`role`]: its order of first use in the frame and its description |
//!
//! States are not keyed: they are compared field by field
//! (`crate::state`).

use ffx_gfx::{Element, Subresource, TextureDesc};

pub type Key = u64;

/// Incremental FNV-1a 64 (the same function as [`ffx_dxbc::hash`]).
#[derive(Debug, Clone, Copy)]
pub struct Fnv(pub u64);

impl Default for Fnv {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    pub fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
        self
    }
    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.bytes(&v.to_le_bytes())
    }
    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.bytes(&v.to_le_bytes())
    }
    pub fn str(&mut self, v: &str) -> &mut Self {
        self.u32(u32::try_from(v.len()).unwrap_or(u32::MAX))
            .bytes(v.as_bytes())
    }
    #[must_use]
    pub fn finish(&self) -> Key {
        self.0
    }
}

/// A shader: FNV-1a 64 of its bytecode.
#[must_use]
pub fn shader(bytecode: &[u8]) -> Key {
    ffx_dxbc::hash(bytecode)
}

/// Bytes per block and block width of a DXGI format (1 for plain
/// formats), when known.
#[must_use]
pub fn format_block(format: u32) -> Option<(u32, u32)> {
    Some(match format {
        1..=4 => (16, 1),
        5..=8 => (12, 1),
        9..=22 => (8, 1),
        23..=47 | 87..=93 => (4, 1),
        48..=59 | 85 | 86 | 115 => (2, 1),
        60..=65 => (1, 1),
        70..=72 | 79..=81 => (8, 4),
        73..=78 | 82..=84 | 94..=99 => (16, 4),
        _ => return None,
    })
}

/// The bytes of one row of a texture level that hold texels (the rest of
/// the row pitch is the readback's padding).
#[must_use]
pub fn row_bytes(format: u32, width: u32) -> Option<usize> {
    let (bytes, block) = format_block(format)?;
    Some((width.div_ceil(block) * bytes) as usize)
}

/// A texture: its width, height, mip count, array size, format and sample
/// count, then the texels of mip 0 of each slice (subresource `slice *
/// mips`), without the row padding.
#[must_use]
pub fn texture(desc: &TextureDesc, subresources: &[Subresource]) -> Key {
    let mut h = Fnv::new();
    for &v in &desc[..6] {
        h.u32(v);
    }
    let mips = desc[2].max(1) as usize;
    let row = row_bytes(desc[4], desc[0]);
    for slice in 0..desc[3].max(1) as usize {
        let Some(s) = subresources.get(slice * mips) else {
            h.u32(u32::MAX);
            continue;
        };
        match row {
            Some(row) if s.row_pitch as usize >= row && s.row_pitch > 0 => {
                for r in s.data.chunks(s.row_pitch as usize) {
                    h.bytes(&r[..row.min(r.len())]);
                }
            }
            _ => {
                h.bytes(&s.data);
            }
        }
    }
    h.finish()
}

/// A static buffer: its width, bind flags and contents.
#[must_use]
pub fn buffer(desc: &ffx_gfx::BufferDesc, data: &[u8]) -> Key {
    Fnv::new().u32(desc[0]).u32(desc[2]).bytes(data).finish()
}

/// The bytes a draw uses from a dynamic ring.
#[must_use]
pub fn ring(bytes: &[u8]) -> Key {
    Fnv::new().u32(0x7269_6e67).bytes(bytes).finish()
}

/// An input layout: its elements in order.
#[must_use]
pub fn layout(elements: &[Element]) -> Key {
    let mut h = Fnv::new();
    for e in elements {
        h.str(&e.semantic)
            .u32(e.index)
            .u32(e.format)
            .u32(e.slot)
            .u32(e.offset)
            .u32(u32::from(e.per_instance))
            .u32(e.step);
    }
    h.finish()
}

/// A render or depth target, by its role in the frame: the `n`th target,
/// counted in order of first use (as render target, depth target, shader
/// input or copy). Its contents change during the frame, so they are not
/// part of the key, and neither is its texture description: the swap
/// chain's buffers are not created through the device, and the layer's
/// table can hold a stale texture at their address (`docs/drawdiff.md`).
/// The view's description (format, dimension) is part of the view's key.
#[must_use]
pub fn role(n: u32) -> Key {
    Fnv::new().u32(0x726f_6c65).u32(n).finish()
}

/// Whether a texture is a render or depth target (`D3D11_BIND_RENDER_TARGET`
/// or `D3D11_BIND_DEPTH_STENCIL`).
#[must_use]
pub fn is_target(desc: &TextureDesc) -> bool {
    desc[8] & (0x20 | 0x40) != 0
}

/// Whether a buffer is a dynamic ring (`D3D11_USAGE_DYNAMIC`).
#[must_use]
pub fn is_ring(desc: &ffx_gfx::BufferDesc) -> bool {
    desc[1] == 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_is_the_shader_hash() {
        assert_eq!(Fnv::new().bytes(b"DXBC1234").finish(), shader(b"DXBC1234"));
    }

    #[test]
    fn row_padding_is_left_out() {
        let desc = [2, 2, 1, 1, 28, 1, 0, 0, 8, 0, 0];
        let tight = Subresource {
            row_pitch: 8,
            data: (0..16).collect(),
        };
        let mut padded = Vec::new();
        padded.extend(0..8u8);
        padded.extend([0xaa; 8]);
        padded.extend(8..16u8);
        padded.extend([0x55; 8]);
        let padded = Subresource {
            row_pitch: 16,
            data: padded,
        };
        assert_eq!(texture(&desc, &[tight]), texture(&desc, &[padded]));
    }

    #[test]
    fn roles_differ_by_order() {
        assert_ne!(role(0), role(1));
    }
}
