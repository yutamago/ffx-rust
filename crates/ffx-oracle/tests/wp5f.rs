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
    clippy::similar_names,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
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

// ---- small functions (ffx_prender::wp5f) ----

use ffx_prender::wp5f::bridge as pr;

#[test]
fn wp5f_teardown_frees_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x006b_6180);
        for k in 0..16 {
            t.reset();
            t.stub(pr::TEXT_UTIL_ENTRY_DTOR, true, 0, 0, 0x41);
            t.stub(pr::OPERATOR_DELETE, false, 1, 0, 0x42);
            t.stub(pr::PHYRE_FREE, false, 1, 0, 0x43);
            t.stub(pr::INPUT_SHUTDOWN_DEVICE, false, 0, 0, 0x44);
            let present = k % 4 != 0;
            for (sym, s) in pr::SINGLETONS {
                let obj = if present { t.zeros(0x200) } else { 0 };
                t.g32(s.global, obj);
                t.check(&format!("{sym:#x} {k}"), sym, Conv::Cdecl, &[], |m| {
                    pr::free_singleton(m, s);
                    Out::None
                });
            }
            let obj = if present { t.zeros(0x40) } else { 0 };
            if present {
                t.w32(obj + 0x1c, rng.pick(&[0, 0x1234]));
                t.w32(obj + 0x20, rng.next());
                t.w32(obj + 0x24, rng.next());
            }
            t.g32(0x00cc_cad0, obj);
            t.check(
                &format!("0x006b60f0 {k}"),
                0x006b_60f0,
                Conv::Cdecl,
                &[],
                |m| {
                    pr::free_singleton_cccad0(m);
                    Out::None
                },
            );
            let obj = if present { t.zeros(0x10) } else { 0 };
            if present {
                t.fake_vtable(obj, &[(1, 0x45, vec![])]);
            }
            t.g32(0x00cc_cac0, obj);
            t.check(
                &format!("0x006b62d0 {k}"),
                0x006b_62d0,
                Conv::Cdecl,
                &[],
                |m| {
                    pr::free_object_cccac0(m);
                    Out::None
                },
            );
            t.g32(0x00cd_ede8, if present { 0x0055_0000 } else { 0 });
            t.check(
                &format!("0x006dc870 {k}"),
                0x006d_c870,
                Conv::Cdecl,
                &[],
                |m| {
                    let r = pr::free_block_cdede8(m);
                    if present { Out::U32(r) } else { Out::None }
                },
            );
            // the facet list
            let mut head = 0;
            for _ in 0..rng.below(4) {
                head = t.words(&[head, rng.next()]);
            }
            t.g32(pr::FACET_LIST, head);
            t.stub(pr::FAC_NODE_DTOR, true, 0, 0, 0x46);
            t.check(
                &format!("0x0094a641 {k}"),
                0x0094_a641,
                Conv::Cdecl,
                &[],
                |m| {
                    pr::facet_list_free(m);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn wp5f_small_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x005f_0950);
        for k in 0..60 {
            t.reset();
            let obj = t.zeros(8);
            t.check(
                &format!("0x0041e6a0 {k}"),
                0x0041_e6a0,
                Conv::Thiscall(obj),
                &[],
                |m| {
                    pr::mkv_reader_base_dtor(m, obj);
                    Out::None
                },
            );
            cdecl_imp(t, 0x00b0_c350, 0, 0x0077_0000);
            cdecl_imp(t, 0x00b0_c38c, 2, 0x33);
            t.check(
                &format!("SqsSePlayState {k}"),
                0x0081_e530,
                Conv::Cdecl,
                &[],
                |m| Out::U32(pr::sqs_se_play_state(m)),
            );
            let junk: Vec<u32> = (0..0x80).map(|_| rng.next()).collect();
            let arr = t.words(&junk);
            let n = rng.below(6);
            t.check(
                &format!("0x005f0950 {k}"),
                0x005f_0950,
                Conv::Cdecl,
                &[arr, n],
                |m| {
                    pr::clear_records(m, arr, n);
                    Out::None
                },
            );
            t.check(
                &format!("0x005f0950 null {k}"),
                0x005f_0950,
                Conv::Cdecl,
                &[0, 1],
                |m| {
                    pr::clear_records(m, 0, 1);
                    Out::None
                },
            );
            // floats (quiet NaNs and ordinary values; no signalling NaNs)
            t.stub(pr::SET_AFTER_POSITION, true, 2, 2, 0x47);
            let pick = |rng: &mut Rng| {
                rng.pick(&[0x3f80_0000u32, 0xc2c8_0000, 0x7fc0_0001, 0, 0x8000_0000])
            };
            let xyz = t.words(&[pick(&mut rng), pick(&mut rng), pick(&mut rng)]);
            let this = t.zeros(0x80);
            let (a, f) = (rng.next(), pick(&mut rng));
            t.check(
                &format!("0x006058b0 {k}"),
                0x0060_58b0,
                Conv::Thiscall(this),
                &[a, xyz, f],
                |m| Out::U32(pr::set_position_then(m, this, a, xyz, f)),
            );
        }
    });
}

#[test]
fn wp5f_vector_grow_and_strings_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x006b_6500);
        for k in 0..200 {
            t.reset();
            t.stub(pr::SECTION_VECTOR_REALLOCATE, true, 1, 1, 0x48);
            cdecl_imp(t, pr::XLENGTH_ERROR, 1, 0x49);
            let first = 0x0100_0000u32;
            let size = rng.pick(&[0u32, 1, 5, 12, 0x1555_5550]);
            let cap = size + rng.pick(&[0u32, 1, 4, 20]);
            let v = t.words(&[first, first + 12 * size, first + 12 * cap]);
            let n = rng.pick(&[0u32, 1, 3, 7, 30, 0x10]);
            t.check(
                &format!("vector grow {k} ({size}, {cap}, {n})"),
                0x006b_6500,
                Conv::Thiscall(v),
                &[n],
                |m| Out::U32(pr::section_vector_grow(m, v, n)),
            );
            // find_first_of over a small or a large string
            let text = b"hello, world; a=b\0";
            let s = t.zeros(0x18);
            let big = t.alloc(text);
            let len = rng.pick(&[0u32, 5, 17]);
            if rng.chance(2) {
                t.write(s, &text[..16]);
                t.w32(s + 0x14, 0xf);
            } else {
                t.w32(s, big);
                t.w32(s + 0x14, 0x1f);
            }
            t.w32(s + 0x10, len.min(if rng.chance(2) { 15 } else { 17 }));
            let set = t.alloc(rng.pick(&[&b",;"[..], b"=", b"xyz", b"o"]));
            let n = rng.pick(&[0u32, 1, 2]);
            let pos = rng.pick(&[0u32, 3, 6, 20]);
            // memchr: the real search, as a stub would not know the set
            let memchr = t.code(&[
                0x8b, 0x44, 0x24, 0x04, // mov eax, [esp + 4]
                0x8b, 0x4c, 0x24, 0x0c, // mov ecx, [esp + 0xc]
                0x8a, 0x54, 0x24, 0x08, // mov dl, [esp + 8]
                0x85, 0xc9, // l: test ecx, ecx
                0x74, 0x0a, // jz miss
                0x38, 0x10, // cmp [eax], dl
                0x74, 0x08, // je hit
                0x40, // inc eax
                0x49, // dec ecx
                0xeb, 0xf4, // jmp l
                0x90, 0x90, //
                0x33, 0xc0, // miss: xor eax, eax
                0xc3, // hit: ret
            ]);
            let at = t.rebase(pr::MEMCHR);
            t.patch(at, &memchr.to_le_bytes());
            t.check(
                &format!("find_first_of {k}"),
                0x006b_68c0,
                Conv::Thiscall(s),
                &[set, pos, n],
                |m| Out::U32(pr::find_first_of(m, s, set, pos, n)),
            );
            // the map name
            cdecl_imp(t, pr::SPRINTF_S, 3, 0x4a);
            t.stub(pr::MAP_CHANGE, false, 1, 0, 0x4b);
            let this = t.zeros(0x50);
            t.write(
                this + 0x30,
                rng.pick(&[&b"bsil0000\0"[..], b"\0", b"bsil00\0"]),
            );
            let name = t.alloc(rng.pick(&[&b"bsil0000\0"[..], b"\0", b"kami0100\0", b"bsil000\0"]));
            t.check(
                &format!("map name {k}"),
                0x006b_6730,
                Conv::Thiscall(this),
                &[name],
                |m| {
                    pr::set_map_name(m, this, name);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn wp5f_opus_tree_clear_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0080_7120);
        for k in 0..40 {
            t.reset();
            t.stub(pr::OPUS_FREE_SUBTREE, true, 1, 1, 0x4c);
            t.stub(pr::OPERATOR_DELETE, false, 1, 0, 0x4d);
            let head = t.zeros(0x10);
            t.w8(head + 0xd, 1);
            // a left spine of 0..4 nodes (left +0, parent +4, right +8)
            let mut next = head;
            for _ in 0..rng.below(5) {
                let node = t.words(&[next, 0, rng.next(), 0]);
                next = node;
            }
            t.w32(head + 4, next);
            let tree = t.words(&[head, rng.next()]);
            t.check(
                &format!("opus clear {k}"),
                0x0080_7120,
                Conv::Thiscall(tree),
                &[],
                |m| {
                    pr::opus_tree_clear(m, tree);
                    Out::None
                },
            );
        }
    });
}

/// `graphicGetTime` as a stub leaving `value` in `ST0` (`fld dword`).
fn time_stub(t: &mut Twins, value: f32) {
    let cell = t.words(&[value.to_bits()]);
    let mut c = vec![0xd9, 0x05];
    c.extend(cell.to_le_bytes()); // fld dword [cell]
    c.push(0xc3);
    let code = t.code(&c);
    t.jump(pr::GRAPHIC_GET_TIME, code);
}

#[test]
fn wp5f_skip_prompt_update_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x006e_8540);
        // (3e7 - 0.1 is not an f32: the stored difference rounds)
        // (1 + 2^-23 less 2^-30 rounds back to 1 + 2^-23 as an f32: equal
        // to a hold of 1 + 2^-23, below it before the rounding)
        let one_up = f32::from_bits(0x3f80_0001);
        let tiny = f32::from_bits(0x3080_0000);
        let times = [
            0.0f32,
            0.5,
            1.0,
            1.25,
            2.0,
            3.0,
            10.0,
            f32::NAN,
            3.0e7,
            one_up,
        ];
        for k in 0..800 {
            t.reset();
            let now = rng.pick(&times);
            time_stub(t, now);
            let p = t.zeros(0x20);
            let hold = rng.pick(&[0.0f32, 1.0, 2.0, 3.0, f32::NAN, one_up]);
            let length = rng.pick(&[0.5f32, 1.0, 2.0, 0.0, f32::NAN, 3.0]);
            t.w32(p, hold.to_bits());
            t.w32(p + 4, rng.pick(&[0.0f32, 0.5, 1.0, 0.1, tiny]).to_bits());
            t.w32(
                p + 8,
                rng.pick(&[0.0f32, 0.5, 1.0, 9.0, 0.1, tiny]).to_bits(),
            );
            t.w32(p + 0xc, length.to_bits());
            t.w32(p + 0x10, rng.next());
            let showing = rng.chance(2);
            let hiding = rng.chance(2);
            t.w8(p + 0x14, 1);
            t.w8(p + 0x15, u8::from(showing));
            t.w8(p + 0x16, u8::from(hiding));
            let what = format!(
                "skip prompt {k} (now {now}, hold {hold}, length {length}, {showing}/{hiding})"
            );
            t.check(&what, 0x006e_8540, Conv::Thiscall(p), &[], |m| {
                let r = pr::skip_prompt_update(m, p);
                // the neither case returns the caller's eax
                if showing || hiding {
                    Out::U8(r as u8)
                } else {
                    Out::None
                }
            });
        }
    });
}

#[test]
fn wp5f_records_maps_and_buffers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0051_30f0);
        for k in 0..120 {
            t.reset();
            // the named record copy
            let fresh = t.zeros(0x40);
            t.stub(pr::PHYRE_MALLOC, false, 2, 0, rng.pick(&[fresh, fresh, 0]));
            t.stub(pr::MEMCPY, false, 3, 0, 0x51);
            let name = t.alloc(rng.pick(&[&b"\0"[..], b"abc\0", b"a longer name\0"]));
            let tagged = match k % 3 {
                0 => 0,
                1 => name,
                _ => name + 1,
            };
            let junk: Vec<u32> = (0..7).map(|_| rng.next()).collect();
            let src = t.words(&junk);
            t.w32(src + 0x14, tagged);
            let dst = t.zeros(0x1c);
            t.check(
                &format!("record copy {k}"),
                0x0051_30f0,
                Conv::Thiscall(dst),
                &[src],
                |m| Out::U32(pr::copy_named_record(m, dst, src)),
            );
            // equal_range over a small tree: keys 10, 20, 20, 30
            let head = t.zeros(0x14);
            t.w8(head + 0xd, 1);
            let node = |t: &mut Twins, key: u32| -> u32 {
                let n = t.zeros(0x14);
                t.w32(n + 0x10, key);
                n
            };
            let (a, b, c, d) = (node(t, 10), node(t, 20), node(t, 20), node(t, 30));
            // b is the root: a < b, c and d to its right (c left of d)
            for (n, l, r, p) in [
                (b, a, d, head),
                (a, head, head, b),
                (d, c, head, b),
                (c, head, head, d),
            ] {
                t.w32(n, l);
                t.w32(n + 8, r);
                t.w32(n + 4, p);
            }
            let empty = rng.chance(6);
            t.w32(head + 4, if empty { head } else { b });
            t.w32(head, if empty { head } else { a });
            let map = t.words(&[head, 4]);
            let key = t.words(&[rng.pick(&[5, 10, 15, 20, 25, 30, 35])]);
            let out = t.zeros(8);
            t.check(
                &format!("equal_range {k}"),
                0x0062_58e0,
                Conv::Thiscall(map),
                &[out, key],
                |m| Out::U32(pr::equal_range_u32(m, map, out, key)),
            );
            // the cluster manager's teardown
            t.stub(pr::CLUSTER_MAP_ERASE, true, 3, 3, 0x52);
            t.stub(pr::OPERATOR_DELETE, false, 1, 0, 0x53);
            t.stub(pr::FREE_OWNED_ARRAY, false, 2, 0, 0x54);
            this_imp_std(t, pr::CLOSE_HANDLE, 1, 1);
            let ev = if rng.chance(2) { t.words(&[0x77]) } else { 0 };
            let mhead = t.words(&[0x1234, 0, 0]);
            let mgr = t.words(&[
                0,
                0,
                rng.pick(&[3, 0x8000_0003, 0, 0x4000_0003]),
                0x4444,
                mhead,
                0,
                ev,
            ]);
            t.check_framed(
                &format!("cluster shutdown {k}"),
                0x0069_9d60,
                Conv::Thiscall(mgr),
                &[],
                4,
                |m, ebp| {
                    pr::cluster_manager_shutdown(m, mgr, ebp - 4);
                    Out::None
                },
            );
            // AtlConvAllocMemory
            let fixed = t.zeros(0x40);
            let heap = t.zeros(0x40);
            let grown = t.zeros(0x80);
            let calloc_ret = rng.pick(&[grown, grown, 0]);
            let recalloc_ret = rng.pick(&[grown, grown, 0]);
            cdecl_imp(t, pr::CALLOC, 2, calloc_ret);
            cdecl_imp(t, pr::RECALLOC, 3, recalloc_ret);
            cdecl_imp(t, pr::FREE, 1, 0x55);
            let cur = rng.pick(&[fixed, heap]);
            let p = t.words(&[cur]);
            let n = rng.pick(&[1u32, 0x10, 0x20, 0x21]);
            let fixed_n = 0x20;
            // (AtlThrow does not return: the paths that reach it are not run)
            let fits = n <= fixed_n;
            if !fits && (cur == heap && recalloc_ret == 0 || cur == fixed && calloc_ret == 0) {
                continue;
            }
            t.check(
                &format!("conv alloc {k}"),
                0x0064_96d0,
                Conv::Cdecl,
                &[p, n, fixed, fixed_n],
                |m| {
                    pr::conv_alloc_memory(m, p, n, fixed, fixed_n);
                    Out::None
                },
            );
        }
    });
}

/// A stdcall import stub popping its `n` words.
fn this_imp_std(t: &mut Twins, slot: u32, n: u32, ret: u32) {
    imp(t, slot, false, n, n, &Ret::Fixed(ret), &[]);
}

// ---- mkvparser (ffx_prender::wp5f_mkv) ----

use ffx_prender::wp5f_mkv::bridge as mkv;

#[test]
fn wp5f_mkv_small_apis_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0042_3840);
        for k in 0..200 {
            t.reset();
            cdecl_imp(t, mkv::WASSERT, 3, 0x61);
            // Segment::GetFirst
            let first = rng.pick(&[0, 0x0044_0000]);
            let list = t.words(&[first, 7]);
            let seg = t.zeros(0x90);
            t.w32(seg + 0x80, rng.pick(&[0, list, list]));
            t.w32(seg + 0x84, rng.pick(&[0, 1, 2, u32::MAX]));
            t.check(
                &format!("Segment::GetFirst {k}"),
                0x0042_3840,
                Conv::Thiscall(seg),
                &[],
                |m| Out::U32(mkv::segment_get_first(m, seg)),
            );
            // Tracks::GetTrackByNumber
            let tr: Vec<u32> = (0..3)
                .map(|i| {
                    let tk = t.zeros(0x30);
                    t.w32(tk + 0x24, i + 1);
                    tk
                })
                .collect();
            let arr = t.words(&[tr[0], 0, tr[1], tr[2]]);
            let tracks = t.zeros(0x30);
            t.w32(tracks + 0x28, arr);
            t.w32(tracks + 0x2c, arr + 4 * rng.pick(&[0, 2, 4]));
            let n = rng.pick(&[0u32, 1, 2, 3, 4, u32::MAX]);
            t.check(
                &format!("GetTrackByNumber {k}"),
                0x0042_9da0,
                Conv::Thiscall(tracks),
                &[n],
                |m| Out::U32(mkv::tracks_get_track_by_number(m, tracks, n)),
            );
            // BlockEntry::EOS
            let e = t.zeros(0x10);
            t.fake_vtable(
                e,
                &[
                    (0, 0, vec![]),
                    (0, 0, vec![]),
                    (0, rng.pick(&[0, 1, 2]), vec![]),
                ],
            );
            t.check(
                &format!("BlockEntry::EOS {k}"),
                0x0042_d050,
                Conv::Thiscall(e),
                &[],
                |m| Out::U32(mkv::block_entry_eos(m, e)),
            );
            // Cluster::GetFirst / GetNext
            let count = rng.pick(&[0u32, 1, 3]);
            let entries: Vec<u32> = (0..4)
                .map(|i| {
                    let en = t.zeros(0x10);
                    t.w32(en + 8, i);
                    if rng.chance(8) { 0 } else { en }
                })
                .collect();
            let arr = t.words(&entries);
            let cl = t.zeros(0x40);
            t.w32(cl + 0x30, arr);
            t.w32(cl + 0x38, count);
            // Parse: a status, the entry count grown by one sometimes
            let status = rng.pick(&[0u32, 0, 1, u32::MAX - 2]);
            let grow = u8::from(rng.chance(2));
            let extra = vec![0x80, 0x41, 0x38, grow]; // add byte [ecx + 0x38], grow
            t.stub_ex(mkv::CLUSTER_PARSE, true, 2, 2, &Ret::Fixed(status), &extra);
            let out = t.words(&[0x5555]);
            t.skip = 0;
            t.check_framed(
                &format!("Cluster::GetFirst {k}"),
                0x0042_c880,
                Conv::Thiscall(cl),
                &[out],
                0x10,
                |m, ebp| Out::U32(mkv::cluster_get_first(m, cl, out, ebp - 0x10, ebp - 4)),
            );
            t.w32(cl + 0x38, count.max(1));
            let cur_i = rng.below(count.max(1));
            let cur = if rng.chance(10) {
                entries[3]
            } else {
                entries[cur_i as usize]
            };
            if cur == 0 {
                t.skip = 4;
                continue;
            }
            t.check_framed(
                &format!("Cluster::GetNext {k}"),
                0x0042_c9d0,
                Conv::Thiscall(cl),
                &[cur, out],
                8,
                |m, ebp| Out::U32(mkv::cluster_get_next(m, cl, cur, out, ebp - 8, ebp + 8)),
            );
            t.skip = 4;
        }
    });
}

/// The first `n` bytes of a game movie (`FFX_Data.vbf`).
fn movie_head(n: usize) -> Option<Vec<u8>> {
    let dir = std::path::PathBuf::from(std::env::var_os("FFX_GAME_DIR")?);
    let archive = ffx_vbf::Archive::open(dir.join("data/FFX_Data.vbf")).ok()?;
    let entry = archive.find("ffx_data/gamedata/ps3data/video/4_480.webm")?;
    let mut bytes = archive.read(entry).ok()?;
    bytes.truncate(n);
    Some(bytes)
}

/// An `IMkvReader` over `data` in the oracles: `Read(pos, len, buf)`
/// copies (status `err` past the data or for a high position word),
/// `Length(&total, &avail)` gives the data's size and `avail`. Layout:
/// vtable, data, size, err, avail.
fn fake_reader(t: &mut Twins, data: &[u8], err: u32, avail: u32) -> u32 {
    let read = t.code(&[
        0x8b, 0x44, 0x24, 0x04, // mov eax, [esp + 4]
        0x83, 0x7c, 0x24, 0x08, 0x00, // cmp dword [esp + 8], 0
        0x75, 0x23, // jne err
        0x8b, 0x54, 0x24, 0x0c, // mov edx, [esp + 0xc]
        0x03, 0xc2, // add eax, edx
        0x3b, 0x41, 0x08, // cmp eax, [ecx + 8]
        0x77, 0x18, // ja err
        0x56, 0x57, // push esi; push edi
        0x8b, 0x71, 0x04, // mov esi, [ecx + 4]
        0x03, 0x74, 0x24, 0x0c, // add esi, [esp + 0xc]
        0x8b, 0x7c, 0x24, 0x18, // mov edi, [esp + 0x18]
        0x8b, 0xca, // mov ecx, edx
        0xf3, 0xa4, // rep movsb
        0x5f, 0x5e, // pop edi; pop esi
        0x33, 0xc0, // xor eax, eax
        0xc2, 0x10, 0x00, // ret 0x10
        0x8b, 0x41, 0x0c, // err: mov eax, [ecx + 0xc]
        0xc2, 0x10, 0x00, // ret 0x10
    ]);
    let length = t.code(&[
        0x8b, 0x44, 0x24, 0x04, // mov eax, [esp + 4]
        0x8b, 0x51, 0x08, // mov edx, [ecx + 8]
        0x89, 0x10, // mov [eax], edx
        0xc7, 0x40, 0x04, 0, 0, 0, 0, // mov dword [eax + 4], 0
        0x8b, 0x44, 0x24, 0x08, // mov eax, [esp + 8]
        0x8b, 0x51, 0x10, // mov edx, [ecx + 0x10]
        0x89, 0x10, // mov [eax], edx
        0xc7, 0x40, 0x04, 0, 0, 0, 0, // mov dword [eax + 4], 0
        0x33, 0xc0, // xor eax, eax
        0xc2, 0x08, 0x00, // ret 8
    ]);
    let vt = t.words(&[read, length, 0]);
    let d = t.alloc(data);
    t.words(&[vt, d, data.len() as u32, err, avail])
}

#[test]
fn wp5f_mkv_ebml_readers_match_the_original() {
    let Some(data) = movie_head(0x4000) else {
        eprintln!("no movie; skipping");
        return;
    };
    Twins::each(|t| {
        let mut rng = Rng(0x0041_e710);
        for k in 0..400 {
            t.reset();
            cdecl_imp(t, mkv::WASSERT, 3, 0x61);
            let err = rng.pick(&[u32::MAX, 1, u32::MAX - 4]);
            let size = data.len() as u32;
            let avail = rng.pick(&[size, size, 0x100, 0, size + 4]);
            let reader = fake_reader(t, &data, err, avail);
            let pos = if rng.chance(10) {
                rng.pick(&[u64::from(size) - 1, u64::from(size), 1 << 32, u64::MAX])
            } else {
                u64::from(rng.below(0x3ff0))
            };
            let (lo, hi) = (pos as u32, (pos >> 32) as u32);
            let len = t.words(&[0x77]);
            t.skip = 0;
            t.check_framed(
                &format!("ReadUInt {k} ({pos:#x})"),
                0x0041_e710,
                Conv::FastcallCallerPops,
                &[reader, len, lo, hi],
                1,
                |m, ebp| Out::U64(mkv::read_uint(m, reader, len, pos, ebp - 1)),
            );
            t.skip = 8;
            t.check_framed(
                &format!("GetUIntLength {k} ({pos:#x}, avail {avail:#x})"),
                0x0041_e850,
                Conv::FastcallCallerPops,
                &[reader, len, lo, hi],
                0x18,
                |m, ebp| Out::U64(mkv::get_uint_length(m, reader, len, pos, ebp - 0x18)),
            );
            t.skip = 4;
            let n = rng.pick(&[0u64, 1, 2, 4, 8, 9, 1 << 32]);
            // the port's scratch for the byte: stack the diff skips (the
            // original uses its own argument slots)
            let scratch = t.frame_ebp(6) - 0x300;
            t.check(
                &format!("UnserializeUInt {k} ({pos:#x}, {n})"),
                0x0041_e980,
                Conv::FastcallCallerPops,
                &[reader, 0, lo, hi, n as u32, (n >> 32) as u32],
                |m| Out::U64(mkv::unserialize_uint(m, reader, pos, n, scratch)),
            );
            let n = rng.pick(&[1u32, 2, 3, 4, 8]);
            let result = t.words(&[0x1111, 0x2222]);
            t.check(
                &format!("UnserializeInt {k} ({pos:#x}, {n})"),
                0x0041_ec10,
                Conv::FastcallCallerPops,
                &[reader, n, lo, hi, result],
                |m| Out::U32(mkv::unserialize_int(m, reader, n, pos, result, scratch)),
            );
        }
    });
}

/// An EBML varint of `v` in `len` bytes.
fn varint(v: u64, len: usize) -> Vec<u8> {
    let marked = v | 1 << (7 * len);
    (0..len).rev().map(|i| (marked >> (8 * i)) as u8).collect()
}

/// A block payload: track 1, a time code, the lacing's header and the
/// frames (sizes from `sizes`); `bad` breaks one thing.
fn block_bytes(rng: &mut Rng, lacing: u8, sizes: &[usize], bad: u32) -> Vec<u8> {
    // (track 0 is a bad block; time codes at and past the short's ends)
    let mut b = varint(u64::from(rng.pick(&[1u32, 2, 0x7e, 0])), 1);
    let any = rng.next() as u16;
    let tc = rng.pick(&[any, 0x8000, 0x7fff, 0xffff]);
    b.extend(tc.to_be_bytes());
    b.push((lacing << 1) | rng.pick(&[0, 0x80, 0x08]));
    let n = sizes.len();
    match lacing {
        0 => {}
        1 => {
            b.push((n - 1) as u8);
            for &s in &sizes[..n - 1] {
                b.extend(std::iter::repeat_n(0xffu8, s / 255));
                b.push((s % 255) as u8);
            }
        }
        2 => b.push((n - 1) as u8),
        _ => {
            b.push((n - 1) as u8);
            b.extend(varint(sizes[0] as u64, rng.pick(&[1, 2, 3])));
            for w in sizes[..n - 1].windows(2) {
                let d = w[1] as i64 - w[0] as i64;
                let len = rng.pick(&[1usize, 2, 3]);
                let bias = (1i64 << (7 * len - 1)) - 1;
                b.extend(varint((d + bias) as u64, len));
            }
        }
    }
    for &s in sizes {
        b.extend((0..s).map(|_| rng.next() as u8));
    }
    match bad {
        1 => b[0] = 0,                // a 0 track varint byte
        2 => b.truncate(b.len() / 2), // short
        3 => {
            let at = rng.below(b.len() as u32) as usize;
            b[at] = rng.next() as u8; // one byte changed
        }
        _ => {}
    }
    b
}

#[test]
fn wp5f_mkv_block_parse_matches_the_original() {
    let head = movie_head(0x2000).unwrap_or_default();
    Twins::each(|t| {
        let mut rng = Rng(0x0042_d470);
        for k in 0..600 {
            t.reset();
            cdecl_imp(t, mkv::WASSERT, 3, 0x61);
            // the movie allocator: a bump allocator over a fresh area
            let area = t.zeros(0x2000);
            let cell = t.words(&[area]);
            let mut bump = vec![0xa1];
            bump.extend(cell.to_le_bytes()); // mov eax, [cell]
            bump.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp + 4]
            bump.extend([0x01, 0x0d]);
            bump.extend(cell.to_le_bytes()); // add [cell], ecx
            bump.push(0xc3);
            let bump = t.code(&bump);
            let slot = t.rebase(mkv::ALLOCATOR);
            t.patch(slot, &bump.to_le_bytes());
            let lacing = rng.pick(&[0u8, 1, 2, 3]);
            let n = if lacing == 0 {
                1
            } else {
                rng.pick(&[1usize, 2, 3, 5])
            };
            let fixed = rng.pick(&[0usize, 1, 7, 300]);
            let sizes: Vec<usize> = (0..n)
                .map(|_| {
                    if lacing == 2 {
                        fixed
                    } else {
                        rng.pick(&[0, 1, 7, 254, 255, 256, 600])
                    }
                })
                .collect();
            let bad = rng.pick(&[0u32, 0, 0, 1, 2, 3]);
            let (data, start, size) = if rng.chance(8) && !head.is_empty() {
                // real movie bytes from anywhere
                let s = rng.below(0x1f00);
                (head.clone(), u64::from(s), u64::from(rng.below(0x100)))
            } else {
                let mut d = vec![0u8; rng.below(8) as usize];
                let off = d.len() as u64;
                let b = block_bytes(&mut rng, lacing, &sizes, bad);
                let len = b.len() as u64;
                d.extend(b);
                // the block's size as stored, or a little off
                let size = match rng.below(8) {
                    0 => len.saturating_sub(1),
                    1 => len + 1,
                    _ => len,
                };
                (d, off, size)
            };
            let reader = fake_reader(t, &data, u32::MAX, data.len() as u32);
            let segment = t.words(&[reader]);
            let cluster = t.words(&[if rng.chance(30) { 0 } else { segment }]);
            let block = t.zeros(0x30);
            t.w32(block, start as u32);
            t.w32(block + 8, size as u32);
            let what =
                format!("Block::Parse {k} (lacing {lacing}, {n} frames, bad {bad}, size {size})");
            let arg = if rng.chance(40) { 0 } else { cluster };
            // the port's locals on the stack the diff skips
            let scratch = t.frame_ebp(1) - 0x400;
            t.check(&what, 0x0042_d470, Conv::Thiscall(block), &[arg], |m| {
                Out::U32(mkv::block_parse(m, block, arg, scratch))
            });
        }
    });
}

/// An EBML element: the id's bytes, a size varint and the payload.
fn element(id: &[u8], payload: &[u8], size_len: usize) -> Vec<u8> {
    let mut e = id.to_vec();
    e.extend(varint(payload.len() as u64, size_len));
    e.extend(payload);
    e
}

#[test]
fn wp5f_mkv_cluster_parse_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0042_a550);
        for k in 0..600 {
            t.reset();
            cdecl_imp(t, mkv::WASSERT, 3, 0x61);
            t.stub(
                mkv::CLUSTER_LOAD,
                true,
                2,
                2,
                rng.pick(&[0, 0, 0, 1, u32::MAX - 1, u32::MAX]),
            );
            t.stub(
                mkv::PARSE_BLOCK_GROUP,
                true,
                4,
                4,
                rng.pick(&[0, 1, u32::MAX - 2]),
            );
            t.stub(
                mkv::PARSE_SIMPLE_BLOCK,
                true,
                4,
                4,
                rng.pick(&[0, 1, u32::MAX - 2]),
            );
            // a cluster's body: a few elements
            let mut body = Vec::new();
            for _ in 0..rng.below(5) {
                let payload: Vec<u8> = (0..rng.pick(&[0u32, 1, 3, 40]))
                    .map(|_| rng.next() as u8)
                    .collect();
                let sl = rng.pick(&[1usize, 2, 8]);
                let e = match rng.below(9) {
                    0 => element(&[0xe7], &payload, sl),
                    1 => element(&[0xa3], &payload, sl),
                    2 => element(&[0xa0], &payload, sl),
                    3 => element(&[0xec], &payload, sl),
                    4 => element(&[0x1f, 0x43, 0xb6, 0x75], &payload, sl),
                    5 => element(&[0x1c, 0x53, 0xbb, 0x6b], &payload, sl),
                    6 => vec![0x80, 0x81, 0x00], // an id of 0
                    7 => vec![0xe7, 0xff],       // the unknown size
                    _ => vec![0x00],             // a bad varint byte
                };
                body.extend(e);
            }
            if rng.chance(4) && !body.is_empty() {
                let cut = rng.below(body.len() as u32) as usize;
                body.truncate(cut);
            }
            let lead = rng.below(6);
            let mut data = vec![0u8; lead as usize];
            data.extend(&body);
            let size = data.len() as u32;
            let avail = rng.pick(&[size, size, size.saturating_sub(3), size / 2]);
            let total = if rng.chance(5) { u32::MAX } else { size };
            let reader = fake_reader(t, &data, u32::MAX, avail);
            // Length's total: patch the reader's size word to total (the
            // reads still go by it)
            if total == u32::MAX {
                // a "total unknown" reader: Length reports -1
                let len_code = t.code(&[
                    0x8b, 0x44, 0x24, 0x04, // mov eax, [esp + 4]
                    0xc7, 0x00, 0xff, 0xff, 0xff, 0xff, // mov dword [eax], -1
                    0xc7, 0x40, 0x04, 0xff, 0xff, 0xff, 0xff, // mov dword [eax + 4], -1
                    0x8b, 0x44, 0x24, 0x08, // mov eax, [esp + 8]
                    0x8b, 0x51, 0x10, // mov edx, [ecx + 0x10]
                    0x89, 0x10, // mov [eax], edx
                    0xc7, 0x40, 0x04, 0, 0, 0, 0, // mov dword [eax + 4], 0
                    0x33, 0xc0, // xor eax, eax
                    0xc2, 0x08, 0x00, // ret 8
                ]);
                let vt = rd32(t, reader);
                t.w32(vt + 4, len_code);
            }
            let segment = t.words(&[reader]);
            // the cluster: segment, element start +8, m_pos +0x18, size
            // +0x20, timecode +0x28, entries +0x30, count +0x38
            let cl = t.zeros(0x40);
            t.w32(cl, segment);
            t.w32(cl + 8, rng.pick(&[0, lead]));
            t.w32(cl + 0x18, lead);
            let esize = match rng.below(4) {
                0 => u64::MAX,
                1 => u64::from(size - lead),
                2 => u64::from(size - lead) / 2,
                _ => 0,
            };
            t.w32(cl + 0x20, esize as u32);
            t.w32(cl + 0x24, (esize >> 32) as u32);
            t.w32(cl + 0x28, rng.pick(&[0, 5, u32::MAX]));
            let block = t.words(&[
                rng.pick(&[0, lead, size + 10]),
                0,
                rng.pick(&[0, 4, 100]),
                0,
            ]);
            let entry = t.zeros(0x10);
            t.fake_vtable(entry, &[(0, 0, vec![]), (0, block, vec![])]);
            let entries = t.words(&[entry, entry]);
            t.w32(cl + 0x30, entries);
            t.w32(cl + 0x38, rng.pick(&[0, 1, 2]));
            let pos = t.words(&[0x1111, 0x2222]);
            let len = t.words(&[0x3333]);
            let what =
                format!("Cluster::Parse {k} (body {body:02x?}, avail {avail}, size {esize:#x})");
            let scratch = t.frame_ebp(2) - 0x400;
            t.check(&what, 0x0042_a550, Conv::Thiscall(cl), &[pos, len], |m| {
                Out::U32(mkv::cluster_parse(m, cl, pos, len, scratch))
            });
        }
    });
}

fn rd32(t: &Twins, a: u32) -> u32 {
    Mem::read(&t.a, a)
}

#[test]
fn wp5f_mkv_cluster_load_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0042_9e10);
        for k in 0..600 {
            t.reset();
            cdecl_imp(t, mkv::WASSERT, 3, 0x61);
            // the children: a time code, blocks, others
            let mut body = Vec::new();
            for _ in 0..rng.below(5) {
                let payload: Vec<u8> = (0..rng.pick(&[0u32, 1, 2, 4, 9, 30]))
                    .map(|_| rng.next() as u8)
                    .collect();
                let sl = rng.pick(&[1usize, 2, 8]);
                let e = match rng.below(9) {
                    0 | 1 => element(&[0xe7], &payload, sl),
                    2 => element(&[0xa3], &payload, sl),
                    3 => element(&[0xa0], &payload, sl),
                    4 => element(&[0xec], &payload, sl),
                    5 => element(&[0x1f, 0x43, 0xb6, 0x75], &payload, sl),
                    6 => element(&[0x1c, 0x53, 0xbb, 0x6b], &payload, sl),
                    7 => vec![0x80, 0x81, 0x00],
                    _ => vec![0xe7, 0xff],
                };
                body.extend(e);
            }
            // the cluster element around it: its id (or another), a size
            let id: &[u8] = rng.pick(&[
                &[0x1f, 0x43, 0xb6, 0x75][..],
                &[0x1f, 0x43, 0xb6, 0x75],
                &[0xec],
                &[0x00],
            ]);
            let mut data = vec![0u8; rng.below(4) as usize];
            let start = data.len() as u32;
            data.extend(id);
            match rng.below(5) {
                0 => data.push(0xff), // unknown size
                1 => data.push(0x80), // size 0
                2 => data.extend(varint(body.len() as u64 / 2, 2)),
                _ => data.extend(varint(body.len() as u64, rng.pick(&[1, 4, 8]))),
            }
            data.extend(&body);
            if rng.chance(5) {
                let cut = rng.below(data.len() as u32 + 1) as usize;
                data.truncate(cut);
            }
            let size = data.len() as u32;
            // (an available count past the data makes the reads fail)
            let avail = rng.pick(&[size, size, size.saturating_sub(2), size + 4]);
            let reader = fake_reader(t, &data, rng.pick(&[u32::MAX, u32::MAX - 4]), avail);
            let segment = t.words(&[reader]);
            let cl = t.zeros(0x40);
            t.w32(cl, segment);
            t.w32(cl + 8, start);
            t.w32(cl + 0x18, start);
            t.w32(cl + 0x20, u32::MAX);
            t.w32(cl + 0x24, u32::MAX);
            // loaded already sometimes
            let tc = if rng.chance(6) {
                rng.pick(&[3u64, 0])
            } else {
                u64::MAX
            };
            t.w32(cl + 0x28, tc as u32);
            t.w32(cl + 0x2c, (tc >> 32) as u32);
            let pos = t.words(&[0x1111, 0x2222]);
            let len = t.words(&[0x3333]);
            let what = format!("Cluster::Load {k} ({data:02x?})");
            let scratch = t.frame_ebp(2) - 0x400;
            t.check(&what, 0x0042_9e10, Conv::Thiscall(cl), &[pos, len], |m| {
                Out::U32(mkv::cluster_load(m, cl, pos, len, scratch))
            });
        }
    });
}

/// The movie allocator and deallocator as a bump allocator and a logging
/// stub; the allocator returns 0 when `fail`.
fn movie_alloc(t: &mut Twins, fail: bool) {
    let area = t.zeros(0x6000);
    let cell = t.words(&[if fail { 0 } else { area }]);
    let mut bump = vec![0xa1];
    bump.extend(cell.to_le_bytes()); // mov eax, [cell]
    bump.extend([0x85, 0xc0, 0x74, 0x0a]); // test eax, eax; jz +10
    bump.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp + 4]
    bump.extend([0x01, 0x0d]);
    bump.extend(cell.to_le_bytes()); // add [cell], ecx
    bump.push(0xc3);
    let bump = t.code(&bump);
    let slot = t.rebase(mkv::ALLOCATOR);
    t.patch(slot, &bump.to_le_bytes());
    let free = t.stub_code(mkv::DEALLOCATOR, false, 1, 0, &Ret::Fixed(0x71), &[]);
    let slot = t.rebase(mkv::DEALLOCATOR);
    t.patch(slot, &free.to_le_bytes());
}

#[test]
fn wp5f_mkv_simple_blocks_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0042_aca0);
        for k in 0..500 {
            t.reset();
            cdecl_imp(t, mkv::WASSERT, 3, 0x61);
            movie_alloc(t, rng.chance(20));
            // ParseSimpleBlock over a synthetic block
            t.stub(
                mkv::CREATE_BLOCK,
                true,
                8,
                8,
                rng.pick(&[0, 0, 0, u32::MAX - 1]),
            );
            let lacing = rng.pick(&[0u8, 0, 1, 3]);
            let n = if lacing == 0 { 1 } else { 2 };
            let sizes = vec![rng.pick(&[0usize, 3, 20]); n];
            let bad = rng.pick(&[0u32, 0, 0, 1, 2, 3]);
            let mut data = vec![0u8; rng.below(4) as usize];
            let start = data.len() as u32;
            let b = block_bytes(&mut rng, lacing, &sizes, bad);
            let bsize = match rng.below(6) {
                0 => b.len().saturating_sub(2),
                1 => 3,
                _ => b.len(),
            } as u64;
            data.extend(b);
            let size = data.len() as u32;
            let avail = rng.pick(&[size, size, size.saturating_sub(3), start + 2]);
            let reader = fake_reader(t, &data, rng.pick(&[u32::MAX, u32::MAX - 4]), avail);
            let segment = t.words(&[reader]);
            let cl = t.zeros(0x40);
            t.w32(cl, segment);
            let pos = t.words(&[start, 0]);
            let len = t.words(&[0x3333]);
            let scratch = t.frame_ebp(4) - 0x400;
            let what = format!("ParseSimpleBlock {k} ({data:02x?}, size {bsize}, avail {avail})");
            t.check(
                &what,
                0x0042_aca0,
                Conv::Thiscall(cl),
                &[bsize as u32, 0, pos, len],
                |m| {
                    Out::U32(mkv::parse_simple_block(
                        m,
                        cl,
                        bsize.cast_signed(),
                        pos,
                        len,
                        scratch,
                    ))
                },
            );
            // CreateBlock over an entry array: none, room, full (its stub
            // above out of the way; an allocation that fails here faults
            // in the original)
            t.reset();
            cdecl_imp(t, mkv::WASSERT, 3, 0x61);
            movie_alloc(t, false);
            t.stub(mkv::CREATE_BLOCK_GROUP, true, 6, 6, 0x72);
            t.stub(mkv::CREATE_SIMPLE_BLOCK, true, 4, 4, 0x73);
            let cl = t.zeros(0x40);
            match rng.below(3) {
                0 => t.w32(cl + 0x38, u32::MAX),
                1 => {
                    let arr = t.words(&[1, 2, 3, 4]);
                    t.w32(cl + 0x30, arr);
                    t.w32(cl + 0x34, 4);
                    t.w32(cl + 0x38, rng.below(4));
                }
                _ => {
                    let arr = t.words(&[1, 2, 3]);
                    t.w32(cl + 0x30, arr);
                    t.w32(cl + 0x34, 3);
                    t.w32(cl + 0x38, 3);
                }
            }
            let id = rng.pick(&[0x20u32, 0x23, 0x23, 0x21]);
            let (s, z) = (rng.next(), rng.next());
            t.check(
                &format!("CreateBlock {k}"),
                0x0042_c130,
                Conv::Thiscall(cl),
                &[id, 0, s, 0, z, 0, 7, 0],
                |m| {
                    Out::U32(mkv::create_block(
                        m,
                        cl,
                        i64::from(id),
                        i64::from(s),
                        i64::from(z),
                        7,
                    ))
                },
            );
            t.reset();
            cdecl_imp(t, mkv::WASSERT, 3, 0x61);
            movie_alloc(t, rng.chance(20));
            // CreateSimpleBlock: the block parse stubbed (0 or an error);
            // the entry's deleting destructor a stub through the real
            // vtable's slot 0
            t.stub(mkv::BLOCK_PARSE, true, 1, 1, rng.pick(&[0, u32::MAX - 1]));
            let vt = t.rebase(mkv::SIMPLE_BLOCK_VTABLE);
            let del: u32 = Mem::read(&t.a, vt);
            // back to its ffx.sym address (the snapshots load elsewhere)
            let del = del - t.rebase(0x0040_0000) + 0x0040_0000;
            let stub = t.stub_code(del, true, 1, 1, &Ret::Fixed(0x74), &[]);
            t.jump(del, stub);
            let cl = t.zeros(0x40);
            let arr = t.words(&[9, 9, 9, 9]);
            t.w32(cl + 0x30, arr);
            t.w32(cl + 0x34, 4);
            t.w32(cl + 0x38, rng.below(4));
            let (s, z) = (rng.next(), rng.next());
            t.check(
                &format!("CreateSimpleBlock {k}"),
                0x0042_c740,
                Conv::Thiscall(cl),
                &[s, 0, z, 0],
                |m| Out::U32(mkv::create_simple_block(m, cl, i64::from(s), i64::from(z))),
            );
        }
    });
}

/// A `BlockGroup` payload: a `Block` (a synthetic block's bytes) and
/// optional `BlockDuration`, `ReferenceBlock`s, `DiscardPadding`, others.
fn group_bytes(rng: &mut Rng) -> Vec<u8> {
    let mut g = Vec::new();
    for _ in 0..=rng.below(5) {
        let sl = rng.pick(&[1usize, 2]);
        let e = match rng.below(8) {
            0..=2 => {
                let lacing = rng.pick(&[0u8, 0, 1]);
                let n = if lacing == 0 { 1 } else { 2 };
                let sizes = vec![rng.pick(&[0usize, 2, 9]); n];
                let bad = rng.pick(&[0, 0, 0, 3]);
                let b = block_bytes(rng, lacing, &sizes, bad);
                element(&[0xa1], &b, sl)
            }
            3 => {
                let n = rng.pick(&[1usize, 2, 8, 9]);
                let v: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
                element(&[0x9b], &v, sl)
            }
            4 => {
                let n = rng.pick(&[1usize, 2, 3]);
                let v: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
                element(&[0xfb], &v, sl)
            }
            5 => {
                let n = rng.pick(&[1usize, 2]);
                let v: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
                element(&[0x75, 0xa2], &v, sl)
            }
            6 => element(&[0xec], &[1, 2, 3], sl),
            _ => vec![0x80, 0x81, 0],
        };
        g.extend(e);
    }
    g
}

#[test]
fn wp5f_mkv_block_groups_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0042_af10);
        for k in 0..500 {
            t.reset();
            cdecl_imp(t, mkv::WASSERT, 3, 0x61);
            movie_alloc(t, false);
            // ParseBlockGroup over a synthetic group
            t.stub(
                mkv::CREATE_BLOCK,
                true,
                8,
                8,
                rng.pick(&[0, 0, u32::MAX - 1]),
            );
            let g = group_bytes(&mut rng);
            let mut data = vec![0u8; rng.below(4) as usize];
            let start = data.len() as u32;
            data.extend(&g);
            let gsize = match rng.below(6) {
                0 => g.len().saturating_sub(1),
                1 => g.len() + 2,
                _ => g.len(),
            } as u64;
            let size = data.len() as u32;
            let avail = rng.pick(&[size, size, size.saturating_sub(2), size + 3]);
            let reader = fake_reader(t, &data, u32::MAX, avail);
            let segment = t.words(&[reader]);
            let cl = t.zeros(0x40);
            t.w32(cl, segment);
            let pos = t.words(&[start, 0]);
            let len = t.words(&[0x3333]);
            let scratch = t.frame_ebp(4) - 0x400;
            let what = format!("ParseBlockGroup {k} ({data:02x?}, size {gsize}, avail {avail})");
            t.check(
                &what,
                0x0042_af10,
                Conv::Thiscall(cl),
                &[gsize as u32, 0, pos, len],
                |m| {
                    Out::U32(mkv::parse_block_group(
                        m,
                        cl,
                        gsize.cast_signed(),
                        pos,
                        len,
                        scratch,
                    ))
                },
            );
            // CreateBlockGroup over a well-formed group (it asserts, not
            // fails, on a bad one); the group's parse stubbed
            t.reset();
            cdecl_imp(t, mkv::WASSERT, 3, 0x61);
            movie_alloc(t, rng.chance(20));
            t.stub(
                mkv::BLOCK_GROUP_PARSE,
                true,
                0,
                0,
                rng.pick(&[0, 0, u32::MAX - 1]),
            );
            let vt = t.rebase(mkv::BLOCK_GROUP_VTABLE);
            let del: u32 = Mem::read(&t.a, vt);
            let del = del - t.rebase(0x0040_0000) + 0x0040_0000;
            let stub = t.stub_code(del, true, 1, 1, &Ret::Fixed(0x74), &[]);
            t.jump(del, stub);
            let mut g = Vec::new();
            let b = block_bytes(&mut rng, 0, &[5], 0);
            g.extend(element(&[0xa1], &b, 1));
            if rng.chance(2) {
                g.extend(element(&[0x9b], &[0, 7], 1));
            }
            for _ in 0..rng.below(3) {
                let v = rng.pick(&[&[0x00u8][..], &[0xff], &[0x00, 0x10], &[0xff, 0xf0]]);
                g.extend(element(&[0xfb], v, 1));
            }
            if rng.chance(3) {
                g.extend(element(&[0xec], &[9], 1));
            }
            let data = g.clone();
            let reader = fake_reader(t, &data, u32::MAX, data.len() as u32);
            let segment = t.words(&[reader]);
            let cl = t.zeros(0x40);
            t.w32(cl, segment);
            let arr = t.words(&[9, 9, 9, 9]);
            t.w32(cl + 0x30, arr);
            t.w32(cl + 0x34, 4);
            t.w32(cl + 0x38, rng.below(4));
            let gsize = data.len() as u32;
            let scratch = t.frame_ebp(6) - 0x400;
            let what = format!("CreateBlockGroup {k} ({data:02x?})");
            t.check(
                &what,
                0x0042_c300,
                Conv::Thiscall(cl),
                &[0, 0, gsize, 0, 3, 0],
                |m| {
                    Out::U32(mkv::create_block_group(
                        m,
                        cl,
                        0,
                        i64::from(gsize),
                        3,
                        scratch,
                    ))
                },
            );
        }
    });
}
