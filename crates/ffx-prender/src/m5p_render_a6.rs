//! M5 stage 1, `PhyreEngine` list `render-a`, batch 6: the indexed
//! instanced draw of the D3D11 renderer and the animation network
//! optimiser's buffer slots (`PNetworkOptimizeBufferAllocator`: find a
//! slot, grow the slot table).

use ffx_graphic::host::{Conv, Host};

use crate::m5p_render_a::PHYRE_FREE;

/// `PhyreMalloc(size)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb40;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    /// COM method `slot` (word index) of `obj` (stdcall, `obj` first).
    fn com(h: &mut impl Host, obj: u32, slot: u32, args: &[u32]) -> u32 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt + 4 * slot);
        let mut all = vec![obj];
        all.extend_from_slice(args);
        h.call(f, Conv::Stdcall, &all)
    }

    /// `FUN_00592de0(this; topology, count, indices, index, instances)`
    /// (`ret 0x14`) -> `al`: 0 when the topology has no D3D value
    /// (`PRendererD3D11_mapPrimitiveTopology`); else on the context `+0x70`
    /// `IASetPrimitiveTopology`, `IASetIndexBuffer(buffer, R32_UINT (42)
    /// for 8-byte indices else R16_UINT (57), offset +0x2c)` (the buffer
    /// word 0 of element `index` of the 20-byte sharray at `+0x14` when
    /// `+0x30` bit 0, else none; an empty sharray reads `20 index`) and
    /// `DrawIndexedInstanced(count, instances, 0, 0, 0)`; 1.
    pub fn draw_indexed_instanced(
        h: &mut impl Host,
        this: u32,
        topology: u32,
        count: u32,
        indices: u32,
        index: u32,
        instances: u32,
    ) -> u8 {
        let t = cdecl(h, 0x0058_ab60, &[topology]);
        if t == 0 {
            return 0;
        }
        let ctx: u32 = h.read(this + 0x70);
        com(h, ctx, 24, &[t]);
        let ctx: u32 = h.read(this + 0x70);
        let offset: u32 = h.read(indices + 0x2c);
        let format = if h.read::<u8>(indices + 0xc) == 8 {
            0x2a
        } else {
            0x39
        };
        let buffer = if h.read::<u8>(indices + 0x30) & 1 != 0 {
            let n = h.read::<u32>(indices + 0x14) & 0x7fff_ffff;
            let base = if n > 1 {
                h.read(indices + 0x18)
            } else if n != 0 {
                indices + 0x18
            } else {
                0
            };
            h.read(base.wrapping_add(index.wrapping_mul(20)))
        } else {
            0
        };
        com(h, ctx, 19, &[buffer, format, offset]);
        let ctx: u32 = h.read(this + 0x70);
        com(h, ctx, 20, &[count, instances, 0, 0, 0]);
        1
    }

    /// `FUN_00616330(this)` -> `al`: the slot table (8-byte slots `[used
    /// byte, size]` at `+4`, count `+8`) grown to `max(count + 16, 2
    /// count)` slots (`PhyreMalloc`, the size -1 on overflow): new slots
    /// cleared, the old ones copied, the old table freed; 0 without
    /// memory (`eax` 0), else `al` 1 over the free's `eax` (1 without an
    /// old table).
    pub fn grow_slots(h: &mut impl Host, this: u32) -> u32 {
        let count: u32 = h.read(this + 8);
        let doubled = count.wrapping_add(count);
        let n = count.wrapping_add(0x10).max(doubled);
        let size = n.saturating_mul(8);
        let table = cdecl(h, PHYRE_MALLOC, &[size]);
        if table == 0 {
            return 0;
        }
        // the original's countdown loop runs n times unless n - 1 is negative
        let fill = if n.wrapping_sub(1) & 0x8000_0000 != 0 {
            0
        } else {
            n
        };
        for i in 0..fill {
            let s = table.wrapping_add(8 * i);
            h.write(s, 0u8);
            h.write(s + 4, 0u32);
        }
        let mut i = 0;
        while i < h.read::<u32>(this + 8) {
            let old: u32 = h.read(this + 4);
            let a: u32 = h.read(old.wrapping_add(8 * i));
            h.write(table.wrapping_add(8 * i), a);
            let b: u32 = h.read(old.wrapping_add(8 * i + 4));
            h.write(table.wrapping_add(8 * i + 4), b);
            i += 1;
        }
        let old: u32 = h.read(this + 4);
        let eax = if old == 0 {
            0
        } else {
            cdecl(h, PHYRE_FREE, &[old])
        };
        h.write(this + 4, table);
        h.write(this + 8, n);
        (eax & !0xff) | 1
    }

    /// `accept`'s virtual 0 `(index, this)` (thiscall), `al`.
    fn ask(h: &mut impl Host, accept: u32, i: u32, this: u32) -> bool {
        let vt: u32 = h.read(accept);
        let f: u32 = h.read(vt);
        h.call(f, Conv::Thiscall(accept), &[i, this]) & 0xff != 0
    }

    /// `FUN_006161e0(this; size, accept)` (`ret 8`) -> a slot index or
    /// -1: among the unused slots with a buffer, the smallest one of at
    /// least `size`, else the largest one, that `accept` takes (its
    /// virtual 0 `(index, this)`, `al`); else the first unused slot
    /// without a buffer; else the table grown (`FUN_00616330`) and the
    /// search run again. The slot is marked used and its size raised to
    /// `size`.
    pub fn find_slot(h: &mut impl Host, this: u32, size: u32, accept: u32) -> u32 {
        let mut best = 0xffff_ffff_u32;
        let mut best_size = 0u32;
        let mut i = 0;
        while i < h.read::<u32>(this + 8) {
            let slots: u32 = h.read(this + 4);
            let s = slots.wrapping_add(8 * i);
            let sz: u32 = h.read(s + 4);
            if h.read::<u8>(s) == 0
                && sz != 0
                && sz >= size
                && (best == 0xffff_ffff || sz < best_size)
                && ask(h, accept, i, this)
            {
                best = i;
                let slots: u32 = h.read(this + 4);
                best_size = h.read(slots.wrapping_add(8 * i + 4));
            }
            i += 1;
        }
        if best == 0xffff_ffff {
            let mut i = 0;
            while i < h.read::<u32>(this + 8) {
                let slots: u32 = h.read(this + 4);
                let s = slots.wrapping_add(8 * i);
                let sz: u32 = h.read(s + 4);
                if h.read::<u8>(s) == 0
                    && sz != 0
                    && (best == 0xffff_ffff || sz > best_size)
                    && ask(h, accept, i, this)
                {
                    best = i;
                    let slots: u32 = h.read(this + 4);
                    best_size = h.read(slots.wrapping_add(8 * i + 4));
                }
                i += 1;
            }
        }
        if best == 0xffff_ffff {
            let count: u32 = h.read(this + 8);
            let slots: u32 = h.read(this + 4);
            for i in 0..count {
                let s = slots.wrapping_add(8 * i);
                if h.read::<u8>(s) == 0 && h.read::<u32>(s + 4) == 0 {
                    best = i;
                    break;
                }
            }
        }
        if best == 0xffff_ffff {
            if h.call_sym(0x0061_6330, Conv::Thiscall(this), &[]) as u8 == 0 {
                return best;
            }
            return h.call_sym(0x0061_61e0, Conv::Thiscall(this), &[size, accept]);
        }
        let slots: u32 = h.read(this + 4);
        h.write(slots.wrapping_add(8 * best), 1u8);
        let slots: u32 = h.read(this + 4);
        let s = slots.wrapping_add(8 * best);
        let cur: u32 = h.read(s + 4);
        h.write(s + 4, cur.max(size));
        best
    }
}
