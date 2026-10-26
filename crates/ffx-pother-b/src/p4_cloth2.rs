//! The cloth mesh's stream and draw tables (`0x006e17f0`), built from the
//! source mesh instance `ClothInstancing_attach` hands it.
//!
//! The cloth mesh (`ClothInstancingDynamicMesh`) keeps two `PSharray`s
//! (count word, bit 31 "not owned"; then the storage): its streams at
//! `+8`/`+0xc` (12 bytes: offset, semantic, format byte, `0xff`; bytes 10
//! and 11 never written) and its draw entries at `+0x10`/`+0x14` (16
//! bytes: first stream, stream count, vertex count, a word), with the
//! cloth's stream semantic at `+0x20` and its format at `+0x24`.

use ffx_graphic::host::{Conv, Host};

use crate::PHYRE_FREE;
use crate::p4_mesh::owns;

/// `Phyre_PhyreMalloc_0042fb80` (cdecl `(size, align)`).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `Phyre_PGeometry_FormatSize_0055bc40` (cdecl `(format)`).
pub const FORMAT_SIZE: u32 = 0x0055_bc40;

const COUNT: u32 = 0x7fff_ffff;

pub mod bridge {
    //! The originals over game memory.

    use super::{COUNT, Conv, FORMAT_SIZE, Host, PHYRE_FREE, PHYRE_MALLOC, owns};

    /// Appends one `elem`-byte element to the `PSharray` at `arr` (count
    /// word, then storage), as the original's inline growth does: new
    /// storage of `(n + 1) * elem` bytes (`n` the count without bit 31;
    /// nothing changes when it fails), the old elements copied by `copy`
    /// (a destination is skipped, and the source not advanced, when it is
    /// null), the new one written by `new` at its end, the old storage
    /// freed when it is not the new one, owned and non-null; the count
    /// becomes `n + 1`, owned.
    fn push<H: Host>(
        h: &mut H,
        arr: u32,
        elem: u32,
        copy: fn(&mut H, u32, u32),
        new: impl FnOnce(&mut H, u32),
    ) {
        let n = h.read::<u32>(arr) & COUNT;
        let p = h.call_sym(
            PHYRE_MALLOC,
            Conv::Cdecl,
            &[n.wrapping_add(1).wrapping_mul(elem), 4],
        );
        if p == 0 {
            return;
        }
        let n = h.read::<u32>(arr) & COUNT;
        let mut from: u32 = h.read(arr + 4);
        for i in 0..n {
            let to = p.wrapping_add(i.wrapping_mul(elem));
            if to != 0 {
                copy(h, to, from);
                from = from.wrapping_add(elem);
            }
        }
        let last = p.wrapping_add(n.wrapping_mul(elem));
        if last != 0 {
            new(h, last);
        }
        let old: u32 = h.read(arr + 4);
        if old != p && owns(h.read(arr)) && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(arr + 4, p);
        h.write(arr, n.wrapping_add(1));
    }

    /// Copies a draw entry (four words).
    fn copy16<H: Host>(h: &mut H, to: u32, from: u32) {
        for k in 0..4 {
            let w: u32 = h.read(from.wrapping_add(4 * k));
            h.write(to.wrapping_add(4 * k), w);
        }
    }

    /// Copies a stream entry: two words and the bytes 8 and 9.
    fn copy12<H: Host>(h: &mut H, to: u32, from: u32) {
        for k in 0..2 {
            let w: u32 = h.read(from.wrapping_add(4 * k));
            h.write(to.wrapping_add(4 * k), w);
        }
        for k in 8..10 {
            let b: u8 = h.read(from.wrapping_add(k));
            h.write(to.wrapping_add(k), b);
        }
    }

    /// Empties the `PSharray` at `arr` (keeping bit 31); an owned one
    /// frees its storage and clears the pointer.
    fn clear(h: &mut impl Host, arr: u32) {
        let w = h.read::<u32>(arr) & 0x8000_0000;
        h.write(arr, w);
        // owned and empty: the original tests bit 31 and the word apart
        if w == 0 {
            let data: u32 = h.read(arr + 4);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            h.write(arr + 4, 0u32);
        }
    }

    /// `ClothInstancingDynamicMesh::buildStreams(mesh, src)`
    /// (`0x006e17f0`, thiscall, `ret 4`, nothing in `eax` callers read):
    /// empties the draw entries and the streams, then per segment `c` of
    /// the source's mesh (`[src]`: count, segments of 0x6c bytes at `+4`)
    /// adds a draw entry: with the source's dynamic-mesh instance
    /// (`src + 0x18`) a copy of its mesh's (`inst + 4`) entry `c`, the new
    /// entry's first stream the stream count, and that entry's streams
    /// copied after it; without one, a zero entry. A segment whose words
    /// `+0x20` and `+0x24` are not -1 then walks its vertex sets (`+0x28`
    /// count, 0x40-byte sets at `+0x2c`): each sets the entry's vertex
    /// count (`+8`) from the set's `+4`, and each of the set's data blocks
    /// (`+8` count, 12-byte blocks at `+0xc`) of the cloth's semantic
    /// (`+0x20`) adds a stream (offset: the sum of the earlier ones'
    /// `FormatSize(format)` in this set, the semantic, the format byte,
    /// `0xff`) and counts it in the entry (whose first stream is set when
    /// it had none).
    pub fn build_streams(h: &mut impl Host, mesh: u32, src: u32) {
        let inst: u32 = h.read(src + 0x18);
        let source: u32 = h.read(src);
        let segments = h.read::<u32>(source) & COUNT;
        clear(h, mesh + 0x10);
        clear(h, mesh + 8);
        for c in 0..segments {
            if inst == 0 {
                push(h, mesh + 0x10, 16, copy16, |h, at| {
                    for k in 0..4 {
                        h.write(at + 4 * k, 0u32);
                    }
                });
            } else {
                let dm: u32 = h.read(inst + 4);
                let table: u32 = h.read(dm + 0x14);
                let entry = table.wrapping_add(c.wrapping_mul(16));
                let n: u32 = h.read(entry.wrapping_add(4));
                push(h, mesh + 0x10, 16, copy16, |h, at| copy16(h, at, entry));
                // the new entry's first stream (written even when the
                // growth failed: the last entry there is)
                let count = h.read::<u32>(mesh + 0x10) & COUNT;
                let data: u32 = h.read(mesh + 0x14);
                let streams = h.read::<u32>(mesh + 8) & COUNT;
                h.write(
                    data.wrapping_add(count.wrapping_mul(16)).wrapping_sub(16),
                    streams,
                );
                for j in 0..n {
                    let first: u32 =
                        h.read(h.read::<u32>(dm + 0x14).wrapping_add(c.wrapping_mul(16)));
                    let base: u32 = h.read(dm + 0xc);
                    let from = base.wrapping_add(first.wrapping_add(j).wrapping_mul(12));
                    push(h, mesh + 8, 12, copy12, |h, at| copy12(h, at, from));
                }
            }
            let seg = h.read::<u32>(source + 4).wrapping_add(c.wrapping_mul(0x6c));
            let sets = h.read::<u32>(seg + 0x28) & COUNT;
            if h.read::<u32>(seg + 0x20) == u32::MAX
                || h.read::<u32>(seg + 0x24) == u32::MAX
                || sets == 0
            {
                continue;
            }
            for k in 0..sets {
                let set = h.read::<u32>(seg + 0x2c).wrapping_add(k.wrapping_mul(0x40));
                let blocks = h.read::<u32>(set + 8) & COUNT;
                let mut offset = 0u32;
                let vertices: u32 = h.read(set + 4);
                let draw: u32 = h.read(mesh + 0x14);
                h.write(
                    draw.wrapping_add(c.wrapping_mul(16).wrapping_add(8)),
                    vertices,
                );
                for j in 0..blocks {
                    let block = h.read::<u32>(set + 0xc).wrapping_add(j.wrapping_mul(12));
                    if h.read::<u32>(block + 4) != h.read::<u32>(mesh + 0x20) {
                        continue;
                    }
                    let entry = h.read::<u32>(mesh + 0x14).wrapping_add(c.wrapping_mul(16));
                    if h.read::<u32>(entry + 4) == 0 {
                        let streams = h.read::<u32>(mesh + 8) & COUNT;
                        h.write(entry, streams);
                    }
                    let entry = h.read::<u32>(mesh + 0x14).wrapping_add(c.wrapping_mul(16));
                    let used: u32 = h.read(entry + 4);
                    h.write(entry + 4, used.wrapping_add(1));
                    let format: u8 = h.read(mesh + 0x24);
                    let semantic: u32 = h.read(mesh + 0x20);
                    push(h, mesh + 8, 12, copy12, |h, at| {
                        h.write(at, offset);
                        h.write(at + 4, semantic);
                        h.write(at + 8, format);
                        h.write(at + 9, 0xffu8);
                    });
                    let format: u32 = h.read(mesh + 0x24);
                    let size = h.call_sym(FORMAT_SIZE, Conv::Cdecl, &[format]);
                    offset = offset.wrapping_add(size);
                }
            }
        }
    }
}
