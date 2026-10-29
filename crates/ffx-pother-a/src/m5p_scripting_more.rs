//! `PhyreEngine`'s scripting support outside the script layer (M5 stage 1,
//! list `phyre-scripting`, batch 6): the scheduler's entry removal and
//! period change, the `Vector` constructor, `PhyreGetTypeOf` and the
//! `PhyreObject` equality.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_scripting_lua::{
    ERROR_FILE, ERROR_FUNCTION, ERROR_LINE, LUA_CHECKNUMBER, LUA_ERROR,
};
use crate::m5p_scripting_mid::{LUA_GETTOP, bridge as mid};
use crate::m5p_scripting_small::{
    LUA_GETFIELD, LUA_NEWUSERDATA, LUA_REGISTRYINDEX, LUA_SETMETATABLE, STR_PHYRE_OBJECT,
    STR_VECTOR, bridge as small,
};

/// The scheduler running its update (entries are then only marked).
pub const SCHEDULER_RUNNING: u32 = 0x00cb_d9b8;
/// `Phyre_PFreeList_deallocate(list; p)` (thiscall).
pub const FREELIST_DEALLOCATE: u32 = 0x0049_c980;
/// `lua_pushboolean(L, b)` (cdecl).
pub const LUA_PUSHBOOLEAN: u32 = 0x0094_b5a0;
/// The class descriptor `PhyreGetTypeOf` pushes types with.
pub const TYPE_CLASS: u32 = 0x00c9_0760;
/// `Phyre_PRendering_NameIs(name; s)` `0x004ea7a0` (thiscall).
pub const NAME_IS: u32 = 0x004e_a7a0;
/// The class `0x005ce320` looks for in a target's chain.
pub const MESH_INSTANCE_CLASS: u32 = 0x00ca_91f8;
/// `PhyreGetTypeOf`'s error: file, line, function, message.
pub const TYPEOF_ERROR: (u32, u32, u32, u32) = (0x00b0_e268, 0x1ec, 0x00b0_e4dc, 0x00b0_e500);

/// The ports over game memory.
pub mod bridge {
    use super::{
        Conv, ERROR_FILE, ERROR_FUNCTION, ERROR_LINE, FREELIST_DEALLOCATE, Host, LUA_CHECKNUMBER,
        LUA_ERROR, LUA_GETFIELD, LUA_GETTOP, LUA_NEWUSERDATA, LUA_PUSHBOOLEAN, LUA_REGISTRYINDEX,
        LUA_SETMETATABLE, MESH_INSTANCE_CLASS, NAME_IS, SCHEDULER_RUNNING, STR_PHYRE_OBJECT,
        STR_VECTOR, TYPE_CLASS, TYPEOF_ERROR, mid, small, x87,
    };

    /// Whether a scheduler entry is the one named `name` (null matches
    /// any): its owner's name table (`owner + 0xc`, index `+0x10`) holds
    /// the same pointer or an equal string.
    fn name_matches(h: &impl Host, owner: u32, e: u32, name: u32) -> bool {
        if name == 0 {
            return true;
        }
        let names: u32 = h.read(owner.wrapping_add(0xc));
        let i: u32 = h.read(e.wrapping_add(0x10));
        let w: u32 = h.read(names.wrapping_add(i.wrapping_mul(4)));
        let s = w.wrapping_sub(w & 1);
        s == name || (s != 0 && crate::str_eq(h, s, name))
    }

    /// The u16 counters at `+0x84` for `phase`, `phase + step`, ... below
    /// 0x100 changed by `delta`.
    fn count(h: &mut impl Host, this: u32, phase: u32, step: u32, delta: u16) {
        if step == 0 {
            return;
        }
        let mut i = phase;
        while i < 0x100 {
            let at = this.wrapping_add(0x84).wrapping_add(i.wrapping_mul(2));
            let v: u16 = h.read(at);
            h.write(at, v.wrapping_add(delta));
            i = i.wrapping_add(step);
        }
    }

    /// An empty name is no name.
    fn nonempty(h: &impl Host, name: u32) -> u32 {
        if name != 0 && h.read::<u8>(name) == 0 {
            0
        } else {
            name
        }
    }

    /// `0x005d1370` (thiscall `(owner, key, name)`): the scheduler's
    /// entries of this owner, key and function name (any for null)
    /// uncounted and, unless the scheduler is running (`0x00cbd9b8`; then
    /// marked done, `+0x18` = 1), unlinked from `+0x24` and freed. 0 when
    /// any matched, else 0x13.
    pub fn scheduler_remove(h: &mut impl Host, this: u32, owner: u32, key: u32, name: u32) -> u32 {
        let name = nonempty(h, name);
        let running = h.read::<u32>(h.rebase(SCHEDULER_RUNNING)) != 0;
        let mut found = false;
        let mut e: u32 = h.read(this.wrapping_add(0x24));
        while e != 0 {
            let next: u32 = h.read(e);
            if h.read::<u32>(e.wrapping_add(4)) == owner
                && h.read::<u32>(e.wrapping_add(8)) == key
                && name_matches(h, owner, e, name)
            {
                found = true;
                let step: u16 = h.read(e.wrapping_add(0x14));
                let phase: u16 = h.read(e.wrapping_add(0x16));
                count(h, this, u32::from(phase), u32::from(step), 0xffff);
                if running {
                    h.write(e.wrapping_add(0x18), 1u32);
                } else {
                    let mut cell = this.wrapping_add(0x24);
                    let mut x: u32 = h.read(cell);
                    while x != 0 {
                        if x == e {
                            let n: u32 = h.read(x);
                            h.write(cell, n);
                            h.write(x, 0u32);
                            break;
                        }
                        cell = x;
                        x = h.read(x);
                    }
                    h.write(e.wrapping_add(0x10), u32::MAX);
                    h.call_sym(
                        FREELIST_DEALLOCATE,
                        Conv::Thiscall(this.wrapping_add(4)),
                        &[e],
                    );
                }
            }
            e = next;
        }
        if found { 0 } else { 0x13 }
    }

    /// `0x005d1670` (thiscall `(owner, key, name, step)`): the matching
    /// entries (as [`scheduler_remove`]) re-counted with the new period
    /// `step` (a u16). 0 when any matched, else 0x13.
    pub fn scheduler_set_step(
        h: &mut impl Host,
        this: u32,
        owner: u32,
        key: u32,
        name: u32,
        step: u32,
    ) -> u32 {
        let name = nonempty(h, name);
        let mut found = false;
        let mut e: u32 = h.read(this.wrapping_add(0x24));
        while e != 0 {
            let next: u32 = h.read(e);
            if h.read::<u32>(e.wrapping_add(4)) == owner
                && h.read::<u32>(e.wrapping_add(8)) == key
                && name_matches(h, owner, e, name)
            {
                found = true;
                let old: u16 = h.read(e.wrapping_add(0x14));
                let phase: u16 = h.read(e.wrapping_add(0x16));
                count(h, this, u32::from(phase), u32::from(old), 0xffff);
                let step = step as u16;
                h.write(e.wrapping_add(0x14), step);
                let phase: u16 = h.read(e.wrapping_add(0x16));
                count(h, this, u32::from(phase), u32::from(step), 1);
            }
            e = next;
        }
        if found { 0 } else { 0x13 }
    }

    /// `0x00570f80` `(L)`: a new `Vector` from up to four numbers (each
    /// missing one repeats the last; 0.0 without any); 1.
    pub fn vector_new(h: &mut impl Host, l: u32) -> u32 {
        let top = h.call_sym(LUA_GETTOP, Conv::Cdecl, &[l]);
        let u = h.call_sym(LUA_NEWUSERDATA, Conv::Cdecl, &[l, 0x10]);
        if u == 0 {
            return 1;
        }
        let name = h.rebase(STR_VECTOR);
        h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
        h.call_sym(LUA_SETMETATABLE, Conv::Cdecl, &[l, 0xffff_fffe]);
        let mut last = x87::store(0.0);
        for i in 0..4u32 {
            if (i as i32) < (top as i32) {
                last = h
                    .call_sym_st0(LUA_CHECKNUMBER, Conv::Cdecl, &[l, i + 1])
                    .to_bits();
            }
            h.write(u.wrapping_add(4 * i), x87::store(x87::load(last)));
        }
        1
    }

    /// `Phyre_PScripting_PhyreGetTypeOf` `0x00436b90` `(L)`: the class of
    /// the `PhyreObject` argument 1 pushed as a `PhyreObject` of the class
    /// descriptor type (its name, `+0x18`); 1. Anything else raises a Lua
    /// error (0).
    pub fn get_type_of(h: &mut impl Host, l: u32) -> u32 {
        let u = mid::check_userdata(h, STR_PHYRE_OBJECT, l, 1);
        if u != 0 {
            let d: u32 = h.read(u);
            let t = if d == 0 { 0 } else { d.wrapping_add(0x18) };
            let class = h.rebase(TYPE_CLASS);
            small::push_object(h, l, t, class);
            return 1;
        }
        let (file, line, function, msg) = TYPEOF_ERROR;
        let file = h.rebase(file);
        let at = h.rebase(ERROR_FILE);
        h.write(at, file);
        let at = h.rebase(ERROR_LINE);
        h.write(at, line);
        let function = h.rebase(function);
        let at = h.rebase(ERROR_FUNCTION);
        h.write(at, function);
        let msg = h.rebase(msg);
        h.call_sym(LUA_ERROR, Conv::Cdecl, &[l, msg]);
        0
    }

    /// `0x00436aa0` `(L)`: whether arguments 1 and 2 are the same object
    /// (both `PhyreObject`s with the same object, or neither one), pushed
    /// as a boolean; 1.
    pub fn objects_equal(h: &mut impl Host, l: u32) -> u32 {
        let a = mid::check_userdata(h, STR_PHYRE_OBJECT, l, 1);
        let b = mid::check_userdata(h, STR_PHYRE_OBJECT, l, 2);
        let eq = match (a, b) {
            (0, 0) => true,
            (0, _) | (_, 0) => false,
            _ => h.read::<u32>(a.wrapping_add(4)) == h.read::<u32>(b.wrapping_add(4)),
        };
        h.call_sym(LUA_PUSHBOOLEAN, Conv::Cdecl, &[l, u32::from(eq)]);
        1
    }

    /// `0x004d3d30` `(out_a, out_b, &cursor)`: a packed list at the bit
    /// array `*cursor`: a 4-bit count, then that many pairs of 5-bit
    /// values (into `out_a`, `out_b`); the cursor moves past the words it
    /// used. Returns the count.
    pub fn decode_pairs(h: &mut impl Host, out_a: u32, out_b: u32, cursor: u32) -> u32 {
        let words: u32 = h.read(cursor);
        let n = mid::read_bits_at(h, words, 0, 4);
        let mut pos = 4u32;
        for k in 0..n {
            let a = mid::read_bits_at(h, words, pos, 5);
            h.write(out_a.wrapping_add(4 * k), a);
            let b = mid::read_bits_at(h, words, pos.wrapping_add(5), 5);
            h.write(out_b.wrapping_add(4 * k), b);
            pos = pos.wrapping_add(10);
        }
        let at: u32 = h.read(cursor);
        h.write(
            cursor,
            at.wrapping_add((pos.wrapping_add(0x1f) >> 5).wrapping_mul(4)),
        );
        n
    }

    /// `0x005ce320` (thiscall `(out, targets, n)`): for each of the `n`
    /// 0x1c-byte binding targets of kind 1 whose class chain (`+0`,
    /// parents at `+0x40`) holds the mesh instance class, and whose result
    /// (6 words at `out`) is still unset, the mesh's segment names
    /// (the context's first object, names `+0x24`, count `+0x18`) are
    /// matched to the target's name (`+0x14`): `{1, segment, 0x40}`
    /// (the last match). 1 without targets or results, else 0.
    pub fn bind_mesh_targets(h: &mut impl Host, this: u32, out: u32, targets: u32, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        if targets == 0 || out == 0 {
            return 1;
        }
        let ctx: u32 = h.read(this.wrapping_add(4));
        let first: u32 = h.read(ctx);
        let names: u32 = h.read(first.wrapping_add(0x24));
        let count: u32 = h.read(first.wrapping_add(0x18));
        let count = count & 0x7fff_ffff;
        let segments: u32 = h.read(ctx.wrapping_add(0xc));
        let class = h.rebase(MESH_INSTANCE_CLASS);
        for k in 0..n {
            let t = targets.wrapping_add(k.wrapping_mul(0x1c));
            let o = out.wrapping_add(k.wrapping_mul(0x18));
            if h.read::<u32>(t.wrapping_add(0x10)) != 1 {
                continue;
            }
            let mut d: u32 = h.read(t);
            while d != 0 && d != class {
                d = h.read(d.wrapping_add(0x40));
            }
            if d == 0 || h.read::<u32>(o) != 0 {
                continue;
            }
            let w: u32 = h.read(t.wrapping_add(0x14));
            let name = w.wrapping_sub(w & 1);
            for i in 0..count {
                let cell = names.wrapping_add(4 * i);
                let s: u32 = h.read(cell);
                if s.wrapping_sub(s & 1) != 0
                    && h.call_sym(NAME_IS, Conv::Thiscall(cell), &[name]) as u8 != 0
                {
                    h.write(o, 1u32);
                    h.write(
                        o.wrapping_add(4),
                        segments.wrapping_add(i.wrapping_mul(0x40)),
                    );
                    h.write(o.wrapping_add(8), 0x40u32);
                }
            }
        }
        0
    }
}
