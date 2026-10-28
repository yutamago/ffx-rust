//! M5 stage 1, list `menu`, batch 4: the Tk platform glue: the CD
//! reader's entry points (through the `GetCD` function table: `+0x00`
//! read, `+0x08` sync, `+0x10` open, `+0x14` size), the memory checks, the
//! vsync callback setup, the software reset, the memory card sync, the
//! DECI2 debug channel (`TkInitDeci2`, its send and `TkRemove`), a pad
//! actuator query and the memory card path builder.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

pub mod bridge {
    use ffx_bridge::Global;
    use ffx_graphic::host::{Conv, Host};

    /// The memory card slot whose load position `0x008884e0` resets.
    pub const MC_SLOT: Global<u32> = Global::new(0x0133_0234);
    /// The vsync thread (`TkSetVsyncHandler`) and the vsync counter.
    pub const VSYNC_THREAD: Global<u32> = Global::new(0x0133_0268);
    pub const VSYNC_COUNT: Global<u32> = Global::new(0x0133_0254);
    /// The DECI2 channel: its socket, the send header, state.
    pub const DECI2_SOCKET: Global<u32> = Global::new(0x0132_f570);
    pub const DECI2_HEADER: Global<u32> = Global::new(0x0132_f574);
    pub const DECI2_WORD78: Global<u32> = Global::new(0x0132_f578);
    pub const DECI2_LEN: Global<u16> = Global::new(0x0132_f57c);
    pub const DECI2_SENDING: Global<u8> = Global::new(0x0132_f580);
    pub const DECI2_BYTE81: Global<u8> = Global::new(0x0132_f581);
    pub const DECI2_WORD84: Global<u32> = Global::new(0x0132_f584);
    pub const DECI2_WORD88: Global<u32> = Global::new(0x0132_f588);
    pub const DECI2_WORD6C: Global<u32> = Global::new(0x0132_d56c);
    /// `TkRemove`'s packet: `+0x0a` the name's length, `+0x0c` the name.
    pub const REMOVE_PACKET: u32 = 0x0133_0700;

    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const SCE_DECI2_OPEN: u32 = 0x0062_f680;
    pub const SCE_DECI2_REQ_SEND: u32 = 0x0062_f690;
    pub const SCE_MC_SYNC: u32 = 0x0062_f9f0;
    pub const SCE_PAD_INFO_ACT: u32 = 0x0062_fb40;
    pub const POST_RESET: u32 = 0x0042_fa70;
    pub const GRAPHIC_SET_SCREEN_FADE_COLOR: u32 = 0x0064_3900;
    pub const GRAPHIC_SET_SCREEN_FADE_LAST_REASON: u32 = 0x0064_3940;
    pub const GRAPHIC_LOCK_SCREEN_FADE_COLOR: u32 = 0x0064_1e20;
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const GET_CD: u32 = 0x0076_d0d0;
    pub const SG_MEM_CHECK: u32 = 0x0083_b7b0;
    pub const SG_MEM_SET_ALIGN: u32 = 0x0083_c4a0;
    pub const TK_IS_DECI2_SEND: u32 = 0x0088_5420;
    pub const DECI2_HANDLER: u32 = 0x0088_52a0;
    pub const TK_IS_DEBUG: u32 = 0x0088_7d10;
    pub const VSYNC_HANDLER: u32 = 0x0088_7d60;
    pub const TK_SYNC_V_DI: u32 = 0x0088_7f20;
    pub const TK_SYNC_V_EI: u32 = 0x0088_7f40;
    pub const TK_MC_GET_LOAD_FILE_POS: u32 = 0x0088_8300;
    pub const TK_MC_SET_LOAD_FILE_POS: u32 = 0x0088_8480;
    pub const TK_SET_VSYNC_HANDLER: u32 = 0x0088_8680;
    pub const TK_GET_TGS: u32 = 0x0088_d6d0;
    /// The import slot of the CRT's `sprintf`.
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    /// `TkMemLock`'s source file name for `Sg_MemCheck`.
    pub const MEM_FILE: u32 = 0x00b5_e968;

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// Entry `slot` (byte offset) of the CD reader's table.
    fn cd_call(h: &mut impl Host, cd: u32, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(at(cd, slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `TkCdRead(file, sector, buf)` (`0x0088d140`): open, size, read
    /// into `buf` and wait, the cache flushed around the read. Returns
    /// the last flush's `eax`.
    pub fn cd_read(h: &mut impl Host, file: u32, sector: u32, buf: u32) -> u32 {
        let cd = cdecl(h, GET_CD, &[]);
        cd_call(h, cd, 0x10, &[file]);
        cd_call(h, cd, 0x14, &[sector]);
        cdecl(h, FLUSH_CACHE, &[0]);
        cd_call(h, cd, 0, &[sector, buf, 0, 0]);
        cd_call(h, cd, 8, &[0]);
        cdecl(h, FLUSH_CACHE, &[0])
    }

    /// `TkCdReadSync(mode)` (`0x0088d1c0`): the reader's sync entry (the
    /// original's tail jump with its caller's word). Returns its `eax`.
    pub fn cd_read_sync(h: &mut impl Host, mode: u32) -> u32 {
        let cd = cdecl(h, GET_CD, &[]);
        cd_call(h, cd, 8, &[mode])
    }

    /// `TkCdReadNB(file, sector, buf)` (`0x0088d190`): open, flush, start
    /// the read without waiting. Returns the read entry's `eax`.
    pub fn cd_read_nb(h: &mut impl Host, file: u32, sector: u32, buf: u32) -> u32 {
        let cd = cdecl(h, GET_CD, &[]);
        cd_call(h, cd, 0x10, &[file]);
        cdecl(h, FLUSH_CACHE, &[0]);
        cd_call(h, cd, 0, &[sector, buf, 0, 0])
    }

    /// `TkCdSize(file, sector)` (`0x0088d390`): open, then the size entry.
    pub fn cd_size(h: &mut impl Host, file: u32, sector: u32) -> u32 {
        let cd = cdecl(h, GET_CD, &[]);
        cd_call(h, cd, 0x10, &[file]);
        cd_call(h, cd, 0x14, &[sector])
    }

    /// `TkCdReadWithMalloc(file, sector)` (`0x0088d1d0`): open, size
    /// rounded up to 2 KB sectors (as a signed division), a buffer
    /// (`Sg_MemSetAlign(0x40)`, `user_malloc`, its first word cleared),
    /// the read and the wait. Returns the buffer.
    pub fn cd_read_with_malloc(h: &mut impl Host, file: u32, sector: u32) -> u32 {
        let cd = cdecl(h, GET_CD, &[]);
        cd_call(h, cd, 0x10, &[file]);
        let n = cd_call(h, cd, 0x14, &[sector]) as i32;
        let size = (n.wrapping_add(0x7ff) / 0x800).wrapping_mul(0x800) as u32;
        cdecl(h, SG_MEM_SET_ALIGN, &[0x40]);
        let buf = cdecl(h, USER_MALLOC, &[size]);
        h.write(buf, 0u32);
        cdecl(h, FLUSH_CACHE, &[0]);
        cd_call(h, cd, 0, &[sector, buf, 0, 0]);
        cd_call(h, cd, 8, &[0]);
        buf
    }

    /// `TkMemLock()` / `TkMemUnlock()` (`0x0088e680` / `0x0088e6b0`): two
    /// `Sg_MemCheck(heap, file, line)` calls, heap 0 then 1, at the
    /// original's `line` and `line + 1`. Returns the second's `eax`.
    pub fn mem_check_pair(h: &mut impl Host, line: u32) -> u32 {
        let file = h.rebase(MEM_FILE);
        cdecl(h, SG_MEM_CHECK, &[0, file, line]);
        cdecl(h, SG_MEM_CHECK, &[1, file, line + 1])
    }

    /// `TkSoftWareReset()` (`0x00872550`): `PostReset(1)`, the screen
    /// fade colour to black opaque (`(0, 0, 0, 1.0)`), its reason 3, then
    /// the colour locked (the original's tail call). Returns its `eax`.
    pub fn software_reset(h: &mut impl Host) -> u32 {
        cdecl(h, POST_RESET, &[1]);
        cdecl(h, GRAPHIC_SET_SCREEN_FADE_COLOR, &[0, 0, 0, 1f32.to_bits()]);
        cdecl(h, GRAPHIC_SET_SCREEN_FADE_LAST_REASON, &[3]);
        cdecl(h, GRAPHIC_LOCK_SCREEN_FADE_COLOR, &[])
    }

    /// `TkInitAllSyncVCallback()` (`0x008886f0`): vsync interrupts off,
    /// no thread, the counter 0, interrupts on, the vsync handler
    /// installed. Returns `TkSetVsyncHandler`'s `eax`.
    pub fn init_all_sync_v_callback(h: &mut impl Host) -> u32 {
        cdecl(h, TK_SYNC_V_DI, &[]);
        VSYNC_THREAD.set(h, u32::MAX);
        VSYNC_COUNT.set(h, 0);
        cdecl(h, TK_SYNC_V_EI, &[]);
        let handler = h.rebase(VSYNC_HANDLER);
        cdecl(h, TK_SET_VSYNC_HANDLER, &[handler])
    }

    /// `TkPadInfoAct(port, slot, actuator)` (`0x00888a40`): 0 for actuator
    /// 0xff, else `scePadInfoAct(port, slot, actuator, 3)`: 8 when it
    /// answers, 1 when not.
    pub fn pad_info_act(h: &mut impl Host, port: u32, slot: u32, act: u32) -> u32 {
        if act == 0xff {
            return 0;
        }
        if cdecl(h, SCE_PAD_INFO_ACT, &[port, slot, act, 3]) != 0 {
            8
        } else {
            1
        }
    }

    /// `TkMcSync(mode, &cmd, &result)` (`0x008884e0`, frame at `ebp`): 1
    /// while `TkGetTGS()`; else `sceMcSync(mode, ebp - 0xc, ebp - 8)`,
    /// whose command and result go to the out pointers when given. A
    /// finished command 1 with a failed result (negative) puts the slot's
    /// load position back (`TkMcSetLoadFilePos(slot, -1)` unless already
    /// negative). Returns `sceMcSync`'s result.
    pub fn mc_sync(h: &mut impl Host, ebp: u32, mode: u32, cmd_out: u32, res_out: u32) -> u32 {
        if cdecl(h, TK_GET_TGS, &[]) != 0 {
            return 1;
        }
        let (cmd_at, res_at) = (ebp.wrapping_sub(0xc), ebp.wrapping_sub(8));
        let r = cdecl(h, SCE_MC_SYNC, &[mode, cmd_at, res_at]);
        let res: u32 = h.read(res_at);
        let cmd: u32 = h.read(cmd_at);
        h.write(ebp.wrapping_sub(4), r);
        if r == 1 && cmd == 1 && (res as i32) < 0 {
            let slot = MC_SLOT.get(h);
            if (cdecl(h, TK_MC_GET_LOAD_FILE_POS, &[slot]) as i32) >= 0 {
                let slot = MC_SLOT.get(h);
                cdecl(h, TK_MC_SET_LOAD_FILE_POS, &[slot, u32::MAX]);
            }
        }
        if cmd_out != 0 {
            h.write(cmd_out, cmd);
        }
        if res_out != 0 {
            h.write(res_out, res);
        }
        r
    }

    /// `strlen` of the C string at `s`.
    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(at(s, n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `TkMcMakePath(dst, name)` (`0x00888030`, frame at `ebp`): `"/"`,
    /// then `sprintf(ebp - 0x400, name, 0)` appended, and the last two
    /// characters replaced by `"??"`. Returns what the original leaves in
    /// `eax`: the formatted length plus one with its low byte cleared.
    pub fn mc_make_path(h: &mut impl Host, ebp: u32, dst: u32, name: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x400);
        let slot = h.rebase(SPRINTF_SLOT);
        let f: u32 = h.read(slot);
        h.call(f, Conv::Cdecl, &[buf, name, 0]);
        h.write(dst, 0x2fu16);
        let n = strlen(h, buf) + 1;
        let end = at(dst, strlen(h, dst));
        for i in 0..n {
            let b = h.read::<u8>(at(buf, i));
            h.write(at(end, i), b);
        }
        let len = strlen(h, dst);
        h.write(at(dst, len.wrapping_sub(2)), 0x3f3fu16);
        n & !0xff
    }

    /// `TkInitDeci2()` (`0x00885240`): the channel's state cleared and
    /// `sceDeci2Open(0xeffa, &socket, handler)` stored as the socket.
    /// Returns it.
    pub fn init_deci2(h: &mut impl Host) -> u32 {
        DECI2_SENDING.set(h, 0);
        DECI2_BYTE81.set(h, 0);
        DECI2_WORD78.set(h, 0);
        let socket_at = h.rebase(0x0132_f570);
        let handler = h.rebase(DECI2_HANDLER);
        let s = cdecl(h, SCE_DECI2_OPEN, &[0xeffa, socket_at, handler]);
        DECI2_SOCKET.set(h, s);
        DECI2_WORD6C.set(h, 0);
        DECI2_WORD84.set(h, 0);
        DECI2_WORD88.set(h, 0);
        s
    }

    /// `TkDeci2Send(packet, len)` (`0x008854c0`): the header
    /// (`packet | 0x20000000`), the length, sending set; the cache
    /// flushed; `sceDeci2ReqSend(socket, packet[7])`. Returns its `eax`.
    pub fn deci2_send(h: &mut impl Host, packet: u32, len: u32) -> u32 {
        let id = u32::from(h.read::<u8>(at(packet, 7)));
        DECI2_HEADER.set(h, packet | 0x2000_0000);
        DECI2_SENDING.set(h, 1);
        DECI2_LEN.set(h, len as u16);
        cdecl(h, FLUSH_CACHE, &[0]);
        let s = DECI2_SOCKET.get(h);
        cdecl(h, SCE_DECI2_REQ_SEND, &[s, id])
    }

    /// `TkRemove(name)` (`0x0088c740`): in debug builds (`TkIsDebug`), the
    /// name copied into the remove packet with its length, then sent
    /// (up to 0x80 tries, each after the previous send ends). Returns what
    /// the original leaves in `eax`: 0, or the send's result.
    pub fn remove(h: &mut impl Host, name: u32) -> u32 {
        if cdecl(h, TK_IS_DEBUG, &[]) == 0 {
            return 0;
        }
        let packet = h.rebase(REMOVE_PACKET);
        let dst = at(packet, 0xc);
        let mut i = 0u32;
        loop {
            let b = h.read::<u8>(at(name, i));
            h.write(at(dst, i), b);
            i = i.wrapping_add(1);
            if b == 0 {
                break;
            }
        }
        let n = strlen(h, dst);
        h.write(at(packet, 0xa), n as u16);
        while cdecl(h, TK_IS_DECI2_SEND, &[]) != 0 {}
        let mut r = 0;
        for _ in 0..0x80 {
            r = deci2_send(h, packet, 0x10c);
            if (r as i32) >= 0 {
                return r;
            }
            loop {
                r = cdecl(h, TK_IS_DECI2_SEND, &[]);
                if r == 0 {
                    break;
                }
            }
        }
        r
    }
}
