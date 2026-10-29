//! M5 stage 1, the menu, battle and script leftovers (round 4), batch 3:
//! the rest of the event debugger's value printers (`0x0087b0c0` to
//! `0x0087bc20`, `0x0087ee10`; entries of the debugger's display tables,
//! data `0x00c537d4..0x00c542a8`), the source viewer's line splitter
//! (`AtelSetUpSourceDebug`) and the right-aligned print entry.
//!
//! As in [`crate::m5m_c`]: a printer formats into the 32-byte buffer at
//! `ebp - 0x24` of its frame (the security cookie at `ebp - 4`) with the
//! CRT's `sprintf` and prints at the debug font's cursor in the colour
//! `0x01328a74` (`TkPutDebugFontStringP`, proportional, unless noted).
//! The bridges take that frame's `ebp`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

/// What a flag printer tests: the byte at `+o` against `mask`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flag {
    pub offset: u32,
    pub mask: u8,
    /// The text when set and when clear.
    pub on: u32,
    pub off: u32,
    /// The colour: the current one, or 0.
    pub colour0: bool,
}

/// The flag printers (`TkPutDebugFontStringP` of one of two texts).
pub const FLAGS: [(u32, Flag); 5] = [
    (
        0x0087_b580,
        flag(0x38, 0x20, 0x00b7_c980, 0x00b7_c97c, false),
    ),
    (
        0x0087_b5b0,
        flag(0x38, 0x40, 0x00b7_c980, 0x00b7_c97c, false),
    ),
    (
        0x0087_b5e0,
        flag(0x38, 0x80, 0x00b7_c980, 0x00b7_c97c, false),
    ),
    (
        0x0087_ba70,
        flag(0x34, 0x02, 0x00b5_cd4c, 0x00b5_cd50, true),
    ),
    (
        0x0087_bc20,
        flag(0x34, 0x80, 0x00b5_cd70, 0x00b5_cd78, false),
    ),
];

const fn flag(offset: u32, mask: u8, on: u32, off: u32, colour0: bool) -> Flag {
    Flag {
        offset,
        mask,
        on,
        off,
        colour0,
    }
}

/// The control work's `f32` printers: `%f` of the `f32` at `+o` of
/// `AtelGetCtrlWork(0x01328a70)`.
pub const CTRL_FLOATS: [(u32, u32); 6] = [
    (0x0087_b6b0, 0x218),
    (0x0087_b710, 0x21c),
    (0x0087_b770, 0x220),
    (0x0087_b7d0, 0x228),
    (0x0087_b830, 0x22c),
    (0x0087_b890, 0x230),
];

pub mod bridge {
    use ffx_bridge::Global;
    use ffx_graphic::host::{Conv, Host};

    use super::Flag;

    /// The debug font's colour index; the debugger's current control
    /// work.
    pub const FONT_COLOR: Global<u32> = Global::new(0x0132_8a74);
    pub const DEBUG_CTRL: Global<u32> = Global::new(0x0132_8a70);

    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    pub const FMT_F: u32 = 0x00b4_0274;
    pub const FMT_D: u32 = 0x00b0_f604;
    /// `"%04x"`-style motion format (`0x00b5ccf8`), the program counter's
    /// (`0x00b5ccf0`).
    pub const FMT_MOTION: u32 = 0x00b5_ccf8;
    pub const FMT_PC: u32 = 0x00b5_ccf0;
    pub const NO_MOTION: u32 = 0x00b4_865c;
    pub const TEXT_ON: u32 = 0x00b7_c980;
    pub const TEXT_OFF: u32 = 0x00b7_c97c;
    pub const NO_MODEL: u32 = 0x00b5_5278;

    pub const SCRIPT_WORKER_LEVEL: u32 = 0x0087_2650;
    pub const SCRIPT_PC_OFFSET: u32 = 0x0087_25e0;
    pub const ATEL_GET_POS_WORK_ADRS: u32 = 0x0086_c2b0;
    pub const ATEL_GET_CTRL_WORK: u32 = 0x0086_afa0;
    pub const ATEL_GET_BASIC_WORK: u32 = 0x0086_a830;
    pub const ATEL_GET_WORKER_BITS: u32 = 0x0086_a440;
    pub const ATEL_DEBUG_PRINT_ENTRY: u32 = 0x0087_afa0;
    pub const CH_GET_BODY_HIT: u32 = 0x0082_ab10;
    pub const CH_GET_THICKNESS: u32 = 0x0082_ad70;
    pub const TK_PUT_DEBUG_FONT_STRING: u32 = 0x0088_f140;
    pub const TK_PUT_DEBUG_FONT_STRING_P: u32 = 0x0088_f4c0;
    pub const TK_PUT_DEBUG_FONT_STRING_P_EUC: u32 = 0x0088_f4e0;
    pub const TK_PUT_DEBUG_FONT_STRING_PR: u32 = 0x0088_f5c0;
    /// The worker-bits names (`.data`, `0x00c53748`).
    pub const BITS_NAMES: u32 = 0x00c5_3748;

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn sprintf(h: &mut impl Host, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
        h.call(f, Conv::Cdecl, args)
    }

    /// `TkPutDebugFontStringP(colour, 0, s)` in the current colour.
    fn put_p(h: &mut impl Host, s: u32) -> u32 {
        let c = FONT_COLOR.get(h);
        cdecl(h, TK_PUT_DEBUG_FONT_STRING_P, &[c, 0, s])
    }

    /// [`put_p`] of the `.rdata` text at `sym`.
    fn put_sym(h: &mut impl Host, sym: u32) -> u32 {
        let s = h.rebase(sym);
        put_p(h, s)
    }

    /// `sprintf(buf, "%d", v)` then [`put_p`].
    fn put_d(h: &mut impl Host, ebp: u32, v: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x24);
        let fmt = h.rebase(FMT_D);
        sprintf(h, &[buf, fmt, v]);
        put_p(h, buf)
    }

    /// `sprintf(buf, "%f", (double) f)` then [`put_p`].
    fn put_f(h: &mut impl Host, ebp: u32, f: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x24);
        let fmt = h.rebase(FMT_F);
        let d = f64::from(f32::from_bits(f)).to_bits();
        sprintf(h, &[buf, fmt, d as u32, (d >> 32) as u32]);
        put_p(h, buf)
    }

    fn worker_level(h: &mut impl Host, worker: u32) -> u32 {
        let n = u32::from(h.read::<u8>(at(worker, 0x32)));
        cdecl(h, SCRIPT_WORKER_LEVEL, &[worker, n])
    }

    /// `0x0087b0c0(worker)`: a character's motion number (the level
    /// work's `+0x48` block's `u16 +2`) in the motion format, fixed width,
    /// in colour 7 while bit 15 is set (the colour restored after); other
    /// workers the "no motion" text. Returns the print's `eax`.
    pub fn print_motion_no(h: &mut impl Host, ebp: u32, worker: u32) -> u32 {
        let head: u32 = h.read(worker);
        if h.read::<u8>(head) != 1 {
            return put_sym(h, NO_MOTION);
        }
        let saved = FONT_COLOR.get(h);
        let l = worker_level(h, worker);
        let block: u32 = h.read(at(l, 0x48));
        let m = u32::from(h.read::<u16>(at(block, 2)));
        if m & 0x8000 != 0 {
            FONT_COLOR.set(h, 7);
        }
        let buf = ebp.wrapping_sub(0x24);
        let fmt = h.rebase(FMT_MOTION);
        sprintf(h, &[buf, fmt, m]);
        let c = FONT_COLOR.get(h);
        let r = cdecl(h, TK_PUT_DEBUG_FONT_STRING, &[c, 0, buf]);
        FONT_COLOR.set(h, saved);
        r
    }

    /// The name of state `v` (`0x0087b170`'s jump tables), if it has one.
    fn state_name(v: u32) -> Option<u32> {
        const LOW: [u32; 13] = [
            0x00b5_cec0,
            0x00b5_cecc,
            0x00b5_ced4,
            0x00b5_cee0,
            0x00b5_ceec,
            0x00b5_cef8,
            0x00b5_cf00,
            0x00b5_cf0c,
            0x00b5_cf18,
            0x00b5_cf24,
            0x00b5_cf30,
            0x00b5_cf3c,
            0x00b5_cf48,
        ];
        Some(match v {
            0..=0xc => LOW[v as usize],
            0xd => 0x00b5_cf54,
            0xe => 0x00b5_cf60,
            0x14 => 0x00b5_cf6c,
            0x15 => 0x00b5_cf78,
            0x16 => 0x00b5_cf84,
            0x17 => 0x00b5_cf90,
            0x18 => 0x00b5_cf9c,
            0x19 => 0x00b5_cfa8,
            0x1a => 0x00b5_cfb4,
            0x1b => 0x00b5_cfc0,
            0xffff_ffff => 0x00b5_ceb4,
            _ => return None,
        })
    }

    /// `0x0087b170(worker)`: the worker's state `+0x40` by name
    /// (`TkPutDebugFontStringPEuc`), or as a number. Returns the print's
    /// `eax`.
    pub fn print_state(h: &mut impl Host, ebp: u32, worker: u32) -> u32 {
        let v: u32 = h.read(at(worker, 0x40));
        match state_name(v) {
            Some(s) => {
                let s = h.rebase(s);
                let c = FONT_COLOR.get(h);
                cdecl(h, TK_PUT_DEBUG_FONT_STRING_P_EUC, &[c, 0, s])
            }
            None => put_d(h, ebp, v),
        }
    }

    /// `0x0087b360(worker)`: the byte `+0xaf` as a number, 0xf as a text.
    pub fn print_byte_af(h: &mut impl Host, ebp: u32, worker: u32) -> u32 {
        let b = h.read::<u8>(at(worker, 0xaf));
        if b == 0xf {
            return put_sym(h, 0x00b5_cfcc);
        }
        put_d(h, ebp, u32::from(b))
    }

    /// `0x0087ee10(worker, i)`: with bit `i` of `u16 +0xb14`, slot `i`'s
    /// number (`u16 +0xb04 + 2i`) and its two flags (`+0xb08 + 4i` bits 0
    /// and 1, each one of two marks); else the empty mark.
    pub fn print_slot(h: &mut impl Host, ebp: u32, worker: u32, i: u32) -> u32 {
        let bits = u32::from(h.read::<u16>(at(worker, 0xb14)));
        if (1u32 << (i & 31)) & bits == 0 {
            return put_sym(h, 0x00b7_c97c);
        }
        let n = u32::from(h.read::<u16>(at(worker, 0xb04 + 2 * i)));
        put_d(h, ebp, n);
        let f: u8 = h.read(at(worker, 0xb08 + 4 * i));
        put_sym(h, if f & 1 != 0 { 0x00b5_cfd4 } else { 0x00b5_cfd8 });
        let f: u8 = h.read(at(worker, 0xb08 + 4 * i));
        put_sym(h, if f & 2 != 0 { 0x00b5_cfd4 } else { 0x00b5_cfd8 })
    }

    /// `0x0087b420(worker)`: the model's body hit (`Ch_GetBodyHit`) as on
    /// or off; no model (`+0x9c`) its text.
    pub fn print_body_hit(h: &mut impl Host, worker: u32) -> u32 {
        let m: u32 = h.read(at(worker, 0x9c));
        if m == 0 {
            return put_sym(h, NO_MODEL);
        }
        let on = cdecl(h, CH_GET_BODY_HIT, &[m]);
        put_sym(h, if on != 0 { TEXT_ON } else { TEXT_OFF })
    }

    /// `0x0087b490(worker)`: the model's thickness (`Ch_GetThickness`,
    /// stored as `f32` at `ebp - 0x28` on the way), `%f`; no model its
    /// text.
    pub fn print_thickness(h: &mut impl Host, ebp: u32, worker: u32) -> u32 {
        let m: u32 = h.read(at(worker, 0x9c));
        if m == 0 {
            return put_sym(h, NO_MODEL);
        }
        let t = h.call_sym_st0(CH_GET_THICKNESS, Conv::Cdecl, &[m]);
        h.write(ebp.wrapping_sub(0x28), t.to_bits());
        put_f(h, ebp, t.to_bits())
    }

    /// `0x0087b520(worker)`: the position work's `f32 +0x2c`, `%f`.
    pub fn print_pos_2c(h: &mut impl Host, ebp: u32, worker: u32) -> u32 {
        let pos = cdecl(h, ATEL_GET_POS_WORK_ADRS, &[worker]);
        let f: u32 = h.read(at(pos, 0x2c));
        put_f(h, ebp, f)
    }

    /// A flag printer ([`super::FLAGS`]).
    pub fn print_flag(h: &mut impl Host, f: Flag, worker: u32) -> u32 {
        let b: u8 = h.read(at(worker, f.offset));
        let s = h.rebase(if b & f.mask != 0 { f.on } else { f.off });
        let c = if f.colour0 { 0 } else { FONT_COLOR.get(h) };
        cdecl(h, TK_PUT_DEBUG_FONT_STRING_P, &[c, 0, s])
    }

    /// `0x0087b610(worker)`: the level byte `+0x32`, `%d`.
    pub fn print_level(h: &mut impl Host, ebp: u32, worker: u32) -> u32 {
        let b = u32::from(h.read::<u8>(at(worker, 0x32)));
        put_d(h, ebp, b)
    }

    /// `PrintCtrlPlayer_0087b660()`: the debugger's control work's player
    /// (`u16 +0xa`): none prints its text, else its entry
    /// (`AtelDebug_PrintEntry(AtelGetBasicWork(id))`).
    pub fn print_ctrl_player(h: &mut impl Host) -> u32 {
        let c = DEBUG_CTRL.get(h);
        let w = cdecl(h, ATEL_GET_CTRL_WORK, &[c]);
        let id = u32::from(h.read::<u16>(at(w, 0xa)));
        if id == 0xffff {
            return put_sym(h, 0x00b5_cfdc);
        }
        let b = cdecl(h, ATEL_GET_BASIC_WORK, &[id]);
        cdecl(h, ATEL_DEBUG_PRINT_ENTRY, &[b])
    }

    /// A control work `f32` printer ([`super::CTRL_FLOATS`]).
    pub fn print_ctrl_float(h: &mut impl Host, ebp: u32, offset: u32) -> u32 {
        let c = DEBUG_CTRL.get(h);
        let w = cdecl(h, ATEL_GET_CTRL_WORK, &[c]);
        let f: u32 = h.read(at(w, offset));
        put_f(h, ebp, f)
    }

    /// `0x0087b8f0(worker)`: the name of the worker's bits 10..12
    /// (`Atel_GetWorkerBits10_12(u16 +0x2e)` into the `.data` table).
    pub fn print_bits_name(h: &mut impl Host, worker: u32) -> u32 {
        let n = u32::from(h.read::<u16>(at(worker, 0x2e)));
        let k = cdecl(h, ATEL_GET_WORKER_BITS, &[n]);
        let t = h.rebase(BITS_NAMES);
        let s: u32 = h.read(at(t, k.wrapping_mul(4)));
        put_p(h, s)
    }

    /// `0x0087b920(worker)`: on/off of `u16 +0x34` bit 9 (a third text
    /// with `+0x36` bit 3), then one of two texts by `+0x34` bit 5.
    pub fn print_flags_34(h: &mut impl Host, worker: u32) -> u32 {
        let w = h.read::<u16>(at(worker, 0x34));
        let mut s = if w & 0x200 != 0 { TEXT_ON } else { TEXT_OFF };
        if h.read::<u8>(at(worker, 0x36)) & 8 != 0 {
            s = 0x00b5_cfe4;
        }
        put_sym(h, s);
        let b = h.read::<u8>(at(worker, 0x34));
        put_sym(
            h,
            if b & 0x20 != 0 {
                0x00b5_cfe8
            } else {
                0x00b5_cff0
            },
        )
    }

    /// `0x0087b980(worker)`: one of three texts by `u16 +0x38` bits 2 and
    /// 8 (8 wins).
    pub fn print_flags_38(h: &mut impl Host, worker: u32) -> u32 {
        let v = h.read::<u16>(at(worker, 0x38));
        let mut s = 0x00b5_ca88;
        if v & 4 != 0 {
            s = 0x00b5_cff8;
        }
        if v & 0x100 != 0 {
            s = 0x00b5_d000;
        }
        put_sym(h, s)
    }

    /// `0x0087b9c0(worker)`: the program counter (`Script_PcOffset`) in
    /// the colour of the level's mode byte `+0x1f` (0 colour 0, 1 colour
    /// 7, 2 colour 2, else as it is; restored after).
    pub fn print_pc(h: &mut impl Host, ebp: u32, worker: u32) -> u32 {
        let l = worker_level(h, worker);
        let saved = FONT_COLOR.get(h);
        match h.read::<u8>(at(l, 0x1f)) {
            0 => FONT_COLOR.set(h, 0),
            1 => FONT_COLOR.set(h, 7),
            2 => FONT_COLOR.set(h, 2),
            _ => {}
        }
        let l = worker_level(h, worker);
        let pc = cdecl(h, SCRIPT_PC_OFFSET, &[worker, l]);
        let buf = ebp.wrapping_sub(0x24);
        let fmt = h.rebase(FMT_PC);
        sprintf(h, &[buf, fmt, pc]);
        let r = put_p(h, buf);
        FONT_COLOR.set(h, saved);
        r
    }

    /// `0x0087baa0(worker)`: the byte `+0x33` in the motion format, fixed
    /// width.
    pub fn print_byte_33(h: &mut impl Host, ebp: u32, worker: u32) -> u32 {
        let b = u32::from(h.read::<u8>(at(worker, 0x33)));
        let buf = ebp.wrapping_sub(0x24);
        let fmt = h.rebase(FMT_MOTION);
        sprintf(h, &[buf, fmt, b]);
        let c = FONT_COLOR.get(h);
        cdecl(h, TK_PUT_DEBUG_FONT_STRING, &[c, 0, buf])
    }

    /// `0x0087baf0(worker)`: in colour 0, one of two texts by `+0x34`
    /// bit 6, then one of two by the debugger's control work's byte `+8`.
    pub fn print_flags_ctrl(h: &mut impl Host, worker: u32) -> u32 {
        let b = h.read::<u8>(at(worker, 0x34));
        let s = h.rebase(if b & 0x40 != 0 {
            0x00b5_cd54
        } else {
            0x00b5_cd58
        });
        cdecl(h, TK_PUT_DEBUG_FONT_STRING_P, &[0, 0, s]);
        let c = DEBUG_CTRL.get(h);
        let w = cdecl(h, ATEL_GET_CTRL_WORK, &[c]);
        let s = if h.read::<u8>(at(w, 8)) != 0 {
            0x00b5_cd5c
        } else {
            0x00b5_cd64
        };
        let s = h.rebase(s);
        cdecl(h, TK_PUT_DEBUG_FONT_STRING_P, &[0, 0, s])
    }

    /// `0x0087bb40(worker)`: the word `+0xc4`, `%d`.
    pub fn print_word_c4(h: &mut impl Host, ebp: u32, worker: u32) -> u32 {
        let v: u32 = h.read(at(worker, 0xc4));
        put_d(h, ebp, v)
    }

    /// `0x0087bb90(worker)`: the length of the list at `+0x90`, `%d`, in
    /// colour 7 from 8 entries and 5 from 12 (restored after).
    pub fn print_list_len(h: &mut impl Host, ebp: u32, worker: u32) -> u32 {
        let mut p: u32 = h.read(at(worker, 0x90));
        let saved = FONT_COLOR.get(h);
        let mut n = 0u32;
        if p != 0 {
            loop {
                p = h.read(p);
                n = n.wrapping_add(1);
                if p == 0 {
                    break;
                }
            }
            if (n as i32) >= 8 {
                FONT_COLOR.set(h, 7);
            }
            if (n as i32) >= 0xc {
                FONT_COLOR.set(h, 5);
            }
        }
        let r = put_d(h, ebp, n);
        FONT_COLOR.set(h, saved);
        r
    }

    /// `0x0087e6d0(s)`: `TkPutDebugFontStringPR(colour, 0, s)`.
    pub fn print_right(h: &mut impl Host, s: u32) -> u32 {
        let c = FONT_COLOR.get(h);
        cdecl(h, TK_PUT_DEBUG_FONT_STRING_PR, &[c, 0, s])
    }

    /// `AtelSetUpSourceDebug(src, len, on, page, size)` (`0x00885b00`): a
    /// source file's pages for the viewer: with `src` and `on` set, page
    /// records (16 bytes: next, line count, text start, 0) chained from
    /// `page`, a new one after each 64 lines while `size / 16` records
    /// last; the text ends at `src + len`, a 0 or a `0x1a`. The last
    /// record ends the chain. Returns `eax` as the original leaves it
    /// (the scan's position, or `src`).
    pub fn set_up_source_debug(
        h: &mut impl Host,
        src: u32,
        len: u32,
        on: u32,
        page: u32,
        size: u32,
    ) -> u32 {
        if src == 0 || on == 0 {
            return src;
        }
        let end = src.wrapping_add(len);
        let mut rec = page;
        h.write(at(rec, 8), src);
        h.write(at(rec, 0xc), 0u32);
        let mut b: u8 = h.read(src);
        let mut p = at(src, 1);
        let mut left = (size >> 4) as i32;
        let mut lines = 0u32;
        if end > p {
            loop {
                if b == 0 || b == 0x1a {
                    break;
                }
                if b == b'\n' {
                    lines += 1;
                    if lines >= 0x40 {
                        left -= 1;
                        if left <= 0 {
                            break;
                        }
                        h.write(at(rec, 4), lines);
                        let next = at(rec, 0x10);
                        h.write(rec, next);
                        rec = next;
                        lines = 0;
                        h.write(at(rec, 8), p);
                        h.write(at(rec, 0xc), 0u32);
                    }
                }
                b = h.read(p);
                p = at(p, 1);
                if end <= p {
                    break;
                }
            }
        }
        h.write(rec, 0u32);
        h.write(at(rec, 4), lines);
        p
    }

    /// `AtelDebug_PrintSlot_0087ee10`.
    pub const PRINT_SLOT: u32 = 0x0087_ee10;

    /// `0x0087b3e0(worker)` / `0x0087b400(worker)`: slot 0 / 1
    /// (`AtelDebug_PrintSlot_0087ee10(worker, i)`'s `eax`).
    pub fn print_slot_n(h: &mut impl Host, worker: u32, i: u32) -> u32 {
        cdecl(h, PRINT_SLOT, &[worker, i])
    }
}
