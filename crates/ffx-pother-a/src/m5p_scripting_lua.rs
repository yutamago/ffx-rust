//! `PhyreEngine`'s scripting support outside the script layer (M5 stage 1,
//! list `phyre-scripting`, batch 3): the script object accessors
//! (`PScriptAccessors::PObjectAccessor<T>::Get`, one generic port for the
//! pointer copies and the reference one) and the Lua C functions of the
//! `Vector` and `Matrix` types that need no frame buffers (lengths,
//! squared lengths, scale and translation matrices).
//!
//! The error paths call `luaL_error`, which never returns in the game (it
//! unwinds the Lua call with `longjmp`, past the shim as past the
//! original's frame); the ports set the same globals first and return
//! what the original's unreachable tail returns.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_scripting_mid::{LUA_TYPE, bridge as mid};
use crate::m5p_scripting_small::{
    LUA_GETFIELD, LUA_NEWUSERDATA, LUA_PUSHNUMBER, LUA_REGISTRYINDEX, LUA_SETMETATABLE, LUA_SETTOP,
    STR_MATRIX, STR_PHYRE_OBJECT, STR_VECTOR,
};

/// `luaL_error(L, fmt, ...)` (cdecl, varargs; never returns in the game).
pub const LUA_ERROR: u32 = 0x0094_cd50;
/// `0x0094c9e0` `(L, arg)` (cdecl): `luaL_checknumber`, the `float` in
/// `ST0`.
pub const LUA_CHECKNUMBER: u32 = 0x0094_c9e0;
/// `lua_gettop(L)` (cdecl).
pub const LUA_GETTOP: u32 = 0x0094_b160;
/// The error location globals: file, line, function.
pub const ERROR_FILE: u32 = 0x00c9_6a98;
/// See [`ERROR_FILE`].
pub const ERROR_LINE: u32 = 0x00c9_6a9c;
/// See [`ERROR_FILE`].
pub const ERROR_FUNCTION: u32 = 0x00c9_6aa0;
/// The source file name the accessors report.
pub const STR_SCRIPTING_INL: u32 = 0x00b0_dbd8;
/// "Object pointer obtained from script was of type ..." (the pointer
/// accessors).
pub const STR_WRONG_TYPE: u32 = 0x00b0_dce8;
/// "Unable to obtain object pointer from script ..." (Lua type %d).
pub const STR_NOT_OBJECT: u32 = 0x00b0_dd48;
/// The reference accessor's wrong-type message.
pub const STR_REF_WRONG_TYPE: u32 = 0x00b0_dc90;
/// The reference accessor's no-object message.
pub const STR_REF_NO_OBJECT: u32 = 0x00b0_dc30;
/// `Phyre_PClassDescriptor_getDefaultInstance(class)` (thiscall).
pub const DEFAULT_INSTANCE: u32 = 0x0043_cbb0;

/// A pointer accessor `PObjectAccessor<T *>::Get(L)`.
pub struct Accessor {
    /// The function.
    pub addr: u32,
    /// `T`'s class descriptor.
    pub class: u32,
    /// Its `__FUNCTION__` string.
    pub function: u32,
    /// What the object pointer is adjusted by (a base-class offset).
    pub adjust: u32,
}

/// The pointer accessors.
pub const ACCESSORS: [Accessor; 7] = [
    Accessor {
        addr: 0x0043_5650,
        class: 0x00c9_02e8,
        function: 0x00b0_e1b0,
        adjust: 0,
    },
    Accessor {
        addr: 0x0044_b970,
        class: 0x00c9_1958,
        function: 0x00b0_ffa8,
        adjust: 0,
    },
    Accessor {
        addr: 0x004c_fe80,
        class: 0x00ca_4bc8,
        function: 0x00b1_f428,
        adjust: 0,
    },
    Accessor {
        addr: 0x0056_0450,
        class: 0x00cb_0ff0,
        function: 0x00b2_ea38,
        adjust: 0,
    },
    Accessor {
        addr: 0x0043_bad0,
        class: 0x00c9_0760,
        function: 0x00b0_ebc8,
        adjust: 0x18,
    },
    Accessor {
        addr: 0x0052_1d80,
        class: 0x00ca_b920,
        function: 0x00b2_8350,
        adjust: 4,
    },
    Accessor {
        addr: 0x0055_57f0,
        class: 0x00ca_fb70,
        function: 0x00b2_d7f0,
        adjust: 4,
    },
];

/// `PObjectAccessor<PEntity &>::Get`'s class and function string.
pub const ENTITY_CLASS: u32 = 0x00c9_1dd8;
/// See [`ENTITY_CLASS`].
pub const ENTITY_FUNCTION: u32 = 0x00b1_0570;

/// The vector length functions: (function, components, square root).
pub const LENGTHS: [(u32, u32, bool); 4] = [
    (0x0057_3160, 3, true),
    (0x0057_3230, 4, true),
    (0x0057_3300, 3, false),
    (0x0057_33c0, 4, false),
];

/// What a unary `Vector`/`Matrix` function returns.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnaryKind {
    /// A new `Matrix` (`PMatrix4_Copy` of the result).
    Matrix,
    /// A new `Vector`, the four result words.
    Vector4,
    /// A new `Vector` from the first three components (copied to the
    /// frame first), the three result words and 0.
    Vector3,
}

/// A unary Lua C function `(L)`: argument 1 checked as the input type,
/// the math routine `op(out, in)` (cdecl) into the frame, the result as a
/// new userdata; 1, or 0 when argument 1 is not of the type.
pub struct Unary {
    /// The function.
    pub addr: u32,
    /// The math routine.
    pub op: u32,
    /// The input's metatable name.
    pub input: u32,
    /// The result.
    pub kind: UnaryKind,
}

/// The unary functions.
pub const UNARY: [Unary; 4] = [
    // sce_Vectormath_Scalar_Aos_inverse
    Unary {
        addr: 0x0057_19a0,
        op: 0x0057_3d00,
        input: STR_MATRIX,
        kind: UnaryKind::Matrix,
    },
    // PMatrix4_Transpose_004f93f0
    Unary {
        addr: 0x0057_1e00,
        op: 0x004f_93f0,
        input: STR_MATRIX,
        kind: UnaryKind::Matrix,
    },
    // Phyre_Vector4_normalize_005744a0
    Unary {
        addr: 0x0057_3860,
        op: 0x0057_44a0,
        input: STR_VECTOR,
        kind: UnaryKind::Vector4,
    },
    // Vector3_normalize_004525a0
    Unary {
        addr: 0x0057_3760,
        op: 0x0045_25a0,
        input: STR_VECTOR,
        kind: UnaryKind::Vector3,
    },
];

/// `0x00574520` `(out, &angles)` (cdecl): the rotation matrix of three
/// angles in radians.
pub const ROTATION: u32 = 0x0057_4520;
/// `PMatrix4_Copy(out; m)` (thiscall, one word).
pub const MATRIX_COPY: u32 = 0x0045_3340;
/// Degrees to radians (`double`).
pub const DEG_TO_RAD: u32 = 0x00b1_1330;
/// The frame bytes [`bridge::unary`] and [`bridge::matrix_rotation`] need.
pub const FRAME_BYTES: u32 = 0x60;

/// The ports over game memory.
pub mod bridge {
    use super::{
        Accessor, Conv, DEFAULT_INSTANCE, ENTITY_CLASS, ENTITY_FUNCTION, ERROR_FILE,
        ERROR_FUNCTION, ERROR_LINE, Host, LUA_CHECKNUMBER, LUA_ERROR, LUA_GETFIELD, LUA_GETTOP,
        LUA_NEWUSERDATA, LUA_PUSHNUMBER, LUA_REGISTRYINDEX, LUA_SETMETATABLE, LUA_SETTOP, LUA_TYPE,
        STR_MATRIX, STR_NOT_OBJECT, STR_PHYRE_OBJECT, STR_REF_NO_OBJECT, STR_REF_WRONG_TYPE,
        STR_SCRIPTING_INL, STR_VECTOR, STR_WRONG_TYPE, mid, x87,
    };
    use super::{DEG_TO_RAD, MATRIX_COPY, ROTATION, Unary, UnaryKind};

    /// The error location globals set before `luaL_error`.
    fn error_at(h: &mut impl Host, line: u32, function: u32) {
        let file = h.rebase(STR_SCRIPTING_INL);
        let function = h.rebase(function);
        let at = h.rebase(ERROR_FILE);
        h.write(at, file);
        let at = h.rebase(ERROR_LINE);
        h.write(at, line);
        let at = h.rebase(ERROR_FUNCTION);
        h.write(at, function);
    }

    /// Whether the class chain from `d` (parents at `+0x40`) reaches
    /// `class`.
    fn derives(h: &impl Host, d: u32, class: u32) -> bool {
        let mut e = d;
        while e != class {
            e = h.read(e.wrapping_add(0x40));
            if e == 0 {
                return false;
            }
        }
        true
    }

    /// An [`Accessor`] `(L)`: the object of the `PhyreObject` userdata on
    /// top of the stack (popped) when its class derives from `T` (adjusted
    /// to `T`), 0 for nil; a wrong class or a non-object raises a Lua
    /// error (0).
    pub fn accessor_get(h: &mut impl Host, a: &Accessor, l: u32) -> u32 {
        let u = mid::check_userdata(h, STR_PHYRE_OBJECT, l, u32::MAX);
        if u != 0 {
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
            let obj: u32 = h.read(u.wrapping_add(4));
            let d: u32 = h.read(u);
            if obj != 0 {
                let class = h.rebase(a.class);
                if d != 0 && !derives(h, d, class) {
                    let have: u32 = h.read(d.wrapping_add(0x18));
                    let want: u32 = h.read(class.wrapping_add(0x18));
                    error_at(h, 0x6a, a.function);
                    let fmt = h.rebase(STR_WRONG_TYPE);
                    h.call_sym(LUA_ERROR, Conv::Cdecl, &[l, fmt, have, want]);
                    return 0;
                }
                return obj.wrapping_sub(a.adjust);
            }
        }
        if h.call_sym(LUA_TYPE, Conv::Cdecl, &[l, u32::MAX]) == 0 {
            h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
            return 0;
        }
        let t = h.call_sym(LUA_TYPE, Conv::Cdecl, &[l, u32::MAX]);
        error_at(h, 0x76, a.function);
        let fmt = h.rebase(STR_NOT_OBJECT);
        h.call_sym(LUA_ERROR, Conv::Cdecl, &[l, fmt, t]);
        0
    }

    /// `PObjectAccessor<PEntity &>::Get` `0x0044f7f0` `(L)`: as
    /// [`accessor_get`] for a reference: the top is popped first; no
    /// object gives the class's default instance and a Lua error, a
    /// class that does not derive from `PEntity` a Lua error; the object
    /// otherwise.
    pub fn entity_get(h: &mut impl Host, l: u32) -> u32 {
        let u = mid::check_userdata(h, STR_PHYRE_OBJECT, l, u32::MAX);
        h.call_sym(LUA_SETTOP, Conv::Cdecl, &[l, 0xffff_fffe]);
        let class = h.rebase(ENTITY_CLASS);
        let want: u32 = h.read(class.wrapping_add(0x18));
        let (obj, d) = if u == 0 {
            let obj = h.call_sym(DEFAULT_INSTANCE, Conv::Thiscall(class), &[]);
            (obj, 0)
        } else {
            (h.read(u.wrapping_add(4)), h.read(u))
        };
        if d == 0 {
            error_at(h, 0x8d, ENTITY_FUNCTION);
            let fmt = h.rebase(STR_REF_NO_OBJECT);
            h.call_sym(LUA_ERROR, Conv::Cdecl, &[l, fmt, want]);
            return obj;
        }
        if !derives(h, d, class) {
            let have: u32 = h.read(d.wrapping_add(0x18));
            error_at(h, 0x8f, ENTITY_FUNCTION);
            let fmt = h.rebase(STR_REF_WRONG_TYPE);
            h.call_sym(LUA_ERROR, Conv::Cdecl, &[l, fmt, have, want]);
        }
        obj
    }

    /// [`super::LENGTHS`] `(L)`: argument 1's `Vector` (3 or 4
    /// components) squared length, or length (`fsqrt`), pushed (1); 0
    /// when argument 1 is not a `Vector`. Each partial sum is rounded to
    /// `float`, as the original stores it.
    pub fn vector_length(h: &mut impl Host, l: u32, n: u32, root: bool) -> u32 {
        let v = mid::check_userdata(h, STR_VECTOR, l, 1);
        if v == 0 {
            return 0;
        }
        let mut sum = 0u32;
        for i in 0..n {
            let c: u32 = h.read(v.wrapping_add(4 * i));
            let c = x87::load(c);
            let sq = x87::mul(c, c);
            sum = if i == 0 {
                x87::store(sq)
            } else {
                x87::store(x87::add(x87::load(sum), sq))
            };
        }
        if root {
            sum = x87::store(x87::sqrt(x87::load(sum)));
        }
        h.call_sym(LUA_PUSHNUMBER, Conv::Cdecl, &[l, sum]);
        1
    }

    /// The three numbers of a scale or translation: argument 1 for all
    /// three, or arguments 1..3 when there is more than one.
    fn three_numbers(h: &mut impl Host, l: u32) -> [u32; 3] {
        let top = h.call_sym(LUA_GETTOP, Conv::Cdecl, &[l]);
        let a = h
            .call_sym_st0(LUA_CHECKNUMBER, Conv::Cdecl, &[l, 1])
            .to_bits();
        let mut out = [a, a, a];
        if (top as i32) > 1 {
            out[1] = h
                .call_sym_st0(LUA_CHECKNUMBER, Conv::Cdecl, &[l, 2])
                .to_bits();
            out[2] = h
                .call_sym_st0(LUA_CHECKNUMBER, Conv::Cdecl, &[l, 3])
                .to_bits();
        }
        out
    }

    /// A new `Matrix` userdata (the metatable set when allocated).
    fn new_matrix(h: &mut impl Host, l: u32) -> u32 {
        let u = h.call_sym(LUA_NEWUSERDATA, Conv::Cdecl, &[l, 0x40]);
        if u != 0 {
            let name = h.rebase(STR_MATRIX);
            h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
            h.call_sym(LUA_SETMETATABLE, Conv::Cdecl, &[l, 0xffff_fffe]);
        }
        u
    }

    /// `0x00571480` `(L)`: a new `Matrix`, the scale by the numbers (one
    /// for all axes, or three); 1.
    pub fn matrix_scale(h: &mut impl Host, l: u32) -> u32 {
        let [x, y, z] = three_numbers(h, l);
        let m = new_matrix(h, l);
        let one = 0x3f80_0000u32;
        let words = [x, 0, 0, 0, 0, y, 0, 0, 0, 0, z, 0, 0, 0, 0, one];
        for (i, w) in (0u32..).zip(words) {
            h.write(m.wrapping_add(4 * i), w);
        }
        1
    }

    /// `0x00571540` `(L)`: a new `Matrix`, the translation by the numbers
    /// (row 3); 1.
    pub fn matrix_translation(h: &mut impl Host, l: u32) -> u32 {
        let [x, y, z] = three_numbers(h, l);
        let m = new_matrix(h, l);
        let one = 0x3f80_0000u32;
        let words = [one, 0, 0, 0, 0, one, 0, 0, 0, 0, one, 0, x, y, z, one];
        for (i, w) in (0u32..).zip(words) {
            h.write(m.wrapping_add(4 * i), w);
        }
        1
    }

    /// A new `Vector` userdata (the metatable set when allocated).
    fn new_vector(h: &mut impl Host, l: u32) -> u32 {
        let u = h.call_sym(LUA_NEWUSERDATA, Conv::Cdecl, &[l, 0x10]);
        if u != 0 {
            let name = h.rebase(STR_VECTOR);
            h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
            h.call_sym(LUA_SETMETATABLE, Conv::Cdecl, &[l, 0xffff_fffe]);
        }
        u
    }

    /// A word copied through the x87 (`fld dword` / `fstp dword`).
    fn copy_float(h: &mut impl Host, to: u32, from: u32) {
        let f: u32 = h.read(from);
        h.write(to, x87::store(x87::load(f)));
    }

    /// A [`Unary`] `(L)`; `frame` is [`super::FRAME_BYTES`] bytes the
    /// math routine writes (the original's frame).
    pub fn unary(h: &mut impl Host, op: &Unary, l: u32, frame: u32) -> u32 {
        let v = mid::check_userdata(h, op.input, l, 1);
        if v == 0 {
            return 0;
        }
        match op.kind {
            UnaryKind::Matrix => {
                let u = new_matrix(h, l);
                let r = h.call_sym(op.op, Conv::Cdecl, &[frame, v]);
                h.call_sym(MATRIX_COPY, Conv::Thiscall(u), &[r]);
            }
            UnaryKind::Vector4 => {
                let u = new_vector(h, l);
                let r = h.call_sym(op.op, Conv::Cdecl, &[frame, v]);
                for i in 0..4 {
                    copy_float(h, u.wrapping_add(4 * i), r.wrapping_add(4 * i));
                }
            }
            UnaryKind::Vector3 => {
                let u = new_vector(h, l);
                let tmp = frame.wrapping_add(0x40);
                for i in 0..3 {
                    copy_float(h, tmp.wrapping_add(4 * i), v.wrapping_add(4 * i));
                }
                let r = h.call_sym(op.op, Conv::Cdecl, &[frame, tmp]);
                for i in 0..3 {
                    copy_float(h, u.wrapping_add(4 * i), r.wrapping_add(4 * i));
                }
                h.write(u.wrapping_add(0xc), 0u32);
            }
        }
        1
    }

    /// `0x005713e0` `(L)`: a new `Matrix`, the rotation by the three
    /// numbers in degrees (each times `DEG_TO_RAD`, rounded to `float`);
    /// 1. `frame` as for [`unary`].
    pub fn matrix_rotation(h: &mut impl Host, l: u32, frame: u32) -> u32 {
        let k: u64 = h.read(h.rebase(DEG_TO_RAD));
        let k = f64::from_bits(k);
        let mut angles = [0u32; 3];
        for (i, a) in (1u32..).zip(angles.iter_mut()) {
            let x = h.call_sym_st0_f64(LUA_CHECKNUMBER, Conv::Cdecl, &[l, i]);
            *a = x87::store(x87::mul(x, k));
        }
        let u = new_matrix(h, l);
        let vec = frame.wrapping_add(0x40);
        for (i, &a) in (0u32..).zip(angles.iter()) {
            h.write(vec.wrapping_add(4 * i), x87::store(x87::load(a)));
        }
        let r = h.call_sym(ROTATION, Conv::Cdecl, &[frame, vec]);
        h.call_sym(MATRIX_COPY, Conv::Thiscall(u), &[r]);
        1
    }
}
