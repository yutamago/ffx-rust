//! Battle-window state of the `TO` layer (`TOBtl*`): help and sign windows,
//! the cursor position stack, Lulu's limit table, target callbacks.

/// One entry of `TOBtlSetLuluLimitTbl`'s running sums of ASCII digits:
/// the previous entry (0 for the first) plus the digit's value (bytes,
/// wrapping).
#[must_use]
pub const fn lulu_limit_step(prev: u8, digit: u8) -> u8 {
    prev.wrapping_sub(0x30).wrapping_add(digit)
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The battle help window is open (`TOBtlCloseHelpWin` clears it).
    pub const HELP_WIN: Global<u8> = Global::new(0x0133_c93c);
    /// Set by `TOBtlInitAnm`.
    pub const INIT_ANM: Global<u8> = Global::new(0x0134_0452);
    /// The sign window (`TOBtlCloseSignWindow` clears it).
    pub const SIGN_WINDOW: Global<u32> = Global::new(0x0133_f520);
    /// The simple help message (`TOBtlCloseSimpleHelpMes` clears it).
    pub const SIMPLE_HELP: Global<u32> = Global::new(0x0133_f510);
    /// `TOGetBtlWindowControlEnable`'s flag.
    pub const WINDOW_CONTROL: Global<u32> = Global::new(0x00c5_8ef8);
    /// Cursor positions pushed (`i16`, at most 4).
    pub const CUR_POS_COUNT: Global<i16> = Global::new(0x0133_d6f0);
    /// The cursor positions: 8-byte records, x at `+0`, y at `+4`, z at `+6`.
    pub const CUR_POS: Global<[u16; 4]> = Global::new(0x0133_f7b4);
    /// Lulu's limit table (16 bytes).
    pub const LULU_LIMIT: Global<[u8; 16]> = Global::new(0x0133_f798);

    /// `TOBtlCloseHelpWin()` (`0x00890eb0`).
    pub fn close_help_win(mem: &mut impl Mem) {
        HELP_WIN.set(mem, 0);
    }

    /// `TOBtlInitAnm()` (`0x008a0cd0`).
    pub fn init_anm(mem: &mut impl Mem) {
        INIT_ANM.set(mem, 1);
    }

    /// `TOBtlCloseSignWindow()` (`0x00890ee0`).
    pub fn close_sign_window(mem: &mut impl Mem) {
        SIGN_WINDOW.set(mem, 0);
    }

    /// `TOBtlCloseSimpleHelpMes()` (`0x00890ef0`).
    pub fn close_simple_help(mem: &mut impl Mem) {
        SIMPLE_HELP.set(mem, 0);
    }

    /// `TOGetBtlWindowControlEnable()` (`0x0089b590`).
    pub fn window_control_enable(mem: &impl Mem) -> u32 {
        WINDOW_CONTROL.get(mem)
    }

    /// `TOBtlSetCurPos(x, y, z)` (`0x008999f0`): pushes a cursor position
    /// while fewer than 4 are pushed (the count is signed).
    pub fn set_cur_pos(mem: &mut impl Mem, x: u16, y: u16, z: u16) {
        let count = CUR_POS_COUNT.get(mem);
        if count >= 4 {
            return;
        }
        let r = CUR_POS
            .ptr(mem)
            .addr
            .wrapping_add(i32::from(count).cast_unsigned().wrapping_mul(8));
        mem.write(r, x);
        mem.write(r.wrapping_add(4), y);
        mem.write(r.wrapping_add(6), z);
        CUR_POS_COUNT.set(mem, count.wrapping_add(1));
    }

    /// `TOBtlSetLuluLimitTbl(digits)` (`0x0089a2f0`): 16 entries, each
    /// read back from the table and the digits as the original goes.
    pub fn set_lulu_limit_table(mem: &mut impl Mem, digits: u32) {
        let t = LULU_LIMIT.ptr(mem).addr;
        for i in 0..16 {
            let prev = if i == 0 { 0 } else { mem.read::<u8>(t + i - 1) };
            let d = mem.read::<u8>(digits.wrapping_add(i));
            mem.write(t + i, super::lulu_limit_step(prev, d));
        }
    }

    /// `TOBtlSetTargetCallback(w)` (`0x008a1090`): the window's target
    /// handlers (`+0x90`, `+0x98`, `+0x9c`, `+0xa0`; `+0x94` cleared), code
    /// addresses of this image.
    pub fn set_target_callback(mem: &mut impl Mem, w: u32) {
        let at = |o: u32| w.wrapping_add(o);
        mem.write(at(0x94), 0u32);
        for (o, code) in [
            (0x98, 0x008a_8780),
            (0x9c, 0x008a_8be0),
            (0xa0, 0x008a_2ac0),
            (0x90, 0x008a_84c0),
        ] {
            let c = mem.rebase(code);
            mem.write(at(o), c);
        }
    }

    /// What `TOCloseBtlWindow` clears: a word, then nine words.
    pub const BTL_WINDOW_OPEN: Global<u32> = Global::new(0x023c_c08c);
    /// The nine words after it.
    pub const BTL_WINDOW_STATE: Global<[u32; 9]> = Global::new(0x0133_d74c);

    /// `TOCloseBtlWindow()` (`0x0089ade0`): returns 0 (left in `eax`).
    pub fn close_btl_window(mem: &mut impl Mem) -> u32 {
        BTL_WINDOW_OPEN.set(mem, 0);
        BTL_WINDOW_STATE.set(mem, [0; 9]);
        0
    }

    /// The battle command-window stack (`0x0133c900`): 0xf0 bytes per
    /// level, indexed by [`ACTION_DEPTH`].
    pub const ACTION_STACK: Global<[u8; 0xf0]> = Global::new(0x0133_c900);
    /// The current level (`0x023cc092`, i8).
    pub const ACTION_DEPTH: Global<i8> = Global::new(0x023c_c092);
    /// Stored at `+0x58` of the level left (`0x023cc088`, u16).
    pub const ACTION_PREV_VALUE: Global<u16> = Global::new(0x023c_c088);
    /// Cleared when the level left shows command 0x311e or 0x3029
    /// (`0x0133c94c`).
    pub const ACTION_MASK: Global<u32> = Global::new(0x0133_c94c);
    /// Restored from [`ACTION_SAVED`] when going back to a level whose
    /// `+0x52` is 1 (`0x0133d0d0`, 0x140 bytes).
    pub const ACTION_LIST: Global<[u8; 0x140]> = Global::new(0x0133_d0d0);
    /// See [`ACTION_LIST`] (`0x0133d210`).
    pub const ACTION_SAVED: Global<[u8; 0x140]> = Global::new(0x0133_d210);
    /// Zeroed with that restore (`0x0133f106`, u16).
    pub const ACTION_LIST_CURSOR: Global<u16> = Global::new(0x0133_f106);

    /// `TOBtlSetPrevAction()` (`0x0089a630`): back one level of the command
    /// windows. The level left: `+0x51` 5, `+0x58` [`ACTION_PREV_VALUE`],
    /// `+0x66` 0xff, [`ACTION_MASK`] cleared for commands 0x311e and 0x3029
    /// (`+0x5c`), and when its `+0x52` is 1 the next level's `+6` takes its
    /// `+6`. The level returned to: `+0x51` 2, its cursor `+0x5b` back one
    /// when at or past `+0x5a` and positive, and when its `+0x52` is 1 the
    /// list restored. 0 at level 0 (or below), else 1.
    pub fn set_prev_action(mem: &mut impl Mem) -> u32 {
        let depth = ACTION_DEPTH.get(mem);
        if depth <= 0 {
            return 0;
        }
        let base = ACTION_STACK.ptr(mem).addr;
        let level = |d: i8| base.wrapping_add(i32::from(d).cast_unsigned().wrapping_mul(0xf0));
        let e = level(depth);
        mem.write(e + 0x51, 5u8);
        let prev = ACTION_PREV_VALUE.get(mem);
        mem.write(e + 0x58, prev);
        mem.write(e + 0x66, 0xffu16);
        let command: u16 = mem.read(e + 0x5c);
        if command == 0x311e || command == 0x3029 {
            ACTION_MASK.set(mem, 0);
        }
        if mem.read::<u8>(e + 0x52) == 1 {
            let v: u16 = mem.read(e + 6);
            mem.write(e + 0xf6, v);
        }
        let depth = depth - 1;
        ACTION_DEPTH.set(mem, depth);
        let p = level(depth);
        mem.write(p + 0x51, 2u8);
        let cursor: i8 = mem.read(p + 0x5b);
        if cursor >= mem.read::<i8>(p + 0x5a) && cursor > 0 {
            mem.write(p + 0x5b, cursor - 1);
        }
        if mem.read::<u8>(p + 0x52) == 1 {
            let saved: [u8; 0x140] = mem.read(ACTION_SAVED.ptr(mem).addr);
            let list = ACTION_LIST.ptr(mem).addr;
            mem.write(list, saved);
            ACTION_LIST_CURSOR.set(mem, 0);
        }
        1
    }

    /// The damage values shown (`0x0133f108`): an `i16` count, then from
    /// `0x0133f110` up to 63 records of 16 bytes.
    pub const DAMAGE_COUNT: Global<i16> = Global::new(0x0133_f108);
    /// The first damage record.
    pub const DAMAGE_VALUES: Global<[u8; 16]> = Global::new(0x0133_f110);

    /// `TOBtlSetDamageValue(a, b, c, d, e, f, g, h)` (`0x00899a40`): a
    /// record (`u16`, `u16`, `u32`, `u32`, four bytes) appended; the new
    /// count, or -1 with 63 already.
    pub fn set_damage_value(mem: &mut impl Mem, a: [u32; 8]) -> i32 {
        add_damage_record(mem, a, true)
    }

    /// `0x0089baa0(a, b, d, e, f, g, h)`: [`set_damage_value`] without the
    /// word at `+4` (left as it was).
    pub fn set_damage_value_short(mem: &mut impl Mem, a: [u32; 7]) -> i32 {
        add_damage_record(mem, [a[0], a[1], 0, a[2], a[3], a[4], a[5], a[6]], false)
    }

    fn add_damage_record(mem: &mut impl Mem, a: [u32; 8], word4: bool) -> i32 {
        let n = DAMAGE_COUNT.get(mem);
        if n >= 0x3f {
            return -1;
        }
        let r = DAMAGE_VALUES
            .ptr(mem)
            .addr
            .wrapping_add(i32::from(n).cast_unsigned() << 4);
        #[allow(clippy::cast_possible_truncation)] // the low halves
        {
            mem.write(r, a[0] as u16);
            mem.write(r.wrapping_add(2), a[1] as u16);
        }
        if word4 {
            mem.write(r.wrapping_add(4), a[2]);
        }
        mem.write(r.wrapping_add(8), a[3]);
        for (k, w) in (0u32..).zip(&a[4..]) {
            mem.write(r.wrapping_add(0xc + k), w.to_le_bytes()[0]);
        }
        let n = n.wrapping_add(1);
        DAMAGE_COUNT.set(mem, n);
        i32::from(n)
    }

    /// The two target lists (`0x0133f0d8`, `0x0133f0ec`): a count, then
    /// two-byte entries whose first byte is a target bit.
    pub const TARGET_LISTS: [u32; 2] = [0x0133_f0d8, 0x0133_f0ec];
    /// The masks they are OR-ed into (`0x0133d0d4`, `0x0133d0d8`) and the
    /// union (`0x0133d0dc`).
    pub const TARGET_MASKS: Global<[u32; 3]> = Global::new(0x0133_d0d4);

    /// `TOBtlMakeTargetListBit()` (`0x008985b0`): each list's target bits
    /// OR-ed into its mask (stored only for a non-empty list), the union of
    /// both into the third; returns the sum of the counts.
    pub fn make_target_list_bit(mem: &mut impl Mem) -> i32 {
        let masks = TARGET_MASKS.ptr(mem).addr;
        let mut out = [0u32; 2];
        let mut total = 0i32;
        for (k, &list) in TARGET_LISTS.iter().enumerate() {
            let list = mem.rebase(list);
            let n: i32 = mem.read(list);
            let at = masks.wrapping_add(4 * u32::try_from(k).unwrap_or(0));
            let mut m: u32 = mem.read(at);
            for i in 0..n.max(0).cast_unsigned() {
                let bit: u8 = mem.read(list.wrapping_add(4 + 2 * i));
                m |= 1u32.wrapping_shl(u32::from(bit));
            }
            if n > 0 {
                mem.write(at, m);
            }
            out[k] = m;
            total = total.wrapping_add(n);
        }
        mem.write(masks.wrapping_add(8), out[0] | out[1]);
        total
    }

    /// An integer as `fild` and `fstp dword` leave it.
    #[allow(clippy::cast_precision_loss)]
    fn int_f32(n: i32) -> f32 {
        n as f32
    }

    /// `TOBtlInitChrNum()` (`0x008979d0`) and `TOBtlCtrlChrNum()`
    /// (`0x00891140`, the same code): the battle menu's two character
    /// lists from `MsGetSaveWindow(0, 9 / 0xb, &count)` (`count` in the
    /// word `count_cell`, zeroed first): trailing 0xff entries dropped,
    /// the count to `0x0133f0d8` / `0x0133f0ec`, the `u16` entries copied
    /// to `0x0133f0dc` / `0x0133f0f0` in order. Returns the original's
    /// `eax`: the second list's pointer with its low half the last entry
    /// copied, if any. Callees through `call` (`ffx.sym` addresses).
    pub fn btl_chr_num<M: Mem>(
        mem: &mut M,
        count_cell: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut eax = 0;
        for (window, count_at, entries_at) in [
            (9u32, 0x0133_f0d8u32, 0x0133_f0dcu32),
            (0xb, 0x0133_f0ec, 0x0133_f0f0),
        ] {
            mem.write(count_cell, 0u32);
            let list = call(mem, 0x0078_5410, &[0, window, count_cell]);
            let mut count: i32 = mem.read(count_cell);
            while count > 0
                && mem.read::<u16>(list.wrapping_add(2 * (count - 1).cast_unsigned())) == 0xff
            {
                count -= 1;
                mem.write(count_cell, count);
            }
            mem.write(mem.rebase(count_at), count);
            eax = list;
            let entries = mem.rebase(entries_at);
            for i in 0..count.max(0).cast_unsigned() {
                let v: u16 = mem.read(list.wrapping_add(2 * i));
                mem.write(entries.wrapping_add(2 * i), v);
                eax = eax & 0xffff_0000 | u32::from(v);
            }
        }
        eax
    }

    /// `TOBtlMakeStatusBuff(chr, out)` (`0x00898260`): the status
    /// message ids of `chr` written to `out` (`u16` each): 0x1006 + bit
    /// for bits 0..=10 of `MsGetChrStatusBits_0078f930(chr)`, 0x1011 when
    /// `MsGetStatusThreat(chr)` is 1, 0x1012 + i for the 13 nonzero
    /// counter bytes of `MsMenuCountStatus(chr)`, 0x1020 + bit for bits
    /// 0..=14 of `MsMenuBitStatus(chr)`. Returns how many.
    #[allow(clippy::cast_possible_truncation)] // ids and indices fit in u16
    pub fn btl_make_status_buff<M: Mem>(
        mem: &mut M,
        chr: u32,
        out: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut n = 0u32;
        let mut push = |mem: &mut M, id: u16| {
            mem.write(out.wrapping_add(2 * n), id);
            n += 1;
        };
        let bits = call(mem, 0x0078_f930, &[chr]) & 0xffff;
        for i in 0..12u32 {
            if i == 0xb {
                if call(mem, 0x0079_afc0, &[chr]) == 1 {
                    push(mem, 0x1011);
                }
            } else if bits >> i & 1 != 0 {
                push(mem, 0x1006 + i as u16);
            }
        }
        let counts = call(mem, 0x0078_fa80, &[chr]);
        for i in 0..0xdu32 {
            if mem.read::<u8>(counts.wrapping_add(i)) != 0 {
                push(mem, 0x1012 + i as u16);
            }
        }
        let bits = call(mem, 0x0078_f950, &[chr]) & 0xffff;
        for i in 0..0xfu32 {
            if bits >> i & 1 != 0 {
                push(mem, 0x1020 + i as u16);
            }
        }
        n
    }

    /// `TOBtlSetTargetHelpMes(chr)` (`0x0089a8b0`): the battle help text
    /// (`g_BtlHelpText`, `0x0133f538`, cleared) for the target `chr`
    /// (nothing for 0xff): the character's name
    /// (`MsGetRamChrName`), for a monster (`chr` from 0x14) after a tag
    /// by `MsGetRamChrIconType(chr)` and `MsGetRamChrIconNumber(chr)` `n`:
    /// 0x14 a letter (Japanese without English: `,` and the digit `n`;
    /// languages 9/10 `n + 'Z'`, others `n + 'P'`) and menu text 0x1049;
    /// 0x15 menu texts 0x1047 and 0x1049; 0x16 0x1047, 0x1048 with its
    /// second byte raised by `n`, and 0x1049. Then measured
    /// (`TOGetMessageWH(0, text, 0x0133f534, 0x0133f530, 0, 0)`). The
    /// three menu texts (`MsMenuGetText(1, id, 0)`) are copied first, as
    /// the original does, into its three 16-byte stack buffers (0x1047,
    /// 0x1049, 0x1048 in that order in memory; a longer text runs on into
    /// the next, past the last one the original corrupts its frame: here
    /// it stops; with a 0x1048 shorter than two bytes the raised byte
    /// is its NUL and the original reads on into stale stack, here
    /// zeros). Callees through `call`.
    pub fn btl_set_target_help_mes<M: Mem>(
        mem: &mut M,
        chr: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        const MENU_GET_TEXT: u32 = 0x0078_fcf0;
        const CHR_NAME: u32 = 0x0079_af10;
        const ICON_TYPE: u32 = 0x0079_ae30;
        const ICON_NUMBER: u32 = 0x0079_ae10;
        const LANG: u32 = 0x008a_c2f0;
        const ENGLISH: u32 = 0x0078_51d0;
        const MESSAGE_WH: u32 = 0x008b_9650;
        // the stack buffers' offsets: 0x1047, 0x1049, 0x1048
        const MONSTER: usize = 0;
        const OF: usize = 0x10;
        const LETTER: usize = 0x20;
        if chr == 0xff {
            return;
        }
        let mut texts = [0u8; 0x30];
        for (id, at) in [(0x1047, MONSTER), (0x1048, LETTER), (0x1049, OF)] {
            let src = call(mem, MENU_GET_TEXT, &[1, id, 0]);
            for (k, slot) in (0u32..).zip(texts[at..].iter_mut()) {
                *slot = mem.read(src.wrapping_add(k));
                if *slot == 0 {
                    break;
                }
            }
        }
        // a text of the buffers, to its NUL
        let local = |texts: &[u8; 0x30], at: usize| -> Vec<u8> {
            let s = &texts[at..];
            let end = s.iter().position(|&c| c == 0).map_or(s.len(), |n| n + 1);
            s[..end].to_vec()
        };
        let text = mem.rebase(0x0133_f538);
        mem.write_bytes(text, &[0u8; 0x80]);
        // the end of the help text
        let end = |mem: &M| {
            let mut p = text;
            while mem.read::<u8>(p) != 0 {
                p = p.wrapping_add(1);
            }
            p
        };
        let name = call(mem, CHR_NAME, &[chr]);
        if chr.cast_signed() >= 0x14 {
            match call(mem, ICON_TYPE, &[chr]) {
                0x14 => {
                    #[allow(clippy::cast_possible_truncation)] // al
                    let n = call(mem, ICON_NUMBER, &[chr]) as u8;
                    if call(mem, LANG, &[]) == 0 && call(mem, ENGLISH, &[]) == 0 {
                        mem.write_bytes(text, &[b',', n.wrapping_add(b'0'), 0]);
                    } else {
                        let letter = if call(mem, LANG, &[]) == 10 || call(mem, LANG, &[]) == 9 {
                            b'Z'
                        } else {
                            b'P'
                        };
                        mem.write_bytes(text, &[n.wrapping_add(letter), 0]);
                    }
                    let at = end(mem);
                    mem.write_bytes(at, &local(&texts, OF));
                }
                0x15 => {
                    mem.write_bytes(text, &local(&texts, MONSTER));
                    let at = end(mem);
                    mem.write_bytes(at, &local(&texts, OF));
                }
                0x16 => {
                    #[allow(clippy::cast_possible_truncation)] // al
                    let n = call(mem, ICON_NUMBER, &[chr]) as u8;
                    texts[LETTER + 1] = texts[LETTER + 1].wrapping_add(n);
                    mem.write_bytes(text, &local(&texts, MONSTER));
                    let at = end(mem);
                    mem.write_bytes(at, &local(&texts, LETTER));
                    let at = end(mem);
                    mem.write_bytes(at, &local(&texts, OF));
                }
                _ => {}
            }
        }
        // the name (strcat over game memory)
        let mut len = 0u32;
        while mem.read::<u8>(name.wrapping_add(len)) != 0 {
            len += 1;
        }
        let mut bytes = vec![0u8; len as usize + 1];
        mem.read_bytes(name, &mut bytes);
        let at = end(mem);
        mem.write_bytes(at, &bytes);
        let (w, h) = (mem.rebase(0x0133_f534), mem.rebase(0x0133_f530));
        call(mem, MESSAGE_WH, &[0, text, w, h, 0, 0]);
    }

    /// `TOBtlSetChangeHelpMes(n)` (`0x008997c0`): for a character or aeon
    /// (`n` 0..=0x11) or command 0x30ff, the battle help text
    /// `g_BtlHelpText` (`0x0133f538`, 0x80 bytes) cleared and filled with
    /// `MsGetMenuChrInfo(n, 0)` or, for 0x30ff, the text at the offset
    /// `MsGetComData(0x30ff, &base)` gives (`u16 +8`) from `base` (the
    /// word `out_cell`); copied up to its NUL (unbounded, as the
    /// original). Then `TOGetMessageWH(0, text, 0x0133f534, 0x0133f530, 0,
    /// 0)`, whose `eax` it returns. Other numbers do nothing (the
    /// original's `eax` is then its caller's; the port returns 0).
    pub fn btl_set_change_help_mes<M: Mem>(
        mem: &mut M,
        n: u32,
        out_cell: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if !(0..=0x11).contains(&n.cast_signed()) && n != 0x30ff {
            return 0;
        }
        let text = mem.rebase(0x0133_f538);
        mem.write_bytes(text, &[0u8; 0x80]);
        let mut src = n;
        if n < 8 {
            src = call(mem, 0x0078_f7c0, &[n, 0]);
        }
        if n.wrapping_sub(8) <= 9 {
            src = call(mem, 0x0078_f7c0, &[n, 0]);
        }
        if n == 0x30ff {
            mem.write(out_cell, n);
            let data = call(mem, 0x0079_a4b0, &[0x30ff, out_cell]);
            let base: u32 = mem.read(out_cell);
            src = base.wrapping_add(u32::from(mem.read::<u16>(data.wrapping_add(8))));
        }
        let mut k = 0u32;
        loop {
            let c: u8 = mem.read(src.wrapping_add(k));
            mem.write(text.wrapping_add(k), c);
            if c == 0 {
                break;
            }
            k += 1;
        }
        let (w, h) = (mem.rebase(0x0133_f534), mem.rebase(0x0133_f530));
        call(mem, 0x008b_9650, &[0, text, w, h, 0, 0])
    }

    /// `TOBtlSetInfoWinMes(chr)` (`0x00899f20`): the battle info window's
    /// first line (`g_BtlInfoWinLines`, `0x0133f618`) after
    /// `TOBtlInitInfoWinMes`: the text `MsGetMenuChrInfo(chr, 1)` measured
    /// by `TOGetMessageWH(0, text, &w, &h, 0, 0)` (`f32 w` and the
    /// height in the words `size`, `size + 4`); its `i16` mode 8 when `w`
    /// is below the window's `i16 0x0133f61e`, else 2; x and y from the
    /// `i16`s `0x0133f68c`/`0x0133f690` as `f32`; `ftol(w)` as `u16` at
    /// `+0x14`; the text at `+0x18`; and the `u16 0x0133f688` 1. Returns
    /// 1.
    pub fn btl_set_info_win_mes<M: Mem>(
        mem: &mut M,
        chr: u32,
        size: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, 0x0089_7bd0, &[]);
        let text = call(mem, 0x0078_f7c0, &[chr, 1]);
        call(mem, 0x008b_9650, &[0, text, size, size + 4, 0, 0]);
        let w: f32 = mem.read(size);
        let limit: i16 = mem.read(mem.rebase(0x0133_f61e));
        let mode: u16 = if f64::from(w) < f64::from(limit) {
            8
        } else {
            2
        };
        let line = mem.rebase(0x0133_f618);
        mem.write(line, mode);
        for (at, from) in [(0xc, 0x0133_f68c), (0x10, 0x0133_f690)] {
            let v = i32::from(mem.read::<i16>(mem.rebase(from)));
            mem.write(line + at, int_f32(v));
        }
        let width = ffx_bridge::x87::ftol(mem, f64::from(w));
        #[allow(clippy::cast_possible_truncation)] // the original stores ax
        mem.write(line + 0x14, width as u16);
        mem.write(line + 0x18, text);
        mem.write(mem.rebase(0x0133_f688), 1u16);
        1
    }

    /// `TOBtlDrawLibraRight(x)` (`0x0089c7d0`): the Libra target's
    /// name (`MsGetMenuChrInfo(i16 0x0133f6aa, 2)`, measured by
    /// `TOGetMessageWH` into the words `size`, `size + 4` and not used
    /// further) drawn by `TOMkpCrossExtMesFontLClut(0, text, X2(915) + x,
    /// Y2(590), 0, 0.64, 0.5)`; returns its `eax`. `remap(mem, y_axis,
    /// v)` is `graphicUiRemapX2`/`Y2`.
    pub fn btl_draw_libra_right<M: Mem>(
        mem: &mut M,
        x: i32,
        size: u32,
        mut remap: impl FnMut(&mut M, bool, f32) -> f32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let chr = i32::from(mem.read::<i16>(mem.rebase(0x0133_f6aa))).cast_unsigned();
        let text = call(mem, 0x0078_f7c0, &[chr, 2]);
        call(mem, 0x008b_9650, &[0, text, size, size + 4, 0, 0]);
        let y = remap(mem, true, 590.0);
        let dx = remap(mem, false, 915.0);
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let x = (f64::from(dx) + f64::from(int_f32(x))) as f32;
        let args = [
            0,
            text,
            x.to_bits(),
            y.to_bits(),
            0,
            0.64f32.to_bits(),
            0.5f32.to_bits(),
        ];
        call(mem, 0x0090_16b0, &args)
    }

    /// The battle info window's status cycle (`0x0133f5c0`, 0x58 bytes):
    /// `i16 +0` the character, `u16 +2` the frame timer, `+4` the shown
    /// entry, `+6` the count, `+8` the `u16` message ids.
    const INFO_STATUS: u32 = 0x0133_f5c0;

    /// `TOBtlSetInfoWinParam(chr)` (`0x00899fd0`): the info window for
    /// `chr`. The status cycle is cleared and set to `chr`, timer 0x14;
    /// its HP and maximum (`MsGetRamChrHP`/`HPmax`) go to the `u16`s
    /// `0x0133f614`/`0x0133f616`; the ids appended as in
    /// [`btl_make_status_buff`] without the threat entry (bits 0..=11 of
    /// `MsGetChrStatusBits_0078f930`, the 13 counters, the 15
    /// `MsMenuBitStatus` bits), the count kept at `+6`. Then the first
    /// line as [`btl_set_info_win_mes`] with mode 2 and no position
    /// (`size` the words `TOGetMessageWH` writes). Returns 1.
    #[allow(clippy::cast_possible_truncation)] // the original stores the low halves
    pub fn btl_set_info_win_param<M: Mem>(
        mem: &mut M,
        chr: u32,
        size: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let cycle = mem.rebase(INFO_STATUS);
        mem.write_bytes(cycle, &[0u8; 0x58]);
        mem.write(cycle + 4, 0u32);
        mem.write(cycle, chr as u16);
        mem.write(cycle + 2, 0x14u16);
        let hp = call(mem, 0x0079_add0, &[chr]);
        mem.write(mem.rebase(0x0133_f614), hp as u16);
        let max = call(mem, 0x0079_adf0, &[chr]);
        mem.write(mem.rebase(0x0133_f616), max as u16);
        let shown = |mem: &M| i32::from(mem.read::<i16>(cycle)).cast_unsigned();
        let push = |mem: &mut M, id: u16| {
            let count: i16 = mem.read(cycle + 6);
            let at = cycle
                .wrapping_add(8)
                .wrapping_add((2 * i32::from(count)).cast_unsigned());
            mem.write(at, id);
            mem.write(cycle + 6, count.wrapping_add(1));
        };
        let chr_word = shown(mem);
        let bits = call(mem, 0x0078_f930, &[chr_word]) & 0xffff;
        for i in 0..12u16 {
            if bits >> i & 1 != 0 {
                push(mem, 0x1006 + i);
            }
        }
        let chr_word = shown(mem);
        let counts = call(mem, 0x0078_fa80, &[chr_word]);
        for i in 0..0xdu16 {
            if mem.read::<u8>(counts.wrapping_add(u32::from(i))) != 0 {
                push(mem, 0x1012 + i);
            }
        }
        let chr_word = shown(mem);
        let bits = call(mem, 0x0078_f950, &[chr_word]) & 0xffff;
        for i in 0..0xfu16 {
            if bits >> i & 1 != 0 {
                push(mem, 0x1020 + i);
            }
        }
        let text = call(mem, 0x0078_f7c0, &[chr, 1]);
        call(mem, 0x008b_9650, &[0, text, size, size + 4, 0, 0]);
        let w: f32 = mem.read(size);
        let line = mem.rebase(0x0133_f618);
        mem.write(line, 2u16);
        let width = ffx_bridge::x87::ftol(mem, f64::from(w));
        mem.write(line + 0x14, width as u16);
        mem.write(line + 0x18, text);
        mem.write(mem.rebase(0x0133_f688), 1u16);
        1
    }

    /// `TOBtlDrawInfoWinStatus(_, x, y)` (`0x00895240`): one frame of the
    /// info window's status cycle: the timer (`u16 +2`) counts down and,
    /// past 0, restarts at 0x14 and moves to the next entry (`+4`, back
    /// to 0 at the count); that entry's id names `MsMenuGetText(1, id,
    /// 1)`, centred in `X2(380)` from `x` (`ToGetBtlEasyFontWidth(text,
    /// &w, 0, 0.78, 1.0)` into the word `width`) and drawn by
    /// `ToMakeBtlEasyEdgeFont(text, x, y, 0, 0.78, 1.0)`; returns its
    /// `eax`. `remap(mem, y_axis, v)` is `graphicUiRemapX2`/`Y2`.
    pub fn btl_draw_info_win_status<M: Mem>(
        mem: &mut M,
        [x, y]: [i32; 2],
        width: u32,
        mut remap: impl FnMut(&mut M, bool, f32) -> f32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let cycle = mem.rebase(INFO_STATUS);
        let timer: u16 = mem.read(cycle + 2);
        mem.write(cycle + 2, timer.wrapping_sub(1));
        let entry = if timer == 0 {
            mem.write(cycle + 2, 0x14u16);
            let mut next = mem.read::<u16>(cycle + 4).wrapping_add(1);
            mem.write(cycle + 4, next);
            if next == mem.read::<u16>(cycle + 6) {
                next = 0;
                mem.write(cycle + 4, next);
            }
            next
        } else {
            mem.read(cycle + 4)
        };
        let at = cycle
            .wrapping_add(8)
            .wrapping_add((2 * i32::from(entry.cast_signed())).cast_unsigned());
        let id = u32::from(mem.read::<u16>(at));
        let text = call(mem, 0x0078_fcf0, &[1, id, 1]);
        let scale = 0.78f32.to_bits();
        let one = 1f32.to_bits();
        call(mem, 0x0090_5290, &[text, width, 0, scale, one]);
        let y = int_f32(y);
        let span = remap(mem, false, 380.0);
        let w: f32 = mem.read(width);
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let x = ((f64::from(span) - f64::from(w)) * 0.5 + f64::from(int_f32(x))) as f32;
        call(
            mem,
            0x0090_5930,
            &[text, x.to_bits(), y.to_bits(), 0, scale, one],
        )
    }

    /// `BtlEnd_InitState_008bdeb0(kind)` (own name; called by the
    /// battle-end screen's set-up): by the low byte of `kind`, 1 sets the
    /// state word `0x01869f08` to 0x1d, 2 to 0, anything else calls
    /// `AtelSetBattleEndKeepRecoverType(kind)` and sets 1. Then
    /// `0x01869f0c`, `g_TkBtlEndFlag` and the byte `0x00c5a068` are
    /// cleared and `BtlEnd_BuildList_008bde00` builds the list (its `eax`
    /// returned). Callees through `call` (`ffx.sym` addresses).
    pub fn btl_end_init_state<M: Mem>(
        mem: &mut M,
        kind: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let state = match kind & 0xff {
            1 => 0x1d,
            2 => 0,
            _ => {
                call(mem, 0x0086_f3a0, &[kind]);
                1
            }
        };
        mem.write(mem.rebase(0x0186_9f08), state);
        mem.write(mem.rebase(0x0186_9f0c), 0u32);
        mem.write(mem.rebase(0x0186_9fbc), 0u32);
        mem.write(mem.rebase(0x00c5_a068), 0u8);
        call(mem, 0x008b_de00, &[])
    }

    /// `BtlEnd_BuildList_008bde00()` (own name): the battle-end list.
    /// `0x01869f19` is set when any character `g_BtlCountsBattle`
    /// (`0x02310ea0`, bytes) counts has a nonzero `g_TkBtlEndResults`
    /// (`0x02310f20`, words) entry. For each counted character `i` (of 8),
    /// entry `n` (14 bytes from `0x01869f20`) gets: `+0` `BtlEnd_RowX(n,
    /// count)`, `+2` `6n`, `+4` 0, `+6` `i` (`u16`s), `+8`
    /// `TkMenuGetPlayerDamegedColor(i)`, `+9` `BtlEnd_InParty(i)`, `+0xa`
    /// 0 (bytes). The count goes to `g_TkBtlEndList` (`0x01869f18`), then
    /// `BtlEnd_CompactList` (its `eax` returned). Callees through `call`.
    pub fn btl_end_build_list<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let counts = mem.rebase(0x0231_0ea0);
        let results = mem.rebase(0x0231_0f20);
        let any_result = mem.rebase(0x0186_9f19);
        mem.write(any_result, 0u8);
        let mut count = 0u32;
        for i in 0..8 {
            if mem.read::<u8>(counts + i) != 0 {
                if mem.read::<u32>(results + 4 * i) != 0 {
                    mem.write(any_result, 1u8);
                }
                count += 1;
            }
        }
        let mut n = 0u32;
        let mut entry = mem.rebase(0x0186_9f20);
        for i in 0..8 {
            if mem.read::<u8>(counts + i) == 0 {
                continue;
            }
            #[allow(clippy::cast_possible_truncation)] // u16 stores
            {
                mem.write(entry + 2, (n * 6) as u16);
                mem.write(entry + 4, 0u16);
                mem.write(entry + 6, i as u16);
                let colour = call(mem, 0x008b_9eb0, &[i]);
                mem.write(entry + 8, colour as u8);
                let in_party = call(mem, 0x008b_9ec0, &[i]);
                mem.write(entry + 9, in_party as u8);
                let x = call(mem, 0x008b_a370, &[n, count]);
                mem.write(entry, x as u16);
            }
            mem.write(entry + 0xa, 0u8);
            n += 1;
            entry += 0xe;
        }
        #[allow(clippy::cast_possible_truncation)] // at most 8
        mem.write(mem.rebase(0x0186_9f18), count as u8);
        call(mem, 0x008b_a400, &[])
    }

    /// `TOBtlDrawDamageValue2()` (`0x00893290`): after
    /// `TOMkpSetEnvSubFont`, draws each of the `i16` count at `0x0133f108`
    /// (read again after each) 16-byte entries from `0x0133f110` (`i16` x
    /// `+0`, y `+2`, kind `+4`, value `+8`, colour bytes `+0xc..+0xf`), then
    /// clears the count. Before each, a pending 9999 (`0x01133390`) or 99999
    /// (`0x01133394`) damage unlocks achievement 0x12 or 0x15 and is
    /// cleared. In magic 0x11b, kind 3 entries are skipped. Values 0..=9
    /// are digits (`TOMakePktBtlDamageNumRGBA(value, x, y, r, g, b, a)`),
    /// 0x3d, 0x3c, 0x3b and 0x3a icons 0..=3 (`TOMakePktBtlIconA(icon, x,
    /// y, a)`); other values draw nothing. Callees through `call`; returns
    /// 0.
    pub fn draw_damage_value2<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let count = mem.rebase(0x0133_f108);
        let damage_9999 = mem.rebase(0x0113_3390);
        let damage_99999 = mem.rebase(0x0113_3394);
        call(mem, 0x0090_3300, &[]);
        let mut entry = mem.rebase(0x0133_f110);
        let mut i = 0i32;
        while i < i32::from(mem.read::<i16>(count)) {
            if mem.read::<u32>(damage_9999) != 0 {
                call(mem, 0x0082_2380, &[0x12]);
                mem.write(damage_9999, 0u32);
            }
            if mem.read::<u32>(damage_99999) != 0 {
                call(mem, 0x0082_2380, &[0x15]);
                mem.write(damage_99999, 0u32);
            }
            let magic = call(mem, 0x009d_a3b0, &[]);
            if magic != 0x11b || mem.read::<u32>(entry + 4) != 3 {
                let value: i32 = mem.read(entry + 8);
                let x = i32::from(mem.read::<i16>(entry)).cast_unsigned();
                let y = i32::from(mem.read::<i16>(entry + 2)).cast_unsigned();
                let colour = |m: &M, k: u32| u32::from(m.read::<u8>(entry + 0xc + k));
                if (0..=9).contains(&value) {
                    let rgba = [
                        colour(mem, 0),
                        colour(mem, 1),
                        colour(mem, 2),
                        colour(mem, 3),
                    ];
                    let args = [
                        value.cast_unsigned(),
                        x,
                        y,
                        rgba[0],
                        rgba[1],
                        rgba[2],
                        rgba[3],
                    ];
                    call(mem, 0x008f_b840, &args);
                } else {
                    let icon = match value {
                        0x3d => Some(0),
                        0x3c => Some(1),
                        0x3b => Some(2),
                        0x3a => Some(3),
                        _ => None,
                    };
                    if let Some(icon) = icon {
                        let alpha = colour(mem, 3);
                        call(mem, 0x008f_bcc0, &[icon, x, y, alpha]);
                    }
                }
            }
            i += 1;
            entry += 0x10;
        }
        mem.write(count, 0u16);
        0
    }

    /// `TOBtlDrawSummonPrim(chr, x, y)` (`0x008a0ba0`) and
    /// `TOBtlDrawChangePrim(chr, x, y)` (`0x0089f0e0`): a battle window
    /// at the integer position (`x`, `y`) with the name of character `chr`
    /// centred in it and its HP/MP plate (`TOBtlDrawCrossParamPlate`).
    /// `remap(mem, y_axis, v)` is `graphicUiRemapY2`/`X2` (an `f32`);
    /// `scratch` is the 4-byte name width `ToGetBtlEasyFontWidth` writes;
    /// callees through `call` (`ffx.sym` addresses). Returns the plate's
    /// `eax`.
    pub struct BtlPrim<'a, M, R, C> {
        pub mem: &'a mut M,
        pub remap: R,
        pub call: C,
        pub scratch: u32,
    }

    impl<M, R, C> BtlPrim<'_, M, R, C>
    where
        M: Mem,
        R: FnMut(&mut M, bool, f32) -> f32,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        fn x(&mut self, v: f32) -> f64 {
            f64::from((self.remap)(self.mem, false, v))
        }

        fn y(&mut self, v: f32) -> f64 {
            f64::from((self.remap)(self.mem, true, v))
        }

        fn run(&mut self, sym: u32, args: &[u32]) -> u32 {
            (self.call)(self.mem, sym, args)
        }

        /// The name width of `name` at `scale` (`ToGetBtlEasyFontWidth(name,
        /// &w, 0, scale, 1.0)`).
        fn name_width(&mut self, name: u32, scale: f32) -> f32 {
            let args = [name, self.scratch, 0, scale.to_bits(), 1.0f32.to_bits()];
            self.run(0x0090_5290, &args);
            self.mem.read(self.scratch)
        }

        /// `ToMakeBtlEasyEdgeFont(name, x, y, 0, scale, 1.0)`.
        fn edge_font(&mut self, name: u32, x: f32, y: f32, scale: f32) {
            let args = [
                name,
                x.to_bits(),
                y.to_bits(),
                0,
                scale.to_bits(),
                1.0f32.to_bits(),
            ];
            self.run(0x0090_5930, &args);
        }

        /// `TOBtlDrawCrossParamPlate(chr, x, y)`; its `eax`.
        fn plate(&mut self, chr: u32, x: f32, y: f32) -> u32 {
            self.run(0x0089_2cb0, &[chr, x.to_bits(), y.to_bits()])
        }

        /// `TOBtlDrawSummonPrim`: the net window (`TODrawBtlNetWindowXYWHType
        /// (x, y, X2(344), Y2(114), 0)`), the name at y `Y2(16) + y` in
        /// width scale 0.8 centred in `X2(344)` (drawn at scale 0.7), the
        /// plate at (`X2(15) + x`, `Y2(61) + y`).
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        pub fn summon(&mut self, chr: u32, x: i32, y: i32) -> u32 {
            let (fx, fy) = (x as f32, y as f32);
            let h = self.y(114.0) as f32;
            let w = self.x(344.0) as f32;
            let window = [fx.to_bits(), fy.to_bits(), w.to_bits(), h.to_bits(), 0];
            self.run(0x008f_41b0, &window);
            let name = self.run(0x0089_76f0, &[chr]);
            let width = self.name_width(name, 0.8);
            let name_y = (self.y(16.0) + f64::from(fy)) as f32;
            let name_x = ((self.x(344.0) - f64::from(width)) * 0.5 + f64::from(fx)) as f32;
            self.edge_font(name, name_x, name_y, 0.7);
            let plate_y = (self.y(61.0) + f64::from(fy)) as f32;
            let plate_x = (self.x(15.0) + f64::from(fx)) as f32;
            self.plate(chr, plate_x, plate_y)
        }

        /// `TOBtlDrawChangePrim`: the frame `TOMkpShapePartIndexXYWHC2(part,
        /// 0, X2(20) + x, Y2(14) + y, X2(70), Y2(50), grey, grey)` with part
        /// 0x1c0 for an aeon (`chr >= 8`), 0x1d4 for character 6 while
        /// `AtelGetScenarioFlag() < 0x434` (signed), else `chr + 0x1b8`; the
        /// name at y `Y2(16) + y` and x `X2(210) + x` less half its width
        /// (scale 0.71 both); the plate at (`X2(12) + x`, `Y2(62) + y`).
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        pub fn change(&mut self, chr: u32, x: i32, y: i32) -> u32 {
            let part = if chr.cast_signed() >= 8 {
                0x1c0
            } else if chr == 6 && self.run(0x0086_c470, &[]).cast_signed() < 0x434 {
                0x1d4
            } else {
                chr.wrapping_add(0x1b8)
            };
            let (fx, fy) = (x as f32, y as f32);
            let h = self.y(50.0) as f32;
            let w = self.x(70.0) as f32;
            let top = (self.y(14.0) + f64::from(fy)) as f32;
            let left = (self.x(20.0) + f64::from(fx)) as f32;
            let grey = 0x8080_8080;
            let frame = [
                part,
                0,
                left.to_bits(),
                top.to_bits(),
                w.to_bits(),
                h.to_bits(),
                grey,
                grey,
            ];
            self.run(0x0090_34b0, &frame);
            let name = self.run(0x0089_76f0, &[chr]);
            let width = self.name_width(name, 0.71);
            let name_y = (self.y(16.0) + f64::from(fy)) as f32;
            let name_x = (self.x(210.0) + f64::from(fx) - f64::from(width) * 0.5) as f32;
            self.edge_font(name, name_x, name_y, 0.71);
            let plate_y = (self.y(62.0) + f64::from(fy)) as f32;
            let plate_x = (self.x(12.0) + f64::from(fx)) as f32;
            self.plate(chr, plate_x, plate_y)
        }
    }

    /// The battle status window's party (`0x0133f7ac`, three `u16`
    /// characters, 0xff for none).
    pub const STATUS_PARTY: u32 = 0x0133_f7ac;
    /// The battle status window's rows (`0x0133f7d8`, three of 0x90 bytes).
    pub const STATUS_ROWS: u32 = 0x0133_f7d8;

    /// `TOBtlInitStatusWin()` (`0x00898010`): clears the three status rows,
    /// takes the party from `MsGetSaveWindow(0, 0x12, &n)` (its first three
    /// `u16`s; `n`, in `count`, is trimmed past trailing 0xff entries), and
    /// for each character `c` there (re-read before each call) fills its
    /// row: HP (`MsGetRamChrHP`) at `+0xc` and `+0x10`, MP
    /// (`MsGetRamChrMP`) at `+0x24` and `+0x28`, the limit gauge scaled to
    /// the `i16` width at `0x025d0aa4` (`gauge * width / max`, signed) at
    /// `+0x34` and `+0x38`, `TOBtlMakeStatusBuff(c, row + 0x42)` at `+0x40`
    /// (`u16`) and 0x14 at `+0x3c` (`u16`); `+0x14`, `+0x2c` (words) and
    /// `+0x18`, `+0x30` (bytes) cleared. A zero maximum (or `i32::MIN /
    /// -1`) faults in the original; here it gives 0. Callees through
    /// `call`. Returns what the original leaves in `eax`: 0x14 when the
    /// last slot holds a character, else 0xff.
    pub fn init_status_win<M: Mem>(
        mem: &mut M,
        count: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let rows = mem.rebase(STATUS_ROWS);
        for k in (0..0x1b0).step_by(4) {
            mem.write(rows + k, 0u32);
        }
        mem.write(count, 0u32);
        let window = call(mem, 0x0078_5410, &[0, 0x12, count]);
        let mut n: i32 = mem.read(count);
        while n > 0 && mem.read::<u16>(window.wrapping_add((2 * n - 2).cast_unsigned())) == 0xff {
            n -= 1;
            mem.write(count, n);
        }
        let party = mem.rebase(STATUS_PARTY);
        for k in 0..3 {
            let c: u16 = mem.read(window.wrapping_add(2 * k));
            mem.write(party + 2 * k, c);
        }
        let width = mem.rebase(0x025d_0aa4);
        let mut eax = 0;
        for k in 0..3 {
            let slot = party + 2 * k;
            let row = rows + 0x90 * k;
            let c = u32::from(mem.read::<u16>(slot));
            if c == 0xff {
                eax = 0xff;
                continue;
            }
            let hp = call(mem, 0x0079_add0, &[c]);
            mem.write(row + 0xc, hp);
            mem.write(row + 0x10, hp);
            let c = u32::from(mem.read::<u16>(slot));
            mem.write(row + 0x14, 0u32);
            mem.write(row + 0x18, 0u8);
            let mp = call(mem, 0x0079_ae50, &[c]);
            mem.write(row + 0x24, mp);
            mem.write(row + 0x28, mp);
            let c = u32::from(mem.read::<u16>(slot));
            mem.write(row + 0x2c, 0u32);
            mem.write(row + 0x30, 0u8);
            let gauge = call(mem, 0x0079_5550, &[c]).cast_signed();
            let scale = i32::from(mem.read::<i16>(width));
            let c = u32::from(mem.read::<u16>(slot));
            let max = call(mem, 0x0079_5590, &[c]).cast_signed();
            let bar = gauge.wrapping_mul(scale).checked_div(max).unwrap_or(0);
            mem.write(row + 0x34, bar);
            mem.write(row + 0x38, bar);
            let c = u32::from(mem.read::<u16>(slot));
            let buff = call(mem, 0x0089_8260, &[c, row + 0x42]);
            #[allow(clippy::cast_possible_truncation)] // the u16 store
            mem.write(row + 0x40, buff as u16);
            mem.write(row + 0x3c, 0x14u16);
            eax = 0x14;
        }
        eax
    }

    /// The battle status window's mode (`0x0133d782`): 0 HP, 1 MP, 2 the
    /// statuses, 3 the limit gauges.
    pub const STATUS_MODE: u32 = 0x0133_d782;
    /// `TOBtlInitStatusWin()`.
    const INIT_STATUS_WIN: u32 = 0x0089_8010;
    /// The scratch [`ctrl_status_win`] needs: the save-window count and
    /// `MsGetComData`'s second result.
    pub const CTRL_STATUS_WORK: u32 = 8;

    /// One shown value of a status row stepping to its target: `at` the
    /// target, `at + 4` the shown value, `at + 8` the step, the byte
    /// `frames` the steps left. A new target sets the step to a tenth of
    /// the difference over 10 frames; then the shown value steps, or with
    /// no frames left is the target.
    fn step_to<M: Mem>(mem: &mut M, at: u32, frames: u32, value: u32) {
        let target: u32 = mem.read(at);
        if value != target {
            let step = value.cast_signed().wrapping_sub(target.cast_signed()) / 10;
            mem.write(at + 8, step);
            mem.write(frames, 10u8);
            mem.write(at, value);
        }
        let left: i8 = mem.read(frames);
        if left > 0 {
            let shown = mem.read::<u32>(at + 4).wrapping_add(mem.read(at + 8));
            mem.write(at + 4, shown);
            mem.write(frames, left - 1);
        } else {
            mem.write(at + 4, value);
            mem.write(at + 8, 0u32);
            mem.write(frames, 0u8);
        }
    }

    /// `TOBtlCtrlStatusWin()` (`0x008920d0`): one frame of the battle
    /// status window. The mode ([`STATUS_MODE`]) follows the current
    /// command (`MsGetComData` of the `u16` at `0x0133c95c + 0xf0 *
    /// g_BtlActionDepth`): with its `+0x22` bit 0 set, bits 1–2 (a change
    /// first re-inits the window, `TOBtlInitStatusWin`); else, or with no
    /// command (`0x023cc088` 0xff, no re-init), 3. The window is re-inited
    /// when the party (`MsGetSaveWindow(0, 0x12, &n)`'s first three
    /// `u16`s, `n` trimmed past trailing 0xff entries) differs from
    /// [`STATUS_PARTY`]. Then each character's row ([`STATUS_ROWS`]) steps
    /// its shown HP (`+0xc` target, `+0x10` shown, `+0x14` step, byte
    /// `+0x18` frames) and MP (`+0x24`..`+0x30`) toward the current values,
    /// sets the HP colour byte `+0x19` from `MsCalcWeakLevel(shown, max)`
    /// (1 and 2: 2, 3: 0x53, else 0) and `+0x31` to 0, and moves the
    /// shown limit gauge `+0x38` up by 6 a frame (never past) toward its
    /// target `+0x34`: the gauge scaled to the `i16` width at
    /// `0x025d0aa4` (`gauge * width / max`, 0 without a positive maximum;
    /// a zero second maximum faults in the original, here it gives 0).
    /// Callees through `call`; the counts at `work`
    /// ([`CTRL_STATUS_WORK`]).
    pub fn ctrl_status_win<M: Mem>(
        mem: &mut M,
        work: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let mode_at = mem.rebase(STATUS_MODE);
        let mut mode = 3u8;
        if ACTION_PREV_VALUE.get(mem) != 0xff {
            let depth = i32::from(ACTION_DEPTH.get(mem));
            let id_at = mem
                .rebase(0x0133_c95c)
                .wrapping_add(depth.wrapping_mul(0xf0).cast_unsigned());
            let id = u32::from(mem.read::<u16>(id_at));
            let command = call(mem, 0x0079_a4b0, &[id, work + 4]);
            let flags: u8 = mem.read(command + 0x22);
            if flags & 1 == 0 {
                call(mem, INIT_STATUS_WIN, &[]);
            } else {
                mode = (flags >> 1) & 3;
                if mem.read::<u8>(mode_at) != mode {
                    call(mem, INIT_STATUS_WIN, &[]);
                }
            }
        }
        mem.write(mode_at, mode);

        // the party, trimmed
        mem.write(work, 0u32);
        let window = call(mem, 0x0078_5410, &[0, 0x12, work]);
        let mut n: i32 = mem.read(work);
        while n > 0 && mem.read::<u16>(window.wrapping_add((2 * n - 2).cast_unsigned())) == 0xff {
            n -= 1;
            mem.write(work, n);
        }
        let party = mem.rebase(STATUS_PARTY);
        if mem.read::<u32>(window) != mem.read::<u32>(party)
            || mem.read::<u16>(window + 4) != mem.read::<u16>(party + 4)
        {
            call(mem, INIT_STATUS_WIN, &[]);
        }

        // each character's row
        let rows = mem.rebase(STATUS_ROWS);
        let width = mem.rebase(0x025d_0aa4);
        for k in 0..3 {
            let slot = party + 2 * k;
            let row = rows + 0x90 * k;
            let chr = |mem: &M| u32::from(mem.read::<u16>(slot));
            if chr(mem) == 0xff {
                continue;
            }
            let hp = call(mem, 0x0079_add0, &[chr(mem)]);
            let mp = call(mem, 0x0079_ae50, &[chr(mem)]);
            let hp_max = call(mem, 0x0079_adf0, &[chr(mem)]);
            call(mem, 0x0079_ae70, &[chr(mem)]);
            step_to(mem, row + 0xc, row + 0x18, hp);
            let shown: u32 = mem.read(row + 0x10);
            let colour = match call(mem, 0x0078_bf00, &[shown, hp_max]) {
                1 | 2 => 2u8,
                3 => 0x53,
                _ => 0,
            };
            mem.write(row + 0x19, colour);
            step_to(mem, row + 0x24, row + 0x30, mp);
            mem.write(row + 0x31, 0u8);
            let bar = if call(mem, 0x0079_5590, &[chr(mem)]).cast_signed() > 0 {
                let gauge = call(mem, 0x0079_5550, &[chr(mem)]).cast_signed();
                let scale = i32::from(mem.read::<i16>(width));
                let max = call(mem, 0x0079_5590, &[chr(mem)]).cast_signed();
                gauge.wrapping_mul(scale).checked_div(max).unwrap_or(0)
            } else {
                0
            };
            mem.write(row + 0x34, bar);
            let shown: i32 = mem.read(row + 0x38);
            if shown < bar {
                mem.write(row + 0x38, shown.wrapping_add(6));
            }
            if mem.read::<i32>(row + 0x38) > bar {
                mem.write(row + 0x38, bar);
            }
        }
    }

    /// The battle info window's two "shown" flags: the status line
    /// (`0x0133c93e`) and the message lines ([`INFO_MES_SHOWN`], read by
    /// `TOBtlDrawInfoWinMes`).
    pub const INFO_SHOWN: u32 = 0x0133_c93e;
    /// `g_BtlInfoWinShown`.
    pub const INFO_MES_SHOWN: u32 = 0x0133_c93d;
    /// The time the status line was last shown (`f32`, `graphicGetTime`).
    pub const INFO_SHOWN_TIME: u32 = 0x0133_c944;
    /// The character the target cursor is on (`u16`, the low half of the
    /// word at `0x0133d1f4`).
    pub const TARGET_CHR: u32 = 0x0133_d1f4;
    /// The character the status line and the message lines were last set
    /// up for (`i16`s).
    pub const INFO_PARAM_CHR: u32 = 0x0133_f5b8;
    pub const INFO_MES_CHR: u32 = 0x0133_f5bc;
    /// `MsMenuCheckLook()`, `MsMenuCheckChrLook(chr)`,
    /// `MsMenuCheckChrLive(chr)`, `TOBtlSetInfoWinParam(chr)`,
    /// `TOBtlSetInfoWinMes(chr)` and `graphicGetTime()`.
    const CHECK_LOOK: u32 = 0x0078_f9e0;
    const CHECK_CHR_LOOK: u32 = 0x0078_f990;
    const CHECK_CHR_LIVE: u32 = 0x0078_f970;
    const SET_INFO_WIN_PARAM: u32 = 0x0089_9fd0;
    const SET_INFO_WIN_MES: u32 = 0x0089_9f20;
    const GET_TIME: u32 = 0x0064_1410;

    /// The command window at the current action depth is open (the byte
    /// `0x0133c952 + 0xf0 * g_BtlActionDepth` is 1).
    fn command_window_open(mem: &impl Mem) -> bool {
        let depth = i32::from(ACTION_DEPTH.get(mem));
        let at = mem
            .rebase(0x0133_c952)
            .wrapping_add(depth.wrapping_mul(0xf0).cast_unsigned());
        mem.read::<u8>(at) == 1
    }

    /// The target cursor's character as the callees get it (`movzx`).
    fn target_chr(mem: &impl Mem) -> u32 {
        u32::from(mem.read::<u16>(mem.rebase(TARGET_CHR)))
    }

    /// The target is a party member (characters 0x14..=0x1b are the
    /// party slots, `(u16)(chr - 0x14) <= 7`).
    fn target_in_party(mem: &impl Mem) -> bool {
        (0x14..=0x1b).contains(&target_chr(mem))
    }

    /// The info window's lines follow the target: when it differs from
    /// the character at `last`, `set(chr)` (through `call`) and `last` takes
    /// the target, re-read after the call.
    fn follow_target<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        last: u32,
        set: u32,
    ) {
        let last = mem.rebase(last);
        let chr = target_chr(mem);
        #[allow(clippy::cast_possible_truncation)] // a u16
        if mem.read::<u16>(last) != chr as u16 {
            call(mem, set, &[chr]);
            #[allow(clippy::cast_possible_truncation)]
            mem.write(last, target_chr(mem) as u16);
        }
    }

    /// `TOBtlCtrlInfoWin()` (`0x00891480`): one frame of the battle info
    /// window's status line. Shown ([`INFO_SHOWN`] 1) only while the
    /// command window is open and the target is a party member that is
    /// either looked at (`MsMenuCheckLook()` -1 and
    /// `MsMenuCheckChrLook(chr)` 1) or alive (`MsMenuCheckChrLive(chr)`
    /// 1); a target other than [`INFO_PARAM_CHR`] sets the line up first
    /// (`TOBtlSetInfoWinParam`). The frame it appears, the time
    /// (`graphicGetTime`, through `fcall`) goes to [`INFO_SHOWN_TIME`].
    /// Callees through `call`.
    pub fn ctrl_info_win<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        let shown = mem.rebase(INFO_SHOWN);
        let was: u8 = mem.read(shown);
        if !command_window_open(mem) {
            mem.write(shown, 0u8);
            return;
        }
        let call = &mut call;
        let looked = call(mem, CHECK_LOOK, &[]) == u32::MAX
            && target_in_party(mem)
            && call(mem, CHECK_CHR_LOOK, &[target_chr(mem)]) == 1;
        if looked {
            follow_target(mem, call, INFO_PARAM_CHR, SET_INFO_WIN_PARAM);
        }
        mem.write(shown, u8::from(looked));
        if call(mem, CHECK_CHR_LIVE, &[target_chr(mem)]) == 1 && target_in_party(mem) {
            follow_target(mem, call, INFO_PARAM_CHR, SET_INFO_WIN_PARAM);
            mem.write(shown, 1u8);
        }
        if was == 0 && mem.read::<u8>(shown) == 1 {
            let now = ffx_bridge::x87::store(fcall(mem, GET_TIME, &[]));
            mem.write(mem.rebase(INFO_SHOWN_TIME), now);
        }
    }

    /// The info window's message lines (`g_BtlInfoWinLines`, four
    /// records of 0x1c bytes from `0x0133f618`): `u16 +0` the state (0
    /// unused, 2 scrolling in, 3 scrolling out, 8 a line that fits and
    /// stands still), `f32 +0xc` x, `f32 +0x10` y, `i16 +0x14` the width,
    /// `+0x18` the text; the count of lines in use is the `u16` at
    /// `0x0133f688`.
    pub const INFO_LINES: u32 = 0x0133_f618;
    const INFO_LINE: u32 = 0x1c;
    /// The info window's box (`g_BtlInfoWinBox`): `i16` left at `+0`,
    /// right at `+8`.
    const INFO_BOX: u32 = 0x0133_f694;

    /// The right end of a line (`width + x`, at 53 bits).
    fn line_end(mem: &impl Mem, line: u32) -> f64 {
        let width = i32::from(mem.read::<i16>(line + 0x14));
        ffx_bridge::x87::add(
            f64::from(width),
            ffx_bridge::x87::load(mem.read(line + 0xc)),
        )
    }

    /// `TOBtlCtrlInfoWinMes()` (`0x00891580`): one frame of the battle
    /// info window's message lines. [`INFO_MES_SHOWN`] follows the target
    /// as in [`ctrl_info_win`] (looked at: `MsMenuCheckLook()` -1 alone;
    /// alive: `MsMenuCheckChrLive(chr)` 1), the lines set up by
    /// `TOBtlSetInfoWinMes` for a new target ([`INFO_MES_CHR`]). Then,
    /// unless the first line stands still (state 8), the ticker: every
    /// line in use moves left by 1.2 (the `f32` value); a line scrolling
    /// in whose right end has passed 80 units inside the box's right edge
    /// (`end < right - 80`) starts the next line over again at the right edge (the same
    /// width, y and text) and scrolls out itself; a line scrolling out
    /// whose end has passed the left edge is dropped; the lines in use
    /// are packed to the front (the rest cleared) and counted. Callees
    /// through `call`.
    pub fn ctrl_info_win_mes<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87;
        let shown = mem.rebase(INFO_MES_SHOWN);
        if !command_window_open(mem) {
            mem.write(shown, 0u8);
            return;
        }
        let call = &mut call;
        let looked = call(mem, CHECK_LOOK, &[]) == u32::MAX && target_in_party(mem);
        if looked {
            follow_target(mem, call, INFO_MES_CHR, SET_INFO_WIN_MES);
        }
        mem.write(shown, u8::from(looked));
        if call(mem, CHECK_CHR_LIVE, &[target_chr(mem)]) == 1 && target_in_party(mem) {
            follow_target(mem, call, INFO_MES_CHR, SET_INFO_WIN_MES);
            mem.write(shown, 1u8);
        }

        let lines = mem.rebase(INFO_LINES);
        let line = |i: u32| lines + INFO_LINE * i;
        let state = |mem: &M, i: u32| mem.read::<u16>(line(i));
        if state(mem, 0) == 8 {
            return;
        }
        for i in 0..4 {
            if state(mem, i) != 0 {
                // the step is the double 0x3ff3333340000000, 1.2 as an f32
                let x = x87::sub(x87::load(mem.read(line(i) + 0xc)), f64::from(1.2f32));
                mem.write(line(i) + 0xc, x87::store(x));
            }
        }
        let bx = mem.rebase(INFO_BOX);
        let right = i32::from(mem.read::<i16>(bx + 8));
        let left = f64::from(mem.read::<i16>(bx));
        let start = f64::from(int_f32(right - 0x50));
        for i in 0..3 {
            let (this, next) = (line(i), line(i + 1));
            if state(mem, i) == 2 && line_end(mem, this) < start {
                mem.write(next, 2u16);
                mem.write(next + 0xc, int_f32(right));
                mem.write(next + 0x14, mem.read::<u16>(this + 0x14));
                mem.write(next + 0x18, mem.read::<u32>(this + 0x18));
                mem.write(next + 0x10, x87::store(x87::load(mem.read(this + 0x10))));
                mem.write(this, 3u16);
            }
        }
        for i in 0..4 {
            if state(mem, i) == 3 && line_end(mem, line(i)) < left {
                mem.write(line(i), 0u16);
            }
        }
        let mut kept = [0u8; 4 * INFO_LINE as usize];
        let mut n = 0u16;
        for i in 0..4 {
            if state(mem, i) != 0 {
                let mut record = [0u8; INFO_LINE as usize];
                mem.read_bytes(line(i), &mut record);
                let at = usize::from(n) * INFO_LINE as usize;
                kept[at..at + INFO_LINE as usize].copy_from_slice(&record);
                n += 1;
            }
        }
        mem.write_bytes(lines, &kept);
        mem.write(mem.rebase(0x0133_f688), n);
    }

    /// The limit windows' shared callees: `TOSeqStartSceneDrawContinue(2,
    /// name)`, `TOSeqCheckTermScene(2, name)`, `TOSeqStartScene(2, name)`,
    /// `TOSeqStopScene(2, name)`, the limit timer's stops and start
    /// (`0x0089ab20`, `0x0089aba0`, `0x0089ab50`), `SndPlayClick(n)`,
    /// `MsMenuLimitSet_007b0470(kind, result, left, total)` and
    /// `TOBtlInitLimitTimer()`.
    mod limit {
        pub const SEQ_START_CONTINUE: u32 = 0x008a_f1b0;
        pub const SEQ_CHECK_TERM: u32 = 0x008a_e070;
        pub const SEQ_START: u32 = 0x008a_f0d0;
        pub const SEQ_STOP: u32 = 0x008a_f4f0;
        pub const TIMER_STOP: u32 = 0x0089_ab20;
        pub const TIMER_STOP2: u32 = 0x0089_aba0;
        pub const TIMER_START: u32 = 0x0089_ab50;
        pub const PLAY_CLICK: u32 = 0x0088_6ba0;
        pub const LIMIT_SET: u32 = 0x007b_0470;
        pub const INIT_TIMER: u32 = 0x0089_7f00;
        /// The limit timer: the time left and the total (`f32`).
        pub const TIME_LEFT: u32 = 0x0133_f790;
        pub const TIME_TOTAL: u32 = 0x0133_f78c;
        /// The scene names (`.rdata`).
        pub const BAURO: u32 = 0x00b5_eccc;
        pub const BAURO2: u32 = 0x00b5_ecd4;
        pub const BGOOD: u32 = 0x00b5_ecdc;
        pub const BTIDU: u32 = 0x00b5_ecec;
        pub const BLULU: u32 = 0x00b5_ed14;
        pub const BLULU2: u32 = 0x00b5_ed1c;
        pub const BGOODL: u32 = 0x00b5_ed24;
    }

    /// A limit window's scene call `f(2, name)`.
    fn scene<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        f: u32,
        name: u32,
    ) -> u32 {
        let name = mem.rebase(name);
        call(mem, f, &[2, name])
    }

    /// One step more of a limit window's state (`u16 +1`).
    fn next_state(mem: &mut impl Mem, state: u32) {
        let v = mem.read::<u16>(state).wrapping_add(1);
        mem.write(state, v);
    }

    /// The time left is up (`fldz; fcomp`: a NaN is not).
    fn time_up(mem: &impl Mem) -> bool {
        mem.read::<f32>(mem.rebase(limit::TIME_LEFT)) <= 0.0
    }

    /// `MsMenuLimitSet_007b0470(kind, result, left, total)`, the times
    /// copied through x87.
    fn limit_set<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        kind: u32,
        result: u32,
    ) {
        let copy = |mem: &M, a: u32| {
            ffx_bridge::x87::store(ffx_bridge::x87::load(mem.read(mem.rebase(a))))
        };
        let (left, total) = (copy(mem, limit::TIME_LEFT), copy(mem, limit::TIME_TOTAL));
        call(mem, limit::LIMIT_SET, &[kind, result, left, total]);
    }

    /// Auron's limit window: its state (`i16 0x0133d6f4`), the result
    /// (`0x0133d6f2`), the button pressed (`u16 0x025d09d4`), the
    /// combination's position (`i16 0x0133d6fc`), its buttons (`0x0133d704`,
    /// `u16`s) and length (`i32 0x0133d700`).
    pub const AURON_STATE: u32 = 0x0133_d6f4;
    const AURON_RESULT: u32 = 0x0133_d6f2;

    /// `TOBtlCtrlAuronLimitWindow()` (`0x00890f70`): Auron's limit
    /// (Bushido) window, one state a frame: 1 the `bauro` scene
    /// (result 0), 2 on its end `bauro2`, 3 the timer stops, 4 the timer
    /// runs (time up: 7) while each button pressed must be the
    /// combination's next (else a click 0x69 and back to its start); the
    /// whole of it is 5: the timer stops, result 1, `bgood` and a click
    /// 0x68, 6 waits for `bgood`'s end, 7 the timer stops with result 0,
    /// 8 the scenes stop, 9 a frame, 10 the result to
    /// `MsMenuLimitSet_007b0470(2, ..)` and the timer re-inits (0). Callees
    /// through `call`.
    pub fn ctrl_auron_limit_window<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let state = mem.rebase(AURON_STATE);
        let result = mem.rebase(AURON_RESULT);
        let call = &mut call;
        match mem.read::<i16>(state) {
            1 => {
                mem.write(result, 0u8);
                scene(mem, call, limit::SEQ_START_CONTINUE, limit::BAURO);
                next_state(mem, state);
            }
            2 => {
                if scene(mem, call, limit::SEQ_CHECK_TERM, limit::BAURO) == 1 {
                    scene(mem, call, limit::SEQ_START, limit::BAURO2);
                    next_state(mem, state);
                }
            }
            3 => {
                call(mem, limit::TIMER_STOP, &[]);
                next_state(mem, state);
            }
            4 => {
                call(mem, limit::TIMER_START, &[]);
                if time_up(mem) {
                    mem.write(state, 7u16);
                    return;
                }
                let pressed: u16 = mem.read(mem.rebase(0x025d_09d4));
                if pressed == 0 {
                    return;
                }
                let at = mem.rebase(0x0133_d6fc);
                let i: i16 = mem.read(at);
                let buttons: u32 = mem.read(mem.rebase(0x0133_d704));
                let want: u16 =
                    mem.read(buttons.wrapping_add(i32::from(i).cast_unsigned().wrapping_mul(2)));
                let i = if pressed == want {
                    i.wrapping_add(1)
                } else {
                    call(mem, limit::PLAY_CLICK, &[0x69]);
                    0
                };
                mem.write(at, i);
                if i32::from(i) >= mem.read::<i32>(mem.rebase(0x0133_d700)) {
                    mem.write(state, 5u16);
                }
            }
            5 => {
                call(mem, limit::TIMER_STOP2, &[]);
                mem.write(result, 1u8);
                scene(mem, call, limit::SEQ_START, limit::BGOOD);
                call(mem, limit::PLAY_CLICK, &[0x68]);
                mem.write(state, 6u16);
            }
            6 | 8 => {
                if mem.read::<i16>(state) == 6
                    && scene(mem, call, limit::SEQ_CHECK_TERM, limit::BGOOD) != 1
                {
                    return;
                }
                scene(mem, call, limit::SEQ_STOP, limit::BAURO);
                scene(mem, call, limit::SEQ_STOP, limit::BAURO2);
                mem.write(state, 9u16);
            }
            7 => {
                call(mem, limit::TIMER_STOP2, &[]);
                mem.write(result, 0u8);
                mem.write(state, 8u16);
            }
            9 => next_state(mem, state),
            10 => {
                let r = i32::from(mem.read::<i8>(result)).cast_unsigned();
                limit_set(mem, call, 2, r);
                call(mem, limit::INIT_TIMER, &[]);
                mem.write(state, 0u16);
            }
            _ => {}
        }
    }

    /// Tidus's limit window: its state (`i16 0x0133d734`), the result
    /// (`0x0133d6f3`), the bar (`i16`s from `0x0133d736`: the start, `+4`
    /// the length, `+6`/`+8` the good part, `+0xc` the position, `+0xe`
    /// the step).
    pub const TIDUS_STATE: u32 = 0x0133_d734;
    const TIDUS_RESULT: u32 = 0x0133_d6f3;

    /// `TOBtlCtrlTidusLimitWindow()` (`0x00892320`): Tidus's limit (Swordplay)
    /// window, its states as Auron's ([`ctrl_auron_limit_window`]) with
    /// the `btidu` scene and `MsMenuLimitSet_007b0470(0, ..)`; state 4 moves
    /// the marker by its step between the bar's ends (bouncing back and
    /// turning the step at each); a press (`TkGetControlerTrigHist` bit
    /// 0x20 of the held or new presses) inside the good part is state 5,
    /// elsewhere a click 3, the marker back to the start and the step
    /// forward again. Callees through `call`.
    pub fn ctrl_tidus_limit_window<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let state = mem.rebase(TIDUS_STATE);
        let result = mem.rebase(TIDUS_RESULT);
        let call = &mut call;
        match mem.read::<i16>(state) {
            1 => {
                mem.write(result, 0u8);
                scene(mem, call, limit::SEQ_START_CONTINUE, limit::BTIDU);
                next_state(mem, state);
            }
            2 => {
                if scene(mem, call, limit::SEQ_CHECK_TERM, limit::BTIDU) == 1 {
                    next_state(mem, state);
                }
            }
            3 => {
                call(mem, limit::TIMER_STOP, &[]);
                next_state(mem, state);
            }
            4 => tidus_bar(mem, call, state),
            5 => {
                call(mem, limit::TIMER_STOP2, &[]);
                mem.write(result, 1u8);
                scene(mem, call, limit::SEQ_START, limit::BGOOD);
                call(mem, limit::PLAY_CLICK, &[0x68]);
                mem.write(state, 6u16);
            }
            6 | 8 => {
                if mem.read::<i16>(state) == 6
                    && scene(mem, call, limit::SEQ_CHECK_TERM, limit::BGOOD) != 1
                {
                    return;
                }
                scene(mem, call, limit::SEQ_STOP, limit::BTIDU);
                mem.write(state, 9u16);
            }
            7 => {
                call(mem, limit::TIMER_STOP2, &[]);
                mem.write(result, 0u8);
                mem.write(state, 8u16);
            }
            9 => next_state(mem, state),
            10 => {
                let r = i32::from(mem.read::<i8>(result)).cast_unsigned();
                limit_set(mem, call, 0, r);
                call(mem, limit::INIT_TIMER, &[]);
                mem.write(state, 0u16);
            }
            _ => {}
        }
    }

    /// Tidus's state 4 (see [`ctrl_tidus_limit_window`]).
    fn tidus_bar<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        state: u32,
    ) {
        call(mem, limit::TIMER_START, &[]);
        let held = call(mem, 0x0088_8e80, &[0, 0, u32::MAX]) & 0xffff;
        let new = call(mem, 0x0088_8e80, &[0, 0, 0]) & 0xffff;
        if time_up(mem) {
            mem.write(state, 7u16);
            return;
        }
        let at = |mem: &M, o: u32| mem.rebase(0x0133_d736 + o);
        let (pos_at, step_at) = (at(mem, 0xc), at(mem, 0xe));
        let (mut pos, step, start) = if held & 0x20 != 0 || new & 0x20 != 0 {
            let p: i16 = mem.read(pos_at);
            let good = (mem.read::<i16>(at(mem, 6)), mem.read::<i16>(at(mem, 8)));
            if good.0 <= p && p <= good.1 {
                mem.write(state, 5u16);
                return;
            }
            call(mem, limit::PLAY_CLICK, &[3]);
            let step = mem.read::<i16>(step_at).wrapping_abs();
            let start: i16 = mem.read(at(mem, 0));
            mem.write(step_at, step);
            (start, step, start)
        } else {
            let step: i16 = mem.read(step_at);
            (mem.read::<i16>(pos_at), step, mem.read::<i16>(at(mem, 0)))
        };
        let turn = |mem: &mut M| {
            let s = mem.read::<i16>(step_at).wrapping_neg();
            mem.write(step_at, s);
        };
        pos = pos.wrapping_add(step);
        let length: i16 = mem.read(at(mem, 4));
        mem.write(pos_at, pos);
        if i32::from(pos) > i32::from(length) + i32::from(start) {
            pos = length.wrapping_add(start).wrapping_mul(2).wrapping_sub(pos);
            mem.write(pos_at, pos);
            turn(mem);
        }
        if pos < start {
            mem.write(pos_at, start.wrapping_mul(2).wrapping_sub(pos));
            turn(mem);
        }
    }

    /// Lulu's limit window: its state (`i16 0x0133d708`), the gauge (`i16
    /// 0x0133d71c`) and its full width (`i16 0x0133d70e`), the presses
    /// (`0x0133d71e`) and level (`0x0133d71f`, at most 16) against
    /// [`LULU_LIMIT`], the directions so far (`0x0133d721`.., their count
    /// `i8 0x0133d731`), the frame time and the last reset (`f32
    /// 0x0133c920`, `0x0133c924`), the keyboard's two buttons (`0x0133c928`,
    /// `0x0133c92c`), keyboard input on (`0x0133c930`) and the control mode
    /// (`0x00c58ef0`, 0 the keyboard).
    pub const LULU_STATE: u32 = 0x0133_d708;
    mod lulu {
        pub const GAUGE: u32 = 0x0133_d71c;
        pub const WIDTH: u32 = 0x0133_d70e;
        pub const PRESSES: u32 = 0x0133_d71e;
        pub const LEVEL: u32 = 0x0133_d71f;
        pub const DIRS: u32 = 0x0133_d721;
        pub const COUNT: u32 = 0x0133_d731;
        pub const NOW: u32 = 0x0133_c920;
        pub const RESET: u32 = 0x0133_c924;
        pub const KEY_A: u32 = 0x0133_c928;
        pub const KEY_B: u32 = 0x0133_c92c;
        pub const KEYBOARD: u32 = 0x0133_c930;
        pub const CONTROL_MODE: u32 = 0x00c5_8ef0;
        /// The circles a full turn makes, starting from each direction
        /// (4..=7; the original's stack bytes, read at the first direction's
        /// offset): turning one way, the other.
        pub const TURNS: [u8; 16] = [7, 6, 5, 4, 7, 6, 5, 4, 4, 5, 6, 7, 4, 5, 6, 7];
        /// The time between turns that resets the keyboard's buttons
        /// (`f32 0x00b5edc8`, about 0.5001).
        pub const RESET_TIME: u32 = 0x3f00_068e;
    }

    /// `TOBtlCtrlLuluLimitWindow()` (`0x00891b80`): Lulu's limit (Fury)
    /// window. Every frame the time (`graphicGetTime`, through `fcall`) goes
    /// to `0x0133c920` (and to the reset time when that is 0). States: 1
    /// the `blulu` scene, 2 on its end `blulu2`, 3 the timer starts with
    /// keyboard input on and the control mode read
    /// (`getCurrentControlMode`), 4 (time up: 6) two passes reading the
    /// stick (`TkGetControlerAnalogHist(0, 0, axis, all)`, held then new) or
    /// the keyboard (`setLuluODKeyboardInput`): a direction 4–7 (right,
    /// down, left, up by the stick's 0x60/0xa0 thresholds; none keeps the
    /// last), the keyboard's buttons reset when idle past
    /// about 0.5 ([`lulu::RESET_TIME`]); both keyboard buttons count a press at once
    /// (a level more when `(presses + 1) / 2` reaches the level's table
    /// entry); a new direction is appended. Five directions making a full
    /// circle either way (see [`lulu::TURNS`]) start `bgoodl` and count a
    /// press (a level when the presses reach the entry); the directions
    /// shift by four. The gauge rises by 3 a frame toward `level * width /
    /// 16`, or is set to it. 6 ends keyboard input
    /// (`inputClearAllInputInfo`), stops and re-inits the timer and hands
    /// the level to `MsMenuLimitSet_007b0470(5, ..)`; 7 a frame; 8 the
    /// scenes stop; 9 done (0). Callees through `call`.
    #[allow(clippy::too_many_lines)] // one state machine, as the original
    pub fn ctrl_lulu_limit_window<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use ffx_bridge::x87;
        let r = |mem: &M, a: u32| mem.rebase(a);
        let call = &mut call;
        let now = x87::store(fcall(mem, 0x0064_1410, &[]));
        mem.write(r(mem, lulu::NOW), now);
        if mem.read::<f32>(r(mem, lulu::RESET)) == 0.0 {
            let t = x87::store(fcall(mem, 0x0064_1410, &[]));
            mem.write(r(mem, lulu::RESET), t);
        }
        let elapsed = x87::load(x87::store(x87::sub(
            x87::load(mem.read(r(mem, lulu::NOW))),
            x87::load(mem.read(r(mem, lulu::RESET))),
        )));
        let state = r(mem, LULU_STATE);
        match mem.read::<i16>(state) {
            1 => {
                scene(mem, call, limit::SEQ_START_CONTINUE, limit::BLULU);
                next_state(mem, state);
            }
            2 => {
                if scene(mem, call, limit::SEQ_CHECK_TERM, limit::BLULU) == 1 {
                    scene(mem, call, limit::SEQ_START, limit::BLULU2);
                    next_state(mem, state);
                }
            }
            3 => {
                call(mem, limit::TIMER_START, &[]);
                mem.write(r(mem, lulu::KEYBOARD), 1u32);
                let mode = call(mem, 0x0063_8e30, &[]);
                next_state(mem, state);
                mem.write(r(mem, lulu::CONTROL_MODE), mode);
            }
            4 => {
                if time_up(mem) {
                    mem.write(state, 6u16);
                    return;
                }
                lulu_turns(mem, call, elapsed);
            }
            6 => {
                if mem.read::<u32>(r(mem, lulu::KEYBOARD)) != 0 {
                    call(mem, 0x0063_0c40, &[]);
                    mem.write(r(mem, lulu::RESET), 0u32);
                    for a in [lulu::KEYBOARD, lulu::KEY_A, lulu::KEY_B] {
                        mem.write(r(mem, a), 0u32);
                    }
                }
                call(mem, limit::TIMER_STOP2, &[]);
                call(mem, limit::INIT_TIMER, &[]);
                let level = u32::from(mem.read::<u8>(r(mem, lulu::LEVEL)));
                limit_set(mem, call, 5, level);
                mem.write(state, 7u16);
            }
            7 => next_state(mem, state),
            8 => {
                scene(mem, call, limit::SEQ_STOP, limit::BLULU);
                scene(mem, call, limit::SEQ_STOP, limit::BLULU2);
                next_state(mem, state);
            }
            9 => mem.write(state, 0u16),
            _ => {}
        }
    }

    /// One press more for Lulu's level: the presses count up (to 16 levels:
    /// the level is held at 16); the level rises when `reached(presses)`
    /// meets its [`LULU_LIMIT`] entry. Returns the level.
    fn lulu_press<M: Mem>(mem: &mut M, reached: impl Fn(u8) -> u32) -> u8 {
        let at = |mem: &M, a: u32| mem.rebase(a);
        let level: u8 = mem.read(at(mem, lulu::LEVEL));
        if level >= 0x10 {
            mem.write(at(mem, lulu::LEVEL), 0x10u8);
            return 0x10;
        }
        let presses = mem.read::<u8>(at(mem, lulu::PRESSES)).wrapping_add(1);
        mem.write(at(mem, lulu::PRESSES), presses);
        let need = u32::from(mem.read::<u8>(LULU_LIMIT.ptr(mem).addr + u32::from(level)));
        if reached(presses) >= need {
            mem.write(at(mem, lulu::LEVEL), level + 1);
            return level + 1;
        }
        level
    }

    /// Lulu's gauge rises by 3 toward `level * width / 16` (signed, the
    /// width rounded toward 0), or is set to it (16-bit).
    fn lulu_gauge(mem: &mut impl Mem, level: u8) {
        let at = mem.rebase(lulu::GAUGE);
        let per = i32::from(mem.read::<i16>(mem.rebase(lulu::WIDTH))) / 16;
        let gauge: i16 = mem.read(at);
        if i32::from(gauge) < i32::from(level).wrapping_mul(per) {
            mem.write(at, gauge.wrapping_add(3));
        } else {
            #[allow(clippy::cast_possible_truncation)] // the 16-bit product
            mem.write(at, (i32::from(level).wrapping_mul(per)) as i16);
        }
    }

    /// Lulu's state 4 (see [`ctrl_lulu_limit_window`]).
    #[allow(clippy::too_many_lines)] // the two passes and the circle check
    fn lulu_turns<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        elapsed: f64,
    ) {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let (mut x, mut y, mut dir) = (0x80u8, 0x80u8, 0xffu8);
        for pass in 0..2u32 {
            if mem.read::<u32>(r(mem, lulu::CONTROL_MODE)) == 0 {
                let keys = [r(mem, lulu::KEY_A), r(mem, lulu::KEY_B)];
                call(mem, 0x0064_5eb0, &keys);
            } else {
                let all = if pass == 0 { u32::MAX } else { 0 };
                #[allow(clippy::cast_possible_truncation)] // the low byte
                {
                    x = call(mem, 0x0088_8c20, &[0, 0, 0, all]) as u8;
                    y = call(mem, 0x0088_8c20, &[0, 0, 1, all]) as u8;
                }
            }
            // right, down, left, up
            let picked = if x >= 0xa0 && y < 0x60 {
                Some(4)
            } else if x >= 0xa0 && y >= 0xa0 {
                Some(5)
            } else if x < 0x60 && y >= 0xa0 {
                Some(6)
            } else if x < 0x60 && y < 0x60 {
                Some(7)
            } else {
                None
            };
            let mut both = false;
            match picked {
                Some(d) => {
                    dir = d;
                    both = true;
                }
                None if elapsed > f64::from(f32::from_bits(lulu::RESET_TIME)) => {
                    let now: u32 = mem.read(r(mem, lulu::NOW));
                    mem.write(r(mem, lulu::KEY_A), 0u32);
                    mem.write(
                        r(mem, lulu::RESET),
                        ffx_bridge::x87::store(ffx_bridge::x87::load(now)),
                    );
                    mem.write(r(mem, lulu::KEY_B), 0u32);
                }
                None => both = true,
            }
            if both
                && mem.read::<u32>(r(mem, lulu::KEY_A)) != 0
                && mem.read::<u32>(r(mem, lulu::KEY_B)) != 0
            {
                // both keyboard buttons: a press at once
                mem.write(r(mem, lulu::KEY_A), 0u32);
                mem.write(r(mem, lulu::KEY_B), 0u32);
                let level = lulu_press(mem, |p| u32::from(p).div_ceil(2));
                lulu_gauge(mem, level);
                break;
            }
            if dir != 0xff {
                let count_at = r(mem, lulu::COUNT);
                let count: i8 = mem.read(count_at);
                if count == 0 {
                    mem.write(r(mem, lulu::DIRS), dir);
                    mem.write(count_at, 1u8);
                } else {
                    let at = r(mem, lulu::DIRS).wrapping_add(i32::from(count).cast_unsigned());
                    if i32::from(mem.read::<i8>(at.wrapping_sub(1))) != i32::from(dir) {
                        mem.write(at, dir);
                        mem.write(count_at, count.wrapping_add(1));
                    }
                }
            }
        }

        // five directions: a full circle either way
        if mem.read::<i8>(r(mem, lulu::COUNT)) >= 5 {
            let dirs = r(mem, lulu::DIRS);
            let seen: [u8; 5] = mem.read(dirs);
            // the game only stores 4..=7; the original reads its own frame
            // at any first direction, the port knows only 0..=7
            let first = usize::from(seen[0]);
            let turn = |from: usize| -> [u8; 5] {
                let t = &lulu::TURNS;
                [t[from], t[from + 1], t[from + 2], t[from + 3], t[from + 4]]
            };
            // one way from the first direction (byte offset +4), or the other (7 - it)
            if first < 8 && (seen == turn(first + 4) || seen == turn(7 - first)) {
                scene(mem, call, limit::SEQ_START, limit::BGOODL);
                lulu_press(mem, u32::from);
            }
            let rest: [u8; 12] = mem.read(dirs + 4);
            mem.write(dirs, rest);
            mem.write(r(mem, lulu::COUNT), 0u8);
        }
        let level: u8 = mem.read(r(mem, lulu::LEVEL));
        lulu_gauge(mem, level);
    }

    /// `TOBtlDrawStatusWin()` (`0x00896450`): the battle status window,
    /// one row per party slot ([`STATUS_PARTY`], 0xff skipped) at `x0 =
    /// X2(1231)`, `y0 = Y2(835 + 52 k)` in the mode of [`STATUS_MODE`]
    /// (any other draws only nothing). Every row draws the character's
    /// name (`MsGetRamChrName`, `ToMakeBtlEasyEdgeFont` at `X2(21) + x0`,
    /// `y0 - Y2(4)` in the row's HP colour `+0x19`, 0.68). Mode 3 (the
    /// limit gauges) first marks the character of the current command
    /// (`0x023cc088`) with a frame and draws its gauge (`+0x38`,
    /// `TOBtlDrawStatusLimitGauge`), then the HP and MP labels (shapes 0xe9
    /// at `X2(240)`, 0xea at `X2(420)`; their sizes and offsets differ for
    /// language 11, `TOGetFFXLang`) and the shown HP (`+0x10`) and MP
    /// (`+0x28`, colour `+0x31`) values. Modes 0 and 1 mark the characters
    /// of `0x0133d178`'s bits with the frame and draw the HP (0) or MP (1)
    /// label, the shown value, "/" and the maximum (`MsGetRamChrHPmax` /
    /// `MPmax`); mode 2 the marked frame and the row's statuses (`u16`
    /// count `+0x40`, one shown at a time from `+0x42` for 20 frames,
    /// `MsMenuGetText(1, id, 1)`, `ToMakeBtlEasyFont` at `X2(279)`, 0.78).
    /// In screen units remapped by `graphicUiRemapX2`/`Y2` (`remap(mem,
    /// y_axis, v)`); callees through `call` (`ffx.sym` addresses).
    pub struct StatusWindow<'a, M, R, C> {
        pub mem: &'a mut M,
        pub remap: R,
        pub call: C,
    }

    /// The frame shape's UVs (`f32`: 1536, 458, 2047, 510) and the
    /// current command's (983, 95, 1584, 147).
    const STATUS_FRAME_UV: [f32; 4] = [1536.0, 458.0, 2047.0, 510.0];
    const STATUS_COMMAND_FRAME_UV: [f32; 4] = [983.0, 95.0, 1584.0, 147.0];

    /// One label shape: (the shape, x at `X2(at)`, width `X2(w)`, height
    /// `Y2(h)`, the UVs, the offsets subtracted from x and y), for
    /// language 11 and for the others. The UVs (`.rdata` bits): HP 0.3613,
    /// 0.0117, 0.4443, 0.0410 and 0.3477, 0.0146, 0.4258, 0.0400; MP 0.4883,
    /// 0.0117, 0.5693, 0.0410 and 0.4980, 0.0146, 0.5566, 0.0400.
    struct Label {
        shape: u32,
        w: f32,
        h: f32,
        /// As `f32` bits.
        uv: [u32; 4],
        back: (f64, f64),
    }

    const HP_LABEL: [Label; 2] = [
        Label {
            shape: 0xe9,
            w: 68.0,
            h: 24.0,
            uv: [0x3eb9_0000, 0x3c40_0000, 0x3ee3_8000, 0x3d28_0000],
            back: (3.0, 2.0),
        },
        Label {
            shape: 0xe9,
            w: 61.5,
            h: 23.0,
            uv: [0x3eb2_0000, 0x3c70_0000, 0x3eda_0000, 0x3d24_0000],
            back: (0.0, 0.0),
        },
    ];
    const MP_LABEL: [Label; 2] = [
        Label {
            shape: 0xea,
            // 66.4 as `f32` (0x4284cccd)
            w: 66.4,
            h: 24.0,
            uv: [0x3efa_0000, 0x3c40_0000, 0x3f11_c000, 0x3d28_0000],
            back: (5.0, 2.0),
        },
        Label {
            shape: 0xea,
            w: 49.0,
            h: 23.0,
            uv: [0x3eff_0000, 0x3c70_0000, 0x3f0e_8000, 0x3d24_0000],
            back: (0.0, 0.0),
        },
    ];
    /// The "/" between a value and its maximum (`0x00b8f88c`).
    const SLASH_TEXT: u32 = 0x00b8_f88c;

    impl<M, R, C> StatusWindow<'_, M, R, C>
    where
        M: Mem,
        R: FnMut(&mut M, bool, f32) -> f32,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        fn x(&mut self, v: f32) -> f64 {
            f64::from((self.remap)(self.mem, false, v))
        }

        fn y(&mut self, v: f32) -> f64 {
            f64::from((self.remap)(self.mem, true, v))
        }

        fn run(&mut self, sym: u32, args: &[u32]) -> u32 {
            (self.call)(self.mem, sym, args)
        }

        /// `TOMkpShapeXYWHUV(shape, x, y, w, h, u0, v0, u1, v1)`.
        fn shape(&mut self, shape: u32, xywh: [f32; 4], uv: [f32; 4]) {
            let [x, y, w, h] = xywh.map(f32::to_bits);
            let [u0, v0, u1, v1] = uv.map(f32::to_bits);
            self.run(0x0090_3bb0, &[shape, x, y, w, h, u0, v0, u1, v1]);
        }

        /// A frame around the row (`-1`, `x0 - X2(65)`, `y0 - Y2(2)`,
        /// `X2(680)`, `Y2(52)`).
        #[allow(clippy::cast_possible_truncation)] // the f32 stores
        fn frame(&mut self, x0: f32, y0: f32, uv: [f32; 4]) {
            let h = self.y(52.0) as f32;
            let w = self.x(680.0) as f32;
            let y = (f64::from(y0) - self.y(2.0)) as f32;
            let x = (f64::from(x0) - self.x(65.0)) as f32;
            self.shape(u32::MAX, [x, y, w, h], uv);
        }

        /// The frame for characters marked in `0x0133d178`.
        fn marked_frame(&mut self, chr: u16, x0: f32, y0: f32) {
            let marks: u32 = self.mem.read(self.mem.rebase(0x0133_d178));
            if marks & (1u32 << (chr & 0x1f)) != 0 {
                self.frame(x0, y0, STATUS_FRAME_UV);
            }
        }

        /// The name (`ToMakeBtlEasyEdgeFont(name, X2(21) + x0, y0 - Y2(4),
        /// colour, 0.68, 1.0)`).
        #[allow(clippy::cast_possible_truncation)] // the f32 stores
        fn name(&mut self, slot: u32, row: u32, x0: f32, y0: f32) {
            let colour = u32::from(self.mem.read::<u8>(row + 0x19));
            let y = (f64::from(y0) - self.y(4.0)) as f32;
            let x = (self.x(21.0) + f64::from(x0)) as f32;
            let chr = u32::from(self.mem.read::<u16>(slot));
            let name = self.run(0x0079_af10, &[chr]);
            let args = [
                x.to_bits(),
                y.to_bits(),
                colour,
                0.68f32.to_bits(),
                1.0f32.to_bits(),
            ];
            self.run(
                0x0090_5930,
                &[name, args[0], args[1], args[2], args[3], args[4]],
            );
        }

        /// A label shape at `X2(at) + x0`, `Y2(5) + y0` (see [`Label`]).
        #[allow(clippy::cast_possible_truncation)] // the f32 stores
        fn label(&mut self, labels: &[Label; 2], at: f32, x0: f32, y0: f32) {
            let lang = self.run(0x008a_c2f0, &[]);
            let label = if lang == 0xb { &labels[0] } else { &labels[1] };
            let height = self.y(label.h) as f32;
            let width = self.x(label.w) as f32;
            let top = (self.y(5.0) + f64::from(y0) - label.back.1) as f32;
            let left = (self.x(at) + f64::from(x0) - label.back.0) as f32;
            self.shape(
                label.shape,
                [left, top, width, height],
                label.uv.map(f32::from_bits),
            );
        }

        /// A number right-aligned at `X2(at) + x0`, `Y2(-2) + y0`
        /// (`ToMakeBtlEasyEdgeDigitRight(value, x, y, colour, 0.65, 0)`).
        #[allow(clippy::cast_possible_truncation)] // the f32 stores
        fn digits(
            &mut self,
            value: impl FnOnce(&mut Self) -> u32,
            colour: u32,
            at: f32,
            x0: f32,
            y0: f32,
        ) {
            let y = (self.y(-2.0) + f64::from(y0)) as f32;
            let x = (self.x(at) + f64::from(x0)) as f32;
            let value = value(self);
            let scale = 0.65f32.to_bits();
            self.run(
                0x0090_5890,
                &[value, x.to_bits(), y.to_bits(), colour, scale, 0],
            );
        }

        /// "/" at `X2(420) + x0`, `Y2(6) + y0`, then the maximum (`max`,
        /// by character) at `X2(540)`.
        #[allow(clippy::cast_possible_truncation)] // the f32 stores
        fn slash_and_max(&mut self, max: u32, slot: u32, x0: f32, y0: f32) {
            let y = (self.y(6.0) + f64::from(y0)) as f32;
            let x = (self.x(420.0) + f64::from(x0)) as f32;
            let text = self.mem.rebase(SLASH_TEXT);
            let scale = 0.65f32.to_bits();
            self.run(0x0090_1660, &[text, x.to_bits(), y.to_bits(), 0, scale, 0]);
            self.digits(
                |w| {
                    let chr = u32::from(w.mem.read::<u16>(slot));
                    w.run(max, &[chr])
                },
                0,
                540.0,
                x0,
                y0,
            );
        }

        /// Mode 2's statuses, one at a time (see the type).
        #[allow(clippy::cast_possible_truncation)] // the f32 stores
        fn statuses(&mut self, row: u32, k: u32, x0: f32, y0: f32) {
            let count: i16 = self.mem.read(row + 0x40);
            if count == 0 {
                return;
            }
            let timer: i16 = self.mem.read(row + 0x3c);
            self.mem.write(row + 0x3c, timer.wrapping_sub(1));
            if timer <= 0 {
                let shown = self.mem.read::<i16>(row + 0x3e).wrapping_add(1);
                self.mem.write(row + 0x3e, shown);
                self.mem.write(row + 0x3c, 0x14i16);
                if self.mem.read::<i16>(row + 0x3e) >= count {
                    self.mem.write(row + 0x3e, 0i16);
                }
            }
            let y = (self.y(4.0) + f64::from(y0)) as f32;
            let x = (self.x(279.0) + f64::from(x0)) as f32;
            let shown =
                i32::from(self.mem.read::<i16>(row + 0x3e)).wrapping_add((0x48 * k).cast_signed());
            let ids = self.mem.rebase(STATUS_ROWS + 0x42);
            let id = self
                .mem
                .read::<u16>(ids.wrapping_add(shown.cast_unsigned().wrapping_mul(2)));
            let text = self.run(0x0078_fcf0, &[1, u32::from(id), 1]);
            let args = [
                x.to_bits(),
                y.to_bits(),
                0,
                0.78f32.to_bits(),
                1.0f32.to_bits(),
            ];
            self.run(
                0x0090_5ab0,
                &[text, args[0], args[1], args[2], args[3], args[4]],
            );
        }

        /// The window (see the type).
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // the f32 stores, fild
        pub fn draw(&mut self) {
            self.run(0x008f_dee0, &[0, 0, 0x200, 0x1a0]);
            let party = self.mem.rebase(STATUS_PARTY);
            let rows = self.mem.rebase(STATUS_ROWS);
            for k in 0..3u32 {
                let slot = party + 2 * k;
                let row = rows + 0x90 * k;
                let chr: u16 = self.mem.read(slot);
                if chr == 0xff {
                    continue;
                }
                let x0 = self.x(1231.0) as f32;
                let y0 = self.y((0x343 + 0x34 * k) as f32) as f32;
                let mode = self.mem.read::<i8>(self.mem.rebase(STATUS_MODE));
                match mode {
                    0 => {
                        self.marked_frame(chr, x0, y0);
                        self.name(slot, row, x0, y0);
                        self.label(&HP_LABEL, 240.0, x0, y0);
                        let colour = u32::from(self.mem.read::<u8>(row + 0x19));
                        self.digits(|w| w.mem.read(row + 0x10), colour, 395.0, x0, y0);
                        self.slash_and_max(0x0079_adf0, slot, x0, y0);
                    }
                    1 => {
                        self.marked_frame(chr, x0, y0);
                        self.name(slot, row, x0, y0);
                        self.label(&MP_LABEL, 240.0, x0, y0);
                        let colour = u32::from(self.mem.read::<u8>(row + 0x19));
                        self.digits(|w| w.mem.read(row + 0x28), colour, 395.0, x0, y0);
                        self.slash_and_max(0x0079_ae70, slot, x0, y0);
                    }
                    2 => {
                        self.marked_frame(chr, x0, y0);
                        self.name(slot, row, x0, y0);
                        self.statuses(row, k, x0, y0);
                    }
                    3 => {
                        let current = i32::from(ACTION_PREV_VALUE.get(self.mem).cast_signed());
                        if current == i32::from(self.mem.read::<u16>(slot)) {
                            self.frame(x0, y0, STATUS_COMMAND_FRAME_UV);
                        }
                        // the limit gauge
                        let h = self.y(8.0) as f32;
                        let w = self.x(520.0) as f32;
                        let y = (self.y(35.0) + f64::from(y0)) as f32;
                        let x = (self.x(21.0) + f64::from(x0)) as f32;
                        let bar: u32 = self.mem.read(row + 0x38);
                        let xywh = [x, y, w, h].map(f32::to_bits);
                        self.run(0x0089_60e0, &[bar, xywh[0], xywh[1], xywh[2], xywh[3]]);
                        self.name(slot, row, x0, y0);
                        self.label(&HP_LABEL, 240.0, x0, y0);
                        let colour = u32::from(self.mem.read::<u8>(row + 0x19));
                        self.digits(|w| w.mem.read(row + 0x10), colour, 395.0, x0, y0);
                        self.label(&MP_LABEL, 420.0, x0, y0);
                        let colour = u32::from(self.mem.read::<u8>(row + 0x31));
                        self.digits(|w| w.mem.read(row + 0x28), colour, 540.0, x0, y0);
                    }
                    _ => {}
                }
            }
        }
    }

    /// The scratch [`WeaponAbilityWindow::draw`] needs: the getters' second
    /// results and the measured width.
    pub const WEAPON_ABILITY_WORK: u32 = 8;

    /// `TOBtlDrawWeaponAbilityWindow(weapon, x, y)` (`0x008f34c0`): the
    /// abilities of a weapon (`MsGetSaveWeapon(weapon, &_)`, after
    /// `MsCalcWeaponCap`; nothing without abilities: `+0xb` the count,
    /// `u16`s from `+0xe`, 0xff an empty slot). The full-screen scissor;
    /// the window is as wide as `X2(90)` plus the widest name
    /// (`ToGetBtlEasyFontWidth`, 0.78, kept as a truncated `u16`) or at
    /// least `ftol(X2(300))`, `Y2(52 n - 2)` high, raised by `Y2(52 (n -
    /// 3))` past three rows (`TODrawBtlNetWindowXYWHType(x, y, w, h, 0)`).
    /// Each row (`Y2(3 + 52 i)` from `y`): an empty slot "-" text
    /// (`MsMenuGetText(1, 0x1005, 0)`) at `X2(16)`, an ability its icon
    /// (`DrawCrossMenuIconXYWHRGBA` at `X2(11)`, `Y2(row + 4)`, `X2(29)` by
    /// `Y2(36)`, the record's `+0x68`, grey 0x80) and name (`MsGetRomAbility`
    /// text offset plus its base) at `X2(45)`; both with `ToMakeBtlEasyFont`
    /// at 0.78. In screen units remapped by `graphicUiRemapX2`/`Y2`
    /// (`remap(mem, y_axis, v)`); callees through `call` (`ffx.sym`
    /// addresses); `work` holds [`WEAPON_ABILITY_WORK`] bytes.
    pub struct WeaponAbilityWindow<'a, M, R, C> {
        pub mem: &'a mut M,
        pub remap: R,
        pub call: C,
        pub work: u32,
    }

    impl<M, R, C> WeaponAbilityWindow<'_, M, R, C>
    where
        M: Mem,
        R: FnMut(&mut M, bool, f32) -> f32,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        fn x(&mut self, v: f32) -> f64 {
            f64::from((self.remap)(self.mem, false, v))
        }

        fn y(&mut self, v: f32) -> f64 {
            f64::from((self.remap)(self.mem, true, v))
        }

        fn run(&mut self, sym: u32, args: &[u32]) -> u32 {
            (self.call)(self.mem, sym, args)
        }

        /// An ability's name: its record (`MsGetRomAbility(id, &base)`) and
        /// the text (`u16 +0` plus the base).
        fn ability(&mut self, id: u16) -> (u32, u32) {
            let record = self.run(0x0079_0970, &[u32::from(id), self.work]);
            let base: u32 = self.mem.read(self.work);
            let text = u32::from(self.mem.read::<u16>(record)).wrapping_add(base);
            (record, text)
        }

        /// `ToMakeBtlEasyFont(text, x, y, 0, 0.78, 1.0)`.
        fn text(&mut self, text: u32, x: f32, y: f32) {
            let scale = 0.78f32.to_bits();
            let one = 1.0f32.to_bits();
            self.run(
                0x0090_5ab0,
                &[text, x.to_bits(), y.to_bits(), 0, scale, one],
            );
        }

        /// The window (see the type).
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss,
            clippy::many_single_char_names
        )] // f32 stores, fild; x, y as the original's
        pub fn draw(&mut self, weapon: u32, x: f32, y: f32) {
            use ffx_bridge::x87::ftol;
            let rec = self.run(0x007a_bbd0, &[weapon, self.work]);
            self.run(0x0079_8eb0, &[rec]);
            let count = |s: &Self| u32::from(s.mem.read::<u8>(rec + 0xb));
            if count(self) == 0 {
                return;
            }
            self.run(0x008f_9230, &[0, 0, 0x200, 0x1a0]);

            // the widest name
            let mut widest: u32 = 0;
            let mut i = 0;
            while i < count(self) {
                let id: u16 = self.mem.read(rec + 0xe + 2 * i);
                if id != 0xff {
                    let (_, text) = self.ability(id);
                    let width_at = self.work + 4;
                    let args = [text, width_at, 0, 0.78f32.to_bits(), 1.0f32.to_bits()];
                    self.run(0x0090_5290, &args);
                    let width = f64::from(self.mem.read::<f32>(width_at));
                    #[allow(clippy::cast_possible_truncation)] // the low half
                    let shown = f64::from(widest as i16);
                    if width > shown {
                        widest = ftol(self.mem, width) & 0xffff;
                    }
                }
                i += 1;
            }
            #[allow(clippy::cast_possible_truncation)] // the low half
            let least = self.x(300.0);
            let names = if least > f64::from(widest as i16) {
                let again = self.x(300.0);
                ftol(self.mem, again) & 0xffff
            } else {
                widest
            };

            // the window, raised past three rows
            let n = count(self);
            let mut y = y;
            if n > 3 {
                let up = self.y(((n - 3) * 0x34) as f32);
                y = (f64::from(y) - up) as f32;
            }
            let h = self.y((n * 0x34 - 2) as f32) as f32;
            #[allow(clippy::cast_possible_truncation)] // the low half
            let width = (self.x(90.0) + f64::from(names as i16)) as f32;
            let net = [x, y, width, h].map(f32::to_bits);
            self.run(0x008f_41b0, &[net[0], net[1], net[2], net[3], 0]);

            // the rows
            let mut i = 0;
            let mut row = 3u32;
            while i < count(self) {
                let id: u16 = self.mem.read(rec + 0xe + 2 * i);
                if id == 0xff {
                    let ty = (self.y(row as f32) + f64::from(y)) as f32;
                    let tx = (self.x(16.0) + f64::from(x)) as f32;
                    let text = self.run(0x0078_fcf0, &[1, 0x1005, 0]);
                    self.text(text, tx, ty);
                } else {
                    let (record, text) = self.ability(id);
                    let icon = u32::from(self.mem.read::<u8>(record + 0x68));
                    let ih = self.y(36.0) as f32;
                    let iw = self.x(29.0) as f32;
                    let iy = (self.y((row + 4) as f32) + f64::from(y)) as f32;
                    let ix = (self.x(11.0) + f64::from(x)) as f32;
                    let [ix, iy, iw, ih] = [ix, iy, iw, ih].map(f32::to_bits);
                    self.run(0x008e_6af0, &[ix, iy, iw, ih, icon, 0x80, 0x80, 0x80, 0x80]);
                    let ty = (self.y(row as f32) + f64::from(y)) as f32;
                    let tx = (self.x(45.0) + f64::from(x)) as f32;
                    self.text(text, tx, ty);
                }
                i += 1;
                row += 0x34;
            }
        }
    }

    /// `TOBtlDrawCtbWindow(x, y)` (`0x008a0590`): the battle turn-order
    /// (CTB) window, unless `0x0133d6e0` is 1. In screen units remapped by
    /// `graphicUiRemapX2`/`Y2` (`remap(mem, y_axis, v)`, an `f32`) it spans
    /// left `X2(1637)`, top `Y2(194)`, right `X2(1765)`, bottom `Y2(482)`:
    ///
    /// - the full-screen scissor, the net window around it (8 out, 16
    ///   wider and taller), the scroll arrows (`DrawCrossCtbIcon` 0x1b6
    ///   while the scroll `i8 0x01340446` is positive, 0x1b7 while below
    ///   10), the scroll bar and the window's own scissor;
    /// - each of the 32 entries at `0x01340478` (record pointers, null for
    ///   none) not sliding (`+0x14` not `TOBtlCtbSlideStep_008a2b10`):
    ///   `TODrawBtlCtbParts(i8 +1, +4, x, y)` at `X2(15) + left + i16
    ///   +0x10` and `Y2(i16 +0x12 / 22 * 48 + scroll row * 48 / 22) + top`
    ///   (scroll row `i16 0x01340448`), the first, unscrolled one with the
    ///   highlight shape 0x1d8 first;
    /// - when the command window at `g_BtlActionDepth` is open (byte
    ///   `0x0133c952 + 0xf0 * depth` 1): a packet scissor and, for each of
    ///   the first 16 rows whose entry's character bit (`+1`) is set in
    ///   `0x01340470`, the cursor shape 0x1d6 at `ftol(x + left + 8)`, `y`
    ///   and the target frame 0x190.
    ///
    /// Ends with the full-screen scissor again and returns its `eax`
    /// (without drawing, the original leaves `eax` as it was: 0 here).
    /// Callees through `call` (`ffx.sym` addresses).
    pub struct CtbWindow<'a, M, R, C> {
        pub mem: &'a mut M,
        pub remap: R,
        pub call: C,
    }

    /// Shape UVs of the CTB target frame 0x190 (`u0, v0, u1, v1` as `f32`
    /// bits: 0.5527, 0.5107, 0.6191, 0.5547 and 0.5527, 0.4521, 0.6191,
    /// 0.4961): rows of characters (below 0x14) and of monsters.
    const CTB_FRAME_UV_CHARACTER: [u32; 4] = [0x3f0d_8000, 0x3f02_c000, 0x3f1e_8000, 0x3f0e_0000];
    const CTB_FRAME_UV_MONSTER: [u32; 4] = [0x3f0d_8000, 0x3ee7_8000, 0x3f1e_8000, 0x3efe_0000];
    /// Shape UVs of the CTB highlight 0x1d8 (`f32` bits: 0.7070, 0.2891,
    /// 0.7393, 0.3350).
    const CTB_HIGHLIGHT_UV: [u32; 4] = [0x3f35_0000, 0x3e94_0000, 0x3f3d_4000, 0x3eab_8000];
    /// The shape colour of the target frame (twice).
    const CTB_FRAME_COLOUR: u32 = 0x4d80_8080;

    impl<M, R, C> CtbWindow<'_, M, R, C>
    where
        M: Mem,
        R: FnMut(&mut M, bool, f32) -> f32,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        fn x(&mut self, v: f32) -> f64 {
            f64::from((self.remap)(self.mem, false, v))
        }

        fn y(&mut self, v: f32) -> f64 {
            f64::from((self.remap)(self.mem, true, v))
        }

        fn run(&mut self, sym: u32, args: &[u32]) -> u32 {
            (self.call)(self.mem, sym, args)
        }

        fn scroll(&self) -> i8 {
            self.mem.read(self.mem.rebase(0x0134_0446))
        }

        fn scroll_row(&self) -> i16 {
            self.mem.read(self.mem.rebase(0x0134_0448))
        }

        fn entry(&self, k: u32) -> u32 {
            self.mem.read(self.mem.rebase(0x0134_0478) + 4 * k)
        }

        /// The full-screen scissor (`TODrawScissorXYWH(0, 0, 512, 416)`).
        fn full_scissor(&mut self) -> u32 {
            self.run(0x008f_9230, &[0, 0, 0x200, 0x1a0])
        }

        /// The window (see the type).
        #[allow(
            clippy::cast_possible_truncation,
            clippy::too_many_lines,
            clippy::many_single_char_names
        )]
        pub fn draw(&mut self, x: i32, y: u32) -> u32 {
            let hidden: u32 = self.mem.read(self.mem.rebase(0x0133_d6e0));
            if hidden == 1 {
                return 0;
            }
            let left = self.x(1637.0) as f32;
            let top = self.y(194.0) as f32;
            let right = self.x(1765.0) as f32;
            let bottom = self.y(482.0) as f32;
            let height = (f64::from(bottom) - f64::from(top)) as f32;
            self.full_scissor();

            // the net window: 8 out, 16 wider and taller
            let net_h = (self.y(16.0) + f64::from(height)) as f32;
            let width = (f64::from(right) - f64::from(left)) as f32;
            let net_w = (self.x(16.0) + f64::from(width)) as f32;
            let net_y = (f64::from(top) - self.y(8.0)) as f32;
            let net_x = (f64::from(left) - self.x(8.0)) as f32;
            let net = [net_x, net_y, net_w, net_h].map(f32::to_bits);
            self.run(0x008f_41b0, &[net[0], net[1], net[2], net[3], 0]);

            // the scroll arrows
            let mut scroll = self.scroll();
            if scroll > 0 {
                let ay = (f64::from(top) - self.y(10.0)) as f32;
                let ax = (self.x(44.0) + f64::from(left)) as f32;
                self.run(0x0089_e660, &[0x1b6, ax.to_bits(), ay.to_bits(), 0, 0]);
                scroll = self.scroll();
            }
            if scroll < 10 {
                let ay = (self.y(10.0) + f64::from(bottom)) as f32;
                let ax = (self.x(44.0) + f64::from(left)) as f32;
                self.run(0x0089_e660, &[0x1b7, ax.to_bits(), ay.to_bits(), 0, 0]);
                scroll = self.scroll();
            }
            let bar = self.x(8.0) as f32;
            let scroll_args = [
                left.to_bits(),
                top.to_bits(),
                bar.to_bits(),
                height.to_bits(),
                i32::from(scroll).cast_unsigned(),
                6,
                0x10,
            ];
            self.run(0x008e_6cc0, &scroll_args);
            let window = [left, top, right, bottom].map(f32::to_bits);
            self.run(0x008e_8980, &window);

            // the entries
            let sliding = self.mem.rebase(0x008a_2b10);
            for k in 0..32 {
                let entry = self.entry(k);
                if entry == 0 {
                    continue;
                }
                let ex = f64::from(self.mem.read::<i16>(entry + 0x10));
                let ex = ((ex + f64::from(left)) + self.x(15.0)) as f32;
                let ey = f64::from(self.mem.read::<i16>(entry + 0x12));
                let row = f64::from(self.scroll_row());
                let at = (ey / 22.0 * 48.0 + 48.0 * row / 22.0) as f32;
                let ey = (self.y(at) + f64::from(top)) as f32;
                let entry = self.entry(k);
                if self.mem.read::<u32>(entry + 0x14) == sliding {
                    continue;
                }
                if k == 0 && self.scroll_row() == 0 {
                    let h = self.y(33.0) as f32;
                    let w = self.x(23.0) as f32;
                    let sy = (self.y(7.0) + f64::from(top)) as f32;
                    let sx = (f64::from(right) - self.x(23.0)) as f32;
                    let [u0, v0, u1, v1] = CTB_HIGHLIGHT_UV;
                    let args = [
                        0x1d8,
                        sx.to_bits(),
                        sy.to_bits(),
                        w.to_bits(),
                        h.to_bits(),
                        u0,
                        v0,
                        u1,
                        v1,
                    ];
                    self.run(0x0090_3bb0, &args);
                }
                let entry = self.entry(k);
                let chr = i32::from(self.mem.read::<i8>(entry + 1)).cast_unsigned();
                let icon: u32 = self.mem.read(entry + 4);
                self.run(0x008a_16d0, &[chr, icon, ex.to_bits(), ey.to_bits()]);
            }

            // the targets of the open command window
            let depth = i32::from(self.mem.read::<i8>(self.mem.rebase(0x023c_c092)));
            let open = self
                .mem
                .rebase(0x0133_c952)
                .wrapping_add((depth * 0xf0).cast_unsigned());
            if self.mem.read::<u8>(open) == 1 {
                use ffx_bridge::x87::ftol;
                let b = ftol(self.mem, f64::from(bottom));
                let r = ftol(self.mem, f64::from(right));
                let t = ftol(self.mem, f64::from(top));
                let l = ftol(self.mem, f64::from(left) - 30.0);
                self.run(0x008f_dee0, &[l, t, r, b]);
                #[allow(clippy::cast_precision_loss)] // as the original's fild
                let fx = x as f32;
                for k in 0..16u32 {
                    let row = f64::from(self.scroll_row()) * 48.0 / 22.0;
                    #[allow(clippy::cast_precision_loss)]
                    let at = (row + f64::from((0x30 * k) as f32)) as f32;
                    let ry = (self.y(at) + f64::from(top)) as f32;
                    let entry = self.entry(k);
                    let chr: u8 = self.mem.read(entry + 1);
                    let targets: u32 = self.mem.read(self.mem.rebase(0x0134_0470));
                    if targets & (1u32 << (chr & 0x1f)) == 0 {
                        continue;
                    }
                    let cx = ftol(self.mem, f64::from(fx) + f64::from(left) + 8.0);
                    self.run(0x0090_3a10, &[0x1d6, cx, y]);
                    let entry = self.entry(k);
                    let uv = if self.mem.read::<i8>(entry + 1) < 0x14 {
                        CTB_FRAME_UV_CHARACTER
                    } else {
                        CTB_FRAME_UV_MONSTER
                    };
                    let h = self.y(33.0) as f32;
                    let w = self.x(54.0) as f32;
                    let fy = (self.y(6.0) + f64::from(ry)) as f32;
                    let fx2 = (f64::from(left) - self.x(30.0)) as f32;
                    let [u0, v0, u1, v1] = uv;
                    let args = [
                        0x190,
                        fx2.to_bits(),
                        fy.to_bits(),
                        w.to_bits(),
                        h.to_bits(),
                        u0,
                        v0,
                        u1,
                        v1,
                        CTB_FRAME_COLOUR,
                        CTB_FRAME_COLOUR,
                    ];
                    self.run(0x0090_3ee0, &args);
                }
            }
            self.full_scissor()
        }
    }
}
