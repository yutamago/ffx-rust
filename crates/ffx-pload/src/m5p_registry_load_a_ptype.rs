//! `PType` and `PTypeDefault<T>` (M5 stage 1, list
//! `phyre-registry-load-a`): the primitive types' virtual methods.
//!
//! Every primitive type (`char`, `PUInt8`, `PInt16`, `PUInt16`, `PInt32`,
//! `PUInt32`, `float`, `double`, `PQWord` (16 bytes), `PInt64`, `PUInt64`,
//! `bool`) is a `PTypeDefault<T>` whose vtable holds, per type:
//!
//! - the scalar deleting destructor (`thiscall(flags) ret 4`): the vtable
//!   back to `PType`'s (`0x00b0e568`), `PhyreFree(this)` when bit 0 of the
//!   flags is set, `eax` = this ([`TYPES`] `.dtor`);
//! - `getValueFromScriptContext(L, out)` (`ret 8`, `ecx` unused): the Lua
//!   value on top of the stack converted and stored at `out`, then
//!   `lua_settop(L, -2)` pops it ([`get_from_script`]);
//! - `putValueToScriptContext(L, value)` (`ret 8`, `ecx` unused):
//!   `lua_checkstack(L, 1)`, then the value pushed ([`put_to_script`]);
//! - `swapEndian(p, n)` (`ret 8`, `ecx` unused): the value at `p`
//!   byte-swapped in place; `al` = 1 ([`swap_endian`]).
//!
//! Lua's number is a `float` in this build (`lua_tonumberx` ends in `fld
//! dword`, `lua_pushnumber` takes a `float` word).

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

/// `PType`'s vtable (`Phyre_PType_vftable`).
pub const PTYPE_VTABLE: u32 = 0x00b0_e568;
/// `PhyreFree` (cdecl, one word).
pub const FREE: u32 = 0x0042_fb20;
/// `lua_checkstack(L, n)` (cdecl).
pub const LUA_CHECKSTACK: u32 = 0x0094_ac00;
/// `lua_tointegerx(L, index, isnum)` (cdecl).
pub const LUA_TOINTEGERX: u32 = 0x0094_bf60;
/// `lua_tonumberx(L, index, isnum)` (cdecl, `float` in `ST0`).
pub const LUA_TONUMBERX: u32 = 0x0094_c040;
/// `lua_toboolean(L, index)` (cdecl).
pub const LUA_TOBOOLEAN: u32 = 0x0094_bf00;
/// `lua_settop(L, index)` (cdecl).
pub const LUA_SETTOP: u32 = 0x0094_bdc0;
/// `lua_pushinteger(L, n)` (cdecl).
pub const LUA_PUSHINTEGER: u32 = 0x0094_b690;
/// `lua_pushnumber(L, float)` (cdecl).
pub const LUA_PUSHNUMBER: u32 = 0x0094_b750;
/// `lua_pushboolean(L, b)` (cdecl).
pub const LUA_PUSHBOOLEAN: u32 = 0x0094_b5a0;
/// `PTypeDispenser::Instance()` (cdecl).
pub const DISPENSER_INSTANCE: u32 = 0x0043_9040;
/// `PTypeDispenser::findByName(name)` (thiscall, one word).
pub const DISPENSER_FIND: u32 = 0x0043_7570;
/// `PType::GetFixupForPTypePointer`'s error for a descriptor whose flags
/// bit 0 is set: 0x12.
pub const FIXUP_REFUSED: u32 = 0x12;
/// `PType::ResolveFixupForPTypePointer`'s error for a name it can't
/// resolve: 0x13.
pub const NOT_FOUND: u32 = 0x13;

/// A primitive type's value as its methods see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `char`.
    I8,
    /// `PUInt8`.
    U8,
    /// `PInt16`.
    I16,
    /// `PUInt16`.
    U16,
    /// `PInt32`.
    I32,
    /// `PUInt32`.
    U32,
    /// `float`.
    F32,
    /// `double`.
    F64,
    /// `PQWord`: 16 bytes.
    QWord,
    /// `PInt64`.
    I64,
    /// `PUInt64`.
    U64,
    /// `bool`.
    Bool,
}

/// One `PTypeDefault<T>`'s methods by address (0 where its slot is a
/// shared or unhookable function outside this list).
#[derive(Debug, Clone, Copy)]
pub struct Type {
    /// `T`'s name.
    pub name: &'static str,
    /// The value.
    pub kind: Kind,
    /// Its scalar deleting destructor.
    pub dtor: u32,
    /// `getValueFromScriptContext`.
    pub get: u32,
    /// `putValueToScriptContext`.
    pub put: u32,
    /// `swapEndian`.
    pub swap: u32,
}

/// The twelve primitive types, in their vtables' order.
pub const TYPES: [Type; 12] = [
    Type {
        name: "char",
        kind: Kind::I8,
        dtor: 0x0043_8e00,
        get: 0x0043_9670,
        put: 0x0043_9910,
        swap: 0x0043_9bd0,
    },
    Type {
        name: "PUInt8",
        kind: Kind::U8,
        dtor: 0x0043_8e30,
        get: 0x0043_96a0,
        put: 0x0043_9940,
        swap: 0x0043_9be0,
    },
    Type {
        name: "PInt16",
        kind: Kind::I16,
        dtor: 0x0043_8e60,
        get: 0x0043_96d0,
        put: 0x0043_9970,
        swap: 0x0043_9bf0,
    },
    Type {
        name: "PUInt16",
        kind: Kind::U16,
        dtor: 0x0043_8e90,
        get: 0x0043_9700,
        put: 0x0043_99a0,
        swap: 0x0043_9c20,
    },
    Type {
        name: "PInt32",
        kind: Kind::I32,
        dtor: 0x0043_8ec0,
        get: 0x0043_9730,
        put: 0x0043_99d0,
        swap: 0x0043_9c40,
    },
    Type {
        name: "PUInt32",
        kind: Kind::U32,
        dtor: 0x0043_8ef0,
        get: 0x0043_9760,
        put: 0x0043_9a00,
        swap: 0x0043_9c80,
    },
    Type {
        name: "float",
        kind: Kind::F32,
        dtor: 0x0043_8f20,
        get: 0x0043_9790,
        put: 0x0043_9a30,
        swap: 0x0043_9cc0,
    },
    Type {
        name: "double",
        kind: Kind::F64,
        dtor: 0x0043_8f50,
        get: 0x0043_97c0,
        put: 0x0043_9a60,
        swap: 0x0043_9d00,
    },
    Type {
        name: "PQWord",
        kind: Kind::QWord,
        dtor: 0x0043_8f80,
        get: 0x0043_97f0,
        put: 0x0043_9a90,
        swap: 0x0043_9d30,
    },
    Type {
        name: "PInt64",
        kind: Kind::I64,
        dtor: 0x0043_8fb0,
        get: 0x0043_9840,
        put: 0x0043_9ac0,
        swap: 0x0043_9d70,
    },
    Type {
        name: "PUInt64",
        kind: Kind::U64,
        dtor: 0x0043_8fe0,
        get: 0x0043_9880,
        put: 0x0043_9af0,
        swap: 0x0043_9d90,
    },
    Type {
        name: "bool",
        kind: Kind::Bool,
        dtor: 0x0043_9010,
        get: 0x0043_98c0,
        put: 0x0043_9b20,
        swap: 0x0043_9db0,
    },
];

/// `PType`'s own scalar deleting destructor (`0x00437710`), the same code
/// as the twelve in [`TYPES`].
pub const PTYPE_DTOR: u32 = 0x0043_7710;

/// Pure parts.
pub mod core {
    /// `0x004377d0` `cdecl(name, seed, len)`: the type-name hash, `h = 33
    /// h + (b & 0x1f)` over `len` bytes, or up to the terminating zero when
    /// `len` is 0. A null name gives the seed.
    #[must_use]
    pub fn name_hash(seed: u32, bytes: impl IntoIterator<Item = u8>) -> u32 {
        bytes.into_iter().fold(seed, |h, b| {
            h.wrapping_mul(33).wrapping_add(u32::from(b & 0x1f))
        })
    }

    /// `0x00439230` / `0x004392c0` `cdecl(lo, hi)`: the 64-bit value with
    /// its bytes reversed, in `edx:eax`. The `PInt64` version (`0x00439230`)
    /// shifts arithmetically and masks each byte, so both give the plain
    /// byte swap.
    #[must_use]
    pub const fn swap64(v: u64) -> u64 {
        v.swap_bytes()
    }
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        Conv, DISPENSER_FIND, DISPENSER_INSTANCE, FIXUP_REFUSED, FREE, Host, Kind, LUA_CHECKSTACK,
        LUA_PUSHBOOLEAN, LUA_PUSHINTEGER, LUA_PUSHNUMBER, LUA_SETTOP, LUA_TOBOOLEAN,
        LUA_TOINTEGERX, LUA_TONUMBERX, Mem, NOT_FOUND, PTYPE_VTABLE, core, x87,
    };

    /// `PType::~PType` (`0x00437700`, thiscall, no words): the vtable back
    /// to `PType`'s. Leaves `eax` alone.
    pub fn ptype_dtor(m: &mut impl Mem, this: u32) {
        let vt = m.rebase(PTYPE_VTABLE);
        m.write(this, vt);
    }

    /// A `PType`/`PTypeDefault<T>` scalar deleting destructor
    /// (`thiscall(flags) ret 4`): [`ptype_dtor`], then `PhyreFree(this)`
    /// when bit 0 of the flags is set; returns this.
    pub fn delete_type(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        ptype_dtor(h, this);
        if flags & 1 != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `PType::PType(a, b, c)` (`0x00438cc0`, thiscall, `ret 0xc`): an
    /// empty list at `+4` (three links to itself, two zero words), the
    /// three arguments at `+0x18`..`+0x20`, `PType`'s vtable, `+0x24` and
    /// `+0x28` zero; returns this.
    pub fn construct(m: &mut impl Mem, this: u32, a: u32, b: u32, c: u32) -> u32 {
        let list = this + 4;
        m.write(list, list);
        m.write(list + 4, list);
        m.write(list + 8, list);
        m.write(list + 0xc, 0u32);
        m.write(list + 0x10, 0u32);
        m.write(this + 0x18, a);
        m.write(this + 0x1c, b);
        m.write(this + 0x20, c);
        let vt = m.rebase(PTYPE_VTABLE);
        m.write(this, vt);
        m.write(this + 0x24, 0u32);
        m.write(this + 0x28, 0u32);
        this
    }

    /// `PType::GetFixupForPTypePointer(type, &name, &len)` (`0x00437740`,
    /// cdecl): `0x12` when the type's class descriptor (its slot 1) has
    /// flags bit 0; else its name (`+0x18`) and the name's length with the
    /// zero, 0.
    pub fn get_fixup(h: &mut impl Host, ty: u32, name_out: u32, len_out: u32) -> u32 {
        let desc = h.call_virtual(ty, 1, &[]);
        if desc != 0 {
            let flags: u8 = h.read(desc + 0x90);
            if flags & 1 != 0 {
                return FIXUP_REFUSED;
            }
        }
        let name: u32 = h.read(ty + 0x18);
        let mut n = 0u32;
        while h.read::<u8>(name.wrapping_add(n)) != 0 {
            n += 1;
        }
        h.write(len_out, n + 1);
        h.write(name_out, name);
        0
    }

    /// `PType::ResolveFixupForPTypePointer(out, name)` (`0x00437830`,
    /// cdecl): the type the dispenser has under `name` (0 for a null
    /// name) into `out`; 0, or `0x13` when there is none.
    pub fn resolve_fixup(h: &mut impl Host, out: u32, name: u32) -> u32 {
        let ty = if name == 0 {
            0
        } else {
            let d = h.call_sym(DISPENSER_INSTANCE, Conv::Cdecl, &[]);
            h.call_sym(DISPENSER_FIND, Conv::Thiscall(d), &[name])
        };
        h.write(out, ty);
        if ty == 0 { NOT_FOUND } else { 0 }
    }

    /// `0x004377d0` `cdecl(name, seed, len)`: [`core::name_hash`] over
    /// `len` bytes, or the zero-terminated name when `len` is 0.
    pub fn name_hash(m: &impl Mem, name: u32, seed: u32, len: u32) -> u32 {
        if name == 0 {
            return seed;
        }
        if len != 0 {
            core::name_hash(seed, (0..len).map(|i| m.read::<u8>(name.wrapping_add(i))))
        } else {
            let mut h = seed;
            let mut p = name;
            loop {
                let b: u8 = m.read(p);
                if b == 0 {
                    return h;
                }
                h = core::name_hash(h, [b]);
                p = p.wrapping_add(1);
            }
        }
    }

    /// `getValueFromScriptContext(L, out)` of `kind`: the value on top of
    /// Lua's stack (`lua_tointegerx(L, -1, 0)`, `lua_tonumberx` for the
    /// floats, `lua_toboolean(L, -1) != 0`), `lua_settop(L, -2)`, then the
    /// store: the integer's low bytes, `cdq` to 64 bits (`PQWord` adds 8
    /// zero bytes), the `float` as stored, the `double` from the `float`.
    /// `eax`: `lua_settop`'s, or `out` (`PInt16`, `PUInt16`, the floats,
    /// `bool`), or the high word (`PQWord`).
    pub fn get_from_script(h: &mut impl Host, kind: Kind, l: u32, out: u32) -> u32 {
        const TOP: u32 = (-1i32).cast_unsigned();
        const POP: u32 = (-2i32).cast_unsigned();
        match kind {
            Kind::F32 | Kind::F64 => {
                let x = h.call_sym_st0(LUA_TONUMBERX, Conv::Cdecl, &[l, TOP, 0]);
                h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, POP]);
                if kind == Kind::F32 {
                    h.write(out, x87::store(x87::load(x.to_bits())));
                } else {
                    h.write(out, f64::from(x));
                }
                out
            }
            Kind::Bool => {
                let b = h.call_sym(LUA_TOBOOLEAN, Conv::Cdecl, &[l, TOP]);
                h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, POP]);
                h.write(out, u8::from(b != 0));
                out
            }
            _ => {
                let v = h.call_sym(LUA_TOINTEGERX, Conv::Cdecl, &[l, TOP, 0]);
                let r = h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, POP]);
                let wide = u64::from(v)
                    | if v & 0x8000_0000 != 0 {
                        0xffff_ffff << 32
                    } else {
                        0
                    };
                match kind {
                    Kind::I8 | Kind::U8 => h.write(out, v as u8),
                    Kind::I16 | Kind::U16 => {
                        h.write(out, v as u16);
                        return out;
                    }
                    Kind::I32 | Kind::U32 => h.write(out, v),
                    Kind::I64 | Kind::U64 => h.write(out, wide),
                    _ => {
                        h.write(out, wide);
                        h.write(out + 8, 0u64);
                        return (wide >> 32) as u32;
                    }
                }
                r
            }
        }
    }

    /// `putValueToScriptContext(L, value)` of `kind`: `lua_checkstack(L,
    /// 1)`, then `lua_pushinteger` of the value widened to 32 bits (the low
    /// word of the 64-bit ones), `lua_pushnumber` of the `float` (the
    /// `double` rounded to `float`, through the original's own argument
    /// slot), `lua_pushboolean` of the byte. Returns the push's `eax`.
    pub fn put_to_script(h: &mut impl Host, kind: Kind, l: u32, value: u32) -> u32 {
        h.call_sym(LUA_CHECKSTACK, Conv::Cdecl, &[l, 1]);
        match kind {
            Kind::F32 => {
                let x = x87::store(x87::load(h.read(value)));
                h.call_sym(LUA_PUSHNUMBER, Conv::Cdecl, &[l, x])
            }
            Kind::F64 => {
                let d: f64 = h.read(value);
                let x = x87::store(d);
                h.call_sym(LUA_PUSHNUMBER, Conv::Cdecl, &[l, x])
            }
            Kind::Bool => {
                let b: u8 = h.read(value);
                h.call_sym(LUA_PUSHBOOLEAN, Conv::Cdecl, &[l, u32::from(b)])
            }
            _ => {
                let v = match kind {
                    Kind::I8 => i32::from(h.read::<i8>(value)).cast_unsigned(),
                    Kind::U8 => u32::from(h.read::<u8>(value)),
                    Kind::I16 => i32::from(h.read::<i16>(value)).cast_unsigned(),
                    Kind::U16 => u32::from(h.read::<u16>(value)),
                    _ => h.read(value),
                };
                h.call_sym(LUA_PUSHINTEGER, Conv::Cdecl, &[l, v])
            }
        }
    }

    /// `swapEndian(p, n)` of `kind`: the value at `p` byte-swapped (a
    /// `PQWord` swaps its two halves too; the floats go through the x87
    /// first, which quiets a signalling NaN). Returns `eax`: `al` = 1
    /// over what the swap left in the register.
    pub fn swap_endian(m: &mut impl Mem, kind: Kind, p: u32) -> u32 {
        match kind {
            Kind::I8 | Kind::U8 | Kind::Bool => 1,
            Kind::I16 | Kind::U16 => {
                let w: u16 = m.read(p);
                m.write(p, w.swap_bytes());
                if kind == Kind::I16 {
                    (u32::from(w) << 8) & 0x00ff_ff00 | 1
                } else {
                    u32::from(w.swap_bytes()) & 0xff00 | 1
                }
            }
            Kind::I32 => {
                let v: u32 = m.read(p);
                m.write(p, v.swap_bytes());
                (v >> 8) & 0xff00 | 1
            }
            Kind::U32 | Kind::F32 => {
                let raw: u32 = m.read(p);
                let v = if kind == Kind::F32 {
                    x87::store(x87::load(raw))
                } else {
                    raw
                };
                m.write(p, v.swap_bytes());
                if kind == Kind::F32 {
                    (v << 16) & 0xffff_ff00 | 1
                } else {
                    (((v & 0xff00) | (v << 16)) << 8) & 0xffff_ff00 | 1
                }
            }
            Kind::F64 | Kind::I64 | Kind::U64 => {
                let raw: u64 = m.read(p);
                let v = if kind == Kind::F64 { quiet64(raw) } else { raw };
                let s = core::swap64(v);
                m.write(p, s);
                (s as u32) & 0xffff_ff00 | 1
            }
            Kind::QWord => {
                let lo: u64 = m.read(p);
                let hi: u64 = m.read(p + 8);
                let new_lo = core::swap64(hi);
                let new_hi = core::swap64(lo);
                m.write(p, new_lo);
                m.write(p + 8, new_hi);
                (new_hi as u32) & 0xffff_ff00 | 1
            }
        }
    }

    /// `fld qword; fstp qword`: a signalling NaN comes out quiet.
    fn quiet64(bits: u64) -> u64 {
        let nan = bits & 0x7ff0_0000_0000_0000 == 0x7ff0_0000_0000_0000
            && bits & 0x000f_ffff_ffff_ffff != 0;
        if nan { bits | 1 << 51 } else { bits }
    }

    /// `0x00439230` / `0x004392c0` `cdecl(lo, hi)`: [`core::swap64`].
    #[must_use]
    pub fn swap64(lo: u32, hi: u32) -> u64 {
        core::swap64(u64::from(lo) | u64::from(hi) << 32)
    }
}
