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
        for f in [0x0089_85b0u32, 0x0089_8490, 0x0089_8330, 0x0089_83f0, 0x0089_8640] {
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
    pub fn control_message_window<M: Mem>(mem: &mut M, mut call: impl FnMut(&mut M, u32, &[u32]) -> u32) {
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
            mem.write(w.wrapping_add(0x1a), if kind & 0x8000 != 0 { 3u16 } else { 1 });
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
            total = ffx_bridge::x87::ftol(mem, ffx_bridge::x87::add(f32_of(total), w)).cast_signed();
            let c: u32 = mem.read(count);
            mem.write(count, c.wrapping_add(1));
        }
        total as f32
    }
}
