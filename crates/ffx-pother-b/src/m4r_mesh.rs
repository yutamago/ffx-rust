//! M4 leftovers (`m4-rest`): the dynamic meshes' 16-byte draw table
//! resize (`0x00498f30`, list h1-render-a) and the radial lines' mesh
//! setup (`0x006f8c50`, list p4-other-b).
//!
//! Layouts used:
//! - a `PArray` of 16-byte elements: `+0` the count (bit 31: the storage
//!   is not owned), `+4` the storage;
//! - `RadialLine` (`RadialLine_ctor_006f7a30`): `+0x20` the mesh holder
//!   (a `PArray` of 0x6c-byte segment entries), `+0x30` and `+0x34` the
//!   two line counts, `+0x50..+0x64` the vertex and index buffers this
//!   function allocates;
//! - the segment entry (`PMeshSegmentEntry_ctor_004337d0`): `+0x10` the
//!   primitive kind, `+0x28` a `PArray` of 0x40-byte stream blocks,
//!   `+0x30` the index block (`+0` first, `+4` last, `+8` count, `+0xc`
//!   the format byte, `+0x34` the byte size).

/// `Phyre_PhyreMalloc_0042fb80` (cdecl `(size, align)`).
pub const MALLOC: u32 = 0x0042_fb80;
/// `PhyreInitElements16_00497db0` (cdecl `(data, count)`).
pub const INIT16: u32 = 0x0049_7db0;
/// `PMeshSegmentEntries_construct_00434430` (cdecl `(data, n)`).
pub const SEGMENTS_CONSTRUCT: u32 = 0x0043_4430;
/// `PMeshSegmentEntries_destroy_00434210` (cdecl `(data, n)`).
pub const SEGMENTS_DESTROY: u32 = 0x0043_4210;
/// `Phyre_Ctor_0055dc40` (thiscall `(block)`): a 0x40-byte stream block.
pub const BLOCK_CTOR: u32 = 0x0055_dc40;
/// `PArray40_setBuffer_00434e40` (thiscall `(array, data, count)`,
/// `ret 8`).
pub const ARRAY40_SET: u32 = 0x0043_4e40;
/// `operator_new_006304c0` (cdecl `(size)`).
pub const NEW: u32 = 0x0063_04c0;
/// The radial lines' four vertex-stream fillers and the index filler
/// (thiscall `(lines)`, plain `ret`): `0x006f85a0`, `0x006f7e60`,
/// `0x006f83b0`, `0x006f81c0`, then `0x006f8110`.
pub const FILL: [u32; 4] = [0x006f_85a0, 0x006f_7e60, 0x006f_83b0, 0x006f_81c0];
pub const FILL_INDICES: u32 = 0x006f_8110;
/// `Phyre_PGeometry_FormatSize_0055bc40` (cdecl `(format)`).
pub const FORMAT_SIZE: u32 = 0x0055_bc40;
/// `Phyre_PGeometry_PIndexDataBlockNVN_isBound` (thiscall `(block)`) ->
/// `al`.
pub const IS_BOUND: u32 = 0x0056_2db0;
/// `PIndexBufferD3D11_release_005648c0` (thiscall `(block)`).
pub const RELEASE: u32 = 0x0056_48c0;

/// `count * scale` as `operator new[]`'s size: `0xffffffff` when the
/// 64-bit product overflows (`mul`, `seto`, `neg`, `or`).
#[must_use]
pub fn array_size(count: u32, scale: u32) -> u32 {
    let p = u64::from(count) * u64::from(scale);
    if p >> 32 == 0 { p as u32 } else { u32::MAX }
}

pub mod bridge {
    //! The originals over game memory.

    use ffx_graphic::host::{Conv, Host};

    use super::{
        ARRAY40_SET, BLOCK_CTOR, FILL, FILL_INDICES, FORMAT_SIZE, INIT16, IS_BOUND, MALLOC, NEW,
        RELEASE, SEGMENTS_CONSTRUCT, SEGMENTS_DESTROY, array_size,
    };

    const OWNED_MASK: u32 = 0x7fff_ffff;
    /// 1.0f.
    const ONE: u32 = 0x3f80_0000;

    /// `0x00498f30` (thiscall `(array, count)`, `ret 4`) -> 0, or 0xd
    /// when the allocation fails: resizes a `PArray` of 16-byte elements
    /// to `count` zeroed elements. Nothing happens when the count is
    /// already `count`; the old storage is freed when it is owned (bit 31
    /// of the count clear), non-null and not the new one.
    pub fn resize16(h: &mut impl Host, array: u32, count: u32) -> u32 {
        if count == h.read::<u32>(array) {
            return 0;
        }
        let mut data = 0;
        if count != 0 {
            data = h.call_sym(MALLOC, Conv::Cdecl, &[count << 4, 4]);
            if data == 0 {
                return 0xd;
            }
            h.call_sym(INIT16, Conv::Cdecl, &[data, count]);
        }
        let old = h.read::<u32>(array + 4);
        let head = h.read::<u32>(array);
        if old != data && head >> 31 == 0 && old != 0 {
            h.call_sym(crate::PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(array + 4, data);
        h.write(array, count);
        0
    }

    /// The line count times four: the vertices (`([+0x34] + [+0x30]) <<
    /// 2`), re-read where the original re-reads it.
    fn vertices(h: &impl Host, lines: u32) -> u32 {
        h.read::<u32>(lines + 0x34)
            .wrapping_add(h.read::<u32>(lines + 0x30))
            << 2
    }

    /// `RadialLine_createMeshData_006f8c50` (thiscall `(lines)`, plain
    /// `ret`) -> the last callee's `eax` (`FORMAT_SIZE`'s, or `IS_BOUND`'s
    /// / `RELEASE`'s when the index block's size changes): builds the
    /// radial lines' mesh.
    ///
    /// 1. The holder (`[lines + 0x20]`) gets one new segment entry unless
    ///    it has one (`PhyreMalloc(0x6c, 4)`, constructed; an owned old
    ///    array of a different address is destroyed).
    /// 2. The entry: `+0 = 0`, `+0x10 = 1`; its stream array (`+0x28`)
    ///    gets three new 0x40-byte blocks unless it has three.
    /// 3. Five arrays of `n = vertices * 4`-byte rows (`n*12`, `n*16`,
    ///    `n*8`, `n*8`, `n*4` bytes) at `+0x50..+0x60`, and `n/2`
    ///    0x20-byte quads at `+0x64` (`{0.0, 0.0, 1.0 x 6}` each; 0 when
    ///    the allocation fails).
    /// 4. The fillers, the identity index list at `+0x60`, the index
    ///    filler.
    /// 5. The index block (`+0x30`): 0..n-1, format 8; its byte size
    ///    becomes `FormatSize(8) * n`, released first when it is bound.
    pub fn create_mesh_data(h: &mut impl Host, lines: u32) -> u32 {
        // 1. the segment entry
        let holder = h.read::<u32>(lines + 0x20);
        if h.read::<u32>(holder) != 1 {
            let seg = h.call_sym(MALLOC, Conv::Cdecl, &[0x6c, 4]);
            if seg != 0 {
                h.call_sym(SEGMENTS_CONSTRUCT, Conv::Cdecl, &[seg, 1]);
                let head = h.read::<u32>(holder);
                let old = h.read::<u32>(holder + 4);
                if old != seg && head >> 31 == 0 {
                    h.call_sym(SEGMENTS_DESTROY, Conv::Cdecl, &[old, head & OWNED_MASK]);
                }
                h.write(holder + 4, seg);
                h.write(holder, 1_u32);
            }
        }
        // 2. the entry's fields and stream blocks
        let entry = h.read::<u32>(h.read::<u32>(lines + 0x20) + 4);
        h.write(entry, 0_u32);
        h.write(entry + 0x10, 1_u32);
        let streams = entry + 0x28;
        if h.read::<u32>(streams) != 3 {
            let blocks = h.call_sym(MALLOC, Conv::Cdecl, &[0xc0, 4]);
            if blocks != 0 {
                for i in 0..3 {
                    let block = blocks.wrapping_add(i << 6);
                    if block != 0 {
                        h.call_sym(BLOCK_CTOR, Conv::Thiscall(block), &[]);
                    }
                }
                h.call_sym(ARRAY40_SET, Conv::Thiscall(streams), &[blocks, 3]);
            }
        }
        // 3. the vertex arrays; each size from a fresh read of the counts
        for (field, rows) in [(0x50, 12), (0x54, 16), (0x58, 8), (0x5c, 8)] {
            let n = vertices(h, lines);
            let p = h.call_sym(NEW, Conv::Cdecl, &[array_size(n.wrapping_mul(rows / 4), 4)]);
            h.write(lines + field, p);
        }
        let n = vertices(h, lines);
        let p = h.call_sym(NEW, Conv::Cdecl, &[array_size(n, 4)]);
        let quads = vertices(h, lines) >> 1;
        h.write(lines + 0x60, p);
        let q = h.call_sym(NEW, Conv::Cdecl, &[array_size(quads, 0x20)]);
        if q != 0 {
            for k in 0..quads {
                let at = q.wrapping_add(k << 5);
                h.write(at, 0_u32);
                h.write(at + 4, 0_u32);
                for off in [8, 0xc, 0x10, 0x14, 0x18, 0x1c] {
                    h.write(at + off, ONE);
                }
            }
        }
        h.write(lines + 0x64, q);
        // 4. the fillers and the identity index list
        for f in FILL {
            h.call_sym(f, Conv::Thiscall(lines), &[]);
        }
        let mut i = 0;
        while i < vertices(h, lines) {
            let list = h.read::<u32>(lines + 0x60);
            h.write(list.wrapping_add(i << 2), i);
            i += 1;
        }
        h.call_sym(FILL_INDICES, Conv::Thiscall(lines), &[]);
        // 5. the index block
        let n = vertices(h, lines);
        let block = entry + 0x30;
        h.write(block + 8, n);
        h.write(block + 0xc, 8_u8);
        h.write(block, 0_u32);
        h.write(block + 4, n.wrapping_sub(1));
        let size = h.call_sym(FORMAT_SIZE, Conv::Cdecl, &[8]);
        let bytes = size.wrapping_mul(n);
        if h.read::<u32>(block + 0x34) == bytes {
            return size;
        }
        let mut r = h.call_sym(IS_BOUND, Conv::Thiscall(block), &[]);
        if r & 0xff != 0 {
            r = h.call_sym(RELEASE, Conv::Thiscall(block), &[]);
        }
        h.write(block + 0x34, bytes);
        r
    }
}
