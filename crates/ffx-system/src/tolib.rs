//! Small pieces of the `TO*` library that decide what the menus, windows
//! and message text show: name lookups through the kernel and battle
//! tables, message-code counting and skipping, window scroll steps, slot
//! state tests and the menu's bookkeeping words. Functions of other
//! modules they call are passed in as callbacks.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::msg::bridge::skip_code;
    use crate::window::bridge::MSG_WINDOWS;

    /// The flag `0x009047a0` sets (`0x018716b0`).
    pub const PKT_SIDE: Global<u32> = Global::new(0x0187_16b0);

    /// `0x009047a0(x)`: for `x` 1 or 2, [`PKT_SIDE`] = `x - 1`. Returns
    /// what `eax` ends as (`x - 1` then, else `x`).
    pub fn set_pkt_side(mem: &mut impl Mem, x: u32) -> u32 {
        if x == 1 || x == 2 {
            PKT_SIDE.set(mem, x - 1);
            return x - 1;
        }
        x
    }

    /// The menu's script-request flag (`0x0186a218`).
    pub const MENU_ATEL_ACTIVE: Global<u8> = Global::new(0x0186_a218);

    /// `0x008be350()`: with [`MENU_ATEL_ACTIVE`] set, controller 6 removed
    /// (`AtelRemoveCtrlWork(6)`, the script module's) and the flag
    /// cleared.
    pub fn menu_atel_release<M: Mem>(mem: &mut M, remove: impl FnOnce(&mut M, u32)) {
        if MENU_ATEL_ACTIVE.get(mem) != 0 {
            remove(mem, 6);
            MENU_ATEL_ACTIVE.set(mem, 0);
        }
    }

    /// While 0, `TOBtlPadScrollDown` records the scroll start
    /// (`0x0133c938`).
    pub const SCROLL_LOCK: Global<u32> = Global::new(0x0133_c938);
    /// The recorded scroll start (`0x0133f7a8`).
    pub const SCROLL_START: Global<i32> = Global::new(0x0133_f7a8);
    /// The empty step function the scrolls install (`0x00898e40`, `ret`).
    pub const SCROLL_DONE: u32 = 0x0089_8e40;

    /// `TOBtlPadScrollUp(w)` (`0x00898e20`) and `TOBtlPadScrollDown(w)`
    /// (`0x00898df0`): the window's offset (`+0x3c`, `i16`) moved by one
    /// line (`height`, `TOGetLineFeedHeight`), up or down, and its step
    /// function (`+0x88`) set to [`SCROLL_DONE`]; down first records the
    /// offset in [`SCROLL_START`] unless [`SCROLL_LOCK`] is set. Returns
    /// `height`.
    pub fn btl_pad_scroll(mem: &mut impl Mem, w: u32, height: u32, down: bool) -> u32 {
        let off = w.wrapping_add(0x3c);
        let y: i16 = mem.read(off);
        #[allow(clippy::cast_possible_truncation)] // `add/sub word, ax`
        let h = height as i16;
        if down {
            if SCROLL_LOCK.get(mem) == 0 {
                SCROLL_START.set(mem, i32::from(y));
            }
            mem.write(off, y.wrapping_add(h));
        } else {
            mem.write(off, y.wrapping_sub(h));
        }
        let done = mem.rebase(SCROLL_DONE);
        mem.write(w.wrapping_add(0x88), done);
        height
    }

    /// `0x008a2ba0(p)`: a four-frame counter (`+0xf`, `i8`); on wrapping
    /// to 0 the step function (`+0x14`) becomes `0x008a2bc0`. Returns 0.
    pub fn tick4(mem: &mut impl Mem, p: u32) -> u32 {
        let at = p.wrapping_add(0xf);
        let n = mem.read::<i8>(at).wrapping_add(1);
        mem.write(at, n);
        if n >= 4 {
            mem.write(at, 0u8);
            let f = mem.rebase(0x008a_2bc0);
            mem.write(p.wrapping_add(0x14), f);
        }
        0
    }

    /// `0x008dfe70(w)`: state (`+0x2c`, `u16`) 4 and the step function
    /// (`+0x124`) itself. Returns 0.
    pub fn message_hold(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(0x2c), 4u16);
        let f = mem.rebase(0x008d_fe70);
        mem.write(w.wrapping_add(0x124), f);
        0
    }

    /// The menu pad history (`0x025d09c0`, 0xc0 bytes).
    pub const MENU_PAD_DATA: Global<[u8; 0xc0]> = Global::new(0x025d_09c0);
    /// Its four analog bytes (`0x025d09de`), centred at `0x80`.
    pub const MENU_PAD_ANALOG: Global<u32> = Global::new(0x025d_09de);

    /// `TOInitPadData()` (`0x008adbd0`) and `TOMenuClearControlPadBuff()`
    /// (`0x008be3d0`): the history zeroed (`memset`), the analog bytes
    /// centred. Returns the history's address (`memset`'s result).
    pub fn init_pad_data(mem: &mut impl Mem) -> u32 {
        MENU_PAD_DATA.set(mem, [0; 0xc0]);
        MENU_PAD_ANALOG.set(mem, 0x8080_8080);
        MENU_PAD_DATA.ptr(mem).addr
    }

    /// The menu's draw-buffer word (`0x01841c60`).
    pub const MENU_DBUFF: Global<u32> = Global::new(0x0184_1c60);

    /// `TOMenuGetDBuffDC(&out)` (`0x008a8e30`): `Sg_GetDBuffDC(0)` (passed
    /// in), its result returned; a non-null `out` gets [`MENU_DBUFF`].
    pub fn menu_get_dbuff_dc<M: Mem>(
        mem: &mut M,
        out: u32,
        get: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let r = get(mem, 0);
        if out != 0 {
            let v = MENU_DBUFF.get(mem);
            mem.write(out, v);
        }
        r
    }

    /// The menu graph's draw context (`0x0186a20c`) and its buffer
    /// (`0x0186a210`).
    pub const MENU_GRAPH: Global<u32> = Global::new(0x0186_a20c);

    /// `TOInitMenuGraph()` (`0x008be330`): [`MENU_GRAPH`] =
    /// `Sg_GetDBuffDC(0x0186a210)`; returns it.
    pub fn init_menu_graph<M: Mem>(mem: &mut M, get: impl FnOnce(&mut M, u32) -> u32) -> u32 {
        let buf = mem.rebase(0x0186_a210);
        let r = get(mem, buf);
        MENU_GRAPH.set(mem, r);
        r
    }

    /// `TOMenuOpenPktBuffTmp()` (`0x008bef30`): the packet buffers at
    /// `0x015c0870` (`TOSetPktBuffAdrs(base, 0x80000, 0x75300)`), then (a
    /// tail call) `TOOpenPktbuff()`, passed in.
    pub fn menu_open_pkt_buff_tmp<M: Mem>(mem: &mut M, open: impl FnOnce(&mut M) -> u32) -> u32 {
        let base = mem.rebase(0x015c_0870);
        crate::pkt::bridge::set_pkt_buff(mem, base, 0x8_0000, 0x7_5300);
        open(mem)
    }

    /// The help window `0x008b3790` closes (`0x0186625c`).
    pub const HELP_WINDOW: Global<u32> = Global::new(0x0186_625c);

    /// `0x008b3790()`: with a window open, `+0x41` = 1 and the pointer
    /// cleared. Returns the window (0 with none).
    pub fn close_help_window(mem: &mut impl Mem) -> u32 {
        let w = HELP_WINDOW.get(mem);
        if w != 0 {
            mem.write(w.wrapping_add(0x41), 1u8);
            HELP_WINDOW.set(mem, 0);
        }
        w
    }

    /// A name string from a kernel or battle table: `get(id, &strings)`
    /// returns the record, whose `u16` at `field` is an offset into
    /// `strings`. `0x008b8d70` (`MsGetComData`, field 0), `0x008b8d90`
    /// (`MsGetRomAbility`, 0); with `hiragana` (`MsGetSaveConfigHiragana`
    /// non-zero) the next word's offset: `0x008bebb0`, `0x008beb70`
    /// (`MsGetRomPlyCommand`, 0 and 8), `0x008bec30`, `0x008bebf0`
    /// (`MsImportantPtr`, 0 and 8).
    pub fn table_name<M: Mem>(
        mem: &mut M,
        id: u32,
        field: u32,
        tmp: u32,
        get: impl FnOnce(&mut M, u32, u32) -> u32,
    ) -> u32 {
        let rec = get(mem, id, tmp);
        let off = u32::from(mem.read::<u16>(rec.wrapping_add(field)));
        off.wrapping_add(mem.read(tmp))
    }

    /// The save slots' "show all states" flag (`0x0186628a`).
    pub const SLOTS_ALL: Global<u8> = Global::new(0x0186_628a);
    /// `g_TkSaveSlots` (`0x01866290`).
    pub const SAVE_SLOTS: Global<u8> = Global::new(0x0186_6290);

    /// `0x008b3730(_, i)`: whether save slot `i` is shown: state 1, or with
    /// [`SLOTS_ALL`] any state up to 2 or 4.
    pub fn save_slot_shown(mem: &impl Mem, i: u32) -> u32 {
        let s: u8 = mem.read(SAVE_SLOTS.ptr(mem).addr.wrapping_add(i));
        let shown = if SLOTS_ALL.get(mem) != 0 {
            s <= 2 || s == 4
        } else {
            s == 1
        };
        u32::from(shown)
    }

    /// `0x008b7f80(p)`: the line breaks of message `p`: its `0x10` codes
    /// not followed by `0xff`, counted while stepping code by code to the
    /// terminator.
    pub fn count_line_breaks(mem: &impl Mem, mut p: u32) -> u32 {
        let mut n = 0;
        while mem.read::<u8>(p) != 0 {
            if mem.read::<u8>(p) == 0x10 && mem.read::<u8>(p.wrapping_add(1)) != 0xff {
                n += 1;
            }
            p = skip_code(mem, p);
        }
        n
    }

    /// `0x008df690(p, n)`: `p` advanced code by code until `n` counted
    /// codes are passed (bytes up to 2 and from 0x26 count); `p` for `n`
    /// 0.
    pub fn skip_counted_codes(mem: &impl Mem, mut p: u32, mut n: u32) -> u32 {
        if n == 0 {
            return p;
        }
        loop {
            let c: u8 = mem.read(p);
            if c <= 2 {
                n = n.wrapping_sub(1);
            }
            if c >= 0x26 {
                n = n.wrapping_sub(1);
            }
            p = skip_code(mem, p);
            if n == 0 {
                return p;
            }
        }
    }

    /// `0x008b89a0(i, text)`: message window `i`'s text (`+0x94`) and its
    /// first shown position (`+0x98`, `TOSkipCodesAbove2`); returns the
    /// latter.
    pub fn set_message_text(mem: &mut impl Mem, i: i32, text: u32) -> u32 {
        let at = MSG_WINDOWS
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(4));
        let w: u32 = mem.read(at);
        mem.write(w.wrapping_add(0x94), text);
        let p = crate::msg::bridge::skip_codes_above_2(mem, text);
        let w: u32 = mem.read(at);
        mem.write(w.wrapping_add(0x98), p);
        p
    }

    /// `0x00890e30(tag, end, flags)`: a 16-byte header at `tag` for the
    /// quadwords up to `end`: their count - 1 (`u16`), `0x1000` (`u16`),
    /// two zero words, and the count - 1 OR `flags`. Returns `end`.
    pub fn gif_tag(mem: &mut impl Mem, tag: u32, end: u32, flags: u32) -> u32 {
        let n = (end.wrapping_sub(tag).wrapping_add(0xf) >> 4).wrapping_sub(1);
        #[allow(clippy::cast_possible_truncation)] // `mov word`
        mem.write(tag, n as u16);
        mem.write(tag.wrapping_add(2), 0x1000u16);
        mem.write(tag.wrapping_add(4), [0u32, 0, n | flags]);
        end
    }

    /// `0x008a8f00(bits, i)`: bit `i` of a `u16` bit array (signed index
    /// arithmetic, as compiled); 0 or 1.
    pub fn bit16(mem: &impl Mem, bits: u32, i: i32) -> u32 {
        let word = i / 16;
        let bit = i % 16;
        let w = u32::from(mem.read::<u16>(bits.wrapping_add(word.cast_unsigned().wrapping_mul(2))));
        u32::from(w & 1u32.wrapping_shl(bit.cast_unsigned()) != 0)
    }

    /// The message variables (`g_MsgVars`, `0x01869ab0`): 32 pointers per
    /// window to records whose byte 0 is the type and byte 1 its copy.
    pub const MSG_VARS: Global<u32> = Global::new(0x0186_9ab0);

    fn copy_var_types(mem: &mut impl Mem, i: i32) -> (u32, u8) {
        let base = MSG_VARS
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_shl(7));
        let mut last = 0;
        for k in 0..32 {
            let r: u32 = mem.read(base.wrapping_add(4 * k));
            last = mem.read::<u8>(r);
            mem.write(r.wrapping_add(1), last);
        }
        (base, last)
    }

    /// `0x008b8680(i)`: window `i`'s 32 message variables: each type byte
    /// copied to the next byte, then each type set to `0xff`. Returns the
    /// address after the window's 32 pointers.
    pub fn reset_message_vars(mem: &mut impl Mem, i: i32) -> u32 {
        let (base, _) = copy_var_types(mem, i);
        for k in 0..32 {
            let r: u32 = mem.read(base.wrapping_add(4 * k));
            mem.write(r, 0xffu8);
        }
        base.wrapping_add(0x80)
    }

    /// The per-window parameter blocks (`0x01868a90`, 0x138 bytes each).
    pub const MSG_PARAMS: Global<u32> = Global::new(0x0186_8a90);

    /// `TOInitMessage(i)` (`0x008b8210`): window `i`'s block (0x130 bytes)
    /// and parameter block (0x138) zeroed (`memset`), the window's `+0xf0`
    /// pointed at its parameters, and the 32 variable types copied to
    /// their second bytes. Returns the last type byte.
    pub fn init_message(mem: &mut impl Mem, i: i32) -> u32 {
        let off = i.cast_unsigned().wrapping_mul(4);
        let w: u32 = mem.read(MSG_WINDOWS.ptr(mem).addr.wrapping_add(off));
        mem.write(w, [0u8; 0x130]);
        let p: u32 = mem.read(MSG_PARAMS.ptr(mem).addr.wrapping_add(off));
        mem.write(p, [0u8; 0x138]);
        let w: u32 = mem.read(MSG_WINDOWS.ptr(mem).addr.wrapping_add(off));
        let p: u32 = mem.read(MSG_PARAMS.ptr(mem).addr.wrapping_add(off));
        mem.write(w.wrapping_add(0xf0), p);
        u32::from(copy_var_types(mem, i).1)
    }

    /// The macro dictionary `0x008b56d0` selects (`0x01869eb8`).
    pub const MACRO_DIC_LANG: Global<u32> = Global::new(0x0186_9eb8);

    /// `0x008b56d0()`: the macro dictionary of language `lang`
    /// (`TOGetFFXLang`): 1 us `0x00b63018`, 2 fr `0x00b63e28`, 3 es
    /// `0x00b642d8`, 4 de `0x00b634c8`, 5 it `0x00b63978`, 9 kr
    /// `0x00b650e8`, 10 ch `0x00b64788`; others leave it. Returns `lang -
    /// 1`.
    pub fn select_macro_dic(mem: &mut impl Mem, lang: u32) -> u32 {
        let d = match lang {
            1 => Some(0x00b6_3018),
            2 => Some(0x00b6_3e28),
            3 => Some(0x00b6_42d8),
            4 => Some(0x00b6_34c8),
            5 => Some(0x00b6_3978),
            9 => Some(0x00b6_50e8),
            10 => Some(0x00b6_4788),
            _ => None,
        };
        if let Some(d) = d {
            let d = mem.rebase(d);
            MACRO_DIC_LANG.set(mem, d);
        }
        lang.wrapping_sub(1)
    }

    /// `0x008b5760(p)`: whether the code at `p` joins its neighbours, for
    /// language `lang`: Japanese (0) `p[-2]` and `p[1]` ',' (0x2c) with
    /// `p[-1]` up to 0x63 and `p[2]` below it; Korean and Chinese (9, 10)
    /// `p[-2]` from 0x30 and `p[-1]`, `p[1]` in `0x5a..=0x8d`; never for
    /// the others. 1 or 0.
    pub fn joins_neighbours(mem: &impl Mem, p: u32, lang: u32) -> u32 {
        let at = |k: i32| mem.read::<u8>(p.wrapping_add(k.cast_unsigned()));
        let joins = match lang {
            0 => at(-2) == 0x2c && at(1) == 0x2c && at(-1) <= 0x63 && at(2) < 0x63,
            9 | 10 => {
                at(-2) >= 0x30 && (0x5a..=0x8d).contains(&at(-1)) && (0x5a..=0x8d).contains(&at(1))
            }
            _ => false,
        };
        u32::from(joins)
    }

    /// The battle limit timer (`0x0133f77c`): on flag (`+0`), the point
    /// (`+4`, two `u16`), a word (`+8`) and four floats (`+0xc..+0x18`).
    pub const LIMIT_TIMER: Global<[u32; 7]> = Global::new(0x0133_f77c);

    /// `TOBtlInitLimitTimer()` (`0x00897f00`): the timer's floats and word
    /// zeroed, its point read from scene 2's `0x00b5ecb4` parameter
    /// (`TOSeqGetPoint`, passed in with the two output addresses), the flag
    /// cleared; a playing sound effect `0x8000004e` stopped (`SndIsSepPlay`,
    /// `SndSepStop`, the sound module's). Returns the last call's `eax`.
    pub fn btl_init_limit_timer<M: Mem>(
        mem: &mut M,
        get_point: impl FnOnce(&mut M, u32, u32, u32, u32),
        mut sound: impl FnMut(&mut M, bool, u32) -> u32,
    ) -> u32 {
        let t = LIMIT_TIMER.ptr(mem).addr;
        for off in [0xc, 0x10, 0x14, 0x18, 8] {
            mem.write(t.wrapping_add(off), 0u32);
        }
        let name = mem.rebase(0x00b5_ecb4);
        get_point(mem, 2, name, t.wrapping_add(4), t.wrapping_add(6));
        mem.write(t, 0u32);
        let playing = sound(mem, false, 0x8000_004e);
        if playing == 0 {
            return 0;
        }
        sound(mem, true, 0x8000_004e)
    }

    /// The save/load menu's words (`0x018663c4`, `0x01866250`) and two
    /// toggles (`0x01866268`).
    pub const SAVE_MENU_WORDS: [u32; 2] = [0x0186_63c4, 0x0186_6250];
    /// Its two flag bytes.
    pub const SAVE_MENU_FLAGS: Global<[u8; 2]> = Global::new(0x0186_6268);

    /// The menu calls of the save/load menu set-up.
    pub trait SaveMenu<M> {
        /// `TkMcSetDebugLock(v)`.
        fn set_debug_lock(&mut self, mem: &mut M, v: u32);
        /// `MenuSaveLoad_ResetToggles()`.
        fn reset_toggles(&mut self, mem: &mut M);
        /// `TkMenuSetScreenX2Flag(v)`.
        fn set_screen_x2(&mut self, mem: &mut M, v: u32);
        /// `TkMn2SetMainFrameRatioDiff(v)`; its `eax`.
        fn set_frame_ratio_diff(&mut self, mem: &mut M, v: i32) -> u32;
    }

    /// `0x008b3510(menu)` (load, all slot states) and `0x008b3580(menu)`
    /// (save): the first word cleared before `TkMcSetDebugLock(1)`, the
    /// menu's `+0x1c` and the second word cleared, the two flags zeroed,
    /// the toggles reset, [`SLOTS_ALL`] = `all`, the screen-x2 flag off,
    /// the frame ratio difference -16. Returns that call's `eax`.
    pub fn save_menu_open<M: Mem>(
        mem: &mut M,
        menu: u32,
        all: bool,
        k: &mut impl SaveMenu<M>,
    ) -> u32 {
        let w0 = mem.rebase(SAVE_MENU_WORDS[0]);
        mem.write(w0, 0u32);
        k.set_debug_lock(mem, 1);
        mem.write(menu.wrapping_add(0x1c), 0u32);
        let w1 = mem.rebase(SAVE_MENU_WORDS[1]);
        mem.write(w1, 0u32);
        SAVE_MENU_FLAGS.set(mem, [0, 0]);
        k.reset_toggles(mem);
        SLOTS_ALL.set(mem, u8::from(all));
        k.set_screen_x2(mem, 0);
        k.set_frame_ratio_diff(mem, -16)
    }

    /// The script and sound calls of [`menu_music`].
    pub trait MenuMusic<M> {
        /// `AtelGetKeepPlayStatusWhiteInOut()`.
        fn keep_white_in_out(&mut self, mem: &mut M) -> u32;
        /// `AtelIsKeepPlayStatusClear11()`.
        fn keep_clear11(&mut self, mem: &mut M) -> u32;
        /// `AtelGetMusicNumber()`.
        fn music_number(&mut self, mem: &mut M) -> u32;
        /// `AtelRegistMusicNumber(n)`.
        fn regist_music_number(&mut self, mem: &mut M, n: u32);
        /// `SndGetNowPlayMusic()`.
        fn now_playing(&mut self, mem: &mut M) -> u32;
        /// `SndMusicCrossStart(n, frames)`.
        fn cross_start(&mut self, mem: &mut M, n: u32, frames: u32) -> u32;
        /// `SndMusicVolumeFadeOut(n, frames)`.
        fn fade_out(&mut self, mem: &mut M, n: u32, frames: u32) -> u32;
    }

    /// `0x008bdf60()`: the field music after the menu: nothing while the
    /// play status keeps white in/out; with status clear 11 the scene's
    /// music cross-faded in over 0x78 frames unless already playing (a
    /// negative number fades out instead); otherwise the number cleared to
    /// -1 and what plays faded out over 0x78 frames. Returns the last
    /// call's `eax`.
    pub fn menu_music<M: Mem>(mem: &mut M, k: &mut impl MenuMusic<M>) -> u32 {
        let w = k.keep_white_in_out(mem);
        if w != 0 {
            return w;
        }
        if k.keep_clear11(mem) == 0 {
            k.regist_music_number(mem, u32::MAX);
        } else if k.music_number(mem).cast_signed() >= 0 {
            let n = k.music_number(mem);
            let now = k.now_playing(mem);
            if n == now {
                return now;
            }
            let n = k.music_number(mem);
            return k.cross_start(mem, n, 0x78);
        }
        let now = k.now_playing(mem);
        k.fade_out(mem, now, 0x78)
    }

    /// The Libra window's state (`0x0133f6a8`, `i16`).
    pub const LIBRA_STATE: Global<i16> = Global::new(0x0133_f6a8);
    /// Its timer (`0x0133f702`, `u16`).
    pub const LIBRA_TIMER: Global<u16> = Global::new(0x0133_f702);
    /// Auto-close for the Libra window (`0x0133d6e0`).
    pub const LIBRA_AUTO: Global<u32> = Global::new(0x0133_d6e0);
    /// The menu pad's trigger word (`0x025d09d4`).
    pub const MENU_PAD_TRG: Global<u16> = Global::new(0x025d_09d4);

    /// `TOBtlCtrlLibraWindow()` (`0x00891a30`): the Libra window's state
    /// machine. 1: timer 0x14, next. 3, 6, 7: next. 4: with auto-close the
    /// trigger word forced to 0x20; on trigger bit 0x20 a click
    /// (`SndPlayClick(1)`, passed in) and next. 8: back to 0. Others wait.
    /// Returns what `eax` ends as.
    pub fn btl_ctrl_libra_window<M: Mem>(
        mem: &mut M,
        click: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let s = LIBRA_STATE.get(mem);
        let mut eax = i32::from(s).wrapping_sub(1).cast_unsigned();
        match s {
            1 => {
                LIBRA_TIMER.set(mem, 0x14);
                LIBRA_STATE.set(mem, s.wrapping_add(1));
                eax = 0x14;
            }
            3 | 6 | 7 => LIBRA_STATE.set(mem, s.wrapping_add(1)),
            4 => {
                if LIBRA_AUTO.get(mem) == 1 {
                    MENU_PAD_TRG.set(mem, 0x20);
                    eax = 0x20;
                }
                if MENU_PAD_TRG.get(mem) & 0x20 != 0 {
                    eax = click(mem, 1);
                    let s = LIBRA_STATE.get(mem);
                    LIBRA_STATE.set(mem, s.wrapping_add(1));
                }
            }
            8 => {
                LIBRA_STATE.set(mem, 0);
                eax = 0;
            }
            _ => {}
        }
        eax
    }

    /// The battle's per-character bytes `TOBtlInit` clears (`0x0133d770`,
    /// 18) and its 180 records of 0x24 bytes (`0x0133d788`).
    pub const BTL_CHR_BYTES: Global<[u8; 18]> = Global::new(0x0133_d770);
    /// The first record.
    pub const BTL_RECORDS: Global<[u8; 0x24]> = Global::new(0x0133_d788);

    /// `TOBtlInit()` (`0x008977f0`): the 18 bytes zeroed; each record's
    /// words `+0`..`+8` and `+0x14`..`+0x20` zeroed, `+0xc` and `+0x10`
    /// `0x00ff00ff`, and the `u16` at `+0x14` 0xff. Returns the address
    /// after the records.
    pub fn btl_init(mem: &mut impl Mem) -> u32 {
        BTL_CHR_BYTES.set(mem, [0; 18]);
        let mut r = BTL_RECORDS.ptr(mem).addr;
        for _ in 0..180 {
            mem.write(r, [0u32; 3]);
            mem.write(r.wrapping_add(0x14), [0u32; 4]);
            mem.write(r.wrapping_add(0xc), [0x00ff_00ffu32; 2]);
            mem.write(r.wrapping_add(0x14), 0xffu16);
            r = r.wrapping_add(0x24);
        }
        r.wrapping_add(0xc)
    }

    /// The menu's main windows (`0x01840900`, 16 records of 0x98 bytes):
    /// owner `+0x3f`, a free flag `+0x41`, and handlers `+0xc` (close),
    /// `+0x14` (can free), `+0x18` (free).
    pub const MAIN_WINDOWS: Global<[u8; 0x98]> = Global::new(0x0184_0900);

    /// `0x008a9230(owner)`: each main window of `owner`: its close handler
    /// called; with the free flag, freed (its free handler, else
    /// `TkMenuMainFreeWindow`, passed in) unless the can-free handler
    /// returns 0. Every handler gets the window. Returns what `eax` ends
    /// as.
    pub fn close_main_windows<M: Mem>(
        mem: &mut M,
        owner: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut free: impl FnMut(&mut M, u32) -> u32,
    ) -> u32 {
        let mut eax = 0;
        let base = MAIN_WINDOWS.ptr(mem).addr;
        for k in 0..16 {
            let w = base.wrapping_add(k * 0x98);
            eax = u32::from(mem.read::<u8>(w.wrapping_add(0x3f)));
            if eax != owner {
                continue;
            }
            let f: u32 = mem.read(w.wrapping_add(0xc));
            if f != 0 {
                eax = call(mem, f, &[w]);
            }
            if mem.read::<u8>(w.wrapping_add(0x41)) == 0 {
                continue;
            }
            let can: u32 = mem.read(w.wrapping_add(0x14));
            if can != 0 {
                eax = call(mem, can, &[w]);
                if eax == 0 {
                    continue;
                }
            }
            let f: u32 = mem.read(w.wrapping_add(0x18));
            eax = if f != 0 {
                call(mem, f, &[w])
            } else {
                free(mem, w)
            };
        }
        eax
    }

    /// The TO windows (`g_Windows`, `0x01876da8`).
    pub const WINDOWS: Global<u32> = Global::new(0x0187_6da8);

    /// `0x00906dd0(i, x, y, w, h)`: window `i`'s size (`+0x10`, `+0x14`,
    /// `u16`) and rectangle as floats (`+0x28` x, `+0x2c` y, `+0x30` x + w,
    /// `+0x34` y + h). Returns the window.
    #[allow(clippy::cast_precision_loss, clippy::many_single_char_names)] // fild, fstp dword; the rectangle
    pub fn set_window_rect(mem: &mut impl Mem, i: i32, rect: [i32; 4]) -> u32 {
        let [x, y, w, h] = rect;
        let at = WINDOWS
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(4));
        let win: u32 = mem.read(at);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // `mov word`
        {
            mem.write(win.wrapping_add(0x10), w as u16);
            mem.write(win.wrapping_add(0x14), h as u16);
        }
        mem.write(win.wrapping_add(0x28), x as f32);
        mem.write(win.wrapping_add(0x2c), y as f32);
        mem.write(win.wrapping_add(0x30), x.wrapping_add(w) as f32);
        mem.write(win.wrapping_add(0x34), y.wrapping_add(h) as f32);
        win
    }

    /// The window entries `0x00901090` moves (`0x018762e0`, 16 of 16
    /// bytes).
    pub const WINDOW_MOVES: Global<[u8; 16]> = Global::new(0x0187_62e0);

    /// `0x00901090(i, x, y, n)`: for `i` below 16 and a new position: entry
    /// `i` active (`+0` 1) at `x`, `y` (`+2`, `+4`); unless it is pinned
    /// (`+0xe` 1) its counters `+6`, `+8` = `n` and `+0xa` = `0x000a000a`.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::many_single_char_names
    )] // `mov word`; the entry's fields
    pub fn move_window_entry(mem: &mut impl Mem, i: i32, x: i32, y: i32, n: u32) {
        if i >= 16 {
            return;
        }
        let e = WINDOW_MOVES
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned() << 4);
        let (cx, cy): (i16, i16) = (mem.read(e.wrapping_add(2)), mem.read(e.wrapping_add(4)));
        if x == i32::from(cx) && y == i32::from(cy) {
            return;
        }
        mem.write(e, 1u16);
        mem.write(e.wrapping_add(2), x as u16);
        mem.write(e.wrapping_add(4), y as u16);
        if mem.read::<u16>(e.wrapping_add(0xe)) != 1 {
            mem.write(e.wrapping_add(6), [n as u16; 2]);
            mem.write(e.wrapping_add(0xa), 0x000a_000au32);
        }
    }

    /// The font scale `TOGetButtonIconWidth` doubles (`0x00c5d784`, f32).
    pub const FONT_SCALE: Global<f32> = Global::new(0x00c5_d784);

    /// `TOGetButtonIconWidth(code)` (`0x008b65e0`): the icon's base width
    /// (0x40..0x4f 14, 0x30..0x33 18, 0x34..0x37 20, 0x38..0x39 42, 0x60
    /// 55, else 0) plus twice [`FONT_SCALE`], in 53-bit precision, truncated
    /// (`_ftol2_sse`).
    pub fn button_icon_width(mem: &impl Mem, code: u32) -> u32 {
        use ffx_bridge::x87;
        let w: i32 = match code {
            0x40..=0x4f => 0xe,
            0x30..=0x33 => 0x12,
            0x34..=0x37 => 0x14,
            0x38..=0x39 => 0x2a,
            0x60 => 0x37,
            _ => 0,
        };
        let s = x87::load(FONT_SCALE.get(mem).to_bits());
        x87::ftol(mem, x87::add(x87::add(s, s), f64::from(w)))
    }

    /// `0x008adf80(i)`: all eight sequence slots of scene `i` freed (state
    /// and name cleared, whatever their state). Returns `i << 8`.
    pub fn clear_scene_slots(mem: &mut impl Mem, i: u32) -> u32 {
        let base = crate::scene::bridge::SLOTS
            .ptr(mem)
            .addr
            .wrapping_add(i << 8);
        for j in 0..8 {
            let s = base.wrapping_add(j * 0x20);
            mem.write(s, 0u16);
            mem.write(s.wrapping_add(8), 0u8);
        }
        i << 8
    }

    /// Message-window slot `i`'s part at `off` (`0x01865b3c` table).
    fn slot_part(mem: &impl Mem, i: u32, off: u32) -> u32 {
        let slots = crate::window::bridge::MSG_WINDOW_SLOTS.ptr(mem).addr;
        let s: u32 = mem.read(slots.wrapping_add(i.wrapping_mul(4)));
        mem.read(s.wrapping_add(off))
    }

    /// `0x008b0020(i, a, b, c)`: message slot `i` in mode 1 (`u16` at
    /// `+0x1c` of its `+0x14` part), its `+0x18` part's bytes `+1` 0, `+2`
    /// `a`, `+3` `b`, `+7` 0, `+6` `c`, and the `+0x14` part's handler
    /// (`+0xfc`) `0x008b64e0`. Returns the `+0x14` part.
    #[allow(clippy::many_single_char_names)] // the arguments
    pub fn message_slot_mode1(mem: &mut impl Mem, i: u32, a: u32, b: u32, c: u32) -> u32 {
        let p = slot_part(mem, i, 0x14);
        mem.write(p.wrapping_add(0x1c), 1u16);
        let q = slot_part(mem, i, 0x18);
        mem.write(q.wrapping_add(1), 0u8);
        mem.write(q.wrapping_add(2), a.to_le_bytes()[0]);
        mem.write(q.wrapping_add(3), b.to_le_bytes()[0]);
        mem.write(q.wrapping_add(7), 0u8);
        mem.write(q.wrapping_add(6), c.to_le_bytes()[0]);
        let p = slot_part(mem, i, 0x14);
        let h = mem.rebase(0x008b_64e0);
        mem.write(p.wrapping_add(0xfc), h);
        p
    }

    /// `0x008af730(i, x, y, v)`: message slot `i`'s `+0x18` part: bytes
    /// `+2`, `+3` = `x`, `y` clamped to 0..=7, words `+0x10`, `+0xc` = `v`;
    /// the `+0x14` part's handler `0x008b62b0`. Returns the `+0x14` part.
    #[allow(clippy::many_single_char_names)] // the arguments
    pub fn message_slot_cursor(mem: &mut impl Mem, i: u32, x: i32, y: i32, v: u32) -> u32 {
        let clamp = |n: i32| n.clamp(0, 7).to_le_bytes()[0];
        let q = slot_part(mem, i, 0x18);
        mem.write(q.wrapping_add(2), clamp(x));
        let q = slot_part(mem, i, 0x18);
        mem.write(q.wrapping_add(3), clamp(y));
        let q = slot_part(mem, i, 0x18);
        mem.write(q.wrapping_add(0x10), v);
        let q = slot_part(mem, i, 0x18);
        mem.write(q.wrapping_add(0xc), v);
        let p = slot_part(mem, i, 0x14);
        let h = mem.rebase(0x008b_62b0);
        mem.write(p.wrapping_add(0xfc), h);
        p
    }

    /// `0x008fd1f0(fbp, fbw, psm, fbmsk)`: at the packet cursor, a GIF tag
    /// for one A+D register and `FRAME_1` (0x4c) from the four fields
    /// (shifts 16, 8, 8, each sign-extended and OR-ed in); the cursor moves
    /// 0x20. Returns `fbp`.
    pub fn frame_packet(mem: &mut impl Mem, f: [i32; 4]) -> u32 {
        let p = crate::pkt::bridge::PACKET_CURSOR.get(mem);
        mem.write(p, [0x8001u32, 0x1000_0000, 0xe, 0]);
        crate::pkt::bridge::adp_reg(mem, p.wrapping_add(0x10), &f, &[16, 8, 8], 0x4c);
        crate::pkt::bridge::PACKET_CURSOR.set(mem, p.wrapping_add(0x20));
        f[0].cast_unsigned()
    }

    /// The menu calls of [`menu_main_window_open`].
    pub trait MainWindows<M> {
        /// `TkMenuMainAllocWindow()`.
        fn alloc(&mut self, mem: &mut M) -> u32;
        /// `TkMenuMainRegistWindow(w)`; its `eax`.
        fn regist(&mut self, mem: &mut M, w: u32) -> u32;
    }

    /// The window `0x008a8e70` opened (`0x018408fc`) and five words it
    /// clears (`0x01841c00`).
    pub const MENU_TOP_WINDOW: Global<u32> = Global::new(0x0184_08fc);
    /// The words.
    pub const MENU_TOP_STATE: Global<[u32; 5]> = Global::new(0x0184_1c00);

    /// `0x008a8e70()`: a main window allocated and set up (handlers
    /// `0x008a91d0`, `0x008a9350`, layer 6, visible, position
    /// `0x00090000`, width 0x200), kept in [`MENU_TOP_WINDOW`], five state
    /// words cleared, then registered. Returns the registration's `eax`.
    pub fn menu_main_window_open<M: Mem>(mem: &mut M, k: &mut impl MainWindows<M>) -> u32 {
        let w = k.alloc(mem);
        MENU_TOP_WINDOW.set(mem, w);
        mem.write(w.wrapping_add(8), 0u32);
        let h1 = mem.rebase(0x008a_91d0);
        let h2 = mem.rebase(0x008a_9350);
        mem.write(w.wrapping_add(0xc), [h1, h2, 0, 0, 0]);
        mem.write(w.wrapping_add(0x3e), 6u8);
        mem.write(w.wrapping_add(0x36), 1u8);
        mem.write(w.wrapping_add(0x4a), 0x0009_0000u32);
        mem.write(w.wrapping_add(0x4e), 0x200u16);
        MENU_TOP_STATE.set(mem, [0; 5]);
        k.regist(mem, w)
    }

    /// `0x00901000(i, x, y, n)`: [`move_window_entry`]'s twin: for a new
    /// position, entry `i` active at `x`, `y`; when not yet started (`+0xe`
    /// 0) the word at `+8` cleared, `+6` = `n`, `+0xc` = `n / 14 + 2`
    /// (signed), and `+0xe` 1.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::many_single_char_names
    )] // `mov word`; the entry
    pub fn start_window_entry(mem: &mut impl Mem, i: i32, x: i32, y: i32, n: i32) {
        if i >= 16 {
            return;
        }
        let e = WINDOW_MOVES
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned() << 4);
        let (cx, cy): (i16, i16) = (mem.read(e.wrapping_add(2)), mem.read(e.wrapping_add(4)));
        if x == i32::from(cx) && y == i32::from(cy) {
            return;
        }
        let started = mem.read::<u16>(e.wrapping_add(0xe)) != 0;
        mem.write(e, 1u16);
        mem.write(e.wrapping_add(2), x as u16);
        mem.write(e.wrapping_add(4), y as u16);
        if !started {
            mem.write(e.wrapping_add(8), 0u32);
            mem.write(e.wrapping_add(6), n as u16);
            mem.write(e.wrapping_add(0xc), (n / 14 + 2) as u16);
            mem.write(e.wrapping_add(0xe), 1u16);
        }
    }

    /// `TOSetExeWindowArea(bank)` (`0x00906ce0`): [`WINDOWS`]' eight
    /// pointers to the first (`bank` 0, `0x018765e8`) or second
    /// (`0x018769c8`) bank of 0x7c-byte windows.
    pub fn set_exe_window_area(mem: &mut impl Mem, bank: u32) {
        let first = if bank == 0 { 0x0187_65e8 } else { 0x0187_69c8 };
        let first = mem.rebase(first);
        let at = WINDOWS.ptr(mem).addr;
        for k in 0..8 {
            mem.write(at.wrapping_add(4 * k), first.wrapping_add(0x7c * k));
        }
    }

    /// The battle's target masks: `0x0133d154` cursor, `+4` event 1,
    /// `+8` event 0, `+0xc` the command's range, `+0x10` its flag mask,
    /// `+0x14` dead, `+0x18` unreachable, `+0x1c` the command's targets.
    pub const TARGET_BITS: Global<[u32; 8]> = Global::new(0x0133_d154);
    /// The selected command (`0x0133d0d2`, `u16`).
    pub const TARGET_COMMAND: Global<u16> = Global::new(0x0133_d0d2);

    /// `TOBtlMakeTargetCommandBit()` (`0x008983f0`): from the selected
    /// command's flags (`+0x1a` of `MsGetComData`'s record, passed in with
    /// a scratch word): 0x40 makes the dead mask all ones; the unreachable
    /// mask always all ones; 0x800 and 0x400 OR event 1's and event 0's
    /// masks into the targets; the range `0xfff00000` with 2, else
    /// `0x000fffff`; the flag mask all ones with 0x20. Returns the flag
    /// mask.
    pub fn btl_make_target_command_bit<M: Mem>(
        mem: &mut M,
        tmp: u32,
        get_com: impl FnOnce(&mut M, u32, u32) -> u32,
    ) -> u32 {
        let id = u32::from(TARGET_COMMAND.get(mem));
        let rec = get_com(mem, id, tmp);
        let t = TARGET_BITS.ptr(mem).addr;
        let flags: u16 = mem.read(rec.wrapping_add(0x1a));
        if flags & 0x40 != 0 {
            mem.write(t.wrapping_add(0x14), u32::MAX);
        }
        mem.write(t.wrapping_add(0x18), u32::MAX);
        let mut targets: u32 = mem.read(t.wrapping_add(0x1c));
        if flags & 0x800 != 0 {
            targets |= mem.read::<u32>(t.wrapping_add(4));
            mem.write(t.wrapping_add(0x1c), targets);
        }
        if flags & 0x400 != 0 {
            targets |= mem.read::<u32>(t.wrapping_add(8));
            mem.write(t.wrapping_add(0x1c), targets);
        }
        let range: u32 = if flags & 2 != 0 {
            0xfff0_0000
        } else {
            0x000f_ffff
        };
        mem.write(t.wrapping_add(0xc), range);
        let mask = if flags & 0x20 != 0 { u32::MAX } else { 0 };
        mem.write(t.wrapping_add(0x10), mask);
        mask
    }

    /// The battle calls [`btl_make_target_chr_status_bit`] makes per
    /// character.
    pub trait ChrStatus<M> {
        /// `MsGetChrCursorFlag(chr)`.
        fn cursor_flag(&mut self, mem: &mut M, chr: u32) -> u32;
        /// `MsGetChrEventFlag(chr, command)`.
        fn event_flag(&mut self, mem: &mut M, chr: u32, command: u32) -> u32;
        /// `MsGetChrDist(chr)`.
        fn dist(&mut self, mem: &mut M, chr: u32) -> u32;
        /// `MsGetChrStatDeath(chr)`.
        fn dead(&mut self, mem: &mut M, chr: u32) -> u32;
    }

    /// `TOBtlMakeTargetChrStatusBit()` (`0x00898330`): the cursor, event,
    /// dead and unreachable masks rebuilt for the 28 characters in the
    /// target union (`0x0133d0dc`): cursor flag 1, event flag 1 (event 1)
    /// or 0 (event 0), distance 0 (unreachable), not dead (dead mask; as
    /// compiled, the bit is set when `MsGetChrStatDeath` returns 0).
    pub fn btl_make_target_chr_status_bit<M: Mem>(mem: &mut M, k: &mut impl ChrStatus<M>) {
        let t = TARGET_BITS.ptr(mem).addr;
        for off in [0, 4, 8, 0x18, 0x14] {
            mem.write(t.wrapping_add(off), 0u32);
        }
        let union = mem.rebase(0x0133_d0dc);
        let or = |mem: &mut M, off: u32, bit: u32| {
            let v: u32 = mem.read(t.wrapping_add(off));
            mem.write(t.wrapping_add(off), v | bit);
        };
        for chr in 0..0x1cu32 {
            let bit = 1u32 << chr;
            if mem.read::<u32>(union) & bit == 0 {
                continue;
            }
            if k.cursor_flag(mem, chr) == 1 {
                or(mem, 0, bit);
            }
            let cmd = u32::from(TARGET_COMMAND.get(mem));
            match k.event_flag(mem, chr, cmd) {
                0 => or(mem, 8, bit),
                1 => or(mem, 4, bit),
                _ => {}
            }
            if k.dist(mem, chr) == 0 {
                or(mem, 0x18, bit);
            }
            if k.dead(mem, chr) == 0 {
                or(mem, 0x14, bit);
            }
        }
    }

    /// The font scales `TOGetFontWidthInPiexl` caches: `0x018716a8` (screen
    /// width / 512) and `0x018716ac` (height / 416).
    pub const FONT_SCALES: Global<[f32; 2]> = Global::new(0x0187_16a8);

    /// `TOGetFontWidthInPiexl(a, b)` (`0x008fa6b0`): with the height scale
    /// still 0 both scales computed from the font screen size
    /// (`graphicFontGetScreenWH`, passed in; `w * 2^-9`, `h / 416`); then
    /// `a * b * height_scale + 0.5` truncated (`_ftol2_sse`), as `f32`,
    /// divided by the width scale. In 53-bit precision, returned in `st(0)`.
    #[allow(clippy::many_single_char_names)] // the factors and scales
    pub fn font_width_in_pixel<M: Mem>(
        mem: &mut M,
        a: f32,
        b: f32,
        screen_wh: impl FnOnce(&mut M) -> (i32, i32),
    ) -> f32 {
        use ffx_bridge::x87::{add, div, load, mul, store};
        let mut s = FONT_SCALES.get(mem);
        if s[1] == 0.0 {
            let (w, h) = screen_wh(mem);
            #[allow(clippy::cast_precision_loss)] // fild, fstp dword
            let (w, h) = (w as f32, h as f32);
            s[0] = f32::from_bits(store(mul(f64::from(w), 1.0 / 512.0)));
            s[1] = f32::from_bits(store(div(f64::from(h), 416.0)));
            FONT_SCALES.set(mem, s);
        }
        let x = mul(
            mul(load(a.to_bits()), load(b.to_bits())),
            load(s[1].to_bits()),
        );
        let n = ffx_bridge::x87::ftol(mem, add(x, 0.5)).cast_signed();
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword
        let n = n as f32;
        f32::from_bits(store(div(f64::from(n), load(s[0].to_bits()))))
    }

    /// The three words that keep `0x008b8430` from widening (`0x01340824`
    /// movie fading, `0x0133c948`, `0x01327100`).
    pub const WIDE_BLOCKERS: [u32; 3] = [0x0134_0824, 0x0133_c948, 0x0132_7100];

    /// `0x008b8430(i, x)`: an x position in window `i` moved for a wide
    /// screen. Only for type-1 windows (`TOGetWindowType`) with none of
    /// [`WIDE_BLOCKERS`] set; there, with the font screen size `w`, `h`
    /// (`graphicFontGetScreenWH`, passed in), `hs = h / 416`, `ws = w /
    /// 512`, the result is `(w - 512 hs) / (2 ws) + x hs / ws`, the
    /// intermediates stored as `f32` where the original stores them;
    /// otherwise `x`. In 53-bit precision, returned in `st(0)`.
    #[allow(clippy::many_single_char_names)] // the screen terms
    pub fn wide_window_x<M: Mem>(
        mem: &mut M,
        i: i32,
        x: f32,
        screen_wh: impl FnOnce(&mut M) -> (i32, i32),
    ) -> f32 {
        use ffx_bridge::x87::{add, div, load, mul, store, sub};
        if crate::window::bridge::window_type(mem, i) != 1 {
            return x;
        }
        for g in WIDE_BLOCKERS {
            let g = mem.rebase(g);
            if mem.read::<u32>(g) != 0 {
                return x;
            }
        }
        let (w, h) = screen_wh(mem);
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword
        let (w, h) = (f64::from(w as f32), f64::from(h as f32));
        let f = |v: f64| f64::from(f32::from_bits(store(v)));
        let hs = f(div(h, 416.0));
        let ws = f(mul(w, 1.0 / 512.0));
        let diff = f(sub(w, mul(512.0, hs)));
        let two = f(add(ws, ws));
        let q1 = div(diff, two);
        let r = f(mul(div(hs, ws), load(x.to_bits())));
        f32::from_bits(store(add(q1, r)))
    }

    /// The font spaces `TOGetFontSpaceInPiexl` caches (`0x018716b4`,
    /// `0x018716b8`, f32).
    pub const FONT_SPACES: Global<[f32; 2]> = Global::new(0x0187_16b4);

    /// The scale factor of the wide space (`0x00b66c70`, `0.7f32` as a
    /// double).
    const SPACE_SCALE: f64 = 0.699_999_988_079_071;

    /// `TOGetFontSpaceInPiexl(kind)` (`0x008fa570`): the space widths in
    /// pixels, computed once (while the first is 0): the scales as
    /// [`font_width_in_pixel`] computes them when the width scale is 0;
    /// then for languages 1..=8 (`lang`, asked twice) both spaces
    /// `trunc(ws * 0.7 + 0.5) / ws`, for the others the narrow one
    /// `trunc(ws * 0.5 + 0.5) / ws` and the wide one `trunc(ws * 0.7 +
    /// 0.5) / ws`. Kind 1 or 3 gets the wide one. In 53-bit precision,
    /// returned in `st(0)`.
    pub fn font_space_in_pixel<M: Mem>(
        mem: &mut M,
        kind: u32,
        mut lang: impl FnMut(&mut M) -> u32,
        screen_wh: impl FnOnce(&mut M) -> (i32, i32),
    ) -> f32 {
        use ffx_bridge::x87::{add, div, ftol, load, mul, store};
        if FONT_SPACES.get(mem)[0] == 0.0 {
            let mut s = FONT_SCALES.get(mem);
            if s[0] == 0.0 {
                let (w, h) = screen_wh(mem);
                #[allow(clippy::cast_precision_loss)] // fild, fstp dword
                let (w, h) = (w as f32, h as f32);
                s[0] = f32::from_bits(store(mul(f64::from(w), 1.0 / 512.0)));
                s[1] = f32::from_bits(store(div(f64::from(h), 416.0)));
                FONT_SCALES.set(mem, s);
            }
            let ws = load(s[0].to_bits());
            let snap = |mem: &M, k: f64| {
                let n = ftol(mem, add(mul(ws, k), 0.5)).cast_signed();
                #[allow(clippy::cast_precision_loss)] // fild, fstp dword
                let n = n as f32;
                f32::from_bits(store(div(f64::from(n), ws)))
            };
            // asked twice, as the original does
            let latin = lang(mem).cast_signed() > 0 && lang(mem).cast_signed() < 9;
            let spaces = if latin {
                let v = snap(mem, SPACE_SCALE);
                [v, v]
            } else {
                [snap(mem, 0.5), snap(mem, SPACE_SCALE)]
            };
            FONT_SPACES.set(mem, spaces);
        }
        let s = FONT_SPACES.get(mem);
        if kind.to_le_bytes()[0] == 3 || kind.to_le_bytes()[0] == 1 {
            s[1]
        } else {
            s[0]
        }
    }

    /// `0x008ada30(i)`: message-window slot `i` set up: its seven words
    /// zeroed, its parts pointed at window `i` (`TOGetWindowStructAdrs`),
    /// message `i` and parameter entry `i`; the window cleared (0x7c bytes,
    /// `memset`), the message initialised (`TOInitMessage`); the window and
    /// message tagged with `i` (`+2`, `+4`), the slot's `+4` -1, and the
    /// message's handler `+0xfc` `0x008b64a0`. Returns the message.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::many_single_char_names
    )] // `mov word, si`; slot, window, message
    pub fn init_message_slot(mem: &mut impl Mem, i: i32) -> u32 {
        let slots = crate::window::bridge::MSG_WINDOW_SLOTS.ptr(mem).addr;
        let at = slots.wrapping_add(i.cast_unsigned().wrapping_mul(4));
        let s: u32 = mem.read(at);
        mem.write(s, [0u32; 7]);
        let w = crate::window::bridge::window(mem, i);
        let s: u32 = mem.read(at);
        mem.write(s.wrapping_add(0x10), w);
        let m = crate::window::bridge::message_window(mem, i);
        let s: u32 = mem.read(at);
        mem.write(s.wrapping_add(0x14), m);
        let e = crate::window::bridge::table_01868a90_entry(mem, i);
        let s: u32 = mem.read(at);
        mem.write(s.wrapping_add(0x18), e);
        let w: u32 = mem.read(s.wrapping_add(0x10));
        mem.write(w, [0u8; 0x7c]);
        init_message(mem, i);
        let s: u32 = mem.read(at);
        let w: u32 = mem.read(s.wrapping_add(0x10));
        mem.write(w.wrapping_add(2), i as u16);
        let m: u32 = mem.read(s.wrapping_add(0x14));
        mem.write(m.wrapping_add(4), i as u16);
        mem.write(s.wrapping_add(4), 0xffffu16);
        let m: u32 = mem.read(s.wrapping_add(0x14));
        let h = mem.rebase(0x008b_64a0);
        mem.write(m.wrapping_add(0xfc), h);
        m
    }

    /// The image sizes `TOGetImageWH` reads for the screen-sized images
    /// (`0x01841dac`, 16-byte entries: `i16` width, `i16` height).
    pub const IMAGE_SIZES: Global<[i16; 2]> = Global::new(0x0184_1dac);

    /// The size of image `id` in `TOGetImageWH`: a constant, or an entry of
    /// [`IMAGE_SIZES`] (`Err(k)`). Ids `0x3c00` and `0x3d00` pick their
    /// entry by the language and battle state ([`get_image_wh`]).
    ///
    /// # Errors
    ///
    /// The index of the [`IMAGE_SIZES`] entry to read instead.
    pub fn image_wh_of(id: u32) -> Result<(f32, f32), u32> {
        Ok(match id {
            0x2eac => (512.0, 416.0),
            0x2ecc => (512.0, 128.0),
            0x2ed0 | 0x2ed4 | 0x3e81 => (512.0, 256.0),
            0x2f00 | 0x2f04 | 0x0013_2f00 | 0x0013_2f04 => (512.0, 512.0),
            0x3d80 => (128.0, 256.0),
            0x3dc0 => (256.0, 128.0),
            0x3e00 | 0x3f00 | 0x0013_2f08 | 0x0013_2f0c => (256.0, 256.0),
            0x3e80 | 0x3e83 => (2048.0, 1024.0),
            0x3e86 => (1024.0, 1024.0),
            0x3d01 => return Err(3),
            0x3d40 => return Err(4),
            0x3d60 => return Err(5),
            // 0x3d10, 0x3d20, 0x3d30, and every unknown id (the original
            // also calls the empty `dbgPrintf` for those)
            _ => return Err(0),
        })
    }

    /// `TOGetImageWH(id, &w, &h)` (`0x008ac400`): the width and height of
    /// image `id` as floats ([`image_wh_of`]; `TkGetLang` and
    /// `MsBattleGetStatus` passed in). Returns `h`'s address.
    pub fn get_image_wh<M: Mem>(
        mem: &mut M,
        id: u32,
        out: [u32; 2],
        lang: impl FnOnce(&mut M) -> u32,
        battle: impl FnOnce(&mut M) -> u32,
    ) -> u32 {
        let k = match id {
            0x3c00 => Err(if lang(mem) != 0 { 4 } else { 0 }),
            0x3d00 => Err(if battle(mem) != 0 { 2 } else { 1 }),
            _ => image_wh_of(id),
        };
        let (w, h) = k.unwrap_or_else(|k| entry_wh(mem, k));
        mem.write(out[0], w);
        mem.write(out[1], h);
        out[1]
    }

    fn entry_wh(mem: &impl Mem, k: u32) -> (f32, f32) {
        let e = IMAGE_SIZES.ptr(mem).addr.wrapping_add(k << 4);
        let w: i16 = mem.read(e);
        let h: i16 = mem.read(e.wrapping_add(2));
        (f32::from(w), f32::from(h))
    }

    /// The limit sound effect (`0x8000004e`).
    pub const LIMIT_SE: u32 = 0x8000_004e;

    /// `0x0089ab20()` and `0x0089aba0()`: the limit timer's flag
    /// ([`LIMIT_TIMER`] `+0`) 1 and a playing [`LIMIT_SE`] stopped
    /// (`SndIsSepPlay`, `SndSepStop`, passed in as `sound(stop, id)`).
    /// Returns the last call's `eax`.
    pub fn limit_timer_stop<M: Mem>(
        mem: &mut M,
        mut sound: impl FnMut(&mut M, bool, u32) -> u32,
    ) -> u32 {
        let t = LIMIT_TIMER.ptr(mem).addr;
        mem.write(t, 1u32);
        if sound(mem, false, LIMIT_SE) == 0 {
            return 0;
        }
        sound(mem, true, LIMIT_SE)
    }

    /// `0x0089ab50()`: unless the flag is already 2, flag 2 and
    /// [`LIMIT_SE`] started (`SndSepPlaySimple`) when not playing;
    /// `sound(play, id)` is `SndIsSepPlay` or `SndSepPlaySimple`.
    pub fn limit_timer_start<M: Mem>(mem: &mut M, mut sound: impl FnMut(&mut M, bool, u32) -> u32) {
        let t = LIMIT_TIMER.ptr(mem).addr;
        if mem.read::<u32>(t) == 2 {
            return;
        }
        mem.write(t, 2u32);
        if sound(mem, false, LIMIT_SE) == 0 {
            sound(mem, true, LIMIT_SE);
        }
    }

    /// The window layout `0x00905e90` copies into (`0x01876148`).
    pub const WINDOW_LAYOUT: Global<u32> = Global::new(0x0187_6148);

    /// `0x00905e90(table, i)`: layout `i` of a layout table (a count, then
    /// offsets from the table): its count `n` (forced to 1 for layout
    /// 0x43) and `n` 12-byte entries copied to [`WINDOW_LAYOUT`] (`memcpy`
    /// of `12 n + 4` bytes), the count rewritten to 1 when it is 1.
    /// Returns the copy, or 0 for `i` out of range.
    pub fn copy_window_layout(mem: &mut impl Mem, table: u32, i: u32) -> u32 {
        if i >= mem.read::<u32>(table) {
            return 0;
        }
        let off: u32 = mem.read(table.wrapping_add(4).wrapping_add(i.wrapping_mul(4)));
        let src = off.wrapping_add(table);
        let mut n: u32 = mem.read(src);
        if i == 0x43 {
            n = 1;
        }
        let dst = WINDOW_LAYOUT.ptr(mem).addr;
        let len = n.wrapping_mul(12).wrapping_add(4);
        for k in 0..len {
            let v: u8 = mem.read(src.wrapping_add(k));
            mem.write(dst.wrapping_add(k), v);
        }
        if n == 1 {
            WINDOW_LAYOUT.set(mem, 1);
        }
        dst
    }

    /// The words `TOMenuInit` clears: `0x025d16b8..=0x025d16c0`,
    /// `0x0186a214`, and the halves `0x0186a1d0`, `0x0186a1ec`.
    pub const MENU_INIT_WORDS: [u32; 4] = [0x025d_16c0, 0x025d_16bc, 0x025d_16b8, 0x0186_a214];

    /// `TOMenuInit()` (`0x008be800`): the menu words cleared, then (a tail
    /// call) [`init_pad_data`].
    pub fn menu_init(mem: &mut impl Mem) -> u32 {
        for g in MENU_INIT_WORDS {
            let g = mem.rebase(g);
            mem.write(g, 0u32);
        }
        for g in [0x0186_a1d0, 0x0186_a1ec] {
            let g = mem.rebase(g);
            mem.write(g, 0u16);
        }
        init_pad_data(mem)
    }

    /// `TOMenuInitBrush()` (`0x00900f40`): the 16 window-move entries
    /// ([`WINDOW_MOVES`], 0x100 bytes) zeroed (`memset`); returns their
    /// address.
    pub fn menu_init_brush(mem: &mut impl Mem) -> u32 {
        let at = WINDOW_MOVES.ptr(mem).addr;
        mem.write(at, [0u8; 0x100]);
        at
    }

    /// `TOAmapMakeItemList(k)` (`0x008c6730`): the menu's item list
    /// refreshed (`TkMenuItemRefresh`, passed in, a tail call) in mode 2,
    /// 1, 3 or 6 for `k` 0..=3; other `k` return `k`.
    pub fn amap_make_item_list<M: Mem>(
        mem: &mut M,
        k: u32,
        refresh: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        match k {
            0 => refresh(mem, 2),
            1 => refresh(mem, 1),
            2 => refresh(mem, 3),
            3 => refresh(mem, 6),
            _ => k,
        }
    }

    /// The texture-name pointers `TOGetShapTextureName` picks from
    /// (`0x00c59d58..0x00c59eb8`, the per-language groups from
    /// `0x00c59e4c`).
    pub const SHAP_NAMES: u32 = 0x00c5_9d58;
    /// The cached name for image `0x2f00` (`0x025d16c8`) and the name of
    /// image `0x2f04` (`0x025d16cc`).
    pub const SHAP_CACHE: Global<[u32; 2]> = Global::new(0x025d_16c8);

    /// The four per-language names (`0x3e00`, `0x3f00`, `0x3dc0`, `0x2ecc`)
    /// of language `lang`, as image addresses of their pointers.
    fn shap_group(lang: u32) -> [u32; 4] {
        let g = |a: u32| [a, a + 4, a + 8, a + 0xc];
        match lang {
            // ebx comes from `0x00c5997c` for Japanese, as compiled
            0 => [0x00c5_997c, 0x00c5_9d58, 0x00c5_9d5c, 0x00c5_9d60],
            2 => g(0x00c5_9e5c),
            3 => g(0x00c5_9e6c),
            4 => g(0x00c5_9e7c),
            5 => g(0x00c5_9e8c),
            9 => g(0x00c5_9e9c),
            10 => g(0x00c5_9eac),
            _ => g(0x00c5_9e4c),
        }
    }

    /// `TOGetShapTextureName(id)` (`0x008ac8c0`): the texture name of
    /// shape image `id`, a pointer read from the name table:
    /// - `0x2ecc`, `0x3dc0`, `0x3e00`, `0x3f00`: by language (`lang`,
    ///   `TOGetFFXLang`, asked first);
    /// - `0x2f00`: cached in [`SHAP_CACHE`]; when 0, picked by language
    ///   (asked again) and stored, except languages 6..=8 and above 10,
    ///   which return the 0;
    /// - `0x2f04`: [`SHAP_CACHE`]'s second word;
    /// - `0x2eac`, `0x2ed0`, `0x2ed4`, `0x3e80`..`0x3e84`, `0x3e86`: fixed
    ///   entries;
    /// - `0x132f00`, `0x132f04`, `0x132f08`, `0x132f0c`: one of eight
    ///   16-byte groups by language;
    /// - anything else: `id` itself.
    pub fn shap_texture_name<M: Mem>(
        mem: &mut M,
        id: u32,
        mut lang: impl FnMut(&mut M) -> u32,
    ) -> u32 {
        let l = lang(mem);
        let word = |mem: &M, a: u32| -> u32 { mem.read(mem.rebase(a)) };
        let grp = shap_group(l);
        let quad = [
            word(mem, grp[0]),
            word(mem, grp[1]),
            word(mem, grp[2]),
            word(mem, grp[3]),
        ];
        let fixed = match id {
            0x2eac => Some(0x00c5_9d6c),
            0x2ed0 => Some(0x00c5_9d64),
            0x2ed4 => Some(0x00c5_9d68),
            0x3e80 => Some(0x00c5_9d70),
            0x3e81 => Some(0x00c5_9d74),
            0x3e82 => Some(0x00c5_9d78),
            0x3e83 => Some(0x00c5_9d7c),
            0x3e84 => Some(0x00c5_9d80),
            0x3e86 => Some(0x00c5_9d84),
            _ => None,
        };
        if let Some(a) = fixed {
            return word(mem, a);
        }
        match id {
            0x2ecc => quad[3],
            0x3dc0 => quad[2],
            0x3e00 => quad[0],
            0x3f00 => quad[1],
            0x2f04 => SHAP_CACHE.get(mem)[1],
            0x2f00 => {
                let c = SHAP_CACHE.get(mem);
                if c[0] != 0 {
                    return c[0];
                }
                let a = match lang(mem) {
                    0 => 0x00c5_9dac,
                    1 => 0x00c5_9dc8,
                    2 => 0x00c5_9db8,
                    3 => 0x00c5_9dc4,
                    4 => 0x00c5_9db4,
                    5 => 0x00c5_9dbc,
                    9 => 0x00c5_9dc0,
                    10 => 0x00c5_9db0,
                    _ => return c[0],
                };
                let v = word(mem, a);
                SHAP_CACHE.set(mem, [v, c[1]]);
                v
            }
            0x0013_2f00 | 0x0013_2f04 | 0x0013_2f08 | 0x0013_2f0c => {
                let k = match l {
                    1..=5 => l,
                    9 => 6,
                    10 => 7,
                    _ => 0,
                };
                word(mem, 0x00c5_9dcc + 0x10 * k + (id - 0x0013_2f00))
            }
            _ => id,
        }
    }

    /// The window-layout tables of `0x008fa7d0`, `0x008fa450` and
    /// `0x008fa790`.
    pub const LAYOUT_TABLES: [(u32, u32); 3] = [
        (0x008f_a7d0, 0x00c5_b460),
        (0x008f_a450, 0x00c5_c2c8),
        (0x008f_a790, 0x00c5_d578),
    ];

    /// One of [`LAYOUT_TABLES`]' functions: [`copy_window_layout`] from its
    /// table.
    pub fn copy_layout_from(mem: &mut impl Mem, table: u32, i: u32) -> u32 {
        let t = mem.rebase(table);
        copy_window_layout(mem, t, i)
    }

    /// `0x008fa740(kind, font, size)`: a character's advance: the width
    /// [`font_width_in_pixel`]`(font.scale (+0x1c), size)` plus the space
    /// [`font_space_in_pixel`]`(kind)` (passed in together as `width`,
    /// `space`), rounded to `f32`. Returned in `st(0)`.
    pub fn font_advance<M: Mem>(
        mem: &mut M,
        kind: u32,
        font: u32,
        size: f32,
        width: impl FnOnce(&mut M, f32, f32) -> f32,
        space: impl FnOnce(&mut M, u32) -> f32,
    ) -> f32 {
        use ffx_bridge::x87::{add, load, store};
        let scale: f32 = mem.read(font.wrapping_add(0x1c));
        let w = width(mem, scale, size);
        let s = space(mem, kind);
        f32::from_bits(store(add(load(s.to_bits()), load(w.to_bits()))))
    }

    /// `TOAddFontWidthSpace(glyph, font, size)` (`0x008e8ed0`): the glyph's
    /// x (`+4`, f32) snapped to the pixel grid (`trunc(x * ws + 0.5) / ws`,
    /// the scales computed first while the width scale is 0), then advanced
    /// by [`font_advance`] of the glyph's kind (byte `+0x1a`).
    #[allow(clippy::many_single_char_names)] // scales, position, pixel count
    pub fn add_font_width_space<M: Mem>(
        mem: &mut M,
        glyph: u32,
        font: u32,
        size: f32,
        screen_wh: impl FnOnce(&mut M) -> (i32, i32),
        width: impl FnOnce(&mut M, f32, f32) -> f32,
        space: impl FnOnce(&mut M, u32) -> f32,
    ) {
        use ffx_bridge::x87::{add, div, ftol, load, mul, store};
        let mut s = FONT_SCALES.get(mem);
        if s[0] == 0.0 {
            let (w, h) = screen_wh(mem);
            #[allow(clippy::cast_precision_loss)] // fild, fstp dword
            let (w, h) = (w as f32, h as f32);
            s[0] = f32::from_bits(store(mul(f64::from(w), 1.0 / 512.0)));
            s[1] = f32::from_bits(store(div(f64::from(h), 416.0)));
            FONT_SCALES.set(mem, s);
        }
        let ws = load(s[0].to_bits());
        let at = glyph.wrapping_add(4);
        let x: f32 = mem.read(at);
        let px = load(store(mul(ws, load(x.to_bits()))));
        let n = ftol(mem, add(px, 0.5)).cast_signed();
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword
        let n = n as f32;
        let snapped = f32::from_bits(store(div(f64::from(n), ws)));
        mem.write(at, snapped);
        let scale: f32 = mem.read(font.wrapping_add(0x1c));
        let w = width(mem, scale, size);
        let kind = u32::from(mem.read::<u8>(glyph.wrapping_add(0x1a)));
        let sp = space(mem, kind);
        let x: f32 = mem.read(at);
        let r = add(
            add(load(sp.to_bits()), load(w.to_bits())),
            load(x.to_bits()),
        );
        mem.write(at, f32::from_bits(store(r)));
    }

    /// `0x008b8d10(i, out, x, y, w, h)`: a rectangle of window `i` as four
    /// floats: `x` moved for the wide screen ([`wide_window_x`], passed in
    /// as `wide`), `y`, that plus `w`, `y + h`. Returns `y + h`.
    #[allow(clippy::cast_precision_loss, clippy::many_single_char_names)] // fild; the rectangle
    pub fn wide_window_rect<M: Mem>(
        mem: &mut M,
        i: i32,
        out: u32,
        r: [i32; 4],
        wide: impl FnOnce(&mut M, i32, f32) -> f32,
    ) -> i32 {
        use ffx_bridge::x87::{add, load, store};
        let [x, y, w, h] = r;
        let left = wide(mem, i, x as f32);
        mem.write(out, left);
        mem.write(out.wrapping_add(4), y as f32);
        let right = store(add(load(left.to_bits()), f64::from(w as f32)));
        mem.write(out.wrapping_add(8), right);
        let bottom = y.wrapping_add(h);
        mem.write(out.wrapping_add(0xc), bottom as f32);
        bottom
    }

    /// Whether the C strings at `a` and `b` are equal.
    #[allow(clippy::many_single_char_names)] // the two strings and their bytes
    fn c_str_eq(mem: &impl Mem, a: u32, b: u32) -> bool {
        let mut k = 0u32;
        loop {
            let (x, y): (u8, u8) = (mem.read(a.wrapping_add(k)), mem.read(b.wrapping_add(k)));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            k += 1;
        }
    }

    /// The interned texture names (`0x025cc5a0`, 64 slots of 0x100 bytes)
    /// and their count (`0x01841d90`).
    pub const NAME_SLOTS: Global<[u8; 0x100]> = Global::new(0x025c_c5a0);
    /// The count.
    pub const NAME_COUNT: Global<i32> = Global::new(0x0184_1d90);

    /// `0x008ab440(name)`: the slot holding the C string `name`: an
    /// existing equal one, else the next slot (cleared, then the string
    /// copied with its terminator; the original only prints, through the
    /// empty `dbgPrintf`, when 64 slots are already used, and writes past
    /// them). Returns the slot.
    pub fn intern_name(mem: &mut impl Mem, name: u32) -> u32 {
        let base = NAME_SLOTS.ptr(mem).addr;
        let n = NAME_COUNT.get(mem);
        let mut k = 0;
        while k < n {
            let slot = base.wrapping_add(k.cast_unsigned() << 8);
            if c_str_eq(mem, slot, name) {
                return slot;
            }
            k += 1;
        }
        let slot = base.wrapping_add(n.cast_unsigned() << 8);
        mem.write(slot, [0u8; 0x100]);
        let mut i = 0u32;
        loop {
            let c: u8 = mem.read(name.wrapping_add(i));
            mem.write(slot.wrapping_add(i), c);
            if c == 0 {
                break;
            }
            i += 1;
        }
        NAME_COUNT.set(mem, n.wrapping_add(1));
        slot
    }

    /// The menu calls of [`summon_list_init`].
    pub trait SummonList<M> {
        /// `MsMenuStatusNum(id, strings)`.
        fn status_num(&mut self, mem: &mut M, id: u32, strings: u32) -> u32;
        /// `TkMenuGetCommandMax(list, kind, a, b)`.
        fn command_max(&mut self, mem: &mut M, list: u32, kind: u32, a: u32, b: u32) -> i32;
        /// `TkMenuGetCommandOffset(list, kind, a, b)`.
        fn command_offset(&mut self, mem: &mut M, list: u32, kind: u32, a: u32, b: u32) -> i32;
        /// `MsGetSavePlyJoin(chr)`.
        fn joined(&mut self, mem: &mut M, chr: u32) -> u32;
    }

    /// The summon menu's state (`0x01841c38`): `+0` the shown aeons
    /// (`g_TkMenuSummons`), `+0xc` every aeon (`g_TkMenuSummonTotals`),
    /// `+0x18` the shown count, `+0x1c` a word, `+0x20` the total count,
    /// `+0x24` the enabled mask.
    pub const SUMMON_MENU: u32 = 0x0184_1c38;

    /// `0x008a9090()`: the summon list built from the menu's command list 5
    /// (`MsMenuStatusNum(0, 0)`, its count and offset): every aeon id into
    /// the totals; the joined ones (`MsGetSavePlyJoin`) also into the shown
    /// list and the enabled mask. Command 0x64 stands for the three magus
    /// sisters (0xf, 0x10, 0x11), added together when 0xf has joined.
    #[allow(clippy::many_single_char_names)] // the menu state, list and cursor
    pub fn summon_list_init<M: Mem>(mem: &mut M, k: &mut impl SummonList<M>) {
        let g = mem.rebase(SUMMON_MENU);
        let d = k.status_num(mem, 0, 0);
        mem.write(g.wrapping_add(0x24), 0u32);
        mem.write(g, [u32::MAX; 2]);
        mem.write(g.wrapping_add(8), 0xffffu16);
        mem.write(g.wrapping_add(0x18), 0u32);
        let n = k.command_max(mem, d, 5, 1, 0);
        let off = k.command_offset(mem, d, 5, 1, 0);
        mem.write(g.wrapping_add(0x20), n);
        if n <= 0 {
            mem.write(g.wrapping_add(0x1c), 0u32);
            return;
        }
        let mut p = d
            .wrapping_add(off.cast_unsigned().wrapping_mul(4))
            .wrapping_add(2);
        let totals = g.wrapping_add(0xc);
        let mut i = 0i32;
        let byte_at = |i: i32| totals.wrapping_add(i.cast_unsigned());
        while i < n {
            let id = u32::from(mem.read::<u16>(p));
            if id == 0x64 {
                if k.joined(mem, 0xf) != 0 {
                    let c = mem.read::<u32>(g.wrapping_add(0x18)).wrapping_add(3);
                    let m: u32 = mem.read(g.wrapping_add(0x24));
                    mem.write(g.wrapping_add(0x24), m | 0x38000);
                    mem.write(g.wrapping_add(c).wrapping_sub(3), 0x100fu16);
                    mem.write(g.wrapping_add(c).wrapping_sub(1), 0x11u8);
                    mem.write(g.wrapping_add(0x18), c);
                }
                mem.write(byte_at(i), 0x100fu16);
                i += 2;
                p = p.wrapping_add(8);
                mem.write(byte_at(i), 0x11u8);
            } else {
                #[allow(clippy::cast_possible_truncation)] // `mov byte, bl`
                let b = id as u8;
                if k.joined(mem, id) != 0 {
                    let c = mem.read::<u32>(g.wrapping_add(0x18)).wrapping_add(1);
                    mem.write(g.wrapping_add(c).wrapping_sub(1), b);
                    mem.write(g.wrapping_add(0x18), c);
                    let m: u32 = mem.read(g.wrapping_add(0x24));
                    mem.write(g.wrapping_add(0x24), m | 1u32.wrapping_shl(id));
                }
                mem.write(byte_at(i), b);
            }
            i += 1;
            p = p.wrapping_add(4);
        }
        mem.write(g.wrapping_add(0x1c), 0u32);
    }

    /// The loaded font/menu textures (`0x01841da0`, 16-byte entries: image
    /// data, CLUT data, image size, `i16` width and height; [`IMAGE_SIZES`]
    /// is the size pair).
    pub const TEXTURES: Global<[u32; 4]> = Global::new(0x0184_1da0);
    /// The two fixed image buffers (textures 0 and 4).
    pub const TEXTURE_IMAGE_BUFFERS: [(u16, u32); 2] = [(0, 0x0184_b610), (4, 0x0185_b610)];
    /// The CLUT buffers of textures 0..=5.
    pub const TEXTURE_CLUT_BUFFERS: [u32; 6] = [
        0x0186_4e10,
        0x0186_56c0,
        0x0186_5840,
        0x0186_59c0,
        0x0186_55f0,
        0x0186_5b60,
    ];

    /// `TOFTCTransfer(file)` (`0x008ac130`): a font/menu texture file
    /// (version `u16` `+4` from 0xc8, slot `u16` `+8`) registered in
    /// [`TEXTURES`]: image (`+0x20` offset, `+0x24` size), size (`+0x28`,
    /// `+0x2a`); textures 0 and 4 copy the image into their own buffer;
    /// slots 0..=5 copy the CLUT (`+0x30` offset, `+0x10` size) into theirs,
    /// others get none. Returns 0, or 1 for an old file or a slot above 5.
    /// (The original also passes the slot to the empty `dbgPrintf`.)
    #[allow(clippy::many_single_char_names)] // file, slot, entry, size
    pub fn ftc_transfer(mem: &mut impl Mem, f: u32) -> u32 {
        if mem.read::<u16>(f.wrapping_add(4)) < 0xc8 {
            return 1;
        }
        let k = mem.read::<u16>(f.wrapping_add(8));
        let e = TEXTURES.ptr(mem).addr.wrapping_add(u32::from(k) << 4);
        let image = f.wrapping_add(mem.read(f.wrapping_add(0x20)));
        let size: u32 = mem.read(f.wrapping_add(0x24));
        mem.write(e, image);
        mem.write(e.wrapping_add(8), size);
        let w: u16 = mem.read(f.wrapping_add(0x28));
        let h: u16 = mem.read(f.wrapping_add(0x2a));
        mem.write(e.wrapping_add(0xc), w);
        mem.write(e.wrapping_add(0xe), h);
        for (slot, buf) in TEXTURE_IMAGE_BUFFERS {
            if k == slot {
                let buf = mem.rebase(buf);
                copy_bytes(mem, buf, image, size);
                mem.write(e, buf);
            }
        }
        let Some(&clut) = TEXTURE_CLUT_BUFFERS.get(usize::from(k)) else {
            mem.write(e.wrapping_add(4), 0u32);
            return 1;
        };
        let clut = mem.rebase(clut);
        mem.write(e.wrapping_add(4), clut);
        let src = f.wrapping_add(mem.read(f.wrapping_add(0x30)));
        let n: u32 = mem.read(f.wrapping_add(0x10));
        copy_bytes(mem, clut, src, n);
        0
    }

    /// `memcpy(dst, src, n)`, byte by byte.
    fn copy_bytes(mem: &mut impl Mem, dst: u32, src: u32, n: u32) {
        for i in 0..n {
            let b: u8 = mem.read(src.wrapping_add(i));
            mem.write(dst.wrapping_add(i), b);
        }
    }

    /// The menu calls of [`player_list_init`].
    pub trait PlayerList<M> {
        /// `MsGetSaveInParty(&count)`: the party's ids.
        fn in_party(&mut self, mem: &mut M, count: u32) -> u32;
        /// `MsGetSaveOutParty(&count)`: the reserve's ids.
        fn out_party(&mut self, mem: &mut M, count: u32) -> u32;
        /// `MsGetSavePlyJoined(chr)`.
        fn joined(&mut self, mem: &mut M, chr: u32) -> u32;
    }

    /// The player menu's state (`0x01841c14`): `+0` the player ids
    /// (`g_TkMenuPlayers`), `+8` the party-and-reserve count, `+0xc` the
    /// full count, `+0x10` the party count, `+0x14` a word, `+0x18` the
    /// party bits (`g_TkMenuPartyBits`), `+0x1c` the listed mask, `+0x20`
    /// the joined mask.
    pub const PLAYER_MENU: u32 = 0x0184_1c14;

    /// `0x008a8f40(flags)`: the player list built from the party and the
    /// reserve (`MsGetSaveInParty`, `MsGetSaveOutParty`, each with a
    /// scratch count), then every other joined character (0..=7,
    /// `MsGetSavePlyJoined`); free slots (0xff) and character 7 never
    /// listed, nobody when `flags`' high half is 1.
    #[allow(clippy::many_single_char_names)] // menu state, list, count
    pub fn player_list_init<M: Mem>(
        mem: &mut M,
        flags: u32,
        count: u32,
        k: &mut impl PlayerList<M>,
    ) {
        let g = mem.rebase(PLAYER_MENU);
        let none = flags & 0xffff_0000 == 0x1_0000;
        mem.write(g.wrapping_add(0x1c), [0u32; 2]);
        let mut mask = 0u32;
        let mut n = 0u32;
        let take = |mem: &mut M, p: u32, party: bool, mask: &mut u32, n: &mut u32| {
            let len: i32 = mem.read(count);
            for d in 0..len.max(0).cast_unsigned() {
                let c: u8 = mem.read(p.wrapping_add(d));
                if c == 0xff || c == 7 || none {
                    continue;
                }
                mem.write(g.wrapping_add(*n), c);
                *n += 1;
                let bit = 1u32.wrapping_shl(u32::from(c));
                *mask |= bit;
                if party {
                    let v: u32 = mem.read(g.wrapping_add(0x18));
                    mem.write(g.wrapping_add(0x18), v | bit);
                }
            }
        };
        let p = k.in_party(mem, count);
        mem.write(g.wrapping_add(0x18), 0u32);
        take(mem, p, true, &mut mask, &mut n);
        mem.write(g.wrapping_add(0x10), n);
        let p = k.out_party(mem, count);
        take(mem, p, false, &mut mask, &mut n);
        mem.write(g.wrapping_add(8), n);
        mem.write(g.wrapping_add(0x1c), mask);
        let mut joined = 0u32;
        for b in 0..8u32 {
            if k.joined(mem, b) != 0 && b != 7 && !none {
                let bit = 1u32 << b;
                joined |= bit;
                if mask & bit == 0 {
                    mem.write(g.wrapping_add(n), b.to_le_bytes()[0]);
                    n += 1;
                }
            }
        }
        mem.write(g.wrapping_add(0xc), n);
        mem.write(g.wrapping_add(0x20), joined | mask);
        mem.write(g.wrapping_add(0x14), 0u32);
    }

    /// The battle info window's state (`0x0133f5bc..0x0133f6a2`).
    pub const INFO_WIN: u32 = 0x0133_f5bc;

    /// `TOBtlInitInfoWinMes()` (`0x00897bd0`): the battle info window laid
    /// out for the screen: its corners (x 700 and 1400, y 101, 109 and
    /// 160) through `graphicUiRemapX2`/`Y2` (passed in as `remap(y, v)`)
    /// and `_ftol2_sse`, then three line records of position, size and
    /// float origin, all from those. Returns 0.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // `mov word, ax`; fild
    pub fn btl_init_info_win_mes<M: Mem>(
        mem: &mut M,
        mut remap: impl FnMut(&mut M, bool, f32) -> f32,
    ) -> u32 {
        let g = |off: u32| INFO_WIN.wrapping_add(off);
        let w16 = |mem: &mut M, a: u32, v: u16| {
            let a = mem.rebase(a);
            mem.write(a, v);
        };
        let w32 = |mem: &mut M, a: u32, v: u32| {
            let a = mem.rebase(a);
            mem.write(a, v);
        };
        let b = mem.rebase(0x0133_c93d);
        mem.write(b, 0u8);
        w16(mem, g(0), 0xff);
        let mut pick = |mem: &mut M, y: bool, v: f32| -> u16 {
            let r = remap(mem, y, v);
            ffx_bridge::x87::ftol(mem, f64::from(r)) as u16
        };
        let a0 = pick(mem, false, 700.0);
        w16(mem, 0x0133_f68c, a0);
        let b0 = pick(mem, true, 101.0);
        w16(mem, 0x0133_f690, b0);
        let c0 = pick(mem, false, 700.0);
        w16(mem, 0x0133_f694, c0);
        let d0 = pick(mem, true, 109.0);
        w16(mem, 0x0133_f698, d0);
        let e0 = pick(mem, false, 1400.0);
        w16(mem, 0x0133_f69c, e0);
        let f0 = pick(mem, true, 160.0);
        w16(mem, 0x0133_f6a0, f0);
        let dx = e0.wrapping_sub(a0);
        let x = f32::from(a0.cast_signed());
        let y = f32::from(b0.cast_signed());
        for (a, v) in [
            (0x0133_f688, 0),
            (0x0133_f618, 0),
            (0x0133_f61a, c0),
            (0x0133_f61c, d0),
            (0x0133_f61e, dx),
        ] {
            w16(mem, a, v);
        }
        for (a, v) in [
            (0x0133_f624, x),
            (0x0133_f628, y),
            (0x0133_f640, x),
            (0x0133_f644, y),
        ] {
            w32(mem, a, v.to_bits());
        }
        for (a, v) in [
            (0x0133_f65c, x),
            (0x0133_f660, y),
            (0x0133_f678, x),
            (0x0133_f67c, y),
        ] {
            w32(mem, a, v.to_bits());
        }
        for a in [0x0133_f630, 0x0133_f64c, 0x0133_f668, 0x0133_f684] {
            w32(mem, a, 0);
        }
        for (a, v) in [
            (0x0133_f62c, 0),
            (0x0133_f634, 0),
            (0x0133_f636, c0),
            (0x0133_f638, d0),
            (0x0133_f63a, dx),
            (0x0133_f648, 0),
            (0x0133_f650, 0),
            (0x0133_f652, c0),
            (0x0133_f654, d0),
            (0x0133_f656, dx),
            (0x0133_f664, 0),
            (0x0133_f66c, 0),
            (0x0133_f66e, c0),
            (0x0133_f670, d0),
            (0x0133_f672, dx),
            (0x0133_f680, 0),
        ] {
            w16(mem, a, v);
        }
        0
    }

    /// `TOSortMessageWindowPriority(out)` (`0x008b00a0`): the message-window
    /// slots in use (`u16` 1 at the slot object) as (slot, priority `+4`)
    /// pairs, unused ones (0x7fff, 0x7fff), put in order by the compiled
    /// exchange sort (for each `c`, every `j` with a lower signed priority
    /// swapped into place `c`); then `out` (eight `u16`) filled with 0xffff
    /// and the slots in order up to the first unused one. Returns how many
    /// were written.
    pub fn sort_message_window_priority(mem: &mut impl Mem, out: u32) -> u32 {
        let slots = crate::window::bridge::MSG_WINDOW_SLOTS.ptr(mem).addr;
        let mut e = [(0x7fffu16, 0x7fffi16); 8];
        for (k, item) in (0u16..).zip(e.iter_mut()) {
            let s: u32 = mem.read(slots.wrapping_add(4 * u32::from(k)));
            if mem.read::<u16>(s) == 1 {
                *item = (k, mem.read(s.wrapping_add(4)));
            }
        }
        for c in 0..8 {
            for j in 0..8 {
                if e[j].1 < e[c].1 {
                    e.swap(c, j);
                }
            }
        }
        mem.write(out, [u32::MAX; 4]);
        let mut n = 0u32;
        for &(idx, _) in &e {
            if idx == 0x7fff {
                break;
            }
            mem.write(out.wrapping_add(2 * n), idx);
            n += 1;
        }
        n
    }

    /// Whether `TOGetFontWidth` corrects for the screen's aspect
    /// (`0x018663ec`).
    pub const FONT_ASPECT_ON: Global<u32> = Global::new(0x0186_63ec);

    /// The calls `TOGetFontWidth` makes.
    pub trait FontWidth<M> {
        /// `graphicFontGetScreenWH()`.
        fn screen_wh(&mut self, mem: &mut M) -> (i32, i32);
        /// `TOGetFFXLang()`.
        fn lang(&mut self, mem: &mut M) -> u32;
        /// `TOGetFontWidthInPiexl(width, scale)`.
        fn width(&mut self, mem: &mut M, width: f32, scale: f32) -> f32;
        /// `TOGetFontSpaceInPiexl(kind)`.
        fn space(&mut self, mem: &mut M, kind: u32) -> f32;
    }

    /// `TOGetFontWidth(text, kind, &width, table)` (`0x008b9330`): the
    /// on-screen width of the character at `text` (stored at `width`) and
    /// the address after it. The glyph width comes from the font's width
    /// table `table` (`TOGetFontWidthTblAdrs`, [`TEXTURES`] `+4`; a
    /// Korean/Chinese double-byte page with code 4, 0x410 further on) or,
    /// for the codes 0x26..=0x2f and 6, from tables 0..=3 and 5, a quarter
    /// of its signed byte; without one, the stale word the original reads
    /// (the screen's width/height ratio). Then [`font_width_in_pixel`] of
    /// it at the kind's scale plus [`font_space_in_pixel`], times the
    /// aspect correction (`w/h / 1.2307693`) when [`FONT_ASPECT_ON`]. A
    /// positive width is stored, except that a run of repeated 0x47
    /// codes adds it (twice for table 4) once per code; codes 6 and below
    /// 0x30 are two bytes long. In 53-bit precision.
    #[allow(
        clippy::too_many_lines,
        clippy::cast_precision_loss,
        clippy::many_single_char_names
    )] // one routine; fild; the cursors
    pub fn font_width<M: Mem>(
        mem: &mut M,
        text: u32,
        kind: u32,
        out: u32,
        table: u32,
        k: &mut impl FontWidth<M>,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, load, mul, store};
        let orig = text;
        let mut p = text;
        let (w, h) = k.screen_wh(mem);
        let ratio = f32::from_bits(store(div(f64::from(w as f32), f64::from(h as f32))));
        let aspect = f32::from_bits(store(div(f64::from(ratio), 1.230_769_276_618_957_5)));
        let lang = k.lang(mem);
        let big = (lang == 9 || lang == 10) && mem.read::<u8>(p) == 4;
        if big {
            p = p.wrapping_add(1);
        }
        mem.write(out, 0f32);
        let mut off = if big { 0x410u32 } else { 0 };
        let scale: f32 = if kind == 3 || kind == 1 {
            FONT_SCALE.get(mem)
        } else {
            f32::from_bits(0x3f47_ae14)
        };
        let tbl = |mem: &M, t: u32| -> u32 {
            mem.read(TEXTURES.ptr(mem).addr.wrapping_add((t << 4) + 4))
        };
        let quarter = |mem: &M, at: u32| -> f32 {
            let v = i32::from(mem.read::<i8>(at));
            f32::from_bits(store(mul(f64::from(v), 0.25)))
        };
        let mut glyph = ratio;
        let b: u8 = mem.read(p);
        let next = u32::from(mem.read::<u8>(p.wrapping_add(1)));
        if b >= 0x30 {
            let t = tbl(mem, table);
            off = off.wrapping_sub(0x30).wrapping_add(u32::from(b));
            glyph = quarter(mem, t.wrapping_add(off));
        }
        let row = u32::from(b).wrapping_mul(0xd0).wrapping_add(next);
        if (0x2c..=0x2f).contains(&b) {
            let t = tbl(mem, 0);
            glyph = quarter(
                mem,
                t.wrapping_add(row).wrapping_add(off).wrapping_sub(0x2320),
            );
        }
        for (lo, t, base) in [(0x2au8, 1, 0x2250u32), (0x28, 2, 0x20b0), (0x26, 3, 0x1f10)] {
            if b == lo || b == lo + 1 {
                let t = tbl(mem, t);
                glyph = quarter(mem, t.wrapping_add(row).wrapping_sub(base));
            }
        }
        if b == 6 {
            let t = tbl(mem, 5);
            glyph = if t == 0 {
                56.0
            } else {
                quarter(mem, t.wrapping_add(next).wrapping_sub(0x30))
            };
        }
        let wpx = k.width(mem, glyph, scale);
        let sp = k.space(mem, kind);
        let mut r = add(load(sp.to_bits()), load(wpx.to_bits()));
        if FONT_ASPECT_ON.get(mem) != 0 {
            r = mul(r, load(aspect.to_bits()));
        }
        let res = f32::from_bits(store(r));
        let first: u8 = mem.read(orig);
        let c: u8 = mem.read(p);
        let repeat = first == 0x47 && mem.read::<u8>(p.wrapping_add(1)) == c;
        if res > 0.0 {
            if repeat {
                if c != 0x47 {
                    return p;
                }
                let res = load(res.to_bits());
                loop {
                    let o: f32 = mem.read(out);
                    let v = if table == 4 {
                        add(mul(res, 2.0), load(o.to_bits()))
                    } else {
                        add(load(o.to_bits()), res)
                    };
                    p = p.wrapping_add(1);
                    mem.write(out, f32::from_bits(store(v)));
                    if mem.read::<u8>(p) != 0x47 {
                        return p;
                    }
                }
            }
            mem.write(out, res);
        }
        let c: u8 = mem.read(p);
        if c == 6 || c < 0x30 {
            return p.wrapping_add(2);
        }
        if first == 0x47 && mem.read::<u8>(p.wrapping_add(1)) == c {
            if c != 0x47 {
                return p;
            }
            loop {
                p = p.wrapping_add(1);
                if mem.read::<u8>(p) != 0x47 {
                    return p;
                }
            }
        }
        p.wrapping_add(1)
    }

    /// `0x008fa4f0(id)`: the window layout of `id` from the image tables:
    /// ids `0..=0xc6` and `0xc8..=0x18e` from the first, `0x190..=0x256`
    /// from the second, `0x258..=0x289` from the third (a tail call); each
    /// later range overriding, and otherwise `id` itself.
    pub fn window_layout_of(mem: &mut impl Mem, id: u32) -> u32 {
        let [first, second, third] = [LAYOUT_TABLES[0].1, LAYOUT_TABLES[1].1, LAYOUT_TABLES[2].1];
        let mut r = id;
        if id <= 0xc6 {
            r = copy_layout_from(mem, first, id);
        }
        if id.wrapping_sub(0xc8) <= 0xc6 {
            r = copy_layout_from(mem, first, id - 0xc8);
        }
        if id.wrapping_sub(0x190) <= 0xc6 {
            r = copy_layout_from(mem, second, id - 0x190);
        }
        if id.wrapping_sub(0x258) <= 0x31 {
            return copy_layout_from(mem, third, id - 0x258);
        }
        r
    }

    /// `0x008a8e20()`: `MsGetSaveEventAddress() + 0x3d48`, the save's gil
    /// (`g_SaveGil`, `0x011307d8`).
    pub fn save_gil_address(mem: &impl Mem) -> u32 {
        mem.rebase(0x0112_ca90).wrapping_add(0x3d48)
    }

    /// The calls [`texture_name`] makes outside this module.
    pub trait TextureName<M> {
        /// `MsBattleGetStatus()`.
        fn battle_status(&mut self, mem: &mut M) -> u32;
        /// `TkGetLang()`.
        fn tk_lang(&mut self, mem: &mut M) -> u32;
        /// `TOGetFFXLang()` (through its thunk `0x008ac2f0`).
        fn ffx_lang(&mut self, mem: &mut M) -> u32;
        /// `dbgPrintf(fmt)`, for an unknown id.
        fn print(&mut self, mem: &mut M, fmt: u32);
    }

    /// Image addresses of [`texture_name`]'s name tables, `(plain, flagged)`.
    const TEXTURE_TABLES: [(u32, u32); 7] = [
        (0x00c5_9568, 0x00c5_95a8), // 0x3c00, TkGetLang 0
        (0x00c5_95e8, 0x00c5_9628), // 0x3c00 other languages, 0x3d40
        (0x00c5_9668, 0x00c5_96e8), // 0x3d10
        (0x00c5_96a8, 0x00c5_9728), // 0x3d20
        (0x00c5_9768, 0x00c5_97a8), // 0x3d30
        (0x00c5_97f8, 0x00c5_9838), // 0x3d80, by the byte
        (0x00c5_9bc0, 0x00c5_9c00), // 0x3d00, appended to a prefix
    ];

    fn c_strlen(mem: &impl Mem, p: u32) -> u32 {
        let mut n = 0u32;
        while mem.read::<u8>(p.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `TOGetTextureName(id, nibbles, index, high, &size)` (`0x008acf50`):
    /// the name of picture `index` of texture family `id`. One nibble of
    /// `nibbles` (the high one when `high`) picks a column `k` (twice the
    /// nibble's low three bits; its bit 3 or `high` picks the second,
    /// "flagged" table set). By `id`:
    ///
    /// - `0x3c00`: `TkGetLang()` 0 or not picks the table pair;
    /// - `0x3d00`: the prefix (`0x025d08a0`, or `0x025d06a0` in battle by
    ///   `MsBattleGetStatus`) copied to `0x025d07a0` (`0x025d05a0`), the
    ///   table's name appended, and the result interned (`0x008ab440`);
    /// - `0x3d10`, `0x3d20`, `0x3d30`, `0x3d40`: one table pair each;
    /// - `0x3d01`, `0x3d60`: by `TOGetFFXLang()` 0, 9 or 10 a table pair
    ///   indexed by `index` alone, flagged by `high` only (another language
    ///   reads the word at `k + 4 * index`, as the original does);
    /// - `0x3d80`: indexed by the byte `nibbles`, flagged by `high`;
    /// - anything else: `dbgPrintf("VIRTUOS ASSERTION:Font not found.")`
    ///   and `k` returned.
    ///
    /// The tables are indexed by `index + k` otherwise. `size` gets the two
    /// words of entry `(index + k) / 2` of `0x00b5f8d0` (flagged) or
    /// `0x00b5f890` (entry 0 for an unknown id).
    #[allow(clippy::too_many_lines)]
    pub fn texture_name<M: Mem>(
        mem: &mut M,
        id: u32,
        nibbles: u32,
        index: u32,
        high: u32,
        size: u32,
        calls: &mut impl TextureName<M>,
    ) -> u32 {
        let high = high & 0xffff != 0;
        let byte = nibbles & 0xff;
        let nib = if high { byte >> 4 } else { byte & 0xf };
        let (nib, second) = if nib >= 8 {
            (nib - 8, true)
        } else {
            (nib, false)
        };
        let k = nib * 2;
        let half = |v: u32| i32::from(u16::try_from(v & 0xffff).unwrap_or(0).cast_signed());
        let index = half(index);
        let id = half(id);
        let at = |mem: &M, table: u32, i: i32| -> u32 {
            let t = mem.rebase(table);
            mem.read(t.wrapping_add(i.cast_unsigned().wrapping_mul(4)))
        };
        let pair = |i: usize, flagged: bool| {
            let (p, f) = TEXTURE_TABLES[i];
            if flagged { f } else { p }
        };
        let flagged = second || high;
        let by_lang =
            |mem: &mut M, calls: &mut dyn FnMut(&mut M) -> u32, tables: [(u32, u32); 3]| {
                let lang = calls(mem);
                let idx = index + k.cast_signed();
                let name = match lang {
                    0 => at(mem, if high { tables[0].1 } else { tables[0].0 }, index),
                    9 => at(mem, if high { tables[1].1 } else { tables[1].0 }, index),
                    10 => at(mem, if high { tables[2].1 } else { tables[2].0 }, index),
                    _ => mem.read(k.wrapping_add(index.cast_unsigned().wrapping_mul(4))),
                };
                (name, idx, high)
            };
        let (name, idx, flag) = match id {
            0x3c00 => {
                let table = usize::from(calls.tk_lang(mem) != 0);
                let idx = index + k.cast_signed();
                (at(mem, pair(table, flagged), idx), idx, flagged)
            }
            0x3d00 => {
                let (src, dst) = if calls.battle_status(mem) == 0 {
                    (0x025d_08a0, 0x025d_07a0)
                } else {
                    (0x025d_06a0, 0x025d_05a0)
                };
                let (src, dst) = (mem.rebase(src), mem.rebase(dst));
                let mut n = 0u32;
                loop {
                    let c: u8 = mem.read(src.wrapping_add(n));
                    mem.write(dst.wrapping_add(n), c);
                    n = n.wrapping_add(1);
                    if c == 0 {
                        break;
                    }
                }
                let idx = index + k.cast_signed();
                let tail = at(mem, pair(6, flagged), idx);
                let len = c_strlen(mem, tail);
                let end = dst.wrapping_add(c_strlen(mem, dst));
                for i in 0..=len {
                    let c: u8 = mem.read(tail.wrapping_add(i));
                    mem.write(end.wrapping_add(i), c);
                }
                (intern_name(mem, dst), idx, flagged)
            }
            0x3d10 | 0x3d20 | 0x3d30 | 0x3d40 => {
                let table = match id {
                    0x3d10 => 2,
                    0x3d20 => 3,
                    0x3d30 => 4,
                    _ => 1,
                };
                let idx = index + k.cast_signed();
                (at(mem, pair(table, flagged), idx), idx, flagged)
            }
            0x3d01 => by_lang(
                mem,
                &mut |m| calls.ffx_lang(m),
                [
                    (0x00c5_9878, 0x00c5_9938),
                    (0x00c5_98f8, 0x00c5_99c0),
                    (0x00c5_98b8, 0x00c5_9980),
                ],
            ),
            0x3d60 => by_lang(
                mem,
                &mut |m| calls.ffx_lang(m),
                [
                    (0x00c5_9a00, 0x00c5_9b00),
                    (0x00c5_9ac0, 0x00c5_9b80),
                    (0x00c5_9a40, 0x00c5_9b40),
                ],
            ),
            0x3d80 => {
                let idx = byte.cast_signed();
                (at(mem, pair(5, high), idx), idx, high)
            }
            _ => {
                let fmt = mem.rebase(0x00b6_2dd0);
                calls.print(mem, fmt);
                (k, -1, false)
            }
        };
        let sizes = mem.rebase(if flag { 0x00b5_f8d0 } else { 0x00b5_f890 });
        let e = sizes.wrapping_add((idx / 2).cast_unsigned().wrapping_mul(8));
        let w: [u32; 2] = mem.read(e);
        mem.write(size, w);
        name
    }

    /// `g_TOMessageWindowSlots` (`0x01865b3c`): 8 message-window objects.
    pub const MESSAGE_SLOTS: Global<[u32; 8]> = Global::new(0x0186_5b3c);
    /// The message-window frame paddings (`0x00c59ed8`): per style, i16
    /// left, top, right, bottom.
    pub const FRAME_PADDING: u32 = 0x00c5_9ed8;
    /// `g_CurrentMapNo` (`0x012fbc40`).
    pub const CURRENT_MAP: Global<u32> = Global::new(0x012f_bc40);

    /// Message window `i`'s frame padding (left, top, right, bottom), by
    /// the style (`+4` of its object's `+0x10`).
    fn frame_padding(mem: &impl Mem, i: i32) -> [i32; 4] {
        let slot = MESSAGE_SLOTS
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(4));
        let obj: u32 = mem.read(slot);
        let style: u32 = mem.read(obj.wrapping_add(0x10));
        let s = i32::from(mem.read::<i16>(style.wrapping_add(4)));
        let at = mem
            .rebase(FRAME_PADDING)
            .wrapping_add(s.cast_unsigned().wrapping_mul(8));
        let p: [i16; 4] = mem.read(at);
        p.map(i32::from)
    }

    /// An x87 value through `fstp dword`, back on the x87.
    #[allow(clippy::cast_possible_truncation)] // the original rounds to f32
    fn dword(v: f64) -> f64 {
        f64::from(v as f32)
    }

    /// An `int` through `fild` and `fstp dword`, back on the x87.
    #[allow(clippy::cast_precision_loss)] // the original rounds to f32
    fn f32_of(v: i32) -> f64 {
        f64::from(v as f32)
    }

    /// `0x008afbb0(i, x, y, w, h)`: message window `i` placed. Outside
    /// message-window mode 1 the frame padding is taken off the rectangle
    /// and put back after the x87 rounding (`ftol`). On map 0x17 a window at
    /// (400, 0x110) moves by (-45, +5). With a screen (passed in) at least
    /// 0x2d0 high it is kept inside the safe area: vertically within
    /// 31.2..384.8 (by the padding's top, for window type 1); horizontally
    /// for type 1 within 7.5%..92.5% of the screen width mapped back
    /// through the 416-line scale and the 4:3 offset (±10), else within
    /// 38.4..473.6. Then `TOSetWindowRect` (map 0x15b turns a 0x1d8 x 0xf6
    /// window 0x1ba wide there only) and the message area, padded again.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn place_message_window<M: Mem>(
        mem: &mut M,
        i: i32,
        rect: [i32; 4],
        screen_wh: impl FnOnce(&mut M) -> (i32, i32),
    ) {
        use ffx_bridge::x87;
        let [mut x, mut y, mut w, mut h] = rect;
        let mode = crate::menu::bridge::message_window_mode(mem);
        if mode != 1 {
            let [l, t, r, b] = frame_padding(mem, i);
            x = x.wrapping_add(l);
            y = y.wrapping_add(t);
            w = w.wrapping_sub(r.wrapping_add(l));
            h = h.wrapping_sub(b.wrapping_add(t));
        }
        let mut fx = f32_of(x);
        let mut fy = f32_of(y);
        if CURRENT_MAP.get(mem) == 0x17 && x == 400 && y == 0x110 {
            fx = dword(x87::sub(fx, 45.0));
            fy = dword(x87::add(fy, 5.0));
        }
        let ftol = |mem: &M, v: f64| x87::ftol(mem, v).cast_signed();
        if mode == 1 {
            x = ftol(mem, fx);
            y = ftol(mem, fy);
        } else {
            let [l, t, r, b] = frame_padding(mem, i);
            x = ftol(mem, x87::sub(fx, f64::from(l)));
            y = ftol(mem, x87::sub(fy, f64::from(t)));
            w = w.wrapping_add(r.wrapping_add(l));
            h = h.wrapping_add(b.wrapping_add(t));
        }
        let (sw, sh) = screen_wh(mem);
        if sh >= 0x2d0 {
            let bottom = y.wrapping_add(h);
            let (l, t) = if crate::window::bridge::window_type(mem, i) == 1 {
                let p = frame_padding(mem, i);
                (p[0], p[1])
            } else {
                (0, 0)
            };
            let left = l.wrapping_add(x);
            let right = x.wrapping_add(w).wrapping_sub(l);
            let top = f32_of(y.wrapping_add(t));
            let low: f64 = 31.199_995_040_893_555;
            let high = f64::from(f32::from_bits(0x43c0_6667));
            if low <= top || top.is_nan() {
                let b = f32_of(bottom.wrapping_sub(t));
                if b > high {
                    y = ftol(
                        mem,
                        x87::add(x87::sub(384.800_018_310_546_9, f32_of(h)), f32_of(t)),
                    );
                }
            } else {
                y = ftol(mem, x87::sub(low, f32_of(t)));
            }
            let lf = f32_of(l);
            if crate::window::bridge::window_type(mem, i) == 1 {
                let shf = f32_of(sh);
                let swf = f32_of(sw);
                let scale = dword(x87::div(shf, 416.0));
                let q = x87::div(1.230_769_276_618_957_5, x87::div(swf, shf));
                let off = dword(x87::mul(x87::mul(swf, x87::sub(1.0, q)), 0.5));
                let lim = x87::mul(x87::mul(swf, 0.149_999_976_158_142_1), 0.5);
                let v = x87::sub(x87::add(x87::mul(scale, f32_of(left)), off), 1.0);
                if v < lim {
                    let u = x87::div(x87::sub(lim, off), scale);
                    x = ftol(mem, x87::add(x87::sub(u, lf), 10.0));
                } else {
                    let lim2 = x87::mul(x87::mul(swf, 1.850_000_023_841_858), 0.5);
                    let v = x87::add(1.0, x87::add(x87::mul(f32_of(right), scale), off));
                    if v > lim2 {
                        let u = x87::div(x87::sub(lim2, off), scale);
                        x = ftol(mem, x87::sub(x87::add(x87::sub(u, f32_of(w)), lf), 10.0));
                    }
                }
            } else if f32_of(left) < f64::from(f32::from_bits(0x4219_9998)) {
                x = ftol(mem, x87::sub(38.399_993_896_484_375, lf));
            } else if 473.600_006_103_515_6 < f32_of(right) {
                x = ftol(
                    mem,
                    x87::add(x87::sub(473.600_006_103_515_6, f32_of(w)), lf),
                );
            }
        }
        let shown_w = if CURRENT_MAP.get(mem) == 0x15b && w == 0x1d8 && h == 0xf6 {
            0x1ba
        } else {
            w
        };
        set_window_rect(mem, i, [x, y, shown_w, h]);
        if crate::menu::bridge::message_window_mode(mem) == 1 {
            crate::window::bridge::set_message_area(mem, i, [x, y, w, h]);
        } else {
            let [l, t, r, b] = frame_padding(mem, i);
            crate::window::bridge::set_message_area(
                mem,
                i,
                [
                    x.wrapping_add(l),
                    y.wrapping_add(t),
                    w.wrapping_sub(r).wrapping_sub(l),
                    h.wrapping_sub(b).wrapping_sub(t),
                ],
            );
        }
    }

    /// `g_TkMenuTutoChosen` (`0x025d16bc`).
    pub const TUTO_CHOSEN: Global<u32> = Global::new(0x025d_16bc);
    /// Per message window: 1 when [`set_message_window_position`] split a
    /// wide window into 300-unit rows (`0x025d0b20`).
    pub const MESSAGE_WRAPPED: Global<[u32; 8]> = Global::new(0x025d_0b20);
    /// A word (`0x01327104`) that, set, keeps a type-1 message window from
    /// being clamped horizontally.
    pub const MESSAGE_UNCLAMPED: Global<u32> = Global::new(0x0132_7104);

    /// `TOSetMessageWindowPosition(i, anchor, x, y)` (`0x008af850`): the
    /// anchor and position stored (u16 `+0xc`, `+8`, `+0xa` of the object
    /// and `+0x18`, `+0x14`, `+0x16` of its `+0x14` record); the size is the
    /// record's text size (`+0x40`, `+0x42`) plus the frame padding, and
    /// with the tutorial choice set a text wider than 0x190 becomes 300 wide
    /// with `width / 300 + 1` rows ([`MESSAGE_WRAPPED`]). The anchor moves
    /// the position (1 bottom, 2 right, 3 bottom right, 4 centre); the width
    /// is clamped to 0..=0x200 and `x` into the 512 columns (except for
    /// magic 0x281 on maps 0x5e and 0x12e, or a type-1 window while
    /// [`MESSAGE_UNCLAMPED`] is set); the height to 4..=0x7f (mode 1, 0x7b
    /// rows) or 4..=0x1a0 (0x19c rows) and `y` to 4.. that bound; then
    /// [`place_message_window`].
    #[allow(
        clippy::many_single_char_names,
        clippy::cast_possible_truncation,
        clippy::too_many_lines
    )] // 16-bit arithmetic
    pub fn set_message_window_position<M: Mem>(
        mem: &mut M,
        i: i32,
        anchor: u32,
        x: i32,
        y: i32,
        screen_wh: impl FnOnce(&mut M) -> (i32, i32),
    ) {
        let (mut x, mut y) = (x, y);
        let slot = MESSAGE_SLOTS
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(4));
        let obj: u32 = mem.read(slot);
        let low = |v: i32| v.cast_unsigned() as u16;
        let a16 = anchor as u16;
        mem.write(obj.wrapping_add(0xc), a16);
        mem.write(obj.wrapping_add(8), low(x));
        mem.write(obj.wrapping_add(0xa), low(y));
        let rec: u32 = mem.read(obj.wrapping_add(0x14));
        mem.write(rec.wrapping_add(0x18), a16);
        mem.write(rec.wrapping_add(0x14), low(x));
        mem.write(rec.wrapping_add(0x16), low(y));
        let [l, t, r, b] = frame_padding(mem, i).map(|v| v.cast_unsigned() as u16);
        let pad_w = u32::from(r.wrapping_add(l));
        let pad_h = u32::from(b.wrapping_add(t));
        let rec: u32 = mem.read(obj.wrapping_add(0x14));
        let text_w: u16 = mem.read(rec.wrapping_add(0x40));
        let text_h: u16 = mem.read(rec.wrapping_add(0x42));
        let wrapped = MESSAGE_WRAPPED
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(4));
        let s16 = |v: u32| i32::from((v as u16).cast_signed());
        let (rows_h, w_add) =
            if TUTO_CHOSEN.get(mem) != 0 && i32::from(text_w.cast_signed()) + s16(pad_w) > 0x190 {
                mem.write(wrapped, 1u32);
                let rows = i32::from(text_w.cast_signed()) / 300 + 1;
                let h = rows.wrapping_mul(i32::from(text_h)).cast_unsigned() as u16;
                (u32::from(h), 300u32)
            } else {
                mem.write(wrapped, 0u32);
                (u32::from(text_h), u32::from(text_w))
            };
        let mut h = pad_h.wrapping_add(rows_h);
        let w = pad_w.wrapping_add(w_add);
        let (sw, sh) = (s16(w), s16(h));
        match anchor.wrapping_sub(1) {
            0 => y = y.wrapping_sub(sh),
            1 => x = x.wrapping_sub(sw),
            2 => {
                x = x.wrapping_sub(sw);
                y = y.wrapping_sub(sh);
            }
            3 => {
                x = x.wrapping_sub(sw / 2);
                y = y.wrapping_sub(sh / 2);
            }
            _ => {}
        }
        let width = if sw > 0x200 {
            0x200
        } else if sw < 0 {
            0
        } else {
            w
        };
        let width = s16(width);
        let map = CURRENT_MAP.get(mem);
        let exempt = ((map == 0x5e || map == 0x12e)
            && crate::menu::bridge::MAGIC_FILE.get(mem)[0] == 0x281)
            || (MESSAGE_UNCLAMPED.get(mem) != 0 && crate::window::bridge::window_type(mem, i) == 1);
        if !exempt {
            let lim = 0x200 - width;
            x = if x > lim { lim } else { x.max(0) };
        }
        let rows = if crate::menu::bridge::message_window_mode(mem) == 1 {
            if s16(h) > 0x7f {
                h = 0x7f;
            } else if s16(h) < 4 {
                h = 4;
            }
            0x7b
        } else {
            if s16(h) > 0x1a0 {
                h = 0x1a0;
            } else if s16(h) < 4 {
                h = 4;
            }
            0x19c
        };
        let height = s16(h);
        let lim = rows - height;
        if y > lim {
            y = lim;
        } else if y < 4 {
            y = 4;
        }
        place_message_window(mem, i, [x, y, width, height], screen_wh);
    }

    /// `0x008afa90(i, anchor, x, y, w, h)`: message window `i` placed with
    /// an explicit size: the anchor moves the position (1 bottom, 2 right,
    /// 3 bottom right, 4 centre), `w` is clamped to 0..=0x200, `h` to
    /// 4..=0x19c, `x` into the 512 columns and `y` to 4..=`0x19c - h`; then
    /// [`place_message_window`].
    #[allow(clippy::many_single_char_names)]
    pub fn set_message_window_rect<M: Mem>(
        mem: &mut M,
        i: i32,
        anchor: u32,
        rect: [i32; 4],
        screen_wh: impl FnOnce(&mut M) -> (i32, i32),
    ) {
        let [mut x, mut y, w, h] = rect;
        match anchor.wrapping_sub(1) {
            0 => y = y.wrapping_sub(h),
            1 => x = x.wrapping_sub(w),
            2 => {
                x = x.wrapping_sub(w);
                y = y.wrapping_sub(h);
            }
            3 => {
                x = x.wrapping_sub(w / 2);
                y = y.wrapping_sub(h / 2);
            }
            _ => {}
        }
        let w = if w > 0x200 { 0x200 } else { w.max(0) };
        let h = h.clamp(4, 0x19c);
        let lim = 0x200 - w;
        let x = if x > lim { lim } else { x.max(0) };
        let lim = 0x19c - h;
        let y = if y > lim { lim } else { y.max(4) };
        place_message_window(mem, i, [x, y, w, h], screen_wh);
    }

    /// The calls the battle limit-window set-ups make outside this module.
    pub trait LimitWindowCalls<M> {
        /// `TOSeqGetNum(i, name, &n)`.
        fn seq_num(&mut self, mem: &mut M, i: u32, name: u32, out: u32);
        /// `TOSeqGetPoint(i, name, &x, &y)`.
        fn seq_point(&mut self, mem: &mut M, i: u32, name: u32, x: u32, y: u32);
        /// `TOSeqGetColor(i, name, &rgba)`.
        fn seq_color(&mut self, mem: &mut M, i: u32, name: u32, out: u32);
        /// `TOInitPadData()`.
        fn init_pad(&mut self, mem: &mut M);
        /// `TOBtlInitLimitTimer()`.
        fn init_limit_timer(&mut self, mem: &mut M);
        /// `graphicUiRemapX2(v)` or, with `y`, `graphicUiRemapY2(v)`.
        fn remap(&mut self, mem: &mut M, y: bool, v: f32) -> f32;
    }

    /// The common start of the limit-window set-ups: `TOSeqGetNum(2,
    /// "dumm..", 0x0133f778)`, `TOInitPadData()`, `TOBtlInitLimitTimer()`.
    fn limit_window_start<M: Mem>(mem: &mut M, calls: &mut impl LimitWindowCalls<M>) {
        let name = mem.rebase(0x00b5_ecbc);
        let out = mem.rebase(0x0133_f778);
        calls.seq_num(mem, 2, name, out);
        calls.init_pad(mem);
        calls.init_limit_timer(mem);
    }

    /// `words` words cleared from the image address `from`.
    fn zero_words<M: Mem>(mem: &mut M, from: u32, words: u32) {
        let at = mem.rebase(from);
        for k in 0..words {
            mem.write(at.wrapping_add(4 * k), 0u32);
        }
    }

    /// `TOSeqGetPoint(2, name, &x, &y)` into image addresses.
    fn limit_point<M: Mem>(mem: &mut M, calls: &mut impl LimitWindowCalls<M>, name: u32, x: u32) {
        let (name, x) = (mem.rebase(name), mem.rebase(x));
        calls.seq_point(mem, 2, name, x, x.wrapping_add(2));
    }

    /// `TOSeqGetColor(2, name, &rgba)` into an image address.
    fn limit_color<M: Mem>(mem: &mut M, calls: &mut impl LimitWindowCalls<M>, name: u32, out: u32) {
        let (name, out) = (mem.rebase(name), mem.rebase(out));
        calls.seq_color(mem, 2, name, out);
    }

    /// `TOBtlInitAuronLimitWindow()` (`0x00897970`): the common start, the
    /// window record `0x0133d6f4` (5 words) cleared, its point from the
    /// scene (`+2`, `+4`) and the word `+6` 4.
    pub fn btl_init_auron_limit_window<M: Mem>(mem: &mut M, calls: &mut impl LimitWindowCalls<M>) {
        limit_window_start(mem, calls);
        zero_words(mem, 0x0133_d6f4, 5);
        limit_point(mem, calls, 0x00b5_ecc4, 0x0133_d6f6);
        let at = mem.rebase(0x0133_d6fa);
        mem.write(at, 4u32);
    }

    /// `TOBtlInitLuluLimitWindow()` (`0x00897f70`): the common start, the
    /// record `0x0133d708` (0x2c bytes) cleared, a point (`+2`), `+6` 0xc0,
    /// two colours (`+8`, `+0xc`) and a second point (`+0x10`).
    pub fn btl_init_lulu_limit_window<M: Mem>(mem: &mut M, calls: &mut impl LimitWindowCalls<M>) {
        limit_window_start(mem, calls);
        zero_words(mem, 0x0133_d708, 11);
        limit_point(mem, calls, 0x00b5_ecf4, 0x0133_d70a);
        let at = mem.rebase(0x0133_d70e);
        mem.write(at, 0xc0u16);
        limit_color(mem, calls, 0x00b5_ecfc, 0x0133_d710);
        limit_color(mem, calls, 0x00b5_ed04, 0x0133_d714);
        limit_point(mem, calls, 0x00b5_ed0c, 0x0133_d718);
    }

    /// `TOBtlInitTidusLimitWindow()` (`0x00898180`): the common start, the
    /// record `0x0133d734` (6 words) cleared, `+2`, `+4`, `+6` the remapped
    /// (`graphicUiRemapX2`/`Y2`, then `ftol`) constants at `0x00b5ef70`,
    /// `0x00b5ef98`, `0x00b5f010`, and a colour at `+0x14`.
    #[allow(clippy::cast_possible_truncation)] // `mov word, ax`
    pub fn btl_init_tidus_limit_window<M: Mem>(mem: &mut M, calls: &mut impl LimitWindowCalls<M>) {
        limit_window_start(mem, calls);
        zero_words(mem, 0x0133_d734, 6);
        for (k, (c, y)) in [
            (0x00b5_ef70, false),
            (0x00b5_ef98, true),
            (0x00b5_f010, false),
        ]
        .into_iter()
        .enumerate()
        {
            let c = mem.rebase(c);
            let v: f32 = mem.read(c);
            let r = calls.remap(mem, y, v);
            let w = ffx_bridge::x87::ftol(mem, f64::from(r)) as u16;
            let at = mem.rebase(0x0133_d736).wrapping_add(2 * k as u32);
            mem.write(at, w);
        }
        limit_color(mem, calls, 0x00b5_ece4, 0x0133_d748);
    }

    /// `0x00897860()` (from `TOInitBtlWindow`): the 24 floats at
    /// `0x0133fa08` set from constants (0, 1 and six image floats), then
    /// four colours from the scene (`0x0133f7d4`, `0x0133fa88`,
    /// `0x0133fa8c`, `0x0133fa90`).
    pub fn btl_init_active_chr_cursor<M: Mem>(mem: &mut M, calls: &mut impl LimitWindowCalls<M>) {
        // which value each float gets: 0 = 0.0, 1 = 1.0, else an image float
        const SOURCES: [u32; 24] = [
            0,
            0x00b4_9998,
            0,
            1,
            0x00b9_2340,
            0x00b5_f048,
            0,
            1,
            1,
            0x00b5_f048,
            0,
            1,
            0,
            0,
            0x00b1_0f38,
            1,
            0x00b5_165c,
            0x00b5_f044,
            0x00b1_0f38,
            1,
            0x00b4_3d28,
            0x00b5_f044,
            0x00b1_0f38,
            1,
        ];
        let base = mem.rebase(0x0133_fa08);
        for (k, s) in SOURCES.into_iter().enumerate() {
            let bits = match s {
                0 => 0u32,
                1 => 1.0f32.to_bits(),
                a => {
                    let a = mem.rebase(a);
                    mem.read(a)
                }
            };
            #[allow(clippy::cast_possible_truncation)] // 24 entries
            let at = base.wrapping_add(4 * k as u32);
            mem.write(at, bits);
        }
        for (name, out) in [
            (0x00b5_ed34, 0x0133_f7d4),
            (0x00b5_ed3c, 0x0133_fa88),
            (0x00b5_ed44, 0x0133_fa8c),
            (0x00b5_ed4c, 0x0133_fa90),
        ] {
            limit_color(mem, calls, name, out);
        }
    }

    /// Whose overdrive window [`LimitOpenCalls::limit_time`] times.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Limit {
        /// `NkzGetAuronLimitTime(level)`.
        Auron,
        /// `NkzGetLuluLimitTime(group)`.
        Lulu,
        /// `NkzGetTidusLimitTime(level)`.
        Tidus,
    }

    /// The overdrive-table calls of the limit-window openers.
    pub trait LimitOpenCalls<M> {
        /// The time limit (`NkzGet*LimitTime(arg)`).
        fn limit_time(&mut self, mem: &mut M, who: Limit, arg: u32) -> f32;
        /// `NkzGetAuronLimitButtonData(level)`.
        fn auron_buttons(&mut self, mem: &mut M, level: u32) -> u32;
        /// `NkzSetLuluLimitTbl(group)`.
        fn lulu_table(&mut self, mem: &mut M, group: u32);
        /// `NkzTidusLimitTableA(level)` or, with `b`, `NkzTidusLimitTableB`.
        fn tidus_table(&mut self, mem: &mut M, b: bool, level: u32) -> u32;
    }

    /// The level of an overdrive command `cmd`: `base`, `base + 0x10`,
    /// `+ 0x20`, `+ 0x30` give 0..=3, anything else 0.
    fn limit_level(cmd: u32, base: u32) -> u32 {
        match cmd.wrapping_sub(base) {
            0x10 => 1,
            0x20 => 2,
            0x30 => 3,
            _ => 0,
        }
    }

    /// The limit timer started (`0x0133f77c` 1) with `t` as its three
    /// times (`0x0133f78c`, `0x0133f790`, `0x0133f794`).
    fn start_limit_timer(mem: &mut impl Mem, t: f32) {
        for (a, v) in [
            (0x0133_f78c, t.to_bits()),
            (0x0133_f77c, 1),
            (0x0133_f790, t.to_bits()),
            (0x0133_f794, t.to_bits()),
        ] {
            let a = mem.rebase(a);
            mem.write(a, v);
        }
    }

    /// `TOBtlOpenAuronLimitWindow(cmd)` (`0x00898ad0`): the level from the
    /// command (0x22, 0x32, 0x42, 0x52), the timer started with its time,
    /// the button sequence (`NkzGetAuronLimitButtonData`) and its length up
    /// to the 0xffff end in `0x0133d704`/`0x0133d700`, the window open
    /// (`0x0133d6f4` 1). Returns 1.
    pub fn btl_open_auron_limit_window<M: Mem>(
        mem: &mut M,
        cmd: u32,
        calls: &mut impl LimitOpenCalls<M>,
    ) -> u32 {
        let level = limit_level(cmd, 0x22);
        let t = calls.limit_time(mem, Limit::Auron, level);
        start_limit_timer(mem, t);
        let seq = calls.auron_buttons(mem, level);
        let (seq_at, len_at) = (mem.rebase(0x0133_d704), mem.rebase(0x0133_d700));
        mem.write(seq_at, seq);
        let mut n = 0u32;
        mem.write(len_at, n);
        while mem.read::<u16>(seq.wrapping_add(n.wrapping_mul(2))) != 0xffff {
            n = n.wrapping_add(1);
            mem.write(len_at, n);
        }
        let open = mem.rebase(0x0133_d6f4);
        mem.write(open, 1u16);
        1
    }

    /// `TOBtlOpenLuluLimitWindow(cmd)` (`0x00898bf0`): the group `cmd -
    /// 0x3078`, the timer started with its time, `NkzSetLuluLimitTbl`, then
    /// `0x0133d71e` the byte `0x0133f798`, `0x0133d71c` 3, `0x0133d71f` 1 and
    /// the window open (`0x0133d708` 1). Returns 1.
    pub fn btl_open_lulu_limit_window<M: Mem>(
        mem: &mut M,
        cmd: u32,
        calls: &mut impl LimitOpenCalls<M>,
    ) -> u32 {
        let group = cmd.wrapping_sub(0x3078);
        let t = calls.limit_time(mem, Limit::Lulu, group);
        start_limit_timer(mem, t);
        calls.lulu_table(mem, group);
        let src = mem.rebase(0x0133_f798);
        let v: u8 = mem.read(src);
        let w = mem.rebase(0x0133_d708);
        mem.write(w.wrapping_add(0x16), v);
        mem.write(w.wrapping_add(0x14), 3u16);
        mem.write(w.wrapping_add(0x17), 1u8);
        mem.write(w, 1u16);
        1
    }

    /// `TOBtlOpenTidusLimitWindow(cmd)` (`0x00898ca0`): the level from the
    /// command (0x20, 0x30, 0x40, 0x50), the timer started with its time,
    /// table B's value at `+0x10` of the window `0x0133d734`, its `+2` copied
    /// to `+0xe`, table A's width at `+0xc` and the band `+8`..`+0xa`
    /// centred on `+6` / 2 from `+2`; the window open. Returns 1.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::many_single_char_names
    )] // 16-bit stores
    pub fn btl_open_tidus_limit_window<M: Mem>(
        mem: &mut M,
        cmd: u32,
        calls: &mut impl LimitOpenCalls<M>,
    ) -> u32 {
        let level = limit_level(cmd, 0x20);
        let t = calls.limit_time(mem, Limit::Tidus, level);
        start_limit_timer(mem, t);
        let w = mem.rebase(0x0133_d734);
        let b = calls.tidus_table(mem, true, level);
        mem.write(w.wrapping_add(0x10), b as u16);
        let x: u16 = mem.read(w.wrapping_add(2));
        mem.write(w.wrapping_add(0xe), x);
        let a = calls.tidus_table(mem, false, level) as u16;
        let width: i16 = mem.read(w.wrapping_add(6));
        let half = i32::from(width) / 2;
        let ha = i32::from(a.cast_signed()) / 2;
        mem.write(w.wrapping_add(0xc), a);
        let x: u16 = mem.read(w.wrapping_add(2));
        mem.write(w.wrapping_add(8), ((half - ha) as u16).wrapping_add(x));
        mem.write(w.wrapping_add(0xa), ((ha + half) as u16).wrapping_add(x));
        mem.write(w, 1u16);
        1
    }

    /// `TOBtlCtrlLimitTimer()` (`0x00891ac0`): while the limit timer runs
    /// (`0x0133f77c` 2) a frame is counted (`0x0133f788` += 1) and the time
    /// left (`0x0133f790`) is the limit (`0x0133f78c`) less the frames over
    /// 30 (25 in video mode 1, PAL). At 0 or below both it and the shown
    /// time (`0x0133f794`) become 0; otherwise the shown time gets a random
    /// hundredths digit: `(rnd() % 10 - 10 * ftol(left * -10)) / 100` (the
    /// battle `rnd`, passed in). A NaN time takes the second path, as the
    /// original's `fcom` does.
    #[allow(clippy::cast_possible_truncation)] // values already rounded to f32
    pub fn btl_ctrl_limit_timer<M: Mem>(mem: &mut M, rnd: impl FnOnce(&mut M) -> u32) {
        use ffx_bridge::x87;
        let g = |mem: &M, a: u32| mem.rebase(a);
        if mem.read::<u32>(g(mem, 0x0133_f77c)) != 2 {
            return;
        }
        let frames_at = g(mem, 0x0133_f788);
        let frames = f64::from(mem.read::<f32>(frames_at));
        let frames = dword(x87::add(frames, 1.0));
        mem.write(frames_at, frames as f32);
        let per_second = if crate::config::bridge::video_mode(mem) == 0 {
            30.0
        } else {
            25.0
        };
        let limit = f64::from(mem.read::<f32>(g(mem, 0x0133_f78c)));
        let left = dword(x87::sub(limit, x87::div(frames, per_second)));
        let left_at = g(mem, 0x0133_f790);
        let shown_at = g(mem, 0x0133_f794);
        mem.write(left_at, left as f32);
        if left <= 0.0 {
            mem.write(left_at, 0f32);
            mem.write(shown_at, 0f32);
            return;
        }
        let digit = rnd(mem).cast_signed() % 10;
        let tens = x87::ftol(mem, x87::mul(left, -10.0)).cast_signed();
        let v = digit.wrapping_sub(tens.wrapping_mul(10));
        let shown = dword(x87::div(f32_of(v), 100.0));
        mem.write(shown_at, shown as f32);
    }

    /// `TOBtlMemoryCursorPos()` (`0x008989d0`): unless the command
    /// `0x0133ca4c` is 0xffff, 0xfffe or 0x3002, the cursor state of the
    /// `0x023cc092` + 1 command windows (0xf0 bytes each from `0x0133c952`)
    /// is saved for character `0x023cc088` (0x168 bytes each from
    /// `0x0133d788`, 0x24 per window): the window kind, the depth and its
    /// per-level positions and scrolls, and five more cursor words; the
    /// character's saved flag (`0x0133d770 + i`) set.
    #[allow(clippy::cast_possible_truncation)] // the image is 32-bit
    pub fn btl_memory_cursor_pos(mem: &mut impl Mem) {
        let cmd: u16 = mem.read(mem.rebase(0x0133_ca4c));
        if matches!(cmd, 0xffff | 0xfffe | 0x3002) {
            return;
        }
        let n = i32::from(mem.read::<i8>(mem.rebase(0x023c_c092)));
        let chr = i32::from(mem.read::<i16>(mem.rebase(0x023c_c088)));
        if n >= 0 {
            let mut dst = mem
                .rebase(0x0133_d78a)
                .wrapping_add(chr.wrapping_mul(0x168).cast_unsigned());
            let mut src = mem.rebase(0x0133_c95a);
            for _ in 0..=n {
                let kind = i16::from(mem.read::<i8>(src.wrapping_sub(8)));
                mem.write(dst.wrapping_sub(2), kind);
                let depth: i8 = mem.read(src);
                mem.write(dst, i16::from(depth));
                for j in 0..i32::from(depth).max(0).cast_unsigned() {
                    let a: u16 = mem.read(src.wrapping_add(4 + 2 * j));
                    mem.write(dst.wrapping_add(2 + 2 * j), a);
                    let b: u16 = mem.read(src.wrapping_add(0xc + 2 * j));
                    mem.write(dst.wrapping_add(0xa + 2 * j), b);
                }
                let v: u16 = mem.read(src.wrapping_add(2));
                mem.write(dst.wrapping_add(0x12), v);
                let v: u32 = mem.read(src.wrapping_add(0xce));
                mem.write(dst.wrapping_add(0x1a), v);
                for (from, to) in [(0xca, 0x16), (0xcc, 0x18), (0x38, 0x1e), (0x36, 0x14)] {
                    let v: u16 = mem.read(src.wrapping_add(from));
                    mem.write(dst.wrapping_add(to), v);
                }
                dst = dst.wrapping_add(0x24);
                src = src.wrapping_add(0xf0);
            }
        }
        let flag = mem.rebase(0x0133_d770).wrapping_add(chr.cast_unsigned());
        mem.write(flag, 1u8);
    }

    /// `TOBtlMakeTargetGroupBit()` (`0x00898490`): the 28 group masks at
    /// `0x0133d0e4` cleared and rebuilt by the selected command's group mode
    /// (`+0x1a` of its `MsGetComData` record, bits 2-3, kept in
    /// `0x0133d0e0`): 0 each character of the target union (`0x0133d0dc`)
    /// alone; 1 the party (`0x0133d0d4`, characters 0..20) and the
    /// monsters (`0x0133d0d8`, 20..28) by `MsGetChrGroup` (passed in), the
    /// monsters' groups from mask 20; 2 the party and the monsters as two
    /// groups (masks 0 and 20); 3 the cursor's character (`0x0133d0d0`)
    /// alone.
    pub fn btl_make_target_group_bit<M: Mem>(
        mem: &mut M,
        tmp: u32,
        get_com: impl FnOnce(&mut M, u32, u32) -> u32,
        mut chr_group: impl FnMut(&mut M, u32) -> u32,
    ) {
        let id = u32::from(TARGET_COMMAND.get(mem));
        let rec = get_com(mem, id, tmp);
        let groups = mem.rebase(0x0133_d0e4);
        for k in 0..28u32 {
            mem.write(groups.wrapping_add(4 * k), 0u32);
        }
        let flags: u16 = mem.read(rec.wrapping_add(0x1a));
        let mode = u32::from(flags >> 2) & 3;
        let mode_at = mem.rebase(0x0133_d0e0);
        mem.write(mode_at, mode);
        let word = |mem: &M, a: u32| -> u32 { mem.read(mem.rebase(a)) };
        let or = |mem: &mut M, at: u32, bit: u32| {
            let v: u32 = mem.read(at);
            mem.write(at, v | bit);
        };
        match mode {
            0 => {
                for chr in 0..28u32 {
                    let bit = 1u32 << chr;
                    if word(mem, 0x0133_d0dc) & bit != 0 {
                        mem.write(groups.wrapping_add(4 * chr), bit);
                    }
                }
            }
            1 => {
                for (range, mask, base) in [(0..20u32, 0x0133_d0d4, 0), (20..28, 0x0133_d0d8, 20)] {
                    for chr in range {
                        let bit = 1u32 << chr;
                        if word(mem, mask) & bit != 0 {
                            let g = chr_group(mem, chr);
                            let at = groups
                                .wrapping_add(4 * base)
                                .wrapping_add(g.wrapping_mul(4));
                            or(mem, at, bit);
                        }
                    }
                }
            }
            2 => {
                let party = word(mem, 0x0133_d0d4);
                mem.write(groups, party);
                let monsters = word(mem, 0x0133_d0d8);
                mem.write(groups.wrapping_add(0x50), monsters);
            }
            _ => {
                let c = u32::from(mem.read::<u16>(mem.rebase(0x0133_d0d0)));
                mem.write(groups.wrapping_add(c.wrapping_mul(4)), 1u32 << (c & 31));
            }
        }
    }

    /// `TOBtlMakeTargetTargetBit()` (`0x00898640`): the target lists
    /// rebuilt from the group masks (`0x0133d0e4`, 28 words) and the
    /// selectable mask (`0x0133d174` = the AND of `0x0133d154`,
    /// `0x0133d168`, `0x0133d16c`, `0x0133d170`): party targets into
    /// `0x0133d180` (count `0x0133d17c`), monster targets into
    /// `0x0133d1d4` (count `0x0133d1d0`), by the group mode `0x0133d0e0`:
    /// 0 the groups listed at `0x0133f0dc`/`0x0133f0f0` (counts
    /// `0x0133f0d8`/`0x0133f0ec`), 1 groups 0..8 and 20..28, 2 masks 0 and
    /// 20, 3 the cursor character (`0x0133d0d0`) alone. The command's flags
    /// (`+0x1a` of its `MsGetComData` record, passed in with a scratch word)
    /// pick the list: bit 1 the monsters', else the party's, where mode 0
    /// starts at the cursor character's entry. Then the list (`0x0133d1f8`,
    /// `0x0133d200`), its count (`0x0133d1f6`), the start (`0x0133d1fc`), bit
    /// 5 (`0x0133d1fe`), the selected mask (`0x0133d178`) and its highest
    /// character (`0x0133d1f4`, 0xff in modes 1 and 2), and those into the
    /// command window `0x023cc092` (0xf0 bytes each from `0x0133c992`).
    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)] // 16-bit counters
    pub fn btl_make_target_target_bit<M: Mem>(
        mem: &mut M,
        tmp: u32,
        get_com: impl FnOnce(&mut M, u32, u32) -> u32,
    ) {
        let at = |mem: &M, a: u32| mem.rebase(a);
        let party = at(mem, 0x0133_d180);
        let monsters = at(mem, 0x0133_d1d4);
        for k in 0..20u32 {
            mem.write(party.wrapping_add(4 * k), 0u32);
        }
        for k in 0..8u32 {
            mem.write(monsters.wrapping_add(4 * k), 0u32);
        }
        let id = u32::from(TARGET_COMMAND.get(mem));
        let rec = get_com(mem, id, tmp);
        let word = |mem: &M, a: u32| -> u32 { mem.read(mem.rebase(a)) };
        let mask = word(mem, 0x0133_d16c)
            & word(mem, 0x0133_d168)
            & word(mem, 0x0133_d170)
            & word(mem, 0x0133_d154);
        let mode = word(mem, 0x0133_d0e0);
        let (mask_at, n1_at, n2_at) = (
            at(mem, 0x0133_d174),
            at(mem, 0x0133_d17c),
            at(mem, 0x0133_d1d0),
        );
        mem.write(mask_at, mask);
        mem.write(n1_at, 0u16);
        mem.write(n2_at, 0u16);
        let groups = at(mem, 0x0133_d0e4);
        // one target added to a list: the count re-read and stored as
        // the original does
        let add = |mem: &mut M, list: u32, n_at: u32, n: &mut u16, v: u32| {
            let i = u32::from(*n).cast_signed() as i16;
            mem.write(list.wrapping_add((i32::from(i) * 4).cast_unsigned()), v);
            let c: u16 = mem.read(n_at);
            *n = c.wrapping_add(1);
            mem.write(n_at, *n);
        };
        let (mut n1, mut n2) = (0u16, 0u16);
        match mode {
            0 => {
                // the other count is re-read after a list that had entries
                // to scan (a long list can run into it)
                let count = word(mem, 0x0133_f0d8).cast_signed();
                if count > 0 {
                    let ids = at(mem, 0x0133_f0dc);
                    for k in 0..count.cast_unsigned() {
                        let g = u32::from(mem.read::<u16>(ids.wrapping_add(2 * k)));
                        let m: u32 = mem.read(mask_at);
                        let v = mem.read::<u32>(groups.wrapping_add(4 * g)) & m;
                        if v != 0 {
                            add(mem, party, n1_at, &mut n1, v);
                        }
                    }
                    n2 = mem.read(n2_at);
                }
                let count = word(mem, 0x0133_f0ec).cast_signed();
                if count > 0 {
                    let ids = at(mem, 0x0133_f0f0);
                    for k in 0..count.cast_unsigned() {
                        let g = u32::from(mem.read::<u16>(ids.wrapping_add(2 * k)));
                        let m: u32 = mem.read(mask_at);
                        let v = mem.read::<u32>(groups.wrapping_add(4 * g)) & m;
                        if v != 0 {
                            add(mem, monsters, n2_at, &mut n2, v);
                        }
                    }
                    n1 = mem.read(n1_at);
                }
            }
            1 => {
                for g in 0..8u32 {
                    let m: u32 = mem.read(mask_at);
                    let v = mem.read::<u32>(groups.wrapping_add(4 * g)) & m;
                    if v != 0 {
                        add(mem, party, n1_at, &mut n1, v);
                    }
                }
                n2 = mem.read(n2_at);
                for g in 20..28u32 {
                    let m: u32 = mem.read(mask_at);
                    let v = mem.read::<u32>(groups.wrapping_add(4 * g)) & m;
                    if v != 0 {
                        add(mem, monsters, n2_at, &mut n2, v);
                    }
                }
                n1 = mem.read(n1_at);
            }
            2 => {
                let v = mem.read::<u32>(groups) & mask;
                if v != 0 {
                    n1 = 1;
                    mem.write(party, v);
                    mem.write(n1_at, 1u16);
                }
                let v = mem.read::<u32>(groups.wrapping_add(0x50)) & mask;
                if v != 0 {
                    n2 = 1;
                    mem.write(n2_at, 1u16);
                    mem.write(monsters, v);
                }
            }
            3 => {
                let c: u16 = mem.read(at(mem, 0x0133_d0d0));
                let bit = 1u32 << (c & 31);
                if c < 0x14 {
                    n1 = 1;
                    mem.write(n1_at, 1u16);
                    mem.write(party, bit);
                } else if c < 0x1c {
                    n2 = 1;
                    mem.write(n2_at, 1u16);
                    mem.write(monsters, bit);
                }
            }
            _ => {}
        }
        let (start_at, count_at, which_at, list_at, bit5_at) = (
            at(mem, 0x0133_d1fc),
            at(mem, 0x0133_d1f6),
            at(mem, 0x0133_d200),
            at(mem, 0x0133_d1f8),
            at(mem, 0x0133_d1fe),
        );
        mem.write(start_at, 0u16);
        let flags: u8 = mem.read(rec.wrapping_add(0x1a));
        let mut start = 0u16;
        let (list, which) = if flags & 2 != 0 {
            mem.write(count_at, n2);
            mem.write(which_at, 1u16);
            mem.write(list_at, monsters);
            (monsters, 1u16)
        } else {
            mem.write(which_at, 0u16);
            mem.write(list_at, party);
            mem.write(count_at, n1);
            if mode == 0 {
                let n = i32::from(n1.cast_signed());
                let c: u8 = mem.read(at(mem, 0x0133_d0d0));
                let bit = 1u32 << (c & 31);
                for k in 0..n.max(0).cast_unsigned() {
                    if mem.read::<u32>(party.wrapping_add(4 * k)) == bit {
                        start = k as u16;
                        mem.write(start_at, start);
                        break;
                    }
                }
            }
            (party, 0)
        };
        mem.write(bit5_at, u16::from(flags & 0x20 != 0));
        let sel: u32 =
            mem.read(list.wrapping_add((i32::from(start.cast_signed()) * 4).cast_unsigned()));
        mem.write(at(mem, 0x0133_d178), sel);
        let top_at = at(mem, 0x0133_d1f4);
        if mode == 0 || mode == 3 {
            for c in 0..0x1cu16 {
                if sel & (1 << c) != 0 {
                    mem.write(top_at, c);
                }
            }
        } else {
            mem.write(top_at, 0xffu16);
        }
        let depth = i32::from(mem.read::<i8>(at(mem, 0x023c_c092)));
        let w = at(mem, 0x0133_c992).wrapping_add(depth.wrapping_mul(0xf0).cast_unsigned());
        mem.write(w.wrapping_add(0x96), sel);
        mem.write(w.wrapping_add(0x92), which);
        mem.write(w.wrapping_add(0x94), mode as u16);
        mem.write(w, start);
    }

    /// `0x0089a2a0(t)`: the limit timer started (`0x0133f77c` 1) with `t`
    /// as its limit, time left and shown time (as [`start_limit_timer`]
    /// in another order: the flag first).
    pub fn btl_start_limit_timer(mem: &mut impl Mem, t: f32) {
        for (a, v) in [
            (0x0133_f77c, 1),
            (0x0133_f78c, t.to_bits()),
            (0x0133_f790, t.to_bits()),
            (0x0133_f794, t.to_bits()),
        ] {
            let a = mem.rebase(a);
            mem.write(a, v);
        }
    }

    /// `0x0089a2d0(x, y)`: the limit timer's position (u16 `0x0133f780`,
    /// `0x0133f782`).
    #[allow(clippy::cast_possible_truncation)] // `mov word`
    pub fn btl_set_limit_timer_pos(mem: &mut impl Mem, x: u32, y: u32) {
        let at = mem.rebase(0x0133_f780);
        mem.write(at, x as u16);
        mem.write(at.wrapping_add(2), y as u16);
    }

    /// `TOBtlSetDefaultCursorPos()` (`0x00899ac0`): the command window
    /// `0x023cc092` restored from character `0x023cc088`'s saved cursor
    /// (see [`btl_memory_cursor_pos`]) when the saved key (`+0x14`) still
    /// matches the window's (`+0xa`). A window still at kind 0 looks its
    /// saved command (`+0xc` + 2 × level) up in its list (`+0x20`, count
    /// `+0x24`): found, it takes the kind, the position, the level's scroll
    /// (read through the found index as a window number, as compiled), the
    /// row scroll clamped to the list's pages (columns `+0x38`, rows
    /// `+0x3a`), the position and the saved target mask; not found, the
    /// scroll and position words are cleared. A window of kind 1 with a
    /// saved mask then selects the target list entry by the saved target
    /// mode (`+0x1a`; `+0x18` picks the monster list): 0 the entry equal to
    /// the mask, with its highest character; 1 the first entry sharing a
    /// bit; 2 the first entry. A zero column count makes the original fault
    /// (`idiv`); here it gives a zero quotient.
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::many_single_char_names
    )] // 16-bit stores
    pub fn btl_set_default_cursor_pos(mem: &mut impl Mem) {
        let d = i32::from(mem.read::<i8>(mem.rebase(0x023c_c092)));
        let c = i32::from(mem.read::<i16>(mem.rebase(0x023c_c088)));
        let e = d.wrapping_add(c.wrapping_mul(10));
        let saved = mem
            .rebase(0x0133_d788)
            .wrapping_add(e.wrapping_mul(0x24).cast_unsigned());
        let win = mem
            .rebase(0x0133_c952)
            .wrapping_add(d.wrapping_mul(0xf0).cast_unsigned());
        let key: u16 = mem.read(saved.wrapping_add(0x14));
        if mem.read::<u16>(win.wrapping_add(0xa)) != key {
            return;
        }
        if mem.read::<u8>(win) == 0 {
            let n = mem.read::<u32>(win.wrapping_add(0x22)).cast_signed();
            let mut found = None;
            if n > 0 {
                let lvl = i32::from(mem.read::<i8>(win.wrapping_add(9)));
                let at = saved
                    .wrapping_add(0xc)
                    .wrapping_add((lvl * 2).cast_unsigned());
                let target = u32::from(mem.read::<u16>(at));
                let list: u32 = mem.read(win.wrapping_add(0x1e));
                for j in 0..n {
                    let v = i32::from(mem.read::<i16>(list.wrapping_add((j * 2).cast_unsigned())));
                    if target == v.cast_unsigned() {
                        found = Some(j);
                        break;
                    }
                }
            }
            match found {
                Some(j) => {
                    let kind: u8 = mem.read(saved);
                    mem.write(win, kind);
                    let lvl = i32::from(mem.read::<i8>(win.wrapping_add(9)));
                    mem.write(
                        win.wrapping_add(0xc)
                            .wrapping_add((lvl * 2).cast_unsigned()),
                        j as u16,
                    );
                    let lvl = i32::from(mem.read::<i8>(win.wrapping_add(9)));
                    // the scroll is read through the found index as a window number
                    let other = mem
                        .rebase(0x0133_c95b)
                        .wrapping_add(j.wrapping_mul(0xf0).cast_unsigned());
                    let olvl = i32::from(mem.read::<i8>(other));
                    let src = mem.rebase(0x0133_d794).wrapping_add(
                        (olvl.wrapping_add(e.wrapping_mul(0x12)) * 2).cast_unsigned(),
                    );
                    let v: u16 = mem.read(src);
                    mem.write(
                        win.wrapping_add(0x14)
                            .wrapping_add((lvl * 2).cast_unsigned()),
                        v,
                    );
                    mem.write(win.wrapping_add(0xa), key);
                    let cols_u = mem.read::<u16>(win.wrapping_add(0x36));
                    let cols = i32::from(cols_u.cast_signed());
                    let q = j.checked_div(cols).unwrap_or(0);
                    let scroll_at = win.wrapping_add(0x3e);
                    mem.write(
                        scroll_at,
                        (q.wrapping_sub(1)).wrapping_mul(i32::from(cols_u)) as u16,
                    );
                    let cols = i32::from(mem.read::<i16>(win.wrapping_add(0x36)));
                    let rows = i32::from(mem.read::<i16>(win.wrapping_add(0x38)));
                    let n = mem.read::<u32>(win.wrapping_add(0x22)).cast_signed();
                    if n < rows.wrapping_mul(cols) {
                        mem.write(scroll_at, 0u16);
                    } else {
                        if j < cols {
                            mem.write(scroll_at, 0u16);
                        }
                        let cols = i32::from(mem.read::<i16>(win.wrapping_add(0x36)));
                        let n = mem.read::<u32>(win.wrapping_add(0x22)).cast_signed();
                        let pages = n
                            .wrapping_sub(1)
                            .wrapping_add(cols)
                            .checked_div(cols)
                            .unwrap_or(0);
                        if j > pages.wrapping_sub(1).wrapping_mul(cols).wrapping_sub(1) {
                            let rows: u16 = mem.read(win.wrapping_add(0x38));
                            let cols: u16 = mem.read(win.wrapping_add(0x36));
                            let v = (pages as u16).wrapping_sub(rows).wrapping_mul(cols);
                            mem.write(scroll_at, v);
                        }
                    }
                    mem.write(win.wrapping_add(0x40), j as u16);
                    let mask: u32 = mem.read(saved.wrapping_add(0x1c));
                    mem.write(win.wrapping_add(0xd6), mask);
                }
                None => mem.write(win.wrapping_add(0x3e), 0u32),
            }
        }
        if mem.read::<u8>(win) != 1 {
            return;
        }
        let mask: u32 = mem.read(saved.wrapping_add(0x1c));
        if mask == 0 {
            return;
        }
        let party = mem.rebase(0x0133_d180);
        let monsters = mem.rebase(0x0133_d1d4);
        let (start_at, which_at, sel_at, top_at, count_at, list_at) = (
            mem.rebase(0x0133_d1fc),
            mem.rebase(0x0133_d200),
            mem.rebase(0x0133_d178),
            mem.rebase(0x0133_d1f4),
            mem.rebase(0x0133_d1f6),
            mem.rebase(0x0133_d1f8),
        );
        let (party_n_at, monster_n_at) = (mem.rebase(0x0133_d17c), mem.rebase(0x0133_d1d0));
        let monster_side = mem.read::<u16>(saved.wrapping_add(0x18)) != 0;
        let (list, n_at, which, len, tops) = if monster_side {
            (monsters, monster_n_at, 1u16, 8u32, 20u16..28)
        } else {
            (party, party_n_at, 0u16, 20, 0..20)
        };
        match mem.read::<i16>(saved.wrapping_add(0x1a)) {
            0 => {
                let Some(i) = (0..len).find(|&i| mem.read::<u32>(list.wrapping_add(4 * i)) == mask)
                else {
                    return;
                };
                mem.write(which_at, which);
                mem.write(start_at, i as u16);
                let sel: u32 = mem.read(list.wrapping_add(4 * i));
                mem.write(sel_at, sel);
                for c in tops {
                    if sel & (1 << c) != 0 {
                        mem.write(top_at, c);
                    }
                }
                let n: u16 = mem.read(n_at);
                mem.write(list_at, list);
                mem.write(count_at, n);
            }
            1 => {
                let n: u16 = mem.read(n_at);
                let n = i32::from(n.cast_signed());
                if n <= 0 {
                    return;
                }
                let Some(i) = (0..n.cast_unsigned())
                    .find(|&i| mem.read::<u32>(list.wrapping_add(4 * i)) & mask != 0)
                else {
                    return;
                };
                mem.write(start_at, i as u16);
                mem.write(which_at, which);
                let sel: u32 = mem.read(list.wrapping_add(4 * i));
                mem.write(sel_at, sel);
                mem.write(count_at, n as u16);
                mem.write(top_at, 0xffu16);
                mem.write(list_at, list);
            }
            2 => {
                mem.write(which_at, which);
                mem.write(start_at, 0u16);
                let sel: u32 = mem.read(list);
                mem.write(sel_at, sel);
                mem.write(top_at, 0xffu16);
                let n: u16 = mem.read(n_at);
                mem.write(list_at, list);
                mem.write(count_at, n);
            }
            _ => {}
        }
    }

    /// `TOReset2()` (`0x008adee0`): the sequence slots cleared
    /// (`0x008adf20`), `TkMenuInitSeq()`, the three scene data blocks set
    /// (`TkMenuSetSceneDataAdrs(0, 0x0185f610)`, `(1, 0x0185fe10)`, `(2,
    /// 0x01841e10)`), then `TOSeqInitParam()` (its result returned). The
    /// calls go through `call(sym, args)`.
    pub fn reset2<M: Mem>(mem: &mut M, mut call: impl FnMut(&mut M, u32, &[u32]) -> u32) -> u32 {
        call(mem, 0x008a_df20, &[]);
        call(mem, 0x00a7_bb00, &[]);
        for (i, a) in [(0u32, 0x0185_f610u32), (1, 0x0185_fe10), (2, 0x0184_1e10)] {
            let a = mem.rebase(a);
            call(mem, 0x00a7_c7f0, &[i, a]);
        }
        call(mem, 0x008a_e6f0, &[])
    }

    /// `TOStdGetItemName(item)` (`0x008b8db0`): `MsGetRomItem(item, &base)`
    /// (through `call`, with a scratch word `tmp`) gives a record whose
    /// first `u16` is the name's offset from `base`.
    pub fn std_get_item_name<M: Mem>(
        mem: &mut M,
        item: u32,
        tmp: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let rec = call(mem, 0x0079_09f0, &[item, tmp]);
        let off = u32::from(mem.read::<u16>(rec));
        let base: u32 = mem.read(tmp);
        off.wrapping_add(base)
    }

    /// `0x008be020(i, k)`: `BtlEnd_SetFlag(i)`, then the byte `+0` of
    /// entry `k` (14 bytes each) of `0x01869f2a` set to 0xf.
    pub fn btl_end_set_flag_f<M: Mem>(
        mem: &mut M,
        i: u32,
        k: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        call(mem, 0x008b_db60, &[i]);
        let at = mem.rebase(0x0186_9f2a).wrapping_add(k.wrapping_mul(14));
        mem.write(at, 0xfu8);
    }

    /// `TOMenuTermHelp()` (`0x008bf0b0`): with the help state
    /// (`0x0186a230`) at 3 the close sound (`SndSepPlaySimple(0x8000006c)`);
    /// `TkMenuMainSleepMenu(0xb)`; unless `0x0186a22c` is set
    /// `TkMenuMainActiveMenu(1, 0)`; the state 0.
    pub fn menu_term_help<M: Mem>(mem: &mut M, mut call: impl FnMut(&mut M, u32, &[u32]) -> u32) {
        let state = mem.rebase(0x0186_a230);
        if mem.read::<u32>(state) == 3 {
            call(mem, 0x0088_6e80, &[0x8000_006c]);
        }
        call(mem, 0x008a_ad70, &[0xb]);
        if mem.read::<u16>(mem.rebase(0x0186_a22c)) == 0 {
            call(mem, 0x008a_a100, &[1, 0]);
        }
        mem.write(state, 0u32);
    }

    /// `TOMenuTermAbilityMap2()` (`0x008e27e0`): `TkMenuSetScreenX2Flag(0)`,
    /// `SndSepPlaySimple(0x80000001)`, `TkMenuMainSleepMenu(0x13)`,
    /// `TkMenuMainActiveMenu(1, 0)`.
    pub fn menu_term_ability_map2<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        call(mem, 0x008a_b020, &[0]);
        call(mem, 0x0088_6e80, &[0x8000_0001]);
        call(mem, 0x008a_ad70, &[0x13]);
        call(mem, 0x008a_a100, &[1, 0]);
    }

    /// `0x00906e60(i, kind)`: window `i`'s kind (`u16 +4`) set and its
    /// handlers installed (`yiWindowSetHandlers(window)`).
    #[allow(clippy::cast_possible_truncation)] // `mov word`
    pub fn set_window_kind<M: Mem>(
        mem: &mut M,
        i: u32,
        kind: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let slot = crate::window::bridge::WINDOWS
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(4));
        let w: u32 = mem.read(slot);
        mem.write(w.wrapping_add(4), kind as u16);
        let w: u32 = mem.read(slot);
        call(mem, 0x0090_7d80, &[w]);
    }

    /// `0x008b8dd0(id)`: `MsGetSaveWeaponName(id)` (a tail call, through
    /// `call`).
    pub fn std_get_weapon_name<M: Mem>(
        mem: &mut M,
        id: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, 0x007a_bdf0, &[id])
    }

    /// `0x008af6a0(id)`: the flag `0x00c59558` set to 1 when `id` is listed
    /// for the current map in the table at `0x00b5f730` (0x20-byte entries:
    /// map, then ids up to -1; a map of 0 ends it, and the first entry's map
    /// is compiled in as 0xa2), else to 0. Skipped while the flag is
    /// negative, or while the save menu word (`MsGetSaveMenu`) has 0x200 set
    /// and `0x007851b0` returns 1. Returns 0.
    pub fn check_map_message<M: Mem>(
        mem: &mut M,
        id: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let menu = call(mem, 0x0078_5290, &[]);
        if mem.read::<u32>(menu) & 0x200 != 0 && call(mem, 0x0078_51b0, &[]) == 1 {
            return 0;
        }
        let flag = mem.rebase(0x00c5_9558);
        if mem.read::<u32>(flag).cast_signed() < 0 {
            return 0;
        }
        let map = CURRENT_MAP.get(mem);
        let table = mem.rebase(0x00b5_f730);
        let mut k = 0u32;
        let mut entry_map = 0xa2u32;
        loop {
            let entry = table.wrapping_add(k.wrapping_mul(0x20));
            if map == entry_map {
                let mut at = entry.wrapping_add(4);
                loop {
                    let v: u32 = mem.read(at);
                    if v == u32::MAX {
                        break;
                    }
                    if v == id {
                        mem.write(flag, 1u32);
                        return 0;
                    }
                    at = at.wrapping_add(4);
                }
            }
            k = k.wrapping_add(1);
            entry_map = mem.read(table.wrapping_add(k.wrapping_mul(0x20)));
            if entry_map == 0 {
                mem.write(flag, 0u32);
                return 0;
            }
        }
    }

    /// `0x008b5680()`: with the latch `0x00c5a03c` at 1: the counter
    /// `0x018663e0` counted; on map `0x00c3347c` (`g_GraphicMapId` equal)
    /// the second counter `0x018663e4` (unless -1, finished) counted, and at
    /// 0x3a achievement 0x21 unlocked (`achievementUnlockAchievement`,
    /// through `call`) and the counter finished; the latch 0xff.
    pub fn count_map_visits<M: Mem>(mem: &mut M, mut call: impl FnMut(&mut M, u32, &[u32]) -> u32) {
        let latch = mem.rebase(0x00c5_a03c);
        if mem.read::<u8>(latch) != 1 {
            return;
        }
        let map: u32 = mem.read(mem.rebase(0x0133_0844));
        let total = mem.rebase(0x0186_63e0);
        let t: u32 = mem.read(total);
        mem.write(total, t.wrapping_add(1));
        if map == mem.read::<u32>(mem.rebase(0x00c3_347c)) {
            let counter = mem.rebase(0x0186_63e4);
            let mut v: u32 = mem.read(counter);
            if v != u32::MAX {
                v = v.wrapping_add(1);
                mem.write(counter, v);
            }
            if v == 0x3a {
                call(mem, 0x0082_2380, &[0x21]);
                mem.write(counter, u32::MAX);
            }
        }
        mem.write(latch, 0xffu8);
    }

    /// `0x00906b30(n)`: the 26 collected bits of `0x011307a0` (save data):
    /// -1 clears them; 0..=0x19 sets bit `n` and unlocks achievement 1, and
    /// achievement 0x1c once all 26 are set; 0xff sets all and unlocks
    /// 0x1c (`achievementUnlockAchievement`, through `call`).
    pub fn collect_bit<M: Mem>(
        mem: &mut M,
        n: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let bits = mem.rebase(0x0113_07a0);
        if n == u32::MAX {
            mem.write(bits, 0u32);
        } else if n <= 0x19 {
            let v: u32 = mem.read(bits);
            mem.write(bits, v | (1 << n));
            call(mem, 0x0082_2380, &[1]);
            if mem.read::<u32>(bits) == 0x03ff_ffff {
                call(mem, 0x0082_2380, &[0x1c]);
            }
        } else if n == 0xff {
            mem.write(bits, 0x03ff_ffffu32);
            call(mem, 0x0082_2380, &[0x1c]);
        }
    }

    /// `0x008b3610()`: the 0x400 bytes at `0x008b3630()` (through `call`)
    /// cleared.
    pub fn clear_block_159e468<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let at = call(mem, 0x008b_3630, &[]);
        mem.write_bytes(at, &[0u8; 0x400]);
    }

    /// `0x008b7c00(text, kind, out, _)`: `TOGetFontWidth(text, kind, out,
    /// table)` (`width(table)`, passed in) with table 0 for languages 0, 9
    /// and 10 (Japanese, Korean, Chinese) and 4 otherwise (`TOGetFFXLang`,
    /// `lang`, asked up to three times).
    pub fn font_width_for_lang<M: Mem>(
        mem: &mut M,
        mut lang: impl FnMut(&mut M) -> u32,
        width: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let table = if lang(mem) == 0 || lang(mem) == 9 || lang(mem) == 0xa {
            0
        } else {
            4
        };
        width(mem, table)
    }

    /// The calls [`btl_search_rev_near_target`] makes outside this module.
    pub trait NearTarget<M> {
        /// `MsGetChrBtlPos(chr, vec)` into the 16-byte scratch `vec`.
        fn btl_pos(&mut self, mem: &mut M, chr: u32, vec: u32);
        /// `MsGetChrDist(chr)`.
        fn dist(&mut self, mem: &mut M, chr: u32) -> u32;
        /// `_CIsqrt(x)` (`0x009497b8`), the value left in ST0.
        fn sqrt(&mut self, mem: &mut M, x: f64) -> f64;
        /// `_CIacos(x)` (`0x009498da`), the value left in ST0.
        fn acos(&mut self, mem: &mut M, x: f64) -> f64;
    }

    /// `TOBtlSearchRevNearTarget()` (`0x008990f0`): the target to jump to
    /// on the other side, returned (eax) as an index into that side's
    /// target list (the decompilation misses the return). The party's
    /// (`0x0133d180`) and the monsters' (`0x0133d1d4`, only those at
    /// distance 0 when there are any) mean positions give the axis between
    /// the sides (x and z; y is 0). From the cursor character
    /// (`0x0133d1f4`) the other side's nearest target gives the distance
    /// unit (at least 1; the first list entry's last target starts it).
    /// Each target scores `angle from the axis in degrees × distance /
    /// unit`, and the lowest score wins (the first entry's last target
    /// starts it); a degenerate angle counts as `acos(1)`. With an empty
    /// other side the original returns an uninitialised word; this
    /// returns 0. `vec` is a 16-byte scratch for the positions.
    #[allow(
        clippy::too_many_lines,
        clippy::many_single_char_names,
        clippy::cast_possible_truncation,
        clippy::similar_names
    )]
    pub fn btl_search_rev_near_target<M: Mem>(
        mem: &mut M,
        vec: u32,
        calls: &mut impl NearTarget<M>,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, mul, sub};
        let party = mem.rebase(0x0133_d180);
        let monsters = mem.rebase(0x0133_d1d4);
        let (n1_at, n2_at) = (mem.rebase(0x0133_d17c), mem.rebase(0x0133_d1d0));
        let count = |mem: &M, at: u32| i32::from(mem.read::<i16>(at));
        let pos = |mem: &mut M, calls: &mut dyn NearTarget<M>, chr: u32| -> [f64; 3] {
            calls.btl_pos(mem, chr, vec);
            let v: [f32; 3] = mem.read(vec);
            v.map(f64::from)
        };
        let entry = |mem: &M, list: u32, i: i32| -> u32 {
            mem.read(list.wrapping_add(i.cast_unsigned().wrapping_mul(4)))
        };
        // the party's mean position (y kept 0)
        let mut ap = [0f64; 3];
        let mut i = 0;
        while i < count(mem, n1_at) {
            for chr in 0..20u32 {
                if entry(mem, party, i) & (1 << chr) != 0 {
                    let p = pos(mem, calls, chr);
                    ap[0] = dword(add(p[0], ap[0]));
                    ap[1] = 0.0;
                    ap[2] = dword(add(p[2], ap[2]));
                }
            }
            i += 1;
        }
        let n = f64::from(count(mem, n1_at));
        ap = ap.map(|v| dword(div(v, n)));
        // the monsters at distance 0, if any, else all of them
        let mut near = 0i32;
        let mut i = 0;
        while i < count(mem, n2_at) {
            for chr in 20..28u32 {
                if entry(mem, monsters, i) & (1 << chr) != 0 && calls.dist(mem, chr) == 0 {
                    near += 1;
                }
            }
            i += 1;
        }
        let mut am = [0f64; 3];
        let mut i = 0;
        while i < count(mem, n2_at) {
            for chr in 20..28u32 {
                if entry(mem, monsters, i) & (1 << chr) != 0
                    && (near <= 0 || calls.dist(mem, chr) == 0)
                {
                    let p = pos(mem, calls, chr);
                    am[0] = dword(add(am[0], p[0]));
                    am[1] = 0.0;
                    am[2] = dword(add(am[2], p[2]));
                }
            }
            i += 1;
        }
        let used = if near == 0 { count(mem, n2_at) } else { near };
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword
        let m = f64::from(used as f32);
        am = am.map(|v| dword(div(v, m)));
        let cursor = u32::from(mem.read::<u16>(mem.rebase(0x0133_d1f4)));
        let p = pos(mem, calls, cursor);
        // the other side, its axis and its list (copied, as the original)
        let (list, n_at, lo, hi, axis) = if cursor < 0x14 {
            let d = [sub(am[0], ap[0]), sub(am[1], ap[1]), sub(am[2], ap[2])].map(dword);
            (monsters, n2_at, 20u32, 28u32, d)
        } else {
            let d = [sub(ap[0], am[0]), sub(ap[1], am[1]), sub(ap[2], am[2])].map(dword);
            (party, n1_at, 0, 20, d)
        };
        let copy: Vec<u32> = (0..(hi - lo).max(8))
            .map(|k| entry(mem, list, k.cast_signed()))
            .collect();
        let n = count(mem, n_at);
        let rel = |mem: &mut M, calls: &mut dyn NearTarget<M>, chr: u32| -> [f64; 3] {
            let q = pos(mem, calls, chr);
            [sub(q[0], p[0]), sub(q[1], p[1]), sub(q[2], p[2])].map(dword)
        };
        let len = |calls: &mut dyn NearTarget<M>, mem: &mut M, q: [f64; 3]| -> f64 {
            let s = add(add(mul(q[1], q[1]), mul(q[0], q[0])), mul(q[2], q[2]));
            dword(calls.sqrt(mem, s))
        };
        // the nearest target of the other side: the unit
        let mut unit = f64::NAN; // uninitialised in the original until a target is seen
        let mut i = 0;
        while i < count(mem, n_at) {
            for chr in lo..hi {
                if entry(mem, list, i) & (1 << chr) != 0 {
                    let q = rel(mem, calls, chr);
                    let r = len(calls, mem, q);
                    if i == 0 || unit > r {
                        unit = r;
                    }
                }
            }
            i += 1;
        }
        if 1.0 > unit {
            unit = 1.0;
        }
        if n <= 0 {
            return 0;
        }
        let mut best_score = 0f64;
        let mut best = 1u32 << lo;
        for j in 0..n {
            for chr in lo..hi {
                if copy[j.cast_unsigned() as usize] & (1 << chr) == 0 {
                    continue;
                }
                let q = rel(mem, calls, chr);
                let dot = dword(add(
                    add(mul(q[0], axis[0]), mul(q[1], axis[1])),
                    mul(q[2], axis[2]),
                ));
                let dl = len(calls, mem, axis);
                let ql = len(calls, mem, q);
                let cos = if dl == 0.0 || ql == 0.0 || dot == 0.0 {
                    1.0
                } else {
                    dword(div(dot, mul(dl, ql)))
                };
                let deg = dword(div(
                    mul(calls.acos(mem, cos), 180.0),
                    3.141_592_741_012_573_2,
                ));
                let ratio = dword(div(ql, unit));
                let score = dword(mul(deg, ratio));
                if j == 0 || best_score > score {
                    best_score = score;
                    best = j.cast_unsigned();
                }
            }
        }
        best
    }

    /// `TOInitSceneMessageWindow()` (`0x008adbf0`): for each of the 8
    /// message-window objects (re-read before every store): u16 `+0` and
    /// `+8` of its `+0x10` record, u16 `+0` and `+0x2c` of its `+0x14`
    /// record and bytes `+0`, `+1` of its `+0x18` record cleared, in the
    /// original's order.
    pub fn init_scene_message_window(mem: &mut impl Mem) {
        let slots = MESSAGE_SLOTS.ptr(mem).addr;
        for i in 0..8u32 {
            let slot = slots.wrapping_add(4 * i);
            for (rec, off, wide) in [
                (0x10u32, 0u32, true),
                (0x14, 0, true),
                (0x18, 0, false),
                (0x10, 8, true),
                (0x14, 0x2c, true),
                (0x18, 1, false),
            ] {
                let obj: u32 = mem.read(slot);
                let r: u32 = mem.read(obj.wrapping_add(rec));
                if wide {
                    mem.write(r.wrapping_add(off), 0u16);
                } else {
                    mem.write(r.wrapping_add(off), 0u8);
                }
            }
        }
    }

    /// `TOSetExeMessageArea(bank)` (`0x008b86e0`): the message windows'
    /// eight pointers (`0x018676f0`) to the first (`bank` 0, `0x018663f0`)
    /// or second (`0x01866d70`) bank of 0x130-byte windows, their eight
    /// line records (`0x01868a90`; 0x138 bytes from `0x01867710` or
    /// `0x018680d0`), and the 256 macro-command pointers of [`MSG_VARS`]
    /// (`0x01869ab0`) to consecutive 8-byte records from `0x01868ab0` or
    /// `0x018692b0`.
    pub fn set_exe_message_area(mem: &mut impl Mem, bank: u32) {
        let (win, line, var) = if bank == 0 {
            (0x0186_63f0, 0x0186_7710, 0x0186_8ab0)
        } else {
            (0x0186_6d70, 0x0186_80d0, 0x0186_92b0)
        };
        let (win, line, var) = (mem.rebase(win), mem.rebase(line), mem.rebase(var));
        let (wins, lines, vars) = (
            mem.rebase(0x0186_76f0),
            mem.rebase(0x0186_8a90),
            mem.rebase(0x0186_9ab0),
        );
        for k in 0..8u32 {
            mem.write(wins.wrapping_add(4 * k), win.wrapping_add(0x130 * k));
            mem.write(lines.wrapping_add(4 * k), line.wrapping_add(0x138 * k));
        }
        for k in 0..256u32 {
            mem.write(vars.wrapping_add(4 * k), var.wrapping_add(8 * k));
        }
    }

    /// `TOInitMessageCallBack(msg)` (`0x008df6d0`): message window `msg`'s
    /// handlers from its kind's row (`+8`, 0x24-byte rows at `0x00c5b1b0`):
    /// words 0..5 into `+0x100..=+0x110`, word 5 into `+0x12c` (in message
    /// mode 1 `0x008dfea0` instead), words 6 and 7 into `+0xf4`, `+0xf8`.
    pub fn init_message_call_back(mem: &mut impl Mem, msg: u32) {
        let table = mem.rebase(0x00c5_b1b0);
        for (k, off) in [
            (0u32, 0x100u32),
            (1, 0x104),
            (2, 0x108),
            (3, 0x10c),
            (4, 0x110),
        ] {
            let kind: u32 = mem.read(msg.wrapping_add(8));
            let v: u32 = mem.read(
                table
                    .wrapping_add(kind.wrapping_mul(0x24))
                    .wrapping_add(4 * k),
            );
            mem.write(msg.wrapping_add(off), v);
        }
        let v = if crate::menu::bridge::message_window_mode(mem) == 1 {
            mem.rebase(0x008d_fea0)
        } else {
            let kind: u32 = mem.read(msg.wrapping_add(8));
            mem.read(
                table
                    .wrapping_add(kind.wrapping_mul(0x24))
                    .wrapping_add(0x14),
            )
        };
        mem.write(msg.wrapping_add(0x12c), v);
        let kind: u32 = mem.read(msg.wrapping_add(8));
        let row = table.wrapping_add(kind.wrapping_mul(0x24));
        let a: u32 = mem.read(row.wrapping_add(0x18));
        mem.write(msg.wrapping_add(0xf4), a);
        let b: u32 = mem.read(row.wrapping_add(0x1c));
        mem.write(msg.wrapping_add(0xf8), b);
    }

    /// `TOMenuMakeSaveFileName()` (`0x008b3a40`): the save folder name in
    /// `0x023cc0a0`: the PS2 product code of the language (`TOGetFFXLang`,
    /// passed in: 0 `BISLPS-25088`, 1 `BASLUS-20312`, 2 `BESCES-50491`, 3
    /// `BESCES-50494`, 4 `BESCES-50492`, 5 `BESCES-50493`, 9
    /// `BKSLPM-67513`, 16 `BISLPS-25088`, 17 `BISLPM-65115`, 18
    /// `BESCES-50490`; any other leaves the buffer as it is) with
    /// `FF0906%02d` appended (the slot number is formatted later); the
    /// word `0x01866240` cleared.
    pub fn menu_make_save_file_name(mem: &mut impl Mem, lang: u32) {
        let code = match lang {
            0 | 16 => Some(0x00c5_b360),
            1 => Some(0x00c5_b370),
            2 => Some(0x00c5_b3c0),
            3 => Some(0x00c5_b3a0),
            4 => Some(0x00c5_b390),
            5 => Some(0x00c5_b3b0),
            9 => Some(0x00c5_b3e0),
            17 => Some(0x00c5_b3d0),
            18 => Some(0x00c5_b380),
            _ => None,
        };
        let out = mem.rebase(0x023c_c0a0);
        if let Some(src) = code {
            let src = mem.rebase(src);
            let mut k = 0u32;
            loop {
                let c: u8 = mem.read(src.wrapping_add(k));
                mem.write(out.wrapping_add(k), c);
                k = k.wrapping_add(1);
                if c == 0 {
                    break;
                }
            }
        }
        let tail = mem.rebase(0x00c5_b3f0);
        let len = c_strlen(mem, tail);
        let end = out.wrapping_add(c_strlen(mem, out));
        for k in 0..=len {
            let c: u8 = mem.read(tail.wrapping_add(k));
            mem.write(end.wrapping_add(k), c);
        }
        let w = mem.rebase(0x0186_6240);
        mem.write(w, 0u32);
    }

    /// `TOFuncMacroCmdValue(cmd)` (`0x008b6210`): the macro command's value
    /// (`+4`) as decimal (`sprintf("%d")`) in `0x01869ee4`; with the
    /// command's type `+1` at 2 the digit-group mark 0x46 goes before each
    /// group of three from the right (the count includes a minus sign).
    /// Returns the buffer.
    pub fn func_macro_cmd_value(mem: &mut impl Mem, cmd: u32) -> u32 {
        let v: i32 = mem.read(cmd.wrapping_add(4));
        let text = v.to_string().into_bytes();
        let grouped = mem.read::<u8>(cmd.wrapping_add(1)) == 2;
        let out = mem.rebase(0x0186_9ee4);
        let mut at = out;
        let mut left = text.len();
        for (i, &c) in text.iter().enumerate() {
            if left.is_multiple_of(3) && i > 0 && grouped {
                mem.write(at, 0x46u8);
                at = at.wrapping_add(1);
            }
            mem.write(at, c);
            at = at.wrapping_add(1);
            left -= 1;
        }
        mem.write(at, 0u8);
        out
    }

    /// The input calls of [`get_pad_status`].
    pub trait PadInput<M> {
        /// `inputGetCurTime()`, the value left in ST0.
        fn cur_time(&mut self, mem: &mut M) -> f64;
        /// `TkGetControlerHist(a, b, c)`.
        fn hist(&mut self, mem: &mut M, a: u32, b: u32, c: u32) -> u32;
        /// `TkGetControlerAnalogHist(a, b, axis, d)`.
        fn analog(&mut self, mem: &mut M, a: u32, b: u32, axis: u32, d: u32) -> u32;
    }

    /// `TOGetPadStatus()` (`0x008b7cd0`): the TO library's pad words.
    /// `0x025d09d2` held, `+2` triggered, `+4` repeated buttons; the same
    /// at `0x025d09e2` with the d-pad bits made from the left stick
    /// (`0x025d09de` x, `0x025d09df` y, below 0x18 or above 0xe8). All are
    /// cleared, and refilled only while `0x025d09c1` or `g_TOPadGetCtrl`
    /// (`0x025d09c0`) is set: the d-pad's own set if any d-pad button is
    /// held, else the stick's. Each button but 0x800 and 0x100 repeats: a
    /// fresh press (count `0x025d09c2 + k` 0) repeats at once; while held
    /// (not freshly triggered) the count rises and the held time
    /// (`0x025d0a00 + 4k`) adds the time since the last frame
    /// (`0x025d0a40 + 4k`, from `inputGetCurTime`); past 0.2333 s it
    /// repeats and restarts at 0.1333 s with count 4; a released button
    /// resets both. The chosen set is copied to the other; the control
    /// flag is cleared.
    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)] // `mov word`
    pub fn get_pad_status<M: Mem>(mem: &mut M, calls: &mut impl PadInput<M>) {
        use ffx_bridge::x87::{add, sub};
        let base = mem.rebase(0x025d_09c0);
        let w = |off: u32| base.wrapping_add(off);
        let prev_alt: u16 = mem.read(w(0x22));
        let prev: u16 = mem.read(w(0x12));
        let t = dword(calls.cur_time(mem));
        mem.write(w(0x12), 0u32);
        mem.write(w(0x16), 0u16);
        mem.write(w(0x22), 0u32);
        mem.write(w(0x26), 0u16);
        if mem.read::<u8>(w(1)) == 0 && mem.read::<u8>(w(0)) == 0 {
            return;
        }
        let h = calls.hist(mem, 0, 0, 0) as u16;
        mem.write(w(0x12), h);
        mem.write(w(0x14), (h ^ prev) & h);
        let sx = calls.analog(mem, 0, 0, 2, 0) as u8;
        mem.write(w(0x1e), sx);
        let sy = calls.analog(mem, 0, 0, 3, 0) as u8;
        let held: u16 = mem.read(w(0x12));
        mem.write(w(0x1f), sy);
        let mut alt = held & 0xfff;
        mem.write(w(0x22), alt);
        for (on, bit) in [(sy < 0x18, 0x1000u16), (sy > 0xe8, 0x4000)] {
            if on {
                alt |= bit;
                mem.write(w(0x22), alt);
            }
        }
        let sx: u8 = mem.read(w(0x1e));
        for (on, bit) in [(sx < 0x18, 0x8000u16), (sx > 0xe8, 0x2000)] {
            if on {
                alt |= bit;
                mem.write(w(0x22), alt);
            }
        }
        mem.write(w(0x24), (alt ^ prev_alt) & alt);
        let (held_at, trg_at, rep_at) = if held & 0xf000 == 0 {
            (w(0x22), w(0x24), w(0x26))
        } else {
            (w(0x12), w(0x14), w(0x16))
        };
        let limit = 0.233_333_319_425_582_89;
        let restart = f64::from(f32::from_bits(0x3e08_8888));
        for k in 0..16u32 {
            let bit = 1u16 << k;
            if bit == 0x800 || bit == 0x100 {
                continue;
            }
            let cnt_at = w(2 + k);
            let acc_at = mem.rebase(0x025d_0a00).wrapping_add(4 * k);
            let last_at = mem.rebase(0x025d_0a40).wrapping_add(4 * k);
            let or_rep = |mem: &mut M| {
                let r: u16 = mem.read(rep_at);
                mem.write(rep_at, r | bit);
            };
            let mut skip = false;
            if mem.read::<u16>(trg_at) & bit != 0 {
                if mem.read::<u8>(cnt_at) == 0 {
                    or_rep(mem);
                }
                skip = mem.read::<u16>(trg_at) & bit != 0;
            }
            if !skip {
                let c: u8 = mem.read(cnt_at);
                mem.write(cnt_at, c.wrapping_add(1));
                let last = f64::from(mem.read::<f32>(last_at));
                let acc = f64::from(mem.read::<f32>(acc_at));
                mem.write(acc_at, dword(add(sub(t, last), acc)) as f32);
            }
            if mem.read::<u16>(held_at) & bit == 0 {
                mem.write(cnt_at, 0u8);
                mem.write(acc_at, 0f32);
            }
            if f64::from(mem.read::<f32>(acc_at)) > limit {
                or_rep(mem);
                mem.write(cnt_at, 4u8);
                mem.write(acc_at, restart as f32);
            }
            mem.write(last_at, t as f32);
        }
        let held: u16 = mem.read(w(0x12));
        if held & 0xf000 == 0 {
            for off in [0x12u32, 0x14, 0x16] {
                let v: u16 = mem.read(w(off + 0x10));
                mem.write(w(off), v);
            }
        } else {
            mem.write(w(0x22), held);
            for off in [0x14u32, 0x16] {
                let v: u16 = mem.read(w(off));
                mem.write(w(off + 0x10), v);
            }
        }
        mem.write(w(0), 0u8);
    }

    /// `TOMenuGetPadStatus()` (`0x008be500`): the menu's pad words (as
    /// [`get_pad_status`]'s: `0x025d09d2` held, triggered, repeated, then
    /// `+6..+0xa` the same of both pads, `TkGetControler*Hist(0, 0, 0)`
    /// with `(0, 0, -1)` OR-ed in). With the dummy controller on
    /// (`TkMenuGetDummyControlerFlag`, flushed first) they come from the
    /// dummy (`TkMenuGetDummyControler`/`T`/`R`, sticks
    /// `TOMenuGetDummyAnalogControler`) and nothing more happens. Else the
    /// stick-made d-pad set (`0x025d09e2`) is built and repeats: a fresh
    /// press at once, then after 0.4667 s, again every 0.1667 s (restart
    /// 0.3, count 9). Calls go through `call(sym, args)`; the time is
    /// `inputGetCurTime` (`time`).
    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)] // `mov word`
    pub fn menu_get_pad_status<M: Mem>(
        mem: &mut M,
        time: impl FnOnce(&mut M) -> f64,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, sub};
        let base = mem.rebase(0x025d_09c0);
        let w = |off: u32| base.wrapping_add(off);
        let prev_alt: u16 = mem.read(w(0x22));
        let t = dword(time(mem));
        mem.write(w(0x12), 0u32);
        mem.write(w(0x16), 0u16);
        mem.write(w(0x22), 0u32);
        mem.write(w(0x26), 0u16);
        if call(mem, 0x008c_a510, &[]) == 1 {
            call(mem, 0x008c_a490, &[]);
        }
        if call(mem, 0x008c_a510, &[]) == 1 {
            for (sym, a, b) in [
                (0x008c_a500u32, 0x12u32, 0x18u32),
                (0x008c_a530, 0x14, 0x1a),
                (0x008c_a520, 0x16, 0x1c),
            ] {
                let v = call(mem, sym, &[]) as u16;
                mem.write(w(a), v);
                let v = call(mem, sym, &[]) as u16;
                mem.write(w(b), v);
            }
            let v = call(mem, 0x008c_a320, &[2]) as u8;
            mem.write(w(0x1e), v);
            let v = call(mem, 0x008c_a320, &[3]) as u8;
            mem.write(w(0x1f), v);
            return;
        }
        let (hist, trig, rept) = (0x0088_8d70u32, 0x0088_8e80u32, 0x0088_8e50u32);
        for (sym, off) in [(hist, 0x12u32), (trig, 0x14), (rept, 0x16)] {
            let v = call(mem, sym, &[0, 0, 0]) as u16;
            mem.write(w(off), v);
        }
        for (sym, off) in [(hist, 0x18u32), (trig, 0x1a), (rept, 0x1c)] {
            let v = call(mem, sym, &[0, 0, 0]) as u16;
            mem.write(w(off), v);
            let v2 = call(mem, sym, &[0, 0, u32::MAX]) as u16;
            let cur: u16 = mem.read(w(off));
            mem.write(w(off), cur | v2);
        }
        let sx = call(mem, 0x0088_8c20, &[0, 0, 2, 0]) as u8;
        mem.write(w(0x1e), sx);
        let sy = call(mem, 0x0088_8c20, &[0, 0, 3, 0]) as u8;
        let held: u16 = mem.read(w(0x12));
        mem.write(w(0x1f), sy);
        let mut alt = held & 0xfff;
        mem.write(w(0x22), alt);
        for (on, bit) in [(sy < 0x18, 0x1000u16), (sy > 0xe8, 0x4000)] {
            if on {
                alt |= bit;
                mem.write(w(0x22), alt);
            }
        }
        let sx: u8 = mem.read(w(0x1e));
        for (on, bit) in [(sx < 0x18, 0x8000u16), (sx > 0xe8, 0x2000)] {
            if on {
                alt |= bit;
                mem.write(w(0x22), alt);
            }
        }
        mem.write(w(0x24), (alt ^ prev_alt) & alt);
        let limit = 0.466_666_638_851_165_77;
        let restart = f64::from(f32::from_bits(0x3e99_9999));
        for k in 0..16u32 {
            let bit = 1u16 << k;
            if bit == 0x800 || bit == 0x100 {
                continue;
            }
            let cnt_at = w(2 + k);
            let acc_at = mem.rebase(0x025d_0a00).wrapping_add(4 * k);
            let last_at = mem.rebase(0x025d_0a40).wrapping_add(4 * k);
            let or_rep = |mem: &mut M| {
                let r: u16 = mem.read(w(0x26));
                mem.write(w(0x26), r | bit);
            };
            let pressed = mem.read::<u16>(w(0x24)) & bit;
            if pressed != 0 && mem.read::<u8>(cnt_at) == 0 {
                or_rep(mem);
            }
            if pressed == 0 {
                let c: u8 = mem.read(cnt_at);
                mem.write(cnt_at, c.wrapping_add(1));
                let last = f64::from(mem.read::<f32>(last_at));
                let acc = f64::from(mem.read::<f32>(acc_at));
                mem.write(acc_at, dword(add(sub(t, last), acc)) as f32);
            }
            if mem.read::<u16>(w(0x22)) & bit == 0 {
                mem.write(cnt_at, 0u8);
                mem.write(acc_at, 0f32);
            }
            if f64::from(mem.read::<f32>(acc_at)) > limit {
                or_rep(mem);
                mem.write(cnt_at, 9u8);
                mem.write(acc_at, restart as f32);
            }
            mem.write(last_at, t as f32);
        }
    }

    /// The kernel accessors at `0x00771c30..=0x007724f0` (one per menu
    /// field, reached through a table): `MsGetSaveCommand(chr, cmd)` for a
    /// fixed command (through `call`).
    pub fn kernel_save_command<M: Mem>(
        mem: &mut M,
        chr: u32,
        cmd: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, 0x0078_5020, &[chr, cmd])
    }

    /// A kernel accessor: the word at `off` of `MsGetSavePlayerPtr(chr)`.
    pub fn kernel_player_word<M: Mem>(
        mem: &mut M,
        chr: u32,
        off: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let p = call(mem, 0x0077_6f50, &[chr]);
        mem.read(p.wrapping_add(off))
    }

    /// A kernel accessor: `Kernel_GetAbilityName` of the `u16` at `off` of
    /// record `i` (0x1d8 bytes each) of the table `*0x0112a034`.
    pub fn kernel_slot_ability_name<M: Mem>(
        mem: &mut M,
        i: u32,
        off: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table: u32 = mem.read(mem.rebase(0x0112_a034));
        let id =
            u32::from(mem.read::<u16>(table.wrapping_add(i.wrapping_mul(0x1d8)).wrapping_add(off)));
        call(mem, 0x0077_4720, &[id])
    }

    /// A kernel accessor that only forwards to `f` (a tail call).
    pub fn kernel_forward<M: Mem>(
        mem: &mut M,
        f: u32,
        arg: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, f, &[arg])
    }

    /// `TOSetExeMessageWindowArea(bank)` (`0x008af5c0`): the eight message
    /// window objects ([`MESSAGE_SLOTS`]) to the first (`bank` 0,
    /// `0x01865ce0`) or second (`0x01865dc0`) bank of 0x1c-byte objects,
    /// then `0x01841e0c` = `bank` (`0x008af690`), [`set_exe_window_area`]
    /// and [`set_exe_message_area`].
    pub fn set_exe_message_window_area(mem: &mut impl Mem, bank: u32) {
        let first = mem.rebase(if bank == 0 { 0x0186_5ce0 } else { 0x0186_5dc0 });
        let at = MESSAGE_SLOTS.ptr(mem).addr;
        for k in 0..8u32 {
            mem.write(at.wrapping_add(4 * k), first.wrapping_add(0x1c * k));
        }
        crate::accessor::bridge::set_word_global(mem, 0x008a_f690, bank);
        set_exe_window_area(mem, bank);
        set_exe_message_area(mem, bank);
    }

    /// `TOCheckBtlCommandTarget(chr, cmd)` (`0x0089abd0`): the target block
    /// `0x0133d0d0` (0x140 bytes) cleared with the cursor character and
    /// command set, the command window's start and mask cleared, then the
    /// list, group, status, command and target masks rebuilt
    /// (`TOBtlMakeTarget*Bit`, through `call`). With `0x0133c94c` at 0x100
    /// the target is instead the cursor mask (`0x0133d15c & 0x0133d154 &
    /// 0x0133d0dc`) as a single party entry in mode 2. Returns 1.
    #[allow(clippy::cast_possible_truncation)] // `mov word`
    pub fn btl_check_command_target<M: Mem>(
        mem: &mut M,
        chr: u32,
        cmd: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let block = mem.rebase(0x0133_d0d0);
        mem.write_bytes(block, &[0u8; 0x140]);
        mem.write(block, chr as u16);
        mem.write(block.wrapping_add(2), cmd as u16);
        let depth = i32::from(mem.read::<i8>(mem.rebase(0x023c_c092)));
        let w = mem
            .rebase(0x0133_c992)
            .wrapping_add(depth.wrapping_mul(0xf0).cast_unsigned());
        mem.write(w, 0u16);
        mem.write(w.wrapping_add(0x96), 0u32);
        for f in [
            0x0089_85b0u32,
            0x0089_8490,
            0x0089_8330,
            0x0089_83f0,
            0x0089_8640,
        ] {
            call(mem, f, &[]);
        }
        if mem.read::<u32>(mem.rebase(0x0133_c94c)) == 0x100 {
            let word = |mem: &M, a: u32| -> u32 { mem.read(mem.rebase(a)) };
            let mask = word(mem, 0x0133_d15c) & word(mem, 0x0133_d154) & word(mem, 0x0133_d0dc);
            let at = |mem: &M, a: u32| mem.rebase(a);
            let party = at(mem, 0x0133_d180);
            mem.write(at(mem, 0x0133_d1fe), 0u32);
            mem.write(at(mem, 0x0133_d1fc), 0u16);
            mem.write(at(mem, 0x0133_d0e0), 2u32);
            mem.write(at(mem, 0x0133_d1f8), party);
            mem.write(at(mem, 0x0133_d1f4), 0x0001_00ffu32);
            mem.write(at(mem, 0x0133_d17c), 1u16);
            mem.write(party, mask);
            mem.write(at(mem, 0x0133_d178), mask);
        }
        1
    }

    /// `TOControlMessageWindow()` (`0x008ab910`): the pad read
    /// (`TOGetPadStatus`), then for each active message window (`u16 +0`
    /// of its object) `TOControlWindow(i)` and `TOControlMessage(i)`; when
    /// the window reported a change its message function is set
    /// (`TOSetMessageFunc(i, +8 of the object's +0x10 record)`), and a
    /// window waiting for the pad (`+6` 1) reads it
    /// (`TOGetControlPadStatus(i)`). Both pad words at 0x100 (held and
    /// triggered) step the debug pad-print mode (`TODebugAddPadPrintFlag`).
    /// Calls go through `call`.
    pub fn control_message_window<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        call(mem, 0x008b_7cd0, &[]);
        let slots = MESSAGE_SLOTS.ptr(mem).addr;
        for i in 0..8u32 {
            let obj = |mem: &M| -> u32 { mem.read(slots.wrapping_add(4 * i)) };
            if mem.read::<u16>(obj(mem)) == 0 {
                continue;
            }
            let changed = call(mem, 0x0090_6bd0, &[i]);
            call(mem, 0x008b_5850, &[i]);
            if changed != 0 {
                let rec: u32 = mem.read(obj(mem).wrapping_add(0x10));
                let f = i32::from(mem.read::<i16>(rec.wrapping_add(8))).cast_unsigned();
                call(mem, 0x008b_8af0, &[i, f]);
            }
            if mem.read::<u16>(obj(mem).wrapping_add(6)) == 1 {
                call(mem, 0x008b_6b70, &[i]);
            }
        }
        let pad = mem.rebase(0x025d_09d2);
        if mem.read::<u16>(pad) == 0x100 && mem.read::<u16>(pad.wrapping_add(2)) == 0x100 {
            call(mem, 0x008a_bd00, &[]);
        }
    }

    /// `TOSetMessageType(i, type)` (`0x008b8bb0`): message window `i`'s
    /// kind (`+8`, the low nibble) and flags from `type`: `+0x10` 0x4000,
    /// `+0x1a` 1 with 0x2000 (3 with 0x8000 too), `+0xc` 1 with 0x1000 (or
    /// when the save menu word lacks 0x100), `+0x12` 1 with 0x400; kind 3
    /// sets `+0x1a` to 3 with 0x8000, else 1. Then
    /// [`init_message_call_back`]; returns 0. `MsGetSaveMenu` goes through
    /// `call`.
    #[allow(clippy::cast_possible_truncation)]
    pub fn set_message_type<M: Mem>(
        mem: &mut M,
        i: u32,
        kind: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let slot = crate::window::bridge::MSG_WINDOWS
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(4));
        let m = |mem: &M| -> u32 { mem.read(slot) };
        mem.write(m(mem).wrapping_add(8), kind & 0xf);
        mem.write(m(mem).wrapping_add(0x10), (kind & 0x4000) as u16);
        let scroll = if kind & 0x2000 == 0 {
            0u16
        } else if kind & 0x8000 != 0 {
            3
        } else {
            1
        };
        mem.write(m(mem).wrapping_add(0x1a), scroll);
        mem.write(m(mem).wrapping_add(0xc), u16::from(kind & 0x1000 != 0));
        mem.write(m(mem).wrapping_add(0x12), u16::from(kind & 0x400 != 0));
        let menu = call(mem, 0x0078_5290, &[]);
        if mem.read::<u32>(menu) & 0x100 == 0 {
            mem.write(m(mem).wrapping_add(0xc), 1u16);
        }
        let w = m(mem);
        if mem.read::<u32>(w.wrapping_add(8)) == 3 {
            mem.write(
                w.wrapping_add(0x1a),
                if kind & 0x8000 != 0 { 3u16 } else { 1 },
            );
        }
        init_message_call_back(mem, m(mem));
        0
    }

    /// `0x008b70c0(text, arg)`: the width of the whole of `text`, a glyph
    /// at a time through `TOGetFontWidthForLang(text, 0, &w, arg)` (passed
    /// in as `width`, with the scratch float `out`), summed as
    /// `ftol(total + w)`; the glyph count in `0x018663e8`. Returns the total
    /// as a float.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)] // fild, fstp dword
    pub fn text_width<M: Mem>(
        mem: &mut M,
        text: u32,
        arg: u32,
        out: u32,
        mut width: impl FnMut(&mut M, u32, u32, u32) -> u32,
    ) -> f32 {
        let count = mem.rebase(0x0186_63e8);
        let mut total = 0i32;
        let mut p = text;
        mem.write(count, 0u32);
        while mem.read::<u8>(p) != 0 {
            p = width(mem, p, out, arg);
            let w = f64::from(mem.read::<f32>(out));
            total =
                ffx_bridge::x87::ftol(mem, ffx_bridge::x87::add(f32_of(total), w)).cast_signed();
            let c: u32 = mem.read(count);
            mem.write(count, c.wrapping_add(1));
        }
        total as f32
    }

    /// A kernel accessor: the name function `f` (`Kernel_GetAbilityName`
    /// or `Kernel_GetItemName`) of the `u16` at `off` of record `i` (0x1d8
    /// bytes each) of the table `*0x0112a034`.
    pub fn kernel_slot_name<M: Mem>(
        mem: &mut M,
        i: u32,
        off: u32,
        f: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table: u32 = mem.read(mem.rebase(0x0112_a034));
        let id =
            u32::from(mem.read::<u16>(table.wrapping_add(i.wrapping_mul(0x1d8)).wrapping_add(off)));
        call(mem, f, &[id])
    }

    /// Kernel accessors `0x007739c0`, `0x00773a30`, `0x00773aa0`: unless a
    /// request is pending (byte `0x0112a038`), post one: kind `kind`
    /// (`0x0112a058`), value `v` (`0x0112a054`), pending set. Returns `v`
    /// (eax; with a request pending the original leaves eax as it found it,
    /// and this returns 0).
    pub fn kernel_post_request(mem: &mut impl Mem, v: u32, kind: u32) -> u32 {
        let pending = mem.rebase(0x0112_a038);
        if mem.read::<u8>(pending) != 0 {
            return 0;
        }
        mem.write(mem.rebase(0x0112_a058), kind);
        mem.write(mem.rebase(0x0112_a054), v);
        mem.write(pending, 1u8);
        v
    }

    /// Kernel accessor `0x00772dc0(i)`: 0x1f40 for 0xf9, else the word at
    /// `+0x194` of record `i` of `*0x0112a034`.
    pub fn kernel_slot_word_194(mem: &impl Mem, i: u32) -> u32 {
        if i == 0xf9 {
            return 0x1f40;
        }
        let table: u32 = mem.read(mem.rebase(0x0112_a034));
        mem.read(
            table
                .wrapping_add(i.wrapping_mul(0x1d8))
                .wrapping_add(0x194),
        )
    }

    /// Kernel accessor `0x00772e40(i)`: the gil (`MsGetGIL`) left after
    /// the price at `+0x194` of record `i`, or 0 when it is short.
    pub fn kernel_gil_after<M: Mem>(
        mem: &mut M,
        i: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let gil = call(mem, 0x0078_4e80, &[]);
        let table: u32 = mem.read(mem.rebase(0x0112_a034));
        let price: u32 = mem.read(
            table
                .wrapping_add(i.wrapping_mul(0x1d8))
                .wrapping_add(0x194),
        );
        let v = gil.wrapping_sub(price);
        if v.cast_signed() < 0 { 0 } else { v }
    }

    /// Kernel accessor `0x00773b10(sfx)`: `SndSepPlaySimple(sfx |
    /// 0x80000000)`.
    pub fn kernel_play_sfx<M: Mem>(
        mem: &mut M,
        sfx: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, 0x0088_6e80, &[sfx | 0x8000_0000])
    }

    /// Kernel accessor `0x007725b0(i)`: event flag byte `i` of
    /// `MsGetSaveEventFlag()`, sign-extended.
    pub fn kernel_event_byte<M: Mem>(
        mem: &mut M,
        i: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let base = call(mem, 0x0078_5250, &[]);
        i32::from(mem.read::<i8>(base.wrapping_add(i))).cast_unsigned()
    }

    /// Kernel accessor `0x00773800(i, v)`: event flag byte `i` set to `v`;
    /// returns the flags' address (eax).
    #[allow(clippy::cast_possible_truncation)] // `mov byte`
    pub fn kernel_set_event_byte<M: Mem>(
        mem: &mut M,
        i: u32,
        v: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let base = call(mem, 0x0078_5250, &[]);
        mem.write(base.wrapping_add(i), v as u8);
        base
    }

    /// Kernel accessor `0x00772900(chr)`: 1 when `MsGetSavePlyJoin(chr)` is
    /// 1, else 0.
    pub fn kernel_joined<M: Mem>(
        mem: &mut M,
        chr: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        u32::from(call(mem, 0x0078_5380, &[chr]) == 1)
    }

    /// Kernel accessor `0x00772620(n)`: event flag bit `n` (byte `n / 8`
    /// of `MsGetSaveEventFlag()`, truncating, bit `n & 7`): 1 or 0.
    pub fn kernel_event_bit<M: Mem>(
        mem: &mut M,
        n: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let base = call(mem, 0x0078_5250, &[]);
        let byte = (n.cast_signed() / 8).cast_unsigned();
        let v = i32::from(mem.read::<i8>(base.wrapping_add(byte)));
        u32::from(v & (1 << (n & 7)) != 0)
    }

    /// Kernel accessor `0x00773880(n, on)`: event flag bit `n` cleared,
    /// then set when `on`. Returns eax as left: `n / 8` with the byte's
    /// final value in its low byte.
    pub fn kernel_set_event_bit<M: Mem>(
        mem: &mut M,
        n: u32,
        on: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let base = call(mem, 0x0078_5250, &[]);
        let idx = (n.cast_signed() / 8).cast_unsigned();
        let at = base.wrapping_add(idx);
        let mask = 1u8 << (n & 7);
        let v: u8 = mem.read(at);
        let mut v = v & !mask;
        mem.write(at, v);
        if on != 0 {
            v |= mask;
            mem.write(at, v);
        }
        (idx & !0xff) | u32::from(v)
    }

    /// `0x00872a00()`: 1 when the previous level's name hash
    /// (`graphicPreviousLevelGetNameHash`, through `call`) is 0x1d58, or
    /// when bits 0-1 of byte `+3` of `*0x01326b28` are 1; else 0.
    pub fn previous_level_check<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if call(mem, 0x0064_27e0, &[]) == 0x1d58 {
            return 1;
        }
        let p: u32 = mem.read(mem.rebase(0x0132_6b28));
        u32::from(mem.read::<u8>(p.wrapping_add(3)) & 3 == 1)
    }

    /// The scratch [`layout_message_choices`] hands its callees (0x30
    /// bytes, the original's frame in this order): `+0` an Al Bhed
    /// character (4 bytes), `+4` a font width, `+8` and `+0xc` a message's
    /// width and height, `+0x10` a number's text (32 bytes).
    pub const CHOICE_SCRATCH_BYTES: u32 = 0x30;

    /// [`layout_message_choices`]'s pen.
    struct ChoicePen {
        /// The window (`i16` at `+4` of the record).
        window: u32,
        /// The text mode (`i16` at `+0x1a`), for the width functions.
        mode: u32,
        /// The pen's x, an `f32` (as bits).
        x: u32,
        /// The line's y: 0x10 for the first, 0x10 more per line break.
        y: i32,
        /// The choice item whose width the next item or break closes.
        open: Option<u32>,
        /// See [`CHOICE_SCRATCH_BYTES`].
        scratch: u32,
    }

    impl ChoicePen {
        /// The window's line record (`0x01868a90[window]`).
        fn lines(&self, mem: &impl Mem) -> u32 {
            mem.read(
                mem.rebase(0x0186_8a90)
                    .wrapping_add(self.window.wrapping_mul(4)),
            )
        }

        /// The window (`g_MsgWindows[window]`).
        fn record(&self, mem: &impl Mem) -> u32 {
            mem.read(
                mem.rebase(0x0186_76f0)
                    .wrapping_add(self.window.wrapping_mul(4)),
            )
        }

        /// Where `TOGetMessageWH` leaves a width.
        fn text_width(&self) -> u32 {
            self.scratch.wrapping_add(8)
        }

        /// The x that centres the last measured text in the window
        /// (`(right +0x50 - left +0x48 - width) * 0.5`).
        fn centred(&self, mem: &impl Mem) -> u32 {
            use ffx_bridge::x87::{load, mul, store, sub};
            let w = self.record(mem);
            let right: u32 = mem.read(w.wrapping_add(0x50));
            let left: u32 = mem.read(w.wrapping_add(0x48));
            let width: u32 = mem.read(self.text_width());
            store(mul(sub(sub(load(right), load(left)), load(width)), 0.5))
        }

        /// `x += width`, at `f32` (both `f32`).
        fn advance(&mut self, width: u32) {
            use ffx_bridge::x87::{add, load, store};
            self.x = store(add(load(width), load(self.x)));
        }

        /// The open item's width (`+2`) becomes the pen's distance from
        /// its x (`+4`), truncated; no item is open after.
        fn close(&mut self, mem: &mut impl Mem) {
            use ffx_bridge::x87::{ftol, load, sub};
            if let Some(item) = self.open.take() {
                let start = f64::from(mem.read::<i16>(item.wrapping_add(4)));
                #[allow(clippy::cast_possible_truncation)] // `mov word`
                let width = ftol(mem, sub(load(self.x), start)) as u16;
                mem.write(item.wrapping_add(2), width);
            }
        }
    }

    /// The callees of [`layout_message_choices`].
    const GET_GROUP: u32 = 0x008d_f610;
    const SKIP_CODE: u32 = 0x008d_fd80;
    const MESSAGE_WH: u32 = 0x008b_9650;
    const FONT_WIDTH: u32 = 0x008b_7be0;
    const CONV_ALBHED: u32 = 0x0090_6780;
    const ICON_WIDTH: u32 = 0x008b_65e0;
    const INT_TO_MES: u32 = 0x008b_8290;
    const SET_CHOICE: u32 = 0x008b_8880;
    const MACRO_WIDTH: u32 = 0x008b_6a30;

    /// A group-2 code at `text` (`7` alignment, `0x0b` button icon, `0x10`
    /// choice item) moves the pen.
    fn choice_code<M: Mem>(
        mem: &mut M,
        pen: &mut ChoicePen,
        text: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{ftol, load, store};
        let code: u8 = mem.read(text);
        let arg: u8 = mem.read(text.wrapping_add(1));
        let (window, mode, width, height) = (
            pen.window,
            pen.mode,
            pen.text_width(),
            pen.scratch.wrapping_add(0xc),
        );
        if code == 7 {
            // alignment of the rest of the line: 0x38 and 0x31 centre it,
            // 0x40..0xc0 a fixed x, 0xe1..=0xef a small step right
            call(
                mem,
                MESSAGE_WH,
                &[window, text.wrapping_add(2), width, height, mode, 3],
            );
            if arg == 0x38 {
                pen.x = pen.centred(mem);
                return;
            }
            if arg == 0x31 {
                pen.x = pen.centred(mem);
            }
            if (0x40..0xc0).contains(&arg) {
                pen.x = store(f64::from(i32::from(arg) * 4 - 0x100));
            }
            if (0xe1..=0xef).contains(&arg) {
                pen.advance(store(f64::from(arg - 0xe0)));
            }
            return;
        }
        if code == 0xb {
            let icon = call(mem, ICON_WIDTH, &[u32::from(arg)]).cast_signed();
            pen.advance(store(f64::from(icon)));
        }
        if code != 0x10 {
            return;
        }
        if (0x50..0x60).contains(&arg) {
            // a number item: its text, measured, set as a choice of
            // arg - 0x4f items across the number's width
            let n = u32::from(arg) - 0x4f;
            let number = pen.scratch.wrapping_add(0x10);
            let value: u32 = mem.read(pen.lines(mem).wrapping_add(0xc));
            call(mem, INT_TO_MES, &[value, number, n, 1]);
            call(mem, MESSAGE_WH, &[window, number, width, height, mode, 0]);
            let measured: u32 = mem.read(width);
            let total = ftol(mem, load(measured));
            let x = ftol(mem, load(pen.x));
            call(
                mem,
                SET_CHOICE,
                &[window, n, x, pen.y.cast_unsigned(), total],
            );
            pen.advance(measured);
            return;
        }
        pen.close(mem);
        if arg != 0xff {
            // item arg - 0x30 starts here: 8 bytes from the line record
            // +0x38 (flag 1, width, x, y)
            let lines = pen.lines(mem);
            let count: u8 = mem.read(lines.wrapping_add(5));
            mem.write(lines.wrapping_add(5), count.wrapping_add(1));
            let item = pen
                .lines(mem)
                .wrapping_sub(0x148)
                .wrapping_add(u32::from(arg) * 8);
            mem.write(item, 1u32);
            #[allow(clippy::cast_possible_truncation)] // `mov word`
            let x = ftol(mem, load(pen.x)) as u16;
            mem.write(item.wrapping_add(4), x);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // `mov word`
            mem.write(item.wrapping_add(6), pen.y as u16);
            pen.open = Some(item);
        }
    }

    /// `TOLayoutMessageChoices_008b9a60(w, sel)` (`0x008b9a60`): walks the
    /// text of message window record `w` (`+0x94`; window `i16 +4`, mode
    /// `i16 +0x1a`) and lays out its choice items in the window's line
    /// record (`0x01868a90[i]`: `+4` and the item count `+5` cleared
    /// first). A pen starts at x 0 on line y 0x10; by the codes' groups
    /// (`TOGetGroup`):
    /// - 1 (line codes 0-3): the open item closed, x back to 0 (centred
    ///   for a centred window, `+0xe` 1); 0 ends the text; 1 and 2 start
    ///   over at y 0x10 and end it for a non-zero `sel & 0xf`; 3 steps y
    ///   by 0x10 and ends it for `sel & 0xf` 3.
    /// - 2: [`choice_code`].
    /// - 3 (macros): x + `TOGetChangeMacroWidth(i, text, mode)`.
    /// - 4 (characters): x + `TOGetFontWidthInterMode(text, mode, &w)`,
    ///   through `TOConvAlbhedWord` for an Al Bhed window (`+0x10` 0x4000).
    ///
    /// Then the record's `+8` becomes the widest item; on map 0x17 for
    /// record `+0x14` 0x1c7, `+0x16` 199 the items stack instead (x 0, y
    /// `+ k × 21.5`). `scratch` is [`CHOICE_SCRATCH_BYTES`] long. Returns
    /// what the original leaves in `eax`: the item count, or the line
    /// record with none.
    pub fn layout_message_choices<M: Mem>(
        mem: &mut M,
        w: u32,
        sel: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, ftol, load, mul, store};
        let mut pen = ChoicePen {
            window: i32::from(mem.read::<i16>(w.wrapping_add(4))).cast_unsigned(),
            mode: i32::from(mem.read::<i16>(w.wrapping_add(0x1a))).cast_unsigned(),
            x: 0,
            y: 0x10,
            open: None,
            scratch,
        };
        let mut text: u32 = mem.read(w.wrapping_add(0x94));
        let sel = sel & 0xf;
        let mut max_y = 0;
        let lines = pen.lines(mem);
        mem.write(lines.wrapping_add(4), 0u8);
        let lines = pen.lines(mem);
        mem.write(lines.wrapping_add(5), 0u8);
        loop {
            let code: u8 = mem.read(text);
            let mut more = true;
            match call(mem, GET_GROUP, &[u32::from(code)]) {
                1 => {
                    pen.close(mem);
                    pen.x = 0;
                    if mem.read::<i16>(pen.record(mem).wrapping_add(0xe)) == 1 {
                        let (width, height) = (pen.text_width(), scratch.wrapping_add(0xc));
                        call(
                            mem,
                            MESSAGE_WH,
                            &[pen.window, text.wrapping_add(1), width, height, pen.mode, 3],
                        );
                        pen.x = pen.centred(mem);
                    }
                    match mem.read::<u8>(text) {
                        0 => more = false,
                        1 | 2 => {
                            pen.y = 0x10;
                            more = sel == 0;
                        }
                        3 => {
                            pen.y += 0x10;
                            if sel == 3 && pen.y > max_y {
                                max_y = pen.y;
                                more = false;
                            }
                        }
                        _ => {}
                    }
                }
                2 => choice_code(mem, &mut pen, text, &mut call),
                3 => {
                    let width = fcall(mem, MACRO_WIDTH, &[pen.window, text, pen.mode]);
                    pen.x = store(add(width, load(pen.x)));
                }
                4 => {
                    let width = scratch.wrapping_add(4);
                    let albhed = mem.read::<u16>(pen.record(mem).wrapping_add(0x10)) == 0x4000;
                    if albhed {
                        call(mem, CONV_ALBHED, &[text, scratch]);
                        call(mem, FONT_WIDTH, &[scratch, pen.mode, width]);
                    } else {
                        call(mem, FONT_WIDTH, &[text, pen.mode, width]);
                    }
                    let width: u32 = mem.read(width);
                    pen.advance(width);
                }
                _ => {}
            }
            text = call(mem, SKIP_CODE, &[text]);
            if !more {
                break;
            }
        }
        // the widest item, and map 0x17's stacked items
        let lines = pen.lines(mem);
        mem.write(lines.wrapping_add(8), 0u16);
        let mut eax = pen.lines(mem);
        let stacked = mem.read::<u32>(mem.rebase(0x012f_bc40)) == 0x17
            && mem.read::<i16>(w.wrapping_add(0x14)) == 0x1c7
            && mem.read::<i16>(w.wrapping_add(0x16)) == 199;
        let mut k = 0i32;
        while k < i32::from(mem.read::<i8>(pen.lines(mem).wrapping_add(5))) {
            let item =
                |mem: &M, pen: &ChoicePen| pen.lines(mem).wrapping_add((k * 8).cast_unsigned());
            if stacked {
                mem.write(item(mem, &pen).wrapping_add(0x3c), 0u16);
                let at = item(mem, &pen).wrapping_add(0x3e);
                let y = f64::from(mem.read::<i16>(at));
                #[allow(clippy::cast_possible_truncation)] // `mov word`
                let y = ftol(mem, add(mul(f64::from(k), 21.5), y)) as u16;
                mem.write(at, y);
            }
            let lines = pen.lines(mem);
            let width: i16 = mem.read(item(mem, &pen).wrapping_add(0x3a));
            if width > mem.read::<i16>(lines.wrapping_add(8)) {
                mem.write(lines.wrapping_add(8), width);
            }
            k += 1;
            eax = i32::from(mem.read::<i8>(pen.lines(mem).wrapping_add(5))).cast_unsigned();
        }
        eax
    }

    /// `0x008b8880(i, n, x, y, total)`: message window `i` set up as a
    /// choice of `n` items (`+0x1c` 2): its line record (`0x01868a90`)
    /// gets `+1` and `+4` cleared, `+5` `n`, the cursor and top
    /// (`+2`, `+3`) clamped to 0..`n - 1`, and per item an 8-byte cell at
    /// `+0x38` (1, width `total / n`) followed by the item's position
    /// (`x + k × width`, `y`). A zero `n` makes the original fault
    /// (`idiv`); here it gives a zero width.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::many_single_char_names
    )] // byte and halfword stores
    pub fn set_message_choice(mem: &mut impl Mem, i: u32, n: u32, x: u32, y: u32, total: u32) {
        let m: u32 = mem.read(
            crate::window::bridge::MSG_WINDOWS
                .ptr(mem)
                .addr
                .wrapping_add(i.wrapping_mul(4)),
        );
        mem.write(m.wrapping_add(0x1c), 2u16);
        let slot = mem.rebase(0x0186_8a90).wrapping_add(i.wrapping_mul(4));
        let l: u32 = mem.read(slot);
        mem.write(l.wrapping_add(1), 0u8);
        let l: u32 = mem.read(slot);
        mem.write(l.wrapping_add(4), 0u8);
        let l: u32 = mem.read(slot);
        mem.write(l.wrapping_add(5), n as u8);
        let lim = n.cast_signed().wrapping_sub(1);
        let clamp = |v: i8| -> u8 {
            let v = i32::from(v);
            if v > lim { lim as u8 } else { v.max(0) as u8 }
        };
        for off in [2u32, 3] {
            let l: u32 = mem.read(slot);
            let v: i8 = mem.read(l.wrapping_add(off));
            mem.write(l.wrapping_add(off), clamp(v));
        }
        let step = total
            .cast_signed()
            .checked_div(n.cast_signed())
            .unwrap_or(0);
        let mut px = x;
        let mut k = 0i32;
        while k < n.cast_signed() {
            let cell = (k * 8).cast_unsigned();
            let l: u32 = mem.read(slot);
            mem.write(l.wrapping_add(cell + 0x38), 1u16);
            let l: u32 = mem.read(slot);
            mem.write(l.wrapping_add(cell + 0x3a), step as u16);
            let l: u32 = mem.read(slot);
            mem.write(l.wrapping_add(cell + 0x3c), px as u16);
            let l: u32 = mem.read(slot);
            mem.write(l.wrapping_add(cell + 0x3e), y as u16);
            px = px.wrapping_add(step.cast_unsigned());
            k += 1;
        }
    }

    /// `0x00906e90(win)`: window `win`'s `+6` set to 0x1000 and its
    /// rectangle floats (`+0x28..=+0x34`) snapped: truncated to an
    /// integer (`fistp` with chop), its low 16 bits taken unsigned, and
    /// stored as floats at `+0x48..=+0x54`.
    #[allow(clippy::cast_possible_truncation)] // the low halfword
    pub fn snap_window_rect(mem: &mut impl Mem, win: u32) {
        mem.write(win.wrapping_add(6), 0x1000u16);
        for k in 0..4u32 {
            let v = f64::from(mem.read::<f32>(win.wrapping_add(0x28 + 4 * k)));
            let low = ffx_bridge::x87::cvttsd2si(v) as u16;
            mem.write(win.wrapping_add(0x48 + 4 * k), f32::from(low));
        }
    }

    /// `0x008ad900()`: the TO library's language (`0x01865b5c`), language
    /// swap flag (`g_FFXLangSwapFlag`) and video mode (`g_FFXVideoMode`,
    /// 1 for PAL) from `TkGetLang()` (through `call`): 0 by
    /// `FFXGetAsiaMode()` (0 Japanese 0, 1 → 0x11, 2 → 9, all swapped,
    /// NTSC; another leaves them); 1 English (1, NTSC); 2..=5 (2, 3, 4,
    /// 5) and 18 (0x12) PAL. Other languages leave them.
    pub fn init_language<M: Mem>(mem: &mut M, mut call: impl FnMut(&mut M, u32, &[u32]) -> u32) {
        let set = |mem: &mut M, lang: u32, swap: u32, video: u32| {
            mem.write(mem.rebase(0x0186_5b5c), lang);
            mem.write(mem.rebase(0x0186_5cdc), swap);
            mem.write(mem.rebase(0x0186_5f00), video);
        };
        match call(mem, 0x0088_7c30, &[]) {
            0 => match call(mem, 0x0088_7c20, &[]) {
                0 => set(mem, 0, 1, 0),
                1 => set(mem, 0x11, 1, 0),
                2 => set(mem, 9, 1, 0),
                _ => {}
            },
            1 => set(mem, 1, 0, 0),
            l @ 2..=5 => set(mem, l, 0, 1),
            18 => set(mem, 0x12, 0, 1),
            _ => {}
        }
    }

    /// `0x008b9220(arg)`: a timed macro message: while `0x018663d0` is 1 a
    /// frame is counted (`0x018663d4`), then [`count_map_visits`]'s
    /// counter steps (`TOCountMapVisits`). With frames counted, state
    /// `0x018663d1` 0 picks the map's table (`TOSelectMacroDic_008b56d0`
    /// leaves it in `0x01869eb8`; row `g_GraphicMapId - 0x189` in
    /// `0x018663d8`) and moves to state 1; state 1 shows the next message
    /// (`TOMessageCallF8_008b8ca0(arg)`, column `0x018663dc` stepped up to
    /// 0x32) when its frame (`table[row × 0x32 + column]`) is reached, and
    /// at a 0 frame everything is cleared. Calls go through `call`.
    pub fn timed_macro_message<M: Mem>(
        mem: &mut M,
        arg: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let g = |mem: &M, a: u32| mem.rebase(a);
        let frames_at = g(mem, 0x0186_63d4);
        if mem.read::<u8>(g(mem, 0x0186_63d0)) == 1 {
            let f: u32 = mem.read(frames_at);
            mem.write(frames_at, f.wrapping_add(1));
        }
        call(mem, 0x008b_5680, &[]);
        let n: u32 = mem.read(frames_at);
        if n.cast_signed() <= 0 {
            return;
        }
        let state_at = g(mem, 0x0186_63d1);
        let table_at = g(mem, 0x0186_9eb8);
        let (row_at, col_at) = (g(mem, 0x0186_63d8), g(mem, 0x0186_63dc));
        match mem.read::<u8>(state_at) {
            0 => {
                mem.write(table_at, 0u32);
                call(mem, 0x008b_56d0, &[]);
                if mem.read::<u32>(table_at) != 0 {
                    let map: u32 = mem.read(g(mem, 0x0133_0844));
                    mem.write(state_at, 1u8);
                    mem.write(row_at, map.wrapping_sub(0x189));
                }
            }
            1 => {
                let due = |mem: &M| -> u32 {
                    let t: u32 = mem.read(table_at);
                    let row: u32 = mem.read(row_at);
                    let col: u32 = mem.read(col_at);
                    mem.read(
                        t.wrapping_add(row.wrapping_mul(0x32).wrapping_add(col).wrapping_mul(4)),
                    )
                };
                if due(mem) == n {
                    let col: u32 = mem.read(col_at);
                    if col.cast_signed() < 0x32 {
                        mem.write(col_at, col.wrapping_add(1));
                    }
                    call(mem, 0x008b_8ca0, &[arg]);
                }
                if due(mem) == 0 {
                    mem.write(g(mem, 0x0186_63d0), 0u8);
                    mem.write(frames_at, 0u32);
                    mem.write(state_at, 0u8);
                    mem.write(row_at, 0u32);
                    mem.write(col_at, 0u32);
                    mem.write(g(mem, 0x0186_63e0), 0u32);
                }
            }
            _ => {}
        }
    }

    /// `0x00774680(rec)`: 1 when `0x00c416c0` is 8 and record `rec` (not
    /// null) has an index `+0x26` above 8 whose entry in `*0x0112a0a4` is
    /// -400, and holds the language's id at `+4` (0x25a for languages 2
    /// and 3, 0x276 for 10 and 11, else 0x266; `TOGetFFXLang`, `lang`) with
    /// 0xbb at `+6`; else 0.
    pub fn is_lang_special_record<M: Mem>(
        mem: &mut M,
        rec: u32,
        lang: impl FnOnce(&mut M) -> u32,
    ) -> u32 {
        if mem.read::<u32>(mem.rebase(0x00c4_16c0)) != 8 || rec == 0 {
            return 0;
        }
        let k = i32::from(mem.read::<i16>(rec.wrapping_add(0x26)));
        if k <= 8 {
            return 0;
        }
        let table: u32 = mem.read(mem.rebase(0x0112_a0a4));
        if mem.read::<u32>(table.wrapping_add(k.cast_unsigned().wrapping_mul(4))) != 0xffff_fe70 {
            return 0;
        }
        let want: u16 = match lang(mem) {
            2 | 3 => 0x25a,
            10 | 11 => 0x276,
            _ => 0x266,
        };
        u32::from(
            mem.read::<u16>(rec.wrapping_add(4)) == want
                && mem.read::<u16>(rec.wrapping_add(6)) == 0xbb,
        )
    }

    /// `TOInitFont1208()` (`0x00821aa0`): 0x38 bytes at `0x012fbbbc` and
    /// `g_GsRegisterSink` cleared.
    pub fn init_font_1208(mem: &mut impl Mem) {
        let at = mem.rebase(0x012f_bbbc);
        mem.write_bytes(at, &[0u8; 0x38]);
        mem.write(mem.rebase(0x0230_56e8), 0u32);
    }

    /// `EventFontTextureArrayReset()` (`0x008ab550`): the interned-name
    /// count (`0x01841d90`) and its 0x4000-byte slots (`0x025cc5a0`)
    /// cleared.
    pub fn event_font_texture_array_reset(mem: &mut impl Mem) {
        mem.write(mem.rebase(0x0184_1d90), 0u32);
        let at = mem.rebase(0x025c_c5a0);
        mem.write_bytes(at, &[0u8; 0x4000]);
    }

    /// `0x008a2030(max)`: the gil-input record (`0x01340454`, 7 words)
    /// cleared and set up: the gil (`MsGetGIL`, through `call`) at `+8` and
    /// `+0x14`, the start value `+0xc` = the gil, or `max` when that is not
    /// negative and lower; `+2` the number of digits of that value
    /// (`sprintf("%d")`, minus one); `0x023cc038` cleared. Returns the
    /// digit count less one.
    #[allow(clippy::cast_possible_truncation)] // `mov word`
    pub fn init_gil_input<M: Mem>(
        mem: &mut M,
        max: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let rec = mem.rebase(0x0134_0454);
        for k in 0..7u32 {
            mem.write(rec.wrapping_add(4 * k), 0u32);
        }
        let gil = call(mem, 0x0078_4e80, &[]);
        mem.write(rec.wrapping_add(8), gil);
        let start = if max.cast_signed() < 0 || gil.cast_signed() < max.cast_signed() {
            gil
        } else {
            max
        };
        mem.write(rec.wrapping_add(0x14), gil);
        mem.write(rec.wrapping_add(4), 0u32);
        mem.write(rec.wrapping_add(0xc), start);
        mem.write(rec.wrapping_add(0x10), 0u32);
        let digits = start.cast_signed().to_string().len() as u32;
        mem.write(mem.rebase(0x023c_c038), 0u32);
        let n = digits.wrapping_sub(1);
        mem.write(rec.wrapping_add(2), n as u16);
        n
    }

    /// The record search of `0x00775ce0` and `0x00775de0`: the current
    /// page and record (`0x0112a088`, `0x0112a084`), the relative page
    /// `0x0112a0a0` and `0x0112a098` cleared; then the pages of
    /// `*0x0112a01c` (8 bytes each: offset from `*0x0112a03c`, i16 record
    /// count; `0x0112a080` pages) are searched, each page's records first
    /// reset (`ResetRecords_00774740`, through `call`), for the first
    /// 0x138-byte record `hit` accepts. Found: it becomes current, the page
    /// relative to `*0x0112a03c` goes in `0x0112a0a0`, and its `+0x1c`
    /// value pairs (at `*0x0112a028 + 8 × +0x1e`) are stored into the
    /// value table `*0x0112a0a4`.
    #[allow(clippy::many_single_char_names)]
    fn find_record<M: Mem>(
        mem: &mut M,
        mut hit: impl FnMut(&M, u32) -> bool,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let g = |mem: &M, a: u32| mem.rebase(a);
        let word = |mem: &M, a: u32| -> u32 { mem.read(mem.rebase(a)) };
        for a in [0x0112_a0a0u32, 0x0112_a098, 0x0112_a084, 0x0112_a088] {
            mem.write(g(mem, a), 0u32);
        }
        if word(mem, 0x0112_a01c) == 0 {
            return;
        }
        let mut page = 0u32;
        while page.cast_signed() < word(mem, 0x0112_a080).cast_signed() {
            let pages = word(mem, 0x0112_a01c);
            let entry = pages.wrapping_add(page.wrapping_mul(8));
            let mut rec = mem.read::<u32>(entry).wrapping_add(word(mem, 0x0112_a03c));
            let n = i32::from(mem.read::<i16>(entry.wrapping_add(4)));
            call(mem, 0x0077_4740, &[]);
            for _ in 0..n.max(0) {
                if hit(mem, rec) {
                    mem.write(g(mem, 0x0112_a088), page);
                    let rel = page.wrapping_sub(word(mem, 0x0112_a03c));
                    mem.write(g(mem, 0x0112_a084), rec);
                    mem.write(g(mem, 0x0112_a0a0), rel);
                    let count = mem.read::<u16>(rec.wrapping_add(0x1c));
                    if count == 0 {
                        return;
                    }
                    let start = i32::from(mem.read::<i16>(rec.wrapping_add(0x1e)));
                    let mut pair =
                        word(mem, 0x0112_a028).wrapping_add(start.wrapping_mul(8).cast_unsigned());
                    let values = word(mem, 0x0112_a0a4);
                    let mut j = 0u32;
                    while j < u32::from(mem.read::<u16>(rec.wrapping_add(0x1c))) {
                        let k: u32 = mem.read(pair);
                        let v: u32 = mem.read(pair.wrapping_add(4));
                        mem.write(values.wrapping_add(k.wrapping_mul(4)), v);
                        j += 1;
                        pair = pair.wrapping_add(8);
                    }
                    return;
                }
                rec = rec.wrapping_add(0x138);
            }
            page += 1;
        }
    }

    /// `0x00775ce0()`: [`find_record`] for the record at the previous
    /// relative page's start (`0x0112a0a0 + *0x0112a03c` taken before the
    /// clear, compared with the record address).
    pub fn find_record_at<M: Mem>(mem: &mut M, call: impl FnMut(&mut M, u32, &[u32]) -> u32) {
        let target = mem
            .read::<u32>(mem.rebase(0x0112_a0a0))
            .wrapping_add(mem.read::<u32>(mem.rebase(0x0112_a03c)));
        find_record(mem, |_, rec| rec == target, call);
    }

    /// `0x00775de0()`: [`find_record`] for the first record with bit 0 of
    /// `+1` set.
    pub fn find_record_flagged<M: Mem>(mem: &mut M, call: impl FnMut(&mut M, u32, &[u32]) -> u32) {
        find_record(
            mem,
            |m, rec| m.read::<u8>(rec.wrapping_add(1)) & 1 != 0,
            call,
        );
    }

    /// `0x007767d0()`: fixed edits to the menu's records (0x138 bytes) for
    /// two screens (`0x00c416c0`). Screen 5, first page: records 3 and 4
    /// get `+6` 0x53 and 0x83, 5 is unlinked (`UnlinkRecord_00776910`,
    /// through `call`), 7 cleared at `+1` and unlinked, 8 flagged (`+1`
    /// bit 0) with `+6` 0x56, 9 `+6` 0x86. Screen 0xb: on page 13 records 7
    /// and 12 unlinked and records 4 and 8 get `+6` 0xb4 and 0xb8; on page
    /// 14 records 5 and 8 unlinked and 9 and 6 get 0x88 and 0x84.
    pub fn patch_menu_records<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let word = |mem: &M, a: u32| -> u32 { mem.read(mem.rebase(a)) };
        match word(mem, 0x00c4_16c0) {
            5 => {
                let pages = word(mem, 0x0112_a01c);
                if pages == 0 {
                    return;
                }
                let n = i32::from(mem.read::<i16>(pages.wrapping_add(4)));
                let mut rec = mem.read::<u32>(pages).wrapping_add(word(mem, 0x0112_a03c));
                for i in 0..n.max(0) {
                    match i {
                        3 => mem.write(rec.wrapping_add(6), 0x53u16),
                        4 => mem.write(rec.wrapping_add(6), 0x83u16),
                        5 => {
                            call(mem, 0x0077_6910, &[rec]);
                        }
                        7 => {
                            mem.write(rec.wrapping_add(1), 0u8);
                            call(mem, 0x0077_6910, &[rec]);
                        }
                        8 => {
                            let f: u8 = mem.read(rec.wrapping_add(1));
                            mem.write(rec.wrapping_add(1), f | 1);
                            mem.write(rec.wrapping_add(6), 0x56u16);
                        }
                        9 => mem.write(rec.wrapping_add(6), 0x86u16),
                        _ => {}
                    }
                    rec = rec.wrapping_add(0x138);
                }
            }
            0xb => {
                let pages = word(mem, 0x0112_a01c);
                if pages == 0 {
                    return;
                }
                let base = mem
                    .read::<u32>(pages.wrapping_add(0x68))
                    .wrapping_add(word(mem, 0x0112_a03c));
                call(mem, 0x0077_6910, &[base.wrapping_add(0x888)]);
                call(mem, 0x0077_6910, &[base.wrapping_add(0xea0)]);
                mem.write(base.wrapping_add(0x4e6), 0xb4u16);
                mem.write(base.wrapping_add(0x9c6), 0xb8u16);
                let base = mem
                    .read::<u32>(pages.wrapping_add(0x70))
                    .wrapping_add(word(mem, 0x0112_a03c));
                call(mem, 0x0077_6910, &[base.wrapping_add(0x618)]);
                call(mem, 0x0077_6910, &[base.wrapping_add(0x9c0)]);
                mem.write(base.wrapping_add(0xafe), 0x88u16);
                mem.write(base.wrapping_add(0x756), 0x84u16);
            }
            _ => {}
        }
    }

    /// `0x00775820(_, image, kind, &w, &h)`: the texture size of a menu
    /// picture on screens 5 and 0xb (`0x00c416c0`): each known image needs
    /// its kind, then gets (w, h) of 0x200², 0x1e0², 0x100², 0x100×0x80,
    /// 0x80×0x100, 0x80² or 0x80×0x40; anything else leaves both alone.
    pub fn menu_picture_size(mem: &mut impl Mem, image: u32, kind: u32, w: u32, h: u32) {
        const BIG: (u16, u16) = (0x200, 0x200);
        let table: &[(u32, u32, (u16, u16))] = match mem.read::<u32>(mem.rebase(0x00c4_16c0)) {
            5 => &[
                (5, 5, BIG),
                (6, 0x10, BIG),
                (7, 0xf, BIG),
                (0xc, 10, (0x1e0, 0x1e0)),
                (0x10, 0xe, (0x80, 0x80)),
                (10, 8, (0x100, 0x80)),
                (8, 6, (0x100, 0x100)),
                (0x11, 0xd, (0x80, 0x40)),
                (9, 7, (0x80, 0x100)),
                (0xb, 9, (0x100, 0x80)),
                (0xe, 0xb, (0x100, 0x100)),
                (0xf, 0xc, (0x80, 0x100)),
            ],
            0xb => &[
                (0x13, 5, BIG),
                (0x14, 0x10, BIG),
                (0x15, 0xf, BIG),
                (0x1a, 10, (0x1e0, 0x1e0)),
                (0x1c, 0xb, (0x100, 0x100)),
                (0x1d, 0xc, (0x80, 0x100)),
                (0x19, 9, (0x100, 0x80)),
                (0x1e, 0xe, (0x80, 0x80)),
                (0x18, 8, (0x100, 0x80)),
                (0x16, 6, (0x100, 0x100)),
                (0x1f, 0xd, (0x80, 0x40)),
                (0x17, 7, (0x80, 0x100)),
            ],
            _ => &[],
        };
        if let Some(&(_, _, (sw, sh))) = table.iter().find(|&&(i, k, _)| i == image && k == kind) {
            mem.write(w, sw);
            mem.write(h, sh);
        }
    }

    /// `0x00775a20(_, _, recs, last)`: on screen 8 (`0x00c416c0`) the
    /// layout of records `0..=last` (0x138 bytes from `recs`; halfwords
    /// from `+4`: x, y, w, h, and the value index at `+0x26`) is moved for
    /// the language (`TOGetFFXLang`, `lang`; 3 and 5 are the wide
    /// layouts): fixed (x, y, w, h) rewrites of known boxes, and for value
    /// entries of -400 in `*0x0112a0a4` the boxes matching the language
    /// special-record test of [`is_lang_special_record`].
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn relayout_menu_records<M: Mem>(
        mem: &mut M,
        recs: u32,
        last: u32,
        mut lang: impl FnMut(&mut M) -> u32,
    ) {
        if mem.read::<u32>(mem.rebase(0x00c4_16c0)) != 8 || last.cast_signed() < 0 {
            return;
        }
        let wide = |l: u32| l == 3 || l == 5;
        let mut p = recs.wrapping_add(4);
        for _ in 0..=last {
            let h = |mem: &M, k: u32| -> i16 { mem.read(p.wrapping_add(2 * k)) };
            let set = |mem: &mut M, vals: &[(u32, u16)]| {
                for &(k, v) in vals {
                    mem.write(p.wrapping_add(2 * k), v);
                }
            };
            let idx = h(mem, 0x11);
            let x = h(mem, 0);
            let mut done = false;
            if x == 0x236 {
                if h(mem, 1) == 0xb3 {
                    set(mem, &[(0, 0x23a), (1, 0xaf), (2, 0xa5), (3, 0x2a)]);
                    done = true;
                } else if h(mem, 1) == 0x8a {
                    set(mem, &[(0, 0x23a), (1, 0x89), (2, 0xa0), (3, 0x8a)]);
                    done = true;
                }
            }
            if !done {
                if x == 0x20e && h(mem, 1) == 0xb3 && h(mem, 2) == 0xff && h(mem, 3) == 0x37 {
                    set(mem, &[(0, 0x236), (1, 0xb3), (2, 0xaa), (3, 0x24)]);
                } else {
                    let table: u32 = mem.read(mem.rebase(0x0112_a0a4));
                    let entry: u32 =
                        mem.read(table.wrapping_add((i32::from(idx) * 4).cast_unsigned()));
                    if entry == 0xffff_fe70 {
                        if idx >= 9 && x == 0x23e && h(mem, 1) == 0xc5 {
                            let x = match lang(mem) {
                                2 | 3 => 0x25a,
                                10 | 11 => 0x276,
                                _ => 0x266,
                            };
                            set(mem, &[(0, x), (1, 0xbb)]);
                        } else if (2..9).contains(&idx) {
                            let y = h(mem, 1);
                            if x == 0x20e && y == 0x71 && h(mem, 2) == 0xff {
                                set(mem, &[(3, 0xc2)]);
                                if wide(lang(mem)) {
                                    set(mem, &[(0, 0x222), (1, 0x80), (2, 0xdc), (3, 0xb1)]);
                                } else {
                                    set(mem, &[(0, 0x236), (1, 0x8a), (2, 0xaa), (3, 0x89)]);
                                }
                            } else if x == 0x23e && y == 0x7e {
                                if wide(lang(mem)) {
                                    set(mem, &[(0, 0x24d), (1, 0x8c)]);
                                } else {
                                    set(mem, &[(0, 0x261), (1, 0x92)]);
                                }
                            } else if x == 0x27a && y == 0xae {
                                if wide(lang(mem)) {
                                    set(mem, &[(0, 0x25c), (1, 0xbd)]);
                                } else {
                                    set(mem, &[(0, 0x270), (1, 0xb3)]);
                                }
                            } else if x == 0x27a && y == 0xc6 {
                                if wide(lang(mem)) {
                                    set(mem, &[(0, 0x25c), (1, 0xcb)]);
                                } else {
                                    set(mem, &[(0, 0x270), (1, 0xc1)]);
                                }
                            } else if x == 0x28c && y == 0x107 {
                                if wide(lang(mem)) {
                                    set(mem, &[(0, 0x28c), (1, 0xff)]);
                                } else {
                                    set(mem, &[(0, 0x282), (1, 0xeb)]);
                                }
                            } else if x == 0x28c && y == 0x119 {
                                if wide(lang(mem)) {
                                    set(mem, &[(0, 0x28c), (1, 0x111)]);
                                } else {
                                    set(mem, &[(0, 0x282), (1, 0xf9)]);
                                }
                            }
                        }
                    }
                }
            }
            p = p.wrapping_add(0x138);
        }
    }

    /// `0x00775530()`: whether the menu cursor may move from the current
    /// record (`0x0112a084`) in the pressed direction, left in byte
    /// `0x0112a039` (1 blocked). The pad word is picked by the record's
    /// `+2` (0 `0x0112a090`, 1 `0x0112a08c`, 2 `0x0112a094`); its last
    /// direction bit wins (0x1000 up 0, 0x4000 1, 0x2000 3, 0x8000 2, 0x80
    /// 5, 0x10 6, 0x40 7, 0x20 4) and is kept in `0x0112a08c`. The
    /// direction's 12-byte link at `+0x30` names a target record (`+8`,
    /// relative) and a condition (`+0`, index into the function table
    /// `*0x00c416d0`, argument `+2`, expected `+4`). Each record at the
    /// target's address is checked: no condition (negative) or a condition
    /// whose result disagrees with the expected flag blocks. The
    /// condition calls go through `cond(function, argument)`.
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::many_single_char_names
    )]
    pub fn menu_cursor_blocked<M: Mem>(mem: &mut M, mut cond: impl FnMut(&mut M, u32, u32) -> u32) {
        let g = |mem: &M, a: u32| mem.rebase(a);
        let word = |mem: &M, a: u32| -> u32 { mem.read(mem.rebase(a)) };
        let blocked = g(mem, 0x0112_a039);
        mem.write(blocked, 0u8);
        let pages = word(mem, 0x0112_a01c);
        let cur = word(mem, 0x0112_a084);
        if pages == 0 || cur == 0 {
            return;
        }
        let pad_at = match mem.read::<u8>(cur.wrapping_add(2)) {
            0 => 0x0112_a090,
            1 => 0x0112_a08c,
            2 => 0x0112_a094,
            _ => return,
        };
        let bits = u32::from(mem.read::<u16>(g(mem, pad_at)));
        if bits == 0 {
            return;
        }
        let last_at = g(mem, 0x0112_a08c);
        let mut dir = -1i32;
        for (bit, d) in [
            (0x1000u32, 0),
            (0x4000, 1),
            (0x2000, 3),
            (0x8000, 2),
            (0x80, 5),
            (0x10, 6),
            (0x40, 7),
            (0x20, 4),
        ] {
            if bits & bit != 0 {
                dir = d;
                mem.write(last_at, bit as u16);
            }
        }
        if dir < 0 {
            return;
        }
        let link = (dir * 0xc).cast_unsigned();
        let base0 = word(mem, 0x0112_a03c);
        let target = mem
            .read::<u32>(cur.wrapping_add(link + 0x38))
            .wrapping_add(base0);
        if target == 0 {
            return;
        }
        let page_count = word(mem, 0x0112_a080);
        let mut page = 0i32;
        while page < page_count.cast_signed() {
            let pages = word(mem, 0x0112_a01c);
            let entry = pages.wrapping_add((page * 8).cast_unsigned());
            let n = i32::from(mem.read::<i16>(entry.wrapping_add(4)));
            let mut rec = mem.read::<u32>(entry).wrapping_add(word(mem, 0x0112_a03c));
            for _ in 0..n.max(0) {
                if rec == target {
                    let cur = word(mem, 0x0112_a084);
                    let c = i32::from(mem.read::<i16>(cur.wrapping_add(link + 0x30)));
                    if c < 0 {
                        mem.write(blocked, 1u8);
                        return;
                    }
                    let arg =
                        i32::from(mem.read::<i16>(cur.wrapping_add(link + 0x32))).cast_unsigned();
                    let table = word(mem, 0x00c4_16d0);
                    let f: u32 = mem.read(table.wrapping_add((c * 4).cast_unsigned()));
                    let r = cond(mem, f, arg);
                    let cur = word(mem, 0x0112_a084);
                    let expect = mem.read::<u8>(cur.wrapping_add(link + 0x34)) != 0;
                    if (r != 0) != expect {
                        mem.write(blocked, 1u8);
                        return;
                    }
                }
                rec = rec.wrapping_add(0x138);
            }
            page += 1;
        }
    }

    /// `0x00776110(&result)`: one step of the help/menu screen's cursor.
    /// `*result` 0 nothing, 1 moved, 2 an entry chosen.
    ///
    /// With a request pending (`0x0112a038` 1) the request is handed over
    /// (`TOMenuHelpSetExtMode(kind, value)`, banner prints through
    /// `dbgPrintf`, `0x0112a060` = 1 on screen 0xb for kind 1 else 0,
    /// `TOMenuTermHelp()`); any request state then becomes 2 with the pad
    /// words cleared. Otherwise the pressed direction (as in
    /// [`menu_cursor_blocked`]) is followed to its target record: a failed
    /// condition plays the record's `+0xf8` sound and the scan goes on; a
    /// passed one moves the cursor there (sounds `+0xf6` for a page change,
    /// `+0xf0` within the page, unless one already played), resets the
    /// records on a page change, copies the target's value pairs, clears
    /// the pad words and reports 1. Without a move, an entry of kind 1
    /// (`+0xfb`, pad 0x40) or 2 (pad 0x20) is chosen when it has no
    /// condition (`+0x100` negative) or the condition's result (argument
    /// `+0x102`) differs from the flag `+0xfc`; links pass the same way: the request kind
    /// (`+0xfa`) and value (the value table entry `+0xfe`, unless -1) are
    /// posted with its sound (`+0xf2` / `+0xf4`), and `0x0112a05c` is
    /// cleared (kind 1) or set (kind 2) on screen 0xb; a failed kind-1
    /// condition plays `+0xf8` and clears `0x0112a05c` there. Calls go
    /// through `call`, conditions through `cond(function, argument)`.
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::many_single_char_names
    )]
    pub fn menu_cursor_step<M: Mem>(
        mem: &mut M,
        result: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut cond: impl FnMut(&mut M, u32, u32) -> u32,
    ) {
        let g = |mem: &M, a: u32| mem.rebase(a);
        let word = |mem: &M, a: u32| -> u32 { mem.read(mem.rebase(a)) };
        let old_page = word(mem, 0x0112_a088);
        let state_at = g(mem, 0x0112_a038);
        let state: u8 = mem.read(state_at);
        mem.write(result, 0u32);
        let clear_pads = |mem: &mut M| {
            for a in [0x0112_a090u32, 0x0112_a08c, 0x0112_a094] {
                mem.write(mem.rebase(a), 0u16);
            }
        };
        let sound = |mem: &mut M,
                     call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
                     rec: u32,
                     off: u32|
         -> bool {
            let s: i16 = mem.read(rec.wrapping_add(off));
            if s == 0 {
                return false;
            }
            call(
                mem,
                0x0088_6e80,
                &[i32::from(s).cast_unsigned() | 0x8000_0000],
            );
            true
        };
        let screen_b = |mem: &M| word(mem, 0x00c4_16c0) == 0xb;
        if state != 0 {
            if state == 1 {
                let (kind, value) = (word(mem, 0x0112_a058), word(mem, 0x0112_a054));
                call(mem, 0x008b_ef10, &[kind, value]);
                let (line, gap, args) = (
                    g(mem, 0x00b5_30dc),
                    g(mem, 0x00b5_30f8),
                    g(mem, 0x00b5_3114),
                );
                call(mem, 0x0062_f500, &[line]);
                call(mem, 0x0062_f500, &[line]);
                call(mem, 0x0062_f500, &[gap]);
                let (kind, value) = (word(mem, 0x0112_a058), word(mem, 0x0112_a054));
                call(mem, 0x0062_f500, &[args, kind, value]);
                call(mem, 0x0062_f500, &[gap]);
                call(mem, 0x0062_f500, &[line]);
                call(mem, 0x0062_f500, &[line]);
                let flag = u32::from(screen_b(mem) && word(mem, 0x0112_a058) == 1);
                mem.write(g(mem, 0x0112_a060), flag);
                call(mem, 0x008b_f0b0, &[]);
            }
            mem.write(state_at, 2u8);
            clear_pads(mem);
            return;
        }
        if word(mem, 0x0112_a01c) == 0 || word(mem, 0x0112_a084) == 0 {
            return;
        }
        let cur = word(mem, 0x0112_a084);
        let last_at = g(mem, 0x0112_a08c);
        let pad_at = match mem.read::<u8>(cur.wrapping_add(2)) {
            0 => Some(0x0112_a090),
            1 => Some(0x0112_a08c),
            2 => Some(0x0112_a094),
            _ => None,
        };
        let bits = pad_at.map_or(0, |a| u32::from(mem.read::<u16>(g(mem, a))));
        let mut dir = -1i32;
        if bits != 0 {
            for (bit, d) in [
                (0x1000u32, 0),
                (0x4000, 1),
                (0x2000, 3),
                (0x8000, 2),
                (0x80, 5),
                (0x10, 6),
                (0x40, 7),
                (0x20, 4),
            ] {
                if bits & bit != 0 {
                    dir = d;
                    mem.write(last_at, bit as u16);
                }
            }
        }
        if dir >= 0 {
            let link = (dir * 0xc).cast_unsigned();
            let target = mem
                .read::<u32>(cur.wrapping_add(link + 0x38))
                .wrapping_add(word(mem, 0x0112_a03c));
            let mut played = false;
            if target != 0 {
                let mut page = 0u32;
                let any = word(mem, 0x0112_a080).cast_signed() > 0;
                while page.cast_signed() < word(mem, 0x0112_a080).cast_signed() {
                    let entry = word(mem, 0x0112_a01c).wrapping_add(page * 8);
                    let n = i32::from(mem.read::<i16>(entry.wrapping_add(4)));
                    let mut rec = mem.read::<u32>(entry).wrapping_add(word(mem, 0x0112_a03c));
                    for _ in 0..n.max(0) {
                        if rec == target {
                            let cur = word(mem, 0x0112_a084);
                            let c = i32::from(mem.read::<i16>(cur.wrapping_add(link + 0x30)));
                            let passed = if c < 0 {
                                true
                            } else {
                                let arg = i32::from(mem.read::<i16>(cur.wrapping_add(link + 0x32)))
                                    .cast_unsigned();
                                let f: u32 = mem.read(
                                    word(mem, 0x00c4_16d0).wrapping_add((c * 4).cast_unsigned()),
                                );
                                let r = cond(mem, f, arg);
                                let cur = word(mem, 0x0112_a084);
                                (r != 0) != (mem.read::<u8>(cur.wrapping_add(link + 0x34)) != 0)
                            };
                            if passed {
                                let cur = word(mem, 0x0112_a084);
                                let mut once = played;
                                if !once && page != old_page {
                                    once = sound(mem, &mut call, cur, 0xf6);
                                }
                                let cur = word(mem, 0x0112_a084);
                                if !once && page == old_page {
                                    sound(mem, &mut call, cur, 0xf0);
                                }
                                mem.write(g(mem, 0x0112_a084), target);
                                mem.write(g(mem, 0x0112_a088), page);
                                if page != old_page {
                                    call(mem, 0x0077_4740, &[]);
                                }
                                let rel = page.wrapping_sub(word(mem, 0x0112_a03c));
                                mem.write(g(mem, 0x0112_a0a0), rel);
                                let count = mem.read::<u16>(rec.wrapping_add(0x1c));
                                if count != 0 {
                                    let start = i32::from(mem.read::<i16>(rec.wrapping_add(0x1e)));
                                    let mut pair = word(mem, 0x0112_a028)
                                        .wrapping_add(start.wrapping_mul(8).cast_unsigned());
                                    let values = word(mem, 0x0112_a0a4);
                                    let mut j = 0u32;
                                    while j < u32::from(mem.read::<u16>(rec.wrapping_add(0x1c))) {
                                        let k: u32 = mem.read(pair);
                                        let v: u32 = mem.read(pair.wrapping_add(4));
                                        mem.write(values.wrapping_add(k.wrapping_mul(4)), v);
                                        j += 1;
                                        pair = pair.wrapping_add(8);
                                    }
                                }
                                clear_pads(mem);
                                mem.write(result, 1u32);
                                return;
                            }
                            let cur = word(mem, 0x0112_a084);
                            if sound(mem, &mut call, cur, 0xf8) {
                                played = true;
                            }
                        }
                        rec = rec.wrapping_add(0x138);
                    }
                    page += 1;
                }
                if any && mem.read::<u8>(state_at) != 0 {
                    return;
                }
            }
        }
        let cur = word(mem, 0x0112_a084);
        let last = u32::from(mem.read::<u16>(last_at));
        let kind = mem.read::<i8>(cur.wrapping_add(0xfb));
        let need = match kind {
            1 => 0x40,
            2 => 0x20,
            _ => return,
        };
        if last & need == 0 {
            return;
        }
        let c = i32::from(mem.read::<i16>(cur.wrapping_add(0x100)));
        let passed = if c < 0 {
            true
        } else {
            let arg = i32::from(mem.read::<i16>(cur.wrapping_add(0x102))).cast_unsigned();
            let f: u32 = mem.read(word(mem, 0x00c4_16d0).wrapping_add((c * 4).cast_unsigned()));
            let r = cond(mem, f, arg);
            let cur = word(mem, 0x0112_a084);
            (r != 0) != (mem.read::<u8>(cur.wrapping_add(0xfc)) != 0)
        };
        let cur = word(mem, 0x0112_a084);
        if passed {
            let k = i32::from(mem.read::<i8>(cur.wrapping_add(0xfa))).cast_unsigned();
            mem.write(g(mem, 0x0112_a058), k);
            mem.write(g(mem, 0x0112_a054), 0u32);
            let vi: i16 = mem.read(cur.wrapping_add(0xfe));
            if vi != -1 {
                let v: u32 = mem
                    .read(word(mem, 0x0112_a0a4).wrapping_add((i32::from(vi) * 4).cast_unsigned()));
                mem.write(g(mem, 0x0112_a054), v);
            }
            sound(mem, &mut call, cur, if kind == 1 { 0xf2 } else { 0xf4 });
            mem.write(state_at, 1u8);
            mem.write(result, 2u32);
            if screen_b(mem) {
                mem.write(g(mem, 0x0112_a05c), u32::from(kind == 2));
            }
        } else {
            sound(mem, &mut call, cur, 0xf8);
            if kind == 1 {
                if screen_b(mem) {
                    mem.write(g(mem, 0x0112_a05c), 0u32);
                }
            } else if screen_b(mem) {
                mem.write(g(mem, 0x0112_a05c), 1u32);
            }
        }
    }

    /// `0x00774a50(record)`: whether a menu record's own condition holds.
    /// The condition index (i16 `+0x20`, none when negative) picks a
    /// function from `*0x00c416d0`, called with the i16 `+0x22`; mode
    /// (`+0x2b`) 0 wants a zero result, 1 a nonzero one, anything else
    /// fails.
    pub fn menu_record_condition<M: Mem>(
        mem: &mut M,
        record: u32,
        mut cond: impl FnMut(&mut M, u32, u32) -> u32,
    ) -> u32 {
        let c = mem.read::<i16>(record.wrapping_add(0x20));
        if c < 0 {
            return 0;
        }
        let arg = i32::from(mem.read::<i16>(record.wrapping_add(0x22))).cast_unsigned();
        let table: u32 = mem.read(mem.rebase(0x00c4_16d0));
        let f: u32 = mem.read(table.wrapping_add((i32::from(c) * 4).cast_unsigned()));
        let r = cond(mem, f, arg);
        match mem.read::<i8>(record.wrapping_add(0x2b)) {
            0 => u32::from(r == 0),
            1 => u32::from(r != 0),
            _ => 0,
        }
    }

    /// `0x0089dfa0(text)`: the pixel width of a battle-window text at 12
    /// per character (codes stepped with `TOSkipCode`), less 2; -2 for an
    /// empty text.
    pub fn text_width_12<M: Mem>(
        mem: &mut M,
        text: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut width = 0u32;
        let mut p = text;
        while mem.read::<u8>(p) != 0 {
            width = width.wrapping_add(0xc);
            p = call(mem, 0x008d_fd80, &[p]);
        }
        width.wrapping_sub(2)
    }

    /// `TOBtlSetTarget(slot, a, b)`: marks battle window `slot` (0xf0-byte
    /// entries at `0x0133c950`) as targeting, runs
    /// `TOCheckBtlCommandTarget(a, b)`, counts the request (`+0xa6`), keeps
    /// the current target (`0x0133d1fc` at `+0x42`, `0x0133d178` at
    /// `+0xd8`) and hands the entry to `TOBtlSetTargetCallback`, whose eax
    /// it returns.
    pub fn to_btl_set_target<M: Mem>(
        mem: &mut M,
        slot: u32,
        a: u32,
        b: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let entry = mem
            .rebase(0x0133_c950)
            .wrapping_add(slot.wrapping_mul(0xf0));
        mem.write(entry.wrapping_add(2), 1u8);
        call(mem, 0x0089_abd0, &[a, b]);
        let target: u16 = mem.read(mem.rebase(0x0133_d1fc));
        let count: u16 = mem.read(entry.wrapping_add(0xa6));
        mem.write(entry.wrapping_add(0xa6), count.wrapping_add(1));
        mem.write(entry.wrapping_add(0x42), target);
        let which: u32 = mem.read(mem.rebase(0x0133_d178));
        mem.write(entry.wrapping_add(0xd8), which);
        call(mem, 0x008a_1090, &[entry])
    }

    /// `TOClearBtlWindow()`: flags `0x0133f77c`, stops sound 0x8000004e
    /// when it plays, resets the pads, selects message area 0 and clears
    /// `0x0133c948`. Returns the area call's eax.
    pub fn to_clear_btl_window<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        mem.write(mem.rebase(0x0133_f77c), 1u32);
        if call(mem, 0x0088_6680, &[0x8000_004e]) != 0 {
            call(mem, 0x0088_6eb0, &[0x8000_004e]);
        }
        call(mem, 0x008a_dbd0, &[]);
        let r = call(mem, 0x008a_f5c0, &[0]);
        mem.write(mem.rebase(0x0133_c948), 0u32);
        r
    }

    /// `TOReset1()`: resets the pads, clears `0x01841e0c` and the message
    /// window mode (`0x018659bc`), then for message areas 1 and 0 inits the
    /// 8 message slots and 8 macro commands, and finally runs
    /// `TOMenuInit`, `TOBtlInit` and `TODapsInit`, returning the last's eax.
    pub fn to_reset1<M: Mem>(mem: &mut M, mut call: impl FnMut(&mut M, u32, &[u32]) -> u32) -> u32 {
        call(mem, 0x008a_dbd0, &[]);
        mem.write(mem.rebase(0x0184_1e0c), 0u32);
        mem.write(mem.rebase(0x0186_59bc), 0u32);
        for area in [1u32, 0] {
            call(mem, 0x008a_f5c0, &[area]);
            for i in 0..8 {
                call(mem, 0x008a_da30, &[i]);
            }
            for i in 0..8 {
                call(mem, 0x008b_81d0, &[i]);
            }
        }
        call(mem, 0x008b_e800, &[]);
        call(mem, 0x0089_77f0, &[]);
        call(mem, 0x0089_dc40, &[])
    }

    /// `0x00774860(&out, &code, arg)`: expands one 8-byte menu text code
    /// (words `w0`, `w1`) at `*out` and steps `*code` past it.
    ///
    /// Kind `(w0 >> 8) & 0xff` 1 copies the string returned by the table
    /// function `w0 >> 16` (from `*0x00c416d0`) for `(w1 & 0xffff) + arg`,
    /// without its terminator. Kind 0 takes a number: from the table
    /// function called with `w1 & 0xffff` (`w0 & 0xff` 0), from the value
    /// table `*0x0112a0a4` (1), or 0. With a nonzero `(w1 >> 16) & 0xff` the
    /// flag and number go to `0x0112a06c` / `0x0112a070`; otherwise the
    /// number is written as six digits, leading zeros as `:` (the last digit
    /// always shown). Other kinds write nothing.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn menu_text_code<M: Mem>(
        mem: &mut M,
        out: u32,
        code: u32,
        arg: u32,
        mut cond: impl FnMut(&mut M, u32, u32) -> u32,
    ) {
        let mut dst: u32 = mem.read(out);
        let at: u32 = mem.read(code);
        let w0: u32 = mem.read(at);
        let w1: u32 = mem.read(at.wrapping_add(4));
        let table: u32 = mem.read(mem.rebase(0x00c4_16d0));
        let func = |mem: &M| -> u32 { mem.read(table.wrapping_add((w0 >> 16) * 4)) };
        match (w0 >> 8) & 0xff {
            1 => {
                let f = func(mem);
                let src = cond(mem, f, (w1 & 0xffff).wrapping_add(arg));
                let mut c: u8 = mem.read(src);
                let mut i = 0u32;
                while c != 0 {
                    mem.write(dst, c);
                    dst = dst.wrapping_add(1);
                    i = i.wrapping_add(1);
                    c = mem.read(src.wrapping_add(i));
                }
            }
            0 => {
                let v = match w0 & 0xff {
                    0 => {
                        let f = func(mem);
                        cond(mem, f, w1 & 0xffff)
                    }
                    1 => {
                        let values: u32 = mem.read(mem.rebase(0x0112_a0a4));
                        mem.read(values.wrapping_add((w1 & 0xffff).wrapping_mul(4)))
                    }
                    _ => 0,
                };
                let flag = (w1 >> 16) & 0xff;
                if flag != 0 {
                    mem.write(mem.rebase(0x0112_a06c), flag);
                    mem.write(mem.rebase(0x0112_a070), v);
                } else {
                    let mut digits = [0u16; 6];
                    let mut q = v.cast_signed();
                    for d in &mut digits {
                        let next = q / 10;
                        *d = q.wrapping_sub(next * 10) as u16;
                        q = next;
                    }
                    let mut seen = false;
                    for d in digits[1..].iter_mut().rev() {
                        if *d == 0 {
                            *d = if seen { 0x30 } else { 0x3a };
                        } else {
                            *d = d.wrapping_add(0x30);
                            seen = true;
                        }
                    }
                    digits[0] = digits[0].wrapping_add(0x30);
                    for d in digits.iter().rev() {
                        mem.write(dst, *d as u8);
                        dst = dst.wrapping_add(1);
                    }
                }
            }
            _ => {}
        }
        mem.write(out, dst);
        mem.write(code, at.wrapping_add(8));
    }

    /// `0x0089e410(text)`: the pixel width of a text from the glyph table
    /// at `0x00c58f02` (4 bytes per glyph: `+1` its width, `+0` a flag
    /// adding 5). Codes below 0x30 take glyph `0xa0 + next byte`, others
    /// `code - 0x30`; codes are stepped with `TOSkipCode`. Each glyph adds
    /// its width plus 1 through x87 (`ftol`, so 0x80000000 once out of
    /// range).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn text_width_glyphs<M: Mem>(
        mem: &mut M,
        text: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let glyphs = mem.rebase(0x00c5_8f02);
        let mut width = 0i32;
        let mut p = text;
        loop {
            let c: u8 = mem.read(p);
            if c == 0 {
                return width.cast_unsigned();
            }
            let index = if c < 0x30 {
                u32::from(mem.read::<u8>(p.wrapping_add(1))) + 0xa0
            } else {
                u32::from(c) - 0x30
            };
            let glyph = glyphs.wrapping_add(index * 4);
            let sum = 1.0 + f64::from(mem.read::<u8>(glyph.wrapping_add(1))) + f64::from(width);
            width = if (-2_147_483_648.0..2_147_483_648.0).contains(&sum) {
                sum as i32
            } else {
                i32::MIN
            };
            if mem.read::<u8>(glyph) != 0 {
                width = width.wrapping_add(5);
            }
            p = call(mem, 0x008d_fd80, &[p]);
        }
    }

    /// `0x008b0000(window, kind, type)`: `TOSetWindowKind(window, kind)`
    /// then `TOSetMessageType(window, type)`, returning the latter's eax.
    pub fn to_set_window_kind_and_type<M: Mem>(
        mem: &mut M,
        window: u32,
        kind: u32,
        message_type: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, 0x0090_6e60, &[window, kind]);
        call(mem, 0x008b_8bb0, &[window, message_type])
    }

    /// `getScaleButtonIconWidth(id, scale)` (`0x009062b0`): the width of a
    /// button icon for the HD UI.
    ///
    /// Button ids 0x258..=0x263 and 0x264..=0x267 give
    /// `remap(ftol(0.5 + scale × 54.0))` and `remap(ftol(0.5 + scale ×
    /// 64.0))`; 0x268 and 0x269 give `remap(ftol(scale × 64.5 + 0.5))` and
    /// `remap(ftol(scale × 147.4 + 0.5))` (constants at `0x00b5f1f8`,
    /// `0x00b41308`, `0x00b66cb0`, `0x00b66cd0`, `0x00b92220`; the rounded
    /// value goes through an f32 before `remap`, `graphicUiRemapX2`), any
    /// other id 0. The character `0x025d16e4` then overrides it: digits and
    /// space 9.0, `:`/`;` 13.6, `@`..`O` 20.0 when the low nibble is a
    /// multiple of 3 and 9.0 otherwise (`@` itself 33.5). Constants are read
    /// from the image.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::many_single_char_names
    )]
    pub fn scale_button_icon_width<M: Mem>(
        mem: &mut M,
        id: u32,
        scale: f32,
        mut remap: impl FnMut(&mut M, f32) -> f32,
    ) -> f32 {
        let d = |mem: &M, a: u32| -> f64 { mem.read(mem.rebase(a)) };
        let f = |mem: &M, a: u32| -> f32 { mem.read(mem.rebase(a)) };
        let half = d(mem, 0x00b9_2220);
        let s = f64::from(scale);
        let factor = match id {
            0x258..=0x263 => Some(d(mem, 0x00b5_f1f8)),
            0x264..=0x267 => Some(d(mem, 0x00b4_1308)),
            0x268 => Some(d(mem, 0x00b6_6cb0)),
            0x269 => Some(d(mem, 0x00b6_6cd0)),
            _ => None,
        };
        let mut r = 0.0f32;
        if let Some(k) = factor {
            let i = ffx_bridge::x87::ftol(mem, s * k + half).cast_signed();
            r = remap(mem, i as f32);
        }
        let c: u32 = mem.read(mem.rebase(0x025d_16e4));
        if c.wrapping_sub(0x30) <= 9 || c == 0x20 {
            return f(mem, 0x00b5_5140);
        }
        if c.wrapping_sub(0x3a) <= 1 {
            return f(mem, 0x00b6_6d78);
        }
        if c.wrapping_sub(0x40) <= 0xf {
            r = if (c & 0xf).is_multiple_of(3) {
                f(mem, 0x00b4_ae10)
            } else {
                f(mem, 0x00b5_5140)
            };
            if c == 0x40 {
                r = f(mem, 0x00b6_6d7c);
            }
        }
        r
    }

    /// `TOGetScaleButtonIconWidth(c, scale)` (`0x008fa800`): keeps the
    /// character in `0x025d16e4` and picks the icon id for
    /// [`scale_button_icon_width`] (passed in as `icon(id, scale)`): `' '`,
    /// `'!'`, `'"'` and odd `@`..`O` 0x259, even `@`..`O` 0x258, the digits
    /// `c + 0x230` except `1`/`2` with the confirm button swapped
    /// (`TOGetSystemConfirmButtonAssign() == 1`: 0x262/0x261), `` ` `` 0x270;
    /// anything else 0.0.
    pub fn to_get_scale_button_icon_width<M: Mem>(
        mem: &mut M,
        c: u32,
        scale: f32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut icon: impl FnMut(&mut M, u32, f32) -> f32,
    ) -> f32 {
        mem.write(mem.rebase(0x025d_16e4), c);
        let id = match c {
            0x20..=0x22 => 0x259,
            0x40..=0x4f => {
                if c & 1 != 0 {
                    0x259
                } else {
                    0x258
                }
            }
            0x30..=0x39 => {
                if call(mem, 0x008a_cf30, &[]) == 1 && (0x31..=0x32).contains(&c) {
                    if c == 0x31 { 0x262 } else { 0x261 }
                } else {
                    c + 0x230
                }
            }
            0x60 => 0x270,
            _ => return 0.0,
        };
        icon(mem, id, scale)
    }

    /// `TOBtlGetChrName(i)` (`0x008976f0`): a tail jump to
    /// [`to_get_save_chr_name`].
    pub fn to_btl_get_chr_name<M: Mem>(
        mem: &mut M,
        i: u32,
        call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        to_get_save_chr_name(mem, i, call)
    }

    /// The battle message windows `TODrawAMES*` open (`0x0089b140`..):
    /// the slot set up as kind/type `kind_type`, both texts `text`, the
    /// position `(anchor, x, y)`, value 4, made active and opened. Returns
    /// what `TOOpenWindowAndMessage` leaves in `eax`.
    fn open_battle_message<M: Mem>(
        mem: &mut M,
        window: u32,
        text: u32,
        kind_type: [u32; 2],
        position: [u32; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const INIT_SLOT: u32 = 0x008a_da30;
        const KIND_AND_TYPE: u32 = 0x008b_0000;
        const TEXTS: u32 = 0x008b_89d0;
        const POSITION: u32 = 0x008a_f850;
        const VALUE: u32 = 0x008a_fb90;
        const ACTIVE: u32 = 0x008a_f7d0;
        const OPEN: u32 = 0x008a_de10;
        let [anchor, x, y] = position;
        call(mem, INIT_SLOT, &[window]);
        call(mem, KIND_AND_TYPE, &[window, kind_type[0], kind_type[1]]);
        call(mem, TEXTS, &[window, text, text]);
        call(mem, POSITION, &[window, anchor, x, y]);
        call(mem, VALUE, &[window, 4]);
        call(mem, ACTIVE, &[window]);
        call(mem, OPEN, &[window])
    }

    /// `TODrawAMES(window, text, x, y, anchor)` (`0x0089b140`): a battle
    /// message at `(x, y)`, kind 0 type 0. On map 0x185 (`g_CurrentMapNo`)
    /// a message at (0xfa, 0x96) moves up to y 0x8c when the language
    /// (`TOGetFFXLang`, through `0x008ac2f0`) is 2..=5.
    pub fn to_draw_ames<M: Mem>(
        mem: &mut M,
        args: [u32; 5],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const MAP_NO: u32 = 0x012f_bc40;
        const LANG: u32 = 0x008a_c2f0;
        let [window, text, x, mut y, anchor] = args;
        if mem.read::<u32>(mem.rebase(MAP_NO)) == 0x185 && x == 0xfa && y == 0x96 {
            let lang = call(mem, LANG, &[]).cast_signed();
            if lang > 1 && call(mem, LANG, &[]).cast_signed() <= 5 {
                y = 0x8c;
            }
        }
        open_battle_message(mem, window, text, [0, 0], [anchor, x, y], &mut call)
    }

    /// `TODrawAMES_CENTERMES(window, text)` (`0x0089b1d0`): a battle
    /// message centred at (0x100, 0xe0), kind 1 type 0x2402.
    pub fn to_draw_ames_centermes<M: Mem>(
        mem: &mut M,
        window: u32,
        text: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        open_battle_message(mem, window, text, [1, 0x2402], [4, 0x100, 0xe0], &mut call)
    }

    /// `TODrawAMES_EVJIMAKU(window, text)` (`0x0089b230`): an event
    /// subtitle at (0x100, 0x1c0), kind 1 type 0x2002.
    pub fn to_draw_ames_evjimaku<M: Mem>(
        mem: &mut M,
        window: u32,
        text: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        open_battle_message(mem, window, text, [1, 0x2002], [4, 0x100, 0x1c0], &mut call)
    }

    /// `TODrawAMES_EVJIMAKU_ALBHED(window, text)` (`0x0089b290`): the
    /// subtitle with the Al Bhed type bit 0x4000 (type 0x6002).
    pub fn to_draw_ames_evjimaku_albhed<M: Mem>(
        mem: &mut M,
        window: u32,
        text: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        open_battle_message(mem, window, text, [1, 0x6002], [4, 0x100, 0x1c0], &mut call)
    }

    /// `TOGetSaveChrName(i)` (`0x008ac850`): characters from 0x14 on come
    /// from `MsGetRamChrName(i)`; the others from `MsGetSaveChrName(i)`,
    /// after `MsGetSaveConfigEnglish()` when the language
    /// (`TOGetFFXLang`, through `0x008ac2f0`) is 0. Returns the name call's
    /// eax.
    pub fn to_get_save_chr_name<M: Mem>(
        mem: &mut M,
        i: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if i.cast_signed() >= 0x14 {
            return call(mem, 0x0079_af10, &[i]);
        }
        if call(mem, 0x008a_c2f0, &[]) == 0 {
            call(mem, 0x0078_51d0, &[]);
        }
        call(mem, 0x0078_4fb0, &[i])
    }

    /// `TOGetChangeMacroWordCount(table, index)` (`0x008b6b00`): the number
    /// of words in the text `TOGetChangeMacroAddress(table, index)` (0 without one):
    /// each code below 0x26 other than 4 counts one, and a group-3 code
    /// (`TOGetGroup`) adds the count of its own macro
    /// (`TOGetChangeMacroWordCount(table, code address)`, through `count`); codes are
    /// stepped with `TOSkipCode`.
    pub fn to_get_change_macro_word_count<M: Mem>(
        mem: &mut M,
        table: u32,
        index: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        count: &mut impl FnMut(&mut M, u32, u32) -> u32,
    ) -> u32 {
        let mut n = 0u32;
        let mut p = call(mem, 0x008b_6660, &[table, index]);
        if p == 0 {
            return 0;
        }
        loop {
            let c: u8 = mem.read(p);
            if c == 0 {
                return n;
            }
            if call(mem, 0x008d_f610, &[u32::from(c)]) == 3 {
                n = n.wrapping_add(count(mem, table, p));
            }
            let c: u8 = mem.read(p);
            if c >= 0x26 || c == 4 {
                n = n.wrapping_add(1);
            }
            p = call(mem, 0x008d_fd80, &[p]);
        }
    }

    /// `TOGetSaveWindow(a, kind, &count)` (`0x0089b5a0`): the text of
    /// `MsGetSaveWindow(a, kind, &n)` with its length in `*count`. Trailing
    /// 0xff halfwords are dropped, except for kinds 6, 17 and 20; kinds 7
    /// and 15 report at least 1.
    pub fn to_get_save_window<M: Mem>(
        mem: &mut M,
        a: u32,
        kind: u32,
        count: u32,
        local: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        mem.write(local, 0u32);
        let p = call(mem, 0x0078_5410, &[a, kind, local]);
        let mut n: i32 = mem.read(local);
        let case = kind.wrapping_sub(6);
        let raw = matches!(case, 0 | 11 | 14);
        if raw {
            mem.write(count, n);
            return p;
        }
        while n > 0
            && mem.read::<u16>(
                p.wrapping_add(n.cast_unsigned().wrapping_mul(2))
                    .wrapping_sub(2),
            ) == 0xff
        {
            n -= 1;
        }
        mem.write(count, n);
        if matches!(case, 1 | 9) && n <= 0 {
            mem.write(count, 1u32);
        }
        p
    }

    /// `TOValueToMesFontL(value, out, width)` (`0x008b9140`): `value` as
    /// decimal (`%d`, or right-aligned `%<width>d` for a positive width)
    /// in the large message font: digits copied, spaces written as `^`
    /// (language 0 without `MsGetSaveConfigEnglish`), `:` (languages 9 and
    /// 10) or 0xfe, other characters dropped, then a terminator. The
    /// language (`TOGetFFXLang` through `0x008ac2f0`) is asked for every
    /// space as the original does.
    pub fn to_value_to_mes_font_l<M: Mem>(
        mem: &mut M,
        value: u32,
        out: u32,
        width: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let v = value.cast_signed();
        let w = width.cast_signed();
        let text = if w > 0 {
            format!("{v:>w$}", w = w.cast_unsigned() as usize)
        } else {
            format!("{v}")
        };
        let mut p = out;
        for c in text.bytes() {
            let b = match c {
                b'0'..=b'9' => c,
                b' ' => {
                    let lang = call(mem, 0x008a_c2f0, &[]);
                    if lang == 0 && call(mem, 0x0078_51d0, &[]) == 0 {
                        0x5e
                    } else if call(mem, 0x008a_c2f0, &[]) == 0xa || call(mem, 0x008a_c2f0, &[]) == 9
                    {
                        0x3a
                    } else {
                        0xfe
                    }
                }
                _ => continue,
            };
            mem.write(p, b);
            p = p.wrapping_add(1);
        }
        mem.write(p, 0u8);
    }

    /// `TOCheckBtlCommandUse(chr, command)` (`0x0089aca0`): 0 for command
    /// 0xfe, -2 for 0xff, -1 when `MsGetCommandUse` says 1, -2 when
    /// `MsCheckUseCommand(chr, data, 0x0133f106)` fails. Item-like
    /// commands (`+0x16 & 0xf8` 8 or 0x10) return their window kind
    /// `+0x17`, or -4/-3 when `TOGetSaveWindow` gives an empty list;
    /// others 0xfe when `+0x1a` bit 0 is set, else 0xfe plus whether
    /// `0x0133c94c` is not 0x100. `scratch` holds the two out-words.
    pub fn to_check_btl_command_use<M: Mem>(
        mem: &mut M,
        chr: u32,
        command: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        match command {
            0xfe => return 0,
            0xff => return 0xffff_fffe,
            _ => {}
        }
        let data = call(mem, 0x0079_a4b0, &[command, scratch]);
        if call(mem, 0x0079_a5b0, &[chr, command]) == 1 {
            return 0xffff_ffff;
        }
        let user: u16 = mem.read(mem.rebase(0x0133_f106));
        if call(mem, 0x0078_c690, &[chr, data, u32::from(user)]) == 0xffff_ffff {
            return 0xffff_fffe;
        }
        let kind = u32::from(mem.read::<u8>(data.wrapping_add(0x16))) & 0xf8;
        if kind == 8 || kind == 0x10 {
            let window = u32::from(mem.read::<u8>(data.wrapping_add(0x17)));
            let count = scratch.wrapping_add(4);
            mem.write(count, command);
            call(mem, 0x0089_b5a0, &[chr, window, count]);
            if mem.read::<u32>(count) == 0 {
                return u32::from(kind != 8).wrapping_sub(4);
            }
            return u32::from(mem.read::<u8>(data.wrapping_add(0x17)));
        }
        if mem.read::<u8>(data.wrapping_add(0x1a)) & 1 != 0 {
            return 0xfe;
        }
        0xfe + u32::from(mem.read::<u32>(mem.rebase(0x0133_c94c)) != 0x100)
    }

    /// The battle help text buffer (0x80 bytes) and the size
    /// `TOGetMessageWH` gives it: width (`f32`) `0x0133f534`, height
    /// `0x0133f530`.
    const HELP_TEXT: u32 = 0x0133_f538;

    /// The help text's copy and size, as `TOBtlSetDirectHelpMes` and
    /// `TOBtlSetCommandHelpMes` set them: the buffer cleared, `text` copied
    /// up to its terminator (unbounded, as the original), then
    /// `TOGetMessageWH(0, buffer, &w, &h, 0, 0)`; its `eax`.
    fn set_help_text<M: Mem>(
        mem: &mut M,
        text: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let buffer = mem.rebase(HELP_TEXT);
        for k in 0..0x20 {
            mem.write(buffer.wrapping_add(4 * k), 0u32);
        }
        let mut k = 0u32;
        loop {
            let c: u8 = mem.read(text.wrapping_add(k));
            mem.write(buffer.wrapping_add(k), c);
            if c == 0 {
                break;
            }
            k = k.wrapping_add(1);
        }
        let (w, h) = (mem.rebase(0x0133_f534), mem.rebase(0x0133_f530));
        call(mem, 0x008b_9650, &[0, buffer, w, h, 0, 0])
    }

    /// `TOBtlSetDirectHelpMes(text)` (`0x00899ec0`): the help text set to
    /// `text` (nothing for null). Returns `TOGetMessageWH`'s `eax`, `None`
    /// where the original leaves the caller's.
    pub fn to_btl_set_direct_help_mes<M: Mem>(
        mem: &mut M,
        text: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<u32> {
        (text != 0).then(|| set_help_text(mem, text, &mut call))
    }

    /// `TOBtlSetCommandHelpMes(command)` (`0x00899970`): the help text of
    /// a command (`MsGetComData(command & 0xffff, &strings)`, its help at
    /// `strings + u16 [entry + 8]`), measured once in place and once after
    /// the copy. `scratch` (4 bytes) stands in for the original's
    /// `strings` local. Returns the last `TOGetMessageWH`'s `eax`.
    pub fn to_btl_set_command_help_mes<M: Mem>(
        mem: &mut M,
        command: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let entry = call(mem, 0x0079_a4b0, &[command & 0xffff, scratch]);
        let strings: u32 = mem.read(scratch);
        let text = u32::from(mem.read::<u16>(entry.wrapping_add(8))).wrapping_add(strings);
        let (w, h) = (mem.rebase(0x0133_f534), mem.rebase(0x0133_f530));
        call(mem, 0x008b_9650, &[0, text, w, h, 0, 0]);
        set_help_text(mem, text, &mut call)
    }

    /// `TOBtlOpenSimpleHelpMes(i, message)` (`0x0089dba0`): opens help
    /// line `i` (16-byte entries at `0x0133f510`: `+0` 1, `+4` 7, `+8` the
    /// message, `+0xc`/`+0xe` sizes), measures it with
    /// `TOGetMessageWH(7, message, &w, +0xe, 1, 0)` and stores `ftol(w)` at
    /// `+0xc`. `w` lives in `scratch`, starting as `i`. Returns 1.
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_btl_open_simple_help_mes<M: Mem>(
        mem: &mut M,
        i: u32,
        message: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let entry = mem.rebase(0x0133_f510).wrapping_add(i << 4);
        mem.write(entry.wrapping_add(0xe), 0u16);
        mem.write(entry.wrapping_add(0xc), 0u16);
        mem.write(entry, 1u32);
        mem.write(entry.wrapping_add(4), 7u32);
        mem.write(entry.wrapping_add(8), message);
        mem.write(scratch, i);
        call(
            mem,
            0x008b_9650,
            &[7, message, scratch, entry.wrapping_add(0xe), 1, 0],
        );
        let w: f32 = mem.read(scratch);
        let v = ffx_bridge::x87::ftol(mem, f64::from(w));
        mem.write(entry.wrapping_add(0xc), v as u16);
        1
    }

    /// The pad character of [`to_int_to_mes`]: `]` for language 0 without
    /// `MsGetSaveConfigEnglish`, `:` for languages 9 and 10, else 0xfe.
    fn int_pad<M: Mem>(mem: &mut M, call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32) -> u8 {
        if call(mem, 0x008a_c2f0, &[]) == 0 && call(mem, 0x0078_51d0, &[]) == 0 {
            0x5d
        } else if call(mem, 0x008a_c2f0, &[]) == 0xa || call(mem, 0x008a_c2f0, &[]) == 9 {
            0x3a
        } else {
            0xfe
        }
    }

    /// `TOIntToMes(value, out, digits, mode)` (`0x008b8290`): a
    /// non-negative number (negatives become 0) as `digits` characters.
    /// Mode 0 keeps the low digits (`value % 10^digits`, powers at
    /// `0x00c5a044`) and pads on the left with [`int_pad`] characters;
    /// mode 1 clamps to `10^digits - 1` and pads with `0`. Other modes
    /// write an empty text. Returns the number shown.
    ///
    /// Both modes copy `digits` bytes of the 16-byte `sprintf` buffer, so
    /// after a short number they copy past its terminator: the original
    /// copies stale stack bytes there (behind the terminator, so no text
    /// changes); the port copies what its buffer holds, zeros apart from
    /// an earlier longer number.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )]
    pub fn to_int_to_mes<M: Mem>(
        mem: &mut M,
        value: u32,
        out: u32,
        digits: u32,
        mode: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut v = value.cast_signed().max(0);
        let d = digits.cast_signed();
        let power = |mem: &M| -> i32 {
            mem.read(mem.rebase(0x00c5_a044).wrapping_add(digits.wrapping_mul(4)))
        };
        let mut buf = [0u8; 16];
        let print = |buf: &mut [u8; 16], v: i32| -> i32 {
            let s = v.to_string();
            let n = s.len().min(15);
            buf[..n].copy_from_slice(&s.as_bytes()[..n]);
            buf[n] = 0;
            n as i32
        };
        let mut p = out;
        let copy = |mem: &mut M, p: &mut u32, buf: &[u8; 16], from: i32, n: i32| {
            for k in 0..n {
                let b = buf.get((from + k) as usize).copied().unwrap_or(0);
                mem.write(*p, b);
                *p = p.wrapping_add(1);
            }
        };
        match mode {
            0 => {
                let mut len = print(&mut buf, v);
                if len > d {
                    let m = power(mem);
                    v = if m == 0 { 0 } else { v.wrapping_rem(m) };
                    len = print(&mut buf, v);
                }
                if len < d {
                    for _ in 0..d - len {
                        let c = int_pad(mem, &mut call);
                        mem.write(p, c);
                        p = p.wrapping_add(1);
                    }
                    len = d;
                }
                let from = len.wrapping_sub(d);
                if from < len {
                    copy(mem, &mut p, &buf, from, len - from);
                }
            }
            1 => {
                let max = power(mem).wrapping_sub(1);
                if v > max {
                    v = max;
                } else {
                    v = v.max(0);
                }
                let mut len = print(&mut buf, v);
                if len < d {
                    for _ in 0..d - len {
                        mem.write(p, b'0');
                        p = p.wrapping_add(1);
                    }
                    len = d;
                }
                if len > 0 {
                    copy(mem, &mut p, &buf, 0, len);
                }
            }
            _ => {}
        }
        mem.write(p, 0u8);
        v.cast_unsigned()
    }

    /// The kana of [`to_conv_albhed_word`]: an optional `4` lead and page
    /// byte for the converted id's 0xd0-row `q`, then
    /// `(q + 1) × 0x30 + low` (the low byte of `low`). Returns the cursor.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn albhed_kana<M: Mem>(mem: &mut M, mut out: u32, q: i32, low: i32) -> u32 {
        let mut put = |mem: &mut M, b: u8| {
            mem.write(out, b);
            out = out.wrapping_add(1);
        };
        if q > 4 {
            put(mem, 4);
            if q != 5 {
                put(mem, (q + 0x26) as u8);
            }
        } else if (q - 1).cast_unsigned() <= 3 {
            put(mem, (q + 0x2b) as u8);
        }
        let b = ((q as u8).wrapping_add(1))
            .wrapping_mul(0x30)
            .wrapping_add(low as u8);
        put(mem, b);
        mem.write(out, 0u8);
        out
    }

    /// `TOConvAlbhedWord(in, out)` (`0x00906780`): one character of an Al
    /// Bhed text in its translation, written to `out` with a terminator.
    ///
    /// The known letters are the collected bits of `0x011307a0` (none when
    /// `0x018763e0` is -1, all 26 when it is 1). Returns 1 when the
    /// character was translated, 2 when its letter is known and it stays
    /// (or always 2 with all letters), 0 otherwise.
    ///
    /// Language 0 without English: codes 0xaf.. stay (0); 0x5f..=0xae
    /// use the 4-byte table at `0x0187626c` (`+2` the letter, `+1` the
    /// translation); others copy a lead byte below 0x30 and the code.
    /// Languages 9 and 10 (two-byte kana, an optional `4` lead adding
    /// 0x410): the kana id from `0x5e` on (up to 0x6b4 / 0x521 more) has
    /// letter `(id - 0x5e) % 26`; unknown ones shift by a fixed amount
    /// (Japanese 0x359/-0x340, Korean 0x255/-0x1a0 then `TOShiftListedId`)
    /// and are re-encoded. Other languages translate 0x50..=0x69 and
    /// 0x70..=0x89 through the table at `0x018762a8`.
    #[allow(clippy::too_many_lines)]
    pub fn to_conv_albhed_word<M: Mem>(
        mem: &mut M,
        input: u32,
        out: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let switch: i32 = mem.read(mem.rebase(0x0187_63e0));
        let collected: u32 = mem.read(mem.rebase(0x0113_07a0));
        let mask = match switch {
            -1 => 0,
            1 => 0x03ff_ffff,
            _ => collected,
        };
        let all = |mask: u32| if mask == 0x03ff_ffff { 2 } else { 0 };
        let tail = |mem: &mut M, out: u32, b: u8| {
            mem.write(out, b);
            mem.write(out.wrapping_add(1), 0u8);
        };
        let lang = call(mem, 0x008a_c2f0, &[]);
        if lang == 0 && call(mem, 0x0078_51d0, &[]) == 0 {
            let c: u8 = mem.read(input);
            if c >= 0xaf {
                tail(mem, out, c);
                return 0;
            }
            if c >= 0x5f {
                let entry = mem.rebase(0x0187_626c).wrapping_add(u32::from(c) * 4);
                let bit = u32::from(mem.read::<u8>(entry.wrapping_add(2))) & 0x1f;
                if mask & (1 << bit) != 0 {
                    tail(mem, out, c);
                    return 2;
                }
                let t: u8 = mem.read(entry.wrapping_add(1));
                tail(mem, out, t);
                return 1;
            }
            let (mut p, mut o) = (input, out);
            if c < 0x30 {
                mem.write(o, c);
                o = o.wrapping_add(1);
                p = p.wrapping_add(1);
            }
            let c: u8 = mem.read(p);
            tail(mem, o, c);
            return all(mask);
        }
        let lang = call(mem, 0x008a_c2f0, &[]);
        if lang != 0xa && call(mem, 0x008a_c2f0, &[]) != 9 {
            let c: u8 = mem.read(input);
            if (0x50..=0x69).contains(&c) || (0x70..=0x89).contains(&c) {
                let entry = mem.rebase(0x0187_62a8).wrapping_add(u32::from(c) * 4);
                let bit = u32::from(mem.read::<u8>(entry.wrapping_add(2))) & 0x1f;
                if mask & (1 << bit) != 0 {
                    tail(mem, out, c);
                    return 2;
                }
                let t: u8 = mem.read(entry.wrapping_add(1));
                tail(mem, out, t);
                return 1;
            }
            tail(mem, out, c);
            return all(mask);
        }
        // two-byte kana
        let mut p = input;
        let mut base = 0i32;
        let c: u8 = mem.read(p);
        if c < 0x2c && c != 4 {
            mem.write(out, c);
            let next: u8 = mem.read(p.wrapping_add(1));
            mem.write(out.wrapping_add(1), next);
            mem.write(out.wrapping_add(2), 0u8);
            return all(mask);
        }
        if c == 4 {
            base = 0x410;
            p = p.wrapping_add(1);
        }
        let c: u8 = mem.read(p);
        let id = if (0x2c..0x30).contains(&c) {
            let next: u8 = mem.read(p.wrapping_add(1));
            i32::from(c) * 0xd0 - 0x2320 + i32::from(next) + base
        } else {
            i32::from(c) - 0x30 + base
        };
        let japanese = call(mem, 0x008a_c2f0, &[]) == 0xa;
        let range = if japanese { 0x6b4 } else { 0x521 };
        let mut o = out;
        if (id - 0x5e).cast_unsigned() <= range {
            let letter = (id - 0x5e) % 0x1a;
            if mask & (1 << (letter & 0x1f)) != 0 {
                if id >= 0x410 {
                    mem.write(o, 4u8);
                    o = o.wrapping_add(1);
                }
                let c: u8 = mem.read(p);
                if c < 0x30 {
                    mem.write(o, c);
                    o = o.wrapping_add(1);
                    p = p.wrapping_add(1);
                }
                let c: u8 = mem.read(p);
                tail(mem, o, c);
                return 2;
            }
            if japanese {
                let id = if id > 0x3b8 { id - 0x340 } else { id + 0x359 };
                albhed_kana(mem, o, id / 0xd0, id);
            } else {
                let id = if id > 0x2b4 { id - 0x1a0 } else { id + 0x255 };
                let s = call(mem, 0x0090_66f0, &[id.cast_unsigned()]).cast_signed();
                albhed_kana(mem, o, s / 0xd0, s);
            }
            return 1;
        }
        if id >= 0x410 {
            mem.write(o, 4u8);
            o = o.wrapping_add(1);
        }
        let c: u8 = mem.read(p);
        if c < 0x30 {
            mem.write(o, c);
            o = o.wrapping_add(1);
            p = p.wrapping_add(1);
        }
        let c: u8 = mem.read(p);
        tail(mem, o, c);
        all(mask)
    }

    /// `TOGetInternationalBaseDicType()` (`0x008ac760`): the font
    /// dictionary, 4 or 0. Languages 0, 9 and 10 give 4 only with
    /// `MsGetSaveConfigEnglish`; the others always 4.
    pub fn to_get_international_base_dic_type<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let lang = 0x008a_c2f0;
        if call(mem, lang, &[]) != 0 && call(mem, lang, &[]) != 9 && call(mem, lang, &[]) != 0xa {
            return 4;
        }
        if call(mem, 0x0078_51d0, &[]) != 0 {
            4
        } else {
            0
        }
    }

    /// The slant of a glyph in [`to_get_font_info`]: -1.0 for language 10,
    /// 1.0 for languages 1..=8, else 0.0 (constant `0x00b92340`).
    fn font_slant<M: Mem>(mem: &mut M, call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32) -> f32 {
        let lang = 0x008a_c2f0;
        if call(mem, lang, &[]) == 0xa {
            return mem.read(mem.rebase(0x00b9_2340));
        }
        if call(mem, lang, &[]).cast_signed() > 0 && call(mem, lang, &[]).cast_signed() < 9 {
            1.0
        } else {
            0.0
        }
    }

    /// The cell of glyph `index` in [`to_get_font_info`]: its sheet
    /// position (columns of 14 for every two glyphs, rows of 18), the
    /// parity at `+4`, and the box `+8..+0x14` with the width at `+0x1c`.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    fn font_cell<M: Mem>(mem: &mut M, out: u32, index: i32, width: f32) {
        let row = index / 0x12;
        let x = ((index - row * 0x12) / 2 * 0xe) as f32;
        let y = (row * 0x12) as f32;
        let tall: f64 = mem.read(mem.rebase(0x00b5_87d8));
        mem.write(out.wrapping_add(8), x);
        mem.write(out.wrapping_add(4), (index & 1) as u16);
        mem.write(out.wrapping_add(0xc), y);
        mem.write(
            out.wrapping_add(0x10),
            (f64::from(x) + f64::from(width)) as f32,
        );
        mem.write(out.wrapping_add(0x14), (f64::from(y) + tall) as f32);
    }

    /// `TOGetFontInfo(text, out)` (`0x008b7240`): the font page, sheet cell
    /// and size of the glyph at `text`, and the text after it.
    ///
    /// `out`: `+0` the page (0x3c00 by default, 0x3d40 with English or
    /// other languages, 0x3d30/0x3d10 for languages 9/10), `+2` 0x14, `+4`
    /// the cell parity, `+8..+0x14` the cell box, `+0x18` the slant,
    /// `+0x1c` the width (a quarter of the width-table byte from
    /// `TOGetFontWidthTblAdrs`), `+0x20` 18.0. Codes from 0x2c use the
    /// language dictionary (a lead below 0x30 picks a 0xd0 row, a `4` lead
    /// adds 0x410 in languages 9/10); 0x2a/0x2b, 0x28/0x29 and 0x26/0x27
    /// are two-byte codes of tables 1-3 (page 0x3d00, 0x3d01 for table 3;
    /// table 1 sets the parity when `0x01871534` is set); 6 is a table-5
    /// icon (14.0 wide without a table). A `:` joining its neighbours
    /// (`TOJoinsNeighbours`) is 5.0 wide, and a run of `G` in a text that
    /// starts with one is one glyph of the run's width. Other codes, and
    /// 0x5c..=0x5e in Japanese, get a fixed 10.0-wide box. Constants are
    /// read from the image.
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss
    )]
    pub fn to_get_font_info<M: Mem>(
        mem: &mut M,
        text: u32,
        out: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let lang = 0x008a_c2f0;
        let english = 0x0078_51d0;
        let image_f32 = |mem: &M, a: u32| -> f32 { mem.read(mem.rebase(a)) };
        let quarter: f64 = mem.read(mem.rebase(0x00b3_4f90));
        let tall = image_f32(mem, 0x00b5_4c84);
        let mut p = text;
        let mut base = 0i32;
        let flag: u32 = mem.read(mem.rebase(0x025d_16c0));
        let mut page: u16 = if flag == 1 && call(mem, lang, &[]) == 0 {
            if call(mem, english, &[]) != 0 {
                0x3d40
            } else {
                0x3c00
            }
        } else {
            0x3c00
        };
        let c0: u8 = mem.read(text);
        let plain = call(mem, lang, &[]) == 0
            && call(mem, english, &[]) == 0
            && (0x5c..=0x5e).contains(&c0);
        let width_of = |mem: &M, table: u32, index: i32| -> f32 {
            let b: i8 = mem.read(table.wrapping_add(index.cast_unsigned()));
            (f64::from(b) * quarter) as f32
        };
        if !plain {
            page = match call(mem, lang, &[]) {
                0 => 0x3c00,
                l @ (9 | 0xa) => {
                    if c0 == 4 {
                        base = 0x410;
                        p = p.wrapping_add(1);
                    }
                    if l == 9 { 0x3d30 } else { 0x3d10 }
                }
                _ => 0x3d40,
            };
            let c: u8 = mem.read(p);
            if c >= 0x2c {
                let dic = call(mem, 0x008a_c760, &[]);
                let table = call(mem, 0x008a_c3d0, &[dic]);
                let c: u8 = mem.read(p);
                if c < 0x30 {
                    base += (i32::from(c) - 0x2b) * 0xd0;
                    p = p.wrapping_add(1);
                }
                let index = i32::from(mem.read::<u8>(p)) + base - 0x30;
                mem.write(out, page);
                mem.write(out.wrapping_add(2), 0x14u16);
                let slant = font_slant(mem, &mut call);
                mem.write(out.wrapping_add(0x18), slant);
                let mut width = width_of(mem, table, index);
                mem.write(out.wrapping_add(0x1c), width);
                if mem.read::<u8>(p) == 0x3a {
                    if call(mem, 0x008b_5760, &[p]) != 0 {
                        width = image_f32(mem, 0x00b3_eb98);
                    }
                    mem.write(out.wrapping_add(0x1c), width);
                }
                mem.write(out.wrapping_add(0x20), tall);
                font_cell(mem, out, index, width);
                if mem.read::<u8>(text) != 0x47 {
                    return p.wrapping_add(1);
                }
                let mut run = 0u32;
                if mem.read::<u8>(p) == 0x47 {
                    run = 1;
                    while mem.read::<u8>(p.wrapping_add(run)) == 0x47 {
                        run += 1;
                    }
                    if run > 1 {
                        let dic = call(mem, 0x008a_c760, &[]);
                        let w: f32 = mem.read(out.wrapping_add(0x1c));
                        let mut wide = f64::from(run as f32) * f64::from(w);
                        if dic == 4 {
                            wide += wide;
                        }
                        mem.write(out.wrapping_add(0x1c), wide as f32);
                        let four: f64 = mem.read(mem.rebase(0x00b4_3d80));
                        let x: f32 = mem.read(out.wrapping_add(8));
                        mem.write(out.wrapping_add(8), (f64::from(x) + four) as f32);
                        let right: f32 = mem.read(out.wrapping_add(0x10));
                        mem.write(out.wrapping_add(0x10), (f64::from(right) - four) as f32);
                    }
                }
                return p.wrapping_add(run);
            }
            let two_byte = match c {
                0x2a | 0x2b => Some((1u32, 0x2250, 0x0014_3d00u32)),
                0x28 | 0x29 => Some((2, 0x20b0, 0x0014_3d00)),
                0x26 | 0x27 => Some((3, 0x1f10, 0x0014_3d01)),
                6 => Some((5, 0, 0x0014_3d60)),
                _ => None,
            };
            if let Some((which, bias, head)) = two_byte {
                let next = i32::from(mem.read::<u8>(p.wrapping_add(1)));
                let index = if c == 6 {
                    next - 0x30
                } else {
                    next - bias + i32::from(c) * 0xd0
                };
                let table = call(mem, 0x008a_c3d0, &[which]);
                mem.write(out, head);
                let slant = font_slant(mem, &mut call);
                mem.write(out.wrapping_add(0x18), slant);
                let width = if c == 6 && table == 0 {
                    image_f32(mem, 0x00b5_4f84)
                } else {
                    width_of(mem, table, index)
                };
                mem.write(out.wrapping_add(0x1c), width);
                mem.write(out.wrapping_add(0x20), tall);
                font_cell(mem, out, index, width);
                if which == 1 && mem.read::<u32>(mem.rebase(0x0187_1534)) != 0 {
                    mem.write(out.wrapping_add(4), 1u16);
                }
                return p.wrapping_add(2);
            }
        }
        // the fixed box
        mem.write(out.wrapping_add(0x18), 0.0f32);
        mem.write(out, page);
        let narrow = image_f32(mem, 0x00b9_2318);
        mem.write(out.wrapping_add(0x1c), narrow);
        mem.write(out.wrapping_add(2), 0x14u32);
        mem.write(out.wrapping_add(0x20), tall);
        let x = image_f32(mem, 0x00b5_f314);
        mem.write(out.wrapping_add(8), x);
        mem.write(out.wrapping_add(0xc), 0.0f32);
        let right = image_f32(mem, 0x00b5_67a8);
        mem.write(out.wrapping_add(0x10), right);
        let bottom: f64 = mem.read(mem.rebase(0x00b5_87d8));
        let zero: f64 = mem.read(mem.rebase(0x00b0_db90));
        mem.write(out.wrapping_add(0x14), (bottom + zero) as f32);
        p.wrapping_add(1)
    }

    /// `TOGetFontInfoInterMode(text, out, inter)` (`0x008b78b0`): as
    /// [`to_get_font_info`] for the international mode's fonts. Outside
    /// Japanese and languages 9/10 and with `inter` set, a code from 0x2c
    /// is a glyph of width table 4 (page 0x3d40, the slant `+0x18` left as
    /// it is) and code 6 a two-byte icon of table 5 (page 0x3d60, slant
    /// 0); other codes are left to the caller (the same `text` back).
    /// Else Japanese without English gets the fixed 10.0-wide box for
    /// 0x5c..=0x5e (page 0x3c00); in languages 9/10 a `4` lead adds 0x410;
    /// codes below 0x2c are left to the caller; codes from 0x2c are glyphs
    /// of width table 0 (page 0x3c00), a lead below 0x30 picking the 0xd0
    /// row (replacing the 0x410), slanted 2.0 for glyphs 0xea..=0x103.
    /// `out` as in [`to_get_font_info`] (`+0`/`+2` the page and 0x14 as one
    /// word). Returns the text after the glyph. Callees (`TOGetFFXLang`,
    /// `TOGetFontWidthTblAdrs`, `MsGetSaveConfigEnglish`) through `call`.
    pub fn to_get_font_info_inter_mode<M: Mem>(
        mem: &mut M,
        text: u32,
        out: u32,
        inter: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const LANG: u32 = 0x008a_c2f0;
        const WIDTH_TABLE: u32 = 0x008a_c3d0;
        const ENGLISH: u32 = 0x0078_51d0;
        let at = |off: u32| out.wrapping_add(off);
        // a glyph of `table`: the page word, the width (a quarter of the
        // width-table byte), the height 18 and the sheet cell
        let glyph = |mem: &mut M, head: u32, table: u32, index: i32| {
            mem.write(at(0), head);
            let b: i8 = mem.read(table.wrapping_add(index.cast_unsigned()));
            #[allow(clippy::cast_possible_truncation)] // fstp dword
            let width = (f64::from(b) * 0.25) as f32;
            mem.write(at(0x1c), width);
            mem.write(at(0x20), 18.0f32);
            font_cell(mem, out, index, width);
        };
        let lang = call(mem, LANG, &[]);
        let local =
            lang != 0 && call(mem, LANG, &[]) != 9 && call(mem, LANG, &[]) != 10 && inter != 0;
        if local {
            let c: u8 = mem.read(text);
            if c >= 0x2c {
                let table = call(mem, WIDTH_TABLE, &[4]);
                let index = i32::from(mem.read::<u8>(text)) - 0x30;
                glyph(mem, 0x0014_3d40, table, index);
                return text.wrapping_add(1);
            }
            if c != 6 {
                return text;
            }
            let table = call(mem, WIDTH_TABLE, &[5]);
            let index = i32::from(mem.read::<u8>(text.wrapping_add(1))) - 0x30;
            mem.write(at(0x18), 0.0f32);
            glyph(mem, 0x0014_3d60, table, index);
            return text.wrapping_add(2);
        }

        let c: u8 = mem.read(text);
        if call(mem, LANG, &[]) == 0 && call(mem, ENGLISH, &[]) == 0 && (0x5c..=0x5e).contains(&c) {
            // the fixed box
            mem.write(at(0x18), 0.0f32);
            mem.write(at(0x1c), 10.0f32);
            mem.write(at(0), 0x0014_3c00u32);
            mem.write(at(4), 0u16);
            mem.write(at(0x20), 18.0f32);
            mem.write(at(8), 70.0f32);
            mem.write(at(0xc), 0.0f32);
            mem.write(at(0x10), 80.0f32);
            mem.write(at(0x14), 18.0f32);
            return text.wrapping_add(1);
        }
        let mut p = text;
        let mut base = 0i32;
        if matches!(call(mem, LANG, &[]), 9 | 10) && mem.read::<u8>(p) == 4 {
            p = p.wrapping_add(1);
            base = 0x410;
        }
        if mem.read::<u8>(p) < 0x2c {
            return p;
        }
        let table = call(mem, WIDTH_TABLE, &[0]);
        let lead: u8 = mem.read(p);
        if lead < 0x30 {
            base = (i32::from(lead) - 0x2b) * 0xd0;
            p = p.wrapping_add(1);
        }
        let index = i32::from(mem.read::<u8>(p)) + base - 0x30;
        let slant = if index.wrapping_sub(0xea).cast_unsigned() <= 0x19 {
            2.0f32
        } else {
            0.0
        };
        mem.write(at(0x18), slant);
        glyph(mem, 0x0014_3c00, table, index);
        p.wrapping_add(1)
    }

    /// Clears the 0x40-byte glyph record at `0x01841d50` that the battle
    /// text widths pass to `TOGetFontInfo` (the originals `memset` it).
    fn clear_glyph_record<M: Mem>(mem: &mut M) -> u32 {
        let at = mem.rebase(0x0184_1d50);
        for k in 0..0x10 {
            mem.write(at.wrapping_add(4 * k), 0u32);
        }
        at
    }

    /// The advance of the glyph at `p` (`TOGetFontInfo` into the glyph
    /// record, then `TOGetFontAdvance(0, record, scale)`), added to the f32
    /// at `out`.
    #[allow(clippy::cast_possible_truncation)]
    fn add_glyph_advance<M: Mem>(
        mem: &mut M,
        p: u32,
        out: u32,
        scale: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        let record = mem.rebase(0x0184_1d50);
        let next = call(mem, 0x008b_7240, &[p, record]);
        let advance = fcall(mem, 0x008f_a740, &[0, record, scale]);
        let w: f32 = mem.read(out);
        mem.write(out, (advance + f64::from(w)) as f32);
        next
    }

    /// Takes the trailing letter space (`TOGetFontSpaceInPiexl(0)`) off
    /// the f32 at `out`.
    #[allow(clippy::cast_possible_truncation)]
    fn drop_letter_space<M: Mem>(
        mem: &mut M,
        out: u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        let w: f32 = mem.read(out);
        let space = fcall(mem, 0x008f_a570, &[0]);
        mem.write(out, (f64::from(w) - space) as f32);
    }

    /// `ToGetBtlEasyFontWidth(text, &width, _, scale)` (`0x00905290`): the
    /// width of a battle text at `scale` (f32 bits): the advances of its
    /// group-4 glyphs (`TOGetGroup`), less one letter space. Calls go
    /// through `call`, float results (ST0) through `fcall`.
    pub fn to_get_btl_easy_font_width<M: Mem>(
        mem: &mut M,
        text: u32,
        out: u32,
        scale: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        clear_glyph_record(mem);
        mem.write(out, 0.0f32);
        let mut p = text;
        loop {
            let c: u8 = mem.read(p);
            if c == 0 {
                break;
            }
            if call(mem, 0x008d_f610, &[u32::from(c)]) == 4 {
                add_glyph_advance(mem, p, out, scale, &mut call, &mut fcall);
            }
            p = call(mem, 0x008d_fd80, &[p]);
        }
        drop_letter_space(mem, out, &mut fcall);
    }

    /// `ToGetBtlEasyDigitWidth(digits, value, &width, scale)`
    /// (`0x00905190`): the width of `value` written by
    /// `TOValueToMesFontL(value, text, digits)` into the 32-byte `text`
    /// (the original's stack buffer): every glyph's advance, less one
    /// letter space.
    #[allow(clippy::too_many_arguments)]
    pub fn to_get_btl_easy_digit_width<M: Mem>(
        mem: &mut M,
        digits: u32,
        value: u32,
        out: u32,
        scale: u32,
        text: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        call(mem, 0x008b_9140, &[value, text, digits]);
        clear_glyph_record(mem);
        mem.write(out, 0.0f32);
        let mut p = text;
        while mem.read::<u8>(p) != 0 {
            add_glyph_advance(mem, p, out, scale, &mut call, &mut fcall);
            p = call(mem, 0x008d_fd80, &[p]);
        }
        drop_letter_space(mem, out, &mut fcall);
    }

    /// `ToGetBtlEasyDigitWidth2(value, &width, scale, extra)`
    /// (`0x00905230`): [`to_get_btl_easy_digit_width`] with as many digits
    /// as `value` has (1 for 0).
    pub fn to_get_btl_easy_digit_width2<M: Mem>(
        mem: &mut M,
        value: u32,
        out: u32,
        scale: u32,
        extra: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let mut digits = 0u32;
        let mut q = value.cast_signed();
        if q == 0 {
            digits = 1;
        } else {
            while q != 0 {
                q /= 10;
                digits += 1;
            }
        }
        call(mem, 0x0090_5190, &[digits, value, out, scale, extra]);
    }

    /// `TOGetWidthMacro_help(table, code, scale, extra)` (`0x008fa920`):
    /// the width of the macro text `TOGetChangeMacroAddress(table, code)`
    /// (0.0 without one): `7 n` spaces of `n - 0xe0` (0xe0..0xf0), nested
    /// macros 0x12..=0x22 (through `fcall` to itself), other glyphs their
    /// advance at `scale`.
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_get_width_macro_help<M: Mem>(
        mem: &mut M,
        table: u32,
        code: u32,
        scale: u32,
        extra: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> f32 {
        let record = clear_glyph_record(mem);
        let mut acc = 0.0f32;
        let mut p = call(mem, 0x008b_6660, &[table, code]);
        if p == 0 {
            return 0.0;
        }
        loop {
            let c: u8 = mem.read(p);
            if c == 0 {
                return acc;
            }
            if c == 7 {
                let n: u8 = mem.read(p.wrapping_add(1));
                if (0xe0..0xf0).contains(&n) {
                    acc = (f64::from(f32::from(n - 0xe0)) + f64::from(acc)) as f32;
                }
                p = call(mem, 0x008d_fd80, &[p]);
            } else if (0x12..=0x22).contains(&c) {
                let v = fcall(mem, 0x008f_a920, &[table, p, scale, extra]);
                acc = (v + f64::from(acc)) as f32;
                p = call(mem, 0x008d_fd80, &[p]);
            } else {
                p = call(mem, 0x008b_7240, &[p, record]);
                let advance = fcall(mem, 0x008f_a740, &[0, record, scale]);
                acc = (advance + f64::from(acc)) as f32;
            }
        }
    }

    /// `ToGetCrossExtMesFontWidth(table, text, &width, scale, extra)`
    /// (`0x00905320`): the width of a message text: group-4 glyphs their
    /// advance, group-3 macros their width (`TOGetWidthMacro_help`, unless
    /// `table` is -1), group-2 `7 n` spaces of `n - 0xe0` (0xe1..0xf0) and
    /// `0xb n` button icons (`TOGetScaleButtonIconWidth(n, 0x00b66864)`
    /// plus a letter space), less one letter space. It first asks
    /// `graphicFontGetScreenWH` into `scratch` (8 bytes), as the original
    /// does.
    #[allow(clippy::cast_possible_truncation, clippy::too_many_arguments)]
    pub fn to_get_cross_ext_mes_font_width<M: Mem>(
        mem: &mut M,
        table: u32,
        text: u32,
        out: u32,
        scale: u32,
        extra: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        call(mem, 0x0064_0db0, &[scratch, scratch.wrapping_add(4)]);
        clear_glyph_record(mem);
        mem.write(out, 0.0f32);
        let mut p = text;
        loop {
            let c: u8 = mem.read(p);
            if c == 0 {
                break;
            }
            let add = match call(mem, 0x008d_f610, &[u32::from(c)]) {
                4 => {
                    add_glyph_advance(mem, p, out, scale, &mut call, &mut fcall);
                    None
                }
                3 if table != u32::MAX => Some(fcall(mem, 0x008f_a920, &[table, p, scale, extra])),
                2 if c == 7 => {
                    let n: u8 = mem.read(p.wrapping_add(1));
                    (n > 0xe0 && n < 0xf0).then(|| f64::from(f32::from(n - 0xe0)))
                }
                2 if c == 0xb => {
                    let n = u32::from(mem.read::<u8>(p.wrapping_add(1)));
                    let size: u32 = mem.read(mem.rebase(0x00b6_6864));
                    let icon = fcall(mem, 0x008f_a800, &[n, size]);
                    let space = fcall(mem, 0x008f_a570, &[0]);
                    Some(space + icon)
                }
                _ => None,
            };
            if let Some(v) = add {
                let w: f32 = mem.read(out);
                mem.write(out, (v + f64::from(w)) as f32);
            }
            p = call(mem, 0x008d_fd80, &[p]);
        }
        drop_letter_space(mem, out, &mut fcall);
    }

    /// `TOGetChangeMacroAddress(window, code)` (`0x008b6660`): the text a
    /// macro code stands for in message window `window`, or 0.
    ///
    /// By the code byte: 0x10 `n` (0x50..=0x57) writes the window's counter
    /// (`*(0x01868a90[window]) + 0xc`) as `n - 0x4f` digits into
    /// `0x01869ec4` (`TOIntToMes` mode 1, kept back in the counter); 0x12
    /// is a macro slot (`0x01869ab0[window × 32 + n - 0x30]`, see below, or
    /// for `n` from 0x50 a window string `u16` offset table at
    /// `*0x025d16c4 - 0x140`); 0x13 a character name
    /// (`TOGetSaveChrName`); 0x14, 0x15 and 0x18 the code itself; 0x16 and
    /// 0x17 an item text (`MsGetRomItem`); 0x19..=0x22 a text table
    /// (`get_ptr32_func` of `0x01869eb0[English]`, entry `code - 0x13`)
    /// indexed by the window's `+0x20` base; 0x23 a key item
    /// (`MsImportantName`).
    ///
    /// A macro slot of kind (`+1`) 0xff is the empty text (`0x00c5a040`);
    /// 0 points at `+4`; 1 and 2 go through `TOFuncMacroCmdValue`;
    /// 0x33..=0x36 name an item, weapon, command or ability (`+4`); 0x37 is
    /// the field pickup (`MsFieldItemGetMes`, then gil, item, key item or
    /// weapon from `0x02310dac..`, the empty text unless exactly one kind);
    /// 0x13..=0x23 fold `+4 / 0xd0` into the kind (and keep `+4 % 0xd0`)
    /// and expand that code (through `scratch + 4`, the original's stack);
    /// 0x2b..=0x32 write `+4` as `kind - 0x2a` digits. `scratch` (8 bytes)
    /// stands in for the original's stack words.
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::many_single_char_names
    )]
    pub fn to_get_change_macro_address<M: Mem>(
        mem: &mut M,
        window: u32,
        code: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let empty = mem.rebase(0x00c5_a040);
        let digits = mem.rebase(0x0186_9ec4);
        let c: u8 = mem.read(code);
        let next: u8 = mem.read(code.wrapping_add(1));
        let n = u32::from(next.wrapping_sub(0x30));
        let window_base = |mem: &M| -> i32 {
            let w: u32 = mem.read(mem.rebase(0x0186_76f0).wrapping_add(window.wrapping_mul(4)));
            i32::from(mem.read::<i16>(w.wrapping_add(0x20)))
        };
        match c {
            0x10 => {
                if !(0x50..=0x57).contains(&next) {
                    return 0;
                }
                let slot = mem.rebase(0x0186_8a90).wrapping_add(window.wrapping_mul(4));
                let w: u32 = mem.read(slot);
                let count: u32 = mem.read(w.wrapping_add(0xc));
                let r = call(
                    mem,
                    0x008b_8290,
                    &[count, digits, u32::from(next) - 0x4f, 1],
                );
                let w: u32 = mem.read(slot);
                mem.write(w.wrapping_add(0xc), r);
                digits
            }
            0x12 => {
                if next >= 0x50 {
                    let strings: u32 = mem.read(mem.rebase(0x025d_16c4));
                    let index = window_base(mem) + i32::from(next) * 2;
                    let at = strings
                        .wrapping_add((index * 2).cast_unsigned())
                        .wrapping_sub(0x140);
                    return u32::from(mem.read::<u16>(at)).wrapping_add(strings);
                }
                let slot = mem
                    .rebase(0x0186_9ab0)
                    .wrapping_add((window.wrapping_shl(5).wrapping_add(n)).wrapping_mul(4));
                let entry = |mem: &M| -> u32 { mem.read(slot) };
                let e = entry(mem);
                let kind: u8 = mem.read(e.wrapping_add(1));
                if kind == 0xff {
                    return empty;
                }
                let mut out = if kind == 0 {
                    mem.read(e.wrapping_add(4))
                } else {
                    code
                };
                if kind == 1 {
                    out = call(mem, 0x008b_6210, &[e]);
                }
                let e = entry(mem);
                if mem.read::<u8>(e.wrapping_add(1)) == 2 {
                    out = call(mem, 0x008b_6210, &[e]);
                }
                let e = entry(mem);
                let kind: u8 = mem.read(e.wrapping_add(1));
                let arg: u32 = mem.read(e.wrapping_add(4));
                let name = match kind {
                    0x33 => Some(0x008b_8db0),
                    0x34 => Some(0x008b_8dd0),
                    0x35 => Some(0x008b_8d70),
                    0x36 => Some(0x008b_8d90),
                    _ => None,
                };
                if let Some(f) = name {
                    return call(mem, f, &[arg]);
                }
                if kind == 0x37 {
                    call(mem, 0x0079_8ff0, &[arg]);
                    let g = |mem: &M, a: u32| mem.rebase(a);
                    let gil: i32 = mem.read(g(mem, 0x0231_0dac));
                    let items = i16::from(mem.read::<i8>(g(mem, 0x0231_0db0)));
                    let keys = i16::from(mem.read::<i8>(g(mem, 0x0231_0db1)));
                    let weapons = i16::from(mem.read::<i8>(g(mem, 0x0231_0db2)));
                    let kinds = keys
                        .wrapping_add(weapons)
                        .wrapping_add(items)
                        .wrapping_add(i16::from(gil > 0));
                    if kinds != 1 {
                        return empty;
                    }
                    if gil > 0 {
                        call(mem, 0x008b_8290, &[gil.cast_unsigned(), digits, 0, 0]);
                        return digits;
                    }
                    if items > 0 {
                        let item = u32::from(mem.read::<u16>(g(mem, 0x0231_0db4)));
                        return call(mem, 0x008b_8db0, &[item]);
                    }
                    if keys > 0 {
                        let key = u32::from(mem.read::<u16>(g(mem, 0x0231_0dcc)));
                        return call(mem, 0x0079_0860, &[key]);
                    }
                    if weapons > 0 {
                        let weapon = u32::from(mem.read::<u16>(g(mem, 0x0231_0dce)));
                        return call(mem, 0x008b_8dd0, &[weapon]);
                    }
                }
                let e = entry(mem);
                let kind = mem.read::<i8>(e.wrapping_add(1));
                if (0x13..=0x23).contains(&kind) {
                    let v: i32 = mem.read(e.wrapping_add(4));
                    mem.write(
                        e.wrapping_add(1),
                        kind.cast_unsigned().wrapping_add((v / 0xd0) as u8),
                    );
                    let e = entry(mem);
                    let v: i32 = mem.read(e.wrapping_add(4));
                    mem.write(e.wrapping_add(4), v % 0xd0);
                    let e = entry(mem);
                    let buf = scratch.wrapping_add(4);
                    let k: u8 = mem.read(e.wrapping_add(1));
                    let low: u8 = mem.read(e.wrapping_add(4));
                    mem.write(buf, k);
                    mem.write(buf.wrapping_add(1), low.wrapping_add(0x30));
                    mem.write(buf.wrapping_add(2), 0u8);
                    out = call(mem, 0x008b_6660, &[window, buf]);
                }
                let e = entry(mem);
                let kind = mem.read::<i8>(e.wrapping_add(1));
                if (0x2b..=0x32).contains(&kind) {
                    let v: u32 = mem.read(e.wrapping_add(4));
                    let count = (i32::from(kind) - 0x2a).cast_unsigned();
                    call(mem, 0x008b_8290, &[v, digits, count, 0]);
                    return digits;
                }
                out
            }
            0x13 => call(mem, 0x008a_c850, &[n]),
            0x14 | 0x15 | 0x18 => code,
            0x16 | 0x17 => {
                let item = call(mem, 0x0079_09f0, &[n, scratch]);
                let base: u32 = mem.read(scratch);
                u32::from(mem.read::<u16>(item)).wrapping_add(base)
            }
            0x19..=0x22 => {
                let english = call(mem, 0x0078_51d0, &[]);
                let tables: u32 = mem.read(
                    mem.rebase(0x0186_9eb0)
                        .wrapping_add(english.wrapping_mul(4)),
                );
                let table = call(mem, 0x00a4_4650, &[tables]);
                let text: u32 = mem.read(table.wrapping_add(u32::from(c) * 4).wrapping_sub(0x4c));
                let index = window_base(mem) + n.cast_signed() * 2;
                u32::from(mem.read::<u16>(text.wrapping_add((index * 2).cast_unsigned())))
                    .wrapping_add(text)
            }
            0x23 => call(mem, 0x0079_0860, &[n]),
            _ => 0,
        }
    }

    /// `TOGetChangeMacroWidth(window, code, mode)` (`0x008b6a30`): the width
    /// of the macro text `TOGetChangeMacroAddress(window, code)` (0.0
    /// without one): nested macros (group 3, through `fcall` to itself),
    /// `7 n` spaces of `n - 0xe0` (0xe1..=0xef), other glyphs
    /// `TOGetFontWidthInterMode(glyph, mode, &w)`. `w` lives in `scratch`
    /// and keeps its last value between glyphs, starting at 0.0.
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_get_change_macro_width<M: Mem>(
        mem: &mut M,
        window: u32,
        code: u32,
        mode: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> f32 {
        let mut acc = 0.0f32;
        mem.write(scratch, 0.0f32);
        let mut p = call(mem, 0x008b_6660, &[window, code]);
        if p == 0 {
            return 0.0;
        }
        loop {
            let c: u8 = mem.read(p);
            if c == 0 {
                return acc;
            }
            if call(mem, 0x008d_f610, &[u32::from(c)]) == 3 {
                let v = fcall(mem, 0x008b_6a30, &[window, p, mode]);
                acc = (v + f64::from(acc)) as f32;
                p = call(mem, 0x008d_fd80, &[p]);
                continue;
            }
            let n: u8 = mem.read(p.wrapping_add(1));
            if c == 7 && (0xe1..=0xef).contains(&n) {
                acc = (f64::from(f32::from(n - 0xe0)) + f64::from(acc)) as f32;
                p = call(mem, 0x008d_fd80, &[p]);
                continue;
            }
            p = call(mem, 0x008b_7be0, &[p, mode, scratch]);
            let w: f32 = mem.read(scratch);
            acc = (f64::from(w) + f64::from(acc)) as f32;
        }
    }

    /// `TOGetMessageWH(window, text, &w, &h, mode, flags)` (`0x008b9650`):
    /// the width (f32, the widest line) and height (`u16`, the deepest line
    /// plus 2) of a message text, returning the text after the code that
    /// ended it.
    ///
    /// Lines are 0x10 high. Codes 1 and 2 end a line (and the text when
    /// `flags & 0xf` is at least 1), 3 adds a line (ending the text at mode
    /// 3), 0 ends it. Glyphs (group 4) add `TOGetFontWidthInterMode` widths,
    /// translated first in an Al Bhed window (`+0x10` 0x4000,
    /// `TOConvAlbhedWord`); code 5 skips a byte. Controls: `7 n` sets the
    /// line start (`n × 4 - 0x100` for 0x40..0xc0), adds `n - 0xe0` spaces
    /// (0xe1..=0xef), and `7 0x38` marks the window (`+0xe`) unless `+0x24`;
    /// `0xe 0x40`/`0xe 0x41` open and close a 5.0 wrap indent (also taken on
    /// a line break; mode 3 starts with it open); `0xb n` adds a button icon
    /// (`TOGetScaleButtonIconWidth`, scale 0.73 and 1.5 more at modes 1 and
    /// 3, else 0.65 and 1.0 more); `0x10 n` counts the window's counter
    /// (`0x50..0x60`, measured through `fcall` to itself) and marks `+0x1e`
    /// unless `flags & 0x100` or `n` is 0xff. Macros (group 3) add
    /// `TOGetChangeMacroWidth`. `0x018663e8` counts the codes and
    /// `0x018663ec` flags a type-1 window (0..=7, `TOGetWindowType`) while
    /// no battle window or event message is up. `scratch` (0x40 bytes)
    /// stands in for the original's stack.
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::many_single_char_names
    )]
    pub fn to_get_message_wh<M: Mem>(
        mem: &mut M,
        window: u32,
        text: u32,
        out_w: u32,
        out_h: u32,
        mode: u32,
        flags: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        let g = |mem: &M, a: u32| mem.rebase(a);
        let tmp = scratch;
        let albhed = scratch.wrapping_add(4);
        let sub_w = scratch.wrapping_add(0x10);
        let sub_h = scratch.wrapping_add(0x14);
        let digits = scratch.wrapping_add(0x20);
        let indent: f64 = mem.read(g(mem, 0x00b4_1d20));
        let mut max_w = 0.0f32;
        let mut cur_w = 0.0f32;
        mem.write(tmp, 0.0f32);
        let mut max_h = 0i32;
        let mut going = true;
        if window <= 7 {
            let typed = call(mem, 0x0090_6c40, &[window]) == 1
                && mem.read::<u32>(g(mem, 0x0134_0824)) == 0
                && mem.read::<u32>(g(mem, 0x0133_c948)) == 0;
            let flag = typed && mem.read::<u32>(g(mem, 0x0132_7100)) == 0;
            mem.write(g(mem, 0x0186_63ec), u32::from(flag));
        }
        let line_end = flags & 0xf;
        let high = flags & 0xff00;
        let mut wrap = (mode ^ 3).cast_signed() < 1;
        let mut line = 0x10i32;
        mem.write(g(mem, 0x0186_63e8), 0u32);
        let window_info =
            |mem: &M| -> u32 { mem.read(g(mem, 0x0186_76f0).wrapping_add(window.wrapping_mul(4))) };
        let mut p = text;
        loop {
            let c: u8 = mem.read(p);
            let mut add: Option<f64> = None;
            match call(mem, 0x008d_f610, &[u32::from(c)]) {
                1 => {
                    if wrap && cur_w > 0.0 {
                        cur_w = (f64::from(cur_w) + indent) as f32;
                    }
                    if max_w < cur_w {
                        max_w = cur_w;
                    }
                    cur_w = 0.0;
                    match c {
                        0 => {
                            if line > max_h {
                                max_h = line;
                            }
                            going = false;
                        }
                        1 | 2 => {
                            if line > max_h {
                                max_h = line;
                            }
                            line = 0x10;
                            if line_end >= 1 {
                                going = false;
                            }
                        }
                        3 => {
                            line += 0x10;
                            if line_end == 3 && line > max_h {
                                max_h = line;
                                going = false;
                            }
                        }
                        _ => {}
                    }
                }
                2 => {
                    let n: u8 = mem.read(p.wrapping_add(1));
                    if c == 7 {
                        if window.cast_signed() >= 0 && n == 0x38 {
                            let w = window_info(mem);
                            if mem.read::<u16>(w.wrapping_add(0x24)) == 0 {
                                mem.write(w.wrapping_add(0xe), 1u16);
                            }
                        }
                        if (0x40..0xc0).contains(&n) {
                            cur_w = (i32::from(n) * 4 - 0x100) as f32;
                        }
                        if (0xe1..=0xef).contains(&n) {
                            add = Some(f64::from(f32::from(n - 0xe0)));
                        }
                    } else {
                        if c == 0xe {
                            if n == 0x40 && !wrap {
                                wrap = true;
                            }
                            if n == 0x41 && wrap && cur_w > 0.0 {
                                wrap = false;
                                cur_w = (f64::from(cur_w) + indent) as f32;
                            }
                        }
                        if c == 0xb {
                            let (scale, extra) = if mode == 1 || mode == 3 {
                                (
                                    mem.read::<u32>(g(mem, 0x00b6_55e4)),
                                    mem.read::<f64>(g(mem, 0x00b4_1d00)),
                                )
                            } else {
                                (mem.read::<u32>(g(mem, 0x00b5_edf0)), 1.0)
                            };
                            let icon = fcall(mem, 0x008f_a800, &[u32::from(n), scale]);
                            cur_w = ((icon + extra) + f64::from(cur_w)) as f32;
                        }
                        if c == 0x10 && window.cast_signed() >= 0 {
                            if (0x50..0x60).contains(&n) {
                                let counter: u32 = mem.read(
                                    mem.read::<u32>(
                                        g(mem, 0x0186_8a90).wrapping_add(window.wrapping_mul(4)),
                                    )
                                    .wrapping_add(0xc),
                                );
                                call(mem, 0x008b_8290, &[counter, digits, u32::from(n) - 0x4f, 1]);
                                call(mem, 0x008b_9650, &[window, digits, sub_w, sub_h, mode, 0]);
                                let w: f32 = mem.read(sub_w);
                                cur_w = (f64::from(w) + f64::from(cur_w)) as f32;
                                let info = window_info(mem);
                                mem.write(info.wrapping_add(0x1e), 1u16);
                            }
                            if high & 0x100 == 0 && mem.read::<u8>(p.wrapping_add(1)) != 0xff {
                                let info = window_info(mem);
                                mem.write(info.wrapping_add(0x1e), 1u16);
                            }
                        }
                    }
                }
                3 => add = Some(fcall(mem, 0x008b_6a30, &[window, p, mode])),
                4 => {
                    if c == 5 {
                        p = p.wrapping_add(1);
                    } else {
                        let glyph = if window.cast_signed() >= 0
                            && mem.read::<u16>(window_info(mem).wrapping_add(0x10)) == 0x4000
                        {
                            call(mem, 0x0090_6780, &[p, albhed]);
                            albhed
                        } else {
                            p
                        };
                        call(mem, 0x008b_7be0, &[glyph, mode, tmp]);
                        add = Some(f64::from(mem.read::<f32>(tmp)));
                    }
                }
                _ => {}
            }
            if let Some(v) = add {
                cur_w = (v + f64::from(cur_w)) as f32;
            }
            p = call(mem, 0x008d_fd80, &[p]);
            let count: u32 = mem.read(g(mem, 0x0186_63e8));
            mem.write(g(mem, 0x0186_63e8), count.wrapping_add(1));
            if !going {
                break;
            }
        }
        mem.write(out_w, max_w);
        mem.write(out_h, (max_h + 2) as u16);
        mem.write(g(mem, 0x0186_63ec), 0u32);
        p
    }

    /// `TOBtlFinishCommand()` (`0x008975a0`): sends the chosen command
    /// (`TOControlMsSetMenuCommand`, `MsSetBtlCommand(0x023cc040,
    /// 0x023cc038, 0)`), saves the cursor when the cursor-save option is on
    /// and the command (`0x023cc040`) is at most 0x11, clears the command
    /// window state (`0x0133c93c`, `0x023cc08c`, nine words at
    /// `0x0133d74c`), resets the scenes (`TOSeqInitAllScene(2)`) and, when
    /// `0x0133d6e0` is 0, sets `0x023cc088` to 0xff. Returns 0xff then,
    /// else the scene reset's eax.
    pub fn to_btl_finish_command<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let g = |mem: &M, a: u32| mem.rebase(a);
        call(mem, 0x0089_afc0, &[]);
        let target: u32 = mem.read(g(mem, 0x023c_c038));
        call(mem, 0x0079_2d60, &[g(mem, 0x023c_c040), target, 0]);
        if call(mem, 0x0078_51c0, &[]) == 1 && mem.read::<u8>(g(mem, 0x023c_c040)) <= 0x11 {
            call(mem, 0x0089_89d0, &[]);
        }
        mem.write(g(mem, 0x0133_c93c), 0u8);
        mem.write(g(mem, 0x023c_c08c), 0u32);
        for k in 0..9 {
            mem.write(g(mem, 0x0133_d74c).wrapping_add(4 * k), 0u32);
        }
        let r = call(mem, 0x008a_e5a0, &[2]);
        if mem.read::<u32>(g(mem, 0x0133_d6e0)) == 0 {
            mem.write(g(mem, 0x023c_c088), 0xffu16);
            return 0xff;
        }
        r
    }

    /// `TOBtlSetCommand(level, chr, command)` (`0x00899870`): opens the
    /// sub-window of battle-window level `level` (0xf0-byte entries at
    /// `0x0133c950`) for `command`: its kind `+6` (0 for 0xfe, 0xd for
    /// 0xfffe, 0xc for 0xffff, else the command data's `+0x17`) and sets
    /// `+0xa` to 1; a real command checks the current level's command (`+0xc`):
    /// 0x311e sets `0x0133c94c` to 0x100 and marks `+0xa` 2, 0x3029 sets it
    /// to 0x10000 and `+0xa4` to 2. Then `+0x20` takes the
    /// `TOGetSaveWindow(chr +8, kind, +0x24)` text, `+0x88` the window
    /// function `0x00898e40`, and the layout and callback are set
    /// (`TOBtlSetWindowLayout`, `TOBtlSetCallback`, whose eax it returns).
    /// `scratch` stands in for the original's argument slot that
    /// `MsGetComData` writes.
    pub fn to_btl_set_command<M: Mem>(
        mem: &mut M,
        level: u32,
        chr: u32,
        command: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let _ = chr;
        let g = |mem: &M, a: u32| mem.rebase(a);
        let entry = g(mem, 0x0133_c950).wrapping_add(level.wrapping_mul(0xf0));
        mem.write(entry.wrapping_add(2), 0u8);
        let kind = match command {
            0xfe => Some(0u16),
            0xfffe => Some(0xd),
            0xffff => Some(0xc),
            _ => None,
        };
        if let Some(kind) = kind {
            mem.write(entry.wrapping_add(6), kind);
            mem.write(entry.wrapping_add(0xa), 1u16);
        } else {
            mem.write(scratch, level);
            let data = call(mem, 0x0079_a4b0, &[command, scratch]);
            let kind = u16::from(mem.read::<u8>(data.wrapping_add(0x17)));
            mem.write(entry.wrapping_add(6), kind);
            let depth = i32::from(mem.read::<i8>(g(mem, 0x023c_c092)));
            let current =
                g(mem, 0x0133_c950).wrapping_add(depth.wrapping_mul(0xf0).cast_unsigned());
            mem.write(entry.wrapping_add(0xa), 1u16);
            if mem.read::<u16>(current.wrapping_add(0xc)) == 0x311e {
                mem.write(g(mem, 0x0133_c94c), 0x100u32);
                mem.write(current.wrapping_add(0xa), 2u8);
            }
            if mem.read::<u16>(current.wrapping_add(0xc)) == 0x3029 {
                mem.write(g(mem, 0x0133_c94c), 0x0001_0000u32);
                mem.write(current.wrapping_add(0xa4), 2u16);
            }
        }
        let kind = i32::from(mem.read::<i16>(entry.wrapping_add(6))).cast_unsigned();
        let owner = i32::from(mem.read::<i16>(entry.wrapping_add(8))).cast_unsigned();
        let text = call(mem, 0x0089_b5a0, &[owner, kind, entry.wrapping_add(0x24)]);
        mem.write(entry.wrapping_add(0x20), text);
        mem.write(entry.wrapping_add(0x88), g(mem, 0x0089_8e40));
        call(mem, 0x008a_10d0, &[entry]);
        call(mem, 0x008a_0de0, &[entry])
    }

    /// `TOBtlSetNextAction(chr, command)` (`0x0089a330`): steps the battle
    /// command windows one level deeper for `command`, returning 1, or 0
    /// when `TOCheckBtlCommandUse` refuses it.
    ///
    /// At a targeting level (`+2` 1) the command is the target pick 0x3115
    /// and the command list is saved (`0x0133d0d0` to `0x0133d210`, 0x140
    /// bytes); with all targets chosen (`+0xa6` at least `+0xa4`) the
    /// command finishes (`TOBtlFinishCommand`) instead, and at the 0x10000
    /// pick mode `0x0133f106` takes `MsGetCommandMP`. A use check of 0xff
    /// finishes too. The current level is marked done (`+1` 6) with the
    /// command in its history (`+0x16 + 2 × +0xb`); the next level is
    /// cleared and given the character, command, `+0x16` 0xff, the target
    /// counts of the level before (or 1 at level 0) and `+1` 1. Checks of
    /// 0x80 or less open the sub-window (`TOBtlSetCommand`), 0xfe a target
    /// pick (`TOBtlSetTarget`); with cursor saving the saved cursor comes
    /// back (`TOBtlSetDefaultCursorPos`). A new target pick with no
    /// forced target (`0x0133d168` -1, `0x0133d0e0` 0) starts on the first
    /// dead party member (`MsGetChrStatDeath`), if any. `scratch` stands in
    /// for the original's stack word.
    #[allow(clippy::too_many_lines)]
    pub fn to_btl_set_next_action<M: Mem>(
        mem: &mut M,
        chr: u32,
        command: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let g = |mem: &M, a: u32| mem.rebase(a);
        let depth_at = g(mem, 0x023c_c092);
        let level_at = |mem: &M, depth: i8| -> u32 {
            mem.rebase(0x0133_c950)
                .wrapping_add((i32::from(depth) * 0xf0).cast_unsigned())
        };
        let depth: i8 = mem.read(depth_at);
        let level = level_at(mem, depth);
        let mut command = command;
        if mem.read::<u8>(level.wrapping_add(2)) == 1 {
            let done: i16 = mem.read(level.wrapping_add(0xa6));
            let want: i16 = mem.read(level.wrapping_add(0xa4));
            if done >= want {
                call(mem, 0x0089_75a0, &[]);
                return 1;
            }
            command = 0x3115;
            for k in 0..0x50u32 {
                let v: u32 = mem.read(g(mem, 0x0133_d0d0).wrapping_add(4 * k));
                mem.write(g(mem, 0x0133_d210).wrapping_add(4 * k), v);
            }
            if mem.read::<u32>(g(mem, 0x0133_c94c)) == 0x0001_0000 {
                let own = u32::from(mem.read::<u16>(level.wrapping_add(0xc)));
                let data = call(mem, 0x0079_a4b0, &[own, scratch]);
                let depth: i8 = mem.read(depth_at);
                let who = i32::from(mem.read::<i16>(level_at(mem, depth).wrapping_add(8)))
                    .cast_unsigned();
                let mp = call(mem, 0x0078_cf70, &[who, data]);
                #[allow(clippy::cast_possible_truncation)] // mov word, ax
                mem.write(g(mem, 0x0133_f106), mp as u16);
            }
        }
        let check = match command {
            0xfffe => 0xdi32,
            0xffff => 0xc,
            _ => {
                let r = call(mem, 0x0089_aca0, &[chr, command]).cast_signed();
                if r < 0 {
                    return 0;
                }
                if r == 0xff {
                    call(mem, 0x0089_75a0, &[]);
                    return 1;
                }
                r
            }
        };
        let depth: i8 = mem.read(depth_at);
        if depth >= 0 {
            let level = level_at(mem, depth);
            mem.write(level.wrapping_add(1), 6u8);
            let slot = i32::from(mem.read::<i8>(level.wrapping_add(0xb)));
            #[allow(clippy::cast_possible_truncation)] // mov word, cx
            mem.write(
                level
                    .wrapping_add(0x16)
                    .wrapping_add((slot * 2).cast_unsigned()),
                command as u16,
            );
        }
        let depth = depth.wrapping_add(1);
        mem.write(depth_at, depth);
        let next = level_at(mem, depth);
        for k in 0..0x3cu32 {
            mem.write(next.wrapping_add(4 * k), 0u32);
        }
        #[allow(clippy::cast_possible_truncation)] // mov word, dx / cx
        {
            mem.write(next.wrapping_add(8), chr as u16);
            mem.write(next.wrapping_add(0xc), command as u16);
        }
        mem.write(next.wrapping_add(0x16), 0xffu16);
        if depth == 0 {
            mem.write(g(mem, 0x0133_c9f4), 1u32);
        } else {
            let before = next.wrapping_sub(0xf0);
            let want: u16 = mem.read(before.wrapping_add(0xa4));
            mem.write(next.wrapping_add(0xa4), want);
            let done: u16 = mem.read(before.wrapping_add(0xa6));
            mem.write(next.wrapping_add(0xa6), done);
        }
        mem.write(next.wrapping_add(1), 1u8);
        let depth_arg = i32::from(depth).cast_unsigned();
        if check <= 0x80 {
            call(mem, 0x0089_9870, &[depth_arg, chr, command]);
        }
        if check == 0xfe {
            let depth: i8 = mem.read(depth_at);
            call(
                mem,
                0x0089_a860,
                &[i32::from(depth).cast_unsigned(), chr, command],
            );
        }
        if call(mem, 0x0078_51c0, &[]) == 1 {
            let depth: i8 = mem.read(depth_at);
            let screen = i32::from(mem.read::<i16>(g(mem, 0x023c_c088)));
            let saved: u8 = mem.read(g(mem, 0x0133_d770).wrapping_add(screen.cast_unsigned()));
            let pick: u16 = mem.read(g(mem, 0x0133_ca4c));
            let level = level_at(mem, depth);
            if saved == 1
                && pick != 0xffff
                && pick != 0xfffe
                && mem.read::<i16>(level.wrapping_add(8)) <= 0x11
            {
                call(mem, 0x0089_9ac0, &[level]);
            }
        }
        let depth: i8 = mem.read(depth_at);
        let level = level_at(mem, depth);
        if mem.read::<u8>(level.wrapping_add(2)) == 1
            && mem.read::<u32>(g(mem, 0x0133_d168)) == 0xffff_ffff
            && mem.read::<u32>(g(mem, 0x0133_d0e0)) == 0
        {
            let count: i32 = mem.read(g(mem, 0x0133_f0d8));
            let mut i = 0i32;
            while i < count {
                let who = g(mem, 0x0133_f0dc).wrapping_add((i * 2).cast_unsigned());
                let id = u32::from(mem.read::<u16>(who));
                if call(mem, 0x0079_5780, &[id]) == 1 {
                    mem.write(g(mem, 0x0133_d200), 0u16);
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    // mov word, si
                    mem.write(g(mem, 0x0133_d1fc), i as u16);
                    let target: u32 =
                        mem.read(g(mem, 0x0133_d180).wrapping_add((i * 4).cast_unsigned()));
                    mem.write(g(mem, 0x0133_d178), target);
                    let id: u16 = mem.read(who);
                    mem.write(g(mem, 0x0133_d1f4), id);
                    mem.write(g(mem, 0x0133_d1f8), g(mem, 0x0133_d180));
                    let n: u16 = mem.read(g(mem, 0x0133_d17c));
                    mem.write(g(mem, 0x0133_d1f6), n);
                    return 1;
                }
                i += 1;
            }
        }
        1
    }

    /// The battle text cursor at `0x025d09a0` the `ToMakeBtlEasy*` writers
    /// lay glyphs out with: `+0` the colour words, `+4`/`+8` the pen
    /// position (f32 bits), `+0xc..+0x18` cleared, `+0x1a` the icon mode 0,
    /// `+0x1b` the alpha. The glyph record at `0x01841d50` is cleared too.
    fn btl_easy_cursor<M: Mem>(mem: &mut M, colour: u32, x: u32, y: u32, alpha: u32) -> u32 {
        clear_glyph_record(mem);
        let c = mem.rebase(0x025d_09a0);
        for k in 0..7 {
            mem.write(c.wrapping_add(4 * k), 0u32);
        }
        mem.write(c, colour);
        mem.write(c.wrapping_add(4), x);
        mem.write(c.wrapping_add(8), y);
        #[allow(clippy::cast_possible_truncation)] // mov byte, al
        mem.write(c.wrapping_add(0x1b), alpha as u8);
        c
    }

    /// The number of decimal digits of `value` (1 for 0).
    fn digit_count(value: u32) -> u32 {
        let mut q = value.cast_signed();
        if q == 0 {
            return 1;
        }
        let mut n = 0;
        while q != 0 {
            q /= 10;
            n += 1;
        }
        n
    }

    /// `ToMakeBtlEasyDigit(digits, value, x, y, alpha, scale)`
    /// (`0x00905440`): lays out `value` (`TOValueToMesFontL` into the
    /// 32-byte `text`, the original's stack) from pen `x, y`: each glyph
    /// gets its record (`TOGetFontInfo`), its shadow and face packets
    /// (`TOMakePktBtlEasyFontShadow`, `TOMakePktBtlEasyFont`) and moves the
    /// pen (`TOAddFontWidthSpace`). Float arguments are f32 bits.
    #[allow(clippy::too_many_arguments)]
    pub fn to_make_btl_easy_digit<M: Mem>(
        mem: &mut M,
        digits: u32,
        value: u32,
        x: u32,
        y: u32,
        alpha: u32,
        scale: u32,
        text: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        call(mem, 0x008b_9140, &[value, text, digits]);
        btl_easy_digit_glyphs(mem, text, [x, y, alpha, scale], &mut call);
    }

    /// The glyph loop of the `ToMakeBtlEasyDigit*` writers: the cursor
    /// at pen `x, y`, then for each glyph of `text` its record, shadow and
    /// face packets and the pen advance.
    fn btl_easy_digit_glyphs<M: Mem>(
        mem: &mut M,
        text: u32,
        pen: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [x, y, alpha, scale] = pen;
        let cursor = btl_easy_cursor(mem, 0x8080_8080, x, y, alpha);
        let record = mem.rebase(0x0184_1d50);
        let mut p = text;
        while mem.read::<u8>(p) != 0 {
            call(mem, 0x008b_7240, &[p, record]);
            call(mem, 0x008f_bae0, &[cursor, record, scale]);
            call(mem, 0x008f_b940, &[cursor, record, scale]);
            call(mem, 0x008e_8ed0, &[cursor, record, scale]);
            p = call(mem, 0x008d_fd80, &[p]);
        }
    }

    /// `ToMakeBtlEasyDigitRightAbmap(value, x, y, alpha, scale, extra)`
    /// (`0x00905660`): `value` right-aligned on `x` (its
    /// `ToGetBtlEasyDigitWidth` taken off), written with as many digits as
    /// it has. `scratch` (0x24 bytes: the width, then the text) stands in
    /// for the original's stack.
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_make_btl_easy_digit_right_abmap<M: Mem>(
        mem: &mut M,
        args: [u32; 6],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [value, x, y, alpha, scale, extra] = args;
        let n = digit_count(value);
        let text = scratch.wrapping_add(4);
        call(mem, 0x0090_5190, &[n, value, scratch, scale, extra]);
        call(mem, 0x008b_9140, &[value, text, n]);
        let w: f32 = mem.read(scratch);
        let left = (f64::from(f32::from_bits(x)) - f64::from(w)) as f32;
        btl_easy_digit_glyphs(mem, text, [left.to_bits(), y, alpha, scale], &mut call);
    }

    /// `0x008b89d0(window, text, other)`: gives message window `window`
    /// (`0x018676f0`) its texts: `+0x5c`/`+0x60` `text`, `+0x74`/`+0x78`
    /// `text` too with the language swap on, English and `0x025d16bc` set,
    /// else `other`; `+0x90`/`+0x94` the swapped pair (with the swap and
    /// English) or the first; `+0x20` 0 and `+0x22` whether bit test
    /// `0x007851b0` or `0x00c59558` is 1. Then the no-op `0x006ef310`
    /// (with `scratch`) and the window's own function at `+0xf4`
    /// (through `cond`, with the window).
    pub fn set_message_window_texts<M: Mem>(
        mem: &mut M,
        window: u32,
        text: u32,
        other: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut cond: impl FnMut(&mut M, u32, u32) -> u32,
    ) {
        let info = |mem: &M| -> u32 {
            mem.read(mem.rebase(0x0186_76f0).wrapping_add(window.wrapping_mul(4)))
        };
        let swapped = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            call(mem, 0x008a_c390, &[]) == 1 && call(mem, 0x0078_51d0, &[]) == 1
        };
        let second = if swapped(mem, &mut call) && mem.read::<u32>(mem.rebase(0x025d_16bc)) == 1 {
            text
        } else {
            other
        };
        for (off, v) in [
            (0x5cu32, text),
            (0x60, text),
            (0x74, second),
            (0x78, second),
        ] {
            let w = info(mem);
            mem.write(w.wrapping_add(off), v);
        }
        let (first, last) = if swapped(mem, &mut call) {
            (0x74, 0x78)
        } else {
            (0x5c, 0x60)
        };
        let w = info(mem);
        let v: u32 = mem.read(w.wrapping_add(first));
        mem.write(w.wrapping_add(0x90), v);
        let w = info(mem);
        let v: u32 = mem.read(w.wrapping_add(last));
        mem.write(w.wrapping_add(0x94), v);
        let w = info(mem);
        mem.write(w.wrapping_add(0x20), 0u16);
        let flag =
            call(mem, 0x0078_51b0, &[]) == 1 || mem.read::<u32>(mem.rebase(0x00c5_9558)) == 1;
        let w = info(mem);
        mem.write(w.wrapping_add(0x22), u16::from(flag));
        call(mem, 0x006e_f310, &[scratch]);
        let w = info(mem);
        let f: u32 = mem.read(w.wrapping_add(0xf4));
        cond(mem, f, w);
    }

    /// `ToMakeBtlEasyDigit2(value, x, y, alpha, scale, extra)`
    /// (`0x00905550`): [`to_make_btl_easy_digit`] with as many digits as
    /// `value` has.
    pub fn to_make_btl_easy_digit2<M: Mem>(
        mem: &mut M,
        args: [u32; 6],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [value, x, y, alpha, scale, extra] = args;
        let n = digit_count(value);
        call(mem, 0x0090_5440, &[n, value, x, y, alpha, scale, extra]);
    }

    /// `ToMakeBtlEasyDigitRight(value, x, y, alpha, scale, extra)`
    /// (`0x009055c0`): [`to_make_btl_easy_digit2`] right-aligned on `x`:
    /// the pen starts at `x` less the number's width
    /// (`ToGetBtlEasyDigitWidth` into `width`, the original's argument
    /// slot).
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_make_btl_easy_digit_right<M: Mem>(
        mem: &mut M,
        args: [u32; 6],
        width: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [value, x, y, alpha, scale, extra] = args;
        let n = digit_count(value);
        mem.write(width, value);
        call(mem, 0x0090_5190, &[n, value, width, scale, extra]);
        let w: f32 = mem.read(width);
        let left = (f64::from(f32::from_bits(x)) - f64::from(w)) as f32;
        mem.write(width, left);
        call(
            mem,
            0x0090_5440,
            &[n, value, left.to_bits(), y, alpha, scale, extra],
        );
    }

    /// The battle digit labels `0x008f3300` and `0x008f3420` (`which` 0
    /// and 1, both called by `0x0076f850`): `ToMakeBtlEasyDigitRight(value,
    /// x + dx, y + dy, alpha, scale, 0.0)` with the integer `x`, `y` turned
    /// into `f32` first and each sum rounded to `f32` (`dx` is a double).
    /// By the language (`TOGetFFXLang`, through `0x008ac2f0`):
    /// - `0x008f3300`: dx 4.0 (`0x00b43d80`), scale 0.8 (`0x00b34254`), dy
    ///   -3.0 (`0x00b59884`), or -2.0 (`0x00b41da4`) for languages 1..=8 and
    ///   from 12.
    /// - `0x008f3420`: dx 0.0 (`0x00b0db90`), dy -5.5 (`0x00b66e38`) with
    ///   scale 0.77 (`0x00b66c18`) for language 0, -6.0 (`0x00b66e3c`) for 9,
    ///   else -5.0 (`0x00b499b8`), scale 0.8.
    ///
    /// Returns the first argument (left in `eax`).
    pub fn to_mkp_btl_digit_right<M: Mem>(
        mem: &mut M,
        which: u8,
        args: [u32; 5],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87;
        const LANG: u32 = 0x008a_c2f0;
        const DRAW: u32 = 0x0090_55c0;
        let image_f32 = |mem: &M, a: u32| -> u32 {
            x87::round_trip(f32::from_bits(mem.read(mem.rebase(a)))).to_bits()
        };
        let [ret, value, x, y, alpha] = args;
        let (dx, dy, scale) = if which == 0 {
            let mut dy = image_f32(mem, 0x00b5_9884);
            let lang = call(mem, LANG, &[]).cast_signed();
            if lang != 0 && (lang <= 8 || lang > 0xb) {
                dy = image_f32(mem, 0x00b4_1da4);
            }
            (0x00b4_3d80, dy, image_f32(mem, 0x00b3_4254))
        } else {
            let mut scale = image_f32(mem, 0x00b3_4254);
            let mut dy = image_f32(mem, 0x00b6_6e38);
            match call(mem, LANG, &[]) {
                0 => scale = image_f32(mem, 0x00b6_6c18),
                9 => dy = image_f32(mem, 0x00b6_6e3c),
                _ => dy = image_f32(mem, 0x00b4_99b8),
            }
            (0x00b0_db90, dy, scale)
        };
        #[allow(clippy::cast_precision_loss)] // fild then fstp dword: rounded
        let int_f32 = |v: u32| f64::from(v.cast_signed() as f32);
        let dx: f64 = mem.read(mem.rebase(dx));
        let x = x87::store(x87::add(int_f32(x), dx));
        let y = x87::store(x87::add(int_f32(y), x87::load(dy)));
        call(mem, DRAW, &[value, x, y, alpha, scale, 0]);
        ret
    }

    /// `ToMakeBtlEasyEdgeDigitRight(value, x, y, alpha, scale, extra)`
    /// (`0x00905890`): `value` drawn by `ToMakeBtlEasyEdgeDigit` with its
    /// right edge at `x`. The digit count is that of `value` (1 for 0,
    /// the sign not counted); the width comes from
    /// `ToGetBtlEasyDigitWidth(digits, value, &w, scale, extra)` into
    /// `scratch` (4 bytes, the original reuses its `value` slot), and the
    /// text starts at `x - w` (rounded to `f32`). The floats are passed on
    /// through `fld`/`fstp dword`. Returns what the drawer leaves in `eax`.
    pub fn to_make_btl_easy_edge_digit_right<M: Mem>(
        mem: &mut M,
        args: [u32; 6],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87;
        const WIDTH: u32 = 0x0090_5190;
        const DRAW: u32 = 0x0090_57c0;
        let [value, x, y, alpha, scale, extra] = args;
        let rt = |bits: u32| x87::round_trip(f32::from_bits(bits)).to_bits();
        let mut digits = 0u32;
        let mut v = value.cast_signed();
        if v == 0 {
            digits = 1;
        }
        while v != 0 {
            digits += 1;
            v /= 10;
        }
        call(mem, WIDTH, &[digits, value, scratch, rt(scale), rt(extra)]);
        let width: u32 = mem.read(scratch);
        let left = x87::store(x87::sub(x87::load(x), x87::load(width)));
        call(
            mem,
            DRAW,
            &[digits, value, left, rt(y), alpha, rt(scale), rt(extra)],
        )
    }

    /// `ToMakeBtlEasyEdgeDigit(digits, value, x, y, alpha, scale, extra)`
    /// (`0x009057c0`): `value` written by `TOValueToMesFontL` into the
    /// 32-byte `text` and laid out with `ToMakeBtlEasyEdgeFont`.
    pub fn to_make_btl_easy_edge_digit<M: Mem>(
        mem: &mut M,
        digits: u32,
        args: [u32; 6],
        text: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [value, x, y, alpha, scale, extra] = args;
        call(mem, 0x008b_9140, &[value, text, digits]);
        call(mem, 0x0090_5930, &[text, x, y, alpha, scale, extra]);
    }

    /// `ToMakeBtlEasyEdgeDigit2(value, x, y, alpha, scale, extra)`
    /// (`0x00905820`): [`to_make_btl_easy_edge_digit`] with as many digits
    /// as `value` has.
    pub fn to_make_btl_easy_edge_digit2<M: Mem>(
        mem: &mut M,
        args: [u32; 6],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [value, x, y, alpha, scale, extra] = args;
        let n = digit_count(value);
        call(mem, 0x0090_57c0, &[n, value, x, y, alpha, scale, extra]);
    }

    /// `ToMakeBtlEasyEdgeFont(text, x, y, alpha, scale, extra)`
    /// (`0x00905930`): `ToMakeBtlEasyEdgeFontA` with edge alpha 0x80.
    pub fn to_make_btl_easy_edge_font<M: Mem>(
        mem: &mut M,
        args: [u32; 6],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [text, x, y, alpha, scale, extra] = args;
        call(mem, 0x0090_5970, &[text, x, y, alpha, scale, extra, 0x80]);
    }

    /// `ToMakeBtlEasyEdgeFontA(text, x, y, alpha, scale, extra, edge)`
    /// (`0x00905970`): lays out an edged battle text from pen `x, y`
    /// (colour 0x80 grey with edge alpha `edge`): line codes (group 1) move
    /// the pen back to `x` and 16.0 down; glyphs (group 4) get their record,
    /// the edge and face packets (`TOMakeBtlEasyFontEdge` with the UI draw
    /// type 1 and shadow type 0, `TOMakePktBtlEasyFont` with shadow type
    /// 1, then draw type 0) and move the pen (`TOAddFontWidthSpace`).
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_make_btl_easy_edge_font_a<M: Mem>(
        mem: &mut M,
        args: [u32; 7],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [text, x, y, alpha, scale, _extra, edge] = args;
        let colour = 0x0080_8080 | (edge & 0xff) << 24;
        let cursor = btl_easy_cursor(mem, colour, x, y, alpha);
        let record = mem.rebase(0x0184_1d50);
        let down: f64 = mem.read(mem.rebase(0x00b3_13a0));
        let mut p = text;
        loop {
            let c: u8 = mem.read(p);
            if c == 0 {
                return;
            }
            match call(mem, 0x008d_f610, &[u32::from(c)]) {
                1 => {
                    mem.write(cursor.wrapping_add(4), x);
                    let pen: f32 = mem.read(cursor.wrapping_add(8));
                    mem.write(cursor.wrapping_add(8), (f64::from(pen) + down) as f32);
                }
                4 => {
                    call(mem, 0x008b_7240, &[p, record]);
                    call(mem, 0x0064_3af0, &[1]);
                    call(mem, 0x0064_0dd0, &[0]);
                    call(mem, 0x008f_ae40, &[cursor, record, scale]);
                    call(mem, 0x0064_0dd0, &[1]);
                    call(mem, 0x008f_b940, &[cursor, record, scale]);
                    call(mem, 0x0064_3af0, &[0]);
                    call(mem, 0x008e_8ed0, &[cursor, record, scale]);
                }
                _ => {}
            }
            p = call(mem, 0x008d_fd80, &[p]);
        }
    }

    /// `ToMakeBtlEasyFont(text, x, y, alpha, scale)` (`0x00905ab0`): lays
    /// out a battle text from pen `x, y` in grey 0x80: line codes move the
    /// pen back to `x` and 16.0 down; glyphs get their record, the shadow
    /// (unless `0x018716bc`, with UI draw type 1 and shadow types 0/1
    /// around it) and face packets and move the pen
    /// (`TOAddFontWidthSpace`); `0xb n` button icons (`setButtonData(n, -1)`,
    /// then `resetButtonData`) are drawn at the pen
    /// (`TODrawScaleButtonIconRGBA(n, x, y + 1.7, 0.6, grey)`, or `+ 3.2`
    /// at 0.73 for icon modes 1 and 3) unless `n` is 0x4a, and move it by
    /// the icon width (`TOGetScaleButtonIconWidth` through `fcall`) plus
    /// 1.0 (1.5).
    #[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
    pub fn to_make_btl_easy_font<M: Mem>(
        mem: &mut M,
        args: [u32; 5],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        let [text, x, y, alpha, scale] = args;
        let g = |mem: &M, a: u32| mem.rebase(a);
        let cursor = btl_easy_cursor(mem, 0x8080_8080, x, y, alpha);
        let record = g(mem, 0x0184_1d50);
        let down: f64 = mem.read(g(mem, 0x00b3_13a0));
        let mut p = text;
        loop {
            let c: u8 = mem.read(p);
            if c == 0 {
                return;
            }
            match call(mem, 0x008d_f610, &[u32::from(c)]) {
                1 => {
                    mem.write(cursor.wrapping_add(4), x);
                    let pen: f32 = mem.read(cursor.wrapping_add(8));
                    mem.write(cursor.wrapping_add(8), (f64::from(pen) + down) as f32);
                }
                2 => {
                    if c == 0xb {
                        let n = u32::from(mem.read::<u8>(p.wrapping_add(1)));
                        call(mem, 0x008b_1350, &[n, 0xffff_ffff]);
                        let n = u32::from(mem.read::<u8>(p.wrapping_add(1)));
                        if n != 0x4a {
                            let wide = matches!(mem.read::<u8>(cursor.wrapping_add(0x1a)), 1 | 3);
                            let (size, drop) = if wide {
                                (g(mem, 0x00b6_55e4), g(mem, 0x00b6_6c88))
                            } else {
                                (g(mem, 0x00b3_eb48), g(mem, 0x00b5_83a0))
                            };
                            let size: u32 = mem.read(size);
                            let drop: f64 = mem.read(drop);
                            let pen_y: f32 = mem.read(cursor.wrapping_add(8));
                            let icon_y = (f64::from(pen_y) + drop) as f32;
                            let pen_x: u32 = mem.read(cursor.wrapping_add(4));
                            call(
                                mem,
                                0x008f_8ee0,
                                &[n, pen_x, icon_y.to_bits(), size, 0x80, 0x80, 0x80, 0x80],
                            );
                            let wide = matches!(mem.read::<u8>(cursor.wrapping_add(0x1a)), 1 | 3);
                            let (size, gap) = if wide {
                                (
                                    mem.read::<u32>(g(mem, 0x00b6_55e4)),
                                    mem.read::<f64>(g(mem, 0x00b4_1d00)),
                                )
                            } else {
                                (mem.read::<u32>(g(mem, 0x00b3_eb48)), 1.0)
                            };
                            let n = u32::from(mem.read::<u8>(p.wrapping_add(1)));
                            let icon = fcall(mem, 0x008f_a800, &[n, size]);
                            let pen_x: f32 = mem.read(cursor.wrapping_add(4));
                            mem.write(
                                cursor.wrapping_add(4),
                                ((icon + gap) + f64::from(pen_x)) as f32,
                            );
                        }
                    }
                    call(mem, 0x008b_1330, &[]);
                }
                4 => {
                    call(mem, 0x008b_7240, &[p, record]);
                    if mem.read::<u32>(g(mem, 0x0187_16bc)) == 0 {
                        call(mem, 0x0064_3af0, &[1]);
                        call(mem, 0x0064_0dd0, &[0]);
                        call(mem, 0x008f_bae0, &[cursor, record, scale]);
                        call(mem, 0x0064_0dd0, &[1]);
                        call(mem, 0x008f_b940, &[cursor, record, scale]);
                        call(mem, 0x0064_3af0, &[0]);
                    } else {
                        call(mem, 0x008f_b940, &[cursor, record, scale]);
                    }
                    call(mem, 0x008e_8ed0, &[cursor, record, scale]);
                }
                _ => {}
            }
            p = call(mem, 0x008d_fd80, &[p]);
        }
    }

    /// `TOMakePktBtlEasyFont(pen, glyph, scale)` (`0x008fb940`) and, with
    /// `shadow`, `TOMakePktBtlEasyFontShadow` (`0x008fbae0`): the quad of
    /// one battle glyph at the pen of the text cursor `pen` (see
    /// [`to_make_btl_easy_font`]) from its `TOGetFontInfo` record `glyph`,
    /// handed to `graphicFontDraw` when `BeginScissor_XYUV` keeps it.
    ///
    /// The sprite record (in `scratch`, 0xb0 bytes: the image size at `+0`,
    /// the record at `+0x10`, the texture word pair at `+0xa0`) holds the
    /// corners `x, y` = pen + (0, `K_y × scale × slant`) and
    /// pen + (`width × scale × K_x`, `K_y × scale × (18 + slant)`)
    /// (`K_x`, `K_y` at `0x00c5d778`, `0x00c5d77c`), the cell's UVs over
    /// the texture size (`TOGetImageWH`), the pen colour twice and the depth
    /// 2000. The shadow is the same moved by `0x018716e0`/`0x018716e4`, its
    /// texture variant 1 and draw offsets 2.0; the face draws at layer
    /// parity + 2. With `edge` (`TOMakeBtlEasyFontEdge`, `0x008fae40`,
    /// which wins over `shadow`) the slant and height use `K_x`, the
    /// texture variant is 1, and the quad is drawn eight times at layer 0,
    /// offset by ±2 (and 0) in x and y over `graphicFontGetEdgeRatio`
    /// (through `fcall`). Record words the original leaves unset are zero
    /// here.
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::cast_possible_truncation
    )]
    pub fn to_make_pkt_btl_easy_font<M: Mem>(
        mem: &mut M,
        pen: u32,
        glyph: u32,
        scale: u32,
        shadow: bool,
        edge: bool,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        let shadow = shadow && !edge;
        let g = |mem: &M, a: u32| mem.rebase(a);
        let f = |mem: &M, a: u32| -> f64 { f64::from(mem.read::<f32>(a)) };
        for k in 0..0x2c {
            mem.write(scratch.wrapping_add(4 * k), 0u32);
        }
        let size = scratch;
        let rec = scratch.wrapping_add(0x10);
        let extra = scratch.wrapping_add(0xa0);
        let page = u32::from(mem.read::<u16>(glyph));
        let alpha = u32::from(mem.read::<u8>(pen.wrapping_add(0x1b)));
        let parity = u32::from(mem.read::<u16>(glyph.wrapping_add(4)));
        let texture = call(
            mem,
            0x008a_cf50,
            &[page, alpha, parity, u32::from(shadow || edge), extra],
        );
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let ratio = if edge {
            fcall(mem, 0x0064_0d70, &[]) as f32
        } else {
            1.0
        };
        let page = i32::from(mem.read::<i16>(glyph)).cast_unsigned();
        call(mem, 0x008a_c400, &[page, size, size.wrapping_add(4)]);
        let (w_img, h_img) = (f(mem, size), f(mem, size.wrapping_add(4)));
        let s = f64::from(f32::from_bits(scale));
        let (px, py) = (f(mem, pen.wrapping_add(4)), f(mem, pen.wrapping_add(8)));
        let slant = f(mem, glyph.wrapping_add(0x18));
        let kx = f(mem, g(mem, 0x00c5_d778));
        let ky = if edge {
            kx
        } else {
            f(mem, g(mem, 0x00c5_d77c))
        };
        let (ox, oy) = if shadow {
            (f(mem, g(mem, 0x0187_16e0)), f(mem, g(mem, 0x0187_16e4)))
        } else {
            (0.0, 0.0)
        };
        let (x0, y0, x1, y1) = if shadow {
            let x1 = (f(mem, glyph.wrapping_add(0x1c)) * s * kx + px) + ox;
            let y1 = oy + ((s * (f(mem, glyph.wrapping_add(0x20)) + slant)) * ky + py);
            (px + ox, (s * slant + py) + oy, x1, y1)
        } else {
            let x1 = f(mem, glyph.wrapping_add(0x1c)) * s * kx + px;
            let y1 = py + ky * (s * (f(mem, glyph.wrapping_add(0x20)) + slant));
            (px, py + ky * (s * slant), x1, y1)
        };
        let colour: [u8; 4] = mem.read(pen);
        mem.write(rec, x0 as f32);
        mem.write(rec.wrapping_add(4), y0 as f32);
        mem.write(
            rec.wrapping_add(8),
            (f(mem, glyph.wrapping_add(8)) / w_img) as f32,
        );
        mem.write(
            rec.wrapping_add(0xc),
            (f(mem, glyph.wrapping_add(0xc)) / h_img) as f32,
        );
        mem.write(rec.wrapping_add(0x20), x1 as f32);
        mem.write(rec.wrapping_add(0x24), y1 as f32);
        mem.write(
            rec.wrapping_add(0x28),
            (f(mem, glyph.wrapping_add(0x10)) / w_img) as f32,
        );
        mem.write(
            rec.wrapping_add(0x2c),
            (f(mem, glyph.wrapping_add(0x14)) / h_img) as f32,
        );
        for (k, c) in colour.iter().enumerate() {
            let k = k as u32 * 4;
            mem.write(rec.wrapping_add(0x10 + k), u32::from(*c));
            mem.write(rec.wrapping_add(0x30 + k), u32::from(*c));
        }
        mem.write(rec.wrapping_add(0x80), 2000u32);
        mem.write(rec.wrapping_add(0x88), 0u32);
        let kept = call(
            mem,
            0x008e_5a20,
            &[
                rec,
                rec.wrapping_add(4),
                rec.wrapping_add(0x20),
                rec.wrapping_add(0x24),
                rec.wrapping_add(8),
                rec.wrapping_add(0xc),
                rec.wrapping_add(0x28),
                rec.wrapping_add(0x2c),
            ],
        );
        if kept != 0 && edge {
            let ratio = f64::from(ratio);
            for (dx, dy) in [
                (-2.0f64, -2.0),
                (0.0, -2.0),
                (2.0, -2.0),
                (-2.0, 0.0),
                (2.0, 0.0),
                (-2.0, 2.0),
                (0.0, 2.0),
                (2.0, 2.0),
            ] {
                let (ox, oy) = ((dx / ratio) as f32, (dy / ratio) as f32);
                call(
                    mem,
                    0x0064_0660,
                    &[rec, texture, ox.to_bits(), oy.to_bits(), 0],
                );
            }
        } else if kept != 0 {
            let layer = i32::from(mem.read::<i16>(glyph.wrapping_add(4)));
            let (offset, layer) = if shadow {
                (mem.read::<u32>(g(mem, 0x00b9_2268)), layer)
            } else {
                (0.0f32.to_bits(), layer + 2)
            };
            call(
                mem,
                0x0064_0660,
                &[rec, texture, offset, offset, layer.cast_unsigned()],
            );
        }
    }

    /// `TODrawScaleButtonIconRGBA(c, x, y, scale, r, g, b, a)`
    /// (`0x008f8ee0`): draws the button icon for the text character `c`
    /// at `x, y` (f32 bits) through `DrawScaleButtonIconRGBA(id, x, y,
    /// scale, rgba, rgba)`, choosing the icon as
    /// [`to_get_scale_button_icon_width`] does: `4`..`7` sit 2.0 higher;
    /// `' '`..`'"'` draw the 10×10 sheet cell (96, 224)..(139, 267) of
    /// icon 0x258 (`TOMkpShapeXYWHUVC2`); `@`..`O` stack the four
    /// direction parts 0x258/0x259, 0x25a/0x25b, 0x25c/0x25d and
    /// 0x25e/0x25f by the low four bits; digits use `c + 0x230` (`1`/`2`
    /// as 0x262/0x261 with the confirm button swapped); `` ` `` 0x270.
    #[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
    pub fn to_draw_scale_button_icon_rgba<M: Mem>(
        mem: &mut M,
        args: [u32; 8],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [mut c, x, mut y, scale, r, g, b, a] = args;
        let rgba = (a & 0xff) << 24 | (b & 0xff) << 16 | (g & 0xff) << 8 | (r & 0xff);
        if c.wrapping_sub(0x34) <= 3 {
            let two: f64 = mem.read(mem.rebase(0x00b9_2270));
            y = ((f64::from(f32::from_bits(y)) - two) as f32).to_bits();
        }
        let image = |mem: &M, a: u32| -> u32 { mem.read(mem.rebase(a)) };
        if (0x20..=0x22).contains(&c) {
            let ten = image(mem, 0x00b9_2318);
            let shape = [
                0x258,
                x,
                y,
                ten,
                ten,
                image(mem, 0x00b4_3db8),
                image(mem, 0x00b6_6da0),
                image(mem, 0x00b6_6d8c),
                image(mem, 0x00b5_4cfc),
                rgba,
                rgba,
            ];
            call(mem, 0x0090_3ee0, &shape);
        }
        let icon = [0, x, y, scale, rgba, rgba];
        let with = |id: u32| {
            let mut v = icon;
            v[0] = id;
            v
        };
        if c.wrapping_sub(0x40) <= 0xf {
            c &= 0xf;
            call(
                mem,
                0x008e_7a90,
                &with(if c & 1 != 0 { 0x259 } else { 0x258 }),
            );
            call(
                mem,
                0x008e_7a90,
                &with(if c & 2 != 0 { 0x25b } else { 0x25a }),
            );
            call(
                mem,
                0x008e_7a90,
                &with(if c & 4 != 0 { 0x25d } else { 0x25c }),
            );
            call(
                mem,
                0x008e_7a90,
                &with(if c & 8 != 0 { 0x25f } else { 0x25e }),
            );
        }
        if c.wrapping_sub(0x30) <= 9 {
            if call(mem, 0x008a_cf30, &[]) == 1 && (0x31..=0x32).contains(&c) {
                let id = if c == 0x31 { 0x262 } else { 0x261 };
                call(mem, 0x008e_7a90, &with(id));
                return;
            }
            call(mem, 0x008e_7a90, &with(c + 0x230));
        }
        if c == 0x60 {
            call(mem, 0x008e_7a90, &with(0x270));
        }
    }

    /// `0x008b80b0(window, text)`: the number of words in a message up to
    /// its first line code (0..=2): glyphs (group 4) count one, macros
    /// (group 3) and `0x10 n` counters (0x50..0x60) their words
    /// (`TOGetChangeMacroWordCount`), `0xb` icons and `9 n` codes from
    /// 0xc0 one each.
    pub fn count_message_words<M: Mem>(
        mem: &mut M,
        window: u32,
        text: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut n = 0u32;
        let mut p = text;
        loop {
            let c: u8 = mem.read(p);
            if c <= 2 {
                return n;
            }
            match call(mem, 0x008d_f610, &[u32::from(c)]) {
                2 => {
                    let next: u8 = mem.read(p.wrapping_add(1));
                    if c == 0x10 && (0x50..0x60).contains(&next) {
                        n = n.wrapping_add(call(mem, 0x008b_6b00, &[window, p]));
                    }
                    let c: u8 = mem.read(p);
                    if c == 0xb {
                        n = n.wrapping_add(1);
                    }
                    if c == 9 && mem.read::<u8>(p.wrapping_add(1)) >= 0xc0 {
                        n = n.wrapping_add(1);
                    }
                }
                3 => n = n.wrapping_add(call(mem, 0x008b_6b00, &[window, p])),
                4 => n = n.wrapping_add(1),
                _ => {}
            }
            p = call(mem, 0x008d_fd80, &[p]);
        }
    }

    /// `TOCountMessageWordsAbove_008b8140(window, text, limit)`
    /// (`0x008b8140`): [`count_message_words`] up to the first code at or
    /// below `limit` (compared signed) instead of the first line code.
    pub fn count_message_words_above<M: Mem>(
        mem: &mut M,
        window: u32,
        text: u32,
        limit: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut n = 0u32;
        let mut p = text;
        loop {
            let c: u8 = mem.read(p);
            if i32::from(c) <= limit.cast_signed() {
                return n;
            }
            match call(mem, 0x008d_f610, &[u32::from(c)]) {
                2 => {
                    let next: u8 = mem.read(p.wrapping_add(1));
                    if c == 0x10 && (0x50..0x60).contains(&next) {
                        n = n.wrapping_add(call(mem, 0x008b_6b00, &[window, p]));
                    }
                    let c: u8 = mem.read(p);
                    if c == 0xb {
                        n = n.wrapping_add(1);
                    }
                    if c == 9 && mem.read::<u8>(p.wrapping_add(1)) >= 0xc0 {
                        n = n.wrapping_add(1);
                    }
                }
                3 => n = n.wrapping_add(call(mem, 0x008b_6b00, &[window, p])),
                4 => n = n.wrapping_add(1),
                _ => {}
            }
            p = call(mem, 0x008d_fd80, &[p]);
        }
    }

    /// `TOGetMessageSizePacked_008b7170(text)` (`0x008b7170`): the size of
    /// a message in window 0 at mode 0 (`TOGetMessageWH`) as one word, the
    /// height (`i16`) shifted up 16 and OR-ed with the width's 64-bit
    /// truncation (inline `fistp qword`, low half). `scratch` (8 bytes)
    /// stands in for the original's stack.
    pub fn message_size_packed<M: Mem>(
        mem: &mut M,
        text: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(
            mem,
            0x008b_9650,
            &[0, text, scratch, scratch.wrapping_add(4), 0, 0],
        );
        let width: f32 = mem.read(scratch);
        let height: i16 = mem.read(scratch.wrapping_add(4));
        (i32::from(height).cast_unsigned() << 16)
            | ffx_bridge::x87::ftol2(ffx_bridge::x87::load(width.to_bits()))
    }

    /// `0x008b7130(text, &w, &h)`: the size of a message in window 0 at
    /// mode 0 (`TOGetMessageWH`), the width through `ftol`. `scratch` (8
    /// bytes) stands in for the original's stack. Returns 0.
    #[allow(clippy::cast_possible_truncation)]
    pub fn message_size<M: Mem>(
        mem: &mut M,
        text: u32,
        w: u32,
        h: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(
            mem,
            0x008b_9650,
            &[0, text, scratch, scratch.wrapping_add(4), 0, 0],
        );
        let width: f32 = mem.read(scratch);
        let v = ffx_bridge::x87::ftol(mem, f64::from(width));
        mem.write(w, v);
        let height: i16 = mem.read(scratch.wrapping_add(4));
        mem.write(h, i32::from(height));
        0
    }

    /// `TOFirstInitMessageNormal(window, text, pos)` (`0x008df520`):
    /// centres a message in its window: `pos + 4` becomes the left edge
    /// plus half of what the width leaves of the window (edges `+0x48` and
    /// `+0x50`), the width from
    /// `TOGetMessageWH(+4, text, &w, &h, +0x1a, 0x103)`. `scratch` (8
    /// bytes) stands in for the original's stack. Returns `pos`.
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_first_init_message_normal<M: Mem>(
        mem: &mut M,
        window: u32,
        text: u32,
        pos: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let id = i32::from(mem.read::<i16>(window.wrapping_add(4))).cast_unsigned();
        let mode = i32::from(mem.read::<i16>(window.wrapping_add(0x1a))).cast_unsigned();
        mem.write(scratch.wrapping_add(4), window);
        call(
            mem,
            0x008b_9650,
            &[id, text, scratch, scratch.wrapping_add(4), mode, 0x103],
        );
        let f = |mem: &M, a: u32| f64::from(mem.read::<f32>(a));
        let half: f64 = mem.read(mem.rebase(0x00b9_2220));
        let left = f(mem, window.wrapping_add(0x48));
        let x = ((f(mem, window.wrapping_add(0x50)) - left) - f(mem, scratch)) * half + left;
        mem.write(pos.wrapping_add(4), x as f32);
        pos
    }

    /// `TOMkpShapeXYWHUV(shape, x, y, w, h, u0, v0, u1, v1)` (`0x00903bb0`)
    /// and, with `colours`, `TOMkpShapeXYWHUVC2(…, c0, c1)` (`0x00903ee0`):
    /// the quad of a UI shape: corners `x, y` and `x + w, y + h`, clipped
    /// with its UVs by `BeginScissor_XYUV`, then the UVs scaled for the
    /// shape's sheet and the quad handed to `graphicDrawUIElement(rec,
    /// texture, 1, 0, 0)`.
    ///
    /// Shapes -1, -3 and -6 divide the UVs by the size of image 0x3e80,
    /// 0x3e83 or 0x3e86 (`TOGetImageWH`); -2 scales u by 1/2048 and v by
    /// 1/1024 (image 0x3e82); -4 (C2 only) both by 1/256 (0x3e84); 0..=0xc6
    /// and 0xc8..=0x18e use sheet 0x3e00, 0x190..=0x256 sheet 0x3f00, and
    /// 0x258..=0x289 scale u by 1/1024 and v by 1/512 (sheet 0x3dc0)
    /// (`TOGetShapTextureName`). Other shapes leave the texture unset in
    /// the original (a stale stack word); the port passes 0. Colours are
    /// grey 0x80, or the bytes of `c0`/`c1` for the two corners. `scratch`
    /// (0xd0 bytes) stands in for the original's stack: corners at `+0`,
    /// UVs at `+0x10`, image size at `+0x20`, the record at `+0x30`.
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::many_single_char_names
    )]
    pub fn to_mkp_shape_xywhuv<M: Mem>(
        mem: &mut M,
        args: [u32; 9],
        colours: Option<[u32; 2]>,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [shape, x, y, w, h, u0, v0, u1, v1] = args;
        let f = |bits: u32| f64::from(f32::from_bits(bits));
        let rd = |mem: &M, a: u32| f64::from(mem.read::<f32>(a));
        for k in 0..0x34 {
            mem.write(scratch.wrapping_add(4 * k), 0u32);
        }
        let corner = scratch;
        let uv = scratch.wrapping_add(0x10);
        let size = scratch.wrapping_add(0x20);
        let rec = scratch.wrapping_add(0x30);
        mem.write(corner, x);
        mem.write(corner.wrapping_add(4), y);
        mem.write(corner.wrapping_add(8), (f(x) + f(w)) as f32);
        mem.write(corner.wrapping_add(0xc), (f(y) + f(h)) as f32);
        for (k, v) in [u0, v0, u1, v1].into_iter().enumerate() {
            mem.write(uv.wrapping_add(4 * k as u32), v);
        }
        let kept = call(
            mem,
            0x008e_5a20,
            &[
                corner,
                corner.wrapping_add(4),
                corner.wrapping_add(8),
                corner.wrapping_add(0xc),
                uv,
                uv.wrapping_add(4),
                uv.wrapping_add(8),
                uv.wrapping_add(0xc),
            ],
        );
        if kept == 0 {
            return;
        }
        let scale_uv = |mem: &mut M, ku: f64, kv: f64| {
            for (k, s) in [(0u32, ku), (8, ku), (4, kv), (0xc, kv)] {
                let v = rd(mem, uv.wrapping_add(k));
                mem.write(uv.wrapping_add(k), (v * s) as f32);
            }
        };
        let divide_uv = |mem: &mut M| {
            let (wi, hi) = (rd(mem, size), rd(mem, size.wrapping_add(4)));
            for (k, s) in [(0u32, wi), (8, wi), (4, hi), (0xc, hi)] {
                let v = rd(mem, uv.wrapping_add(k));
                mem.write(uv.wrapping_add(k), (v / s) as f32);
            }
        };
        let image = |mem: &M, a: u32| -> f64 { mem.read(mem.rebase(a)) };
        let mut texture = 0u32;
        let sheet = match shape.cast_signed() {
            -1 | -3 | -6 => {
                let id = match shape.cast_signed() {
                    -1 => 0x3e80,
                    -3 => 0x3e83,
                    _ => 0x3e86,
                };
                call(mem, 0x008a_c400, &[id, size, size.wrapping_add(4)]);
                divide_uv(mem);
                Some(id)
            }
            -2 => {
                let (ku, kv) = (image(mem, 0x00b5_8590), image(mem, 0x00b5_8b50));
                scale_uv(mem, ku, kv);
                Some(0x3e82)
            }
            -4 if colours.is_some() => {
                let k = image(mem, 0x00b5_0198);
                scale_uv(mem, k, k);
                Some(0x3e84)
            }
            _ => None,
        };
        if let Some(id) = sheet {
            texture = call(mem, 0x008a_c8c0, &[id]);
        } else {
            if shape <= 0xc6 {
                texture = call(mem, 0x008a_c8c0, &[0x3e00]);
            }
            if shape.wrapping_sub(0xc8) <= 0xc6 {
                texture = call(mem, 0x008a_c8c0, &[0x3e00]);
            }
            if shape.wrapping_sub(0x190) <= 0xc6 {
                texture = call(mem, 0x008a_c8c0, &[0x3f00]);
            }
            if shape.wrapping_sub(0x258) <= 0x31 {
                let (ku, kv) = (image(mem, 0x00b5_8b50), image(mem, 0x00b4_1cc0));
                scale_uv(mem, ku, kv);
                texture = call(mem, 0x008a_c8c0, &[0x3dc0]);
            }
        }
        let [c0, c1] = colours.unwrap_or([0x8080_8080, 0x8080_8080]);
        for (k, off) in [(0u32, 0u32), (4, 4), (8, 0x20), (0xc, 0x24)] {
            let v: u32 = mem.read(corner.wrapping_add(k));
            mem.write(rec.wrapping_add(off), v);
        }
        for (k, off) in [(0u32, 8u32), (4, 0xc), (8, 0x28), (0xc, 0x2c)] {
            let v: u32 = mem.read(uv.wrapping_add(k));
            mem.write(rec.wrapping_add(off), v);
        }
        for k in 0..4u32 {
            mem.write(rec.wrapping_add(0x10 + 4 * k), (c0 >> (8 * k)) & 0xff);
            mem.write(rec.wrapping_add(0x30 + 4 * k), (c1 >> (8 * k)) & 0xff);
        }
        mem.write(rec.wrapping_add(0x88), 0u32);
        call(mem, 0x0063_eee0, &[rec, texture, 1, 0, 0]);
    }

    /// The colour words of a UI record: the bytes of `c` (r, g, b, a) as
    /// four words at `at`.
    fn colour_words<M: Mem>(mem: &mut M, at: u32, c: u32) {
        for k in 0..4u32 {
            mem.write(at.wrapping_add(4 * k), (c >> (8 * k)) & 0xff);
        }
    }

    /// `TODrawCrossBoxXYWHC2(x, y, w, h, c0, c1)` (`0x008f4b20`): an
    /// untextured gradient box from colour `c0` (top left) to `c1` (bottom
    /// right): corners `x, y` and `x + w, y + h` (f32 bits), clipped by
    /// `BeginScissor_XY`, drawn with `graphicDrawUIElement(rec, 0, 1, 0, 0)`.
    /// `scratch` (0xa0 bytes) holds the corners (`+0`) and the record
    /// (`+0x10`); the UV words the original leaves unset are zero here.
    #[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
    pub fn to_draw_cross_box_xywh_c2<M: Mem>(
        mem: &mut M,
        args: [u32; 6],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [x, y, w, h, c0, c1] = args;
        let f = |b: u32| f64::from(f32::from_bits(b));
        for k in 0..0x28 {
            mem.write(scratch.wrapping_add(4 * k), 0u32);
        }
        let corner = scratch;
        let rec = scratch.wrapping_add(0x10);
        mem.write(corner, x);
        mem.write(corner.wrapping_add(4), y);
        mem.write(corner.wrapping_add(8), (f(x) + f(w)) as f32);
        mem.write(corner.wrapping_add(0xc), (f(y) + f(h)) as f32);
        let kept = call(
            mem,
            0x008e_58e0,
            &[
                corner,
                corner.wrapping_add(4),
                corner.wrapping_add(8),
                corner.wrapping_add(0xc),
            ],
        );
        if kept == 0 {
            return;
        }
        for (k, off) in [(0u32, 0u32), (4, 4), (8, 0x20), (0xc, 0x24)] {
            let v: u32 = mem.read(corner.wrapping_add(k));
            mem.write(rec.wrapping_add(off), v);
        }
        colour_words(mem, rec.wrapping_add(0x10), c0);
        colour_words(mem, rec.wrapping_add(0x30), c1);
        mem.write(rec.wrapping_add(0x88), 0u32);
        call(mem, 0x0063_eee0, &[rec, 0, 1, 0, 0]);
    }

    /// `0x008f8bb0(cell, x, y, w, h)`: draws cell `cell` of sheet 0x2ecc
    /// (columns of 170 by rows of 14 pixels, nine rows per column) over
    /// `x, y, w, h` (f32 bits) in grey, unclipped. `scratch` (0xa0 bytes)
    /// holds the sheet size (`+0`) and the record (`+0x10`).
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::many_single_char_names
    )]
    pub fn draw_sheet_cell<M: Mem>(
        mem: &mut M,
        args: [u32; 5],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [index, x, y, w, h] = args;
        let f = |b: u32| f64::from(f32::from_bits(b));
        for k in 0..0x28 {
            mem.write(scratch.wrapping_add(4 * k), 0u32);
        }
        let size = scratch;
        let rec = scratch.wrapping_add(0x10);
        let texture = call(mem, 0x008a_c8c0, &[0x2ecc]);
        call(mem, 0x008a_c400, &[0x2ecc, size, size.wrapping_add(4)]);
        let (sw, sh) = (
            f64::from(mem.read::<f32>(size)),
            f64::from(mem.read::<f32>(size.wrapping_add(4))),
        );
        let i = index.cast_signed();
        let (column, row) = ((i / 9).wrapping_mul(0xaa), (i % 9).wrapping_mul(0xe));
        let u = |p: i32| (f64::from(p as f32) / sw) as f32;
        let v = |p: i32| (f64::from(p as f32) / sh) as f32;
        mem.write(rec, x);
        mem.write(rec.wrapping_add(4), y);
        mem.write(rec.wrapping_add(8), u(column));
        mem.write(rec.wrapping_add(0xc), v(row));
        colour_words(mem, rec.wrapping_add(0x10), 0x8080_8080);
        mem.write(rec.wrapping_add(0x20), (f(x) + f(w)) as f32);
        mem.write(rec.wrapping_add(0x24), (f(y) + f(h)) as f32);
        mem.write(rec.wrapping_add(0x28), u(column.wrapping_add(0xaa)));
        mem.write(rec.wrapping_add(0x2c), v(row.wrapping_add(0xe)));
        colour_words(mem, rec.wrapping_add(0x30), 0x8080_8080);
        mem.write(rec.wrapping_add(0x88), 0u32);
        call(mem, 0x0063_eee0, &[rec, texture, 1, 0, 0]);
    }

    /// The integer box record of [`to_draw_box_xywh_c2`] and
    /// [`to_make_pkt_box_xywh_rgba`]: corners `x, y` and `x + w, y + h`
    /// (wrapping integer sums, then `fild`), the two corner colours, and
    /// `+0x88`, `+0x90`, `+0x94` cleared; drawn untextured with
    /// `graphicDrawUIElement(rec, 0, 1, 0, 0)`.
    #[allow(clippy::cast_precision_loss)]
    fn int_box<M: Mem>(
        mem: &mut M,
        rect: [u32; 4],
        colours: [[u32; 4]; 2],
        rec: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [x, y, w, h] = rect.map(u32::cast_signed);
        for k in 0..0x26 {
            mem.write(rec.wrapping_add(4 * k), 0u32);
        }
        mem.write(rec, x as f32);
        mem.write(rec.wrapping_add(4), y as f32);
        mem.write(rec.wrapping_add(0x20), x.wrapping_add(w) as f32);
        mem.write(rec.wrapping_add(0x24), y.wrapping_add(h) as f32);
        for (corner, c) in [0x10u32, 0x30].into_iter().zip(colours) {
            for (k, v) in c.into_iter().enumerate() {
                #[allow(clippy::cast_possible_truncation)] // four words
                mem.write(rec.wrapping_add(corner + 4 * k as u32), v);
            }
        }
        call(mem, 0x0063_eee0, &[rec, 0, 1, 0, 0]);
    }

    /// `TODrawBoxXYWHC2(x, y, w, h, c0, c1)` (`0x008f3db0`): an integer
    /// gradient box (see [`int_box`]) from colour `c0` to `c1`, skipped
    /// while the ability map is up (`graphicGetAbmap() == 1`). `rec` is
    /// 0x98 bytes of scratch.
    pub fn to_draw_box_xywh_c2<M: Mem>(
        mem: &mut M,
        args: [u32; 6],
        rec: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [x, y, w, h, c0, c1] = args;
        if call(mem, 0x0064_1060, &[]) == 1 {
            return;
        }
        let bytes = |c: u32| [c & 0xff, (c >> 8) & 0xff, (c >> 16) & 0xff, c >> 24];
        int_box(mem, [x, y, w, h], [bytes(c0), bytes(c1)], rec, &mut call);
    }

    /// `TOMakePktBoxXYWHRGBA(x, y, w, h, r, g, b, a)` (`0x008fb150`): an
    /// integer box (see [`int_box`]) in one colour.
    #[allow(clippy::many_single_char_names)]
    pub fn to_make_pkt_box_xywh_rgba<M: Mem>(
        mem: &mut M,
        args: [u32; 8],
        rec: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [x, y, w, h, r, g, b, a] = args;
        int_box(mem, [x, y, w, h], [[r, g, b, a]; 2], rec, &mut call);
    }

    /// `TODrawMenuBG()` (`0x008f5c10`): the menu background, shape -2
    /// (`TOMkpShapeXYWHUV`) over the box `0, 0` + (`0x00b41d9c`,
    /// `0x00b41d98`) (512 × 416) with UVs `0, 0`..(`0x00b4a1ac`,
    /// `0x00b55178`) (1920, 1024), constants read from the image.
    pub fn to_draw_menu_bg<M: Mem>(mem: &mut M, mut call: impl FnMut(&mut M, u32, &[u32]) -> u32) {
        let k = |mem: &M, a: u32| -> u32 { mem.read(mem.rebase(a)) };
        let zero = 0.0f32.to_bits();
        let (w, h) = (k(mem, 0x00b4_1d9c), k(mem, 0x00b4_1d98));
        let (u1, v1) = (k(mem, 0x00b4_a1ac), k(mem, 0x00b5_5178));
        call(
            mem,
            0x0090_3bb0,
            &[0xffff_fffe, zero, zero, w, h, zero, zero, u1, v1],
        );
    }

    /// `TOMkpCrossExtMesFontLClut(text, window, x, y, flags, scale, extra)`
    /// (`0x009016b0`): `TOMkpCrossExtMesFontLClutTypeRGBA` with type 0
    /// and grey (0x80) colour.
    pub fn to_mkp_cross_ext_mes_font_l_clut<M: Mem>(
        mem: &mut M,
        args: [u32; 7],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [text, window, x, y, flags, scale, extra] = args;
        call(
            mem,
            0x0090_1700,
            &[
                text, window, x, y, flags, 0, 0x80, 0x80, 0x80, 0x80, scale, extra,
            ],
        );
    }

    /// The pass of a 16 × 12 message glyph a `TOMakePktFont1612*` writer
    /// draws.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Font1612Pass {
        /// `TOMakePktFont1612Edge(pen, glyph)` (`0x008fc870`): the outline,
        /// the quad drawn ten times around the glyph.
        Edge,
        /// `TOMakePktFont1612Shadow(pen, glyph, layer)` (`0x008fcca0`): the
        /// drop shadow; layer 4 draws without the scissor test.
        Shadow(u32),
        /// `TOMakePktFont1612Font(pen, glyph, mode)` (`0x008fcad0`): the
        /// face; mode 5 draws at a higher layer without the scissor test.
        Face(u32),
        /// `TOMakePktFontLItalicEdge(pen, glyph)` (`0x008fce90`): the
        /// outline of an italic glyph (pen kind 3), unclipped.
        ItalicEdge,
        /// `TOMakePktFontLItalicFont(pen, glyph)` (`0x008fd0a0`): the face
        /// of an italic glyph, unclipped.
        ItalicFace,
    }

    impl Font1612Pass {
        /// Whether this pass draws the outline (ten offset copies).
        fn is_edge(self) -> bool {
            matches!(self, Self::Edge | Self::ItalicEdge)
        }

        /// Whether this pass is an italic one: fixed scale, depth 2001, no
        /// scissor, layer 0.
        fn is_italic(self) -> bool {
            matches!(self, Self::ItalicEdge | Self::ItalicFace)
        }
    }

    /// Where the edge pass draws its ten copies, in pixels before the
    /// `graphicFontGetEdgeRatio` division. The italic edge uses the same
    /// pattern from image floats: -2 at `0x00b41da4`, 2 at `0x00b92268`
    /// and -1 at `0x00b92340` (see [`font_1612_edge_offsets`]).
    const FONT_1612_EDGE_OFFSETS: [(i8, i8); 10] = [
        (-2, -2),
        (0, -2),
        (2, -2),
        (-1, 0),
        (-2, 0),
        (1, 0),
        (2, 0),
        (-2, 2),
        (0, 2),
        (2, 2),
    ];

    /// The edge offsets of `pass` as the x87 holds them: the integers of
    /// [`FONT_1612_EDGE_OFFSETS`], or for the italic edge the same pattern
    /// with its -2, 2 and -1 read from the image.
    fn font_1612_edge_offsets<M: Mem>(mem: &M, pass: Font1612Pass) -> [(f64, f64); 10] {
        if !pass.is_italic() {
            return FONT_1612_EDGE_OFFSETS.map(|(dx, dy)| (f64::from(dx), f64::from(dy)));
        }
        let image = |a: u32| f64::from(mem.read::<f32>(mem.rebase(a)));
        let (a, b, c) = (image(0x00b4_1da4), image(0x00b9_2268), image(0x00b9_2340));
        [
            (a, a),
            (0.0, a),
            (b, a),
            (c, 0.0),
            (a, 0.0),
            (1.0, 0.0),
            (b, 0.0),
            (a, b),
            (0.0, b),
            (b, b),
        ]
    }

    /// The glyph scale of a 16 × 12 text pen: `0x00c5d784` (1.0) for the
    /// edged kinds 1 and 3 (pen byte `+0x1a`) and for the italic passes,
    /// `0x00b5ee10` (0.78) otherwise.
    fn font_1612_scale<M: Mem>(mem: &M, pen: u32, pass: Font1612Pass) -> f32 {
        let kind = mem.read::<u8>(pen.wrapping_add(0x1a));
        let at = if kind == 1 || kind == 3 || pass.is_italic() {
            0x00c5_d784
        } else {
            0x00b5_ee10
        };
        mem.read(mem.rebase(at))
    }

    /// The sprite record of one 16 × 12 glyph pass (layout, no drawing):
    /// the quad from the pen (`+4` x, `+8` y, colour bytes at `+0`) and
    /// the glyph's `TOGetFontInfo` cell (u0, v0, u1, v1 at `+8..+0x14`,
    /// slant `+0x18`, width `+0x1c`, height `+0x20`), all scaled by
    /// [`font_1612_scale`], the UVs over the texture size `w`, `h`.
    ///
    /// The face and shadow stretch the width by `K_x` (`0x00c5d778`) and
    /// the height by `K_y` (`0x00c5d77c`); the shadow then moves the quad
    /// by `0x018716e0`/`0x018716e4`. The edges and the italic face use the
    /// plain cell size. Both colours are the pen's, the depth is 2000
    /// (2001 for the italic passes).
    #[allow(clippy::cast_possible_truncation)] // fstp dword
    fn font_1612_record<M: Mem>(
        mem: &mut M,
        pen: u32,
        glyph: u32,
        pass: Font1612Pass,
        (w, h): (f64, f64),
        rec: u32,
    ) {
        let f = |mem: &M, a: u32| -> f64 { f64::from(mem.read::<f32>(a)) };
        let cell = |mem: &M, off: u32| f(mem, glyph.wrapping_add(off));
        let scale = f64::from(font_1612_scale(mem, pen, pass));
        let (px, py) = (f(mem, pen.wrapping_add(4)), f(mem, pen.wrapping_add(8)));
        let (slant, width, height) = (cell(mem, 0x18), cell(mem, 0x1c), cell(mem, 0x20));
        let top = scale * slant + py;
        let (x1, y1) = if pass.is_edge() || pass.is_italic() {
            (width * scale + px, scale * (height + slant) + py)
        } else {
            let kx = f(mem, mem.rebase(0x00c5_d778));
            let ky = f(mem, mem.rebase(0x00c5_d77c));
            (width * scale * kx + px, scale * (height + slant) * ky + py)
        };
        let (x0, y0, x1, y1) = if let Font1612Pass::Shadow(_) = pass {
            let ox = f(mem, mem.rebase(0x0187_16e0));
            let oy = f(mem, mem.rebase(0x0187_16e4));
            (px + ox, top + oy, x1 + ox, y1 + oy)
        } else {
            (px, top, x1, y1)
        };
        let corners = [
            (0x00, x0),
            (0x04, y0),
            (0x08, cell(mem, 0x08) / w),
            (0x0c, cell(mem, 0x0c) / h),
            (0x20, x1),
            (0x24, y1),
            (0x28, cell(mem, 0x10) / w),
            (0x2c, cell(mem, 0x14) / h),
        ];
        for (off, v) in corners {
            mem.write(rec.wrapping_add(off), v as f32);
        }
        let colour: [u8; 4] = mem.read(pen);
        for (k, c) in (0u32..).zip(colour) {
            mem.write(rec.wrapping_add(0x10 + 4 * k), u32::from(c));
            mem.write(rec.wrapping_add(0x30 + 4 * k), u32::from(c));
        }
        mem.write(
            rec.wrapping_add(0x80),
            if pass.is_italic() { 2001u32 } else { 2000 },
        );
        mem.write(rec.wrapping_add(0x88), 0u32);
    }

    /// One pass of `TOMakePktFont1612Edge`/`Shadow`/`Font` (see
    /// [`Font1612Pass`]): the glyph's texture (`TOGetTextureName`, variant 1
    /// for edge and shadow), its record ([`font_1612_record`]) and the
    /// `graphicFontDraw` calls when `BeginScissor_XYUV` keeps the quad.
    ///
    /// The shadow draws offset by `0x00b92268` (2.0) in both axes at layer
    /// parity + `layer` (glyph word `+4`), the face at parity + 2 (+ 6 in
    /// mode 5), the edge ten times at layer 0 offset by
    /// [`FONT_1612_EDGE_OFFSETS`] over `graphicFontGetEdgeRatio` (through
    /// `fcall`). `scratch` holds 0xb0 bytes: the texture size at `+0`, the
    /// record at `+0x10`, the texture word pair at `+0xa0`; record words
    /// the original leaves unset are zero here.
    #[allow(clippy::cast_possible_truncation)] // fstp dword
    pub fn to_make_pkt_font_1612_pass<M: Mem>(
        mem: &mut M,
        pen: u32,
        glyph: u32,
        pass: Font1612Pass,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        const TEXTURE_NAME: u32 = 0x008a_cf50;
        const IMAGE_WH: u32 = 0x008a_c400;
        const SCISSOR: u32 = 0x008e_5a20;
        const FONT_DRAW: u32 = 0x0064_0660;
        const EDGE_RATIO: u32 = 0x0064_0d70;
        for k in 0..0x2c {
            mem.write(scratch.wrapping_add(4 * k), 0u32);
        }
        let size = scratch;
        let rec = scratch.wrapping_add(0x10);
        let extra = scratch.wrapping_add(0xa0);
        let page = mem.read::<u16>(glyph);
        let parity = mem.read::<u16>(glyph.wrapping_add(4));
        let alpha = mem.read::<u8>(pen.wrapping_add(0x1b));
        let variant = u32::from(!matches!(
            pass,
            Font1612Pass::Face(_) | Font1612Pass::ItalicFace
        ));
        let texture = call(
            mem,
            TEXTURE_NAME,
            &[
                u32::from(page),
                u32::from(alpha),
                u32::from(parity),
                variant,
                extra,
            ],
        );
        let ratio = if pass.is_edge() {
            f64::from(fcall(mem, EDGE_RATIO, &[]) as f32)
        } else {
            1.0
        };
        let signed_page = i32::from(page.cast_signed()).cast_unsigned();
        call(mem, IMAGE_WH, &[signed_page, size, size.wrapping_add(4)]);
        let w = f64::from(mem.read::<f32>(size));
        let h = f64::from(mem.read::<f32>(size.wrapping_add(4)));
        font_1612_record(mem, pen, glyph, pass, (w, h), rec);
        let field = |off: u32| rec.wrapping_add(off);
        let scissor = [
            field(0),
            field(4),
            field(0x20),
            field(0x24),
            field(8),
            field(0xc),
            field(0x28),
            field(0x2c),
        ];
        let unclipped =
            pass.is_italic() || matches!(pass, Font1612Pass::Shadow(4) | Font1612Pass::Face(5));
        if !unclipped && call(mem, SCISSOR, &scissor) == 0 {
            return;
        }
        let layer = i32::from(parity.cast_signed());
        match pass {
            Font1612Pass::Edge | Font1612Pass::ItalicEdge => {
                for (dx, dy) in font_1612_edge_offsets(mem, pass) {
                    let dx = (dx / ratio) as f32;
                    let dy = (dy / ratio) as f32;
                    call(
                        mem,
                        FONT_DRAW,
                        &[rec, texture, dx.to_bits(), dy.to_bits(), 0],
                    );
                }
            }
            Font1612Pass::Shadow(extra_layer) => {
                let offset: u32 = mem.read(mem.rebase(0x00b9_2268));
                let layer = layer.cast_unsigned().wrapping_add(extra_layer);
                call(mem, FONT_DRAW, &[rec, texture, offset, offset, layer]);
            }
            Font1612Pass::Face(mode) => {
                let layer = layer + if mode == 5 { 6 } else { 2 };
                call(mem, FONT_DRAW, &[rec, texture, 0, 0, layer.cast_unsigned()]);
            }
            Font1612Pass::ItalicFace => {
                call(mem, FONT_DRAW, &[rec, texture, 0, 0, 0]);
            }
        }
    }

    /// `TOMakePktFont1612(pen, glyph)` (`0x008fc7c0`): one 16 × 12 glyph
    /// by the pen's kind (byte `+0x1a`): kind 1 the edge then the face,
    /// kind 3 the italic edge and face (`TOMakePktFontLItalicEdge`/`Font`,
    /// `0x008fce90`/`0x008fd0a0`), both advancing by `0x00c5d784`;
    /// otherwise the shadow (UI draw type 1, shadow type 0) then the face
    /// (shadow type 1), draw type 0 again, advancing by `0x00b5ee10`
    /// (`TOAddFontWidthSpace`).
    pub fn to_make_pkt_font_1612<M: Mem>(
        mem: &mut M,
        pen: u32,
        glyph: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        const EDGE: u32 = 0x008f_c870;
        const FACE: u32 = 0x008f_cad0;
        const SHADOW: u32 = 0x008f_cca0;
        const SET_DRAW_TYPE: u32 = 0x0064_3af0;
        const SET_SHADOW_TYPE: u32 = 0x0064_0dd0;
        const ADD_WIDTH_SPACE: u32 = 0x008e_8ed0;
        let space = match mem.read::<u8>(pen.wrapping_add(0x1a)) {
            1 => {
                call(mem, EDGE, &[pen, glyph]);
                call(mem, FACE, &[pen, glyph, 0]);
                0x00c5_d784
            }
            3 => {
                call(mem, 0x008f_ce90, &[pen, glyph]);
                call(mem, 0x008f_d0a0, &[pen, glyph]);
                0x00c5_d784
            }
            _ => {
                call(mem, SET_DRAW_TYPE, &[1]);
                call(mem, SET_SHADOW_TYPE, &[0]);
                call(mem, SHADOW, &[pen, glyph, 0]);
                call(mem, SET_SHADOW_TYPE, &[1]);
                call(mem, FACE, &[pen, glyph, 0]);
                call(mem, SET_DRAW_TYPE, &[0]);
                0x00b5_ee10
            }
        };
        let space: u32 = mem.read(mem.rebase(space));
        call(mem, ADD_WIDTH_SPACE, &[pen, glyph, space]);
    }

    /// `TOMakePktFont1612Font_00906270(pen, glyph)` and
    /// `TOMakePktFont1612Shadow_00906290(pen, glyph)`: the face or shadow
    /// pass at its default (0).
    pub fn to_make_pkt_font_1612_default<M: Mem>(
        mem: &mut M,
        pen: u32,
        glyph: u32,
        shadow: bool,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let pass = if shadow { 0x008f_cca0 } else { 0x008f_cad0 };
        call(mem, pass, &[pen, glyph, 0]);
    }

    /// What a cross-menu icon name row names, with the constants that
    /// differ between `TODrawCrossMenuIconAbilityName` (`0x008f4f10`) and
    /// `TODrawCrossMenuIconCommandName` (`0x008f5000`).
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum CrossMenuIconName {
        /// An ability: `MsGetRomAbility`, icon byte `+0x68`, icon x
        /// `0x00b49980`, text at (`0x00b5f374`, `0x00b92318`).
        Ability,
        /// A command: `MsGetComData`, icon byte `+0x14`, icon x
        /// `0x00b5a480`, text at (`0x00b5ef30`, `0x00b92314`).
        Command,
    }

    /// `TODrawCrossMenuIconAbilityName(id, x, y, alpha)` and
    /// `TODrawCrossMenuIconCommandName(id, x, y, alpha)`: one row of a
    /// cross menu, the entry's icon and its name.
    ///
    /// The entry record (from `MsGetRomAbility`/`MsGetComData`, which
    /// also store the text base at `scratch`) gives the name (base +
    /// word `+0`) and the icon id. The icon goes to
    /// `DrawCrossMenuIconXYWHRGBA` at (x + `X2(icon x)`, y +
    /// `Y2(0x00b5f2e8)`) sized `X2(0x00b59830)` × `Y2(0x00b5f304)` in grey
    /// (0x80); the name to `ToMakeBtlEasyFont` at (x + `X2(text x)`, y +
    /// `Y2(text y)`) with `alpha`, scale `0x00b5ee10` (0.78) and 1.0. The
    /// remaps (`graphicUiRemapX2`/`Y2`, `remap(y_axis, v)`) are taken in
    /// the original's order.
    #[allow(clippy::too_many_arguments)]
    pub fn to_draw_cross_menu_icon_name<M: Mem>(
        mem: &mut M,
        kind: CrossMenuIconName,
        id: u32,
        (x, y): (u32, u32),
        alpha: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut remap: impl FnMut(&mut M, bool, f32) -> f32,
    ) {
        const DRAW_ICON: u32 = 0x008e_6af0;
        const BTL_EASY_FONT: u32 = 0x0090_5ab0;
        let (lookup, icon_at, icon_x, text_x, text_y) = match kind {
            CrossMenuIconName::Ability => {
                (0x0079_0970, 0x68, 0x00b4_9980, 0x00b5_f374, 0x00b9_2318)
            }
            CrossMenuIconName::Command => {
                (0x0079_a4b0, 0x14, 0x00b5_a480, 0x00b5_ef30, 0x00b9_2314)
            }
        };
        let constant = |mem: &M, a: u32| -> f32 { mem.read(mem.rebase(a)) };
        let entry = call(mem, lookup, &[id, scratch]);
        let name = u32::from(mem.read::<u16>(entry)).wrapping_add(mem.read(scratch));
        let icon = u32::from(mem.read::<u8>(entry.wrapping_add(icon_at)));
        let (x, y) = (f64::from(f32::from_bits(x)), f64::from(f32::from_bits(y)));
        // `fadd` onto the remapped value, then `fstp dword`
        #[allow(clippy::cast_possible_truncation)]
        let shift = |base: f64, r: f32| (f64::from(r) + base) as f32;
        let value = constant(mem, 0x00b5_f304);
        let h = remap(mem, true, value);
        let value = constant(mem, 0x00b5_9830);
        let w = remap(mem, false, value);
        let value = constant(mem, 0x00b5_f2e8);
        let icon_y = shift(y, remap(mem, true, value));
        let value = constant(mem, icon_x);
        let icon_x = shift(x, remap(mem, false, value));
        let args = [
            icon_x.to_bits(),
            icon_y.to_bits(),
            w.to_bits(),
            h.to_bits(),
            icon,
            0x80,
            0x80,
            0x80,
            0x80,
        ];
        call(mem, DRAW_ICON, &args);
        let scale: u32 = mem.read(mem.rebase(0x00b5_ee10));
        let value = constant(mem, text_y);
        let name_y = shift(y, remap(mem, true, value));
        let value = constant(mem, text_x);
        let name_x = shift(x, remap(mem, false, value));
        let args = [
            name,
            name_x.to_bits(),
            name_y.to_bits(),
            alpha,
            scale,
            1.0f32.to_bits(),
        ];
        call(mem, BTL_EASY_FONT, &args);
    }

    /// `TOMkpRotateShapeXYWHUVC2(shape, x, y, w, h, u0, v0, u1, v1, c0,
    /// c1, angle)` (`0x00902ef0`): a UI shape quad turned by `angle`
    /// (radians) about its centre, drawn as four vertices by
    /// `graphicDrawUIElement(rec, texture, 1, 4, 0)` without clipping.
    ///
    /// Shape -1 divides the UVs by the size of image 0x3e80
    /// (`TOGetImageWH`) and uses its texture; shapes 0..=0xc6 and
    /// 0xc8..=0x18e use sheet 0x3e00, 0x190..=0x256 sheet 0x3f00 and
    /// 0x258..=0x289 sheet 0x3dc0 (`TOGetShapTextureName`). Other shapes
    /// leave the texture unset in the original (a stale stack word); the
    /// port passes 0.
    ///
    /// With the half size `hw`, `hh` (by the f64 `0x00b92220`, 0.5), the
    /// centre `c` and `cos`, `sin` of the angle (`CIcos`/`CIsin`,
    /// `0x0088b3e0`/`0x0088c3a0`, through `fcall`), the corners are
    /// `c + R(∓hw, ∓hh)` in the order (-hw, -hh), (hw, -hh), (hw, hh),
    /// (-hw, hh), each through `graphicUiRemapX2`/`Y2` (`remap`). The
    /// vertices carry the UVs (u0, v1), (u1, v1), (u1, v0), (u0, v0) and
    /// the colours `c0`, `c1`, `c1`, `c0`. `scratch` (0xa0 bytes) holds the
    /// image size at `+0` and the record at `+8`.
    #[allow(clippy::cast_possible_truncation)] // fstp dword
    pub fn to_mkp_rotate_shape_xywhuv_c2<M: Mem>(
        mem: &mut M,
        args: [u32; 12],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
        mut remap: impl FnMut(&mut M, bool, f32) -> f32,
    ) {
        const IMAGE_WH: u32 = 0x008a_c400;
        const SHAPE_TEXTURE: u32 = 0x008a_c8c0;
        const DRAW_UI: u32 = 0x0063_eee0;
        const CI_COS: u32 = 0x0088_b3e0;
        const CI_SIN: u32 = 0x0088_c3a0;
        let [shape, x, y, w, h, u0, v0, u1, v1, c0, c1, angle] = args;
        let float = |bits: u32| f64::from(f32::from_bits(bits));
        let half: f64 = mem.read(mem.rebase(0x00b9_2220));
        let half_w = (float(w) * half) as f32;
        let half_h = (float(h) * half) as f32;
        let centre_x = f64::from((float(w) * half + float(x)) as f32);
        let centre_y = f64::from((float(h) * half + float(y)) as f32);
        for k in 0..0x28 {
            mem.write(scratch.wrapping_add(4 * k), 0u32);
        }
        let rec = scratch.wrapping_add(8);
        let mut uv = [u0, v0, u1, v1].map(f32::from_bits);
        let texture = if shape == u32::MAX {
            call(mem, IMAGE_WH, &[0x3e80, scratch, scratch.wrapping_add(4)]);
            let image_w = f64::from(mem.read::<f32>(scratch));
            let image_h = f64::from(mem.read::<f32>(scratch.wrapping_add(4)));
            for (k, v) in uv.iter_mut().enumerate() {
                let size = if k % 2 == 0 { image_w } else { image_h };
                *v = (f64::from(*v) / size) as f32;
            }
            call(mem, SHAPE_TEXTURE, &[0x3e80])
        } else {
            let mut texture = 0;
            for (first, last, sheet) in [
                (0, 0xc6, 0x3e00),
                (200, 0x18e, 0x3e00),
                (400, 0x256, 0x3f00),
                (600, 0x289, 0x3dc0),
            ] {
                if (first..=last).contains(&shape) {
                    texture = call(mem, SHAPE_TEXTURE, &[sheet]);
                }
            }
            texture
        };
        let [u0, v0, u1, v1] = uv;
        let cos = f64::from(fcall(mem, CI_COS, &[angle]) as f32);
        let sin = f64::from(fcall(mem, CI_SIN, &[angle]) as f32);
        let (hw, hh) = (f64::from(half_w), f64::from(half_h));
        // the original's products and sums, each kept at double precision
        let left_x = -hw * cos + centre_x;
        let left_y = sin * -hw + centre_y;
        let right_x = cos * hw + centre_x;
        let right_y = sin * hw + centre_y;
        let (sin_hh, cos_hh) = (sin * hh, cos * hh);
        let (sin_neg_hh, cos_neg_hh) = (-hh * sin, -hh * cos);
        let corners = [
            (left_x - sin_hh, left_y + cos_hh, u0, v1, c0),
            (right_x - sin_hh, right_y + cos_hh, u1, v1, c1),
            (right_x - sin_neg_hh, cos_neg_hh + right_y, u1, v0, c1),
            (left_x - sin_neg_hh, cos_neg_hh + left_y, u0, v0, c0),
        ];
        for (k, (vx, vy, u, v, colour)) in (0u32..).zip(corners) {
            let vertex = rec.wrapping_add(0x20 * k);
            let vx = remap(mem, false, vx as f32);
            let vy = remap(mem, true, vy as f32);
            for (off, value) in [(0, vx), (4, vy), (8, u), (0xc, v)] {
                mem.write(vertex.wrapping_add(off), value);
            }
            for (byte, value) in (0u32..).zip(colour.to_le_bytes()) {
                mem.write(vertex.wrapping_add(0x10 + 4 * byte), u32::from(value));
            }
        }
        call(mem, DRAW_UI, &[rec, texture, 1, 4, 0]);
    }

    /// The u edges of the battle network window's nine slices (left
    /// column, middle, right column), image floats.
    const NET_WINDOW_U: [u32; 4] = [0x00b6_6bf0, 0x00b6_6bf4, 0x00b6_6bfc, 0x00b6_6c00];
    /// The v edges of its rows (top, middle, bottom) for the plain kinds.
    const NET_WINDOW_V: [u32; 4] = [0x00b4_1cb8, 0x00b6_6c1c, 0x00b6_6c20, 0x00b6_6c24];
    /// The v edges of its rows for the pulsing kinds (a second strip).
    const NET_WINDOW_V_PULSE: [u32; 4] = [0x00b6_6c30, 0x00b6_6914, 0x00b5_ee18, 0x00b6_6c38];

    /// `TODrawBtlNetWindowXYWHType(x, y, w, h, kind)` (`0x008f41b0`): the
    /// battle network window, a nine-slice frame of shape 0x194 cells
    /// (`TOMkpShapeXYWHUV`) over the box `x, y, w, h`.
    ///
    /// The corners are `X2(8)` × `Y2(8)` (`graphicUiRemapX2`/`Y2` of
    /// `0x00b92314`, through `remap`); the edges between them span the box
    /// less `X2(16)`/`Y2(16)` (`0x00b9231c`). Right and bottom cells start
    /// at `x + w` and `y + h` (kept at double precision) less the corner
    /// size; every sum is rounded to f32 as the argument.
    ///
    /// Kinds 0 and 2 draw the eight border cells from the plain strip
    /// ([`NET_WINDOW_V`]); kinds 1 and 3 from the pulsing strip
    /// ([`NET_WINDOW_V_PULSE`]) with `TOMkpShapeXYWHUVC2` in grey (0x80)
    /// whose alpha is `trunc(cos(t × 0x00b2c5c8) × 32 + 96)` for the time
    /// `t` (`graphicGetTime`, through `fcall`; `cos` is the C runtime's
    /// `_CIcos` on the unrounded product). Kinds 0, 1 and 4 fill the centre
    /// from the plain strip. Cells go column by column (left, middle top
    /// and bottom, right), the centre last.
    #[allow(clippy::cast_possible_truncation, clippy::too_many_arguments)] // fstp dword; fistp
    pub fn to_draw_btl_net_window_xywh_type<M: Mem>(
        mem: &mut M,
        (x, y, w, h): (f32, f32, f32, f32),
        kind: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
        mut cos: impl FnMut(&mut M, f64) -> f64,
        mut remap: impl FnMut(&mut M, bool, f32) -> f32,
    ) {
        const SHAPE: u32 = 0x0090_3bb0;
        const SHAPE_C2: u32 = 0x0090_3ee0;
        const GET_TIME: u32 = 0x0064_1410;
        const CELL: u32 = 0x194;
        let image = |mem: &M, a: u32| -> f64 { f64::from(mem.read::<f32>(mem.rebase(a))) };
        let time = fcall(mem, GET_TIME, &[]);
        let wave = f64::from(cos(mem, time * mem.read::<f64>(mem.rebase(0x00b2_c5c8))) as f32);
        let depth: f64 = mem.read(mem.rebase(0x00b4_12f8));
        let middle: f64 = mem.read(mem.rebase(0x00b5_ee40));
        let alpha = ((wave * depth + middle) as i64).cast_unsigned() as u32; // fistp qword (chop), low word
        let colour = (alpha << 24).wrapping_add(0x0080_8080);
        let corner = image(mem, 0x00b9_2314) as f32;
        let inset = image(mem, 0x00b9_231c) as f32;
        let corner_w = remap(mem, false, corner);
        let corner_h = remap(mem, true, corner);
        let inset_w = remap(mem, false, inset);
        let inset_h = remap(mem, true, inset);
        let right = f64::from(x) + f64::from(w);
        let bottom = f64::from(y) + f64::from(h);
        // a column or row: its start and size, for slice 0, 1 or 2
        let span =
            |slice: usize, start: f32, size: f32, far: f64, corner: f32, inset: f32| match slice {
                0 => (start, corner),
                1 => (
                    (f64::from(corner) + f64::from(start)) as f32,
                    (f64::from(size) - f64::from(inset)) as f32,
                ),
                _ => ((far - f64::from(corner)) as f32, corner),
            };
        let pulse = kind == 1 || kind == 3;
        let border = kind <= 3;
        let centre = kind == 0 || kind == 1 || kind == 4;
        for (col, row) in [
            (0, 0),
            (0, 1),
            (0, 2),
            (1, 0),
            (1, 2),
            (2, 0),
            (2, 1),
            (2, 2),
            (1, 1),
        ] {
            let is_centre = (col, row) == (1, 1);
            if !(if is_centre { centre } else { border }) {
                continue;
            }
            let (cx, cw) = span(col, x, w, right, corner_w, inset_w);
            let (cy, ch) = span(row, y, h, bottom, corner_h, inset_h);
            let rows = if pulse && !is_centre {
                &NET_WINDOW_V_PULSE
            } else {
                &NET_WINDOW_V
            };
            let uv = [
                NET_WINDOW_U[col],
                rows[row],
                NET_WINDOW_U[col + 1],
                rows[row + 1],
            ]
            .map(|a| mem.read::<u32>(mem.rebase(a)));
            let quad = [CELL, cx.to_bits(), cy.to_bits(), cw.to_bits(), ch.to_bits()];
            if pulse && !is_centre {
                let mut args = [0u32; 11];
                args[..5].copy_from_slice(&quad);
                args[5..9].copy_from_slice(&uv);
                args[9..].copy_from_slice(&[colour, colour]);
                call(mem, SHAPE_C2, &args);
            } else {
                let mut args = [0u32; 9];
                args[..5].copy_from_slice(&quad);
                args[5..].copy_from_slice(&uv);
                call(mem, SHAPE, &args);
            }
        }
    }

    /// `AdjustButtonIconY(scale, r, g, b, a)` (`0x008e5280`): how far a
    /// button icon sits above the text line, `0x00b4838c` (-1.5) when the
    /// `TOGetFFXLang` thunk (`0x008ac2f0`) reports a non-zero language,
    /// `0x00b92338` (-0.5) otherwise; its arguments are unused.
    pub fn adjust_button_icon_y<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> f32 {
        let at = if call(mem, 0x008a_c2f0, &[]) != 0 {
            0x00b4_838c
        } else {
            0x00b9_2338
        };
        mem.read(mem.rebase(at))
    }

    /// The glyph record buffer the help-text writers fill with
    /// `TOGetFontInfo` (0x40 bytes, cleared on entry).
    const HELP_GLYPH: u32 = 0x0184_1d50;

    /// `TOAddPktChangeMacro_help(key, code, pen, scale, extra, shadow)`
    /// (`0x008e9ca0`): the packets of a macro's replacement text
    /// (`TOGetChangeMacroAddress(key, code)`) at the text cursor `pen`.
    ///
    /// Each glyph gets its record in [`HELP_GLYPH`] and is drawn by
    /// `TOMakeBtlEasyFontEdge` (or `TOMakePktBtlEasyFontShadow` when
    /// `shadow` is set) and `TOMakePktBtlEasyFont`, then the pen moves on
    /// (`TOAddFontWidthSpace`), all at `scale`. Code 7 with a byte
    /// 0xe0..=0xef moves the pen right by that byte less 0xe0; macro codes
    /// 0x12..=0x22 nest (this function again, same key). Returns the packet
    /// cursor `0x018716cc`.
    pub fn to_add_pkt_change_macro_help<M: Mem>(
        mem: &mut M,
        args: [u32; 6],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const CHANGE_MACRO_ADDRESS: u32 = 0x008b_6660;
        const SKIP_CODE: u32 = 0x008d_fd80;
        const FONT_INFO: u32 = 0x008b_7240;
        const EDGE: u32 = 0x008f_ae40;
        const SHADOW: u32 = 0x008f_bae0;
        const FACE: u32 = 0x008f_b940;
        const ADD_WIDTH_SPACE: u32 = 0x008e_8ed0;
        const ITSELF: u32 = 0x008e_9ca0;
        let [key, code, pen, scale, extra, shadow] = args;
        let glyph = mem.rebase(HELP_GLYPH);
        for k in 0..0x10 {
            mem.write(glyph.wrapping_add(4 * k), 0u32);
        }
        let mut p = call(mem, CHANGE_MACRO_ADDRESS, &[key, code]);
        while p != 0 && mem.read::<u8>(p) != 0 {
            match mem.read::<u8>(p) {
                7 => {
                    let step = mem.read::<u8>(p.wrapping_add(1));
                    if (0xe0..0xf0).contains(&step) {
                        let x = f64::from(f32::from(step - 0xe0))
                            + f64::from(mem.read::<f32>(pen.wrapping_add(4)));
                        #[allow(clippy::cast_possible_truncation)] // fstp dword
                        mem.write(pen.wrapping_add(4), x as f32);
                    }
                    p = call(mem, SKIP_CODE, &[p]);
                }
                0x12..=0x22 => {
                    call(mem, ITSELF, &[key, p, pen, scale, extra, shadow]);
                    p = call(mem, SKIP_CODE, &[p]);
                }
                _ => {
                    p = call(mem, FONT_INFO, &[p, glyph]);
                    call(
                        mem,
                        if shadow == 0 { EDGE } else { SHADOW },
                        &[pen, glyph, scale],
                    );
                    call(mem, FACE, &[pen, glyph, scale]);
                    call(mem, ADD_WIDTH_SPACE, &[pen, glyph, scale]);
                }
            }
        }
        mem.read(mem.rebase(0x0187_16cc))
    }

    /// `TOAddPktChangeMacro(key, code, pen)` (`0x008e9b80`; the Switch
    /// build's name): [`to_add_pkt_change_macro_help`] in the 16×12 font.
    ///
    /// Each glyph gets its record in [`HELP_GLYPH`]. A pen whose kind byte
    /// (`+0x1a`) is 1 draws the edge (`TOMakePktFont1612Edge`, through the
    /// thunk `0x00906260`) and the face (`TOMakePktFont1612Font_00906270`),
    /// then advances at the scale in `0x00c5d784`. Any other pen draws
    /// `TOMakePktFont1612Shadow(pen, glyph, 0)` and
    /// `TOMakePktFont1612Font(pen, glyph, 0)` and advances at `0x00b5ee10`
    /// (0.78). Code 7 and the nested macros work as in the battle-font
    /// form. Returns the packet cursor `0x018716cc`.
    pub fn to_add_pkt_change_macro<M: Mem>(
        mem: &mut M,
        args: [u32; 3],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const CHANGE_MACRO_ADDRESS: u32 = 0x008b_6660;
        const SKIP_CODE: u32 = 0x008d_fd80;
        const FONT_INFO: u32 = 0x008b_7240;
        const EDGE: u32 = 0x0090_6260;
        const FACE_DEFAULT: u32 = 0x0090_6270;
        const SHADOW: u32 = 0x008f_cca0;
        const FACE: u32 = 0x008f_cad0;
        const ADD_WIDTH_SPACE: u32 = 0x008e_8ed0;
        const ITSELF: u32 = 0x008e_9b80;
        const EDGED_SCALE: u32 = 0x00c5_d784;
        const SCALE: u32 = 0x00b5_ee10;
        let [key, code, pen] = args;
        let glyph = mem.rebase(HELP_GLYPH);
        for k in 0..0x10 {
            mem.write(glyph.wrapping_add(4 * k), 0u32);
        }
        let mut p = call(mem, CHANGE_MACRO_ADDRESS, &[key, code]);
        while p != 0 && mem.read::<u8>(p) != 0 {
            match mem.read::<u8>(p) {
                7 => {
                    let step = mem.read::<u8>(p.wrapping_add(1));
                    if (0xe0..0xf0).contains(&step) {
                        let x = f64::from(f32::from(step - 0xe0))
                            + f64::from(mem.read::<f32>(pen.wrapping_add(4)));
                        #[allow(clippy::cast_possible_truncation)] // fstp dword
                        mem.write(pen.wrapping_add(4), x as f32);
                    }
                    p = call(mem, SKIP_CODE, &[p]);
                }
                0x12..=0x22 => {
                    call(mem, ITSELF, &[key, p, pen]);
                    p = call(mem, SKIP_CODE, &[p]);
                }
                _ => {
                    p = call(mem, FONT_INFO, &[p, glyph]);
                    let scale = if mem.read::<u8>(pen.wrapping_add(0x1a)) == 1 {
                        call(mem, EDGE, &[pen, glyph]);
                        call(mem, FACE_DEFAULT, &[pen, glyph]);
                        EDGED_SCALE
                    } else {
                        call(mem, SHADOW, &[pen, glyph, 0]);
                        call(mem, FACE, &[pen, glyph, 0]);
                        SCALE
                    };
                    // fld dword then fstp dword: a signalling NaN comes out quiet
                    let scale = ffx_bridge::x87::round_trip(mem.read(mem.rebase(scale)));
                    call(mem, ADD_WIDTH_SPACE, &[pen, glyph, scale.to_bits()]);
                }
            }
        }
        mem.read(mem.rebase(0x0187_16cc))
    }

    /// `TOGetChangeMacroAdvance_008b6bb0(key, code, kind, scale, extra)`
    /// (`0x008b6bb0`): the width of a macro's replacement text
    /// (`TOGetChangeMacroAddress(key, code)`), 0 when there is none.
    ///
    /// The sum (an `f32`, rounded after each step) adds, per code by
    /// `TOGetGroup`: a nested macro's width (group 3, this function again
    /// with the same arguments); for 7 with a byte 0xe1..=0xef that byte
    /// less 0xe0; for anything else the glyph's
    /// `TOGetFontAdvance_008fa740(kind, glyph, scale)` after `TOGetFontInfo`
    /// into [`HELP_GLYPH`]. The result is the sum less
    /// `TOGetFontSpaceInPiexl(kind)`, rounded to `f32` (left in ST0).
    #[allow(clippy::cast_possible_truncation)] // fstp dword
    pub fn to_get_change_macro_advance<M: Mem>(
        mem: &mut M,
        args: [u32; 5],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> f32 {
        const CHANGE_MACRO_ADDRESS: u32 = 0x008b_6660;
        const GET_GROUP: u32 = 0x008d_f610;
        const SKIP_CODE: u32 = 0x008d_fd80;
        const FONT_INFO: u32 = 0x008b_7240;
        const ADVANCE: u32 = 0x008f_a740;
        const SPACE_PIXELS: u32 = 0x008f_a570;
        const ITSELF: u32 = 0x008b_6bb0;
        let [key, code, kind, scale, extra] = args;
        // passed on through fld/fstp dword
        let scale = ffx_bridge::x87::round_trip(f32::from_bits(scale)).to_bits();
        let extra = ffx_bridge::x87::round_trip(f32::from_bits(extra)).to_bits();
        let glyph = mem.rebase(HELP_GLYPH);
        let mut p = call(mem, CHANGE_MACRO_ADDRESS, &[key, code]);
        if p == 0 {
            return 0.0;
        }
        let mut sum = 0f32;
        while mem.read::<u8>(p) != 0 {
            let byte = mem.read::<u8>(p);
            let arg = mem.read::<u8>(p.wrapping_add(1));
            if call(mem, GET_GROUP, &[u32::from(byte)]) == 3 {
                let nested = fcall(mem, ITSELF, &[key, p, kind, scale, extra]);
                sum = (nested + f64::from(sum)) as f32;
                p = call(mem, SKIP_CODE, &[p]);
            } else if byte == 7 && (0xe1..=0xef).contains(&arg) {
                sum = (f64::from(f32::from(arg - 0xe0)) + f64::from(sum)) as f32;
                p = call(mem, SKIP_CODE, &[p]);
            } else {
                p = call(mem, FONT_INFO, &[p, glyph]);
                let advance = fcall(mem, ADVANCE, &[kind, glyph, scale]);
                sum = (advance + f64::from(sum)) as f32;
            }
        }
        let space = fcall(mem, SPACE_PIXELS, &[kind]);
        (f64::from(sum) - space) as f32
    }

    /// `TOGetMessageWH1612_008b6cb0(window, text, &w, &h, scale, extra,
    /// kind, flags)` (`0x008b6cb0`): measures a message text in the
    /// 16×12 font. Returns `h` (left in `eax`).
    ///
    /// Line widths add up per code by `TOGetGroup`, each sum rounded to
    /// `f32`:
    /// - 4 (glyph): `TOGetFontAdvance_008fa740(kind, glyph, scale)` after
    ///   `TOGetFontInfo` into [`HELP_GLYPH`]. When `window` (signed) is a
    ///   message window whose `+0x10` is 0x4000, `TOConvAlbhedWord` runs
    ///   first into a scratch word.
    /// - 3 (macro): `TOGetChangeMacroAdvance_008b6bb0(window, code, kind,
    ///   scale, extra)`.
    /// - 2 (control): 7 with a byte 0x40..0xbf sets the width to
    ///   `4 × byte - 0x100`, then 0xe1..=0xef adds its byte less 0xe0
    ///   (7 0x38 also marks `+0xe` of a window whose `+0x24` is 0). `0xe
    ///   0x40` opens a padded run when none is open, `0xe 0x41` closes it
    ///   and adds 5 to a positive width. `0xb n` adds
    ///   `TOGetButtonIconWidth(n)`. `0x10 n` (with a window) for n
    ///   0x50..0x5f measures `TOIntToMes(counter +0xc, n - 0x4f digits, 1)`
    ///   (this function again, flags 0) and adds its width; it marks the
    ///   window's `+0x1e` then, and also for any n but 0xff unless flags
    ///   bit 8 is set.
    /// - 1 (line end): a padded run (kind 3) adds 5 to a positive width;
    ///   the widest line is kept and the width cleared. Code 0 ends the
    ///   text, 1 and 2 start a new line of height 16 (and end the text when
    ///   the flags' low nibble is set), 3 adds 16 to the line height (and
    ///   ends the text when the nibble is 3 and the height grew the tallest).
    ///   The tallest line height is kept at code 0, 1, 2 (and 3, nibble 3).
    ///
    /// `w` is the widest line less `TOGetFontSpaceInPiexl(kind)`, `h` is
    /// `(tallest + 2) × scale`, both stored as `f32`. `scratch` is 0x30
    /// bytes for the counter text (0x20), the nested sizes and the Al Bhed
    /// word.
    #[allow(
        clippy::cast_possible_truncation, // fstp dword
        clippy::cast_precision_loss,      // fild then fstp dword
        clippy::too_many_lines,
        clippy::too_many_arguments
    )]
    pub fn to_get_message_wh_1612<M: Mem>(
        mem: &mut M,
        args: [u32; 8],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87;
        const GET_GROUP: u32 = 0x008d_f610;
        const SKIP_CODE: u32 = 0x008d_fd80;
        const FONT_INFO: u32 = 0x008b_7240;
        const ADVANCE: u32 = 0x008f_a740;
        const SPACE_PIXELS: u32 = 0x008f_a570;
        const MACRO_ADVANCE: u32 = 0x008b_6bb0;
        const ALBHED: u32 = 0x0090_6780;
        const ICON_WIDTH: u32 = 0x008b_65e0;
        const INT_TO_MES: u32 = 0x008b_8290;
        const ITSELF: u32 = 0x008b_6cb0;
        const COUNTERS: u32 = 0x0186_8a90;
        let [window, text, out_w, out_h, scale, extra, kind, flags] = args;
        let image_f32 = |mem: &M, a: u32| -> f32 { x87::round_trip(mem.read(mem.rebase(a))) };
        let image_f64 = |mem: &M, a: u32| -> f64 { mem.read(mem.rebase(a)) };
        let window_at = |mem: &M| -> u32 {
            let slot = MSG_WINDOWS
                .ptr(mem)
                .addr
                .wrapping_add(window.wrapping_mul(4));
            mem.read(slot)
        };
        let has_window = window.cast_signed() >= 0;
        let glyph = mem.rebase(HELP_GLYPH);
        // passed on through fld/fstp dword
        let scale_arg = x87::round_trip(f32::from_bits(scale)).to_bits();
        let extra_arg = x87::round_trip(f32::from_bits(extra)).to_bits();
        let mode = flags & 0xf;
        let (mut widest, mut tallest, mut width) = (0f32, 0f32, 0f32);
        let line = image_f32(mem, 0x00b9_231c);
        let mut height = line;
        let mut padded = (kind ^ 3).cast_signed() < 1;
        let five = image_f64(mem, 0x00b4_1d20);
        let mut going = true;
        let mut p = text;
        while going {
            let byte = mem.read::<u8>(p);
            match call(mem, GET_GROUP, &[u32::from(byte)]) {
                1 => {
                    if padded && 0.0 < width {
                        width = (f64::from(width) + five) as f32;
                    }
                    if widest < width {
                        widest = width;
                    }
                    width = 0.0;
                    match byte {
                        0 => {
                            if tallest < height {
                                tallest = height;
                            }
                            going = false;
                        }
                        1 | 2 => {
                            if tallest < height {
                                tallest = height;
                            }
                            height = image_f32(mem, 0x00b9_231c);
                            if mode != 0 {
                                going = false;
                            }
                        }
                        3 => {
                            height = (f64::from(height) + image_f64(mem, 0x00b3_13a0)) as f32;
                            if mode == 3 && tallest < height {
                                tallest = height;
                                going = false;
                            }
                        }
                        _ => {}
                    }
                }
                2 => {
                    let arg = mem.read::<u8>(p.wrapping_add(1));
                    if byte == 7 {
                        if has_window && arg == 0x38 {
                            let w = window_at(mem);
                            if mem.read::<u16>(w.wrapping_add(0x24)) == 0 {
                                mem.write(w.wrapping_add(0xe), 1u16);
                            }
                        }
                        if (0x40..0xc0).contains(&arg) {
                            width = (i32::from(arg) * 4 - 0x100) as f32;
                        }
                        if (0xe1..=0xef).contains(&arg) {
                            width = (f64::from(f32::from(arg - 0xe0)) + f64::from(width)) as f32;
                        }
                    } else {
                        if byte == 0xe {
                            if arg == 0x40 && !padded {
                                padded = true;
                            }
                            if arg == 0x41 && padded && 0.0 < width {
                                width = (f64::from(width) + five) as f32;
                                padded = false;
                            }
                        }
                        if byte == 0xb {
                            let icon = call(mem, ICON_WIDTH, &[u32::from(arg)]);
                            let icon = icon.cast_signed() as f32;
                            width = (f64::from(icon) + f64::from(width)) as f32;
                        }
                        if byte == 0x10 && has_window {
                            if (0x50..0x60).contains(&arg) {
                                let counter: u32 = mem.read(
                                    mem.rebase(COUNTERS).wrapping_add(window.wrapping_mul(4)),
                                );
                                let value = mem.read::<u32>(counter.wrapping_add(0xc));
                                call(mem, INT_TO_MES, &[value, scratch, u32::from(arg - 0x4f), 1]);
                                let (w, h) =
                                    (scratch.wrapping_add(0x20), scratch.wrapping_add(0x24));
                                call(
                                    mem,
                                    ITSELF,
                                    &[window, scratch, w, h, scale_arg, extra_arg, kind, 0],
                                );
                                let nested = mem.read::<f32>(w);
                                width = (f64::from(nested) + f64::from(width)) as f32;
                                mem.write(window_at(mem).wrapping_add(0x1e), 1u16);
                            }
                            if flags & 0x100 == 0 && arg != 0xff {
                                mem.write(window_at(mem).wrapping_add(0x1e), 1u16);
                            }
                        }
                    }
                }
                3 => {
                    let nested =
                        fcall(mem, MACRO_ADVANCE, &[window, p, kind, scale_arg, extra_arg]);
                    width = (nested + f64::from(width)) as f32;
                }
                4 => {
                    if has_window && mem.read::<u16>(window_at(mem).wrapping_add(0x10)) == 0x4000 {
                        call(mem, ALBHED, &[p, scratch.wrapping_add(0x28)]);
                    }
                    call(mem, FONT_INFO, &[p, glyph]);
                    let advance = fcall(mem, ADVANCE, &[kind, glyph, scale_arg]);
                    width = (advance + f64::from(width)) as f32;
                }
                _ => {}
            }
            p = call(mem, SKIP_CODE, &[p]);
        }
        let space = fcall(mem, SPACE_PIXELS, &[kind]);
        mem.write(out_w, x87::store(x87::sub(f64::from(widest), space)));
        let minus_two = image_f64(mem, 0x00b2_c5d0);
        let h = x87::mul(x87::sub(f64::from(tallest), minus_two), x87::load(scale));
        mem.write(out_h, x87::store(h));
        out_h
    }

    /// The battle-font text cursor the cross-menu writers lay out with
    /// (see [`to_make_btl_easy_font`]).
    const CROSS_PEN: u32 = 0x025d_09a0;

    /// `TOMkpCrossExtMesFontLClutTypeRGBA(key, text, x, y, mode, kind, r,
    /// g, b, a, scale, extra)` (`0x00901700`): a cross-menu text in the
    /// battle font at `x, y`, with the pen ([`CROSS_PEN`]) reset to colour
    /// `r, g, b, a`, kind byte `kind` (`+0x1a`) and mode byte `mode`
    /// (`+0x1b`).
    ///
    /// By `TOGetGroup` of each code:
    /// - 1 (new line): x back to the start; y down by `0x00b66c98` (13.6)
    ///   when `scale` equals `0x00b5edec` (0.64), else by `scale × 16`.
    /// - 2 (control): 7 with a byte 0xe1..=0xef moves the pen right by
    ///   that byte less 0xe0. `0xa n` sets the mode's low nibble to
    ///   `(n & 0xf) - 1` (when non-zero) and its high nibble to
    ///   `(n & 0xf0) - 0x40` (from 0x40). `0xb n` draws button icon `n`
    ///   (`setButtonData(n, -1)`, `TODrawScaleButtonIconRGBA` at scale
    ///   `0x00b66864` (0.63), at y, or at `y + 3.0 + AdjustButtonIconY`
    ///   unless `scale` is 0.64) and moves the pen by the icon width plus
    ///   `TOGetFontSpaceInPiexl(0)`. Other than 7 they end with
    ///   `resetButtonData`.
    /// - 3 (macro): `TOAddPktChangeMacro_help(key, …, scale, extra, 0)`
    ///   unless `key` is -1.
    /// - 4 (glyph): its record ([`HELP_GLYPH`]), the edge (unless
    ///   `0x018716bc` is set), the face and the advance at `scale`.
    #[allow(
        clippy::cast_possible_truncation, // fstp dword
        clippy::too_many_lines,
        clippy::many_single_char_names, // the colour and the position
        clippy::float_cmp // the original's exact `fucomp`
    )]
    pub fn to_mkp_cross_ext_mes_font_l_clut_type_rgba<M: Mem>(
        mem: &mut M,
        args: [u32; 12],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        const GET_GROUP: u32 = 0x008d_f610;
        const SKIP_CODE: u32 = 0x008d_fd80;
        const FONT_INFO: u32 = 0x008b_7240;
        const EDGE: u32 = 0x008f_ae40;
        const FACE: u32 = 0x008f_b940;
        const ADD_WIDTH_SPACE: u32 = 0x008e_8ed0;
        const MACRO_HELP: u32 = 0x008e_9ca0;
        const SET_BUTTON: u32 = 0x008b_1350;
        const RESET_BUTTON: u32 = 0x008b_1330;
        const ADJUST_ICON_Y: u32 = 0x008e_5280;
        const DRAW_ICON: u32 = 0x008f_8ee0;
        const ICON_WIDTH: u32 = 0x008f_a800;
        const SPACE_PIXELS: u32 = 0x008f_a570;
        let [key, text, x, y, mode, kind, r, g, b, a, scale, extra] = args;
        let glyph = mem.rebase(HELP_GLYPH);
        let pen = mem.rebase(CROSS_PEN);
        for k in 0..0x10 {
            mem.write(glyph.wrapping_add(4 * k), 0u32);
        }
        for k in 0..7 {
            mem.write(pen.wrapping_add(4 * k), 0u32);
        }
        mem.write(pen, [r, g, b, a].map(|c| c.to_le_bytes()[0]));
        mem.write(pen.wrapping_add(4), f32::from_bits(x));
        mem.write(pen.wrapping_add(8), f32::from_bits(y));
        mem.write(
            pen.wrapping_add(0x1a),
            [kind.to_le_bytes()[0], mode.to_le_bytes()[0]],
        );
        let image = |mem: &M, a: u32| -> u32 { mem.read(mem.rebase(a)) };
        let pen_x = |mem: &M| f64::from(mem.read::<f32>(pen.wrapping_add(4)));
        let pen_y = |mem: &M| f64::from(mem.read::<f32>(pen.wrapping_add(8)));
        let scale_f = f32::from_bits(scale);
        let small = f32::from_bits(image(mem, 0x00b5_edec));
        let mut p = text;
        while mem.read::<u8>(p) != 0 {
            match call(mem, GET_GROUP, &[u32::from(mem.read::<u8>(p))]) {
                1 => {
                    mem.write(pen.wrapping_add(4), f32::from_bits(x));
                    let y = if small == scale_f {
                        pen_y(mem) + mem.read::<f64>(mem.rebase(0x00b6_6c98))
                    } else {
                        f64::from(scale_f) * mem.read::<f64>(mem.rebase(0x00b3_13a0)) + pen_y(mem)
                    };
                    mem.write(pen.wrapping_add(8), y as f32);
                }
                2 => {
                    let code = mem.read::<u8>(p);
                    let arg = mem.read::<u8>(p.wrapping_add(1));
                    if code == 7 {
                        if arg > 0xe0 && arg < 0xf0 {
                            let x = f64::from(f32::from(arg - 0xe0)) + pen_x(mem);
                            mem.write(pen.wrapping_add(4), x as f32);
                        }
                    } else {
                        if code == 0xa {
                            let at = pen.wrapping_add(0x1b);
                            let mut m = mem.read::<u8>(at);
                            if arg & 0xf != 0 {
                                m = (m & 0xf0) | ((arg & 0xf) - 1);
                                mem.write(at, m);
                            }
                            if arg & 0xf0 >= 0x40 {
                                m = (m & 0xf) | ((arg & 0xf0) - 0x40);
                                mem.write(at, m);
                            }
                        }
                        if code == 0xb {
                            let icon = u32::from(arg);
                            call(mem, SET_BUTTON, &[icon, u32::MAX]);
                            let icon_scale = image(mem, 0x00b6_6864);
                            let small = mem.read::<f64>(mem.rebase(0x00b6_6c68));
                            let icon_y = if small == f64::from(scale_f) {
                                mem.read::<f32>(pen.wrapping_add(8))
                            } else {
                                let lift = fcall(
                                    mem,
                                    ADJUST_ICON_Y,
                                    &[icon_scale, 0x80, 0x80, 0x80, 0x80],
                                );
                                let three = mem.read::<f64>(mem.rebase(0x00b2_c5c8));
                                ((pen_y(mem) + three) + lift) as f32
                            };
                            let pen_x_bits = mem.read::<u32>(pen.wrapping_add(4));
                            let args = [
                                icon,
                                pen_x_bits,
                                icon_y.to_bits(),
                                icon_scale,
                                0x80,
                                0x80,
                                0x80,
                                0x80,
                            ];
                            call(mem, DRAW_ICON, &args);
                            let width = fcall(mem, ICON_WIDTH, &[icon, icon_scale]);
                            let space = fcall(mem, SPACE_PIXELS, &[0]);
                            mem.write(pen.wrapping_add(4), ((space + width) + pen_x(mem)) as f32);
                        }
                        call(mem, RESET_BUTTON, &[]);
                    }
                }
                3 if key != u32::MAX => {
                    call(mem, MACRO_HELP, &[key, p, pen, scale, extra, 0]);
                }
                4 => {
                    call(mem, FONT_INFO, &[p, glyph]);
                    if image(mem, 0x0187_16bc) == 0 {
                        call(mem, EDGE, &[pen, glyph, scale]);
                    }
                    call(mem, FACE, &[pen, glyph, scale]);
                    call(mem, ADD_WIDTH_SPACE, &[pen, glyph, scale]);
                }
                _ => {}
            }
            p = call(mem, SKIP_CODE, &[p]);
        }
    }

    /// `TOAddPktBaseMesFontLAbmap(cursor, key, text, x, y, mode, kind, r,
    /// g, b, a, shadow)` (`0x008e8fb0`): a text in the 16×12 font at the
    /// integer position `x, y`, laid out with the pen [`CROSS_PEN`] (reset
    /// to colour `r, g, b, a`, kind byte `kind` at `+0x1a`, mode byte
    /// `mode` at `+0x1b`); the packet cursor `0x018716cc` starts at
    /// `cursor`. Returns the packet cursor.
    ///
    /// By `TOGetGroup` of each code:
    /// - 1 (new line): x back to the start, y down by `0x00b313a0` (16.0).
    /// - 2 (control): 7 with a byte 0xe1..=0xef moves the pen right by
    ///   that byte less 0xe0. `0xa` sets the mode nibbles as in
    ///   [`to_mkp_cross_ext_mes_font_l_clut_type_rgba`]. `0xb n` draws
    ///   button icon `n` at scale `0x00b5edf0` (0.65), at
    ///   `y + 3.0 + AdjustButtonIconY`, and moves the pen by the icon width
    ///   plus 1. Other than 7 they end with `resetButtonData`.
    /// - 3 (macro): [`to_add_pkt_change_macro`] unless `key` is -1.
    /// - 4 (glyph): its record ([`HELP_GLYPH`]); unless the kind is 1,
    ///   `TOMakePktFont1612Shadow` and `TOMakePktFont1612Font` (passes 4
    ///   and 5 when `shadow` is positive, else 0 and 0) and the advance at
    ///   `0x00b5ee10` (0.78).
    ///
    /// The original first asks `graphicFontGetScreenWH` for the screen
    /// size, which only stores constants into locals it never reads; the
    /// port leaves that call out.
    #[allow(
        clippy::cast_possible_truncation, // fstp dword
        clippy::cast_precision_loss,      // fild then fstp dword: rounded
        clippy::too_many_lines,
        clippy::many_single_char_names // the colour and the position
    )]
    pub fn to_add_pkt_base_mes_font_l_abmap<M: Mem>(
        mem: &mut M,
        args: [u32; 12],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        const GET_GROUP: u32 = 0x008d_f610;
        const SKIP_CODE: u32 = 0x008d_fd80;
        const FONT_INFO: u32 = 0x008b_7240;
        const SHADOW: u32 = 0x008f_cca0;
        const FACE: u32 = 0x008f_cad0;
        const ADD_WIDTH_SPACE: u32 = 0x008e_8ed0;
        const CHANGE_MACRO: u32 = 0x008e_9b80;
        const SET_BUTTON: u32 = 0x008b_1350;
        const RESET_BUTTON: u32 = 0x008b_1330;
        const ADJUST_ICON_Y: u32 = 0x008e_5280;
        const DRAW_ICON: u32 = 0x008f_8ee0;
        const ICON_WIDTH: u32 = 0x008f_a800;
        const CURSOR: u32 = 0x0187_16cc;
        let [cursor, key, text, x, y, mode, kind, r, g, b, a, shadow] = args;
        let glyph = mem.rebase(HELP_GLYPH);
        let pen = mem.rebase(CROSS_PEN);
        for k in 0..0x10 {
            mem.write(glyph.wrapping_add(4 * k), 0u32);
        }
        for k in 0..7 {
            mem.write(pen.wrapping_add(4 * k), 0u32);
        }
        mem.write(mem.rebase(CURSOR), cursor);
        let start_x = x.cast_signed() as f32;
        mem.write(pen, [r, g, b, a].map(|c| c.to_le_bytes()[0]));
        mem.write(pen.wrapping_add(4), start_x);
        mem.write(pen.wrapping_add(8), y.cast_signed() as f32);
        mem.write(
            pen.wrapping_add(0x1a),
            [kind.to_le_bytes()[0], mode.to_le_bytes()[0]],
        );
        let image_f32 =
            |mem: &M, a: u32| -> f32 { ffx_bridge::x87::round_trip(mem.read(mem.rebase(a))) };
        let image_f64 = |mem: &M, a: u32| -> f64 { mem.read(mem.rebase(a)) };
        let pen_x = |mem: &M| f64::from(mem.read::<f32>(pen.wrapping_add(4)));
        let pen_y = |mem: &M| f64::from(mem.read::<f32>(pen.wrapping_add(8)));
        let mut p = text;
        while mem.read::<u8>(p) != 0 {
            match call(mem, GET_GROUP, &[u32::from(mem.read::<u8>(p))]) {
                1 => {
                    mem.write(pen.wrapping_add(4), start_x);
                    let y = pen_y(mem) + image_f64(mem, 0x00b3_13a0);
                    mem.write(pen.wrapping_add(8), y as f32);
                }
                2 => {
                    let code = mem.read::<u8>(p);
                    let arg = mem.read::<u8>(p.wrapping_add(1));
                    if code == 7 {
                        if arg > 0xe0 && arg < 0xf0 {
                            let x = f64::from(f32::from(arg - 0xe0)) + pen_x(mem);
                            mem.write(pen.wrapping_add(4), x as f32);
                        }
                    } else {
                        if code == 0xa {
                            let at = pen.wrapping_add(0x1b);
                            let mut m = mem.read::<u8>(at);
                            if arg & 0xf != 0 {
                                m = (m & 0xf0) | ((arg & 0xf) - 1);
                                mem.write(at, m);
                            }
                            if arg & 0xf0 >= 0x40 {
                                m = (m & 0xf) | ((arg & 0xf0) - 0x40);
                                mem.write(at, m);
                            }
                        }
                        if code == 0xb {
                            let icon = u32::from(arg);
                            call(mem, SET_BUTTON, &[icon, u32::MAX]);
                            let icon_scale = image_f32(mem, 0x00b5_edf0).to_bits();
                            let lift =
                                fcall(mem, ADJUST_ICON_Y, &[icon_scale, 0x80, 0x80, 0x80, 0x80]);
                            let three = image_f64(mem, 0x00b2_c5c8);
                            let icon_y = ((pen_y(mem) + three) + lift) as f32;
                            let icon_x = ffx_bridge::x87::round_trip(mem.read(pen.wrapping_add(4)));
                            let args = [
                                icon,
                                icon_x.to_bits(),
                                icon_y.to_bits(),
                                icon_scale,
                                0x80,
                                0x80,
                                0x80,
                                0x80,
                            ];
                            call(mem, DRAW_ICON, &args);
                            let icon_scale = image_f32(mem, 0x00b5_edf0).to_bits();
                            let width = fcall(mem, ICON_WIDTH, &[icon, icon_scale]);
                            mem.write(pen.wrapping_add(4), ((width + 1.0) + pen_x(mem)) as f32);
                        }
                        call(mem, RESET_BUTTON, &[]);
                    }
                }
                3 if key != u32::MAX => {
                    call(mem, CHANGE_MACRO, &[key, p, pen]);
                }
                4 => {
                    call(mem, FONT_INFO, &[p, glyph]);
                    if mem.read::<u8>(pen.wrapping_add(0x1a)) != 1 {
                        let (edge, face) = if shadow.cast_signed() > 0 {
                            (4, 5)
                        } else {
                            (0, 0)
                        };
                        call(mem, SHADOW, &[pen, glyph, edge]);
                        call(mem, FACE, &[pen, glyph, face]);
                        let scale = image_f32(mem, 0x00b5_ee10).to_bits();
                        call(mem, ADD_WIDTH_SPACE, &[pen, glyph, scale]);
                    }
                }
                _ => {}
            }
            p = call(mem, SKIP_CODE, &[p]);
        }
        mem.read(mem.rebase(CURSOR))
    }

    /// The UV rectangle of cross-menu icon `icon` (layout): the icons sit
    /// 17 to a row on sheet 0x3dc0, each 15 × 18 texels, over the sheet
    /// size `w`, `h`.
    #[allow(clippy::cast_possible_truncation)] // fstp dword
    fn cross_menu_icon_uv(icon: u8, (w, h): (f64, f64)) -> [f32; 4] {
        let (col, row) = (u16::from(icon % 17), u16::from(icon / 17));
        let texel = |v: u16, size: f64| (f64::from(v) / size) as f32;
        [
            texel(col * 15, w),
            texel(row * 18, h),
            texel(col * 15 + 15, w),
            texel(row * 18 + 18, h),
        ]
    }

    /// `DrawCrossMenuIconXYWHRGBA(x, y, w, h, icon, r, g, b, a)`
    /// (`0x008e6af0`): one cross-menu icon (see [`cross_menu_icon_uv`]) in
    /// the box `x, y, w, h` with the colour bytes `r, g, b, a` on both
    /// corners and depth 500, handed to `graphicDrawUIElement(rec, texture,
    /// 1, 0, 0)` when `BeginScissor_XYUV` keeps it. `scratch` holds 0xa0
    /// bytes: the sheet size at `+0`, the record at `+0x10`; record words
    /// the original leaves unset are zero here.
    #[allow(clippy::cast_possible_truncation)] // fstp dword
    pub fn draw_cross_menu_icon_xywh_rgba<M: Mem>(
        mem: &mut M,
        args: [u32; 9],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        const SHEET: u32 = 0x3dc0;
        const SHAPE_TEXTURE: u32 = 0x008a_c8c0;
        const IMAGE_WH: u32 = 0x008a_c400;
        const SCISSOR: u32 = 0x008e_5a20;
        const DRAW_UI: u32 = 0x0063_eee0;
        let [x, y, w, h, icon, colour @ ..] = args;
        for k in 0..0x28 {
            mem.write(scratch.wrapping_add(4 * k), 0u32);
        }
        let rec = scratch.wrapping_add(0x10);
        let texture = call(mem, SHAPE_TEXTURE, &[SHEET]);
        call(mem, IMAGE_WH, &[SHEET, scratch, scratch.wrapping_add(4)]);
        let sheet_w = f64::from(mem.read::<f32>(scratch));
        let sheet_h = f64::from(mem.read::<f32>(scratch.wrapping_add(4)));
        let [u0, v0, u1, v1] = cross_menu_icon_uv(icon.to_le_bytes()[0], (sheet_w, sheet_h));
        let float = |bits: u32| f64::from(f32::from_bits(bits));
        let x1 = (float(x) + float(w)) as f32;
        let y1 = (float(y) + float(h)) as f32;
        let fields = [
            (0x00, f32::from_bits(x)),
            (0x04, f32::from_bits(y)),
            (0x08, u0),
            (0x0c, v0),
            (0x20, x1),
            (0x24, y1),
            (0x28, u1),
            (0x2c, v1),
        ];
        for (off, v) in fields {
            mem.write(rec.wrapping_add(off), v);
        }
        for (k, c) in (0u32..).zip(colour) {
            let byte = u32::from(c.to_le_bytes()[0]);
            mem.write(rec.wrapping_add(0x10 + 4 * k), byte);
            mem.write(rec.wrapping_add(0x30 + 4 * k), byte);
        }
        mem.write(rec.wrapping_add(0x80), 500u32);
        let field = |off: u32| rec.wrapping_add(off);
        let scissor = [
            field(0),
            field(4),
            field(0x20),
            field(0x24),
            field(8),
            field(0xc),
            field(0x28),
            field(0x2c),
        ];
        if call(mem, SCISSOR, &scissor) != 0 {
            call(mem, DRAW_UI, &[rec, texture, 1, 0, 0]);
        }
    }

    /// `TOMenuDrawTimer(frames, x, y, alpha, kind)` (`0x008e19f0`): a play
    /// time in frames (60 a second) drawn right-aligned at `x` as
    /// hours, a `J` separator and minutes, in the battle edge font at
    /// scale 0.63 (`0x00b66864`) and 1.5 (`0x00b3eb58`). The minutes
    /// (`frames / 60 % 60`, `"%02d"`) are measured with
    /// `ToGetBtlEasyFontWidth` and drawn with `ToMakeBtlEasyEdgeFont` at
    /// `x - width`; each next piece goes left by its width plus one letter
    /// space (`TOGetFontSpaceInPiexl(0)`), two for languages 1..=8. The
    /// separator is drawn only for `kind` 0x25 (with 0x25 as its alpha);
    /// the hours (`frames / 3600`, at most 999) are measured with
    /// `ToGetBtlEasyDigitWidth2` and drawn with `ToMakeBtlEasyEdgeDigit2`.
    /// Every `y` is `y - graphicUiRemapY2(9.0)`. Each position is rounded
    /// to `f32`; the sums before it are doubles.
    ///
    /// `scratch` stands for the original's frame (0x10 bytes): the width at
    /// `+0`, the separator text at `+4`, the minutes at `+8`. Float results
    /// (ST0) come through `fcall`.
    pub fn to_menu_draw_timer<M: Mem>(
        mem: &mut M,
        args: [u32; 5],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use ffx_bridge::x87;
        const LANG: u32 = 0x008a_c2f0;
        const SPACE: u32 = 0x008f_a570;
        const REMAP_Y2: u32 = 0x0064_4820;
        const FONT_WIDTH: u32 = 0x0090_5290;
        const EDGE_FONT: u32 = 0x0090_5930;
        const DIGIT_WIDTH2: u32 = 0x0090_5230;
        const EDGE_DIGIT2: u32 = 0x0090_5820;
        let image_f32 = |mem: &M, a: u32| -> u32 {
            x87::round_trip(f32::from_bits(mem.read(mem.rebase(a)))).to_bits()
        };
        let [frames, x, y, alpha, kind] = args;
        let (width, sep, minutes) = (scratch, scratch.wrapping_add(4), scratch.wrapping_add(8));
        let hours = (frames / 3600).min(999);
        let mins = frames / 60 % 60;
        #[allow(clippy::cast_possible_truncation)] // mins < 60
        let digits = [b'0' + (mins / 10) as u8, b'0' + (mins % 10) as u8, 0];
        mem.write(minutes, digits);
        mem.write(sep, 0x4au16);
        let scale = image_f32(mem, 0x00b6_6864);
        let wide = image_f32(mem, 0x00b3_eb58);
        let nine = image_f32(mem, 0x00b5_5140);
        let two: f64 = mem.read(mem.rebase(0x00b9_2270));
        let w = |mem: &M| f64::from(mem.read::<f32>(width));
        // y - graphicUiRemapY2(9.0), rounded to f32
        let low_y = |mem: &mut M, fcall: &mut dyn FnMut(&mut M, u32, &[u32]) -> f64| {
            let r = fcall(mem, REMAP_Y2, &[nine]);
            x87::store(x87::sub(x87::load(y), r))
        };
        // x less one piece: its width and one letter space (two for 1..=8)
        let left_of = |mem: &mut M,
                       x: u32,
                       call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
                       fcall: &mut dyn FnMut(&mut M, u32, &[u32]) -> f64| {
            let lang = call(mem, LANG, &[]).cast_signed();
            let double = lang > 0 && call(mem, LANG, &[]).cast_signed() < 9;
            let mut gap = fcall(mem, SPACE, &[0]);
            if double {
                gap = x87::mul(gap, two);
            }
            x87::store(x87::sub(x87::load(x), x87::add(gap, w(mem))))
        };
        call(mem, FONT_WIDTH, &[minutes, width, alpha, scale, wide]);
        let x = x87::store(x87::sub(x87::load(x), w(mem)));
        let ly = low_y(mem, &mut fcall);
        call(mem, EDGE_FONT, &[minutes, x, ly, alpha, scale, wide]);
        call(mem, FONT_WIDTH, &[sep, width, alpha, scale, wide]);
        let x = left_of(mem, x, &mut call, &mut fcall);
        if kind == 0x25 {
            let ly = low_y(mem, &mut fcall);
            call(mem, EDGE_FONT, &[sep, x, ly, 0x25, scale, wide]);
        }
        call(mem, DIGIT_WIDTH2, &[hours, width, scale, wide]);
        let x = left_of(mem, x, &mut call, &mut fcall);
        let ly = low_y(mem, &mut fcall);
        call(mem, EDGE_DIGIT2, &[hours, x, ly, alpha, scale, wide]);
    }

    /// `DrawCrossCtbIcon(id, x, y, w, h)` (`0x0089e660`): a battle command
    /// or CTB icon from the cross-menu sheet, drawn with
    /// `TOMkpShapeXYWHUV(400, x, y, w, h, u0, v0, u1, v1)` between
    /// `setButtonData(-1, id)` and, for ids 0x1b6/0x1b7 and the unknown
    /// ones, `resetButtonData()` (the other ids return without it). UVs are
    /// texels over 1024 (`0x00b58b50`); positions and products are doubles
    /// rounded to `f32` where the original stores. By `id`:
    /// - 0x1b8..=0x1c2: `u0` texel `id % 2 × 129 + 768`, 120 wide; `v0`
    ///   texel `(id - 0x1b8) / 2 × 96`, 80 high.
    /// - 0x1c3..=0x1ca: a frame (`u, v` 0.875977/0.46875 to
    ///   0.993164/0.546875), then the icon at `x + 0.3w`, `y + 0.2h`, size
    ///   `0.4w × 0.6h`, `u0` texel `(id - 0x1c3) × 31 + 769`, 30 wide, `v`
    ///   0.810547 to 0.855469.
    /// - 0x1cb..=0x1d2: a frame (0.75/0.5625 to 0.867188/0.640625), then the
    ///   icon at `x + 0.52w`, `y + 0.55h`, size `0.288w × 0.4h`, `u0` texel
    ///   `(id - 0x1cb) % 4 × 60 + 769` (one more for 0x1cf), 47 wide; `v0`
    ///   0.873047 below 0x1cf else 0.931641, 50 high.
    /// - 0x1d3, 0x1d4: the two frames alone.
    /// - 0x1b6, 0x1b7: two arrow halves sized by `graphicUiRemapX2`/`Y2`
    ///   (through `fcall`), not by `w`, `h`.
    ///
    /// Returns what the last call leaves in `eax`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn draw_cross_ctb_icon<M: Mem>(
        mem: &mut M,
        args: [u32; 5],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, round_trip, store, sub};
        const SET_BUTTON: u32 = 0x008b_1350;
        const RESET_BUTTON: u32 = 0x008b_1330;
        const SHAPE: u32 = 0x0090_3bb0;
        const REMAP_X2: u32 = 0x0064_47e0;
        const REMAP_Y2: u32 = 0x0064_4820;
        let f = |mem: &M, a: u32| -> u32 {
            round_trip(f32::from_bits(mem.read(mem.rebase(a)))).to_bits()
        };
        let d = |mem: &M, a: u32| -> f64 { mem.read(mem.rebase(a)) };
        #[allow(clippy::cast_precision_loss)] // fild then fstp dword: rounded
        let int_f32 = |v: i32| f64::from(v as f32);
        let id = args[0];
        let [x, y, w, h] =
            [args[1], args[2], args[3], args[4]].map(|v| round_trip(f32::from_bits(v)).to_bits());
        let n = id.cast_signed();
        let (fx, fy, fw, fh) = (load(x), load(y), load(w), load(h));
        let texel = d(mem, 0x00b5_8b50);
        call(mem, SET_BUTTON, &[u32::MAX, id]);
        let rel = |base: u32| id.wrapping_sub(base);
        let shape = |mem: &mut M,
                     call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
                     xywh: [u32; 4],
                     uv: [u32; 4]| {
            let [x, y, w, h] = xywh;
            let [u0, v0, u1, v1] = uv;
            call(mem, SHAPE, &[400, x, y, w, h, u0, v0, u1, v1])
        };
        if rel(0x1b8) <= 10 {
            let a = store(add(
                mul(int_f32(n % 2), d(mem, 0x00b5_f210)),
                d(mem, 0x00b5_f298),
            ));
            let b = store(mul(
                int_f32(rel(0x1b8).cast_signed() / 2),
                d(mem, 0x00b5_ee40),
            ));
            let b80 = store(add(d(mem, 0x00b5_f200), load(b)));
            let v1 = store(mul(load(b80), texel));
            let a120 = store(add(d(mem, 0x00b5_f208), load(a)));
            let u1 = store(mul(load(a120), texel));
            let v0 = store(mul(load(b), texel));
            let u0 = store(mul(load(a), texel));
            return shape(mem, &mut call, [x, y, w, h], [u0, v0, u1, v1]);
        }
        if rel(0x1c3) <= 7 {
            let frame = [0x00b5_f158, 0x00b5_f0d8, 0x00b5_f168, 0x00b5_f0f4].map(|a| f(mem, a));
            shape(mem, &mut call, [x, y, w, h], frame);
            let t = int_f32(rel(0x1c3).cast_signed());
            let u0 = store(mul(
                add(mul(t, d(mem, 0x00b5_99d8)), d(mem, 0x00b5_f2a0)),
                texel,
            ));
            let u1 = store(add(d(mem, 0x00b5_f170), load(u0)));
            let uv = [u0, f(mem, 0x00b5_f140), u1, f(mem, 0x00b5_f148)];
            let h2 = store(mul(d(mem, 0x00b4_8320), fh));
            let w2 = store(mul(d(mem, 0x00b4_8318), fw));
            let y2 = store(add(mul(fh, d(mem, 0x00b3_eb60)), fy));
            let x2 = store(add(mul(fw, d(mem, 0x00b4_8308)), fx));
            return shape(mem, &mut call, [x2, y2, w2, h2], uv);
        }
        if rel(0x1cb) <= 7 {
            let frame = [0x00b4_3d18, 0x00b5_f104, 0x00b5_f14c, 0x00b5_f110].map(|a| f(mem, a));
            shape(mem, &mut call, [x, y, w, h], frame);
            let t = int_f32(rel(0x1cb).cast_signed() % 4);
            let base = store(mul(
                add(mul(t, d(mem, 0x00b4_a180)), d(mem, 0x00b5_f2a0)),
                texel,
            ));
            let u0 = if id == 0x1cf {
                store(add(texel, load(base)))
            } else {
                base
            };
            let v0 = f(mem, if n >= 0x1cf { 0x00b5_f15c } else { 0x00b5_f154 });
            let v1 = store(add(d(mem, 0x00b5_f180), load(v0)));
            let u1 = store(add(d(mem, 0x00b5_f178), load(u0)));
            let h2 = store(mul(d(mem, 0x00b4_8318), fh));
            let w2 = store(mul(d(mem, 0x00b5_f190), fw));
            let y2 = store(add(mul(fh, d(mem, 0x00b5_f1a0)), fy));
            let x2 = store(add(mul(fw, d(mem, 0x00b5_f198)), fx));
            return shape(mem, &mut call, [x2, y2, w2, h2], [u0, v0, u1, v1]);
        }
        match id {
            0x1d3 => {
                let uv = [0x00b5_f158, 0x00b5_f104, 0x00b5_f168, 0x00b5_f110].map(|a| f(mem, a));
                return shape(mem, &mut call, [x, y, w, h], uv);
            }
            0x1d4 => {
                let uv = [0x00b4_3d18, 0x00b5_f114, 0x00b5_f14c, 0x00b5_f134].map(|a| f(mem, a));
                return shape(mem, &mut call, [x, y, w, h], uv);
            }
            0x1b6 | 0x1b7 => {
                let remap =
                    |mem: &mut M,
                     fcall: &mut dyn FnMut(&mut M, u32, &[u32]) -> f64,
                     sym: u32,
                     at: u32| { fcall(mem, sym, &[f(mem, at)]) };
                let up = id == 0x1b6;
                // the upper half: 14 × 15 at x + remapX2(11) - 1, y - 11 (or 2)
                let h1 = store(remap(mem, &mut fcall, REMAP_Y2, 0x00b5_99e8));
                let w1 = store(remap(mem, &mut fcall, REMAP_X2, 0x00b5_4f84));
                let lift = d(mem, if up { 0x00b5_f1b0 } else { 0x00b9_2270 });
                let y1 = store(sub(fy, lift));
                let r = remap(mem, &mut fcall, REMAP_X2, 0x00b5_f2ec);
                let x1 = store(sub(add(r, fx), d(mem, 0x00b9_2248)));
                let uv = [0x00b5_f12c, 0x00b5_f160, 0x00b5_f13c, 0x00b5_f164].map(|a| f(mem, a));
                shape(mem, &mut call, [x1, y1, w1, h1], uv);
                // the lower half: 24 × 12
                let h2 = store(remap(mem, &mut fcall, REMAP_Y2, 0x00b4_7de8));
                let w2 = store(remap(mem, &mut fcall, REMAP_X2, 0x00b5_4f90));
                let y2 = if up {
                    let r = remap(mem, &mut fcall, REMAP_Y2, 0x00b5_9830);
                    store(sub(sub(fy, r), d(mem, 0x00b2_c5c8)))
                } else {
                    let r = remap(mem, &mut fcall, REMAP_Y2, 0x00b5_ee70);
                    store(add(add(r, fy), d(mem, 0x00b9_2270)))
                };
                let r = remap(mem, &mut fcall, REMAP_X2, 0x00b5_f2ec);
                let x2 = store(add(r, fx));
                let uv = if up {
                    [0x00b5_f118, 0x00b5_f120, 0x00b5_f138, 0x00b5_f130]
                } else {
                    [0x00b5_f118, 0x00b5_f144, 0x00b5_f138, 0x00b5_f150]
                };
                let uv = uv.map(|a| f(mem, a));
                shape(mem, &mut call, [x2, y2, w2, h2], uv);
            }
            _ => {}
        }
        call(mem, RESET_BUTTON, &[])
    }

    /// `TOBtlDrawInfoWinMes()` (`0x00895140`): with libra's auto mode
    /// ([`LIBRA_AUTO`]) not 1, the info window shown (`0x0133c93d`) and
    /// bit 0x400 of the menu settings (`MsGetSaveMenu`), the window's four
    /// text lines (0x1c-byte records from `0x0133f618`: `+0` an `i16`
    /// shown flag, `+0xc`/`+0x10` the float position, `+0x18` the text)
    /// drawn with `TOMkpCrossExtMesFontLClut(0, text, x, y, 0, 0.78, 1.0)`
    /// inside a font scissor over the `i16` box at `0x0133f694..0x0133f6a2`
    /// (`OpenFontScissor`, `CloseScissor`).
    pub fn to_btl_draw_info_win_mes<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::round_trip;
        let byte = |mem: &M, a: u32| -> u8 { mem.read(mem.rebase(a)) };
        if LIBRA_AUTO.get(mem) == 1 || byte(mem, 0x0133_c93d) == 0 {
            return;
        }
        let settings = call(mem, 0x0078_5290, &[]);
        if mem.read::<u32>(settings) & 0x400 == 0 {
            return;
        }
        let corner = |mem: &M, a: u32| f32::from(mem.read::<i16>(mem.rebase(a))).to_bits();
        let bx = [0x0133_f694, 0x0133_f698, 0x0133_f69c, 0x0133_f6a0].map(|a| corner(mem, a));
        call(mem, 0x008e_8940, &bx);
        let scale = round_trip(f32::from_bits(mem.read(mem.rebase(0x00b5_ee10)))).to_bits();
        let rt = |mem: &M, a: u32| round_trip(f32::from_bits(mem.read(a))).to_bits();
        for k in 0..4u32 {
            let line = mem.rebase(0x0133_f618).wrapping_add(k * 0x1c);
            if mem.read::<i16>(line) != 0 {
                let text: u32 = mem.read(line.wrapping_add(0x18));
                let (x, y) = (
                    rt(mem, line.wrapping_add(0xc)),
                    rt(mem, line.wrapping_add(0x10)),
                );
                call(
                    mem,
                    0x0090_16b0,
                    &[0, text, x, y, 0, scale, 1.0f32.to_bits()],
                );
            }
        }
        call(mem, 0x008e_5cd0, &[]);
    }

    /// The help line's scroll offset (`0x0133c940`, `f32`).
    pub const HELP_SCROLL: Global<f32> = Global::new(0x0133_c940);

    /// `TOBtlDrawHelpWin()` (`0x008936d0`): the battle help window. With
    /// libra's auto mode ([`LIBRA_AUTO`]) not 1 and bit 0x400 of the menu
    /// settings (`MsGetSaveMenu`): closed (`g_BtlHelpWinOpen`,
    /// `0x0133c93c`, not 1) it resets [`HELP_SCROLL`]; open, it draws the
    /// band (`TOMkpShapeXYWHUVC2(-1, ...)`, grey 0x80808080/0x808080) and
    /// frame (`TOMkpShapeXYWHUV(0x3e, ...)`) inside `TODrawScissorXYWH(0,
    /// 0, 0x200, 0x1a0)`, then the help text (`0x0133f538`) with
    /// `TOMkpCrossExtMesFontLClut(0, text, x, y, 0, 0.78, 1.0)` at
    /// `remapX2(235)`, `remapY2(101)`. A text wider than `remapX2(435)`
    /// scrolls while the info window is shown or libra's auto mode is not
    /// 0: inside `OpenFontScissor(40, 40, 178, 62)` the offset drops by 0.8
    /// a frame, the text is drawn at `x + offset`, and again 20 further
    /// while its end is left of 158; once the end passes 40 the offset
    /// moves on by `width + 20`. Positions come from `graphicUiRemapX2`/`Y2`
    /// (through `fcall`); sums are doubles, rounded to `f32` where stored.
    ///
    /// `scratch` holds the text width (`ToGetCrossExtMesFontWidth`).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn to_btl_draw_help_win<M: Mem>(
        mem: &mut M,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use ffx_bridge::x87::{add, load, round_trip, store, sub};
        const REMAP_X2: u32 = 0x0064_47e0;
        const REMAP_Y2: u32 = 0x0064_4820;
        const FONT: u32 = 0x0090_16b0;
        const TEXT: u32 = 0x0133_f538;
        let f = |mem: &M, a: u32| -> u32 {
            round_trip(f32::from_bits(mem.read(mem.rebase(a)))).to_bits()
        };
        let d = |mem: &M, a: u32| -> f64 { mem.read(mem.rebase(a)) };
        let remap = |mem: &mut M,
                     fcall: &mut dyn FnMut(&mut M, u32, &[u32]) -> f64,
                     sym: u32,
                     at: u32| { fcall(mem, sym, &[f(mem, at)]) };
        if LIBRA_AUTO.get(mem) == 1 {
            return;
        }
        let settings = call(mem, 0x0078_5290, &[]);
        if mem.read::<u32>(settings) & 0x400 == 0 {
            return;
        }
        if mem.read::<u8>(mem.rebase(0x0133_c93c)) != 1 {
            HELP_SCROLL.set(mem, 0.0);
            return;
        }
        call(mem, 0x008f_9230, &[0, 0, 0x200, 0x1a0]);
        // the band: remapX2(145), remapY2(96), remapX2(1600) × remapY2(55)
        let h = store(remap(mem, &mut fcall, REMAP_Y2, 0x00b5_eea8));
        let w = store(remap(mem, &mut fcall, REMAP_X2, 0x00b5_f030));
        let y = store(remap(mem, &mut fcall, REMAP_Y2, 0x00b4_3db8));
        let x = store(remap(mem, &mut fcall, REMAP_X2, 0x00b5_eef0));
        let [v0, u1, v1] = [0x00b5_efb0, 0x00b5_f030, 0x00b5_efcc].map(|a| f(mem, a));
        let grey = [0x8080_8080, 0x0080_8080];
        call(
            mem,
            0x0090_3ee0,
            &[u32::MAX, x, y, w, h, 0, v0, u1, v1, grey[0], grey[1]],
        );
        // the frame: remapX2(145), remapY2(78), remapX2(95) × remapY2(22)
        let h = store(remap(mem, &mut fcall, REMAP_Y2, 0x00b5_ee64));
        let w = store(remap(mem, &mut fcall, REMAP_X2, 0x00b5_eedc));
        let y = store(remap(mem, &mut fcall, REMAP_Y2, 0x00b5_eed0));
        let x = store(remap(mem, &mut fcall, REMAP_X2, 0x00b5_eef0));
        let uv = [0x00b5_edf4, 0x00b5_ee20, 0x00b5_ee0c, 0x00b5_ee2c].map(|a| f(mem, a));
        call(
            mem,
            0x0090_3bb0,
            &[0x3e, x, y, w, h, uv[0], uv[1], uv[2], uv[3]],
        );
        let scale = f(mem, 0x00b5_ee10);
        let one = 1.0f32.to_bits();
        let text = mem.rebase(TEXT);
        call(mem, 0x0090_5320, &[0, text, scratch, scale, one]);
        let wd = f64::from(mem.read::<f32>(scratch));
        let shown = mem.read::<u8>(mem.rebase(0x0133_c93d)) == 1;
        let scrolls = (shown || LIBRA_AUTO.get(mem) != 0) && {
            let limit = remap(mem, &mut fcall, REMAP_X2, 0x00b5_ef6c);
            limit < wd
        };
        let line_y = |mem: &mut M, fcall: &mut dyn FnMut(&mut M, u32, &[u32]) -> f64| {
            store(remap(mem, fcall, REMAP_Y2, 0x00b5_eee4))
        };
        if !scrolls {
            let y = line_y(mem, &mut fcall);
            let x = store(remap(mem, &mut fcall, REMAP_X2, 0x00b5_ef2c));
            call(mem, FONT, &[0, text, x, y, 0, scale, one]);
            return;
        }
        let edge = f(mem, 0x00b4_3db4);
        call(
            mem,
            0x008e_8940,
            &[edge, edge, f(mem, 0x00b5_ef08), f(mem, 0x00b5_eeb4)],
        );
        let s = store(sub(
            load(HELP_SCROLL.get(mem).to_bits()),
            d(mem, 0x00b3_eb68),
        ));
        HELP_SCROLL.set(mem, f32::from_bits(s));
        let scroll = |mem: &M| load(HELP_SCROLL.get(mem).to_bits());
        let y = line_y(mem, &mut fcall);
        let r = remap(mem, &mut fcall, REMAP_X2, 0x00b5_ef2c);
        let x = store(add(r, scroll(mem)));
        call(mem, FONT, &[0, text, x, y, 0, scale, one]);
        let r = remap(mem, &mut fcall, REMAP_X2, 0x00b5_ef2c);
        let twenty = d(mem, 0x00b4_9968);
        if add(add(r, scroll(mem)), wd) < d(mem, 0x00b5_ee48) {
            let y = line_y(mem, &mut fcall);
            let r = remap(mem, &mut fcall, REMAP_X2, 0x00b5_ef2c);
            let x = store(add(add(add(r, scroll(mem)), wd), twenty));
            call(mem, FONT, &[0, text, x, y, 0, scale, one]);
        }
        let r = remap(mem, &mut fcall, REMAP_X2, 0x00b5_ef2c);
        let s = scroll(mem);
        if add(add(r, s), wd) < d(mem, 0x00b5_af28) {
            let v = store(add(add(wd, twenty), s));
            HELP_SCROLL.set(mem, f32::from_bits(v));
        }
        call(mem, 0x008e_5cd0, &[]);
    }

    /// The command list window `w`'s entry under its cursor (`+0x42`,
    /// `i16`) in its list of `i16` commands (`+0x20`).
    fn cursor_entry<M: Mem>(mem: &M, w: u32) -> u32 {
        let list: u32 = mem.read(w.wrapping_add(0x20));
        let cursor: i16 = mem.read(w.wrapping_add(0x42));
        list.wrapping_add((i32::from(cursor) * 2).cast_unsigned())
    }

    /// Slot `base` (`+0x16` picked commands, `+0xe` their cursors) of the
    /// current pick (`+0xb`, `i8`).
    fn pick_slot<M: Mem>(mem: &M, w: u32, base: u32) -> u32 {
        let n: i8 = mem.read(w.wrapping_add(0xb));
        w.wrapping_add(base)
            .wrapping_add((i32::from(n) * 2).cast_unsigned())
    }

    /// The command under the cursor and the cursor recorded as the
    /// current pick.
    fn record_pick<M: Mem>(mem: &mut M, w: u32) {
        let command: u16 = mem.read(cursor_entry(mem, w));
        let at = pick_slot(mem, w, 0x16);
        mem.write(at, command);
        let cursor: u16 = mem.read(w.wrapping_add(0x42));
        let at = pick_slot(mem, w, 0xe);
        mem.write(at, cursor);
    }

    /// Cancel with picks made (`+0xb` above 0): the last one undone (its
    /// slots set to 0xff, the cursor and current command `+0x1e` back to
    /// the pick before), `MsSetUseItem(0)` and click 4. False with none.
    fn undo_pick<M: Mem>(
        mem: &mut M,
        w: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> bool {
        let n: i8 = mem.read(w.wrapping_add(0xb));
        if n <= 0 {
            return false;
        }
        for base in [0x16, 0xe] {
            let at = pick_slot(mem, w, base);
            mem.write(at, 0xffu16);
        }
        mem.write(w.wrapping_add(0xb), n.wrapping_sub(1));
        let cursor: u16 = mem.read(pick_slot(mem, w, 0xe));
        mem.write(w.wrapping_add(0x42), cursor);
        let command: u16 = mem.read(pick_slot(mem, w, 0x16));
        mem.write(w.wrapping_add(0x1e), command);
        call(mem, 0x0079_0790, &[0]);
        call(mem, 0x0088_6ba0, &[4]);
        true
    }

    /// Confirm: `TOCheckBtlCommandUse(actor +8, checked)` not negative
    /// commits the pick ([`commit_pick`]); a refused one clicks 3.
    fn confirm_pick<M: Mem>(
        mem: &mut M,
        w: u32,
        checked: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let actor = i32::from(mem.read::<i16>(w.wrapping_add(8))).cast_unsigned();
        if call(mem, 0x0089_aca0, &[actor, checked]).cast_signed() >= 0 {
            commit_pick(mem, w, call);
        } else {
            call(mem, 0x0088_6ba0, &[3]);
        }
    }

    /// An accepted pick: recorded and counted; below the picks needed
    /// (`+0xa`) `MsSetUseItem(first pick)` and click 1, else
    /// `TOBtlSetNextAction(actor, command under the cursor)` 1 gives click
    /// 1 and anything else click 3.
    fn commit_pick<M: Mem>(mem: &mut M, w: u32, call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32) {
        const CLICK: u32 = 0x0088_6ba0;
        record_pick(mem, w);
        let n = mem.read::<i8>(w.wrapping_add(0xb)).wrapping_add(1);
        mem.write(w.wrapping_add(0xb), n);
        if n < mem.read::<i8>(w.wrapping_add(0xa)) {
            let first = u32::from(mem.read::<u16>(w.wrapping_add(0x16)));
            call(mem, 0x0079_0790, &[first]);
            call(mem, CLICK, &[1]);
            return;
        }
        let actor = i32::from(mem.read::<i16>(w.wrapping_add(8))).cast_unsigned();
        let command = i32::from(mem.read::<i16>(cursor_entry(mem, w))).cast_unsigned();
        let ok = call(mem, 0x0089_a330, &[actor, command]) == 1;
        call(mem, CLICK, &[if ok { 1 } else { 3 }]);
    }

    /// The repeated d-pad moves the cursor (`+0x42`) unless two opposite
    /// directions are held (`0x9000`, `0x3000`, `0xc000`, `0x6000` of the
    /// held word): up (0x1000) and down (0x4000) a row ([`wrap_row`]), left
    /// (0x8000) and right (0x2000) an entry, clamped to the list (`+0x44`).
    /// Only an exact repeated word counts.
    fn dpad_move<M: Mem>(mem: &mut M, w: u32, pad: u32) {
        let r16 = |mem: &M, off: u32| -> i16 { mem.read(w.wrapping_add(off)) };
        let w16 = |mem: &mut M, off: u32, v: i16| mem.write(w.wrapping_add(off), v);
        let held: u16 = mem.read(pad);
        let both = |m: u16| held & m == m;
        if both(0x9000) || both(0x3000) || both(0xc000) || both(0x6000) {
            return;
        }
        let old = r16(mem, 0x42);
        let rep: u16 = mem.read(pad.wrapping_add(4));
        let step = r16(mem, 0x38);
        let count = r16(mem, 0x44);
        match rep {
            0x1000 => w16(mem, 0x42, old.wrapping_sub(step)),
            0x4000 => w16(mem, 0x42, old.wrapping_add(step)),
            0x8000 => w16(mem, 0x42, old.wrapping_sub(1)),
            0x2000 => w16(mem, 0x42, old.wrapping_add(1)),
            _ => {}
        }
        if rep == 0x1000 || rep == 0x4000 {
            wrap_row(mem, w);
        }
        if rep == 0x8000 || rep == 0x2000 {
            if r16(mem, 0x42) < 0 {
                w16(mem, 0x42, 0);
            }
            if r16(mem, 0x42) >= count {
                w16(mem, 0x42, count.wrapping_sub(1));
            }
        }
    }

    /// After a move by a row: a cursor above the list wraps by one row
    /// (`+0x38` columns), one past its end (`+0x44` entries) goes up a row
    /// if that is a later row than the last entry's, else to the last
    /// entry. A zero column count makes the original divide by zero (a
    /// crash); here the quotient is 0.
    #[allow(clippy::cast_possible_truncation)] // `mov word`
    fn wrap_row<M: Mem>(mem: &mut M, w: u32) {
        let step: i16 = mem.read(w.wrapping_add(0x38));
        let count: i16 = mem.read(w.wrapping_add(0x44));
        let at = w.wrapping_add(0x42);
        let cursor: i16 = mem.read(at);
        if cursor < 0 {
            mem.write(at, step.wrapping_add(cursor));
        }
        let cursor: i16 = mem.read(at);
        if cursor >= count {
            let div = |a: i32| a.checked_div(i32::from(step)).unwrap_or(0);
            if div(i32::from(cursor)) > div(i32::from(count) - 1) {
                mem.write(at, cursor.wrapping_sub(step));
            } else {
                mem.write(at, count.wrapping_sub(1));
            }
        }
    }

    /// Records the pick, then keeps the cursor's row shown: the first
    /// shown row (`+0x40`) goes up a row when the cursor is above it and
    /// down one when it is past the `+0x3a` rows shown (with `fns`, the
    /// step function `+0x88` becomes `TOBtlPadScrollDown`/`Up`), then is
    /// clamped to the list (0 when it all fits). Returns the cursor.
    #[allow(clippy::cast_possible_truncation)] // `mov word`
    fn follow_cursor<M: Mem>(mem: &mut M, w: u32, fns: bool) -> i16 {
        let r16 = |mem: &M, off: u32| -> i16 { mem.read(w.wrapping_add(off)) };
        record_pick(mem, w);
        let cursor = r16(mem, 0x42);
        let step = r16(mem, 0x38);
        if cursor < r16(mem, 0x40) {
            let top = r16(mem, 0x40).wrapping_sub(step);
            if fns {
                let f = mem.rebase(0x0089_8df0);
                mem.write(w.wrapping_add(0x88), f);
            }
            mem.write(w.wrapping_add(0x40), top);
        }
        let shown = i32::from(r16(mem, 0x3a)).wrapping_mul(i32::from(step));
        if i32::from(cursor) >= i32::from(r16(mem, 0x40)).wrapping_add(shown) {
            let top = r16(mem, 0x40).wrapping_add(step);
            mem.write(w.wrapping_add(0x40), top);
            if fns {
                let f = mem.rebase(0x0089_8e20);
                mem.write(w.wrapping_add(0x88), f);
            }
        }
        let count = i32::from(r16(mem, 0x44));
        let top = if count > shown {
            let rows = (count - 1 + i32::from(step))
                .checked_div(i32::from(step))
                .unwrap_or(0);
            let last = rows
                .wrapping_sub(i32::from(r16(mem, 0x3a)))
                .wrapping_mul(i32::from(step));
            let top = i32::from(r16(mem, 0x40));
            if top > last { last } else { top.max(0) }
        } else {
            0
        };
        mem.write(w.wrapping_add(0x40), top as i16);
        cursor
    }

    /// `TOGetPadStatusPrepare(w)` (`0x0089d610`): one frame of a battle
    /// command list window `w` while the battle windows are open
    /// (`g_BtlWindowOpen`, `0x023cc08c`) and its step function (`+0x88`) is
    /// the idle [`SCROLL_DONE`]. The TO pad words (`g_TOMenuPad`,
    /// `0x025d09d2`: held, `+2` triggered, `+4` repeated; libra's auto mode
    /// forces the trigger word to 0x40):
    /// - cancel (0x40): [`undo_pick`], else at a deeper window level
    ///   (`g_BtlActionDepth`) `TOBtlSetPrevAction()`, `+1` stepped, click 4.
    /// - confirm (0x20): [`confirm_pick`] of the command under the cursor.
    /// - 0x04: click 3.
    /// - otherwise, unless two opposite d-pad directions are held, the
    ///   repeated up/down move the cursor a row ([`wrap_row`]), left/right
    ///   one entry (clamped to the list; [`dpad_move`]); then
    ///   [`follow_cursor`] with the scroll step functions, and a moved
    ///   cursor sets `+0x1e` and clicks 1.
    pub fn to_get_pad_status_prepare<M: Mem>(
        mem: &mut M,
        w: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        const CLICK: u32 = 0x0088_6ba0;
        let pad = mem.rebase(0x025d_09d2);
        if mem.read::<u32>(mem.rebase(0x023c_c08c)) == 0
            || mem.read::<u32>(w.wrapping_add(0x88)) != mem.rebase(SCROLL_DONE)
        {
            return;
        }
        if LIBRA_AUTO.get(mem) == 1 {
            mem.write(pad.wrapping_add(2), 0x40u16);
        }
        let trigger: u8 = mem.read(pad.wrapping_add(2));
        if trigger & 0x40 != 0 {
            if !undo_pick(mem, w, &mut call) && mem.read::<i8>(mem.rebase(0x023c_c092)) > 0 {
                call(mem, 0x0089_a630, &[]);
                let n = mem.read::<u8>(w.wrapping_add(1)).wrapping_add(1);
                mem.write(w.wrapping_add(1), n);
                call(mem, CLICK, &[4]);
            }
            return;
        }
        if trigger & 0x20 != 0 {
            let command = i32::from(mem.read::<i16>(cursor_entry(mem, w))).cast_unsigned();
            confirm_pick(mem, w, command, &mut call);
            return;
        }
        if trigger & 4 != 0 {
            call(mem, CLICK, &[3]);
            return;
        }
        let old: i16 = mem.read(w.wrapping_add(0x42));
        dpad_move(mem, w, pad);
        let cursor = follow_cursor(mem, w, true);
        if cursor != old {
            let command: u16 = mem.read(cursor_entry(mem, w));
            mem.write(w.wrapping_add(0x1e), command);
            call(mem, CLICK, &[1]);
        }
    }

    /// The battle windows' wait functions that hand a frame to a pad
    /// handler once the window has opened far enough (`f32 w + 0xe0`).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum WaitBtlWin {
        /// `TOWaitBtlWinLeft(w)` (`0x008a8a40`): `TOBtlGetLeftPad` from
        /// 0.45 on (the double constant holds 0.45f32); returns 0.
        Left,
        /// `TOWaitBtlWinRight_008a8af0(w)`: `TOBtlGetRightPad` from 0.45;
        /// returns 0.
        Right,
        /// `TOWaitBtlWinStd_008a8b20(w)`: `TOBtlGetStdPad` from 0.3
        /// (`f32`); returns 0.
        Std,
        /// `TOWaitBtlWinPrepare_008a8ac0(w)`: `TOGetPadStatusPrepare` from
        /// 0.3; returns 1.
        Prepare,
        /// `TOWaitBtlWinChange(w)` (`0x008a87a0`): `TOBtlGetChangePad`
        /// from 0.3 (`f32`); returns 1.
        Change,
        /// `TOWaitBtlWinStdMode2_008a8b50(w)` (the summon list's): the
        /// battle list mode (`0x0133d6e8`) 2 around `TOBtlGetStdPad`, from
        /// 0.45 (the double holds 0.45f32); returns 1.
        StdMode2,
        /// `TOWaitBtlWinStdMode3_008a8b90(w)` (the weapon and kinds 7/0xf
        /// lists'): as [`WaitBtlWin::StdMode2`] with mode 3; returns 0
        /// when below the threshold, else 1.
        StdMode3,
    }

    /// One [`WaitBtlWin`] frame of window `w`: the handler runs unless the
    /// opening (`f32 w + 0xe0`) is below the threshold (a NaN runs it).
    /// Callees through `call` (`ffx.sym` addresses).
    pub fn wait_btl_win<M: Mem>(
        mem: &mut M,
        kind: WaitBtlWin,
        w: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const STD_PAD: u32 = 0x0089_cde0;
        let open: f32 = mem.read(w.wrapping_add(0xe0));
        let (below, handler, result) = match kind {
            WaitBtlWin::Left => (open < 0.45f32, 0x008a_22f0, 0),
            WaitBtlWin::Right => (open < 0.45f32, 0x008a_2690, 0),
            WaitBtlWin::Std => (0.3f32 > open, STD_PAD, 0),
            WaitBtlWin::Prepare => (0.3f32 > open, 0x0089_d610, 1),
            WaitBtlWin::Change => (0.3f32 > open, 0x008a_2160, 1),
            WaitBtlWin::StdMode2 | WaitBtlWin::StdMode3 => (open < 0.45f32, STD_PAD, 1),
        };
        let mode = match kind {
            WaitBtlWin::StdMode2 => Some(2u32),
            WaitBtlWin::StdMode3 => Some(3),
            _ => None,
        };
        if below {
            return if matches!(kind, WaitBtlWin::StdMode3) {
                0
            } else {
                result
            };
        }
        let list_mode = mem.rebase(0x0133_d6e8);
        if let Some(m) = mode {
            mem.write(list_mode, m);
        }
        call(mem, handler, &[w]);
        if mode.is_some() {
            mem.write(list_mode, 0u32);
        }
        result
    }

    /// `TOOpenBtlWinLeft()` (`0x008a8620`) and `TOOpenBtlWinRight()`
    /// (`0x008a86d0`): `TOSeqStartScene(2, "bleftwlay" / "brightwlay")`
    /// (the strings at `0x00b5f070` / `0x00b5f07c`); return 1.
    pub fn open_btl_win_side<M: Mem>(
        mem: &mut M,
        right: bool,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let name = mem.rebase(if right { 0x00b5_f07c } else { 0x00b5_f070 });
        call(mem, 0x008a_f0d0, &[2, name]);
        1
    }

    /// The battle gil window (Bribe, Zanmato's pay): `i16 0x01340458`
    /// the digit the cursor is on, `i16 0x01340456` the highest digit,
    /// `i32 0x0134045c` the party's gil, `0x01340460` the most that can be
    /// paid, `0x01340464` the amount, `0x01340468` what would be left.
    pub mod gil {
        pub const DIGIT: u32 = 0x0134_0458;
        pub const TOP_DIGIT: u32 = 0x0134_0456;
        pub const OWNED: u32 = 0x0134_045c;
        pub const MOST: u32 = 0x0134_0460;
        pub const AMOUNT: u32 = 0x0134_0464;
        pub const LEFT: u32 = 0x0134_0468;
    }

    /// `TOWaitBtlWinInputGil(w)` (`0x008a87d0`): one frame of the gil
    /// window on the battle pad trigger (`u16 0x025d09d6`; with
    /// `0x0133d6e0` set no pad: character 0xe (`g_BtlWindowChr`) pays 1 at
    /// once, others cancel). Left/right (0x8000/0x2000) move the digit,
    /// wrapping between 0 and the highest; up/down (0x1000/0x4000) add or
    /// take ten to the digit's power (a click while below the most / above
    /// 0); the amount is kept in 0..=the most and the gil left updated.
    /// Circle (0x20) asks `TOBtlSetNextAction(i16 w+8, *(i16 *)(w+0x20))`:
    /// 1 takes the amount (`0x023cc038`, at least 1 for the automatic
    /// character 0xe, and `0x01340440`) with a click, else the error click
    /// 3. Cross (0x40) below the first action level clears both, steps back
    /// (`TOBtlSetPrevAction`), counts `w+1` up and clicks 4. Returns 1 with
    /// neither pressed, else 0. The digit stays in 0..=8 (a higher top
    /// digit makes the original read its own stack past the powers of
    /// ten). Callees (`SndPlayClick` and the two above) through `call`.
    pub fn wait_btl_win_input_gil<M: Mem>(
        mem: &mut M,
        w: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const CLICK: u32 = 0x0088_6ba0;
        const NEXT_ACTION: u32 = 0x0089_a330;
        const PREV_ACTION: u32 = 0x0089_a630;
        let g = |mem: &M, a: u32| mem.rebase(a);
        let automatic = mem.read::<u32>(g(mem, 0x0133_d6e0)) == 1;
        let chr_0e = |mem: &M| mem.read::<u16>(g(mem, 0x023c_c088)) == 0xe;
        let amount_at = g(mem, gil::AMOUNT);
        let digit_at = g(mem, gil::DIGIT);
        let mut pad: u16 = mem.read(g(mem, 0x025d_09d6));
        if automatic {
            pad = if chr_0e(mem) {
                mem.write(amount_at, 1u32);
                0x20
            } else {
                0x40
            };
        }
        for (bit, step) in [(0x8000u16, 1i16), (0x2000, -1)] {
            if pad & bit != 0 {
                let d = mem.read::<i16>(digit_at).wrapping_add(step);
                mem.write(digit_at, d);
                call(mem, CLICK, &[1]);
            }
        }
        let digit: i16 = mem.read(digit_at);
        let top: i16 = mem.read(g(mem, gil::TOP_DIGIT));
        if digit < 0 {
            mem.write(digit_at, top);
        } else if digit > top {
            mem.write(digit_at, 0i16);
        }
        let most: i32 = mem.read(g(mem, gil::MOST));
        // ten to the digit's power (the original's table of 1..=10^8)
        let power = |mem: &M| {
            u32::try_from(mem.read::<i16>(digit_at))
                .ok()
                .filter(|&d| d <= 8)
                .map_or(0, |d| 10i32.pow(d))
        };
        let mut amount: i32 = mem.read(amount_at);
        if pad & 0x1000 != 0 {
            if amount < most {
                call(mem, CLICK, &[1]);
                amount = mem.read(amount_at);
            }
            amount = amount.wrapping_add(power(mem));
            mem.write(amount_at, amount);
        }
        if pad & 0x4000 != 0 {
            if amount > 0 {
                call(mem, CLICK, &[1]);
                amount = mem.read(amount_at);
            }
            amount = amount.wrapping_sub(power(mem));
        }
        let most: i32 = mem.read(g(mem, gil::MOST));
        let amount = amount.min(most).max(0);
        let owned: i32 = mem.read(g(mem, gil::OWNED));
        mem.write(g(mem, gil::LEFT), owned.wrapping_sub(amount));
        mem.write(amount_at, amount);

        let (taken, shown) = (g(mem, 0x023c_c038), g(mem, 0x0134_0440));
        if pad & 0x20 != 0 {
            let list: u32 = mem.read(w.wrapping_add(0x20));
            let target = i32::from(mem.read::<i16>(list)).cast_unsigned();
            let window = i32::from(mem.read::<i16>(w.wrapping_add(8))).cast_unsigned();
            if call(mem, NEXT_ACTION, &[window, target]) != 1 {
                call(mem, CLICK, &[3]);
                return 0;
            }
            let amount: i32 = mem.read(amount_at);
            let automatic = mem.read::<u32>(g(mem, 0x0133_d6e0)) == 1;
            let chosen = if automatic && chr_0e(mem) && amount < 1 {
                1
            } else {
                amount
            };
            mem.write(taken, chosen);
            mem.write(shown, amount);
            call(mem, CLICK, &[1]);
            return 0;
        }
        if pad & 0x40 == 0 {
            return 1;
        }
        if crate::btl::bridge::ACTION_DEPTH.get(mem) > 0 {
            mem.write(taken, 0u32);
            mem.write(shown, 0u32);
            call(mem, PREV_ACTION, &[]);
            let n = mem.read::<u8>(w.wrapping_add(1)).wrapping_add(1);
            mem.write(w.wrapping_add(1), n);
            call(mem, CLICK, &[4]);
        }
        0
    }

    /// `TOInitBtlWinChange(w)` (`0x008a84d0`): the battle change window's
    /// state (`0x01340800`, eight words) cleared, then `u16` 3 and the
    /// window's `i16 w+8` at its start, `u16 w+0x42` 0, the party list
    /// `TOGetSaveWindow(w+8, 10, 0x0134080c)` to `w+0x20`, and the
    /// `bclose` scene (`0x00b5f094`) started on layer 2 after
    /// `TOSeqInitScene(2)`. Returns 0.
    pub fn init_btl_win_change<M: Mem>(
        mem: &mut M,
        w: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let state = mem.rebase(0x0134_0800);
        mem.write_bytes(state, &[0u8; 0x20]);
        mem.write(state, 3u16);
        mem.write(state + 2, mem.read::<u16>(w.wrapping_add(8)));
        mem.write(w.wrapping_add(0x42), 0u16);
        let window = i32::from(mem.read::<i16>(w.wrapping_add(8))).cast_unsigned();
        let list = call(mem, 0x0089_b5a0, &[window, 10, state + 0xc]);
        mem.write(w.wrapping_add(0x20), list);
        call(mem, 0x008a_eec0, &[2]);
        let name = mem.rebase(0x00b5_f094);
        call(mem, 0x008a_f0d0, &[2, name]);
        0
    }

    /// `TOFuncPadMessageNormal(w)` (`0x008b64a0`): with trigger bit 0x20
    /// (`0x025d09d4`) the message window's `+0xf8` function
    /// (`TOMessageCallF8_008b8ca0(w)`); then on an Eternal Calm map
    /// (`isEternalCalmScene(g_GraphicMapId)`) `TOTimedMacroMessage(w)`,
    /// whose `eax` it returns (else `isEternalCalmScene`'s 0).
    pub fn func_pad_message_normal<M: Mem>(
        mem: &mut M,
        w: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u8>(mem.rebase(0x025d_09d4)) & 0x20 != 0 {
            call(mem, 0x008b_8ca0, &[w]);
        }
        let map: u32 = mem.read(mem.rebase(0x0133_0844));
        let eternal_calm = call(mem, 0x0067_ab50, &[map]);
        if eternal_calm == 0 {
            return 0;
        }
        call(mem, 0x008b_9220, &[w])
    }

    /// `TOMenuEndScreenX2_008e2790(w)` (own name): `w + 0x1c` cleared,
    /// `TkMenuSetScreenX2Flag(0)` (its `eax` returned), the byte
    /// `0x01340858` set to 1.
    pub fn menu_end_screen_x2<M: Mem>(
        mem: &mut M,
        w: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        mem.write(w.wrapping_add(0x1c), 0u32);
        let eax = call(mem, 0x008a_b020, &[0]);
        mem.write(mem.rebase(0x0134_0858), 1u8);
        eax
    }

    /// The window's line record (`0x01868a90[w]`), read again at each use
    /// as the original does.
    fn pad_record<M: Mem>(mem: &M, w: u32) -> u32 {
        mem.read(mem.rebase(0x0186_8a90).wrapping_add(w.wrapping_mul(4)))
    }

    /// The pad words: triggered (`0x025d09d4`) and repeated (`0x025d09d6`).
    const PAD_TRIGGER: u32 = 0x025d_09d4;
    const PAD_REPEAT: u32 = 0x025d_09d6;

    /// `TOFuncPadMessageSelect(w)` (`0x008b64e0`), the `+0xfc` pad handler
    /// of a choice message; `r` is its line record (`0x01868a90[w]`: `+2`
    /// cursor, `+3` default, `+4` first and `+5` end row, `+6` picks to
    /// make, `+7` picks made, `+0x18` the picks). By the triggered buttons:
    /// - 0x40: the cursor back to the default row, click 4 when it moved.
    /// - 0x20: the cursor row stored as pick `+7` (a signed index), `+7`
    ///   stepped, and with all picks made `+1` set to 2; click 1.
    /// - else the repeated 0x4000/0x2000 move it down and 0x1000/0x8000 up
    ///   (both may apply), wrapping inside `+4..+5`; click 1 when it moved.
    ///
    /// Returns `SndPlayClick`'s `eax` (a tail jump); without a click the
    /// original's `eax` is left over (the record pointer or the pad word),
    /// which its only caller, `TOGetControlPadStatus`, passes on to callers
    /// that drop it: 0 here.
    pub fn func_pad_message_select<M: Mem>(
        mem: &mut M,
        w: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const CLICK: u32 = 0x0088_6ba0;
        let r = pad_record(mem, w);
        let was: u8 = mem.read(r.wrapping_add(2));
        let trigger: u16 = mem.read(mem.rebase(PAD_TRIGGER));
        if trigger & 0x40 != 0 {
            let default: u8 = mem.read(r.wrapping_add(3));
            if was == default {
                return 0;
            }
            mem.write(r.wrapping_add(2), default);
            return call(mem, CLICK, &[4]);
        }
        if trigger & 0x20 != 0 {
            let made = i32::from(mem.read::<i8>(r.wrapping_add(7)));
            mem.write(r.wrapping_add(0x18).wrapping_add_signed(made), was);
            let at = pad_record(mem, w).wrapping_add(7);
            mem.write(at, mem.read::<u8>(at).wrapping_add(1));
            let r = pad_record(mem, w);
            if mem.read::<u8>(r.wrapping_add(7)) == mem.read::<u8>(r.wrapping_add(6)) {
                mem.write(r.wrapping_add(1), 2u8);
            }
            return call(mem, CLICK, &[1]);
        }
        let mut repeat: u16 = mem.read(mem.rebase(PAD_REPEAT));
        if repeat & 0x6000 != 0 {
            mem.write(r.wrapping_add(2), was.wrapping_add(1));
            repeat = mem.read(mem.rebase(PAD_REPEAT));
        }
        if repeat & 0x9000 != 0 {
            let at = pad_record(mem, w).wrapping_add(2);
            mem.write(at, mem.read::<u8>(at).wrapping_sub(1));
        }
        let r = pad_record(mem, w);
        if mem.read::<i8>(r.wrapping_add(2)) >= mem.read::<i8>(r.wrapping_add(5)) {
            mem.write(r.wrapping_add(2), mem.read::<u8>(r.wrapping_add(4)));
        }
        let r = pad_record(mem, w);
        if mem.read::<i8>(r.wrapping_add(2)) < mem.read::<i8>(r.wrapping_add(4)) {
            let end: u8 = mem.read(r.wrapping_add(5));
            mem.write(r.wrapping_add(2), end.wrapping_sub(1));
        }
        if mem.read::<u8>(pad_record(mem, w).wrapping_add(2)) != was {
            return call(mem, CLICK, &[1]);
        }
        0
    }

    /// `TOFuncPadMessageInputValue(w)` (`0x008b62b0`), the `+0xfc` pad
    /// handler of a number entry; `r` is its line record
    /// (`0x01868a90[w]`: `+2` cursor column, `+3` its start, `+4` first and
    /// `+5` end column, `+0xc` the value, `+0x10` its start). The value's
    /// digits are taken first: digit `k` (1..=8) is `value / 10^k` of what
    /// the higher ones leave (`idiv` by the power table `0x00c5a040`, so
    /// the ones are never kept). Then by the repeated buttons, each with
    /// click 1:
    /// - 0x2000 moves the cursor right, 0x8000 left;
    /// - 0x1000 adds `10^k` at column `k = +5 - +2`, or takes `9 * 10^k`
    ///   when that digit is 9; 0x4000 takes `10^k`, or adds `9 * 10^k`
    ///   when it is 0.
    ///
    /// The cursor then wraps inside `+4..+5`; trigger 0x40 restores cursor
    /// and value (click 4), 0x20 sets `+1` to 2 (click 1).
    ///
    /// The digits live in a zeroed 16-byte frame buffer holding column `k`
    /// at byte `k - 1`: columns 9..=16 read its zeroed tail, and column 0 (the cursor moved past the
    /// last column by 0x2000 in the same frame) reads the byte below it, the
    /// top byte of the caller's saved `esi`, which in the game is the
    /// window index `TOControlMessageWindow(ForMenu)` loops over (0..8),
    /// so 0. Columns past 16 (the stack cookie and beyond) don't occur
    /// with a record of at most 9 columns; they read 0 here. A zero power
    /// in the table would fault the original; it divides as 0 here.
    pub fn func_pad_message_input_value<M: Mem>(
        mem: &mut M,
        w: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        const CLICK: u32 = 0x0088_6ba0;
        const POWERS: u32 = 0x00c5_a040;
        let power = |mem: &M, k: i32| -> i32 {
            mem.read(
                mem.rebase(POWERS)
                    .wrapping_add(k.wrapping_mul(4).cast_unsigned()),
            )
        };
        let first = pad_record(mem, w);
        let mut digits = [0u8; 17];
        let mut left: i32 = mem.read(first.wrapping_add(0xc));
        for (k, slot) in (1..9).zip(digits[1..=8].iter_mut()).rev() {
            let p = power(mem, k);
            *slot = left.checked_div(p).unwrap_or(0).to_le_bytes()[0];
            left = left.checked_rem(p).unwrap_or(0);
        }
        let digit = |k: i32| {
            usize::try_from(k)
                .ok()
                .and_then(|k| digits.get(k))
                .copied()
                .unwrap_or(0)
        };
        let column = |mem: &M, r: u32| {
            i32::from(mem.read::<i8>(r.wrapping_add(5)))
                - i32::from(mem.read::<i8>(r.wrapping_add(2)))
        };
        let mut repeat: u16 = mem.read(mem.rebase(PAD_REPEAT));
        if repeat & 0x2000 != 0 {
            let at = first.wrapping_add(2);
            mem.write(at, mem.read::<u8>(at).wrapping_add(1));
            call(mem, CLICK, &[1]);
            repeat = mem.read(mem.rebase(PAD_REPEAT));
        }
        if repeat & 0x8000 != 0 {
            let at = pad_record(mem, w).wrapping_add(2);
            mem.write(at, mem.read::<u8>(at).wrapping_sub(1));
            call(mem, CLICK, &[1]);
            repeat = mem.read(mem.rebase(PAD_REPEAT));
        }
        if repeat & 0x1000 != 0 {
            let r = pad_record(mem, w);
            let k = column(mem, r);
            let p = power(mem, k);
            let step = if digit(k) == 9 { p.wrapping_mul(-9) } else { p };
            let at = r.wrapping_add(0xc);
            mem.write(at, mem.read::<i32>(at).wrapping_add(step));
            call(mem, CLICK, &[1]);
            repeat = mem.read(mem.rebase(PAD_REPEAT));
        }
        if repeat & 0x4000 != 0 {
            let r = pad_record(mem, w);
            let k = column(mem, r);
            let p = power(mem, k);
            let step = if digit(k) == 0 {
                p.wrapping_mul(9)
            } else {
                p.wrapping_neg()
            };
            let at = r.wrapping_add(0xc);
            mem.write(at, mem.read::<i32>(at).wrapping_add(step));
            call(mem, CLICK, &[1]);
        }
        let r = pad_record(mem, w);
        let end = i32::from(mem.read::<i8>(r.wrapping_add(5)));
        let start = i32::from(mem.read::<i8>(r.wrapping_add(4)));
        let cursor = i32::from(mem.read::<i8>(r.wrapping_add(2)));
        let cursor = if cursor > end - 1 {
            start - end + cursor
        } else if cursor < start {
            cursor - start + end
        } else {
            cursor
        };
        mem.write(r.wrapping_add(2), cursor.to_le_bytes()[0]);
        let mut trigger: u16 = mem.read(mem.rebase(PAD_TRIGGER));
        if trigger & 0x40 != 0 {
            let r = pad_record(mem, w);
            mem.write(r.wrapping_add(2), mem.read::<u8>(r.wrapping_add(3)));
            let r = pad_record(mem, w);
            mem.write(r.wrapping_add(0xc), mem.read::<u32>(r.wrapping_add(0x10)));
            call(mem, CLICK, &[4]);
            trigger = mem.read(mem.rebase(PAD_TRIGGER));
        }
        if trigger & 0x20 != 0 {
            mem.write(pad_record(mem, w).wrapping_add(1), 2u8);
            call(mem, CLICK, &[1]);
        }
    }

    /// `TOBtlGetChangePad(w)` (`0x008a2160`): one frame of the battle
    /// change window `w` (the party list `+0x20` of `i16` ids, its length
    /// `i32 0x0134080c`, the row `i16 +0x42`). Unless both a down
    /// (0x4000/4) and an up (0x1000/1) button are held (`0x025d09d2`), the
    /// row moves down on repeat 0x4000 (`0x025d09d6`) or
    /// `inputIsRepButton(0x08000000)`, unless 0x4000 and 4 are both held,
    /// and up likewise (0x1000, `0x10000000`, held 0x1000 and 1); it wraps
    /// in the list. The row's id goes to `u16 +0x16 + 2 side` and the row to
    /// `+0xe + 2 side` (`side` = `i8 +0xb`), the help text follows
    /// (`TOBtlSetChangeHelpMes(id)`). A moved row clicks 1; else with
    /// Libra's auto mode (`0x0133d6e0` 1) the trigger word (`0x025d09d4`)
    /// is forced to 0x40, and then:
    /// - 0x40 at a deeper window level (`i8 0x023cc092` above 0):
    ///   `TOBtlSetPrevAction()`, `+1` stepped, click 4;
    /// - 0x20: `TOCheckBtlCommandUse(i16 +8, u16 +0x16)` negative clicks 3,
    ///   else `+1` stepped, `TOBtlSetNextAction(i16 +8, u16 +0x16)`,
    ///   click 1.
    ///
    /// Returns `SndPlayClick`'s `eax` (tail jumps); without a click the
    /// original's `eax` is left over, and its only caller,
    /// `TOWaitBtlWinChange`, drops it: 0 here.
    pub fn btl_get_change_pad<M: Mem>(
        mem: &mut M,
        w: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const REP_BUTTON: u32 = 0x0063_0d30;
        const CHANGE_HELP: u32 = 0x0089_97c0;
        const PREV_ACTION: u32 = 0x0089_a630;
        const CHECK_USE: u32 = 0x0089_aca0;
        const NEXT_ACTION: u32 = 0x0089_a330;
        const CLICK: u32 = 0x0088_6ba0;
        let held_at = mem.rebase(0x025d_09d2);
        let repeat_at = mem.rebase(PAD_REPEAT);
        let count_at = mem.rebase(0x0134_080c);
        let row_at = w.wrapping_add(0x42);
        let step = |mem: &mut M, by: i16| {
            mem.write(row_at, mem.read::<i16>(row_at).wrapping_add(by));
        };
        let was = mem.read::<i16>(row_at);
        let mut held: u16 = mem.read(held_at);
        if held & 0x4004 == 0 || held & 0x1001 == 0 {
            if !(held & 0x4000 != 0 && mem.read::<u8>(held_at) & 4 != 0) {
                if mem.read::<u16>(repeat_at) & 0x4000 != 0
                    || call(mem, REP_BUTTON, &[0x0800_0000]) != 0
                {
                    step(mem, 1);
                }
                held = mem.read(held_at);
            }
            if !(held & 0x1000 != 0 && mem.read::<u8>(held_at) & 1 != 0)
                && (mem.read::<u16>(repeat_at) & 0x1000 != 0
                    || call(mem, REP_BUTTON, &[0x1000_0000]) != 0)
            {
                step(mem, -1);
            }
        }
        let mut count: i32 = mem.read(count_at);
        if i32::from(mem.read::<i16>(row_at)) >= count {
            mem.write(row_at, 0u16);
            count = mem.read(count_at);
        }
        if mem.read::<i16>(row_at) < 0 {
            // lea eax, [ecx - 1]; mov [w + 0x42], ax
            let last = count.wrapping_sub(1).to_le_bytes();
            mem.write(row_at, u16::from_le_bytes([last[0], last[1]]));
        }
        // the row's entry of the party list
        let entry = |mem: &M| {
            let row = i32::from(mem.read::<i16>(row_at));
            let list: u32 = mem.read(w.wrapping_add(0x20));
            list.wrapping_add(row.wrapping_mul(2).cast_unsigned())
        };
        let side = |mem: &M| {
            i32::from(mem.read::<i8>(w.wrapping_add(0xb)))
                .wrapping_mul(2)
                .cast_unsigned()
        };
        let id: u16 = mem.read(entry(mem));
        mem.write(w.wrapping_add(0x16).wrapping_add(side(mem)), id);
        let row: u16 = mem.read(row_at);
        mem.write(w.wrapping_add(0xe).wrapping_add(side(mem)), row);
        let id = i32::from(mem.read::<i16>(entry(mem))).cast_unsigned();
        call(mem, CHANGE_HELP, &[id]);
        if mem.read::<i16>(row_at) != was {
            return call(mem, CLICK, &[1]);
        }
        let trigger_at = mem.rebase(PAD_TRIGGER);
        if mem.read::<u32>(mem.rebase(0x0133_d6e0)) == 1 {
            mem.write(trigger_at, 0x40u16);
        }
        let trigger: u8 = mem.read(trigger_at);
        let step_level = |mem: &mut M| {
            let at = w.wrapping_add(1);
            mem.write(at, mem.read::<u8>(at).wrapping_add(1));
        };
        if trigger & 0x40 != 0 {
            if mem.read::<i8>(mem.rebase(0x023c_c092)) <= 0 {
                return 0;
            }
            call(mem, PREV_ACTION, &[]);
            step_level(mem);
            return call(mem, CLICK, &[4]);
        }
        if trigger & 0x20 == 0 {
            return 0;
        }
        let args = |mem: &M| {
            [
                i32::from(mem.read::<i16>(w.wrapping_add(8))).cast_unsigned(),
                u32::from(mem.read::<u16>(w.wrapping_add(0x16))),
            ]
        };
        let a = args(mem);
        if call(mem, CHECK_USE, &a).cast_signed() < 0 {
            return call(mem, CLICK, &[3]);
        }
        step_level(mem);
        let a = args(mem);
        call(mem, NEXT_ACTION, &a);
        call(mem, CLICK, &[1])
    }

    /// `TOBtlGetLeftPad(w)` (`0x008a22f0`, `cancel` 0x2000) and
    /// `TOBtlGetRightPad(w)` (`0x008a2690`, `cancel` 0x8000): one frame of
    /// the battle command window `w` opened to that side. Libra's auto mode
    /// forces the trigger word (`0x025d09d4`) to `cancel`. Then, by the
    /// triggered buttons:
    /// - confirm (0x20): [`confirm_pick`], checking the current pick's
    ///   command.
    /// - 0x10: the character's skip command (`MsMenuGetChrSkipCommand`)
    ///   becomes its battle command (`0x023cc040`: `+0` actor, `+3` 1, `+8`
    ///   the command, `+0xa` 0xff, `+0x10` and `0x0133d178` the actor's
    ///   bit; `MsSetBtlCommand(record, 0, 0)`), the window and help closed
    ///   (`TOCloseBtlWindow(i16 0x023cc088)`, `TOBtlCloseHelpWin`,
    ///   `TOSeqInitAllScene(2)`), `0x023cc088` set to 0xff and click 1;
    ///   without one, click 3 and the buttons read again.
    /// - 0x04: with no summon out (`MsGetSaveChrSummon`)
    ///   `TOBtlSetNextAction(actor, 0x3002)` 1 clicks 1, else click 3.
    /// - `cancel`: [`undo_pick`], else at a deeper window level
    ///   `TOBtlSetPrevAction()`, `+1` stepped, click 1.
    /// - otherwise the repeated up (0x1000) and down (0x4000) move the
    ///   cursor a row ([`wrap_row`]), then [`follow_cursor`]; a moved cursor
    ///   sets `+0x1e`, records the pick again and clicks 1.
    pub fn to_btl_get_side_pad<M: Mem>(
        mem: &mut M,
        w: u32,
        cancel: u16,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        const CLICK: u32 = 0x0088_6ba0;
        let pad = mem.rebase(0x025d_09d2);
        if LIBRA_AUTO.get(mem) == 1 {
            mem.write(pad.wrapping_add(2), cancel);
        }
        let mut trigger: u8 = mem.read(pad.wrapping_add(2));
        let actor = i32::from(mem.read::<i16>(w.wrapping_add(8))).cast_unsigned();
        if trigger & 0x20 != 0 {
            let checked = u32::from(mem.read::<u16>(pick_slot(mem, w, 0x16)));
            confirm_pick(mem, w, checked, &mut call);
            return;
        }
        if trigger & 0x10 != 0 {
            #[allow(clippy::cast_possible_truncation)] // `movzx ecx, ax`
            let skip = call(mem, 0x0078_fb80, &[actor]) as u16;
            if skip != 0 {
                let rec = mem.rebase(0x023c_c040);
                let who: u8 = mem.read(w.wrapping_add(8));
                mem.write(rec, who);
                mem.write(rec.wrapping_add(3), 1u8);
                mem.write(rec.wrapping_add(8), skip);
                mem.write(rec.wrapping_add(0xa), 0xffu16);
                let bit = 1u32.wrapping_shl(u32::from(who));
                mem.write(rec.wrapping_add(0x10), bit);
                let sel = mem.rebase(0x0133_d178);
                mem.write(sel, bit);
                call(mem, 0x0079_2d60, &[rec, 0, 0]);
                let chr = mem.rebase(0x023c_c088);
                let current = i32::from(mem.read::<i16>(chr)).cast_unsigned();
                call(mem, 0x0089_ade0, &[current]);
                call(mem, 0x0089_0eb0, &[]);
                call(mem, 0x008a_e5a0, &[2]);
                mem.write(chr, 0xffu16);
                call(mem, CLICK, &[1]);
                return;
            }
            call(mem, CLICK, &[3]);
            trigger = mem.read(pad.wrapping_add(2));
        }
        if trigger & 4 != 0 {
            let summoned = call(mem, 0x0078_4ff0, &[actor]) != 0;
            let actor = i32::from(mem.read::<i16>(w.wrapping_add(8))).cast_unsigned();
            if !summoned && call(mem, 0x0089_a330, &[actor, 0x3002]) == 1 {
                call(mem, CLICK, &[1]);
            } else {
                call(mem, CLICK, &[3]);
            }
            return;
        }
        if mem.read::<u32>(pad.wrapping_add(2)) & u32::from(cancel) != 0 {
            if !undo_pick(mem, w, &mut call) && mem.read::<i8>(mem.rebase(0x023c_c092)) > 0 {
                call(mem, 0x0089_a630, &[]);
                let n = mem.read::<u8>(w.wrapping_add(1)).wrapping_add(1);
                mem.write(w.wrapping_add(1), n);
                call(mem, CLICK, &[1]);
            }
            return;
        }
        let at = w.wrapping_add(0x42);
        let old: i16 = mem.read(at);
        let step: i16 = mem.read(w.wrapping_add(0x38));
        let rep: u16 = mem.read(pad.wrapping_add(4));
        if rep & 0x1000 != 0 {
            mem.write(at, old.wrapping_sub(step));
        }
        let rep: u16 = mem.read(pad.wrapping_add(4));
        if rep & 0x4000 != 0 {
            let c: i16 = mem.read(at);
            mem.write(at, c.wrapping_add(step));
        }
        wrap_row(mem, w);
        let cursor = follow_cursor(mem, w, false);
        if cursor != old {
            let command: u16 = mem.read(cursor_entry(mem, w));
            mem.write(w.wrapping_add(0x1e), command);
            record_pick(mem, w);
            call(mem, CLICK, &[1]);
        }
    }

    /// [`to_btl_get_std_pad`]'s buttons before the d-pad (the skip
    /// command, 0x04, and the left and right windows); true when one of
    /// them ends the frame.
    #[allow(clippy::cast_possible_truncation)] // `movzx edi, ax`
    fn std_pad_shortcut<M: Mem>(
        mem: &mut M,
        w: u32,
        pad: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> bool {
        const CLICK: u32 = 0x0088_6ba0;
        const NEXT_ACTION: u32 = 0x0089_a330;
        let depth = |mem: &M| mem.read::<i8>(mem.rebase(0x023c_c092));
        let actor = |mem: &M| i32::from(mem.read::<i16>(w.wrapping_add(8))).cast_unsigned();
        let trigger = |mem: &M| mem.read::<u32>(pad.wrapping_add(2));
        if depth(mem) == 0 && trigger(mem) & 0x10 != 0 {
            let a = actor(mem);
            let skip = call(mem, 0x0078_fb80, &[a]) as u16;
            if skip != 0 {
                call(mem, CLICK, &[1]);
                let rec = mem.rebase(0x023c_c040);
                mem.write(rec.wrapping_add(8), skip);
                let who: u8 = mem.read(w.wrapping_add(8));
                mem.write(rec, who);
                mem.write(rec.wrapping_add(3), 1u8);
                mem.write(rec.wrapping_add(0xa), 0xffu16);
                let bit = |mem: &M| 1u32.wrapping_shl(u32::from(mem.read::<u8>(w.wrapping_add(8))));
                let b = bit(mem);
                mem.write(rec.wrapping_add(0x10), b);
                let b = bit(mem);
                mem.write(mem.rebase(0x0133_d178), b);
                call(mem, 0x0079_2d60, &[rec, 0, 0]);
                let chr = mem.rebase(0x023c_c088);
                let current = i32::from(mem.read::<i16>(chr)).cast_unsigned();
                call(mem, 0x0089_ade0, &[current]);
                mem.write(mem.rebase(0x0133_c93c), 0u8);
                call(mem, 0x008a_e5a0, &[2]);
                mem.write(chr, 0xffu16);
                return true;
            }
            call(mem, CLICK, &[3]);
        }
        if trigger(mem) & 4 != 0 {
            let ok = depth(mem) == 0
                && {
                    let a = actor(mem);
                    call(mem, 0x0078_4ff0, &[a]) == 0
                }
                && {
                    let a = actor(mem);
                    call(mem, NEXT_ACTION, &[a, 0x3002]) == 1
                };
            call(mem, CLICK, &[if ok { 1 } else { 3 }]);
            return true;
        }
        if depth(mem) == 0 {
            if trigger(mem) & 0x8000 != 0 {
                let a = actor(mem);
                if call(mem, 0x0078_f700, &[a]).cast_signed() > 0 {
                    let a = actor(mem);
                    call(mem, NEXT_ACTION, &[a, 0xffff]);
                    call(mem, CLICK, &[1]);
                    return true;
                }
            }
            if trigger(mem) & 0x2000 != 0 {
                mem.write(scratch, 0u32);
                let a = actor(mem);
                let list = call(mem, 0x0078_5410, &[a, 0xd, scratch]);
                let mut n: i32 = mem.read(scratch);
                // trailing empty (0xff) entries do not count
                while n > 0
                    && mem.read::<u16>(
                        list.wrapping_add(n.wrapping_mul(2).wrapping_sub(2).cast_unsigned()),
                    ) == 0xff
                {
                    n -= 1;
                    mem.write(scratch, n);
                }
                if n == 0 {
                    call(mem, CLICK, &[3]);
                } else {
                    let a = actor(mem);
                    call(mem, NEXT_ACTION, &[a, 0xfffe]);
                    call(mem, CLICK, &[1]);
                }
                return true;
            }
        }
        false
    }

    /// `TOBtlGetStdPad(w)` (`0x0089cde0`): one frame of the battle
    /// command list window `w` (items, skills: the list at `+0x20`, the
    /// picks at `+0xb`/`+0xa`) while the battle windows are open
    /// (`g_BtlWindowOpen`) and its step function (`+0x88`) is the idle
    /// [`SCROLL_DONE`]. By the trigger word (`0x025d09d4`):
    /// - 0x10 at the top window level (`g_BtlActionDepth` 0): the
    ///   character's skip command (`MsMenuGetChrSkipCommand`) clicks 1 and
    ///   becomes its battle command (`0x023cc040` as in
    ///   [`to_btl_get_side_pad`]), the window closed
    ///   (`TOCloseBtlWindow(i16 0x023cc088)`), the help window flag
    ///   (`g_BtlHelpWinOpen`) cleared, `TOSeqInitAllScene(2)`, `0x023cc088`
    ///   0xff; without one click 3 and go on.
    /// - 0x04: at the top level with no summon out
    ///   (`MsGetSaveChrSummon`) `TOBtlSetNextAction(actor, 0x3002)` 1
    ///   clicks 1; anything else clicks 3.
    /// - at the top level, 0x8000 with `MsCheckLeftWindow(actor)` above 0
    ///   goes to the left window (`TOBtlSetNextAction(actor, 0xffff)`,
    ///   click 1); 0x2000 goes to the right one (`0xfffe`, click 1) when
    ///   `MsGetSaveWindow(actor, 0xd, &n)` lists a command, trailing 0xff
    ///   entries not counted (`n` lives in `scratch`), else clicks 3.
    ///
    /// Otherwise the d-pad moves the cursor ([`dpad_move`]) and the pick
    /// is recorded. Libra's auto mode (`0x0133d6e0` 1) sets `0x0133d6e4`
    /// unless it or `g_BtlCommandMissed` is 1 already, and forces the
    /// trigger word to 0x40. Then cancel (0x40) undoes a pick
    /// ([`undo_pick`]) or at a deeper level goes back
    /// (`TOBtlSetPrevAction()`, `+1` stepped, click 4); confirm (0x20)
    /// checks the pick (`TOCheckBtlCommandUse`; refused: click 3 and
    /// `0x0133d6e4` set) and commits it ([`commit_pick`]); anything else
    /// keeps the cursor's row shown ([`follow_cursor`]) and a moved
    /// cursor sets `+0x1e`, records the pick again and clicks 1.
    pub fn to_btl_get_std_pad<M: Mem>(
        mem: &mut M,
        w: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        const CLICK: u32 = 0x0088_6ba0;
        let pad = mem.rebase(0x025d_09d2);
        if mem.read::<u32>(mem.rebase(0x023c_c08c)) == 0
            || mem.read::<u32>(w.wrapping_add(0x88)) != mem.rebase(SCROLL_DONE)
        {
            return;
        }
        if std_pad_shortcut(mem, w, pad, scratch, &mut call) {
            return;
        }
        let depth = |mem: &M| mem.read::<i8>(mem.rebase(0x023c_c092));
        let actor = |mem: &M| i32::from(mem.read::<i16>(w.wrapping_add(8))).cast_unsigned();
        let old: i16 = mem.read(w.wrapping_add(0x42));
        dpad_move(mem, w, pad);
        record_pick(mem, w);
        if LIBRA_AUTO.get(mem) == 1 {
            let (flag, missed) = (mem.rebase(0x0133_d6e4), mem.rebase(0x0133_d6ec));
            if mem.read::<u32>(flag) != 1 && mem.read::<u32>(missed) != 1 {
                mem.write(flag, 1u32);
            }
            MENU_PAD_TRG.set(mem, 0x40);
        }
        let trigger: u8 = mem.read(pad.wrapping_add(2));
        if trigger & 0x40 != 0 {
            if !undo_pick(mem, w, &mut call) && depth(mem) > 0 {
                call(mem, 0x0089_a630, &[]);
                let n = mem.read::<u8>(w.wrapping_add(1)).wrapping_add(1);
                mem.write(w.wrapping_add(1), n);
                call(mem, CLICK, &[4]);
            }
            return;
        }
        if trigger & 0x20 != 0 {
            let checked = u32::from(mem.read::<u16>(pick_slot(mem, w, 0x16)));
            let a = actor(mem);
            if call(mem, 0x0089_aca0, &[a, checked]).cast_signed() < 0 {
                call(mem, CLICK, &[3]);
                mem.write(mem.rebase(0x0133_d6e4), 1u32);
                return;
            }
            commit_pick(mem, w, &mut call);
            return;
        }
        let cursor = follow_cursor(mem, w, true);
        if cursor != old {
            let command: u16 = mem.read(cursor_entry(mem, w));
            mem.write(w.wrapping_add(0x1e), command);
            record_pick(mem, w);
            call(mem, CLICK, &[1]);
        }
    }

    /// `MenuItem_ApplyHeal_008c8c40(target)` (`0x008c8c40`, called by
    /// `TkMenuCtrlItem`): the item in use (`u16` at `0x0186a3a8`) heals.
    /// With the single-target flag (`0x0186a391`) at 1, `target` gets
    /// `TkMenuItemHealAmount_008c69a0(item, target, 0)` through
    /// `TkMenuAddHPCapped`; otherwise every aeon (with bit 0x80 of the
    /// item's `TkMsGetRomItem` record `+0x22`) and then every party member
    /// (bit 0x40) gets its own amount, through `TkMenuGetSummonListMax` /
    /// `FromIndex` and `TkMenuGetPlayerListMax` / `FromIndex2`. `scratch`
    /// receives the record's strings. Returns what the last call leaves in
    /// `eax` (the record when neither list is walked).
    pub fn menu_item_apply_heal<M: Mem>(
        mem: &mut M,
        target: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const AMOUNT: u32 = 0x008c_69a0;
        const ADD_HP: u32 = 0x008c_8d10;
        let item = |mem: &M| u32::from(mem.read::<u16>(mem.rebase(0x0186_a3a8)));
        let i = item(mem);
        let amount = call(mem, AMOUNT, &[i, target, 0]);
        if mem.read::<u8>(mem.rebase(0x0186_a391)) == 1 {
            return call(mem, ADD_HP, &[target, amount]);
        }
        let i = item(mem);
        let record = call(mem, 0x008a_b280, &[i, scratch]);
        let mut eax = record;
        for (bit, max, get) in [
            (0x80u8, 0x008a_9bf0, 0x008a_9bc0),
            (0x40, 0x008a_9b30, 0x008a_9ad0),
        ] {
            if mem.read::<u8>(record.wrapping_add(0x22)) & bit == 0 {
                continue;
            }
            let n = call(mem, max, &[]);
            eax = n;
            for k in 0..n.cast_signed().max(0) {
                let who = call(mem, get, &[k.cast_unsigned()]);
                let i = item(mem);
                let amount = call(mem, AMOUNT, &[i, who, 0]);
                eax = call(mem, ADD_HP, &[who, amount]);
            }
        }
        eax
    }

    /// `DrawWaterWaveShapeC2(x, y, w, h, u, v, uw, vh, su, sv, c0, c1)`
    /// (`0x008e7d30`): a quad of the water-wave sheet (shape 0x3e81,
    /// `TOGetShapTextureName`, size from `TOGetImageWH`) whose texture
    /// scrolls with time: `u = frac((graphicGetTime() × su + u) / sheet
    /// width)` and `v` the same with `sv`, `v` and the height (the time
    /// read twice, through `fcall`), then `u1 = u + uw / width`, `v1 = v +
    /// vh / height`. Each step is rounded to `f32` where the original
    /// stores; the fraction subtracts `_ftol2_sse`'s integer as an `f32`.
    /// The record (`x, y, u, v`, the bytes of `c0`; `x + w, y + h, u1, v1`,
    /// the bytes of `c1`; `+0x90`, `+0x94` 0) goes to
    /// `graphicDrawUIElement(record, texture, 1, 0, 0)`.
    ///
    /// `scratch` (0xa0 bytes) stands for the original's frame: the sheet
    /// size at `+0`, the record at `+8`; the record's other words are
    /// left as they are (stack garbage in the original). Returns what the
    /// draw call leaves in `eax`.
    #[allow(clippy::many_single_char_names)] // x, y, w, h, u, v: the quad
    pub fn draw_water_wave_shape_c2<M: Mem>(
        mem: &mut M,
        args: [u32; 12],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, ftol, load, mul, round_trip, store, sub};
        const SHAPE: u32 = 0x3e81;
        let [x, y, w, h, u, v, uw, vh, su, sv, c0, c1] = args;
        let rec = scratch.wrapping_add(8);
        let field = |off: u32| rec.wrapping_add(off);
        mem.write(field(0x90), 0u32);
        mem.write(field(0x94), 0u32);
        let texture = call(mem, 0x008a_c8c0, &[SHAPE]);
        call(mem, 0x008a_c400, &[SHAPE, scratch, scratch.wrapping_add(4)]);
        let sheet_w: u32 = mem.read(scratch);
        let sheet_h: u32 = mem.read(scratch.wrapping_add(4));
        mem.write(field(0), round_trip(f32::from_bits(x)));
        mem.write(field(4), round_trip(f32::from_bits(y)));
        mem.write(field(0x20), store(add(load(x), load(w))));
        mem.write(field(0x24), store(add(load(y), load(h))));
        // the scrolled coordinate's fraction of the sheet
        let mut scroll = |mem: &mut M, start: u32, speed: u32, size: u32| -> u32 {
            let t = fcall(mem, 0x0064_1410, &[]);
            let s = store(add(mul(t, load(speed)), load(start)));
            let s = load(store(div(load(s), load(size))));
            let whole = ftol(mem, s).cast_signed();
            #[allow(clippy::cast_precision_loss)] // fild then fstp dword
            let whole = f64::from(whole as f32);
            store(sub(s, whole))
        };
        let fu = scroll(mem, u, su, sheet_w);
        let fv = scroll(mem, v, sv, sheet_h);
        mem.write(field(8), fu);
        mem.write(field(0xc), fv);
        mem.write(
            field(0x28),
            store(add(load(fu), div(load(uw), load(sheet_w)))),
        );
        mem.write(
            field(0x2c),
            store(add(load(fv), div(load(vh), load(sheet_h)))),
        );
        for (base, c) in [(0x10u32, c0), (0x30, c1)] {
            for (k, b) in (0u32..).zip(c.to_le_bytes()) {
                mem.write(field(base + 4 * k), u32::from(b));
            }
        }
        call(mem, 0x0063_eee0, &[rec, texture, 1, 0, 0])
    }

    /// `QuickRecoverMenuDraw`'s per-slot scroll state: offset (`f32`,
    /// `0x01876110`), text hash (`0x01876120`), phase (`u8`, `0x01871708`)
    /// and frame count (`0x01876130`), four slots.
    pub const QR_SCROLL_OFFSET: u32 = 0x0187_6110;

    /// `QuickRecoverMenuDraw(text, x, y, w, alpha, scale, extra, align,
    /// scroll, slot)` (`0x00904ba0`, from `TOBtlDrawCommandWindow`): a
    /// battle-font label fitted into width `w`, drawn with
    /// `ToMakeBtlEasyEdgeFont(text, x, y, alpha, scale, extra)`. The text's
    /// width comes from `ToGetBtlEasyFontWidth`. `align` 1 centres it at
    /// `x` (by `w` when wider), 0 starts it at `x`; any other value leaves
    /// the left edge unset (stack garbage in the original, the word at
    /// `scratch + 0xc` here).
    ///
    /// A text that fits is drawn as is. A wider one:
    /// - with `scroll` 1 marquees in slot `min(slot, 3)`: a new text (its
    ///   xor hash's low nibble differs) restarts it; it waits 51 frames,
    ///   slides left 0.5 a frame until its end reaches the box's, waits 51
    ///   frames and starts over. It is drawn twice (10 apart) inside
    ///   `OpenFontScissor(left, y - 2, left + w, y + 20)`.
    /// - otherwise is cut to fit with an ellipsis (`"HHH"` for languages
    ///   1..=8, else `">"`): glyph by glyph through the TO font writer
    ///   (`g_TOFontCursor`, `0x025d09a0`: colour 0x80808080, `+4` x, `+8`
    ///   y, `+0x1b` alpha), group 1 codes starting a new line 16 lower at
    ///   `x`. A glyph that would pass `left + w - ellipsis width` ends the
    ///   text: a `G` there (0x47) is first narrowed by the `G` glyph's width
    ///   until it fits and drawn. The ellipsis follows at the cursor.
    ///
    /// Sums are doubles of the `f32` values, rounded where the original
    /// stores. `scratch` (0x50 bytes) stands for the original's frame: text
    /// width `+0`, ellipsis width `+4`, ellipsis text `+8`, left edge or
    /// `G` code `+0xc`, glyph record `+0x10`. Float results come through
    /// `fcall`.
    #[allow(
        clippy::too_many_lines,
        clippy::many_single_char_names,
        clippy::neg_cmp_op_on_partial_ord // the x87 tests: NaN takes the negated side
    )]
    pub fn quick_recover_menu_draw<M: Mem>(
        mem: &mut M,
        args: [u32; 10],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use ffx_bridge::x87::{add, load, mul, round_trip, store, sub};
        const LANG: u32 = 0x008a_c2f0;
        const EDGE_FONT: u32 = 0x0090_5930;
        const FONT_INFO: u32 = 0x008b_7240;
        const ADVANCE: u32 = 0x008f_a740;
        let rt = |v: u32| round_trip(f32::from_bits(v)).to_bits();
        let d = |mem: &M, a: u32| -> f64 { mem.read(mem.rebase(a)) };
        let [text, x, y, w, alpha, scale, extra, align, scroll, slot] = args;
        let (scale, extra) = (rt(scale), rt(extra));
        let (lw_at, ew_at, ellipsis, left_at, info) = (
            scratch,
            scratch.wrapping_add(4),
            scratch.wrapping_add(8),
            scratch.wrapping_add(0xc),
            scratch.wrapping_add(0x10),
        );
        let glyph = mem.rebase(0x0184_1d50);
        for k in 0..0x10 {
            mem.write(glyph.wrapping_add(4 * k), 0u32);
        }
        let pen = mem.rebase(0x025d_09a0);
        for k in 0..7 {
            mem.write(pen.wrapping_add(4 * k), 0u32);
        }
        mem.write(ellipsis, 0u32);
        let lang = call(mem, LANG, &[]).cast_signed();
        if lang > 0 && call(mem, LANG, &[]).cast_signed() < 9 {
            mem.write(ellipsis, [0x48u8, 0x48, 0x48]);
        } else {
            mem.write(ellipsis, 0x3eu8);
        }
        call(mem, 0x0090_5290, &[text, lw_at, alpha, scale, extra]);
        let lw = f64::from(mem.read::<f32>(lw_at));
        let (fx, fw) = (load(x), load(w));
        let half = d(mem, 0x00b9_2220);
        match align {
            1 => {
                let span = if lw > fw { fw } else { lw };
                mem.write(left_at, store(sub(fx, mul(span, half))));
            }
            0 => mem.write(left_at, rt(x)),
            _ => {}
        }
        let left = rt(mem.read(left_at));
        let draw = |mem: &mut M,
                    call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
                    t: u32,
                    x: u32,
                    y: u32| {
            call(mem, EDGE_FONT, &[t, x, y, alpha, scale, extra]);
        };
        if scroll == 1 {
            let slot = if slot.cast_signed() >= 4 { 3 } else { slot };
            let offset = mem
                .rebase(QR_SCROLL_OFFSET)
                .wrapping_add(slot.wrapping_mul(4));
            let hash = mem.rebase(0x0187_6120).wrapping_add(slot.wrapping_mul(4));
            let phase = mem.rebase(0x0187_1708).wrapping_add(slot);
            let count = mem.rebase(0x0187_6130).wrapping_add(slot.wrapping_mul(4));
            let mut h = 0u8;
            let mut p = text;
            loop {
                let c: u8 = mem.read(p);
                if c == 0 {
                    break;
                }
                h ^= c;
                p = p.wrapping_add(1);
            }
            let h = u32::from(h & 0xf);
            if mem.read::<u32>(hash) != h {
                mem.write(offset, 0f32);
                mem.write(phase, 0u8);
                mem.write(count, 0u32);
                mem.write(hash, h);
            }
            if !(lw > fw) {
                draw(mem, &mut call, text, left, rt(y));
                return;
            }
            let tick = |mem: &mut M| -> u32 {
                let n = mem.read::<u32>(count).wrapping_add(1);
                mem.write(count, n);
                n
            };
            match mem.read::<i8>(phase) {
                0 => {
                    let n = tick(mem);
                    mem.write(offset, 0f32);
                    if n > 50 {
                        mem.write(phase, 1u8);
                        mem.write(count, 0u32);
                    }
                }
                1 => {
                    let o = store(sub(load(mem.read(offset)), half));
                    mem.write(offset, o);
                    let room = sub(fw, lw);
                    if load(o) <= room {
                        mem.write(offset, store(room));
                        mem.write(phase, 2u8);
                    }
                }
                2 => {
                    if tick(mem) > 50 {
                        mem.write(phase, 0u8);
                        mem.write(count, 0u32);
                    }
                }
                _ => {
                    mem.write(offset, 0f32);
                    mem.write(phase, 0u8);
                    mem.write(count, 0u32);
                }
            }
            let (fy, fl) = (load(y), load(left));
            let bottom = store(add(d(mem, 0x00b4_9968), fy));
            let right = store(add(fw, fl));
            let top = store(sub(fy, d(mem, 0x00b9_2270)));
            call(mem, 0x008e_8940, &[left, top, right, bottom]);
            let first = add(load(mem.read(offset)), fl);
            draw(mem, &mut call, text, store(first), rt(y));
            let second = add(
                add(add(load(mem.read(offset)), fl), lw),
                d(mem, 0x00b9_22b0),
            );
            draw(mem, &mut call, text, store(second), rt(y));
            call(mem, 0x008e_5cd0, &[]);
            return;
        }
        if lw <= fw {
            draw(mem, &mut call, text, left, rt(y));
            return;
        }
        // cut to fit, with the ellipsis
        call(mem, 0x0090_5290, &[ellipsis, ew_at, alpha, scale, extra]);
        let fl = load(left);
        mem.write(pen, 0x8080_8080u32);
        mem.write(pen.wrapping_add(0x1a), 0u8);
        mem.write(pen.wrapping_add(0x1b), alpha.to_le_bytes()[0]);
        let ew = f64::from(mem.read::<f32>(ew_at));
        let limit = load(store(sub(add(fl, fw), ew)));
        mem.write(pen.wrapping_add(4), left);
        mem.write(pen.wrapping_add(8), rt(y));
        let pen_x = |mem: &M| f64::from(mem.read::<f32>(pen.wrapping_add(4)));
        let emit = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            call(mem, 0x0064_3af0, &[1]);
            call(mem, 0x0064_0dd0, &[0]);
            call(mem, 0x008f_ae40, &[pen, glyph, scale]);
            call(mem, 0x0064_0dd0, &[1]);
            call(mem, 0x008f_b940, &[pen, glyph, scale]);
            call(mem, 0x0064_3af0, &[0]);
            call(mem, 0x008e_8ed0, &[pen, glyph, scale]);
        };
        let mut p = text;
        loop {
            let c: u8 = mem.read(p);
            if c == 0 {
                return;
            }
            match call(mem, 0x008d_f610, &[u32::from(c)]) {
                1 => {
                    mem.write(pen.wrapping_add(4), rt(x));
                    let row: f32 = mem.read(pen.wrapping_add(8));
                    let row = store(add(f64::from(row), d(mem, 0x00b3_13a0)));
                    mem.write(pen.wrapping_add(8), row);
                }
                4 => {
                    call(mem, FONT_INFO, &[p, glyph]);
                    let advance = fcall(mem, ADVANCE, &[0, glyph, scale]);
                    if limit < add(advance, pen_x(mem)) {
                        break;
                    }
                    emit(mem, &mut call);
                }
                _ => {}
            }
            p = call(mem, 0x008d_fd80, &[p]);
        }
        if mem.read::<u8>(p) == 0x47 {
            mem.write(left_at, 0x47u16);
            call(mem, FONT_INFO, &[left_at, info]);
            let narrow = glyph.wrapping_add(0x1c);
            loop {
                let advance = fcall(mem, ADVANCE, &[0, glyph, scale]);
                if !(add(advance, pen_x(mem)) > limit) {
                    break;
                }
                let g = load(mem.read(narrow));
                let by = load(mem.read(info.wrapping_add(0x1c)));
                mem.write(narrow, store(sub(g, by)));
            }
            emit(mem, &mut call);
        }
        let at: u32 = mem.read(pen.wrapping_add(4));
        let row: u32 = mem.read(pen.wrapping_add(8));
        draw(mem, &mut call, ellipsis, rt(at), rt(row));
    }

    /// `0x008df570(w)`: a text window's scroll clock. While `+0xb4` counts
    /// down nothing else happens; at 0 the fraction `+0xb0` gains
    /// `Tk_GetScale(MsGetSaveConfigEnglish()) × +0xac` (the step as an
    /// `f32`, the sum at the x87's 53 bits, truncated), and every whole 30
    /// (25 in the 50 Hz video mode, `TOGetFFXVideoMode` non-zero) moves
    /// into the line count `+0xa0`. Returns what the original leaves in
    /// `eax` (the lines added, the fraction, or the countdown).
    pub fn text_scroll_tick<M: Mem>(
        mem: &mut M,
        w: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87;
        let countdown: u32 = mem.read(w.wrapping_add(0xb4));
        if countdown != 0 {
            let n = countdown.wrapping_sub(1);
            mem.write(w.wrapping_add(0xb4), n);
            return n;
        }
        let english = call(mem, 0x0078_51d0, &[]);
        let scale = fcall(mem, 0x0087_2360, &[english]);
        let step = x87::store(f64::from(mem.read::<i32>(w.wrapping_add(0xac))));
        let fraction = f64::from(mem.read::<i32>(w.wrapping_add(0xb0)));
        let sum = x87::add(x87::mul(scale, x87::load(step)), fraction);
        let fraction = x87::ftol(mem, sum);
        mem.write(w.wrapping_add(0xb0), fraction);
        let per_line = if call(mem, 0x008a_c3a0, &[]) == 0 {
            30
        } else {
            25
        };
        let fraction = fraction.cast_signed();
        if fraction < per_line {
            return fraction.cast_unsigned();
        }
        let lines: i32 = mem.read(w.wrapping_add(0xa0));
        let more = fraction / per_line;
        mem.write(w.wrapping_add(0xa0), lines.wrapping_add(more));
        mem.write(w.wrapping_add(0xb0), fraction % per_line);
        more.cast_unsigned()
    }
    /// `TOMkpShapeXYC2(id, x, y, c0, c1)` (`0x00903a50`).
    pub const SHAPE_XY_C2: u32 = 0x0090_3a50;

    /// `TOMkpShapeXYC2(id, x, y, rgba, rgba)`.
    fn shape_xy_c2<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        [x, y, rgba]: [u32; 3],
        id: u32,
    ) {
        call(mem, SHAPE_XY_C2, &[id, x, y, rgba, rgba]);
    }

    /// `TOAdpButtonIconRGBA_008e99a0(cursor, c, x, y, r, g, b, a)`: sets the
    /// packet cursor ([`crate::pkt::bridge::PACKET_CURSOR`]), then draws the
    /// icon for code `c` at `(x, y)` as `TOMkpShapeXYC2` shapes in the
    /// colour of the four low bytes (both corners): `@`..`O` (the d-pad)
    /// one shape per direction bit (0x258 or 0x259 for bit 1, 0x25a/0x25b
    /// bit 2, 0x25c/0x25d bit 4, 0x25e/0x25f bit 8, the second when set);
    /// the digits `c + 0x230`, except `1`/`2` with the confirm button
    /// swapped (`TOGetSystemConfirmButtonAssign() == 1`: 0x262/0x261,
    /// returning at once); `` ` `` 0x270. Returns the packet cursor.
    #[allow(clippy::many_single_char_names)] // the original's arguments
    pub fn button_icon_rgba<M: Mem>(
        mem: &mut M,
        args: [u32; 8],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let [cursor, mut c, x, y, r, g, b, a] = args;
        let cursor_at = crate::pkt::bridge::PACKET_CURSOR;
        cursor_at.set(mem, cursor);
        let rgba = (a & 0xff) << 24 | (b & 0xff) << 16 | (g & 0xff) << 8 | (r & 0xff);
        if c.wrapping_sub(0x40) <= 0xf {
            c &= 0xf;
            for (bit, off, on) in [
                (1, 0x258, 0x259),
                (2, 0x25a, 0x25b),
                (4, 0x25c, 0x25d),
                (8, 0x25e, 0x25f),
            ] {
                shape_xy_c2(
                    mem,
                    &mut call,
                    [x, y, rgba],
                    if c & bit != 0 { on } else { off },
                );
            }
        }
        if c.wrapping_sub(0x30) <= 9 {
            if call(mem, 0x008a_cf30, &[]) == 1 && (0x31..=0x32).contains(&c) {
                shape_xy_c2(
                    mem,
                    &mut call,
                    [x, y, rgba],
                    if c == 0x31 { 0x262 } else { 0x261 },
                );
                return cursor_at.get(mem);
            }
            shape_xy_c2(mem, &mut call, [x, y, rgba], c + 0x230);
        }
        if c == 0x60 {
            shape_xy_c2(mem, &mut call, [x, y, rgba], 0x270);
        }
        cursor_at.get(mem)
    }

    /// `TOAdpButtonIcon_008e9970(cursor, c, x, y)`:
    /// [`button_icon_rgba`] in grey (`0x80` each).
    pub fn button_icon<M: Mem>(
        mem: &mut M,
        [cursor, c, x, y]: [u32; 4],
        call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        button_icon_rgba(mem, [cursor, c, x, y, 0x80, 0x80, 0x80, 0x80], call)
    }

    /// The scratch [`set_draw_info`] hands `TOGetMessageWH`: the width and
    /// height it writes (`[ebp-8]`, `[ebp+8]`), 8 bytes.
    pub const SET_DRAW_INFO_SCRATCH: u32 = 8;

    /// `TOSetDrawInfo(code, window, pen)` (`0x008df8f0`): applies the
    /// control code at `code` (`code[0]`, its argument `n = code[1]`, read
    /// again at each test) to message window `window` and pen `pen` (`+4`
    /// x, `+8` y, `+0x1a` the icon mode byte, `+0x1b` the colour nibbles):
    /// - 7: `n` 0xe0..=0xef moves the pen right by `n - 0xe0`. Otherwise
    ///   `TOGetMessageWH(window +4, code + 2, &w, &h, window +0x1a, 0x103)`
    ///   (the two `i16` sign-extended; `w` starts at 0.0 and `h` holds
    ///   `code`, in `scratch`), then `'8'` (with the `u16` at
    ///   `window + 0x24` 0) and `'1'` run
    ///   `TOFirstInitMessageNormal(window, [window + 0x8c] + 2, pen)`,
    ///   `'2'` puts the pen's x at `window + 0x50 - w` and 0x40..=0xbf at
    ///   `window + 0x48 + (4 n - 0x100)`.
    /// - 9: `'0'` zeroes `window + 0xac`, another `n` below 0xc0 sets it to
    ///   `n - '0'`; `n` from 0xc0 sets `window + 0xb4` to `n - 0xc0`.
    /// - 0xa: a non-zero low nibble `l` puts `l - 1` in the colour byte's
    ///   low nibble; a high nibble from 4 puts `(n & 0xf0) - 0x40` in its
    ///   high one.
    /// - 0xb: button icon `n` (`setButtonData(n, -1)` first,
    ///   `resetButtonData` last). Icon modes 1 and 3 draw it at scale 0.73
    ///   inside `graphicSetButtonIconScale(1.15, 1.15)` /
    ///   `graphicResetButtonIconScale`, at `y + 5.5 + AdjustButtonIconY`.
    ///   Otherwise at 0.65: language 10 at `y + 3.0 + AdjustButtonIconY`
    ///   (then, if the language is also 11 on map 0xb8, the pen moves 8);
    ///   language 9 with `graphicGetAbmap() == 1` at `y + 1.0` without the
    ///   adjustment; else at `y (+ 2.0 for '4'..='7') + 3.0 +
    ///   AdjustButtonIconY` (then on map 0xb8 the pen moves 7). After a
    ///   move, `'8'` on map 0xb8 takes 1.0 back. Then `window + 0x9c`
    ///   counts the icon and, while `window + 0xac` is 0, `window + 0xa0`
    ///   steps up to `window + 0xa8` (signed); the pen moves by
    ///   `TOGetScaleButtonIconWidth(n, scale)` plus 1.5 (modes 1, 3, read
    ///   again) or 1.0.
    /// - 0xe: `'0'..='3'` set the icon mode to `n - '0'`; mode 3 with `'A'`
    ///   moves the pen 5.0; `'@'` sets mode 3 and `'A'` mode 1.
    /// - 8, 0xc, 0xd and anything else: nothing.
    ///
    /// Sums are formed at the x87's 53 bits in the original's order and
    /// stored as `f32`. `call` and `fcall` take `ffx.sym` addresses.
    #[allow(clippy::too_many_lines)]
    pub fn set_draw_info<M: Mem>(
        mem: &mut M,
        [code, window, pen]: [u32; 3],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use ffx_bridge::x87;
        const MESSAGE_WH: u32 = 0x008b_9650;
        const FIRST_INIT_MESSAGE: u32 = 0x008d_f520;
        const SET_BUTTON: u32 = 0x008b_1350;
        const RESET_BUTTON: u32 = 0x008b_1330;
        const ADJUST_ICON_Y: u32 = 0x008e_5280;
        const DRAW_ICON: u32 = 0x008f_8ee0;
        const ICON_WIDTH: u32 = 0x008f_a800;
        const LANG: u32 = 0x008a_c2f0;
        const GET_ABMAP: u32 = 0x0064_1060;
        const SET_ICON_SCALE: u32 = 0x0064_2ec0;
        const RESET_ICON_SCALE: u32 = 0x0064_2c60;
        const CURRENT_MAP: u32 = 0x012f_bc40;
        const SCALE: f32 = 0.65;
        const SCALE_BIG: f32 = 0.73;
        let arg = |mem: &M| mem.read::<u8>(code.wrapping_add(1));
        let store = |v: f64| f32::from_bits(x87::store(v));
        let pen_x = pen.wrapping_add(4);
        let pen_y = pen.wrapping_add(8);
        let mode = pen.wrapping_add(0x1a);
        let f = |mem: &M, at: u32| f64::from(mem.read::<f32>(at));
        match mem.read::<u8>(code) {
            7 => {
                let n = arg(mem);
                if (0xe0..0xf0).contains(&n) {
                    let x = x87::add(f64::from(n - 0xe0), f(mem, pen_x));
                    mem.write(pen_x, store(x));
                    return;
                }
                mem.write(scratch, 0.0f32);
                mem.write(scratch.wrapping_add(4), code);
                let lines = i32::from(mem.read::<i16>(window.wrapping_add(4)));
                let width = i32::from(mem.read::<i16>(window.wrapping_add(0x1a)));
                let args = [
                    lines.cast_unsigned(),
                    code.wrapping_add(2),
                    scratch,
                    scratch.wrapping_add(4),
                    width.cast_unsigned(),
                    0x103,
                ];
                call(mem, MESSAGE_WH, &args);
                for (c, needs_zero) in [(b'8', true), (b'1', false)] {
                    if arg(mem) == c
                        && (!needs_zero || mem.read::<u16>(window.wrapping_add(0x24)) == 0)
                    {
                        let line = mem.read::<u32>(window.wrapping_add(0x8c)).wrapping_add(2);
                        call(mem, FIRST_INIT_MESSAGE, &[window, line, pen]);
                    }
                }
                if arg(mem) == b'2' {
                    let x = x87::sub(f(mem, window.wrapping_add(0x50)), f(mem, scratch));
                    mem.write(pen_x, store(x));
                }
                let n = arg(mem);
                if (0x40..0xc0).contains(&n) {
                    let step = f64::from(u32::from(n) * 4 - 0x100);
                    let x = x87::add(step, f(mem, window.wrapping_add(0x48)));
                    mem.write(pen_x, store(x));
                }
            }
            9 => {
                let wait = window.wrapping_add(0xac);
                if arg(mem) == b'0' {
                    mem.write(wait, 0u32);
                }
                let n = arg(mem);
                if n < 0xc0 && n != b'0' {
                    mem.write(wait, u32::from(n).wrapping_sub(0x30));
                }
                let n = arg(mem);
                if n >= 0xc0 {
                    mem.write(window.wrapping_add(0xb4), u32::from(n - 0xc0));
                }
            }
            0xa => {
                let colour = pen.wrapping_add(0x1b);
                let low = arg(mem) & 0xf;
                if low != 0 {
                    let c = mem.read::<u8>(colour) & 0xf0 | (low - 1);
                    mem.write(colour, c);
                }
                let high = arg(mem) & 0xf0;
                if high >= 0x40 {
                    let c = mem.read::<u8>(colour) & 0xf | (high - 0x40);
                    mem.write(colour, c);
                }
            }
            0xb => {
                let n = u32::from(arg(mem));
                call(mem, SET_BUTTON, &[n, u32::MAX]);
                let icon = |mem: &mut M,
                            call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
                            y: f32,
                            scale: f32| {
                    let x: u32 = mem.read(pen_x);
                    let n = u32::from(mem.read::<u8>(code.wrapping_add(1)));
                    let args = [n, x, y.to_bits(), scale.to_bits(), 0x80, 0x80, 0x80, 0x80];
                    call(mem, DRAW_ICON, &args);
                };
                let big = matches!(mem.read::<u8>(mode), 1 | 3);
                if big {
                    call(mem, SET_ICON_SCALE, &[1.15f32.to_bits(), 1.15f32.to_bits()]);
                    let lift = fcall(
                        mem,
                        ADJUST_ICON_Y,
                        &[SCALE_BIG.to_bits(), 0x80, 0x80, 0x80, 0x80],
                    );
                    let y = store(x87::add(x87::add(f(mem, pen_y), 5.5), lift));
                    icon(mem, &mut call, y, SCALE_BIG);
                    call(mem, RESET_ICON_SCALE, &[]);
                } else {
                    let map = |mem: &M| mem.read::<u32>(mem.rebase(CURRENT_MAP));
                    let moved = if call(mem, LANG, &[]) == 10 {
                        let lift = fcall(
                            mem,
                            ADJUST_ICON_Y,
                            &[SCALE.to_bits(), 0x80, 0x80, 0x80, 0x80],
                        );
                        let y = store(x87::add(x87::add(f(mem, pen_y), 3.0), lift));
                        icon(mem, &mut call, y, SCALE);
                        (call(mem, LANG, &[]) == 0xb && map(mem) == 0xb8).then_some(8.0)
                    } else {
                        if call(mem, LANG, &[]) == 9 && call(mem, GET_ABMAP, &[]) == 1 {
                            let y = store(x87::add(f(mem, pen_y), 1.0));
                            icon(mem, &mut call, y, SCALE);
                        } else {
                            let mut y = mem.read::<f32>(pen_y);
                            if (0x34..=0x37).contains(&arg(mem)) {
                                y = store(x87::add(f64::from(y), 2.0));
                            }
                            let lift = fcall(
                                mem,
                                ADJUST_ICON_Y,
                                &[SCALE.to_bits(), 0x80, 0x80, 0x80, 0x80],
                            );
                            let y = store(x87::add(x87::add(f64::from(y), 3.0), lift));
                            icon(mem, &mut call, y, SCALE);
                        }
                        (map(mem) == 0xb8).then_some(7.0)
                    };
                    if let Some(step) = moved {
                        let x = store(x87::add(f(mem, pen_x), step));
                        mem.write(pen_x, x);
                        if map(mem) == 0xb8 && arg(mem) == b'8' {
                            mem.write(pen_x, store(x87::sub(f64::from(x), 1.0)));
                        }
                    }
                }
                let count = window.wrapping_add(0x9c);
                let v = mem.read::<u32>(count).wrapping_add(1);
                mem.write(count, v);
                if mem.read::<u32>(window.wrapping_add(0xac)) == 0 {
                    let shown = window.wrapping_add(0xa0);
                    let v = mem.read::<i32>(shown);
                    if v < mem.read::<i32>(window.wrapping_add(0xa8)) {
                        mem.write(shown, v.wrapping_add(1));
                    }
                }
                let n = u32::from(arg(mem));
                let x = if matches!(mem.read::<u8>(mode), 1 | 3) {
                    let w = fcall(mem, ICON_WIDTH, &[n, SCALE_BIG.to_bits()]);
                    x87::add(x87::add(w, 1.5), f(mem, pen_x))
                } else {
                    let w = fcall(mem, ICON_WIDTH, &[n, SCALE.to_bits()]);
                    x87::add(x87::add(w, 1.0), f(mem, pen_x))
                };
                mem.write(pen_x, store(x));
                call(mem, RESET_BUTTON, &[]);
            }
            0xe => {
                let n = arg(mem);
                if (0x30..=0x33).contains(&n) {
                    mem.write(mode, n - 0x30);
                }
                if mem.read::<u8>(mode) == 3 && arg(mem) == b'A' {
                    mem.write(pen_x, store(x87::add(f(mem, pen_x), 5.0)));
                }
                if arg(mem) == b'@' {
                    mem.write(mode, 3u8);
                }
                if arg(mem) == b'A' {
                    mem.write(mode, 1u8);
                }
            }
            _ => {}
        }
    }

    /// `MsGetComData(id, &strings)` (battle module) and
    /// `TOBtlOpenSimpleHelpMes(i, message)`.
    pub const MS_GET_COM_DATA: u32 = 0x0079_a4b0;
    pub const OPEN_SIMPLE_HELP: u32 = 0x0089_dba0;
    /// `TOGetFontWidthInterMode(text, font, &width)`.
    pub const FONT_WIDTH_INTER_MODE: u32 = 0x008b_7be0;

    /// `TOBtlOpenSignWindow(id)` (`0x00898c60`): when command `id`'s
    /// record (`MsGetComData(id as u16, &strings)`, `strings` in
    /// `scratch`) has bit 2 of its `+0x1c`, opens simple help window 1
    /// on its name (`u16 +0` + `strings`) and returns 1; else 0.
    pub fn btl_open_sign_window<M: Mem>(
        mem: &mut M,
        id: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let record = call(mem, MS_GET_COM_DATA, &[id & 0xffff, scratch]);
        if mem.read::<u8>(record.wrapping_add(0x1c)) & 4 == 0 {
            return 0;
        }
        let strings: u32 = mem.read(scratch);
        let name = u32::from(mem.read::<u16>(record)).wrapping_add(strings);
        call(mem, OPEN_SIMPLE_HELP, &[1, name]);
        1
    }

    /// `TOGetTextWidth_008b8040(text)`: the sum of the words
    /// `TOGetFontWidthInterMode(p, 0, &w)` writes while it steps `p`
    /// through the text (until a NUL), added as integers (the callee's
    /// width is an `f32`, as for [`text_width`]; the original adds its
    /// bits). `w` lives in `scratch`, which
    /// starts as `text` (the original's argument slot) and keeps its last
    /// value when a call leaves it.
    pub fn text_width_inter_mode<M: Mem>(
        mem: &mut M,
        text: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        mem.write(scratch, text);
        let mut sum = 0u32;
        let mut p = text;
        while mem.read::<u8>(p) != 0 {
            p = call(mem, FONT_WIDTH_INTER_MODE, &[p, 0, scratch]);
            sum = sum.wrapping_add(mem.read(scratch));
        }
        sum
    }

    /// `TOGetColonWidth_008dfe20(font)`: the width
    /// `TOGetFontWidthInterMode(":", font, &w)` gives (`w` from 0.0, an
    /// `f32`), truncated by `ftol2_sse`. `scratch` holds the `':'` (byte
    /// `+0`) and `w` (`+4`).
    pub fn colon_width<M: Mem>(
        mem: &mut M,
        font: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let width = scratch.wrapping_add(4);
        mem.write(width, 0.0f32);
        mem.write(scratch, b':');
        call(mem, FONT_WIDTH_INTER_MODE, &[scratch, font, width]);
        let w: f32 = mem.read(width);
        ffx_bridge::x87::ftol(mem, ffx_bridge::x87::load(w.to_bits()))
    }

    /// `TOGetLangColonWidth_008df4f0(kind, font)`: the `':'` width
    /// (`TOGetColonWidth_008dfe20(font)`) for kind 0 in language 2 and
    /// kind 1 in languages 1..=5 (`0x008dfe10`, a jump to
    /// `TOGetFFXLang`, asked first); 0 otherwise.
    pub fn lang_colon_width<M: Mem>(
        mem: &mut M,
        kind: u32,
        font: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let lang = call(mem, 0x008d_fe10, &[]);
        let wanted = match kind {
            0 => lang == 2,
            1 => lang.wrapping_sub(1) <= 4,
            _ => false,
        };
        if wanted {
            call(mem, 0x008d_fe20, &[font])
        } else {
            0
        }
    }

    /// `TOAddPktChrNameIcon_008eba80(cursor, chr, x, y, colour)` (own
    /// name): the packet cursor (`g_TOPktCursor`, `0x018716cc`) set to
    /// `cursor`, then menu icon 0x13 at (`x + 3`, `y + 1`)
    /// (`TOAddPktMenuIcon`) and the character's name
    /// (`TOGetSaveChrName(chr)`, read first) in grey at (`x + 0x16`, `y`)
    /// (`0x008ea000`, colour passed on), each result stored back as the
    /// cursor. Returns the final cursor. Callees through `call`.
    pub fn add_pkt_chr_name_icon<M: Mem>(
        mem: &mut M,
        [cursor, chr, x, y, colour]: [u32; 5],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let pkt_cursor = mem.rebase(0x0187_16cc);
        let name = call(mem, 0x008a_c850, &[chr]);
        mem.write(pkt_cursor, cursor);
        let p = call(
            mem,
            0x008e_b240,
            &[cursor, x.wrapping_add(3), y.wrapping_add(1), 0x13],
        );
        mem.write(pkt_cursor, p);
        let p = call(
            mem,
            0x008e_a000,
            &[p, name, x.wrapping_add(0x16), y, colour],
        );
        mem.write(pkt_cursor, p);
        p
    }

    /// `TODrawFont3d80String_008ee060(cursor, text, x, y, nibbles, c0, c1,
    /// c2, c3)` (own name): the NUL-terminated `text` drawn character by
    /// character through `TODrawFont3d80Glyph_008eeee0` (each cursor fed
    /// to the next), 10 units apart from `x`, after passing `cursor`
    /// through `Identity_008f2f70`. Returns the last cursor. Callees
    /// through `call` (`ffx.sym` addresses).
    pub fn draw_font_3d80_string<M: Mem>(
        mem: &mut M,
        [cursor, text, x, y, nibbles, c0, c1, c2, c3]: [u32; 9],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut cursor = call(mem, 0x008f_2f70, &[cursor]);
        let (mut text, mut x) = (text, x);
        while mem.read::<u8>(text) != 0 {
            let args = [cursor, text, x, y, nibbles, c0, c1, c2, c3];
            cursor = call(mem, 0x008e_eee0, &args);
            text = text.wrapping_add(1);
            x = x.wrapping_add(10);
        }
        cursor
    }

    /// Bytes [`draw_font_3d80_glyph`] needs as scratch: the 0x98-byte
    /// sprite record `graphicFontDraw` reads, then the two image sizes
    /// (`f32` width and height, twice).
    pub const GLYPH_3D80_SCRATCH: u32 = 0xa8;

    /// `TODrawFont3d80Glyph_008eeee0(cursor, text, x, y, nibbles, c0, c1,
    /// c2, c3)` (own name): the character `*text` (signed) from the 0x3d80
    /// font sheet, drawn twice through `graphicFontDraw(record, texture,
    /// 0.0, 0.0, 0)`. Character `c - 0x20` sits in cell column
    /// `(c % 32) / 2` (8 wide) and row `c / 32` (10 high, from 192), odd
    /// characters on the second texture of a pair. The first draw takes
    /// `TOGetTextureName(0x3d80, 2 * (nibbles >> 4 & 0xf) + odd, 0, 1,
    /// &size)` at (`x + 1`, `y`)..(`x + 0xb`, `y + 0xc`); the second
    /// `TOGetTextureName(0x3d80, 2 * (nibbles & 0xf) + odd, 0, 0, &size)`
    /// at (`x`, `y - 1`)..(`x + 0xa`, `y + 0xb`). UVs divide the cell
    /// corners (the right edge 3.5 in, at double precision) by the sizes
    /// `TOGetImageWH(0x3d80, ..)` gives: the second pair for the first
    /// draw, the first for the second. Both corners carry the four words
    /// `c0..c3`, the record's `+0x88` is 0 and `+0x90` holds `size` (8
    /// bytes, zeroed before each lookup). The original leaves
    /// `+0x40..+0x88` and `+0x8c` uninitialised; the port leaves `scratch`
    /// as given there. `scratch` holds [`GLYPH_3D80_SCRATCH`] bytes.
    /// Returns `cursor + 0x10`. Callees through `call`.
    pub fn draw_font_3d80_glyph<M: Mem>(
        mem: &mut M,
        [cursor, text, x, y, nibbles, c0, c1, c2, c3]: [u32; 9],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let record = scratch;
        let size = record + 0x90;
        let (w1, h1, w2, h2) = (
            scratch + 0x98,
            scratch + 0x9c,
            scratch + 0xa0,
            scratch + 0xa4,
        );
        let c = i32::from(mem.read::<i8>(text)) - 0x20;
        let column = c % 32 / 2 * 8;
        let row = c / 32 * 10 + 0xc0;
        let odd = c.cast_unsigned() & 1;
        let (x, y) = (x.cast_signed(), y.cast_signed());
        let sheet_cell = Glyph3d80Cell::new(column, row, [c0, c1, c2, c3]);

        mem.write(size, 0u64);
        let first = (nibbles >> 4 & 0xf) * 2 + odd;
        let name = call(mem, 0x008a_cf50, &[0x3d80, first, 0, 1, size]);
        call(mem, 0x008a_c400, &[0x3d80, w1, h1]);
        call(mem, 0x008a_c400, &[0x3d80, w2, h2]);
        let rect = [
            x.wrapping_add(1),
            y,
            x.wrapping_add(0xb),
            y.wrapping_add(0xc),
        ];
        sheet_cell.fill(mem, record, rect, (w2, h2));
        call(mem, 0x0064_0660, &[record, name, 0, 0, 0]);

        mem.write(size, 0u64);
        let second = (nibbles & 0xf) * 2 + odd;
        let name = call(mem, 0x008a_cf50, &[0x3d80, second, 0, 0, size]);
        let rect = [
            x,
            y.wrapping_sub(1),
            x.wrapping_add(0xa),
            y.wrapping_add(0xb),
        ];
        sheet_cell.fill(mem, record, rect, (w1, h1));
        call(mem, 0x0064_0660, &[record, name, 0, 0, 0]);
        cursor.wrapping_add(0x10)
    }

    /// One 0x3d80 glyph cell of [`draw_font_3d80_glyph`]: its corners on
    /// the sheet and the record's per-corner words.
    struct Glyph3d80Cell {
        left: f32,
        top: f32,
        /// `(column + 11) - 3.5`, at double precision.
        right: f64,
        bottom: f32,
        words: [u32; 4],
    }

    impl Glyph3d80Cell {
        #[allow(clippy::cast_precision_loss)] // as the original's fild
        fn new(column: i32, row: i32, words: [u32; 4]) -> Self {
            Self {
                left: column as f32,
                top: row as f32,
                right: f64::from(column + 0xb) - 3.5,
                bottom: (row + 0xb) as f32,
                words,
            }
        }

        /// The record's two corners for screen `rect` (left, top, right,
        /// bottom) over the image size read from `(w, h)`; `+0x88` to 0.
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        fn fill(&self, mem: &mut impl Mem, record: u32, rect: [i32; 4], (w, h): (u32, u32)) {
            let w = f64::from(mem.read::<f32>(w));
            let h = f64::from(mem.read::<f32>(h));
            mem.write(record + 0x88, 0u32);
            let corners = [
                (
                    rect[0],
                    rect[1],
                    f64::from(self.left) / w,
                    f64::from(self.top) / h,
                ),
                (rect[2], rect[3], self.right / w, f64::from(self.bottom) / h),
            ];
            for (k, (cx, cy, u, v)) in (0u32..).zip(corners) {
                let at = record + 0x20 * k;
                mem.write(at, cx as f32);
                mem.write(at + 4, cy as f32);
                mem.write(at + 8, u as f32);
                mem.write(at + 0xc, v as f32);
                for (j, word) in (0u32..).zip(self.words) {
                    mem.write(at + 0x10 + 4 * j, word);
                }
            }
        }
    }

    /// Bytes [`setup_message_window`] needs as scratch: the two `f32`
    /// sizes and the word `TOGetMessageWH` writes through its pointers.
    pub const SETUP_MESSAGE_WINDOW_SCRATCH: u32 = 0xc;

    /// `TOSetupMessageWindow_008b8de0(w)` (own name; caller `0x008b09b0`):
    /// sizes and places message window `w` for its text (`+0x94`) after
    /// its callback `+0xf4(w)`. Window fields are `i16` unless noted: id
    /// `+4`, font `+0x1a`, flags `+0x10` (`u16`), choice style `+0x22`,
    /// centre `+0x30`/`+0x34` (`f32`), area `+0x40`/`+0x42`, size
    /// `+0x44`/`+0x46`, rect `+0x48`, word counts `+0xa0`/`+0xa4`/`+0xa8`
    /// (`i32`).
    ///
    /// Without a message (`+8` 0): `+2` = 0, the counts to
    /// `TOCountMessageWords_008b80b0(id, text)`, the size from
    /// `TOGetMessageWH(id, text, &w, &+0x46, font, 1)` (`+0x44` =
    /// `ftol(w)`), and the rect at x `ftol(f32((area_x - width) / 2) +
    /// centre_x)` (stored as `f32` first), y 1 in window mode 1, else
    /// `ftol(f32((area_y - height) / 2) + centre_y)`; returns
    /// `TOWideWindowRect_008b8d10`'s `eax`.
    ///
    /// With one: `+0x24` and `+0x28` count the words before byte 3
    /// (`TOCountMessageWordsAbove_008b8140`), `+0x28` adding those of the
    /// macro text `0x025d09bc` when positive. The first line is measured
    /// with flags `| 3`, the whole text (from the pointer it returns) with
    /// `| 1` into `+0x46` and `h`. Choice style 1 also adds the macro
    /// width to a positive first-line width, keeps `ftol` of that at
    /// `+0x2a` and makes the width `ftol(f32(i16 ftol(h)) + w)`, `+0xa0` =
    /// `+0x28`; otherwise the width is `ftol(h)`. Then the rect (x as
    /// above, y `ftol` of the unstored sum), `+0xa8` = the words in the
    /// text (plus the macro text's for style 1 with choices), `+2` = 1
    /// when that equals the choice count (`+0x28` for style 1, `+0x24`
    /// else), and `+0xa0` = `+0xa4` = `+0xa8`; returns `+0xa8`.
    ///
    /// Either way, with `+0x1e` 1, `TOLayoutMessageChoices_008b9a60(w,
    /// 1)` follows and its `eax` is returned. `call` takes absolute
    /// addresses (the callback is one); `fcall` calls
    /// `TOGetChangeMacroWidth` (an `ffx.sym` address) for its `ST0`.
    #[allow(clippy::too_many_lines)]
    pub fn setup_message_window<M: Mem>(
        mem: &mut M,
        w: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::ftol;
        let i16_at = |mem: &M, off: u32| -> i32 { i32::from(mem.read::<i16>(w + off)) };
        let (first_w, whole_h, word) = (scratch, scratch + 4, scratch + 8);
        mem.write(first_w, 0.0f32);
        mem.write(whole_h, 0.0f32);
        mem.write(word, 0u32);
        let callback: u32 = mem.read(w + 0xf4);
        call(mem, callback, &[w]);
        let mut sym = |mem: &mut M, f: u32, args: &[u32]| {
            let at = mem.rebase(f);
            call(mem, at, args)
        };
        let text: u32 = mem.read(w + 0x94);
        let macro_text = mem.rebase(0x025d_09bc);

        // x: half the free width, as f32, plus the centre, stored as f32
        #[allow(clippy::cast_possible_truncation)] // the f32 store
        let x_of = |mem: &M| -> f32 {
            #[allow(clippy::cast_precision_loss)]
            let half = ((i16_at(mem, 0x40) - i16_at(mem, 0x44)) / 2) as f32;
            (f64::from(half) + f64::from(mem.read::<f32>(w + 0x30))) as f32
        };
        // y's sum, before ftol
        let y_of = |mem: &M| -> f64 {
            #[allow(clippy::cast_precision_loss)]
            let half = ((i16_at(mem, 0x42) - i16_at(mem, 0x46)) / 2) as f32;
            f64::from(half) + f64::from(mem.read::<f32>(w + 0x34))
        };

        let mut eax;
        if mem.read::<u32>(w + 8) == 0 {
            mem.write(w + 2, 0u16);
            let id = i16_at(mem, 4).cast_unsigned();
            let words = sym(mem, 0x008b_80b0, &[id, text]);
            for off in [0xa8, 0xa0, 0xa4] {
                mem.write(w + off, words);
            }
            let font = i16_at(mem, 0x1a).cast_unsigned();
            sym(mem, 0x008b_9650, &[id, text, whole_h, w + 0x46, font, 1]);
            let width = ftol(mem, f64::from(mem.read::<f32>(whole_h)));
            #[allow(clippy::cast_possible_truncation)]
            mem.write(w + 0x44, width as u16);
            let mode = sym(mem, 0x008a_c7b0, &[]);
            let width = i16_at(mem, 0x44);
            let x = x_of(mem);
            let y = if mode == 1 {
                1
            } else {
                #[allow(clippy::cast_possible_truncation)]
                let y = ftol(mem, y_of(mem)) as i16;
                i32::from(y)
            };
            let height = i16_at(mem, 0x46);
            let x = ftol(mem, f64::from(x));
            let args = [
                id,
                w + 0x48,
                x,
                y.cast_unsigned(),
                width.cast_unsigned(),
                height.cast_unsigned(),
            ];
            eax = sym(mem, 0x008b_8d10, &args);
        } else {
            let id = i16_at(mem, 4).cast_unsigned();
            let choices = sym(mem, 0x008b_8140, &[id, text, 3]);
            #[allow(clippy::cast_possible_truncation)]
            let choices = choices as u16;
            mem.write(w + 0x24, choices);
            mem.write(w + 0x28, choices);
            if choices.cast_signed() > 0 {
                let id = i16_at(mem, 4).cast_unsigned();
                let more = sym(mem, 0x008b_8140, &[id, macro_text, 0]);
                let total: u16 = mem.read(w + 0x28);
                #[allow(clippy::cast_possible_truncation)]
                mem.write(w + 0x28, total.wrapping_add(more as u16));
            }
            let flags = u32::from(mem.read::<u16>(w + 0x10));
            let style_1 = mem.read::<i16>(w + 0x22) == 1;
            let font = i16_at(mem, 0x1a).cast_unsigned();
            let id = i16_at(mem, 4).cast_unsigned();
            let rest = sym(
                mem,
                0x008b_9650,
                &[id, text, first_w, word, font, flags | 3],
            );
            if style_1 {
                let mut first = mem.read::<f32>(first_w);
                if 0.0 < first {
                    let font = i16_at(mem, 0x1a).cast_unsigned();
                    let id = i16_at(mem, 4).cast_unsigned();
                    let extra = fcall(mem, 0x008b_6a30, &[id, macro_text, font]);
                    #[allow(clippy::cast_possible_truncation)]
                    {
                        first = (extra + f64::from(first)) as f32;
                    }
                    mem.write(first_w, first);
                }
                let first = mem.read::<f32>(first_w);
                let narrow = ftol(mem, f64::from(first));
                #[allow(clippy::cast_possible_truncation)]
                mem.write(w + 0x2a, narrow as u16);
            }
            let flags = u32::from(mem.read::<u16>(w + 0x10));
            let font = i16_at(mem, 0x1a).cast_unsigned();
            let id = i16_at(mem, 4).cast_unsigned();
            sym(
                mem,
                0x008b_9650,
                &[id, rest, whole_h, w + 0x46, font, flags | 1],
            );
            let whole = ftol(mem, f64::from(mem.read::<f32>(whole_h)));
            let width = if style_1 {
                #[allow(clippy::cast_possible_truncation)]
                let whole = f32::from(whole as i16);
                let first = mem.read::<f32>(first_w);
                ftol(mem, f64::from(whole) + f64::from(first))
            } else {
                whole
            };
            #[allow(clippy::cast_possible_truncation)]
            mem.write(w + 0x44, width as u16);
            if style_1 {
                mem.write(w + 0xa0, i16_at(mem, 0x28));
            }
            let height = i16_at(mem, 0x46);
            let width = i16_at(mem, 0x44);
            #[allow(clippy::cast_possible_truncation)]
            let y = ftol(mem, y_of(mem)) as i16;
            let x = ftol(mem, f64::from(x_of(mem)));
            let id = i16_at(mem, 4).cast_unsigned();
            let args = [
                id,
                w + 0x48,
                x,
                i32::from(y).cast_unsigned(),
                width.cast_unsigned(),
                height.cast_unsigned(),
            ];
            sym(mem, 0x008b_8d10, &args);
            let id = i16_at(mem, 4).cast_unsigned();
            let text: u32 = mem.read(w + 0x94);
            let words = sym(mem, 0x008b_80b0, &[id, text]);
            mem.write(w + 0xa8, words);
            let style = i16_at(mem, 0x22);
            let compare = if style == 1 {
                if i16_at(mem, 0x24) > 0 {
                    let address = sym(mem, 0x008b_6660, &[0, macro_text]);
                    let more = sym(mem, 0x008b_80b0, &[0, address]);
                    let words: u32 = mem.read(w + 0xa8);
                    mem.write(w + 0xa8, words.wrapping_add(more));
                    true
                } else {
                    false
                }
            } else {
                true
            };
            if compare && i16_at(mem, 0x24) > 0 {
                let choices = if i16_at(mem, 0x22) == 1 {
                    i16_at(mem, 0x28)
                } else {
                    i16_at(mem, 0x24)
                };
                if mem.read::<i32>(w + 0xa8) == choices {
                    mem.write(w + 2, 1u16);
                }
            }
            let words: u32 = mem.read(w + 0xa8);
            mem.write(w + 0xa0, words);
            mem.write(w + 0xa4, words);
            eax = words;
        }
        if mem.read::<i16>(w + 0x1e) == 1 {
            eax = sym(mem, 0x008b_9a60, &[w, 1]);
        }
        eax
    }

    /// The pen the 16x12 message writers draw with (`0x025d09a0`): the
    /// colour bytes `+0..+3`, x `+4`, y `+8` (f32), `+0xc..+0x18` cleared,
    /// the shadow/edge kind `+0x1a`, the colour nibbles `+0x1b`.
    pub const MES_PEN: u32 = 0x025d_09a0;
    /// The glyph record they fill (`0x01841d50`, 0x40 bytes; its width
    /// `+0x1c`).
    pub const MES_GLYPH: u32 = 0x0184_1d50;

    /// `0xa n` on the pen's colour byte `+0x1b`: a non-zero low nibble `l`
    /// puts `l - 1` in its low nibble, a high nibble from 4 puts
    /// `(n & 0xf0) - 0x40` in its high one (`n` read for each).
    fn pen_colour_code(mem: &mut impl Mem, pen: u32, code: u32) {
        let at = pen.wrapping_add(0x1b);
        let mut c: u8 = mem.read(at);
        let low = mem.read::<u8>(code.wrapping_add(1)) & 0xf;
        if low != 0 {
            c = c & 0xf0 | (low - 1);
            mem.write(at, c);
        }
        let high = mem.read::<u8>(code.wrapping_add(1)) & 0xf0;
        if high >= 0x40 {
            c = c & 0xf | (high - 0x40);
            mem.write(at, c);
        }
    }

    /// `TOGetGroup`'s argument as the writers pass it: the text byte in
    /// the low byte of a register whose upper bytes still hold `upper`.
    fn group_arg(mem: &impl Mem, p: u32, upper: u32) -> u32 {
        upper & 0xffff_ff00 | u32::from(mem.read::<u8>(p))
    }

    /// `TOAddPktBaseMesFontLClutTypeRGBA(cursor, key, text, x, y, colour,
    /// kind, r, g, b, a)` (`0x008e92a0`): writes `text` with the 16x12 font
    /// from the integer pen `(x, y)` (stored as `f32`) and returns the
    /// packet cursor ([`crate::pkt::bridge::PACKET_CURSOR`], first set to
    /// `cursor`). The glyph record is cleared and the pen set up
    /// ([`MES_PEN`]: colour bytes `r g b a`, `+0x1a` = `kind`, `+0x1b` =
    /// `colour`). Per `TOGetGroup` of each code:
    /// - 1 (new line): x back to its start, y + 16.0.
    /// - 2 (control): 7 with 0xe1..=0xef moves x by `n - 0xe0`; 0xa sets
    ///   the colour nibbles; 0xb draws button icon `n`
    ///   (`TOAdpButtonIconRGBA_008e99a0(cursor, n, ftol(x), ftol(y), r, g,
    ///   b, a)`) and moves x by `f32(TOGetButtonIconWidth(n)) *
    ///   [0x00c5d778]`; codes other than 7 end with `resetButtonData`.
    /// - 3 (macro): `TOAddPktChangeMacro(key, text, pen)` unless `key` is
    ///   -1.
    /// - 4 (glyph): its record (`TOGetFontInfo`), then with kind 1 the edge
    ///   and face packets and `TOAddFontWidthSpace(pen, record,
    ///   [0x00c5d784])`; else the shadow and face between
    ///   `graphicSetUiDrawType(1)`/`(0)` and `graphicFontSetUiShadowType(0)`
    ///   /`(1)`, and the width space at 0.78.
    ///
    /// `TOGetGroup` gets the byte with the upper bytes the original's
    /// register holds (`a`'s for the first code, then the text pointer's).
    #[allow(
        clippy::too_many_lines,
        clippy::many_single_char_names,
        clippy::cast_possible_truncation // mov byte
    )]
    pub fn base_mes_font_l_clut_type_rgba<M: Mem>(
        mem: &mut M,
        args: [u32; 11],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87;
        let [cursor, key, text, x, y, colour, kind, r, g, b, a] = args;
        let store = |v: f64| f32::from_bits(x87::store(v));
        let cursor_at = crate::pkt::bridge::PACKET_CURSOR;
        let glyph = mem.rebase(MES_GLYPH);
        let pen = mem.rebase(MES_PEN);
        let pen_x = pen.wrapping_add(4);
        let pen_y = pen.wrapping_add(8);
        mem.write(glyph, [0u32; 0x10]);
        mem.write(pen, [0u32; 7]);
        cursor_at.set(mem, cursor);
        let x0 = store(f64::from(x.cast_signed()));
        mem.write(pen.wrapping_add(1), g as u8);
        mem.write(pen.wrapping_add(2), b as u8);
        mem.write(pen_x, x0);
        mem.write(pen.wrapping_add(3), a as u8);
        mem.write(pen.wrapping_add(0x1a), kind as u8);
        mem.write(pen_y, store(f64::from(y.cast_signed())));
        mem.write(pen, r as u8);
        mem.write(pen.wrapping_add(0x1b), colour as u8);
        let f = |mem: &M, at: u32| f64::from(mem.read::<f32>(at));
        let mut p = text;
        let mut upper = a;
        while mem.read::<u8>(p) != 0 {
            match call(mem, 0x008d_f610, &[group_arg(mem, p, upper)]) {
                1 => {
                    mem.write(pen_x, x0);
                    mem.write(pen_y, store(x87::add(f(mem, pen_y), 16.0)));
                }
                2 => {
                    let code: u8 = mem.read(p);
                    if code == 7 {
                        let n: u8 = mem.read(p.wrapping_add(1));
                        if n > 0xe0 && n < 0xf0 {
                            let x = x87::add(f64::from(n - 0xe0), f(mem, pen_x));
                            mem.write(pen_x, store(x));
                        }
                    } else {
                        if code == 0xa {
                            pen_colour_code(mem, pen, p);
                        }
                        if mem.read::<u8>(p) == 0xb {
                            let n = u32::from(mem.read::<u8>(p.wrapping_add(1)));
                            call(mem, 0x008b_1350, &[n, u32::MAX]);
                            let yi = x87::ftol(mem, f(mem, pen_y));
                            let xi = x87::ftol(mem, f(mem, pen_x));
                            let c = cursor_at.get(mem);
                            let n = u32::from(mem.read::<u8>(p.wrapping_add(1)));
                            let args = [c, n, xi, yi, r, g, b, a];
                            let c = call(mem, 0x008e_99a0, &args);
                            cursor_at.set(mem, c);
                            let n = u32::from(mem.read::<u8>(p.wrapping_add(1)));
                            let w = call(mem, 0x008b_65e0, &[n]).cast_signed();
                            let w = f64::from(store(f64::from(w)));
                            let unit = f(mem, mem.rebase(0x00c5_d778));
                            let x = x87::add(x87::mul(w, unit), f(mem, pen_x));
                            mem.write(pen_x, store(x));
                        }
                        call(mem, 0x008b_1330, &[]);
                    }
                }
                3 => {
                    if key != u32::MAX {
                        call(mem, 0x008e_9b80, &[key, p, pen]);
                    }
                }
                4 => {
                    call(mem, 0x008b_7240, &[p, glyph]);
                    let space = if mem.read::<u8>(pen.wrapping_add(0x1a)) == 1 {
                        call(mem, 0x0090_6260, &[pen, glyph]);
                        call(mem, 0x0090_6270, &[pen, glyph]);
                        mem.read::<u32>(mem.rebase(0x00c5_d784))
                    } else {
                        call(mem, 0x0064_3af0, &[1]);
                        call(mem, 0x0064_0dd0, &[0]);
                        call(mem, 0x0090_6290, &[pen, glyph]);
                        call(mem, 0x0064_0dd0, &[1]);
                        call(mem, 0x0090_6270, &[pen, glyph]);
                        call(mem, 0x0064_3af0, &[0]);
                        0.78f32.to_bits()
                    };
                    let space = x87::round_trip(f32::from_bits(space)).to_bits();
                    call(mem, 0x008e_8ed0, &[pen, glyph, space]);
                }
                _ => {}
            }
            p = call(mem, 0x008d_fd80, &[p]);
            upper = p;
        }
        cursor_at.get(mem)
    }

    /// `TOAddPktMesFont1612_008ea060(cursor, text, x, y, colour, r, g, b,
    /// a)`: the plain variant of [`base_mes_font_l_clut_type_rgba`]: the
    /// pen as there (kind 0), glyphs (group 4) with their shadow and face
    /// packets and x moved by the record's width `+0x1c` plus 1.0; control
    /// codes 0xa (colour nibbles) and 0xb (button icon
    /// `TOAdpButtonIcon_008e9970(cursor, n, ftol(x), ftol(y))`, x + 22.0),
    /// every control code ending with `resetButtonData`; other groups
    /// skipped. Returns the packet cursor (`cursor` for an empty text).
    #[allow(clippy::many_single_char_names, clippy::cast_possible_truncation)] // mov byte
    pub fn mes_font_1612<M: Mem>(
        mem: &mut M,
        args: [u32; 9],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87;
        let [cursor, text, x, y, colour, r, g, b, a] = args;
        let store = |v: f64| f32::from_bits(x87::store(v));
        let cursor_at = crate::pkt::bridge::PACKET_CURSOR;
        let glyph = mem.rebase(MES_GLYPH);
        let pen = mem.rebase(MES_PEN);
        let pen_x = pen.wrapping_add(4);
        let pen_y = pen.wrapping_add(8);
        mem.write(glyph, [0u32; 0x10]);
        mem.write(pen, 0u32);
        mem.write(pen.wrapping_add(0x18), 0u32);
        mem.write(pen_x, 0u32);
        mem.write(pen_x, store(f64::from(x.cast_signed())));
        for off in [8, 0xc, 0x10, 0x14] {
            mem.write(pen.wrapping_add(off), 0u32);
        }
        mem.write(pen_y, store(f64::from(y.cast_signed())));
        mem.write(pen, [r as u8, g as u8, b as u8, a as u8]);
        mem.write(pen.wrapping_add(0x1a), 0u8);
        mem.write(pen.wrapping_add(0x1b), colour as u8);
        cursor_at.set(mem, cursor);
        if mem.read::<u8>(text) == 0 {
            return cursor;
        }
        let f = |mem: &M, at: u32| f64::from(mem.read::<f32>(at));
        let mut p = text;
        let mut upper = 0;
        loop {
            match call(mem, 0x008d_f610, &[group_arg(mem, p, upper)]) {
                2 => {
                    if mem.read::<u8>(p) == 0xa {
                        pen_colour_code(mem, pen, p);
                    }
                    if mem.read::<u8>(p) == 0xb {
                        let n = u32::from(mem.read::<u8>(p.wrapping_add(1)));
                        call(mem, 0x008b_1350, &[n, u32::MAX]);
                        let yi = x87::ftol(mem, f(mem, pen_y));
                        let xi = x87::ftol(mem, f(mem, pen_x));
                        let n = u32::from(mem.read::<u8>(p.wrapping_add(1)));
                        let c = cursor_at.get(mem);
                        let c = call(mem, 0x008e_9970, &[c, n, xi, yi]);
                        cursor_at.set(mem, c);
                        mem.write(pen_x, store(x87::add(f(mem, pen_x), 22.0)));
                    }
                    call(mem, 0x008b_1330, &[]);
                }
                4 => {
                    call(mem, 0x008b_7240, &[p, glyph]);
                    call(mem, 0x0090_6290, &[pen, glyph]);
                    call(mem, 0x0090_6270, &[pen, glyph]);
                    let w = f(mem, glyph.wrapping_add(0x1c));
                    let x = x87::add(x87::add(w, 1.0), f(mem, pen_x));
                    mem.write(pen_x, store(x));
                }
                _ => {}
            }
            p = call(mem, 0x008d_fd80, &[p]);
            upper = p;
            if mem.read::<u8>(p) == 0 {
                return cursor_at.get(mem);
            }
        }
    }

    /// The text [`cross_number`] formats into (the original's 32-byte
    /// stack buffer).
    pub const CROSS_NUMBER_TEXT: u32 = 0x20;

    /// `TOAdpCrossNumber_009019f0(value, x, y, colour, scale, gap)`:
    /// `value` as `"%d"` (`sprintf(text, fmt, value)`, passed in; the format
    /// `0x00b5f090`) drawn glyph by glyph with `TOAdpCrossFontM_FFX(p, x, y,
    /// colour, 0x80, 0x80, 0x80, 0x80, scale)`, ending at `x`: the pen starts
    /// at `x - scale * (n * (gap + 10) - gap)` for `n` characters (each
    /// operation at 53 bits, stored as `f32`) and moves `(gap + 10) * scale`
    /// (kept at 53 bits) per glyph, `TOSkipCode` stepping the text. `call`
    /// takes `ffx.sym` addresses.
    #[allow(clippy::many_single_char_names)] // the original's arguments
    pub fn cross_number<M: Mem>(
        mem: &mut M,
        [value, x, y, colour, scale, gap]: [u32; 6],
        text: u32,
        sprintf: impl FnOnce(&mut M, [u32; 3]),
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87;
        let fmt = mem.rebase(0x00b5_f090);
        sprintf(mem, [text, fmt, value]);
        let mut n = 0u32;
        while mem.read::<u8>(text.wrapping_add(n)) != 0 {
            n += 1;
        }
        let load = |bits: u32| x87::load(bits);
        let (s, gap) = (load(scale), load(gap));
        let wide = x87::add(10.0, gap);
        // fild, then fstp dword
        let count = f64::from(f32::from_bits(x87::store(f64::from(n.cast_signed()))));
        let back = x87::mul(x87::sub(x87::mul(count, wide), gap), s);
        let mut pen = x87::store(x87::sub(load(x), back));
        // ("%d" gives at least one character)
        if mem.read::<u8>(text) == 0 {
            return;
        }
        let step = x87::mul(wide, s);
        let y = x87::round_trip(f32::from_bits(y)).to_bits();
        let scale = x87::round_trip(f32::from_bits(scale)).to_bits();
        let mut p = text;
        loop {
            let args = [p, pen, y, colour, 0x80, 0x80, 0x80, 0x80, scale];
            call(mem, 0x008e_c720, &args);
            pen = x87::store(x87::add(load(pen), step));
            p = call(mem, 0x008d_fd80, &[p]);
            if mem.read::<u8>(p) == 0 {
                return;
            }
        }
    }

    /// `ShapeDataModify(shape, id)` (`0x008e8a10`): the HD port's fixes to
    /// the shape record `shape` (`+0`/`+4` and `+0x20`/`+0x24` the corners,
    /// `+8`/`+0xc` and `+0x28`/`+0x2c` the UVs, `f32`; `+0x10..+0x18`,
    /// `+0x1c`, `+0x30..+0x38`, `+0x3c` colour words) for a few shape ids
    /// (signed): new corners through `graphicUiRemapX`/`Y` (`remap`, the
    /// 720p remaps), new UVs, fixed offsets. Shapes 0x19b and 0x19c clear
    /// both alphas when the first colour is black, else make both colours
    /// grey (0x80); with 0x1fb they then pulse each non-zero alpha to
    /// `ftol(f32(cos(graphicGetTime() * 3)) * 10 + 118)` (`fcall`, `cos` the
    /// C runtime's `_CIcos`). 0x1024 and 0x102d move up 20 or 30, take 25
    /// off the right edge and turn alphas from 0x46 (signed) into 0x80.
    /// Sums are formed at 53 bits and stored as `f32`.
    #[allow(clippy::too_many_lines)]
    pub fn shape_data_modify<M: Mem>(
        mem: &mut M,
        shape: u32,
        id: i32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
        mut cos: impl FnMut(&mut M, f64) -> f64,
        mut remap: impl FnMut(&mut M, bool, f32) -> f32,
    ) {
        use ffx_bridge::x87;
        const GET_TIME: u32 = 0x0064_1410;
        let at = |off: u32| shape.wrapping_add(off);
        let f = |mem: &M, off: u32| f64::from(mem.read::<f32>(at(off)));
        let store = |v: f64| f32::from_bits(x87::store(v));
        let mut corners = |mem: &mut M, [x0, y0, x1, y1]: [Option<f32>; 4]| {
            for (off, y, v) in [
                (0, false, x0),
                (4, true, y0),
                (0x20, false, x1),
                (0x24, true, y1),
            ] {
                if let Some(v) = v {
                    let r = remap(mem, y, v);
                    mem.write(at(off), r);
                }
            }
        };
        let uvs = |mem: &mut M, uv: [f32; 4]| {
            for (off, v) in [8, 0xc, 0x28, 0x2c].into_iter().zip(uv) {
                mem.write(at(off), v);
            }
        };
        // each set alpha pulsing with the clock
        let mut pulse = |mem: &mut M| {
            for off in [0x1c, 0x3c] {
                if mem.read::<u32>(at(off)) != 0 {
                    let t = fcall(mem, GET_TIME, &[]);
                    let c = f64::from(store(cos(mem, x87::mul(t, 3.0))));
                    let alpha = x87::ftol(mem, x87::add(x87::mul(c, 10.0), 118.0));
                    mem.write(at(off), alpha);
                }
            }
        };
        // black: no alphas; else grey colours and the pulse
        let grey_or_clear = |mem: &mut M, pulse: &mut dyn FnMut(&mut M)| {
            if [0x10, 0x14, 0x18]
                .iter()
                .all(|&off| mem.read::<u32>(at(off)) == 0)
            {
                mem.write(at(0x1c), 0u32);
                mem.write(at(0x3c), 0u32);
                return;
            }
            for off in [0x10, 0x14, 0x18, 0x30, 0x34, 0x38] {
                mem.write(at(off), 0x80u32);
            }
            pulse(mem);
        };
        match id {
            0x3e => corners(mem, [Some(36.0), None, Some(200.0), None]),
            0x19b => {
                corners(mem, [Some(138.0), Some(702.0), Some(268.0), Some(716.0)]);
                uvs(mem, [0.0, 400.0 / 1024.0, 240.0 / 1024.0, 434.0 / 1024.0]);
                grey_or_clear(mem, &mut pulse);
            }
            0x19c => {
                mem.write(at(0), 0.0f32);
                corners(mem, [None, Some(571.0), Some(130.0), Some(585.0)]);
                uvs(
                    mem,
                    [
                        465.0 / 1024.0,
                        406.0 / 1024.0,
                        705.0 / 1024.0,
                        440.0 / 1024.0,
                    ],
                );
                grey_or_clear(mem, &mut pulse);
            }
            0x19f => {
                mem.write(at(0), 0.0f32);
                corners(mem, [None, Some(700.0), Some(275.0), Some(720.0)]);
                uvs(mem, [0.0, 846.0 / 1024.0, 518.0 / 1024.0, 896.0 / 1024.0]);
            }
            0x1f7 => {
                mem.write(at(4), store(x87::sub(f(mem, 4), 10.0)));
                mem.write(at(0x24), store(x87::sub(f(mem, 0x24), 10.0)));
            }
            0x1fb => {
                corners(mem, [Some(200.0), Some(695.0), Some(280.0), Some(725.0)]);
                pulse(mem);
            }
            0x1fc => {
                mem.write(at(4), store(x87::sub(f(mem, 4), 10.0)));
                mem.write(at(0x24), store(x87::add(f(mem, 0x24), 0.0)));
                mem.write(at(0), store(x87::add(f(mem, 0), 15.0)));
                mem.write(at(0x20), store(x87::add(f(mem, 0x20), 35.0)));
            }
            0x1023 => {
                let w = x87::sub(x87::sub(f(mem, 0x20), f(mem, 0)), 25.0);
                mem.write(at(0x20), store(w));
                mem.write(at(0), 0.0f32);
                corners(mem, [None, Some(560.0), None, Some(584.0)]);
            }
            0x1024 | 0x102d => {
                let up = if id == 0x1024 { 20.0 } else { 30.0 };
                mem.write(at(4), store(x87::sub(f(mem, 4), up)));
                mem.write(at(0x24), store(x87::sub(f(mem, 0x24), up)));
                mem.write(at(0x20), store(x87::sub(f(mem, 0x20), 25.0)));
                for off in [0x1c, 0x3c] {
                    if mem.read::<i32>(at(off)) >= 0x46 {
                        mem.write(at(off), 0x80u32);
                    }
                }
            }
            _ => {}
        }
    }

    /// The message box both [`mes_box`] writers size from: its height and
    /// width (`i16` at `0x025d1642`, `0x025d1644`) and the frame's two words
    /// (`0x025d163c`, `0x025d1638`).
    pub const MES_BOX: u32 = 0x025d_1638;

    /// The box's height and width (sign-extended) and frame words.
    fn mes_box_dims(mem: &impl Mem) -> (u32, u32, u32, u32) {
        let at = mem.rebase(MES_BOX);
        let h = i32::from(mem.read::<i16>(at.wrapping_add(0xa))).cast_unsigned();
        let w = i32::from(mem.read::<i16>(at.wrapping_add(0xc))).cast_unsigned();
        (h, w, mem.read(at.wrapping_add(4)), mem.read(at))
    }

    /// `TODrawMesBox_008ff370(text, x, y, extra)`: the shadow
    /// (`TOMakePktBoxXYWHRGBA(x + 4, y + 4, w, h, 0, 0, 0, 0x80)`), the frame
    /// (`0x008fd390(x, y, w, h, [0x025d163c], [0x025d1638])`) and the text
    /// (`0x00901cb0(text, x + 15, y + 2, extra)`), with the box size of
    /// [`MES_BOX`]. Returns the last call's `eax`.
    pub fn mes_box<M: Mem>(
        mem: &mut M,
        [text, x, y, extra]: [u32; 4],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (h, w, _, _) = mes_box_dims(mem);
        let shadow = [x.wrapping_add(4), y.wrapping_add(4), w, h, 0, 0, 0, 0x80];
        call(mem, 0x008f_b150, &shadow);
        let (h, w, frame_a, frame_b) = mes_box_dims(mem);
        call(mem, 0x008f_d390, &[x, y, w, h, frame_a, frame_b]);
        call(
            mem,
            0x0090_1cb0,
            &[text, x.wrapping_add(0xf), y.wrapping_add(2), extra],
        )
    }

    /// The words `TODrawMesBoxFit_008ff3e0` hands `TOGetMessageWH` (the
    /// text's width and height, the original's locals).
    pub const MES_BOX_FIT_SCRATCH: u32 = 8;

    /// `TODrawMesBoxFit_008ff3e0(text, x, y, extra)`: [`mes_box`] with the
    /// box as wide as the text: `TOGetMessageWH(0, text, &w, &h, 0, 0)` (in
    /// `scratch`), the shadow `ftol(478.0 - w)` and the frame `ftol(482.0 -
    /// w)` wide.
    pub fn mes_box_fit<M: Mem>(
        mem: &mut M,
        [text, x, y, extra]: [u32; 4],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87;
        let args = [0, text, scratch, scratch.wrapping_add(4), 0, 0];
        call(mem, 0x008b_9650, &args);
        let w = |mem: &M| x87::load(mem.read::<u32>(scratch));
        let (h, _, _, _) = mes_box_dims(mem);
        let shadow_w = x87::ftol(mem, x87::sub(478.0, w(mem)));
        let shadow = [
            x.wrapping_add(4),
            y.wrapping_add(4),
            shadow_w,
            h,
            0,
            0,
            0,
            0x80,
        ];
        call(mem, 0x008f_b150, &shadow);
        let (h, _, frame_a, frame_b) = mes_box_dims(mem);
        let frame_w = x87::ftol(mem, x87::sub(482.0, w(mem)));
        call(mem, 0x008f_d390, &[x, y, frame_w, h, frame_a, frame_b]);
        call(
            mem,
            0x0090_1cb0,
            &[text, x.wrapping_add(0xf), y.wrapping_add(2), extra],
        )
    }

    /// `BtlDrawRatio_008955e0()`: with `0x0133f77c` set, draws
    /// `0x00901490(f, x + 15, y - 20, colour)` for the `f32` `f` at
    /// `0x0133f794`, `x`/`y` the `i16` at `0x0133f780`/`0x0133f782`; with the
    /// byte `0x0133c93f` set `y` is first refreshed to the low half of
    /// `ftol(graphicUiRemapY2(350))` (`remap`). The colour is
    /// `0x80008080` below 1.0, else (NaN too) `0x80808080`.
    pub fn btl_draw_ratio<M: Mem>(
        mem: &mut M,
        mut remap: impl FnMut(&mut M, bool, f32) -> f32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87;
        let g = |mem: &M, a: u32| mem.rebase(a);
        if mem.read::<u32>(g(mem, 0x0133_f77c)) == 0 {
            return;
        }
        let y_at = g(mem, 0x0133_f782);
        if mem.read::<u8>(g(mem, 0x0133_c93f)) != 0 {
            let v = remap(mem, true, 350.0);
            #[allow(clippy::cast_possible_truncation)] // mov word, cx
            let y = x87::ftol(mem, f64::from(v)) as u16;
            mem.write(y_at, y);
        }
        let y = i32::from(mem.read::<i16>(y_at));
        let f: u32 = mem.read(g(mem, 0x0133_f794));
        let colour = if x87::load(f) < 1.0 {
            0x8000_8080
        } else {
            0x8080_8080
        };
        let x = i32::from(mem.read::<i16>(g(mem, 0x0133_f780)));
        let f = x87::round_trip(f32::from_bits(f)).to_bits();
        let args = [
            f,
            x.wrapping_add(0xf).cast_unsigned(),
            y.wrapping_sub(0x14).cast_unsigned(),
            colour,
        ];
        call(mem, 0x0090_1490, &args);
    }
    /// `TOAddPktBaseMesFontLClutTypeRGBA` and `0x008ea060`, the writers
    /// the wrappers below call.
    pub const BASE_MES_FONT: u32 = 0x008e_92a0;
    pub const MES_FONT_1612: u32 = 0x008e_a060;
    /// `TOValueToMesFontL(value, out, width)`.
    pub const VALUE_TO_MES_FONT_L: u32 = 0x008b_9140;

    /// `TOAddPktBaseMesFont_008ed0a0(cursor, text, x, y, colour)`:
    /// `TOAddPktBaseMesFontLClutTypeRGBA(cursor, -1, text, x, y, colour, 0,
    /// 0x80, 0x80, 0x80, 0x80)`.
    pub fn base_mes_font<M: Mem>(
        mem: &mut M,
        [cursor, text, x, y, colour]: [u32; 5],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let args = [
            cursor,
            u32::MAX,
            text,
            x,
            y,
            colour,
            0,
            0x80,
            0x80,
            0x80,
            0x80,
        ];
        call(mem, BASE_MES_FONT, &args)
    }

    /// `TOAddPktBaseMesFontPlain_008ed060(cursor, text, x, y)`: the writer
    /// with key 0 (not -1), colour 0 and grey.
    pub fn base_mes_font_plain<M: Mem>(
        mem: &mut M,
        [cursor, text, x, y]: [u32; 4],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let args = [cursor, 0, text, x, y, 0, 0, 0x80, 0x80, 0x80, 0x80];
        call(mem, BASE_MES_FONT, &args)
    }

    /// `TOAddPktBaseMesFontRGBA_008ed160(cursor, text, x, y, colour, r, g,
    /// b, a)`: the writer with key -1 and kind 0.
    #[allow(clippy::many_single_char_names)] // the original's arguments
    pub fn base_mes_font_rgba<M: Mem>(
        mem: &mut M,
        [cursor, text, x, y, colour, r, g, b, a]: [u32; 9],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let args = [cursor, u32::MAX, text, x, y, colour, 0, r, g, b, a];
        call(mem, BASE_MES_FONT, &args)
    }

    /// `TOAddPktBaseMesFontLClutRGBA_copy_008ee210(cursor, key, text, x, y,
    /// colour, r, g, b, a)`: the writer with kind 0.
    #[allow(clippy::many_single_char_names)] // the original's arguments
    pub fn base_mes_font_l_clut_rgba<M: Mem>(
        mem: &mut M,
        [cursor, key, text, x, y, colour, r, g, b, a]: [u32; 10],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let args = [cursor, key, text, x, y, colour, 0, r, g, b, a];
        call(mem, BASE_MES_FONT, &args)
    }

    /// `TOAddPktMesFont1612Grey_008ea000(cursor, text, x, y, colour)`:
    /// `0x008ea060` in grey (0x80 each).
    pub fn mes_font_1612_grey<M: Mem>(
        mem: &mut M,
        [cursor, text, x, y, colour]: [u32; 5],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let args = [cursor, text, x, y, colour, 0x80, 0x80, 0x80, 0x80];
        call(mem, MES_FONT_1612, &args)
    }

    /// The text `TOAddPktMesValue_008f2ff0` formats into (the original's
    /// 32-byte local).
    pub const MES_VALUE_TEXT: u32 = 0x20;

    /// `TOAddPktMesValue_008f2ff0(cursor, value, x, y, colour)`: `value`
    /// written as text (`TOValueToMesFontL(value, text, 0)`) with
    /// [`base_mes_font`].
    pub fn mes_value<M: Mem>(
        mem: &mut M,
        [cursor, value, x, y, colour]: [u32; 5],
        text: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, VALUE_TO_MES_FONT_L, &[value, text, 0]);
        let args = [
            cursor,
            u32::MAX,
            text,
            x,
            y,
            colour,
            0,
            0x80,
            0x80,
            0x80,
            0x80,
        ];
        call(mem, BASE_MES_FONT, &args)
    }
}
