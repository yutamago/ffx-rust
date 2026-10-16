//! The main module's GS packet (`Sg_Gs*`) and debug lines (`Sg_Db*`).
//!
//! The packet is an array of 16-byte quadwords at [`bridge::PACKET`];
//! [`bridge::COUNT`] is the next one. Each `Sg_Gs*` register writer fills
//! one A+D quadword (64-bit data, then the register number as 64 bits)
//! and advances the count. The originals build the data from `int`
//! arguments with 64-bit shifts and ORs of the sign-extended values, so a
//! negative field sets every bit above it; [`pack`] does the same.
//!
//! Most of them end with one of their arguments (or a computed field) in
//! `EAX`; the bridge returns that value so a caller that reads it gets
//! the original's.

/// Packs `fields` as the originals do: starting from the last, each
/// earlier one is OR-ed in (sign-extended) after shifting the value so far
/// left by the given width.
#[must_use]
pub fn pack(last: i32, fields: &[(u32, i32)]) -> u64 {
    let mut v = i64::from(last);
    for &(shift, f) in fields {
        v = (v << shift) | i64::from(f);
    }
    v.cast_unsigned()
}

/// `a / 2` rounded toward zero (`cdq; sub eax, edx; sar eax, 1`).
#[must_use]
pub const fn half(a: i32) -> i32 {
    a / 2
}

/// GS register numbers the writers use.
pub mod reg {
    /// `PRIM`.
    pub const PRIM: u32 = 0x00;
    /// `RGBAQ`.
    pub const RGBAQ: u32 = 0x01;
    /// `UV`.
    pub const UV: u32 = 0x03;
    /// `XYZF2`.
    pub const XYZF2: u32 = 0x04;
    /// `TEX1_1`.
    pub const TEX1_1: u32 = 0x14;
    /// `XYOFFSET_1`.
    pub const XYOFFSET_1: u32 = 0x18;
    /// `XYOFFSET_2`.
    pub const XYOFFSET_2: u32 = 0x19;
    /// `TEXFLUSH`.
    pub const TEXFLUSH: u32 = 0x3f;
    /// `SCISSOR_1`.
    pub const SCISSOR_1: u32 = 0x40;
    /// `SCISSOR_2`.
    pub const SCISSOR_2: u32 = 0x41;
    /// `ALPHA_1`.
    pub const ALPHA_1: u32 = 0x42;
    /// `DIMX`.
    pub const DIMX: u32 = 0x44;
    /// `DTHE`.
    pub const DTHE: u32 = 0x45;
    /// `TEST_1`.
    pub const TEST_1: u32 = 0x47;
    /// `TEST_2`.
    pub const TEST_2: u32 = 0x48;
    /// `ZBUF_1`.
    pub const ZBUF_1: u32 = 0x4e;
    /// `ZBUF_2`.
    pub const ZBUF_2: u32 = 0x4f;
    /// `FRAME_1`.
    pub const FRAME_1: u32 = 0x4c;
    /// `FRAME_2`.
    pub const FRAME_2: u32 = 0x4d;
}

/// The originals over game memory (`ffx_bridge::Mem`).
#[allow(clippy::many_single_char_names)] // coordinates, channels
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{half, pack, reg};

    /// The next quadword of the packet.
    pub const COUNT: Global<u32> = Global::new(0x0130_32bc);
    /// The quadword the open tag starts at (`Sg_GsStart`).
    pub const START: Global<u32> = Global::new(0x0130_32c0);
    /// The packet.
    pub const PACKET: Global<u32> = Global::new(0x0130_32c4);
    /// The open tag's quadword (`Sg_GsStart`).
    pub const TAG: Global<u32> = Global::new(0x0130_32c8);
    /// The debug line buffer's write pointer (`Sg_DbLine`, `Sg_DbVertex`).
    pub const DB_PTR: Global<u32> = Global::new(0x0130_32d0);
    /// `g_FFXIsDebug`: the debug lines are recorded only when set.
    pub const IS_DEBUG: Global<u8> = Global::new(0x0133_c910);
    /// `g_GuScreenSize`: height, then width (`i32`).
    pub const SCREEN_H: Global<i32> = Global::new(0x023c_bbe0);
    /// See [`SCREEN_H`].
    pub const SCREEN_W: Global<i32> = Global::new(0x023c_bbe4);

    /// The quadword at the count.
    fn next(mem: &impl Mem) -> u32 {
        PACKET.get(mem).wrapping_add(COUNT.get(mem) << 4)
    }

    fn bump(mem: &mut impl Mem) {
        let n = COUNT.get(mem).wrapping_add(1);
        COUNT.set(mem, n);
    }

    /// Writes an A+D quadword at the count and advances it; the quadword.
    fn put(mem: &mut impl Mem, data: u64, r: u32) -> u32 {
        let q = next(mem);
        mem.write(q, [data, u64::from(r)]);
        bump(mem);
        q
    }

    /// `Sg_SetScreenWH(w, h)` (`0x00821d10`): `EAX` ends as `h`.
    pub fn set_screen_wh(mem: &mut impl Mem, w: i32, h: i32) -> i32 {
        SCREEN_W.set(mem, w);
        SCREEN_H.set(mem, h);
        h
    }

    /// `Sg_GsInc()` (`0x00841a50`).
    pub fn inc(mem: &mut impl Mem) {
        bump(mem);
    }

    /// `Sg_GsStartVIF()` (`0x00841d50`): the count to 3 (after the VIF
    /// codes).
    pub fn start_vif(mem: &mut impl Mem) {
        COUNT.set(mem, 3);
    }

    /// `Sg_GsGet()` (`0x00841a30`): the quadword at the count.
    pub fn get(mem: &impl Mem) -> u32 {
        next(mem)
    }

    /// `Sg_GsStart()` (`0x00841d10`): opens a GIF tag (`NLOOP` 0x4d2 for
    /// now, `NREG` 1, `A+D`) at the count; the tag's quadword.
    pub fn start(mem: &mut impl Mem) -> u32 {
        START.set(mem, COUNT.get(mem));
        let q = next(mem);
        mem.write(q, [0x4d2u32, 0x1000_0000, 0xe, 0]);
        TAG.set(mem, q);
        bump(mem);
        q
    }

    /// `Sg_GsEnd2()` (`0x00841800`): the DMA and VIF headers in the first
    /// three quadwords, sized by the count; returns the quadword at the
    /// count.
    pub fn end2(mem: &mut impl Mem) -> u32 {
        let n = COUNT.get(mem);
        let p = PACKET.get(mem);
        let words: [u32; 12] = [
            n.wrapping_sub(1) | 0x1000_0000,
            0,
            0,
            0,
            0,
            0,
            0x1000_0000,
            n.wrapping_sub(2) | 0x5000_0000,
            n.wrapping_sub(3) | 0x8000,
            0x1000_0000,
            0xe,
            0,
        ];
        mem.write(p, words);
        p.wrapping_add(n << 4)
    }

    /// `Sg_GsTEXFLUSH()` (`0x00841ee0`): the register half only (the
    /// data `TEXFLUSH` ignores is left as it was); the packet's address.
    pub fn texflush(mem: &mut impl Mem) -> u32 {
        let q = next(mem);
        mem.write(q.wrapping_add(8), u64::from(reg::TEXFLUSH));
        bump(mem);
        PACKET.get(mem)
    }

    /// `Sg_GsPRIM(prim)` (`0x00841a60`); `prim`.
    pub fn prim(mem: &mut impl Mem, prim: i32) -> i32 {
        put(mem, pack(prim, &[]), reg::PRIM);
        prim
    }

    /// `Sg_GsDTHE(on)` (`0x00841730`); `on`.
    pub fn dthe(mem: &mut impl Mem, on: i32) -> i32 {
        put(mem, pack(on, &[]), reg::DTHE);
        on
    }

    /// `Sg_GsDIMX()` (`0x008416f0`): the fixed dither matrix
    /// `0xffff_ffff_ffff_fffc`; the packet's address.
    pub fn dimx(mem: &mut impl Mem) -> u32 {
        put(mem, 0xffff_ffff_ffff_fffc, reg::DIMX);
        PACKET.get(mem)
    }

    /// `Sg_GsRGBAQ(r, g, b, a)` (`0x00841aa0`); `r`.
    pub fn rgbaq(mem: &mut impl Mem, r: i32, g: i32, b: i32, a: i32) -> i32 {
        put(mem, pack(a, &[(8, b), (8, g), (8, r)]), reg::RGBAQ);
        r
    }

    /// `Sg_GsALPHA(a, b, c, d, fix)` (`0x00841670`); `a`.
    pub fn alpha(mem: &mut impl Mem, abcd: [i32; 4], fix: i32) -> i32 {
        let [a, b, c, d] = abcd;
        put(
            mem,
            pack(fix, &[(26, d), (2, c), (2, b), (2, a)]),
            reg::ALPHA_1,
        );
        a
    }

    fn test(mem: &mut impl Mem, f: [i32; 8], r: u32) -> i32 {
        let [ate, atst, aref, afail, dest_enable, dest_mode, zte, ztst] = f;
        let word = pack(
            ztst,
            &[
                (1, zte),
                (1, dest_mode),
                (1, dest_enable),
                (2, afail),
                (8, aref),
                (3, atst),
                (1, ate),
            ],
        );
        put(mem, word, r);
        ate
    }

    /// `Sg_GsTEST(ate, atst, aref, afail, date, datm, zte, ztst)`
    /// (`0x00841d80`); `ate`.
    pub fn test_1(mem: &mut impl Mem, f: [i32; 8]) -> i32 {
        test(mem, f, reg::TEST_1)
    }

    /// `Sg_GsTEST2(...)` (`0x00841e30`), for context 2.
    pub fn test_2(mem: &mut impl Mem, f: [i32; 8]) -> i32 {
        test(mem, f, reg::TEST_2)
    }

    fn scissor(mem: &mut impl Mem, x0: i32, x1: i32, y0: i32, y1: i32, r: u32) -> i32 {
        let x0 = x0.max(0);
        let x1 = x1.min(SCREEN_W.get(mem).wrapping_sub(1));
        let y0 = y0.max(0);
        let y1 = y1.min(SCREEN_H.get(mem).wrapping_sub(1));
        put(mem, pack(y1, &[(16, y0), (16, x1), (16, x0)]), r);
        x0
    }

    /// `Sg_GsSCISSOR(x0, x1, y0, y1)` (`0x00841b90`): clamped to the
    /// screen (signed); the clamped `x0`.
    pub fn scissor_1(mem: &mut impl Mem, x0: i32, x1: i32, y0: i32, y1: i32) -> i32 {
        scissor(mem, x0, x1, y0, y1, reg::SCISSOR_1)
    }

    /// `Sg_GsSCISSOR2(x0, x1, y0, y1)` (`0x00841c40`).
    pub fn scissor_2(mem: &mut impl Mem, x0: i32, x1: i32, y0: i32, y1: i32) -> i32 {
        scissor(mem, x0, x1, y0, y1, reg::SCISSOR_2)
    }

    fn frame(mem: &mut impl Mem, fbp: u32, fbmsk: u32, r: u32) -> u32 {
        let fbw = ((SCREEN_W.get(mem) / 64) & 0x3f).cast_unsigned() | 0x100;
        let lo = (fbp & 0x1ff) | (fbw << 16);
        put(mem, u64::from(lo) | u64::from(fbmsk) << 32, r);
        fbmsk
    }

    /// `Sg_GsFRAME(fbp, fbmsk)` (`0x00841870`): the frame buffer at `fbp`,
    /// the screen's width in 64-pixel units, `PSMCT16`-less 32-bit format;
    /// `fbmsk`.
    pub fn frame_1(mem: &mut impl Mem, fbp: u32, fbmsk: u32) -> u32 {
        frame(mem, fbp, fbmsk, reg::FRAME_1)
    }

    /// `Sg_GsFRAME2(fbp, fbmsk)` (`0x008418f0`).
    pub fn frame_2(mem: &mut impl Mem, fbp: u32, fbmsk: u32) -> u32 {
        frame(mem, fbp, fbmsk, reg::FRAME_2)
    }

    fn frame_16(mem: &mut impl Mem, fbp: u32, fbmsk: u32, r: u32) -> u32 {
        let lo = (fbp & 0x1ff) | 0x0a08_0000;
        put(mem, u64::from(lo) | u64::from(fbmsk) << 32, r);
        fbmsk
    }

    /// `Sg_GsFRAME_16(fbp, fbmsk)` (`0x008419d0`): width 8 (512 pixels),
    /// 16-bit format.
    pub fn frame_16_1(mem: &mut impl Mem, fbp: u32, fbmsk: u32) -> u32 {
        frame_16(mem, fbp, fbmsk, reg::FRAME_1)
    }

    /// `Sg_GsFRAME2_16(fbp, fbmsk)` (`0x00841970`).
    pub fn frame_16_2(mem: &mut impl Mem, fbp: u32, fbmsk: u32) -> u32 {
        frame_16(mem, fbp, fbmsk, reg::FRAME_2)
    }

    fn xyoffset(mem: &mut impl Mem, x: i32, y: i32, r: u32) -> i32 {
        let yv = 0x800i32
            .wrapping_sub(half(SCREEN_H.get(mem)))
            .wrapping_sub(y)
            .wrapping_shl(4);
        let xv = 0x800i32
            .wrapping_sub(half(SCREEN_W.get(mem)))
            .wrapping_sub(x)
            .wrapping_shl(4);
        put(mem, pack(yv, &[(32, xv)]), r);
        xv
    }

    /// `Sg_GsXYOFFSET(x, y)` (`0x00842000`): `(2048 - w/2 - x) * 16` and
    /// the same for `y` (12.4 fixed point); the x field.
    pub fn xyoffset_1(mem: &mut impl Mem, x: i32, y: i32) -> i32 {
        xyoffset(mem, x, y, reg::XYOFFSET_1)
    }

    /// `Sg_GsXYOFFSET2(x, y)` (`0x00842070`).
    pub fn xyoffset_2(mem: &mut impl Mem, x: i32, y: i32) -> i32 {
        xyoffset(mem, x, y, reg::XYOFFSET_2)
    }

    /// The 12.4 screen position of `(x, y)` centred in 4096.
    fn screen_xy(mem: &impl Mem, x: i32, y: i32) -> (i32, i32) {
        let xv = half(0x1000i32.wrapping_sub(SCREEN_W.get(mem)))
            .wrapping_add(x)
            .wrapping_shl(4);
        let yv = half(0x1000i32.wrapping_sub(SCREEN_H.get(mem)))
            .wrapping_add(y)
            .wrapping_shl(4);
        (xv, yv)
    }

    /// Writes an `XYZF2` quadword: the 16-bit x and y, then `zf`.
    fn xyzf2_put(mem: &mut impl Mem, xv: i32, yv: i32, zf: u32) {
        let q = next(mem);
        mem.write(q, crate::lo16(xv.cast_unsigned()));
        mem.write(q.wrapping_add(2), crate::lo16(yv.cast_unsigned()));
        mem.write(q.wrapping_add(4), [zf, reg::XYZF2, 0]);
        bump(mem);
    }

    /// `Sg_GsXYZF2(x, y)` (`0x00842140`): z and fog 0; the y field.
    pub fn xyzf2(mem: &mut impl Mem, x: i32, y: i32) -> i32 {
        let (xv, yv) = screen_xy(mem, x, y);
        xyzf2_put(mem, xv, yv, 0);
        yv
    }

    /// `Sg_GsXYZF2V(x, y, zf)` (`0x00842210`); `zf`.
    pub fn xyzf2v(mem: &mut impl Mem, x: i32, y: i32, zf: u32) -> u32 {
        let (xv, yv) = screen_xy(mem, x, y);
        xyzf2_put(mem, xv, yv, zf);
        zf
    }

    fn uv(mem: &mut impl Mem, u: u32, v: u32) -> u32 {
        let q = next(mem);
        mem.write(q, crate::lo16(u));
        mem.write(q.wrapping_add(2), crate::lo16(v));
        mem.write(q.wrapping_add(4), [0, reg::UV, 0]);
        bump(mem);
        v
    }

    /// `Sg_GsUV(u, v)` (`0x00841f60`): texel corners (`u * 16`); `v * 16`.
    pub fn uv_1(mem: &mut impl Mem, u: u32, v: u32) -> u32 {
        uv(mem, u.wrapping_shl(4), v.wrapping_shl(4))
    }

    /// `Sg_GsUV2(u, v)` (`0x00841fb0`): texel ends (`u * 16 + 15`).
    pub fn uv_2(mem: &mut impl Mem, u: u32, v: u32) -> u32 {
        uv(
            mem,
            u.wrapping_shl(4).wrapping_add(0xf),
            v.wrapping_shl(4).wrapping_add(0xf),
        )
    }

    /// `Sg_GsTexHokan(on)` (`0x00841f10`): `TEX1` with bilinear filtering
    /// (0x25) or nearest (5); the quadword.
    pub fn tex_hokan(mem: &mut impl Mem, on: u32) -> u32 {
        put(mem, if on == 0 { 5 } else { 0x25 }, reg::TEX1_1)
    }

    /// `Sg_GsloadSetBase(load)` (`0x0083d370`): the low 14 bits of the
    /// word at `+0x24` from the `u16` at `+0x3a`.
    pub fn gsload_set_base(mem: &mut impl Mem, load: u32) {
        let w: u32 = mem.read(load.wrapping_add(0x24));
        let b: u16 = mem.read(load.wrapping_add(0x3a));
        mem.write(
            load.wrapping_add(0x24),
            (w & !0x3fff) | (u32::from(b) & 0x3fff),
        );
    }

    fn db_word(mem: &mut impl Mem, w: u32) {
        if IS_DEBUG.get(mem) != 0 {
            let p = DB_PTR.get(mem);
            mem.write(p, w);
            DB_PTR.set(mem, p.wrapping_add(4));
        }
    }

    /// `Sg_DbLine()` (`0x00841470`): starts a line (`0x10001`) in debug builds.
    pub fn db_line(mem: &mut impl Mem) {
        db_word(mem, 0x0001_0001);
    }

    /// `Sg_DbLineStrip()` (`0x00841490`): starts a line strip (`0x20001`).
    pub fn db_line_strip(mem: &mut impl Mem) {
        db_word(mem, 0x0002_0001);
    }

    /// `Sg_DbVertex(v)` (`0x00841610`): a vertex record (3, x, y, z) in
    /// debug builds.
    pub fn db_vertex(mem: &mut impl Mem, v: u32) {
        if IS_DEBUG.get(mem) == 0 {
            return;
        }
        let p = DB_PTR.get(mem);
        mem.write(p, 3u32);
        let x: f32 = mem.read(v);
        mem.write(p.wrapping_add(4), ffx_bridge::x87::round_trip(x));
        DB_PTR.set(mem, p.wrapping_add(0x10));
        for k in 1..3 {
            let c: f32 = mem.read(v.wrapping_add(4 * k));
            mem.write(p.wrapping_add(4 + 4 * k), ffx_bridge::x87::round_trip(c));
        }
    }

    /// A vertex record (3, x, y, z) from values, as `Sg_DbVertex` writes
    /// it from a vector.
    fn db_vertex_xyz(mem: &mut impl Mem, v: [f32; 3]) {
        if IS_DEBUG.get(mem) == 0 {
            return;
        }
        let p = DB_PTR.get(mem);
        mem.write(p, 3u32);
        mem.write(p.wrapping_add(4), v.map(ffx_bridge::x87::round_trip));
        DB_PTR.set(mem, p.wrapping_add(0x10));
    }

    /// `Sg_GsSetPacket(p)` (`0x00841cf0`): the packet at `p`, or at
    /// `Sg_PacketGetPtr()` for null, the count at 1; returns the packet.
    pub fn set_packet(mem: &mut impl Mem, p: u32) -> u32 {
        let p = if p == 0 {
            crate::state::bridge::packet_get_ptr(mem)
        } else {
            p
        };
        COUNT.set(mem, 1);
        PACKET.set(mem, p);
        p
    }

    fn zbuf(mem: &mut impl Mem, zmsk: u32, r: u32) -> u32 {
        let zbp = crate::state::bridge::vram_get_addr_z().cast_signed() / 2048;
        let lo = zbp.cast_unsigned() | 0x3100_0000;
        put(mem, u64::from(lo) | u64::from(zmsk) << 32, r);
        zmsk
    }

    /// `Sg_GsZBUF(zmsk)` (`0x008422f0`): the depth buffer at
    /// `Sg_VramGetAddrZ() / 2048`, 24-bit; `zmsk` above.
    pub fn zbuf_1(mem: &mut impl Mem, zmsk: u32) -> u32 {
        zbuf(mem, zmsk, reg::ZBUF_1)
    }

    /// `Sg_GsZBUF2(zmsk)` (`0x00842350`).
    pub fn zbuf_2(mem: &mut impl Mem, zmsk: u32) -> u32 {
        zbuf(mem, zmsk, reg::ZBUF_2)
    }

    /// The debug font texture: base pointer (`u16` at `+2`... of the word
    /// at `0x01325170`) and width (`0x01325220`) (`Sg_SetDebugTEX_0`).
    pub const DEBUG_TEX: Global<u32> = Global::new(0x0132_5170);
    /// The debug font texture's buffer width word.
    pub const DEBUG_TEX_WIDTH: Global<u32> = Global::new(0x0132_5220);

    /// `Sg_SetDebugTEX_0(q)` (`0x00851160`): an A+D `TEX0_1` quadword at
    /// `q` for the debug font: the data merged into what `q` held (`TBP0`
    /// from the high half of [`DEBUG_TEX`], `TBW` from the width, `PSM`,
    /// `TW`, `TH`, `TCC`, `TFX`, `CBP` from its low half); returns the high
    /// data word.
    pub fn set_debug_tex0(mem: &mut impl Mem, q: u32) -> u32 {
        let tex = DEBUG_TEX.get(mem);
        let lo: u32 = mem.read(q);
        let lo = (lo & !0x3fff) | ((tex >> 16) & 0x3fff);
        let width = (DEBUG_TEX_WIDTH.get(mem) & 0x1f80) | 0x3a_8000;
        let lo = (lo & 0xdd40_3fff) | (width << 7);
        let hi: u32 = mem.read(q.wrapping_add(4));
        let hi = (hi & 0xffff_ffb7) | 0x37;
        let hi = (hi & 0x8000_007f) | ((tex & 0x3fff) << 7);
        mem.write(q, [lo, hi, 6, 0]);
        hi
    }

    /// `Sg_Db2DVertex(x, y)` (`0x00840fc0`): a 2D vertex record (`u16` 4,
    /// x, y) in debug builds.
    pub fn db_2d_vertex(mem: &mut impl Mem, x: u32, y: u32) {
        if IS_DEBUG.get(mem) == 0 {
            return;
        }
        let p = DB_PTR.get(mem);
        mem.write(p, [4, crate::lo16(x), crate::lo16(y)]);
        DB_PTR.set(mem, p.wrapping_add(8));
    }

    /// `Sg_DbColor(r, g, b)` (`0x00841230`): a colour record (2, r, g, b)
    /// in debug builds.
    pub fn db_color(mem: &mut impl Mem, r: u32, g: u32, b: u32) {
        if IS_DEBUG.get(mem) == 0 {
            return;
        }
        let p = DB_PTR.get(mem);
        mem.write(p, [2, crate::lo8(r), crate::lo8(g), crate::lo8(b)]);
        DB_PTR.set(mem, p.wrapping_add(4));
    }

    /// The debug circles' unit circle: 13 cosines, then 13 sines (`f32`).
    pub const DB_COS: Global<[f32; 13]> = Global::new(0x0130_32d4);
    /// See [`DB_COS`].
    pub const DB_SIN: Global<[f32; 13]> = Global::new(0x0130_3308);

    /// A circle of radius `r` around `c` as a line strip of 13 vertices:
    /// component `a` gets `cos * r + c.a`, component `b` `r * sin + c.b`,
    /// the third `c` as it is.
    fn db_circle(mem: &mut impl Mem, c: u32, r: f32, a: usize, b: usize) {
        use ffx_bridge::x87::Reg;
        db_line_strip(mem);
        let r = Reg::load(r);
        for k in 0..13 {
            let cos = DB_COS.get(mem)[k];
            let sin = DB_SIN.get(mem)[k];
            let raw: [f32; 3] = mem.read(c);
            let mut v = raw.map(ffx_bridge::x87::round_trip);
            v[a] = (Reg::load(cos) * r).add_m32(raw[a]).store();
            v[b] = r.mul_m32(sin).add_m32(raw[b]).store();
            db_vertex_xyz(mem, v);
        }
    }

    /// `Sg_DbCircle(c, r)` (`0x008410a0`): in the x-z plane.
    pub fn db_circle_xz(mem: &mut impl Mem, c: u32, r: f32) {
        db_circle(mem, c, r, 0, 2);
    }

    /// `Sg_DbCircleXY(c, r)` (`0x00841170`).
    pub fn db_circle_xy(mem: &mut impl Mem, c: u32, r: f32) {
        db_circle(mem, c, r, 0, 1);
    }

    /// `Sg_DbCircleYZ(c, r)` (`0x008411d0`).
    pub fn db_circle_yz(mem: &mut impl Mem, c: u32, r: f32) {
        db_circle(mem, c, r, 1, 2);
    }

    /// `Sg_DbNull(c, s)` (`0x008414b0`): a locator, three lines of length
    /// `2 s` through `c` along the axes (one `Sg_DbLine`, six vertices).
    pub fn db_null(mem: &mut impl Mem, c: u32, s: f32) {
        use ffx_bridge::x87::{Reg, round_trip};
        db_line(mem);
        let c0: f32 = mem.read(c);
        let c1: f32 = mem.read(c.wrapping_add(4));
        let c2: f32 = mem.read(c.wrapping_add(8));
        let mut v = [c0, c1, c2].map(round_trip);
        v[0] = Reg::load(c0).sub_m32(s).store();
        db_vertex_xyz(mem, v);
        let c0: f32 = mem.read(c);
        v[0] = Reg::load(c0).add_m32(s).store();
        db_vertex_xyz(mem, v);
        let c0: f32 = mem.read(c);
        let c1: f32 = mem.read(c.wrapping_add(4));
        v[0] = round_trip(c0);
        v[1] = Reg::load(c1).sub_m32(s).store();
        db_vertex_xyz(mem, v);
        let c1: f32 = mem.read(c.wrapping_add(4));
        v[1] = Reg::load(s).add_m32(c1).store();
        db_vertex_xyz(mem, v);
        let c1: f32 = mem.read(c.wrapping_add(4));
        let c2: f32 = mem.read(c.wrapping_add(8));
        v[1] = round_trip(c1);
        v[2] = Reg::load(c2).sub_m32(s).store();
        db_vertex_xyz(mem, v);
        let c2: f32 = mem.read(c.wrapping_add(8));
        v[2] = Reg::load(c2).add_m32(s).store();
        db_vertex_xyz(mem, v);
    }
}
