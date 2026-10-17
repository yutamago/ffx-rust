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
}
