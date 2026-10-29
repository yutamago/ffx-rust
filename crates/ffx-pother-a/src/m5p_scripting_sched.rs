//! `PhyreEngine`'s scripting support outside the script layer (M5 stage 1,
//! list `phyre-scripting`, batch 4): the script scheduler's entry
//! management (add, insert with the least loaded phase, the scripted
//! component's update), `PScriptedComponent`'s copy and script name, the
//! name component lookup, the `PSharray` external buffer setters of the
//! LOD classes, and `fixObjectsAfterLoading` / constructor steps of
//! dynamic descriptors, animation slots, script methods and the depth of
//! field.
//!
//! Several originals hand a callee the address of a word of their own
//! frame or argument area (`0x0054dcd0`, the name table lookup, reads the
//! name through it); the ports take that word's address as `cell` (a
//! `StackFrame` in the shims).

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_scripting_mid::bridge as mid;
use crate::m5p_scripting_small::{PHYRE_FREE, STRING_ASSIGN};

/// `0x0054dcd0` (thiscall on a name table, two words: `&name`, 0): the
/// name's index, negative when absent.
pub const NAME_INDEX: u32 = 0x0054_dcd0;
/// `Phyre_PFreeList_allocateNewBlock(list; size)` (thiscall).
pub const FREELIST_GROW: u32 = 0x0049_c8d0;
/// `0x005d1370` (thiscall, three words): the scheduler's remove.
pub const SCHEDULER_REMOVE: u32 = 0x005d_1370;
/// `0x005d5450` `(L)` (cdecl): the `lua_close` jump.
pub const CLOSE_STATE: u32 = 0x005d_5450;
/// `0x00449c80` (thiscall): a dynamic descriptor's member fixup.
pub const DYNAMIC_FIX_MEMBERS: u32 = 0x0044_9c80;
/// `0x0043ea30` (thiscall on a namespace, one word: the name): the class
/// of that name.
pub const NAMESPACE_FIND: u32 = 0x0043_ea30;
/// `Phyre_PClassDescriptor_getDefaultBuffer(class)` (thiscall).
pub const DEFAULT_BUFFER: u32 = 0x0043_cb80;
/// `0x004494a0` (thiscall, one word): the dynamic descriptor's defaults.
pub const DYNAMIC_SET_DEFAULTS: u32 = 0x0044_94a0;
/// `Phyre_PClassDescriptor_addToNamespace(class)` (thiscall).
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;
/// The dynamic descriptors' serial number counter.
pub const DYNAMIC_SERIAL: u32 = 0x00c9_0758;
/// The animation slot's three controller constructors (thiscall).
pub const SLOT_CTORS: [(u32, u32); 3] =
    [(0xc, 0x0052_25a0), (0x24, 0x0052_31e0), (0x38, 0x0052_5240)];
/// The global list of name component links.
pub const NAME_LIST: u32 = 0x00c0_cd50;
/// `PDepthOfFieldBase::initializeCommonShaders(this; a, b)` (thiscall).
pub const DOF_INIT_SHADERS: u32 = 0x00a1_85a0;
/// `ShaderParams_FindByName(params; name)` (thiscall).
pub const SHADER_PARAM: u32 = 0x0056_cbf0;
/// The shader name the post effects pass.
pub const POSTFX_SHADER: u32 = 0x0194_3da0;
/// The depth of field's parameters: (name, offset).
pub const DOF_PARAMS: [(u32, u32); 3] = [
    (0x00b7_6734, 0xfb4),
    (0x00b7_6748, 0xfb8),
    (0x00b7_6754, 0xfbc),
];

/// The ports over game memory.
pub mod bridge {
    use super::{
        ADD_TO_NAMESPACE, CLOSE_STATE, Conv, DEFAULT_BUFFER, DOF_INIT_SHADERS, DOF_PARAMS,
        DYNAMIC_FIX_MEMBERS, DYNAMIC_SERIAL, DYNAMIC_SET_DEFAULTS, FREELIST_GROW, Host, NAME_INDEX,
        NAME_LIST, NAMESPACE_FIND, PHYRE_FREE, POSTFX_SHADER, SCHEDULER_REMOVE, SHADER_PARAM,
        SLOT_CTORS, STRING_ASSIGN, mid, x87,
    };
    use crate::m5p_scripting_small::bridge as small;

    /// The C string of a `PString` word (bit 0 tags the pointer).
    fn pstr(h: &impl Host, at: u32) -> u32 {
        let w: u32 = h.read(at);
        w.wrapping_sub(w & 1)
    }

    /// `name`'s index in the owner's name table (`owner + 8`), through
    /// `cell` (the word the original hands the lookup).
    fn name_index(h: &mut impl Host, owner: u32, name: u32, cell: u32) -> u32 {
        h.write(cell, name);
        h.call_sym(
            NAME_INDEX,
            Conv::Thiscall(owner.wrapping_add(8)),
            &[cell, 0],
        )
    }

    /// A free scheduler entry from the free list at `this + 4` (grown by
    /// `+0xc` when empty), or `None`.
    fn take_entry(h: &mut impl Host, this: u32) -> Option<u32> {
        let mut e: u32 = h.read(this.wrapping_add(4));
        if e == 0 {
            let size: u32 = h.read(this.wrapping_add(0xc));
            h.call_sym(FREELIST_GROW, Conv::Thiscall(this.wrapping_add(4)), &[size]);
            e = h.read(this.wrapping_add(4));
            if e == 0 {
                return None;
            }
        }
        let next: u32 = h.read(e);
        h.write(this.wrapping_add(4), next);
        Some(e)
    }

    /// `0x005d01d0` (thiscall `(owner, a1, a2, name, a4)`): a scheduler
    /// entry for the owner's named function, put on the list at `+0x20`;
    /// 0x13 when the name is unknown, 0xd when no entry is free, else 0.
    pub fn scheduler_add(h: &mut impl Host, this: u32, a: [u32; 5], cell: u32) -> u32 {
        let idx = name_index(h, a[0], a[3], cell);
        if (idx as i32) < 0 {
            return 0x13;
        }
        let Some(e) = take_entry(h, this) else {
            return 0xd;
        };
        h.write(e.wrapping_add(8), a[1]);
        h.write(e.wrapping_add(0xc), a[2]);
        h.write(e.wrapping_add(0x10), idx);
        h.write(e.wrapping_add(0x14), 0u32);
        h.write(e.wrapping_add(0x18), 0u32);
        h.write(e.wrapping_add(4), a[0]);
        h.write(e.wrapping_add(0x1c), a[4]);
        let head: u32 = h.read(this.wrapping_add(0x20));
        h.write(e, head);
        h.write(this.wrapping_add(0x20), e);
        0
    }

    /// `0x005d0a80` (thiscall `(n)`): for a period `n` in 2..=0x100, the
    /// phase (0..n) whose counters (`+0x84`, every `n`th below the last
    /// whole period) sum least, the first on ties; 0 for other periods.
    pub fn least_phase(h: &impl Host, this: u32, n: u32) -> u32 {
        if n.wrapping_sub(2) > 0xfe {
            return 0;
        }
        let limit = 0x100 - 0x100 % n;
        let mut best = 0u32;
        let mut best_sum = 0u32;
        for phase in 0..n {
            let mut sum = 0u32;
            let mut j = phase;
            while j < limit {
                let c: u16 = h.read(this.wrapping_add(0x84).wrapping_add(j.wrapping_mul(2)));
                sum = sum.wrapping_add(u32::from(c));
                j = j.wrapping_add(n);
            }
            if phase == 0 || sum < best_sum {
                best_sum = sum;
                best = phase;
            }
        }
        best
    }

    /// `0x005d0280` (thiscall `(owner, a1, a2, name, step, a5)`): a
    /// periodic scheduler entry (period `step`, at least 1) for the
    /// owner's named function, at the least loaded phase ([`least_phase`],
    /// its counters bumped by `0x005d0e90`), put on the list at `+0x24`:
    /// 1 without a name, 0x13 when it is unknown, 0xd when no entry is
    /// free, else 0.
    pub fn scheduler_insert(h: &mut impl Host, this: u32, a: [u32; 6], cell: u32) -> u32 {
        if a[3] == 0 {
            return 1;
        }
        let idx = name_index(h, a[0], a[3], cell);
        if (idx as i32) < 0 {
            return 0x13;
        }
        let step = if a[4] > 1 { a[4] } else { 1 };
        let Some(e) = take_entry(h, this) else {
            return 0xd;
        };
        let phase = least_phase(h, this, step);
        small::count_every(h, this, phase, step);
        h.write(e.wrapping_add(4), a[0]);
        h.write(e.wrapping_add(8), a[1]);
        h.write(e.wrapping_add(0x10), idx);
        h.write(e.wrapping_add(0xc), a[2]);
        h.write(e.wrapping_add(0x14), step as u16);
        h.write(e.wrapping_add(0x16), phase as u16);
        h.write(e.wrapping_add(0x1c), a[5]);
        h.write(e.wrapping_add(0x18), 0u32);
        let head: u32 = h.read(this.wrapping_add(0x24));
        h.write(e, head);
        h.write(this.wrapping_add(0x24), e);
        0
    }

    /// `0x00553440` (thiscall `(&name)`): the scripted component's
    /// function name (`+0x10`) set to `name`; with a component (`+0x14`)
    /// a non-empty name must be in its name table (0x13 when not); 0.
    pub fn set_function_name(h: &mut impl Host, this: u32, pname: u32, cell: u32) -> u32 {
        let comp: u32 = h.read(this.wrapping_add(0x14));
        let s = pstr(h, pname);
        if comp != 0 && s != 0 && h.read::<u8>(s) != 0 && (name_index(h, comp, s, cell) as i32) < 0
        {
            return 0x13;
        }
        let own = this.wrapping_add(0x10);
        if own != pname {
            h.call_sym(STRING_ASSIGN, Conv::Thiscall(own), &[s]);
        }
        0
    }

    /// `0x00554b60` (thiscall): the scripted component's run: with a
    /// scheduler (`+0x18`) and component (`+0x14`) and its entry there
    /// ([`mid::has_entry`]), the named function (`+0x24`, when the table
    /// knows it) through the scheduler's slot 4 (a non-zero result
    /// returned); then the entry removed (`0x005d1370`), an owned Lua
    /// state (`+0x2d`, `+0x1c`) closed, `+0x2c` cleared; the remove's
    /// `eax`, or 0.
    pub fn scripted_run(h: &mut impl Host, this: u32, cell: u32) -> u32 {
        let sched: u32 = h.read(this.wrapping_add(0x18));
        let comp: u32 = h.read(this.wrapping_add(0x14));
        if sched == 0 || comp == 0 {
            return 0;
        }
        let name = pstr(h, this.wrapping_add(0x10));
        let key = this.wrapping_add(4);
        if mid::has_entry(h, sched, comp, key, name) == 0 {
            return 0;
        }
        let f = pstr(h, this.wrapping_add(0x24));
        if f != 0 && h.read::<u8>(f) != 0 {
            let comp: u32 = h.read(this.wrapping_add(0x14));
            if (name_index(h, comp, f, cell) as i32) >= 0 {
                let sched: u32 = h.read(this.wrapping_add(0x18));
                let r = h.call_virtual(sched, 4, &[comp, key, f]);
                if r != 0 {
                    return r;
                }
            }
        }
        let name = pstr(h, this.wrapping_add(0x10));
        let sched: u32 = h.read(this.wrapping_add(0x18));
        let comp: u32 = h.read(this.wrapping_add(0x14));
        let r = h.call_sym(SCHEDULER_REMOVE, Conv::Thiscall(sched), &[comp, key, name]);
        if h.read::<u8>(this.wrapping_add(0x2d)) != 0 {
            let l: u32 = h.read(this.wrapping_add(0x1c));
            if l != 0 {
                h.call_sym(CLOSE_STATE, Conv::Cdecl, &[l]);
                h.write(this.wrapping_add(0x1c), 0u32);
                h.write(this.wrapping_add(0x2d), 0u8);
            }
        }
        h.write(this.wrapping_add(0x2c), 0u8);
        r
    }

    /// `PString::Assign` of `src + off` to `this + off` unless they are
    /// the same string.
    fn assign_pstr(h: &mut impl Host, this: u32, src: u32, off: u32) {
        let to = this.wrapping_add(off);
        let from = src.wrapping_add(off);
        if to != from {
            let s = pstr(h, from);
            h.call_sym(STRING_ASSIGN, Conv::Thiscall(to), &[s]);
        }
    }

    /// `0x00553b80` (thiscall `(src)`): `PScriptedComponent`'s copy
    /// assignment: the words `+4..+0xc` (from `src + 4`, address 0 for a
    /// null `src`), `+0x14..+0x1c`, `+0x28`, the bytes `+0x2c..+0x2e`,
    /// and the `PString`s `+0x10`, `+0x20`, `+0x24` (assigned unless the
    /// same); `this`.
    pub fn scripted_assign(h: &mut impl Host, this: u32, src: u32) -> u32 {
        let from = if src == 0 { 0 } else { src.wrapping_add(4) };
        for i in 0..3u32 {
            let w: u32 = h.read(from.wrapping_add(4 * i));
            h.write(this.wrapping_add(4 + 4 * i), w);
        }
        assign_pstr(h, this, src, 0x10);
        for off in [0x14u32, 0x18, 0x1c] {
            let w: u32 = h.read(src.wrapping_add(off));
            h.write(this.wrapping_add(off), w);
        }
        assign_pstr(h, this, src, 0x20);
        assign_pstr(h, this, src, 0x24);
        let w: u32 = h.read(src.wrapping_add(0x28));
        h.write(this.wrapping_add(0x28), w);
        for off in [0x2cu32, 0x2d, 0x2e] {
            let b: u8 = h.read(src.wrapping_add(off));
            h.write(this.wrapping_add(off), b);
        }
        this
    }

    /// `0x0045ac30` `(name)`: the name component (its link at `+0xc` in
    /// the global list `0x00c0cd50`) whose `PString` (`+0x14`) is `name`,
    /// or for a null `name` the first without one; 0 when none.
    pub fn find_name_component(h: &impl Host, name: u32) -> u32 {
        let list = h.rebase(NAME_LIST);
        let mut link: u32 = h.read(list);
        loop {
            if link == list || link == 0 {
                return 0;
            }
            let rec = link.wrapping_sub(0xc);
            if rec == 0 {
                return 0;
            }
            let s = pstr(h, rec.wrapping_add(0x14));
            if name == 0 {
                if s == 0 {
                    return rec;
                }
            } else if s != 0 && crate::str_eq(h, s, name) {
                return rec;
            }
            link = h.read(rec.wrapping_add(0xc));
        }
    }

    /// `0x005a80f0` (`words` 1, `PLODGroup`) / `0x005a8190` (`words` 3,
    /// `PLODLevel`) (thiscall `(buf, n)`): a `PSharray`'s external buffer:
    /// 5 for a negative count; one element is copied inline (count 1);
    /// else the array points at `buf` (count with bit 31). An owned
    /// out-of-line block it no longer uses is freed; 0.
    pub fn sharray_set_external(h: &mut impl Host, this: u32, buf: u32, n: u32, words: u32) -> u32 {
        if (n as i32) < 0 {
            return 5;
        }
        let head: u32 = h.read(this);
        let count = head & 0x7fff_ffff;
        let inline = this.wrapping_add(4);
        let cur = if count > 1 {
            h.read(inline)
        } else if count != 0 {
            inline
        } else {
            0
        };
        let (data, new_head) = if n == 1 {
            for i in 0..words {
                let w: u32 = h.read(buf.wrapping_add(4 * i));
                h.write(inline.wrapping_add(4 * i), w);
            }
            (inline, 1)
        } else {
            (buf, n | 0x8000_0000)
        };
        if cur != data && cur != inline && head & 0x8000_0000 == 0 && cur != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[cur]);
        }
        if data != inline {
            h.write(inline, data);
        }
        h.write(this, new_head);
        0
    }

    /// `0x00448680` `(p)`: each dynamic class descriptor (`+0x30` less
    /// 0x18): its members fixed up, a serial number (`0x00c90758`), its
    /// defaults from the namespace's (`+0x28`) class of the same name
    /// (whose descriptor is then destroyed, slot 7 with 1), the namespace
    /// stored at `+0x3c` and the class added to it; 0.
    pub fn fix_dynamic_classes(h: &mut impl Host, p: u32) -> u32 {
        let mut n: u32 = h.read(p.wrapping_add(0x3c));
        let at: u32 = h.read(p.wrapping_add(0x30));
        let mut c = if at == 0 { 0 } else { at.wrapping_sub(0x18) };
        let class: u32 = h.read(p.wrapping_add(0x2c));
        let ns: u32 = h.read(p.wrapping_add(0x28));
        let stride: u32 = h.read(class.wrapping_add(0x1c));
        let serial_at = h.rebase(DYNAMIC_SERIAL);
        while n != 0 {
            h.call_sym(DYNAMIC_FIX_MEMBERS, Conv::Thiscall(c), &[]);
            let serial: u32 = h.read(serial_at);
            h.write(c.wrapping_add(0x7c), serial);
            let serial: u32 = h.read(serial_at);
            h.write(serial_at, serial.wrapping_add(1));
            let name: u32 = h.read(c.wrapping_add(0x18));
            let old = h.call_sym(NAMESPACE_FIND, Conv::Thiscall(ns), &[name]);
            if old != 0 {
                let d = h.call_sym(DEFAULT_BUFFER, Conv::Thiscall(old), &[]);
                if d != 0 {
                    h.call_sym(DYNAMIC_SET_DEFAULTS, Conv::Thiscall(c), &[d]);
                }
                h.call_virtual(old, 7, &[1]);
            }
            h.write(c.wrapping_add(0x3c), ns);
            h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(c), &[]);
            c = c.wrapping_add(stride);
            n = n.wrapping_sub(1);
        }
        0
    }

    /// `0x00619400` (thiscall): a script method record's destructor: its
    /// two `PString`s (`+0x30`, `+0x2c`) released when owned (bit 0
    /// clear), unlinked (and linked to itself), its data (`+0xc`) freed
    /// when flag bit 0 of `+0x10` is set. `eax`: `PhyreFree`'s, else the
    /// old next link.
    pub fn method_script_dtor(h: &mut impl Host, this: u32) -> u32 {
        for off in [0x30u32, 0x2c] {
            let w: u32 = h.read(this.wrapping_add(off));
            if w & 1 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[w]);
            }
        }
        let prev: u32 = h.read(this);
        let next: u32 = h.read(this.wrapping_add(4));
        h.write(prev.wrapping_add(4), next);
        h.write(next, prev);
        h.write(this, this);
        h.write(this.wrapping_add(4), this);
        if h.read::<u8>(this.wrapping_add(0x10)) & 1 != 0 {
            let data: u32 = h.read(this.wrapping_add(0xc));
            return h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
        next
    }

    /// `0x00541780` `(array, n)`: `n` animation slots (0x4c bytes)
    /// constructed: weights 1.0 at `+4`, `+8`, 0 at `+0`, and the three
    /// controllers (`+0xc`, `+0x24`, `+0x38`).
    pub fn construct_slots(h: &mut impl Host, array: u32, n: u32) {
        for i in 0..n {
            let e = i.wrapping_mul(0x4c).wrapping_add(array);
            if e == 0 {
                continue;
            }
            let one = x87::store(1.0);
            h.write(e.wrapping_add(4), one);
            h.write(e.wrapping_add(8), one);
            h.write(e, 0u32);
            for (off, ctor) in SLOT_CTORS {
                h.call_sym(ctor, Conv::Thiscall(e.wrapping_add(off)), &[]);
            }
        }
    }

    /// `0x00a07f00` `(p)`: the depth of field effects (`+0x30` less 4):
    /// slot 1 `(0, 0)`; with a context, `initializeCommonShaders` and the
    /// three parameters; the first non-zero result, else 0.
    pub fn fix_dof(h: &mut impl Host, p: u32) -> u32 {
        let at: u32 = h.read(p.wrapping_add(0x30));
        let mut e = if at == 0 { 0 } else { at.wrapping_sub(4) };
        let n: u32 = h.read(p.wrapping_add(0x3c));
        let class: u32 = h.read(p.wrapping_add(0x2c));
        let stride: u32 = h.read(class.wrapping_add(0x1c));
        for _ in 0..n {
            let r = h.call_virtual(e, 1, &[0, 0]);
            if r != 0 {
                return r;
            }
            let ctx: u32 = h.read(e.wrapping_add(8));
            if ctx != 0 && h.read::<u32>(ctx) != 0 {
                let first = small::first_of(h, e);
                let shader = h.rebase(POSTFX_SHADER);
                let r = h.call_sym(DOF_INIT_SHADERS, Conv::Thiscall(e), &[shader, first]);
                if r != 0 {
                    return r;
                }
                for (name, off) in DOF_PARAMS {
                    let name = h.rebase(name);
                    let v = h.call_sym(SHADER_PARAM, Conv::Thiscall(first), &[name]);
                    h.write(e.wrapping_add(off), v);
                }
            }
            e = e.wrapping_add(stride);
        }
        0
    }
}
