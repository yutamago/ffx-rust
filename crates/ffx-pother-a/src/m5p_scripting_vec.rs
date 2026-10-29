//! `PhyreEngine`'s scripting support outside the script layer (M5 stage 1,
//! list `phyre-scripting`, batch 8): the `Vector` Lua functions of two
//! vectors: the 3- and 4-component dot products and the cross product.
//! Each partial result is rounded to `float` where the original stores it
//! (`fstp dword`); products of two floats are exact in the x87's 53 bits.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_scripting_mid::bridge as mid;
use crate::m5p_scripting_small::{
    LUA_GETFIELD, LUA_NEWUSERDATA, LUA_PUSHNUMBER, LUA_REGISTRYINDEX, LUA_SETMETATABLE, STR_VECTOR,
};

/// The dot products: (function, components).
pub const DOTS: [(u32, u32); 2] = [(0x0057_2a40, 3), (0x0057_2b70, 4)];

/// The component-wise arithmetic of a `Vector` with a `Vector` or a
/// number (either side).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Arith {
    /// `+`.
    Add,
    /// `-`.
    Sub,
    /// `*`.
    Mul,
    /// `/`.
    Div,
}

/// The arithmetic functions: (function, operation).
pub const ARITH: [(u32, Arith); 4] = [
    (0x0057_2450, Arith::Add),
    (0x0057_3930, Arith::Sub),
    (0x0057_3480, Arith::Mul),
    (0x0057_2810, Arith::Div),
];

/// `luaL_checknumber(L, arg)` (cdecl, the `float` in `ST0`).
pub const LUA_CHECKNUMBER: u32 = 0x0094_c9e0;

/// The ports over game memory.
pub mod bridge {
    use super::{
        Arith, Conv, Host, LUA_CHECKNUMBER, LUA_GETFIELD, LUA_NEWUSERDATA, LUA_PUSHNUMBER,
        LUA_REGISTRYINDEX, LUA_SETMETATABLE, STR_VECTOR, mid, x87,
    };

    fn f(h: &impl Host, v: u32, i: u32) -> f64 {
        x87::load(h.read(v.wrapping_add(4 * i)))
    }

    /// [`super::DOTS`] `(L)`: the dot product of the `Vector`s argument 1
    /// and 2 (`n` components; the sum rounded to `float` after each
    /// term) pushed; 1, or 0 when either is not a `Vector`.
    pub fn dot(h: &mut impl Host, l: u32, n: u32) -> u32 {
        let a = mid::check_userdata(h, STR_VECTOR, l, 1);
        let b = mid::check_userdata(h, STR_VECTOR, l, 2);
        if a == 0 || b == 0 {
            return 0;
        }
        let mut sum = x87::store(x87::mul(f(h, a, 0), f(h, b, 0)));
        for i in 1..n {
            let p = x87::mul(f(h, a, i), f(h, b, i));
            sum = x87::store(x87::add(x87::load(sum), p));
        }
        h.call_sym(LUA_PUSHNUMBER, Conv::Cdecl, &[l, sum]);
        1
    }

    /// `0x00572690` `(L)`: a new `Vector`, the cross product of the
    /// `Vector`s argument 1 and 2 (each component `a*b - c*d` rounded to
    /// `float`, w 0); 1, or 0 when either is not a `Vector`.
    pub fn cross(h: &mut impl Host, l: u32) -> u32 {
        let a = mid::check_userdata(h, STR_VECTOR, l, 1);
        let b = mid::check_userdata(h, STR_VECTOR, l, 2);
        if a == 0 || b == 0 {
            return 0;
        }
        let u = h.call_sym(LUA_NEWUSERDATA, Conv::Cdecl, &[l, 0x10]);
        if u != 0 {
            let name = h.rebase(STR_VECTOR);
            h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
            h.call_sym(LUA_SETMETATABLE, Conv::Cdecl, &[l, 0xffff_fffe]);
        }
        let (a0, a1, a2) = (f(h, a, 0), f(h, a, 1), f(h, a, 2));
        let (b0, b1, b2) = (f(h, b, 0), f(h, b, 1), f(h, b, 2));
        let r0 = x87::store(x87::sub(x87::mul(a1, b2), x87::mul(a2, b1)));
        let r1 = x87::store(x87::sub(x87::mul(a2, b0), x87::mul(a0, b2)));
        let r2 = x87::store(x87::sub(x87::mul(a0, b1), x87::mul(a1, b0)));
        h.write(u, r0);
        h.write(u.wrapping_add(4), r1);
        h.write(u.wrapping_add(8), r2);
        h.write(u.wrapping_add(0xc), x87::store(0.0));
        1
    }

    /// One component of an [`Arith`] at the x87's 53 bits, rounded to
    /// `float`.
    fn apply(op: Arith, a: f64, b: f64) -> u32 {
        x87::store(match op {
            Arith::Add => x87::add(a, b),
            Arith::Sub => x87::sub(a, b),
            Arith::Mul => x87::mul(a, b),
            Arith::Div => x87::div(a, b),
        })
    }

    /// [`super::ARITH`] `(L)`: a new `Vector`, argument 1 `op` argument 2
    /// component by component, where one side may be a number (checked
    /// with `luaL_checknumber` before the userdata is made); 1, or 0 when
    /// neither is a `Vector` (the division reads address 0 then, as the
    /// original does).
    pub fn arith(h: &mut impl Host, l: u32, op: Arith) -> u32 {
        let a = mid::check_userdata(h, STR_VECTOR, l, 1);
        let b = mid::check_userdata(h, STR_VECTOR, l, 2);
        // the division alone does not check its second side: with neither a
        // Vector it divides by the words at address 0 (and faults)
        if a == 0 && b == 0 && op != Arith::Div {
            return 0;
        }
        let n = if a == 0 || b == 0 {
            let arg = if a == 0 { 1 } else { 2 };
            let x = h.call_sym_st0(LUA_CHECKNUMBER, Conv::Cdecl, &[l, arg]);
            x87::load(x.to_bits())
        } else {
            0.0
        };
        let u = h.call_sym(LUA_NEWUSERDATA, Conv::Cdecl, &[l, 0x10]);
        if u != 0 {
            let name = h.rebase(STR_VECTOR);
            h.call_sym(LUA_GETFIELD, Conv::Cdecl, &[l, LUA_REGISTRYINDEX, name]);
            h.call_sym(LUA_SETMETATABLE, Conv::Cdecl, &[l, 0xffff_fffe]);
        }
        for i in 0..4 {
            let x = if a == 0 { n } else { f(h, a, i) };
            let y = if b == 0 { n } else { f(h, b, i) };
            h.write(u.wrapping_add(4 * i), apply(op, x, y));
        }
        1
    }
}
