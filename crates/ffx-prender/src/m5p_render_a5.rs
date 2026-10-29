//! M5 stage 1, `PhyreEngine` list `render-a`, batch 5: the sprite
//! collection's frame bookkeeping (add, remove, sizes, the buffer count),
//! the texture atlas records, and the destructors of the D3D11
//! renderer's member blocks (sequences of member destructors and CRT
//! `__ehvec_dtor` calls).

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_render_a::{PHYRE_MALLOC_ALIGNED, f32_move};
use crate::m5p_render_a2::EHVEC_DTOR;
use crate::m5p_render_a3::MEMCPY;

/// One step of a destructor sequence.
#[derive(Debug, Clone, Copy)]
pub enum Step {
    /// `f(this + off)` (thiscall).
    Member(u32, u32),
    /// `__ehvec_dtor(this + off, size, count, dtor)` (stdcall).
    Vector(u32, u32, u32, u32),
}

/// `PZeroQuad`'s destructor.
const QUAD_DTOR: u32 = 0x005b_3640;

/// The destructor sequences `(this)`, by address: each step in order,
/// the last one's `eax` returned.
pub const DTOR_SEQUENCES: &[(u32, &[Step])] = &[
    (
        0x0058_4f70,
        &[
            Step::Member(QUAD_DTOR, 0x48),
            Step::Vector(8, 0x10, 4, QUAD_DTOR),
        ],
    ),
    (
        0x0058_5110,
        &[
            Step::Member(QUAD_DTOR, 0x80),
            Step::Vector(0, 0x10, 8, QUAD_DTOR),
        ],
    ),
    (
        0x0058_4fd0,
        &[
            Step::Member(0x0058_4aa0, 0x1344),
            Step::Member(0x0059_c770, 0x12f8),
            Step::Member(0x004b_e970, 0x12f8),
            Step::Vector(0x718, 0x17c, 8, 0x004b_c000),
            Step::Member(QUAD_DTOR, 0x2a8),
            Step::Vector(0x228, 0x10, 8, QUAD_DTOR),
            Step::Member(QUAD_DTOR, 0x20c),
            Step::Member(QUAD_DTOR, 0x1f0),
            Step::Member(0x0058_5180, 0x1c8),
            Step::Member(0x0058_5180, 0x1a0),
            Step::Member(0x004c_3010, 0x12c),
            Step::Member(0x004c_3010, 0xb8),
            Step::Member(QUAD_DTOR, 0x48),
            Step::Vector(8, 0x10, 4, QUAD_DTOR),
        ],
    ),
];

/// The steps of a [`DTOR_SEQUENCES`] copy (none for another address).
#[must_use]
pub const fn dtor_sequence(addr: u32) -> &'static [Step] {
    let mut i = 0;
    while i < DTOR_SEQUENCES.len() {
        if DTOR_SEQUENCES[i].0 == addr {
            return DTOR_SEQUENCES[i].1;
        }
        i += 1;
    }
    &[]
}

/// The sprite collection's size setters `(this; frame, w, h)` (`ret
/// 0xc`): (address, the offset of the two floats in the frame record).
pub const SPRITE_SIZES: &[(u32, u32)] = &[(0x0056_e850, 0x10), (0x0056_e960, 0x18)];

/// The f32 constants of a new frame record (`FUN_0056c150`).
pub const NEW_FRAME_K1: u32 = 0x00b3_04f8;
pub const NEW_FRAME_K2: u32 = 0x00b3_04fc;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// A [`DTOR_SEQUENCES`] destructor `(this)`.
    pub fn dtor_sequence(h: &mut impl Host, this: u32, steps: &[Step]) -> u32 {
        let mut r = 0;
        for s in steps {
            r = match *s {
                Step::Member(f, off) => this_call(h, f, this + off, &[]),
                Step::Vector(off, size, count, dtor) => {
                    let dtor = h.rebase(dtor);
                    h.call_sym(EHVEC_DTOR, Conv::Stdcall, &[this + off, size, count, dtor])
                }
            };
        }
        r
    }

    /// `FUN_00584aa0(this)`: the red-black tree emptied (`0x00575000(this;
    /// node)` on the first node `+0xc` until it is the sentinel or null),
    /// the free list `+0x18` reset and destroyed (`Phyre_PFreeList_reset`,
    /// `0x0049c5b0`), the two halves destroyed (`0x00574760` twice); the
    /// last `eax`.
    pub fn tree_dtor(h: &mut impl Host, this: u32) -> u32 {
        loop {
            let n: u32 = h.read(this + 0xc);
            if n == this || n == 0 {
                break;
            }
            this_call(h, 0x0057_5000, this, &[n]);
        }
        this_call(h, 0x0049_cc10, this + 0x18, &[]);
        this_call(h, 0x0049_c5b0, this + 0x18, &[]);
        this_call(h, 0x0057_4760, this, &[]);
        this_call(h, 0x0057_4760, this, &[])
    }

    /// `FUN_00565110(dst, src, count)` (cdecl): 20-byte `PSubTextureInfo`
    /// records: the `PString` `+0` duplicated (`PhyreMallocAligned(len +
    /// 1, 2)`, `memcpy`; 0 for none or no memory), the floats `+4..+0x10`
    /// moved; a destination at 0 skipped.
    pub fn subtextures_copy(h: &mut impl Host, dst: u32, src: u32, count: u32) {
        let mut s = src;
        for i in 0..count {
            let d = dst.wrapping_add(20 * i);
            if d == 0 {
                continue;
            }
            let word: u32 = h.read(s);
            let text = word - (word & 1);
            let mut block = 0;
            if text != 0 {
                let mut n = 0u32;
                while h.read::<u8>(text.wrapping_add(n)) != 0 {
                    n += 1;
                }
                block = cdecl(h, PHYRE_MALLOC_ALIGNED, &[n + 1, 2]);
                if block != 0 {
                    cdecl(h, MEMCPY, &[block, text, n + 1]);
                }
            }
            for k in 1..5 {
                let v: u32 = h.read(s + 4 * k);
                h.write(d + 4 * k, f32_move(v));
            }
            h.write(d, block);
            s = s.wrapping_add(20);
        }
    }

    /// `FUN_0056c150(this; out)` (`ret 4`) -> 0, 5 (no room: `+0x20` at
    /// the capacity `+0x1c`, or no records `+0xb8`) or 9: a new frame:
    /// `*out` = the free-list head `+0xc0`, which takes the next free id
    /// (`[+0xc8][id]`); the id maps to the slot `+0x20` (`[+0xc8][id]`) and
    /// back (`[+0xd0][slot] = id`); the count grows, the dirty byte `+6`
    /// is set, and the slot's 48-byte record is reset (0, 0, 0, 1, 0, 0,
    /// 1, 1, K1, K1, K2, 0; K1 and K2 the f32 at `0x00b304f8` and
    /// `0x00b304fc`).
    pub fn add_frame(h: &mut impl Host, this: u32, out: u32) -> u32 {
        let used: u32 = h.read(this + 0x20);
        if used >= h.read::<u32>(this + 0x1c) || h.read::<u32>(this + 0xb8) == 0 {
            return 5;
        }
        let k1 = h.rebase(NEW_FRAME_K1);
        let k1 = f32_move(h.read(k1));
        let k2 = h.rebase(NEW_FRAME_K2);
        let k2 = f32_move(h.read(k2));
        let one = 1.0f32.to_bits();
        let record = [0, 0, 0, one, 0, 0, one, one, k1, k1, k2, 0];
        let head: u32 = h.read(this + 0xc0);
        h.write(out, head);
        let next: u32 = h.read(this + 0xc8);
        let free: u32 = h.read(next.wrapping_add(head.wrapping_mul(4)));
        h.write(this + 0xc0, free);
        let id: u32 = h.read(out);
        let slot: u32 = h.read(this + 0x20);
        h.write(next.wrapping_add(id.wrapping_mul(4)), slot);
        let slot: u32 = h.read(this + 0x20);
        let id: u32 = h.read(out);
        let back: u32 = h.read(this + 0xd0);
        h.write(back.wrapping_add(slot.wrapping_mul(4)), id);
        let records: u32 = h.read(this + 0xb8);
        h.write(this + 0x20, slot.wrapping_add(1));
        h.write(this + 6, 1u8);
        if records == 0 {
            return 9;
        }
        let id: u32 = h.read(out);
        let table: u32 = h.read(this + 0xc8);
        let idx: u32 = h.read(table.wrapping_add(id.wrapping_mul(4)));
        if idx >= h.read::<u32>(this + 0x20) {
            return 5;
        }
        let rec = records.wrapping_add(idx.wrapping_mul(48));
        for (i, v) in (0u32..).zip(record) {
            h.write(rec + 4 * i, v);
        }
        0
    }

    /// `FUN_0056c230(this)` -> the new frame's id, -2 when the free head
    /// `+0xc0` is above 0x7fffffff, -1 when `FUN_0056c150(this; &id)`
    /// fails; `slot` stands for the original's local id (0 first).
    pub fn add_frame_id(h: &mut impl Host, this: u32, slot: u32) -> u32 {
        h.write(slot, 0u32);
        if h.read::<u32>(this + 0xc0) > 0x7fff_ffff {
            return 0xffff_fffe;
        }
        if this_call(h, 0x0056_c150, this, &[slot]) != 0 {
            return 0xffff_ffff;
        }
        h.read(slot)
    }

    /// `FUN_0056df90(this; frame)` (`ret 4`) -> 0, 9 or 5: the frame's
    /// record replaced by the last one, whose id now maps to the freed
    /// slot (`order[last id] += -(distance in records)`, the original's
    /// division by -48), the frame's id pushed on the free list
    /// (`[+0xc8][frame]` = the old head, head `+0xc0` = frame), the back
    /// map fixed and its last entry -1; count `+0x20` down, dirty byte
    /// `+6` set.
    pub fn remove_frame(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let records: u32 = h.read(this + 0xb8);
        if records == 0 {
            return 9;
        }
        let order: u32 = h.read(this + 0xc8);
        let count: u32 = h.read(this + 0x20);
        let idx: u32 = h.read(order.wrapping_add(frame.wrapping_mul(4)));
        if idx >= count {
            return 5;
        }
        let rec = records.wrapping_add(idx.wrapping_mul(48));
        let last = records
            .wrapping_sub(0x30)
            .wrapping_add(count.wrapping_mul(48));
        for i in 0..12 {
            let v: u32 = h.read(last + 4 * i);
            h.write(rec + 4 * i, v);
        }
        let back: u32 = h.read(this + 0xd0);
        let count: u32 = h.read(this + 0x20);
        let moved: u32 = h.read(back.wrapping_add(count.wrapping_mul(4)).wrapping_sub(4));
        let order: u32 = h.read(this + 0xc8);
        let cell = order.wrapping_add(moved.wrapping_mul(4));
        // imul by 0xd5555555, sar 3, plus the sign: the distance / -48
        let d = last.wrapping_sub(rec) as i32;
        let hi = ((i64::from(d) * i64::from(0xd555_5555_u32 as i32)) >> 32) as i32;
        let t = hi >> 3;
        let q = t.wrapping_add(((t as u32) >> 31) as i32);
        let c: u32 = h.read(cell);
        h.write(cell, c.wrapping_add(q as u32));
        let order: u32 = h.read(this + 0xc8);
        let fcell = order.wrapping_add(frame.wrapping_mul(4));
        let head: u32 = h.read(this + 0xc0);
        let old: u32 = h.read(fcell);
        h.write(fcell, head);
        let back: u32 = h.read(this + 0xd0);
        h.write(back.wrapping_add(old.wrapping_mul(4)), moved);
        let count: u32 = h.read(this + 0x20);
        let back: u32 = h.read(this + 0xd0);
        h.write(
            back.wrapping_add(count.wrapping_mul(4)).wrapping_sub(4),
            0xffff_ffff_u32,
        );
        h.write(this + 0x20, count.wrapping_sub(1));
        h.write(this + 0xc0, frame);
        h.write(this + 6, 1u8);
        0
    }

    /// An unsigned word as `fild` + (2^32 when negative) + `fstp dword`.
    fn u32_to_f32(v: u32) -> f64 {
        x87::load(x87::store(f64::from(v)))
    }

    /// A [`SPRITE_SIZES`] setter `(this; frame, w, h)` -> 0, 9 or 5: the
    /// frame record's two floats `w / texture width` and `h / texture
    /// height` (the texture `+0x90`, sizes `+0x1c`/`+0x20`; every value an
    /// unsigned word made f32, the quotients rounded to f32).
    pub fn sprite_size(h: &mut impl Host, this: u32, frame: u32, w: u32, hh: u32, off: u32) -> u32 {
        if h.read::<u32>(this + 0xb8) == 0 {
            return 9;
        }
        let order: u32 = h.read(this + 0xc8);
        let idx: u32 = h.read(order.wrapping_add(frame.wrapping_mul(4)));
        if idx >= h.read::<u32>(this + 0x20) {
            return 5;
        }
        let records: u32 = h.read(this + 0xb8);
        let rec = records.wrapping_add(idx.wrapping_mul(48));
        let tex: u32 = h.read(this + 0x90);
        let tw: u32 = h.read(tex + 0x1c);
        h.write(
            rec + off,
            x87::store(x87::div(u32_to_f32(w), u32_to_f32(tw))),
        );
        let tex: u32 = h.read(this + 0x90);
        let th: u32 = h.read(tex + 0x20);
        h.write(
            rec + off + 4,
            x87::store(x87::div(u32_to_f32(hh), u32_to_f32(th))),
        );
        0
    }

    /// `FUN_0056e410(this; n)` (`ret 4`): the sprite collection's vertex
    /// buffer `+0x50`: a bound one released (`PVertexBufferD3D11_release`,
    /// its non-zero status returned); its count set to `max(n, 1)` (with a
    /// warning 0x00b2f378 while bound; `PBufferD3D11_setCount`'s status,
    /// `+0x10` = 0 on success); an unbound buffer created
    /// (`PVertexBufferD3D11_create`'s status); else 0.
    pub fn set_vertex_count(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let vb: u32 = h.read(this + 0x50);
        if this_call(h, 0x0056_2da0, vb, &[]) & 0xff != 0 {
            let r = this_call(h, 0x0056_46d0, vb, &[]);
            if r != 0 {
                return r;
            }
        }
        let n = n.max(1);
        if n != h.read::<u32>(vb + 0x18) & 0x7fff_ffff {
            if this_call(h, 0x0056_2da0, vb, &[]) & 0xff != 0 {
                let text = h.rebase(0x00b2_f378);
                cdecl(h, 0x0043_52f0, &[1, text]);
            }
            let r = this_call(h, 0x0056_3fb0, vb, &[n]);
            if r != 0 {
                return r;
            }
            h.write(vb + 0x10, 0u32);
        }
        if this_call(h, 0x0056_2da0, vb, &[]) as u8 == 0 {
            let r = this_call(h, 0x0056_1a20, vb, &[]);
            if r != 0 {
                return r;
            }
        }
        0
    }

    /// `FUN_0056c270(this; v)` (`ret 4`): a block of `count + 1` words
    /// (`PhyreMallocAligned`) and `Phyre_PArray_pushInto4(this + 0xc;
    /// block, &v)`, whose `eax` it returns; `slot` stands for the
    /// original's argument slot holding `v`.
    pub fn push_word(h: &mut impl Host, this: u32, v: u32, slot: u32) -> u32 {
        let count = h.read::<u32>(this + 0xc) & 0x7fff_ffff;
        let block = cdecl(h, PHYRE_MALLOC_ALIGNED, &[(count + 1) << 2, 4]);
        h.write(slot, v);
        this_call(h, 0x0054_bc40, this + 0xc, &[block, slot])
    }
}
