//! Content keys: `ffx-drawdiff`'s scheme (`docs/drawdiff.md`, "Keys"),
//! plus what the archive side needs to make the same keys: the
//! descriptions the game gives the objects it makes from clusters
//! (`docs/formats/scene.md`, "Asset parity").

pub use ffx_drawdiff::key::{Fnv, Key, buffer, row_bytes, shader, texture};

/// `D3D11_USAGE_IMMUTABLE`: every static vertex and index buffer.
pub const USAGE_IMMUTABLE: u32 = 1;
/// `D3D11_BIND_VERTEX_BUFFER | D3D11_BIND_SHADER_RESOURCE`.
pub const BIND_VERTICES: u32 = 0x9;
/// `D3D11_BIND_INDEX_BUFFER | D3D11_BIND_SHADER_RESOURCE`.
pub const BIND_INDICES: u32 = 0xa;

/// The description the game gives a model's vertex buffer (one per
/// `PDataBlock`) or index buffer (one per segment) of `len` bytes.
#[must_use]
pub fn static_buffer_desc(len: usize, indices: bool) -> ffx_gfx::BufferDesc {
    let bind = if indices { BIND_INDICES } else { BIND_VERTICES };
    [
        u32::try_from(len).unwrap_or(u32::MAX),
        USAGE_IMMUTABLE,
        bind,
        0,
        0,
        0,
    ]
}

/// The key of a static buffer the game makes from `bytes`.
#[must_use]
pub fn static_buffer(bytes: &[u8], indices: bool) -> Key {
    buffer(&static_buffer_desc(bytes.len(), indices), bytes)
}

/// The `DXGI_FORMAT` of a texture cluster's format.
#[must_use]
pub fn dxgi_format(format: ffx_phyre::Format) -> u32 {
    format.dxgi()
}

/// FNV-1a 64 of raw bytes: a texture level's texels alone, to tell which
/// level of a file a captured texture starts at when its description
/// differs.
#[must_use]
pub fn content(bytes: &[u8]) -> Key {
    Fnv::new().bytes(bytes).finish()
}

/// A captured subresource's rows without the pitch padding.
#[must_use]
pub fn pack_rows(data: &[u8], row_pitch: usize, row_bytes: usize) -> Vec<u8> {
    if row_pitch == 0 {
        return data.to_vec();
    }
    data.chunks(row_pitch)
        .flat_map(|r| &r[..row_bytes.min(r.len())])
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_keys_are_drawdiff_keys() {
        let bytes = [1u8, 2, 3, 4];
        assert_eq!(
            static_buffer(&bytes, false),
            buffer(&[4, 1, 9, 0, 0, 0], &bytes)
        );
        assert_ne!(static_buffer(&bytes, false), static_buffer(&bytes, true));
    }

    #[test]
    fn packing() {
        assert_eq!(pack_rows(&[1, 2, 0, 0, 3, 4, 0, 0], 4, 2), vec![1, 2, 3, 4]);
        assert_eq!(content(b""), 0xcbf2_9ce4_8422_2325);
    }
}
