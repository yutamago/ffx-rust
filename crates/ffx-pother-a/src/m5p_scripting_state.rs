//! `PhyreEngine`'s scripting support outside the script layer (M5 stage 1,
//! list `phyre-scripting`, batch 5): the script states and their runs
//! (`PScripting_NewStateForScript`, `PScriptedComponent_createThread`,
//! `PScheduledScript::execute` and `executePersistent`) and the `Vector`
//! and `Matrix` member setters (`VectorNewIndexHandler`,
//! `MatrixNewIndexHandler`).

use ffx_graphic::host::{Conv, Host};

use crate::m5p_scripting_lua::{
    ERROR_FILE, ERROR_FUNCTION, ERROR_LINE, LUA_CHECKNUMBER, LUA_ERROR,
};
use crate::m5p_scripting_mid::{LUA_PCALLK, LUA_TOLSTRING, PRINTF, bridge as mid};
use crate::m5p_scripting_small::{
    LUA_CLOSE, LUA_GETGLOBAL, LUA_PUSHLIGHTUSERDATA, LUA_SETGLOBAL, LUA_SETTOP, PUSH_OBJECT,
    STR_PHYRE_PSCRIPT, STR_VECTOR, bridge as small,
};

/// `lua_newstate(alloc, ud)` (cdecl).
pub const LUA_NEWSTATE: u32 = 0x0095_06d0;
/// `0x009540f0` `(L)` (cdecl): `luaL_openlibs`.
pub const LUA_OPENLIBS: u32 = 0x0095_40f0;
/// `lua_gc(L, what, data)` (cdecl).
pub const LUA_GC: u32 = 0x0094_ae50;
/// `Phyre_PScripting_GetScriptFunctionNames(&L, &names)` (cdecl).
pub const GET_FUNCTION_NAMES: u32 = 0x005d_5860;
/// `0x00570d50` `(L, idx)` (cdecl): the `Matrix` userdata at `idx`, or 0.
pub const CHECK_MATRIX: u32 = 0x0057_0d50;
/// `0x00574680` (thiscall on a matrix, two words: row, &vector): a row
/// set.
pub const MATRIX_SET_ROW: u32 = 0x0057_4680;
/// The Lua allocator `0x005d5ba0` the scripted components' states use.
pub const LUA_ALLOC: u32 = 0x005d_5ba0;
/// The scheduler's insert (thiscall, six words).
pub const SCHEDULER_INSERT: u32 = 0x005d_0280;
/// The scheduler's add forwarder (thiscall, four words).
pub const SCHEDULER_ADD: u32 = 0x005d_0260;
/// `0x005d1670` (thiscall, four words): the scheduler's start.
pub const SCHEDULER_START: u32 = 0x005d_1670;
/// `0x0054dcd0` (thiscall on a name table, `&name`, 0): the name's index.
pub const NAME_INDEX: u32 = 0x0054_dcd0;
/// The source file the vector math errors name.
pub const STR_VECTORMATH_CPP: u32 = 0x00b3_07ec;
/// The member setters' messages and `__FUNCTION__`s: (message, function,
/// line) for `Vector`, then `Matrix`.
pub const NEWINDEX_ERRORS: [(u32, u32, u32); 2] = [
    (0x00b3_08d4, 0x00b3_08a8, 0x25c),
    (0x00b3_095c, 0x00b3_0930, 0x37c),
];
/// `execute: lua_pcall failed - error code: %d\n%s (%s)\n`.
pub const STR_PCALL_FAILED_NAMED: u32 = 0x00b3_f3b0;
/// `execute: no valid script entryPoint set (%s)\n`.
pub const STR_NO_ENTRY: u32 = 0x00b3_f3e4;
/// `execute`'s second `lua_pcall` failure format.
pub const STR_PCALL_FAILED_2: u32 = 0x00b2_d298;
/// `%s\n`.
pub const STR_LINE: u32 = 0x00b1_2ad0;

/// The ports over game memory.
pub mod bridge {
    use super::{
        CHECK_MATRIX, Conv, ERROR_FILE, ERROR_FUNCTION, ERROR_LINE, GET_FUNCTION_NAMES, Host,
        LUA_ALLOC, LUA_CHECKNUMBER, LUA_CLOSE, LUA_ERROR, LUA_GC, LUA_GETGLOBAL, LUA_NEWSTATE,
        LUA_OPENLIBS, LUA_PCALLK, LUA_PUSHLIGHTUSERDATA, LUA_SETGLOBAL, LUA_SETTOP, LUA_TOLSTRING,
        MATRIX_SET_ROW, NAME_INDEX, NEWINDEX_ERRORS, PRINTF, PUSH_OBJECT, SCHEDULER_ADD,
        SCHEDULER_INSERT, SCHEDULER_START, STR_LINE, STR_NO_ENTRY, STR_PCALL_FAILED_2,
        STR_PCALL_FAILED_NAMED, STR_PHYRE_PSCRIPT, STR_VECTOR, STR_VECTORMATH_CPP, mid, small,
    };

    /// The C string of a `PString` word (bit 0 tags the pointer).
    fn pstr(h: &impl Host, at: u32) -> u32 {
        let w: u32 = h.read(at);
        w.wrapping_sub(w & 1)
    }

    /// `PScripting_NewStateForScript` `0x005d53b0` `(script, alloc)`: a new
    /// Lua state with the libraries, the `Phyre` types and the global
    /// functions; with a script, the script loaded, the global
    /// `phyre_pscript` set and its chunk run (or its function names
    /// read); the state, or 0 (closed) when that failed. `cell` is the
    /// state's word whose address the original hands the runner.
    pub fn new_state(h: &mut impl Host, script: u32, alloc: u32, cell: u32) -> u32 {
        let l = h.call_sym(LUA_NEWSTATE, Conv::Cdecl, &[alloc, 0]);
        h.write(cell, l);
        if l == 0 {
            return 0;
        }
        h.call_sym(LUA_OPENLIBS, Conv::Cdecl, &[l]);
        small::register_phyre_namespace(h, l);
        mid::register_functions(h, l);
        if script == 0 {
            return l;
        }
        if mid::load_script(h, l, script) as u8 != 0 {
            h.call_sym(LUA_PUSHLIGHTUSERDATA, Conv::Cdecl, &[l, script]);
            let name = h.rebase(STR_PHYRE_PSCRIPT);
            h.call_sym(LUA_SETGLOBAL, Conv::Cdecl, &[l, name]);
            let names = script.wrapping_add(8);
            let count: u32 = h.read(names);
            let r = if count << 1 == 0 {
                h.call_sym(GET_FUNCTION_NAMES, Conv::Cdecl, &[cell, names])
            } else {
                mid::run_chunk(h, cell)
            };
            if r == 0 {
                return l;
            }
        }
        h.call_sym(LUA_CLOSE, Conv::Cdecl, &[l]);
        0
    }

    /// `PScriptedComponent_createThread` `0x00554860` (thiscall `(L)`):
    /// with a scheduler (`+0x18`) and a component (`+0x14`), the
    /// component's Lua state (`L`, or for a component with a script flag
    /// `+0x20` its own new state, [`new_state`]; 0 otherwise) and its
    /// periodic entry (`0x005d0280`), plus the start function (`+0x20`)
    /// when its table knows it (`0x005d0260`, a non-zero result returned);
    /// a started component (`+0x2e`, `+0x2c`) is kicked (`0x005d1670`).
    /// 7 without a scheduler or state, else the insert's result. `cell`
    /// and `state_cell` are the words the original keeps in its argument
    /// area and [`new_state`]'s frame.
    pub fn create_thread(h: &mut impl Host, this: u32, l: u32, cell: u32, state_cell: u32) -> u32 {
        let mut ret = 7;
        let sched: u32 = h.read(this.wrapping_add(0x18));
        let comp: u32 = h.read(this.wrapping_add(0x14));
        if sched != 0 && comp != 0 {
            if l != 0 {
                h.write(this.wrapping_add(0x1c), l);
            } else if h.read::<u8>(comp.wrapping_add(0x20)) == 0 {
                h.write(this.wrapping_add(0x1c), 0u32);
            } else {
                if h.read::<u8>(this.wrapping_add(0x2d)) != 0
                    && h.read::<u32>(this.wrapping_add(0x1c)) != 0
                {
                    return 0;
                }
                let alloc = h.rebase(LUA_ALLOC);
                let st = new_state(h, comp, alloc, state_cell);
                h.write(this.wrapping_add(0x1c), st);
                if st == 0 {
                    return 7;
                }
                h.write(this.wrapping_add(0x2d), 1u8);
            }
            let state: u32 = h.read(this.wrapping_add(0x1c));
            let period: u32 = h.read(this.wrapping_add(0x28));
            let name = pstr(h, this.wrapping_add(0x10));
            let a2: u32 = h.read(this.wrapping_add(0xc));
            let sched: u32 = h.read(this.wrapping_add(0x18));
            let comp: u32 = h.read(this.wrapping_add(0x14));
            let key = this.wrapping_add(4);
            ret = h.call_sym(
                SCHEDULER_INSERT,
                Conv::Thiscall(sched),
                &[comp, key, a2, name, period, state],
            );
            let start = pstr(h, this.wrapping_add(0x20));
            if start != 0 && h.read::<u8>(start) != 0 {
                let comp: u32 = h.read(this.wrapping_add(0x14));
                h.write(cell, start);
                let idx = h.call_sym(NAME_INDEX, Conv::Thiscall(comp.wrapping_add(8)), &[cell, 0]);
                if (idx as i32) >= 0 {
                    let state: u32 = h.read(this.wrapping_add(0x1c));
                    let sched: u32 = h.read(this.wrapping_add(0x18));
                    let r = h.call_sym(
                        SCHEDULER_ADD,
                        Conv::Thiscall(sched),
                        &[comp, key, start, state],
                    );
                    if r != 0 {
                        return r;
                    }
                }
            }
            if ret == 0 {
                h.write(this.wrapping_add(0x2c), 1u8);
            }
        }
        if h.read::<u8>(this.wrapping_add(0x2e)) != 0 {
            let sched: u32 = h.read(this.wrapping_add(0x18));
            let comp: u32 = h.read(this.wrapping_add(0x14));
            if sched != 0 && comp != 0 && h.read::<u8>(this.wrapping_add(0x2c)) != 0 {
                let name = pstr(h, this.wrapping_add(0x10));
                h.call_sym(
                    SCHEDULER_START,
                    Conv::Thiscall(sched),
                    &[comp, this.wrapping_add(4), name, 0],
                );
                h.write(this.wrapping_add(0x2c), 0u8);
            }
        }
        ret
    }

    /// The error location globals set before `luaL_error`.
    fn error_at(h: &mut impl Host, line: u32, function: u32) {
        let file = h.rebase(STR_VECTORMATH_CPP);
        let function = h.rebase(function);
        let at = h.rebase(ERROR_FILE);
        h.write(at, file);
        let at = h.rebase(ERROR_LINE);
        h.write(at, line);
        let at = h.rebase(ERROR_FUNCTION);
        h.write(at, function);
    }

    /// `Phyre_PScripting_VectorNewIndexHandler` `0x005736c0` `(L)`: the
    /// `Vector` (argument 1) member named by a one-letter key (argument
    /// 2: `x y z w` or `0..3`) set to the number argument 3; a key on
    /// something else raises a Lua error; 0.
    pub fn vector_newindex(h: &mut impl Host, l: u32) -> u32 {
        let key = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, 2, 0]);
        if key == 0 {
            return 0;
        }
        let v = mid::check_userdata(h, STR_VECTOR, l, 1);
        if v == 0 {
            let (msg, function, line) = NEWINDEX_ERRORS[0];
            error_at(h, line, function);
            let msg = h.rebase(msg);
            h.call_sym(LUA_ERROR, Conv::Cdecl, &[l, msg, key]);
            return 0;
        }
        let c: u8 = h.read(key);
        if c != 0 && h.read::<u8>(key.wrapping_add(1)) == 0 {
            let i = small::swizzle_index(u32::from(c));
            if (i as i32) >= 0 {
                let f = h.call_sym_st0(LUA_CHECKNUMBER, Conv::Cdecl, &[l, 3]);
                h.write(v.wrapping_add(i.wrapping_mul(4)), f.to_bits());
            }
        }
        0
    }

    /// `Phyre_PScripting_MatrixNewIndexHandler` `0x00571d60` `(L)`: the
    /// `Matrix` (argument 1) row named by a one-digit key (`0..3`) set to
    /// the `Vector` argument 3; a key on something else raises a Lua
    /// error; 0.
    pub fn matrix_newindex(h: &mut impl Host, l: u32) -> u32 {
        let key = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, 2, 0]);
        if key == 0 {
            return 0;
        }
        let m = h.call_sym(CHECK_MATRIX, Conv::Cdecl, &[l, 1]);
        if m == 0 {
            let (msg, function, line) = NEWINDEX_ERRORS[1];
            error_at(h, line, function);
            let msg = h.rebase(msg);
            h.call_sym(LUA_ERROR, Conv::Cdecl, &[l, msg, key]);
            return 0;
        }
        let c: u8 = h.read(key);
        if c != 0 && h.read::<u8>(key.wrapping_add(1)) == 0 {
            let row = i32::from(c as i8).wrapping_sub(0x30) as u32;
            if row <= 3 {
                let v = mid::check_userdata(h, STR_VECTOR, l, 3);
                if v != 0 {
                    h.call_sym(MATRIX_SET_ROW, Conv::Thiscall(m), &[row, v]);
                }
            }
        }
        0
    }

    /// The script's (`+4`) name for the messages.
    fn script_name(h: &impl Host, this: u32) -> u32 {
        let script: u32 = h.read(this.wrapping_add(4));
        pstr(h, script.wrapping_add(0x1c))
    }

    /// `Phyre_PScripting_PScheduledScript_execute` `0x00618330` (thiscall
    /// `(L, function)`): the script (`+4`) loaded into `L`, made the
    /// global `phyre_pscript` and run; then, with an entry point
    /// (`+0x10` not negative), its function `function` (the script's name
    /// table `+0xc`) called with the target object (`+8`, `+0xc`). 7
    /// without a script, 1 without a state, 9 when it does not load, else
    /// 0 (errors printed).
    pub fn execute(h: &mut impl Host, this: u32, l: u32, function: u32) -> u32 {
        let script: u32 = h.read(this.wrapping_add(4));
        if script == 0 {
            return 7;
        }
        if l == 0 {
            return 1;
        }
        if mid::load_script(h, l, script) as u8 == 0 {
            return 9;
        }
        let script: u32 = h.read(this.wrapping_add(4));
        small::set_script_global(h, l, script);
        let r = h.call_sym(LUA_PCALLK, Conv::Cdecl, &[l, 0, 0, 0, 0, 0]);
        if r != 0 {
            let msg = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, u32::MAX, 0]);
            let name = script_name(h, this);
            let fmt = h.rebase(STR_PCALL_FAILED_NAMED);
            h.call_sym(PRINTF, Conv::Cdecl, &[3, fmt, r, msg, name]);
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
            return 0;
        }
        let entry: u32 = h.read(this.wrapping_add(0x10));
        if (entry as i32) < 0 {
            let name = script_name(h, this);
            let fmt = h.rebase(STR_NO_ENTRY);
            h.call_sym(PRINTF, Conv::Cdecl, &[3, fmt, name]);
            return 0;
        }
        call_function(h, this, l, function, false);
        0
    }

    /// The script's function `function` (its name table `+0xc`) called
    /// in `L` with the target object (`+8`, `+0xc`); an error printed
    /// (level 3; `named`: with the script's name, as `executePersistent`
    /// prints it) and popped, a message printed at level 2.
    fn call_function(h: &mut impl Host, this: u32, l: u32, function: u32, named: bool) -> bool {
        let script: u32 = h.read(this.wrapping_add(4));
        let names: u32 = h.read(script.wrapping_add(0xc));
        let name = pstr(h, names.wrapping_add(function.wrapping_mul(4)));
        h.call_sym(LUA_GETGLOBAL, Conv::Cdecl, &[l, name]);
        let obj: u32 = h.read(this.wrapping_add(8));
        let d: u32 = h.read(this.wrapping_add(0xc));
        h.call_sym(PUSH_OBJECT, Conv::Cdecl, &[l, obj, d]);
        let r = h.call_sym(LUA_PCALLK, Conv::Cdecl, &[l, 1, 0, 0, 0, 0]);
        let msg = h.call_sym(LUA_TOLSTRING, Conv::Cdecl, &[l, u32::MAX, 0]);
        if r != 0 {
            if named {
                let name = script_name(h, this);
                let fmt = h.rebase(STR_PCALL_FAILED_NAMED);
                h.call_sym(PRINTF, Conv::Cdecl, &[3, fmt, r, msg, name]);
            } else {
                let fmt = h.rebase(STR_PCALL_FAILED_2);
                h.call_sym(PRINTF, Conv::Cdecl, &[3, fmt, r, msg]);
            }
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
            return false;
        }
        if msg != 0 {
            let fmt = h.rebase(STR_LINE);
            h.call_sym(PRINTF, Conv::Cdecl, &[2, fmt, msg]);
        }
        true
    }

    /// `Phyre_PScripting_PScheduledScript_executePersistent` `0x00618490`
    /// (thiscall `(function)`): the script's function called in the
    /// script's own state (`+0x1c`) as [`execute`]'s second step (an
    /// error names the script), then a full garbage collection
    /// (`lua_gc(L, 2, 0)`). 7 without a script, 1 without a state, else 0.
    pub fn execute_persistent(h: &mut impl Host, this: u32, function: u32) -> u32 {
        let script: u32 = h.read(this.wrapping_add(4));
        if script == 0 {
            return 7;
        }
        let l: u32 = h.read(this.wrapping_add(0x1c));
        if l == 0 {
            return 1;
        }
        call_function(h, this, l, function, true);
        let l: u32 = h.read(this.wrapping_add(0x1c));
        h.call_sym(LUA_GC, Conv::Cdecl, &[l, 2, 0]);
        0
    }
}
