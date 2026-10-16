//! Every row of `ffx_auto` (the trivial functions `tools/autoport.py` ports)
//! against the original machine code: random arguments, random contents
//! behind every pointer and index the row uses, random globals, and the
//! snapshot's own globals when `FFX_SNAPSHOT` is set. `eax` is compared in
//! all 32 bits (for a row that leaves it alone, the value it had before the
//! call), and every byte written (`a.diff(&mut b)`).
//!
//! A failing row is reported in the file `AUTOPORT_FAILURES` names (one
//! `address<TAB>reason` line each) as well as by the test failing;
//! `tools/autoport.py --verify` drops those rows from the table.

use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use ffx_auto::{Kind, Row, Step, Val};
use ffx_bridge::Mem;
use ffx_oracle::{Conv, Oracle, Pages};

/// Bytes of each block an argument or loaded pointer points into (the
/// pointer is in the middle: the generator keeps offsets within ±0x1f000).
const BLOCK: usize = 0x4_0000;

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// Any value, often an edge case.
    fn value(&mut self) -> u32 {
        const EDGES: [u32; 10] = [
            0,
            1,
            2,
            0x7f,
            0x80,
            0xff,
            0xffff,
            0x7fff_ffff,
            0x8000_0000,
            u32::MAX,
        ];
        let r = self.next();
        if r.is_multiple_of(4) {
            EDGES[(r >> 8) as usize % EDGES.len()]
        } else {
            self.next()
        }
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(n + 4);
        while v.len() < n {
            v.extend_from_slice(&self.next().to_le_bytes());
        }
        v.truncate(n);
        v
    }
}

/// Where the oracles come from.
enum Source {
    Image(PathBuf),
    Snapshot(PathBuf, std::sync::Arc<Pages>),
}

impl Source {
    fn get() -> Option<Self> {
        let Some(dir) = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from) else {
            eprintln!("FFX_GAME_DIR is not set; skipping");
            return None;
        };
        let exe = dir.join("FFX.exe");
        Some(match std::env::var_os("FFX_SNAPSHOT") {
            Some(s) => {
                let pages = Pages::read(PathBuf::from(s)).unwrap();
                Source::Snapshot(exe, pages)
            }
            None => Source::Image(exe),
        })
    }

    fn oracle(&self) -> Oracle {
        match self {
            Source::Image(exe) => Oracle::load(exe).unwrap(),
            Source::Snapshot(exe, pages) => Oracle::from_snapshot(exe, pages).unwrap(),
        }
    }

    /// Two oracles started alike, with a `mov eax, x; ret` stub in some
    /// padding of the code (to set `eax` before a call).
    fn pair(&self) -> (Oracle, Oracle, u32) {
        let (mut a, mut b) = (self.oracle(), self.oracle());
        let start = a.rebase(0x0040_1000);
        let code = Oracle::read(&mut a, start, 0x1_0000).unwrap();
        let at = code
            .windows(8)
            .position(|w| w.iter().all(|&x| x == 0xcc))
            .expect("int3 padding in the first 64 KiB of .text");
        let stub = start + u32::try_from(at).unwrap();
        a.stub_return(stub, 0).unwrap();
        b.stub_return(stub, 0).unwrap();
        (a, b, stub)
    }

    fn snapshot(&self) -> bool {
        matches!(self, Source::Snapshot(..))
    }
}

/// The same block of random bytes in both oracles; returns its middle.
fn block(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> u32 {
    let bytes = rng.bytes(BLOCK);
    let p = a.alloc_bytes(&bytes).unwrap();
    assert_eq!(b.alloc_bytes(&bytes).unwrap(), p);
    p + u32::try_from(BLOCK / 2).unwrap()
}

fn put(a: &mut Oracle, b: &mut Oracle, addr: u32, size: u8, v: u32) {
    let bytes = &v.to_le_bytes()[..usize::from(size)];
    a.write(addr, bytes).unwrap();
    b.write(addr, bytes).unwrap();
}

/// Whether an address expression starts from the image.
fn from_image(v: &Val) -> bool {
    match v {
        Val::Image(_) => true,
        Val::Bin(_, x, y) => from_image(x) || from_image(y),
        Val::Un(_, x) => from_image(x),
        _ => false,
    }
}

/// The arguments for one call, after preparing what the row loads.
fn inputs(
    a: &mut Oracle,
    b: &mut Oracle,
    row: &Row,
    rng: &mut Xorshift,
    keep_globals: bool,
) -> Vec<u32> {
    let mut args = Vec::new();
    for k in row.args {
        let v = match k {
            Kind::Value => rng.value(),
            Kind::Pointer => block(a, b, rng),
            Kind::Index => rng.next() % 8,
        };
        args.push(v);
    }
    let mut tmps = [0u32; ffx_auto::MAX_TMPS];
    // bytes already given a pointer or index, not to be randomised again
    let mut fixed: Vec<(u32, u32)> = Vec::new();
    for s in row.steps {
        let Step::Load {
            tmp,
            size,
            addr,
            kind,
        } = *s
        else {
            continue;
        };
        let at = ffx_auto::eval(b, &addr, &args, &tmps);
        let end = at.wrapping_add(u32::from(size));
        let free = !fixed.iter().any(|&(x, y)| at < y && x < end);
        let v = match kind {
            // a slot loaded again keeps what an earlier load found there
            Kind::Pointer if free => block(a, b, rng),
            Kind::Index if free => rng.next() % 8,
            _ => {
                if kind == Kind::Value && from_image(&addr) && !keep_globals && free {
                    put(a, b, at, size, rng.value());
                }
                let mut buf = [0u8; 4];
                b.read_bytes(at, &mut buf[..usize::from(size)]);
                tmps[usize::from(tmp)] = u32::from_le_bytes(buf);
                continue;
            }
        };
        put(a, b, at, size, v);
        fixed.push((at, end));
        tmps[usize::from(tmp)] = v & (u32::MAX >> (32 - 8 * u32::from(size)));
    }
    args
}

/// One call of a row on both oracles; the difference, if any.
fn check(
    a: &mut Oracle,
    b: &mut Oracle,
    stub: u32,
    row: &Row,
    args: &[u32],
    mark: u32,
) -> Option<String> {
    a.stub_return(stub, mark).unwrap();
    b.stub_return(stub, mark).unwrap();
    a.clear_dirty();
    b.clear_dirty();
    a.call(stub, Conv::Cdecl, &[]).unwrap();
    let conv = match row.conv {
        ffx_auto::Conv::Cdecl => Conv::Cdecl,
        ffx_auto::Conv::Stdcall => Conv::Stdcall,
        ffx_auto::Conv::Fastcall => Conv::Fastcall,
        ffx_auto::Conv::Thiscall => Conv::Thiscall(args[0]),
    };
    let stack = if row.conv == ffx_auto::Conv::Thiscall {
        &args[1..]
    } else {
        args
    };
    let at = a.rebase(row.address);
    let got = match a.call(at, conv, stack) {
        Ok(r) => r.eax,
        Err(e) => return Some(format!("the original fails: {e}")),
    };
    let want = match catch_unwind(AssertUnwindSafe(|| ffx_auto::call(b, row, args))) {
        Ok(v) => row.result.map_or(mark, |_| v),
        Err(_) => return Some("the row panics".into()),
    };
    if got != want {
        return Some(format!(
            "eax {got:#010x}, the row {want:#010x} (args {args:x?})"
        ));
    }
    let diff = a.diff(b).unwrap();
    if !diff.is_empty() {
        let mut s = format!("{} bytes differ (args {args:x?}):", diff.len());
        for (at, x, y) in diff.iter().take(4) {
            let _ = write!(s, " {at:#x} {x:#04x}/{y:#04x}");
        }
        return Some(s);
    }
    None
}

#[test]
fn auto_rows_match_the_original() {
    let Some(source) = Source::get() else {
        return;
    };
    let (mut a, mut b, mut stub) = source.pair();
    let rounds = if source.snapshot() { 8 } else { 32 };
    let mut failures = Vec::new();
    for row in ffx_auto::ROWS {
        let mut rng = Xorshift(row.address ^ 0x5eed_a070);
        for round in 0..rounds {
            a.reset_heap();
            b.reset_heap();
            let keep = source.snapshot() && round == 0;
            let args = inputs(&mut a, &mut b, row, &mut rng, keep);
            let mark = rng.next();
            if let Some(why) = check(&mut a, &mut b, stub, row, &args, mark) {
                failures.push((row.address, why));
                // the oracles differ now: start again
                (a, b, stub) = source.pair();
                break;
            }
        }
    }
    if let Some(path) = std::env::var_os("AUTOPORT_FAILURES") {
        let mut text = String::new();
        for (address, why) in &failures {
            let _ = writeln!(text, "{address:#010x}\t{why}");
        }
        std::fs::write(path, text).unwrap();
    }
    for (address, why) in &failures {
        eprintln!("{address:#010x}: {why}");
    }
    assert!(
        failures.is_empty(),
        "{} rows differ from the original",
        failures.len()
    );
    eprintln!("{} rows match the original", ffx_auto::ROWS.len());
}
