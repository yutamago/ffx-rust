//! M5 `main`, round 3: the `Gu_` debug GUI's packet builders that are
//! still the game's (the textured sprite, the control frame) and the
//! debug `printf` behind `Gu_Printf`, `SgPrint` and the line printer.
//!
//! Packets: `g_GuPacket` (`0x013036c4`) holds 16-byte GIF quadwords,
//! `g_GuPacketCount` (`0x013036c0`) counts them; `Gu_SetPrim` and friends
//! append at `g_GuPacketData` (`0x013036c8`) (the same buffer). Screen
//! positions are 12.4 fixed point, centred in 4096 by the screen size
//! (`0x023cbbe0` height, `0x023cbbe4` width), y scaled by `g_GuHScale`
//! (`0x00c4df54`, 8.8).

/// The originals over game memory.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines
)]
pub mod bridge {
    use ffx_bridge::{Global, Mem};
    use ffx_graphic::host::{Conv, Host};

    pub const H_SCALE: Global<i32> = Global::new(0x00c4_df54);
    pub const PACKET_COUNT: Global<u32> = Global::new(0x0130_36c0);
    pub const PACKET: Global<u32> = Global::new(0x0130_36c4);
    pub const PACKET_DATA: Global<u32> = Global::new(0x0130_36c8);
    /// The drawing origin (two `i32`, `Gu_PacketBeginTag`).
    pub const PACKET_ORIGIN: Global<[i32; 2]> = Global::new(0x0130_36d4);
    /// The text size in 1/256 (`Gu_PrintSetSize`).
    pub const PRINT_SIZE: Global<i32> = Global::new(0x0130_36dc);
    /// The GUI's fade (`Gui_WindowSetGlobalTrans`).
    pub const FADE: Global<i32> = Global::new(0x0130_3730);
    pub const SCREEN_H: Global<i32> = Global::new(0x023c_bbe0);
    pub const SCREEN_W: Global<i32> = Global::new(0x023c_bbe4);
    /// `Gu_WindowSetHokanEnable`: the frames ease towards their targets.
    pub const HOKAN: Global<u32> = Global::new(0x00c4_df60);
    /// The frame's two face colours (`b, g, r` bytes at `+0`..`+2` and
    /// `+4`..`+6`).
    pub const FRAME_COLOURS: u32 = 0x00c4_df58;
    /// `Gu_SetTest(ate, atst, aref, afail, date, datm, zte, ztst)`.
    pub const GU_SET_TEST: u32 = 0x0084_85d0;

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn quad_at(p: u32, i: u32) -> u32 {
        at(p, i.wrapping_mul(16))
    }

    /// `(0x1000 - size) / 2`, the half border that centres the screen in
    /// 4096 (`cdq; sub; sar`: rounded towards zero).
    fn half(size: i32) -> i32 {
        0x1000i32.wrapping_sub(size) / 2
    }

    /// `FUN_00846840(x, y, w, h, u, v, du, dv, skew)` (cdecl): six
    /// `UV`/`XYZ2` quadword pairs, two triangles of a `w` x `h` sprite at
    /// (`x`, `y`) textured from (`u`, `v`) to (`u + du`, `v + dv`), all 12.4
    /// fixed point (z 0). The top edge is moved right by `skew`; the height
    /// is scaled by the text size (`g_GuPrintSize`, 1/256) and both y's by
    /// `g_GuHScale`. Twelve quadwords; returns the last `y` (in `eax`).
    #[allow(clippy::too_many_arguments)]
    pub fn sprite(
        h: &mut impl Host,
        x: u32,
        y: u32,
        w: u32,
        hgt: u32,
        u: u32,
        v: u32,
        du: u32,
        dv: u32,
        skew: u32,
    ) -> u32 {
        let hs = H_SCALE.get(h);
        let top = hs.wrapping_mul(y as i32) >> 8;
        let tall = (hs.wrapping_mul(hgt as i32) >> 8).wrapping_mul(PRINT_SIZE.get(h)) >> 8;
        let (u0, v0) = (u << 4, v << 4);
        let (u1, v1) = (u0.wrapping_add(du << 4), v0.wrapping_add(dv << 4));
        let x = x as i32;
        let (w, skew) = (w as i32, skew as i32);
        let n = PACKET_COUNT.get(h);
        let bottom = top.wrapping_add(tall);
        // (u, v) and the corner (x offset, y) of each of the six vertices.
        let verts = [
            (u0, v0, x.wrapping_add(skew), top),
            (u1, v0, x.wrapping_add(w).wrapping_add(skew), top),
            (u0, v1, x, bottom),
            (u1, v0, x.wrapping_add(w).wrapping_add(skew), top),
            (u1, v1, x.wrapping_add(w), bottom),
            (u0, v1, x, bottom),
        ];
        let mut last = 0u32;
        for (k, (tu, tv, sx, sy)) in verts.into_iter().enumerate() {
            let k = k as u32 * 2;
            let p = PACKET.get(h);
            h.write(quad_at(p, n.wrapping_add(k)), [tu, tv]);
            let px = (half(SCREEN_W.get(h)).wrapping_add(sx) << 4) as u32;
            let py = (half(SCREEN_H.get(h)).wrapping_add(sy) << 4) as u32;
            h.write(quad_at(p, n.wrapping_add(k + 1)), [px, py, 0, 0]);
            last = py;
        }
        PACKET_COUNT.set(h, n.wrapping_add(12));
        last
    }

    fn i16_at(mem: &impl Mem, p: u32, o: u32) -> i32 {
        i32::from(mem.read::<i16>(at(p, o)))
    }

    /// One step of the frame's easing: `v` moves towards `target` by
    /// `(target - v) / min(|target - v|, n)`.
    fn ease(v: i32, target: i32, n: i32) -> i32 {
        let d = target.wrapping_sub(v);
        if d == 0 {
            return v;
        }
        let a = d.wrapping_abs();
        let div = if a < n { a } else { n };
        if div == 0 {
            return v;
        }
        v.wrapping_add(d.wrapping_div(div))
    }

    /// A 64-bit quadword half (`low`, `high`) of an `RGBAQ` register value.
    fn rgbaq(mem: &mut impl Mem, q: u32, v: u64) {
        mem.write(q, [v as u32, (v >> 32) as u32, 1, 0]);
    }

    /// An `XYZ2` quadword: the screen position, then 0, 4, 0.
    fn xyz(mem: &mut impl Mem, q: u32, x: i32, y: i32) {
        let px = (half(SCREEN_W.get(mem)).wrapping_add(x) << 4) as u16;
        let py = (half(SCREEN_H.get(mem)).wrapping_add(y) << 4) as u16;
        mem.write(q, [px, py]);
        mem.write(at(q, 4), [0u32, 4, 0]);
    }

    /// The frame's face colour `k` (0 or 1): `0x8000_ffbb_ggrr`-style
    /// `RGBAQ` from the bytes at [`FRAME_COLOURS`] `+ 4k`.
    fn face(mem: &impl Mem, k: u32) -> u64 {
        let c = at(mem.rebase(FRAME_COLOURS), 4 * k);
        let b = |i: u32| u64::from(mem.read::<u8>(at(c, i)));
        ((0x8000_ff00 | b(2)) << 8 | b(1)) << 8 | b(0)
    }

    /// `FUN_00846b10(control, dx, dy)` (cdecl): a control's raised frame.
    /// With `Gu_WindowSetHokanEnable` on, the rectangle `+0x08..+0x0e`
    /// (`i16` x, y, w, h) first eases towards `+0x10..+0x16` in `+0x24`
    /// steps (2 when 0) and the colour bytes `+0x1c..+0x1e` move a third
    /// of the way towards `+0x18..+0x1a`. Then a GIF tag (the drawing
    /// origin cleared), `Gu_SetTest(1, 1, 0, ...)`, an alpha quadword, and
    /// three strips of triangles over the rectangle moved by (`dx`, `dy`):
    /// the face (fade-tinted, `0x808080` with flag bit 0 of `+0x20`, else
    /// `0xe0e0e0`), the face colour from `+0x1c..` with the fade, and the
    /// two edges in the frame's two colours (swapped by the flag). Patches
    /// the tag's count; returns the tag's address.
    pub fn frame(h: &mut impl Host, c: u32, dx: u32, dy: u32) -> u32 {
        if HOKAN.get(h) != 0 {
            let s = i16_at(h, c, 0x24);
            let n = if s == 0 { 2 } else { s };
            for o in [0x8, 0xa, 0xc] {
                let v = ease(i16_at(h, c, o), i16_at(h, c, o + 8), n);
                h.write(at(c, o), v as u16);
            }
            let hgt = ease(i16_at(h, c, 0xe), i16_at(h, c, 0x16), n);
            for (cur, target) in [(0x1d, 0x19), (0x1e, 0x1a), (0x1c, 0x18)] {
                let v = h.read::<u8>(at(c, cur));
                let t = h.read::<u8>(at(c, target));
                let step = (i32::from(t) - i32::from(v) + 2) / 3;
                h.write(at(c, cur), v.wrapping_add(step as u8));
            }
            h.write(at(c, 0xe), hgt as u16);
        }
        let x0 = i16_at(h, c, 0x8).wrapping_add(dx as i32);
        let y0 = i16_at(h, c, 0xa).wrapping_add(dy as i32);
        let x1 = i16_at(h, c, 0xc).wrapping_add(x0);
        let y1 = i16_at(h, c, 0xe).wrapping_add(y0);
        let start = PACKET_COUNT.get(h);
        let tag = quad_at(PACKET.get(h), start);
        h.write(tag, [0x3039u32, 0x1025_c000, 0xe, 0]);
        PACKET_ORIGIN.set(h, [0, 0]);
        PACKET_COUNT.set(h, start.wrapping_add(1));
        h.call_sym(GU_SET_TEST, Conv::Cdecl, &[1, 1, 0, 0, 0, 0, 0, 0]);
        let b = PACKET_COUNT.get(h);
        let d = PACKET_DATA.get(h);
        h.write(quad_at(d, b), [0x44u32, 0, 0x42, 0]);
        let fade = i64::from(FADE.get(h));
        let lit = h.read::<u8>(at(c, 0x20)) & 1 != 0;
        let face_tint: u64 = if lit { 0x80_8080 } else { 0xe0_e0e0 };
        rgbaq(
            h,
            quad_at(d, b + 1),
            (fade << 24) as u64 | face_tint | 0x8000 << 32,
        );
        let hs = H_SCALE.get(h);
        let sy0 = hs.wrapping_mul(y0) >> 8;
        xyz(h, quad_at(PACKET_DATA.get(h), b + 2), x0, sy0);
        let fade2 = i64::from(FADE.get(h) | 0x80_0000);
        let col = |i: u32| i64::from(h.read::<u8>(at(c, i)));
        let face2 = ((fade2 << 8 | col(0x1e)) << 8 | col(0x1d)) << 8 | col(0x1c);
        let d = PACKET_DATA.get(h);
        rgbaq(h, quad_at(d, b + 3), face2 as u64);
        let d = PACKET_DATA.get(h);
        xyz(h, quad_at(d, b + 4), x1, sy0);
        let sy1 = hs.wrapping_mul(y1) >> 8;
        xyz(h, quad_at(d, b + 5), x0, sy1);
        xyz(h, quad_at(d, b + 6), x1, sy0);
        xyz(h, quad_at(d, b + 7), x0, sy1);
        let col = |i: u32| i64::from(h.read::<u8>(at(c, i)));
        let face3 = (fade << 16 | col(0x1d) | 0x80_0000_c000) << 8 | col(0x1c);
        rgbaq(h, quad_at(d, b + 8), face3 as u64);
        xyz(h, quad_at(d, b + 9), x1, sy1);
        h.write(quad_at(d, b + 10), [1u32, 0, 0, 0]);
        let (first, second) = if lit { (1, 0) } else { (0, 1) };
        let edge = face(h, first);
        rgbaq(h, quad_at(d, b + 11), edge);
        let d = PACKET_DATA.get(h);
        xyz(h, quad_at(d, b + 12), x0, sy1);
        xyz(h, quad_at(d, b + 13), x0, sy0);
        xyz(h, quad_at(d, b + 14), x0, sy0);
        xyz(h, quad_at(d, b + 15), x1, sy0);
        let edge = face(h, second);
        rgbaq(h, quad_at(d, b + 16), edge);
        let d = PACKET_DATA.get(h);
        xyz(h, quad_at(d, b + 17), x1, sy0);
        xyz(h, quad_at(d, b + 18), x1, sy1);
        xyz(h, quad_at(d, b + 19), x1, sy1);
        let end = b.wrapping_add(21);
        PACKET_COUNT.set(h, end);
        xyz(h, quad_at(d, b + 20), x0, sy1);
        let used = end.wrapping_sub(start).wrapping_sub(1);
        let t: u32 = h.read(tag);
        h.write(tag, t ^ ((used ^ t) & 0x7fff));
        tag
    }

    /// The printer `sg_vfprintf` calls for each character
    /// (`SetPrintfFunc`), and the defaults a `%` conversion starts from:
    /// flags, the sign character, the width and the precision.
    pub const PRINTF_FUNC: Global<u32> = Global::new(0x00c4_dfb0);
    pub const DEFAULT_FLAGS: Global<u32> = Global::new(0x0132_5150);
    pub const DEFAULT_SIGN: Global<u8> = Global::new(0x0132_5154);
    pub const DEFAULT_WIDTH: Global<i32> = Global::new(0x0132_5158);
    pub const DEFAULT_PRECISION: Global<i32> = Global::new(0x0132_515c);
    /// `memchr`'s import slot (cdecl).
    pub const MEMCHR: u32 = 0x00b0_c390;
    /// The digit strings of `%X`/`%p` and `%x`.
    pub const HEX_UPPER: u32 = 0x00b5_b4cc;
    pub const HEX_LOWER: u32 = 0x00b5_b4e0;

    /// Flag bits of a conversion.
    const LEFT: u32 = 1;
    const PLUS: u32 = 2;
    const ZERO: u32 = 4;
    const PRECISION: u32 = 8;

    fn put(h: &mut impl Host, c: i8) {
        let f = PRINTF_FUNC.get(h);
        h.call(f, Conv::Cdecl, &[i32::from(c) as u32]);
    }

    /// The next variadic word (`args` walks one word at a time).
    fn next_arg(h: &impl Mem, arg: &mut u32) -> u32 {
        *arg = at(*arg, 4);
        h.read(*arg)
    }

    /// The minimum digit count of an integer conversion: the precision
    /// when one was given, else the width with `0` (less one for a sign),
    /// at least 1.
    fn min_digits(flags: u32, width: i32, prec: i32, sign: u8) -> i32 {
        if flags & PRECISION != 0 {
            return prec;
        }
        let mut n = prec;
        if flags & ZERO != 0 {
            n = width;
            if sign != 0 {
                n = width.wrapping_sub(1);
            }
        }
        if n < 1 { 1 } else { n }
    }

    /// The digits of `v` (most significant first, none for 0): decimal or
    /// binary by division, hex through the digit string `table`;
    /// zero-padded to `min`, then the sign in front.
    fn digits(h: &impl Mem, mut v: u32, radix: Radix, min: i32, sign: u8) -> Vec<u8> {
        let mut out = Vec::new();
        while v != 0 {
            match radix {
                Radix::Div(base) => {
                    out.push((v % base) as u8 + b'0');
                    v /= base;
                }
                Radix::Hex(table) => {
                    out.push(h.read::<u8>(at(table, v & 0xf)));
                    v >>= 4;
                }
            }
        }
        while (out.len() as i32) < min {
            out.push(b'0');
        }
        if sign != 0 {
            out.push(sign);
        }
        out.reverse();
        out
    }

    #[derive(Clone, Copy)]
    enum Radix {
        Div(u32),
        Hex(u32),
    }

    /// Reads the next format character.
    fn step(h: &impl Mem, p: &mut u32) -> i8 {
        *p = at(*p, 1);
        h.read(*p)
    }

    /// `sg_vfprintf(unused, fmt, args)` (`0x008511d0`, cdecl): the debug
    /// `printf`: each character goes to the printer at `0x00c4dfb0` (a
    /// sign-extended `char`, cdecl). `%` takes flags `-`, `+`, ` `, `0`,
    /// a width (`*` or digits added to the default width: `w * 10 + d`),
    /// a precision (`.` with `*` or digits added to the default), and the
    /// conversions `b` (binary), `c`, `d`/`i`, `n`, `p` (`%.8X`), `s`,
    /// `u`, `x`, `X` and `%%`; anything else ends the output. Returns the
    /// number of characters printed.
    ///
    /// The original builds a number's digits backwards from its frame's
    /// top (`ebp - 5`); a precision past its 0x100-byte buffer overwrites
    /// its own frame. The port keeps the digits in a vector.
    pub fn vfprintf(h: &mut impl Host, fmt: u32, args: u32) -> u32 {
        let mut p = fmt;
        let mut c = h.read::<i8>(p);
        let mut count = 0u32;
        let mut arg = args.wrapping_sub(4);
        while c != 0 {
            if c != b'%' as i8 {
                put(h, c);
                count = count.wrapping_add(1);
                c = step(h, &mut p);
                continue;
            }
            let mut flags = DEFAULT_FLAGS.get(h);
            let mut sign = DEFAULT_SIGN.get(h);
            let mut width = DEFAULT_WIDTH.get(h);
            let mut prec = DEFAULT_PRECISION.get(h);
            loop {
                c = step(h, &mut p);
                match c as u8 {
                    b'-' => flags |= LEFT,
                    b'+' => flags |= PLUS,
                    b' ' => sign = b' ',
                    b'0' => flags |= ZERO,
                    _ => break,
                }
            }
            if c == b'*' as i8 {
                width = next_arg(h, &mut arg) as i32;
                if width < 0 {
                    flags |= LEFT;
                    width = width.wrapping_neg();
                }
                c = step(h, &mut p);
            } else {
                while (b'0' as i8..=b'9' as i8).contains(&c) {
                    width = width.wrapping_mul(10).wrapping_add(i32::from(c) - 0x30);
                    c = step(h, &mut p);
                }
            }
            if c == b'.' as i8 {
                c = step(h, &mut p);
                if c == b'*' as i8 {
                    prec = next_arg(h, &mut arg) as i32;
                    c = step(h, &mut p);
                } else {
                    while (b'0' as i8..=b'9' as i8).contains(&c) {
                        prec = prec.wrapping_mul(10).wrapping_add(i32::from(c) - 0x30);
                        c = step(h, &mut p);
                    }
                }
                if prec >= 0 {
                    flags |= PRECISION;
                }
            }
            if flags & LEFT != 0 {
                flags &= !ZERO;
            }
            // What to print: built here, or a string in game memory.
            let mut text: Vec<u8> = Vec::new();
            let mut string: Option<u32> = None;
            match c as u8 {
                b'b' | b'u' => {
                    let base = if c == b'b' as i8 { 2 } else { 10 };
                    let v = next_arg(h, &mut arg);
                    let min = min_digits(flags, width, prec, 0);
                    text = digits(h, v, Radix::Div(base), min, 0);
                }
                b'd' | b'i' => {
                    let v = next_arg(h, &mut arg);
                    let (v, s) = if (v as i32) < 0 {
                        (v.wrapping_neg(), b'-')
                    } else if flags & PLUS != 0 {
                        (v, b'+')
                    } else {
                        (v, sign)
                    };
                    let min = min_digits(flags, width, prec, s);
                    text = digits(h, v, Radix::Div(10), min, s);
                }
                b'c' => text.push(next_arg(h, &mut arg) as u8),
                b'n' => {
                    let to = next_arg(h, &mut arg);
                    h.write(to, count);
                    c = step(h, &mut p);
                    continue;
                }
                b'p' | b'X' | b'x' => {
                    let table = if c == b'x' as i8 {
                        HEX_LOWER
                    } else {
                        HEX_UPPER
                    };
                    if c == b'p' as i8 {
                        flags |= PRECISION;
                        prec = 8;
                    }
                    let v = next_arg(h, &mut arg);
                    let min = min_digits(flags, width, prec, 0);
                    let table = h.rebase(table);
                    text = digits(h, v, Radix::Hex(table), min, 0);
                }
                b's' => string = Some(next_arg(h, &mut arg)),
                b'%' => {
                    put(h, c);
                    count = count.wrapping_add(1);
                    c = step(h, &mut p);
                    continue;
                }
                _ => return count,
            }
            let len = match string {
                None => text.len() as i32,
                Some(s) if flags & PRECISION == 0 => {
                    let mut n = 0u32;
                    while h.read::<u8>(at(s, n)) != 0 {
                        n = n.wrapping_add(1);
                    }
                    n as i32
                }
                Some(s) => {
                    let f: u32 = h.read(h.rebase(MEMCHR));
                    let z = h.call(f, Conv::Cdecl, &[s, 0, prec as u32]);
                    if z == 0 {
                        prec
                    } else {
                        z.wrapping_sub(s) as i32
                    }
                }
            };
            if len < width && flags & LEFT == 0 {
                while width > len {
                    put(h, b' ' as i8);
                    width -= 1;
                    count = count.wrapping_add(1);
                }
            }
            for k in 0..len.max(0) {
                let ch = match string {
                    Some(s) => h.read::<i8>(at(s, k as u32)),
                    None => text[k as usize] as i8,
                };
                put(h, ch);
            }
            count = count.wrapping_add(len as u32);
            if len < width {
                count = count.wrapping_add((width - len) as u32);
                for _ in len..width {
                    put(h, b' ' as i8);
                }
            }
            c = step(h, &mut p);
        }
        count
    }
}
