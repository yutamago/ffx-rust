//! `MSVCP110`'s functions `FFX.exe` imports, in Rust (docs/host.md, "The C
//! runtime DLLs", m5-wp5c), over MSVCP110's own object layouts (the
//! player's DLL 11.0.51106.1, from its machine code):
//!
//! - `basic_streambuf<char>`: the vtable, then the six pointer cells'
//!   addresses (`_IGfirst` `+0xc`, `_IPfirst` `+0x10`, `_IGnext` `+0x1c`,
//!   `_IPnext` `+0x20`, `_IGcount` `+0x2c`, `_IPcount` `+0x30`): the
//!   accessors read through them; `sputc`, `sgetc`, `snextc`, `sputn` call
//!   the stream's virtuals (`overflow` `+0xc`, `underflow` `+0x18`, `uflow`
//!   `+0x1c`, `xsputn` `+0x24`) when the buffer can't serve them;
//! - `ios_base`/`basic_ios<char>`: `_Mystate` `+0xc`, `_Except` `+0x10`,
//!   `_Fmtfl` `+0x14`, `_Wide` `+0x20` (64 bits), `_Mystrbuf` `+0x38`,
//!   `_Tiestr` `+0x3c`, `_Fillch` `+0x40`.
//!
//! The accessors are kept as Rust sides with the DLLs too
//! (`tests/crt_dll.rs` compares them with the DLL's). The rest are
//! no-DLL sides: the `_X*` throw helpers stop the thread with the
//! exception the DLL would throw (no handler runs without the emulator),
//! the locks are the run's no-ops, and the data imports are objects in
//! the arena (`std::cin` a stream at its end of input, the vtables of
//! the stream classes the game constructs inline whose entries stop the
//! thread: their only callers are the `Settings_*` streams, which are
//! ported). `FFX.exe`'s callers of the stream constructors
//! (`Settings_Format` and `Settings_JoinBindings`' `std::stringstream`)
//! are ported (`ffx_system`), so those constructors are not reached; their
//! Rust sides stop the thread naming them.

#![allow(clippy::many_single_char_names)] // the C names

use ffx_bridge::Mem;

use crate::crt_dll::dll_export;
use crate::exec::{Exec, Stop, Trap};
use crate::imports::ImportFn;
use crate::table::{Args, Conv, Ret};

const CPP: &str = "MSVCP110.dll";

const IGFIRST: u32 = 0xc;
const IPFIRST: u32 = 0x10;
const IGNEXT: u32 = 0x1c;
const IPNEXT: u32 = 0x20;
const IGCOUNT: u32 = 0x2c;
const IPCOUNT: u32 = 0x30;

const MYSTATE: u32 = 0xc;
const EXCEPT: u32 = 0x10;
const FMTFL: u32 = 0x14;
const WIDE: u32 = 0x20;
const MYSTRBUF: u32 = 0x38;
const TIESTR: u32 = 0x3c;
const FILLCH: u32 = 0x40;

/// `streambuf` virtuals' vtable offsets.
const OVERFLOW: u32 = 0xc;
const UNDERFLOW: u32 = 0x18;
const UFLOW: u32 = 0x1c;
const XSPUTN: u32 = 0x24;

macro_rules! sb {
    ($m:literal, $sig:literal) => {
        concat!(
            "?",
            $m,
            "@?$basic_streambuf@DU?$char_traits@D@std@@@std@@",
            $sig
        )
    };
}
macro_rules! ios {
    ($m:literal, $sig:literal) => {
        concat!("?", $m, "@?$basic_ios@DU?$char_traits@D@std@@@std@@", $sig)
    };
}

/// Kept with the DLLs.
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (CPP, sb!("eback", "IBEPADXZ"), Conv::Thiscall, 0, |e, a| {
        get(e, a, IGFIRST, None)
    }),
    (CPP, sb!("gptr", "IBEPADXZ"), Conv::Thiscall, 0, |e, a| {
        get(e, a, IGNEXT, None)
    }),
    (CPP, sb!("egptr", "IBEPADXZ"), Conv::Thiscall, 0, |e, a| {
        get(e, a, IGNEXT, Some(IGCOUNT))
    }),
    (CPP, sb!("pbase", "IBEPADXZ"), Conv::Thiscall, 0, |e, a| {
        get(e, a, IPFIRST, None)
    }),
    (CPP, sb!("pptr", "IBEPADXZ"), Conv::Thiscall, 0, |e, a| {
        get(e, a, IPNEXT, None)
    }),
    (CPP, sb!("epptr", "IBEPADXZ"), Conv::Thiscall, 0, |e, a| {
        get(e, a, IPNEXT, Some(IPCOUNT))
    }),
    (CPP, sb!("setg", "IAEXPAD00@Z"), Conv::Thiscall, 3, setg),
    (CPP, sb!("setp", "IAEXPAD0@Z"), Conv::Thiscall, 2, setp2),
    (CPP, sb!("setp", "IAEXPAD00@Z"), Conv::Thiscall, 3, setp3),
    (CPP, sb!("pbump", "IAEXH@Z"), Conv::Thiscall, 1, |e, a| {
        bump(e, a, IPNEXT, IPCOUNT)
    }),
    (CPP, sb!("gbump", "IAEXH@Z"), Conv::Thiscall, 1, |e, a| {
        bump(e, a, IGNEXT, IGCOUNT)
    }),
    (CPP, sb!("_Pninc", "IAEPADXZ"), Conv::Thiscall, 0, pninc),
    (CPP, sb!("sputc", "QAEHD@Z"), Conv::Thiscall, 1, sputc),
    (CPP, sb!("sgetc", "QAEHXZ"), Conv::Thiscall, 0, sgetc),
    (CPP, sb!("snextc", "QAEHXZ"), Conv::Thiscall, 0, snextc),
    (CPP, sb!("sputn", "QAE_JPBD_J@Z"), Conv::Thiscall, 3, sputn),
    (
        CPP,
        "?rdstate@ios_base@std@@QBEHXZ",
        Conv::Thiscall,
        0,
        |e, a| field(e, a, MYSTATE),
    ),
    (CPP, "?good@ios_base@std@@QBE_NXZ", Conv::Thiscall, 0, good),
    (CPP, "?eof@ios_base@std@@QBE_NXZ", Conv::Thiscall, 0, eof),
    (
        CPP,
        "?flags@ios_base@std@@QBEHXZ",
        Conv::Thiscall,
        0,
        |e, a| field(e, a, FMTFL),
    ),
    (
        CPP,
        "?width@ios_base@std@@QBE_JXZ",
        Conv::Thiscall,
        0,
        width,
    ),
    (
        CPP,
        "?width@ios_base@std@@QAE_J_J@Z",
        Conv::Thiscall,
        2,
        set_width,
    ),
    (CPP, ios!("fill", "QBEDXZ"), Conv::Thiscall, 0, fill),
    (
        CPP,
        ios!("tie", "QBEPAV?$basic_ostream@DU?$char_traits@D@std@@@2@XZ"),
        Conv::Thiscall,
        0,
        |e, a| field(e, a, TIESTR),
    ),
    (
        CPP,
        ios!(
            "rdbuf",
            "QBEPAV?$basic_streambuf@DU?$char_traits@D@std@@@2@XZ"
        ),
        Conv::Thiscall,
        0,
        |e, a| field(e, a, MYSTRBUF),
    ),
    (
        CPP,
        ios!("setstate", "QAEXH_N@Z"),
        Conv::Thiscall,
        2,
        setstate,
    ),
    (
        CPP,
        "?_Orphan_all@_Container_base0@std@@QAEXXZ",
        Conv::Thiscall,
        0,
        |_, _| Ret::VOID,
    ),
    (CPP, "??0id@locale@std@@QAE@I@Z", Conv::Thiscall, 1, id_new),
];

/// The streams' classes, for the stops.
const STREAMS: [&str; 5] = [
    "??0?$basic_iostream@DU?$char_traits@D@std@@@std@@QAE@PAV?$basic_streambuf@DU?$char_traits@D@std@@@1@@Z",
    "??0?$basic_istream@DU?$char_traits@D@std@@@std@@QAE@PAV?$basic_streambuf@DU?$char_traits@D@std@@@1@_N@Z",
    "??0?$basic_ostream@DU?$char_traits@D@std@@@std@@QAE@PAV?$basic_streambuf@DU?$char_traits@D@std@@@1@_N@Z",
    "??0?$basic_ios@DU?$char_traits@D@std@@@std@@IAE@XZ",
    "??0?$basic_streambuf@DU?$char_traits@D@std@@@std@@IAE@XZ",
];

/// Without the DLLs.
pub const NO_DLL: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (CPP, "?_Xbad_alloc@std@@YAXXZ", Conv::Cdecl, 0, |e, _| {
        throw(e, "std::bad_alloc", 0)
    }),
    (
        CPP,
        "?_Xlength_error@std@@YAXPBD@Z",
        Conv::Cdecl,
        1,
        |e, a| throw(e, "std::length_error", arg(a, 0)),
    ),
    (
        CPP,
        "?_Xout_of_range@std@@YAXPBD@Z",
        Conv::Cdecl,
        1,
        |e, a| throw(e, "std::out_of_range", arg(a, 0)),
    ),
    (
        CPP,
        "?_Xruntime_error@std@@YAXPBD@Z",
        Conv::Cdecl,
        1,
        |e, a| throw(e, "std::runtime_error", arg(a, 0)),
    ),
    (CPP, "??0_Lockit@std@@QAE@H@Z", Conv::Thiscall, 1, lockit),
    (CPP, "??1_Lockit@std@@QAE@XZ", Conv::Thiscall, 0, |_, _| {
        Ret::VOID
    }),
    (
        CPP,
        "?uncaught_exception@std@@YA_NXZ",
        Conv::Cdecl,
        0,
        |_, _| Ret::eax(0),
    ),
    (CPP, "??Bid@locale@std@@QAEIXZ", Conv::Thiscall, 0, id_value),
    (
        CPP,
        "?_Getgloballocale@locale@std@@CAPAV_Locimp@12@XZ",
        Conv::Cdecl,
        0,
        |e, _| stops(e, "locale::_Getgloballocale"),
    ),
    (
        CPP,
        "?getloc@ios_base@std@@QBE?AVlocale@2@XZ",
        Conv::Thiscall,
        1,
        |e, _| stops(e, "ios_base::getloc"),
    ),
    (
        CPP,
        "?_Getcat@?$ctype@D@std@@SAIPAPBVfacet@locale@2@PBV42@@Z",
        Conv::Cdecl,
        2,
        |e, _| stops(e, "ctype<char>::_Getcat"),
    ),
    (
        CPP,
        "?is@?$ctype@D@std@@QBE_NFD@Z",
        Conv::Thiscall,
        2,
        ctype_is,
    ),
    (
        CPP,
        "?_Ipfx@?$basic_istream@DU?$char_traits@D@std@@@std@@QAE_N_N@Z",
        Conv::Thiscall,
        1,
        |e, _| stops(e, "basic_istream::_Ipfx"),
    ),
    (
        CPP,
        "?_Osfx@?$basic_ostream@DU?$char_traits@D@std@@@std@@QAEXXZ",
        Conv::Thiscall,
        0,
        |e, _| stops(e, "basic_ostream::_Osfx"),
    ),
    (
        CPP,
        "?flush@?$basic_ostream@DU?$char_traits@D@std@@@std@@QAEAAV12@XZ",
        Conv::Thiscall,
        0,
        |e, _| stops(e, "basic_ostream::flush"),
    ),
    (
        CPP,
        "??6?$basic_ostream@DU?$char_traits@D@std@@@std@@QAEAAV01@H@Z",
        Conv::Thiscall,
        1,
        |e, _| stops(e, "basic_ostream::operator<<(int)"),
    ),
    (
        CPP,
        "?_Syserror_map@std@@YAPBDH@Z",
        Conv::Cdecl,
        1,
        |e, a| error_map(e, a, false),
    ),
    (
        CPP,
        "?_Winerror_map@std@@YAPBDH@Z",
        Conv::Cdecl,
        1,
        |e, a| error_map(e, a, true),
    ),
    (CPP, STREAMS[0], Conv::Thiscall, 2, |e, _| {
        stops(e, "basic_iostream::basic_iostream")
    }),
    (CPP, STREAMS[1], Conv::Thiscall, 3, |e, _| {
        stops(e, "basic_istream::basic_istream")
    }),
    (CPP, STREAMS[2], Conv::Thiscall, 3, |e, _| {
        stops(e, "basic_ostream::basic_ostream")
    }),
    (CPP, STREAMS[3], Conv::Thiscall, 0, |e, _| {
        stops(e, "basic_ios::basic_ios")
    }),
    (CPP, STREAMS[4], Conv::Thiscall, 0, |e, _| {
        stops(e, "basic_streambuf::basic_streambuf")
    }),
    (
        CPP,
        "??1?$basic_iostream@DU?$char_traits@D@std@@@std@@UAE@XZ",
        Conv::Thiscall,
        0,
        |e, _| stops(e, "basic_iostream::~basic_iostream"),
    ),
    (
        CPP,
        "??1?$basic_istream@DU?$char_traits@D@std@@@std@@UAE@XZ",
        Conv::Thiscall,
        0,
        |e, _| stops(e, "basic_istream::~basic_istream"),
    ),
    (
        CPP,
        "??1?$basic_ostream@DU?$char_traits@D@std@@@std@@UAE@XZ",
        Conv::Thiscall,
        0,
        |e, _| stops(e, "basic_ostream::~basic_ostream"),
    ),
    (
        CPP,
        "??1?$basic_ios@DU?$char_traits@D@std@@@std@@UAE@XZ",
        Conv::Thiscall,
        0,
        |e, _| stops(e, "basic_ios::~basic_ios"),
    ),
    (
        CPP,
        "??1?$basic_streambuf@DU?$char_traits@D@std@@@std@@UAE@XZ",
        Conv::Thiscall,
        0,
        |e, _| stops(e, "basic_streambuf::~basic_streambuf"),
    ),
    (
        CPP,
        "?_Add_vtordisp1@?$basic_istream@DU?$char_traits@D@std@@@std@@UAEXXZ",
        Conv::Thiscall,
        0,
        |_, _| Ret::VOID,
    ),
    (
        CPP,
        "?_Add_vtordisp2@?$basic_ostream@DU?$char_traits@D@std@@@std@@UAEXXZ",
        Conv::Thiscall,
        0,
        |_, _| Ret::VOID,
    ),
    (
        CPP,
        ios!("_Add_vtordisp1", "UAEXXZ"),
        Conv::Thiscall,
        0,
        |_, _| Ret::VOID,
    ),
    (
        CPP,
        ios!("_Add_vtordisp2", "UAEXXZ"),
        Conv::Thiscall,
        0,
        |_, _| Ret::VOID,
    ),
    (
        CPP,
        sb!("xsputn", "MAE_JPBD_J@Z"),
        Conv::Thiscall,
        3,
        xsputn,
    ),
    (
        CPP,
        sb!("xsgetn", "MAE_JPAD_J@Z"),
        Conv::Thiscall,
        3,
        xsgetn,
    ),
    (CPP, sb!("uflow", "MAEHXZ"), Conv::Thiscall, 0, uflow),
    (
        CPP,
        sb!("showmanyc", "MAE_JXZ"),
        Conv::Thiscall,
        0,
        |_, _| Ret {
            edx: Some(0),
            ..Ret::eax(0)
        },
    ),
    (
        CPP,
        sb!("imbue", "MAEXABVlocale@2@@Z"),
        Conv::Thiscall,
        1,
        |_, _| Ret::VOID,
    ),
    (CPP, sb!("sync", "MAEHXZ"), Conv::Thiscall, 0, |_, _| {
        Ret::eax(0)
    }),
    (
        CPP,
        sb!("setbuf", "MAEPAV12@PAD_J@Z"),
        Conv::Thiscall,
        3,
        |_, a| Ret::eax(a.ecx),
    ),
    (CPP, sb!("_Lock", "UAEXXZ"), Conv::Thiscall, 0, |_, _| {
        Ret::VOID
    }),
    (CPP, sb!("_Unlock", "UAEXXZ"), Conv::Thiscall, 0, |_, _| {
        Ret::VOID
    }),
];

fn arg(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

/// The cell at `this + at`'s address field, read through.
fn cell(mem: &impl Mem, this: u32, at: u32) -> u32 {
    let p: u32 = mem.read(this + at);
    mem.read(p)
}

fn set_cell(mem: &mut impl Mem, this: u32, at: u32, v: u32) {
    let p: u32 = mem.read(this + at);
    mem.write(p, v);
}

/// `eback`/`gptr`/`pbase`/`pptr` (`*cell`), `egptr`/`epptr` (`*next +
/// *count`).
fn get(e: &mut Exec, args: &Args, at: u32, count: Option<u32>) -> Ret {
    let mem = e.mem();
    let v = cell(&mem, args.ecx, at);
    Ret::eax(count.map_or(v, |c| v.wrapping_add(cell(&mem, args.ecx, c))))
}

/// The `eax` the DLL's void setters leave: the last cell's address they
/// loaded (callers don't read it; kept so the registers agree).
fn left(mem: &impl Mem, this: u32, at: u32) -> Ret {
    Ret::eax(mem.read(this + at))
}

/// `setg(first, next, last)`.
fn setg(e: &mut Exec, args: &Args) -> Ret {
    let (first, next, last) = (arg(args, 0), arg(args, 1), arg(args, 2));
    let mut mem = e.mem();
    set_cell(&mut mem, args.ecx, IGFIRST, first);
    set_cell(&mut mem, args.ecx, IGNEXT, next);
    set_cell(&mut mem, args.ecx, IGCOUNT, last.wrapping_sub(next));
    left(&mem, args.ecx, IGCOUNT)
}

/// `setp(first, last)`.
fn setp2(e: &mut Exec, args: &Args) -> Ret {
    let (first, last) = (arg(args, 0), arg(args, 1));
    let mut mem = e.mem();
    set_cell(&mut mem, args.ecx, IPFIRST, first);
    set_cell(&mut mem, args.ecx, IPNEXT, first);
    set_cell(&mut mem, args.ecx, IPCOUNT, last.wrapping_sub(first));
    left(&mem, args.ecx, IPCOUNT)
}

/// `setp(first, next, last)`.
fn setp3(e: &mut Exec, args: &Args) -> Ret {
    let (first, next, last) = (arg(args, 0), arg(args, 1), arg(args, 2));
    let mut mem = e.mem();
    set_cell(&mut mem, args.ecx, IPFIRST, first);
    set_cell(&mut mem, args.ecx, IPNEXT, next);
    set_cell(&mut mem, args.ecx, IPCOUNT, last.wrapping_sub(next));
    left(&mem, args.ecx, IPCOUNT)
}

/// `pbump(n)`/`gbump(n)`: `*count -= n`, `*next += n`.
fn bump(e: &mut Exec, args: &Args, next: u32, count: u32) -> Ret {
    let n = arg(args, 0);
    let mut mem = e.mem();
    let c = cell(&mem, args.ecx, count);
    set_cell(&mut mem, args.ecx, count, c.wrapping_sub(n));
    let p = cell(&mem, args.ecx, next);
    set_cell(&mut mem, args.ecx, next, p.wrapping_add(n));
    left(&mem, args.ecx, next)
}

/// `_Pninc()`: `--*_IPcount`, `(*_IPnext)++`'s old value.
fn pninc(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let c = cell(&mem, args.ecx, IPCOUNT);
    set_cell(&mut mem, args.ecx, IPCOUNT, c.wrapping_sub(1));
    let p = cell(&mem, args.ecx, IPNEXT);
    set_cell(&mut mem, args.ecx, IPNEXT, p.wrapping_add(1));
    Ret::eax(p)
}

/// The count a buffer pointer has (0 without a buffer).
fn avail(mem: &impl Mem, this: u32, next: u32, count: u32) -> i32 {
    if cell(mem, this, next) == 0 {
        0
    } else {
        cell(mem, this, count).cast_signed()
    }
}

/// Calls the stream's virtual at vtable offset `slot` with `words`.
fn virtual_call(e: &mut Exec, this: u32, slot: u32, words: Vec<u32>) -> Ret {
    let f: u32 = {
        let mem = e.mem();
        let vt: u32 = mem.read(this);
        mem.read(vt + slot)
    };
    let a = Args {
        ecx: this,
        stack: words,
        ..Args::default()
    };
    e.call(f, Conv::Thiscall, &a).unwrap_or(Ret::VOID)
}

/// `sputc(c)`: into the put area when it has room, else `overflow(c)`;
/// `(unsigned char) c` or `overflow`'s result.
fn sputc(e: &mut Exec, args: &Args) -> Ret {
    let this = args.ecx;
    #[allow(clippy::cast_possible_truncation)] // the char
    let c = arg(args, 0) as u8;
    let room = avail(&e.mem(), this, IPNEXT, IPCOUNT) > 0;
    if !room {
        return virtual_call(e, this, OVERFLOW, vec![u32::from(c)]);
    }
    let p = pninc(e, args).eax;
    e.mem().write(p, c);
    Ret::eax(u32::from(c))
}

/// `sgetc()`: the next character when the get area has one, else
/// `underflow()`.
fn sgetc(e: &mut Exec, args: &Args) -> Ret {
    let this = args.ecx;
    let mem = e.mem();
    if avail(&mem, this, IGNEXT, IGCOUNT) > 0 {
        let p = cell(&mem, this, IGNEXT);
        return Ret::eax(u32::from(mem.read::<u8>(p)));
    }
    drop(mem);
    virtual_call(e, this, UNDERFLOW, Vec::new())
}

/// `snextc()`: with two or more characters in the get area, past one and
/// the next; else `sbumpc()` (one character, or `uflow()`) and, unless it
/// gave `EOF`, `sgetc()`.
fn snextc(e: &mut Exec, args: &Args) -> Ret {
    let this = args.ecx;
    let n = avail(&e.mem(), this, IGNEXT, IGCOUNT);
    if n > 1 {
        let mut mem = e.mem();
        let c = cell(&mem, this, IGCOUNT);
        set_cell(&mut mem, this, IGCOUNT, c.wrapping_sub(1));
        let p = cell(&mem, this, IGNEXT).wrapping_add(1);
        set_cell(&mut mem, this, IGNEXT, p);
        return Ret::eax(u32::from(mem.read::<u8>(p)));
    }
    let bumped = if n > 0 {
        let mut mem = e.mem();
        let c = cell(&mem, this, IGCOUNT);
        set_cell(&mut mem, this, IGCOUNT, c.wrapping_sub(1));
        let p = cell(&mem, this, IGNEXT);
        set_cell(&mut mem, this, IGNEXT, p.wrapping_add(1));
        u32::from(mem.read::<u8>(p))
    } else {
        virtual_call(e, this, UFLOW, Vec::new()).eax
    };
    if bumped == u32::MAX {
        return Ret::eax(u32::MAX);
    }
    sgetc(e, args)
}

/// `sputn(p, n)` (64-bit `n`): `xsputn(p, n)`.
fn sputn(e: &mut Exec, args: &Args) -> Ret {
    let words = vec![arg(args, 0), arg(args, 1), arg(args, 2)];
    virtual_call(e, args.ecx, XSPUTN, words)
}

fn field(e: &mut Exec, args: &Args, at: u32) -> Ret {
    Ret::eax(e.mem().read(args.ecx + at))
}

fn good(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(u32::from(e.mem().read::<u32>(args.ecx + MYSTATE) == 0))
}

fn eof(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(e.mem().read::<u32>(args.ecx + MYSTATE) & 1)
}

fn width(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    Ret {
        edx: Some(mem.read(args.ecx + WIDE + 4)),
        ..Ret::eax(mem.read(args.ecx + WIDE))
    }
}

/// `width(n)`: the old width back, `n` set.
fn set_width(e: &mut Exec, args: &Args) -> Ret {
    let old = width(e, args);
    let mut mem = e.mem();
    mem.write(args.ecx + WIDE, arg(args, 0));
    mem.write(args.ecx + WIDE + 4, arg(args, 1));
    old
}

/// `fill()`: the fill character in `al` (the rest of `eax` as the DLL's
/// `mov al` leaves it: here 0).
fn fill(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(u32::from(e.mem().read::<u8>(args.ecx + FILLCH)))
}

/// `setstate(state, reraise)`: for a non-zero `state`, `clear(rdstate |
/// state | badbit without a buffer, reraise)`: `_Mystate` set to it `&
/// 0x17`; a state the exception mask throws for: the DLL (it throws
/// `ios_base::failure`), or the stop.
fn setstate(e: &mut Exec, args: &Args) -> Ret {
    let this = args.ecx;
    let state = arg(args, 0);
    if state == 0 {
        return Ret::VOID;
    }
    let (now, except, buf): (u32, u32, u32) = {
        let mem = e.mem();
        (
            mem.read(this + MYSTATE),
            mem.read(this + EXCEPT),
            mem.read(this + MYSTRBUF),
        )
    };
    let next = (now | state | if buf == 0 { 4 } else { 0 }) & 0x17;
    if except & next != 0 {
        if e.shared().dll(CPP).is_some() {
            return dll_export(e, CPP, ios!("setstate", "QAEXH_N@Z"), Conv::Thiscall, args);
        }
        return throw(e, "std::ios_base::failure", 0);
    }
    e.mem().write(this + MYSTATE, next);
    // `clear`'s `eax`: the state set
    Ret::eax(next)
}

/// `locale::id(n)`: `this->_Id = n`; `this`.
fn id_new(e: &mut Exec, args: &Args) -> Ret {
    e.mem().write(args.ecx, arg(args, 0));
    Ret::eax(args.ecx)
}

/// `locale::id::operator size_t()`: the id, given out from the run's
/// counter (`_Id_cnt`) on first use.
fn id_value(e: &mut Exec, args: &Args) -> Ret {
    let this = args.ecx;
    let v: u32 = e.mem().read(this);
    if v != 0 {
        return Ret::eax(v);
    }
    let next = crate::crt_os::cell(e, "locale::id count").unwrap_or(0) + 1;
    crate::crt_os::set_cell(e, "locale::id count", next);
    e.mem().write(this, next);
    Ret::eax(next)
}

/// `_Lockit(kind)`: `this->_Locktype = kind`; `this` (the run's own
/// structures have their own locks).
fn lockit(e: &mut Exec, args: &Args) -> Ret {
    e.mem().write(args.ecx, arg(args, 0));
    Ret::eax(args.ecx)
}

/// `ctype<char>::is(mask, c)`: the "C" locale's class table (the
/// facet's `_Table` at `+0xc`, `short` entries) `& mask`.
fn ctype_is(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    let table: u32 = mem.read(args.ecx + 0xc);
    #[allow(clippy::cast_possible_truncation)] // the char
    let c = arg(args, 1) as u8;
    let v: u16 = mem.read(table + 2 * u32::from(c));
    let mask = arg(args, 0) & 0xffff;
    Ret::eax(u32::from(u32::from(v) & mask != 0))
}

/// `xsputn(p, n)` (the base class's): `sputc` of each character, the
/// put area filled in runs; the count put (64-bit).
fn xsputn(e: &mut Exec, args: &Args) -> Ret {
    let (p, n) = (arg(args, 0), arg(args, 1));
    let mut done = 0u32;
    while done < n.min(0x7fff_ffff) {
        let mut b = [0u8];
        e.mem().read_bytes(p.wrapping_add(done), &mut b);
        let r = sputc(
            e,
            &Args {
                ecx: args.ecx,
                stack: vec![u32::from(b[0])],
                ..Args::default()
            },
        );
        if r.eax == u32::MAX {
            break;
        }
        done += 1;
    }
    Ret {
        edx: Some(0),
        ..Ret::eax(done)
    }
}

/// `xsgetn(p, n)` (the base class's): characters from the get area or
/// `uflow`, until `n` or `EOF`; the count read (64-bit).
fn xsgetn(e: &mut Exec, args: &Args) -> Ret {
    let (p, n) = (arg(args, 0), arg(args, 1));
    let mut done = 0u32;
    while done < n.min(0x7fff_ffff) {
        let c = uflow(
            e,
            &Args {
                ecx: args.ecx,
                ..Args::default()
            },
        )
        .eax;
        if c == u32::MAX {
            break;
        }
        #[allow(clippy::cast_possible_truncation)]
        e.mem().write(p.wrapping_add(done), c as u8);
        done += 1;
    }
    Ret {
        edx: Some(0),
        ..Ret::eax(done)
    }
}

/// `uflow()` (the base class's): `underflow()`, and past the character
/// when it gave one.
fn uflow(e: &mut Exec, args: &Args) -> Ret {
    let this = args.ecx;
    let c = virtual_call(e, this, UNDERFLOW, Vec::new()).eax;
    if c == u32::MAX {
        return Ret::eax(c);
    }
    if avail(&e.mem(), this, IGNEXT, IGCOUNT) > 0 {
        let mut mem = e.mem();
        let k = cell(&mem, this, IGCOUNT);
        set_cell(&mut mem, this, IGCOUNT, k.wrapping_sub(1));
        let q = cell(&mem, this, IGNEXT);
        set_cell(&mut mem, this, IGNEXT, q.wrapping_add(1));
    }
    Ret::eax(c)
}

/// `_Syserror_map(n)`/`_Winerror_map(n)`: the message texts are the
/// DLL's; without it, "unknown error" (what the DLL gives for a number it
/// doesn't know).
fn error_map(e: &mut Exec, _: &Args, _win: bool) -> Ret {
    let at = e.alloc(16);
    e.mem().write_bytes(at, b"unknown error\0");
    Ret::eax(at)
}

/// The `_X*` helpers: the exception they throw, as a stop (no handler
/// runs without the emulator).
fn throw(e: &mut Exec, what: &str, message: u32) -> Ret {
    let text = if message == 0 {
        String::new()
    } else {
        format!(": {}", crate::imports::read_str(&e.mem(), message, false))
    };
    e.end(Stop::Failed(
        format!("{CPP}: {what} thrown{text} (no handler without the emulator)"),
        0,
    ));
    Ret::VOID
}

/// A function the game reaches only from code that is ported: the stop
/// names it.
fn stops(e: &mut Exec, what: &str) -> Ret {
    e.end(Stop::Failed(
        format!("{CPP}: {what} has no Rust side (its callers in FFX.exe are ported)"),
        0,
    ));
    Ret::VOID
}

/// The data imports' objects without the DLLs, built at the binding
/// (`Shared::bind_imports`): `std::cin` (an `istream` at its end of input
/// with a buffer, so a read finds `eofbit | failbit`), the stream vtables
/// (entries that stop), `_BADOFF` (-1) and `ctype<char>::id` (0, given
/// out on first use).
pub const DATA: [&str; 6] = [
    "MSVCP110.dll!?cin@std@@3V?$basic_istream@DU?$char_traits@D@std@@@1@A",
    "MSVCP110.dll!??_7?$basic_ostream@DU?$char_traits@D@std@@@std@@6B@",
    "MSVCP110.dll!??_7?$basic_istream@DU?$char_traits@D@std@@@std@@6B@",
    "MSVCP110.dll!??_7?$basic_iostream@DU?$char_traits@D@std@@@std@@6B@",
    "MSVCP110.dll!?_BADOFF@std@@3_JB",
    "MSVCP110.dll!?id@?$ctype@D@std@@2V0locale@2@A",
];

/// `std::cin`'s layout: the `istream` part (vbtable pointer, `_Chcount`)
/// and its `basic_ios` at `+0x10`.
const CIN_IOS: u32 = 0x10;

/// Fills the data import `name`'s object at `at` (0x80 bytes) and the
/// traps it needs; the address the IAT slot gets.
pub fn build_data(shared: &crate::exec::Shared, mem: &mut impl Mem, name: &str, at: u32) -> u32 {
    mem.write_bytes(at, &[0; 0x80]);
    match DATA.iter().position(|d| *d == name) {
        Some(0) => {
            // vbtable at +0x60: [0, CIN_IOS]; the ios: good, "skipws |
            // dec", precision 6, a buffer word, fill ' '
            let vbtable = at + 0x60;
            mem.write(at, vbtable);
            mem.write(vbtable + 4, CIN_IOS);
            let ios = at + CIN_IOS;
            mem.write(ios + FMTFL, 0x201u32);
            mem.write(ios + 0x18, 6u32);
            mem.write(ios + MYSTRBUF, at + 0x70);
            mem.write(ios + FILLCH, b' ');
            at
        }
        Some(1..=3) => {
            // a vtable of one entry (the deleting destructor): a stop
            mem.write(at, at + 0x10);
            mem.write_bytes(at + 0x10, &[0xcc]);
            shared.add_trap(
                at + 0x10,
                Trap {
                    run: |e, _, _| stops(e, "a stream class's virtual"),
                    tag: 0,
                    conv: Conv::Thiscall,
                    words: 1,
                },
            );
            at
        }
        Some(4) => {
            mem.write(at, u64::MAX);
            at
        }
        _ => at,
    }
}

/// `std::cin.getline` without the DLLs on a good stream: the run has no
/// console input, so the read finds the end: `_Chcount` 0, `*str` 0,
/// `eofbit | failbit` (the exception mask's throw: the stop).
pub fn getline_at_end(e: &mut Exec, this: u32, ios: u32, str_at: u32) -> Ret {
    let (state, except): (u32, u32) = {
        let mem = e.mem();
        (mem.read(ios + MYSTATE), mem.read(ios + EXCEPT))
    };
    let next = (state | 3) & 0x17;
    if except & next != 0 {
        return throw(e, "std::ios_base::failure", 0);
    }
    let mut mem = e.mem();
    mem.write(this + 8, 0u64);
    if str_at != 0 {
        mem.write(str_at, 0u8);
    }
    mem.write(ios + MYSTATE, next);
    Ret::eax(this)
}
