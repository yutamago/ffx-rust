//! Continuation round 7 of list `h4-glue-other`, batch 4: the PS2 GS
//! layer's frame start, packet transfer and texture loads.

use ffx_graphic::host::{Conv, Host};

use crate::h4_i1::{DBG_PRINTF, EXEC_LOAD_IMAGE, FLUSH_CACHE, SET_DEF_LOAD_IMAGE, SYNC_PATH};
use crate::h4_i2::{PRIM, SCISSOR, SCISSOR2, SET_PACKET, TEST2, XYOFFSET, XYOFFSET2, XYZF2};

/// Where an aligned frame's `ebp` lands: the original's `push ebx; mov
/// ebx, esp; sub esp, 8; and esp, ~0xf; add esp, 4; push ebp` with `esp`
/// at the call (`entry` the return address's slot).
#[must_use]
pub fn aligned_ebp(entry: u32) -> u32 {
    entry.wrapping_sub(4 + 8) & !0xf
}

// ---- Sg_DrawSetEnvironmentInit

/// `g_SgFrame` (`0x023cbbf0`).
pub const SG_FRAME: u32 = 0x023c_bbf0;
/// The frame buffer's base this frame (`0x012ffafc`).
pub const FRAME_BASE: u32 = 0x012f_fafc;
/// The other frame buffer's base (`0x01300700`).
pub const OTHER_FRAME_BASE: u32 = 0x0130_0700;
/// Set to force a new environment (`0x012ffae1`, a byte).
pub const FORCE_ENV: u32 = 0x012f_fae1;
/// Frames to skip the clear (`0x012ffae0`, a byte).
pub const SKIP_CLEAR: u32 = 0x012f_fae0;
/// The second clear's mode (`0x01300771`, a byte: 2 uses the colour at
/// `0x01300772`..`0x01300774`, signed bytes).
pub const SECOND_CLEAR: u32 = 0x0130_0771;
/// The scratch packet (`0x012ffb00`).
pub const SG_PACKET: u32 = 0x012f_fb00;
/// The screen width (`0x023cbbe4`) and height (`g_GuScreenSize`,
/// `0x023cbbe0`).
pub const SCREEN_W: u32 = 0x023c_bbe4;
/// See [`SCREEN_W`].
pub const SCREEN_H: u32 = 0x023c_bbe0;
/// `yiGetBackColor(rgb)` (cdecl).
pub const GET_BACK_COLOR: u32 = 0x0090_86d0;
/// `Sg_GsStart()` (cdecl).
pub const GS_START: u32 = 0x0084_1d10;
/// `Sg_GsDIMX()` (cdecl).
pub const GS_DIMX: u32 = 0x0084_16f0;
/// `Sg_GsDTHE(on)` (cdecl).
pub const GS_DTHE: u32 = 0x0084_1730;
/// `Sg_GsFRAME(base, mask)` (cdecl).
pub const GS_FRAME: u32 = 0x0084_1870;
/// `Sg_GsZBUF(mask)` (cdecl).
pub const GS_ZBUF: u32 = 0x0084_22f0;
/// `Sg_GsFRAME2(base, mask)` (cdecl).
pub const GS_FRAME2: u32 = 0x0084_18f0;
/// `Sg_GsZBUF2(mask)` (cdecl).
pub const GS_ZBUF2: u32 = 0x0084_2350;
/// `Sg_GsRGBAQ(r, g, b, a)` (cdecl).
pub const GS_RGBAQ: u32 = 0x0084_1aa0;
/// `Sg_GsFRAME2_16(base, mask)` (cdecl).
pub const GS_FRAME2_16: u32 = 0x0084_1970;
/// `Sg_GsEnd()` (cdecl).
pub const GS_END: u32 = 0x0084_1770;
/// `Sg_GsSync()` (cdecl).
pub const GS_SYNC: u32 = 0x0084_1d60;
/// Bytes of [`draw_set_environment_init`]'s frame: the background colour
/// at `ebp - 0x14` (three words).
pub const ENV_INIT_FRAME: u32 = 0xc;

/// `Sg_DrawSetEnvironmentInit()` (`0x0082c8d0`, cdecl; `frame` its
/// background colour): the double buffer's bases for this frame (bit 0 of
/// `g_SgFrame`: 0 and 0x34000), the last mode reset (0xff when forced,
/// which clears the force), then a GS packet: both contexts' frame,
/// offset, scissor (the whole screen) and depth set, and unless clears are
/// skipped (that count goes down instead) the screen cleared to the
/// background colour, and again with frame 0xd0 (the colour at
/// `0x01300772` for mode 2) when the second clear is on. Ends, syncs and
/// returns `sceGsSyncPath`'s `eax`.
pub fn draw_set_environment_init(h: &mut impl Host, frame: u32) -> u32 {
    let odd = h.read::<u32>(h.rebase(SG_FRAME)) & 1 != 0;
    let force_at = h.rebase(FORCE_ENV);
    let force = h.read::<u8>(force_at) != 0;
    let base_at = h.rebase(FRAME_BASE);
    h.write(base_at, if odd { 0u32 } else { 0x3_4000 });
    h.write(
        h.rebase(OTHER_FRAME_BASE),
        if odd { 0x3_4000u32 } else { 0 },
    );
    let last = h.rebase(crate::h4_i2::LAST_SCREEN_MODE);
    if force {
        h.write(force_at, 0u8);
        h.write(last, 0xffu8);
    } else {
        h.write(last, 0u8);
    }
    h.call_sym(GET_BACK_COLOR, Conv::Cdecl, &[frame]);
    let packet = h.rebase(SG_PACKET);
    h.call_sym(SET_PACKET, Conv::Cdecl, &[packet]);
    h.call_sym(GS_START, Conv::Cdecl, &[]);
    h.call_sym(GS_DIMX, Conv::Cdecl, &[]);
    h.call_sym(GS_DTHE, Conv::Cdecl, &[0]);
    let (w_at, h_at) = (h.rebase(SCREEN_W), h.rebase(SCREEN_H));
    let base = h.read::<u32>(base_at) >> 11;
    h.call_sym(GS_FRAME, Conv::Cdecl, &[base, 0]);
    h.call_sym(XYOFFSET, Conv::Cdecl, &[0, 0]);
    let (sw, sh) = (h.read::<u32>(w_at), h.read::<u32>(h_at));
    h.call_sym(SCISSOR, Conv::Cdecl, &[0, sw, 0, sh]);
    h.call_sym(GS_ZBUF, Conv::Cdecl, &[0]);
    let base = h.read::<u32>(base_at) >> 11;
    h.call_sym(GS_FRAME2, Conv::Cdecl, &[base, 0]);
    h.call_sym(XYOFFSET2, Conv::Cdecl, &[0, 0]);
    let (sw, sh) = (h.read::<u32>(w_at), h.read::<u32>(h_at));
    h.call_sym(SCISSOR2, Conv::Cdecl, &[0, sw, 0, sh]);
    h.call_sym(GS_ZBUF2, Conv::Cdecl, &[0]);
    let skip_at = h.rebase(SKIP_CLEAR);
    let skip: u8 = h.read(skip_at);
    if skip == 0 {
        h.call_sym(TEST2, Conv::Cdecl, &[0, 0, 0, 0, 0, 0, 1, 1]);
        h.call_sym(PRIM, Conv::Cdecl, &[0x206]);
        let (r, g, b) = (
            h.read::<u32>(frame),
            h.read::<u32>(frame.wrapping_add(4)),
            h.read::<u32>(frame.wrapping_add(8)),
        );
        h.call_sym(GS_RGBAQ, Conv::Cdecl, &[r, g, b, 0]);
        clear_rect(h, w_at, h_at);
        let second_at = h.rebase(SECOND_CLEAR);
        let mode: u8 = h.read(second_at);
        if mode != 0 {
            if mode == 2 {
                let mut c = [0u32; 3];
                for (k, v) in c.iter_mut().enumerate() {
                    *v = i32::from(h.read::<i8>(second_at.wrapping_add(1 + k as u32))) as u32;
                }
                h.call_sym(GS_RGBAQ, Conv::Cdecl, &[c[0], c[1], c[2], 0]);
            }
            h.call_sym(GS_FRAME2_16, Conv::Cdecl, &[0xd0, 0]);
            clear_rect(h, w_at, h_at);
        }
    } else {
        h.write(skip_at, skip.wrapping_sub(1));
    }
    h.call_sym(GS_END, Conv::Cdecl, &[]);
    h.call_sym(GS_SYNC, Conv::Cdecl, &[]);
    h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0])
}

/// A sprite over the whole screen: `XYZF2(0, 0)`, `XYZF2(w, h)`.
fn clear_rect(h: &mut impl Host, w_at: u32, h_at: u32) {
    h.call_sym(XYZF2, Conv::Cdecl, &[0, 0]);
    let (sw, sh) = (h.read::<u32>(w_at), h.read::<u32>(h_at));
    h.call_sym(XYZF2, Conv::Cdecl, &[sw, sh]);
}

// ---- op_gs1_transfer

/// The packet `op_gs1_transfer` sends (`0x011a0720`, 0xa0 bytes).
pub const GS1_PACKET: u32 = 0x011a_0720;
/// The DMA channel (`0x023328e0`).
pub const DMA_CHANNEL: u32 = 0x0233_28e0;
/// `sceDmaSend(channel, chain)` (cdecl; empty on the PC).
pub const DMA_SEND: u32 = 0x0062_f710;
/// Bytes of [`gs1_transfer`]'s frame, from the aligned `ebp - 0xc0`: a
/// 0x10-byte head the original never writes, then the packet's copy.
pub const GS1_FRAME: u32 = 0xc0;

/// `op_gs1_transfer()` (`0x007eb580`, cdecl; `frame` see [`GS1_FRAME`],
/// 16-byte aligned): the 40 words at `0x011a0720` copied to `frame +
/// 0x10`, a path sync, `FlushCache(0)`, `sceDmaSend(channel, frame)` and
/// a path sync. The frame's first 0x10 bytes are never written (junk
/// read: [ebp-0xc0..-0xb0] never written; 0 (brief 2026-10-24)), but
/// `sceDmaSend` is an empty function on the PC and reads nothing. Returns
/// the last `sceGsSyncPath`'s `eax`.
pub fn gs1_transfer(h: &mut impl Host, frame: u32) -> u32 {
    let packet = h.rebase(GS1_PACKET);
    for k in 0..40u32 {
        let v: u32 = h.read(packet.wrapping_add(4 * k));
        h.write(frame.wrapping_add(0x10 + 4 * k), v);
    }
    h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0]);
    h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
    let channel: u32 = h.read(h.rebase(DMA_CHANNEL));
    h.call_sym(DMA_SEND, Conv::Cdecl, &[channel, frame]);
    h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0])
}

// ---- op_oef2_texture_load

/// `"Virtuos ERROR: Texture data was NULL in yonishi
/// op_oef2_texture_load()\n"` (`0x00b58168`).
pub const NULL_TEXTURE: u32 = 0x00b5_8168;
/// `op_texflush()` (cdecl).
pub const TEX_FLUSH: u32 = 0x007e_c790;
/// Bytes of the load-image record the texture loads keep at the aligned
/// `ebp - 0x60`.
pub const LOAD_FRAME: u32 = 0x60;

/// `op_oef2_texture_load(texture, flush)` (`0x007e5030`, cdecl; `frame`
/// its load-image record, 16-byte aligned): a null texture is logged.
/// Else its image (dbp: `u16 +0` low 14 bits; dbw: byte `+0xa` * 16 / 64;
/// psm: byte `+0x18`; x: byte `+8` * 16; y: byte `+9` * 16; w `u16
/// +0xc`, h `u16 +0xe`, data `[+0x14]`) is loaded whole when `w * h` is
/// below 0x40000, else in 256-row strips at y 0 (dbp advanced by `w`,
/// data by `w << 8`, the last strip the rows left), each strip followed
/// by a path sync. With `flush`, a path sync and `op_texflush`.
pub fn oef2_texture_load(h: &mut impl Host, frame: u32, texture: u32, flush: u32) {
    if texture == 0 {
        let s = h.rebase(NULL_TEXTURE);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[s]);
        return;
    }
    let at = |off: u32| texture.wrapping_add(off);
    let dbp = u32::from(h.read::<u16>(at(0)) & 0x3fff);
    let x = u32::from(h.read::<u8>(at(8))) << 4;
    let dbw_bytes = u32::from(h.read::<u8>(at(0xa))) << 4;
    let w = u32::from(h.read::<u16>(at(0xc)));
    let hh = u32::from(h.read::<u16>(at(0xe)));
    // a signed division by 64 of a small positive number
    let dbw = dbw_bytes >> 6;
    if (hh.wrapping_mul(w) as i32) < 0x4_0000 {
        let y = (u32::from(h.read::<u8>(at(9))) << 4) & 0xffff;
        let psm = u32::from(h.read::<u8>(at(0x18)));
        h.call_sym(
            SET_DEF_LOAD_IMAGE,
            Conv::Cdecl,
            &[frame, dbp, dbw, psm, x, y, w, hh],
        );
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0]);
        let data: u32 = h.read(at(0x14));
        h.call_sym(EXEC_LOAD_IMAGE, Conv::Cdecl, &[frame, data]);
    } else {
        let mut dbp = dbp;
        // `(w << 8) / 256` as a signed division, cut to 16 bits: `w`
        let step = w;
        let mut data: u32 = h.read(at(0x14));
        let mut row = 0i32;
        let mut left = hh;
        if hh as i32 > 0 {
            loop {
                row += 0x100;
                let rows = if row > hh as i32 { left } else { 0x100 };
                let psm = u32::from(h.read::<u8>(at(0x18)));
                h.call_sym(
                    SET_DEF_LOAD_IMAGE,
                    Conv::Cdecl,
                    &[frame, dbp, dbw, psm, x, 0, w, rows],
                );
                h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
                h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0]);
                h.call_sym(EXEC_LOAD_IMAGE, Conv::Cdecl, &[frame, data]);
                h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0]);
                dbp = dbp.wrapping_add(step);
                data = data.wrapping_add(step << 8);
                left = left.wrapping_sub(0x100);
                if row >= hh as i32 {
                    break;
                }
            }
        }
    }
    if flush != 0 {
        h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0]);
        h.call_sym(TEX_FLUSH, Conv::Cdecl, &[]);
    }
}

// ---- the Sa menu's texture load

/// The next free GS address for the Sa menu's textures (`0x00c416c8`).
pub const SA_GS_BASE: u32 = 0x00c4_16c8;
/// The Sa menu's palette records (`0x0112a0ac`, 0x10 bytes each: `u16`
/// w, `u16` h, the GS address).
pub const SA_PALETTES: u32 = 0x0112_a0ac;
/// The Sa menu's image records (`0x0112a0a8`, 0x10 bytes each: the psm
/// word, then the GS address at `+0xc`).
pub const SA_IMAGES: u32 = 0x0112_a0a8;
/// `"sbin_head->head_cnt=%d\n"` (`0x00b53364`).
pub const HEAD_COUNT_FORMAT: u32 = 0x00b5_3364;
/// `"%d : w=%d h=%d\n"` (`0x00b5337c`).
pub const PALETTE_FORMAT: u32 = 0x00b5_337c;
/// `"image %d mode\n"` (`0x00b5338c`).
pub const IMAGE_MODE_FORMAT: u32 = 0x00b5_338c;
/// `"tex ten w=%d h=%d\n"` (`0x00b5339c`).
pub const IMAGE_SIZE_FORMAT: u32 = 0x00b5_339c;

/// The GS pixel format of an sbin image's bit depth.
fn sbin_psm(bits: u8) -> u32 {
    match bits {
        4 => 0x14,
        8 => 0x13,
        0x10 => 2,
        0x18 => 1,
        _ => 0,
    }
}

/// `SaMenu_loadSbinTextures_00775ed0(sbin)` (cdecl, from
/// `sa_menu_start`; `frame` its load-image record, 16-byte aligned): the
/// sbin's `[sbin + 4]` entries (0x10 bytes each from `+0x10`: `u16`
/// width, `u16` height, a bit depth byte at `+4`, the palette's data
/// offset at `+8` and the image's at `+0xc`) loaded into GS memory from
/// `[0x00c416c8]` on. First each palette (16x16 PSMCT32, 4 blocks each;
/// its record at `0x0112a0ac`), then each image (its psm from the bit
/// depth, width / 64 buffer width; the address advanced by bits times
/// width times height over 2048, a signed division; its record at
/// `0x0112a0a8`), with the original's debug prints. Returns the next
/// free address.
pub fn sa_load_sbin_textures(h: &mut impl Host, frame: u32, sbin: u32) -> u32 {
    let mut gs: u32 = h.read(h.rebase(SA_GS_BASE));
    h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0]);
    let count: u32 = h.read(sbin.wrapping_add(4));
    let f = h.rebase(HEAD_COUNT_FORMAT);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[f, count]);
    let palettes = h.rebase(SA_PALETTES);
    let mut i = 0i32;
    while i < h.read::<i32>(sbin.wrapping_add(4)) {
        let entry = sbin.wrapping_add(0x10 + 0x10 * i as u32);
        let record = palettes.wrapping_add(0x10 * i as u32);
        let w: u16 = h.read(entry);
        h.write(record, w);
        let hh: u16 = h.read(entry.wrapping_add(2));
        h.write(record.wrapping_add(2), hh);
        h.write(record.wrapping_add(4), gs);
        let hh = u32::from(h.read::<u16>(entry.wrapping_add(2)));
        let w = u32::from(h.read::<u16>(entry));
        let f = h.rebase(PALETTE_FORMAT);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[f, i as u32, w, hh]);
        h.call_sym(
            SET_DEF_LOAD_IMAGE,
            Conv::Cdecl,
            &[frame, gs, 1, 0, 0, 0, 0x10, 0x10],
        );
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        let offset: u32 = h.read(entry.wrapping_add(8));
        h.call_sym(
            EXEC_LOAD_IMAGE,
            Conv::Cdecl,
            &[frame, offset.wrapping_add(sbin)],
        );
        h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0]);
        gs = gs.wrapping_add(4);
        i += 1;
    }
    let images = h.rebase(SA_IMAGES);
    let mut i = 0i32;
    while i < h.read::<i32>(sbin.wrapping_add(4)) {
        let entry = sbin.wrapping_add(0x14 + 0x10 * i as u32);
        let record = images.wrapping_add(0x10 * i as u32);
        let psm = sbin_psm(h.read(entry));
        h.write(record, psm);
        h.write(record.wrapping_add(0xc), gs);
        let hh = u32::from(h.read::<u16>(entry.wrapping_sub(2)));
        let w = u32::from(h.read::<u16>(entry.wrapping_sub(4)));
        h.call_sym(
            SET_DEF_LOAD_IMAGE,
            Conv::Cdecl,
            &[frame, gs, w >> 6, psm, 0, 0, w, hh],
        );
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        let offset: u32 = h.read(entry.wrapping_add(8));
        h.call_sym(
            EXEC_LOAD_IMAGE,
            Conv::Cdecl,
            &[frame, offset.wrapping_add(sbin)],
        );
        h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0]);
        let bits = u32::from(h.read::<u8>(entry));
        let f = h.rebase(IMAGE_MODE_FORMAT);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[f, bits]);
        let f = h.rebase(IMAGE_SIZE_FORMAT);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[f, w, hh]);
        let bits = u32::from(h.read::<u8>(entry));
        let size = bits.wrapping_mul(hh).wrapping_mul(w) as i32;
        gs = gs.wrapping_add(((size + ((size >> 31) & 0x7ff)) >> 11) as u32);
        i += 1;
    }
    gs
}
