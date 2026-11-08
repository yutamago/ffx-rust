//! M5 round 11 (`tail11`): `Settings_Parse` and `Settings_Format`
//! (`ffx_system::l11_settings`) against the original, on the image alone
//! and on a snapshot (`FFX_SNAPSHOT`). Needs `FFX_GAME_DIR`.
//!
//! The originals run an inlined `std::istringstream` / `std::ostringstream`
//! over the game's own `MSVCP110.dll`, which both oracles load
//! (`Oracle::load_cpp_runtime`); the ports use Rust strings. Both oracles
//! run the streams once before the cases, so the lazily set statics
//! (locale ids, facets) are alike. `user_malloc` is a bump allocator in
//! memory left out of the diff (the original's strings and stream buffer
//! live there, the port allocates only the by-value strings it hands
//! `Settings_SetValue`), `operator delete` returns at once, and `printf`
//! logs its format pointer and the text of its `%s` argument.
//! `Settings_SetValue` and what it calls stay original in both.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_system::l11_settings::bridge as ls;

/// Deterministic xorshift stream.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn pick<T: Copy>(&mut self, v: &[T]) -> T {
        v[self.below(v.len() as u32) as usize]
    }
}

/// The port's view of an oracle.
struct G<'a>(&'a mut Oracle);

impl Mem for G<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        Mem::write_bytes(self.0, addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

impl Host for G<'_> {
    fn call(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(c, false));
        let conv = match c {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        self.0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
    }
    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
}

/// `user_malloc(size)`.
const USER_MALLOC: u32 = 0x0068_7020;
/// `operator delete(p)`.
const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `Settings_Construct(this; path)`.
const CONSTRUCT: u32 = 0x0040_1880;

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The `printf` log: a cursor, then entries (format pointer, text, NUL).
    log: u32,
}

const LOG_LEN: u32 = 0x4_0000;
const HEAP_LEN: u32 = 0x40_0000;

impl Twins {
    fn new(exe: &std::path::Path, mut a: Oracle, mut b: Oracle) -> Self {
        a.load_cpp_runtime(exe).unwrap();
        b.load_cpp_runtime(exe).unwrap();
        let mut t = Self { a, b, log: 0 };
        // the bump heap (private): the cursor, then the blocks
        let heap = t.zeros(HEAP_LEN);
        t.w32(heap, heap + 0x10);
        t.a.add_private(heap..heap + HEAP_LEN);
        t.b.add_private(heap..heap + HEAP_LEN);
        let mut c = vec![0x51, 0xa1]; // push ecx; mov eax, [cursor]
        c.extend(heap.to_le_bytes());
        c.extend([0x8b, 0x4c, 0x24, 0x08]); // mov ecx, [esp+8]
        c.extend([0x83, 0xc1, 0x1f, 0x83, 0xe1, 0xf0]); // add ecx, 31; and ecx, -16
        c.extend([0x01, 0x0d]);
        c.extend(heap.to_le_bytes()); // add [cursor], ecx
        c.extend([0x59, 0xc3]); // pop ecx; ret
        let malloc = t.code(&c);
        t.jump(USER_MALLOC, malloc);
        let delete = t.code(&[0xc3]);
        t.jump(OPERATOR_DELETE, delete);
        // printf: the format pointer and the text, logged
        t.log = t.zeros(LOG_LEN);
        let log = t.log;
        t.w32(log, log + 4);
        let mut c = vec![0x56, 0x57, 0x8b, 0x3d]; // push esi; push edi; mov edi, [log]
        c.extend(log.to_le_bytes());
        c.extend([0x8b, 0x44, 0x24, 0x0c, 0xab]); // mov eax, [esp+0xc]; stosd
        c.extend([0x8b, 0x74, 0x24, 0x10]); // mov esi, [esp+0x10]
        c.extend([0xac, 0xaa, 0x84, 0xc0, 0x75, 0xfa]); // lodsb; stosb; test al, al; jnz
        c.extend([0x89, 0x3d]);
        c.extend(log.to_le_bytes()); // mov [log], edi
        c.extend([0x5f, 0x5e, 0x31, 0xc0, 0xc3]); // pop edi; pop esi; xor eax, eax; ret
        let printf = t.code(&c);
        t.a.bind_import("printf", Some(printf)).unwrap();
        t.b.bind_import("printf", Some(printf)).unwrap();
        // the streams once in both: the statics they set on first use
        let this = t.zeros(0x640);
        let buf = t.zeros(0x2000);
        let text = t.alloc(b"Language=en\nQuality=VQ_HIGH\n\0");
        for o in [&mut t.a, &mut t.b] {
            let at = o.rebase(CONSTRUCT);
            o.call(at, Conv::Thiscall(this), &[0]).unwrap();
            let at = o.rebase(ls::FORMAT);
            o.call(at, Conv::Thiscall(this), &[buf]).unwrap();
            let at = o.rebase(ls::PARSE);
            o.call(at, Conv::Thiscall(this), &[text]).unwrap();
        }
        t.w32(log, log + 4);
        t.a.clear_dirty();
        t.b.clear_dirty();
        t
    }

    fn all() -> Vec<(&'static str, Self)> {
        let Some(dir) = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from) else {
            eprintln!("FFX_GAME_DIR is not set; skipping");
            return vec![];
        };
        let exe = dir.join("FFX.exe");
        let mut out = vec![(
            "image",
            Self::new(
                &exe,
                Oracle::load(&exe).unwrap(),
                Oracle::load(&exe).unwrap(),
            ),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push((
                "snapshot",
                Self::new(
                    &exe,
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                ),
            ));
        } else {
            eprintln!("FFX_SNAPSHOT is not set; image only");
        }
        out
    }

    fn alloc(&mut self, bytes: &[u8]) -> u32 {
        let x = self.a.alloc_bytes(bytes).unwrap();
        assert_eq!(self.b.alloc_bytes(bytes).unwrap(), x);
        x
    }

    fn zeros(&mut self, n: u32) -> u32 {
        self.alloc(&vec![0; n as usize])
    }

    fn code(&mut self, c: &[u8]) -> u32 {
        let x = self.a.alloc_code(c).unwrap();
        assert_eq!(self.b.alloc_code(c).unwrap(), x);
        x
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    /// A `jmp` from the original `sym` to `to`.
    fn jump(&mut self, sym: u32, to: u32) {
        let at = self.a.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(to.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
    }

    /// The original `sym` (thiscall on `this`, one stack word) in `a`, the
    /// port in `b`: `eax` and every byte written must agree.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        this: u32,
        arg: u32,
        port: impl FnOnce(&mut G) -> u32,
    ) {
        let at = self.a.rebase(sym);
        let ra = self
            .a
            .call(at, Conv::Thiscall(this), &[arg])
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let eax = port(&mut G(&mut self.b));
        assert_eq!(ra.eax, eax, "{what}: eax");
        let d = self.a.diff(&mut self.b).unwrap();
        if !d.is_empty() {
            let log_a = Oracle::read(&mut self.a, self.log, 0x200).unwrap();
            let log_b = Oracle::read(&mut self.b, self.log, 0x200).unwrap();
            panic!(
                "{what}: memory differs at {} bytes, first {:x?}\nlog a {}\nlog b {}",
                d.len(),
                &d[..d.len().min(8)],
                String::from_utf8_lossy(&log_a),
                String::from_utf8_lossy(&log_b)
            );
        }
        self.a.clear_dirty();
        self.b.clear_dirty();
    }
}

/// A settings object: the 24 values (valid indices for keys with several
/// value strings, any word now and then for the others), the binding
/// tables and their count; `valid` false gives a preset past the table's
/// columns.
fn settings(t: &mut Twins, rng: &mut Rng, valid: bool) -> u32 {
    let this = t.zeros(0x640);
    let counts = [
        8u32, 1, 3, 4, 2, 1, 1, 1, 1, 1, 2, 5, 4, 3, 1, 2, 6, 2, 4, 2, 3, 1, 1, 1,
    ];
    for (k, &n) in (0u32..).zip(counts.iter()) {
        let v = if n > 1 {
            rng.below(n)
        } else {
            match rng.below(4) {
                0 => rng.next(),
                1 => rng.below(200),
                2 => (rng.below(2000) as i32 - 1000).cast_unsigned(),
                _ => rng.pick(&[0, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 0x0780_0438]),
            }
        };
        t.w32(this + 4 * k, v);
    }
    if !valid {
        let v = rng.pick(&[5u32, 0xffff_ffff, 7]);
        t.w32(this + 0xc, v);
    }
    for k in 0..3 * 0x51u32 {
        let v = if rng.below(3) == 0 {
            rng.next()
        } else {
            rng.below(0x30)
        };
        t.w32(this + 0x60 + 4 * k, v);
    }
    let n = rng.pick(&[0u32, 1, 2, 5, 0x51, 0xffff_ffff]);
    t.w32(this + 0x42c, n);
    this
}

/// `Settings_Format`: every key's kind (value strings, Resolution, the
/// three binding lists of 0, 1, several, 0x51 and a negative count, plain
/// numbers negative and past 16 bits), with and without a buffer.
#[test]
fn l11_settings_format() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x22f0_0011);
        for i in 0..200 {
            let this = settings(&mut t, &mut rng, true);
            let buf = if rng.below(5) == 0 {
                0
            } else {
                t.zeros(0x2000)
            };
            if buf != 0 {
                let junk: Vec<u8> = (0..0x2000).map(|_| rng.next() as u8 | 1).collect();
                t.write(buf, &junk);
            }
            t.check(
                &format!("{name} {i} buf {buf:#x}"),
                ls::FORMAT,
                this,
                buf,
                |g| ls::format(g, this, buf),
            );
        }
    }
}

/// One line of a settings text: valid and invalid keys and values, in
/// any case, with `\r`, long and empty.
fn line(rng: &mut Rng) -> Vec<u8> {
    const KEYS: [&str; 24] = [
        "Language",
        "Resolution",
        "ScreenMode",
        "Quality",
        "Mute",
        "MasterVolume",
        "VoiceVolume",
        "MusicVolume",
        "SFXVolume",
        "Brightness",
        "VSync",
        "Anisotropic",
        "Shadow",
        "TextureQuality",
        "MSAA",
        "ColorCorrection",
        "PostAA",
        "UnSharpMask",
        "HDAO",
        "Invincible",
        "EncounterRate",
        "ffxKeyBinding",
        "ffx2KeyBinding",
        "lmKeyBinding",
    ];
    const VALUES: [&str; 24] = [
        "en",
        "1920*1080",
        "SM_WINDOW",
        "VQ_CUSTOM",
        "On",
        "80",
        "-5",
        "100",
        "0",
        "50",
        "Off",
        "16",
        "3",
        "TQ_LOW",
        "4",
        "On",
        "PAA_SMAA2",
        "Off",
        "High",
        "On",
        "ER_HIGH",
        "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20",
        "3,4,5",
        "0,1,2,3,4,5,6,7,8,9,10,11,12",
    ];
    let k = rng.below(24) as usize;
    let mut key: Vec<u8> = KEYS[k].bytes().collect();
    match rng.below(8) {
        0 => key.make_ascii_uppercase(),
        1 => key.make_ascii_lowercase(),
        2 => key.push(b'x'),
        3 => key = b"NotAKeyAtAllButLong".to_vec(),
        _ => {}
    }
    let mut value: Vec<u8> = match rng.below(6) {
        0 => b"bogus".to_vec(),
        1 => b"averyveryverylongvaluestring".to_vec(),
        2 => format!("{}", rng.below(100_000) as i32 - 50_000).into_bytes(),
        3 => b"VQ_LOW".to_vec(),
        _ => VALUES[k].bytes().collect(),
    };
    if rng.below(4) == 0 {
        value.make_ascii_lowercase();
    }
    let mut l = match rng.below(12) {
        0 => b"no equals sign here at all".to_vec(),
        1 => b"short".to_vec(),
        2 => Vec::new(),
        3 => {
            let mut l = key;
            l.extend(b"=a=b");
            l
        }
        _ => {
            let mut l = key;
            l.push(b'=');
            l.extend(value);
            l
        }
    };
    if rng.below(8) == 0 {
        l.push(b'\r');
    }
    l
}

/// `Settings_Parse`: texts of 0..30 lines (with and without the last
/// `\n`, empty), every key or some (the binding lists seen or not),
/// presets 0..3 and past the table, languages that set the default
/// bindings' globals or not.
#[test]
fn l11_settings_parse() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1e00_0011);
        for i in 0..200 {
            let valid = rng.below(4) != 0;
            let this = settings(&mut t, &mut rng, valid);
            // the default bindings' entries 18 and 19, so each case shows
            // whether the language sets them
            for table in ls::DEFAULT_BINDINGS {
                let at = t.a.rebase(table);
                let (x, y) = (rng.below(0x30), rng.below(0x30));
                t.w32(at + 0x48, x);
                t.w32(at + 0x4c, y);
            }
            let n = rng.pick(&[0u32, 1, 2, 5, 12, 30, 60]);
            let seps: [&[u8]; 7] = [b"\n", b"\n", b"\r\n", b" ", b"\t", b"  \x0b\x0c\n", b"\xa0"];
            let mut text = Vec::new();
            if rng.below(6) == 0 {
                text.extend(rng.pick(&seps));
            }
            for k in 0..n {
                if k > 0 {
                    text.extend(rng.pick(&seps));
                }
                text.extend(line(&mut rng));
            }
            if rng.below(2) == 0 {
                text.extend(rng.pick(&seps));
            }
            if rng.below(20) == 0 {
                // all three binding lists and a custom preset
                text.extend(
                    b"\nffxKeyBinding=1,2,3 ffx2KeyBinding=4,5 lmKeyBinding=6\nQuality=VQ_CUSTOM",
                );
            }
            text.push(0);
            let at = t.alloc(&text);
            let frame = t.a.stack_range().start + 0x40;
            let what = format!("{name} {i}: {}", String::from_utf8_lossy(&text));
            t.check(&what, ls::PARSE, this, at, |g| {
                ls::parse(g, frame, this, at)
            });
        }
    }
}

/// `Input_BuildPadMap` (`ffx_system::l11_padmap`): the pair array empty or
/// holding pairs, its capacity below 75, at 75 and above (so the pushes
/// grow it or not), with no data yet; random settings words. The pair
/// array's own functions stay original in both oracles; `user_malloc`
/// is a bump allocator in compared memory here (both sides allocate
/// alike). The last pair's slot is never the null address, so the
/// original's null check is not reached.
#[test]
fn l11_build_pad_map() {
    use ffx_system::l11_padmap::bridge as pm;
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x15c0_0011);
        let heap = t.zeros(0x40_0000);
        t.w32(heap, heap + 0x10);
        let mut c = vec![0x51, 0xa1]; // push ecx; mov eax, [cursor]
        c.extend(heap.to_le_bytes());
        c.extend([0x8b, 0x4c, 0x24, 0x08]); // mov ecx, [esp+8]
        c.extend([0x83, 0xc1, 0x1f, 0x83, 0xe1, 0xf0]); // add ecx, 31; and ecx, -16
        c.extend([0x01, 0x0d]);
        c.extend(heap.to_le_bytes()); // add [cursor], ecx
        c.extend([0x59, 0xc3]); // pop ecx; ret
        let malloc = t.code(&c);
        t.jump(USER_MALLOC, malloc);
        // user_free: nothing to do for the bump heap
        let free = t.code(&[0xc3]);
        t.jump(0x0068_7010, free);
        let mut seen = [0u32; 3];
        for i in 0..150 {
            let settings = t.zeros(0x400);
            for k in 0..0x100 {
                let v = rng.next();
                t.w32(settings + 4 * k, v);
            }
            let at = t.a.rebase(pm::SETTINGS);
            t.w32(at, settings);
            let manager = t.zeros(0xb0);
            let array = manager + 0x9c;
            let cap = rng.pick(&[0u32, 1, 10, 0x4a, 0x4b, 0x4c, 200]);
            let data = if cap == 0 && rng.below(2) == 0 {
                0
            } else {
                t.zeros(8 * cap.max(1))
            };
            let count = if cap == 0 { 0 } else { rng.below(cap) };
            t.w32(array, count);
            t.w32(array + 4, cap);
            t.w32(array + 8, data);
            let frame = t.a.stack_range().start + 0x40;
            let what = format!("{name} {i} cap {cap} count {count}");
            let at = t.a.rebase(0x0063_15c0);
            let ra =
                t.a.call(at, Conv::Thiscall(manager), &[])
                    .unwrap_or_else(|e| panic!("{what}: {e}"));
            let eax = pm::build_pad_map(&mut G(&mut t.b), frame, manager);
            assert_eq!(ra.eax, eax, "{what}: eax");
            let d = t.a.diff(&mut t.b).unwrap();
            assert!(
                d.is_empty(),
                "{what}: differs at {:x?}",
                &d[..d.len().min(8)]
            );
            t.a.clear_dirty();
            t.b.clear_dirty();
            seen[usize::from(cap >= 0x4b)] += 1;
            seen[2] += u32::from(data == 0);
        }
        eprintln!("{name}: {seen:?}");
    }
}
