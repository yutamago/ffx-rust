//! M5 `main`: more of the main loop's debug side: the screen capture to
//! a host file (`Sg_CaptureToFile`) and the ID model's debug draw
//! (`Sg_IdDrawMain` and its triangle packet builder).
//!
//! These are the PS2 debug tools the PC build keeps; they only run with
//! the debug switches on. The GS and DMA calls are the PC's stand-ins.

/// The originals over game memory.
#[allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::chunks_exact_to_as_chunks,
    clippy::if_not_else
)] // registers, points; pixel loops as written
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `MSVCR110!sprintf`'s import slot (cdecl, varargs).
    pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
    /// `"%s%s"`, `"host0:"`, `"/bin/chmod 777 %s"` (`.rdata`).
    pub const FMT_CONCAT: u32 = 0x00b4_0eb0;
    pub const STR_HOST0: u32 = 0x00b5_9c78;
    pub const FMT_CHMOD: u32 = 0x00b5_9cac;
    /// `sceOpen(name, mode)`, `sceWrite(fd, p, n)`, `sceClose(fd)`
    /// (cdecl).
    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_WRITE: u32 = 0x0062_fdc0;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    /// `user_malloc(size)`, `user_free_thunk(p)` (cdecl).
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const USER_FREE: u32 = 0x0068_7010;
    /// `sceGsSetDefStoreImage(img, base, bw, psm, x, y, w, h)`,
    /// `sceGsExecStoreImage(img, p)`, `sceGsSyncPath(mode, timeout)`,
    /// `FlushCache(mode)` (cdecl).
    pub const SET_DEF_STORE_IMAGE: u32 = 0x0062_f830;
    pub const EXEC_STORE_IMAGE: u32 = 0x0062_f7b0;
    pub const GS_SYNC_PATH: u32 = 0x0062_f880;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    /// `SystemOnHost(command, out)` (cdecl).
    pub const SYSTEM_ON_HOST: u32 = 0x0084_ede0;
    /// `g_SgFrame` (its low bit picks the frame buffer shown).
    pub const SG_FRAME: u32 = 0x023c_bbf0;
    /// The two store-image records (front and back buffer).
    pub const STORE_IMAGE_A: u32 = 0x0130_1820;
    pub const STORE_IMAGE_B: u32 = 0x0130_1880;
    /// The BMP header (0x36 bytes) and its height field.
    pub const BMP_HEADER: u32 = 0x00c4_979c;
    pub const BMP_HEIGHT: u32 = 0x00c4_97b2;
    /// The capture's mode bytes: interlaced (both fields stored) and
    /// field blending.
    pub const CAPTURE_INTERLACED: u32 = 0x00c4_97d2;
    pub const CAPTURE_BLEND: u32 = 0x00c4_97d3;
    /// The name buffer's place in the original's frame (`ebp - 0x104`,
    /// 0x100 bytes, then the cookie).
    pub const CAPTURE_FILE_FRAME: u32 = 0x104;

    fn sprintf(h: &mut impl Host, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(SPRINTF_IAT));
        h.call(f, Conv::Cdecl, args)
    }

    /// `Sg_CaptureToFile(name)` (`0x0083a690`, cdecl), with its frame at
    /// `ebp`: opens `host0:<name>`, stores both frame buffers (0x200 x
    /// 0x1a0, 32 bits) into two heap buffers, writes the BMP header and
    /// the picture as 24-bit rows bottom-up (the shown buffer, `B, G, R`
    /// from `R, G, B, x`): line-doubled from every other line when not
    /// interlaced, else every line, blended with the line below when
    /// [`CAPTURE_BLEND`] is set (the bottom line as it is). Then
    /// `chmod 777` on the host and the buffers freed. Returns the last
    /// free's `eax`.
    pub fn capture_to_file(h: &mut impl Host, ebp: u32, name: u32) -> u32 {
        let odd = h.read::<u8>(h.rebase(SG_FRAME)) & 1 != 0;
        let (base_a, base_b) = if odd { (0, 0xd00) } else { (0xd00, 0) };
        let buf = ebp - CAPTURE_FILE_FRAME;
        let fmt = h.rebase(FMT_CONCAT);
        let host0 = h.rebase(STR_HOST0);
        sprintf(h, &[buf, fmt, host0, name]);
        let fd = h.call_sym(SCE_OPEN, Conv::Cdecl, &[buf, 0x602]);
        let a = h.call_sym(USER_MALLOC, Conv::Cdecl, &[0xd_0000]);
        let b = h.call_sym(USER_MALLOC, Conv::Cdecl, &[0xd_0000]);
        let out = h.call_sym(USER_MALLOC, Conv::Cdecl, &[0x13_8000]);
        let img_a = h.rebase(STORE_IMAGE_A);
        let img_b = h.rebase(STORE_IMAGE_B);
        h.call_sym(
            SET_DEF_STORE_IMAGE,
            Conv::Cdecl,
            &[img_a, base_a, 8, 0, 0, 0, 0x200, 0x1a0],
        );
        h.call_sym(
            SET_DEF_STORE_IMAGE,
            Conv::Cdecl,
            &[img_b, base_b, 8, 0, 0, 0, 0x200, 0x1a0],
        );
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        h.call_sym(EXEC_STORE_IMAGE, Conv::Cdecl, &[img_a, a]);
        h.call_sym(EXEC_STORE_IMAGE, Conv::Cdecl, &[img_b, b]);
        h.call_sym(GS_SYNC_PATH, Conv::Cdecl, &[0, 0]);
        h.write(h.rebase(BMP_HEIGHT), 0x1a0u16);
        let header = h.rebase(BMP_HEADER);
        h.call_sym(SCE_WRITE, Conv::Cdecl, &[fd, header, 0x36]);
        let interlaced = h.read::<u8>(h.rebase(CAPTURE_INTERLACED)) != 0;
        let blend = h.read::<u8>(h.rebase(CAPTURE_BLEND)) != 0;
        let mut src = vec![0u8; 0x800];
        let mut below = vec![0u8; 0x800];
        let mut row = vec![0u8; 0x600];
        let mut dst = out;
        if !interlaced {
            // 0xd0 stored lines, each written twice, top line first from
            // the bottom of the buffer
            for k in 0..0xd0u32 {
                h.read_bytes(a + 0xc_f000 - 0x1000 * k, &mut src);
                bgr_row(&src, &mut row);
                h.write_bytes(dst, &row);
                h.write_bytes(dst + 0x600, &row);
                dst += 0xc00;
            }
        } else {
            for line in (0..0x1a0u32).rev() {
                let p = a + 0x800 * line;
                h.read_bytes(p, &mut src);
                if blend && line != 0 {
                    h.read_bytes(p + 0x800, &mut below);
                    for (k, px) in row.chunks_exact_mut(3).enumerate() {
                        let s = 4 * k;
                        let mean = |c: usize| {
                            let sum = u16::from(src[s + c]) + u16::from(below[s + c]);
                            #[allow(clippy::cast_possible_truncation)] // at most 0xff
                            let m = (sum / 2) as u8;
                            m
                        };
                        px[0] = mean(2);
                        px[1] = mean(1);
                        px[2] = mean(0);
                    }
                } else {
                    bgr_row(&src, &mut row);
                }
                h.write_bytes(dst, &row);
                dst += 0x600;
            }
        }
        h.call_sym(SCE_WRITE, Conv::Cdecl, &[fd, out, 0x9_c000]);
        h.call_sym(SCE_CLOSE, Conv::Cdecl, &[fd]);
        let fmt = h.rebase(FMT_CHMOD);
        sprintf(h, &[buf, fmt, name]);
        h.call_sym(SYSTEM_ON_HOST, Conv::Cdecl, &[buf, out]);
        h.call_sym(USER_FREE, Conv::Cdecl, &[b]);
        h.call_sym(USER_FREE, Conv::Cdecl, &[a]);
        h.call_sym(USER_FREE, Conv::Cdecl, &[out])
    }

    /// One 0x200-pixel line, `R, G, B, x` to `B, G, R`.
    fn bgr_row(src: &[u8], row: &mut [u8]) {
        for (px, s) in row.chunks_exact_mut(3).zip(src.chunks_exact(4)) {
            px[0] = s[2];
            px[1] = s[1];
            px[2] = s[0];
        }
    }

    /// `get_ptr32_func(p)` (cdecl): a stored 32-bit pointer resolved.
    pub const GET_PTR32: u32 = 0x00a4_4650;
    /// `Sg_IdProject_00845030(out, v, m)` (cdecl): 0 when the point is
    /// drawn.
    pub const ID_PROJECT: u32 = 0x0084_5030;
    /// `Sg_IdGetShade(word)` (cdecl): a 15-bit colour.
    pub const ID_GET_SHADE: u32 = 0x0083_e980;
    /// The ID draw's packet cursor (entries) and packet.
    pub const ID_PACKET_COUNT: u32 = 0x0130_3648;
    pub const ID_PACKET: u32 = 0x0130_364c;
    /// `FUN_008447a0`'s frame below `ebp`: the three points (`x, y, z,
    /// 1.0`) at `ebp - 0x34`, `- 0x24`, `- 0x14`, and the cookie.
    pub const ID_TRIANGLES_FRAME: u32 = 0x34;

    /// `FUN_008447a0(verts, model, m, packet)` (cdecl, with its frame at
    /// `ebp`): the GIF tag and the GS registers at `packet + 0x10`; then
    /// each of the model's triangles (`i16` count at `+8`, the list
    /// through `+0xc`, 16 bytes each: three `i16` vertex indices, the
    /// shade word at `+0xc`) with its three vertices (`i16 x, y, z` of
    /// 8-byte records) projected into the packet; when all three are
    /// drawn, the shade's three 5-bit channels as the vertices' grey
    /// colours and the cursor advanced by six entries. The triangle count
    /// goes into the tag's low 15 bits, the end tag after the last entry.
    /// Returns the end tag.
    pub fn id_triangles(
        h: &mut impl Host,
        ebp: u32,
        verts: u32,
        model: u32,
        m: u32,
        packet: u32,
    ) -> u32 {
        for (off, v) in [
            (0x10, 1u32),
            (0x14, 0x1000_0000),
            (0x18, 0x00ee_eeee),
            (0x1c, 0),
            (0x20, 0x0007_07fa),
            (0x24, 0),
            (0x28, 0x47),
            (0x2c, 0),
            (0x30, 0),
            (0x34, 0),
        ] {
            h.write(packet + off, v);
        }
        let tag: u32 = h.read(packet + 0x30);
        h.write(packet + 0x30, tag | 0x8000);
        let w: u32 = h.read(packet + 0x34);
        h.write(packet + 0x34, (w & 0x3fff) | 0x6005_c000);
        h.write(packet + 0x38, 0x0041_4141u32);
        h.write(packet + 0x3c, 0u32);
        let count_at = h.rebase(ID_PACKET_COUNT);
        let packet_at = h.rebase(ID_PACKET);
        h.write(packet_at, packet);
        h.write(count_at, 4u32);
        let list_ref: u32 = h.read(model + 0xc);
        let list = h.call_sym(GET_PTR32, Conv::Cdecl, &[list_ref]);
        let points = [ebp - 0x34, ebp - 0x24, ebp - 0x14];
        for p in points {
            h.write(p + 0xc, 1.0f32.to_bits());
        }
        let mut pk: u32 = h.read(packet_at);
        let mut drawn = 0u32;
        let mut i = 0i32;
        let mut tri = list.wrapping_add(4);
        while i < i32::from(h.read::<i16>(model + 8)) {
            let mut all = true;
            for (k, p) in (0u32..).zip(points) {
                let idx = i32::from(h.read::<i16>(tri.wrapping_sub(4) + 2 * k));
                let vert = verts.wrapping_add((idx * 8).cast_unsigned());
                for c in 0..3 {
                    let x = f32::from(h.read::<i16>(vert + 2 * c));
                    h.write(p + 4 * c, x.to_bits());
                }
                let cnt: u32 = h.read(count_at);
                let base: u32 = h.read(packet_at);
                let at = cnt
                    .wrapping_add(1 + 2 * k)
                    .wrapping_mul(16)
                    .wrapping_add(base);
                if h.call_sym(ID_PROJECT, Conv::Cdecl, &[at, p, m]) != 0 {
                    all = false;
                    break;
                }
            }
            if all {
                let word: u32 = h.read(tri + 8);
                let shade = h.call_sym(ID_GET_SHADE, Conv::Cdecl, &[word]);
                pk = h.read(packet_at);
                let cnt: u32 = h.read(count_at);
                let e = pk.wrapping_add(cnt.wrapping_mul(16));
                let s = shade.cast_signed();
                let greys = [
                    (shade & 0x1f) << 3,
                    ((s >> 2) & 0xf8).cast_unsigned(),
                    ((s >> 7) & 0xf8).cast_unsigned(),
                ];
                for (k, g) in (0u32..).zip(greys) {
                    let c = e + 0x20 * k;
                    h.write(c, g);
                    h.write(c + 4, g);
                    h.write(c + 8, g);
                    h.write(c + 0xc, 0x80u32);
                }
                drawn += 1;
                h.write(count_at, cnt.wrapping_add(6));
            } else {
                pk = h.read(packet_at);
            }
            i += 1;
            tri = tri.wrapping_add(0x10);
        }
        let tag: u32 = h.read(packet + 0x30);
        h.write(packet + 0x30, tag ^ ((tag ^ drawn) & 0x7fff));
        h.write(pk + 4, 0u32);
        h.write(pk + 8, 0u32);
        h.write(pk + 0xc, 0u32);
        let cnt: u32 = h.read(count_at);
        let end = cnt.wrapping_sub(1) | 0x7000_0000;
        h.write(pk, end);
        end
    }

    /// `Sg_PacketGetPtr()`, `Sg_IdGetPtr()`, `Ch_SysGetScreenMatrixPtr()`,
    /// `Ch_SysGetCameraMatrixPtr()` (cdecl).
    pub const PACKET_GET_PTR: u32 = 0x0083_b650;
    pub const ID_GET_PTR: u32 = 0x0083_e970;
    pub const SCREEN_MATRIX: u32 = 0x0083_5b70;
    pub const CAMERA_MATRIX: u32 = 0x0083_5ba0;
    /// The ID draw switch.
    pub const ID_DRAW_ON: u32 = 0x0130_34b4;
    /// `memset`'s import thunk (cdecl).
    pub const MEMSET: u32 = 0x0094_964c;
    /// `FFXVu0UnitMatrix(m)`, `FFXVu0MulMatrix(out, a, b)` (cdecl).
    pub const UNIT_MATRIX: u32 = 0x006e_d9a0;
    pub const MUL_MATRIX: u32 = 0x006e_d5c0;
    /// `Sg_IdGetZoom()` (cdecl, `ST0`).
    pub const ID_GET_ZOOM: u32 = 0x0083_e9a0;
    /// `FUN_00844490(list, item, m, packet, dma)` (cdecl): one model's
    /// packet.
    pub const ID_MODEL_PACKET: u32 = 0x0084_4490;
    /// `sceDmaSend(channel, packet)` (cdecl) and its channel word.
    pub const DMA_SEND: u32 = 0x0062_f710;
    pub const DMA_CHANNEL: u32 = 0x023c_bbf4;
    /// `Sg_IdDrawMain`'s frame below `ebp`: the view matrix at `ebp -
    /// 0x84` and the zoom matrix at `ebp - 0x44`, then the cookie.
    pub const ID_DRAW_FRAME: u32 = 0x84;

    /// `Sg_IdDrawMain()` (`0x00844a80`, cdecl, with its frame at `ebp`):
    /// with an ID model and the switch on, the view (camera x screen x
    /// the unit matrix scaled by 1 / zoom, rounded to `f32`) and each of
    /// the model's parts (`i16` count at `+6`) built, flushed, sent and
    /// waited for. Returns nothing the callers read.
    pub fn id_draw_main(h: &mut impl Host, ebp: u32) {
        let packet = h.call_sym(PACKET_GET_PTR, Conv::Cdecl, &[]);
        let id = h.call_sym(ID_GET_PTR, Conv::Cdecl, &[]);
        if id == 0 || h.read::<u32>(h.rebase(ID_DRAW_ON)) == 0 {
            return;
        }
        let screen = h.call_sym(SCREEN_MATRIX, Conv::Cdecl, &[]);
        let camera = h.call_sym(CAMERA_MATRIX, Conv::Cdecl, &[]);
        let view = ebp - 0x84;
        let zoom_m = ebp - 0x44;
        h.call_sym(MEMSET, Conv::Cdecl, &[zoom_m, 0, 0x40]);
        h.call_sym(UNIT_MATRIX, Conv::Cdecl, &[zoom_m]);
        let zoom = h.call_sym_st0_f64(ID_GET_ZOOM, Conv::Cdecl, &[]);
        let s = x87::store(x87::div(1.0, zoom));
        h.write(zoom_m + 0x28, s);
        h.write(zoom_m + 0x14, s);
        h.write(zoom_m, s);
        h.call_sym(MUL_MATRIX, Conv::Cdecl, &[view, camera, screen]);
        h.call_sym(MUL_MATRIX, Conv::Cdecl, &[view, view, zoom_m]);
        let mut i = 0i32;
        let mut off = 0u32;
        while i < i32::from(h.read::<i16>(id + 6)) {
            let dma: u32 = h.read(h.rebase(DMA_CHANNEL));
            let items_ref: u32 = h.read(id + 0x1c);
            let items = h.call_sym(GET_PTR32, Conv::Cdecl, &[items_ref]);
            let list_ref: u32 = h.read(id + 0x18);
            let list = h.call_sym(GET_PTR32, Conv::Cdecl, &[list_ref]);
            h.call_sym(
                ID_MODEL_PACKET,
                Conv::Cdecl,
                &[list, items.wrapping_add(off), view, packet, dma],
            );
            h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
            let dma: u32 = h.read(h.rebase(DMA_CHANNEL));
            h.call_sym(DMA_SEND, Conv::Cdecl, &[dma, packet]);
            h.call_sym(GS_SYNC_PATH, Conv::Cdecl, &[0, 0]);
            i += 1;
            off = off.wrapping_add(0x10);
        }
    }
}
