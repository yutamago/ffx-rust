//! M5 stage 1, list `vfx`, round 5 batch 16: the ppp packet start
//! (`pppPacketStart_00760f30`): the packet state reset and the crosshair's
//! depth read-back into a world point sent to the effect side.
//!
//! The store-image record, the screen point and the world point the
//! original keeps in its frame come in through its `ebp` (the shim passes a
//! `StackFrame`).

use ffx_bridge::Global;

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x9c;
/// The packet state words the start clears, the packet write position
/// (`g_PppPacketPos`) and its start: an absolute address (the store's
/// immediate has no relocation entry), not rebased.
pub const PACKET_STATE: Global<u32> = Global::new(0x00d4_ca3c);
pub const PACKET_COUNT_LO: Global<u32> = Global::new(0x0230_fbc0);
pub const PACKET_COUNT_HI: Global<u32> = Global::new(0x0230_fbc4);
pub const PACKET_POS: Global<u32> = Global::new(0x0230_fd2c);
pub const PACKET_START: u32 = 0x01b0_0000;
/// The read-back request, the frame buffer switch and its two frame
/// buffer records, the crosshair point (`g_PppCrosshair`).
pub const READ_BACK: Global<u32> = Global::new(0x00d4_b4d4);
pub const BUFFER_SWITCH: Global<u32> = Global::new(0x00d4_e140);
pub const BUFFER_OFF: u32 = 0x00d4_e270;
pub const BUFFER_ON: u32 = 0x00d4_e1b0;
pub const CROSSHAIR_X: Global<i32> = Global::new(0x00d4_b4e8);
pub const CROSSHAIR_Y: Global<i32> = Global::new(0x00d4_b4ec);
/// The read-back pixels, the packet the point goes out in and its header.
pub const PIXELS: u32 = 0x00e5_24f0;
pub const SEND_PACKET: u32 = 0x00e5_24a8;
pub const SEND_HEADER: u32 = 0x00b5_16e4;

// ---- callees ----

/// `sceGsSyncPath(mode, timeout)`, `sceGsSetDefStoreImage(rec, bp, bw,
/// psm, x, y, w, h)`, `FlushCache(n)`, `sceGsExecStoreImage(rec, out)`,
/// `ppp_screen_to_world_007626d0(out, p)`, `pppSendPacket(packet, header,
/// p, n)`, `pppClearWait()` (cdecl).
pub const GS_SYNC_PATH: u32 = 0x0062_f880;
pub const SET_DEF_STORE_IMAGE: u32 = 0x0062_f830;
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
pub const EXEC_STORE_IMAGE: u32 = 0x0062_f7b0;
pub const SCREEN_TO_WORLD: u32 = 0x0076_26d0;
pub const PPP_SEND_PACKET: u32 = 0x0076_b3d0;
pub const PPP_CLEAR_WAIT: u32 = 0x0076_b200;

pub mod bridge {
    use super::{
        BUFFER_OFF, BUFFER_ON, BUFFER_SWITCH, CROSSHAIR_X, CROSSHAIR_Y, EXEC_STORE_IMAGE,
        FLUSH_CACHE, GS_SYNC_PATH, PACKET_COUNT_HI, PACKET_COUNT_LO, PACKET_POS, PACKET_START,
        PACKET_STATE, PIXELS, PPP_CLEAR_WAIT, PPP_SEND_PACKET, READ_BACK, SCREEN_TO_WORLD,
        SEND_HEADER, SEND_PACKET, SET_DEF_STORE_IMAGE,
    };
    use crate::h4_e1::int_f32;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `pppPacketStart_00760f30()`: the packet state cleared and the write
    /// position back at its start; with a read-back requested (cleared
    /// here) the 16 x 16 block at the crosshair (+0x100, +0xd0) stored from
    /// the current frame buffer (its `+0xc` & 0x1ff times 32, `+2` & 0x3f,
    /// PSMZ24 0x31), its first pixel's 24-bit depth with the crosshair
    /// point turned into a world point (`ppp_screen_to_world`) and sent
    /// (16 bytes), then `pppClearWait`. Its `eax` is nothing the callers
    /// read.
    pub fn packet_start(h: &mut impl Host, ebp: u32) {
        PACKET_STATE.set(h, 0);
        PACKET_COUNT_LO.set(h, 0);
        PACKET_COUNT_HI.set(h, 0);
        PACKET_POS.set(h, PACKET_START);
        if READ_BACK.get(h) == 0 {
            return;
        }
        let buffer = h.rebase(if BUFFER_SWITCH.get(h) == 0 {
            BUFFER_OFF
        } else {
            BUFFER_ON
        });
        READ_BACK.set(h, 0);
        cdecl(h, GS_SYNC_PATH, &[0, 0]);
        let rec = ebp - 0x80;
        let bp = u32::from(h.read::<u16>(buffer + 0xc) & 0x1ff) << 5;
        let bw = u32::from(h.read::<u16>(buffer + 2) & 0x3f);
        let x = CROSSHAIR_X.get(h).wrapping_add(0x100).cast_unsigned();
        let y = CROSSHAIR_Y.get(h).wrapping_add(0xd0).cast_unsigned();
        cdecl(
            h,
            SET_DEF_STORE_IMAGE,
            &[rec, bp, bw, 0x31, x, y, 0x10, 0x10],
        );
        cdecl(h, FLUSH_CACHE, &[0]);
        let pixels = h.rebase(PIXELS);
        cdecl(h, EXEC_STORE_IMAGE, &[rec, pixels]);
        cdecl(h, GS_SYNC_PATH, &[0, 0]);
        let depth = h.read::<u32>(pixels) & 0x00ff_ffff;
        let point = ebp - 0x94;
        // fild/fstp as the original (its rounding)
        let xyz = [
            int_f32(CROSSHAIR_X.get(h)),
            int_f32(CROSSHAIR_Y.get(h)),
            int_f32(depth.cast_signed()),
            0,
        ];
        for (k, v) in (0_u32..).zip(xyz) {
            h.write(point + 4 * k, v);
        }
        let world = ebp - 0x14;
        cdecl(h, SCREEN_TO_WORLD, &[world, point]);
        let packet = h.rebase(SEND_PACKET);
        let header = h.rebase(SEND_HEADER);
        cdecl(h, PPP_SEND_PACKET, &[packet, header, world, 0x10]);
        cdecl(h, PPP_CLEAR_WAIT, &[]);
    }
}
