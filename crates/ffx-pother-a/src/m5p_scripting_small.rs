//! `PhyreEngine`'s scripting support outside the script layer (M5 stage 1,
//! list `phyre-scripting`, batch 1): the small functions. Descriptor
//! destructors run at exit, deleting destructors, setters, the
//! vararg-to-Lua pushers and Lua-to-value
//! poppers of `PClassCallableMethodScript::PCallerScript`, Lua userdata
//! helpers, list links and a few forwarders.
//!
//! Almost none of this ever ran in a coverage run; the parity tests
//! (`crates/ffx-oracle/tests/m5p_scripting.rs`) build every branch from
//! synthetic memory. Calls to anything outside the list (Lua's C API,
//! `PhyreFree`, the type dispenser) stay calls to the original.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `PClassDescriptor::Destruct` (thiscall, no words).
pub const DESCRIPTOR_DESTRUCT: u32 = 0x0043_b3e0;
/// `operator delete` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `operator delete` `0x00630540` (cdecl, a tail jump to the CRT's).
pub const OPERATOR_DELETE_2: u32 = 0x0063_0540;
/// `Phyre_PhyreFree` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `PStreamFile::closeFile` (thiscall).
pub const STREAM_CLOSE: u32 = 0x0060_7d80;
/// `PStreamFileWin32::isOpen` (thiscall).
pub const STREAM_IS_OPEN: u32 = 0x0060_7f20;
/// `0x00608160` (thiscall, two words): the stream file's write.
pub const STREAM_WRITE: u32 = 0x0060_8160;
/// `PStreamFileWin32::openFile` (thiscall, five words).
pub const STREAM_OPEN: u32 = 0x0060_7f40;
/// `PString::Assign` (thiscall, one word).
pub const STRING_ASSIGN: u32 = 0x0044_5cf0;
/// `lua_close(L)` (cdecl).
pub const LUA_CLOSE: u32 = 0x0095_06b0;
/// `0x0094ac00` `(L, n)` (cdecl): Lua's stack check (`lua_checkstack`).
pub const LUA_CHECKSTACK: u32 = 0x0094_ac00;
/// `lua_newuserdata(L, size)` (cdecl).
pub const LUA_NEWUSERDATA: u32 = 0x0094_b450;
/// `lua_getfield(L, idx, name)` (cdecl).
pub const LUA_GETFIELD: u32 = 0x0094_b030;
/// `0x0094bce0` `(L, idx)` (cdecl): `lua_setmetatable`.
pub const LUA_SETMETATABLE: u32 = 0x0094_bce0;
/// `lua_pushinteger(L, n)` (cdecl).
pub const LUA_PUSHINTEGER: u32 = 0x0094_b690;
/// `lua_pushnumber(L, float)` (cdecl; this Lua's numbers are `float`).
pub const LUA_PUSHNUMBER: u32 = 0x0094_b750;
/// `lua_pushnil(L)` (cdecl).
pub const LUA_PUSHNIL: u32 = 0x0094_b730;
/// `lua_pushlightuserdata(L, p)` (cdecl).
pub const LUA_PUSHLIGHTUSERDATA: u32 = 0x0094_b6c0;
/// `lua_setglobal(L, name)` (cdecl).
pub const LUA_SETGLOBAL: u32 = 0x0094_bc80;
/// `lua_getglobal(L, name)` (cdecl).
pub const LUA_GETGLOBAL: u32 = 0x0094_b080;
/// `lua_touserdata(L, idx)` (cdecl).
pub const LUA_TOUSERDATA: u32 = 0x0094_c1b0;
/// `lua_settop(L, idx)` (cdecl).
pub const LUA_SETTOP: u32 = 0x0094_bdc0;
/// `lua_tointegerx(L, idx, isnum)` (cdecl).
pub const LUA_TOINTEGERX: u32 = 0x0094_bf60;
/// `lua_tonumberx(L, idx, isnum)` (cdecl; the `float` in `ST0`).
pub const LUA_TONUMBERX: u32 = 0x0094_c040;
/// `lua_toboolean(L, idx)` (cdecl).
pub const LUA_TOBOOLEAN: u32 = 0x0094_bf00;
/// `lua_createtable(L, narr, nrec)` (cdecl).
pub const LUA_CREATETABLE: u32 = 0x0094_adc0;
/// `0x0094bd90` `(L, idx)` (cdecl): `lua_settable`.
pub const LUA_SETTABLE: u32 = 0x0094_bd90;
/// `LUA_REGISTRYINDEX` (-1001000).
pub const LUA_REGISTRYINDEX: u32 = 0xfff0_b9d8;
/// "PhyreObject": the metatable of pushed objects.
pub const STR_PHYRE_OBJECT: u32 = 0x00b0_e23c;
/// "Matrix": the metatable of matrix userdata.
pub const STR_MATRIX: u32 = 0x00b3_07bc;
/// "Vector": the metatable of vector userdata.
pub const STR_VECTOR: u32 = 0x00b3_07b4;
/// "`phyre_pscript`": the global holding the script.
pub const STR_PHYRE_PSCRIPT: u32 = 0x00b3_9148;
/// "Phyre": the namespace name.
pub const STR_PHYRE: u32 = 0x00b0_e548;
/// `Phyre_PNamespace_GetGlobalNamespace(name)` (cdecl).
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
/// `Phyre_PScripting_RegisterTypes(L, ns)` (cdecl).
pub const REGISTER_TYPES: u32 = 0x0043_6f90;
/// `0x00436e80` `(L, object, descriptor)` (cdecl): an object pushed.
pub const PUSH_OBJECT: u32 = 0x0043_6e80;
/// `Phyre_PBase_GetClassDescriptor()` (cdecl).
pub const PBASE_DESCRIPTOR: u32 = 0x0043_5740;
/// The class descriptor `0x00435580` pushes objects with.
pub const PBASE_CLASS: u32 = 0x00c9_02e8;
/// `PMatrix4_Copy(out; m)` (thiscall, one word).
pub const MATRIX_COPY: u32 = 0x0045_3340;
/// `Phyre_PhyreOS_Sleep(ms)` (cdecl).
pub const SLEEP: u32 = 0x0043_0030;
/// `crt_memcpy_thunk(dst, src, n)` (cdecl, the CRT import).
pub const MEMCPY: u32 = 0x0094_9256;
/// `Phyre_PScripting_freePairArray(data, count)` (cdecl).
pub const FREE_PAIR_ARRAY: u32 = 0x005c_fa60;
/// `ClothInstancingDynamicMesh_stageStreams(this; a, b, c)` (thiscall).
pub const STAGE_STREAMS: u32 = 0x006e_2000;
/// `0x005d01d0` (thiscall-shaped, five words): the scheduler's add.
pub const SCHEDULER_ADD: u32 = 0x005d_01d0;
/// `0x005d0280` (thiscall-shaped, six words): the scheduler's insert.
pub const SCHEDULER_INSERT: u32 = 0x005d_0280;
/// `PScriptedComponent_createThread(this; L)` (thiscall).
pub const CREATE_THREAD: u32 = 0x0055_4860;
/// `PFXAABase::initializeCommonShaders(this; a, b)` (thiscall).
pub const FXAA_INIT_SHADERS: u32 = 0x00a1_8870;
/// The shader name `0x00a1b780` passes.
pub const FXAA_SHADER: u32 = 0x0194_3da0;
/// The class descriptor `0x00a16100` looks for.
pub const POSTFX_CLASS: u32 = 0x0194_3718;
/// `0x0044ff60` (thiscall, one word): the entity link's check.
pub const ENTITY_LINK_CHECK: u32 = 0x0044_ff60;
/// `0x00450450` (thiscall, one word): the entity link's unlink.
pub const ENTITY_UNLINK: u32 = 0x0045_0450;
/// `0x00507360` (thiscall, one word): the scene link's unlink.
pub const SCENE_UNLINK: u32 = 0x0050_7360;
/// `0x005cea90` (thiscall, one word): the batch's unlink.
pub const BATCH_UNLINK: u32 = 0x005c_ea90;
/// The function pointer `0x005d6050` calls (stdcall, one word).
pub const PRINT_HOOK: u32 = 0x00cb_e6cc;

/// A destructor that stores its class's vtable and tail-jumps to the
/// parent's (`mov [ecx], vtable; jmp parent`).
pub struct TailDtor {
    /// The function.
    pub addr: u32,
    /// The vtable it stores (an `ffx.sym` address).
    pub vtable: u32,
}

/// The descriptor destructors (atexit teardown), parent
/// `PClassDescriptor::Destruct`.
pub const TAIL_DTORS: [TailDtor; 13] = [
    TailDtor {
        addr: 0x0045_a800,
        vtable: 0x00b1_1824,
    },
    TailDtor {
        addr: 0x0055_2c50,
        vtable: 0x00b2_d174,
    },
    TailDtor {
        addr: 0x0055_3a40,
        vtable: 0x00b2_d33c,
    },
    TailDtor {
        addr: 0x005c_f2f0,
        vtable: 0x00b3_89f4,
    },
    TailDtor {
        addr: 0x005d_38e0,
        vtable: 0x00b3_8ccc,
    },
    TailDtor {
        addr: 0x005d_4760,
        vtable: 0x00b3_8ea4,
    },
    TailDtor {
        addr: 0x0061_7880,
        vtable: 0x00b3_f18c,
    },
    TailDtor {
        addr: 0x0061_93d0,
        vtable: 0x00b3_f41c,
    },
    TailDtor {
        addr: 0x0067_2670,
        vtable: 0x00b4_3e04,
    },
    TailDtor {
        addr: 0x0067_3620,
        vtable: 0x00b4_4054,
    },
    TailDtor {
        addr: 0x006e_0a60,
        vtable: 0x00b4_a93c,
    },
    TailDtor {
        addr: 0x006e_0a70,
        vtable: 0x00b4_aa54,
    },
    TailDtor {
        addr: 0x006e_0a80,
        vtable: 0x00b4_a81c,
    },
];

/// A scalar deleting destructor `(this; flags)`: the vtable stored (when
/// any), the destructor called (when any; `ecx` = `this`, or the batch
/// unlink through `+0x10`), then `this` freed through `free` when
/// `flags & 1`. Returns `this`.
pub struct DeletingDtor {
    /// The function.
    pub addr: u32,
    /// The vtable it stores first, if any.
    pub vtable: Option<u32>,
    /// What it runs before the free.
    pub body: DtorBody,
    /// `operator delete` or `PhyreFree` (cdecl).
    pub free: u32,
}

/// What a [`DeletingDtor`] runs before freeing.
#[derive(Clone, Copy)]
pub enum DtorBody {
    /// Nothing.
    None,
    /// The class destructor (thiscall on `this`, no words).
    Dtor(u32),
    /// `[this+0x10]`, when set, unlinks `this` (`BATCH_UNLINK`).
    BatchUnlink,
}

/// The deleting destructors of the list.
pub const DELETING_DTORS: [DeletingDtor; 6] = [
    DeletingDtor {
        addr: 0x0043_0ad0,
        vtable: Some(0x00b0_dbbc),
        body: DtorBody::None,
        free: OPERATOR_DELETE,
    },
    DeletingDtor {
        addr: 0x0061_9540,
        vtable: Some(0x00b0_dbbc),
        body: DtorBody::None,
        free: OPERATOR_DELETE,
    },
    DeletingDtor {
        addr: 0x005c_e5f0,
        vtable: None,
        body: DtorBody::Dtor(0x005c_e550),
        free: OPERATOR_DELETE,
    },
    DeletingDtor {
        addr: 0x00a8_dbb0,
        vtable: None,
        body: DtorBody::Dtor(0x00a8_da90),
        free: OPERATOR_DELETE,
    },
    DeletingDtor {
        addr: 0x006f_9580,
        vtable: Some(0x00b4_c250),
        body: DtorBody::Dtor(0x004f_a470),
        free: PHYRE_FREE,
    },
    DeletingDtor {
        addr: 0x005c_e5c0,
        vtable: Some(0x00b3_8970),
        body: DtorBody::BatchUnlink,
        free: OPERATOR_DELETE,
    },
];

/// A bounded table lookup `(i) -> i < 0x20 ? table[i] : 0`.
pub const TABLE_GETTERS: [(u32, u32); 2] = [(0x0049_e4e0, 0x00c9_af00), (0x004c_b750, 0x00ca_43b0)];

/// `(p, &out_name, &out_len)`: the name at `p+4` and its length with the
/// NUL; returns 0.
pub const NAME_GETTERS: [u32; 4] = [0x0049_e550, 0x004c_b910, 0x004d_3b30, 0x0054_5aa0];

/// `(array, n)`: `n` 8-byte objects' scalar destructors (slot 0, flags
/// 0), then `PhyreFree(array)`.
pub const ARRAY_DESTROYERS: [u32; 4] = [0x0053_7650, 0x0053_76d0, 0x0053_7710, 0x0053_7750];

/// What a vararg pusher reads from the `va_list` and pushes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VaKind {
    /// A `char` word, sign-extended, `lua_pushinteger`.
    I8,
    /// A byte, zero-extended.
    U8,
    /// A `short`, sign-extended.
    I16,
    /// An unsigned short.
    U16,
    /// A word.
    I32,
    /// Two words, the low one pushed.
    I64,
    /// A `double` (promoted `float`) rounded to `float`, `lua_pushnumber`.
    F64,
    /// An object pointer: `PUSH_OBJECT` with `PBase`'s descriptor, or nil.
    Object,
}

/// A `PCallerScript` vararg pusher `(type, &va, L) -> al`: when `type` is
/// its type, takes the next vararg and pushes it to Lua (al 1), else al 0.
pub struct VaPush {
    /// The function.
    pub addr: u32,
    /// The type dispenser's getter it compares with (cdecl, no words).
    pub getter: u32,
    /// What it pushes.
    pub kind: VaKind,
}

/// The pushers.
pub const VA_PUSHERS: [VaPush; 12] = [
    VaPush {
        addr: 0x0061_8630,
        getter: 0x0043_7c90,
        kind: VaKind::I8,
    },
    VaPush {
        addr: 0x0061_8670,
        getter: 0x0043_7d30,
        kind: VaKind::U8,
    },
    VaPush {
        addr: 0x0061_86b0,
        getter: 0x0043_7dd0,
        kind: VaKind::I16,
    },
    VaPush {
        addr: 0x0061_86f0,
        getter: 0x0043_7e70,
        kind: VaKind::U16,
    },
    VaPush {
        addr: 0x0061_8730,
        getter: 0x0043_7f10,
        kind: VaKind::I32,
    },
    VaPush {
        addr: 0x0061_8770,
        getter: 0x0043_7fc0,
        kind: VaKind::I32,
    },
    VaPush {
        addr: 0x0061_87b0,
        getter: 0x0043_8060,
        kind: VaKind::F64,
    },
    VaPush {
        addr: 0x0061_8800,
        getter: 0x0043_8100,
        kind: VaKind::F64,
    },
    VaPush {
        addr: 0x0061_8850,
        getter: PBASE_DESCRIPTOR,
        kind: VaKind::Object,
    },
    VaPush {
        addr: 0x0061_88b0,
        getter: 0x0043_81b0,
        kind: VaKind::I32,
    },
    VaPush {
        addr: 0x0061_88f0,
        getter: 0x0043_8250,
        kind: VaKind::I64,
    },
    VaPush {
        addr: 0x0061_8940,
        getter: 0x0043_82f0,
        kind: VaKind::I64,
    },
];

/// What a Lua-to-value popper stores.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PopKind {
    /// `lua_tointegerx`'s low byte at `+4`.
    B8,
    /// Its low half-word at `+4`.
    B16,
    /// Its word at `+4`.
    B32,
    /// Its word sign-extended to 64 bits at `+8`.
    B64,
    /// `lua_tonumberx`'s `float` at `+4`.
    F32,
    /// `lua_tonumberx`'s `float` as a `double` at `+8`.
    F64,
    /// `lua_toboolean != 0` as a byte at `+4`.
    Bool,
}

/// A `PCallerScript` popper `(type, L, out) -> al`: when `type` is its
/// type, converts Lua's top, pops it and stores `{0, value}` at `out`
/// (when not null), al 1; else al 0. The upper bytes of `eax` on the
/// success path are `lua_settop`'s or `out`'s, as the copy leaves them.
pub struct Pop {
    /// The function.
    pub addr: u32,
    /// The type getter.
    pub getter: u32,
    /// What it stores.
    pub kind: PopKind,
    /// Whether `eax`'s upper bytes are `out`'s (else `lua_settop`'s).
    pub eax_out: bool,
}

/// The poppers.
pub const POPPERS: [Pop; 11] = [
    Pop {
        addr: 0x0061_8a40,
        getter: 0x0043_7c90,
        kind: PopKind::B8,
        eax_out: false,
    },
    Pop {
        addr: 0x0061_8a90,
        getter: 0x0043_7d30,
        kind: PopKind::B8,
        eax_out: false,
    },
    Pop {
        addr: 0x0061_8ae0,
        getter: 0x0043_7dd0,
        kind: PopKind::B16,
        eax_out: true,
    },
    Pop {
        addr: 0x0061_8b30,
        getter: 0x0043_7e70,
        kind: PopKind::B16,
        eax_out: true,
    },
    Pop {
        addr: 0x0061_8b80,
        getter: 0x0043_7f10,
        kind: PopKind::B32,
        eax_out: false,
    },
    Pop {
        addr: 0x0061_8bd0,
        getter: 0x0043_7fc0,
        kind: PopKind::B32,
        eax_out: false,
    },
    Pop {
        addr: 0x0061_8c20,
        getter: 0x0043_8060,
        kind: PopKind::F32,
        eax_out: true,
    },
    Pop {
        addr: 0x0061_8c70,
        getter: 0x0043_8100,
        kind: PopKind::F64,
        eax_out: true,
    },
    Pop {
        addr: 0x0061_8d00,
        getter: 0x0043_8250,
        kind: PopKind::B64,
        eax_out: false,
    },
    Pop {
        addr: 0x0061_8d50,
        getter: 0x0043_82f0,
        kind: PopKind::B64,
        eax_out: false,
    },
    Pop {
        addr: 0x0061_8da0,
        getter: 0x0043_8390,
        kind: PopKind::Bool,
        eax_out: true,
    },
];

/// `eax` after `mov al, v` over `eax`.
#[must_use]
pub const fn with_al(eax: u32, v: u8) -> u32 {
    (eax & !0xff) | v as u32
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        ARRAY_DESTROYERS, BATCH_UNLINK, CREATE_THREAD, Conv, DESCRIPTOR_DESTRUCT, DeletingDtor,
        DtorBody, ENTITY_LINK_CHECK, ENTITY_UNLINK, FREE_PAIR_ARRAY, FXAA_INIT_SHADERS,
        FXAA_SHADER, GLOBAL_NAMESPACE, Host, LUA_CHECKSTACK, LUA_CLOSE, LUA_CREATETABLE,
        LUA_GETFIELD, LUA_GETGLOBAL, LUA_NEWUSERDATA, LUA_PUSHINTEGER, LUA_PUSHLIGHTUSERDATA,
        LUA_PUSHNIL, LUA_PUSHNUMBER, LUA_REGISTRYINDEX, LUA_SETGLOBAL, LUA_SETMETATABLE,
        LUA_SETTABLE, LUA_SETTOP, LUA_TOBOOLEAN, LUA_TOINTEGERX, LUA_TONUMBERX, LUA_TOUSERDATA,
        MATRIX_COPY, MEMCPY, OPERATOR_DELETE_2, PBASE_CLASS, PBASE_DESCRIPTOR, PHYRE_FREE,
        POSTFX_CLASS, PRINT_HOOK, PUSH_OBJECT, Pop, PopKind, REGISTER_TYPES, SCENE_UNLINK,
        SCHEDULER_ADD, SCHEDULER_INSERT, SLEEP, STAGE_STREAMS, STR_MATRIX, STR_PHYRE,
        STR_PHYRE_OBJECT, STR_PHYRE_PSCRIPT, STREAM_CLOSE, STREAM_IS_OPEN, STREAM_OPEN,
        STREAM_WRITE, STRING_ASSIGN, TailDtor, VaKind, VaPush, with_al, x87,
    };

    // ---- destructors ----

    /// A [`TailDtor`]: the vtable stored, then `PClassDescriptor::Destruct`
    /// on `this`; its `eax`.
    pub fn tail_dtor(h: &mut impl Host, d: &TailDtor, this: u32) -> u32 {
        let vt = h.rebase(d.vtable);
        h.write(this, vt);
        h.call_sym(DESCRIPTOR_DESTRUCT, Conv::Thiscall(this), &[])
    }

    /// A [`DeletingDtor`]; returns `this`.
    pub fn deleting_dtor(h: &mut impl Host, d: &DeletingDtor, this: u32, flags: u32) -> u32 {
        if let Some(vt) = d.vtable {
            let vt = h.rebase(vt);
            h.write(this, vt);
        }
        match d.body {
            DtorBody::None => {}
            DtorBody::Dtor(f) => {
                h.call_sym(f, Conv::Thiscall(this), &[]);
            }
            DtorBody::BatchUnlink => {
                let batch: u32 = h.read(this.wrapping_add(0x10));
                if batch != 0 {
                    h.call_sym(BATCH_UNLINK, Conv::Thiscall(batch), &[this]);
                }
            }
        }
        if flags & 1 != 0 {
            h.call_sym(d.free, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x00a8d8b4` `(p)`: `operator delete` `0x00630540`; its `eax`.
    pub fn delete_forward(h: &mut impl Host, p: u32) -> u32 {
        h.call_sym(OPERATOR_DELETE_2, Conv::Cdecl, &[p])
    }

    /// `0x00537650` and its copies ([`ARRAY_DESTROYERS`]) `(array, n)`:
    /// nothing for a null array; else each of the `n` 8-byte objects'
    /// slot 0 with flags 0, then `PhyreFree(array)`.
    pub fn destroy_array(h: &mut impl Host, array: u32, n: u32) {
        let _ = ARRAY_DESTROYERS;
        if array == 0 {
            return;
        }
        let mut obj = array;
        let mut left = n;
        while left != 0 {
            h.call_virtual(obj, 0, &[0]);
            obj = obj.wrapping_add(8);
            left = left.wrapping_sub(1);
        }
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[array]);
    }

    /// `0x005cf2a0` (thiscall): a `PArray` of pairs released: unless bit
    /// 31 of the header marks it external, `freePairArray(data, count)`;
    /// both words zeroed. `eax`: `freePairArray`'s, or 0xfe (the `not al`
    /// of bit 31) for an external block.
    pub fn pair_array_release(h: &mut impl Host, this: u32) -> u32 {
        let head: u32 = h.read(this);
        let eax = if head & 0x8000_0000 == 0 {
            let data: u32 = h.read(this.wrapping_add(4));
            h.call_sym(FREE_PAIR_ARRAY, Conv::Cdecl, &[data, head & 0x7fff_ffff])
        } else {
            0xfe
        };
        h.write(this, 0u32);
        h.write(this.wrapping_add(4), 0u32);
        eax
    }

    // ---- constants, getters and setters ----

    /// [`super::TABLE_GETTERS`] `(i)`: `table[i]` below 0x20, else 0.
    pub fn table_get(h: &impl Host, table: u32, i: u32) -> u32 {
        if i < 0x20 {
            h.read(h.rebase(table).wrapping_add(i.wrapping_mul(4)))
        } else {
            0
        }
    }

    /// [`super::NAME_GETTERS`] `(p, &name, &len)`: `name` = the string at
    /// `p+4`, `len` its length with the NUL; returns 0.
    pub fn name_get(h: &mut impl Host, p: u32, name_out: u32, len_out: u32) -> u32 {
        let s: u32 = h.read(p.wrapping_add(4));
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        h.write(len_out, n.wrapping_add(1));
        h.write(name_out, s);
        0
    }

    /// `0x00a161b0` (thiscall): `**(this+8)`, or 0 when `+8` is null.
    pub fn first_of(h: &impl Host, this: u32) -> u32 {
        let p: u32 = h.read(this.wrapping_add(8));
        if p == 0 { 0 } else { h.read(p) }
    }

    /// `0x005534f0` / `0x00554a70` (thiscall `(v)`): `+0x14` = v; 0.
    pub fn set_14(h: &mut impl Host, this: u32, v: u32) -> u32 {
        h.write(this.wrapping_add(0x14), v);
        0
    }

    /// `0x005534d0` (`+0x18`) / `0x00553500` (`+0x1c`) (thiscall `(v)`): 1
    /// for a null `v`, else the field set and 0.
    pub fn set_nonnull(h: &mut impl Host, this: u32, offset: u32, v: u32) -> u32 {
        if v == 0 {
            return 1;
        }
        h.write(this.wrapping_add(offset), v);
        0
    }

    /// `PScriptableComponent::initialize` `0x005531d0` (thiscall `(a, b,
    /// c)`): each non-null one stored (`a` at `+0x18`, `b` at `+0x14`, `c`
    /// at `+0x1c`); 0.
    pub fn scriptable_initialize(h: &mut impl Host, this: u32, a: u32, b: u32, c: u32) -> u32 {
        if a != 0 {
            h.write(this.wrapping_add(0x18), a);
        }
        if b != 0 {
            h.write(this.wrapping_add(0x14), b);
        }
        if c != 0 {
            h.write(this.wrapping_add(0x1c), c);
        }
        0
    }

    /// `0x00553590` (thiscall): `+0x14`, `+0x18`, `+0x1c` zeroed.
    pub fn clear_script(h: &mut impl Host, this: u32) {
        h.write(this.wrapping_add(0x14), 0u32);
        h.write(this.wrapping_add(0x18), 0u32);
        h.write(this.wrapping_add(0x1c), 0u32);
    }

    /// `0x00a8e620` (thiscall): an empty `btAlignedObjectArray` (own-memory
    /// flag 1, data, size, capacity 0); returns `this`.
    pub fn aligned_array_init(h: &mut impl Host, this: u32) -> u32 {
        h.write(this.wrapping_add(0x10), 1u8);
        h.write(this.wrapping_add(0xc), 0u32);
        h.write(this.wrapping_add(4), 0u32);
        h.write(this.wrapping_add(8), 0u32);
        this
    }

    /// `0x00a8da40` (thiscall): the island manager's constructor: its
    /// vtable, three empty aligned arrays (`+4`, `+0x18`, `+0x2c`) and the
    /// flag at `+0x40` set; returns `this`.
    pub fn island_manager_ctor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(0x00b7_c78c);
        h.write(this, vt);
        aligned_array_init(h, this.wrapping_add(4));
        h.write(this.wrapping_add(0x28), 1u8);
        h.write(this.wrapping_add(0x24), 0u32);
        h.write(this.wrapping_add(0x1c), 0u32);
        h.write(this.wrapping_add(0x20), 0u32);
        h.write(this.wrapping_add(0x3c), 1u8);
        h.write(this.wrapping_add(0x38), 0u32);
        h.write(this.wrapping_add(0x30), 0u32);
        h.write(this.wrapping_add(0x34), 0u32);
        h.write(this.wrapping_add(0x40), 1u8);
        this
    }

    /// `0x005cfdc0` `(&va)`: `va_arg(va, u32)`: the cursor advanced by 4,
    /// the word before it.
    pub fn va_arg_word(h: &mut impl Host, va: u32) -> u32 {
        let p: u32 = h.read(va);
        let p = p.wrapping_add(4);
        h.write(va, p);
        h.read(p.wrapping_sub(4))
    }

    /// `0x004d3bd0` `(a, b)`: `a->+0xc < b->+0xc` (unsigned).
    pub fn less_c(h: &impl Host, a: u32, b: u32) -> u32 {
        let x: u32 = h.read(a.wrapping_add(0xc));
        let y: u32 = h.read(b.wrapping_add(0xc));
        u32::from(x < y)
    }

    /// `0x004361d0` `(list)`: the nodes of a ring list (`next` first).
    pub fn ring_count(h: &impl Host, list: u32) -> u32 {
        let mut n = 0u32;
        let mut p: u32 = h.read(list);
        while p != list {
            n = n.wrapping_add(1);
            p = h.read(p);
        }
        n
    }

    /// `0x005710b0` `(c)`: a swizzle letter's index: '0'..'3' as 0..3,
    /// 'x' 0, 'y' 1, 'z' 2, 'w' 3, anything else -1.
    #[must_use]
    pub fn swizzle_index(c: u32) -> u32 {
        let c = c as u8;
        let d = i32::from(c as i8).wrapping_sub(0x30) as u32;
        if d <= 3 {
            return d;
        }
        match c {
            b'x' => 0,
            b'y' => 1,
            b'z' => 2,
            b'w' => 3,
            _ => u32::MAX,
        }
    }

    /// `0x005d6760` `(&a, &b)`: two `PString`s compared (bit 0 of each
    /// word tags the pointer): 0 when equal, else -1 / 1 by the first
    /// differing byte (unsigned).
    pub fn string_compare(h: &impl Host, a: u32, b: u32) -> u32 {
        let pa: u32 = h.read(a);
        let pb: u32 = h.read(b);
        let mut x = pa.wrapping_sub(pa & 1);
        let mut y = pb.wrapping_sub(pb & 1);
        loop {
            let cx: u8 = h.read(x);
            let cy: u8 = h.read(y);
            if cx != cy {
                return if cx < cy { u32::MAX } else { 1 };
            }
            if cx == 0 {
                return 0;
            }
            x = x.wrapping_add(1);
            y = y.wrapping_add(1);
        }
    }

    /// `0x00a16980` (thiscall `(i)`): the `i`th pointer of the `PArray`
    /// plus 4, 0 past the count or for a null entry.
    pub fn entry_plus_4(h: &impl Host, this: u32, i: u32) -> u32 {
        match entry(h, this, i) {
            0 => 0,
            p => p.wrapping_add(4),
        }
    }

    /// `0x00a169b0` (thiscall `(i)`): the `i`th pointer's word at `+4`, 0
    /// past the count or for a null entry.
    pub fn entry_word_4(h: &impl Host, this: u32, i: u32) -> u32 {
        match entry(h, this, i) {
            0 => 0,
            p => h.read(p.wrapping_add(4)),
        }
    }

    fn entry(h: &impl Host, this: u32, i: u32) -> u32 {
        let head: u32 = h.read(this);
        if i >= head & 0x7fff_ffff {
            return 0;
        }
        let data: u32 = h.read(this.wrapping_add(4));
        h.read(data.wrapping_add(i.wrapping_mul(4)))
    }

    /// `0x00a16100` (thiscall): the first entry of the `PArray` whose
    /// class (`+4`) is [`POSTFX_CLASS`] and whose slot 4 says yes (al);
    /// 0 when none.
    pub fn find_enabled(h: &mut impl Host, this: u32) -> u32 {
        let class = h.rebase(POSTFX_CLASS);
        let mut i = 0u32;
        loop {
            let head: u32 = h.read(this);
            if i >= head & 0x7fff_ffff {
                return 0;
            }
            let data: u32 = h.read(this.wrapping_add(4));
            let e: u32 = h.read(data.wrapping_add(i.wrapping_mul(4)));
            if h.read::<u32>(e.wrapping_add(4)) == class && h.call_virtual(e, 4, &[]) & 0xff != 0 {
                return e;
            }
            i = i.wrapping_add(1);
        }
    }

    /// `0x0044db10` `(p)`: each of the `+0x3c` records from `+0x30`
    /// (stride: the descriptor `+0x2c`'s size at `+0x1c`) gets the
    /// descriptor at `+8`; 0.
    pub fn set_owner_all(h: &mut impl Host, p: u32) -> u32 {
        let mut n: u32 = h.read(p.wrapping_add(0x3c));
        let mut at: u32 = h.read(p.wrapping_add(0x30));
        let owner: u32 = h.read(p.wrapping_add(0x2c));
        let stride: u32 = h.read(owner.wrapping_add(0x1c));
        while n != 0 {
            h.write(at.wrapping_add(8), owner);
            at = at.wrapping_add(stride);
            n = n.wrapping_sub(1);
        }
        0
    }

    /// `0x005d0e90` (thiscall `(start, step)`): the 16-bit counters at
    /// `+0x84` for `start`, `start + step`, ... below 0x100 incremented.
    /// `eax`: `this`, or the counter address past the last one.
    pub fn count_every(h: &mut impl Host, this: u32, start: u32, step: u32) -> u32 {
        if step == 0 || start >= 0x100 {
            return this;
        }
        let mut i = start;
        let mut at = this.wrapping_add(i.wrapping_mul(2)).wrapping_add(0x84);
        loop {
            let v: u16 = h.read(at);
            h.write(at, v.wrapping_add(1));
            i = i.wrapping_add(step);
            at = at.wrapping_add(step.wrapping_mul(2));
            if i >= 0x100 {
                return at;
            }
        }
    }

    /// `0x005cf3f0` (thiscall on `{array, element}`): when the element is
    /// the array's last (20-byte entries at `+0x2c`, count `+0x30`), the
    /// count drops by one.
    pub fn pop_if_last(h: &mut impl Host, this: u32) {
        let array: u32 = h.read(this);
        if array == 0 {
            return;
        }
        let elem: u32 = h.read(this.wrapping_add(4));
        if elem == 0 {
            return;
        }
        let n: u32 = h.read(array.wrapping_add(0x30));
        if n == 0 {
            return;
        }
        let data: u32 = h.read(array.wrapping_add(0x2c));
        let last = data.wrapping_add(n.wrapping_mul(20)).wrapping_sub(20);
        if last == elem {
            h.write(array.wrapping_add(0x30), n.wrapping_sub(1));
        }
    }

    // ---- links ----

    /// `0x00506620` (thiscall `(node)`): `node` (`{next, owner}`) moved to
    /// the front of this list (`+8`), unlinked from its old owner first
    /// (`0x00507360`); `eax` the old head.
    pub fn scene_link(h: &mut impl Host, this: u32, node: u32) -> u32 {
        let owner: u32 = h.read(node.wrapping_add(4));
        if owner != 0 {
            h.call_sym(SCENE_UNLINK, Conv::Thiscall(owner), &[node]);
        }
        h.write(node.wrapping_add(4), this);
        let head: u32 = h.read(this.wrapping_add(8));
        h.write(node, head);
        h.write(this.wrapping_add(8), node);
        head
    }

    /// `0x00507360` (thiscall `(node)`): `node` unlinked from this list
    /// (`+8`) when it is the owner (its owner word cleared). `eax`: `node`
    /// when found, 0 when not; `None` when this is not the owner (the
    /// original leaves the caller's `eax`; its callers drop it).
    pub fn scene_unlink(h: &mut impl Host, this: u32, node: u32) -> Option<u32> {
        let owner: u32 = h.read(node.wrapping_add(4));
        if owner != this {
            return None;
        }
        let mut prev = this.wrapping_add(8);
        let mut p: u32 = h.read(prev);
        while p != 0 {
            if p == node {
                let next: u32 = h.read(p);
                h.write(prev, next);
                h.write(p, 0u32);
                h.write(node.wrapping_add(4), 0u32);
                return Some(p);
            }
            prev = p;
            p = h.read(p);
        }
        h.write(node.wrapping_add(4), 0u32);
        Some(0)
    }

    /// `0x0044fca0` (thiscall `(node)`): `node` (`{next, owner, x}`)
    /// moved to the front of this list (`+0`) unless already there: 0x12
    /// when `0x0044ff60(x)` refuses, else unlinked from its owner
    /// (`0x00450450`) and linked; 0.
    pub fn entity_link(h: &mut impl Host, this: u32, node: u32) -> u32 {
        let owner: u32 = h.read(node.wrapping_add(4));
        if owner == this {
            return 0;
        }
        let x: u32 = h.read(node.wrapping_add(8));
        if h.call_sym(ENTITY_LINK_CHECK, Conv::Thiscall(this), &[x]) as u8 == 0 {
            return 0x12;
        }
        if owner != 0 {
            h.call_sym(ENTITY_UNLINK, Conv::Thiscall(owner), &[node]);
        }
        h.write(node.wrapping_add(4), this);
        let head: u32 = h.read(this);
        h.write(node, head);
        h.write(this, node);
        0
    }

    // ---- forwarders ----

    /// `Phyre_PScripting_CloseScriptContext` `0x009f08a0` (thiscall): the
    /// stream at `+0x10` closed; its `eax`.
    pub fn close_script_context(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(STREAM_CLOSE, Conv::Thiscall(this.wrapping_add(0x10)), &[])
    }

    /// `0x009f08b0` (thiscall): whether the stream at `+0x10` is open.
    pub fn stream_is_open(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(STREAM_IS_OPEN, Conv::Thiscall(this.wrapping_add(0x10)), &[])
    }

    /// `0x009f0900` (thiscall `(a, b)`): the stream at `+0x10`'s write.
    pub fn stream_write(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        h.call_sym(
            STREAM_WRITE,
            Conv::Thiscall(this.wrapping_add(0x10)),
            &[a, b],
        )
    }

    /// `0x009f08c0` (thiscall `(path)`): the stream at `+0x10` opened for
    /// writing (`openFile(path, 0, 0, 0, 1)`), its result at `+4`, the
    /// path kept at `+8`; 1 when the open failed (0), else 0.
    pub fn stream_writer_open(h: &mut impl Host, this: u32, path: u32) -> u32 {
        let r = h.call_sym(
            STREAM_OPEN,
            Conv::Thiscall(this.wrapping_add(0x10)),
            &[path, 0, 0, 0, 1],
        );
        h.write(this.wrapping_add(4), r);
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(this.wrapping_add(8)), &[path]);
        u32::from(r == 0)
    }

    /// `0x009f0910` (thiscall): slot 1 (al), then slot 3 with the length
    /// at `+8` less its bit 0 (al); when both say yes, `+0xc` = 0. `eax`:
    /// the last slot's with al 1 or 0.
    pub fn stream_writer_flush(h: &mut impl Host, this: u32) -> u32 {
        let r = h.call_virtual(this, 1, &[]);
        if r as u8 == 0 {
            return with_al(r, 0);
        }
        let len: u32 = h.read(this.wrapping_add(8));
        let n = len.wrapping_sub(len & 1);
        let r = h.call_virtual(this, 3, &[n]);
        if r as u8 == 0 {
            return with_al(r, 0);
        }
        h.write(this.wrapping_add(0xc), 0u32);
        with_al(r, 1)
    }

    /// `0x0043b9d0` `(obj, x)`: `obj`'s slot 10 (`constructInstance`)
    /// with `x`; its `eax`.
    pub fn construct_through(h: &mut impl Host, obj: u32, x: u32) -> u32 {
        h.call_virtual(obj, 10, &[x])
    }

    /// `0x00437320` (thiscall `(x)`): slot 1 of the object at `+0x14`
    /// with `x` (a tail jump), 0 when there is none.
    pub fn forward_14(h: &mut impl Host, this: u32, x: u32) -> u32 {
        let p: u32 = h.read(this.wrapping_add(0x14));
        if p == 0 {
            return 0;
        }
        h.call_virtual(p, 1, &[x])
    }

    /// `PScripting_RegisterPhyreNamespace` `0x00436f70` `(L)`: the types
    /// of the "Phyre" namespace registered (`RegisterTypes(L, ns, name)`:
    /// the name word is still on the stack and the callee reads it); its
    /// `eax`.
    pub fn register_phyre_namespace(h: &mut impl Host, l: u32) -> u32 {
        let name = h.rebase(STR_PHYRE);
        let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[name]);
        h.call_sym(REGISTER_TYPES, Conv::Cdecl, &[l, ns, name])
    }

    /// `0x006e21c0` (thiscall-shaped, five words, `this` unused): the
    /// list cell `a4` names gets its successor (`*a4->0 = a4->1`), `a1`'s
    /// `+4` is zeroed; returns `a1`.
    pub fn unlink_cell(h: &mut impl Host, a1: u32, a4: u32) -> u32 {
        let cell: u32 = h.read(a4);
        let next: u32 = h.read(a4.wrapping_add(4));
        h.write(cell, next);
        h.write(a1.wrapping_add(4), 0u32);
        a1
    }

    /// `0x006e2190` (thiscall `(a, b, c)`): the streams staged, then slot
    /// 6 of the mesh at `+0x1c` with the same words (a tail jump).
    pub fn cloth_stage_and_draw(h: &mut impl Host, this: u32, a: u32, b: u32, c: u32) -> u32 {
        h.call_sym(STAGE_STREAMS, Conv::Thiscall(this), &[a, b, c]);
        let mesh: u32 = h.read(this.wrapping_add(0x1c));
        h.call_virtual(mesh, 6, &[a, b, c])
    }

    /// `0x005d0260` (thiscall-shaped `(a0, a1, a2, a3)`): the scheduler's
    /// add `0x005d01d0(a0, a1, a1->+8, a2, a3)`; its `eax`.
    pub fn scheduler_add(h: &mut impl Host, this: u32, a: [u32; 4]) -> u32 {
        let w: u32 = h.read(a[1].wrapping_add(8));
        h.call_sym(
            SCHEDULER_ADD,
            Conv::Thiscall(this),
            &[a[0], a[1], w, a[2], a[3]],
        )
    }

    /// `0x005d0370` (thiscall-shaped, five words): `0x005d0280` with the
    /// words and a 0; its `eax`.
    pub fn scheduler_insert(h: &mut impl Host, this: u32, a: [u32; 5]) -> u32 {
        h.call_sym(
            SCHEDULER_INSERT,
            Conv::Thiscall(this),
            &[a[0], a[1], a[2], a[3], a[4], 0],
        )
    }

    /// `0x005d6050` `(x)`: the print hook (stdcall) with `x` when one is
    /// set (1), else 0.
    pub fn print_hook(h: &mut impl Host, x: u32) -> u32 {
        let f: u32 = h.read(h.rebase(PRINT_HOOK));
        if f == 0 {
            return 0;
        }
        h.call(f, Conv::Stdcall, &[x]);
        1
    }

    /// `0x00618130` (thiscall): `memcpy(+4, +0xc, +8 & 0x7fffffff)`;
    /// returns `+0x10`.
    pub fn copy_block(h: &mut impl Host, this: u32) -> u32 {
        let n: u32 = h.read(this.wrapping_add(8));
        let src: u32 = h.read(this.wrapping_add(0xc));
        let dst: u32 = h.read(this.wrapping_add(4));
        h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, n & 0x7fff_ffff]);
        h.read(this.wrapping_add(0x10))
    }

    /// `Phyre_PScripting_PAsyncProcessHeader_process` `0x00617f30`
    /// (thiscall): a header not yet run (state `+0x10` at most 1) runs its
    /// function (`+0xc`, cdecl on the header) and becomes 3; that
    /// function's `eax`, else 0.
    pub fn async_process(h: &mut impl Host, this: u32) -> u32 {
        let state: u32 = h.read(this.wrapping_add(0x10));
        if state > 1 {
            return 0;
        }
        let f: u32 = h.read(this.wrapping_add(0xc));
        let r = h.call(f, Conv::Cdecl, &[this]);
        h.write(this.wrapping_add(0x10), 3u32);
        r
    }

    /// `0x00617e40` (thiscall `(out)`): the header processed as
    /// [`async_process`], then waited for (`PhyreOS::Sleep(1)` until the
    /// state is 3); its two first words copied to `out`. Returns `out`.
    pub fn async_wait(h: &mut impl Host, this: u32, out: u32) -> u32 {
        let state: u32 = h.read(this.wrapping_add(0x10));
        if state <= 1 {
            let f: u32 = h.read(this.wrapping_add(0xc));
            h.call(f, Conv::Cdecl, &[this]);
            h.write(this.wrapping_add(0x10), 3u32);
        }
        while h.read::<u32>(this.wrapping_add(0x10)) != 3 {
            h.call_sym(SLEEP, Conv::Cdecl, &[1]);
        }
        let a: u32 = h.read(this);
        let b: u32 = h.read(this.wrapping_add(4));
        h.write(out, a);
        h.write(out.wrapping_add(4), b);
        out
    }

    /// `0x00524850` `(a, p)`: slot 1 of the object `p` names with `a` and
    /// the two floats at `p+4`, `p+8` (through the x87); its `eax`.
    pub fn event_forward(h: &mut impl Host, a: u32, p: u32) -> u32 {
        let obj: u32 = h.read(p);
        let x: u32 = h.read(p.wrapping_add(4));
        let y: u32 = h.read(p.wrapping_add(8));
        let x = x87::store(x87::load(x));
        let y = x87::store(x87::load(y));
        h.call_virtual(obj, 1, &[a, x, y])
    }

    /// `PScriptedComponent_initialize` `0x00554830` (thiscall `(a, b,
    /// L)`): [`scriptable_initialize`], then `createThread(L)`; its `eax`.
    pub fn scripted_initialize(h: &mut impl Host, this: u32, a: u32, b: u32, l: u32) -> u32 {
        scriptable_initialize(h, this, a, b, l);
        h.call_sym(CREATE_THREAD, Conv::Thiscall(this), &[l])
    }

    /// `0x00a1b780` (thiscall): an FXAA effect with a context whose first
    /// word is set initialises its shaders (`initializeCommonShaders(
    /// FXAA_SHADER, 0x00a161b0's)`); on failure (0) or without a context,
    /// `+0xc` = `+8` and 0. Else the call's `eax`.
    pub fn fxaa_init(h: &mut impl Host, this: u32) -> u32 {
        let ctx: u32 = h.read(this.wrapping_add(8));
        if ctx != 0 && h.read::<u32>(ctx) != 0 {
            let first = super::bridge::first_of(h, this);
            let shader = h.rebase(FXAA_SHADER);
            let r = h.call_sym(FXAA_INIT_SHADERS, Conv::Thiscall(this), &[shader, first]);
            if r != 0 {
                return r;
            }
        }
        let ctx: u32 = h.read(this.wrapping_add(8));
        h.write(this.wrapping_add(0xc), ctx);
        0
    }

    /// `0x005d5450` (`jmp lua_close`) `(L)`.
    pub fn close_state(h: &mut impl Host, l: u32) -> u32 {
        h.call_sym(LUA_CLOSE, Conv::Cdecl, &[l])
    }

    // ---- Lua ----

    /// `0x005d5330` `(L, p)`: `p` as the light userdata global
    /// "`phyre_pscript`"; `lua_setglobal`'s `eax`.
    pub fn set_script_global(h: &mut impl Host, l: u32, p: u32) -> u32 {
        h.call_sym(LUA_PUSHLIGHTUSERDATA, Conv::Cdecl, &[l, p]);
        let name = h.rebase(STR_PHYRE_PSCRIPT);
        h.call_sym(LUA_SETGLOBAL, Conv::Cdecl, &[l, name])
    }

    /// `Phyre_PScripting_GetPScriptFromContext` `0x005d5780` `(L)`: the
    /// global "`phyre_pscript`"'s userdata.
    pub fn get_pscript(h: &mut impl Host, l: u32) -> u32 {
        let name = h.rebase(STR_PHYRE_PSCRIPT);
        h.call_sym(LUA_GETGLOBAL, Conv::Cdecl, &[l, name]);
        let p = h.call_sym(LUA_TOUSERDATA, Conv::Cdecl, &[l, u32::MAX]);
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        p
    }

    /// `0x00436e80` `(L, object, descriptor)`: an 8-byte userdata
    /// `{descriptor, object}` with the `PhyreObject` metatable;
    /// `lua_setmetatable`'s `eax`.
    pub fn push_object(h: &mut impl Host, l: u32, object: u32, descriptor: u32) -> u32 {
        h.call_sym(LUA_CHECKSTACK, Conv::Cdecl, &[l, 1]);
        let u = h.call_sym(LUA_NEWUSERDATA, Conv::Cdecl, &[l, 8]);
        if u != 0 {
            h.write(u, descriptor);
            h.write(u.wrapping_add(4), object);
        }
        set_metatable(h, l, STR_PHYRE_OBJECT)
    }

    /// `0x00436ed0` `(L, obj)`: as [`push_object`] with the object's own
    /// descriptor (`+8`).
    pub fn push_self(h: &mut impl Host, l: u32, obj: u32) -> u32 {
        let d: u32 = h.read(obj.wrapping_add(8));
        push_object(h, l, obj, d)
    }

    fn set_metatable(h: &mut impl Host, l: u32, name: u32) -> u32 {
        let name = h.rebase(name);
        h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
        h.call_sym(LUA_SETMETATABLE, Conv::Cdecl, &[l, 0xffff_fffe])
    }

    /// `0x00435580` `(L, &p)`: the object `p` pushed with `PBase`'s
    /// descriptor `0x00c902e8`, or nil; the last call's `eax`.
    pub fn push_base(h: &mut impl Host, l: u32, pp: u32) -> u32 {
        let p: u32 = h.read(pp);
        if p != 0 {
            let d = h.rebase(PBASE_CLASS);
            return h.call_sym(PUSH_OBJECT, Conv::Cdecl, &[l, p, d]);
        }
        h.call_sym(LUA_CHECKSTACK, Conv::Cdecl, &[l, 1]);
        h.call_sym(LUA_PUSHNIL, Conv::Cdecl, &[l])
    }

    /// `0x00571030` (`size` 0x40, "Matrix") / `0x00571070` (0x10,
    /// "Vector") `(L)`: a userdata with that metatable; the userdata or 0.
    pub fn new_userdata(h: &mut impl Host, l: u32, size: u32, name: u32) -> u32 {
        let u = h.call_sym(LUA_NEWUSERDATA, Conv::Cdecl, &[l, size]);
        if u != 0 {
            set_metatable(h, l, name);
        }
        u
    }

    /// `0x00571f50` `(L, m)`: a new "Matrix" userdata holding a copy of
    /// `m`; `PMatrix4_Copy`'s `eax`, or 0.
    pub fn push_matrix(h: &mut impl Host, l: u32, m: u32) -> u32 {
        let u = h.call_sym(LUA_NEWUSERDATA, Conv::Cdecl, &[l, 0x40]);
        if u == 0 {
            return 0;
        }
        set_metatable(h, l, STR_MATRIX);
        h.call_sym(MATRIX_COPY, Conv::Thiscall(u), &[m])
    }

    /// `0x00436f20` `(L, n, array, type)`: a table of the `n` words of
    /// `array` (indices from 1), each pushed through `type`'s slot 6
    /// (`(L, &word)`); the last call's `eax`.
    pub fn push_array(h: &mut impl Host, l: u32, n: u32, array: u32, ty: u32) -> u32 {
        let mut eax = h.call_sym(LUA_CREATETABLE, Conv::Cdecl, &[l, n, 0]);
        let mut i = 0u32;
        let mut at = array;
        while i < n {
            i = i.wrapping_add(1);
            h.call_sym(LUA_PUSHINTEGER, Conv::Cdecl, &[l, i]);
            h.call_virtual(ty, 6, &[l, at]);
            eax = h.call_sym(LUA_SETTABLE, Conv::Cdecl, &[l, 0xffff_fffd]);
            at = at.wrapping_add(4);
        }
        eax
    }

    /// A [`VaPush`] `(type, &va, L)`.
    pub fn va_push(h: &mut impl Host, p: &VaPush, ty: u32, va: u32, l: u32) -> u32 {
        let mut want = h.call_sym(p.getter, Conv::Cdecl, &[]);
        if p.kind == VaKind::Object {
            want |= 1;
        }
        if ty != want {
            return with_al(want, 0);
        }
        let step = match p.kind {
            VaKind::I64 | VaKind::F64 => 8,
            _ => 4,
        };
        let cur: u32 = h.read(va);
        let cur = cur.wrapping_add(step);
        h.write(va, cur);
        let at = cur.wrapping_sub(step);
        let word: u32 = h.read(at);
        let eax = match p.kind {
            VaKind::Object => {
                let d = h.call_sym(PBASE_DESCRIPTOR, Conv::Cdecl, &[]);
                if word != 0 {
                    h.call_sym(PUSH_OBJECT, Conv::Cdecl, &[l, word, d])
                } else {
                    h.call_sym(LUA_CHECKSTACK, Conv::Cdecl, &[l, 1]);
                    h.call_sym(LUA_PUSHNIL, Conv::Cdecl, &[l])
                }
            }
            VaKind::F64 => {
                let lo = u64::from(word);
                let hi: u32 = h.read(at.wrapping_add(4));
                let f = x87::store(f64::from_bits(lo | u64::from(hi) << 32));
                h.call_sym(LUA_CHECKSTACK, Conv::Cdecl, &[l, 1]);
                h.call_sym(LUA_PUSHNUMBER, Conv::Cdecl, &[l, f])
            }
            kind => {
                let v = match kind {
                    VaKind::I8 => i32::from(word as u8 as i8) as u32,
                    VaKind::U8 => word & 0xff,
                    VaKind::I16 => i32::from(word as u16 as i16) as u32,
                    VaKind::U16 => word & 0xffff,
                    _ => word,
                };
                h.call_sym(LUA_CHECKSTACK, Conv::Cdecl, &[l, 1]);
                h.call_sym(LUA_PUSHINTEGER, Conv::Cdecl, &[l, v])
            }
        };
        with_al(eax, 1)
    }

    /// A [`Pop`] `(type, L, out)`.
    pub fn pop(h: &mut impl Host, p: &Pop, ty: u32, l: u32, out: u32) -> u32 {
        let want = h.call_sym(p.getter, Conv::Cdecl, &[]);
        if ty != want {
            return with_al(want, 0);
        }
        let mut f64_bits = 0u64;
        let mut word = 0u32;
        match p.kind {
            PopKind::F32 => {
                word = h
                    .call_sym_st0(LUA_TONUMBERX, Conv::Cdecl, &[l, u32::MAX, 0])
                    .to_bits();
            }
            PopKind::F64 => {
                f64_bits = h
                    .call_sym_st0_f64(LUA_TONUMBERX, Conv::Cdecl, &[l, u32::MAX, 0])
                    .to_bits();
            }
            PopKind::Bool => {
                word = u32::from(h.call_sym(LUA_TOBOOLEAN, Conv::Cdecl, &[l, u32::MAX]) != 0);
            }
            _ => {
                word = h.call_sym(LUA_TOINTEGERX, Conv::Cdecl, &[l, u32::MAX, 0]);
            }
        }
        let top = h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        if out != 0 {
            h.write(out, 0u32);
            match p.kind {
                PopKind::B8 | PopKind::Bool => h.write(out.wrapping_add(4), word as u8),
                PopKind::B16 => h.write(out.wrapping_add(4), word as u16),
                PopKind::B32 | PopKind::F32 => h.write(out.wrapping_add(4), word),
                PopKind::B64 => {
                    h.write(out.wrapping_add(8), word);
                    h.write(out.wrapping_add(0xc), ((word as i32) >> 31) as u32);
                }
                PopKind::F64 => h.write(out.wrapping_add(8), f64_bits),
            }
        }
        with_al(if p.eax_out { out } else { top }, 1)
    }
}
