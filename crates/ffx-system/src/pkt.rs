//! The `TO` layer's GS packet buffer, screen size and menu scissor.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// Whether packets are added (`TOInitAddPkt` 1, `TOSetAddPkt` 0).
    pub const ADD_PKT: Global<u32> = Global::new(0x00c5_d780);
    /// The packet buffer: start, then two more addresses into it
    /// (`TOSetPktBuffAdrs`).
    pub const PKT_BUFF: Global<[u32; 3]> = Global::new(0x0187_16c0);
    /// Screen width (`u16` read).
    pub const SCREEN_W: Global<u16> = Global::new(0x00c5_d770);
    /// The menu scissor's switch (`ffx_menu::scissor`'s too).
    pub const SCISSOR_ON: Global<u32> = Global::new(0x0187_16d0);
    /// The menu scissor's XYUV mode (`u8`).
    pub const SCISSOR_XYUV: Global<u8> = Global::new(0x0187_16d4);
    /// The menu scissor's box: left, top, right, bottom.
    pub const SCISSOR_BOX: Global<[f32; 4]> = Global::new(0x0187_16f8);
    /// Screen height (`u16` read).
    pub const SCREEN_H: Global<u16> = Global::new(0x00c5_d774);

    /// `TOInitAddPkt()` (`0x00904650`).
    pub fn init_add_pkt(mem: &mut impl Mem) {
        ADD_PKT.set(mem, 1);
    }

    /// `TOSetAddPkt()` (`0x008fae10`).
    pub fn set_add_pkt(mem: &mut impl Mem) {
        ADD_PKT.set(mem, 0);
    }

    /// `TOSetPktBuffAdrs(base, a, b)` (`0x009047c0`): `base + a` first,
    /// then `base`, then `base + b`.
    pub fn set_pkt_buff(mem: &mut impl Mem, base: u32, a: u32, b: u32) {
        let p = PKT_BUFF.ptr(mem).addr;
        mem.write(p + 4, base.wrapping_add(a));
        mem.write(p, base);
        mem.write(p + 8, base.wrapping_add(b));
    }

    /// `TOGetScreenWH(&w, &h)` (`0x008fa900`).
    pub fn screen_wh(mem: &mut impl Mem, w: u32, h: u32) {
        let v = SCREEN_W.get(mem);
        mem.write(w, v);
        let v = SCREEN_H.get(mem);
        mem.write(h, v);
    }

    /// `TODrawScissorXYWH(x, y, w, h)` (`0x008f9230`): the menu scissor on
    /// (not in XYUV mode), its box `(x, y, x + w, y + h)` as floats (each
    /// `i32` rounded to `f32`, the sums wrapping). Returns `y + h`, left in
    /// `eax`.
    pub fn draw_scissor_xywh(mem: &mut impl Mem, x: i32, y: i32, w: i32, h: i32) -> u32 {
        let (right, bottom) = (x.wrapping_add(w), y.wrapping_add(h));
        #[allow(clippy::cast_precision_loss)] // fild then fstp dword: rounded
        let corners = [x, y, right, bottom].map(|v| v as f32);
        SCISSOR_ON.set(mem, 1);
        SCISSOR_XYUV.set(mem, 0);
        SCISSOR_BOX.set(mem, corners);
        bottom.cast_unsigned()
    }

    /// `TOMakePktScissor(x0, y0, x1, y1)` (`0x008fdee0`, the same code at
    /// `0x008f91f0`): the menu scissor on (not in XYUV mode), its box the
    /// four integers as floats.
    pub fn make_pkt_scissor(mem: &mut impl Mem, x0: i32, y0: i32, x1: i32, y1: i32) {
        #[allow(clippy::cast_precision_loss)] // fild then fstp dword: rounded
        let corners = [x0, y0, x1, y1].map(|v| v as f32);
        SCISSOR_ON.set(mem, 1);
        SCISSOR_XYUV.set(mem, 0);
        SCISSOR_BOX.set(mem, corners);
    }

    /// `0x008fa7b0()`: a constant GS register word, in `edx:eax`.
    pub const GS_WORD_008FA7B0: u64 = 0x2005_d406_2131_3e00;

    /// `TOAdpWords4(p, a, b, c, d)` (`0x008ef8e0`): four words into the
    /// packet at `p`; returns `p + 0x10`.
    pub fn adp_words4(mem: &mut impl Mem, p: u32, w: [u32; 4]) -> u32 {
        mem.write(p, w);
        p.wrapping_add(0x10)
    }

    /// `TOAdpTEST(p, ate, atst, aref, afail, date, datm, zte, ztst)`
    /// (`0x008ef970`): the GS `TEST_2` register (0x47) from its fields,
    /// `ztst` highest; each field is sign-extended to 64 bits and OR-ed in
    /// unmasked, as the original does. Returns `p + 0x10`.
    pub fn adp_test(mem: &mut impl Mem, p: u32, f: [i32; 8]) -> u32 {
        // shifts before or-ing in ate .. zte (ztst comes first)
        const SHIFTS: [u32; 7] = [1, 3, 8, 2, 1, 1, 1];
        let mut v = i64::from(f[7]).cast_unsigned();
        for k in (0..7).rev() {
            v = (v << SHIFTS[k]) | i64::from(f[k]).cast_unsigned();
        }
        mem.write(p, v);
        mem.write(p.wrapping_add(8), 0x47u64);
        p.wrapping_add(0x10)
    }

    /// `TOAdpXY2(p, x, y)` (`0x008efb90`): `x * 16 + 8` and `y * 16 + 8`
    /// (12.4 fixed point, pixel centres) into the first two words; returns
    /// `p + 0x10`.
    pub fn adp_xy2(mem: &mut impl Mem, p: u32, x: u32, y: u32) -> u32 {
        mem.write(p, x.wrapping_shl(4).wrapping_add(8));
        mem.write(p.wrapping_add(4), y.wrapping_shl(4).wrapping_add(8));
        p.wrapping_add(0x10)
    }

    /// `TOAdpXYZ2Offset(p, x, y)` (`0x008efbf0`): `(x + 0x700) * 16`, `(y +
    /// 0x730) * 16`, 16, 0 (the GS primitive offset applied); returns `p +
    /// 0x10`.
    pub fn adp_xyz2_offset(mem: &mut impl Mem, p: u32, x: u32, y: u32) -> u32 {
        mem.write(
            p,
            [
                x.wrapping_add(0x700).wrapping_shl(4),
                y.wrapping_add(0x730).wrapping_shl(4),
                0x10,
                0,
            ],
        );
        p.wrapping_add(0x10)
    }

    /// `TOAdpGsDirectRGBA(p, rgba)` (`0x008ef790`): the four bytes of
    /// `rgba` (red low) as four words; returns `p + 0x10`.
    pub fn adp_gs_direct_rgba(mem: &mut impl Mem, p: u32, rgba: u32) -> u32 {
        mem.write(p, rgba.to_le_bytes().map(u32::from));
        p.wrapping_add(0x10)
    }

    /// `OpenFontScissor(x0, y0, x1, y1)` (`0x008e8940`): the menu scissor
    /// on in XYUV mode, its box the four floats (through the x87).
    pub fn open_font_scissor(mem: &mut impl Mem, b: [f32; 4]) {
        SCISSOR_ON.set(mem, 1);
        SCISSOR_XYUV.set(mem, 1);
        SCISSOR_BOX.set(mem, b.map(ffx_bridge::x87::round_trip));
    }

    /// `OpenScissor(x, y, w, h)` (`0x008e8980`): as [`open_font_scissor`]
    /// with the XYUV mode off.
    pub fn open_scissor(mem: &mut impl Mem, b: [f32; 4]) {
        SCISSOR_ON.set(mem, 1);
        SCISSOR_XYUV.set(mem, 0);
        SCISSOR_BOX.set(mem, b.map(ffx_bridge::x87::round_trip));
    }

    /// `TOSetRectDirect(p, x, y, w, h)` (`0x008b8cd0`): the floats `x`, `y`,
    /// `x + w`, `y + h` (32-bit sums). Returns `y + h`.
    #[allow(clippy::cast_precision_loss)] // fild then fstp dword
    pub fn set_rect_direct(mem: &mut impl Mem, p: u32, r: [i32; 4]) -> u32 {
        let right = r[0].wrapping_add(r[2]);
        let bottom = r[1].wrapping_add(r[3]);
        mem.write(p, [r[0] as f32, r[1] as f32, right as f32, bottom as f32]);
        bottom.cast_unsigned()
    }

    /// A GS register packet from its fields (`fields[0]` lowest): each
    /// field sign-extended to 64 bits and OR-ed in unmasked after the value
    /// so far is shifted by `shifts[k]` (the width of field `k`), as the
    /// `TOAdp*` register writers compile; then the register address.
    /// Returns `p + 0x10`.
    pub fn adp_reg(mem: &mut impl Mem, p: u32, fields: &[i32], shifts: &[u32], reg: u64) -> u32 {
        let last = fields.len() - 1;
        let mut v = i64::from(fields[last]).cast_unsigned();
        for k in (0..last).rev() {
            v = (v << shifts[k]) | i64::from(fields[k]).cast_unsigned();
        }
        mem.write(p, v);
        mem.write(p.wrapping_add(8), reg);
        p.wrapping_add(0x10)
    }

    /// The register writers of `0x008ef670..0x008efc60` by address: their
    /// field shifts and GS register.
    pub const ADP_REGS: [(u32, &[u32], u64); 11] = [
        (0x008e_f670, &[2, 2, 2, 0x1a], 0x42), // TOAdpALPHA: A, B, C, D, FIX
        (0x008e_f6e0, &[2, 2, 10, 10, 10], 0x08), // TOAdpCLAMP: WMS, WMT, MINU, MAXU, MINV, MAXV
        (0x008e_f760, &[], 0x46),              // TOAdpCOLCLAMP
        (0x008e_f7d0, &[], 0x4a),              // TOAdpFBA
        (0x008e_f880, &[16, 8, 8], 0x4c),      // TOAdpFRAME: FBP, FBW, PSM, FBMSK
        (0x008e_f910, &[16, 16, 16], 0x40),    // TOAdpSCISSOR
        (0x008e_fad0, &[2, 3, 1, 3, 10, 13], 0x14), // TOAdpTEX1: LCM, MXL, MMAG, MMIN, MTBA, L, K
        (0x008e_fc60, &[32], 0x18),            // TOAdpXYOFFSET: OFX, OFY
        // TOAdpTEX0: TBP0, TBW, PSM, TW, TH, TCC, TFX, CBP, CPSM, CSM, CSA, CLD
        (0x008e_fa00, &[14, 6, 6, 4, 4, 1, 2, 14, 4, 1, 5], 0x06),
        (0x008e_f830, &[8, 8], 0x3d),  // TOAdpFOGCOL: FCR, FCG, FCB
        (0x008e_fd40, &[24, 8], 0x4e), // TOAdpZBUF: ZBP, PSM, ZMSK
    ];

    /// A register writer of [`ADP_REGS`] by its address.
    pub fn adp_reg_at(mem: &mut impl Mem, function: u32, p: u32, fields: &[i32]) -> u32 {
        let (_, shifts, reg) = ADP_REGS
            .iter()
            .find(|e| e.0 == function)
            .copied()
            .expect("a TOAdp register writer");
        adp_reg(mem, p, fields, shifts, reg)
    }

    /// `TOAdpFOG(p, fog)` (`0x008ef800`): the fog value in the top byte;
    /// register 0x0a. Returns `p + 0x10`.
    pub fn adp_fog(mem: &mut impl Mem, p: u32, fog: u32) -> u32 {
        mem.write(p, [0, fog << 24, 0x0a, 0]);
        p.wrapping_add(0x10)
    }

    /// `TOAdpTEXFLUSH(p)` (`0x008efb60`): register 0x3f with no data.
    pub fn adp_texflush(mem: &mut impl Mem, p: u32) -> u32 {
        mem.write(p, [0u32, 0, 0x3f, 0]);
        p.wrapping_add(0x10)
    }

    /// `0x008efbc0(p, x, fx, y, fy)`: `x * 16 + fx` and `y * 16 + fy` (12.4
    /// fixed point) into the first two words. Returns `p + 0x10`.
    pub fn adp_xy_frac(mem: &mut impl Mem, p: u32, w: [u32; 4]) -> u32 {
        mem.write(p, (w[0] << 4).wrapping_add(w[1]));
        mem.write(p.wrapping_add(4), (w[2] << 4).wrapping_add(w[3]));
        p.wrapping_add(0x10)
    }

    /// `0x008efc30(p, a, b)`: `a`, `b`, register 0x10. Returns `p + 0x10`.
    pub fn adp_words2_reg10(mem: &mut impl Mem, p: u32, a: u32, b: u32) -> u32 {
        mem.write(p, [a, b, 0x10, 0]);
        p.wrapping_add(0x10)
    }

    /// `0x008efcd0(p, a, b, c)`: three words and a zero. Returns `p + 0x10`.
    pub fn adp_words3(mem: &mut impl Mem, p: u32, a: u32, b: u32, c: u32) -> u32 {
        mem.write(p, [a, b, c, 0]);
        p.wrapping_add(0x10)
    }

    /// `0x008efc90(p, x, y, z)` and `0x008efd00(p, x, y, z, w)`: `(x +
    /// 0x700) * 16`, `(y + 0x730) * 16`, `z * 16` and `w * 16` (0 for the
    /// three-argument form). Returns `p + 0x10`.
    #[allow(clippy::many_single_char_names)] // the coordinates
    pub fn adp_xyz_offset(
        mem: &mut impl Mem,
        p: u32,
        x: u32,
        y: u32,
        z: u32,
        w: Option<u32>,
    ) -> u32 {
        let words = [
            x.wrapping_add(0x700) << 4,
            y.wrapping_add(0x730) << 4,
            z << 4,
            w.map_or(0, |w| w << 4),
        ];
        mem.write(p, words);
        p.wrapping_add(0x10)
    }

    /// The packet cursor `0x008ff020` writes at (`0x018716cc`).
    pub const PACKET_CURSOR: Global<u32> = Global::new(0x0187_16cc);

    /// `0x008ff020(x, y)`: at the packet cursor, a GIF tag for one A+D
    /// register (`0x8001`, `0x10000000`, `0xe`) and `XYOFFSET_1` (0x18) with
    /// `x` low and `y` high (OR-ed with `x`'s sign); the cursor moves 0x20.
    /// Returns `x`.
    pub fn xyoffset_packet(mem: &mut impl Mem, x: u32, y: u32) -> u32 {
        let p = PACKET_CURSOR.get(mem);
        mem.write(p, [0x8001u32, 0x1000_0000, 0xe, 0]);
        let sign = if x.cast_signed() < 0 { u32::MAX } else { 0 };
        mem.write(p.wrapping_add(0x14), y | sign);
        mem.write(p.wrapping_add(0x10), x);
        mem.write(p.wrapping_add(0x18), [0x18u32, 0]);
        PACKET_CURSOR.set(mem, p.wrapping_add(0x20));
        x
    }

    /// One of the `TOAdp*` writers by address over `p` with its arguments
    /// as words; the advanced packet pointer.
    pub fn adp_call(mem: &mut impl Mem, function: u32, p: u32, a: &[u32]) -> u32 {
        let signed: Vec<i32> = a.iter().map(|&w| w.cast_signed()).collect();
        match function {
            0x008e_f8e0 => adp_words4(mem, p, [a[0], a[1], a[2], a[3]]),
            0x008e_fbf0 => adp_xyz2_offset(mem, p, a[0], a[1]),
            0x008e_fb90 => adp_xy2(mem, p, a[0], a[1]),
            0x008e_f970 => adp_test(mem, p, signed.try_into().expect("eight fields")),
            0x008e_fb60 => adp_texflush(mem, p),
            0x008e_fc90 => adp_xyz_offset(mem, p, a[0], a[1], a[2], None),
            0x008e_fd00 => adp_xyz_offset(mem, p, a[0], a[1], a[2], Some(a[3])),
            0x008e_f790 => adp_gs_direct_rgba(mem, p, a[0]),
            0x008e_f800 => adp_fog(mem, p, a[0]),
            0x008e_fc30 => adp_words2_reg10(mem, p, a[0], a[1]),
            0x008e_fcd0 => adp_words3(mem, p, a[0], a[1], a[2]),
            0x008e_fbc0 => adp_xy_frac(mem, p, [a[0], a[1], a[2], a[3]]),
            f => adp_reg_at(mem, f, p, &signed),
        }
    }

    /// The `TOAdp*` writers' cursor forms (`0x008faa20..0x008fade0`, copies
    /// at `0x009023b0..0x00902620`), (address, writer, argument count): the
    /// writer called on [`PACKET_CURSOR`], which takes what it returns.
    pub const ADP_AT_CURSOR: [(u32, u32, usize); 39] = [
        (0x008f_aa20, 0x008e_f670, 5),
        (0x008f_aa50, 0x008e_f6e0, 6),
        (0x008f_aa80, 0x008e_f760, 1),
        (0x008f_aaa0, 0x008e_f790, 1),
        (0x008f_aac0, 0x008e_f7d0, 1),
        (0x008f_aae0, 0x008e_f800, 1),
        (0x008f_ab00, 0x008e_f830, 3),
        (0x008f_ab30, 0x008e_f880, 4),
        (0x008f_ab60, 0x008e_f790, 1),
        (0x008f_ab80, 0x008e_f8e0, 4),
        (0x008f_abb0, 0x008e_f910, 4),
        (0x008f_abe0, 0x008e_f970, 8),
        (0x008f_ac10, 0x008e_fa00, 12),
        (0x008f_ac50, 0x008e_fad0, 7),
        (0x008f_ac80, 0x008e_fb60, 0),
        (0x008f_aca0, 0x008e_fb90, 2),
        (0x008f_acc0, 0x008e_fbc0, 4),
        (0x008f_acf0, 0x008e_fbf0, 2),
        (0x008f_ad10, 0x008e_fc30, 2),
        (0x008f_ad30, 0x008e_fc60, 2),
        (0x008f_ad50, 0x008e_fc90, 3),
        (0x008f_ad80, 0x008e_fcd0, 3),
        (0x008f_adb0, 0x008e_fd00, 4),
        (0x008f_ade0, 0x008e_fd40, 3),
        (0x0090_23b0, 0x008e_f670, 5),
        (0x0090_23e0, 0x008e_f6e0, 6),
        (0x0090_2410, 0x008e_f760, 1),
        (0x0090_2430, 0x008e_f790, 1),
        (0x0090_2450, 0x008e_f7d0, 1),
        (0x0090_2470, 0x008e_f880, 4),
        (0x0090_24a0, 0x008e_f8e0, 4),
        (0x0090_24d0, 0x008e_f910, 4),
        (0x0090_2500, 0x008e_f970, 8),
        (0x0090_2530, 0x008e_fa00, 12),
        (0x0090_2570, 0x008e_fad0, 7),
        (0x0090_25c0, 0x008e_fb90, 2),
        (0x0090_25e0, 0x008e_fbf0, 2),
        (0x0090_2600, 0x008e_fc60, 2),
        (0x0090_2620, 0x008e_fc90, 3),
    ];

    /// A cursor form of [`ADP_AT_CURSOR`]: `writer(cursor, args...)` stored
    /// back into the cursor and returned.
    pub fn adp_at_cursor(mem: &mut impl Mem, writer: u32, args: &[u32]) -> u32 {
        let p = PACKET_CURSOR.get(mem);
        let next = adp_call(mem, writer, p, args);
        PACKET_CURSOR.set(mem, next);
        next
    }
}
