//! M5 stage 1, list `menu`, batch 3: the event debugger's value printers
//! (`0x0087a4a0` to `0x0087b040`, entries of a display table: each prints
//! one field of a script worker in the debug font) and the debug font's
//! entry points.
//!
//! Each printer formats into a 32-byte buffer at `ebp - 0x24` of its own
//! frame (the security cookie at `ebp - 4`) with the CRT's `sprintf` and
//! prints it at the cursor in the colour [`bridge::FONT_COLOR`]
//! (`TkPutDebugFontStringP`, proportional, or `TkPutDebugFontString`).
//! The bridges take that frame's `ebp`; the shims pass a frame of the
//! same size. The fields:
//!
//! - **position work** (`AtelGetPosWorkAdrs(worker)`): an `f32` at an
//!   offset (`%f`), or one passed through `Math_AngleToDegrees` first;
//! - **level work** (`Script_WorkerLevel(worker, worker +0x32)`): through
//!   its `+0x44` or `+0x48` pointer, an `f32` (`%f`) or an `i16` (`%d`).

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

/// Where a printer's value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// An `f32` at this offset of the position work, `%f`.
    Pos(u32),
    /// The position work's `f32` at this offset in degrees
    /// (`Math_AngleToDegrees`), `%f`.
    PosAngle(u32),
    /// An `f32` at this offset of the level work's `+0x44` block, `%f`.
    Level(u32),
    /// An `i16` at (`+0x44` or `+0x48` block, offset) of the level work,
    /// `%d`, printed with the fixed-width font.
    LevelI16(u32, u32),
}

/// The printers: address and field.
pub const PRINTERS: [(u32, Field); 26] = [
    (0x0087_a4a0, Field::Pos(0x0)),
    (0x0087_a4f0, Field::Pos(0x4)),
    (0x0087_a550, Field::Pos(0x8)),
    (0x0087_a5b0, Field::Level(0x28)),
    (0x0087_a610, Field::Level(0x2c)),
    (0x0087_a670, Field::Level(0x30)),
    (0x0087_a6d0, Field::Pos(0xc)),
    (0x0087_a730, Field::Level(0x10)),
    (0x0087_a790, Field::Pos(0x1c)),
    (0x0087_a7f0, Field::Level(0x38)),
    (0x0087_a850, Field::LevelI16(0x44, 6)),
    (0x0087_a8b0, Field::LevelI16(0x44, 4)),
    (0x0087_a910, Field::LevelI16(0x48, 6)),
    (0x0087_a970, Field::LevelI16(0x48, 4)),
    (0x0087_a9d0, Field::Pos(0x20)),
    (0x0087_aa40, Field::PosAngle(0x20)),
    (0x0087_aaa0, Field::Pos(0x24)),
    (0x0087_ab00, Field::Pos(0x34)),
    (0x0087_ab60, Field::PosAngle(0x34)),
    (0x0087_abc0, Field::Pos(0x30)),
    (0x0087_ac20, Field::Pos(0x38)),
    (0x0087_ac80, Field::Pos(0x40)),
    (0x0087_ace0, Field::Pos(0x48)),
    (0x0087_ad40, Field::Pos(0x44)),
    (0x0087_ada0, Field::Pos(0x3c)),
    (0x0087_ae00, Field::Pos(0x4c)),
];

pub mod bridge {
    use ffx_bridge::Global;
    use ffx_graphic::host::{Conv, Host};

    use super::Field;

    /// The debug font's colour index.
    pub const FONT_COLOR: Global<u32> = Global::new(0x0132_8a74);
    /// The debug font's cursor x (`TkPutDebugFontStringPR` moves it left).
    pub const FONT_POS: Global<i32> = Global::new(0x0133_c8d4);

    /// The import slot of the CRT's `sprintf`.
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    /// `"%f"`, `"%d"`, `"%x"` in `.rdata`.
    pub const FMT_F: u32 = 0x00b4_0274;
    pub const FMT_D: u32 = 0x00b0_f604;
    pub const FMT_X: u32 = 0x00b4_1f0c;
    pub const ATEL_GET_POS_WORK_ADRS: u32 = 0x0086_c2b0;
    pub const SCRIPT_WORKER_LEVEL: u32 = 0x0087_2650;
    pub const MATH_ANGLE_TO_DEGREES: u32 = 0x0088_c5c0;
    pub const CH_GET_FIELD_MODE: u32 = 0x0082_ab90;
    pub const TK_GET_STR_WIDTH: u32 = 0x0088_ee70;
    pub const TK_PUT_DEBUG_FONT_CORE: u32 = 0x0088_f240;
    pub const TK_PUT_DEBUG_FONT_STRING: u32 = 0x0088_f140;
    pub const TK_PUT_DEBUG_FONT_STRING_P: u32 = 0x0088_f4c0;
    pub const TK_PUT_DEBUG_FONT_STRING_P_EUC: u32 = 0x0088_f4e0;
    /// The worker kinds' names (`+0xaa`), the field modes' names, the
    /// motion names (`0x0087b040`), in `.data`.
    pub const KIND_NAMES: u32 = 0x00c5_35b4;
    pub const FIELD_MODE_NAMES: u32 = 0x00c5_35cc;
    pub const MOTION_NAMES: u32 = 0x00c5_35fc;
    /// Short strings in `.rdata`: the kind 2 name with flag 0x10, the
    /// separator, the brackets, the "no motion" text.
    pub const KIND2_ALT: u32 = 0x00b5_ce98;
    pub const SEPARATOR: u32 = 0x00b5_ce9c;
    pub const OPEN: u32 = 0x00b5_cea0;
    pub const CLOSE: u32 = 0x00b5_cea4;
    pub const NO_MOTION: u32 = 0x00b4_865c;

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `sprintf(buf, fmt, words...)` through the import table.
    fn sprintf(h: &mut impl Host, args: &[u32]) -> u32 {
        let slot = h.rebase(SPRINTF_SLOT);
        let f: u32 = h.read(slot);
        h.call(f, Conv::Cdecl, args)
    }

    /// `TkPutDebugFontStringP(color, 0, s)` in the current colour.
    fn put_p(h: &mut impl Host, s: u32) -> u32 {
        let c = FONT_COLOR.get(h);
        cdecl(h, TK_PUT_DEBUG_FONT_STRING_P, &[c, 0, s])
    }

    /// One value printer (`PRINTERS`): `worker`'s field formatted into the
    /// frame's buffer and printed. Returns the print's `eax`.
    pub fn print_field(h: &mut impl Host, ebp: u32, field: Field, worker: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x24);
        let (fmt, words): (u32, Vec<u32>) = match field {
            Field::Pos(o) => {
                let pos = cdecl(h, ATEL_GET_POS_WORK_ADRS, &[worker]);
                let v = f64::from(f32::from_bits(h.read(at(pos, o))));
                (FMT_F, double(v))
            }
            Field::PosAngle(o) => {
                let pos = cdecl(h, ATEL_GET_POS_WORK_ADRS, &[worker]);
                let a: u32 = h.read(at(pos, o));
                let deg = h.call_sym_st0(MATH_ANGLE_TO_DEGREES, Conv::Cdecl, &[a]);
                // the original spills the f32 to ebp - 0x28 on its way
                h.write(ebp.wrapping_sub(0x28), deg.to_bits());
                (FMT_F, double(f64::from(deg)))
            }
            Field::Level(o) => {
                let l = worker_level(h, worker);
                let block: u32 = h.read(at(l, 0x44));
                let v = f64::from(f32::from_bits(h.read(at(block, o))));
                (FMT_F, double(v))
            }
            Field::LevelI16(b, o) => {
                let l = worker_level(h, worker);
                let block: u32 = h.read(at(l, b));
                let v = i32::from(h.read::<i16>(at(block, o)));
                (FMT_D, vec![v as u32])
            }
        };
        let mut args = vec![buf, h.rebase(fmt)];
        args.extend(words);
        sprintf(h, &args); // call width: 4 at most (a double or one word)
        let c = FONT_COLOR.get(h);
        let put = if matches!(field, Field::LevelI16(..)) {
            TK_PUT_DEBUG_FONT_STRING
        } else {
            TK_PUT_DEBUG_FONT_STRING_P
        };
        cdecl(h, put, &[c, 0, buf])
    }

    /// `Script_WorkerLevel(worker, worker +0x32)`.
    fn worker_level(h: &mut impl Host, worker: u32) -> u32 {
        let n = u32::from(h.read::<u8>(at(worker, 0x32)));
        cdecl(h, SCRIPT_WORKER_LEVEL, &[worker, n])
    }

    /// A `double` as the two stack words the original pushes.
    fn double(v: f64) -> Vec<u32> {
        let b = v.to_bits();
        vec![b as u32, (b >> 32) as u32]
    }

    /// `0x0087ae60(worker)`: the worker's kind name (`+0xaa` into the name
    /// table; kind 2 with flag 0x10 its alternative; colour 2 while flag
    /// 0x20, restored after), a separator, its `+0xa8` (`%d`), and for a
    /// character (kind 1) with a model `+0x9c` the separator and its field
    /// mode's name (`Ch_GetFieldMode`). Returns what the original leaves in
    /// `eax`: the last print's result, or the kind minus 1 (0 for a
    /// character without a model).
    pub fn print_kind(h: &mut impl Host, ebp: u32, worker: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x24);
        let kind = h.read::<u8>(at(worker, 0xaa));
        let table = h.rebase(KIND_NAMES);
        let mut name: u32 = h.read(at(table, u32::from(kind) * 4));
        let flags = h.read::<u8>(at(worker, 0x36));
        if kind == 2 && flags & 0x10 != 0 {
            name = h.rebase(KIND2_ALT);
        }
        let saved = FONT_COLOR.get(h);
        let mut color = saved;
        if flags & 0x20 != 0 {
            color = 2;
            FONT_COLOR.set(h, 2);
        }
        cdecl(h, TK_PUT_DEBUG_FONT_STRING_P, &[color, 0, name]);
        FONT_COLOR.set(h, saved);
        let sep = h.rebase(SEPARATOR);
        cdecl(h, TK_PUT_DEBUG_FONT_STRING_P, &[saved, 0, sep]);
        let v = u32::from(h.read::<u16>(at(worker, 0xa8)));
        let fmt = h.rebase(FMT_D);
        sprintf(h, &[buf, fmt, v]);
        put_p(h, buf);
        let kind = u32::from(h.read::<u8>(at(worker, 0xaa)));
        if kind != 1 {
            return kind.wrapping_sub(1);
        }
        let model: u32 = h.read(at(worker, 0x9c));
        if model == 0 {
            return 0;
        }
        let mode = cdecl(h, CH_GET_FIELD_MODE, &[model]);
        put_p(h, sep);
        let modes = h.rebase(FIELD_MODE_NAMES);
        let s: u32 = h.read(at(modes, mode.wrapping_mul(4)));
        put_p(h, s)
    }

    /// `0x0087af50(worker)`: the worker's model pointer `+0x9c` in hex,
    /// fixed width. Returns the print's `eax`.
    pub fn print_model(h: &mut impl Host, ebp: u32, worker: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x24);
        let v: u32 = h.read(at(worker, 0x9c));
        let fmt = h.rebase(FMT_X);
        sprintf(h, &[buf, fmt, v]);
        let c = FONT_COLOR.get(h);
        cdecl(h, TK_PUT_DEBUG_FONT_STRING, &[c, 0, buf])
    }

    /// `0x0087afa0(entry)`: an entry's number `+0x2e` (`%d`) and, with a
    /// string table `+0x08` (offsets from `table + table[0x50]`), its name
    /// in brackets (the brackets through `TkPutDebugFontStringPEuc`).
    /// Returns the last print's `eax`.
    pub fn print_entry(h: &mut impl Host, ebp: u32, entry: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x24);
        let n = u32::from(h.read::<u16>(at(entry, 0x2e)));
        let table: u32 = h.read(at(entry, 8));
        let fmt = h.rebase(FMT_D);
        sprintf(h, &[buf, fmt, n]);
        let r = put_p(h, buf);
        if table == 0 {
            return r;
        }
        let n = u32::from(h.read::<u16>(at(entry, 0x2e)));
        let base = at(table, h.read(at(table, 0x50)));
        let name = at(base, h.read(at(base, n * 4)));
        let open = h.rebase(OPEN);
        let c = FONT_COLOR.get(h);
        cdecl(h, TK_PUT_DEBUG_FONT_STRING_P_EUC, &[c, 0, open]);
        put_p(h, name);
        let close = h.rebase(CLOSE);
        let c = FONT_COLOR.get(h);
        cdecl(h, TK_PUT_DEBUG_FONT_STRING_P_EUC, &[c, 0, close])
    }

    /// `0x0087b040(worker)`: a character's motion name (the level work's
    /// `+0x44` block's `u16` `+2`, low 7 bits into the name table; colour
    /// 7 while bit 15, restored after); other workers the "no motion"
    /// text. Returns the print's `eax`.
    pub fn print_motion(h: &mut impl Host, worker: u32) -> u32 {
        let head: u32 = h.read(worker);
        if h.read::<u8>(head) != 1 {
            let s = h.rebase(NO_MOTION);
            return put_p(h, s);
        }
        let saved = FONT_COLOR.get(h);
        let l = worker_level(h, worker);
        let block: u32 = h.read(at(l, 0x44));
        let m = u32::from(h.read::<u16>(at(block, 2)));
        let color = if m & 0x8000 != 0 {
            FONT_COLOR.set(h, 7);
            7
        } else {
            FONT_COLOR.get(h)
        };
        let names = h.rebase(MOTION_NAMES);
        let s: u32 = h.read(at(names, (m & 0x7f) * 4));
        let r = cdecl(h, TK_PUT_DEBUG_FONT_STRING_P, &[color, 0, s]);
        FONT_COLOR.set(h, saved);
        r
    }

    /// `TkPutDebugFontString(color, flags, s)` (`0x0088f140`): the core
    /// (`0x0088f240`) with the fixed-width font. Returns its `eax`.
    pub fn put_string(h: &mut impl Host, color: u32, flags: u32, s: u32) -> u32 {
        cdecl(h, TK_PUT_DEBUG_FONT_CORE, &[0, color, flags, s])
    }

    /// `TkPutDebugFontStringP(color, flags, s)` (`0x0088f4c0`): the core,
    /// proportional.
    pub fn put_string_p(h: &mut impl Host, color: u32, flags: u32, s: u32) -> u32 {
        cdecl(h, TK_PUT_DEBUG_FONT_CORE, &[1, color, flags, s])
    }

    /// `TkPutDebugFontStringPR(color, flags, s)` (`0x0088f5c0`): right
    /// aligned: the cursor moves left by the text's width
    /// (`TkGetStrWidth(1, s)`), then [`put_string_p`].
    pub fn put_string_pr(h: &mut impl Host, color: u32, flags: u32, s: u32) -> u32 {
        let w = cdecl(h, TK_GET_STR_WIDTH, &[1, s]) as i32;
        let x = FONT_POS.get(h).wrapping_sub(w);
        FONT_POS.set(h, x);
        cdecl(h, TK_PUT_DEBUG_FONT_CORE, &[1, color, flags, s])
    }
}
