//! M5 stage 1, round 8 (list `left8`, from `vfx.list`): two opus sprite
//! packets (`opus_map_del_draw`, `opus_encount5_draw`).
//!
//! Callees stay calls to the originals (cdecl). x87 code is written as
//! the formulas at 53 bits (`ffx_bridge::x87`), each value rounded to
//! `f32` where the original stores it.

/// The originals over game memory.
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `sceGsSyncPath(mode, timeout)`, `FlushCache(op)`,
    /// `sceDmaSend(channel, packet)`; the GIF channel pointer they get;
    /// `graphicUpdateRadialLineLogic(on, colour, f, g)`.
    pub const SYNC_PATH: u32 = 0x0062_f880;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const DMA_SEND: u32 = 0x0062_f710;
    pub const DMA_GIF: u32 = 0x0233_28e0;
    pub const RADIAL_LINE: u32 = 0x0064_49d0;

    /// The sprites each packet holds, and the bytes of one input record
    /// (del draw; the encounter's are as long as a sprite) and of one
    /// output sprite.
    pub const SPRITES: u32 = 0x740;
    const DEL_IN: u32 = 0x58;
    const SPRITE: u32 = 0x60;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn f(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    fn sbyte(h: &impl Host, at: u32) -> u32 {
        i32::from(h.read::<i8>(at)).cast_unsigned()
    }

    /// The packet's header (the tag and the GIF settings: alpha
    /// `obj + 0xcb`, the colour bytes `obj + 0xc2`/`+0xc3`), as both
    /// sprite packets write it.
    fn header(h: &mut impl Host, obj: u32, packet: u32) {
        let p = |o: u32| packet.wrapping_add(o);
        for (o, v) in [
            (0x10, 1u32),
            (0x14, 0x5000_0000),
            (0x18, 0xe_eeee),
            (0x1c, 0),
        ] {
            h.write(p(o), v);
        }
        let alpha = u32::from(h.read::<u8>(obj.wrapping_add(0xcb)));
        for (o, v) in [
            (0x30, alpha),
            (0x34, 0),
            (0x38, 0x3b),
            (0x3c, 0),
            (0x40, 5),
            (0x44, 0),
            (0x48, 8),
            (0x4c, 0),
            (0x20, 0x7_000d),
            (0x24, 0),
            (0x28, 0x47),
            (0x2c, 0),
        ] {
            h.write(p(o), v);
        }
        let c2 = u32::from(h.read::<u8>(obj.wrapping_add(0xc2)));
        h.write(p(0x60), c2);
        let c3 = u32::from(h.read::<u8>(obj.wrapping_add(0xc3)));
        h.write(p(0x64), c3);
        for (o, v) in [
            (0x68, 0x42u32),
            (0x50, 0x64a2_1a00),
            (0x54, 2),
            (0x58, 6),
            (0x5c, 0),
        ] {
            h.write(p(o), v);
        }
    }

    /// The packet's tag (`0x70000000 + qwords - 1`) and the sprites' GIF
    /// tag (loop count `(qwords - 7) / 6`) for the sprites up to `end`,
    /// then `sceGsSyncPath`, `FlushCache`, `sceDmaSend` on the GIF channel
    /// and `sceGsSyncPath` again: the last one's `eax`.
    fn send(h: &mut impl Host, packet: u32, end: u32) -> u32 {
        let p = |o: u32| packet.wrapping_add(o);
        let tag = (end.wrapping_sub(packet).cast_signed() >> 4)
            .cast_unsigned()
            .wrapping_add(0x6fff_ffff);
        let nloop = ((tag & 0xffff).cast_signed() - 7) / 6;
        h.write(packet, tag);
        h.write(p(0x70), nloop.cast_unsigned() | 0x8000);
        h.write(p(0x74), (nloop >> 31).cast_unsigned() | 0x602c_c000);
        h.write(p(0x78), 0x41_2412u32);
        h.write(p(0x7c), 0u32);
        cdecl(h, SYNC_PATH, &[0, 0]);
        cdecl(h, FLUSH_CACHE, &[0]);
        let gif = h.read(h.rebase(DMA_GIF));
        cdecl(h, DMA_SEND, &[gif, packet]);
        cdecl(h, SYNC_PATH, &[0, 0])
    }

    /// `opus_map_del_draw(work, obj, recs, offset, packet; slots)`
    /// (`0x00a75170`): a packet of [`SPRITES`] two-vertex sprites at
    /// `packet` ([`header`]), sent ([`send`]: its `eax`).
    ///
    /// Each record `recs + 0x20 + 0x58 k` (unless `work + 0x212`, a pause
    /// flag, is set) first advances its two progress values
    /// `+0x48 += +0x4c` and `+0x50 += +0x54`, each capped at 1; the
    /// sprite's first vertex is `+0x30..` lerped towards `+0x38..` by
    /// `+0x48` (q 1), its second `+0x38..` (q 1), the screen positions
    /// `ftol(+0 + offset.x) << 4`, `ftol(+4 + offset.y) << 4` and
    /// `ftol(+0x10) << 4`, `ftol(+0x14) << 4`, the colours `0x80` and the
    /// depths `ftol(obj + 0x48)` and `ftol(obj + 0x4c)`.
    ///
    /// The original keeps values in its argument slots (`slots`, the
    /// address of the first): `obj`'s word ends as the last `+0x50`
    /// sum (or `offset.x` when paused), `recs`'s as `offset.y` (both as
    /// `f32`), `offset`'s as the first depth, `packet`'s as 0.
    #[allow(clippy::many_single_char_names)]
    pub fn opus_map_del_draw(
        h: &mut impl Host,
        slots: u32,
        work: u32,
        obj: u32,
        recs: u32,
        offset: u32,
        packet: u32,
    ) -> u32 {
        header(h, obj, packet);
        // the offset through the obj and recs argument slots
        let ox = x87::store(f(&*h, offset));
        let slot_obj = slots.wrapping_add(4);
        h.write(slot_obj, ox);
        let oy = x87::store(f(&*h, offset.wrapping_add(4)));
        h.write(slots.wrapping_add(8), oy);
        let near = x87::ftol(&*h, f(&*h, obj.wrapping_add(0x48)));
        h.write(slots.wrapping_add(0xc), near);
        let far = x87::ftol(&*h, f(&*h, obj.wrapping_add(0x4c)));
        let (ox, oy) = (x87::load(ox), x87::load(oy));
        let mut rec = recs.wrapping_add(0x20);
        let mut out = packet.wrapping_add(0x80);
        let one = 1.0_f64;
        for _ in 0..SPRITES {
            let r = |o: u32| rec.wrapping_add(o);
            let s = |o: u32| out.wrapping_add(o);
            if h.read::<u16>(work.wrapping_add(0x212)) == 0 {
                for (val, step) in [(0x48, 0x4c), (0x50, 0x54)] {
                    let sum = x87::store(x87::add(f(&*h, r(step)), f(&*h, r(val))));
                    h.write(slot_obj, sum);
                    h.write(r(val), sum);
                    // fcomp: capped unless below 1 (or unordered)
                    if x87::load(sum) >= one {
                        h.write(r(val), x87::store(one));
                    }
                }
            }
            let t = f(&*h, r(0x48));
            let x = x87::add(
                x87::mul(x87::sub(f(&*h, r(0x38)), f(&*h, r(0x30))), t),
                f(&*h, r(0x30)),
            );
            h.write(s(0), x87::store(x));
            let t = f(&*h, r(0x48));
            let y = x87::add(
                x87::mul(x87::sub(f(&*h, r(0x3c)), f(&*h, r(0x34))), t),
                f(&*h, r(0x34)),
            );
            h.write(s(4), x87::store(y));
            h.write(s(8), x87::store(one));
            let sx = x87::ftol(&*h, x87::add(f(&*h, r(0)), ox));
            h.write(s(0x20), sx << 4);
            let sy = x87::ftol(&*h, x87::add(f(&*h, r(4)), oy));
            h.write(s(0x24), sy << 4);
            h.write(s(0x1c), near);
            h.write(s(0x28), 0x00ff_ffffu32);
            h.write(s(0x2c), 0u32);
            for o in [0x10, 0x14, 0x18] {
                h.write(s(o), 0x80u32);
            }
            let x2 = x87::store(f(&*h, r(0x38)));
            h.write(s(0x30), x2);
            let y2 = x87::store(f(&*h, r(0x3c)));
            h.write(s(0x34), y2);
            h.write(s(0x38), x87::store(one));
            let ex = x87::ftol(&*h, f(&*h, r(0x10)));
            h.write(s(0x50), ex << 4);
            let ey = x87::ftol(&*h, f(&*h, r(0x14)));
            h.write(s(0x54), ey << 4);
            h.write(s(0x58), 0x00ff_ffffu32);
            h.write(s(0x5c), 0u32);
            for o in [0x40, 0x44, 0x48] {
                h.write(s(o), 0x80u32);
            }
            h.write(s(0x4c), far);
            rec = rec.wrapping_add(DEL_IN);
            out = out.wrapping_add(SPRITE);
        }
        h.write(slots.wrapping_add(0x10), 0u32);
        send(h, packet, out)
    }

    /// `opus_encount5_draw`'s frame: the colour it hands
    /// `graphicUpdateRadialLineLogic` lies at `ebp - 0x14`.
    pub const ENCOUNT5_COLOUR: u32 = 0x14;

    /// `opus_encount5_draw(work, obj, recs, offset, packet)`
    /// (`0x007f8e80`): the encounter's radial lines, then a packet of
    /// [`SPRITES`] fading two-vertex sprites at `packet` ([`header`]),
    /// sent ([`send`]: its `eax`). `ebp` is the original's frame pointer:
    /// the colour at `ebp - 0x14` is `(c9 / 255, c9 / 255, ca / 255,
    /// cb / 128)` from `obj + 0xc9..0xcb`, handed with `ftol(obj + 0x48)`
    /// and `ftol(obj + 0x4c)` over 128 (`f32`) to
    /// `graphicUpdateRadialLineLogic(work + 0x212 as i16, ..)`.
    ///
    /// Each record `recs + 0x68 + 0x60 k` (`P`, its fields relative to
    /// it), unless paused (`work + 0x212`): `P+0 += P+4` capped at 1,
    /// `P+8 += P+0xc`. Then its fade byte `P-0x19`: at or past 0.3
    /// (`f32`) `i = ftol(f32(P+8 - 0.3) / P+0xc)` gives `0x80 - 8 i`, or
    /// 0 from 16 on with `P+8` reset to 0; below 0.3 (or NaN)
    /// `i = ftol(P+8 / P+0xc)` gives `8 i`, or `0x80` from 16 on. The
    /// sprite: the first vertex `P-0x18..` lerped towards `P-0x10..` by
    /// `P+0`, the second `P-0x10..` lerped back by `P+8` (q 1 each);
    /// screen positions `ftol(P-0x48 + offset.x) << 4`,
    /// `ftol(P-0x44 + offset.y) << 4`, `ftol(P-0x38) << 4`,
    /// `ftol(P-0x34) << 4`; the colours `obj + 0xc8..0xca` (signed); the
    /// depths are the fade byte times the two `ftol`s over 128.
    #[allow(
        clippy::many_single_char_names,
        clippy::too_many_arguments,
        clippy::too_many_lines
    )]
    pub fn opus_encount5_draw(
        h: &mut impl Host,
        ebp: u32,
        work: u32,
        obj: u32,
        recs: u32,
        offset: u32,
        packet: u32,
    ) -> u32 {
        header(h, obj, packet);
        let ox = f(&*h, offset);
        let oy = f(&*h, offset.wrapping_add(4));
        let near = x87::ftol(&*h, f(&*h, obj.wrapping_add(0x48)));
        let far = x87::ftol(&*h, f(&*h, obj.wrapping_add(0x4c)));
        let [c9, ca, cb] = [0xc9, 0xca, 0xcb].map(|o| f64::from(h.read::<u8>(obj.wrapping_add(o))));
        let r = x87::store(x87::div(c9, 255.0));
        let g = x87::store(x87::div(ca, 255.0));
        let a = x87::store(x87::mul(cb, 0.007_812_5));
        let colour = ebp.wrapping_sub(ENCOUNT5_COLOUR);
        for (k, v) in [r, r, g, a].into_iter().enumerate() {
            h.write(colour.wrapping_add(4 * k as u32), v);
        }
        #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
        let scaled = |v: u32| {
            let v = x87::load((v.cast_signed() as f32).to_bits());
            x87::store(x87::mul(v, 0.007_812_5))
        };
        let (fg, ff) = (scaled(far), scaled(near));
        let on = i32::from(h.read::<i16>(work.wrapping_add(0x212))).cast_unsigned();
        cdecl(h, RADIAL_LINE, &[on, colour, ff, fg]);
        // 0.3 as the f32 constant (0x00b3eb44) and the double (0x00b48308)
        let third = x87::load(0x3e99_999a);
        let (one, zero) = (1.0_f64, 0.0_f64);
        let mut p = recs.wrapping_add(0x68);
        let mut out = packet.wrapping_add(0x80);
        for _ in 0..SPRITES {
            let q = |o: i32| p.wrapping_add_signed(o);
            let s = |o: u32| out.wrapping_add(o);
            if h.read::<u16>(work.wrapping_add(0x212)) == 0 {
                let sum = x87::store(x87::add(f(&*h, q(4)), f(&*h, q(0))));
                h.write(q(0), sum);
                // fcomp: capped unless below 1 (or unordered)
                if x87::load(sum) >= one {
                    h.write(q(0), x87::store(one));
                }
                let fade = x87::store(x87::add(f(&*h, q(0xc)), f(&*h, q(8))));
                h.write(q(8), fade);
            }
            let fade = f(&*h, q(8));
            // fcom: C0 or C3 alone (fade at or past 0.3) takes the first
            // arm; neither or both (below, or NaN) the second
            if fade >= third {
                let v = x87::load(x87::store(x87::sub(fade, third)));
                let i = x87::ftol(&*h, x87::div(v, f(&*h, q(0xc)))).cast_signed();
                if i >= 0x10 {
                    h.write(q(-0x19), 0u8);
                    h.write(q(8), x87::store(zero));
                } else {
                    #[allow(clippy::cast_possible_truncation)] // shl cl, 3
                    let b = 0x80u8.wrapping_sub((i as u8) << 3);
                    h.write(q(-0x19), b);
                }
            } else {
                let i = x87::ftol(&*h, x87::div(fade, f(&*h, q(0xc)))).cast_signed();
                #[allow(clippy::cast_possible_truncation)] // shl al, 3
                let b = if i < 0x10 { (i as u8) << 3 } else { 0x80 };
                h.write(q(-0x19), b);
            }
            let x = x87::add(
                x87::mul(x87::sub(f(&*h, q(-0x10)), f(&*h, q(-0x18))), f(&*h, q(0))),
                f(&*h, q(-0x18)),
            );
            h.write(s(0), x87::store(x));
            let y = x87::add(
                x87::mul(x87::sub(f(&*h, q(-0xc)), f(&*h, q(-0x14))), f(&*h, q(0))),
                f(&*h, q(-0x14)),
            );
            h.write(s(4), x87::store(y));
            h.write(s(8), x87::store(one));
            let sx = x87::ftol(&*h, x87::add(f(&*h, q(-0x48)), ox));
            h.write(s(0x20), sx << 4);
            let sy = x87::ftol(&*h, x87::add(oy, f(&*h, q(-0x44))));
            h.write(s(0x24), sy << 4);
            h.write(s(0x28), 0x00ff_ffffu32);
            h.write(s(0x2c), 0u32);
            for (o, c) in [(0x10, 0xc8), (0x14, 0xc9), (0x18, 0xca)] {
                let v = sbyte(&*h, obj.wrapping_add(c));
                h.write(s(o), v);
            }
            let fade_byte = i32::from(h.read::<u8>(q(-0x19)));
            h.write(
                s(0x1c),
                (fade_byte.wrapping_mul(near.cast_signed()) / 128).cast_unsigned(),
            );
            for (o, (hi, lo)) in [(0x30, (-0x10, -0x18)), (0x34, (-0xc, -0x14))] {
                let hi = f(&*h, q(hi));
                let d = x87::mul(x87::sub(hi, f(&*h, q(lo))), f(&*h, q(8)));
                h.write(s(o), x87::store(x87::sub(hi, d)));
            }
            h.write(s(0x38), x87::store(one));
            let ex = x87::ftol(&*h, f(&*h, q(-0x38)));
            h.write(s(0x50), ex << 4);
            let ey = x87::ftol(&*h, f(&*h, q(-0x34)));
            h.write(s(0x54), ey << 4);
            h.write(s(0x58), 0x00ff_ffffu32);
            h.write(s(0x5c), 0u32);
            for (o, c) in [(0x40, 0xc8), (0x44, 0xc9), (0x48, 0xca)] {
                let v = sbyte(&*h, obj.wrapping_add(c));
                h.write(s(o), v);
            }
            let fade_byte = i32::from(h.read::<u8>(q(-0x19)));
            h.write(
                s(0x4c),
                (fade_byte.wrapping_mul(far.cast_signed()) / 128).cast_unsigned(),
            );
            p = p.wrapping_add(SPRITE);
            out = out.wrapping_add(SPRITE);
        }
        send(h, packet, out)
    }

    /// The CRT's `sprintf` (import slot), `"%f"`, and
    /// `opd_num_tex_tag_00a5e7b0(p, n, pal)`.
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    pub const FORMAT_F: u32 = 0x00b4_0274;
    pub const NUM_TEX_TAG: u32 = 0x00a5_e7b0;
    /// The number font's glyph table (`u` and `v` by character, `i8`
    /// indexes) and the space, minus and point glyphs' cells in it; the
    /// glyph's width and height bytes the wide records add.
    pub const GLYPH_U: u32 = 0x00c8_75b0;
    pub const GLYPH_V: u32 = 0x00c8_76b0;
    pub const GLYPH_SPACE: u32 = 0x00c8_7728;
    pub const GLYPH_MINUS: u32 = 0x00c8_7700;
    pub const GLYPH_POINT: u32 = 0x00c8_770c;
    pub const GLYPH_W: u32 = 0x0228_615a;
    pub const GLYPH_H: u32 = 0x0228_615b;
    /// `opd_num_put_float`'s frame: the `sprintf` buffer at `ebp - 0x204`.
    pub const PUT_FLOAT_FRAME: u32 = 0x204;

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    fn g(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    /// One glyph as the padding, the sign and the point write it: its
    /// words spread over the next records (`+0`, `+0x10`, `+0x40` ..
    /// `+0xe0`), in the original's order (the next glyph writes over
    /// some of them).
    #[allow(clippy::too_many_arguments)]
    fn wide_glyph(h: &mut impl Host, at: u32, u: u32, v: u32, x: u32, y: u32, z: u32) {
        let a = |o: u32| at.wrapping_add(o);
        let (u, v) = (u << 4, v << 4);
        h.write(a(0), u);
        h.write(a(0x80), u.wrapping_add(0x80));
        h.write(a(0x10), v);
        h.write(a(0x40), x);
        h.write(a(0x50), y);
        h.write(a(0x60), z);
        h.write(a(0x90), v.wrapping_add(0x80));
        let w = u32::from(h.read::<u8>(h.rebase(GLYPH_W)));
        h.write(a(0xc0), w.wrapping_add(x));
        h.write(a(0xe0), z);
        let ht = u32::from(h.read::<u8>(h.rebase(GLYPH_H)));
        h.write(a(0xd0), ht.wrapping_add(y));
    }

    /// One digit as the integer and decimal loops write it: the record
    /// at `at` (`0x40` bytes) for the character at `c`; `link` is the
    /// word the original keeps at `ebp - 0x218`.
    fn digit(h: &mut impl Host, at: u32, c: u32, font: u32, x: u32, y: u32, link: u32) {
        let a = |o: u32| at.wrapping_add(o);
        let ch = i32::from(h.read::<i8>(c)).cast_unsigned().wrapping_mul(4);
        let u = h.read::<u32>(h.rebase(GLYPH_U).wrapping_add(ch)) << 4;
        let v = h.read::<u32>(h.rebase(GLYPH_V).wrapping_add(ch)) << 4;
        h.write(a(0x14), y);
        h.write(a(0x18), link);
        h.write(a(0x20), u.wrapping_add(0x80));
        h.write(a(0x10), x);
        h.write(a(0), u);
        h.write(a(4), v);
        h.write(a(0x24), v.wrapping_add(0x80));
        let cw = u32::from(h.read::<u8>(font.wrapping_add(0x1a)));
        h.write(a(0x30), cw.wrapping_add(x));
        let chh = u32::from(h.read::<u8>(font.wrapping_add(0x1b)));
        h.write(a(0x34), chh.wrapping_add(y));
        let z: u32 = h.read(font.wrapping_add(4));
        h.write(a(0x38), z);
    }

    /// `opd_num_put_float_00a5ea30(value, width, decimals, pr, font;
    /// slots)` (`0x00a5ea30`, `value` a double in two words) -> `pr`:
    /// `sprintf(ebp - 0x204, "%f", value)`, then the number's glyphs into
    /// the print buffer `[pr + 8] + 0x40` at the print position (`pr`'s
    /// `u16` column `+0x10` and row `+0x12` times `font`'s cell bytes
    /// `+0x1a`/`+0x1b`, less `font`'s `u16` `+0x16`/`+0x18` (even) times
    /// 8, plus `0x8000`): spaces up to `width` integer digits (at most
    /// the last `width` digits are kept), the sign or a space, the
    /// integer digits, the point and `decimals` digits (read past the
    /// string as they come). `opd_num_tex_tag(buffer, n, 2)` for the `n`
    /// glyphs; the column grows by `n` and `pr + 0xc` gets the end.
    ///
    /// The integer digits' third word is the original's `ebp - 0x218`,
    /// the end of the padding: without padding it is never written.
    /// junk read: [ebp-0x218] never written; 0 (brief 2026-10-24).
    /// The decimals count runs down in its argument slot (`slots + 0xc`).
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn put_float(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        value: [u32; 2],
        width: u32,
        decimals: u32,
        pr: u32,
        font: u32,
    ) -> u32 {
        let width = width.cast_signed();
        let buffer: u32 = h.read(pr.wrapping_add(8));
        let col = u32::from(h.read::<u16>(pr.wrapping_add(0x10)));
        let cw = u32::from(h.read::<u8>(font.wrapping_add(0x1a)));
        let mut x = col
            .wrapping_mul(cw)
            .wrapping_sub((u32::from(h.read::<u16>(font.wrapping_add(0x16))) & !1) << 3)
            .wrapping_add(0x8000);
        let row = u32::from(h.read::<u16>(pr.wrapping_add(0x12)));
        let chh = u32::from(h.read::<u8>(font.wrapping_add(0x1b)));
        let y = row
            .wrapping_mul(chh)
            .wrapping_sub((u32::from(h.read::<u16>(font.wrapping_add(0x18))) & !1) << 3)
            .wrapping_add(0x8000);
        let text = ebp.wrapping_sub(PUT_FLOAT_FRAME);
        let fmt = h.rebase(FORMAT_F);
        import(h, SPRINTF_SLOT, &[text, fmt, value[0], value[1]]);
        let neg = u32::from(h.read::<u8>(text) == b'-');
        let mut dot = neg;
        while h.read::<u8>(text.wrapping_add(dot)) != b'.' {
            dot = dot.wrapping_add(1);
        }
        let mut digits = dot.wrapping_sub(neg).cast_signed();
        let mut p = text.wrapping_add(neg);
        if i32::from(digits as u8) >= width {
            p = text
                .wrapping_add(dot)
                .wrapping_add_signed(width.wrapping_neg());
            digits = width;
        }
        let mut pad = width.wrapping_sub(digits);
        if i32::from(pad as u8) >= width {
            pad = width;
        }
        let mut count = 0_i32;
        let mut at = buffer.wrapping_add(0x40);
        // junk read: [ebp-0x218] never written; 0 (brief 2026-10-24)
        let mut link = 0_u32;
        if pad > 0 {
            count = pad;
            for _ in 0..pad {
                let z: u32 = h.read(font.wrapping_add(4));
                let (u, v) = (g(&*h, GLYPH_SPACE), g(&*h, GLYPH_SPACE + 0x100));
                wide_glyph(h, at, u, v, x, y, z);
                link = at.wrapping_add(0x40);
                at = link;
                x = x.wrapping_add(u32::from(h.read::<u8>(font.wrapping_add(0x1a))));
            }
        }
        let z: u32 = h.read(font.wrapping_add(4));
        let sign = if neg != 0 { GLYPH_MINUS } else { GLYPH_SPACE };
        let (u, v) = (g(&*h, sign), g(&*h, sign + 0x100));
        wide_glyph(h, at, u, v, x, y, z);
        at = at.wrapping_add(0x40);
        count = count.wrapping_add(1);
        x = x.wrapping_add(u32::from(h.read::<u8>(font.wrapping_add(0x1a))));
        if digits > 0 {
            count = count.wrapping_add(digits);
            for _ in 0..digits {
                digit(h, at, p, font, x, y, link);
                p = p.wrapping_add(1);
                at = at.wrapping_add(0x40);
                x = x.wrapping_add(u32::from(h.read::<u8>(font.wrapping_add(0x1a))));
            }
        }
        let z: u32 = h.read(font.wrapping_add(4));
        let (u, v) = (g(&*h, GLYPH_POINT), g(&*h, GLYPH_POINT + 0x100));
        wide_glyph(h, at, u, v, x, y, z);
        let link = at.wrapping_add(0x40);
        at = link;
        x = x.wrapping_add(u32::from(h.read::<u8>(font.wrapping_add(0x1a))));
        p = p.wrapping_add(1);
        count = count.wrapping_add(1);
        let decimals = decimals.cast_signed();
        if decimals > 0 {
            count = count.wrapping_add(decimals);
            for k in (0..decimals).rev() {
                digit(h, at, p, font, x, y, link);
                h.write(slots.wrapping_add(0xc), k.cast_unsigned());
                at = at.wrapping_add(0x40);
                x = x.wrapping_add(u32::from(h.read::<u8>(font.wrapping_add(0x1a))));
                p = p.wrapping_add(1);
            }
        }
        let count = count.cast_unsigned();
        h.call_sym(NUM_TEX_TAG, Conv::Cdecl, &[buffer, count, 2]);
        let col: u16 = h.read(pr.wrapping_add(0x10));
        #[allow(clippy::cast_possible_truncation)] // add word [eax+0x10], cx
        h.write(pr.wrapping_add(0x10), col.wrapping_add(count as u16));
        h.write(pr.wrapping_add(0xc), at);
        pr
    }

    /// `opd_num(format, ...)` and the dump's two line formats: the
    /// address and 16 bytes (`"%8x:%2x,..."`), 16 bytes after spaces.
    pub const OPD_NUM: u32 = 0x00a5_d3c0;
    pub const DUMP_HEAD: u32 = 0x00b7_b9e0;
    pub const DUMP_LINE: u32 = 0x00b7_ba28;

    /// `opd_num_char_256(p)` (`0x00a5d5f0`) -> the last `opd_num`'s
    /// `eax`: 256 bytes at `p` as 16 lines of 16 (each `i8` widened), every
    /// fourth line headed by its address (18 words, `Host::call_wide`).
    pub fn char_256(h: &mut impl Host, p: u32) -> u32 {
        let f = h.rebase(OPD_NUM);
        let mut eax = 0;
        for line in 0..16 {
            let at = p.wrapping_add(16 * line);
            let mut args = Vec::with_capacity(18);
            if line % 4 == 0 {
                args.extend([h.rebase(DUMP_HEAD), at]);
            } else {
                args.push(h.rebase(DUMP_LINE));
            }
            for k in 0..16 {
                let b = h.read::<i8>(at.wrapping_add(k));
                args.push(i32::from(b).cast_unsigned());
            }
            // call width: 18 (the format, the address and 16 bytes)
            eax = h.call_wide(f, &args);
        }
        eax
    }
}
