//! M5 stage 1, list `render`, batch 6: the class descriptors and Lua
//! script bindings of the game's dynamic meshes (distortion grid, radial
//! line, broken screen, shadow mesh and their instances). Each class has
//! the same small template functions with its own vtable, accessor and
//! class record, which the vtables and binding tables reach:
//!
//! - the descriptor's destructor body (`[this] = vtable`, a tail jump to
//!   `Phyre_PClassDescriptor_Destruct`) and its scalar deleting
//!   destructors;
//! - the binding's constant returns (0, -1), the accessor store, the
//!   "not supported" print, the object pushes and the flagged register.

/// `Phyre_PClassDescriptor_Destruct(this)` (a tail-jump target).
pub const DESCRIPTOR_DESTRUCT: u32 = 0x0043_b3e0;
/// `Phyre_PhyreFree(p)`.
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Phyre_PhyrePrintf(level, text)` (cdecl, varargs) and the binding's
/// "not supported" text.
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
pub const NOT_SUPPORTED: u32 = 0x00b0_e998;
/// The Lua object push `0x00436e80(L, object, class)` (cdecl), Lua's
/// `0x0094ac00(L, 1)` and `lua_pushnil(L)`.
pub const LUA_PUSH_CLASS_OBJECT: u32 = 0x0043_6e80;
pub const LUA_0094AC00: u32 = 0x0094_ac00;
pub const LUA_PUSHNIL: u32 = 0x0094_b730;
/// The class record's register `0x0043d6a0(class; object, flag)`.
pub const CLASS_REGISTER: u32 = 0x0043_d6a0;

/// The descriptor vtables.
pub const VT_DISTORTION_GRID: u32 = 0x00b4_41b4;
pub const VT_DISTORTION_GRID_INSTANCE: u32 = 0x00b4_438c;
pub const VT_RADIAL_LINE_INSTANCE: u32 = 0x00b4_44fc;
pub const VT_RADIAL_LINE: u32 = 0x00b4_466c;
pub const VT_SHADOW: u32 = 0x00b4_4ba4;
pub const VT_SHADOW_INSTANCE: u32 = 0x00b4_c17c;
/// The script accessors' `Get(L)` (cdecl).
pub const GET_DISTORTION_GRID: u32 = 0x0067_41e0;
pub const GET_DISTORTION_GRID_INSTANCE: u32 = 0x0067_4db0;
pub const GET_SHADOW: u32 = 0x0067_8660;
pub const GET_SHADOW_INSTANCE: u32 = 0x006f_9650;
/// The class records.
pub const CLASS_DISTORTION_GRID: u32 = 0x00cc_bbe0;
pub const CLASS_DISTORTION_GRID_INSTANCE: u32 = 0x00cc_bce0;
pub const CLASS_SHADOW: u32 = 0x00cc_c158;
pub const CLASS_SHADOW_INSTANCE: u32 = 0x00ce_82f8;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    /// The descriptor's destructor body: `[this] = vtable` and a tail jump
    /// to `Phyre_PClassDescriptor_Destruct(this)`, its `eax`.
    pub fn descriptor_destruct(h: &mut impl Host, this: u32, vtable: u32) -> u32 {
        let vt = h.rebase(vtable);
        h.write(this, vt);
        h.call_sym(DESCRIPTOR_DESTRUCT, Conv::Thiscall(this), &[])
    }

    /// The scalar deleting destructor `(this; flags)` (`ret 4`) -> `this`:
    /// the destructor body, and with bit 0 of the flags (a byte)
    /// `Phyre_PhyreFree(this)`.
    pub fn descriptor_delete(h: &mut impl Host, this: u32, flags: u32, vtable: u32) -> u32 {
        let vt = h.rebase(vtable);
        h.write(this, vt);
        h.call_sym(DESCRIPTOR_DESTRUCT, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            cdecl(h, PHYRE_FREE, &[this]);
        }
        this
    }

    /// The binding's accessor store `(L, out)` (stdcall): `[out] =
    /// get(L)`, its `eax`.
    pub fn accessor_store(h: &mut impl Host, l: u32, out: u32, get: u32) -> u32 {
        let v = cdecl(h, get, &[l]);
        h.write(out, v);
        v
    }

    /// The binding's "not supported" `(a, b)` (stdcall, neither read):
    /// `Phyre_PhyrePrintf(2, text)`'s `eax`.
    pub fn not_supported(h: &mut impl Host) -> u32 {
        let text = h.rebase(NOT_SUPPORTED);
        cdecl(h, PHYRE_PRINTF, &[2, text])
    }

    /// The pointer push `(L, pp)` (stdcall): an object `[pp]` pushed as
    /// its `+4` with the class record; none: `0x0094ac00(L, 1)` and nil.
    pub fn push_pointer(h: &mut impl Host, l: u32, pp: u32, class: u32) -> u32 {
        let p: u32 = h.read(pp);
        if p != 0 {
            let class = h.rebase(class);
            return cdecl(h, LUA_PUSH_CLASS_OBJECT, &[l, p + 4, class]);
        }
        cdecl(h, LUA_0094AC00, &[l, 1]);
        cdecl(h, LUA_PUSHNIL, &[l])
    }

    /// The object push `(L, p)` (stdcall): `p + 4` (0 for none) with the
    /// class record.
    pub fn push_object(h: &mut impl Host, l: u32, p: u32, class: u32) -> u32 {
        let q = if p == 0 { 0 } else { p + 4 };
        let class = h.rebase(class);
        cdecl(h, LUA_PUSH_CLASS_OBJECT, &[l, q, class])
    }

    /// The flagged register `(p, flag)` (stdcall, the flag a byte) -> `al`
    /// 1 (the rest of `eax` the callee's): `0x0043d6a0(class; p + 4 or 0,
    /// flag != 0)`.
    pub fn register_flagged(h: &mut impl Host, p: u32, flag: u32, class: u32) -> u32 {
        let q = if p == 0 { 0 } else { p + 4 };
        let class = h.rebase(class);
        let on = u32::from(flag & 0xff != 0);
        let r = h.call_sym(CLASS_REGISTER, Conv::Thiscall(class), &[q, on]);
        (r & !0xff) | 1
    }

    /// The virtual forward `(this; a, b, c, d, out)` (`ret 0x14`): the
    /// object's virtual 5 with `a..d`, then `[[out]] = [out + 4]`; `eax`
    /// that word.
    pub fn virtual_store(h: &mut impl Host, this: u32, a: [u32; 4], out: u32) -> u32 {
        h.call_virtual(this, 5, &a);
        let dst: u32 = h.read(out);
        let v: u32 = h.read(out + 4);
        h.write(dst, v);
        v
    }
}
