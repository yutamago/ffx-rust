//! M5 `main`: the main loop's debug side (`Sg_MainDebug`: the debug
//! draw list, screen captures to the DECI2 host, the VRAM save check,
//! the debug font upload), the achievement forwarder and the movie
//! frame's draw environment.
//!
//! Most of this only runs with the PS2 debug switches on; the PC build
//! keeps the code. `SystemOnHost` writes the PS2's uncached mirror of its
//! buffers (`| 0x2000_0000`), which on the PC is outside the image: it
//! faults there as the original does.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // registers and points: h, x, y, z, m, v
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `dbgPrintf(fmt, ...)` (cdecl, varargs).
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// `sceDeci2ReqSend(socket, dest)` (cdecl).
    pub const DECI2_SEND: u32 = 0x0062_f690;
    /// `memcpy`'s import thunk (cdecl).
    pub const MEMCPY: u32 = 0x0094_9256;
    /// `MSVCR110!sprintf`'s import slot (cdecl, varargs).
    pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
    /// The DECI2 request: socket, size, two words, the send and receive
    /// buffers' uncached addresses, the reply word (another thread's).
    pub const DECI2_SOCKET: u32 = 0x0130_3d00;
    pub const DECI2_SIZE: u32 = 0x0130_3d04;
    pub const DECI2_WORD08: u32 = 0x0130_3d08;
    pub const DECI2_SEND_PTR: u32 = 0x0130_3d0c;
    pub const DECI2_RECV_PTR: u32 = 0x0130_3d10;
    pub const DECI2_REPLY: u32 = 0x0130_3d14;
    /// The send and receive buffers.
    pub const DECI2_SEND_BUF: u32 = 0x0130_3d40;
    pub const DECI2_RECV_BUF: u32 = 0x0131_3d40;
    /// The PS2's uncached mirror bit.
    pub const UNCACHED: u32 = 0x2000_0000;
    /// `"sceDeci2ReqSend fail, reason code = %08x\n"`,
    /// `"Executed Command failed.\n"`.
    pub const MSG_SEND_FAIL: u32 = 0x00b5_af38;
    pub const MSG_COMMAND_FAIL: u32 = 0x00b5_af64;

    /// `SystemOnHost(command, out)` (`0x0084ede0`, cdecl): sends
    /// `command` to the DECI2 host as an `"EH"` request and waits for the
    /// reply word ([`DECI2_REPLY`], set by the DECI2 callback); a negative
    /// reply is the result, a failed command -1, else the reply's payload
    /// is copied to `out` and its length returned. -1 when the send
    /// fails.
    pub fn system_on_host(h: &mut impl Host, command: u32, out: u32) -> i32 {
        let send = h.rebase(DECI2_SEND_BUF) | UNCACHED;
        h.write(h.rebase(DECI2_SEND_PTR), send);
        let recv = h.rebase(DECI2_RECV_BUF) | UNCACHED;
        h.write(h.rebase(DECI2_RECV_PTR), recv);
        h.write(h.rebase(DECI2_SIZE), 0u32);
        h.write(h.rebase(DECI2_WORD08), 0u32);
        h.write(h.rebase(DECI2_REPLY), 0u32);
        // the command after the 12-byte header, its 0 and one more
        let mut len = 0u32;
        loop {
            let c: u8 = h.read(command + len);
            h.write(send + 0xc + len, c);
            if c == 0 {
                break;
            }
            len += 1;
        }
        h.write(send + 0xc + len + 1, 0u8);
        h.write(h.rebase(DECI2_SIZE), len + 0xd);
        let size: u32 = h.read(h.rebase(DECI2_SIZE));
        #[allow(clippy::cast_possible_truncation)] // the low half, `mov [edi], ax`
        h.write(send, size as u16);
        h.write(send + 2, 0xe000_0000u32);
        h.write(send + 6, 0x4845u16);
        h.write(send + 8, 0u32);
        let socket: u32 = h.read(h.rebase(DECI2_SOCKET));
        let r = h.call_sym(DECI2_SEND, Conv::Cdecl, &[socket, 0x48]);
        if r.cast_signed() < 0 {
            let fmt = h.rebase(MSG_SEND_FAIL);
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[fmt, r]);
            return -1;
        }
        let reply_at = h.rebase(DECI2_REPLY);
        loop {
            let v: u32 = h.read(reply_at);
            if v != 0 {
                if v.cast_signed() < 0 {
                    return h.read::<u32>(reply_at).cast_signed();
                }
                break;
            }
            std::hint::spin_loop();
        }
        let p: u32 = h.read(h.rebase(DECI2_RECV_PTR));
        let n = i32::from(h.read::<u16>(p)) - 0xc;
        if h.read::<u32>(p + 8) != 0 {
            let fmt = h.rebase(MSG_COMMAND_FAIL);
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[fmt]);
            return -1;
        }
        h.call_sym(MEMCPY, Conv::Cdecl, &[out, p + 0xc, n.cast_unsigned()]);
        n
    }

    /// `Sg_GetUserName()` (cdecl).
    pub const GET_USER_NAME: u32 = 0x0082_0610;
    /// `Sg_CaptureToFile(name)` (cdecl).
    pub const CAPTURE_TO_FILE: u32 = 0x0083_a690;
    /// `SystemOnHost` (cdecl).
    pub const SYSTEM_ON_HOST: u32 = 0x0084_ede0;
    /// The capture state: frames left (counts down per file), frames to
    /// skip, every frame on, the scene number, the file number, a pending
    /// host-directory request, the five name bytes.
    pub const CAPTURE_LEFT: u32 = 0x0130_1804;
    pub const CAPTURE_SKIP: u32 = 0x0130_1808;
    pub const CAPTURE_EVERY: u32 = 0x0130_1810;
    pub const CAPTURE_SCENE: u32 = 0x0130_180c;
    pub const CAPTURE_FILE: u32 = 0x0130_1814;
    pub const CAPTURE_MKDIR: u32 = 0x00c4_97d4;
    pub const CAPTURE_NAME: u32 = 0x0130_1944;
    /// The format strings: two host commands, the user's file name, the
    /// plain file name, and the two log lines.
    pub const FMT_HOST1: u32 = 0x00b5_9cc0;
    pub const FMT_HOST2: u32 = 0x00b5_9ce8;
    pub const FMT_USER_FILE: u32 = 0x00b5_9d14;
    pub const FMT_FILE: u32 = 0x00b5_9d50;
    pub const FMT_LOG: u32 = 0x00b1_2ad0;
    pub const FMT_EVERY: u32 = 0x00b5_9d88;
    pub const FMT_LAST: u32 = 0x00b5_9da0;
    /// The name buffer's size in the original's frame (`ebp - 0x204`).
    pub const CAPTURE_BUF: u32 = 0x200;

    fn sprintf(h: &mut impl Host, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(SPRINTF_IAT));
        h.call(f, Conv::Cdecl, args)
    }

    /// The capture name's words after the format (and the user name):
    /// the scene, the five name bytes (signed), the file number.
    fn capture_tail(h: &impl Host) -> [u32; 7] {
        let mut w = [0; 7];
        w[0] = h.read(h.rebase(CAPTURE_SCENE));
        let name = h.rebase(CAPTURE_NAME);
        for (k, slot) in (0u32..).zip(&mut w[1..6]) {
            let b: i8 = h.read(name + k);
            *slot = i32::from(b).cast_unsigned();
        }
        w[6] = h.read(h.rebase(CAPTURE_FILE));
        w
    }

    /// `Sg_Capture()` (`0x0083a270`, cdecl): with [`CAPTURE_LEFT`] set,
    /// names this frame's capture file in `buf` (the original's 0x200
    /// bytes at `ebp - 0x204`; with a user name, first the two host
    /// commands that make its directory, once), then writes it
    /// (`Sg_CaptureToFile`) every frame ([`CAPTURE_EVERY`]) or when the
    /// skip count runs out (one fewer left); 1 when it ran, else 0.
    pub fn capture(h: &mut impl Host, buf: u32) -> u32 {
        if h.read::<u32>(h.rebase(CAPTURE_LEFT)) == 0 {
            return 0;
        }
        if h.call_sym(GET_USER_NAME, Conv::Cdecl, &[]) == 0 {
            let t = capture_tail(h);
            let fmt = h.rebase(FMT_FILE);
            let mut args = vec![buf, fmt];
            args.extend(t);
            sprintf(h, &args); // call width: 9 (buf, format, seven words)
        } else {
            let mkdir = h.rebase(CAPTURE_MKDIR);
            if h.read::<u32>(mkdir) != 0 {
                for fmt in [FMT_HOST1, FMT_HOST2] {
                    let user = h.call_sym(GET_USER_NAME, Conv::Cdecl, &[]);
                    let fmt = h.rebase(fmt);
                    sprintf(h, &[buf, fmt, user]);
                    let log = h.rebase(FMT_LOG);
                    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[log, buf]);
                    h.call_sym(SYSTEM_ON_HOST, Conv::Cdecl, &[buf, buf]);
                }
                h.write(mkdir, 0u32);
            }
            let t = capture_tail(h);
            let user = h.call_sym(GET_USER_NAME, Conv::Cdecl, &[]);
            let fmt = h.rebase(FMT_USER_FILE);
            let mut args = vec![buf, fmt, user];
            args.extend(t);
            sprintf(h, &args); // call width: 10 (buf, format, user, seven words)
        }
        let file = h.rebase(CAPTURE_FILE);
        let every = h.read::<u32>(h.rebase(CAPTURE_EVERY)) != 0;
        let skip_at = h.rebase(CAPTURE_SKIP);
        let skip: u32 = h.read(skip_at);
        if !every && skip != 0 {
            h.write(skip_at, skip - 1);
            return 1;
        }
        let fmt = h.rebase(if every { FMT_EVERY } else { FMT_LAST });
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[fmt, buf]);
        h.call_sym(CAPTURE_TO_FILE, Conv::Cdecl, &[buf]);
        let n: u32 = h.read(file);
        h.write(file, n.wrapping_add(1));
        if !every {
            let left = h.rebase(CAPTURE_LEFT);
            let n: u32 = h.read(left);
            h.write(left, n.wrapping_sub(1));
        }
        1
    }

    /// `Sg_SaveResourceCheck`'s request flag and countdown.
    pub const SAVE_REQUEST: u32 = 0x0130_1bec;
    pub const SAVE_COUNTDOWN: u32 = 0x0130_1ff0;
    /// `Sg_SaveVramResource()` (cdecl).
    pub const SAVE_VRAM: u32 = 0x0083_fd60;

    /// `Sg_SaveResourceCheck()` (`0x0083fd20`, cdecl): a request
    /// ([`SAVE_REQUEST`], cleared) starts a 30-frame countdown; when it
    /// reaches 0 the VRAM is saved (`Sg_SaveVramResource`, a tail jump,
    /// whose `eax` it returns); else the count left.
    pub fn save_resource_check(h: &mut impl Host) -> u32 {
        let req = h.rebase(SAVE_REQUEST);
        let cd = h.rebase(SAVE_COUNTDOWN);
        let n = if h.read::<u32>(req) != 0 {
            h.write(cd, 0x1eu32);
            0x1e
        } else {
            h.read(cd)
        };
        h.write(req, 0u32);
        if n == 0 {
            return 0;
        }
        h.write(cd, n - 1);
        if n - 1 != 0 {
            return n - 1;
        }
        h.call_sym(SAVE_VRAM, Conv::Cdecl, &[])
    }

    /// The GS packet cursor `Sg_Gs*` write at (`index`, `base`).
    pub const GS_INDEX: u32 = 0x0130_32bc;
    pub const GS_BASE: u32 = 0x0130_32c4;

    /// `Sg_GsXYZF2L(x, y, z)` (`0x008421b0`, cdecl): one 16-byte `XYZF2`
    /// register write at the cursor, `x` and `y` as halves; the register
    /// is `XYZ2` (0x4) for `z >= 0`, else `XYZ3` (0xc, no kick).
    pub fn gs_xyzf2l(h: &mut impl Host, x: u32, y: u32, z: i32) {
        let at = h.rebase(GS_INDEX);
        let i: u32 = h.read(at);
        let p = (i << 4).wrapping_add(h.read(h.rebase(GS_BASE)));
        h.write(p, crate::lo16(x));
        h.write(p + 2, crate::lo16(y));
        h.write(p + 4, 0u32);
        h.write(at, i.wrapping_add(1));
        h.write(p + 8, if z < 0 { 0xcu32 } else { 4 });
        h.write(p + 12, 0u32);
    }

    /// The debug draw list (`Sg_DbDraw`'s): the start, and where the last
    /// draw began (its first byte is cleared each frame).
    pub const DB_LIST: u32 = 0x0130_32cc;
    pub const DB_LAST: u32 = 0x0130_32d0;
    /// `g_SgDebugOn` (byte): the debug draw and the GUI.
    pub const DEBUG_ON: u32 = 0x0133_c910;
    /// Callees of `Sg_DbDraw` (all cdecl).
    pub const UNIT_MATRIX: u32 = 0x006e_d9a0;
    pub const SCREEN_MATRIX: u32 = 0x0083_5b70;
    pub const CAMERA_MATRIX: u32 = 0x0083_5ba0;
    pub const MUL_MATRIX: u32 = 0x006e_d5c0;
    pub const PACKET_PTR: u32 = 0x0083_b650;
    pub const GS_SET_PACKET: u32 = 0x0084_1cf0;
    pub const GS_START: u32 = 0x0084_1d10;
    pub const GS_TEST: u32 = 0x0084_1d80;
    pub const GS_PRIM: u32 = 0x0084_1a60;
    pub const GS_RGBAQ: u32 = 0x0084_1aa0;
    pub const GS_XYZF2L: u32 = 0x0084_21b0;
    pub const ROT_TRANS_PERS: u32 = 0x006e_d840;
    pub const GS_END: u32 = 0x0084_1770;
    /// `"Sg_DbDraw: unknown command %d\n"`-style message for a bad
    /// command byte.
    pub const MSG_BAD_COMMAND: u32 = 0x00b5_a6a0;
    /// `Sg_DbDraw`'s frame below `ebp`: the matrix at `-0x44`, the vector
    /// at `-0x54`, the projected point at `-0x64`.
    pub const DB_FRAME: u32 = 0x64;

    /// `Sg_DbDraw()` (`0x008412c0`, cdecl): with the debug switch on, the
    /// view matrix (camera x screen), then the list's commands as GS
    /// writes: 1 `PRIM` (a half), 2 `RGBAQ` (three bytes, q 0x80), 3 a
    /// point (three `f32` and 1.0) projected, 4 a screen point (two
    /// halves); 0 ends it. An unknown command is logged and read again,
    /// forever (as the original). `ebp` is the original's frame
    /// ([`DB_FRAME`] bytes below it).
    pub fn db_draw(h: &mut impl Host, ebp: u32) {
        if h.read::<u8>(h.rebase(DEBUG_ON)) == 0 {
            return;
        }
        let m = ebp - 0x44;
        let v = ebp - 0x54;
        let out = ebp - 0x64;
        h.call_sym(UNIT_MATRIX, Conv::Cdecl, &[m]);
        let screen = h.call_sym(SCREEN_MATRIX, Conv::Cdecl, &[]);
        let camera = h.call_sym(CAMERA_MATRIX, Conv::Cdecl, &[screen]);
        h.call_sym(MUL_MATRIX, Conv::Cdecl, &[m, camera, screen]);
        let last: u32 = h.read(h.rebase(DB_LAST));
        h.write(last, 0u8);
        let packet = h.call_sym(PACKET_PTR, Conv::Cdecl, &[]);
        h.call_sym(GS_SET_PACKET, Conv::Cdecl, &[packet]);
        h.call_sym(GS_START, Conv::Cdecl, &[]);
        h.call_sym(GS_TEST, Conv::Cdecl, &[1, 1, 0, 0, 0, 0, 0, 0]);
        let mut p: u32 = h.read(h.rebase(DB_LIST));
        loop {
            let op: u8 = h.read(p);
            match op {
                0 => break,
                1 => {
                    let prim: u16 = h.read(p + 2);
                    h.call_sym(GS_PRIM, Conv::Cdecl, &[u32::from(prim)]);
                    p += 4;
                }
                2 => {
                    let [_, r, g, b]: [u8; 4] = h.read::<u32>(p).to_le_bytes();
                    let rgb = [r, g, b].map(u32::from);
                    h.call_sym(GS_RGBAQ, Conv::Cdecl, &[rgb[0], rgb[1], rgb[2], 0x80]);
                    p += 4;
                }
                3 => {
                    for k in 0..3 {
                        let f: u32 = h.read(p + 4 + 4 * k);
                        h.write(v + 4 * k, x87::store(x87::load(f)));
                    }
                    h.write(v + 12, 1.0f32.to_bits());
                    h.call_sym(ROT_TRANS_PERS, Conv::Cdecl, &[out, m, v, 0]);
                    let xyz: [u32; 3] = [h.read(out), h.read(out + 4), h.read(out + 8)];
                    h.call_sym(GS_XYZF2L, Conv::Cdecl, &xyz);
                    p += 0x10;
                }
                4 => {
                    let x: u16 = h.read(p + 2);
                    let y: u16 = h.read(p + 4);
                    h.call_sym(GS_XYZF2L, Conv::Cdecl, &[x.into(), y.into(), 0]);
                    p += 8;
                }
                _ => {
                    let fmt = h.rebase(MSG_BAD_COMMAND);
                    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[fmt, u32::from(op)]);
                }
            }
        }
        h.call_sym(GS_END, Conv::Cdecl, &[]);
        let start: u32 = h.read(h.rebase(DB_LIST));
        h.write(h.rebase(DB_LAST), start);
    }

    /// `Sg_IdProject_00845030`'s frame below `ebp` (the projected point
    /// at `-0x14`).
    pub const PROJECT_FRAME: u32 = 0x14;

    /// `0x00845030(out, v, m)` (cdecl; name ours, `Sg_IdProject`): `v`
    /// projected through `m` (`FFXVu0RotTransPers(ebp - 0x14, m, v, 0)`);
    /// a point in front (`z >= 0`) inside x 0x6600..=0x9a00 and y
    /// 0x7000..=0x9000 goes to `out` (`w` 0) and gives 0, else 1.
    pub fn id_project(h: &mut impl Host, ebp: u32, out: u32, v: u32, m: u32) -> u32 {
        let p = ebp - 0x14;
        h.call_sym(ROT_TRANS_PERS, Conv::Cdecl, &[p, m, v, 0]);
        let x: i32 = h.read(p);
        let y: i32 = h.read(p + 4);
        let z: i32 = h.read(p + 8);
        if z < 0 || !(0x6600..=0x9a00).contains(&x) || !(0x7000..=0x9000).contains(&y) {
            return 1;
        }
        h.write(out, x);
        h.write(out + 4, y);
        h.write(out + 8, z);
        h.write(out + 12, 0u32);
        0
    }

    /// `FlushCache`, `sceGsSyncPath`, `sceGsExecLoadImage` (cdecl).
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const GS_SYNC_PATH: u32 = 0x0062_f880;
    pub const GS_LOAD_IMAGE: u32 = 0x0062_f7a0;
    /// The debug font's TIM (`Send_DebugFont`).
    pub const DEBUG_FONT: u32 = 0x0132_5170;

    /// `Load1TIM(tim)` (`0x0083f800`, cdecl): the image (`+0x10`, its
    /// transfer at `+8`) and the palette (`+0x60`, at `+0xc`) loaded to
    /// the GS, each after a path sync; the last sync's `eax`.
    pub fn load_1tim(h: &mut impl Host, tim: u32) -> u32 {
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        let mut r = h.call_sym(GS_SYNC_PATH, Conv::Cdecl, &[0, 0]);
        for (data, xfer) in [(0x10, 8), (0x60, 0xc)] {
            let x: u32 = h.read(tim + xfer);
            h.call_sym(GS_LOAD_IMAGE, Conv::Cdecl, &[tim + data, x]);
            r = h.call_sym(GS_SYNC_PATH, Conv::Cdecl, &[0, 0]);
        }
        r
    }

    /// `Send_DebugFont()` (`0x008510b0`, cdecl): `Load1TIM` of the debug
    /// font; its `eax`.
    pub fn send_debug_font(h: &mut impl Host) -> u32 {
        let tim = h.rebase(DEBUG_FONT);
        h.call_sym(LOAD_1TIM, Conv::Cdecl, &[tim])
    }
    /// `Load1TIM` (cdecl).
    pub const LOAD_1TIM: u32 = 0x0083_f800;

    /// Callees of `Sg_MainDebug` (cdecl).
    pub const ID_DRAW_MAIN: u32 = 0x0084_4a80;
    pub const COUNTERS_FLUSH: u32 = 0x0082_1a40;
    pub const DB_DRAW: u32 = 0x0084_12c0;
    pub const CAPTURE_GUI_HIDE: u32 = 0x0083_a620;
    pub const GU_MAIN: u32 = 0x0084_9850;
    pub const SAVE_RESOURCE_CHECK: u32 = 0x0083_fd20;
    pub const CAPTURE: u32 = 0x0083_a270;
    pub const SET_TK_ANTI_XYB: u32 = 0x0083_b110;
    /// The GUI switch (byte) and its packet word.
    pub const GUI_ON: u32 = 0x012f_bbb3;
    pub const GUI_PACKET: u32 = 0x023c_bbf4;

    /// `Sg_MainDebug()` (`0x008207f0`, cdecl): the ID draw, counter 4
    /// flushed; with the debug switch on, the debug draw list, the GUI
    /// (unless a capture hides it, and with [`GUI_ON`]) and the VRAM
    /// save check, then the capture, after which the anti-aliasing is
    /// turned off for the frame (`Sg_SetTkAntiXYB(0, 1, 0x80)`).
    pub fn main_debug(h: &mut impl Host) {
        h.call_sym(ID_DRAW_MAIN, Conv::Cdecl, &[]);
        h.call_sym(COUNTERS_FLUSH, Conv::Cdecl, &[4]);
        if h.read::<u8>(h.rebase(DEBUG_ON)) == 0 {
            return;
        }
        h.call_sym(DB_DRAW, Conv::Cdecl, &[]);
        if h.call_sym(CAPTURE_GUI_HIDE, Conv::Cdecl, &[]) == 0
            && h.read::<u8>(h.rebase(GUI_ON)) != 0
        {
            let packet = h.call_sym(PACKET_PTR, Conv::Cdecl, &[]);
            let w: u32 = h.read(h.rebase(GUI_PACKET));
            h.call_sym(GU_MAIN, Conv::Cdecl, &[w, packet]);
            h.call_sym(SAVE_RESOURCE_CHECK, Conv::Cdecl, &[]);
        }
        if h.call_sym(CAPTURE, Conv::Cdecl, &[]) != 0 {
            h.call_sym(SET_TK_ANTI_XYB, Conv::Cdecl, &[0, 1, 0x80]);
        }
    }

    /// `SaveDataIsSaveEnabled()` and `SaveDataUnlockAchievement(id)`
    /// (cdecl).
    pub const SAVE_ENABLED: u32 = 0x0064_80e0;
    pub const UNLOCK: u32 = 0x0064_88c0;

    /// `achievementUnlockAchievement(id)` (`0x00822380`, cdecl): when
    /// saving is enabled, a tail jump to `SaveDataUnlockAchievement`
    /// with the caller's `id`; its `eax`, else 0.
    pub fn unlock_achievement(h: &mut impl Host, id: u32) -> u32 {
        let r = h.call_sym(SAVE_ENABLED, Conv::Cdecl, &[]);
        if r == 0 {
            return r;
        }
        h.call_sym(UNLOCK, Conv::Cdecl, &[id])
    }

    /// The movie frame's draw state: the flags, the frame's two
    /// dimensions, its size word and its packet.
    pub const MOVIE_FLAGS: u32 = 0x023c_bbf0;
    pub const MOVIE_W: u32 = 0x023c_bbe0;
    pub const MOVIE_H: u32 = 0x023c_bbe4;
    pub const MOVIE_SIZE: u32 = 0x012f_fafc;
    pub const MOVIE_PACKET: u32 = 0x012f_fb00;
    /// Its `Sg_Gs*` callees (cdecl).
    pub const GS_FRAME1: u32 = 0x0084_1870;
    pub const GS_ZBUF1: u32 = 0x0084_2000;
    pub const GS_XYOFFSET1: u32 = 0x0084_1b90;
    pub const GS_SCISSOR_ON1: u32 = 0x0084_22f0;
    pub const GS_FRAME2: u32 = 0x0084_18f0;
    pub const GS_ZBUF2: u32 = 0x0084_2070;
    pub const GS_XYOFFSET2: u32 = 0x0084_1c40;
    pub const GS_SCISSOR_ON2: u32 = 0x0084_2350;
    pub const GS_KICK: u32 = 0x0084_1d60;

    /// `Sg_DrawSetEnvironmentInitMovie()` (`0x0082caa0`, cdecl): both
    /// contexts' frame (base = the size word >> 11; the size is 0 with
    /// flag bit 0, else width x height), z buffer, offset and scissor
    /// for the movie frame, written and kicked; the path sync's `eax`.
    pub fn draw_set_environment_init_movie(h: &mut impl Host) -> u32 {
        let size_at = h.rebase(MOVIE_SIZE);
        let size = if h.read::<u8>(h.rebase(MOVIE_FLAGS)) & 1 != 0 {
            0
        } else {
            let w: u32 = h.read(h.rebase(MOVIE_W));
            let ht: u32 = h.read(h.rebase(MOVIE_H));
            w.wrapping_mul(ht)
        };
        h.write(size_at, size);
        let packet = h.rebase(MOVIE_PACKET);
        h.call_sym(GS_SET_PACKET, Conv::Cdecl, &[packet]);
        h.call_sym(GS_START, Conv::Cdecl, &[]);
        for (frame, zbuf, offset, scissor) in [
            (GS_FRAME1, GS_ZBUF1, GS_XYOFFSET1, GS_SCISSOR_ON1),
            (GS_FRAME2, GS_ZBUF2, GS_XYOFFSET2, GS_SCISSOR_ON2),
        ] {
            let base = h.read::<u32>(size_at) >> 11;
            h.call_sym(frame, Conv::Cdecl, &[base, 0]);
            h.call_sym(zbuf, Conv::Cdecl, &[0, 0]);
            let w: u32 = h.read(h.rebase(MOVIE_W));
            let ht: u32 = h.read(h.rebase(MOVIE_H));
            h.call_sym(offset, Conv::Cdecl, &[0, ht, 0, w]);
            h.call_sym(scissor, Conv::Cdecl, &[1]);
        }
        h.call_sym(GS_END, Conv::Cdecl, &[]);
        h.call_sym(GS_KICK, Conv::Cdecl, &[]);
        h.call_sym(GS_SYNC_PATH, Conv::Cdecl, &[0, 0])
    }
}
