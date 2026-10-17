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
}
