//! M5 finish, WP5 part 5 (wp5f): the settings code's `std::stringstream`
//! instantiations (`ffx_system::wp5f_sstream`) against the original, on
//! the image with synthetic streams and on the snapshots (`FFX_SNAPSHOT`)
//! over the game's memory. Results and every byte written must agree.
//! Needs `FFX_GAME_DIR`.
//!
//! `MSVCP110`/`MSVCR110` imports are recording stubs over their import
//! slots (thiscall ones log `ecx`); `FFX.exe` callees the ports call
//! through the table are recording stubs over their code. Streams are
//! built by hand: a virtual-base table, a `basic_ios` with its state,
//! flags, width, fill, tie and a streambuf whose `_Lock`/`_Unlock` are
//! stubs.

#![allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod m5_main3_common;

use m5_main3_common::{Out, Ret, Rng, Twins, store_arg};

use ffx_bridge::Mem;
use ffx_oracle::Conv;
use ffx_system::wp5f_sstream::bridge as ss;

/// A recording stub over an import slot: `this` logs `ecx`; `n` words
/// logged, `pops` popped; the result `ret`; `extra` code run first.
fn imp(t: &mut Twins, slot: u32, this: bool, n: u32, pops: u32, ret: &Ret, extra: &[u8]) {
    let a = t.stub_code(slot, this, n, pops, ret, extra);
    let at = t.rebase(slot);
    t.patch(at, &a.to_le_bytes());
}

/// A thiscall import stub popping its `n` words.
fn this_imp(t: &mut Twins, slot: u32, n: u32, ret: u32) {
    imp(t, slot, true, n, n, &Ret::Fixed(ret), &[]);
}

/// A cdecl import stub.
fn cdecl_imp(t: &mut Twins, slot: u32, n: u32, ret: u32) {
    imp(t, slot, false, n, 0, &Ret::Fixed(ret), &[]);
}

/// The data import `slot` pointed at `words` (its value).
fn data_imp(t: &mut Twins, slot: u32, words: &[u32]) -> u32 {
    let p = t.words(words);
    let at = t.rebase(slot);
    t.patch(at, &p.to_le_bytes());
    p
}

/// A stringbuf at `sb` (0x40 bytes): its six pointer members at cells
/// holding `v` = get first, put first, get next, put next, get count, put
/// count; the seek mark and the mode.
fn stringbuf(t: &mut Twins, sb: u32, v: [u32; 6], high: u32, mode: u32) {
    let cells = t.words(&v);
    for (i, off) in [0xcu32, 0x10, 0x1c, 0x20, 0x2c, 0x30]
        .into_iter()
        .enumerate()
    {
        t.w32(sb + off, cells + 4 * i as u32);
    }
    t.w32(sb + 0x38, high);
    t.w32(sb + 0x3c, mode);
}

/// A stream with its `basic_ios` 0x40 bytes in: state, flags, width,
/// fill, tie; its streambuf has stub `_Lock`/`_Unlock` (slots 1, 2).
/// Returns (stream, ios, streambuf).
fn stream(
    t: &mut Twins,
    state: u32,
    flags: u32,
    width: u64,
    fill: u8,
    tie: u32,
) -> (u32, u32, u32) {
    let s = t.zeros(0x100);
    let vb = t.words(&[0, 0x40]);
    t.w32(s, vb);
    let ios = s + 0x40;
    let sb = t.zeros(0x40);
    t.fake_vtable(
        sb,
        &[(0, 0x71, vec![]), (0, 0x72, vec![]), (0, 0x73, vec![])],
    );
    t.w32(ios + 0xc, state);
    t.w32(ios + 0x14, flags);
    t.w32(ios + 0x20, width as u32);
    t.w32(ios + 0x24, (width >> 32) as u32);
    t.w32(ios + 0x38, sb);
    t.w32(ios + 0x3c, tie);
    t.w8(ios + 0x40, fill);
    (s, ios, sb)
}

/// The ostream imports: `flush`, `sputc` (results in turn), `sputn`
/// (`n_ret`), `setstate`, `uncaught_exception`, `_Osfx`.
fn ostream_imports(t: &mut Twins, sputc: &[u32], n_ret: u32, uncaught: u32) {
    this_imp(t, ss::FLUSH, 0, 0x61);
    imp(t, ss::SPUTC, true, 1, 1, &Ret::Seq(sputc.to_vec()), &[]);
    this_imp(t, ss::SPUTN, 3, n_ret);
    this_imp(t, ss::SETSTATE, 2, 0x62);
    cdecl_imp(t, ss::UNCAUGHT_EXCEPTION, 0, uncaught);
    this_imp(t, ss::OSFX, 0, 0x63);
}

#[test]
fn wp5f_stream_ctors_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0040_2f00);
        for k in 0..12 {
            t.reset();
            this_imp(t, ss::IOS_CTOR, 0, 0x51);
            this_imp(t, ss::IOSTREAM_CTOR, 2, 0x52);
            this_imp(t, ss::ISTREAM_CTOR, 3, 0x53);
            this_imp(t, ss::OSTREAM_CTOR, 3, 0x54);
            this_imp(t, ss::STREAMBUF_CTOR, 0, 0x55);
            t.stub(ss::STRINGBUF_INIT, true, 2, 2, 0x56);
            let junk: Vec<u32> = (0..0x30).map(|_| rng.next()).collect();
            let obj = t.words(&junk);
            t.check(
                &format!("stringstream ctor {k}"),
                0x0040_2f00,
                Conv::Thiscall(obj),
                &[7, 8],
                |m| Out::U32(ss::stringstream_ctor(m, obj)),
            );
            let obj = t.words(&junk);
            t.check(
                &format!("ostringstream ctor {k}"),
                0x0040_3730,
                Conv::Thiscall(obj),
                &[7, 8],
                |m| Out::U32(ss::ostringstream_ctor(m, obj)),
            );
            // a small, a large and an empty string
            let s = t.zeros(0x18);
            let big = t.alloc(b"0123456789abcdefghij\0");
            match k % 3 {
                0 => {
                    t.write(s, b"abc\0");
                    t.w32(s + 0x10, 3);
                    t.w32(s + 0x14, 0xf);
                }
                1 => {
                    t.w32(s, big);
                    t.w32(s + 0x10, 20);
                    t.w32(s + 0x14, 0x1f);
                }
                _ => t.w32(s + 0x14, rng.pick(&[0xf, 0x10])),
            }
            let obj = t.words(&junk);
            t.check(
                &format!("istringstream ctor {k}"),
                0x0040_3620,
                Conv::Thiscall(obj),
                &[s, 7, 8],
                |m| Out::U32(ss::istringstream_ctor(m, obj, s)),
            );
        }
    });
}

#[test]
fn wp5f_stream_str_and_dtors_match_the_original() {
    let streams = [
        (ss::STRINGSTREAM, 0x0040_3830u32, 0x0040_2fc0u32),
        (ss::ISTRINGSTREAM, 0x0040_38c0, 0x0040_3700),
        (ss::OSTRINGSTREAM, 0x0040_3920, 0x0040_37e0),
    ];
    Twins::each(|t| {
        let mut rng = Rng(0x0040_3010);
        for k in 0..24 {
            t.reset();
            t.stub(ss::STRINGBUF_STR, true, 1, 1, 0x41);
            t.stub(ss::OPERATOR_DELETE, false, 1, 0, 0x42);
            this_imp(t, ss::STREAMBUF_DTOR, 0, 0x43);
            for slot in [
                ss::IOSTREAM_DTOR,
                ss::ISTREAM_DTOR,
                ss::OSTREAM_DTOR,
                ss::IOS_DTOR,
            ] {
                this_imp(t, slot, 0, slot & 0xff);
            }
            let obj = t.zeros(0x80);
            let out = t.zeros(0x18);
            t.check(
                &format!("stringstream str {k}"),
                0x0040_2ff0,
                Conv::Thiscall(obj),
                &[out],
                |m| Out::U32(ss::stream_str(m, obj, 0x18, out)),
            );
            t.check(
                &format!("ostringstream str {k}"),
                0x0040_3810,
                Conv::Thiscall(obj),
                &[out],
                |m| Out::U32(ss::stream_str(m, obj, 4, out)),
            );
            // the stringbuf's own destructors (the real tidy)
            let sb = t.zeros(0x40);
            let v = [
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
            ];
            stringbuf(t, sb, v, rng.next(), rng.pick(&[0, 1, 3, 0x13]));
            t.check(
                &format!("stringbuf tidy {k}"),
                ss::STRINGBUF_TIDY,
                Conv::Thiscall(sb),
                &[],
                |m| Out::U32(ss::stringbuf_tidy(m, sb)),
            );
            stringbuf(t, sb, v, rng.next(), rng.pick(&[0, 1]));
            let flags = rng.pick(&[0, 1, 2, 3]);
            t.check(
                &format!("stringbuf deleting {k}"),
                0x0040_3890,
                Conv::Thiscall(sb),
                &[flags],
                |m| Out::U32(ss::stringbuf_deleting_dtor(m, sb, flags)),
            );
            // the streams' destructors over the real vbase tables
            t.stub(ss::STRINGBUF_TIDY, true, 0, 0, 0x44);
            let (s, del, vbase) = streams[k % 3];
            let full = t.zeros(0x80);
            let vb = rng.pick(&[ss::SS_VBTABLE, ss::ISS_VBTABLE, ss::OSS_VBTABLE]);
            let vb = t.rebase(vb);
            t.w32(full, vb);
            let ios = full + s.ios;
            t.check(
                &format!("deleting dtor {k}"),
                del,
                Conv::Thiscall(ios),
                &[flags],
                |m| Out::U32(ss::stream_deleting_dtor(m, ios, flags, s)),
            );
            t.w32(full, vb);
            t.check(
                &format!("vbase dtor {k}"),
                vbase,
                Conv::Thiscall(ios),
                &[],
                |m| Out::U32(ss::stream_vbase_dtor(m, ios, s)),
            );
        }
    });
}

/// `_Pninc` as the DLL does it: the put count down, the put pointer's old
/// value returned and incremented (not logged).
fn pninc_code(t: &mut Twins) -> u32 {
    t.code(&[
        0x8b, 0x41, 0x30, // mov eax, [ecx + 0x30]
        0xff, 0x08, // dec dword [eax]
        0x8b, 0x51, 0x20, // mov edx, [ecx + 0x20]
        0x8b, 0x02, // mov eax, [edx]
        0xff, 0x02, // inc dword [edx]
        0xc3, // ret
    ])
}

#[test]
fn wp5f_stringbuf_overflow_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0040_3080);
        for k in 0..300 {
            t.reset();
            let pn = pninc_code(t);
            let at = t.rebase(ss::PNINC);
            t.patch(at, &pn.to_le_bytes());
            let fresh = t.zeros(0x200);
            // (operator new never fails here: the original's `_Xbad_alloc`
            // does not return)
            t.stub(ss::OPERATOR_NEW, false, 1, 0, fresh);
            t.stub(ss::OPERATOR_DELETE, false, 1, 0, 0x42);
            t.stub(ss::MEMCPY, false, 3, 0, 0x43);
            cdecl_imp(t, ss::XBAD_ALLOC, 0, 0x44);
            let buf = t.zeros(0x100);
            let size = rng.pick(&[0u32, 8, 0x20, 0x40, 0x100]);
            let used = if size == 0 { 0 } else { rng.below(size + 1) };
            let pnext = if size == 0 && rng.chance(2) {
                0
            } else {
                buf + used
            };
            let pcount = if pnext == 0 { 0 } else { size - used };
            let gnext = rng.pick(&[0, buf, buf + used / 2]);
            let high = rng.pick(&[0, buf + size, buf + used + 4, pnext]);
            let mode = rng.pick(&[0u32, 1, 4, 5, 8, 9, 0xd, 2, 0x11]);
            let sb = t.zeros(0x40);
            stringbuf(
                t,
                sb,
                [buf, buf, gnext, pnext, rng.below(9), pcount],
                high,
                mode,
            );
            let c = rng.pick(&[0x41u32, u32::MAX, 0x1ff, 0]);
            let what = format!("overflow {k} (size {size:#x}, used {used:#x}, mode {mode:#x})");
            t.check(&what, 0x0040_3080, Conv::Thiscall(sb), &[c], |m| {
                Out::U32(ss::stringbuf_overflow(m, sb, c))
            });
        }
    });
}

#[test]
fn wp5f_stringbuf_get_side_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0040_3260);
        for k in 0..300 {
            t.reset();
            let junk: Vec<u32> = (0..0x40).map(|_| rng.next()).collect();
            let buf = t.words(&junk);
            let gfirst = buf + rng.below(4);
            let gnext = rng.pick(&[0, gfirst, gfirst + 1, gfirst + 8, gfirst + 0x20]);
            let gcount = rng.pick(&[0u32, 1, 4, u32::MAX]);
            let pnext = rng.pick(&[0, gfirst + 0x10, gfirst + 0x30]);
            let high = rng.pick(&[0, gfirst + 0x18, gfirst + 0x40]);
            let mode = rng.pick(&[0u32, 2, 4, 6, 1]);
            let sb = t.zeros(0x40);
            stringbuf(t, sb, [gfirst, gfirst, gnext, pnext, gcount, 4], high, mode);
            let c = if rng.chance(3) && gnext > gfirst {
                u32::from(junk[0].to_le_bytes()[0]) | (gnext - gfirst) << 24
            } else {
                rng.pick(&[u32::MAX, 0x41, 0x1_0041])
            };
            // the same character as the one before the get pointer
            let c = if gnext > gfirst && rng.chance(3) {
                let mut b = [0u8];
                Mem::read_bytes(&t.a, gnext - 1, &mut b);
                u32::from(b[0]) | 0x300
            } else {
                c
            };
            t.check(
                &format!("pbackfail {k}"),
                0x0040_3260,
                Conv::Thiscall(sb),
                &[c],
                |m| Out::U32(ss::stringbuf_pbackfail(m, sb, c)),
            );
            stringbuf(t, sb, [gfirst, gfirst, gnext, pnext, gcount, 4], high, mode);
            t.check(
                &format!("underflow {k}"),
                0x0040_32c0,
                Conv::Thiscall(sb),
                &[],
                |m| Out::U32(ss::stringbuf_underflow(m, sb)),
            );
        }
    });
}

#[test]
fn wp5f_stringbuf_seeks_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0040_3320);
        for k in 0..400 {
            t.reset();
            data_imp(t, ss::BADOFF, &[u32::MAX, u32::MAX]);
            let buf = t.zeros(0x80);
            let gnext = rng.pick(&[0, buf, buf + 0x10]);
            let pnext = rng.pick(&[0, buf + 4, buf + 0x30]);
            let high = rng.pick(&[0, buf + 0x20, buf + 0x40]);
            let sb = t.zeros(0x40);
            stringbuf(t, sb, [buf, buf, gnext, pnext, 8, 8], high, 0);
            let off = rng.pick(&[
                0u64,
                4,
                0x10,
                0x20,
                0x21,
                0x40,
                u64::MAX,
                u64::MAX - 3,
                1 << 32,
            ]);
            let way = rng.pick(&[0, 1, 2, 3]);
            let which = rng.pick(&[0, 1, 2, 3]);
            let out = t.zeros(0x18);
            let what = format!("seekoff {k} (off {off:#x}, way {way}, which {which})");
            t.check(
                &what,
                0x0040_3320,
                Conv::Thiscall(sb),
                &[out, off as u32, (off >> 32) as u32, way, which],
                |m| Out::U32(ss::stringbuf_seekoff(m, sb, out, off, way, which)),
            );
            stringbuf(t, sb, [buf, buf, gnext, pnext, 8, 8], high, 0);
            let fpos = rng.pick(&[0u64, 4, 0x10, u64::MAX]);
            let what = format!("seekpos {k} (off {off:#x}, fpos {fpos:#x}, which {which})");
            t.check(
                &what,
                0x0040_34f0,
                Conv::Thiscall(sb),
                &[
                    out,
                    off as u32,
                    (off >> 32) as u32,
                    fpos as u32,
                    (fpos >> 32) as u32,
                    7,
                    8,
                    which,
                ],
                |m| Out::U32(ss::stringbuf_seekpos(m, sb, out, off, fpos, which)),
            );
        }
    });
}

#[test]
fn wp5f_stringbuf_str_and_init_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0040_3ec0);
        for k in 0..200 {
            t.reset();
            t.stub(ss::STRING_ASSIGN, true, 2, 2, 0x51);
            // the move leaves the temporary as the test says: small or not
            let cap = rng.pick(&[0xfu32, 0x10, 0x1f]);
            t.stub_ex(
                ss::STRING_CTOR_MOVE,
                true,
                1,
                1,
                &Ret::Fixed(0x52),
                &store_arg(0, 0x14, cap),
            );
            t.stub(ss::OPERATOR_DELETE, false, 1, 0, 0x53);
            cdecl_imp(t, ss::MEMMOVE, 3, 0x54);
            let buf = t.zeros(0x80);
            let pnext = rng.pick(&[0, buf + 4, buf + 0x30]);
            let gnext = rng.pick(&[0, buf, buf + 0x10]);
            let high = rng.pick(&[0, buf + 0x20, buf + 0x40]);
            let mode = rng.pick(&[0u32, 2, 4, 6]);
            let sb = t.zeros(0x40);
            stringbuf(t, sb, [buf, buf + 2, gnext, pnext, 8, 8], high, mode);
            let out = t.zeros(0x18);
            t.w32(out, rng.next());
            t.w32(out + 0x10, rng.next());
            t.w32(out + 0x14, rng.next());
            t.skip = 8;
            t.check_framed(
                &format!("stringbuf str {k}"),
                ss::STRINGBUF_STR,
                Conv::Thiscall(sb),
                &[out],
                0x20,
                |m, ebp| Out::U32(ss::stringbuf_str(m, sb, out, ebp - 0x20)),
            );
            t.skip = 4;
            // _Init
            let fresh = t.zeros(0x40);
            t.stub(ss::OPERATOR_NEW, false, 1, 0, fresh);
            t.stub(ss::MEMCPY, false, 3, 0, 0x55);
            cdecl_imp(t, ss::XBAD_ALLOC, 0, 0x56);
            let gnext = rng.pick(&[0, buf]);
            let mode = rng.pick(&[0u32, 2, 4, 6, 0x10, 0x14, 0x12]);
            stringbuf(
                t,
                sb,
                [rng.next(), rng.next(), gnext, rng.next(), 3, 4],
                0,
                mode,
            );
            let n = rng.pick(&[1u32, 8, 0x20]);
            t.check(
                &format!("stringbuf init {k}"),
                ss::STRINGBUF_INIT,
                Conv::Thiscall(sb),
                &[buf, n],
                |m| {
                    ss::stringbuf_init(m, sb, buf, n);
                    Out::None
                },
            );
        }
    });
}

/// The inserters' `sputc` words: the character's byte over a frame word
/// the original never writes otherwise (junk read, brief 2026-10-24: the
/// port passes the byte alone; the frame is zeroed here, so the
/// original's words are the byte too).
#[test]
fn wp5f_ostream_inserters_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0040_4a90);
        for k in 0..400 {
            t.reset();
            let seq: Vec<u32> = (0..6)
                .map(|_| rng.pick(&[0x2c, 0x20, 0x2c, u32::MAX]))
                .collect();
            let text = rng.pick(&["", "a", "hello", "0123456789abcdef!"]);
            let len = text.len() as u32;
            let n_ret = rng.pick(&[len, len, len.wrapping_sub(1)]);
            // (the bool in al: 0x100 is false)
            ostream_imports(t, &seq, n_ret, rng.pick(&[0, 0, 1, 0x100]));
            let tie = if rng.chance(3) { t.zeros(0x10) } else { 0 };
            // (a width past 1024 would overflow the call log)
            let width = rng.pick(&[0u64, 1, 2, 3, 5, 20, 1 << 63]);
            let flags = rng.pick(&[0u32, 0x40, 0x80, 0x1c0, 0x241]);
            let state = rng.pick(&[0u32, 0, 0, 2]);
            let (os, _, sb) = stream(t, state, flags, width, b'*', tie);
            if rng.chance(8) {
                // no streambuf
                t.w32(os + 0x40 + 0x38, 0);
                let _ = sb;
            }
            let what = format!("put_comma {k} (width {width:#x}, flags {flags:#x}, state {state})");
            t.skip = 0x40;
            t.check_framed(&what, 0x0040_4a90, Conv::Thiscall(os), &[], 0x40, |m, _| {
                Out::U32(ss::put_comma(m, os))
            });
            let s = t.alloc(&[text.as_bytes(), b"\0"].concat());
            let (os, _, _) = stream(t, state, flags, width, b'*', tie);
            let what = format!("put_cstr {k} ({text:?}, width {width:#x}, flags {flags:#x})");
            t.check_framed(
                &what,
                0x0040_4ca0,
                Conv::Fastcall,
                &[os, s],
                0x40,
                |m, _| Out::U32(ss::put_cstr(m, os, s)),
            );
            // the std::string
            let st = t.zeros(0x18);
            if len < 16 {
                t.write(st, &[text.as_bytes(), b"\0"].concat());
                t.w32(st + 0x14, 0xf);
            } else {
                t.w32(st, s);
                t.w32(st + 0x14, 0x1f);
            }
            t.w32(st + 0x10, len);
            let (os, _, _) = stream(t, state, flags, width, b'*', tie);
            let what = format!("put_string {k} ({text:?}, width {width:#x}, flags {flags:#x})");
            t.check_framed(
                &what,
                0x0040_4ed0,
                Conv::Fastcall,
                &[os, st],
                0x40,
                |m, _| Out::U32(ss::put_string(m, os, st)),
            );
            t.skip = 4;
        }
    });
}

#[test]
fn wp5f_istream_extractor_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0040_5800);
        for k in 0..200 {
            t.reset();
            let ok = rng.pick(&[1u32, 1, 1, 0, 0x100]);
            this_imp(t, ss::IPFX, 1, ok);
            // getloc stores the locale's _Locimp into its result
            let imp_obj = t.zeros(0x20);
            let dead = t.zeros(0x10);
            t.fake_vtable(dead, &[(1, 0x81, vec![])]);
            let decref = rng.pick(&[0, dead]);
            t.fake_vtable(
                imp_obj,
                &[(0, 0x82, vec![]), (0, 0x83, vec![]), (0, decref, vec![])],
            );
            let imp_word = if rng.chance(6) { 0 } else { imp_obj };
            let mut store = store_arg(0, 0, imp_word);
            store.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp + 4]
            let gl = t.stub_code(ss::GETLOC, true, 1, 1, &Ret::Fixed(0), &store);
            let at = t.rebase(ss::GETLOC);
            t.patch(at, &gl.to_le_bytes());
            // the ctype facet: spaces at ' ' and '\n'
            let mut table = vec![0u8; 0x200];
            table[2 * 0x20] = 0x48;
            table[2 * 0x0a] = 0x08;
            let table = t.alloc(&table);
            let facet = t.zeros(0x20);
            t.w32(facet + 0xc, table);
            t.stub(ss::USE_CTYPE, true, 0, 0, facet);
            // the stream ends after 8 characters at the latest
            let mut chars: Vec<u32> = (0..8)
                .map(|_| rng.pick(&[0x41, 0x42, 0x20, 0x0a, u32::MAX, 0x141]))
                .collect();
            chars.push(u32::MAX);
            imp(t, ss::SGETC, true, 0, 0, &Ret::Fixed(chars[0]), &[]);
            imp(
                t,
                ss::SNEXTC,
                true,
                0,
                0,
                &Ret::Seq(chars[1..].to_vec()),
                &[],
            );
            t.stub(ss::STRING_PUSH_BACK, true, 2, 2, 0x84);
            this_imp(t, ss::SETSTATE, 2, 0x85);
            let width = rng.pick(&[0u64, 1, 3, u64::MAX, 0xffff_fffe, 1 << 32]);
            let (is, _, _) = stream(t, 0, 0, width, b' ', 0);
            let s = t.zeros(0x18);
            let big = t.zeros(0x20);
            if rng.chance(2) {
                t.w32(s, big);
                t.w32(s + 0x14, 0x1f);
            } else {
                t.w32(s + 0x14, 0xf);
            }
            t.w32(s + 0x10, rng.below(9));
            t.skip = 0x18;
            let what = format!("get_string {k} (ok {ok}, width {width:#x}, chars {chars:x?})");
            t.check_framed(
                &what,
                0x0040_5800,
                Conv::Fastcall,
                &[is, s],
                0x1c,
                |m, ebp| Out::U32(ss::get_string(m, is, s, ebp - 0x1c)),
            );
            t.skip = 4;
        }
    });
}

#[test]
fn wp5f_use_facet_and_push_back_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0040_49a0);
        for k in 0..200 {
            t.reset();
            this_imp(t, ss::LOCKIT_CTOR, 1, 0x61);
            this_imp(t, ss::LOCKIT_DTOR, 0, 0x62);
            let id = rng.pick(&[0u32, 3, 9]);
            let id_obj = t.zeros(4);
            let at = t.rebase(ss::CTYPE_ID);
            t.patch(at, &id_obj.to_le_bytes());
            this_imp(t, ss::ID_TO_UINT, 0, id);
            // locales: facet vectors of 5 words
            let made = t.zeros(0x10);
            t.fake_vtable(made, &[(0, 0x63, vec![]), (0, 0x64, vec![])]);
            let mk = |t: &mut Twins, rng: &mut Rng| -> u32 {
                let v: Vec<u32> = (0..5).map(|_| rng.pick(&[0, 0, 0x0123_4560])).collect();
                let v = t.words(&v);
                let imp = t.zeros(0x20);
                t.w32(imp + 8, v);
                t.w32(imp + 0xc, rng.pick(&[0, 4, 5]));
                t.w8(imp + 0x14, rng.pick(&[0, 1]));
                imp
            };
            let local = mk(t, &mut rng);
            let global = mk(t, &mut rng);
            cdecl_imp(t, ss::GLOBAL_LOCALE, 0, global);
            let loc = t.words(&[local]);
            let cache = rng.pick(&[0, 0, 0x0777_0000]);
            t.g32(ss::CTYPE_CACHE, cache);
            // _Getcat(&saved, &loc): stores the new facet, or fails
            // (`_Getcat` failing throws `bad_cast`, which never returns:
            // not run)
            let fail = false;
            let store = store_arg(0, 0, made);
            imp(
                t,
                ss::CTYPE_GETCAT,
                false,
                2,
                0,
                &Ret::Fixed(if fail { u32::MAX } else { 2 }),
                &store,
            );
            this_imp(t, ss::BAD_CAST_CTOR, 1, 0x65);
            t.stub(ss::CXX_THROW, false, 2, 0, 0x66);
            t.stub(ss::FACET_REGISTER, false, 1, 0, 0x67);
            t.skip = 0xc;
            t.check_framed(
                &format!("use_facet {k}"),
                ss::USE_CTYPE,
                Conv::Fastcall,
                &[loc, 0],
                0x20,
                |m, ebp| Out::U32(ss::use_ctype(m, loc, ebp - 0x20)),
            );
            t.skip = 4;
            // Facet_Register (its stub above out of the way)
            t.reset();
            let node = t.zeros(8);
            t.stub(ss::OPERATOR_NEW, false, 1, 0, rng.pick(&[node, node, 0]));
            t.g32(ss::FACET_LIST, rng.next());
            let f = rng.next();
            t.check(
                &format!("Facet_Register {k}"),
                ss::FACET_REGISTER,
                Conv::Cdecl,
                &[f],
                |m| Out::U32(ss::facet_register(m, f)),
            );
            // push_back
            t.stub(ss::STRING_COPY, true, 2, 2, 0x68);
            cdecl_imp(t, ss::XLENGTH_ERROR, 1, 0x69);
            let s = t.zeros(0x18);
            let big = t.zeros(0x30);
            let size = rng.pick(&[0u32, 3, 0xe, 0xf, 0x10, 0x1e, 0x1f]);
            let cap = rng.pick(&[0xfu32, 0x1f, 0x2f]);
            if cap >= 0x10 {
                t.w32(s, big);
            }
            t.w32(s + 0x10, size);
            t.w32(s + 0x14, cap);
            let c = rng.pick(&[0x41u32, 0x1_42]);
            let real = size;
            t.check(
                &format!("push_back {k} (size {real:#x}, cap {cap:#x})"),
                ss::STRING_PUSH_BACK,
                Conv::Thiscall(s),
                &[7, c],
                |m| Out::U32(ss::string_push_back(m, s, c)),
            );
        }
    });
}
