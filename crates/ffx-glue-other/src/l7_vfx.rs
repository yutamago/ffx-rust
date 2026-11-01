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
}
