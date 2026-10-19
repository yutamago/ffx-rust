//! Control codes in the game's encoded text, as the `TO*` message code
//! steps over them (`TOSkipCode`, `TOGetGroup`).

/// `TOSkipCode(p)` (`0x008dfd80`): how far the code at `p` reaches, given
/// its bytes (`byte(i)` is `p[i]`; only the bytes the original reads are
/// asked for). 0 for the terminator; a run of `G` (0x47) as a whole; code
/// 4 takes one argument byte, two when that one is `0x30` or more; codes
/// 1-3, 5, 0x0c, 0x0f, 0x11 and plain characters are one byte, the other
/// codes below 0x30 two.
pub fn skip_code(mut byte: impl FnMut(u32) -> u8) -> u32 {
    match byte(0) {
        0 => 0,
        4 => {
            if byte(1) > 0x2f {
                2
            } else {
                3
            }
        }
        6..=0x0b | 0x0d | 0x0e | 0x10 | 0x12..=0x2f => 2,
        b'G' => {
            let mut n = 1;
            while byte(n) == b'G' {
                n += 1;
            }
            n
        }
        _ => 1,
    }
}

/// `TOGetGroup(code)` (`0x008df610`): the group of a control code (by its
/// low byte): 1 for 0-3, 2 for 7-0x0b, 0x0d, 0x0e and 0x10, 3 for
/// 0x12-0x23, 4 for anything else.
#[must_use]
pub const fn group(code: u32) -> u32 {
    match code & 0xff {
        0..=3 => 1,
        7..=0x0b | 0x0d | 0x0e | 0x10 => 2,
        0x12..=0x23 => 3,
        _ => 4,
    }
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `TOSkipCode(p)` (`0x008dfd80`): the address after the code at `p`.
    pub fn skip_code(mem: &impl Mem, p: u32) -> u32 {
        p.wrapping_add(super::skip_code(|i| mem.read(p.wrapping_add(i))))
    }

    /// The glyph table (`0x00c58f00`): four bytes per glyph, its x and y in
    /// the font sheet, flags (bit 3: narrow) and its width.
    pub const GLYPHS: Global<[u8; 4]> = Global::new(0x00c5_8f00);
    /// The glyph cell height (`0x00b92314`, 8.0).
    pub const GLYPH_HEIGHT: Global<f32> = Global::new(0x00b9_2314);
    /// Added to the glyph's y for its bottom (`0x00b922a8`, 8.0 as a double).
    pub const GLYPH_BOTTOM: Global<f64> = Global::new(0x00b9_22a8);

    /// `0x0089e360(s, out)`: the glyph of the character at `s` (one byte
    /// from `0x30`, or `s[1] + 0xa0` after a lead byte below it) looked up
    /// into the sprite `out`: `+2` 0x14, `+8`/`+0xc` x and y, `+0x10`/`+0x14`
    /// right and bottom, `+0x1c` width, `+0x20` height, `+0x26` the flags
    /// and `+4` their bit 3. Returns `s`.
    pub fn glyph_sprite(mem: &mut impl Mem, s: u32, out: u32) -> u32 {
        let lead: u8 = mem.read(s);
        let index = if lead < 0x30 {
            u16::from(mem.read::<u8>(s.wrapping_add(1))) + 0xa0
        } else {
            u16::from(lead) - 0x30
        };
        let glyph: [u8; 4] = mem.read(GLYPHS.ptr(mem).addr.wrapping_add(u32::from(index) * 4));
        let (x, y, width) = (
            f32::from(glyph[0]),
            f32::from(glyph[1]),
            f32::from(glyph[3]),
        );
        mem.write(out.wrapping_add(2), 0x14u16);
        mem.write(out.wrapping_add(0x1c), width);
        mem.write(out.wrapping_add(0x20), GLYPH_HEIGHT.get(mem));
        mem.write(out.wrapping_add(8), x);
        mem.write(out.wrapping_add(0xc), y);
        mem.write(out.wrapping_add(0x10), width + x);
        #[allow(clippy::cast_possible_truncation)] // fstp to a float
        mem.write(
            out.wrapping_add(0x14),
            (f64::from(y) + GLYPH_BOTTOM.get(mem)) as f32,
        );
        mem.write(out.wrapping_add(0x26), u16::from(glyph[2]));
        mem.write(out.wrapping_add(4), u16::from(glyph[2] >> 3 & 1));
        s
    }

    /// `0x00a7b1a0(dst, src)`: `src` copied with each line feed turned into
    /// a terminator (an empty `dst` for a null `src`). Returns the length
    /// plus one (0 for null).
    pub fn copy_cutting_lines(mem: &mut impl Mem, dst: u32, src: u32) -> u32 {
        if src == 0 {
            mem.write(dst, 0u8);
            return 0;
        }
        let mut len = 0u32;
        while mem.read::<u8>(src.wrapping_add(len)) != 0 {
            len += 1;
        }
        let mut k = 0;
        loop {
            let c: u8 = mem.read(src.wrapping_add(k));
            mem.write(dst.wrapping_add(k), if c == b'\n' { 0 } else { c });
            k += 1;
            if c == 0 {
                return len + 1;
            }
        }
    }

    /// `0x0089e600(src, dst)`: a number string for the battle font: digits
    /// kept, a space as `:`, `-`, `.`, `/` as `G`, `H`, `I`; then the
    /// terminator. Any other character makes the original spin forever (it
    /// does not step past it); here it is skipped.
    pub fn number_font_string(mem: &mut impl Mem, src: u32, dst: u32) {
        let mut out = dst;
        let mut p = src;
        loop {
            let c: u8 = mem.read(p);
            if c == 0 {
                break;
            }
            let mapped = match c {
                b'0'..=b'9' => Some(c),
                b' ' => Some(b':'),
                b'-'..=b'/' => Some(b'G' + (c - b'-')),
                _ => None,
            };
            if let Some(m) = mapped {
                mem.write(out, m);
                out = out.wrapping_add(1);
            }
            p = p.wrapping_add(1);
        }
        mem.write(out, 0u8);
    }

    /// `0x008b7f40(p)`: `p` moved past control codes ([`skip_code`]) while
    /// its byte is above 2; where it stops.
    pub fn skip_codes_above_2(mem: &impl Mem, mut p: u32) -> u32 {
        while mem.read::<u8>(p) > 2 {
            p = skip_code(mem, p);
        }
        p
    }

    /// The ids `0x009066f0` shifts (`0x00b66e44`, zero-terminated).
    pub const SHIFTED_IDS: Global<u32> = Global::new(0x00b6_6e44);

    /// `0x009066f0(id)`: an id in `0x40e..=0x46e` that is 0x40e or in
    /// [`SHIFTED_IDS`] plus 100; any other unchanged.
    pub fn shift_listed_id(mem: &impl Mem, id: i32) -> i32 {
        if !(0x40e..=0x46e).contains(&id) {
            return id;
        }
        let table = SHIFTED_IDS.ptr(mem).addr;
        let mut cur = 0x40e;
        let mut k = 0u32;
        loop {
            if id == cur {
                return id + 0x64;
            }
            cur = mem.read::<i32>(table.wrapping_add(4 * k));
            k += 1;
            if cur == 0 {
                return id;
            }
        }
    }

    /// The message buffer `MsgExpandCodes_00776940` writes into is checked
    /// against (`*0x0112a02c`, 0x4000 bytes).
    pub const MES_BUFFER: u32 = 0x0112_a02c;
    /// The script's variables (`*0x0112a0a4`, words).
    pub const SCRIPT_VARS: u32 = 0x0112_a0a4;
    /// The script's getter, setter and call tables (`*0x00c416d0`,
    /// `*0x00c416d4`, `*0x00c416dc`, function pointers).
    pub const SCRIPT_GETTERS: u32 = 0x00c4_16d0;
    pub const SCRIPT_SETTERS: u32 = 0x00c4_16d4;
    pub const SCRIPT_CALLS: u32 = 0x00c4_16dc;
    /// Set by code `0xb 0xf9`.
    pub const EXPAND_FLAG_F9: u32 = 0x0112_a074;
    /// Where code `0xb 0xf7` jumps: its word plus `*0x0112a03c`.
    pub const EXPAND_JUMP: u32 = 0x0112_a084;
    pub const EXPAND_JUMP_BASE: u32 = 0x0112_a03c;
    /// `yiAssert(cond, text, file)`, `MenuTextCode_00774860(&out, &code,
    /// arg)` and `report_rangecheckfailure()`.
    pub const YI_ASSERT: u32 = 0x0090_ae50;
    pub const MENU_TEXT_CODE: u32 = 0x0077_4860;
    pub const RANGE_CHECK_FAILURE: u32 = 0x0094_9b02;
    /// The words [`expand_codes`] hands `MenuTextCode_00774860`: `out` and
    /// the code pointer (the original's locals).
    pub const EXPAND_SCRATCH_WORDS: u32 = 2;

    /// The two-byte codes past the condition codes' skip
    /// (`0xb 0xf4/0xf6/0xf7/0xf8/0xfa`): their record sizes.
    fn skipped_record(code: u32) -> u32 {
        match code {
            0xf40b => 8,
            0xf60b | 0xf80b => 0xc,
            0xf70b | 0xfa0b => 4,
            _ => 0,
        }
    }

    /// `MsgExpandCodes_00776940(out, text, arg, jump, flag)`: copies the
    /// message `text` to `out`, evaluating its script codes (`0xb` with
    /// 0xf0..=0xfa; codes are one byte from 0x30 and 3, else two):
    /// - `0xf0` (if): a 16-byte record `d0..d3`; `a` is getter
    ///   `d1 & 0xffff` called with `d1 >> 16` (type `d0 & 0xff` 0), variable
    ///   `d1 >> 16` (1) or 0; `b` likewise from `d2` (type byte 2 of `d0`) or
    ///   `d3`; operator byte 1 of `d0` (signed: 1 `!=`, 2 `>=`, 3 `<=`,
    ///   4 `>`, 5 `<`, 6 `&` non-zero, else `==`) pushes the result on the
    ///   condition stack.
    /// - `0xf1` (then) on a false condition and `0xf3` (else) on a true one
    ///   skip to the matching `0xf3`/`0xf2` (`0xf3` only for `0xf1`), over
    ///   nested ifs and the records of the other codes; `0xf2` (end if)
    ///   pops (`report_rangecheckfailure` at a depth from 0x40).
    /// - `0xf4`: `MenuTextCode_00774860(&out, &p, arg)` (in `scratch`).
    /// - `0xf6` (with `jump`): setter `d1 & 0xffff` called with `(d0 >> 16,
    ///   d2)` (type 0) or variable `d0 >> 16` = `d2` (1).
    /// - `0xf7` (with `jump`): [`EXPAND_JUMP`] = its word + `*0x0112a03c`,
    ///   `*jump = 1` and returns 0.
    /// - `0xf8` (with `jump`): call `d0 & 0xffff` of the call table with a
    ///   getter's value (kind byte 2 of `d0` 0), a variable (1), 0 (2, 4)
    ///   or `d2` (3).
    /// - `0xf9` sets [`EXPAND_FLAG_F9`]; `0xfa` sets `*flag` (from 0).
    ///
    /// `0x13 0x30..=0x37`, `0xb 0x30..=0x49`/`0x60` and every other code are
    /// copied. At the NUL: `*jump = 0`, returns the end past it. Each code
    /// first asserts `out` is under 0x4000 past [`MES_BUFFER`] (`yiAssert`).
    /// `call` takes absolute addresses (the tables' and the rebased
    /// symbols'). A condition depth beyond the original's 64-byte stack
    /// is undefined there; here those writes are dropped and reads give 0.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn expand_codes<M: Mem>(
        mem: &mut M,
        [out, text, arg, jump, flag]: [u32; 5],
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if flag != 0 {
            mem.write(flag, 0u32);
        }
        let mut cond = [0u8; 0x40];
        let mut depth = 0u32;
        let (mut out, mut p) = (out, text);
        let entry = |mem: &M, table: u32, i: u32| -> u32 {
            let base: u32 = mem.read(mem.rebase(table));
            mem.read(base.wrapping_add(i.wrapping_mul(4)))
        };
        let var = |mem: &M, i: u32| -> u32 { entry(mem, SCRIPT_VARS, i) };
        let word = |mem: &M, at: u32| -> u32 { mem.read(at) };
        let get = |c: &[u8; 0x40], d: u32| c.get(d as usize).copied().unwrap_or(0);
        // the next code of a skip: (code, its pointer past it) or the NUL
        let next_code = |mem: &M, p: &mut u32| -> Option<u32> {
            loop {
                let c: u8 = mem.read(*p);
                *p = p.wrapping_add(1);
                if c == 0 {
                    return None;
                }
                if c < 0x30 && c != 3 {
                    let hi: u8 = mem.read(*p);
                    *p = p.wrapping_add(1);
                    return Some(u32::from(c) | u32::from(hi) << 8);
                }
            }
        };
        loop {
            let base: u32 = mem.read(mem.rebase(MES_BUFFER));
            if out.wrapping_sub(base).cast_signed() >= 0x4000 {
                let (text, file) = (mem.rebase(0x00b5_313c), mem.rebase(0x00b5_312c));
                call(mem, mem.rebase(YI_ASSERT), &[0, text, file]);
            }
            let c: u8 = mem.read(p);
            p = p.wrapping_add(1);
            if c == 0 {
                break;
            }
            if c >= 0x30 || c == 3 {
                mem.write(out, c);
                out = out.wrapping_add(1);
                continue;
            }
            let a: u8 = mem.read(p);
            p = p.wrapping_add(1);
            let code = u32::from(c) | u32::from(a) << 8;
            let copy = (c == 0x13 && a.wrapping_sub(0x30) <= 7)
                || (c == 0xb && ((0x30..=0x49).contains(&a) || a == 0x60));
            let mut end_if = false;
            match code {
                0xf90b => {
                    mem.write(mem.rebase(EXPAND_FLAG_F9), 1u32);
                }
                _ if copy => {
                    mem.write(out, [c, a]);
                    out = out.wrapping_add(2);
                }
                0xf00b => {
                    let [d0, d1, d2, d3]: [u32; 4] = mem.read(p);
                    p = p.wrapping_add(0x10);
                    let left = match d0 & 0xff {
                        0 => {
                            let f = entry(mem, SCRIPT_GETTERS, d1 & 0xffff);
                            call(mem, f, &[d1 >> 16])
                        }
                        1 => var(mem, d1 >> 16),
                        _ => 0,
                    };
                    let right = match (d0 >> 16) & 0xff {
                        0 => {
                            let f = entry(mem, SCRIPT_GETTERS, d2 & 0xffff);
                            call(mem, f, &[d2 >> 16])
                        }
                        1 => var(mem, d2 >> 16),
                        _ => d3,
                    };
                    let (l, r) = (left.cast_signed(), right.cast_signed());
                    let hit = match (d0 >> 8) & 0xff {
                        1 => r != l,
                        2 => l >= r,
                        3 => l <= r,
                        4 => l > r,
                        5 => l < r,
                        6 => left & right != 0,
                        _ => r == l,
                    };
                    if let Some(slot) = cond.get_mut(depth.wrapping_add(1) as usize) {
                        *slot = u8::from(hit);
                    }
                    depth = depth.wrapping_add(1);
                }
                0xf10b => {
                    if get(&cond, depth) == 0 {
                        let mut nest = 0i32;
                        loop {
                            let Some(code) = next_code(mem, &mut p) else {
                                return finish(mem, out, jump);
                            };
                            if nest <= 0 && code == 0xf30b {
                                break;
                            }
                            if nest <= 0 && code == 0xf20b {
                                end_if = true;
                                break;
                            }
                            match code {
                                0xf00b => {
                                    nest += 1;
                                    p = p.wrapping_add(0x10);
                                }
                                0xf20b => nest -= 1,
                                _ => p = p.wrapping_add(skipped_record(code)),
                            }
                        }
                    }
                }
                0xf20b => end_if = true,
                0xf30b => {
                    if get(&cond, depth) != 0 {
                        let mut nest = 0i32;
                        loop {
                            let Some(code) = next_code(mem, &mut p) else {
                                return finish(mem, out, jump);
                            };
                            match code {
                                0xf00b => {
                                    nest += 1;
                                    p = p.wrapping_add(0x10);
                                }
                                0xf20b if nest <= 0 => {
                                    end_if = true;
                                    break;
                                }
                                0xf20b => nest -= 1,
                                _ => p = p.wrapping_add(skipped_record(code)),
                            }
                        }
                    }
                }
                0xf40b => {
                    mem.write(scratch, [out, p]);
                    let f = mem.rebase(MENU_TEXT_CODE);
                    call(mem, f, &[scratch, scratch.wrapping_add(4), arg]);
                    [out, p] = mem.read(scratch);
                }
                0xf60b => {
                    if jump != 0 {
                        let [d0, d1, d2]: [u32; 3] = mem.read(p);
                        match d0 & 0xff {
                            0 => {
                                let f = entry(mem, SCRIPT_SETTERS, d1 & 0xffff);
                                call(mem, f, &[d0 >> 16, d2]);
                            }
                            1 => {
                                let at = mem.read::<u32>(mem.rebase(SCRIPT_VARS));
                                mem.write(at.wrapping_add((d0 >> 16).wrapping_mul(4)), d2);
                            }
                            _ => {}
                        }
                    }
                    p = p.wrapping_add(0xc);
                }
                0xf70b => {
                    if jump != 0 {
                        let to = word(mem, p).wrapping_add(word(mem, mem.rebase(EXPAND_JUMP_BASE)));
                        mem.write(mem.rebase(EXPAND_JUMP), to);
                        mem.write(jump, 1u32);
                        return 0;
                    }
                    p = p.wrapping_add(4);
                }
                0xf80b => {
                    if jump != 0 {
                        let [d0, d1]: [u32; 2] = mem.read(p);
                        let value = match (d0 >> 16) & 0xff {
                            0 => {
                                let f = entry(mem, SCRIPT_GETTERS, d1 & 0xffff);
                                Some(call(mem, f, &[d1 >> 16]))
                            }
                            1 => Some(var(mem, d1 >> 16)),
                            2 | 4 => Some(0),
                            3 => Some(word(mem, p.wrapping_add(8))),
                            _ => None,
                        };
                        if let Some(v) = value {
                            let f = entry(mem, SCRIPT_CALLS, d0 & 0xffff);
                            call(mem, f, &[v]);
                        }
                    }
                    p = p.wrapping_add(0xc);
                }
                0xfa0b => {
                    if flag != 0 {
                        mem.write(flag, 1u32);
                    }
                    p = p.wrapping_add(4);
                }
                _ => {
                    mem.write(out, [c, a]);
                    out = out.wrapping_add(2);
                }
            }
            if end_if {
                if depth >= 0x40 {
                    let f = mem.rebase(RANGE_CHECK_FAILURE);
                    call(mem, f, &[]);
                    return 0;
                }
                cond[depth as usize] = 0;
                depth = depth.wrapping_sub(1);
            }
        }
        finish(mem, out, jump)
    }

    /// The end of [`expand_codes`]: the NUL written, `*jump = 0`, the end
    /// past the NUL.
    fn finish(mem: &mut impl Mem, out: u32, jump: u32) -> u32 {
        mem.write(out, 0u8);
        if jump != 0 {
            mem.write(jump, 0u32);
        }
        out.wrapping_add(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes() {
        let at = |s: &'static [u8]| move |i: u32| s[i as usize];
        assert_eq!(skip_code(at(b"\0")), 0);
        assert_eq!(skip_code(at(b"GGGx")), 3);
        assert_eq!(skip_code(at(b"\x04\x30")), 2);
        assert_eq!(skip_code(at(b"\x04\x2f")), 3);
        assert_eq!(skip_code(at(b"\x0a")), 2);
        assert_eq!(skip_code(at(b"a")), 1);
        assert_eq!(group(0x112), 3);
    }
}
