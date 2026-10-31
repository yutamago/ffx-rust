//! M5 stage 1, list `vfx`, round 5 batch 4: the ppp packet layer's PS2
//! remnants: the crosshair's GS packet, the CLUT block upload, and the
//! host RPC command and read the effect data receiver uses
//! (`pppDataRcv`).
//!
//! On PC the GS and SIF calls are stubs; the ports keep the calls.
//! Stack cookies are left out; frame buffers handed to callees come in as
//! addresses (the shims pass a `StackFrame`).

use ffx_bridge::Global;

/// The crosshair's double buffer flag (u16) and its two packets.
pub const CROSSHAIR_FLIP: Global<u16> = Global::new(0x00d4_e370);
pub const CROSSHAIR_PACKETS: [u32; 2] = [0x0230_36e0, 0x0230_16e0];
/// The crosshair's colour/position words (`0x011a0738..`, 4 words).
pub const CROSSHAIR_WORDS: u32 = 0x011a_0738;
/// `g_GsRegisterSink`: the DMA channel word the packets go to.
pub const GS_REGISTER_SINK: u32 = 0x0230_56e8;
/// The RPC send buffer (header `0x010d5640`, text at `+8`), the RPC
/// client (`0x010e7644`) and `g_pppWaitDone`.
pub const RPC_BUFFER: u32 = 0x010d_5640;
pub const RPC_CLIENT: u32 = 0x010e_7644;
pub const WAIT_DONE: Global<u32> = Global::new(0x010e_7640);
/// `rpc_read`'s texts: the end message and the command.
pub const RPC_END_TEXT: u32 = 0x00b5_1bf0;
pub const RPC_READ_COMMAND: u32 = 0x00b5_1c04;

// ---- callees ----

/// `sceGsSyncPath(mode, timeout)`, `sceDmaSync(ch, mode, timeout)`,
/// `sceDmaSend(ch, packet)`, `sceGsSetDefLoadImage(li, dbp, dbw, dpsm,
/// x, y, w, h)`, `FlushCache(n)`, `sceGsExecLoadImage(li, src)` (cdecl).
pub const GS_SYNC_PATH: u32 = 0x0062_f880;
pub const DMA_SYNC: u32 = 0x0062_f730;
pub const DMA_SEND: u32 = 0x0062_f710;
pub const SET_DEF_LOAD_IMAGE: u32 = 0x0062_f820;
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
pub const EXEC_LOAD_IMAGE: u32 = 0x0062_f7a0;
/// `FFXSifCallRpc(client, fno, mode, send, ssize, recv, rsize, end,
/// arg)`, `FFXSifInitRpc(mode)`, `debug_exit_trace(n)`, `scePrintf(fmt)`
/// (cdecl).
pub const SIF_CALL_RPC: u32 = 0x00a4_4750;
pub const SIF_INIT_RPC: u32 = 0x00a4_4780;
pub const DEBUG_EXIT_TRACE: u32 = 0x0062_f510;
pub const SCE_PRINTF: u32 = 0x0062_fbf0;
/// `rpc_send_command_0076b030(rec, text)` (cdecl, this batch).
pub const RPC_SEND_COMMAND: u32 = 0x0076_b030;

pub mod bridge {
    use super::{
        CROSSHAIR_FLIP, CROSSHAIR_PACKETS, CROSSHAIR_WORDS, DEBUG_EXIT_TRACE, DMA_SEND, DMA_SYNC,
        EXEC_LOAD_IMAGE, FLUSH_CACHE, GS_REGISTER_SINK, GS_SYNC_PATH, RPC_BUFFER, RPC_CLIENT,
        RPC_END_TEXT, RPC_READ_COMMAND, RPC_SEND_COMMAND, SCE_PRINTF, SET_DEF_LOAD_IMAGE,
        SIF_CALL_RPC, SIF_INIT_RPC, WAIT_DONE,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `pppCrosshairPacket_00760ce0(x0, y0, x1, y1, w, h)` -> `sceDmaSend`'s
    /// `eax`: the other of the two packets (the flip flag toggled) gets a
    /// GIF packet: a texture flush, a yellow line strip from `(x0, y0)` to
    /// `(x1, y1)` in a `w` x `h` area and the four crosshair words; once
    /// path 1 is idle (`sceGsSyncPath` bit 4 clear), it is sent on the GS
    /// register channel (uncached address).
    pub fn crosshair_packet(
        h: &mut impl Host,
        x0: u32,
        y0: u32,
        x1: u32,
        y1: u32,
        w: u32,
        hh: u32,
    ) -> u32 {
        let flip = CROSSHAIR_FLIP.get(h) ^ 1;
        CROSSHAIR_FLIP.set(h, flip);
        let p = h.rebase(CROSSHAIR_PACKETS[usize::from(flip != 0)]);
        let words: [u32; 28] = [
            2,
            0x1000_0000,
            0xe,
            0,
            w,
            hh,
            1,
            0,
            0x3100_0108,
            0,
            0x4e,
            0,
            1,
            0x2000_c000,
            0x55,
            0,
            x0,
            y0,
            0x00ff_ff00,
            0,
            x1,
            y1,
            0x00ff_ff00,
            0,
            0x8001,
            0x1000_0000,
            0xe,
            0,
        ];
        for (k, v) in words.iter().enumerate() {
            h.write(p + 0x10 + 4 * k as u32, *v);
        }
        let src = h.rebase(CROSSHAIR_WORDS);
        for k in 0..4 {
            let v: u32 = h.read(src + 4 * k);
            h.write(p + 0x80 + 4 * k, v);
        }
        while cdecl(h, GS_SYNC_PATH, &[1, 0]) & 0x10 != 0 {}
        let sink = h.rebase(GS_REGISTER_SINK);
        cdecl(h, DMA_SYNC, &[sink, 0, 0]);
        cdecl(h, DMA_SEND, &[sink, p | 0x8000_0000])
    }

    /// `pppLoadClutBlocks_00762810(src, dbp)` -> the last `sceGsSyncPath`'s
    /// `eax`: 16 blocks of 8 x 2 texels from `src` (0x40 bytes each) loaded
    /// to `dbp` (row `2 i`, column 0 or 8), each through the load-image
    /// record at `li` (the original's 16-aligned `ebp - 0x60`).
    pub fn load_clut_blocks(h: &mut impl Host, li: u32, src: u32, dbp: u32) -> u32 {
        cdecl(h, GS_SYNC_PATH, &[0, 0]);
        let mut at = src;
        for row in 0..8_u32 {
            for col in 0..2_u32 {
                cdecl(
                    h,
                    SET_DEF_LOAD_IMAGE,
                    &[li, dbp, 1, 0, col * 8, row * 2, 8, 2],
                );
                cdecl(h, FLUSH_CACHE, &[0]);
                cdecl(h, EXEC_LOAD_IMAGE, &[li, at]);
                at = at.wrapping_add(0x40);
            }
        }
        cdecl(h, GS_SYNC_PATH, &[0, 0])
    }

    /// `rpc_send_command_0076b030(rec, text)` -> 0: the text (with its
    /// terminator) after an 8-byte header (size = length + 8, 0, the
    /// record's `+4`, "EH") in the send buffer, which goes to the host
    /// (`FFXSifCallRpc` function 1); the record keeps the buffer (`+0x10`)
    /// and size (`+8`); the wait flag is set.
    pub fn rpc_send_command(h: &mut impl Host, rec: u32, text: u32) -> u32 {
        let buf = h.rebase(RPC_BUFFER);
        h.write(rec + 0x10, buf);
        let mut len = 0_u32;
        loop {
            let c: u8 = h.read(text.wrapping_add(len));
            h.write(buf + 8 + len, c);
            if c == 0 {
                break;
            }
            len += 1;
        }
        h.write(rec + 8, len + 8);
        let size: u32 = h.read(rec + 8);
        h.write(buf, size as u16);
        h.write(buf + 2, 0_u16);
        let id: u32 = h.read(rec + 4);
        h.write(buf + 4, id as u16);
        h.write(buf + 6, 0x4845_u16);
        let (send, size): (u32, u32) = (h.read(rec + 0x10), h.read(rec + 8));
        let client = h.rebase(RPC_CLIENT);
        cdecl(h, SIF_CALL_RPC, &[client, 1, 0, send, size, 0, 0, 0, 0]);
        WAIT_DONE.set(h, 1);
        0
    }

    /// The sum of the `bytes / 4` words at `at`.
    fn checksum(h: &impl Host, at: u32, bytes: u32) -> u32 {
        (0..bytes >> 2).fold(0_u32, |s, k| s.wrapping_add(h.read(at.wrapping_add(4 * k))))
    }

    /// `rpc_read(rec)` -> the receive buffer (`+0x14`): a 0x218-byte host
    /// read (`FFXSifCallRpc` function 0). An empty reply returns at once;
    /// an error (`+8` = -1) resets the RPC and stops (`debug_exit_trace`);
    /// `+8` = 0x80 ends the transfer (printed, the wait flag set).
    /// Otherwise the reply's kind, id, size (twice) and checksum go to
    /// `+0x28..+0x38` and the record's start callback (`+0x3c`) decides:
    /// 0 checks the reply's own data (`+0x18`); else the read command is
    /// sent and 0xfc00-byte chunks are read (waiting for each) until the
    /// size is consumed, their data (past the 8-byte header) copied to
    /// `+0x18` (the start of it kept at `+0x1c`). The checksum picks the
    /// done (`+0x40`) or failed (`+0x44`) callback.
    pub fn rpc_read(h: &mut impl Host, rec: u32) -> u32 {
        let client = h.rebase(RPC_CLIENT);
        let (send, recv): (u32, u32) = (h.read(rec + 0x10), h.read(rec + 0x14));
        cdecl(h, SIF_CALL_RPC, &[client, 0, 0, send, 0, recv, 0x218, 0, 0]);
        let e: u32 = h.read(rec + 0x14);
        if h.read::<u16>(e) == 0 {
            return e;
        }
        if h.read::<u32>(e + 8) == 0xffff_ffff {
            cdecl(h, SIF_INIT_RPC, &[0]);
            cdecl(h, DEBUG_EXIT_TRACE, &[1]);
        }
        let kind: u32 = h.read(e + 8);
        if kind == 0x80 {
            let text = h.rebase(RPC_END_TEXT);
            cdecl(h, SCE_PRINTF, &[text]);
            let r: u32 = h.read(rec + 0x14);
            WAIT_DONE.set(h, 1);
            return r;
        }
        h.write(rec + 0x28, kind);
        let id: u32 = h.read(e + 0xc);
        h.write(rec + 0x2c, id);
        let size: u32 = h.read(e + 0x10);
        h.write(rec + 0x30, size);
        h.write(rec + 0x38, size);
        let sum: u32 = h.read(e + 0x14);
        h.write(rec + 0x34, sum);
        let start: u32 = h.read(rec + 0x3c);
        let data = if h.call(start, Conv::Cdecl, &[rec]) == 0 {
            h.read::<u32>(rec + 0x14).wrapping_add(0x18)
        } else {
            let text = h.rebase(RPC_READ_COMMAND);
            cdecl(h, RPC_SEND_COMMAND, &[rec, text]);
            let dst: u32 = h.read(rec + 0x18);
            if dst != 0 {
                h.write(rec + 0x1c, dst);
            }
            loop {
                let chunk = loop {
                    let (send, recv): (u32, u32) = (h.read(rec + 0x10), h.read(rec + 0x14));
                    cdecl(
                        h,
                        SIF_CALL_RPC,
                        &[client, 0, 0, send, 0, recv, 0xfc00, 0, 0],
                    );
                    let recv: u32 = h.read(rec + 0x14);
                    if h.read::<u16>(recv) != 0 {
                        break recv;
                    }
                };
                let n = h.read::<u16>(chunk).wrapping_add(0xfff8);
                h.write(chunk, n);
                let dst: u32 = h.read(rec + 0x18);
                if dst != 0 {
                    let src = h.read::<u32>(rec + 0x14).wrapping_add(8);
                    let words = u32::from(n) >> 2;
                    for k in 0..words {
                        let v: u32 = h.read(src.wrapping_add(4 * k));
                        h.write(dst.wrapping_add(4 * k), v);
                    }
                    h.write(rec + 0x18, dst.wrapping_add(4 * words));
                }
                let left = h
                    .read::<u32>(rec + 0x30)
                    .wrapping_sub(u32::from(h.read::<u16>(chunk)));
                h.write(rec + 0x30, left);
                if left.cast_signed() <= 0 {
                    break;
                }
            }
            h.read(rec + 0x1c)
        };
        let bytes: u32 = h.read(rec + 0x38);
        let sum = checksum(h, data, bytes);
        let done = if sum == h.read::<u32>(rec + 0x34) {
            0x40
        } else {
            0x44
        };
        let f: u32 = h.read(rec + done);
        h.call(f, Conv::Cdecl, &[rec]);
        h.read(rec + 0x14)
    }
}
