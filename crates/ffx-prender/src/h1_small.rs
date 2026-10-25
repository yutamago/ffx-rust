//! Small `PhyreEngine` render methods (M4 port round 2, h1-render-a):
//! a vector normalisation, the world cursor's list step, the lights
//! context switch's constructor, array and texture setters, and a
//! descriptor callback.

use ffx_bridge::Mem;
use ffx_bridge::x87::Reg;
use ffx_graphic::host::{Conv, Host};

use crate::basic::FREE;

/// `PType_find_004d3780(name)` (cdecl): the context-switch type of that
/// name, or 0.
pub const FIND_SWITCH_TYPE: u32 = 0x004d_3780;
/// The context-switch type list's head (`PType_find_004d3780` walks it).
pub const SWITCH_TYPES: u32 = 0x00ca_4e18;
/// The context-switch types' running index (each type takes 3).
pub const SWITCH_INDEX: u32 = 0x00ca_4e20;
/// `"NUM_LIGHTS"`, the lights switch's name.
pub const NUM_LIGHTS: u32 = 0x00b1_f9c0;
/// `Phyre::PRendering::PContextSwitch::vftable`.
pub const CONTEXT_SWITCH_VTABLE: u32 = 0x00b1_f7ac;
/// `Phyre::PRendering::PContextSwitchLights::vftable`.
pub const CONTEXT_SWITCH_LIGHTS_VTABLE: u32 = 0x00b1_f7c0;
/// The destructor of a 0x40-byte array element (thiscall, no arguments).
pub const ELEMENT40_DTOR: u32 = 0x0055_dfa0;
/// `PTextureCommonBase::GetMipCountForTextureSize(w, h, d)` (cdecl).
pub const MIP_COUNT: u32 = 0x004c_2430;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        CONTEXT_SWITCH_LIGHTS_VTABLE, CONTEXT_SWITCH_VTABLE, Conv, ELEMENT40_DTOR,
        FIND_SWITCH_TYPE, FREE, Host, MIP_COUNT, Mem, NUM_LIGHTS, Reg, SWITCH_INDEX, SWITCH_TYPES,
    };

    /// `0x004525a0(out, v)` (cdecl): `out` = `v` / |`v`| (three floats),
    /// `out` returned. The original keeps its `f32` temporaries in `v`'s
    /// argument slot; the second result is that slot's last value (the
    /// inverse length's bits).
    ///
    /// x87: `c = x²`, `c = c + y²`, `c = c + z²`, `c = sqrt(c)` (`_CIsqrt`),
    /// `c = 1 / c`, each rounded to `f32`; `out[0] = v[0] c`, then `v[1]`
    /// and `v[2]` are read after the store before them.
    pub fn normalize(m: &mut impl Mem, out: u32, v: u32) -> (u32, u32) {
        let x: f32 = m.read(v);
        let y: f32 = m.read(v + 4);
        let z: f32 = m.read(v + 8);
        let (x, y, z) = (Reg::load(x), Reg::load(y), Reg::load(z));
        let sum = (x * x).store();
        let sum = (Reg::load(sum) + y * y).store();
        let sum = (Reg::load(sum) + z * z).store();
        let length = Reg::load(sum).sqrt().store();
        let inverse = Reg::load((Reg::ONE / Reg::load(length)).store());
        let x: f32 = m.read(v);
        m.write(out, (Reg::load(x) * inverse).store());
        let y: f32 = m.read(v + 4);
        m.write(out + 4, inverse.mul_m32(y).store());
        let z: f32 = m.read(v + 8);
        m.write(out + 8, inverse.mul_m32(z).store());
        (out, inverse.store().to_bits())
    }

    /// A list head's next node, or 0 when it points at itself (empty).
    fn first(m: &impl Mem, head: u32) -> u32 {
        let next: u32 = m.read(head);
        if next == head { 0 } else { next }
    }

    /// `PWorldCursor::nextList(cursor)` (ecx) -> `cursor`: the cursor's
    /// node (`+8`) steps to its successor (0 at the end of its list); at
    /// the end, back at the class entry (`+0`, 0x14 bytes each), it moves to
    /// the next entries while the entry's depth (`+0x10`, signed) is greater
    /// than the cursor's (`+4`) and the step lands on an entry again.
    pub fn next_list(m: &mut impl Mem, cursor: u32) -> u32 {
        let node: u32 = m.read(cursor + 8);
        if node == 0 {
            return cursor;
        }
        let next: u32 = m.read(node);
        let mut node = if next == node { 0 } else { next };
        m.write(cursor + 8, node);
        let mut entry: u32 = m.read(cursor);
        if node != entry {
            return cursor;
        }
        loop {
            let depth: i32 = m.read(cursor + 4);
            let entry_depth: i32 = m.read(entry + 0x10);
            let step = if entry_depth > depth {
                first(m, node)
            } else {
                0
            };
            entry = entry.wrapping_add(0x14);
            node = step;
            m.write(cursor + 8, node);
            m.write(cursor, entry);
            if step != entry {
                return cursor;
            }
        }
    }

    /// `PContextSwitchLights::PContextSwitchLights(this)` -> `this`: the
    /// base's fields (name `"NUM_LIGHTS"` at `+4`, next type `+8`, `+0xc`
    /// byte 0, index `+0x10`, `+0x14` byte 1); a type of that name not
    /// registered yet is linked at the head of the type list, taking the
    /// next 3 indices.
    pub fn context_switch_lights(h: &mut impl Host, this: u32) -> u32 {
        let name = h.rebase(NUM_LIGHTS);
        h.write(this + 4, name);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 0u8);
        let vtable = h.rebase(CONTEXT_SWITCH_VTABLE);
        h.write(this, vtable);
        h.write(this + 0x10, 0u32);
        h.write(this + 0x14, 1u8);
        let name: u32 = h.read(this + 4);
        if name != 0 && h.call_sym(FIND_SWITCH_TYPE, Conv::Cdecl, &[name]) == 0 {
            let head = h.rebase(SWITCH_TYPES);
            let index = h.rebase(SWITCH_INDEX);
            let old: u32 = h.read(head);
            h.write(this + 8, old);
            let n: u32 = h.read(index);
            h.write(index, n.wrapping_add(3));
            h.write(head, this);
            h.write(this + 0x10, 3u32);
        }
        let vtable = h.rebase(CONTEXT_SWITCH_LIGHTS_VTABLE);
        h.write(this, vtable);
        this
    }

    /// `0x00434e40(this, data, count)` (thiscall, `ret 8`): an array of
    /// 0x40-byte elements (`+0` count with bit 31 "not owned", `+4` data)
    /// takes `data` and the count word; an owned old buffer, other than
    /// `data`, has its elements destroyed and is freed. `eax` = the count
    /// word.
    pub fn set_buffer40(h: &mut impl Host, this: u32, data: u32, count: u32) -> u32 {
        let word: u32 = h.read(this);
        let old: u32 = h.read(this + 4);
        if old != data && word >> 31 == 0 && old != 0 {
            let n = word & 0x7fff_ffff;
            for i in 0..n {
                h.call_sym(ELEMENT40_DTOR, Conv::Thiscall(old + 0x40 * i), &[]);
            }
            h.call_sym(FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 4, data);
        h.write(this, count);
        count
    }

    /// `0x004d86e0(descriptor, p)` (`ret 4`, `this` unused): frees `p[1]`
    /// and then `p[0]` when bit 0 of each is clear; `al` = 1.
    pub fn free_two(h: &mut impl Host, p: u32) -> u8 {
        for at in [p + 4, p] {
            let block: u32 = h.read(at);
            if block & 1 == 0 {
                h.call_sym(FREE, Conv::Cdecl, &[block]);
            }
        }
        1
    }

    /// `0x004d66c0(p)` (cdecl) -> 0: each of the `p+0x3c` entries from
    /// `p+0x30` (stride `[[p+0x2c]+0x1c]`) gets `+8` = its type's
    /// (`[+4]`) count word `+0x20` marked "not owned" (bit 31).
    pub fn mark_unowned(m: &mut impl Mem, p: u32) -> u32 {
        let mut n: u32 = m.read(p + 0x3c);
        let mut e: u32 = m.read(p + 0x30);
        let layout: u32 = m.read(p + 0x2c);
        let stride: u32 = m.read(layout + 0x1c);
        while n != 0 {
            let ty: u32 = m.read(e + 4);
            let keep: u32 = m.read(e + 0xc);
            let count: u32 = m.read(ty + 0x20);
            n -= 1;
            m.write(e + 0xc, keep);
            m.write(e + 8, count | 0x8000_0000);
            e = e.wrapping_add(stride);
        }
        0
    }

    /// `PTexture2DBase::setDimensions(this, width, height, format, mips)`
    /// (`ret 0x10`): 3 when `mips` levels don't fit (`1 << mips` greater
    /// than the larger side); else 0, after storing the four (`+0x1c`,
    /// `+0x20`, `+0`, `+0xc`) unless all are unchanged, and the mip count
    /// `+0x10` = `GetMipCountForTextureSize(width, height, 0)` when flag
    /// bit 1 of `+0x14` is set, else `mips`.
    pub fn set_dimensions(
        h: &mut impl Host,
        this: u32,
        width: u32,
        height: u32,
        format: u32,
        mips: u32,
    ) -> u32 {
        if mips != 0 && 1u32 << (mips & 31) > width.max(height) {
            return 3;
        }
        let same = width == h.read::<u32>(this + 0x1c)
            && height == h.read::<u32>(this + 0x20)
            && format == h.read::<u32>(this)
            && mips == h.read::<u32>(this + 0xc);
        if same {
            return 0;
        }
        h.write(this, format);
        let flags: u32 = h.read(this + 0x14);
        h.write(this + 0x1c, width);
        h.write(this + 0x20, height);
        h.write(this + 0xc, mips);
        let count = if flags & 2 != 0 {
            h.call_sym(MIP_COUNT, Conv::Cdecl, &[width, height, 0])
        } else {
            mips
        };
        if count != h.read::<u32>(this + 0x10) {
            h.write(this + 0x10, count);
        }
        0
    }
}
