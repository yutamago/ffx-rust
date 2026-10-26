//! `PBitmapFontText::createMeshGeometry` (`0x00a29770`, list
//! p6-render-c; `m4-rest`): a bitmap-font text's quads written into its
//! mesh's vertex streams.
//!
//! `thiscall(text; count)`, `ret 4` -> 0, or the first non-zero result of
//! a stream's map or unmap. `count` is the number of characters to lay
//! out (newlines included). The text: `+0x64` the mesh (three streams of
//! 0x40 bytes: positions, texture coordinates, colours; `+4` the vertex
//! count, `+0x10` the slot, `+0x14` a state byte, `+0x18` the slot
//! count), `+0x68..+0x70` the draw's first index, last vertex and index
//! count, `+0x158` the font's holder (`+0x30` the font: `+8` the line
//! height, `+0x10` the glyph count, `+0x14` the 0x30-byte glyphs, `+0x20`
//! the texture, whose `+0x1c`/`+0x20` are its width and height), `+0x160`
//! the UTF-8 string, `+0x168` the widest line, `+0x16c` the height,
//! `+0x170` the line spacing, `+0x174` whether it has colours, `+0x178`
//! and `+0x188` the top and bottom colours. A glyph: `+0` the code point,
//! `+0xc`/`+0x10` its texel position, `+0x14`/`+0x18` its size, `+0x1c`/
//! `+0x20` its offset, `+0x24`/`+0x28` the pen's advance, `+0x2c` whether
//! it is stored rotated.
//!
//! The frame: the callees get pointers into the original's frame (the
//! streams' mapped pointers, the string cursor, two glyph records); the
//! bridge keeps those at the original's `ebp` offsets. Floats as the
//! original keeps them (x87 at 53 bits, `f32` at its stores).

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// A vertex stream's map (thiscall `(stream, &pointer)`, `ret 4`) -> 0 or
/// an error.
pub const MAP: u32 = 0x0043_4c50;
/// A vertex stream's unmap (thiscall `(stream; slot, &pointer)`, `ret 8`)
/// -> 0 or an error.
pub const UNMAP: u32 = 0x0056_4cb0;
/// The same unmap with the slot step inside (thiscall `(stream,
/// &pointer)`, `ret 4`).
pub const UNMAP_STEP: u32 = 0x0043_50b0;
/// `Phyre_PUtf8_decodeNext_0045d640` (cdecl `(&cursor)`) -> the code
/// point.
pub const DECODE: u32 = 0x0045_d640;
/// A glyph record's constructor (thiscall, zeroes it).
pub const GLYPH_CTOR: u32 = 0x00a2_bab0;
/// The glyph array's lookup (thiscall `(array; &key, first)`, `ret 8`) ->
/// the index, or -1.
pub const FIND_GLYPH: u32 = 0x00a2_9320;
/// The kerning of a glyph pair (thiscall `(text; a, b)`, `ret 8`) -> a
/// float in `ST0`.
pub const KERNING: u32 = 0x00a2_9700;
/// The larger of two floats, the second on ties and NaNs (cdecl `(a,
/// b)`, floats as words) -> `ST0`.
pub const MAX: u32 = 0x0051_26e0;
/// `Phyre_PhyrePrintf` (cdecl, varargs).
pub const PRINTF: u32 = 0x0043_52f0;
/// The "skipping unrenderable character" warning.
pub const SKIP_WARNING: u32 = 0x00b7_8d38;

/// The frame's size below `ebp`.
pub const FRAME: u32 = 0xc4;
/// `ebp - 0xa4`: the positions' mapped pointer.
pub const MAP_POS: u32 = 0xa4;
/// `ebp - 0xc4`: the texture coordinates' mapped pointer.
pub const MAP_UV: u32 = 0xc4;
/// `ebp - 0xac`: the colours' mapped pointer.
pub const MAP_COL: u32 = 0xac;
/// `ebp - 0x90`: the string cursor.
pub const CURSOR: u32 = 0x90;
/// `ebp - 0x64`: the glyph record the search fills.
pub const GLYPH: u32 = 0x64;
/// `ebp - 0x34`: the next character's key.
pub const KEY: u32 = 0x34;

/// `f32` bits as `fld dword` loads them.
fn ld(bits: u32) -> f64 {
    x87::load(bits)
}

/// `fld dword`, `fstp dword`.
fn copy(bits: u32) -> u32 {
    x87::store(x87::load(bits))
}

/// `fadd dword [m]` on `a`.
fn add_m(a: f64, m: u32) -> f64 {
    x87::Reg::constant(a).add_m32(f32::from_bits(m)).value()
}

/// `fmul dword [m]` on `a`.
fn mul_m(a: f64, m: u32) -> f64 {
    x87::Reg::constant(a).mul_m32(f32::from_bits(m)).value()
}

pub mod bridge {
    //! The original over game memory.

    use super::{
        CURSOR, Conv, DECODE, FIND_GLYPH, GLYPH, GLYPH_CTOR, Host, KERNING, KEY, MAP, MAP_COL,
        MAP_POS, MAP_UV, MAX, PRINTF, SKIP_WARNING, UNMAP, UNMAP_STEP, add_m, copy, ld, mul_m, x87,
    };

    /// The frame at the original's `ebp`.
    struct Frame(u32);

    impl Frame {
        fn at(&self, off: u32) -> u32 {
            self.0.wrapping_sub(off)
        }
    }

    /// The pen and the sizes the loop carries (`f32` bits).
    struct Pen {
        x: u32,
        y: u32,
        /// The current line's tallest glyph.
        line: u32,
        /// The finished lines' height.
        total: u32,
        quads: u32,
    }

    /// The streams' write cursors.
    struct Out {
        pos: u32,
        uv: u32,
        col: u32,
    }

    /// `[stream]`'s unmap: the next slot (0 past the last), kept with the
    /// state byte cleared when the unmap succeeds.
    fn unmap(h: &mut impl Host, stream: u32, pointer: u32) -> u32 {
        let next = h.read::<u32>(stream.wrapping_add(0x10)).wrapping_add(1);
        let count = h.read::<u32>(stream.wrapping_add(0x18)) & 0x7fff_ffff;
        let slot = if next == count { 0 } else { next };
        let r = h.call_sym(UNMAP, Conv::Thiscall(stream), &[slot, pointer]);
        if r == 0 {
            h.write(stream.wrapping_add(0x10), slot);
            h.write(stream.wrapping_add(0x14), 0_u8);
        }
        r
    }

    fn decode(h: &mut impl Host, f: &Frame) -> u32 {
        h.call_sym(DECODE, Conv::Cdecl, &[f.at(CURSOR)])
    }

    /// The larger of `a` and `b` (the original's `0x005126e0`), both
    /// passed through the x87 stack.
    fn max(h: &mut impl Host, a: u32, b: u32) -> u32 {
        h.call_sym_st0(MAX, Conv::Cdecl, &[copy(a), copy(b)])
            .to_bits()
    }

    /// `(f32)(unsigned)n`, read back through the stack.
    fn size(n: u32) -> f64 {
        ld(copy(x87::store(f64::from(n))))
    }

    /// Word `off` of the glyph `g`.
    fn field(h: &impl Host, g: u32, off: u32) -> u32 {
        h.read(g.wrapping_add(off))
    }

    fn has_colours(h: &impl Host, text: u32) -> bool {
        h.read::<u8>(text + 0x174) != 0
    }

    /// Four vertices per character but the string's newlines.
    fn vertex_count(h: &impl Host, text: u32, count: u32) -> u32 {
        let mut s: u32 = h.read(text + 0x160);
        let mut lines = 0_u32;
        loop {
            let c: u8 = h.read(s);
            if c == 0 {
                break;
            }
            lines += u32::from(c == b'\n');
            s = s.wrapping_add(1);
        }
        count.wrapping_sub(lines).wrapping_mul(4)
    }

    /// Maps the positions, the texture coordinates and (with colours) the
    /// colours, and sets their vertex counts: the three streams, or the
    /// failing map's result.
    fn map_streams(
        h: &mut impl Host,
        f: &Frame,
        text: u32,
        vertices: u32,
    ) -> Result<(u32, u32, u32), u32> {
        let mesh: u32 = h.read(text + 0x64);
        let r = h.call_sym(MAP, Conv::Thiscall(mesh), &[f.at(MAP_POS)]);
        if r != 0 {
            return Err(r);
        }
        let uv_stream = mesh.wrapping_add(0x40);
        let r = h.call_sym(MAP, Conv::Thiscall(uv_stream), &[f.at(MAP_UV)]);
        if r != 0 {
            unmap(h, mesh, f.at(MAP_POS));
            return Err(r);
        }
        let mut col_stream = 0;
        if has_colours(h, text) {
            col_stream = h.read::<u32>(text + 0x64).wrapping_add(0x80);
            let r = h.call_sym(MAP, Conv::Thiscall(col_stream), &[f.at(MAP_COL)]);
            if r != 0 {
                // the other two stay mapped
                h.call_sym(UNMAP_STEP, Conv::Thiscall(col_stream), &[f.at(MAP_COL)]);
                return Err(r);
            }
            h.write(col_stream.wrapping_add(4), vertices);
        }
        h.write(mesh.wrapping_add(4), vertices);
        h.write(mesh.wrapping_add(0x44), vertices);
        Ok((mesh, uv_stream, col_stream))
    }

    /// A newline: the pen back to the left, one line height down, the
    /// finished line's height added up.
    fn new_line(h: &mut impl Host, text: u32, font: u32, pen: &mut Pen) {
        let height = x87::store(mul_m(
            ld(h.read(text + 0x170)),
            h.read(font.wrapping_add(8)),
        ));
        pen.x = 0;
        pen.y = x87::store(x87::sub(ld(pen.y), ld(height)));
        pen.total = x87::store(add_m(ld(height), pen.total));
        let so_far = add_m(ld(h.read(text + 0x16c)), pen.line);
        h.write(text + 0x16c, x87::store(so_far));
        pen.line = 0;
    }

    /// `PBitmapFontText::createMeshGeometry(text; count)` with the frame
    /// at `ebp`.
    pub fn create_mesh_geometry(h: &mut impl Host, text: u32, count: u32, ebp: u32) -> u32 {
        if count == 0 {
            return 0;
        }
        let f = Frame(ebp);
        let font: u32 = h.read(h.read::<u32>(text + 0x158).wrapping_add(0x30));
        h.write(text + 0x168, 0_u32);
        h.write(text + 0x16c, 0_u32);
        let vertices = vertex_count(h, text, count);
        let (mesh, uv_stream, col_stream) = match map_streams(h, &f, text, vertices) {
            Ok(streams) => streams,
            Err(r) => return r,
        };

        let mut out = Out {
            pos: h.read(f.at(MAP_POS)),
            uv: h.read(f.at(MAP_UV)),
            col: if has_colours(h, text) {
                h.read(f.at(MAP_COL))
            } else {
                0
            },
        };
        let texture: u32 = h.read(font.wrapping_add(0x20));
        let tex_w: u32 = h.read(texture.wrapping_add(0x1c));
        let tex_h: u32 = h.read(texture.wrapping_add(0x20));
        let string: u32 = h.read(text + 0x160);
        h.write(f.at(CURSOR), string);
        let mut code = decode(h, &f);
        let mut pen = Pen {
            x: 0,
            y: 0,
            line: 0,
            total: 0,
            quads: 0,
        };
        let mut i = 0;
        while i < count {
            if code == 0 {
                break;
            }
            h.call_sym(GLYPH_CTOR, Conv::Thiscall(f.at(GLYPH)), &[]);
            let n = h.read::<u32>(font.wrapping_add(0x10)) & 0x7fff_ffff;
            h.write(f.at(GLYPH), code);
            let mut found = None;
            if n != 0 {
                let glyphs: u32 = h.read(font.wrapping_add(0x14));
                found = (0..n).find(|&k| h.read::<u32>(glyphs.wrapping_add(0x30 * k)) == code);
            }
            if code == u32::from(b'\n') {
                new_line(h, text, font, &mut pen);
                code = decode(h, &f);
            } else if let Some(k) = found {
                code = glyph(h, &f, text, font, k, (tex_w, tex_h), &mut pen, &mut out);
            } else {
                h.call_sym(PRINTF, Conv::Cdecl, &[2, h.rebase(SKIP_WARNING), code]);
                code = decode(h, &f);
                let height = x87::store(mul_m(
                    ld(h.read(text + 0x170)),
                    h.read(font.wrapping_add(8)),
                ));
                pen.x = x87::store(add_m(x87::mul(ld(height), 0.5), pen.x));
                let widest = h.read(text + 0x168);
                let widest = max(h, widest, pen.x);
                h.write(text + 0x168, widest);
            }
            i += 1;
        }

        let quads = pen.quads;
        h.write(text + 0x70, quads.wrapping_mul(6));
        h.write(
            text + 0x6c,
            if quads == 0 {
                0
            } else {
                quads.wrapping_mul(4).wrapping_sub(1)
            },
        );
        h.write(text + 0x68, 0_u32);
        let height = add_m(add_m(ld(pen.total), pen.line), h.read(text + 0x16c));
        h.write(text + 0x16c, x87::store(height));

        for (stream, pointer) in [(mesh, MAP_POS), (uv_stream, MAP_UV)] {
            let r = unmap(h, stream, f.at(pointer));
            if r != 0 {
                return r;
            }
        }
        if has_colours(h, text) {
            let r = unmap(h, col_stream, f.at(MAP_COL));
            if r != 0 {
                return r;
            }
        }
        0
    }

    /// One glyph's quad (index `k` of the font's glyphs); returns the next
    /// code point.
    #[allow(clippy::too_many_arguments)]
    fn glyph(
        h: &mut impl Host,
        f: &Frame,
        text: u32,
        font: u32,
        k: u32,
        (tex_w, tex_h): (u32, u32),
        pen: &mut Pen,
        out: &mut Out,
    ) -> u32 {
        let g = h
            .read::<u32>(font.wrapping_add(0x14))
            .wrapping_add(k.wrapping_mul(0x30));
        let gh = copy(field(h, g, 0x18));
        let ox = copy(field(h, g, 0x1c));
        let oy = copy(field(h, g, 0x20));
        let (x, y) = (ld(pen.x), ld(pen.y));

        // positions: (left, top), (right, top), (right, bottom), (left,
        // bottom)
        let top = x87::store(x87::add(ld(oy), y));
        let left = x87::add(x, ld(ox));
        let right = x87::store(add_m(left, field(h, g, 0x14)));
        let left = x87::store(left);
        let bottom = x87::store(x87::add(x87::sub(ld(oy), ld(gh)), y));
        let pos = out.pos;
        out.pos = pos.wrapping_add(0x20);
        for (j, w) in [left, top, right, top, right, bottom, left, bottom]
            .into_iter()
            .enumerate()
        {
            h.write(pos.wrapping_add(4 * j as u32), w);
        }
        if has_colours(h, text) {
            let col = out.col;
            out.col = col.wrapping_add(0x40);
            for j in 0..16 {
                let from = text + 0x178 + 0x10 * (j / 8) + 4 * (j % 4);
                let w = copy(h.read(from));
                h.write(col.wrapping_add(4 * j), w);
            }
        }

        // texture coordinates
        let tw = size(tex_w);
        let u0 = x87::store(x87::div(ld(field(h, g, 0xc)), tw));
        let th = size(tex_h);
        let v0 = x87::store(x87::div(ld(field(h, g, 0x10)), th));
        let rotated = h.read::<u8>(g.wrapping_add(0x2c)) != 0;
        let (across, down) = if rotated { (0x18, 0x14) } else { (0x14, 0x18) };
        let u1 = add_m(ld(field(h, g, 0xc)), field(h, g, across));
        let u1 = x87::store(x87::div(u1, tw));
        let v1 = add_m(ld(field(h, g, 0x10)), field(h, g, down));
        let v1 = x87::store(x87::div(v1, th));
        let uv = if rotated {
            [u0, v0, u0, v1, u1, v1, u1, v0]
        } else {
            [u0, v0, u1, v0, u1, v1, u0, v1]
        };
        let at = out.uv;
        out.uv = at.wrapping_add(0x20);
        for (j, w) in uv.into_iter().enumerate() {
            h.write(at.wrapping_add(4 * j as u32), w);
        }

        // the pen, then the kerning with the next character
        pen.x = x87::store(x87::add(x, ld(field(h, g, 0x24))));
        pen.y = x87::store(add_m(y, field(h, g, 0x28)));
        let code = decode(h, f);
        if code != 0 {
            h.call_sym(GLYPH_CTOR, Conv::Thiscall(f.at(KEY)), &[]);
            h.write(f.at(KEY), code);
            let next = h.call_sym(
                FIND_GLYPH,
                Conv::Thiscall(font.wrapping_add(0x10)),
                &[f.at(KEY), 0],
            );
            if next & 0x8000_0000 == 0 {
                let g2 = h
                    .read::<u32>(font.wrapping_add(0x14))
                    .wrapping_add(next.wrapping_mul(0x30));
                let kern = h.call_sym_st0(KERNING, Conv::Thiscall(text), &[g, g2]);
                pen.x = x87::store(add_m(f64::from(kern), pen.x));
            }
        }
        pen.line = max(h, pen.line, gh);
        let widest = h.read(text + 0x168);
        let widest = max(h, widest, pen.x);
        h.write(text + 0x168, widest);
        pen.quads = pen.quads.wrapping_add(1);
        code
    }
}
