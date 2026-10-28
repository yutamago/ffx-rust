//! M5 stage 1, list `vfx`, batch 4: the effect program dump's operand
//! printers (a table at `0x00c88884` and on, one per opcode: `u_unit_max`,
//! `u_et_no`, `u_v(`, `u_task_%1d`, `e_anm`, `u_part_init`, ...). Each
//! formats one opcode for `op_num` (`0x007e63a0`), which in this build is
//! a bare `ret`: the printers change nothing but their frame, and they
//! leave in `eax` the last value they computed, which the ports return.
//!
//! The three `sprintf` printers build their line in the frame at `ebp -
//! 0x104` (`sprintf` through its import slot, then
//! `opd_debug_flags_str_00a65570` after it); the stack cookie is left out.

/// `op_num(format, ...)` (cdecl, varargs; a bare `ret`).
pub const OP_NUM: u32 = 0x007e_63a0;
/// `opd_debug_flags_str_00a65570(out, op)` (cdecl).
pub const FLAGS_STR: u32 = 0x00a6_5570;
/// `MSVCR110!sprintf` (the import slot; cdecl, varargs).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

pub mod bridge {
    use super::{FLAGS_STR, OP_NUM, SPRINTF_SLOT};
    use ffx_graphic::host::{Conv, Host};

    fn op_num(h: &mut impl Host, fmt: u32, args: &[u32]) {
        let mut words = vec![h.rebase(fmt)];
        words.extend_from_slice(args);
        h.call_sym(OP_NUM, Conv::Cdecl, &words);
    }

    /// The operand's halfword at `off`, sign-extended.
    fn half(h: &impl Host, op: u32, off: u32) -> u32 {
        i32::from(h.read::<i16>(op + off)) as u32
    }

    /// `(format, op, (i16)op[2])` -> that halfword: `u_unit_max`,
    /// `u_et_no`, `e_anm`, `u_texture_load`, `u_clut_load`, `u_tc_load`,
    /// `e_md`.
    pub fn print_half(h: &mut impl Host, fmt: u32, op: u32) -> u32 {
        let v = half(h, op, 2);
        op_num(h, fmt, &[op, v]);
        v
    }

    /// `(format, op, ((i16)op[0] >> 12) & 3)` -> that value:
    /// `u_task_off_%1d`.
    pub fn print_task_off(h: &mut impl Host, fmt: u32, op: u32) -> u32 {
        let k = (half(h, op, 0) as i32 >> 12) as u32 & 3;
        op_num(h, fmt, &[op, k]);
        k
    }

    /// `u_unit%1d(%7x)`: `(format, op, ((i16)op[0] >> 12) & 3, op +
    /// (i16)op[2])` -> the unit number.
    pub fn print_unit(h: &mut impl Host, fmt: u32, op: u32) -> u32 {
        let k = (half(h, op, 0) as i32 >> 12) as u32 & 3;
        let target = half(h, op, 2).wrapping_add(op);
        op_num(h, fmt, &[op, k, target]);
        k
    }

    /// `u_taskp_%1d` (bit 15 of `op[0]` set) or `u_task_%1d`: `(format, op,
    /// ((i16)op[2] >> 12) & 3, op + (i16)op[2])` -> the target.
    pub fn print_task(h: &mut impl Host, fmt_p: u32, fmt: u32, op: u32) -> u32 {
        let e = half(h, op, 2);
        let target = e.wrapping_add(op);
        let k = (e as i32 >> 12) as u32 & 3;
        let f = if h.read::<u16>(op) & 0x8000 != 0 {
            fmt_p
        } else {
            fmt
        };
        op_num(h, f, &[op, k, target]);
        target
    }

    /// `(format, op)`: `e_flash`, `u_world)`.
    pub fn print_plain(h: &mut impl Host, fmt: u32, op: u32) {
        op_num(h, fmt, &[op]);
    }

    /// `e_anmm(%4d)`: only for an opcode word without bits 12-15 ->
    /// `(i16)op[2]`; else 0xf000.
    pub fn print_anmm(h: &mut impl Host, fmt: u32, op: u32) -> u32 {
        if h.read::<u16>(op) & 0xf000 != 0 {
            return 0xf000;
        }
        print_half(h, fmt, op)
    }

    /// `e_gline(%4x)`: only for `op[2]` 6: `(format, op, (i16)op[4])` ->
    /// that halfword (`None`: nothing printed, `eax` left alone).
    pub fn print_gline(h: &mut impl Host, fmt: u32, op: u32) -> Option<u32> {
        if h.read::<u16>(op + 2) != 6 {
            return None;
        }
        let v = half(h, op, 4);
        op_num(h, fmt, &[op, v]);
        Some(v)
    }

    /// `u_part_init(%2d,%2d)`: `(format, op, (i16)op[2], (i16)op[4])` ->
    /// the first.
    pub fn print_part_init(h: &mut impl Host, fmt: u32, op: u32) -> u32 {
        let a = half(h, op, 2);
        let b = half(h, op, 4);
        op_num(h, fmt, &[op, a, b]);
        a
    }

    /// `e_weapon_shadow`: for an opcode word with bits 12-15, its argument
    /// slot becomes the format and it tail-jumps to `op_num`; -> `op`.
    pub fn print_weapon_shadow(h: &mut impl Host, slots: u32, fmt: u32, op: u32) -> u32 {
        if h.read::<u16>(op) & 0xf000 != 0 {
            let f = h.rebase(fmt);
            h.write(slots, f);
            h.call_sym(OP_NUM, Conv::Cdecl, &[f]);
        }
        op
    }

    /// `u_v(`, `u_s(`, `u_sp(` (frame: the line at `ebp - 0x104`):
    /// `sprintf(line, format, op)`, the opcode's flags after it
    /// (`opd_debug_flags_str_00a65570(line + n, op)`), `op_num(line)`.
    pub fn print_flags(h: &mut impl Host, ebp: u32, fmt: u32, op: u32) {
        let line = ebp - 0x104;
        let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
        let f = h.rebase(fmt);
        let n = h.call(sprintf, Conv::Cdecl, &[line, f, op]);
        h.call_sym(FLAGS_STR, Conv::Cdecl, &[line.wrapping_add(n), op]);
        h.call_sym(OP_NUM, Conv::Cdecl, &[line]);
    }
}
