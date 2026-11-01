//! M5 stage 1, round 6 (list `left6`, from `map2.left`), batch 2: more
//! debug map tool windows: the layer selector, the FP memory window, the
//! number entry, the map scroll; and the FP effect model save.
//!
//! As [`crate::l6_map`]: callees stay calls to the originals (cdecl), stack
//! cookies are left out, frames come in as the original's `ebp`, and the
//! window procedures' `None` is the caller's (or the cookie's) `eax`,
//! which Gu's dispatch drops.

pub use crate::l6_map::{
    ADD_ITEMS, FORMAT_D, GU_PRINTF, MEMSET, PRINT_SET_X, PRINT_SET_Y, RCUI_WORK, SET_TEXT,
    SPRINTF_SLOT, YN_MAP_SYSTEM,
};

/// `0x0193c0f8` (next to `g_RcuiWork`): the FP memory window's holder,
/// cleared on its close.
pub const FP_MEM_HOLDER: u32 = 0x0193_c0f8;
/// The layer, scroll and entry windows' item lists.
pub const LAYER_ITEMS: u32 = 0x00c6_05e0;
pub const SCROLL_ITEMS: u32 = 0x00c5_f5d8;
pub const ENTRY_ITEMS: u32 = 0x00c6_0eb0;
/// `pppFpGetHeapUsed`, `pppFpGetHeapMaxUseSize`, `pppFpGetHeapSize`,
/// `pppFpGetPacketUsed`, `pppFpGetPacketPeak`, `pppFpGetPacketSize`.
pub const FP_STATS: [u32; 6] = [
    0x0072_92d0,
    0x0072_9280,
    0x0072_9290,
    0x0072_93f0,
    0x0072_93d0,
    0x0072_93e0,
];
/// `rcuiPercentPrint_00930ba0(v)` (an f32 by value).
pub const PERCENT_PRINT: u32 = 0x0093_0ba0;
/// f64 100.0.
pub const HUNDRED: u32 = 0x00b9_22e0;
/// The FP memory window's texts: the heap's and the packets' labels, the
/// colon and the KiB format.
pub const FP_HEAP_LABEL: u32 = 0x00b6_87dc;
pub const FP_COLON: u32 = 0x00b4_85cc;
pub const FP_KB_FORMAT: u32 = 0x00b6_87e0;
pub const FP_PACKET_LABEL: u32 = 0x00b6_87e8;
/// `Gu_WindowGetArg(window)`, `rcuiValueSet(p, kind, v)`,
/// `Gu_WindowCloseTree(window)`, `Gu_PacketBeginTag(window)`,
/// `Gu_SetPrim(p)`, `Gu_SetRgbaq(r, g, b, a)`, `Gu_SetXyzf2(x, y)`,
/// `Gu_PacketEndTag()`.
pub const WINDOW_GET_ARG: u32 = 0x0084_aec0;
pub const VALUE_SET: u32 = 0x0093_c270;
pub const WINDOW_CLOSE_TREE: u32 = 0x0084_ab80;
pub const PACKET_BEGIN_TAG: u32 = 0x0084_8530;
pub const SET_PRIM: u32 = 0x0084_8480;
pub const SET_RGBAQ: u32 = 0x0084_84c0;
pub const SET_XYZF2: u32 = 0x0084_8680;
pub const PACKET_END_TAG: u32 = 0x0084_8460;
/// `MSVCR110` `atoi`, `atof` slots; the entry's format.
pub const ATOI_SLOT: u32 = 0x00b0_c40c;
pub const ATOF_SLOT: u32 = 0x00b0_c408;
pub const ENTRY_FORMAT: u32 = 0x00b6_8e3c;
/// `rcuiMouseMoveReset()`, `rcuiMouseMoveBegin(window, id)`,
/// `rcuiMouseMoveEnd()`, `rcuiMouseMoveIsID(window)`,
/// `rcuiMouseMoveGet(dx, dy)`.
pub const MOUSE_MOVE_RESET: u32 = 0x0093_c110;
pub const MOUSE_MOVE_BEGIN: u32 = 0x0093_c150;
pub const MOUSE_MOVE_END: u32 = 0x0093_c050;
pub const MOUSE_MOVE_IS_ID: u32 = 0x0093_c120;
pub const MOUSE_MOVE_GET: u32 = 0x0093_c060;
/// The scroll window's two print formats (x; y and zoom).
pub const SCROLL_X_FORMAT: u32 = 0x00b6_8548;
pub const SCROLL_Y_FORMAT: u32 = 0x00b6_8550;
/// `yiGetRenderStatus()`, `sceOpen(name, mode)`, `sceWrite(fd, buf, n)`,
/// `sceClose(fd)`, `rcPrint(fmt, ...)`, `rcbgChunkTag(i)`,
/// `Yn_SceneCountObjects(kind)`, `Yn_SceneGetObject(kind, n)`,
/// `rcPad16(n)`, `rcbgTrxSave2_009399f0(t, fd)`, `SystemOnHost(cmd, out)`.
pub const GET_RENDER_STATUS: u32 = 0x0090_8f10;
pub const SCE_OPEN: u32 = 0x0062_fa30;
pub const SCE_WRITE: u32 = 0x0062_fdc0;
pub const SCE_CLOSE: u32 = 0x0062_f610;
pub const RC_PRINT: u32 = 0x0092_7550;
pub const CHUNK_TAG: u32 = 0x0092_1b90;
pub const SCENE_COUNT_OBJECTS: u32 = 0x0092_1c20;
pub const SCENE_GET_OBJECT: u32 = 0x0092_1be0;
pub const PAD16: u32 = 0x0092_b2c0;
pub const TRX_SAVE2: u32 = 0x0093_99f0;
pub const SYSTEM_ON_HOST: u32 = 0x0084_ede0;
/// `rcbgFpSaveSub`'s path formats (the render status' bit 0x1000 picks
/// the first), its error texts, the host command and its log line.
pub const FP_SAVE_PATHS: [u32; 2] = [0x00b6_8974, 0x00b6_7cf0];
pub const FP_SAVE_ERROR: [u32; 3] = [0x00b6_7878, 0x00b6_892c, 0x00b6_899c];
pub const FP_SAVE_COMMAND: u32 = 0x00b6_8a3c;
pub const FP_SAVE_LOG: u32 = 0x00b6_8a4c;

/// Frames (`ebp` minus the lowest local): the layer selector's, the FP
/// memory window's, the scroll window's, `rcbgFpSaveSub`'s.
pub const LAYER_FRAME: u32 = 0x14;
pub const FP_MEM_FRAME: u32 = 0x14;
pub const SCROLL_FRAME: u32 = 0x14;
pub const FP_SAVE_FRAME: u32 = 0x340;

pub mod bridge {
    use super::{
        ADD_ITEMS, ATOF_SLOT, ATOI_SLOT, CHUNK_TAG, ENTRY_FORMAT, ENTRY_ITEMS, FORMAT_D, FP_COLON,
        FP_HEAP_LABEL, FP_KB_FORMAT, FP_MEM_HOLDER, FP_PACKET_LABEL, FP_SAVE_COMMAND,
        FP_SAVE_ERROR, FP_SAVE_LOG, FP_SAVE_PATHS, FP_STATS, GET_RENDER_STATUS, GU_PRINTF, HUNDRED,
        LAYER_ITEMS, MEMSET, MOUSE_MOVE_BEGIN, MOUSE_MOVE_END, MOUSE_MOVE_GET, MOUSE_MOVE_IS_ID,
        MOUSE_MOVE_RESET, PACKET_BEGIN_TAG, PACKET_END_TAG, PAD16, PERCENT_PRINT, PRINT_SET_X,
        PRINT_SET_Y, RC_PRINT, RCUI_WORK, SCE_CLOSE, SCE_OPEN, SCE_WRITE, SCENE_COUNT_OBJECTS,
        SCENE_GET_OBJECT, SCROLL_ITEMS, SCROLL_X_FORMAT, SCROLL_Y_FORMAT, SET_PRIM, SET_RGBAQ,
        SET_TEXT, SET_XYZF2, SPRINTF_SLOT, SYSTEM_ON_HOST, TRX_SAVE2, VALUE_SET, WINDOW_CLOSE_TREE,
        WINDOW_GET_ARG, YN_MAP_SYSTEM,
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

    /// The word at `a + off`.
    fn at(h: &impl Host, a: u32, off: u32) -> u32 {
        h.read(a.wrapping_add(off))
    }

    /// `fild dword` then `fstp dword`: an `i32` rounded to `f32`.
    fn int_to_f32(v: u32) -> u32 {
        x87::store(f64::from(v.cast_signed()))
    }

    /// `rcbgChunkTag(i)`'s first word.
    fn tag_word(h: &mut impl Host, i: u32) -> u32 {
        let tag = cdecl(h, CHUNK_TAG, &[i]);
        h.read(tag)
    }

    /// `x` clamped to `0..=hi` (signed; below 0 first, as the original).
    fn clamp_to(x: u32, hi: u32) -> u32 {
        if x.cast_signed() < 0 {
            0
        } else if x.cast_signed() > hi.cast_signed() {
            hi
        } else {
            x
        }
    }

    /// `FUN_00936840(window, msg, param)`: the layer selector window's
    /// procedure (`ebp` the original's frame: the number at `ebp - 0x14`).
    /// Needs `g_RcuiWork` and a map with a layer list (`[map + 8]`; else
    /// `None`: the cookie's `eax`); the selected layer is
    /// `g_RcuiWork + 0xae0` (entries of 0xc0 bytes at `[list + 8]`, count
    /// `[list + 4]`). 0x8001 adds its items; 0x8003 (closed) clears
    /// `+0x48`; 0x8006/0x800a on button 0xb7 step the layer up/down,
    /// wrapping (the wrap writes `g_RcuiWork` read again and leaves it in
    /// `eax`); 0x8011 writes the layer number into control 0xb7 and prints
    /// the entry's `+4` and `+8` at (0x36, 0xf) and (0x36, 0x1c).
    pub fn layer_proc(
        h: &mut impl Host,
        ebp: u32,
        window: u32,
        msg: u32,
        param: u32,
    ) -> Option<u32> {
        let w = RCUI_WORK.get(h);
        if w == 0 {
            return None;
        }
        let sys = YN_MAP_SYSTEM.get(h);
        if sys == 0 {
            return None;
        }
        let map: u32 = h.read(sys);
        if map == 0 {
            return None;
        }
        let list = at(h, map, 8);
        if list == 0 {
            return None;
        }
        let k = at(h, w, 0xae0);
        let ent = k
            .wrapping_mul(3)
            .wrapping_shl(6)
            .wrapping_add(at(h, list, 8));
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(LAYER_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items])
            }
            0x8006 | 0x800a => {
                if param != 0xb7 {
                    return Some(if msg == 0x8006 { 2 } else { 3 });
                }
                let n = at(h, list, 4);
                if msg == 0x8006 {
                    let up = k.wrapping_add(1);
                    h.write(w.wrapping_add(0xae0), up);
                    if up.cast_signed() < n.cast_signed() {
                        return Some(up);
                    }
                    let w = RCUI_WORK.get(h);
                    h.write(w.wrapping_add(0xae0), 0_u32);
                    w
                } else {
                    let down = k.wrapping_sub(1);
                    h.write(w.wrapping_add(0xae0), down);
                    if down.cast_signed() >= 0 {
                        return Some(down);
                    }
                    let w = RCUI_WORK.get(h);
                    h.write(w.wrapping_add(0xae0), n.wrapping_sub(1));
                    w
                }
            }
            0x8003 => {
                h.write(w.wrapping_add(0x48), 0_u32);
                1
            }
            0x8011 => {
                let buf = ebp.wrapping_sub(0x14);
                let fmt = h.rebase(FORMAT_D);
                import(h, SPRINTF_SLOT, &[buf, fmt, k]);
                cdecl(h, SET_TEXT, &[window, 0xb7, buf]);
                let mut r = 0;
                for (y, off) in [(0xf_u32, 4_u32), (0x1c, 8)] {
                    cdecl(h, PRINT_SET_X, &[0x36]);
                    cdecl(h, PRINT_SET_Y, &[y]);
                    let v = at(h, ent, off);
                    let fmt = h.rebase(FORMAT_D);
                    r = cdecl(h, GU_PRINTF, &[fmt, v]);
                }
                r
            }
            _ if i <= 0x10 => 5,
            _ => i,
        })
    }

    /// `FUN_00933aa0(window, msg; slots)`: the FP memory window's
    /// procedure. 0x8003 (closed) clears `0x0193c0f8`; 0x8011 prints the
    /// effect heap's and packet buffer's use and peak as percentages of
    /// their sizes (`rcuiPercentPrint_00930ba0`; a size of 0 counts as 1)
    /// and the sizes in KiB (`/ 1024` toward zero). `ebp` is the
    /// original's frame and `slots` its argument words: it keeps its
    /// counts and floats there (`ebp - 0x14..`, the `msg` slot), which the
    /// bridge leaves as the original does. Other messages leave
    /// `msg - 0x8011`.
    pub fn fp_mem_proc(h: &mut impl Host, ebp: u32, slots: u32, msg: u32) -> u32 {
        match msg {
            0x8003 => {
                let holder = h.rebase(FP_MEM_HOLDER);
                h.write(holder, 0_u32);
                return 0;
            }
            0x8011 => {}
            _ => return msg.wrapping_sub(0x8011),
        }
        let slot = slots.wrapping_add(4);
        let mut st = [0_u32; 6];
        for (v, f) in st.iter_mut().zip(FP_STATS) {
            *v = cdecl(h, f, &[]);
        }
        let [heap_used, heap_peak, heap_size, pk_used, pk_peak, pk_size] = st;
        let heap_size = if heap_size == 0 { 1 } else { heap_size };
        let pk_size = if pk_size == 0 { 1 } else { pk_size };
        h.write(ebp.wrapping_sub(4), pk_size);
        let hundred = f64::from_bits(h.read(h.rebase(HUNDRED)));
        let percent = |part: u32, whole: u32| -> u32 {
            x87::store(x87::mul(
                x87::div(x87::load(part), x87::load(whole)),
                hundred,
            ))
        };
        let kib = |n: u32| -> u32 { (n.cast_signed() / 1024).cast_unsigned() };
        let mut r = 0;
        for (y, label, used, peak, size, used_at, peak_at) in [
            (
                0_u32,
                FP_HEAP_LABEL,
                heap_used,
                heap_peak,
                heap_size,
                8_u32,
                0xc_u32,
            ),
            (9, FP_PACKET_LABEL, pk_used, pk_peak, pk_size, 0x10, 0x14),
        ] {
            cdecl(h, PRINT_SET_X, &[0]);
            cdecl(h, PRINT_SET_Y, &[y]);
            let label = h.rebase(label);
            cdecl(h, GU_PRINTF, &[label]);
            cdecl(h, PRINT_SET_X, &[0x12]);
            let whole = int_to_f32(size);
            let pu = percent(int_to_f32(used), whole);
            h.write(ebp.wrapping_sub(used_at), pu);
            cdecl(h, PERCENT_PRINT, &[pu]);
            cdecl(h, PRINT_SET_X, &[0x2d]);
            let colon = h.rebase(FP_COLON);
            cdecl(h, GU_PRINTF, &[colon]);
            cdecl(h, PRINT_SET_X, &[0x36]);
            let p = int_to_f32(peak);
            h.write(ebp.wrapping_sub(peak_at), p);
            let pp = percent(p, whole);
            h.write(slot, pp);
            cdecl(h, PERCENT_PRINT, &[pp]);
            cdecl(h, PRINT_SET_X, &[0x51]);
            let fmt = h.rebase(FP_KB_FORMAT);
            r = cdecl(h, GU_PRINTF, &[fmt, kib(size)]);
        }
        r
    }

    /// The number entry's sign key: the text shifted one byte left (a
    /// leading `-` dropped; the length read again after each byte, since
    /// the copy can reach it) or right with a `-` put in front. Returns
    /// what the original leaves in `eax`: the index where the left shift
    /// stopped, or the last byte the right shift moved.
    fn entry_sign(h: &mut impl Host, e: u32, len: u32) -> u32 {
        let text = e.wrapping_add(0xc);
        let len_at = e.wrapping_add(0x1c);
        if h.read::<u8>(text) == b'-' {
            let mut j = 0_u32;
            loop {
                let c: u8 = h.read(text.wrapping_add(j).wrapping_add(1));
                h.write(text.wrapping_add(j), c);
                j = j.wrapping_add(1);
                if j.cast_signed() >= at(h, e, 0x1c).cast_signed() {
                    break;
                }
            }
            let n = at(h, e, 0x1c);
            h.write(len_at, n.wrapping_sub(1));
            j
        } else {
            let mut c = len;
            let mut last;
            loop {
                c = c.wrapping_sub(1);
                last = h.read::<u8>(text.wrapping_add(c).wrapping_add(1));
                h.write(text.wrapping_add(c).wrapping_add(2), last);
                if c.cast_signed() < 0 {
                    break;
                }
            }
            let n = at(h, e, 0x1c);
            h.write(len_at, n.wrapping_add(1));
            h.write(text, b'-');
            u32::from(last)
        }
    }

    /// The number entry's digit key `d`.
    fn entry_digit(h: &mut impl Host, e: u32, d: u32) -> u32 {
        let text = e.wrapping_add(0xc);
        let len_at = e.wrapping_add(0x1c);
        if at(h, e, 0x20) != 0 {
            h.write(text, 0x30_u16);
            h.write(len_at, 0_u32);
            h.write(e.wrapping_add(0x24), 0_u32);
            h.write(e.wrapping_add(0x20), 0_u32);
        }
        let len = at(h, e, 0x1c);
        if len.cast_signed() >= 10 {
            return len;
        }
        if at(h, e, 0x24) != 0 {
            if len == 0 {
                h.write(len_at, 1_u32);
            }
            let n = at(h, e, 0x1c);
            h.write(text.wrapping_add(n), b'.');
            let n = at(h, e, 0x1c);
            h.write(len_at, n.wrapping_add(1));
            h.write(e.wrapping_add(0x24), 0_u32);
        }
        let n = at(h, e, 0x1c);
        #[allow(clippy::cast_possible_truncation)] // the original stores `cl`
        h.write(text.wrapping_add(n), (d + 0x30) as u8);
        let n = at(h, e, 0x1c);
        h.write(text.wrapping_add(n).wrapping_add(1), 0_u8);
        let n = at(h, e, 0x1c);
        if n.cast_signed() <= 0 && d == 0 {
            return n;
        }
        h.write(len_at, n.wrapping_add(1));
        n.wrapping_add(1)
    }

    /// `FUN_0093c3f0(window, msg, param; slots)`: the number entry
    /// window's procedure. The entry is the window's argument
    /// (`Gu_WindowGetArg`): `+0` shown, `+4` the target, `+8` its kind
    /// (0xb a float), the text at `+0xc` (its length `+0x1c`), `+0x20`
    /// start afresh, `+0x24` a point pending, `+0x28` entered. 0x8001 adds
    /// its items; 0x8003 hides it; 0x8007 a key (`param`): digits 0..9 (up
    /// to 10 characters, a pending point first), 0xa the point (floats
    /// only), 0xb the sign, 0xc clears; 0x8006 button 0xe enters the text
    /// (`atof` into the target as f32, through the `msg` slot, or `atoi`
    /// through `rcuiValueSet`) and 0xf cancels, both closing the window;
    /// 0x8011 draws the box and the text. `eax` as the original: the entry
    /// where nothing else sets it, the key table's index, the length or
    /// the sign shift's last value.
    pub fn entry_proc(h: &mut impl Host, slots: u32, window: u32, msg: u32, param: u32) -> u32 {
        let e = cdecl(h, WINDOW_GET_ARG, &[window]);
        let text = e.wrapping_add(0xc);
        match msg {
            0x8001 => {
                let items = h.rebase(ENTRY_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items])
            }
            0x8003 => {
                h.write(e, 0_u32);
                e
            }
            0x8007 => match param {
                0..=9 => entry_digit(h, e, param),
                0xa => {
                    if at(h, e, 8) == 0xb {
                        h.write(e.wrapping_add(0x24), 1_u32);
                    }
                    1
                }
                0xb => {
                    let len = at(h, e, 0x1c);
                    if len.cast_signed() <= 0 {
                        return 2;
                    }
                    entry_sign(h, e, len)
                }
                0xc => {
                    h.write(text, 0x30_u16);
                    h.write(e.wrapping_add(0x1c), 0_u32);
                    h.write(e.wrapping_add(0x24), 0_u32);
                    3
                }
                _ => e,
            },
            0x8006 => match param {
                0xe | 0xf => {
                    if param == 0xe {
                        if at(h, e, 8) == 0xb {
                            let f: u32 = h.read(h.rebase(ATOF_SLOT));
                            let v = h.call_st0(f, Conv::Cdecl, &[text]).to_bits();
                            h.write(slots.wrapping_add(4), v);
                            let target = at(h, e, 4);
                            h.write(target, x87::store(x87::load(v)));
                        } else {
                            let v = import(h, ATOI_SLOT, &[text]);
                            let (target, kind) = (at(h, e, 4), at(h, e, 8));
                            cdecl(h, VALUE_SET, &[target, kind, v]);
                        }
                        h.write(e.wrapping_add(0x28), 1_u32);
                    }
                    cdecl(h, WINDOW_CLOSE_TREE, &[window])
                }
                _ => param.wrapping_sub(0xf),
            },
            0x8011 => {
                if e == 0 {
                    return 0;
                }
                if at(h, e, 0) == 0 {
                    return e;
                }
                cdecl(h, PACKET_BEGIN_TAG, &[window]);
                cdecl(h, SET_PRIM, &[2]);
                cdecl(h, SET_RGBAQ, &[0x50, 0x50, 0x50, 0]);
                for (x, y) in [(2_u32, 0_u32), (0x60, 0), (0x60, 0xa), (2, 0xa), (2, 0)] {
                    cdecl(h, SET_XYZF2, &[x, y]);
                }
                cdecl(h, PACKET_END_TAG, &[]);
                cdecl(h, PRINT_SET_X, &[6]);
                cdecl(h, PRINT_SET_Y, &[2]);
                let fmt = h.rebase(ENTRY_FORMAT);
                cdecl(h, GU_PRINTF, &[fmt, text])
            }
            _ => e,
        }
    }

    /// `FUN_009365c0`'s 0x8006: the scroll step for button `param` and the
    /// clamps; `eax` the x before its clamp.
    fn scroll_step(h: &mut impl Host, ebp: u32, list: u32, param: u32, lim: [u32; 2]) -> u32 {
        let x = at(h, list, 0x5c);
        let mut y = at(h, list, 0x60);
        let mut z = at(h, list, 0x64);
        h.write(ebp.wrapping_sub(4), x);
        h.write(ebp.wrapping_sub(0xc), y);
        let r = match param.wrapping_sub(0x53) {
            0 => {
                z = z.wrapping_sub(1);
                x
            }
            1 => {
                z = z.wrapping_add(1);
                x
            }
            2 => {
                y = y.wrapping_sub(1);
                x
            }
            3 => {
                y = y.wrapping_add(1);
                x
            }
            4 => x.wrapping_sub(1),
            5 => x.wrapping_add(1),
            6 => {
                cdecl(h, MOUSE_MOVE_END, &[]);
                x
            }
            _ => x,
        };
        h.write(list.wrapping_add(0x5c), clamp_to(r, lim[0]));
        h.write(list.wrapping_add(0x60), clamp_to(y, lim[1]));
        let z = if z.cast_signed() < -16 {
            (-16_i32).cast_unsigned()
        } else if z.cast_signed() > 16 {
            16
        } else {
            z
        };
        h.write(list.wrapping_add(0x64), z);
        r
    }

    /// `FUN_009365c0(window, msg, param)`: the map scroll window's
    /// procedure (`ebp` the original's frame: the work area, the x limit,
    /// the y and the mouse deltas). Needs `g_RcuiWork` (else `eax` 0) and
    /// a map (`eax` its holder or 0 without). The scroll limits are the
    /// layers' (`[map + 8]`: `+4` entries of 0xc0 bytes at `+8`) largest
    /// `+4`, `+8` less 0x200 and 0xe0 (0x1c0 with `+0xf8`), at least 0.
    /// 0x8001 adds its items and resets the mouse move; 0x8003 (closed)
    /// clears `+0x24`; 0x8007 button 0x59 starts the mouse move; 0x8006
    /// steps the scroll (`+0x5c` x, `+0x60` y, `+0x64` zoom: buttons
    /// 0x53..0x58; 0x59 ends the move) and clamps (zoom to -16..16);
    /// 0x8011 follows the mouse while moving, then prints the three.
    pub fn scroll_proc(
        h: &mut impl Host,
        ebp: u32,
        window: u32,
        msg: u32,
        param: u32,
    ) -> Option<u32> {
        let w = RCUI_WORK.get(h);
        h.write(ebp.wrapping_sub(4), w);
        if w == 0 {
            return Some(0);
        }
        let sys = YN_MAP_SYSTEM.get(h);
        if sys == 0 {
            return Some(0);
        }
        let map: u32 = h.read(sys);
        if map == 0 {
            return Some(sys);
        }
        let list = at(h, map, 8);
        let n = at(h, list, 4);
        let (mut max_x, mut max_y) = (0_u32, 0_u32);
        if n.cast_signed() > 0 {
            let mut e = at(h, list, 8).wrapping_add(8);
            for _ in 0..n {
                let x = at(h, e, 0_u32.wrapping_sub(4));
                if max_x.cast_signed() < x.cast_signed() {
                    max_x = x;
                }
                let y = at(h, e, 0);
                if max_y.cast_signed() < y.cast_signed() {
                    max_y = y;
                }
                e = e.wrapping_add(0xc0);
            }
        }
        let max_x = max_x.wrapping_sub(0x200);
        let max_y = if at(h, list, 0xf8) != 0 {
            max_y.wrapping_sub(0x1c0)
        } else {
            max_y.wrapping_sub(0xe0)
        };
        let lim_x = if max_x.cast_signed() < 0 { 0 } else { max_x };
        let lim_y = if max_y.cast_signed() < 0 { 0 } else { max_y };
        h.write(ebp.wrapping_sub(8), lim_x);
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(SCROLL_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items]);
                cdecl(h, MOUSE_MOVE_RESET, &[])
            }
            0x8003 => {
                h.write(w.wrapping_add(0x24), 0_u32);
                w
            }
            0x8007 => {
                if param != 0x59 {
                    return Some(3);
                }
                cdecl(h, MOUSE_MOVE_BEGIN, &[window, 0x59])
            }
            0x8006 => scroll_step(h, ebp, list, param, [lim_x, lim_y]),
            0x8011 => {
                if cdecl(h, MOUSE_MOVE_IS_ID, &[window]) != 0 {
                    let (dx, dy) = (ebp.wrapping_sub(0x10), ebp.wrapping_sub(0x14));
                    cdecl(h, MOUSE_MOVE_GET, &[dx, dy]);
                    let y = at(h, list, 0x60).wrapping_sub(at(h, dy, 0));
                    let x = at(h, list, 0x5c).wrapping_sub(at(h, dx, 0));
                    h.write(list.wrapping_add(0x5c), clamp_to(x, lim_x));
                    h.write(list.wrapping_add(0x60), clamp_to(y, lim_y));
                }
                let mut r = 0;
                for (y, fmt, off) in [
                    (0xc_u32, SCROLL_Y_FORMAT, 0x64_u32),
                    (0x26, SCROLL_X_FORMAT, 0x5c),
                    (0x30, SCROLL_Y_FORMAT, 0x60),
                ] {
                    cdecl(h, PRINT_SET_X, &[2]);
                    cdecl(h, PRINT_SET_Y, &[y]);
                    let v = at(h, list, off);
                    let fmt = h.rebase(fmt);
                    r = cdecl(h, GU_PRINTF, &[fmt, v]);
                }
                r
            }
            _ if i <= 0x10 => 5,
            _ => i,
        })
    }

    /// `rcbgFpSaveSub(name)` -> the last `rcPrint`'s `eax`: the scene's
    /// effect models (kind 0x20006) saved to one of two host paths +
    /// `name` (`sceOpen` 0x602; a failure prints the error and stops): a
    /// chunk header (tag 0, 16 bytes), per model a header (tag 0xa, its
    /// padded size / 16), its texture data (`rcbgTrxSave2_009399f0`) and
    /// the padding zeros from the frame (`ebp - 0x14`), an end header (tag
    /// 0xc); then the host converts it (`SystemOnHost` on the command built
    /// from the path past its first 5 characters) and the result is logged.
    /// `ebp` is the original's frame (0x340 bytes: the path, the command
    /// and its output, the headers, the counters). A pad past 16 bytes (a
    /// negative size: `rcPad16` gives 17..31) overruns the padding buffer
    /// into the stack cookie, in the original's frame as in the port's.
    pub fn fp_save_sub(h: &mut impl Host, ebp: u32, name: u32) -> u32 {
        let path = ebp.wrapping_sub(0x114);
        let status = cdecl(h, GET_RENDER_STATUS, &[]);
        let fmt = h.rebase(FP_SAVE_PATHS[usize::from(status & 0x1000 == 0)]);
        import(h, SPRINTF_SLOT, &[path, fmt, name]);
        let fd = cdecl(h, SCE_OPEN, &[path, 0x602]);
        h.write(ebp.wrapping_sub(0x33c), fd);
        if fd == u32::MAX {
            let [a, b, c] = FP_SAVE_ERROR.map(|t| h.rebase(t));
            cdecl(h, RC_PRINT, &[a, b, 0x266]);
            return cdecl(h, RC_PRINT, &[c]);
        }
        let hdr = ebp.wrapping_sub(0x324);
        for k in 0..4_u32 {
            h.write(hdr.wrapping_add(4 * k), 0_u32);
        }
        let t = tag_word(h, 0);
        h.write(hdr, t);
        cdecl(h, SCE_WRITE, &[fd, hdr, 0x10]);
        let n = cdecl(h, SCENE_COUNT_OBJECTS, &[0x20006]);
        h.write(ebp.wrapping_sub(0x340), n);
        h.write(ebp.wrapping_sub(0x338), 0_u32);
        let pad_buf = ebp.wrapping_sub(0x14);
        let h2 = ebp.wrapping_sub(0x334);
        let mut k = 0_u32;
        while k.cast_signed() < n.cast_signed() {
            let obj = cdecl(h, SCENE_GET_OBJECT, &[0x20006, k]);
            let a = at(h, obj, 0x94)
                .wrapping_mul(at(h, obj, 0x90))
                .wrapping_add(0x50);
            let size = at(h, obj, 0x54)
                .wrapping_mul(at(h, obj, 0x50))
                .wrapping_add(a);
            let pad = cdecl(h, PAD16, &[size]);
            for j in 0..4_u32 {
                h.write(h2.wrapping_add(4 * j), 0_u32);
            }
            let t = tag_word(h, 0xa);
            h.write(h2, t);
            let total = pad.wrapping_add(size).cast_signed() / 16;
            h.write(h2.wrapping_add(4), total.cast_unsigned());
            cdecl(h, SCE_WRITE, &[fd, h2, 0x10]);
            cdecl(h, TRX_SAVE2, &[obj, fd]);
            cdecl(h, MEMSET, &[pad_buf, 0, pad]);
            cdecl(h, SCE_WRITE, &[fd, pad_buf, pad]);
            k = k.wrapping_add(1);
            h.write(ebp.wrapping_sub(0x338), k);
        }
        let t = tag_word(h, 0xc);
        h.write(hdr, t);
        cdecl(h, SCE_WRITE, &[fd, hdr, 0x10]);
        cdecl(h, SCE_CLOSE, &[fd]);
        let cmd = ebp.wrapping_sub(0x214);
        let fmt = h.rebase(FP_SAVE_COMMAND);
        import(h, SPRINTF_SLOT, &[cmd, fmt, ebp.wrapping_sub(0x10f)]);
        let r = cdecl(h, SYSTEM_ON_HOST, &[cmd, ebp.wrapping_sub(0x314)]);
        let log = h.rebase(FP_SAVE_LOG);
        cdecl(h, RC_PRINT, &[log, cmd, r])
    }
}
