//! M5 stage 1, list `vfx`, round 5 batch 11: the battle debug op script
//! compiler's driver (`Ms_Gui_BattleProc`'s file command): the tokenizer
//! tables set up, the source compiled into a 4 KiB buffer and written to
//! the output file.
//!
//! The output record the original keeps in its frame comes in as an
//! address (the shim passes a `StackFrame`). Also the field effects' (FP)
//! debug listing (`pppFpDispDebug`).

use ffx_bridge::Global;

/// The compiler's state: two error counters, the symbol tree's root and
/// tail, a word and 16 flag bytes, the last error, the class table and
/// the keyword table.
pub const ERRORS: Global<u32> = Global::new(0x0113_fc58);
pub const WARNINGS: Global<u32> = Global::new(0x0113_fc5c);
pub const TREE_ROOT: Global<u32> = Global::new(0x0113_fc84);
pub const TREE_TAIL: Global<u32> = Global::new(0x0113_fc88);
pub const STATE_WORD: Global<u32> = Global::new(0x0113_fc78);
pub const FLAGS: u32 = 0x0113_fc68;
pub const LAST_ERROR: Global<u32> = Global::new(0x0113_e238);
pub const CHAR_CLASS: u32 = 0x0113_fb58;
pub const KEYWORDS: u32 = 0x00c4_4a50;
/// The character class strings (classes 2, 3, 4, 7, 1, 6, 8).
pub const CLASS_STRINGS: [u32; 7] = [
    0x00b5_7dd0,
    0x00b5_7dcc,
    0x00b4_bc60,
    0x00b5_7dc8,
    0x00b5_7dc4,
    0x00b5_7dc0,
    0x00b8_f884,
];
/// The failure messages (errors, warnings, the file).
pub const ERRORS_TEXT: u32 = 0x00b5_7e14;
pub const WARNINGS_TEXT: u32 = 0x00b5_7e1c;
pub const OPEN_FAILED_TEXT: u32 = 0x00b5_7e24;
/// `MSVCR110` slots: `malloc`, `free` (cdecl).
pub const MALLOC_SLOT: u32 = 0x00b0_c468;
pub const FREE_SLOT: u32 = 0x00b0_c370;

/// `g_pppFpDisp`, `g_FpData`, `g_FpRun` (0x80 each), `g_FpDisp` (0x4c
/// each), the state and type name tables, and the listing's texts.
pub const PPP_FP_DISP: Global<u32> = Global::new(0x00ce_c1f0);
pub const FP_DATA: Global<u32> = Global::new(0x00d2_c258);
pub const FP_RUN: u32 = 0x00d2_c260;
pub const FP_DISP: u32 = 0x00d3_0260;
pub const STATE_NAMES: u32 = 0x00c3_a4e0;
pub const TYPE_NAMES: u32 = 0x00c3_a4ec;
pub const LIST_HEAD: [u32; 4] = [0x00b5_1414, 0x00b5_1430, 0x00b5_1440, 0x00b5_146c];
pub const ON_TEXT: u32 = 0x00b4_0648;
pub const OFF_TEXT: u32 = 0x00b4_0644;
pub const SHOWN_TEXT: u32 = 0x00b5_1498;
pub const ENTRY_FORMAT: u32 = 0x00b5_149c;
pub const LIST_END: u32 = 0x00b5_14c4;

/// `g_pppCameraMatrix` and `pppFpInfPrint`'s three formats.
pub const CAMERA_MATRIX: u32 = 0x0230_ff60;
pub const CAMERA_FORMAT: u32 = 0x00b5_13ac;
pub const PLAYER_FORMAT: u32 = 0x00b5_13bc;
pub const DISTANCE_FORMAT: u32 = 0x00b5_13cc;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
/// `FFXVu0InversMatrix(dst, src)`, `FFXVu0CopyVector(dst, src)`,
/// `Sg_GetPlayer()`, `Ch_GetPos(chr, &x, &y, &z)`, `FFXVu0SubVector(dst,
/// a, b)`, `pppVu0Length_copy_0072ab20(v)` (ST0), `op_num_xy_set(x, y)`,
/// `op_num(text)` (cdecl).
pub const INVERSE_MATRIX: u32 = 0x006e_d450;
pub const COPY_VECTOR: u32 = 0x006e_d370;
pub const GET_PLAYER: u32 = 0x0082_d860;
pub const CH_GET_POS: u32 = 0x0082_ac90;
pub const SUB_VECTOR: u32 = 0x006e_d940;
pub const VECTOR_LENGTH: u32 = 0x0072_ab20;
pub const OP_NUM_XY_SET: u32 = 0x007e_65e0;
pub const OP_NUM: u32 = 0x007e_63a0;

// ---- callees ----

/// `MsDbgFile_charClasses_007e12d0(table, s1..s7)`,
/// `MsDbgFile_hashNames_007e2d90(table)`, `FUN_007e2270(source, out)` (the
/// compiler), `MsDbgFile_freeTree_007e1b70(node)`, `sceOpen(name, mode)`,
/// `sceWrite(fd, p, n)`, `sceClose(fd)`, `dbgPrintf(fmt, ...)` (cdecl).
pub const CHAR_CLASSES: u32 = 0x007e_12d0;
pub const HASH_NAMES: u32 = 0x007e_2d90;
pub const COMPILE: u32 = 0x007e_2270;
pub const FREE_TREE: u32 = 0x007e_1b70;
pub const SCE_OPEN: u32 = 0x0062_fa30;
pub const SCE_WRITE: u32 = 0x0062_fdc0;
pub const SCE_CLOSE: u32 = 0x0062_f610;
pub const DBG_PRINTF: u32 = 0x0062_f500;

pub mod bridge {
    use super::{
        CAMERA_FORMAT, CAMERA_MATRIX, CH_GET_POS, CHAR_CLASS, CHAR_CLASSES, CLASS_STRINGS, COMPILE,
        COPY_VECTOR, DBG_PRINTF, DISTANCE_FORMAT, ENTRY_FORMAT, ERRORS, ERRORS_TEXT, FLAGS,
        FP_DATA, FP_DISP, FP_RUN, FREE_SLOT, FREE_TREE, GET_PLAYER, HASH_NAMES, INVERSE_MATRIX,
        KEYWORDS, LAST_ERROR, LIST_END, LIST_HEAD, MALLOC_SLOT, OFF_TEXT, ON_TEXT, OP_NUM,
        OP_NUM_XY_SET, OPEN_FAILED_TEXT, PLAYER_FORMAT, PPP_FP_DISP, SCE_CLOSE, SCE_OPEN,
        SCE_WRITE, SHOWN_TEXT, SPRINTF_SLOT, STATE_NAMES, STATE_WORD, SUB_VECTOR, TREE_ROOT,
        TREE_TAIL, TYPE_NAMES, VECTOR_LENGTH, WARNINGS, WARNINGS_TEXT,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `op_script_compile_007e1010(source, path)`: the tokenizer's class
    /// table and keyword hashes set up, the state cleared, the output
    /// file opened (0x602) with a 4 KiB `malloc` buffer (the record at
    /// `out`, the original's `ebp - 0x14`: buffer, 1, descriptor, used,
    /// 0x1000). A file that doesn't open reports the counters and the path
    /// and sets the last error to 2; otherwise the source is compiled
    /// (`FUN_007e2270`) and what it produced written, the buffer freed and
    /// the file closed. The symbol tree is freed either way.
    pub fn compile(h: &mut impl Host, out: u32, source: u32, path: u32) {
        ERRORS.set(h, 0);
        WARNINGS.set(h, 0);
        let table = h.rebase(CHAR_CLASS);
        let mut args = vec![table];
        for s in CLASS_STRINGS {
            args.push(h.rebase(s));
        }
        // call width: 8
        cdecl(h, CHAR_CLASSES, &args);
        let keywords = h.rebase(KEYWORDS);
        cdecl(h, HASH_NAMES, &[keywords]);
        TREE_ROOT.set(h, 0);
        let root = h.rebase(TREE_ROOT.sym);
        TREE_TAIL.set(h, root);
        STATE_WORD.set(h, 0);
        let flags = h.rebase(FLAGS);
        h.write_bytes(flags, &[0; 0x10]);
        let fd = cdecl(h, SCE_OPEN, &[path, 0x602]);
        h.write(out + 8, fd);
        h.write(out + 4, 1_u32);
        h.write(out + 0x10, 0x1000_u32);
        let buf = import(h, MALLOC_SLOT, &[0x1000]);
        h.write(out, buf);
        h.write(out + 0xc, 0_u32);
        if fd.cast_signed() < 0 {
            for (counter, text) in [(ERRORS, ERRORS_TEXT), (WARNINGS, WARNINGS_TEXT)] {
                let n = counter.get(h);
                if n != 0 {
                    let text = h.rebase(text);
                    cdecl(h, DBG_PRINTF, &[text, n]);
                }
            }
            let text = h.rebase(OPEN_FAILED_TEXT);
            cdecl(h, DBG_PRINTF, &[text, path]);
            LAST_ERROR.set(h, 2);
        } else {
            cdecl(h, COMPILE, &[source, out]);
            let used: u32 = h.read(out + 0xc);
            if used.cast_signed() > 0 && h.read::<u32>(out + 4) != 0 {
                let (fd, buf): (u32, u32) = (h.read(out + 8), h.read(out));
                cdecl(h, SCE_WRITE, &[fd, buf, used]);
            }
            let buf: u32 = h.read(out);
            if buf != 0 {
                import(h, FREE_SLOT, &[buf]);
            }
            let fd: u32 = h.read(out + 8);
            if fd.cast_signed() > 0 {
                cdecl(h, SCE_CLOSE, &[fd]);
            }
        }
        let root = TREE_ROOT.get(h);
        cdecl(h, FREE_TREE, &[root]);
    }

    /// `pppFpDispDebug()` (native 0x8034) -> the last `dbgPrintf`'s `eax`:
    /// the FP listing: the display switch (on/off), then per entry its
    /// index, id words (`+0x64`), run state (stopped at -0x1000, running,
    /// ended by `+0x1d`), shown or hidden (flag 0x10 of its display
    /// record), type (flags 1, 2, 4, 8, else none) and the display
    /// record's two words.
    pub fn fp_disp_debug(h: &mut impl Host) -> u32 {
        let print = |h: &mut _, args: &[u32]| cdecl(h, DBG_PRINTF, args);
        let head = h.rebase(LIST_HEAD[0]);
        print(h, &[head]);
        let on = h.rebase(if PPP_FP_DISP.get(h) != 0 {
            ON_TEXT
        } else {
            OFF_TEXT
        });
        let fmt = h.rebase(LIST_HEAD[1]);
        print(h, &[fmt, on]);
        for text in &LIST_HEAD[2..4] {
            let text = h.rebase(*text);
            print(h, &[text]);
        }
        let data = FP_DATA.get(h);
        if data != 0 {
            let n = u32::from(h.read::<u16>(data + 4));
            let (run, disp) = (h.rebase(FP_RUN), h.rebase(FP_DISP));
            let (states, types) = (h.rebase(STATE_NAMES), h.rebase(TYPE_NAMES));
            for k in 0..n {
                let r = run + 0x80 * k;
                let state = if h.read::<u32>(r) == 0xffff_f000 {
                    0
                } else {
                    1 + u32::from(h.read::<u8>(r + 0x1d) != 0)
                };
                let d = disp + 0x4c * k;
                let flags: u32 = h.read(d);
                let kind = match flags & 0xf {
                    1 => 1,
                    2 => 2,
                    4 => 3,
                    8 => 4,
                    _ => 0,
                };
                let shown = h.rebase(if flags & 0x10 != 0 {
                    OFF_TEXT
                } else {
                    SHOWN_TEXT
                });
                let e = data + 0x64 + 0x50 * k;
                let args = [
                    h.rebase(ENTRY_FORMAT),
                    k,
                    u32::from(h.read::<u16>(e)),
                    u32::from(h.read::<u16>(e + 2)),
                    h.read(states + 4 * state),
                    shown,
                    h.read(types + 4 * kind),
                    h.read(d + 4),
                    h.read(d + 8),
                    flags,
                ];
                print(h, &args);
            }
        }
        let end = h.rebase(LIST_END);
        print(h, &[end])
    }

    /// `pppFpInfPrint()` -> `op_num`'s `eax`: at (2, 2), the camera
    /// position (the inverse camera matrix's row 3), the player's
    /// position (`Ch_GetPos`, zero without a player) and the distance
    /// between them, each truncated to integers (`ftol2_sse`). `frame` is
    /// the original's `ebp - 0x178`: the inverse at `+4`, the camera at
    /// `+0x54`, the player at `+0x64`, the difference at `+0x44`, the text
    /// at `+0x74`.
    pub fn fp_inf_print(h: &mut impl Host, frame: u32) -> u32 {
        let (inverse, camera, player, diff, text) = (
            frame + 4,
            frame + 0x54,
            frame + 0x64,
            frame + 0x44,
            frame + 0x74,
        );
        let matrix = h.rebase(CAMERA_MATRIX);
        cdecl(h, INVERSE_MATRIX, &[inverse, matrix]);
        cdecl(h, COPY_VECTOR, &[camera, inverse + 0x30]);
        h.write(camera + 0xc, 0_u32);
        let chr = cdecl(h, GET_PLAYER, &[]);
        if chr != 0 {
            cdecl(h, CH_GET_POS, &[chr, player, player + 4, player + 8]);
            h.write(player + 0xc, 0_u32);
        } else {
            for k in 0..4 {
                h.write(player + 4 * k, 0_u32);
            }
        }
        cdecl(h, SUB_VECTOR, &[diff, player, camera]);
        let distance = h.call_sym_st0(VECTOR_LENGTH, Conv::Cdecl, &[diff]);
        h.write(frame, distance.to_bits());
        cdecl(h, OP_NUM_XY_SET, &[2, 2]);
        let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
        for (format, at) in [(CAMERA_FORMAT, camera), (PLAYER_FORMAT, player)] {
            let mut v = [0_u32; 3];
            for k in (0..3).rev() {
                v[k as usize] = x87::ftol(h, x87::load(h.read(at + 4 * k)));
            }
            let fmt = h.rebase(format);
            h.call(sprintf, Conv::Cdecl, &[text, fmt, v[0], v[1], v[2]]);
            cdecl(h, OP_NUM, &[text]);
        }
        let d = x87::ftol(h, x87::load(h.read(frame)));
        let fmt = h.rebase(DISTANCE_FORMAT);
        h.call(sprintf, Conv::Cdecl, &[text, fmt, d]);
        cdecl(h, OP_NUM, &[text])
    }
}
