//! M5 stage 1, round 7 (list `left7`, from `vfx.list`): the op script
//! compiler's symbol definition (`op_symbol_define_007e2770`): a symbol
//! made or redefined, its value split into tokens, its description copied.
//!
//! Callees stay calls to the originals (cdecl). The frame (the token
//! buffer and the tokenizer's position) comes in as the original's `ebp`.

/// The originals over game memory.
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// `op_symbol_find_007e11a0(name, tree)`, `ms_malloc_007cf970(n)`,
    /// `op_strdup_007e2e30(s)`, `ms_free(p)`, `op_strlen(s)`,
    /// `op_token_expr_007e2140(src, out, &pos, max, eol)`, the CRT's
    /// `memcpy` thunk, `dbgPrintf(fmt, ...)`.
    pub const SYMBOL_FIND: u32 = 0x007e_11a0;
    pub const MS_MALLOC: u32 = 0x007c_f970;
    pub const OP_STRDUP: u32 = 0x007e_2e30;
    pub const MS_FREE: u32 = 0x007c_f8f0;
    pub const OP_STRLEN: u32 = 0x007e_2250;
    pub const TOKEN_EXPR: u32 = 0x007e_2140;
    pub const MEMCPY: u32 = 0x0094_9256;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// `g_OpScriptTree`, the symbol list's tail (`0x0113fc88`: where the
    /// next symbol's pointer goes), `g_OpScriptErrors`,
    /// `g_OpScriptWarnings`, `g_OpLastError`; the redefinition report's
    /// formats ("\"%s\" : ", "(%d) : ", the redefinition message).
    pub const SCRIPT_TREE: u32 = 0x0113_fc84;
    pub const SYMBOL_TAIL: u32 = 0x0113_fc88;
    pub const SCRIPT_ERRORS: u32 = 0x0113_fc58;
    pub const SCRIPT_WARNINGS: u32 = 0x0113_fc5c;
    pub const LAST_ERROR: u32 = 0x0113_e238;
    pub const FORMAT_FILE: u32 = 0x00b5_7e14;
    pub const FORMAT_LINE: u32 = 0x00b5_7e1c;
    pub const FORMAT_REDEFINED: u32 = 0x00b5_7eb0;
    /// `op_symbol_define_007e2770`'s frame.
    pub const DEFINE_FRAME: u32 = 0x210;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn g(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    /// The symbol's two hashes of the string at `s`: the sum of each
    /// (signed) character xor `0x6f k`, as written and with `A..Z`
    /// lowered.
    fn hashes(h: &impl Host, s: u32) -> (u32, u32) {
        let (mut plain, mut lower) = (0_u32, 0_u32);
        let mut k = 0_u32;
        let mut salt = 0_u32;
        loop {
            let c: u8 = h.read(s.wrapping_add(k));
            if c == 0 {
                break;
            }
            let l = if c.wrapping_sub(0x41) <= 0x19 {
                c + 0x20
            } else {
                c
            };
            plain = plain.wrapping_add(i32::from(c.cast_signed()).cast_unsigned() ^ salt);
            lower = lower.wrapping_add(i32::from(l.cast_signed()).cast_unsigned() ^ salt);
            salt = salt.wrapping_add(0x6f);
            k = k.wrapping_add(1);
        }
        (plain, lower)
    }

    /// `strlen(s)` (the byte loop).
    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0_u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `op_symbol_define_007e2770(name, value, desc, kind)` -> the symbol:
    /// found in `g_OpScriptTree` it is redefined (the file and line, when
    /// set, and the redefinition reported; `g_OpLastError` = 6; its old
    /// description freed), else made (0x24 bytes: `+0` the name's copy,
    /// `+8` its token list, `+0xc` the list's tail, `+0x10` u16 1, `+0x12`
    /// the kind, `+0x14` u16 0, `+0x18`/`+0x1c` the hashes, `+0x20` the
    /// next symbol) and appended to the symbol list. The value's tokens
    /// (`op_token_expr_007e2140` into a 0x200-byte buffer) each become a
    /// 0x24-byte entry (`+0` the token's copy, `+0x10` 1, the hashes)
    /// appended to the symbol's token list. `+4` gets a copy of `desc`
    /// (or 0), `+0x16` its length. `ebp` is the original's frame (the
    /// token at `ebp - 0x204`, the position at `- 0x210`, the symbol at
    /// `- 0x208`, the value at `- 0x20c`).
    #[allow(clippy::too_many_lines)]
    pub fn symbol_define<H: Host>(
        h: &mut H,
        ebp: u32,
        name: u32,
        value: u32,
        desc: u32,
        kind: u32,
    ) -> u32 {
        let fr = |o: u32| ebp.wrapping_sub(o);
        let (buf, pos) = (fr(0x204), fr(0x210));
        h.write(fr(0x20c), value);
        let tree = g(h, SCRIPT_TREE);
        let mut sym = cdecl(h, SYMBOL_FIND, &[name, tree]);
        h.write(fr(0x208), sym);
        if sym == 0 {
            sym = cdecl(h, MS_MALLOC, &[0x24]);
            h.write(fr(0x208), sym);
            let copy = cdecl(h, OP_STRDUP, &[name]);
            h.write(sym, copy);
            h.write(at(sym, 0xc), at(sym, 8));
            h.write(at(sym, 8), 0_u32);
            h.write(at(sym, 0x10), 1_u16);
            h.write(at(sym, 0x14), 0_u16);
            h.write(at(sym, 0x12), kind as u16);
            let (plain, lower) = hashes(h, name);
            h.write(at(sym, 0x18), plain);
            let tail = g(h, SYMBOL_TAIL);
            h.write(at(sym, 0x1c), lower);
            h.write(at(sym, 0x20), 0_u32);
            h.write(tail, sym);
            h.write(h.rebase(SYMBOL_TAIL), at(sym, 0x20));
        } else {
            let file = g(h, SCRIPT_ERRORS);
            if file != 0 {
                let f = h.rebase(FORMAT_FILE);
                cdecl(h, DBG_PRINTF, &[f, file]);
            }
            let line = g(h, SCRIPT_WARNINGS);
            if line != 0 {
                let f = h.rebase(FORMAT_LINE);
                cdecl(h, DBG_PRINTF, &[f, line]);
            }
            let f = h.rebase(FORMAT_REDEFINED);
            cdecl(h, DBG_PRINTF, &[f, name]);
            h.write(h.rebase(LAST_ERROR), 6_u32);
            let old: u32 = h.read(at(sym, 4));
            if old != 0 {
                cdecl(h, MS_FREE, &[old]);
            }
        }
        if h.read::<u8>(value) != 0 {
            h.write(pos, 0_u32);
            let next = |h: &mut H| {
                let len = cdecl(h, OP_STRLEN, &[value]);
                cdecl(h, TOKEN_EXPR, &[value, buf, pos, 0x200, len]) != 0
            };
            while next(h) {
                let item = cdecl(h, MS_MALLOC, &[0x24]);
                let n = strlen(h, buf).wrapping_add(1);
                let copy = cdecl(h, MS_MALLOC, &[n]);
                if n.cast_signed() > 0 {
                    cdecl(h, MEMCPY, &[copy, buf, n]);
                }
                h.write(at(item, 0x14), 0_u16);
                h.write(item, copy);
                h.write(at(item, 8), 0_u32);
                h.write(at(item, 4), 0_u32);
                h.write(at(item, 0x10), 1_u32);
                let (plain, lower) = hashes(h, buf);
                h.write(at(item, 0x18), plain);
                h.write(at(item, 0x1c), lower);
                h.write(at(item, 0x20), 0_u32);
                let tail: u32 = h.read(at(sym, 0xc));
                h.write(tail, item);
                h.write(at(sym, 0xc), at(item, 0x20));
            }
        }
        let copy = if desc == 0 {
            0
        } else {
            let n = strlen(h, desc).wrapping_add(1);
            let copy = cdecl(h, MS_MALLOC, &[n]);
            for k in 0..n {
                let c: u8 = h.read(desc.wrapping_add(k));
                h.write(copy.wrapping_add(k), c);
            }
            copy
        };
        h.write(at(sym, 4), copy);
        let len = if desc == 0 { 0 } else { strlen(h, desc) };
        h.write(at(sym, 0x16), len as u16);
        sym
    }

    /// `op_token_get_007e1d50(file, src, out, &pos, max, &eol, flags)`,
    /// `op_token_007e1f80(src, out, &pos, max, eol)`,
    /// `op_list_values_007e2720(node)`, this function itself (the
    /// recursion), the expansion depth (`0x0113fc7c`), "(" and the two
    /// argument-count messages (too many, too few).
    pub const TOKEN_GET: u32 = 0x007e_1d50;
    pub const TOKEN: u32 = 0x007e_1f80;
    pub const LIST_VALUES: u32 = 0x007e_2720;
    pub const MACRO_EXPAND: u32 = 0x007e_2a60;
    pub const EXPAND_DEPTH: u32 = 0x0113_fc7c;
    pub const S_OPEN: u32 = 0x00b5_7fb8;
    pub const S_TOO_MANY: u32 = 0x00b5_7f88;
    pub const S_TOO_FEW: u32 = 0x00b5_7f64;
    /// `op_macro_expand_007e2a60`'s frame.
    pub const EXPAND_FRAME: u32 = 0x21c;

    /// The file and line, when set, then `msg` with `src`, and
    /// `g_OpLastError = code` (the compiler's error report).
    fn report(h: &mut impl Host, msg: u32, src: u32, code: u32) {
        let file = g(h, SCRIPT_ERRORS);
        if file != 0 {
            let f = h.rebase(FORMAT_FILE);
            cdecl(h, DBG_PRINTF, &[f, file]);
        }
        let line = g(h, SCRIPT_WARNINGS);
        if line != 0 {
            let f = h.rebase(FORMAT_LINE);
            cdecl(h, DBG_PRINTF, &[f, line]);
        }
        let m = h.rebase(msg);
        cdecl(h, DBG_PRINTF, &[m, src]);
        h.write(h.rebase(LAST_ERROR), code);
    }

    /// `op_macro_expand_007e2a60(tree, src, name, &pos, end, eol)` -> 0,
    /// or -1 when `name` is no macro in `tree` (or has no body) or its
    /// arguments don't open with "(": a macro's expansion into the output
    /// at `name` (the macro's name is the output's start). With
    /// parameters (the symbol's token list) the arguments are read from
    /// `src` at `*pos` (`op_token_get_007e1d50`, which may move the line
    /// end in the `eol` argument slot, `slots + 0x14`; then
    /// `op_token_expr_007e2140` per argument), each copied to its
    /// parameter (`+4`, its length `+0x16`); too many or too few are
    /// reported (`g_OpLastError` 0xc, 0xb), `*pos` updated. Then the body
    /// (`+4`, its length `+0x16`) is tokenized into the output
    /// (`op_token_007e1f80`), each identifier token (class 2) expanded in
    /// turn, up to three deep (`0x0113fc7c`), against the script's
    /// symbols and then the macro's own parameters (their values saved
    /// and put back around it), the output advancing by its length. The
    /// parameters' values are freed at the end. `ebp` is the original's
    /// frame (the token at `ebp - 0x204`, the positions at `- 0x214`, `-
    /// 0x20c`, the spills at `- 0x21c..`).
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    pub fn macro_expand<H: Host>(
        h: &mut H,
        ebp: u32,
        slots: u32,
        tree: u32,
        src: u32,
        name: u32,
        pos: u32,
        end: u32,
    ) -> u32 {
        let fr = |o: u32| ebp.wrapping_sub(o);
        let (buf, pos1, pos2) = (fr(0x204), fr(0x214), fr(0x20c));
        let eol = slots.wrapping_add(0x14);
        h.write(fr(0x21c), name);
        h.write(fr(0x208), pos);
        let sym = cdecl(h, SYMBOL_FIND, &[name, tree]);
        h.write(fr(0x210), sym);
        if sym == 0 || h.read::<u32>(at(sym, 4)) == 0 {
            return u32::MAX;
        }
        let mut param: u32 = h.read(at(sym, 8));
        if param != 0 {
            let p: u32 = h.read(pos);
            h.write(pos1, p);
            cdecl(h, TOKEN_GET, &[0, src, buf, pos1, 0x200, eol, 5]);
            let open = h.rebase(S_OPEN);
            let mut k = 0_u32;
            loop {
                let c: u8 = h.read(buf.wrapping_add(k));
                if c != h.read::<u8>(open.wrapping_add(k)) {
                    return u32::MAX;
                }
                k += 1;
                if c == 0 {
                    break;
                }
            }
            loop {
                let e: u32 = h.read(eol);
                let more = cdecl(h, TOKEN_EXPR, &[src, buf, pos1, 0x200, e]);
                if param == 0 {
                    report(h, S_TOO_MANY, src, 0xc);
                } else {
                    let copy = cdecl(h, OP_STRDUP, &[buf]);
                    h.write(at(param, 4), copy);
                    let len = if copy == 0 { 0 } else { strlen(h, copy) };
                    h.write(at(param, 0x16), len as u16);
                    param = h.read(at(param, 0x20));
                }
                if more == 0 {
                    break;
                }
            }
            if param != 0 {
                report(h, S_TOO_FEW, src, 0xb);
            }
            let p: u32 = h.read(pos1);
            let pos: u32 = h.read(fr(0x208));
            h.write(pos, p);
            param = h.read(at(sym, 8));
        }
        let len = i32::from(h.read::<i16>(at(sym, 0x16))).cast_unsigned();
        h.write(fr(0x208), len);
        h.write(pos2, 0_u32);
        let mut done = 0_u32;
        loop {
            let room = end.wrapping_sub(done);
            let dst = done.wrapping_add(name);
            h.write(fr(0x218), dst);
            let body: u32 = h.read(at(sym, 4));
            let class = cdecl(h, TOKEN, &[body, dst, pos2, room, len]);
            let depth = g(h, EXPAND_DEPTH);
            if class == 2 && depth.cast_signed() < 3 {
                h.write(h.rebase(EXPAND_DEPTH), depth.wrapping_add(1));
                let saved = cdecl(h, LIST_VALUES, &[param]);
                let top = g(h, SCRIPT_TREE);
                let body: u32 = h.read(at(sym, 4));
                let r = cdecl(h, MACRO_EXPAND, &[top, body, dst, pos2, room, len]);
                if r != 0 && param != 0 {
                    let body: u32 = h.read(at(sym, 4));
                    cdecl(h, MACRO_EXPAND, &[param, body, dst, pos2, room, len]);
                }
                if saved != 0 {
                    let (mut p, mut v) = (param, saved);
                    while p != 0 {
                        let w: u32 = h.read(v);
                        h.write(at(p, 4), w);
                        p = h.read(at(p, 0x20));
                        v = v.wrapping_add(4);
                    }
                    cdecl(h, MS_FREE, &[saved]);
                }
                let depth = g(h, EXPAND_DEPTH).wrapping_sub(1);
                h.write(h.rebase(EXPAND_DEPTH), depth);
            }
            done = cdecl(h, OP_STRLEN, &[name]);
            if h.read::<u32>(pos2).cast_signed() >= len.cast_signed() {
                break;
            }
        }
        let mut p = param;
        while p != 0 {
            let v: u32 = h.read(at(p, 4));
            if v != 0 {
                cdecl(h, MS_FREE, &[v]);
                h.write(at(p, 4), 0_u32);
            }
            p = h.read(at(p, 0x20));
        }
        0
    }

    /// `op_file_read(&file, path)`, `op_read_line_007e1cc0(&file, out,
    /// max)`, `op_file_puts_007e2fb0(out, s)`, `op_line_length_007e2660(&file,
    /// &line)`, the directive handler (`0x007e15f0`), `sceWrite(fd, buf, n)`,
    /// `sceClose(fd)`; the MSVCR110 `free` slot.
    pub const FILE_READ: u32 = 0x007e_2ed0;
    pub const READ_LINE: u32 = 0x007e_1cc0;
    pub const FILE_PUTS: u32 = 0x007e_2fb0;
    pub const LINE_LENGTH: u32 = 0x007e_2660;
    pub const DIRECTIVE: u32 = 0x007e_15f0;
    pub const SCE_WRITE: u32 = 0x0062_fdc0;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    pub const FREE_SLOT: u32 = 0x00b0_c370;
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    /// The script's line (0x0113fc5c, `g_OpScriptWarnings` above), the
    /// comment and string states (0x0113fc60, 0x0113fc64), the line
    /// marker's buffer (0x0113f958), the conditional stack's flags
    /// (0x0113fc68 + the level at 0x0113fc78: nonzero while skipping),
    /// the lines a directive took (0x0113fc80); the directive keywords
    /// ("#include" and on), "# %d \"%s\"%s", "\r\n", the not-found message.
    pub const IN_COMMENT: u32 = 0x0113_fc60;
    pub const IN_STRING: u32 = 0x0113_fc64;
    pub const LINE_MARK: u32 = 0x0113_f958;
    pub const SKIP_FLAGS: u32 = 0x0113_fc68;
    pub const SKIP_LEVEL: u32 = 0x0113_fc78;
    pub const LINE_COUNT: u32 = 0x0113_fc80;
    pub const DIRECTIVES: u32 = 0x00c4_4a50;
    pub const FORMAT_MARK: u32 = 0x00b5_7fac;
    pub const S_CRLF: u32 = 0x00c4_4a48;
    pub const S_NOT_FOUND: u32 = 0x00b5_7e44;
    /// `op_preprocess_007e2270`'s frame.
    pub const PRE_FRAME: u32 = 0x424;

    /// Skipping (the conditional stack's current flag).
    fn skipping(h: &impl Host) -> bool {
        let level = g(h, SKIP_LEVEL);
        h.read::<u8>(h.rebase(SKIP_FLAGS).wrapping_add(level)) != 0
    }

    /// The line marker `# line "file"\r\n` into 0x0113f958, then out.
    fn line_mark(h: &mut impl Host, out: u32, line: u32, file: u32) -> u32 {
        let (m, f, crlf) = (h.rebase(LINE_MARK), h.rebase(FORMAT_MARK), h.rebase(S_CRLF));
        let sprintf = g(h, SPRINTF_SLOT);
        h.call(sprintf, Conv::Cdecl, &[m, f, line, file, crlf]);
        cdecl(h, FILE_PUTS, &[out, m])
    }

    /// `op_preprocess_007e2270(path, out)` -> `eax` as the original: the op
    /// script preprocessor over one file: read whole (`op_file_read` into
    /// the frame's file at `ebp - 0x420`: buffer, a dirty flag, handle,
    /// length); missing it is reported (`g_OpLastError` 3). Else, with the
    /// script's file and line set to it (line markers written before and,
    /// for the including file, after), line by line: a leading class-1
    /// token copied (unless skipping), a directive word (`#include` and
    /// the rest, from 0x00c44a50) handed to `0x007e15f0` (skipping: only
    /// the ones that run then), then unless skipping every token written
    /// out, identifiers through `op_macro_expand_007e2a60`; the lines a
    /// directive took replaced by line ends. A dirty buffer is written
    /// back, freed and the file closed. `ebp` is the original's frame (the
    /// line at `ebp - 0x404`, the token at `- 0x204`, the position at `-
    /// 0x408`, the line's length at `- 0x40c`, the outer line at `-
    /// 0x424`).
    #[allow(clippy::too_many_lines)]
    pub fn preprocess(h: &mut impl Host, ebp: u32, path: u32, out: u32) -> u32 {
        let fr = |o: u32| ebp.wrapping_sub(o);
        let (file, line, tok, pos, len) = (fr(0x420), fr(0x404), fr(0x204), fr(0x408), fr(0x40c));
        cdecl(h, FILE_READ, &[file, path]);
        if h.read::<u32>(fr(0x418)).cast_signed() < 0 {
            let f = g(h, SCRIPT_ERRORS);
            if f != 0 {
                let m = h.rebase(FORMAT_FILE);
                cdecl(h, DBG_PRINTF, &[m, f]);
            }
            let l = g(h, SCRIPT_WARNINGS);
            if l != 0 {
                let m = h.rebase(FORMAT_LINE);
                cdecl(h, DBG_PRINTF, &[m, l]);
            }
            let m = h.rebase(S_NOT_FOUND);
            let r = cdecl(h, DBG_PRINTF, &[m, path]);
            h.write(h.rebase(LAST_ERROR), 3_u32);
            return r;
        }
        let (old_line, old_file) = (g(h, SCRIPT_WARNINGS), g(h, SCRIPT_ERRORS));
        h.write(fr(0x424), old_line);
        h.write(h.rebase(SCRIPT_ERRORS), path);
        h.write(h.rebase(SCRIPT_WARNINGS), 1_u32);
        h.write(h.rebase(IN_COMMENT), 0_u32);
        h.write(h.rebase(IN_STRING), 0_u32);
        line_mark(h, out, 1, path);
        loop {
            h.write(h.rebase(LINE_COUNT), 0_u32);
            let n = cdecl(h, READ_LINE, &[file, line, 0x200]);
            h.write(len, n);
            h.write(pos, 0_u32);
            let mut class = cdecl(h, TOKEN, &[line, tok, pos, 0x200, n]);
            if class == 1 {
                if !skipping(h) {
                    cdecl(h, FILE_PUTS, &[out, tok]);
                }
                let n: u32 = h.read(len);
                class = cdecl(h, TOKEN, &[line, tok, pos, 0x200, n]);
            }
            if class == 2 {
                let dirs = h.rebase(DIRECTIVES);
                let d = cdecl(h, SYMBOL_FIND, &[tok, dirs]);
                if d != 0 {
                    if !skipping(h) || h.read::<u16>(at(d, 0x14)) == 0 {
                        let kind = i32::from(h.read::<i16>(at(d, 0x10))).cast_unsigned();
                        cdecl(h, DIRECTIVE, &[file, out, line, kind, pos, 0x200, len]);
                    }
                    h.write(tok, 0_u8);
                }
            }
            if !skipping(h) {
                loop {
                    if class == 2 {
                        h.write(h.rebase(EXPAND_DEPTH), 0_u32);
                        let (top, n): (u32, u32) = (g(h, SCRIPT_TREE), h.read(len));
                        cdecl(h, MACRO_EXPAND, &[top, line, tok, pos, 0x200, n]);
                    }
                    cdecl(h, FILE_PUTS, &[out, tok]);
                    let n: u32 = h.read(len);
                    if h.read::<u32>(pos).cast_signed() >= n.cast_signed() {
                        break;
                    }
                    class = cdecl(h, TOKEN, &[line, tok, pos, 0x200, n]);
                }
            }
            let lines = h.rebase(SCRIPT_WARNINGS);
            if cdecl(h, LINE_LENGTH, &[file, lines]).cast_signed() < 0 {
                break;
            }
            let mut k = 0_i32;
            while k < g(h, LINE_COUNT).cast_signed() {
                let crlf = h.rebase(S_CRLF);
                cdecl(h, FILE_PUTS, &[out, crlf]);
                k += 1;
            }
        }
        let size: u32 = h.read(fr(0x414));
        if size.cast_signed() > 0 && h.read::<u32>(fr(0x41c)) != 0 {
            let (buf, fd): (u32, u32) = (h.read(file), h.read(fr(0x418)));
            cdecl(h, SCE_WRITE, &[fd, buf, size]);
        }
        let buf: u32 = h.read(file);
        if buf != 0 {
            let free = g(h, FREE_SLOT);
            h.call(free, Conv::Cdecl, &[buf]);
        }
        let fd: u32 = h.read(fr(0x418));
        if fd.cast_signed() > 0 {
            cdecl(h, SCE_CLOSE, &[fd]);
        }
        let old_line: u32 = h.read(fr(0x424));
        h.write(h.rebase(SCRIPT_ERRORS), old_file);
        h.write(h.rebase(SCRIPT_WARNINGS), old_line);
        let r = if old_file == 0 {
            old_line
        } else {
            line_mark(h, out, old_line, old_file)
        };
        h.write(h.rebase(IN_COMMENT), 0_u32);
        h.write(h.rebase(IN_STRING), 0_u32);
        r
    }
}
