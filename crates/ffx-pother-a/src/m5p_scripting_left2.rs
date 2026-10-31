//! The leftovers of list `phyre-scripting` (M5 stage 1, round "left2"):
//! the script `require`-style loader handler, a scripted component's
//! reload, the dotted member-path check, and the cloth mesh's instance
//! creation the list holds by neighbourhood.
//!
//! The `PString` the originals keep in their frame (or in an argument
//! slot) and hand to callees by address is a word the caller provides
//! (a `StackFrame` in the shims). Its bit 0 marks a string the holder does
//! not own; the pointer is the word less that bit.

use ffx_graphic::host::{Conv, Host};

/// `PhyreFree` (cdecl).
pub const FREE: u32 = 0x0042_fb20;
/// `PhyreMalloc(size, align)` (cdecl).
pub const MALLOC: u32 = 0x0042_fb80;
/// `memcpy` (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;
/// `strchr`'s import slot (cdecl).
pub const STRCHR_SLOT: u32 = 0x00b0_c3c0;
/// `PClassDescriptor::findMember(name)` (thiscall).
pub const FIND_MEMBER: u32 = 0x0043_5ca0;

/// `lua_tolstring(L, i, len)` (the Lua API below is all cdecl).
pub const LUA_TOLSTRING: u32 = 0x0094_bfc0;
/// `luaL_checklstring(L, i, len)`.
pub const LUAL_CHECKLSTRING: u32 = 0x0094_c980;
/// `lua_settop(L, i)`.
pub const LUA_SETTOP: u32 = 0x0094_bdc0;
/// `lua_getfield(L, i, name)`.
pub const LUA_GETFIELD: u32 = 0x0094_b030;
/// `lua_pushnil(L)`.
pub const LUA_PUSHNIL: u32 = 0x0094_b730;
/// `lua_setfield(L, i, name)`.
pub const LUA_SETFIELD: u32 = 0x0094_bc30;
/// `lua_setglobal(L, name)`.
pub const LUA_SETGLOBAL: u32 = 0x0094_bc80;
/// `lua_getglobal(L, name)`.
pub const LUA_GETGLOBAL: u32 = 0x0094_b080;
/// `lua_pcallk(L, nargs, nresults, errfunc, ctx, k)`.
pub const LUA_PCALLK: u32 = 0x0094_b4d0;
/// `lua_type(L, i)`.
pub const LUA_TYPE: u32 = 0x0094_c1e0;
/// `lua_toboolean(L, i)`.
pub const LUA_TOBOOLEAN: u32 = 0x0094_bf00;
/// `LUA_REGISTRYINDEX`.
pub const LUA_REGISTRYINDEX: u32 = 0xfff0_b9d8;

/// `0x005d67b0`'s callees: the module path builder (`(&string, name)`,
/// cdecl), the loaded-module check `(L, name, keep)`, the source loader
/// `(name)`, `GetPScriptFromContext(L)` (all cdecl).
pub const REQUIRE_CALLS: [u32; 4] = [0x005d_6650, 0x005d_5a90, 0x005d_57b0, 0x005d_5780];
/// The registry table the loaded modules go in (its name).
pub const LOADED_TABLE: u32 = 0x00b3_9140;
/// The script load callbacks (stdcall `(script, source, 0, user)` and
/// `(L, source, name, user)`) and their user words.
pub const LOAD_CALLBACKS: [u32; 4] = [0x00cb_e6b4, 0x00cb_e6b8, 0x00cb_e6c8, 0x00cb_e6d4];

/// `reloadScript`'s callees: `PSharray<PString>::indexOf(&name, start)`
/// (thiscall), the script function push `(L, obj, size)` and the object
/// push `(L, obj, class)` (cdecl); its strings and the class pushed.
pub const RELOAD_CALLS: [u32; 3] = [0x0054_dcd0, 0x0043_6ed0, 0x0043_6e80];
/// The name `reloadScript` looks up (a `PString` global) and the class it
/// pushes the strings as.
pub const RELOAD_DATA: [u32; 4] = [0x00c2_1d04, 0x00c9_1048, 0x00b2_d25c, 0x00b2_d298];

/// The cloth mesh's instance: `PCluster::allocateInstanceList` (thiscall,
/// `(count, class, 0)`), the instance class, the instance constructor
/// (thiscall `(mesh)`).
pub const CLOTH_INSTANCE: [u32; 3] = [0x0044_0b70, 0x00cd_efb0, 0x006e_0860];

/// More of the Lua API (cdecl): `luaL_newmetatable(L, name)`,
/// `luaL_setfuncs(L, list, nup)`, `lua_gettop(L)`, `lua_pushlstring(L, s,
/// len)`, `lua_pushcclosure(L, f, n)`, `lua_rawset(L, i)` (`0x0094ba30`),
/// `lua_newuserdata(L, size)`, `lua_setmetatable(L, i)`,
/// `lua_createtable(L, a, h)`, `lua_pushvalue(L, i)`, `luaL_requiref(L,
/// name, open, global)` (`0x0094d870`), `luaL_getsubtable(L, i, name)`.
pub const LUA_MORE: [u32; 12] = [
    0x0094_d550,
    0x0094_d930,
    0x0094_b160,
    0x0094_b6e0,
    0x0094_b5c0,
    0x0094_ba30,
    0x0094_b450,
    0x0094_bce0,
    0x0094_adc0,
    0x0094_b850,
    0x0094_d870,
    0x0094_cef0,
];

/// A metatable's entries: the key (a string and its length) and the C
/// function stored under it.
pub type Entries = [(u32, u32, u32)];

/// `RegisterTypes`' metatable names, the function list both set
/// (`luaL_setfuncs`) and the namespace walk's push (`0x00435940`).
pub const TYPES_DATA: [u32; 4] = [0x00b0_e548, 0x00b0_e23c, 0x00b0_e540, 0x0043_5940];
/// `RegisterTypes`' first metatable (the class table type).
pub const TYPES_CLASS: [(u32, u32, u32); 2] = [
    (0x00b0_e52c, 7, 0x0043_6c80),
    (0x00b0_e550, 10, 0x0043_6d90),
];
/// `RegisterTypes`' `PhyreObject` metatable.
pub const TYPES_OBJECT: [(u32, u32, u32); 3] = [
    (0x00b0_e52c, 7, 0x0043_6430),
    (0x00b0_e550, 10, 0x0043_6860),
    (0x00b0_e55c, 4, 0x0043_6aa0),
];
/// `RegisterVectorMatrixTypes`, called last by `RegisterTypes` (cdecl).
pub const REGISTER_VECTOR_MATRIX: u32 = 0x0057_21b0;
/// `PNamespace::GetGlobalNamespace` (cdecl).
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;

/// `RegisterVectorMatrixTypes`' metatable and global names (vector,
/// matrix) and the vector constructor it registers as a global.
pub const VM_DATA: [u32; 3] = [0x00b3_07b4, 0x00b3_07bc, 0x0057_0f80];
/// The vector metatable.
pub const VM_VECTOR: [(u32, u32, u32); 8] = [
    (0x00b0_e52c, 7, 0x0057_2e70),
    (0x00b0_e550, 10, 0x0057_36c0),
    (0x00b3_0998, 5, 0x0057_2450),
    (0x00b3_09a0, 5, 0x0057_3930),
    (0x00b3_09a8, 5, 0x0057_3480),
    (0x00b3_09b0, 5, 0x0057_2810),
    (0x00b3_09b8, 5, 0x0057_3b70),
    (0x00b0_e55c, 4, 0x0057_2c80),
];
/// The matrix metatable.
pub const VM_MATRIX: [(u32, u32, u32); 5] = [
    (0x00b0_e52c, 7, 0x0057_1770),
    (0x00b0_e550, 10, 0x0057_1d60),
    (0x00b3_09c0, 6, 0x0057_0e30),
    (0x00b3_09a8, 5, 0x0057_1a60),
    (0x00b0_e55c, 4, 0x0057_1600),
];

/// `Require`'s state: the last state (`0x00cbe6b0`), the nesting depth
/// and its maximum (`0x00cbe6ac`, `0x00cbe6a8`), whether sources may be
/// loaded (`0x00cbe6bc`), the third load callback and its user word
/// (`0x00cbe6c4`, `0x00cbe6d0`).
pub const REQUIRE_STATE: [u32; 6] = [
    0x00cb_e6b0,
    0x00cb_e6ac,
    0x00cb_e6a8,
    0x00cb_e6bc,
    0x00cb_e6c4,
    0x00cb_e6d0,
];
/// `Require`'s callees: the library opener lookup (`0x005d5710`) and the
/// chunk load `(L, source)` (`0x00436750`); its messages (too deep, a
/// failed call, a stack not back at 1).
pub const REQUIRE_MORE: [u32; 5] = [
    0x005d_5710,
    0x0043_6750,
    0x00b3_9158,
    0x00b3_91a4,
    0x00b3_91d0,
];
/// How deep `Require` nests before it refuses.
pub const REQUIRE_MAX_DEPTH: u32 = 0x14;

/// The index handlers' Lua API (cdecl): `lua_touserdata(L, i)`,
/// `lua_getmetatable(L, i)`, `lua_rawequal(L, a, b)`, `lua_remove(L, i)`
/// (`0x0094bb70`), `lua_pushlightuserdata(L, p)`, `lua_pushinteger(L,
/// n)`, `lua_settable(L, i)` (`0x0094bd90`), `luaL_error(L, fmt, ...)`
/// (varargs; it never returns in the game: it unwinds the Lua call past
/// the shim, as past the original's frame, so the ports return what the
/// original's unreachable tail returns).
pub const LUA_INDEX: [u32; 8] = [
    0x0094_c1b0,
    0x0094_b0d0,
    0x0094_b8b0,
    0x0094_bb70,
    0x0094_b6c0,
    0x0094_b690,
    0x0094_bd90,
    0x0094_cd50,
];
/// `PClassDescriptor`'s own descriptor (`PClassDescriptorForClassDescriptor`).
pub const CLASS_DESCRIPTOR: u32 = 0x00c9_0760;
/// The named-entry lookups of a descriptor's lists `+0x54`, `+0x4c`,
/// `+0x5c` (thiscall `(name)`), `inheritsFrom(other)` (thiscall), the
/// list count `0x004361d0(list)` (cdecl), the descriptor-property get
/// `0x00437440(L)` (thiscall).
pub const INDEX_CALLS: [u32; 6] = [
    0x0043_5b80,
    0x0043_5c10,
    0x0043_5d30,
    0x0043_1420,
    0x0043_61d0,
    0x0043_7440,
];
/// The C closures the index handler pushes for a method (`+0x54`) and a
/// function (`+0x4c`) entry.
pub const INDEX_CLOSURES: [u32; 2] = [0x0043_7160, 0x0043_71e0];
/// The error location globals: file, line, function.
pub const ERROR_AT: [u32; 3] = [0x00c9_6a98, 0x00c9_6a9c, 0x00c9_6aa0];
/// The handlers' source file and function names (index, new index).
pub const ERROR_NAMES: [u32; 3] = [0x00b0_e268, 0x00b0_e248, 0x00b0_e3f4];
/// The index handler's messages: not an object, a write-only member, not
/// a member (a class with lists, one without), not a descriptor member.
pub const INDEX_MESSAGES: [u32; 5] = [
    0x00b0_e288,
    0x00b0_e2bc,
    0x00b0_e2dc,
    0x00b0_e310,
    0x00b0_e3a0,
];
/// The new-index handler's messages: not an object, not a member, not a
/// descriptor member, read-only (bit 3), constant (bit 4).
pub const NEW_INDEX_MESSAGES: [u32; 5] = [
    0x00b0_e418,
    0x00b0_e44c,
    0x00b0_e470,
    0x00b0_e2bc,
    0x00b0_e4b0,
];

/// `GetScriptFunctionNames`' callees: `lua_rawgeti(L, i, n)`, `lua_next(L,
/// i)`, `lua_iscfunction(L, i)`, the names' release `(data, n)`, the
/// array's push `(block, &string)` (thiscall), `qsort`'s import slot and
/// the comparison it passes; the call error message.
pub const NAMES_CALLS: [u32; 8] = [
    0x0094_b950,
    0x0094_b490,
    0x0094_bf30,
    0x0043_4360,
    0x005d_6b00,
    0x00b0_c3ac,
    0x005d_6760,
    0x00b3_92d4,
];

/// `CopyValuesFromContext`'s Lua API (cdecl): `lua_checkstack(L, n)`,
/// `lua_toboolean`'s push `lua_pushboolean(L, b)`, `lua_tonumberx(L, i,
/// isnum)` (the number in `ST0`), `lua_pushnumber(L, float)` (a `float`
/// word: this Lua's numbers are `float`s), `lua_pushstring(L, s)`,
/// `lua_typename(L, t)`.
pub const LUA_COPY: [u32; 6] = [
    0x0094_ac00,
    0x0094_b5a0,
    0x0094_c040,
    0x0094_b750,
    0x0094_b770,
    0x0094_c210,
];
/// The userdata kinds `CopyValuesFromContext` knows: the vector and
/// matrix checks `(L, i)` and their pushes `(L, value)` (cdecl), then the
/// messages for a value it cannot copy and an unknown userdata.
pub const COPY_USERDATA: [u32; 6] = [
    0x0057_0dc0,
    0x0057_0d50,
    0x0057_2150,
    0x0057_1f50,
    0x00b3_f594,
    0x00b3_f5d4,
];
/// `PCallerScript`'s state pool (a pointer: `+0x28` capacity, `+0x2c`
/// slots of 20 bytes, `+0x30` in use) and a slot's thread (`0x00618130`,
/// thiscall on the slot).
pub const CALLER_POOL: [u32; 2] = [0x00cb_d9b8, 0x0061_8130];
/// `CopyValuesFromContext` (cdecl), as `0x00619e90` calls it.
pub const COPY_VALUES: u32 = 0x0061_96e0;
/// The call error message (`Phyre_PhyrePrintf(3, ..., code, text)`).
pub const CALL_ERROR: u32 = 0x00b2_d298;

/// The ring buffer's callees: its lock and unlock (thiscall on the
/// buffer), the full check `(size)` (thiscall, `al`), `PhyreSleep(ms)`
/// (cdecl); the frame counter a block's time stamp is compared with.
pub const RING_CALLS: [u32; 4] = [0x0043_1920, 0x0043_1a80, 0x005c_e9d0, 0x0043_0030];
/// The frame counter (`0x00cbd9c0`).
pub const RING_FRAME: u32 = 0x00cb_d9c0;
/// The ring buffer's messages: the wait report (every 1000 tries), then
/// the five lines of the give-up report.
pub const RING_MESSAGES: [u32; 6] = [
    0x00b3_8988,
    0x00b3_86f8,
    0x00b3_8760,
    0x00b3_8788,
    0x00b3_87b4,
    0x00b3_87e4,
];
/// How many waits the ring buffer allows before it reports and crashes
/// (on purpose, a write to address 0).
pub const RING_MAX_WAITS: u32 = 10_000;

/// `0x00543150`'s data and callees: the offset it negates, the
/// controller's `setTarget`, the clip's time range, `setTimeController`
/// (thiscall).
pub const CLIP_DATA: [u32; 4] = [0x00ca_b918, 0x0052_6590, 0x0051_6a20, 0x0052_65f0];

/// The `Matrix(...)` constructor's callees: `luaL_checknumber(L, i)` (the
/// number in `ST0`, cdecl), `PMatrix4::identity(out)` (cdecl),
/// `PMatrix4::copy(m)` (thiscall), `PMatrix4::lookAt(eye, target, up)`
/// (thiscall).
pub const MATRIX_NEW: [u32; 4] = [0x0094_c9e0, 0x0045_4cf0, 0x0045_3340, 0x0045_1c30];

/// The matrix `__index`'s callees: the matrix userdata test `(L, i)` and
/// the new vector `(L)` (cdecl).
pub const MATRIX_INDEX: [u32; 2] = [0x0057_0d50, 0x0057_1070];
/// The matrix methods `__index` gives (name, C function).
pub const MATRIX_INDEX_METHODS: [(u32, u32); 2] =
    [(0x00b3_0910, 0x0057_19a0), (0x00b3_091c, 0x0057_1e00)];
/// The `Matrix` table's functions `__index` gives (name, C function).
pub const MATRIX_INDEX_STATICS: [(u32, u32); 4] = [
    (0x00b2_5008, 0x0057_13e0),
    (0x00b2_5020, 0x0057_1480),
    (0x00b2_5014, 0x0057_1540),
    (0x00b3_0928, 0x0057_1200),
];

/// The matrix `__mul`'s callees: the new matrix userdata `(L)` (cdecl),
/// `PMatrix4::rowTimes(out, v)` (thiscall, ret 8), the scale
/// `(out, float)` (thiscall, ret 8), then the metatable names `"Matrix"`
/// and `"Vector"`.
pub const MATRIX_MUL: [u32; 5] = [
    0x0057_1030,
    0x004c_9c20,
    0x0057_0c00,
    0x00b3_07bc,
    0x00b3_07b4,
];

/// The vector `__index`'s callees and data: the vector check `(L, i)`,
/// the swizzle index `(char)` (cdecl), `lua_pushnumber(L, float)`, then
/// the error's source file, function name and message (`"Trying to get
/// member '%s' on ..."`) and its line (542).
pub const VECTOR_INDEX: [u32; 7] = [
    0x0057_0dc0,
    0x0057_10b0,
    0x0094_b750,
    0x00b3_07ec,
    0x00b3_07c4,
    0x00b3_0814,
    0x21e,
];
/// The `Vector` methods `__index` gives (name, C function).
pub const VECTOR_INDEX_METHODS: [(u32, u32); 9] = [
    (0x00b3_0850, 0x0057_2a40),
    (0x00b3_0858, 0x0057_2b70),
    (0x00b3_0860, 0x0057_2690),
    (0x00b3_0868, 0x0057_3160),
    (0x00b3_0870, 0x0057_3230),
    (0x00b3_0878, 0x0057_3300),
    (0x00b3_0884, 0x0057_33c0),
    (0x00b3_0890, 0x0057_3760),
    (0x00b3_089c, 0x0057_3860),
];

/// The occluder quad's `InterlockedCompareExchange` import slot (stdcall)
/// and the double 0.5 it scales the midpoint by.
pub const OCCLUDER_QUAD: [u32; 2] = [0x00b0_c12c, 0x00b9_2220];

/// The ports over game memory.
pub mod bridge {
    use super::CLIP_DATA;
    use super::MATRIX_MUL;
    use super::MATRIX_NEW;
    use super::OCCLUDER_QUAD;
    use super::{
        CALL_ERROR, CALLER_POOL, CLASS_DESCRIPTOR, CLOTH_INSTANCE, COPY_USERDATA, COPY_VALUES,
        Conv, ERROR_AT, ERROR_NAMES, FIND_MEMBER, FREE, GLOBAL_NAMESPACE, Host, INDEX_CALLS,
        INDEX_CLOSURES, INDEX_MESSAGES, LOAD_CALLBACKS, LOADED_TABLE, LUA_COPY, LUA_GETFIELD,
        LUA_GETGLOBAL, LUA_INDEX, LUA_MORE, LUA_PCALLK, LUA_PUSHNIL, LUA_REGISTRYINDEX,
        LUA_SETFIELD, LUA_SETGLOBAL, LUA_SETTOP, LUA_TOBOOLEAN, LUA_TOLSTRING, LUA_TYPE,
        LUAL_CHECKLSTRING, MALLOC, MEMCPY, NAMES_CALLS, NEW_INDEX_MESSAGES, PRINTF,
        REGISTER_VECTOR_MATRIX, RELOAD_CALLS, RELOAD_DATA, REQUIRE_CALLS, REQUIRE_MAX_DEPTH,
        REQUIRE_MORE, REQUIRE_STATE, RING_CALLS, RING_FRAME, RING_MAX_WAITS, RING_MESSAGES,
        STRCHR_SLOT, TYPES_CLASS, TYPES_DATA, TYPES_OBJECT, VM_DATA, VM_MATRIX, VM_VECTOR,
    };
    use super::{MATRIX_INDEX, MATRIX_INDEX_METHODS, MATRIX_INDEX_STATICS};
    use super::{VECTOR_INDEX, VECTOR_INDEX_METHODS};
    use ffx_bridge::Mem;

    /// A `PString` word's pointer (bit 0 cleared).
    fn text(word: u32) -> u32 {
        word.wrapping_sub(word & 1)
    }

    /// `PhyreFree` of a `PString` word the holder owns (bit 0 clear).
    fn release(h: &mut impl Host, word: u32) {
        if word & 1 == 0 {
            h.call_sym(FREE, Conv::Cdecl, &[word]);
        }
    }

    /// The cloth mesh's `createInstance(cluster, existing)` (`0x006e1ec0`,
    /// thiscall on the mesh, `ret 8`): `existing` when given; else a
    /// one-instance list of the instance class in the cluster and its
    /// instance (`+0x34`) constructed for the mesh (the constructor's
    /// `eax`); 0 when either is missing.
    pub fn cloth_create_instance(h: &mut impl Host, mesh: u32, cluster: u32, existing: u32) -> u32 {
        if existing != 0 {
            return existing;
        }
        let class = h.rebase(CLOTH_INSTANCE[1]);
        let list = h.call_sym(CLOTH_INSTANCE[0], Conv::Thiscall(cluster), &[1, class, 0]);
        if list == 0 {
            return 0;
        }
        let inst: u32 = h.read(list + 0x34);
        if inst == 0 {
            return 0;
        }
        h.call_sym(CLOTH_INSTANCE[2], Conv::Thiscall(inst), &[mesh])
    }

    /// The script `require` handler (`0x005d67b0`, cdecl `(L)`, 0
    /// results): the module name (`luaL_checklstring(L, 1)`), the stack
    /// back to its argument; the path built into `string` (a `PString`
    /// word, the original's frame); when the module is not loaded yet
    /// (`0x005d5a90(L, path, 0)`), `registry[_LOADED][path] = nil`
    /// (as the original's pushes leave it) and the global of that name
    /// cleared, then the source loaded (`0x005d57b0`) and, when there is
    /// one, the two load callbacks told (stdcall). The string released
    /// unless borrowed. 0.
    pub fn require(h: &mut impl Host, l: u32, string: u32) -> u32 {
        let name = h.call_sym(LUAL_CHECKLSTRING, Conv::Cdecl, &[l, 1, 0]);
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        if name == 0 {
            return 0;
        }
        h.write(string, 0u32);
        h.call_sym(REQUIRE_CALLS[0], Conv::Cdecl, &[string, name]);
        let word: u32 = h.read(string);
        let path = text(word);
        let loaded = h.call_sym(REQUIRE_CALLS[1], Conv::Cdecl, &[l, path, 0]);
        if loaded & 0xff != 0 {
            let table = h.rebase(LOADED_TABLE);
            h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, table]);
            h.call_sym(LUA_PUSHNIL, Conv::Cdecl, &[l]);
            h.call_sym(LUA_SETFIELD, Conv::Cdecl, &[l, 0xffff_fffe, path]);
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
            h.call_sym(LUA_PUSHNIL, Conv::Cdecl, &[l]);
            h.call_sym(LUA_SETGLOBAL, Conv::Cdecl, &[l, path]);
            let source = h.call_sym(REQUIRE_CALLS[2], Conv::Cdecl, &[path]);
            if source != 0 {
                let first: u32 = h.read(h.rebase(LOAD_CALLBACKS[0]));
                if first != 0 {
                    let script = h.call_sym(REQUIRE_CALLS[3], Conv::Cdecl, &[l]);
                    if script != 0 {
                        let user: u32 = h.read(h.rebase(LOAD_CALLBACKS[1]));
                        let f: u32 = h.read(h.rebase(LOAD_CALLBACKS[0]));
                        h.call(f, Conv::Stdcall, &[script, source, 0, user]);
                    }
                }
                let second: u32 = h.read(h.rebase(LOAD_CALLBACKS[2]));
                if second != 0 {
                    let user: u32 = h.read(h.rebase(LOAD_CALLBACKS[3]));
                    h.call(second, Conv::Stdcall, &[l, source, path, user]);
                }
            }
        }
        release(h, word);
        0
    }

    /// `PScriptableComponent::reloadScript(component)` (`0x005532a0`,
    /// thiscall, `ret 4`): nothing without a component; a message with
    /// both script names; without a state (`+0x1c`) nothing more. When
    /// the name list (`[+0x14] + 8`) holds the reload name: the
    /// component's name as a path in `string` (a `PString` word, the
    /// original's argument slot), the reload function, this, the
    /// component's name and the path pushed and called (`lua_pcallk(L, 3,
    /// 1)`): an error is printed; a `true` result skips the restart. Else
    /// (or without that result) the virtual `+8` then `+4(0, 0, 0)`
    /// restart the script.
    pub fn reload_script(h: &mut impl Host, this: u32, component: u32, string: u32) {
        if component == 0 {
            return;
        }
        let names: u32 = h.read(this + 0x14);
        let own = text(h.read(names + 0x1c));
        let name_at = component + 0x1c;
        let theirs = text(h.read(name_at));
        let msg = h.rebase(RELOAD_DATA[2]);
        h.call_sym(PRINTF, Conv::Cdecl, &[0, msg, theirs, own]);
        // the state is read again for every call, as the original does
        let state = this + 0x1c;
        if h.read::<u32>(state) == 0 {
            return;
        }
        let reload = h.rebase(RELOAD_DATA[0]);
        let at = h.call_sym(RELOAD_CALLS[0], Conv::Thiscall(names + 8), &[reload, 0]);
        let mut handled = false;
        if at.cast_signed() >= 0 {
            h.write(string, 0u32);
            let theirs = text(h.read(name_at));
            h.call_sym(super::REQUIRE_CALLS[0], Conv::Cdecl, &[string, theirs]);
            let global: u32 = h.read(reload);
            let l = h.read(state);
            h.call_sym(LUA_GETGLOBAL, Conv::Cdecl, &[l, global]);
            let size: u32 = h.read(this + 0xc);
            let l = h.read(state);
            h.call_sym(RELOAD_CALLS[1], Conv::Cdecl, &[l, this + 4, size]);
            let class = h.rebase(RELOAD_DATA[1]);
            let l = h.read(state);
            h.call_sym(RELOAD_CALLS[2], Conv::Cdecl, &[l, name_at, class]);
            let l = h.read(state);
            h.call_sym(RELOAD_CALLS[2], Conv::Cdecl, &[l, string, class]);
            let l = h.read(state);
            let err = h.call_sym(LUA_PCALLK, Conv::Cdecl, &[l, 3, 1, 0, 0, 0]);
            if err != 0 {
                let l = h.read(state);
                let text = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, u32::MAX, 0]);
                let msg = h.rebase(RELOAD_DATA[3]);
                h.call_sym(PRINTF, Conv::Cdecl, &[3, msg, err, text]);
            } else {
                let l = h.read(state);
                if h.call_sym(LUA_TYPE, Conv::Cdecl, &[l, u32::MAX]) == 1 {
                    let l = h.read(state);
                    handled = h.call_sym(LUA_TOBOOLEAN, Conv::Cdecl, &[l, u32::MAX]) != 0;
                }
            }
            let l = h.read(state);
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
            let word: u32 = h.read(string);
            release(h, word);
        }
        if !handled {
            h.call_virtual(this, 2, &[]);
            h.call_virtual(this, 1, &[0, 0, 0]);
        }
    }

    /// `0x0050be90` (cdecl `(class, path)`): whether the dotted member path
    /// resolves from `class`: each part (copied into a new string, the
    /// last one whole) looked up as a member (`findMember`), the next
    /// class its type's `vt[1]`; each copy freed. `al` 1 when every part
    /// was found and the path did not end in a dot; 0 for no path, an
    /// empty one or a missing member. A failed copy looks up the name 0.
    pub fn has_member_path(h: &mut impl Host, class: u32, path: u32) -> u8 {
        if path == 0 {
            return 0;
        }
        let mut class = class;
        let mut at = path;
        let mut member = 0;
        loop {
            if h.read::<u8>(at) == 0 {
                break;
            }
            let strchr: u32 = h.read(h.rebase(STRCHR_SLOT));
            let dot = h.call(strchr, Conv::Cdecl, &[at, u32::from(b'.')]);
            let mut copy = 0;
            let next = if dot == 0 {
                let mut len = 0u32;
                while h.read::<u8>(at.wrapping_add(len)) != 0 {
                    len += 1;
                }
                let block = h.call_sym(MALLOC, Conv::Cdecl, &[len + 1, 2]);
                if block != 0 {
                    h.call_sym(MEMCPY, Conv::Cdecl, &[block, at, len + 1]);
                    h.call_sym(FREE, Conv::Cdecl, &[0]);
                    copy = block;
                }
                0
            } else {
                if dot >= at {
                    let len = dot - at;
                    let block = h.call_sym(MALLOC, Conv::Cdecl, &[len + 1, 2]);
                    if block != 0 {
                        h.call_sym(MEMCPY, Conv::Cdecl, &[block, at, len + 1]);
                        h.write(block.wrapping_add(len), 0u8);
                        h.call_sym(FREE, Conv::Cdecl, &[0]);
                        copy = block;
                    }
                }
                dot + 1
            };
            member = h.call_sym(FIND_MEMBER, Conv::Thiscall(class), &[text(copy)]);
            if member == 0 {
                release(h, copy);
                return 0;
            }
            let ty: u32 = h.read(member + 0x18);
            class = h.call_virtual(ty, 1, &[]);
            release(h, copy);
            at = next;
            if at == 0 {
                break;
            }
        }
        u8::from(member != 0 && at == 0)
    }

    /// A metatable's entries set by `lua_rawset` on the table at `top`:
    /// each key pushed with its length, its C function as a closure
    /// without upvalues.
    fn fill_table(h: &mut impl Host, l: u32, top: u32, entries: &super::Entries) {
        for &(key, len, f) in entries {
            let key = h.rebase(key);
            h.call_sym(LUA_MORE[3], Conv::Cdecl, &[l, key, len]);
            let f = h.rebase(f);
            h.call_sym(LUA_MORE[4], Conv::Cdecl, &[l, f, 0]);
            h.call_sym(LUA_MORE[5], Conv::Cdecl, &[l, top]);
        }
    }

    /// `Phyre::PScripting::RegisterTypes(L, ns, name)` (`0x00436f90`,
    /// cdecl): the class-table metatable (new: its functions, two
    /// entries, a userdata of it as the global `name`), the stack back;
    /// every class of `ns` (unless it is the global namespace) with a
    /// method or property list pushed as an object (`0x00435940`); the
    /// `PhyreObject` metatable (new: its functions, three entries); then
    /// `RegisterVectorMatrixTypes(L)`, whose `eax` it returns.
    pub fn register_types(h: &mut impl Host, l: u32, ns: u32, name: u32) -> u32 {
        let meta = h.rebase(TYPES_DATA[0]);
        let funcs = h.rebase(TYPES_DATA[2]);
        if h.call_sym(LUA_MORE[0], Conv::Cdecl, &[l, meta]) != 0 {
            h.call_sym(LUA_MORE[1], Conv::Cdecl, &[l, funcs, 0]);
            let top = h.call_sym(LUA_MORE[2], Conv::Cdecl, &[l]);
            fill_table(h, l, top, &TYPES_CLASS);
            h.call_sym(LUA_MORE[6], Conv::Cdecl, &[l, 0]);
            h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, meta]);
            h.call_sym(LUA_MORE[7], Conv::Cdecl, &[l, 0xffff_fffe]);
            h.call_sym(LUA_SETGLOBAL, Conv::Cdecl, &[l, name]);
        }
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        let global = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        if ns != global {
            let head = ns + 8;
            let mut link: u32 = h.read(head);
            while link != head && link != 0 && link != 0x2c {
                let desc = link - 0x2c;
                let methods = desc + 0x54;
                let props = desc + 0x5c;
                let m: u32 = h.read(methods);
                let p: u32 = h.read(props);
                if (m != methods && m != 0) || (p != props && p != 0) {
                    let top: u32 = h.read(desc + 0x18);
                    h.call_sym(TYPES_DATA[3], Conv::Cdecl, &[l, top, desc]);
                }
                link = h.read(desc + 0x2c);
            }
        }
        let meta = h.rebase(TYPES_DATA[1]);
        if h.call_sym(LUA_MORE[0], Conv::Cdecl, &[l, meta]) != 0 {
            h.call_sym(LUA_MORE[1], Conv::Cdecl, &[l, funcs, 0]);
            let top = h.call_sym(LUA_MORE[2], Conv::Cdecl, &[l]);
            fill_table(h, l, top, &TYPES_OBJECT);
        }
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        h.call_sym(REGISTER_VECTOR_MATRIX, Conv::Cdecl, &[l])
    }

    /// `Phyre::PScripting::RegisterVectorMatrixTypes(L)` (`0x005721b0`,
    /// cdecl): the vector metatable (new: eight entries), the matrix
    /// metatable (new: five entries, and a global table of that name whose
    /// metatable is it), each time the stack back; then the vector
    /// constructor as the global of the vector's name. `lua_setglobal`'s
    /// `eax`.
    pub fn register_vector_matrix(h: &mut impl Host, l: u32) -> u32 {
        let vector = h.rebase(VM_DATA[0]);
        if h.call_sym(LUA_MORE[0], Conv::Cdecl, &[l, vector]) != 0 {
            let top = h.call_sym(LUA_MORE[2], Conv::Cdecl, &[l]);
            fill_table(h, l, top, &VM_VECTOR);
        }
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        let matrix = h.rebase(VM_DATA[1]);
        if h.call_sym(LUA_MORE[0], Conv::Cdecl, &[l, matrix]) != 0 {
            let top = h.call_sym(LUA_MORE[2], Conv::Cdecl, &[l]);
            fill_table(h, l, top, &VM_MATRIX);
            h.call_sym(LUA_MORE[8], Conv::Cdecl, &[l, 0, 0]);
            let table = h.call_sym(LUA_MORE[2], Conv::Cdecl, &[l]);
            h.call_sym(LUA_MORE[9], Conv::Cdecl, &[l, top]);
            h.call_sym(LUA_MORE[7], Conv::Cdecl, &[l, table]);
            h.call_sym(LUA_SETGLOBAL, Conv::Cdecl, &[l, matrix]);
        }
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        let ctor = h.rebase(VM_DATA[2]);
        h.call_sym(LUA_MORE[4], Conv::Cdecl, &[l, ctor, 0]);
        h.call_sym(LUA_SETGLOBAL, Conv::Cdecl, &[l, vector])
    }

    /// `Phyre::PScripting::Require(L)` (`0x005d63a0`, cdecl, 1 result):
    /// the nesting depth counted per state (past 0x14 on the same state:
    /// nil, a message, the depth reset; 1); the module path built into
    /// `string` (a `PString` word, the original's argument slot); unless
    /// loaded already: a library opener (`luaL_requiref`), or, when
    /// sources may load, the source loaded, compiled and called: an error
    /// printed and popped, else `_LOADED[path]` set from the global, the
    /// global pushed and the load callbacks told. A stack not back at one
    /// value gets a message and a nil. 1.
    pub fn require_module(h: &mut impl Host, l: u32, string: u32) -> u32 {
        let name = h.call_sym(LUAL_CHECKLSTRING, Conv::Cdecl, &[l, 1, 0]);
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        let last = h.rebase(REQUIRE_STATE[0]);
        let depth_at = h.rebase(REQUIRE_STATE[1]);
        let max_at = h.rebase(REQUIRE_STATE[2]);
        let mut depth = 0;
        if h.read::<u32>(last) == l {
            depth = h.read(depth_at);
            if depth > REQUIRE_MAX_DEPTH {
                h.call_sym(LUA_PUSHNIL, Conv::Cdecl, &[l]);
                let msg = h.rebase(REQUIRE_MORE[2]);
                h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, name]);
                h.write(depth_at, 0u32);
                return 1;
            }
        } else {
            h.write(last, l);
        }
        let max: u32 = h.read(max_at);
        let depth = depth.wrapping_add(1);
        h.write(depth_at, depth);
        if depth > max {
            h.write(max_at, max.wrapping_add(1));
        }
        if name != 0 {
            h.write(string, 0u32);
            h.call_sym(REQUIRE_CALLS[0], Conv::Cdecl, &[string, name]);
            let word: u32 = h.read(string);
            let path = text(word);
            let loaded = h.call_sym(REQUIRE_CALLS[1], Conv::Cdecl, &[l, path, 1]);
            let already = loaded & 0xff != 0;
            if !already {
                let opener = h.call_sym(REQUIRE_MORE[0], Conv::Cdecl, &[path]);
                if opener != 0 {
                    h.call_sym(LUA_MORE[10], Conv::Cdecl, &[l, path, opener, 1]);
                } else if h.read::<u32>(h.rebase(REQUIRE_STATE[3])) != 0 {
                    load_source(h, l, path);
                }
            }
            release(h, word);
        }
        if h.call_sym(LUA_MORE[2], Conv::Cdecl, &[l]) != 1 {
            let msg = h.rebase(REQUIRE_MORE[4]);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, name]);
            h.call_sym(LUA_PUSHNIL, Conv::Cdecl, &[l]);
        }
        let d: u32 = h.read(depth_at);
        h.write(depth_at, d.wrapping_sub(1));
        1
    }

    /// `Require`'s source path: loaded, compiled, called; the result kept
    /// in `_LOADED` and the callbacks told.
    fn load_source(h: &mut impl Host, l: u32, path: u32) {
        let source = h.call_sym(REQUIRE_CALLS[2], Conv::Cdecl, &[path]);
        if source == 0 {
            return;
        }
        let compiled = h.call_sym(REQUIRE_MORE[1], Conv::Cdecl, &[l, source]) & 0xff != 0;
        if !compiled {
            return;
        }
        let err = h.call_sym(LUA_PCALLK, Conv::Cdecl, &[l, 0, 0, 0, 0, 0]);
        if err != 0 {
            let text = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, u32::MAX, 0]);
            let msg = h.rebase(REQUIRE_MORE[3]);
            h.call_sym(PRINTF, Conv::Cdecl, &[3, msg, err, text]);
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
            return;
        }
        let table = h.rebase(LOADED_TABLE);
        h.call_sym(LUA_MORE[11], Conv::Cdecl, &[l, LUA_REGISTRYINDEX, table]);
        h.call_sym(LUA_GETGLOBAL, Conv::Cdecl, &[l, path]);
        h.call_sym(LUA_SETFIELD, Conv::Cdecl, &[l, 0xffff_fffe, path]);
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        h.call_sym(LUA_GETGLOBAL, Conv::Cdecl, &[l, path]);
        let third: u32 = h.read(h.rebase(REQUIRE_STATE[4]));
        if third != 0 {
            let user: u32 = h.read(h.rebase(REQUIRE_STATE[5]));
            h.call(third, Conv::Stdcall, &[l, source, path, user]);
        }
        if h.read::<u32>(h.rebase(LOAD_CALLBACKS[0])) != 0 {
            let script = h.call_sym(REQUIRE_CALLS[3], Conv::Cdecl, &[l]);
            if script != 0 {
                let user: u32 = h.read(h.rebase(LOAD_CALLBACKS[1]));
                let f: u32 = h.read(h.rebase(LOAD_CALLBACKS[0]));
                h.call(f, Conv::Stdcall, &[script, source, 1, user]);
            }
        }
    }

    /// The error location globals, then `luaL_error(L, fmt, args...)`.
    fn lua_error(h: &mut impl Host, l: u32, line: u32, function: u32, msg: u32, args: &[u32]) {
        let file = h.rebase(ERROR_NAMES[0]);
        let at = h.rebase(ERROR_AT[0]);
        h.write(at, file);
        let at = h.rebase(ERROR_AT[1]);
        h.write(at, line);
        let function = h.rebase(function);
        let at = h.rebase(ERROR_AT[2]);
        h.write(at, function);
        let msg = h.rebase(msg);
        let mut words = vec![l, msg];
        words.extend_from_slice(args);
        h.call_sym(LUA_INDEX[7], Conv::Cdecl, &words);
    }

    /// The `PhyreObject` userdata at stack index 1: its block when the
    /// value is a userdata whose metatable is the registry's `PhyreObject`
    /// (both popped again), else 0.
    fn phyre_object(h: &mut impl Host, l: u32) -> u32 {
        let ud = h.call_sym(LUA_INDEX[0], Conv::Cdecl, &[l, 1]);
        if ud == 0 || h.call_sym(LUA_INDEX[1], Conv::Cdecl, &[l, 1]) == 0 {
            return 0;
        }
        let name = h.rebase(TYPES_DATA[1]);
        h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
        let same = h.call_sym(LUA_INDEX[2], Conv::Cdecl, &[l, u32::MAX, 0xffff_fffe]) != 0;
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffd]);
        if same { ud } else { 0 }
    }

    /// The object a descriptor userdata stands for (`[ud + 4]` less its
    /// `0x18` header, 0 for none).
    fn descriptor_object(h: &mut impl Host, ud: u32) -> u32 {
        let o: u32 = h.read(ud + 4);
        if o == 0 { 0 } else { o.wrapping_sub(0x18) }
    }

    /// A light userdata and the C closure `f` over it (one upvalue).
    fn push_closure(h: &mut impl Host, l: u32, p: u32, f: u32) {
        h.call_sym(LUA_INDEX[4], Conv::Cdecl, &[l, p]);
        let f = h.rebase(f);
        h.call_sym(LUA_MORE[4], Conv::Cdecl, &[l, f, 1]);
    }

    /// `Phyre::PScripting::IndexHandler(L)` (`0x00436430`, cdecl, the
    /// `PhyreObject` metatable's `__index`): for an object and a key, a
    /// function (`+0x4c`) or method (`+0x54`) entry as a closure, a
    /// readable member's value (`vt[3](L, object)`), else the class's own
    /// lookup (`vt[13](object, L, key)`): several results packed in a
    /// table, one or none as they are; on a class descriptor the
    /// descriptor's property or method entries. Errors (Lua errors) for a
    /// non-object, a write-only member or an unknown key; their
    /// unreachable tail returns 0.
    pub fn index_handler(h: &mut impl Host, l: u32) -> u32 {
        let ud = phyre_object(h, l);
        let key = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, 2, 0]);
        let at = ERROR_NAMES[1];
        if ud == 0 {
            lua_error(h, l, 0x164, at, INDEX_MESSAGES[0], &[key]);
            return 0;
        }
        let desc: u32 = h.read(ud);
        let function = h.call_sym(INDEX_CALLS[1], Conv::Thiscall(desc), &[key]);
        if function != 0 {
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
            push_closure(h, l, function, INDEX_CLOSURES[1]);
            return 1;
        }
        let member = h.call_sym(FIND_MEMBER, Conv::Thiscall(desc), &[key]);
        if member != 0 {
            if h.read::<u32>(member + 0x14) & 8 != 0 {
                let name: u32 = h.read(desc + 0x18);
                lua_error(h, l, 0x177, at, INDEX_MESSAGES[1], &[key, name]);
                return 0;
            }
            let object: u32 = h.read(ud + 4);
            h.call_virtual(member, 3, &[l, object]);
            return 1;
        }
        let method = h.call_sym(INDEX_CALLS[0], Conv::Thiscall(desc), &[key]);
        if method != 0 {
            push_closure(h, l, method, INDEX_CLOSURES[0]);
            return 1;
        }
        let object: u32 = h.read(ud + 4);
        let n = h.call_virtual(desc, 13, &[object, l, key]);
        if n.cast_signed() >= 0 {
            if n.cast_signed() > 1 {
                h.call_sym(LUA_MORE[8], Conv::Cdecl, &[l, 0, 0]);
                let first = 0xffff_fffdu32.wrapping_sub(n);
                for i in 1..=n {
                    h.call_sym(LUA_INDEX[5], Conv::Cdecl, &[l, i]);
                    h.call_sym(LUA_MORE[9], Conv::Cdecl, &[l, first.wrapping_add(i)]);
                    h.call_sym(LUA_INDEX[6], Conv::Cdecl, &[l, 0xffff_fffd]);
                }
                return 1;
            }
            return n;
        }
        let class = h.rebase(CLASS_DESCRIPTOR);
        let is_descriptor = h.call_sym(INDEX_CALLS[3], Conv::Thiscall(desc), &[class]) & 0xff != 0;
        if !is_descriptor {
            let a = h.call_sym(INDEX_CALLS[4], Conv::Cdecl, &[desc + 0x54]);
            let b = h.call_sym(INDEX_CALLS[4], Conv::Cdecl, &[desc + 0x4c]);
            let c = h.call_sym(INDEX_CALLS[4], Conv::Cdecl, &[desc + 0x44]);
            let name: u32 = h.read(desc + 0x18);
            if a | b | c != 0 {
                lua_error(h, l, 0x190, at, INDEX_MESSAGES[2], &[key, name]);
            } else {
                lua_error(h, l, 0x192, at, INDEX_MESSAGES[3], &[key, name, name]);
            }
            return 0;
        }
        let o = descriptor_object(h, ud);
        let property = h.call_sym(INDEX_CALLS[2], Conv::Thiscall(o), &[key]);
        if property != 0 {
            h.call_sym(INDEX_CALLS[5], Conv::Thiscall(property), &[l]);
            return 1;
        }
        let method = h.call_sym(INDEX_CALLS[0], Conv::Thiscall(o), &[key]);
        if method != 0 {
            push_closure(h, l, method, INDEX_CLOSURES[0]);
            return 1;
        }
        let name: u32 = h.read(desc + 0x18);
        let oname: u32 = h.read(o + 0x18);
        lua_error(h, l, 0x1a7, at, INDEX_MESSAGES[4], &[key, name, oname]);
        0
    }

    /// `Phyre::PScripting::NewIndexHandler(L)` (`0x00436860`, cdecl, the
    /// `PhyreObject` metatable's `__newindex`): the key taken off the
    /// stack; for an object, a writable member set (`vt[2](L, object)`);
    /// on a class descriptor, the descriptor property set (`vt[5]` for
    /// bit 1 of its flags, else `vt[3]`, `(L, value word)`). 1; Lua errors
    /// for a non-object, an unknown key, a read-only (bit 3) or constant
    /// (bit 4) member, whose unreachable tail returns 0.
    pub fn new_index_handler(h: &mut impl Host, l: u32) -> u32 {
        let key = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, 2, 0]);
        h.call_sym(LUA_INDEX[3], Conv::Cdecl, &[l, 2]);
        let ud = h.call_sym(LUA_INDEX[0], Conv::Cdecl, &[l, 1]);
        let at = ERROR_NAMES[2];
        let valid = ud != 0 && h.call_sym(LUA_INDEX[1], Conv::Cdecl, &[l, 1]) != 0 && {
            let name = h.rebase(TYPES_DATA[1]);
            h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
            let same = h.call_sym(LUA_INDEX[2], Conv::Cdecl, &[l, u32::MAX, 0xffff_fffe]) != 0;
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffd]);
            same
        };
        if !valid {
            lua_error(h, l, 0x1c9, at, NEW_INDEX_MESSAGES[0], &[key]);
            return 0;
        }
        let desc: u32 = h.read(ud);
        let member = h.call_sym(FIND_MEMBER, Conv::Thiscall(desc), &[key]);
        if member != 0 {
            let flags: u32 = h.read(member + 0x14);
            let name: u32 = h.read(desc + 0x18);
            if flags & 8 != 0 {
                lua_error(h, l, 0x1da, at, NEW_INDEX_MESSAGES[3], &[key, name]);
                return 0;
            }
            if flags & 0x10 != 0 {
                lua_error(h, l, 0x1dc, at, NEW_INDEX_MESSAGES[4], &[key, name]);
                return 0;
            }
            let object: u32 = h.read(ud + 4);
            h.call_virtual(member, 2, &[l, object]);
            return 1;
        }
        let class = h.rebase(CLASS_DESCRIPTOR);
        let mut d = desc;
        let mut is_descriptor = false;
        if desc != 0 {
            while d != 0 {
                if d == class {
                    is_descriptor = true;
                    break;
                }
                d = h.read(d + 0x40);
            }
        }
        if !is_descriptor {
            let name: u32 = h.read(desc + 0x18);
            lua_error(h, l, 0x1cf, at, NEW_INDEX_MESSAGES[1], &[key, name]);
            return 0;
        }
        let o = descriptor_object(h, ud);
        let property = h.call_sym(INDEX_CALLS[2], Conv::Thiscall(o), &[key]);
        if property == 0 {
            let name: u32 = h.read(desc + 0x18);
            let oname: u32 = h.read(o + 0x18);
            lua_error(h, l, 0x1d5, at, NEW_INDEX_MESSAGES[2], &[key, name, oname]);
            return 0;
        }
        let target: u32 = h.read(property + 0x14);
        let value: u32 = h.read(property + 0x18);
        let slot = if h.read::<u8>(property + 0x10) & 2 != 0 {
            5
        } else {
            3
        };
        h.call_virtual(target, slot, &[l, value]);
        1
    }

    /// `Phyre::PScripting::GetScriptFunctionNames(script, names)`
    /// (`0x005d5860`, cdecl): the script's chunk run (`lua_pcallk(L, 0,
    /// 0)`; an error is printed, 9); the names array emptied (its strings
    /// released unless external); every string key of the globals table
    /// whose value is a Lua (not C) function copied into a new string (in
    /// `string`, the original's frame) and pushed; the names sorted
    /// (`qsort` with `0x005d6760`). 0.
    pub fn script_function_names(h: &mut impl Host, script: u32, names: u32, string: u32) -> u32 {
        let l: u32 = h.read(script);
        let err = h.call_sym(LUA_PCALLK, Conv::Cdecl, &[l, 0, 0, 0, 0, 0]);
        if err != 0 {
            let l: u32 = h.read(script);
            let text = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, u32::MAX, 0]);
            let l: u32 = h.read(script);
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
            let msg = h.rebase(NAMES_CALLS[7]);
            h.call_sym(PRINTF, Conv::Cdecl, &[3, msg, err, text]);
            return 9;
        }
        let head: u32 = h.read(names);
        if head != 0 {
            let data: u32 = h.read(names + 4);
            if data != 0 && head & 0x8000_0000 == 0 {
                h.call_sym(NAMES_CALLS[3], Conv::Cdecl, &[data, head & 0x7fff_ffff]);
            }
            h.write(names + 4, 0u32);
            h.write(names, 0u32);
        }
        let l: u32 = h.read(script);
        h.call_sym(NAMES_CALLS[0], Conv::Cdecl, &[l, LUA_REGISTRYINDEX, 2]);
        let l: u32 = h.read(script);
        let table = h.call_sym(LUA_MORE[2], Conv::Cdecl, &[l]);
        let l: u32 = h.read(script);
        h.call_sym(LUA_PUSHNIL, Conv::Cdecl, &[l]);
        loop {
            let l: u32 = h.read(script);
            if h.call_sym(NAMES_CALLS[1], Conv::Cdecl, &[l, table]) == 0 {
                break;
            }
            let l: u32 = h.read(script);
            let key = h.call_sym(LUA_TYPE, Conv::Cdecl, &[l, 0xffff_fffe]);
            let l: u32 = h.read(script);
            let value = h.call_sym(LUA_TYPE, Conv::Cdecl, &[l, u32::MAX]);
            if key == 4 && value == 6 {
                let l: u32 = h.read(script);
                if h.call_sym(NAMES_CALLS[2], Conv::Cdecl, &[l, u32::MAX]) == 0 {
                    add_name(h, script, names, string);
                }
            }
            let l: u32 = h.read(script);
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        }
        let l: u32 = h.read(script);
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        let n = h.read::<u32>(names) & 0x7fff_ffff;
        let data: u32 = h.read(names + 4);
        let qsort: u32 = h.read(h.rebase(NAMES_CALLS[5]));
        let compare = h.rebase(NAMES_CALLS[6]);
        h.call(qsort, Conv::Cdecl, &[data, n, 4, compare]);
        0
    }

    /// One function name: the key copied (`PhyreMalloc(len + 1, 2)`; none
    /// for no string or no block) into `string`, pushed with a block of
    /// `count + 1` words, released unless borrowed.
    fn add_name(h: &mut impl Host, script: u32, names: u32, string: u32) {
        let l: u32 = h.read(script);
        let s = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, 0xffff_fffe, 0]);
        let mut copy = 0;
        if s != 0 {
            let mut len = 0u32;
            while h.read::<u8>(s.wrapping_add(len)) != 0 {
                len += 1;
            }
            copy = h.call_sym(MALLOC, Conv::Cdecl, &[len + 1, 2]);
            if copy != 0 {
                h.call_sym(MEMCPY, Conv::Cdecl, &[copy, s, len + 1]);
            }
        }
        h.write(string, copy);
        let count = h.read::<u32>(names) & 0x7fff_ffff;
        let block = h.call_sym(
            MALLOC,
            Conv::Cdecl,
            &[count.wrapping_mul(4).wrapping_add(4), 4],
        );
        h.call_sym(NAMES_CALLS[4], Conv::Thiscall(names), &[block, string]);
        release(h, copy);
    }

    /// `PClassCallableMethodScript::CopyValuesFromContext(from, to, first,
    /// n)` (`0x006196e0`, cdecl): the `n` values of `from` from index
    /// `first` pushed on `to` by type: nil, booleans, light userdata,
    /// numbers (as `float`), strings (then `from` popped once each, as
    /// the original does), vectors, matrices and `PhyreObject`s;
    /// tables, functions and threads are reported (`lua_typename`), and
    /// other userdata too. 1 for a missing state, 5 for one state or no
    /// first index, 9 when `to` cannot grow, else 0.
    pub fn copy_values(h: &mut impl Host, from: u32, to: u32, first: u32, n: u32) -> u32 {
        if from == 0 || to == 0 {
            return 1;
        }
        if from == to || first == 0 {
            return 5;
        }
        if h.call_sym(LUA_COPY[0], Conv::Cdecl, &[to, n]) == 0 {
            return 9;
        }
        let mut i = first;
        for _ in 0..n {
            let ty = h.call_sym(LUA_TYPE, Conv::Cdecl, &[from, i]);
            match ty {
                0 => {
                    h.call_sym(LUA_PUSHNIL, Conv::Cdecl, &[to]);
                }
                1 => {
                    let b = h.call_sym(LUA_TOBOOLEAN, Conv::Cdecl, &[from, i]);
                    h.call_sym(LUA_COPY[1], Conv::Cdecl, &[to, b]);
                }
                2 => {
                    let p = h.call_sym(LUA_INDEX[0], Conv::Cdecl, &[from, i]);
                    h.call_sym(LUA_INDEX[4], Conv::Cdecl, &[to, p]);
                }
                3 => {
                    let v = h.call_sym_st0(LUA_COPY[2], Conv::Cdecl, &[from, i, 0]);
                    let v = ffx_bridge::x87::round_trip(v);
                    h.call_sym(LUA_COPY[3], Conv::Cdecl, &[to, v.to_bits()]);
                }
                4 => {
                    let s = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[from, i, 0]);
                    h.call_sym(LUA_COPY[4], Conv::Cdecl, &[to, s]);
                    h.call_sym(LUA_SETTOP, Conv::Cdecl, &[from, 0xffff_fffe]);
                }
                5 | 6 | 8 => {
                    let name = h.call_sym(LUA_COPY[5], Conv::Cdecl, &[from, ty]);
                    let msg = h.rebase(COPY_USERDATA[4]);
                    h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, name]);
                }
                7 => copy_userdata(h, from, to, i),
                _ => {}
            }
            i = i.wrapping_add(1);
        }
        0
    }

    /// [`copy_values`]' userdata: a vector, a matrix, or a `PhyreObject`
    /// (pushed again as its object and class); anything else reported.
    fn copy_userdata(h: &mut impl Host, from: u32, to: u32, i: u32) {
        let vector = h.call_sym(COPY_USERDATA[0], Conv::Cdecl, &[from, i]);
        let matrix = h.call_sym(COPY_USERDATA[1], Conv::Cdecl, &[from, i]);
        if vector != 0 {
            h.call_sym(COPY_USERDATA[2], Conv::Cdecl, &[to, vector]);
            return;
        }
        if matrix != 0 {
            h.call_sym(COPY_USERDATA[3], Conv::Cdecl, &[to, matrix]);
            return;
        }
        let ud = h.call_sym(LUA_INDEX[0], Conv::Cdecl, &[from, i]);
        let object = h.call_sym(LUA_INDEX[1], Conv::Cdecl, &[from, i]) != 0 && {
            let name = h.rebase(TYPES_DATA[1]);
            h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[from, LUA_REGISTRYINDEX, name]);
            let same = h.call_sym(LUA_INDEX[2], Conv::Cdecl, &[from, u32::MAX, 0xffff_fffe]) != 0;
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[from, 0xffff_fffd]);
            same
        };
        if object && ud != 0 {
            let o: u32 = h.read(ud + 4);
            let class: u32 = h.read(ud);
            h.call_sym(RELOAD_CALLS[2], Conv::Cdecl, &[to, o, class]);
            return;
        }
        let msg = h.rebase(COPY_USERDATA[5]);
        h.call_sym(PRINTF, Conv::Cdecl, &[1, msg]);
    }

    /// `PCallerScript`'s script function call `(L)` (`0x00619e90`,
    /// thiscall on the caller, `ret 4`): the script's function (its name
    /// a `PString` at `[+4] + 0x30`, its chunk at `+0x28`) called with
    /// `L`'s values, in a thread from the state pool when one is free
    /// (the values copied over and the result back) or in `L` itself; a
    /// load or call error is printed (0). The number of results left on
    /// `L`, 0 without a name or a function. The pool slot is given back
    /// when it is the last one taken.
    pub fn caller_call(h: &mut impl Host, this: u32, l: u32) -> u32 {
        let script: u32 = h.read(this + 4);
        let chunk: u32 = h.read(script + 0x28);
        let name = text(h.read(script + 0x30));
        if name == 0 {
            return 0;
        }
        let pool: u32 = h.read(h.rebase(CALLER_POOL[0]));
        let mut slot = 0;
        if pool != 0 {
            let cap = h.read::<u32>(pool + 0x28) & 0x7fff_ffff;
            let used: u32 = h.read(pool + 0x30);
            if used < cap {
                h.write(pool + 0x30, used + 1);
                let slots: u32 = h.read(pool + 0x2c);
                slot = slots.wrapping_add(used.wrapping_mul(20));
            }
        }
        let (owner, thread) = if slot != 0 {
            (pool, h.call_sym(CALLER_POOL[1], Conv::Thiscall(slot), &[]))
        } else {
            (0, l)
        };
        let result = caller_run(h, l, thread, chunk, name);
        if owner != 0 && slot != 0 {
            let used: u32 = h.read(owner + 0x30);
            if used != 0 {
                let slots: u32 = h.read(owner + 0x2c);
                let last = slots.wrapping_add(used.wrapping_mul(20)).wrapping_sub(20);
                if last == slot {
                    h.write(owner + 0x30, used - 1);
                }
            }
        }
        result
    }

    /// [`caller_call`]'s call on `thread`.
    fn caller_run(h: &mut impl Host, l: u32, thread: u32, chunk: u32, name: u32) -> u32 {
        let loaded = h.call_sym(REQUIRE_MORE[1], Conv::Cdecl, &[thread, chunk]) & 0xff != 0;
        if !loaded {
            return 0;
        }
        let msg = h.rebase(CALL_ERROR);
        let err = h.call_sym(LUA_PCALLK, Conv::Cdecl, &[thread, 0, 0, 0, 0, 0]);
        if err != 0 {
            let text = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[thread, u32::MAX, 0]);
            h.call_sym(PRINTF, Conv::Cdecl, &[3, msg, err, text]);
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[thread, 0xffff_fffe]);
            return 0;
        }
        let top = h.call_sym(LUA_MORE[2], Conv::Cdecl, &[l]);
        let thread_top = h.call_sym(LUA_MORE[2], Conv::Cdecl, &[thread]);
        h.call_sym(LUA_GETGLOBAL, Conv::Cdecl, &[thread, name]);
        if h.call_sym(LUA_TYPE, Conv::Cdecl, &[thread, u32::MAX]) == 0 {
            return 0;
        }
        if thread == l {
            let end = top.wrapping_add(1);
            let mut i = 1;
            while i < end {
                h.call_sym(LUA_MORE[9], Conv::Cdecl, &[thread, i]);
                i += 1;
            }
        } else {
            h.call_sym(COPY_VALUES, Conv::Cdecl, &[l, thread, 1, top]);
        }
        let err = h.call_sym(LUA_PCALLK, Conv::Cdecl, &[thread, top, 1, 0, 0, 0]);
        let text = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[thread, u32::MAX, 0]);
        if err != 0 {
            h.call_sym(PRINTF, Conv::Cdecl, &[3, msg, err, text]);
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[thread, 0xffff_fffe]);
            return 0;
        }
        if thread != l {
            h.call_sym(
                COPY_VALUES,
                Conv::Cdecl,
                &[thread, l, thread_top.wrapping_add(1), 1],
            );
        }
        h.call_sym(LUA_MORE[2], Conv::Cdecl, &[l]).wrapping_sub(top)
    }

    /// The ring buffer's reclaim (inside the lock): from the read block
    /// (`+0x28`), every block with no time stamp or one older than the
    /// frame counter is released (the read block moves on). `false` when
    /// nothing is in use or nothing could be released.
    fn ring_reclaim(h: &mut impl Host, ring: u32) -> bool {
        let write: u32 = h.read(ring + 0x24);
        let frame: u32 = h.read(h.rebase(RING_FRAME));
        let mut progressed = false;
        if h.read::<u32>(ring + 0x28) == write {
            return false;
        }
        loop {
            let block: u32 = h.read(ring + 0x28);
            let stamp: u32 = h.read(block + 4);
            if stamp != 0 && stamp.wrapping_sub(frame) & 0x8000_0000 == 0 {
                return progressed;
            }
            let next: u32 = h.read(block);
            h.write(ring + 0x28, next);
            progressed = true;
            if next == write {
                return true;
            }
        }
    }

    /// `PRingBuffer<PAsyncProcessBufferBlock>::allocateBlock(size)`
    /// (`0x005ce6e0`, thiscall, `ret 4`): a block of `size` bytes rounded
    /// up to 16 plus a 16-byte header `{next, stamp 0, size}` at the write
    /// position (`+0x24`), wrapping to the start (`+0x18`) when it would
    /// pass the end (`+0x1c`: the rest becomes a filler block). While the
    /// buffer is full (`0x005ce9d0`) the released blocks are reclaimed,
    /// waiting a millisecond at a time (a report every 1000 waits; after
    /// 10000 a report and a deliberate crash). 0 for a request too large
    /// (`+0x20`), no buffer, or nothing to reclaim; else the block (also
    /// kept at `+0x2c`).
    pub fn ring_allocate(h: &mut impl Host, ring: u32, size: u32) -> u32 {
        let need = (size.wrapping_add(15) & 0xffff_fff0).wrapping_add(16);
        if need >= h.read::<u32>(ring + 0x20) {
            return 0;
        }
        if h.read::<u32>(ring + 0x24) == 0 {
            return 0;
        }
        h.call_sym(RING_CALLS[0], Conv::Thiscall(ring), &[]);
        loop {
            if h.call_sym(RING_CALLS[2], Conv::Thiscall(ring), &[need]) & 0xff != 0 {
                if !ring_reclaim(h, ring) {
                    h.call_sym(RING_CALLS[1], Conv::Thiscall(ring), &[]);
                    return 0;
                }
                let mut tries = 0u32;
                while h.call_sym(RING_CALLS[2], Conv::Thiscall(ring), &[need]) & 0xff != 0 {
                    if tries.is_multiple_of(1000) {
                        let read: u32 = h.read(ring + 0x28);
                        let msg = h.rebase(RING_MESSAGES[0]);
                        h.call_sym(PRINTF, Conv::Cdecl, &[2, msg, read]);
                    }
                    h.call_sym(RING_CALLS[3], Conv::Cdecl, &[1]);
                    if !ring_reclaim(h, ring) {
                        h.call_sym(RING_CALLS[1], Conv::Thiscall(ring), &[]);
                        return 0;
                    }
                    let old = tries;
                    tries += 1;
                    if old > RING_MAX_WAITS {
                        ring_give_up(h, ring);
                    }
                }
            }
            let write: u32 = h.read(ring + 0x24);
            let end: u32 = h.read(ring + 0x1c);
            if write.wrapping_add(need) > end {
                let start: u32 = h.read(ring + 0x18);
                h.write(ring + 0x24, start);
                h.write(write + 4, 0u32);
                h.write(write + 8, end.wrapping_sub(write));
                h.write(write, start);
                continue;
            }
            let mut next = write.wrapping_add(need);
            if next == end {
                next = h.read(ring + 0x18);
            }
            h.write(ring + 0x24, next);
            h.write(write + 8, size);
            h.write(write + 4, 0u32);
            h.write(write, next);
            h.write(ring + 0x2c, write);
            h.call_sym(RING_CALLS[1], Conv::Thiscall(ring), &[]);
            return write;
        }
    }

    /// The ring buffer's give-up report, then the original's deliberate
    /// crash (a write to address 0).
    fn ring_give_up(h: &mut impl Host, ring: u32) {
        for &m in &RING_MESSAGES[1..3] {
            let msg = h.rebase(m);
            h.call_sym(PRINTF, Conv::Cdecl, &[2, msg]);
        }
        let (a, b, c): (u32, u32, u32) = (
            h.read(ring + 0x18),
            h.read(ring + 0x1c),
            h.read(ring + 0x20),
        );
        let msg = h.rebase(RING_MESSAGES[3]);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, msg, a, b, c]);
        let (a, b, c): (u32, u32, u32) = (
            h.read(ring + 0x24),
            h.read(ring + 0x28),
            h.read(ring + 0x2c),
        );
        let msg = h.rebase(RING_MESSAGES[4]);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, msg, a, b, c]);
        let msg = h.rebase(RING_MESSAGES[5]);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, msg]);
        h.write(0, 0u32);
    }

    /// `0x00543150` (thiscall `(clip)`, `ret 4`, the animation utility's
    /// clip switch): the time controller (`+0x24`) asked for its time
    /// (`vt[4]`, a float) at speed 1 and offset `-[0x00cab918]`, then at
    /// the old speed (`+8`), the offset (`+0x34`) moved by the difference;
    /// the clip set (`+0`, `0x00526590` on `+0x38`); its time range
    /// (`0x00516a20` into `range`: end, start; the original's frame) as
    /// start `+0x1c` and length `+0x20`; the time controller handed over
    /// (`0x005265f0`). The first callee error, else the last `eax`.
    pub fn set_clip(h: &mut impl Host, this: u32, clip: u32, range: u32) -> u32 {
        let c = f32::from_bits(h.read(h.rebase(CLIP_DATA[0])));
        let neg = (-ffx_bridge::x87::Reg::load(c)).store();
        let speed = ffx_bridge::x87::round_trip(f32::from_bits(h.read(this + 8)));
        h.write(this + 0x30, 1.0f32.to_bits());
        h.write(this + 0x34, neg.to_bits());
        let tc = this + 0x24;
        let vt: u32 = h.read(tc);
        let f: u32 = h.read(vt + 0x10);
        let a = h.call_st0(f, Conv::Thiscall(tc), &[]);
        h.write(this + 0x30, speed.to_bits());
        let vt: u32 = h.read(tc);
        let f: u32 = h.read(vt + 0x10);
        let b = h.call_st0(f, Conv::Thiscall(tc), &[]);
        let off = f32::from_bits(h.read(this + 0x34));
        let off = ffx_bridge::x87::Reg::load(a)
            .sub_m32(b)
            .add_m32(off)
            .store();
        h.write(this, clip);
        h.write(this + 0x34, off.to_bits());
        let r = h.call_sym(CLIP_DATA[1], Conv::Thiscall(this + 0x38), &[clip]);
        if r != 0 {
            return r;
        }
        let r = h.call_sym(CLIP_DATA[2], Conv::Thiscall(clip), &[range + 4, range]);
        if r != 0 {
            return r;
        }
        let start = f32::from_bits(h.read(range + 4));
        let end = f32::from_bits(h.read(range));
        h.write(this + 0x1c, ffx_bridge::x87::round_trip(start).to_bits());
        h.write(
            this + 0x20,
            ffx_bridge::x87::Reg::load(start)
                .subr_m32(end)
                .store()
                .to_bits(),
        );
        h.call_sym(CLIP_DATA[3], Conv::Thiscall(this + 0x38), &[this + 0xc])
    }

    /// The script's `Matrix(...)` constructor (`0x00570e30`, cdecl `(L)`, 1
    /// result): a 0x40-byte userdata with the matrix metatable; with 16
    /// numbers after the type its 16 floats, with one number that number
    /// in all 16, else the identity (`0x00454cf0`), staged in `frame` (16
    /// words, the original's `ebp - 0x44`) and copied (`0x00453340`).
    /// Nothing more when the userdata cannot be made. 1.
    pub fn matrix_new(h: &mut impl Host, l: u32, frame: u32) -> u32 {
        let top = h.call_sym(LUA_MORE[2], Conv::Cdecl, &[l]);
        let ud = h.call_sym(LUA_MORE[6], Conv::Cdecl, &[l, 0x40]);
        if ud == 0 {
            return 1;
        }
        let name = h.rebase(VM_DATA[1]);
        h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
        h.call_sym(LUA_MORE[7], Conv::Cdecl, &[l, 0xffff_fffe]);
        match top {
            17 => {
                for i in 0..16 {
                    let v = h.call_sym_st0(MATRIX_NEW[0], Conv::Cdecl, &[l, i + 2]);
                    h.write(frame + 4 * i, v.to_bits());
                }
                for i in 0..16 {
                    let v = ffx_bridge::x87::round_trip(f32::from_bits(h.read(frame + 4 * i)));
                    h.write(ud + 4 * i, v.to_bits());
                }
            }
            2 => {
                let v = h.call_sym_st0(MATRIX_NEW[0], Conv::Cdecl, &[l, 2]);
                for i in 0..16 {
                    h.write(frame + 4 * i, v.to_bits());
                }
                h.call_sym(MATRIX_NEW[2], Conv::Thiscall(ud), &[frame]);
            }
            _ => {
                let m = h.call_sym(MATRIX_NEW[1], Conv::Cdecl, &[frame]);
                h.call_sym(MATRIX_NEW[2], Conv::Thiscall(ud), &[m]);
            }
        }
        1
    }

    /// The `Vector` userdata at stack index `i` (`lua_touserdata`, its
    /// metatable the registry's vector one), else 0.
    fn vector_arg(h: &mut impl Host, l: u32, i: u32) -> u32 {
        let ud = h.call_sym(LUA_INDEX[0], Conv::Cdecl, &[l, i]);
        if ud == 0 || h.call_sym(LUA_INDEX[1], Conv::Cdecl, &[l, i]) == 0 {
            return 0;
        }
        let name = h.rebase(VM_DATA[0]);
        h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
        let same = h.call_sym(LUA_INDEX[2], Conv::Cdecl, &[l, u32::MAX, 0xffff_fffe]) != 0;
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffd]);
        if same { ud } else { 0 }
    }

    /// The script's `Matrix.LookAt(eye, target, up)` (`0x00571200`, cdecl
    /// `(L)`, 1 result): a new matrix userdata; with three vectors their
    /// xyz copied into `frame` (+0x40, +0x50, +0x60; 30 words, the
    /// original's `ebp - 0x78`) and `PMatrix4::lookAt` (`0x00451c30`);
    /// else the identity staged at `frame` and copied (`0x00453340`). The
    /// userdata may be 0 (the callee then gets 0 as `this`, as in the
    /// original). 1.
    pub fn matrix_look_at(h: &mut impl Host, l: u32, frame: u32) -> u32 {
        let eye = vector_arg(h, l, 1);
        let target = vector_arg(h, l, 2);
        let up = vector_arg(h, l, 3);
        let ud = h.call_sym(LUA_MORE[6], Conv::Cdecl, &[l, 0x40]);
        if ud != 0 {
            let name = h.rebase(VM_DATA[1]);
            h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
            h.call_sym(LUA_MORE[7], Conv::Cdecl, &[l, 0xffff_fffe]);
        }
        if eye != 0 && target != 0 && up != 0 {
            for (src, dst) in [
                (up, frame + 0x60),
                (target, frame + 0x50),
                (eye, frame + 0x40),
            ] {
                for i in 0..3 {
                    let v = ffx_bridge::x87::round_trip(f32::from_bits(h.read(src + 4 * i)));
                    h.write(dst + 4 * i, v.to_bits());
                }
            }
            h.call_sym(
                MATRIX_NEW[3],
                Conv::Thiscall(ud),
                &[frame + 0x40, frame + 0x50, frame + 0x60],
            );
        } else {
            let one = 1.0f32.to_bits();
            for i in 0..16 {
                h.write(frame + 4 * i, if i % 5 == 0 { one } else { 0 });
            }
            h.call_sym(MATRIX_NEW[2], Conv::Thiscall(ud), &[frame]);
        }
        1
    }

    /// `strcmp(a, b) == 0`, as the originals' inline loops compare.
    fn same_c_string(h: &impl Mem, a: u32, b: u32) -> bool {
        let mut i = 0u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(i));
            if x != h.read::<u8>(b.wrapping_add(i)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            i = i.wrapping_add(1);
        }
    }

    /// The script's matrix `__index` (`0x00571770`, cdecl `(L)`): with a
    /// matrix at 1 (`0x00570d50`), the keys "0".."3" give a new vector
    /// (`0x00571070`) holding that row, and two method names their C
    /// functions; on the `Matrix` table itself four names give theirs
    /// ([`MATRIX_INDEX`]). 1 when something was pushed (or the vector
    /// could not be made), else 0 (no key, an unknown one).
    pub fn matrix_index(h: &mut impl Host, l: u32) -> u32 {
        let key = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, 2, 0]);
        if key == 0 {
            return 0;
        }
        let m = h.call_sym(MATRIX_INDEX[0], Conv::Cdecl, &[l, 1]);
        let first: u8 = h.read(key);
        let methods: &[(u32, u32)] = if m != 0 {
            if first != 0 && h.read::<u8>(key + 1) == 0 {
                // movsx of the key's byte, compared unsigned
                let row = (i32::from(first.cast_signed()) - 0x30).cast_unsigned();
                if row > 3 {
                    return 0;
                }
                let v = h.call_sym(MATRIX_INDEX[1], Conv::Cdecl, &[l]);
                if v != 0 {
                    let src = m + 0x10 * row;
                    for i in 0..4 {
                        let x = ffx_bridge::x87::round_trip(f32::from_bits(h.read(src + 4 * i)));
                        h.write(v + 4 * i, x.to_bits());
                    }
                }
                return 1;
            }
            &MATRIX_INDEX_METHODS
        } else {
            &MATRIX_INDEX_STATICS
        };
        for &(name, f) in methods {
            let name = h.rebase(name);
            if same_c_string(h, key, name) {
                let f = h.rebase(f);
                h.call_sym(LUA_MORE[4], Conv::Cdecl, &[l, f, 0]);
                return 1;
            }
        }
        0
    }

    /// A new userdata of `size` bytes with the registry's metatable
    /// `name` (`lua_newuserdata`, `lua_getfield`, `lua_setmetatable` when
    /// it was made); the userdata or 0.
    fn new_userdata(h: &mut impl Host, l: u32, size: u32, name: u32) -> u32 {
        let ud = h.call_sym(LUA_MORE[6], Conv::Cdecl, &[l, size]);
        if ud != 0 {
            let name = h.rebase(name);
            h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
            h.call_sym(LUA_MORE[7], Conv::Cdecl, &[l, 0xffff_fffe]);
        }
        ud
    }

    /// Copies the four floats at `src` to `dst` through the x87.
    fn copy_vector(h: &mut impl Host, src: u32, dst: u32) {
        for i in 0..4 {
            let x = ffx_bridge::x87::round_trip(f32::from_bits(h.read(src + 4 * i)));
            h.write(dst + 4 * i, x.to_bits());
        }
    }

    /// The script's matrix `__mul` (`0x00571a60`, cdecl `(L)`). Argument 1
    /// is checked as a `Matrix`, argument 2 as a `Matrix` and as a
    /// `Vector` (all three checks always run). Matrix times matrix: a new
    /// matrix userdata, each row of the second times the first
    /// (`PMatrix4::rowTimes`, rows 3, 2, 1, 0 into `frame` +0x20, +0x10,
    /// +0x00, +0x30) staged at `frame` +0x40 and copied in
    /// (`PMatrix4::copy`, `this` the userdata, 0 when it failed). Matrix
    /// times vector: a new vector (`0x00571070`) set to the vector times
    /// the matrix (through `frame` +0x30). Matrix times number
    /// (`luaL_checknumber(L, 2)`, a `float`), or number
    /// (`luaL_checknumber(L, 1)`) times matrix: the scaled matrix
    /// (`0x00570c00` into `frame` +0x40) copied into a new matrix
    /// userdata; the first makes it with `0x00571030` after the number,
    /// the second inline. 1, or 0 when neither side is a matrix.
    ///
    /// `frame` is 32 words, the original's `ebp - 0x8c`.
    pub fn matrix_mul(h: &mut impl Host, l: u32, frame: u32) -> u32 {
        use crate::m5p_scripting_mid::bridge::check_userdata;
        let a = check_userdata(h, MATRIX_MUL[3], l, 1);
        let b = check_userdata(h, MATRIX_MUL[3], l, 2);
        let v = check_userdata(h, MATRIX_MUL[4], l, 2);
        if a == 0 {
            if b == 0 {
                return 0;
            }
            let s = h.call_sym_st0(MATRIX_NEW[0], Conv::Cdecl, &[l, 1]);
            let ud = new_userdata(h, l, 0x40, MATRIX_MUL[3]);
            let out = h.call_sym(
                MATRIX_MUL[2],
                Conv::Thiscall(b),
                &[frame + 0x40, s.to_bits()],
            );
            h.call_sym(MATRIX_NEW[2], Conv::Thiscall(ud), &[out]);
            return 1;
        }
        if b != 0 {
            let ud = new_userdata(h, l, 0x40, MATRIX_MUL[3]);
            let r3 = h.call_sym(MATRIX_MUL[1], Conv::Thiscall(a), &[frame + 0x20, b + 0x30]);
            let r2 = h.call_sym(MATRIX_MUL[1], Conv::Thiscall(a), &[frame + 0x10, b + 0x20]);
            let r1 = h.call_sym(MATRIX_MUL[1], Conv::Thiscall(a), &[frame, b + 0x10]);
            let r0 = h.call_sym(MATRIX_MUL[1], Conv::Thiscall(a), &[frame + 0x30, b]);
            for (i, row) in [r0, r1, r2, r3].into_iter().enumerate() {
                copy_vector(h, row, frame + 0x40 + 0x10 * i as u32);
            }
            h.call_sym(MATRIX_NEW[2], Conv::Thiscall(ud), &[frame + 0x40]);
            return 1;
        }
        if v != 0 {
            let dst = h.call_sym(MATRIX_INDEX[1], Conv::Cdecl, &[l]);
            let r = h.call_sym(MATRIX_MUL[1], Conv::Thiscall(a), &[frame + 0x30, v]);
            copy_vector(h, r, dst);
            return 1;
        }
        let s = h.call_sym_st0(MATRIX_NEW[0], Conv::Cdecl, &[l, 2]);
        let ud = h.call_sym(MATRIX_MUL[0], Conv::Cdecl, &[l]);
        let out = h.call_sym(
            MATRIX_MUL[2],
            Conv::Thiscall(a),
            &[frame + 0x40, s.to_bits()],
        );
        h.call_sym(MATRIX_NEW[2], Conv::Thiscall(ud), &[out]);
        1
    }

    /// `Phyre::PScripting::VectorIndexHandler` (`0x00572e70`, cdecl
    /// `(L)`): the key `lua_tolstring(L, 2)` (none: 0); without a vector
    /// at 1 the error location (file, line 542, function) and
    /// `luaL_error(L, "Trying to get member '%s' ...", key)`, then 0. A
    /// one-character key is a component (`0x005710b0`: '0'..'3', x y z w;
    /// another character: 0 pushed nothing), its float pushed
    /// (`lua_pushnumber`, through the x87); else one of nine method names
    /// pushes its C function (`lua_pushcclosure(L, f, 0)`). 1 when
    /// something was pushed, else 0.
    pub fn vector_index(h: &mut impl Host, l: u32) -> u32 {
        let key = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, 2, 0]);
        if key == 0 {
            return 0;
        }
        let v = h.call_sym(VECTOR_INDEX[0], Conv::Cdecl, &[l, 1]);
        if v == 0 {
            for (at, value) in [
                (ERROR_AT[0], h.rebase(VECTOR_INDEX[3])),
                (ERROR_AT[1], VECTOR_INDEX[6]),
                (ERROR_AT[2], h.rebase(VECTOR_INDEX[4])),
            ] {
                let at = h.rebase(at);
                h.write(at, value);
            }
            let msg = h.rebase(VECTOR_INDEX[5]);
            h.call_sym(LUA_INDEX[7], Conv::Cdecl, &[l, msg, key]);
            return 0;
        }
        let first: u8 = h.read(key);
        if first != 0 && h.read::<u8>(key + 1) == 0 {
            let i = h.call_sym(VECTOR_INDEX[1], Conv::Cdecl, &[u32::from(first)]);
            if i.cast_signed() < 0 {
                return 0;
            }
            let x: u32 = h.read(v.wrapping_add(i.wrapping_mul(4)));
            let x = ffx_bridge::x87::round_trip(f32::from_bits(x));
            h.call_sym(VECTOR_INDEX[2], Conv::Cdecl, &[l, x.to_bits()]);
            return 1;
        }
        for &(name, f) in &VECTOR_INDEX_METHODS {
            let name = h.rebase(name);
            if same_c_string(h, key, name) {
                let f = h.rebase(f);
                h.call_sym(LUA_MORE[4], Conv::Cdecl, &[l, f, 0]);
                return 1;
            }
        }
        0
    }

    /// A float of game memory in an x87 register.
    fn reg(h: &impl Mem, at: u32) -> ffx_bridge::x87::Reg {
        ffx_bridge::x87::Reg::load(f32::from_bits(h.read(at)))
    }

    /// The occluder's plane through a point (`0x00a30eb0`, cdecl `(out, a,
    /// b, p)` -> `out`): the normal `a x b` (each component `f32(x*y -
    /// z*w)`), its squared length summed in `float`s, `_CIsqrt` (the
    /// correctly rounded double square root, as the x87's `fsqrt` at 53
    /// bits; the parity test runs the real intrinsic), `inv = f32(1 /
    /// len)`; `out` = the normal times `inv` (each rounded), and `out[3]` =
    /// the dot product with `p` (`f32(p0 n0)`, then `+ p1 n1`, `+ p2 n2`,
    /// each rounded). The original keeps its sums in its own argument
    /// slot (`b`'s), which its callers never read again.
    pub fn occluder_plane(h: &mut impl Host, out: u32, a: u32, b: u32, p: u32) -> u32 {
        use ffx_bridge::x87::Reg;
        let (a0, a1, a2) = (reg(h, a), reg(h, a + 4), reg(h, a + 8));
        let (b0, b1, b2) = (reg(h, b), reg(h, b + 4), reg(h, b + 8));
        let n0 = Reg::load((a1 * b2 - a2 * b1).store());
        let n1 = Reg::load((a2 * b0 - a0 * b2).store());
        let n2 = Reg::load((b1 * a0 - b0 * a1).store());
        let s = (n0 * n0).store();
        let s = (Reg::load(s) + n1 * n1).store();
        let s = (Reg::load(s) + n2 * n2).store();
        let len = Reg::load(s).sqrt().store();
        let inv = Reg::load((Reg::ONE / Reg::load(len)).store());
        let q0 = (n0 * inv).store();
        let q1 = (n1 * inv).store();
        let q2 = (inv * n2).store();
        let (p0, p1, p2) = (reg(h, p), reg(h, p + 4), reg(h, p + 8));
        let d = (p0 * Reg::load(q0)).store();
        let d = (p1 * Reg::load(q1) + Reg::load(d)).store();
        let d = (p2 * Reg::load(q2) + Reg::load(d)).store();
        h.write(out, q0.to_bits());
        h.write(out + 4, q1.to_bits());
        h.write(out + 8, q2.to_bits());
        h.write(out + 0xc, d.to_bits());
        out
    }

    /// The occluder's edge quad (`0x00a30600`, thiscall `(list; a, b,
    /// d)`, ret 0xc): with a buffer (`+0`), four 16-byte slots reserved at
    /// the index `+4` (`InterlockedCompareExchange`, retried until it takes;
    /// nothing when they would reach the end `+8`), filled with `a`, `b`,
    /// their midpoint `f32(f32(a + b) * 0.5)` (the double at
    /// `0x00b92220`) and the midpoint plus `d`, each with `w` 1.0.
    pub fn occluder_quad(h: &mut impl Host, list: u32, a: u32, b: u32, d: u32) {
        use ffx_bridge::x87::Reg;
        if h.read::<u32>(list) == 0 {
            return;
        }
        let cur = loop {
            let cur: u32 = h.read(list + 4);
            let next = cur.wrapping_add(4);
            if next >= h.read::<u32>(list + 8) {
                return;
            }
            let f: u32 = h.read(h.rebase(OCCLUDER_QUAD[0]));
            if h.call(f, Conv::Stdcall, &[list + 4, next, cur]) == cur {
                break cur;
            }
        };
        let rec = (cur << 4).wrapping_add(h.read(list));
        let one = 1.0f32.to_bits();
        for (i, v) in [a, b].into_iter().enumerate() {
            let at = rec + 0x10 * i as u32;
            for k in 0..3 {
                let x = reg(h, v + 4 * k).store();
                h.write(at + 4 * k, x.to_bits());
            }
            h.write(at + 0xc, one);
        }
        let half = Reg::constant(f64::from_bits(h.read(h.rebase(OCCLUDER_QUAD[1]))));
        let m: [Reg; 3] = core::array::from_fn(|k| {
            Reg::load((reg(h, a + 4 * k as u32) + reg(h, b + 4 * k as u32)).store())
        });
        let c = [
            Reg::load((m[0] * half).store()),
            Reg::load((m[1] * half).store()),
            Reg::load((half * m[2]).store()),
        ];
        for (k, ck) in c.iter().enumerate() {
            h.write(rec + 0x20 + 4 * k as u32, ck.store().to_bits());
        }
        h.write(rec + 0x2c, one);
        for (k, ck) in c.into_iter().enumerate() {
            let k = k as u32;
            let e = (ck + reg(h, d + 4 * k)).store();
            h.write(rec + 0x30 + 4 * k, e.to_bits());
        }
        h.write(rec + 0x3c, one);
    }
}
