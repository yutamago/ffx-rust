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

/// The ports over game memory.
pub mod bridge {
    use super::{
        CLOTH_INSTANCE, Conv, FIND_MEMBER, FREE, Host, LOAD_CALLBACKS, LOADED_TABLE, LUA_GETFIELD,
        LUA_GETGLOBAL, LUA_PCALLK, LUA_PUSHNIL, LUA_REGISTRYINDEX, LUA_SETFIELD, LUA_SETGLOBAL,
        LUA_SETTOP, LUA_TOBOOLEAN, LUA_TOLSTRING, LUA_TYPE, LUAL_CHECKLSTRING, MALLOC, MEMCPY,
        PRINTF, RELOAD_CALLS, RELOAD_DATA, REQUIRE_CALLS, STRCHR_SLOT,
    };

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
}
