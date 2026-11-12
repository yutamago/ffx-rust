//! M5 finish, WP5 part 2 (`ffx_system::wp5_string`): `std::basic_string`
//! as compiled into `FFX.exe`. Parity tests:
//! `crates/ffx-oracle/tests/wp5_string.rs`.
//!
//! Threads: any (strings are built on the main, loader and render
//! threads). No shim keeps state.

use ffx_system::wp5_string::bridge as b;

use super::{mem, ported};

ported! {
    /// `std_string_substr_00402e20`: `out` = up to `n` characters from `pos`; `out`.
    std_string_substr_00402e20 @ 0x0040_2e20 => extern "thiscall" fn wst_00402e20(this: u32, out: u32, pos: u32, n: u32) -> u32 {
        b::substr(&mut mem(), this, out, pos, n)
    }
    /// `std_string_moveAssign_004039b0`: `from`'s characters taken, `from` left empty; `from`'s old capacity.
    std_string_moveAssign_004039b0 @ 0x0040_39b0 => extern "thiscall" fn wst_004039b0(this: u32, from: u32) -> u32 {
        b::move_from(&mut mem(), this, from)
    }
    /// `StdString_assignSubstr_00403a10`: assign(str, pos, n); `this`.
    StdString_assignSubstr_00403a10 @ 0x0040_3a10 => extern "thiscall" fn wst_00403a10(this: u32, src: u32, pos: u32, n: u32) -> u32 {
        b::assign_substr(&mut mem(), this, src, pos, n)
    }
    /// `std_string_assignCstr_00403b30`: assign(const char *); `this`.
    std_string_assignCstr_00403b30 @ 0x0040_3b30 => extern "thiscall" fn wst_00403b30(this: u32, p: u32) -> u32 {
        b::assign_cstr(&mut mem(), this, p)
    }
    /// `std_string_find_00403bd0`: find(s, pos, n); the position or `npos`.
    std_string_find_00403bd0 @ 0x0040_3bd0 => extern "thiscall" fn wst_00403bd0(this: u32, s: u32, pos: u32, n: u32) -> u32 {
        b::find(&mut mem(), this, s, pos, n)
    }
    /// `std_string_begin_00403d80`: the characters.
    std_string_begin_00403d80 @ 0x0040_3d80 => extern "thiscall" fn wst_00403d80(this: u32) -> u32 {
        b::begin(&mem(), this)
    }
    /// `std_string_assign_00404270`: assign(p, n); `this`.
    std_string_assign_00404270 @ 0x0040_4270 => extern "thiscall" fn wst_00404270(this: u32, p: u32, n: u32) -> u32 {
        b::assign(&mut mem(), this, p, n)
    }
    /// `std_string_cut_00404370`: cut to `n` characters; `this`.
    std_string_cut_00404370 @ 0x0040_4370 => extern "thiscall" fn wst_00404370(this: u32, n: u32) -> u32 {
        b::cut(&mut mem(), this, n)
    }
    /// `std_string_erase_004043b0`: erase(off, count); `this`.
    std_string_erase_004043b0 @ 0x0040_43b0 => extern "thiscall" fn wst_004043b0(this: u32, off: u32, count: u32) -> u32 {
        b::erase(&mut mem(), this, off, count)
    }
    /// `std_string_grow_00404480`: _Grow(new size, trim); `new size != 0`.
    std_string_grow_00404480 @ 0x0040_4480 => extern "thiscall" fn wst_00404480(this: u32, new_size: u32, trim: u32) -> u32 {
        b::grow(&mut mem(), this, new_size, trim)
    }
    /// `std_string_Copy`: _Copy(new size, old size); the new buffer.
    std_string_Copy @ 0x0040_4740 => extern "thiscall" fn wst_00404740(this: u32, new_size: u32, old_size: u32) -> u32 {
        b::grow_copy(&mut mem(), this, new_size, old_size)
    }
    /// `std_string_append_substr`: append(str, pos, n); `this`.
    std_string_append_substr @ 0x0065_75a0 => extern "thiscall" fn wst_006575a0(this: u32, src: u32, pos: u32, n: u32) -> u32 {
        b::append_substr(&mut mem(), this, src, pos, n)
    }
    /// `std_string_appendCstr_00657690`: append(const char *); `this`.
    std_string_appendCstr_00657690 @ 0x0065_7690 => extern "thiscall" fn wst_00657690(this: u32, p: u32) -> u32 {
        b::append_cstr(&mut mem(), this, p)
    }
    /// `std_string_append_ptr`: append(p, n); `this`.
    std_string_append_ptr @ 0x0065_76d0 => extern "thiscall" fn wst_006576d0(this: u32, p: u32, n: u32) -> u32 {
        b::append(&mut mem(), this, p, n)
    }
    /// `std_string_compare_006590d0`: compare(pos, n, s, len).
    std_string_compare_006590d0 @ 0x0065_90d0 => extern "thiscall" fn wst_006590d0(this: u32, pos: u32, n: u32, s: u32, len: u32) -> u32 {
        b::compare(&mut mem(), this, pos, n, s, len)
    }
    /// `std_string_ctor_cstr`: `basic_string(const char *)`; `this`.
    std_string_ctor_cstr @ 0x0040_2c90 => extern "thiscall" fn wst_00402c90(this: u32, p: u32) -> u32 {
        b::ctor_cstr(&mut mem(), this, p)
    }
    /// `std_string_ctor_move`: `basic_string(basic_string &&)`; `this`.
    std_string_ctor_move @ 0x0040_2cf0 => extern "thiscall" fn wst_00402cf0(this: u32, from: u32) -> u32 {
        b::ctor_move(&mut mem(), this, from)
    }
    /// `std_string_assign_move`: `operator=(basic_string &&)`; `this`.
    std_string_assign_move @ 0x0040_2d60 => extern "thiscall" fn wst_00402d60(this: u32, from: u32) -> u32 {
        b::assign_move(&mut mem(), this, from)
    }
}
