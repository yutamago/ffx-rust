//! List `phyre-scripting`, round 10 (`tail10`): the Lua `print` the engine
//! registers, `Phyre_PScripting_Print` (`0x005d5c00`, a Lua C function
//! `(L) -> 0`).
//!
//! It formats every stack value of `L` into a 0x200-byte buffer in its
//! frame (`[ebp-0x204]`) with `sprintf_s`, tab-separated, ends it with a
//! newline and hands it to `PhyrePrintf(5, "%s", buffer)`. Each value goes
//! by the first test that holds: a string (`lua_isstring`, printed as
//! `lua_tolstring`'s text), `nil`, a boolean, a non-userdata (`type:%p`
//! with `lua_topointer`), a `Vector` (four floats) or `Matrix` (sixteen)
//! userdata, a userdata whose metatable is the registry's object
//! metatable (`0x00b0e23c`; a `PString` object prints its text, any other
//! object `class name:%p`), else `userdata:%p`.
//!
//! The matrix case passes `sprintf_s` 36 words (buffer, size, format, the
//! matrix, sixteen doubles): the wide call path (`Host::call_wide`, 40
//! words since this round).

use ffx_graphic::host::{Conv, Host};

/// `sprintf_s`'s import slot (cdecl, `MSVCR110.dll`).
pub const SPRINTF_S_SLOT: u32 = 0x00b0_c394;
/// `lua_gettop(L)`.
pub const LUA_GETTOP: u32 = 0x0094_b160;
/// `lua_isstring(L, idx)`.
pub const LUA_ISSTRING: u32 = 0x0094_b310;
/// `lua_tolstring(L, idx, len)`.
pub const LUA_TOLSTRING: u32 = 0x0094_bfc0;
/// `lua_type(L, idx)`.
pub const LUA_TYPE: u32 = 0x0094_c1e0;
/// `lua_toboolean(L, idx)`.
pub const LUA_TOBOOLEAN: u32 = 0x0094_bf00;
/// `lua_isuserdata(L, idx)`.
pub const LUA_ISUSERDATA: u32 = 0x0094_b350;
/// `lua_topointer(L, idx)`.
pub const LUA_TOPOINTER: u32 = 0x0094_c0a0;
/// `lua_typename(L, type)`.
pub const LUA_TYPENAME: u32 = 0x0094_c210;
/// `lua_touserdata(L, idx)`.
pub const LUA_TOUSERDATA: u32 = 0x0094_c1b0;
/// `lua_getmetatable(L, idx)`.
pub const LUA_GETMETATABLE: u32 = 0x0094_b0d0;
/// `lua_getfield(L, idx, name)`.
pub const LUA_GETFIELD: u32 = 0x0094_b030;
/// `lua_rawequal(L, a, b)`.
pub const LUA_RAWEQUAL: u32 = 0x0094_b8b0;
/// `lua_settop(L, idx)`.
pub const LUA_SETTOP: u32 = 0x0094_bdc0;
/// `Phyre_PScripting_checkVector_00570dc0(L, idx)`: the `Vector` userdata.
pub const CHECK_VECTOR: u32 = 0x0057_0dc0;
/// `Phyre_PScripting_testUserData_00570d50(L, idx)`: the `Matrix` one.
pub const TEST_MATRIX: u32 = 0x0057_0d50;
/// `Phyre_PNamespace_GetGlobalNamespace()` (cdecl).
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
/// `Phyre_PNamespace_getClassDescriptor(namespace; name)` (thiscall, ret 4).
pub const CLASS_DESCRIPTOR: u32 = 0x0043_e950;
/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
/// Lua 5.2's `LUA_REGISTRYINDEX` (`-1001000`).
pub const REGISTRY_INDEX: u32 = 0xfff0_b9d8;
/// The registry key of the object metatable.
pub const OBJECT_METATABLE: u32 = 0x00b0_e23c;
/// The class name `getClassDescriptor` is asked for (the `PString`
/// class, whose objects print their text).
pub const STRING_CLASS: u32 = 0x00b0_f6ec;

/// `"\t"`.
pub const FMT_TAB: u32 = 0x00b3_106c;
/// `"%s"`.
pub const FMT_S: u32 = 0x00b0_e160;
/// `"nil"`.
pub const FMT_NIL: u32 = 0x00b3_922c;
/// `"true"`.
pub const FMT_TRUE: u32 = 0x00b3_9230;
/// `"false"`.
pub const FMT_FALSE: u32 = 0x00b3_9238;
/// `"Vector:%p {%f %f %f %f}"`.
pub const FMT_VECTOR: u32 = 0x00b3_9240;
/// `"Matrix:%p\n{%f %f %f %f}\n..."` (four rows).
pub const FMT_MATRIX: u32 = 0x00b3_9258;
/// `"%s:%p"`.
pub const FMT_TYPE_PTR: u32 = 0x00b3_929c;
/// `"userdata:%p"`.
pub const FMT_USERDATA: u32 = 0x00b3_92a4;
/// `"\n"`.
pub const FMT_NEWLINE: u32 = 0x00b7_cae0;

/// The buffer's size (`[ebp-0x204]` up to the cookie at `[ebp-4]`).
pub const BUFFER: u32 = 0x200;

/// The originals over game memory.
pub mod bridge {
    use super::{
        BUFFER, CHECK_VECTOR, CLASS_DESCRIPTOR, Conv, FMT_FALSE, FMT_MATRIX, FMT_NEWLINE, FMT_NIL,
        FMT_S, FMT_TAB, FMT_TRUE, FMT_TYPE_PTR, FMT_USERDATA, FMT_VECTOR, GLOBAL_NAMESPACE, Host,
        LUA_GETFIELD, LUA_GETMETATABLE, LUA_GETTOP, LUA_ISSTRING, LUA_ISUSERDATA, LUA_RAWEQUAL,
        LUA_SETTOP, LUA_TOBOOLEAN, LUA_TOLSTRING, LUA_TOPOINTER, LUA_TOUSERDATA, LUA_TYPE,
        LUA_TYPENAME, OBJECT_METATABLE, PHYRE_PRINTF, REGISTRY_INDEX, SPRINTF_S_SLOT, STRING_CLASS,
        TEST_MATRIX,
    };

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `strlen` as the original inlines it.
    fn str_len(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The floats at `p` as the doubles `fld dword` / `fstp qword` push
    /// into `out`: each the low word, then the high.
    fn doubles(h: &impl Host, p: u32, out: &mut [u32]) {
        for (i, w) in (0u32..).zip(out.as_chunks_mut::<2>().0) {
            let x = ffx_bridge::x87::load(h.read::<u32>(p.wrapping_add(4 * i)));
            let bits = x.to_bits();
            w[0] = bits as u32;
            w[1] = (bits >> 32) as u32;
        }
    }

    /// The most words a value's `sprintf_s` takes: the matrix's buffer,
    /// size, format, pointer and sixteen doubles.
    const WORDS: usize = 36;

    /// `Phyre_PScripting_Print(L)` with its buffer at `buf` (`[ebp-0x204]`,
    /// [`BUFFER`] bytes); returns 0 (no Lua results).
    pub fn print(h: &mut impl Host, l: u32, buf: u32) -> u32 {
        let end = buf.wrapping_add(BUFFER);
        let sprintf: u32 = h.read(h.rebase(SPRINTF_S_SLOT));
        h.write(buf, 0u8);
        let mut at = buf;
        let top = cdecl(h, LUA_GETTOP, &[l]) as i32;
        let mut i = 1i32;
        while i <= top {
            let idx = i as u32;
            if i > 1 {
                let tab = h.rebase(FMT_TAB);
                h.call(sprintf, Conv::Cdecl, &[at, end.wrapping_sub(at), tab]);
                at = at.wrapping_add(str_len(h, at));
            }
            // buffer, size, the format and its values
            let mut args = [0u32; WORDS];
            args[0] = at;
            args[1] = end.wrapping_sub(at);
            let n = if cdecl(h, LUA_ISSTRING, &[l, idx]) != 0 {
                let s = cdecl(h, LUA_TOLSTRING, &[l, idx, 0]);
                args[2..4].copy_from_slice(&[h.rebase(FMT_S), s]);
                4
            } else if cdecl(h, LUA_TYPE, &[l, idx]) == 0 {
                args[2] = h.rebase(FMT_NIL);
                3
            } else if cdecl(h, LUA_TYPE, &[l, idx]) == 1 {
                let fmt = if cdecl(h, LUA_TOBOOLEAN, &[l, idx]) == 0 {
                    FMT_FALSE
                } else {
                    FMT_TRUE
                };
                args[2] = h.rebase(fmt);
                3
            } else if cdecl(h, LUA_ISUSERDATA, &[l, idx]) == 0 {
                let p = cdecl(h, LUA_TOPOINTER, &[l, idx]);
                let ty = cdecl(h, LUA_TYPE, &[l, idx]);
                let name = cdecl(h, LUA_TYPENAME, &[l, ty]);
                args[2..5].copy_from_slice(&[h.rebase(FMT_TYPE_PTR), name, p]);
                5
            } else {
                let vector = cdecl(h, CHECK_VECTOR, &[l, idx]);
                let matrix = cdecl(h, TEST_MATRIX, &[l, idx]);
                if vector != 0 {
                    args[2..4].copy_from_slice(&[h.rebase(FMT_VECTOR), vector]);
                    doubles(h, vector, &mut args[4..12]);
                    12
                } else if matrix != 0 {
                    args[2..4].copy_from_slice(&[h.rebase(FMT_MATRIX), matrix]);
                    doubles(h, matrix, &mut args[4..WORDS]);
                    WORDS
                } else {
                    let (fmt, len) = user_data(h, l, idx);
                    args[2..2 + len].copy_from_slice(&fmt[..len]);
                    2 + len
                }
            };
            // 36 words for the matrix: the wide path (40 words)
            h.call_wide(sprintf, &args[..n]);
            at = at.wrapping_add(str_len(h, at));
            i += 1;
        }
        let newline = h.rebase(FMT_NEWLINE);
        h.call(sprintf, Conv::Cdecl, &[at, end.wrapping_sub(at), newline]);
        let s = h.rebase(FMT_S);
        cdecl(h, PHYRE_PRINTF, &[5, s, buf]);
        0
    }

    /// The format words of a userdata that is neither vector nor matrix,
    /// and how many there are.
    fn user_data(h: &mut impl Host, l: u32, idx: u32) -> ([u32; 3], usize) {
        let u = cdecl(h, LUA_TOUSERDATA, &[l, idx]);
        if cdecl(h, LUA_GETMETATABLE, &[l, idx]) != 0 {
            let key = h.rebase(OBJECT_METATABLE);
            cdecl(h, LUA_GETFIELD, &[l, REGISTRY_INDEX, key]);
            let same = cdecl(h, LUA_RAWEQUAL, &[l, u32::MAX, 0xffff_fffe]) != 0;
            cdecl(h, LUA_SETTOP, &[l, 0xffff_fffd]);
            if same && u != 0 {
                let class: u32 = h.read(u);
                let ns = cdecl(h, GLOBAL_NAMESPACE, &[]);
                let name = h.rebase(STRING_CLASS);
                let string = h.call_sym(CLASS_DESCRIPTOR, Conv::Thiscall(ns), &[name]);
                let object: u32 = h.read(u.wrapping_add(4));
                if class == string {
                    // the `PString`'s text pointer, its low bit a flag
                    let text: u32 = h.read(object);
                    let flag = u32::from(h.read::<u8>(object) & 1);
                    return ([h.rebase(FMT_S), text.wrapping_sub(flag), 0], 2);
                }
                let class_name: u32 = h.read(class.wrapping_add(0x18));
                return ([h.rebase(FMT_TYPE_PTR), class_name, object], 3);
            }
        }
        ([h.rebase(FMT_USERDATA), u, 0], 2)
    }
}
