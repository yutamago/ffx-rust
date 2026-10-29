//! `PhyreEngine`'s scripting support outside the script layer (M5 stage 1,
//! list `phyre-scripting`, batch 2): Lua glue of middle size (the userdata
//! checks, `require`'s loaded table, the library openers, the allocator,
//! chunk loading), the scheduler's list walks, the descriptors'
//! `fixObjectsAfterLoading` steps of post effects, LODs, animatable
//! components and script methods, a float random number and a bit-field
//! read.
//!
//! As in [`crate::m5p_scripting_small`], none of this ever ran in a
//! coverage run; the parity tests build every branch.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_scripting_small::{
    LUA_CHECKSTACK, LUA_CREATETABLE, LUA_GETFIELD, LUA_NEWUSERDATA, LUA_PUSHINTEGER,
    LUA_REGISTRYINDEX, LUA_SETGLOBAL, LUA_SETMETATABLE, LUA_SETTABLE, LUA_SETTOP, LUA_TOUSERDATA,
    MEMCPY, OPERATOR_DELETE, PHYRE_FREE, STR_PHYRE_OBJECT, STR_VECTOR, with_al,
};

/// `lua_type(L, idx)` (cdecl).
pub const LUA_TYPE: u32 = 0x0094_c1e0;
/// `0x0094b130` `(L, idx)` (cdecl): `lua_gettable`.
pub const LUA_GETTABLE: u32 = 0x0094_b130;
/// `lua_gettop(L)` (cdecl).
pub const LUA_GETTOP: u32 = 0x0094_b160;
/// `lua_pcallk(L, nargs, nresults, errfunc, ctx, k)` (cdecl).
pub const LUA_PCALLK: u32 = 0x0094_b4d0;
/// `0x0094bfc0` `(L, idx, &len)` (cdecl): `lua_tolstring`.
pub const LUA_TOLSTRING: u32 = 0x0094_bfc0;
/// `luaL_loadbufferx(L, buf, size, name, mode)` (cdecl).
pub const LUA_LOADBUFFERX: u32 = 0x0094_d280;
/// `0x0094b0d0` `(L, idx)` (cdecl): `lua_getmetatable`.
pub const LUA_GETMETATABLE: u32 = 0x0094_b0d0;
/// `0x0094b8b0` `(L, a, b)` (cdecl): `lua_rawequal`.
pub const LUA_RAWEQUAL: u32 = 0x0094_b8b0;
/// `lua_pushcclosure(L, f, n)` (cdecl).
pub const LUA_PUSHCCLOSURE: u32 = 0x0094_b5c0;
/// `0x0094bbb0` `(L, idx)` (cdecl): `lua_remove`.
pub const LUA_REMOVE: u32 = 0x0094_bbb0;
/// `Phyre_PhyrePrintf(level, fmt, ...)` (cdecl, varargs).
pub const PRINTF: u32 = 0x0043_52f0;
/// `Phyre_PhyreMalloc(size, align)` `0x0042fb80` (cdecl).
pub const MALLOC_ALIGNED: u32 = 0x0042_fb80;
/// `Phyre_PhyreMalloc(size)` `0x0042fb40` (cdecl).
pub const MALLOC: u32 = 0x0042_fb40;
/// `Phyre_PFreeList_deallocate(list; p)` (thiscall).
pub const FREELIST_DEALLOCATE: u32 = 0x0049_c980;
/// `Phyre_PRendering_NameIs(name; s)` `0x004ea7a0` (thiscall).
pub const NAME_IS: u32 = 0x004e_a7a0;
/// `0x005709f0` (thiscall, two words: name, `&out`): a vector or matrix
/// component index by name.
pub const COMPONENT_INDEX: u32 = 0x0057_09f0;
/// `ShaderParams_FindByName(params; name)` (thiscall).
pub const SHADER_PARAM: u32 = 0x0056_cbf0;
/// `PDepthOfFieldBase::initializeCommonShaders(this; a, b)` (thiscall).
pub const DOF_INIT_SHADERS: u32 = 0x00a1_85a0;
/// The shader name the post effects pass.
pub const POSTFX_SHADER: u32 = 0x0194_3da0;
/// `0x009f0660` (thiscall): the stream writer's base destructor.
pub const WRITER_BASE_DTOR: u32 = 0x009f_0660;
/// `0x00607c10` (thiscall): the stream file's destructor (a bare `ret`).
pub const STREAM_DTOR: u32 = 0x0060_7c10;
/// `PStreamFile` constructor `(this; path, a, b, c, d)` (thiscall).
pub const STREAM_CTOR: u32 = 0x0060_7bc0;
/// `PStreamReader::PStreamReader(this; path)` (thiscall).
pub const READER_CTOR: u32 = 0x009f_0630;
/// `PStreamWriterFile`'s vtable.
pub const WRITER_VTABLE: u32 = 0x00b7_3d4c;
/// `0x00449b90` (thiscall, one word): a dynamic member's fixup.
pub const DYNAMIC_MEMBER_FIXUP: u32 = 0x0044_9b90;
/// `Phyre_PClassDescriptor_findMember(class; name)` (thiscall).
pub const FIND_MEMBER: u32 = 0x0043_59d0;
/// `0x0053a2e0` (thiscall, three words): the animation set's evaluate.
pub const ANIM_EVALUATE: u32 = 0x0053_a2e0;
/// `0x00536230` (cdecl, four words): the animation set's bind.
pub const ANIM_BIND: u32 = 0x0053_6230;
/// The class `0x00541c00` looks for in its context's list.
pub const ANIM_SET_CLASS: u32 = 0x00c9_3ae8;
/// `0x00543150` (thiscall, one word).
pub const ANIM_SLOT_CHECK: u32 = 0x0054_3150;
/// `0x00543570` (thiscall, one word).
pub const ANIM_SLOT_FIND: u32 = 0x0054_3570;
/// `0x0052cec0` (thiscall, two words).
pub const ANIM_SET_TARGET: u32 = 0x0052_cec0;
/// `0x0052e660` (thiscall, two words: index, float).
pub const ANIM_SET_WEIGHT: u32 = 0x0052_e660;
/// `0x005a80f0` (thiscall, two words): the LOD group's buffer setter.
pub const LOD_GROUP_SET: u32 = 0x005a_80f0;
/// `0x005a8190` (thiscall, two words): the LOD level's buffer setter.
pub const LOD_LEVEL_SET: u32 = 0x005a_8190;
/// The LOD count.
pub const LOD_COUNT: u32 = 0x00c2_6b7c;
/// The flag `0x005a5110` sets.
pub const LOD_LOADED: u32 = 0x00cb_aa80;
/// The global list `0x0045ad00` links name components into.
pub const NAME_LIST: u32 = 0x00c0_cd50;
/// The registrant list `0x005d52d0` unlinks from.
pub const UTILITY_LIST: u32 = 0x00c9_01a8;
/// `PUtilityScripting`'s base vtable `0x005d52d0` stores.
pub const UTILITY_VTABLE: u32 = 0x00b0_e078;
/// The script method caller's vtable `0x00619980` stores.
pub const METHOD_SCRIPT_CALLER_VTABLE: u32 = 0x00b3_f4f0;
/// The library opener table: count `0x00cbe6d8`, entries `0x00cbe6dc`.
pub const OPENERS_COUNT: u32 = 0x00cb_e6d8;
/// See [`OPENERS_COUNT`].
pub const OPENERS: u32 = 0x00cb_e6dc;
/// The `RunScriptForRequire` error format (`lua_pcall` failed).
pub const STR_PCALL_FAILED: u32 = 0x00b3_90a4;
/// "_LOADED" (the registry's loaded-modules table).
pub const STR_LOADED: u32 = 0x00b3_9140;
/// The counter of the float random number.
pub const RANDOM_COUNTER: u32 = 0x00c9_43e4;
/// 2^32 as a `float`.
pub const TWO_POW_32: u32 = 0x00b1_1f14;
/// 2^-32 as a `double`.
pub const TWO_POW_M32: u32 = 0x00b0_db98;

/// The global functions `RegisterFunctions` sets: (function, name).
pub const GLOBAL_FUNCTIONS: [(u32, u32); 4] = [
    (0x005d_63a0, 0x00b3_92b0), // require
    (0x005d_67b0, 0x00b3_92b8), // unrequire
    (0x005d_6050, 0x00b3_92c4), // profile
    (0x005d_5c00, 0x00b3_92cc), // print
];

/// The post effects' `fixObjectsAfterLoading` (`(p)`): each effect's
/// slot 1 `(0, 0)`, then its shaders with a context; per copy the
/// initialiser.
pub const POSTFX_FIXUPS: [(u32, u32); 5] = [
    (0x00a0_7e80, 0x00a1_a4f0),
    (0x00a0_7fb0, 0x00a1_8870),
    (0x00a0_8030, 0x00a1_a830),
    (0x00a0_80b0, 0x00a1_8c80),
    (0x00a0_8130, 0x00a1_ae00),
];

/// The Lua C functions that call the method of their upvalue's caller:
/// `0x00437160` and `0x004371e0` (identical copies).
pub const UPVALUE_CALLERS: [u32; 2] = [0x0043_7160, 0x0043_71e0];

/// The userdata checks `(L, idx)`: (function, metatable name).
pub const USERDATA_CHECKS: [(u32, u32); 2] =
    [(0x0043_6160, STR_PHYRE_OBJECT), (0x0057_0dc0, STR_VECTOR)];

/// The first record of a descriptor's instance list `p` (`+0x30`, which
/// points 4 bytes into the record; 0 for none), the count `+0x3c` and the
/// stride (the class `+0x2c`'s size `+0x1c`).
fn records(h: &impl Host, p: u32, back: u32) -> (u32, u32, u32) {
    let at: u32 = h.read(p.wrapping_add(0x30));
    let first = if at == 0 { 0 } else { at.wrapping_sub(back) };
    let n: u32 = h.read(p.wrapping_add(0x3c));
    let class: u32 = h.read(p.wrapping_add(0x2c));
    let stride: u32 = h.read(class.wrapping_add(0x1c));
    (first, n, stride)
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        ANIM_BIND, ANIM_EVALUATE, ANIM_SET_CLASS, ANIM_SET_TARGET, ANIM_SET_WEIGHT,
        ANIM_SLOT_CHECK, ANIM_SLOT_FIND, COMPONENT_INDEX, Conv, DOF_INIT_SHADERS,
        DYNAMIC_MEMBER_FIXUP, FIND_MEMBER, FREELIST_DEALLOCATE, GLOBAL_FUNCTIONS, Host, LOD_COUNT,
        LOD_GROUP_SET, LOD_LEVEL_SET, LOD_LOADED, LUA_CHECKSTACK, LUA_CREATETABLE, LUA_GETFIELD,
        LUA_GETMETATABLE, LUA_GETTABLE, LUA_GETTOP, LUA_LOADBUFFERX, LUA_NEWUSERDATA, LUA_PCALLK,
        LUA_PUSHCCLOSURE, LUA_PUSHINTEGER, LUA_RAWEQUAL, LUA_REGISTRYINDEX, LUA_REMOVE,
        LUA_SETGLOBAL, LUA_SETMETATABLE, LUA_SETTABLE, LUA_SETTOP, LUA_TOLSTRING, LUA_TOUSERDATA,
        LUA_TYPE, MALLOC, MALLOC_ALIGNED, MEMCPY, METHOD_SCRIPT_CALLER_VTABLE, NAME_IS, NAME_LIST,
        OPENERS, OPENERS_COUNT, OPERATOR_DELETE, PHYRE_FREE, POSTFX_SHADER, PRINTF, RANDOM_COUNTER,
        READER_CTOR, SHADER_PARAM, STR_LOADED, STR_PCALL_FAILED, STR_VECTOR, STREAM_CTOR,
        STREAM_DTOR, TWO_POW_32, TWO_POW_M32, UTILITY_LIST, UTILITY_VTABLE, WRITER_BASE_DTOR,
        WRITER_VTABLE, records, with_al, x87,
    };
    use crate::m5p_scripting_small::bridge as small;

    /// `lua_upvalueindex(1)`.
    const UPVALUE_1: u32 = 0xfff0_b9d7;

    // ---- Lua ----

    /// `0x00572150` `(L, v)`: a new "Vector" userdata holding the four
    /// floats of `v` (through the x87); `v`, or 0 when the allocation
    /// failed.
    pub fn push_vector(h: &mut impl Host, l: u32, v: u32) -> u32 {
        let u = h.call_sym(LUA_NEWUSERDATA, Conv::Cdecl, &[l, 0x10]);
        if u == 0 {
            return 0;
        }
        let name = h.rebase(STR_VECTOR);
        h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
        h.call_sym(LUA_SETMETATABLE, Conv::Cdecl, &[l, 0xffff_fffe]);
        for i in 0..4u32 {
            let f: u32 = h.read(v.wrapping_add(4 * i));
            h.write(u.wrapping_add(4 * i), x87::store(x87::load(f)));
        }
        v
    }

    /// `0x004363b0` `(L, n, at, type)`: when `L`'s value 0 is a table
    /// (`lua_type` 5), its entries 1..n read into `at` (stride the type's
    /// size `+0x1c`) through the type's slot 3 `(L, at)`. `eax`: the type
    /// when not a table, 5 for `n` 0, else the cursor past the last.
    pub fn read_array(h: &mut impl Host, l: u32, n: u32, at: u32, ty: u32) -> u32 {
        let t = h.call_sym(LUA_TYPE, Conv::Cdecl, &[l, 0]);
        if t != 5 {
            return t;
        }
        let mut eax = t;
        let mut at = at;
        let mut i = 0u32;
        while i < n {
            i = i.wrapping_add(1);
            h.call_sym(LUA_PUSHINTEGER, Conv::Cdecl, &[l, i]);
            h.call_sym(LUA_GETTABLE, Conv::Cdecl, &[l, 0xffff_fffe]);
            h.call_virtual(ty, 3, &[l, at]);
            let size: u32 = h.read(ty.wrapping_add(0x1c));
            at = at.wrapping_add(size);
            eax = at;
        }
        eax
    }

    /// `0x00436e20` `(L, n, at, type)`: a table of the `n` values at `at`
    /// (stride the type's size), each pushed by the type's slot 4 `(L,
    /// at)`; the last `lua_settable`'s `eax`, or the size for `n` 0.
    pub fn push_values(h: &mut impl Host, l: u32, n: u32, at: u32, ty: u32) -> u32 {
        h.call_sym(LUA_CREATETABLE, Conv::Cdecl, &[l, n, 0]);
        let size: u32 = h.read(ty.wrapping_add(0x1c));
        let mut eax = size;
        let mut at = at;
        let mut i = 0u32;
        while i < n {
            i = i.wrapping_add(1);
            h.call_sym(LUA_PUSHINTEGER, Conv::Cdecl, &[l, i]);
            h.call_virtual(ty, 4, &[l, at]);
            eax = h.call_sym(LUA_SETTABLE, Conv::Cdecl, &[l, 0xffff_fffd]);
            at = at.wrapping_add(size);
        }
        eax
    }

    /// `0x005d65f0` `(&L)`: the chunk on `L`'s stack run (`lua_pcallk(L,
    /// 0, 0, 0, 0, 0)`); on an error its message printed (level 3) and
    /// popped, 9; else 0.
    pub fn run_chunk(h: &mut impl Host, pl: u32) -> u32 {
        let l: u32 = h.read(pl);
        let r = h.call_sym(LUA_PCALLK, Conv::Cdecl, &[l, 0, 0, 0, 0, 0]);
        if r == 0 {
            return 0;
        }
        let l: u32 = h.read(pl);
        let msg = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, u32::MAX, 0]);
        let fmt = h.rebase(STR_PCALL_FAILED);
        h.call_sym(PRINTF, Conv::Cdecl, &[3, fmt, r, msg]);
        let l: u32 = h.read(pl);
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        9
    }

    /// `0x005d5ba0` `(ud, old, old_size, new_size)`: the Lua allocator:
    /// size 0 frees `old` (0); else a 16-aligned block of `new_size`, the
    /// old block's bytes (the smaller size) copied and `old` freed.
    pub fn lua_alloc(h: &mut impl Host, old: u32, old_size: u32, new_size: u32) -> u32 {
        if new_size == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
            return 0;
        }
        let new = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[new_size, 0x10]);
        if old != 0 && new != 0 {
            let n = if old_size < new_size {
                old_size
            } else {
                new_size
            };
            h.call_sym(MEMCPY, Conv::Cdecl, &[new, old, n]);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        new
    }

    /// `0x00436750` `(L, &script)`: the script's buffer (`+4`, size `+0`
    /// less bit 31, name the `PString` at `+0x1c`) loaded as a chunk; al 1
    /// when it loaded, else 0 (an error message popped). `eax` above al:
    /// the size (no buffer), `lua_settop`'s (an error), else the load's.
    pub fn load_script(h: &mut impl Host, l: u32, s: u32) -> u32 {
        let name: u32 = h.read(s.wrapping_add(0x1c));
        let name = name.wrapping_sub(name & 1);
        let size: u32 = h.read(s);
        let size = size & 0x7fff_ffff;
        let buf: u32 = h.read(s.wrapping_add(4));
        if buf == 0 {
            return with_al(size, 0);
        }
        let r = h.call_sym(LUA_LOADBUFFERX, Conv::Cdecl, &[l, buf, size, name, 0]);
        if r == 0 {
            return with_al(r, 1);
        }
        h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, u32::MAX, 0]);
        let t = h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        with_al(t, 0)
    }

    /// `Phyre_PScripting_GetLibraryOpener` `0x005d5710` `(name)`: the
    /// function of the opener table entry named `name`, or 0.
    pub fn library_opener(h: &impl Host, name: u32) -> u32 {
        let n: u32 = h.read(h.rebase(OPENERS_COUNT));
        let n = n & 0x7fff_ffff;
        let table: u32 = h.read(h.rebase(OPENERS));
        for i in 0..n {
            let e = table.wrapping_add(i.wrapping_mul(8));
            let s: u32 = h.read(e);
            if crate::str_eq(h, name, s) {
                return h.read(e.wrapping_add(4));
            }
        }
        0
    }

    /// [`super::USERDATA_CHECKS`] `(L, idx)`: the userdata at `idx` when
    /// its metatable is the registry's `name`, else 0.
    pub fn check_userdata(h: &mut impl Host, name: u32, l: u32, idx: u32) -> u32 {
        let u = h.call_sym(LUA_TOUSERDATA, Conv::Cdecl, &[l, idx]);
        if u == 0 || h.call_sym(LUA_GETMETATABLE, Conv::Cdecl, &[l, idx]) == 0 {
            return 0;
        }
        let name = h.rebase(name);
        h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
        let eq = h.call_sym(LUA_RAWEQUAL, Conv::Cdecl, &[l, u32::MAX, 0xffff_fffe]) != 0;
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffd]);
        if eq { u } else { 0 }
    }

    /// `0x00436240` `(L)`: a component index by name (argument 2) from the
    /// upvalue's object (`0x005709f0(name, &index)`, al), pushed (1); 0
    /// when there is no name, object or index. `out` is the index word
    /// the callee writes (the original's frame word, zeroed first).
    pub fn component_index(h: &mut impl Host, l: u32, out: u32) -> u32 {
        let name = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, 2, 0]);
        if name == 0 {
            return 0;
        }
        h.write(out, 0u32);
        let u = h.call_sym(LUA_TOUSERDATA, Conv::Cdecl, &[l, UPVALUE_1]);
        if u == 0 || h.call_sym(COMPONENT_INDEX, Conv::Thiscall(u), &[name, out]) as u8 == 0 {
            return 0;
        }
        h.call_sym(LUA_CHECKSTACK, Conv::Cdecl, &[l, 1]);
        let i: u32 = h.read(out);
        h.call_sym(LUA_PUSHINTEGER, Conv::Cdecl, &[l, i]);
        1
    }

    /// `Phyre_PScripting_RegisterFunctions` `0x005d6070` `(L)`: the four
    /// globals `require`, `unrequire`, `profile`, `print`; the last
    /// `lua_setglobal`'s `eax`.
    pub fn register_functions(h: &mut impl Host, l: u32) -> u32 {
        let mut eax = 0;
        for &(f, name) in &GLOBAL_FUNCTIONS {
            let f = h.rebase(f);
            h.call_sym(LUA_PUSHCCLOSURE, Conv::Cdecl, &[l, f, 0]);
            let name = h.rebase(name);
            eax = h.call_sym(LUA_SETGLOBAL, Conv::Cdecl, &[l, name]);
        }
        eax
    }

    /// `0x005d5a90` `(L, name, keep)`: whether the registry's `_LOADED`
    /// table holds a table under `name`; when it does (al 1) that table
    /// stays on the stack with `keep`, else everything pushed is popped
    /// (al 0). `eax` above al: the last `lua_settop`'s.
    pub fn loaded_module(h: &mut impl Host, l: u32, name: u32, keep: u32) -> u32 {
        let loaded = h.rebase(STR_LOADED);
        h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, loaded]);
        if h.call_sym(LUA_TYPE, Conv::Cdecl, &[l, u32::MAX]) != 5 {
            let t = h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
            return with_al(t, 0);
        }
        h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, u32::MAX, name]);
        if h.call_sym(LUA_TYPE, Conv::Cdecl, &[l, u32::MAX]) != 5 {
            let t = h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffd]);
            return with_al(t, 0);
        }
        h.call_sym(LUA_REMOVE, Conv::Cdecl, &[l, 0xffff_fffe]);
        let top = if keep as u8 == 0 {
            0xffff_fffe
        } else {
            u32::MAX
        };
        let t = h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, top]);
        with_al(t, 1)
    }

    /// [`super::UPVALUE_CALLERS`] `(L)`: the upvalue's caller object
    /// (`+0x14`) gets its signature (slot 4, a 0-terminated word list)
    /// counted; unless flag 4 of `+0x10` is set, extra arguments beyond
    /// it are popped; then the object's slot 1 `(L)`. 0 without an
    /// upvalue or object.
    pub fn upvalue_call(h: &mut impl Host, l: u32) -> u32 {
        let u = h.call_sym(LUA_TOUSERDATA, Conv::Cdecl, &[l, UPVALUE_1]);
        if u == 0 {
            return 0;
        }
        let top = h.call_sym(LUA_GETTOP, Conv::Cdecl, &[l]);
        let obj: u32 = h.read(u.wrapping_add(0x14));
        let sig = if obj == 0 {
            0
        } else {
            h.call_virtual(obj, 4, &[])
        };
        let mut count = 0u32;
        if sig != 0 {
            while h.read::<u32>(sig.wrapping_add(4 * count)) != 0 {
                count = count.wrapping_add(1);
            }
        }
        let flags: u8 = h.read(u.wrapping_add(0x10));
        if flags & 4 == 0 && (count as i32) < (top as i32) {
            let idx = count.wrapping_sub(top).wrapping_sub(1);
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, idx]);
        }
        let obj: u32 = h.read(u.wrapping_add(0x14));
        if obj == 0 {
            return 0;
        }
        h.call_virtual(obj, 1, &[l])
    }

    // ---- objects ----

    /// `0x005d52d0` (thiscall `(flags)`): a registrant's deleting
    /// destructor: its base vtable, unlinked from the global list (next
    /// at `+4`), freed when `flags & 1`; `this`.
    pub fn registrant_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(UTILITY_VTABLE);
        h.write(this, vt);
        let mut cell = h.rebase(UTILITY_LIST);
        if h.read::<u32>(cell) != 0 {
            loop {
                let e: u32 = h.read(cell);
                if e == this {
                    break;
                }
                cell = e.wrapping_add(4);
                if h.read::<u32>(cell) == 0 {
                    break;
                }
            }
            if h.read::<u32>(cell) != 0 {
                let next: u32 = h.read(this.wrapping_add(4));
                h.write(cell, next);
                h.write(this.wrapping_add(4), 0u32);
            }
        }
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x009f07a0` (thiscall `(flags)`): `PStreamWriterFile`'s deleting
    /// destructor: its vtable, the base destructor, the stream's (`+0x10`),
    /// `PhyreFree` when `flags & 1`; `this`.
    pub fn writer_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(WRITER_VTABLE);
        h.write(this, vt);
        h.call_sym(WRITER_BASE_DTOR, Conv::Thiscall(this), &[]);
        h.call_sym(STREAM_DTOR, Conv::Thiscall(this.wrapping_add(0x10)), &[]);
        if flags & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x009f0810` `(path)`: a new `PStreamWriterFile` (0x18 bytes): its
    /// stream opened for writing, the reader part constructed, its
    /// vtable; 0 when the allocation failed.
    pub fn new_writer(h: &mut impl Host, path: u32) -> u32 {
        let p = h.call_sym(MALLOC, Conv::Cdecl, &[0x18]);
        if p == 0 {
            return 0;
        }
        h.call_sym(
            STREAM_CTOR,
            Conv::Thiscall(p.wrapping_add(0x10)),
            &[path, 0, 0, 0, 1],
        );
        h.call_sym(READER_CTOR, Conv::Thiscall(p), &[path]);
        let vt = h.rebase(WRITER_VTABLE);
        h.write(p, vt);
        p
    }

    /// `0x005d1220` (thiscall): the scheduler's finished entries (state
    /// `+0x18` 1) removed from its list (`+0x24`, next first), marked
    /// (`+0x10` = -1) and returned to the free list (`+4`).
    pub fn reap_finished(h: &mut impl Host, this: u32) {
        let head = this.wrapping_add(0x24);
        let mut prev = head;
        loop {
            let e: u32 = h.read(prev);
            if e == 0 {
                return;
            }
            if h.read::<u32>(e.wrapping_add(0x18)) != 1 {
                prev = e;
                continue;
            }
            let mut cell = head;
            loop {
                let x: u32 = h.read(cell);
                if x == 0 {
                    break;
                }
                if x == e {
                    let next: u32 = h.read(x);
                    h.write(cell, next);
                    h.write(x, 0u32);
                    break;
                }
                cell = x;
            }
            h.write(e.wrapping_add(0x10), u32::MAX);
            h.call_sym(
                FREELIST_DEALLOCATE,
                Conv::Thiscall(this.wrapping_add(4)),
                &[e],
            );
        }
    }

    /// `0x005d0e20` (thiscall `(owner, key, name)`): whether a live entry
    /// (state `+0x18` not 1) of the list at `+0x24` has this owner (`+4`)
    /// and key (`+8`), and, for a non-empty `name`, whether the owner's
    /// name table (`+0xc`, index `+0x10`) says `NameIs(name)`.
    pub fn has_entry(h: &mut impl Host, this: u32, owner: u32, key: u32, name: u32) -> u8 {
        let name = if name != 0 && h.read::<u8>(name) == 0 {
            0
        } else {
            name
        };
        let mut e: u32 = h.read(this.wrapping_add(0x24));
        while e != 0 {
            if h.read::<u32>(e.wrapping_add(0x18)) != 1
                && h.read::<u32>(e.wrapping_add(4)) == owner
                && h.read::<u32>(e.wrapping_add(8)) == key
            {
                if name == 0 {
                    return 1;
                }
                let names: u32 = h.read(owner.wrapping_add(0xc));
                let i: u32 = h.read(e.wrapping_add(0x10));
                let at = names.wrapping_add(i.wrapping_mul(4));
                if h.call_sym(NAME_IS, Conv::Thiscall(at), &[name]) as u8 != 0 {
                    return 1;
                }
            }
            e = h.read(e);
        }
        0
    }

    /// `0x0045ea50` `()`: the next float in [0, 1): the counter
    /// `0x00c943e4` incremented and hashed, converted through the x87
    /// (unsigned, rounded to `float`) and scaled by 2^-32.
    pub fn random_float(h: &mut impl Host) -> f32 {
        let at = h.rebase(RANDOM_COUNTER);
        let c: u32 = h.read(at);
        let c = c.wrapping_add(1);
        h.write(at, c);
        let x = (c ^ 0x00bc_602f).wrapping_mul(0x9e37_79b9);
        let y = ((x >> 26) ^ (x << 6) ^ x).wrapping_mul(0x9e37_79b9);
        let z = ((y >> 12) ^ (y << 5)).wrapping_add(y);
        let mut v = f64::from(z as i32);
        if (z as i32) < 0 {
            let k: u32 = h.read(h.rebase(TWO_POW_32));
            v = x87::add(v, x87::load(k));
        }
        let f = x87::store(v);
        let scale: u64 = h.read(h.rebase(TWO_POW_M32));
        let p = x87::mul(x87::load(f), f64::from_bits(scale));
        f32::from_bits(x87::store(p))
    }

    /// `0x004d3ef0` (thiscall `(start, n)`): `n` bits of the bit array
    /// `*this` from bit `start`, low bit first, word by word (a whole
    /// 32-bit word reads as 0: the original's mask `(1 << 32) - 1` is 0).
    pub fn read_bits(h: &impl Host, this: u32, start: u32, n: u32) -> u32 {
        let words: u32 = h.read(this);
        read_bits_at(h, words, start, n)
    }

    /// [`read_bits`] on the bit array at `words`.
    pub fn read_bits_at(h: &impl Host, words: u32, start: u32, n: u32) -> u32 {
        let mut out = 0u32;
        let mut done = 0u32;
        while done < n {
            let pos = start.wrapping_add(done);
            let sh = pos & 0x1f;
            let w: u32 = h.read(words.wrapping_add((pos >> 5).wrapping_mul(4)));
            let end = sh.wrapping_sub(done).wrapping_add(n);
            let end = if end >= 0x20 { 0x20 } else { end };
            let take = end.wrapping_sub(sh);
            let mask = 1u32.wrapping_shl(take).wrapping_sub(1);
            out |= (w.wrapping_shr(sh) & mask).wrapping_shl(done);
            done = done.wrapping_add(take);
        }
        out
    }

    // ---- fixObjectsAfterLoading steps ----

    /// `0x0045ad00` `(p)`: the name components (`+0x30` records) get
    /// their owner (`0x0044db10`) and their links (`+0xc`) put at the
    /// front of the global list `0x00c0cd50`; 0.
    pub fn link_names(h: &mut impl Host, p: u32) -> u32 {
        small::set_owner_all(h, p);
        let (mut rec, mut n, stride) = (
            h.read::<u32>(p.wrapping_add(0x30)),
            h.read::<u32>(p.wrapping_add(0x3c)),
            {
                let class: u32 = h.read(p.wrapping_add(0x2c));
                h.read::<u32>(class.wrapping_add(0x1c))
            },
        );
        let list = h.rebase(NAME_LIST);
        while n != 0 {
            let link = if rec == 0 { 0 } else { rec.wrapping_add(0xc) };
            let head: u32 = h.read(list);
            h.write(link, head);
            h.write(link.wrapping_add(4), list);
            rec = rec.wrapping_add(stride);
            h.write(list, link);
            h.write(head.wrapping_add(4), link);
            n = n.wrapping_sub(1);
        }
        0
    }

    /// `0x00446650` `(p)`: each dynamic member record (`+0x30` less 4):
    /// its two lists (`+4`, `+0x1c`) emptied, flag bit 0 of `+0x14`
    /// cleared, its name pointer (`+0x10`) from the `PString` at `+0x2c`,
    /// and when its class (`+0xc`) has flag 0x90 bit 0, the class's fixup
    /// `0x00449b90(record)`; 0.
    pub fn fix_dynamic_members(h: &mut impl Host, p: u32) -> u32 {
        let (mut r, mut n, stride) = records(h, p, 4);
        while n != 0 {
            let a = r.wrapping_add(4);
            if r != 0 && a != 0 {
                h.write(a, a);
                h.write(a.wrapping_add(4), a);
            }
            let b = r.wrapping_add(0x1c);
            if b != 0 {
                h.write(b, b);
                h.write(b.wrapping_add(4), b);
            }
            let s: u32 = h.read(r.wrapping_add(0x2c));
            let fl: u32 = h.read(r.wrapping_add(0x14));
            h.write(r.wrapping_add(0x14), fl & !1);
            h.write(r.wrapping_add(0x10), s.wrapping_sub(s & 1));
            let class: u32 = h.read(r.wrapping_add(0xc));
            if h.read::<u8>(class.wrapping_add(0x90)) & 1 != 0 {
                h.call_sym(DYNAMIC_MEMBER_FIXUP, Conv::Thiscall(class), &[r]);
            }
            r = r.wrapping_add(stride);
            n = n.wrapping_sub(1);
        }
        0
    }

    /// [`super::POSTFX_FIXUPS`] `(p)`: each effect (`+0x30` less 4): slot
    /// 1 `(0, 0)`; with a context whose first word is set, `init(shader,
    /// first)`; the first non-zero result is returned, else 0.
    pub fn fix_postfx(h: &mut impl Host, init: u32, p: u32) -> u32 {
        let (mut e, n, stride) = records(h, p, 4);
        let mut i = 0u32;
        while i < n {
            let r = h.call_virtual(e, 1, &[0, 0]);
            if r != 0 {
                return r;
            }
            let ctx: u32 = h.read(e.wrapping_add(8));
            if ctx != 0 && h.read::<u32>(ctx) != 0 {
                let first = small::first_of(h, e);
                let shader = h.rebase(POSTFX_SHADER);
                let r = h.call_sym(init, Conv::Thiscall(e), &[shader, first]);
                if r != 0 {
                    return r;
                }
            }
            e = e.wrapping_add(stride);
            i = i.wrapping_add(1);
        }
        0
    }

    /// `0x00a1b710` (thiscall): the depth of field's shaders initialised
    /// with a context (`DOF_INIT_SHADERS`, its non-zero `eax` returned),
    /// then its three parameters looked up; `+0xc` = `+8`; 0.
    pub fn dof_init(h: &mut impl Host, this: u32) -> u32 {
        let ctx: u32 = h.read(this.wrapping_add(8));
        if ctx != 0 && h.read::<u32>(ctx) != 0 {
            let first = small::first_of(h, this);
            let shader = h.rebase(POSTFX_SHADER);
            let r = h.call_sym(DOF_INIT_SHADERS, Conv::Thiscall(this), &[shader, first]);
            if r != 0 {
                return r;
            }
            for (name, off) in [
                (0x00b7_6734u32, 0xfb4u32),
                (0x00b7_6748, 0xfb8),
                (0x00b7_6754, 0xfbc),
            ] {
                let name = h.rebase(name);
                let v = h.call_sym(SHADER_PARAM, Conv::Thiscall(first), &[name]);
                h.write(this.wrapping_add(off), v);
            }
        }
        let ctx: u32 = h.read(this.wrapping_add(8));
        h.write(this.wrapping_add(0xc), ctx);
        0
    }

    /// `0x005a5110` (`PLODGroup`) / `0x005a5190` (`PLODLevel`) `(p)`: with
    /// more than one LOD, each record (`+0x30`) gets its part of the
    /// buffer at `+0x38` (`setter(buf, count)`), whose `count` entries
    /// are initialised (a group's words zeroed; a level's 12-byte entries
    /// `{1.0, 0.0, 4}`). 7 when there is no buffer; 0. The group's copy
    /// first sets the loaded flag `0x00cbaa80`.
    pub fn lod_fixup(h: &mut impl Host, p: u32, level: bool) -> u32 {
        let count: u32 = h.read(h.rebase(LOD_COUNT));
        if !level {
            let at = h.rebase(LOD_LOADED);
            h.write(at, 1u8);
        }
        let mut n: u32 = h.read(p.wrapping_add(0x3c));
        let class: u32 = h.read(p.wrapping_add(0x2c));
        let stride: u32 = h.read(class.wrapping_add(0x1c));
        let mut rec: u32 = h.read(p.wrapping_add(0x30));
        if count <= 1 {
            return 0;
        }
        let mut buf: u32 = h.read(p.wrapping_add(0x38));
        if buf == 0 {
            return 7;
        }
        let (setter, off, size) = if level {
            (LOD_LEVEL_SET, 0x14, 12)
        } else {
            (LOD_GROUP_SET, 0x20, 4)
        };
        while n != 0 {
            h.call_sym(setter, Conv::Thiscall(rec.wrapping_add(off)), &[buf, count]);
            for _ in 0..count {
                let e = buf;
                buf = buf.wrapping_add(size);
                if e != 0 {
                    if level {
                        h.write(e.wrapping_add(8), 4u32);
                        h.write(e, 0x3f80_0000u32);
                        h.write(e.wrapping_add(4), 0u32);
                    } else {
                        h.write(e, 0u32);
                    }
                }
            }
            rec = rec.wrapping_add(stride);
            n = n.wrapping_sub(1);
        }
        0
    }

    /// `0x00619980` `(p)`: each script method record (`+0x30`): its list
    /// node emptied, its name (`+0xc`) from the `PString` at `+0x2c`; a
    /// named one its class (`+8`) does not declare (`findMember` 0) is
    /// linked into the class's list at `+0x4c`; its caller (`+0x18`) gets
    /// the vtable and the record, `+0x14` points at it; 0.
    pub fn fix_method_scripts(h: &mut impl Host, p: u32) -> u32 {
        let (mut r, mut n, stride) = records(h, p, 0);
        while n != 0 {
            if r != 0 {
                h.write(r, r);
                h.write(r.wrapping_add(4), r);
            }
            let s: u32 = h.read(r.wrapping_add(0x2c));
            let name = s.wrapping_sub(s & 1);
            let class: u32 = h.read(r.wrapping_add(8));
            h.write(r.wrapping_add(0xc), name);
            let list = class.wrapping_add(0x4c);
            if name != 0 && h.call_sym(FIND_MEMBER, Conv::Thiscall(class), &[name]) == 0 {
                let tail: u32 = h.read(list.wrapping_add(4));
                h.write(r, list);
                h.write(r.wrapping_add(4), tail);
                h.write(tail, r);
                h.write(list.wrapping_add(4), r);
            }
            let caller = r.wrapping_add(0x18);
            let v = if caller == 0 {
                0
            } else {
                let vt = h.rebase(METHOD_SCRIPT_CALLER_VTABLE);
                h.write(caller, vt);
                h.write(caller.wrapping_add(4), r);
                caller
            };
            h.write(r.wrapping_add(0x14), v);
            r = r.wrapping_add(stride);
            n = n.wrapping_sub(1);
        }
        0
    }

    /// `0x00541c00` (thiscall): an animatable component with a set
    /// (`+0x10`) finds the animation set (class `0x00c93ae8`) in its
    /// context's list (`+8`, next first); with the same set as last time
    /// (`+0x5c`) it re-evaluates (`0x0053a2e0(+0x38, 0, 0)` on `+0x48`
    /// with `+0x40` set to the set meanwhile), else binds (`0x00536230(
    /// +0x48, +0x38, set, entry + 0xc)`, its result). 7 without a set,
    /// else 0.
    pub fn animatable_bind(h: &mut impl Host, this: u32) -> u32 {
        let set: u32 = h.read(this.wrapping_add(0x10));
        if set == 0 {
            return 7;
        }
        let ctx: u32 = h.read(this.wrapping_add(8));
        if ctx == 0 {
            return 0;
        }
        let class = h.rebase(ANIM_SET_CLASS);
        let mut e: u32 = h.read(ctx);
        loop {
            if e == 0 {
                return 0;
            }
            if h.read::<u32>(e.wrapping_add(8)) == class {
                break;
            }
            e = h.read(e);
        }
        if h.read::<u32>(this.wrapping_add(0x5c)) == set {
            let saved: u32 = h.read(this.wrapping_add(0x40));
            h.write(this.wrapping_add(0x40), set);
            h.call_sym(
                ANIM_EVALUATE,
                Conv::Thiscall(this.wrapping_add(0x48)),
                &[this.wrapping_add(0x38), 0, 0],
            );
            h.write(this.wrapping_add(0x40), saved);
            return 0;
        }
        h.call_sym(
            ANIM_BIND,
            Conv::Cdecl,
            &[
                this.wrapping_add(0x48),
                this.wrapping_add(0x38),
                set,
                e.wrapping_add(0xc),
            ],
        )
    }

    /// `0x00543280` (thiscall `(index, target)`): slot `index` (0x4c-byte
    /// entries at `+0x18`, count `+0x14`) bound to `target`: 3 past the
    /// count, 1 for no target, `0x00543150`'s error; else the set's
    /// target and weight (`+0x1c`), [`animatable_bind`], `+0x40` =
    /// `+0x10`, 0.
    pub fn animatable_set_slot(h: &mut impl Host, this: u32, index: u32, target: u32) -> u32 {
        let n: u32 = h.read(this.wrapping_add(0x14));
        if index >= n & 0x7fff_ffff {
            return 3;
        }
        if target == 0 {
            return 1;
        }
        let base: u32 = h.read(this.wrapping_add(0x18));
        let e = index.wrapping_mul(0x4c).wrapping_add(base);
        let r = h.call_sym(ANIM_SLOT_CHECK, Conv::Thiscall(e), &[target]);
        if r != 0 {
            return r;
        }
        let set = this.wrapping_add(0x1c);
        h.call_sym(
            ANIM_SET_TARGET,
            Conv::Thiscall(set),
            &[index, e.wrapping_add(0x38)],
        );
        let w: u32 = h.read(e.wrapping_add(4));
        h.call_sym(
            ANIM_SET_WEIGHT,
            Conv::Thiscall(set),
            &[index, x87::store(x87::load(w))],
        );
        animatable_bind(h, this);
        let s: u32 = h.read(this.wrapping_add(0x10));
        h.write(this.wrapping_add(0x40), s);
        0
    }

    /// `0x00540c10` `(p)`: each animatable component (`+0x30` less 4)
    /// gets its owner (`0x0044db10`), is bound when it has a set, and a
    /// one-entry target list's target (`0x00543570` finding none) goes
    /// into slot 0 ([`animatable_set_slot`]); 0.
    pub fn fix_animatables(h: &mut impl Host, p: u32) -> u32 {
        small::set_owner_all(h, p);
        let (mut c, mut n, stride) = records(h, p, 4);
        while n != 0 {
            if h.read::<u32>(c.wrapping_add(0x10)) != 0 {
                animatable_bind(h, c);
            }
            let count: u32 = h.read(c.wrapping_add(0x14));
            let target = if count & 0x7fff_ffff == 1 {
                let list: u32 = h.read(c.wrapping_add(0x18));
                h.read(list)
            } else {
                0
            };
            if target != 0 && h.call_sym(ANIM_SLOT_FIND, Conv::Thiscall(c), &[target]) == 0 {
                animatable_set_slot(h, c, 0, target);
            }
            c = c.wrapping_add(stride);
            n = n.wrapping_sub(1);
        }
        0
    }
}
