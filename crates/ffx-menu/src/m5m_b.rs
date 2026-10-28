//! M5 stage 1, list `menu`, batch 2: the Tk vsync thread switch, small
//! forwarding helpers, the menu render records' initialisers and the
//! menu texture manager's buffers.
//!
//! - **Menu record** (`MenuRecord_InitMulti` / `_InitSingle`): a 4x4
//!   identity matrix at `+0x00`, the render data pointer `+0x94` (to the
//!   header at `+0xe0`), the bounds `+0x98..+0xac` (min three times the
//!   `f32` at `0x00b45bbc`, max three times `0x00b45bb8`), `+0x90` 1,
//!   `+0xb0` multi (1) or single (0), `+0xb4` 0 (`u16`), `+0xb8` 2. The
//!   header: `+0x00` 1, `+0x04` and `+0x08` 0, `+0x0c..+0x1c` the arrays
//!   inside the record (multi: 12 quads; single: one), `+0x20` the texture
//!   name (`0x00c33614` / `0x00c33610`), `+0x24` 1, `+0x34` and `+0x50` 0.
//! - **Texture manager buffers** (`0x0067c440`, `this` in `ecx`): two
//!   vertex/index buffer pairs, `+0x2bd0` capacity / `+0x2bd4` data (600
//!   of 16 bytes), `+0x2bd8` / `+0x2bdc` (800 of 16), `+0x5828` /
//!   `+0x582c` and `+0x5830` / `+0x5834` alike, each reallocated with
//!   `PhyreMalloc(n, 8)` unless at that capacity (the old one freed when it
//!   owned it: capacity not negative), then all cleared.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

pub mod bridge {
    use ffx_bridge::{Global, Mem};
    use ffx_graphic::host::{Conv, Host};

    /// The vsync emulation thread's handle (-1 when none).
    pub const VSYNC_THREAD: Global<u32> = Global::new(0x0133_0268);
    /// The background thread's semaphore (`TkSignalSemaBGThread`).
    pub const BG_THREAD_SEMA: Global<u32> = Global::new(0x0133_0250);
    /// The menus' texture manager.
    pub const MENU_TEXTURE_MANAGER: Global<u32> = Global::new(0x00cc_c820);

    pub const SCE_GS_SYNC_PATH: u32 = 0x0062_f880;
    pub const CREATE_VSYNC_EMU_THREAD: u32 = 0x0067_9e60;
    pub const END_VSYNC_EMU_THREAD: u32 = 0x0067_9f20;
    pub const THREAD_CREATE: u32 = 0x006f_2ae0;
    pub const THREAD_STOP: u32 = 0x006f_2bb0;
    pub const THREAD_START: u32 = 0x006f_2bc0;
    pub const THREAD_DELETE: u32 = 0x006f_2cc0;
    pub const SIGNAL_SEMA: u32 = 0x006f_2ce0;
    pub const TK_SET_VSYNC_HANDLER: u32 = 0x0088_8680;
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
    pub const MEMSET: u32 = 0x0094_964c;
    pub const MENU_TEXTURES_LOAD_NAMES: u32 = 0x0067_c7a0;
    pub const MAP_PART_INIT_LANG_OVERRIDES: u32 = 0x0067_c620;
    /// The bounds' minimum and maximum (`f32`, -1000 and 1000).
    pub const BOUND_MIN: u32 = 0x00b4_5bbc;
    pub const BOUND_MAX: u32 = 0x00b4_5bb8;
    pub const SINGLE_TEXTURE_NAME: u32 = 0x00c3_3610;
    pub const MULTI_TEXTURE_NAME: u32 = 0x00c3_3614;

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn i16_at(mem: &impl Mem, p: u32, o: u32) -> i32 {
        i32::from(mem.read::<i16>(at(p, o)))
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `TkSetVsyncHandler(handler)` (`0x00888680`): ends the vsync thread
    /// when one runs (`endVsyncEmuThread`, then the thread stopped and
    /// deleted, slot 2), and with a handler starts a new one
    /// (`createVsyncEmuThread`, thread 2 created and started; the start is
    /// the original's tail call with its argument word set to 2). Returns
    /// the start's `eax`, or -1 without a handler.
    pub fn set_vsync_handler(h: &mut impl Host, handler: u32) -> u32 {
        if VSYNC_THREAD.get(h) != u32::MAX {
            cdecl(h, END_VSYNC_EMU_THREAD, &[]);
            cdecl(h, THREAD_STOP, &[2]);
            let t = VSYNC_THREAD.get(h);
            cdecl(h, THREAD_DELETE, &[2, t]);
            VSYNC_THREAD.set(h, u32::MAX);
        }
        if handler == 0 {
            return u32::MAX;
        }
        cdecl(h, CREATE_VSYNC_EMU_THREAD, &[handler]);
        let t = cdecl(h, THREAD_CREATE, &[2, handler, u32::MAX]);
        VSYNC_THREAD.set(h, t);
        cdecl(h, THREAD_START, &[2])
    }

    /// `TkClearVsyncHandler()` (`0x00888830`): `TkSetVsyncHandler(0)`.
    pub fn clear_vsync_handler(h: &mut impl Host) -> u32 {
        cdecl(h, TK_SET_VSYNC_HANDLER, &[0])
    }

    /// `escMenuSetAudioByConfig(mode, timeout)` (`0x0088ec70`): a tail
    /// jump to `sceGsSyncPath` with its caller's two words.
    pub fn esc_menu_set_audio_by_config(h: &mut impl Host, mode: u32, timeout: u32) -> u32 {
        cdecl(h, SCE_GS_SYNC_PATH, &[mode, timeout])
    }

    /// `TkSignalSemaBGThread()` (`0x00888820`): `SignalSema` on the
    /// background thread's semaphore word; returns its result.
    pub fn signal_sema_bg_thread(h: &mut impl Host) -> u32 {
        let sema = BG_THREAD_SEMA.get(h);
        cdecl(h, SIGNAL_SEMA, &[sema])
    }

    /// The array offsets of a menu record's render header (`+0x14`,
    /// `+0x1c`, `+0x10`, `+0x18`, `+0x0c` in that order).
    const MULTI: [(u32, u32); 5] = [
        (0x14, 0x14c),
        (0x1c, 0x44c),
        (0x10, 0x4dc),
        (0x18, 0x71c),
        (0x0c, 0x89c),
    ];
    const SINGLE: [(u32, u32); 5] = [
        (0x14, 0x14c),
        (0x1c, 0x18c),
        (0x10, 0x198),
        (0x18, 0x1c8),
        (0x0c, 0x1e8),
    ];

    fn record_init(mem: &mut impl Mem, record: u32, multi: bool) -> u32 {
        let header = at(record, 0xe0);
        mem.write(at(record, 0x94), header);
        mem.write(at(header, 4), 0u32);
        mem.write(at(header, 8), 0u32);
        for (field, off) in if multi { MULTI } else { SINGLE } {
            mem.write(at(header, field), at(record, off));
        }
        mem.write(at(header, 0x24), 1u32);
        let name_at = mem.rebase(if multi {
            MULTI_TEXTURE_NAME
        } else {
            SINGLE_TEXTURE_NAME
        });
        let name: u32 = mem.read(name_at);
        mem.write(at(header, 0x20), name);
        mem.write(header, 1u32);
        mem.write(at(record, 0x90), 1u32);
        mem.write(at(record, 0xb0), u32::from(multi));
        mem.write(at(record, 0xb4), 0u16);
        for i in 0..16u32 {
            let one = i % 5 == 0;
            mem.write(at(record, 4 * i), if one { 1f32 } else { 0f32 }.to_bits());
        }
        let (lo_at, hi_at) = (mem.rebase(BOUND_MIN), mem.rebase(BOUND_MAX));
        let lo: u32 = mem.read(lo_at);
        let hi: u32 = mem.read(hi_at);
        for k in 0..3 {
            mem.write(at(record, 0x98 + 4 * k), lo);
            mem.write(at(record, 0xa4 + 4 * k), hi);
        }
        mem.write(at(record, 0xb8), 2u32);
        mem.write(at(header, 0x34), 0u32);
        mem.write(at(header, 0x50), 0u32);
        record
    }

    /// `MenuRecord_InitMulti(record)` (`0x0067c030`, stdcall): returns it.
    pub fn record_init_multi(mem: &mut impl Mem, record: u32) -> u32 {
        record_init(mem, record, true)
    }

    /// `MenuRecord_InitSingle(record)` (`0x0067c2d0`, stdcall): returns it.
    pub fn record_init_single(mem: &mut impl Mem, record: u32) -> u32 {
        record_init(mem, record, false)
    }

    /// One buffer of `0x0067c440`: `PhyreMalloc(n * 16, 8)` unless the
    /// capacity is `n` already; the old data freed when the manager owned
    /// it (capacity not negative) and it differs from the new.
    fn ensure(h: &mut impl Host, this: u32, cap_at: u32, n: u32) {
        if h.read::<u32>(at(this, cap_at)) == n {
            return;
        }
        let p = cdecl(h, PHYRE_MALLOC, &[n * 16, 8]);
        if p == 0 {
            return;
        }
        let old: u32 = h.read(at(this, cap_at + 4));
        let cap: u32 = h.read(at(this, cap_at));
        if old != p && cap >> 31 == 0 && old != 0 {
            cdecl(h, PHYRE_FREE, &[old]);
        }
        h.write(at(this, cap_at + 4), p);
        h.write(at(this, cap_at), n);
    }

    /// `MenuTextures_ResetBuffers(this)` (`0x0067c440`, `this` in `ecx`):
    /// the four buffers at their sizes and cleared, the texture names
    /// loaded (`0x0067c7a0` on [`MENU_TEXTURE_MANAGER`]), the map part
    /// overrides set up, and the block `+0x15f0` (0x15e0 bytes) cleared.
    /// Returns `memset`'s `eax`.
    pub fn reset_buffers(h: &mut impl Host, this: u32) -> u32 {
        for (cap_at, n) in [(0x2bd0, 600), (0x2bd8, 800), (0x5828, 600), (0x5830, 800)] {
            ensure(h, this, cap_at, n);
        }
        for (data_at, bytes) in [
            (0x2bd4, 0x2580),
            (0x2bdc, 0x3200),
            (0x582c, 0x2580),
            (0x5834, 0x3200),
        ] {
            let p: u32 = h.read(at(this, data_at));
            cdecl(h, MEMSET, &[p, 0, bytes]);
        }
        let manager = MENU_TEXTURE_MANAGER.get(h);
        h.call_sym(MENU_TEXTURES_LOAD_NAMES, Conv::Thiscall(manager), &[]);
        h.call_sym(MAP_PART_INIT_LANG_OVERRIDES, Conv::Thiscall(this), &[]);
        cdecl(h, MEMSET, &[at(this, 0x15f0), 0, 0x15e0])
    }

    pub const SG_REALLOC_D: u32 = 0x0083_f030;
    pub const MEMCLEAR: u32 = 0x0062_f4e0;
    pub const GU_CONTROL_SET_TEXT: u32 = 0x0084_9610;
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const GU_WINDOW_OPEN: u32 = 0x0084_b250;
    pub const GU_GET_FADE: u32 = 0x0084_8f10;
    pub const SG_MOUSE_MAIN: u32 = 0x0085_1920;
    pub const SG_CLOCK_TICK: u32 = 0x0084_ef10;
    /// The close box's label (`"x"`-like, in `.rdata`).
    pub const CLOSE_LABEL: u32 = 0x00b5_639c;
    /// The open windows, front first.
    pub const WINDOW_LIST: Global<u32> = Global::new(0x0130_3734);

    /// An embedded control's reset after `memclear`: its label points at
    /// its own buffer (`+0x28` = `+0x30`, empty), not hidden or disabled,
    /// `+0x26` 0.
    fn control_clear(mem: &mut impl Mem, c: u32) {
        mem.write(at(c, 0x28), at(c, 0x30));
        mem.write(at(c, 0x30), 0u8);
        let f: u32 = mem.read(at(c, 0x40));
        mem.write(at(c, 0x40), f & !3);
        mem.write(at(c, 0x26), 0u16);
    }

    /// The idle colours of a rectangle record (`+0x18` four 0x80 bytes,
    /// `+0x1c` two more, then `+0x20` 0): what both the window and the
    /// control constructors store with three overlapping writes.
    fn idle_colours(mem: &mut impl Mem, p: u32) {
        mem.write(at(p, 0x18), 0x8080_8080u32);
        mem.write(at(p, 0x1e), 0x8080u32);
        mem.write(at(p, 0x1c), 0x8080u16);
    }

    /// `Gu_WindowAddControl(window, id, text, x, y, w, h)` (`0x0084bae0`):
    /// grows the window's control array by one (`SgReallocD`; controls
    /// whose `+0x26` is 0 get their inline label pointer `+0x28` moved with
    /// it), clears the new control, its id, its rectangle (three copies,
    /// moved by the client origin `+0x80`/`+0x82`), idle colours, and its
    /// text (`Gu_ControlSetText`). Returns the control.
    #[allow(clippy::too_many_arguments)]
    pub fn window_add_control(
        h: &mut impl Host,
        window: u32,
        id: u32,
        text: u32,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
    ) -> u32 {
        let count: i32 = h.read(at(window, 0x90));
        let size = (count.wrapping_add(1) as u32).wrapping_mul(0x44);
        let old: u32 = h.read(at(window, 0x94));
        let list = cdecl(h, SG_REALLOC_D, &[old, size]);
        h.write(at(window, 0x94), list);
        let mut i = 0i32;
        while i < h.read::<i32>(at(window, 0x90)) {
            let c = at(list, (i as u32).wrapping_mul(0x44));
            if h.read::<u16>(at(c, 0x26)) == 0 {
                h.write(at(c, 0x28), at(c, 0x30));
            }
            i += 1;
        }
        let n: u32 = h.read(at(window, 0x90));
        let list: u32 = h.read(at(window, 0x94));
        let c = at(list, n.wrapping_mul(0x44));
        h.write(at(window, 0x90), n.wrapping_add(1));
        cdecl(h, MEMCLEAR, &[c, 0x44]);
        control_clear(h, c);
        h.write(at(c, 0x2e), id as u16);
        let cx = i16_at(h, window, 0x80).wrapping_add(x as i32) as u16;
        let cy = i16_at(h, window, 0x82).wrapping_add(y as i32) as u16;
        for (o, v) in [(0, cx), (2, cy), (4, w as u16), (6, ht as u16)] {
            for k in [0, 8, 0x10] {
                h.write(at(c, o + k), v);
            }
        }
        idle_colours(h, c);
        h.write(at(c, 0x22), 0u16);
        cdecl(h, GU_CONTROL_SET_TEXT, &[c, text]);
        c
    }

    /// `Gu_WindowAddButton(window, id, text, x, y, w, h)` (`0x0084a470`):
    /// a control of kind 1. Returns it.
    #[allow(clippy::too_many_arguments)]
    pub fn window_add_button(
        h: &mut impl Host,
        window: u32,
        id: u32,
        text: u32,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
    ) -> u32 {
        let c = window_add_control(h, window, id, text, x, y, w, ht);
        h.write(at(c, 0x2c), 1u16);
        c
    }

    /// `Gu_WindowAddStatic(window, id, text, x, y, w, h)` (`0x0084a4d0`):
    /// a control of kind 2. Returns it.
    #[allow(clippy::too_many_arguments)]
    pub fn window_add_static(
        h: &mut impl Host,
        window: u32,
        id: u32,
        text: u32,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
    ) -> u32 {
        let c = window_add_control(h, window, id, text, x, y, w, ht);
        h.write(at(c, 0x2c), 2u16);
        c
    }

    /// `Gu_WindowAddClient(window, id, x, y, w, h)` (`0x0084a4a0`): a
    /// control of kind 3 without text. Returns it.
    pub fn window_add_client(
        h: &mut impl Host,
        window: u32,
        id: u32,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
    ) -> u32 {
        let c = window_add_control(h, window, id, 0, x, y, w, ht);
        h.write(at(c, 0x2c), 3u16);
        c
    }

    /// `Gu_VSyncMain()` (`0x0084a460`): `Sg_MouseMain()`, then the clock
    /// tick `0x0084ef10` (the original's tail call); returns its `eax`.
    pub fn vsync_main(h: &mut impl Host) -> u32 {
        cdecl(h, SG_MOUSE_MAIN, &[]);
        cdecl(h, SG_CLOCK_TICK, &[])
    }

    /// `Gu_WindowSetLocalTrans(window, on, fade)` (`0x0084b620`): flag 0x80
    /// from bit 0 of `on`, the window's fade `+0x8c` (`u16`) from `fade`
    /// or, when -1, the GUI's. Returns the fade.
    pub fn window_set_local_trans(h: &mut impl Host, window: u32, on: u32, fade: u32) -> u32 {
        let f: u32 = h.read(at(window, 0x88));
        h.write(at(window, 0x88), f ^ (((on << 7) ^ f) & 0x80));
        let v = if fade == u32::MAX {
            cdecl(h, GU_GET_FADE, &[])
        } else {
            fade
        };
        h.write(at(window, 0x8c), v as u16);
        v
    }

    /// `Gu_WindowCreate(x, y, w, h, flags, callback)` (`0x0084a920`): a new
    /// window (`user_malloc(0x9c)`, cleared, its embedded title control at
    /// `+0x34` reset), placed at `(x, y)` or, while an open window sits
    /// exactly there, 16 right and 8 down (a new column 32 right of the
    /// last at the top when it would leave the 512x416 screen, back to 0
    /// past the right edge). Its rectangle (three copies), idle colours,
    /// callback, client origin `(4, 12 with a title bar (flag 2) else 2)`
    /// and size, scale 0x200; opened (`Gu_WindowOpen`) at the front of
    /// the list, with its title bar (flag 2: control 0x8001, kind 6), close
    /// box (flags 2 and 4: 0x8000, kind 8) and size grip (flag 8: 0x8002,
    /// kind 7), then its create message `0x8001`. Returns the window.
    pub fn window_create(
        h: &mut impl Host,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
        flags: u32,
        callback: u32,
    ) -> u32 {
        let (mut x, mut y, mut column) = (x as i32, y as i32, x as i32);
        let (w, ht) = (w as i32, ht as i32);
        let mut n = 0u32;
        let mut p = WINDOW_LIST.get(h);
        while p != 0 {
            p = h.read(p);
            n = n.wrapping_add(1);
        }
        let mut p = WINDOW_LIST.get(h);
        while p != 0 && n != 0 {
            if x == i16_at(h, p, 0xc) && y == i16_at(h, p, 0xe) {
                y = y.wrapping_add(8);
                x = x.wrapping_add(0x10);
                if ht.wrapping_add(y) > 0x1a0 || w.wrapping_add(x) > 0x200 {
                    column = column.wrapping_add(0x20);
                    if w.wrapping_add(column) > 0x200 {
                        column = 0;
                    }
                    y = 0;
                    x = column;
                }
                p = WINDOW_LIST.get(h);
                n -= 1;
            } else {
                p = h.read(p);
            }
        }
        let win = cdecl(h, USER_MALLOC, &[0x9c]);
        cdecl(h, MEMCLEAR, &[win, 0x9c]);
        let title = at(win, 0x34);
        cdecl(h, MEMCLEAR, &[title, 0x44]);
        control_clear(h, title);
        for (o, v) in [(0xe, y), (0xc, x), (0x10, w), (0x12, ht)] {
            for k in [0, 8, 0x10] {
                h.write(at(win, o + k), v as u16);
            }
        }
        idle_colours(h, at(win, 0xc));
        h.write(at(win, 0x7c), callback);
        let list = WINDOW_LIST.get(h);
        h.write(win, list);
        h.write(at(win, 0x80), 4u16);
        h.write(at(win, 0x2e), flags as u16);
        h.write(at(win, 0x8e), 0x200u16);
        let top: u16 = if flags & 2 != 0 { 0xc } else { 2 };
        h.write(at(win, 0x82), top);
        h.write(at(win, 0x84), (w as u16).wrapping_sub(8));
        h.write(at(win, 0x86), (ht as u16).wrapping_sub(top).wrapping_sub(2));
        cdecl(h, GU_WINDOW_OPEN, &[win]);
        WINDOW_LIST.set(h, win);
        if h.read::<u8>(at(win, 0x2e)) & 2 != 0 {
            let (l, t) = (i16_at(h, win, 0x80), i16_at(h, win, 0x82));
            let cw = i16_at(h, win, 0x10);
            let c = window_add_control(
                h,
                win,
                0x8001,
                0,
                l.wrapping_neg() as u32,
                t.wrapping_neg() as u32,
                cw as u32,
                10,
            );
            h.write(at(c, 0x2c), 6u16);
        }
        let f = h.read::<u16>(at(win, 0x2e));
        if f & 4 != 0 && f & 2 != 0 {
            let t = i16_at(h, win, 0x82);
            let bx = w.wrapping_sub(i16_at(h, win, 0x80)).wrapping_sub(0x10);
            let label = h.rebase(CLOSE_LABEL);
            let c = window_add_control(
                h,
                win,
                0x8000,
                label,
                bx as u32,
                t.wrapping_neg() as u32,
                0xc,
                8,
            );
            h.write(at(c, 0x2c), 8u16);
        }
        if h.read::<u8>(at(win, 0x2e)) & 8 != 0 {
            let gy = i16_at(h, win, 0x86).wrapping_sub(8);
            let gx = i16_at(h, win, 0x84).wrapping_sub(0x10);
            let c = window_add_control(h, win, 0x8002, 0, gx as u32, gy as u32, 0x10, 8);
            h.write(at(c, 0x2c), 7u16);
        }
        let cb: u32 = h.read(at(win, 0x7c));
        if cb != 0 {
            h.call(cb, Conv::Cdecl, &[win, 0x8001, 0]);
        }
        win
    }
}
