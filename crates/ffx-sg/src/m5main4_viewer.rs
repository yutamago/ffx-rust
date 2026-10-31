//! M5 stage 1, list `small`, round 4: the PS2 movie viewer's GS upload
//! packets and frame steps (code only an address in the viewer's
//! unreached code points at; `tools/reach.py` keeps it).

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // registers
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `GsPackTag(out, a0, a1, a2)` (cdecl).
    pub const GS_PACK_TAG: u32 = 0x0077_ead0;

    /// `FUN_0077f160(tag, src, n)` (cdecl): DMA reference tags for `n`
    /// bytes from `src` in pieces of at most 0xffff0 bytes (quadword
    /// counts rounded up), at `tag | 0x20000000` on, 16 bytes each; every
    /// piece but the last gets flag 3.
    pub fn ref_tags(h: &mut impl Host, tag: u32, src: u32, n: u32) {
        let mut out = tag | 0x2000_0000;
        let mut src = src;
        let mut left = n.cast_signed();
        while left > 0 {
            let piece = if left >= 0xf_fff0 { 0xf_fff0 } else { left };
            let qwords = piece.wrapping_add(0xf) / 16;
            left -= piece;
            let flag = if left == 0 { 0 } else { 3 };
            cdecl(h, GS_PACK_TAG, &[out, src, flag, qwords.cast_unsigned()]);
            src = src.wrapping_add(piece.cast_unsigned());
            out = at(out, 0x10);
        }
    }

    /// The viewer (`0x00c421b0`, a pointer; `+0x503cc` its upload
    /// packets, 0x180 bytes each) and its scratch block (`0x0112a840`).
    pub const VIEWER: u32 = 0x00c4_21b0;
    pub const BLOCK: u32 = 0x0112_a840;
    /// `asmAddBlock1(b)`, `asmAddBlock2(b)`, `asmClearBlock0(b)`,
    /// `TexAnim_LoadImageRegs(a, b, bytes)` (cdecl).
    pub const ADD_BLOCK1: u32 = 0x0077_f1d0;
    pub const ADD_BLOCK2: u32 = 0x0077_f1e0;
    pub const CLEAR_BLOCK0: u32 = 0x0077_f200;
    pub const LOAD_IMAGE_REGS: u32 = 0x0077_e770;

    /// `FUN_0077c3c0(i, x, y, mode)` (cdecl): the viewer's block step:
    /// mode 2 adds block 2, mode 3 clears block 0 (`+0x50`); an odd mode
    /// sets the offsets `+0x10/+0x14` to `x` and `y` modulo 16 and adds
    /// block 1; then packet `i`'s image registers loaded into `+0x350`.
    /// Returns `TexAnim_LoadImageRegs`'s `eax`.
    pub fn block_step(h: &mut impl Host, i: u32, x: u32, y: u32, mode: u32) -> u32 {
        let block_at = h.rebase(BLOCK);
        match mode {
            2 => {
                let b: u32 = h.read(block_at);
                cdecl(h, ADD_BLOCK2, &[b]);
            }
            3 => {
                let b: u32 = h.read(block_at);
                cdecl(h, CLEAR_BLOCK0, &[at(b, 0x50)]);
            }
            _ => {}
        }
        if mode & 1 != 0 {
            let dx = x.cast_signed().rem_euclid(16).cast_unsigned();
            let dy = y.cast_signed().rem_euclid(16).cast_unsigned();
            let b: u32 = h.read(block_at);
            h.write(at(b, 0x10), dx);
            h.write(at(b, 0x14), dy);
            cdecl(h, ADD_BLOCK1, &[b]);
        }
        let b: u32 = h.read(block_at);
        let viewer: u32 = h.read(h.rebase(VIEWER));
        let packets: u32 = h.read(at(viewer, 0x5_03cc));
        let src = at(packets, i.wrapping_mul(0x180));
        cdecl(h, LOAD_IMAGE_REGS, &[src, at(b, 0x350), 0x180])
    }

    /// `FlushCache(n)`, `TexAnim_SetDmaTag(q, ...)` (7 words),
    /// `TexAnim_SetGifTag(q, ...)` (9 words), `TexAnim_setTag(q, a, b, c,
    /// flag)`, TRXREG, TRXPOS, TRXDIR (cdecl).
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const SET_DMA_TAG: u32 = 0x0077_eb70;
    pub const SET_GIF_TAG: u32 = 0x0077_ebe0;
    pub const SET_TAG: u32 = 0x0077_eb00;
    pub const TRXREG: u32 = 0x0077_f140;
    pub const TRXPOS: u32 = 0x0077_f100;
    pub const TRXDIR: u32 = 0x0077_f0e0;

    /// `FUN_0077ee40(tag, src, step, w, h, base, half, psm)` (cdecl): a
    /// tiled image upload packet at `tag | 0x20000000`: the DMA and GIF
    /// heads, BITBLTBUF (`base`, `(w + 63) / 64`, `half`, `psm`), TRXREG
    /// 16 x 16; then per 16 x 16 tile (rows `h >> 4`, columns `w >> 4`)
    /// the DMA and GIF tags, TRXPOS, TRXDIR, an image GIF tag (EOP on the
    /// last tile) and a DMA reference to `src` (0x20 quadwords when
    /// `half`, else 0x40), `src` moving by `step`; an end tag. The data
    /// cache is flushed before and after. Returns `FlushCache`'s `eax`.
    #[allow(clippy::too_many_arguments)] // the original's eight words
    pub fn tiled_upload(
        h: &mut impl Host,
        tag: u32,
        src: u32,
        step: u32,
        w: u32,
        hgt: u32,
        base: u32,
        half: u32,
        psm: u32,
    ) -> u32 {
        let rows = hgt.cast_signed() >> 4;
        let cols = w.cast_signed() >> 4;
        cdecl(h, FLUSH_CACHE, &[0]);
        let qwords = if half == 0 { 0x40 } else { 0x20 };
        let mut q = tag | 0x2000_0000;
        cdecl(h, SET_DMA_TAG, &[q, 0, 0, 0, 1, 0, 3]);
        q = at(q, 0x10);
        cdecl(h, SET_GIF_TAG, &[q, 0xe, 0, 1, 0, 0, 0, 0, 2]);
        q = at(q, 0x10);
        let blocks = w.cast_signed().wrapping_add(0x3f) / 0x40;
        cdecl(h, SET_TAG, &[q, base, blocks.cast_unsigned(), half, psm]);
        q = at(q, 0x10);
        cdecl(h, TRXREG, &[q, 0x10, 0x10]);
        q = at(q, 0x10);
        let mut src = src;
        for r in 0..rows.max(0) {
            for c in 0..cols.max(0) {
                let last = u32::from(r == rows - 1 && c == cols - 1);
                cdecl(h, SET_DMA_TAG, &[q, 0, 0, 0, 1, 0, 4]);
                q = at(q, 0x10);
                cdecl(h, SET_GIF_TAG, &[q, 0xe, 0, 1, 0, 0, 0, 0, 2]);
                q = at(q, 0x10);
                let (x, y) = ((c << 4).cast_unsigned(), (r << 4).cast_unsigned());
                cdecl(h, TRXPOS, &[q, 0, x, y]);
                q = at(q, 0x10);
                cdecl(h, TRXDIR, &[q, 0]);
                q = at(q, 0x10);
                cdecl(h, SET_GIF_TAG, &[q, 0, 0, 0, 2, 0, 0, last, qwords]);
                q = at(q, 0x10);
                cdecl(h, SET_DMA_TAG, &[q, 0, src, 0, 3, 0, qwords]);
                src = src.wrapping_add(step);
                q = at(q, 0x10);
            }
        }
        cdecl(h, SET_DMA_TAG, &[q, 0, 0, 0, 7, 0, 0]);
        cdecl(h, FLUSH_CACHE, &[0])
    }

    /// `dbgPrintf`, `sceIpuInit()`, `movieHardWareInit()`,
    /// `chr_texanim_clut_swizzle(dst, src, which)`, `sceIpuSync(a, b)`,
    /// `TexAnim_SyncPath(a)`, `loadImage(addr, bytes)`, `task_wait(n)`,
    /// `sceGsSyncPath(a, b)`, `TexAnim_SyncPath2(a)` (cdecl), the tiled
    /// upload and the reference tags.
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const IPU_INIT: u32 = 0x0062_f8c0;
    pub const HARDWARE_INIT: u32 = 0x0077_d230;
    pub const CLUT_SWIZZLE: u32 = 0x0077_ca10;
    pub const IPU_SYNC: u32 = 0x0062_f8d0;
    pub const SYNC_PATH: u32 = 0x0077_e800;
    pub const LOAD_IMAGE: u32 = 0x0077_e730;
    pub const TASK_WAIT: u32 = 0x0077_a4e0;
    pub const GS_SYNC_PATH: u32 = 0x0062_f880;
    pub const SYNC_PATH2: u32 = 0x0077_eaa0;
    pub const TILED_UPLOAD: u32 = 0x0077_ee40;
    pub const REF_TAGS: u32 = 0x0077_f160;
    /// This module's other originals: the `iPicture` player, the frame
    /// output, the stream end and the picture set-up.
    pub const IPU_MOVIE_SYM: u32 = 0x0077_c4c0;
    pub const FRAME_OUT: u32 = 0x0077_ca80;
    pub const MPEG_STR_END: u32 = 0x0077_dab0;
    pub const PICTURE_SETUP: u32 = 0x0077_ce10;
    /// The formats ("total frame", "iPicture movie"), the CLUT tables and
    /// their sources, `g_GsRegisterSink`.
    pub const TOTAL_FRAMES: u32 = 0x00b5_4428;
    pub const PICTURE_MOVIE: u32 = 0x00b5_443c;
    pub const CLUT_A: u32 = 0x0112_a848;
    pub const CLUT_A_SRC: u32 = 0x00c4_20b0;
    pub const CLUT_B: u32 = 0x0112_a888;
    pub const CLUT_B_SRC: u32 = 0x00c4_20f0;
    pub const GS_SINK: u32 = 0x0230_56e8;

    fn sink(h: &mut impl Host, v: u32) {
        let at_ = h.rebase(GS_SINK);
        h.write(at_, v);
    }

    /// `FUN_0077c4c0(v)` (cdecl): the PS2 `iPicture` movie player. The
    /// movie header (`[v + 0x460f4]`, its entry at `+[+8]`: `+0` the base,
    /// `+4` the frame byte, `+8` the frame total, `+0xc/+0xe` width and
    /// height, records from `+0x20`) is printed, the IPU and the tiled
    /// upload set up, the CLUTs swizzled. Then the records (`+4` the data
    /// bytes, `+8` the frames, data from `+0x10`; frames -1 rewinds to the
    /// first, flagging `+0x460ec` bit 0x2000 and the counter `+0x460e8`
    /// 0) are sent: the data by reference tags (`v + 0x46060`) through
    /// the IPU, each frame loaded (`(w / 16) * (h / 16) << 10` bytes) and
    /// synced, counting frames (`+0x460e8`, `+0x460f0` = 1) until the stop
    /// byte `+0x460fc` (cleared on the way out). Every step writes the GS
    /// register sink as the PS2 did. Returns the last `sceGsSyncPath`'s
    /// `eax`.
    pub fn ipu_movie(h: &mut impl Host, v: u32) -> u32 {
        let head: u32 = h.read(at(v, 0x4_60f4));
        let e = at(head, h.read::<u32>(at(head, 8)));
        let height = u32::from(h.read::<u16>(at(e, 0xe)));
        let width = u32::from(h.read::<u16>(at(e, 0xc)));
        let first = at(e, 0x20);
        let bytes = ((height / 16) * (width / 16)) << 10;
        let frames: u32 = h.read(at(e, 8));
        let f = h.rebase(TOTAL_FRAMES);
        cdecl(h, DBG_PRINTF, &[f, frames]);
        let kind = u32::from(h.read::<u8>(at(e, 4)));
        let f = h.rebase(PICTURE_MOVIE);
        cdecl(h, DBG_PRINTF, &[f, width, height, kind]);
        cdecl(h, IPU_INIT, &[]);
        cdecl(h, HARDWARE_INIT, &[]);
        cdecl(
            h,
            TILED_UPLOAD,
            &[at(v, 0x4_0000), v, 0x400, width, height, 0x3400, 0, 0],
        );
        for (dst, src) in [(CLUT_A, CLUT_A_SRC), (CLUT_B, CLUT_B_SRC)] {
            let (d, s) = (h.rebase(dst), h.rebase(src));
            cdecl(h, CLUT_SWIZZLE, &[d, s, 0]);
        }
        cdecl(h, FLUSH_CACHE, &[0]);
        let base = h.read::<u32>(e) | 0x0120_0000;
        h.write(at(v, 0x4_60e8), 0u32);
        h.write(at(v, 0x4_60ec), 0u32);
        let mut rec = first;
        loop {
            if h.read::<u32>(at(rec, 8)) == u32::MAX {
                let flags: u32 = h.read(at(v, 0x4_60ec));
                h.write(at(v, 0x4_60ec), flags | 0x2000);
                rec = first;
                h.write(at(v, 0x4_60e8), 0u32);
            }
            let data = at(rec, 0x10);
            cdecl(h, FLUSH_CACHE, &[0]);
            let size: u32 = h.read(at(rec, 4));
            let tags = at(v, 0x4_6060);
            cdecl(h, REF_TAGS, &[tags, data, size]);
            sink(h, 0x4000_0000);
            cdecl(h, IPU_SYNC, &[0, 0]);
            sink(h, 0);
            cdecl(h, IPU_SYNC, &[0, 0]);
            cdecl(h, SYNC_PATH, &[tags]);
            sink(h, base);
            let mut k = 0u32;
            while k < h.read::<u32>(at(rec, 8)) {
                sink(h, 0x1001_0000);
                cdecl(h, LOAD_IMAGE, &[v, bytes]);
                k += 1;
                cdecl(h, TASK_WAIT, &[1]);
                if k < h.read::<u32>(at(rec, 8)) {
                    sink(h, 0x4000_0020);
                }
                cdecl(h, GS_SYNC_PATH, &[0, 0]);
                cdecl(h, SYNC_PATH2, &[at(v, 0x4_0000)]);
                cdecl(h, GS_SYNC_PATH, &[0, 0]);
                if h.read::<u8>(at(v, 0x4_60fc)) != 0 {
                    let r = cdecl(h, GS_SYNC_PATH, &[0, 0]);
                    h.write(at(v, 0x4_60fc), 0u8);
                    return r;
                }
                let n: u32 = h.read(at(v, 0x4_60e8));
                h.write(at(v, 0x4_60e8), n.wrapping_add(1));
                h.write(at(v, 0x4_60f0), 1u32);
            }
            rec = at(data, h.read::<u32>(at(rec, 4)));
        }
    }

    fn sink_value(h: &impl Host) -> u32 {
        h.read(h.rebase(GS_SINK))
    }

    /// The PS2 waits on the GS/IPU register (bit 31 busy): on the PC the
    /// sink holds what the code wrote last, so these return at once unless
    /// that value has the bit (then they spin forever, as the original).
    fn wait_ready(h: &impl Host) {
        while sink_value(h) & 0x8000_0000 != 0 {}
    }

    /// The same for bit 8.
    fn wait_idle(h: &impl Host) {
        while sink_value(h) & 0x100 != 0 {}
    }

    /// `TexAnim_LoadImage101(addr, bytes)`, `TexAnim_LoadImageRegs(a, b,
    /// bytes)`, the block step (`0x0077c3c0`) and `FUN_0077cf80()` (the
    /// sink's ready test) (cdecl).
    pub const LOAD_IMAGE_101: u32 = 0x0077_e7c0;
    pub const BLOCK_STEP: u32 = 0x0077_c3c0;
    pub const SINK_READY: u32 = 0x0077_cf80;

    /// `FUN_0077b480()` (cdecl): the viewer's intra-frame decode: the
    /// stream record at the cursor (`[v + 0x5059c]`: `+0` its length,
    /// `+9` the quantiser, `+0xe/+0xf` the picture codes) handed to the
    /// IPU (`LoadImage101` of the record, the cursor moved past its
    /// header), then macroblocks until `FUN_0077cf80` says the IPU is
    /// done: each one's 0x300 bytes loaded from the block buffer
    /// (`[0x0112a840] + 0x50`), decoded (`0x0a` command with the quantiser,
    /// the first with the DC reset bit), its registers loaded into the
    /// output (`[v + 0x503c8] + 0x300 * k`) and stepped (mode 2). The
    /// sink is reset and the cursor moved past the record. Returns 0.
    pub fn intra_decode(h: &mut impl Host) -> u32 {
        let v: u32 = h.read(h.rebase(VIEWER));
        let cur: u32 = h.read(at(v, 0x5_059c));
        let codes = u32::from(h.read::<u8>(at(cur, 0xf)));
        let len: u32 = h.read(cur);
        let quant = u32::from(h.read::<u8>(at(cur, 9)));
        let pic = u32::from(h.read::<u8>(at(cur, 0xe)));
        let c = ((pic.wrapping_add((codes & 1) * 4) << 4).wrapping_add(codes >> 1)) << 16;
        let base = c.wrapping_add(0x0120_0000);
        sink(h, base);
        let s = sink_value(h);
        sink(h, (s | 0x4000_0000).wrapping_add(c));
        wait_ready(h);
        let v: u32 = h.read(h.rebase(VIEWER));
        let cur = h.read::<u32>(at(v, 0x5_059c)).wrapping_add(0x10);
        h.write(at(v, 0x5_059c), cur);
        cdecl(h, LOAD_IMAGE_101, &[cur, len]);
        let mut k = 0u32;
        loop {
            sink(h, 0x3000_0000);
            wait_ready(h);
            let block = h.read::<u32>(h.rebase(BLOCK)).wrapping_add(0x50);
            cdecl(h, LOAD_IMAGE, &[block, 0x300]);
            sink(h, base);
            sink(h, ((u32::from(k == 0) | 0xa) << 10 | quant) << 16);
            wait_ready(h);
            wait_idle(h);
            let block = h.read::<u32>(h.rebase(BLOCK)).wrapping_add(0x50);
            let v: u32 = h.read(h.rebase(VIEWER));
            let out = h
                .read::<u32>(at(v, 0x5_03c8))
                .wrapping_add(k.wrapping_mul(0x300));
            cdecl(h, LOAD_IMAGE_REGS, &[out, block, 0x300]);
            wait_idle(h);
            cdecl(h, BLOCK_STEP, &[k, 0, 0, 2]);
            wait_idle(h);
            k += 1;
            if cdecl(h, SINK_READY, &[0]) == 0 {
                break;
            }
        }
        sink(h, 0);
        let s = sink_value(h);
        sink(h, (s | 0x4000_0000).wrapping_add(c));
        wait_ready(h);
        wait_idle(h);
        let v: u32 = h.read(h.rebase(VIEWER));
        let cur: u32 = h.read(at(v, 0x5_059c));
        h.write(at(v, 0x5_059c), cur.wrapping_add(len));
        0
    }

    /// `FUN_0077ec20(...)` (cdecl, 11 words): the frame's copy to the
    /// display buffer.
    pub const FRAME_COPY: u32 = 0x0077_ec20;

    /// The frame copy for the viewer `v` from buffer `other` at row `y`.
    fn frame_copy(h: &mut impl Host, other: u32, y: u32, fl: u32, tf: u32) {
        let v: u32 = h.read(h.rebase(VIEWER));
        let mb_w: u32 = h.read(at(v, 0x5_0514));
        let args = [
            h.read::<u32>(at(v, 0x5_03d0)),
            h.read::<u32>(at(v, 0x5_03c0 + 4 * other)),
            0,
            y,
            mb_w << 4,
            0x10,
            h.read::<u32>(at(v, 0x5_051c)),
            0,
            0,
            fl,
            tf,
        ];
        cdecl(h, FRAME_COPY, &args);
    }

    fn sign_extend(b: u8) -> u32 {
        i32::from(b.cast_signed()).cast_unsigned()
    }

    /// `FUN_0077ca80()` (cdecl): the viewer's frame output: the decoded
    /// macroblock rows (`+0x50518` rows of `+0x50514` macroblocks, 0x180
    /// bytes each, from `+0x503cc`) loaded and drawn row by row through
    /// the two row buffers (`+0x503c0/+0x503c4`), each finished row
    /// copied to the display (`FUN_0077ec20`, unless `+0x50524`) at `y`
    /// (`+0x506e0`, plus `+0x50520` unless `+0x50763` or `+0x50534`; the
    /// original also divides 0 by the width `+0x5051c` unless
    /// `+0x5075f`, which only faults on a zero width). The GS register
    /// gets each row's command (`(field | 0xe) * 2 | parity` in the top
    /// bits). Returns the last `sceGsSyncPath`'s `eax`.
    pub fn frame_out(h: &mut impl Host) -> u32 {
        let v: u32 = h.read(h.rebase(VIEWER));
        let tf = sign_extend(h.read(at(v, 0x5_076e)));
        let fl = sign_extend(h.read(at(v, 0x5_0761)));
        let rows: u32 = h.read(at(v, 0x5_0518));
        let size = rows
            .wrapping_mul(h.read(at(v, 0x5_0514)))
            .wrapping_mul(0x180);
        let parity = if fl == 0 {
            0
        } else {
            sign_extend(h.read(at(v, 0x5_0762)))
        };
        let mut y: u32 = h.read(at(v, 0x5_06e0));
        if h.read::<u8>(at(v, 0x5_0763)) == 0 && h.read::<u32>(at(v, 0x5_0534)) == 0 {
            y = y.wrapping_add(h.read(at(v, 0x5_0520)));
        }
        cdecl(h, GS_SYNC_PATH, &[0, 0]);
        let v: u32 = h.read(h.rebase(VIEWER));
        let mbs: u32 = h.read(at(v, 0x5_03cc));
        cdecl(h, LOAD_IMAGE_101, &[mbs, size]);
        let mut other = 1u32;
        let mut idx = 0u32;
        let mut k = 0u32;
        let head = ((fl | 0xe).wrapping_mul(2) | parity) << 26;
        let mut row_y = y.wrapping_sub(16);
        let mut v: u32 = h.read(h.rebase(VIEWER));
        while k.cast_signed() < h.read::<i32>(at(v, 0x5_0518)) {
            let mb_w: u32 = h.read(at(v, 0x5_0514));
            let buf: u32 = h.read(at(v, 0x5_03c0 + 4 * idx));
            cdecl(h, LOAD_IMAGE, &[buf, mb_w << 10]);
            v = h.read(h.rebase(VIEWER));
            let mb_w: u32 = h.read(at(v, 0x5_0514));
            sink(h, mb_w | head);
            if k != 0 {
                if h.read::<u32>(at(v, 0x5_0524)) == 0 {
                    frame_copy(h, other, row_y, fl, tf);
                }
                cdecl(h, GS_SYNC_PATH, &[0, 0]);
                let v: u32 = h.read(h.rebase(VIEWER));
                let dst: u32 = h.read(at(v, 0x5_03d0));
                cdecl(h, SYNC_PATH2, &[dst]);
            }
            other ^= 1;
            idx ^= 1;
            wait_ready(h);
            wait_idle(h);
            v = h.read(h.rebase(VIEWER));
            k += 1;
            row_y = row_y.wrapping_add(16);
        }
        if h.read::<u32>(at(v, 0x5_0524)) == 0 {
            let last_y = y.wrapping_sub(16).wrapping_add(k << 4);
            frame_copy(h, other, last_y, fl, tf);
        }
        cdecl(h, GS_SYNC_PATH, &[0, 0]);
        let v: u32 = h.read(h.rebase(VIEWER));
        let dst: u32 = h.read(at(v, 0x5_03d0));
        cdecl(h, SYNC_PATH2, &[dst]);
        wait_idle(h);
        cdecl(h, GS_SYNC_PATH, &[0, 0])
    }

    /// `audioStop(v)`, `syncStrCheckPC_HDD(v)`, `MovieFrameWait(convert)`,
    /// `movieClose(fd)`, `syncStrCheck(v)`, `hdd_state_clear()`,
    /// `CdOpenCheck_sub(sync)`, `SndSetStraightStreamingDigitalMode(x)`,
    /// `TkSndUpdate2()`, `SndReportStandbyStatusDirect(x)` (cdecl), and
    /// the messages.
    pub const AUDIO_STOP: u32 = 0x0077_b170;
    pub const SYNC_STR_CHECK_HDD: u32 = 0x0077_e8e0;
    pub const FRAME_WAIT: u32 = 0x0077_dc50;
    pub const MOVIE_CLOSE: u32 = 0x0093_da00;
    pub const SYNC_STR_CHECK: u32 = 0x0077_e8b0;
    pub const HDD_STATE_CLEAR: u32 = 0x0077_7420;
    pub const CD_OPEN_CHECK: u32 = 0x0077_7000;
    pub const SND_DIGITAL_MODE: u32 = 0x0088_7210;
    pub const TK_SND_UPDATE2: u32 = 0x0088_7b80;
    pub const SND_STANDBY: u32 = 0x0088_6d00;
    pub const MSG_END: u32 = 0x00b5_428c;
    pub const MSG_SYNC_WAIT: u32 = 0x00b5_42a0;
    pub const MSG_FD: u32 = 0x00b5_3920;
    pub const MSG_HDD_FIN: u32 = 0x00b5_42b4;
    pub const MSG_CLOSE_FIN: u32 = 0x00b5_42d0;
    pub const MSG_SYNC_FIN: u32 = 0x00b5_42e4;
    pub const MSG_SOUND_WAIT: u32 = 0x00b5_42f8;
    pub const MSG_SOUND_FIN: u32 = 0x00b5_4310;

    fn printf(h: &mut impl Host, fmt: u32, arg: Option<u32>) -> u32 {
        let f = h.rebase(fmt);
        match arg {
            Some(a) => cdecl(h, DBG_PRINTF, &[f, a]),
            None => cdecl(h, DBG_PRINTF, &[f]),
        }
    }

    /// `mpegstrendsub(v, convert)` (`0x0077dab0`, cdecl): the movie's end.
    /// The audio stopped; an open stream (`+0x50544` not negative) waited
    /// out (HDD mode `+0x5075d`: `syncStrCheckPC_HDD`, then closed; the
    /// global viewer's CD mode `+0x5075c`: `syncStrCheck` until done, a
    /// failed read retried through `CdOpenCheck_sub` until it reports 0,
    /// 1 or 10; else `syncStrCheck`, then closed) and marked closed (-1);
    /// a pending sound mode change (`+0x50714`) switched back to digital
    /// and waited for. `MovieFrameWait(convert)` between polls. Returns
    /// the last call's `eax` (the caller's with no viewer).
    pub fn mpeg_str_end(h: &mut impl Host, v: u32, convert: u32) -> u32 {
        if v == 0 {
            return 0;
        }
        printf(h, MSG_END, Some(convert));
        let mut r = cdecl(h, AUDIO_STOP, &[v]);
        if h.read::<i32>(at(v, 0x5_0544)) >= 0 {
            printf(h, MSG_SYNC_WAIT, None);
            if h.read::<u8>(at(v, 0x5_075d)) != 0 {
                let fd: u32 = h.read(at(v, 0x5_0544));
                printf(h, MSG_FD, Some(fd));
                while cdecl(h, SYNC_STR_CHECK_HDD, &[v]) != 0 {
                    cdecl(h, FRAME_WAIT, &[convert]);
                }
                printf(h, MSG_HDD_FIN, None);
                let fd: u32 = h.read(at(v, 0x5_0544));
                cdecl(h, MOVIE_CLOSE, &[fd]);
                printf(h, MSG_CLOSE_FIN, None);
            } else {
                let g: u32 = h.read(h.rebase(VIEWER));
                if h.read::<u8>(at(g, 0x5_075c)) != 0 {
                    let mut s = cdecl(h, SYNC_STR_CHECK, &[v]);
                    while s != 0 {
                        if s != u32::MAX {
                            cdecl(h, HDD_STATE_CLEAR, &[]);
                            let c = cdecl(h, CD_OPEN_CHECK, &[convert]);
                            if c == 0 || c == 1 || c == 10 {
                                break;
                            }
                        }
                        cdecl(h, FRAME_WAIT, &[convert]);
                        s = cdecl(h, SYNC_STR_CHECK, &[v]);
                    }
                } else {
                    while cdecl(h, SYNC_STR_CHECK, &[v]) != 0 {
                        cdecl(h, FRAME_WAIT, &[convert]);
                    }
                    let fd: u32 = h.read(at(v, 0x5_0544));
                    cdecl(h, MOVIE_CLOSE, &[fd]);
                }
            }
            h.write(at(v, 0x5_0544), u32::MAX);
            r = printf(h, MSG_SYNC_FIN, None);
        }
        if h.read::<u32>(at(v, 0x5_0714)) != 0 {
            printf(h, MSG_SOUND_WAIT, None);
            cdecl(h, SND_DIGITAL_MODE, &[1]);
            loop {
                if convert == 0 {
                    cdecl(h, TK_SND_UPDATE2, &[]);
                }
                if cdecl(h, SND_STANDBY, &[0]) == 0 {
                    break;
                }
                cdecl(h, FRAME_WAIT, &[convert]);
            }
            h.write(at(v, 0x5_0714), 0u32);
            r = printf(h, MSG_SOUND_FIN, None);
        }
        r
    }

    /// `movie_err(msg, file, line)` (cdecl), the buffer report's formats
    /// and the two PS2 memory limits' messages.
    pub const MOVIE_ERR: u32 = 0x0076_efa0;
    pub const MSG_MEM_END: u32 = 0x00b5_4064;
    pub const MSG_BUFF_END: u32 = 0x00b5_407c;
    pub const MOVIE_STR_FILE: u32 = 0x00b5_40a8;
    pub const MSG_LIMIT: u32 = 0x00b5_40c8;
    pub const MSG_LIMIT_ONISHI: u32 = 0x00b5_40f0;
    pub const MSG_DISP_MODE: u32 = 0x00b5_4140;
    pub const MSG_MBX: u32 = 0x00b5_4154;
    pub const MSG_MBY: u32 = 0x00b5_4160;
    pub const MSG_FIELD: u32 = 0x00b5_416c;
    pub const MSG_DISPX: u32 = 0x00b5_4180;
    pub const MSG_DISPY: u32 = 0x00b5_4190;

    /// `code_0077df90()` (cdecl; not a function in the dump): the
    /// viewer's buffers carved from `+0x506f4`: two row buffers of
    /// `MBX << 10` bytes (`+0x503c0/+0x503c4`), the macroblocks at 0x300
    /// (`+0x503c8`) and 0x180 bytes (`+0x503cc`) each, the display
    /// (`+0x503d0`) and a guard word 0x12345678 (`+0x503d4`, MBX * 0x60 plus
    /// 0x50 past it). The end is checked against the PS2's work area
    /// limit (by `+0x5075f`; `movie_err` past it) and the layout printed.
    /// Returns the last `dbgPrintf`'s `eax`.
    pub fn alloc_buffers(h: &mut impl Host) -> u32 {
        let v: u32 = h.read(h.rebase(VIEWER));
        let mbx: u32 = h.read(at(v, 0x5_0514));
        let n = h.read::<u32>(at(v, 0x5_0518)).wrapping_mul(mbx);
        let base: u32 = h.read(at(v, 0x5_06f4));
        let row = mbx << 10;
        h.write(at(v, 0x5_03c0), base);
        h.write(at(v, 0x5_03c4), base.wrapping_add(row));
        let blocks = base.wrapping_add(row).wrapping_add(row);
        h.write(at(v, 0x5_03c8), blocks);
        let small = blocks.wrapping_add(n.wrapping_mul(0x300));
        h.write(at(v, 0x5_03cc), small);
        let disp = small.wrapping_add(n.wrapping_mul(0x180));
        let guard = mbx.wrapping_mul(0x60).wrapping_add(0x50).wrapping_add(disp);
        h.write(at(v, 0x5_03d4), guard);
        h.write(at(v, 0x5_03d0), disp);
        h.write(guard, 0x1234_5678u32);
        let end = h.read::<u32>(at(v, 0x5_03d4)).wrapping_add(4);
        printf(h, MSG_MEM_END, Some(end));
        let g: u32 = h.read(h.rebase(VIEWER));
        let (limit, msg, line) = if h.read::<u8>(at(g, 0x5_075f)) != 0 {
            (0x01fa_0040u32, MSG_LIMIT_ONISHI, 0x1aa)
        } else {
            (0x01f0_0000, MSG_LIMIT, 0x1a2)
        };
        let free = limit.wrapping_sub(end);
        let f = h.rebase(MSG_BUFF_END);
        cdecl(h, DBG_PRINTF, &[f, end, free, free]);
        if end >= limit {
            let (m, file) = (h.rebase(msg), h.rebase(MOVIE_STR_FILE));
            cdecl(h, MOVIE_ERR, &[m, file, line]);
        }
        let g: u32 = h.read(h.rebase(VIEWER));
        let mode = sign_extend(h.read(at(g, 0x5_075f)));
        printf(h, MSG_DISP_MODE, Some(mode));
        let mut r = 0;
        for (fmt, o) in [
            (MSG_MBX, 0x5_0514),
            (MSG_MBY, 0x5_0518),
            (MSG_FIELD, 0x5_0524),
            (MSG_DISPX, 0x5_051c),
            (MSG_DISPY, 0x5_0520),
        ] {
            let g: u32 = h.read(h.rebase(VIEWER));
            let x: u32 = h.read(at(g, o));
            r = printf(h, fmt, Some(x));
        }
        r
    }

    /// `SetByte_012fb8fb(v, _)`, `sa_warn_0076f170(_)`, `FUN_008ab860()`,
    /// `FUN_008adaf0(w, h)`, `FUN_0077ea40()`, `SPAD_CALL(stack, size, f,
    /// arg)` (cdecl), the buffer setup, the decoders and the size message.
    pub const SET_CRT_MODE: u32 = 0x0082_1a20;
    pub const SA_WARN: u32 = 0x0076_f170;
    pub const CRT_RESET: u32 = 0x008a_b860;
    pub const CRT_SIZE: u32 = 0x008a_daf0;
    pub const IPU_RESET: u32 = 0x0077_ea40;
    pub const SPAD_CALL: u32 = 0x0077_a480;
    pub const ALLOC_BUFFERS: u32 = 0x0077_df90;
    pub const INTRA_DECODE: u32 = 0x0077_b480;
    pub const INTER_DECODE: u32 = 0x0077_b630;
    pub const MSG_CRT_SIZE: u32 = 0x00b5_432c;

    /// `code_0077ce10()` (cdecl; not a function in the dump): the
    /// viewer's picture set-up from the stream record at the cursor
    /// (`+0xb` MBX, `+0xc` MBY; display 16 times both): the display mode
    /// `+0x5075f` (512 x 416: 0, or 6 without flag 0x10000 of `+0x505c0`;
    /// 320 wide: 2, x 416: 5; 640 wide: 1, x 416: 4; else kept); a new
    /// mode is printed and applied (the CRT, the buffers); then the IPU
    /// reset, the GS register set to 0x40000000 and the record's decoder
    /// (intra `0x0077b480`, or `0x0077b630` when `+8` is set) run on the
    /// scratch stack (`[0x0112a840] + 0x10d0`, 0x1800 bytes). Returns the
    /// viewer.
    pub fn picture_setup(h: &mut impl Host) -> u32 {
        let v: u32 = h.read(h.rebase(VIEWER));
        let cur: u32 = h.read(at(v, 0x5_059c));
        let mbx = u32::from(h.read::<u8>(at(cur, 0xb)));
        let mby = u32::from(h.read::<u8>(at(cur, 0xc)));
        h.write(at(v, 0x5_0514), mbx);
        h.write(at(v, 0x5_0518), mby);
        let (w, hgt) = (mbx << 4, mby << 4);
        h.write(at(v, 0x5_051c), w);
        h.write(at(v, 0x5_0520), hgt);
        let mode_at = at(v, 0x5_075f);
        match w {
            0x200 => {
                if hgt == 0x1a0 {
                    let flag = h.read::<u32>(at(v, 0x5_05c0)) & 0x1_0000;
                    h.write(mode_at, if flag == 0 { 6u8 } else { 0 });
                }
            }
            0x140 => h.write(mode_at, if hgt == 0x1a0 { 5u8 } else { 2 }),
            0x280 => h.write(mode_at, if hgt == 0x1a0 { 4u8 } else { 1 }),
            _ => {}
        }
        let mode: u8 = h.read(mode_at);
        let mut v = v;
        if mode != h.read::<u8>(at(v, 0x5_0760)) {
            let field: u32 = h.read(at(v, 0x5_0534));
            h.write(at(v, 0x5_0760), mode);
            printf(h, MSG_CRT_SIZE, Some(field));
            let g: u32 = h.read(h.rebase(VIEWER));
            let m = sign_extend(h.read(at(g, 0x5_075f)));
            cdecl(h, SET_CRT_MODE, &[m, 0]);
            let g: u32 = h.read(h.rebase(VIEWER));
            let m = sign_extend(h.read(at(g, 0x5_075f)));
            cdecl(h, SA_WARN, &[m]);
            cdecl(h, CRT_RESET, &[]);
            let g: u32 = h.read(h.rebase(VIEWER));
            let (dw, dh) = (
                h.read::<u32>(at(g, 0x5_051c)),
                h.read::<u32>(at(g, 0x5_0520)),
            );
            cdecl(h, CRT_SIZE, &[dw, dh]);
            cdecl(h, ALLOC_BUFFERS, &[]);
            v = h.read(h.rebase(VIEWER));
        }
        cdecl(h, IPU_RESET, &[]);
        sink(h, 0x4000_0000);
        let cur: u32 = h.read(at(v, 0x5_059c));
        let decoder = h.rebase(if h.read::<u8>(at(cur, 8)) == 0 {
            INTRA_DECODE
        } else {
            INTER_DECODE
        });
        let stack = h.read::<u32>(h.rebase(BLOCK)).wrapping_add(0x10d0);
        cdecl(h, SPAD_CALL, &[stack, 0x1800, decoder, 0]);
        let v: u32 = h.read(h.rebase(VIEWER));
        h.write(at(v, 0x5_06dc), 1u32);
        v
    }

    /// `MovieCdRead(sector, size, buf, v)`, `movie_hdd_file_open(n,
    /// sector)`, `sceClose(fd)`, `readSizeFile(name, buf)`,
    /// `movieOpenCd(v)`, `movieOpen(name)`, `movie_wait()`,
    /// `MovieTakeSizes(slot)`, `MovieRead(fd, buf, size, &read, v)`,
    /// `syncStrPcSelect(fd, b, v)` (cdecl), the messages.
    pub const MOVIE_CD_READ: u32 = 0x0077_7320;
    pub const HDD_FILE_OPEN: u32 = 0x0077_9be0;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    pub const READ_SIZE_FILE: u32 = 0x0093_da60;
    pub const MOVIE_OPEN_CD: u32 = 0x0077_7240;
    pub const MOVIE_OPEN: u32 = 0x0093_da30;
    pub const MOVIE_WAIT: u32 = 0x0077_daa0;
    pub const TAKE_SIZES: u32 = 0x0077_e160;
    pub const MOVIE_READ: u32 = 0x0077_e110;
    pub const STR_SELECT: u32 = 0x0077_e880;
    pub const MSG_SECTOR: u32 = 0x00b5_43a8;
    pub const MSG_FD_ASSERT: u32 = 0x00b5_43b4;
    pub const MSG_DAT: u32 = 0x00b5_43c0;
    pub const MSG_CANNOT_OPEN: u32 = 0x00b5_43d0;
    pub const MOVIE_START_SYM: u32 = 0x0077_cfa0;

    /// The next read of the stream ring: `MovieRead(fd, sizes, slot's
    /// buffer, &slot's read count, v)` for the ring slot `+0x5053c`.
    fn read_slot(h: &mut impl Host, fd: u32, sizes: u32) {
        let v: u32 = h.read(h.rebase(VIEWER));
        let k: u32 = h.read(at(v, 0x5_053c));
        let buf: u32 = h.read(at(v, 0x5_0560u32.wrapping_add(k.wrapping_mul(4))));
        let read = at(v, 0x5_0574u32.wrapping_add(k.wrapping_mul(4)));
        cdecl(h, MOVIE_READ, &[fd, sizes, buf, read, v]);
    }

    /// `FUN_0077cfa0(name, clut)` (cdecl): the PS2 movie's start. The
    /// CLUT data is read into `v + 0x10080` (CD mode `+0x5075c`: from the
    /// stream's sectors, through the HDD file in HDD mode; else the file
    /// `clut`) and its two 64-byte halves copied to the CLUT tables; the
    /// stream opened (`movieOpenCd` or `movieOpen(name)`; a failure is
    /// printed and waits forever, as in the original), its first two ring
    /// slots read (the first marked 2 and taken as the cursor, the ring
    /// index advanced modulo `+0x50754` and marked 1; the second read
    /// unless its size is the one already read). Returns the ring index
    /// (also stored at `+0x50540`). (The original divides by the ring
    /// size: 0 faults there; the port's index is then 0.)
    pub fn movie_start(h: &mut impl Host, name: u32, clut: u32) -> u32 {
        let v: u32 = h.read(h.rebase(VIEWER));
        if h.read::<u8>(at(v, 0x5_075c)) == 0 {
            let f = h.rebase(MSG_DAT);
            cdecl(h, DBG_PRINTF, &[f, name, clut]);
            let v: u32 = h.read(h.rebase(VIEWER));
            cdecl(h, READ_SIZE_FILE, &[clut, at(v, 0x1_0080), v]);
        } else {
            let sector: u32 = h.read(at(v, 0x5_06ec));
            let f = h.rebase(MSG_SECTOR);
            cdecl(h, DBG_PRINTF, &[f, sector]);
            let v: u32 = h.read(h.rebase(VIEWER));
            if h.read::<u8>(at(v, 0x5_075d)) == 0 {
                let (s, n): (u32, u32) = (h.read(at(v, 0x5_06ec)), h.read(at(v, 0x5_06f0)));
                cdecl(h, MOVIE_CD_READ, &[s, n, at(v, 0x1_0080), v]);
            } else {
                let file = sign_extend(h.read(at(v, 0x5_0764)));
                let s: u32 = h.read(at(v, 0x5_06ec));
                let fd = cdecl(h, HDD_FILE_OPEN, &[file, s]);
                let v: u32 = h.read(h.rebase(VIEWER));
                h.write(at(v, 0x5_0544), fd);
                if fd & 0x8000_0000 != 0 {
                    let (m, file) = (h.rebase(MSG_FD_ASSERT), h.rebase(MOVIE_STR_FILE));
                    cdecl(h, MOVIE_ERR, &[m, file, 0x894]);
                }
                let v: u32 = h.read(h.rebase(VIEWER));
                let (s, n): (u32, u32) = (h.read(at(v, 0x5_06ec)), h.read(at(v, 0x5_06f0)));
                cdecl(h, MOVIE_CD_READ, &[s, n, at(v, 0x1_0080), v]);
                let v: u32 = h.read(h.rebase(VIEWER));
                let fd: u32 = h.read(at(v, 0x5_0544));
                cdecl(h, SCE_CLOSE, &[fd]);
                let v: u32 = h.read(h.rebase(VIEWER));
                h.write(at(v, 0x5_0544), u32::MAX);
            }
        }
        cdecl(h, FLUSH_CACHE, &[0]);
        let v: u32 = h.read(h.rebase(VIEWER));
        let (a, b) = (h.rebase(CLUT_A), h.rebase(CLUT_B));
        for i in 0..0x40 {
            let x: u8 = h.read(at(v, 0x1_0080 + i));
            h.write(at(a, i), x);
            let y: u8 = h.read(at(v, 0x1_00c0 + i));
            h.write(at(b, i), y);
        }
        cdecl(h, FLUSH_CACHE, &[0]);
        let v: u32 = h.read(h.rebase(VIEWER));
        let fd = if h.read::<u8>(at(v, 0x5_075c)) == 0 {
            cdecl(h, MOVIE_OPEN, &[name, v])
        } else {
            cdecl(h, MOVIE_OPEN_CD, &[v])
        };
        h.write(at(v, 0x5_0544), fd);
        if fd & 0x8000_0000 != 0 {
            let f = h.rebase(MSG_CANNOT_OPEN);
            cdecl(h, DBG_PRINTF, &[f, name]);
            loop {
                cdecl(h, MOVIE_WAIT, &[]);
            }
        }
        let g: u32 = h.read(h.rebase(VIEWER));
        let k: u32 = h.read(at(g, 0x5_053c));
        let sizes = cdecl(h, TAKE_SIZES, &[k]);
        let fd: u32 = h.read(at(v, 0x5_0544));
        read_slot(h, fd, sizes);
        let g: u32 = h.read(h.rebase(VIEWER));
        let fd: u32 = h.read(at(g, 0x5_0544));
        cdecl(h, STR_SELECT, &[fd, 0, g]);
        let g: u32 = h.read(h.rebase(VIEWER));
        let k: u32 = h.read(at(g, 0x5_053c));
        h.write(at(g, 0x5_0588u32.wrapping_add(k.wrapping_mul(4))), 2u32);
        let k: u32 = h.read(at(g, 0x5_053c));
        let cursor: u32 = h.read(at(g, 0x5_0560u32.wrapping_add(k.wrapping_mul(4))));
        h.write(at(g, 0x5_059c), cursor);
        let ring: i32 = h.read(at(g, 0x5_0754));
        let next = k
            .cast_signed()
            .wrapping_add(1)
            .checked_rem(ring)
            .unwrap_or(0)
            .cast_unsigned();
        h.write(at(g, 0x5_053c), next);
        h.write(at(g, 0x5_0588u32.wrapping_add(next.wrapping_mul(4))), 1u32);
        cdecl(h, FLUSH_CACHE, &[0]);
        let g: u32 = h.read(h.rebase(VIEWER));
        let k: u32 = h.read(at(g, 0x5_053c));
        let sizes = cdecl(h, TAKE_SIZES, &[k]);
        let k: u32 = h.read(at(g, 0x5_053c));
        let taken: u32 = h.read(at(g, 0x5_054cu32.wrapping_add(k.wrapping_mul(4))));
        if taken == h.read::<u32>(at(g, 0x5_0548)) {
            h.write(at(g, 0x5_0540), k);
            return k;
        }
        let fd: u32 = h.read(at(g, 0x5_0544));
        read_slot(h, fd, sizes);
        let g: u32 = h.read(h.rebase(VIEWER));
        let k: u32 = h.read(at(g, 0x5_053c));
        h.write(at(g, 0x5_0540), k);
        k
    }

    /// `FFXGetVideoMode()` (cdecl).
    pub const GET_VIDEO_MODE: u32 = 0x0088_7c40;
    pub const VIEWER_INIT_SYM: u32 = 0x0077_d280;

    /// The viewer's field resets (offset, bytes, value), in the original's
    /// order.
    const RESETS: [(u32, u8, u32); 56] = [
        (0x5_0524, 4, 0x0),
        (0x5_0534, 4, 0x0),
        (0x5_0528, 4, 0x0),
        (0x5_052c, 4, 0x0),
        (0x5_0530, 4, 0x0),
        (0x5_0708, 4, 0xffff),
        (0x5_0538, 4, 0x0),
        (0x5_053c, 4, 0x0),
        (0x5_0540, 4, 0x0),
        (0x5_06cc, 4, 0x0),
        (0x5_06d0, 4, 0x0),
        (0x5_06d4, 4, 0xffff_ffff),
        (0x5_06d8, 4, 0x7f),
        (0x5_05a0, 4, 0x0),
        (0x5_05a4, 1, 0x0),
        (0x5_05a8, 4, 0x0),
        (0x5_05ac, 1, 0x0),
        (0x5_05c0, 4, 0x2),
        (0x5_06c4, 4, 0x0),
        (0x5_06c8, 4, 0x0),
        (0x5_06dc, 4, 0x0),
        (0x5_0544, 4, 0xffff_ffff),
        (0x5_075f, 4, 0xff00),
        (0x5_06e0, 4, 0x0),
        (0x5_0763, 1, 0x0),
        (0x5_0765, 4, 0x0),
        (0x5_0714, 4, 0x0),
        (0x5_071c, 4, 0x0),
        (0x5_070c, 4, 0x0),
        (0x5_0710, 4, 0x0),
        (0x5_076a, 2, 0x0),
        (0x5_0769, 1, 0x0),
        (0x5_0720, 4, 0x0),
        (0x5_0724, 4, 0xffff_ffff),
        (0x5_0728, 4, 0xffff_ffff),
        (0x5_076f, 4, 0x0),
        (0x5_0750, 4, 0x1),
        (0x5_0773, 1, 0x0),
        (0x5_0548, 4, 0x0),
        (0x5_076d, 2, 0x0),
        (0x5_0754, 4, 0x3),
        (0x5_0588, 4, 0x0),
        (0x5_0574, 4, 0x0),
        (0x5_054c, 4, 0x0),
        (0x5_058c, 4, 0x0),
        (0x5_0578, 4, 0x0),
        (0x5_0550, 4, 0x0),
        (0x5_0590, 4, 0x0),
        (0x5_057c, 4, 0x0),
        (0x5_0554, 4, 0x0),
        (0x5_0594, 4, 0x0),
        (0x5_0580, 4, 0x0),
        (0x5_0558, 4, 0x0),
        (0x5_0598, 4, 0x0),
        (0x5_0584, 4, 0x0),
        (0x5_055c, 4, 0x0),
    ];

    /// `FUN_0077d280(v, w, h, big)` (cdecl): `v` becomes the viewer
    /// (`0x00c421b0`); a given size sets the macroblock counts (`/ 16`,
    /// signed) and the display size; the playback state reset; the stream
    /// ring laid out after `+0x50774` (slot sizes 0xe4000, or 0x100000
    /// with `big`; the third slot 8 MB), the work area after it; the audio
    /// rate 0x6400 (0x7800 in video mode). Returns the rate.
    pub fn viewer_init(h: &mut impl Host, v: u32, w: u32, hgt: u32, big: u32) -> u32 {
        h.write(h.rebase(VIEWER), v);
        if w != 0 || hgt != 0 {
            h.write(at(v, 0x5_0514), (w.cast_signed() / 16).cast_unsigned());
            h.write(at(v, 0x5_0518), (hgt.cast_signed() / 16).cast_unsigned());
            h.write(at(v, 0x5_051c), w);
            h.write(at(v, 0x5_0520), hgt);
        }
        for (o, size, x) in RESETS {
            match size {
                1 => h.write(at(v, o), x.to_le_bytes()[0]),
                2 => h.write(
                    at(v, o),
                    u16::from_le_bytes([x.to_le_bytes()[0], x.to_le_bytes()[1]]),
                ),
                _ => h.write(at(v, o), x),
            }
        }
        let (slot, last) = if big == 0 {
            (0xe_4000u32, 0xe_3000u32)
        } else {
            (0x10_0000, 0xf_f000)
        };
        h.write(at(v, 0x5_06f8), last);
        h.write(at(v, 0x5_06fc), slot);
        let first = at(v, 0x5_0774);
        h.write(at(v, 0x5_0560), first);
        let second = first.wrapping_add(slot);
        h.write(at(v, 0x5_0564), second);
        h.write(at(v, 0x5_056c), 0x80_0000u32);
        let third = second.wrapping_add(slot);
        h.write(at(v, 0x5_0568), third);
        h.write(at(v, 0x5_0570), slot.wrapping_add(0x80_0000));
        h.write(at(v, 0x5_06f4), third.wrapping_add(slot));
        for i in 0..8 {
            h.write(at(v, 0x5_0730 + 4 * i), 0u32);
        }
        let mode = cdecl(h, GET_VIDEO_MODE, &[]);
        let rate = if mode == 0 { 0x6400u32 } else { 0x7800 };
        let g: u32 = h.read(h.rebase(VIEWER));
        h.write(at(g, 0x5_0758), rate);
        rate
    }

    /// The ADPCM filter coefficients (`(f0, f1)` `i32` pairs, indexed by
    /// the signed filter number).
    pub const ADPCM_FILTERS: u32 = 0x00c4_2088;
    pub const ADPCM_DECODE_SYM: u32 = 0x0077_a930;

    /// The low 16 bits, sign-extended (`test 0x8000; or 0xffff0000`).
    fn sext16(x: i32) -> i32 {
        let v = x & 0xffff;
        if v & 0x8000 != 0 { v - 0x1_0000 } else { v }
    }

    /// The low 16 bits (`mov word`).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the store
    fn low16(x: i32) -> u16 {
        x as u16
    }

    /// `FUN_0077a930(src, out, state)` (cdecl): 16 ADPCM blocks of 9 bytes
    /// (a header: shift in the low nibble, the filter in the high one,
    /// signed; then 8 bytes of nibbles, low first) decoded into 256
    /// 16-bit samples: each nibble `<< 12` (sign-extended from 16 bits)
    /// `>> shift`, plus `(f0 * s1 + f1 * s2) / 64` (toward zero) of the
    /// two previous samples, kept at full width. `state` holds `(s1, s2)`
    /// before and after. Returns `s2`.
    pub fn adpcm_decode(h: &mut impl Host, src: u32, out: u32, state: u32) -> u32 {
        let mut s1: i32 = h.read(state);
        let mut s2: i32 = h.read(at(state, 4));
        let filters = h.rebase(ADPCM_FILTERS);
        let (mut src, mut out) = (src, out);
        for _ in 0..16 {
            let header = i32::from(h.read::<i8>(src));
            let shift = (header & 0xf).cast_unsigned();
            let filter = (header >> 4).cast_unsigned();
            let f0: i32 = h.read(at(filters, filter.wrapping_mul(8)));
            let f1: i32 = h.read(at(filters, filter.wrapping_mul(8).wrapping_add(4)));
            for j in 0..8 {
                let b = i32::from(h.read::<i8>(at(src, 1 + j)));
                for nibble in [(b & 0xf) << 12, (b & 0xf0) << 8] {
                    let n = sext16(nibble);
                    let predicted = f0.wrapping_mul(s1).wrapping_add(f1.wrapping_mul(s2)) / 64;
                    let sample = predicted.wrapping_add(n >> shift);
                    h.write(out, low16(sample));
                    out = at(out, 2);
                    s2 = s1;
                    s1 = sample;
                }
            }
            src = at(src, 9);
        }
        h.write(state, s1);
        h.write(at(state, 4), s2);
        s2.cast_unsigned()
    }

    pub const CLUT_UPLOAD_SYM: u32 = 0x0077_b9b0;

    /// `code_0077b9b0()` (cdecl; not a function in the dump, which runs
    /// it together with the movie thread at `0x0077ba70`): the GS synced,
    /// the IPU reset, the two CLUT halves swizzled into the viewer
    /// (`+0`, `+0x40`) and each uploaded (64 bytes, `LoadImage101`) with
    /// its GS command (0x50000000, 0x58000000), waiting on the GS
    /// register after each. Returns the GS register's last value.
    pub fn clut_upload(h: &mut impl Host) -> u32 {
        cdecl(h, GS_SYNC_PATH, &[0, 0]);
        cdecl(h, IPU_INIT, &[]);
        cdecl(h, HARDWARE_INIT, &[]);
        let v: u32 = h.read(h.rebase(VIEWER));
        let (a, b) = (h.rebase(CLUT_A), h.rebase(CLUT_B));
        cdecl(h, CLUT_SWIZZLE, &[v, a, 0]);
        cdecl(h, CLUT_SWIZZLE, &[at(v, 0x40), b, 0]);
        cdecl(h, FLUSH_CACHE, &[0]);
        for (o, command) in [(0u32, 0x5000_0000u32), (0x40, 0x5800_0000)] {
            let v: u32 = h.read(h.rebase(VIEWER));
            cdecl(h, LOAD_IMAGE_101, &[at(v, o), 0x40]);
            sink(h, command);
            wait_idle(h);
            wait_ready(h);
        }
        sink_value(h)
    }
}
